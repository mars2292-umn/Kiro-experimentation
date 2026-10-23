//! The verification job, `cargo xtask verify`: the single entry point that
//! CI and local runs use.
//!
//! Steps:
//!
//! 1. Static checks (all rules; see the crate documentation), including
//!    the Profile check (`cargo xtask profile --check`), the proof
//!    assumption gate (R50.2), and the Trust_Base_Register and Gap_Register
//!    checks (R48, R54).
//! 2. Network sandbox (R58.4): establish it and prove it denies a
//!    connection.
//! 3. Target build: every crate whose policy lists the Target or marks it a
//!    Flight_Build root, in the release profile, inside the sandbox, with
//!    `--remap-path-prefix=<root>=/rsk` (R59.1). This is a superset of the
//!    Flight_Build graph; proc-macros and build scripts compile for the
//!    host.
//! 4. Host build: the whole workspace with all targets, inside the sandbox.
//! 5. Build_Manifest (R33.3, R59.2): the manifest of this build with the
//!    binary hash of every Flight_Build binary and the tool versions; it
//!    opens the evidence store that the later steps write to.
//! 6. Post-build checks: the Kernel's third-party Target crates (R6.1), the
//!    third-party crates of the Target build (PR-36/R55.1), every
//!    build-script output (no `RUSTC_BOOTSTRAP`), and the dependency report
//!    with its Trust_Base_Register gate (R58.2/R58.5).
//! 7. R58.4 probe fixture: a crate whose build script and proc-macro fail
//!    the build unless their connection attempt is denied.
//! 8. Tests: `cargo test` for the workspace, including the fixture tests.
//! 8a. Link_Checker (R37) and 8b. Config_Checker (R26.5) on every
//!     Flight_Build binary with a Task_Model; 8c. Analyzer (R29) on the same
//!     binaries with placeholder timing; 8d. Profile_Lint (R3.2, R3.4,
//!     R3.5) over the Kernel and Application sources, then Clippy's
//!     deny-lists on the Target crates.
//! 9. Kernel_Proofs: `cargo-verus verify` for every Kernel crate with the
//!    pinned Verus and Z3 (R44.6), and the PAR-01 cross-check with Verus's
//!    `line_count`.
//! 10. Kani_Harnesses: `cargo kani` for every Kernel crate that defines a
//!     harness, passing only when every check passes (R45.5, R45.7).
//! 10b. Dispatch spike on QEMU (task 3.1): every Flight_Build binary of
//!     the Target build whose policy entry marks it a QEMU spike runs on
//!     `qemu-system-arm -M mps2-an386`; the run passes when the binary
//!     exits through semihosting with code 0 (it prints `RSK-OK`). Skipped
//!     with a warning when QEMU is not installed.
//! 10c. Profile_Conformance_Suite (R3.6, R3.7, R3.11): every case of
//!     docs/conformance_suite.toml, one Evidence_Item each (`conf-PR-nn-cc`,
//!     `fi-PR-nn`, `accepted-<variant>`), and the PO proof-artifact check
//!     (R3.8, R3.10).
//! 11. Evidence_Items (R50.3): one item per step, this one included
//!     (written last, once its own outcome is recorded), plus the proof items
//!     written by steps 9 and 10.
//!
//! All builds use a fresh target directory (`target/xtask-verify`, removed at
//! the start unless `--keep-target` is given), so every build script and
//! proc-macro of the workspace runs inside the sandbox during the job.
//! Outside the sandbox run only the bootstrap build of xtask itself by
//! `cargo xtask` (before the job starts, in `target/debug`) and the
//! `cargo metadata` and `rustc` queries of the static checks, which execute
//! no build script or proc-macro.
//!
//! Proof tools are required (`--proofs required`, the default): R44.6 and
//! R50.1 make the proofs part of every verification job. `--proofs skip`
//! runs the job without them for a developer machine that lacks Verus or
//! Kani; the job then reports a warning and writes no proof Evidence_Item.

use std::path::{Path, PathBuf};

use crate::checks::{self, deps, features, kernel_std};
use crate::cmd::{overridden_incoming_env, Cmd};
use crate::diag::{Diagnostic, Report, Rule};
use crate::evidence::{self, Item, Kind, Store, Tool, Verdict};
use crate::manifest::{flight_binaries, Manifest};
use crate::proofs;
use crate::sandbox::{Mechanism, Mode, Sandbox};
use crate::tools::{Kani, Revision, Verus};
use crate::units::{parse_messages, CfgContext, Outcome};
use crate::workspace::Workspace;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proofs {
    Required,
    Skip,
}

#[derive(Clone, Debug)]
pub struct Options {
    pub keep_target: bool,
    pub sandbox: Mode,
    pub proofs: Proofs,
    /// Where Evidence_Items and the Build_Manifest are written
    /// (`target/xtask-verify/evidence` by default).
    pub evidence_dir: Option<PathBuf>,
}

/// The remapped prefix under which every Flight_Build source path appears.
pub const REMAP_PREFIX: &str = "/rsk";

struct Job {
    all: Vec<Diagnostic>,
    steps: usize,
    total_steps: usize,
    outcomes: Vec<StepOutcome>,
}

struct StepOutcome {
    id: &'static str,
    name: String,
    covers: Vec<&'static str>,
    verdict: Verdict,
    commands: Vec<String>,
    notes: Vec<String>,
}

impl Job {
    fn heading(&mut self, title: &str) {
        self.steps += 1;
        println!("\n== [{}/{}] {title}", self.steps, self.total_steps);
    }

    /// Prints the contents of `report` and records its diagnostics in the
    /// job; returns the notes.
    fn drain(&mut self, report: &mut Report) -> Vec<String> {
        let notes: Vec<String> = report.notes.clone();
        for note in report.notes.drain(..) {
            println!("   {note}");
        }
        for warning in report.warnings.drain(..) {
            println!("   warning: {warning}");
        }
        for diagnostic in report.diagnostics.drain(..) {
            println!("   {diagnostic}");
            self.all.push(diagnostic);
        }
        notes
    }

    /// Prints and records the contents of `report`, and records the step
    /// outcome: the step passed if it added no diagnostic.
    fn flush(&mut self, report: &mut Report, id: &'static str, name: &str, covers: &[&'static str], commands: Vec<String>) {
        let before = self.all.len();
        let notes = self.drain(report);
        self.outcomes.push(StepOutcome {
            id,
            name: name.to_string(),
            covers: covers.to_vec(),
            verdict: Verdict::from_bool(self.all.len() == before),
            commands,
            notes,
        });
    }

    fn finish(self) -> bool {
        println!("\n== summary");
        if self.all.is_empty() {
            println!("   verification passed: 0 violations");
            true
        } else {
            for diagnostic in &self.all {
                println!("   {diagnostic}");
            }
            println!("   verification FAILED: {} violation(s)", self.all.len());
            false
        }
    }
}

fn run_step(cmd: &Cmd, sandbox: &Sandbox, report: &mut Report, what: &str) -> bool {
    println!("   $ {}", cmd.display());
    match cmd.status(Some(sandbox)) {
        Ok(true) => true,
        Ok(false) => {
            report.error(Rule::Step, None, format!("{what} failed (`{}`)", cmd.display()));
            false
        }
        Err(e) => {
            report.error(Rule::Step, None, format!("cannot run `{}`: {e}", cmd.display()));
            false
        }
    }
}

/// The crates the Target build compiles: the Flight_Build roots and every
/// crate whose policy lists the Target.
pub fn target_crates(ws: &Workspace) -> Vec<String> {
    ws.policy
        .crates
        .iter()
        .filter(|(_, c)| c.flight || c.targets.contains(&ws.policy.target))
        .map(|(name, _)| name.clone())
        .collect()
}

/// Crates whose policy lists no host target (Flight_Build binaries).
pub fn target_only_crates(ws: &Workspace) -> Vec<String> {
    ws.policy
        .crates
        .iter()
        .filter(|(_, c)| !c.targets.iter().any(|t| t == crate::policy::HOST))
        .map(|(name, _)| name.clone())
        .collect()
}

/// The `CARGO_ENCODED_RUSTFLAGS` value of the Target build: the path remap
/// that keeps the checkout path out of Flight_Build binaries (R59.1).
pub fn target_rustflags(root: &Path) -> String {
    format!("--remap-path-prefix={}={REMAP_PREFIX}", crate::workspace::canonical(root).display())
}

/// Builds [`target_crates`] for the Target in the release profile and
/// returns the compilation units from Cargo's JSON messages.
pub fn target_build(ws: &Workspace, target_dir: &Path, sandbox: Option<&Sandbox>) -> Result<Outcome, String> {
    let triple = &ws.policy.target;
    let mut cmd = Cmd::cargo_frozen("build")
        .args(["--release", "--target", triple, "--message-format=json-render-diagnostics"])
        .arg("--manifest-path")
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir)
        .env("CARGO_ENCODED_RUSTFLAGS", target_rustflags(&ws.root));
    for name in target_crates(ws) {
        cmd = cmd.args(["-p", &name]);
    }
    println!("   $ {}", cmd.display());
    let out = cmd
        .output_stdout(sandbox)
        .map_err(|e| format!("cannot run the Target build: {e}"))?;
    if !out.status.success() {
        return Err("the Target build failed".to_string());
    }
    Ok(parse_messages(&String::from_utf8_lossy(&out.stdout), target_dir, triple))
}

/// The post-build checks: R6.1 for the third-party crates of the Kernel's
/// Target graph, PR-36/R55.1 for the third-party crates of the Target build,
/// and every build-script output under `target_dir`.
pub fn post_build_checks(ws: &Workspace, outcome: &Outcome, target_dir: &Path, report: &mut Report) {
    match CfgContext::load(&ws.root, &ws.policy.target) {
        Ok(ctx) => {
            kernel_std::check_third_party(ws, outcome, &ctx, report);
            features::check_flight_units(ws, outcome, &ctx, report);
        }
        Err(e) => report.error(Rule::Step, None, e),
    }
    features::check_build_script_outputs(ws, target_dir, report);
}

fn remove_dir(dir: &Path, report: &mut Report) {
    if dir.exists() {
        if let Err(e) = std::fs::remove_dir_all(dir) {
            report.error(Rule::Step, None, format!("cannot remove {}: {e}", dir.display()));
        }
    }
}

/// Runs the job; returns whether it passed.
pub fn run(root: &Path, options: Options) -> bool {
    let mut job = Job {
        all: Vec::new(),
        steps: 0,
        total_steps: 17,
        outcomes: Vec::new(),
    };
    let mut report = Report::default();
    println!("rsk verification job: {}", root.display());

    // ---------------------------------------------------------- 1. static
    job.heading("static checks");
    // The repository checks include the assumption gate (R50.2) and the
    // Trust_Base_Register and Gap_Register checks (R48, R54).
    let ws = checks::repository(root, &mut report);
    crate::profile_gen::generate(root, true, &mut report);
    let own_lines = ws.as_ref().map(kernel_line_total);
    for name in overridden_incoming_env() {
        report.warn(format!("{name} is set in the environment; the job overrides it for every command"));
    }
    // R24.4: the committed Generator outputs are the ones of the declaration.
    crate::gen::run(root, true, &mut report);
    job.flush(
        &mut report,
        "check-static-checks",
        "static checks",
        &[
            "R1.1", "R1.2", "R1.3", "R1.4", "R1.5", "R1.6", "R2.1", "R2.2", "R2.3", "R4.1", "R4.2", "R4.3", "R4.4",
            "R4.5", "R4.6", "R4.7", "R27.3", "R57.1", "R57.5", "R6.1", "R6.2", "R6.3", "R6.4", "PR-20", "PR-36",
            "R50.2", "R55.1", "R55.2", "R55.5", "R58.1", "R58.4", "R24.4", "R23.6", "R48.1", "R48.2", "R48.3",
            "R54.1", "R54.2", "R54.3", "R54.4",
        ],
        vec!["cargo xtask check".to_string(), "cargo xtask profile --check".to_string(), "cargo xtask gen --check".to_string()],
    );
    let Some(ws) = ws else {
        println!("   the workspace could not be loaded; skipping the builds");
        return job.finish();
    };

    // --------------------------------------------------------- 2. sandbox
    job.heading("network sandbox (R58.4)");
    let probe_exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("xtask"));
    let sandbox = match Sandbox::establish(options.sandbox, &probe_exe) {
        Ok(sandbox) if sandbox.mechanism == Mechanism::None => {
            report.warn(
                "network denial is NOT enforced (--network-sandbox off); only the offline Cargo configuration \
                 applies, so this run is not R58.4 evidence",
            );
            sandbox
        }
        Ok(sandbox) => {
            report.note(format!(
                "{}: a connection to {} is denied; builds, build scripts, proc-macros, tests, and proofs run inside it",
                sandbox.describe(),
                crate::sandbox::PROBE_ADDR
            ));
            sandbox
        }
        Err(e) => {
            report.error(Rule::Offline, None, e);
            job.flush(&mut report, "check-network-sandbox", "network sandbox", &["R58.4"], vec![]);
            return job.finish();
        }
    };
    let target_dir = root.join("target").join("xtask-verify");
    if options.keep_target {
        report.warn(format!(
            "--keep-target: {} is reused, so build scripts and proc-macros that are up to date do not run again",
            ws.rel(&target_dir)
        ));
    } else {
        remove_dir(&target_dir, &mut report);
        report.note(format!(
            "fresh target directory {}: every build script and proc-macro runs in this job",
            ws.rel(&target_dir)
        ));
    }
    job.flush(
        &mut report,
        "check-network-sandbox",
        "network sandbox",
        &["R58.4"],
        vec!["cargo xtask probe-network".to_string()],
    );
    let manifest_path = root.join("Cargo.toml");
    let triple = ws.policy.target.clone();

    // ---------------------------------------------------- 3. Target build
    job.heading(&format!("build for the Target ({triple}, release)"));
    let roots: Vec<&str> = ws.policy.flight_roots().collect();
    report.note(format!("Flight_Build roots: {}", roots.join(", ")));
    report.note(format!("crates built for the Target: {}", target_crates(&ws).join(", ")));
    report.note(format!("CARGO_ENCODED_RUSTFLAGS={} (R59.1)", target_rustflags(root)));
    let outcome = match target_build(&ws, &target_dir, Some(&sandbox)) {
        Ok(outcome) => {
            let on_target = outcome.units.iter().filter(|u| u.for_target).count();
            report.note(format!(
                "{} units compiled ({on_target} for the Target, {} for the host), {} build scripts run",
                outcome.units.len(),
                outcome.units.len() - on_target,
                outcome.scripts.len()
            ));
            outcome
        }
        Err(e) => {
            report.error(Rule::Step, None, e);
            Outcome::default()
        }
    };
    job.flush(
        &mut report,
        "test-target-build",
        "Target build",
        &["R55.1", "R55.2", "R58.1", "R58.4", "R59.1"],
        vec![format!("cargo build --frozen --release --target {triple} (Flight_Build roots)")],
    );

    // ------------------------------------------------------ 4. host build
    job.heading("build for the host (workspace, all targets)");
    let mut host = Cmd::cargo_frozen("build")
        .args(["--workspace", "--all-targets"])
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(&target_dir);
    for name in target_only_crates(&ws) {
        host = host.args(["--exclude", &name]);
    }
    run_step(&host, &sandbox, &mut report, "the host build");
    job.flush(&mut report, "test-host-build", "host build", &["R58.1", "R58.4"], vec![host.display()]);

    // ------------------------------------------------------- 5. manifest
    job.heading("Build_Manifest (R33.3, R59.2)");
    let evidence_dir = options
        .evidence_dir
        .clone()
        .unwrap_or_else(|| target_dir.join("evidence"));
    let verus = match Verus::locate() {
        Ok(v) => {
            report.note(format!("Verus {} ({}), {}", v.version, v.toolchain, v.z3_version));
            Some(v)
        }
        Err(e) => {
            if options.proofs == Proofs::Required {
                report.error(Rule::Proof, None, format!("{e}; the Kernel_Proofs are required (R44.6), or pass --proofs skip"));
            } else {
                report.warn(format!("{e}; proofs skipped"));
            }
            None
        }
    };
    let kani = match Kani::locate(root) {
        Ok(k) => {
            report.note(format!("Kani {}, CBMC {}", k.version, k.cbmc_version));
            Some(k)
        }
        Err(e) => {
            if options.proofs == Proofs::Required {
                report.error(Rule::Kani, None, format!("{e}; the Kani_Harnesses are required (R45), or pass --proofs skip"));
            } else {
                report.warn(format!("{e}; Kani skipped"));
            }
            None
        }
    };
    let manifest = build_manifest(&ws, &target_dir, verus.clone(), kani.clone(), &mut report);
    let (manifest_id, profile_version) = match &manifest {
        Some(m) => {
            let file = evidence_dir.join("build-manifest.json");
            if let Err(e) = m.write(&file) {
                report.error(Rule::Evidence, None, e);
            }
            let id = m.identity();
            report.note(format!(
                "Build_Manifest {id} written to {} (revision {}{})",
                ws.rel(&file),
                m.revision.commit,
                if m.revision.dirty { ", dirty" } else { "" }
            ));
            (id, m.profile_version.clone())
        }
        None => ("unknown".to_string(), "unversioned".to_string()),
    };
    let mut store = Store::new(&evidence_dir, Revision::of(root), &profile_version);
    job.flush(&mut report, "check-build-manifest", "Build_Manifest", &["R33.3", "R59.2"], vec![]);

    // ----------------------------------------------- 6. post-build checks
    job.heading("post-build checks (R6.1, PR-36/R55.1, R58.2/R58.5)");
    post_build_checks(&ws, &outcome, &target_dir, &mut report);
    deps::check(&ws, &outcome, &mut report);
    job.flush(
        &mut report,
        "check-post-build",
        "post-build checks",
        &["R6.1", "PR-36", "R55.1", "R58.2", "R58.5"],
        vec![],
    );

    // --------------------------------------------------- 7. probe fixture
    job.heading("R58.4 probe fixture (build script and proc-macro)");
    let mut probe_cmd = Vec::new();
    if sandbox.mechanism == Mechanism::None {
        report.warn("skipped: no network sandbox");
    } else {
        let probe_dir = target_dir.join("network-probe");
        remove_dir(&probe_dir, &mut report);
        let probe = Cmd::cargo_frozen("build")
            .arg("--manifest-path")
            .arg(root.join("xtask/fixtures/network-probe/Cargo.toml"))
            .arg("--target-dir")
            .arg(&probe_dir);
        probe_cmd.push(probe.display());
        if run_step(&probe, &sandbox, &mut report, "the network-probe fixture build") {
            report.note("the fixture's build script and proc-macro both saw their connection attempt denied");
        } else {
            report.error(
                Rule::Offline,
                Some("xtask/fixtures/network-probe".to_string()),
                "a build script or proc-macro was not denied network access",
            );
        }
    }
    job.flush(&mut report, "test-network-probe", "network probe fixture", &["R58.4"], probe_cmd);

    // ------------------------------------------------------------ 8. tests
    job.heading("tests (workspace, including the fixture tests)");
    let mut tests = Cmd::cargo_frozen("test")
        .arg("--workspace")
        .arg("--manifest-path")
        .arg(&manifest_path)
        .arg("--target-dir")
        .arg(&target_dir);
    for name in target_only_crates(&ws) {
        tests = tests.args(["--exclude", &name]);
    }
    run_step(&tests, &sandbox, &mut report, "the tests");
    job.flush(
        &mut report,
        "test-workspace-tests",
        "workspace tests",
        &["R6.2", "R6.4", "R6.3", "PR-20", "PR-36", "R58.1", "R33.3", "R25.5", "R33.2"],
        vec![tests.display()],
    );

    // -------------------------------------------------- 8a. Link_Checker
    job.heading("Link_Checker: stack bounds (R37)");
    let link_cmds = link_check(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "analysis-stack-step", "Link_Checker step", &["R37.1", "R37.2", "R37.3", "R37.4", "PR-17", "PR-18", "R3.3"], link_cmds);

    // ------------------------------------------------- 8b. Config_Checker
    job.heading("Config_Checker (R26.5)");
    let confcheck_cmds = config_check(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "check-config-step", "Config_Checker step", &["R26.4", "R26.5", "R25.2"], confcheck_cmds);

    // ------------------------------------------------------ 8c. Analyzer
    job.heading("Analyzer: response-time analysis with placeholder timing (R29)");
    let rta_cmds = analyzer_run(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "analysis-rta-step", "Analyzer step", &["R29.5", "R28.1", "R3.5"], rta_cmds);

    // --------------------------------------------------- 8d. Profile_Lint
    job.heading("Profile_Lint (R3.2, R3.4, R3.5)");
    let lint_cmds = profile_lint(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "check-profile-lint-step", "Profile_Lint step", &["R3.2", "R3.4", "R3.5"], lint_cmds);

    // ---------------------------------------------------- 9. Kernel_Proofs
    job.heading("Kernel_Proofs (Verus, R44.6) and PAR-01 cross-check");
    let mut verus_ok = true;
    let mut verus_cmds = Vec::new();
    match &verus {
        Some(v) => {
            verus_ok = proofs::run_verus(&ws, v, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
            verus_cmds.push(format!("{} verus verify -p rsk-kernel", v.cargo_verus().display()));
            if let Some(lines) = own_lines {
                proofs::line_count_cross_check(&ws, v, lines, &mut report);
            }
        }
        None => report.warn("Verus not available: Kernel_Proofs not run"),
    }
    job.flush(
        &mut report,
        "check-kernel-proofs-step",
        "Kernel_Proofs step",
        &["R44.6", "R3.10", "R6.2"],
        verus_cmds,
    );

    // --------------------------------------------------- 10. Kani_Harnesses
    job.heading("Kani_Harnesses (R45.5, R45.7)");
    let mut kani_ok = true;
    let mut kani_cmds = Vec::new();
    match &kani {
        Some(k) => {
            kani_ok = proofs::run_kani(&ws, k, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
            kani_cmds.push("cargo kani -p rsk-kernel".to_string());
        }
        None => report.warn("Kani not available: Kani_Harnesses not run"),
    }
    if verus.is_some() || kani.is_some() {
        proofs::name_failed_obligations(verus_ok, kani_ok, &mut report);
    }
    job.flush(&mut report, "check-kani-step", "Kani_Harnesses step", &["R45.5", "R45.7", "R3.10"], kani_cmds);

    // ------------------------------------------------ 10b. QEMU spike
    job.heading("dispatch spike on QEMU (task 3.1)");
    let qemu_cmds = qemu_spike(&ws, &target_dir, &mut store, &manifest_id, &mut report);
    job.flush(
        &mut report,
        "test-qemu-spike-step",
        "QEMU dispatch spike step",
        &["R7.7", "R11.1", "R16.9", "R20.1", "R20.2", "R20.5"],
        qemu_cmds,
    );

    // ------------------------------------ 10c. Profile_Conformance_Suite
    job.heading("Profile_Conformance_Suite (R3.6, R3.7, R3.11)");
    let conf_cmds = {
        let mut cx = crate::conformance::Context { root, ws: &ws, target_dir: &target_dir, sandbox: Some(&sandbox), store: &mut store, manifest_id: Some(manifest_id.clone()), only: None };
        match crate::conformance::run(&mut cx, &mut report) {
            Ok((cmds, _)) => cmds,
            Err(e) => {
                report.error(Rule::Step, Some(crate::conformance::REGISTER.into()), e);
                vec![]
            }
        }
    };
    job.flush(&mut report, "check-conformance-step", "Profile_Conformance_Suite step", &["R3.5", "R3.6", "R3.7", "R3.8", "R3.10", "R3.11"], conf_cmds);

    // ------------------------------------------------------- 11. evidence
    job.heading("Evidence_Items (R50.3)");
    write_step_evidence(&ws, &mut store, &manifest_id, &job.outcomes, manifest.as_ref(), &mut report);
    job.flush(&mut report, "check-evidence", "Evidence_Items", &["R50.3"], vec![]);
    // This step's own item exists only once its outcome is recorded, so it is
    // written last, after the items of the steps above.
    if let Some(step) = job.outcomes.last() {
        match store.write(&step_item(step, &manifest_id, manifest.as_ref())) {
            Ok(_) => report.note(format!(
                "`{}` written; {} Evidence_Items in {}",
                step.id,
                store.written.len(),
                ws.rel(&store.dir)
            )),
            Err(e) => report.error(Rule::Evidence, None, e),
        }
    }
    job.drain(&mut report);

    job.finish()
}

/// Step 10b: runs every QEMU spike binary of the Target build and writes
/// one Test Evidence_Item per binary (verdict from the semihosting exit
/// code), with the console log as its artifact. Returns the command lines.
fn qemu_spike(ws: &Workspace, target_dir: &Path, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    let spikes: Vec<&str> = ws.policy.qemu_spikes().collect();
    if spikes.is_empty() {
        report.note("no crate is marked `qemu-spike = true` in the policy");
        return commands;
    }
    let Some(version) = crate::qemu::version() else {
        report.warn("qemu-system-arm not installed: the dispatch spike was not run");
        return commands;
    };
    for binary in flight_binaries(ws, target_dir) {
        let Ok(binary) = binary else { continue };
        if !spikes.contains(&binary.package.as_str()) {
            continue;
        }
        let elf = target_dir.join(&ws.policy.target).join("release").join(&binary.target_name);
        commands.push(crate::qemu::command_line(&elf));
        let log_name = format!("qemu-{}.log", binary.target_name);
        let (ok, note) = match crate::qemu::run_elf(&elf, 120) {
            Ok(run) => {
                let _ = store.write_artifact(&log_name, &run.log);
                let ok = run.exit_code == 0 && run.log.contains("RSK-OK");
                (ok, format!("{}: exit {} after {:.1}s", binary.target_name, run.exit_code, run.seconds))
            }
            Err(e) => {
                let _ = store.write_artifact(&log_name, &e);
                (false, format!("{}: {e}", binary.target_name))
            }
        };
        if ok {
            report.note(format!("QEMU spike `{}` passed (QEMU {version}, {note})", binary.target_name));
        } else {
            report.error(Rule::Step, None, format!("QEMU spike `{}` failed: {note} (see {log_name})", binary.target_name));
        }
        let mut item = Item::new(
            &format!("test-qemu-spike-{}", binary.target_name),
            Kind::Test,
            &format!("dispatch spike on QEMU ({})", binary.target_name),
            Tool {
                name: "qemu-system-arm".to_string(),
                version: version.clone(),
                solvers: vec![],
            },
            Verdict::from_bool(ok),
        )
        .covers(&["R7.7", "R11.1", "R16.9", "R20.1", "R20.2", "R20.5"]);
        item.command_lines = vec![commands.last().cloned().unwrap_or_default()];
        item.notes = vec![note, format!("machine {}, cpu cortex-m4; not the Target (no cycle counting)", crate::qemu::MACHINE)];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    commands
}

/// R37: the Link_Checker on every Flight_Build binary with a Task_Model;
/// its `rsk-stack-bounds/1` export is an artifact of the evidence store
/// (R37.4), which the Config_Checker step then consumes (R26.2).
fn link_check(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    for binary in flight_binaries(ws, target_dir) {
        let Ok(binary) = binary else { continue };
        let elf = target_dir.join(&ws.policy.target).join("release").join(&binary.target_name);
        let Some(app_dir) = ws.packages.values().find(|p| p.name == binary.package).and_then(|p| p.dir.parent().map(Path::to_path_buf)) else { continue };
        let model = app_dir.join("gen").join("task_model.json");
        if !model.is_file() {
            continue;
        }
        let bounds_name = format!("stack-bounds-{}.json", binary.target_name);
        let bounds = store.dir.join(&bounds_name);
        let mut cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-link", "--manifest-path"])
            .arg(ws.root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--")
            .arg(&elf)
            .arg(&model)
            .arg("--out")
            .arg(&bounds)
            .arg("--profile")
            .arg(ws.root.join("profile").join("profile.toml"))
            .arg("--core-subset")
            .arg(ws.root.join("profile").join("core_subset.toml"));
        for prefix in verified_modules(ws) {
            cmd = cmd.args(["--verified", &prefix]);
        }
        commands.push(cmd.display());
        let log_name = format!("link-{}.log", binary.target_name);
        let (ok, text) = match cmd.output(Some(sandbox)) {
            Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
            Err(e) => (false, format!("cannot run rsk-link: {e}")),
        };
        let _ = store.write_artifact(&log_name, &text);
        let summary = text.lines().rev().find(|l| l.starts_with("rsk-link:")).unwrap_or("rsk-link: no verdict line").to_string();
        for line in text.lines().filter(|l| l.starts_with("note: ")) {
            report.note(format!("Link_Checker `{}`: {}", binary.target_name, &line[6..]));
        }
        if ok {
            report.note(format!("R37: `{}`: {summary}", binary.target_name));
        } else {
            report.error(Rule::Step, None, format!("Link_Checker: `{}`: {summary} (see {log_name})", binary.target_name));
        }
        let mut item = Item::new(
            &format!("analysis-stack-{}", binary.target_name),
            Kind::Analysis,
            &format!("Link_Checker: worst-case stack usage within every stack region ({})", binary.target_name),
            Tool { name: "rsk-link".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            Verdict::from_bool(ok),
        )
        .covers(&["R37.1", "R37.2", "R37.3", "R37.4", "PR-17", "PR-18", "R3.3", "R11.4", "R6.6", "R14.1", "R16.10", "R55.4", "R57.3", "PR-19", "PR-21", "PR-22", "PR-31", "PR-35"]);
        item.command_lines = vec![commands.last().cloned().unwrap_or_default()];
        item.notes = vec![summary];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name, bounds_name];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    if commands.is_empty() {
        report.note("Link_Checker: no Flight_Build binary with a Task_Model");
    }
    commands
}

/// The module paths of the Kernel crates' files that hold a `verus!` block
/// (`rsk_kernel::arch::bound`), for the Link_Checker's classification of
/// implicit panic checks as proof-covered (R14.1, R44.7).
fn verified_modules(ws: &Workspace) -> Vec<String> {
    let mut out = Vec::new();
    for (name, policy) in &ws.policy.crates {
        if !policy.kernel {
            continue;
        }
        let Some(pkg) = ws.packages.values().find(|p| &p.name == name) else { continue };
        let src = pkg.dir.join("src");
        let mut files = Vec::new();
        collect_rs(&src, &mut files);
        for f in files {
            let Ok(text) = std::fs::read_to_string(&f) else { continue };
            if !text.contains("verus! {") {
                continue;
            }
            let rel = f.strip_prefix(&src).unwrap_or(&f).with_extension("");
            let mut segs: Vec<String> = vec![name.replace('-', "_")];
            for c in rel.components() {
                let s = c.as_os_str().to_string_lossy().to_string();
                if s != "lib" && s != "mod" && s != "main" {
                    segs.push(s);
                }
            }
            out.push(segs.join("::"));
        }
    }
    out.sort();
    out
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// R29: the Analyzer on every Flight_Build binary with a Task_Model, with
/// placeholder WCET_Records and Kernel_Timing_Parameters (no WCET_Harness
/// yet): the Task_Model, the Build_Manifest of this job, and the Analyzer's
/// own Profile version are cross-checked (R28, R3.5) and the response-time
/// analysis runs; the report says the timing is not evidence. One Analysis
/// Evidence_Item per binary, `analysis-rta-<binary>`, with the JSON and text
/// reports and the UPPAAL export as artifacts.
fn analyzer_run(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    let manifest_file = store.dir.join("build-manifest.json");
    for binary in flight_binaries(ws, target_dir).into_iter().flatten() {
        let Some(app_dir) = ws.packages.values().find(|p| p.name == binary.package).and_then(|p| p.dir.parent().map(Path::to_path_buf)) else { continue };
        let model = app_dir.join("gen").join("task_model.json");
        if !model.is_file() || !manifest_file.is_file() {
            continue;
        }
        let out_dir = store.dir.join(format!("rta-{}", binary.target_name));
        let cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
            .arg(ws.root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--")
            .arg("--model")
            .arg(&model)
            .arg("--manifest")
            .arg(&manifest_file)
            .arg("--placeholders")
            .arg("--out")
            .arg(&out_dir);
        commands.push(cmd.display());
        let (status, text) = match cmd.output(Some(sandbox)) {
            Ok(o) => (o.status.code().unwrap_or(3), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
            Err(e) => (3, format!("cannot run rsk-analyzer: {e}")),
        };
        let log_name = format!("rta-{}.log", binary.target_name);
        let _ = store.write_artifact(&log_name, &text);
        let ok = status == 0;
        let verdict_line = text.lines().find(|l| l.starts_with("RESULT") || l.contains("PASS") || l.contains("FAIL") || l.contains("REJECTED")).unwrap_or("").trim().to_string();
        if ok {
            report.note(format!("R29: `{}`: the Task set is schedulable with placeholder timing (not timing evidence): {verdict_line}", binary.target_name));
        } else {
            report.error(Rule::Step, None, format!("Analyzer: `{}`: exit {status} (see {log_name})", binary.target_name));
        }
        let mut item = Item::new(
            &format!("analysis-rta-{}", binary.target_name),
            Kind::Analysis,
            &format!("Analyzer: response-time analysis of {} with placeholder WCET_Records and Kernel_Timing_Parameters (development evidence only)", binary.target_name),
            Tool { name: "rsk-analyzer".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            Verdict::from_bool(ok),
        )
        .covers(&["R28.1", "R28.2", "R28.3", "R28.4", "R28.5", "R28.6", "R28.7", "R28.8", "R29.1", "R29.2", "R29.3", "R29.4", "R29.5", "R29.6", "R29.7", "R29.8", "R29.9", "R13.3", "R3.5"]);
        item.command_lines = vec![commands.last().cloned().unwrap_or_default()];
        item.notes = vec!["placeholder timing: every WCET_Record equals the Task's Budget and every Kernel_Timing_Parameter is nominal; the result is a check of the pipeline and of the Budgets' schedulability, not timing evidence (R33.1)".to_string()];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name, format!("rta-{}/report.json", binary.target_name), format!("rta-{}/report.txt", binary.target_name), format!("rta-{}/model.xml", binary.target_name)];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    if commands.is_empty() {
        report.note("Analyzer: no Flight_Build binary with a Task_Model and a Build_Manifest");
    }
    commands
}

/// R3.5: the Profile_Lint over every crate R2.2 subjects to a restriction
/// (the Kernel crates with their Kernel_Unsafe_Module paths, the Target
/// Application crates, the Generator's output directories), then Clippy
/// with the Profile deny-lists denied (PR-10, PR-11, PR-22) on the Target
/// crates. Two Check Evidence_Items: `check-profile-lint` and
/// `check-profile-clippy`.
fn profile_lint(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    let bounds_name = "loop-bounds.json".to_string();
    let mut cmd = Cmd::cargo_frozen("run")
        .args(["--quiet", "-p", "rsk-lint", "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--")
        .arg("--root")
        .arg(&ws.root)
        .arg("--profile")
        .arg(ws.root.join("profile").join("profile.toml"))
        .arg("--core-subset")
        .arg(ws.root.join("profile").join("core_subset.toml"))
        .arg("--out")
        .arg(store.dir.join(&bounds_name));
    let mut covered = 0;
    for (name, policy) in &ws.policy.crates {
        let Some(pkg) = ws.packages.values().find(|p| &p.name == name) else { continue };
        if policy.kernel {
            cmd = cmd.arg("--kernel").arg(&pkg.dir);
            if let Some(m) = &policy.unsafe_module {
                cmd = cmd.arg("--unsafe").arg(pkg.dir.join(m));
            }
            if policy.unsafe_crate {
                cmd = cmd.arg("--unsafe").arg(&pkg.dir);
            }
            covered += 1;
        } else if policy.targets.contains(&ws.policy.target) && (policy.flight || policy.declaration) {
            cmd = cmd.arg("--app").arg(&pkg.dir);
            covered += 1;
            if policy.declaration {
                if let Some(gen) = pkg.dir.parent().map(|p| p.join("gen")) {
                    if gen.is_dir() {
                        cmd = cmd.arg("--gen").arg(gen);
                    }
                }
            }
        }
    }
    let mut model_given = false;
    for binary in flight_binaries(ws, target_dir).into_iter().flatten() {
        if let Some(app_dir) = ws.packages.values().find(|p| p.name == binary.package).and_then(|p| p.dir.parent().map(Path::to_path_buf)) {
            let model = app_dir.join("gen").join("task_model.json");
            if model.is_file() && !model_given {
                cmd = cmd.arg("--task-model").arg(model);
                model_given = true;
            }
        }
    }
    commands.push(cmd.display());
    let (ok, text) = match cmd.output(Some(sandbox)) {
        Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => (false, format!("cannot run rsk-lint: {e}")),
    };
    let _ = store.write_artifact("lint-profile.log", &text);
    let summary = text.lines().rev().find(|l| l.starts_with("rsk-lint:")).unwrap_or("rsk-lint: no verdict line").to_string();
    for line in text.lines().filter(|l| l.starts_with("note: ")) {
        report.note(format!("Profile_Lint: {}", &line[6..]));
    }
    if ok {
        report.note(format!("R3.2: {covered} crates: {summary}"));
    } else {
        report.error(Rule::Step, None, format!("Profile_Lint: {summary} (see lint-profile.log)"));
    }
    let mut item = Item::new(
        "check-profile-lint",
        Kind::Check,
        "Profile_Lint: async, loop bounds, Core_Subset, statics, recursion, dynamic dispatch, and lint-level lowering over the Kernel and Application sources",
        Tool { name: "rsk-lint".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
        Verdict::from_bool(ok),
    )
    .covers(&["R3.2", "R3.4", "R3.5", "R35.1", "R35.2", "R35.3", "R35.5", "R57.2", "PR-05", "PR-09", "PR-16", "PR-17", "PR-18", "PR-21"]);
    item.command_lines = vec![commands[0].clone()];
    item.notes = vec![summary];
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["lint-profile.log".to_string(), bounds_name];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }

    // Clippy with the Profile deny-lists (clippy.toml) denied; every other
    // Clippy lint is allowed here, since this step checks the Profile, not
    // general hygiene (R3.4 concerns the Profile_Lint lints only).
    let mut clippy = Cmd::cargo_frozen("clippy")
        .args(["--release", "--target", &ws.policy.target])
        .arg("--manifest-path")
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir)
        .env("CARGO_ENCODED_RUSTFLAGS", target_rustflags(&ws.root));
    for name in target_crates(ws) {
        clippy = clippy.args(["-p", &name]);
    }
    clippy = clippy.args(["--", "-A", "clippy::all", "-D", "clippy::disallowed_methods", "-D", "clippy::disallowed_types", "-D", "clippy::disallowed_macros"]);
    commands.push(clippy.display());
    let (ok, text) = match clippy.output(Some(sandbox)) {
        Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => (false, format!("cannot run cargo clippy: {e}")),
    };
    let _ = store.write_artifact("lint-clippy.log", &text);
    let errors = text.lines().filter(|l| l.starts_with("error")).count();
    if ok {
        report.note(format!("PR-10, PR-11, PR-22: Clippy deny-lists: no violation in {} Target crates", target_crates(ws).len()));
    } else {
        report.error(Rule::Step, None, format!("Clippy deny-lists: {errors} error line(s) (see lint-clippy.log)"));
    }
    let mut item = Item::new(
        "check-profile-clippy",
        Kind::Check,
        "Profile_Lint: Clippy disallowed-methods, disallowed-types, and disallowed-macros deny-lists over the Target crates",
        Tool { name: "cargo clippy".to_string(), version: clippy_version(), solvers: vec![] },
        Verdict::from_bool(ok),
    )
    .covers(&["R3.2", "PR-10", "PR-11", "PR-22"]);
    item.command_lines = vec![commands[1].clone()];
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["lint-clippy.log".to_string()];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    commands
}

/// R26.5: the Config_Checker on every Flight_Build binary that has a
/// Task_Model (the declaration crate's `gen/task_model.json` next to the
/// binary crate), one Check Evidence_Item per binary.
fn config_check(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    for binary in flight_binaries(ws, target_dir) {
        let Ok(binary) = binary else { continue };
        let elf = target_dir.join(&ws.policy.target).join("release").join(&binary.target_name);
        // The Task_Model of the application: `<app dir>/gen/task_model.json`,
        // where the binary crate lives in `<app dir>/bin`.
        // `packages` is keyed by package id; match on the name.
        let Some(app_dir) = ws.packages.values().find(|p| p.name == binary.package).and_then(|p| p.dir.parent().map(Path::to_path_buf)) else {
            report.note(format!("Config_Checker: package `{}` not found in the workspace", binary.package));
            continue;
        };
        let model = app_dir.join("gen").join("task_model.json");
        if !model.is_file() {
            report.note(format!("Config_Checker: no Task_Model at {} for `{}`", ws.rel(&model), binary.target_name));
            continue;
        }
        let bounds = store.dir.join(format!("stack-bounds-{}.json", binary.target_name));
        let mut cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-confcheck", "--manifest-path"])
            .arg(ws.root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--")
            .arg(&model)
            .arg(&elf);
        if bounds.is_file() {
            // R26.2: the stack region sizes against the Link_Checker's bounds.
            cmd = cmd.arg(&bounds);
        }
        commands.push(cmd.display());
        let log_name = format!("confcheck-{}.log", binary.target_name);
        let (ok, text) = match cmd.output(Some(sandbox)) {
            Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
            Err(e) => (false, format!("cannot run rsk-confcheck: {e}")),
        };
        let _ = store.write_artifact(&log_name, &text);
        let summary = text.lines().rev().find(|l| l.starts_with("rsk-confcheck:")).unwrap_or("rsk-confcheck: no verdict line").to_string();
        if ok {
            report.note(format!("R26.5: `{}` against {}: {summary}", binary.target_name, ws.rel(&model)));
        } else {
            report.error(Rule::Step, None, format!("Config_Checker: `{}` against {}: {summary} (see {log_name})", binary.target_name, ws.rel(&model)));
        }
        // R26.7: the mutation suite, on the same model and binary.
        let work = target_dir.join("confcheck-mutations");
        let suite_ok = match crate::confcheck_mutations::run(&ws.root, &model, &elf, &work, Some(sandbox), report) {
            Ok(ok) => ok,
            Err(e) => {
                report.error(Rule::Step, Some(crate::confcheck_mutations::REGISTER.to_string()), e);
                false
            }
        };
        let mut suite = Item::new(
            &format!("test-config-mutations-{}", binary.target_name),
            Kind::Test,
            &format!("Config_Checker mutation suite: every altered Task_Model or binary is reported ({})", binary.target_name),
            Tool { name: "rsk-confcheck".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            Verdict::from_bool(suite_ok),
        )
        .covers(&["R26.7", "R26.3", "R26.6"]);
        suite.build_manifest = Some(manifest_id.to_string());
        suite.notes = vec![format!("register {}", crate::confcheck_mutations::REGISTER)];
        if let Err(e) = store.write(&suite) {
            report.error(Rule::Evidence, None, e);
        }
        let mut item = Item::new(
            &format!("check-config-{}", binary.target_name),
            Kind::Check,
            &format!("Config_Checker: the Task_Model and the linked binary agree ({})", binary.target_name),
            Tool { name: "rsk-confcheck".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            Verdict::from_bool(ok),
        )
        .covers(&["R26.1", "R26.2", "R26.3", "R26.4", "R26.5", "R25.2", "R15.6", "R16.7"]);
        item.command_lines = vec![commands.last().cloned().unwrap_or_default()];
        item.notes = vec![summary];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    if commands.is_empty() {
        report.note("Config_Checker: no Flight_Build binary with a Task_Model");
    }
    commands
}

/// The Build_System's executable line count of the Kernel (for the PAR-01
/// cross-check).
fn kernel_line_total(ws: &Workspace) -> usize {
    let cfg = checks::kernel_size::target_cfg(ws);
    checks::kernel_size::kernel_sources(ws)
        .into_iter()
        .filter_map(|(_, f)| checks::kernel_size::count_file(&f, &cfg).ok().flatten())
        .map(|c| c.exec)
        .sum()
}

fn build_manifest(
    ws: &Workspace,
    target_dir: &Path,
    verus: Option<Verus>,
    kani: Option<Kani>,
    report: &mut Report,
) -> Option<Manifest> {
    let mut manifest = match Manifest::collect(ws, verus, kani) {
        Ok(m) => m,
        Err(e) => {
            report.error(Rule::Evidence, None, e);
            return None;
        }
    };
    for binary in flight_binaries(ws, target_dir) {
        match binary {
            Ok(b) => {
                report.note(format!(
                    "Flight_Build binary `{}` ({}): {} over {} loadable bytes",
                    b.target_name, b.package, b.hash, b.loadable_bytes
                ));
                manifest.binaries.push(b);
            }
            Err(e) => report.error(Rule::Evidence, None, e),
        }
    }
    if manifest.binaries.is_empty() {
        report.note("no Flight_Build binary crate yet: the manifest records no binary hash");
    }
    // R33.3, R28.8: the Generated_Config checksum of the analysed binary.
    manifest.generated_config_checksum = manifest.binaries.iter().find_map(|b| b.config_checksum.clone());
    if let Some(c) = &manifest.generated_config_checksum {
        report.note(format!("Generated_Config checksum stored in the binary: {c}"));
    }
    Some(manifest)
}

/// The Evidence_Item of one job step.
fn step_item(step: &StepOutcome, manifest_id: &str, manifest: Option<&Manifest>) -> Item {
    let xtask_version = env!("CARGO_PKG_VERSION").to_string();
    let rustc = manifest.map(|m| m.rustc.clone()).unwrap_or_default();
    let mut item = Item::new(
        step.id,
        if step.id.starts_with("test-") { Kind::Test } else { Kind::Check },
        &step.name,
        Tool {
            name: "cargo xtask verify".to_string(),
            version: format!("{xtask_version} ({rustc})"),
            solvers: Vec::new(),
        },
        step.verdict,
    )
    .covers(&step.covers);
    item.command_lines = step.commands.clone();
    item.build_manifest = Some(manifest_id.to_string());
    item.notes = step.notes.clone();
    item
}

/// Writes one Evidence_Item per job step.
fn write_step_evidence(
    ws: &Workspace,
    store: &mut Store,
    manifest_id: &str,
    outcomes: &[StepOutcome],
    manifest: Option<&Manifest>,
    report: &mut Report,
) {
    for step in outcomes {
        if let Err(e) = store.write(&step_item(step, manifest_id, manifest)) {
            report.error(Rule::Evidence, None, e);
        }
    }
    report.note(format!("{} Evidence_Items written to {}", store.written.len(), ws.rel(&store.dir)));
    let covered = evidence::covered_criteria(&store.dir).len();
    report.note(format!("{covered} criterion links recorded by the items of this run"));
}

/// The first line of `cargo clippy --version` (the pinned toolchain's Clippy).
fn clippy_version() -> String {
    Cmd::cargo()
        .args(["clippy", "--version"])
        .output(None)
        .ok()
        .map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim().to_string())
        .unwrap_or_default()
}
