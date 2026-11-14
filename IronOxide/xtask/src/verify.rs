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
//! 10b'. App_Prover (R46): `cargo creusot` over proofs/creusot, the
//!     Partition sources against the rsk API's trusted contracts; skipped
//!     with a warning when Creusot is not installed.
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
use std::process::Command;

use crate::checks::{self, deps, features, kernel_std};
use crate::cmd::{overridden_incoming_env, Cmd};
use crate::diag::{Diagnostic, Report, Rule};
use crate::evidence::{self, Item, Kind, Store, Tool, Verdict};
use crate::manifest::{flight_binaries, Manifest};
use crate::proofs;
use crate::sandbox::{Mechanism, Mode, Sandbox};
use crate::tools::{Creusot, Kani, Revision, Verus};
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
        .filter(|(_, c)| (c.flight || c.targets.contains(&ws.policy.target)) && !c.hil_stimulus)
        .map(|(name, _)| name.clone())
        // A proc-macro (rsk-macros) is compiled for the host as a dependency
        // of `rsk` in every Target build. Named as a root of a `--target`
        // build, Cargo unifies its host dependencies' features into the
        // Target graph (vstd's `std`, enabled by rsk-protocol since task
        // 21.6, would then be built for the Target), so it is not named.
        .filter(|name| {
            ws.package_named(name)
                .is_none_or(|p| !p.targets.iter().any(|t| t.kinds.iter().any(|k| k == "proc-macro")))
        })
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
        total_steps: 24,
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
    // R46.2: the App_Prover's versions in the Build_Manifest; an installation
    // other than the pinned one is an error, its absence a warning (the
    // proofs are then skipped with one in step 10b').
    let creusot = match Creusot::locate(root, &root.join("proofs").join("creusot")) {
        Ok(c) => {
            report.note(format!("Creusot: {}", c.summary()));
            Some(c)
        }
        Err(e) if e.contains("(R46.2)") => {
            report.error(Rule::Proof, None, e);
            None
        }
        Err(e) => {
            report.warn(format!("{e}; the App_Prover proofs will be skipped"));
            None
        }
    };
    let mut hil_builds = Vec::new();
    let manifest = build_manifest(&ws, &target_dir, &sandbox, verus.clone(), kani.clone(), creusot.clone(), &mut hil_builds, &mut report);
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
    // R50.5: the criteria the passing tests name in their names and doc
    // comments join the step's own (the matrix of `cargo xtask trace`).
    let (named, functions) = crate::trace::named_by_tests(root, "#[test]");
    report.note(format!("covers: {} criteria named by {functions} test functions (names and doc comments), with the step's own", named.len()));
    let mut covers: Vec<&'static str> = vec!["R6.2", "R6.4", "R6.3", "PR-20", "PR-36", "R58.1", "R33.3", "R25.5", "R33.2"];
    for c in named {
        if !covers.contains(&c.as_str()) {
            covers.push(Box::leak(c.into_boxed_str()));
        }
    }
    job.flush(&mut report, "test-workspace-tests", "workspace tests", &covers, vec![tests.display()]);

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

    // ------------------------------------------- 8c'. UPPAAL cross-check corpus
    job.heading("UPPAAL cross-check corpus (R32.6, PAR-03)");
    let corpus_cmds = uppaal_corpus(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "check-uppaal-corpus-step", "UPPAAL corpus step", &["R32.6"], corpus_cmds);

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

    // ------------------------------------------- 10b-rig. HIL_Rig self-test
    job.heading("HIL: rig self-test (task 7.1, hil-rig)");
    let rig = match crate::hil::rig(root) {
        Some(Ok(rig)) => Some(rig),
        Some(Err(e)) => {
            report.error(Rule::Step, Some(rsk_hil::config::FILE.into()), e);
            None
        }
        None => {
            report.note(format!("no {} at the workspace root: no HIL_Rig is described; the HIL steps use a single attached probe, if any", rsk_hil::config::FILE));
            None
        }
    };
    let rig_cmds = hil_rig(&ws, rig.as_ref(), &hil_builds, &target_dir, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-rig-step", "HIL_Rig self-test step", &["R34.8", "R48.4"], rig_cmds);

    // ------------------------------------------- 10b-hil. HIL spike on the board
    job.heading("HIL: dispatch spike on the nRF52840-DK (task 3.1, hil-spike)");
    let hil_cmds = hil_spike(&ws, rig.as_ref(), &hil_builds, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-spike-step", "HIL spike step", &["R7.7", "R11.1", "R16.4"], hil_cmds);

    // ------------------------------------------- 10b-hil'. HIL response suite
    job.heading("HIL: response suite, one injected fault per response of R14.5 (task 7.5, hil-response)");
    let resp_cmds = hil_response(&ws, rig.as_ref(), &hil_builds, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-response-step", "HIL response suite step", &["R14.11", "R14.5", "R14.9"], resp_cmds);

    // ------------------------------------------- 10b-hil''. HIL isolation suite and the differential tests
    job.heading("HIL: isolation suite, the MPU access matrix and the EasyDMA cases, with the Hardware_Model differential tests (tasks 7.4 and 3.3, hil-mpu-matrix, hil-dma, hil-dt-hw-NN)");
    let iso_cmds = hil_isolation(&ws, rig.as_ref(), &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-isolation-step", "HIL isolation suite step", &["R16.8", "R17.7", "R49.3"], iso_cmds);

    // ------------------------------------------- 10b-hil'''. HIL boot-check suite
    job.heading("HIL: boot-check suite, mismatched values and corrupted tables (task 7.2, hil-boot-checks)");
    let bc_cmds = hil_boot_checks(&ws, rig.as_ref(), &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-boot-checks-step", "HIL boot-check suite step", &["R12.6"], bc_cmds);

    // ------------------------------------------- 10b-hil''''. HIL cycle attribution
    job.heading("HIL: cycle attribution, the Budget accounting against the DWT (task 7.5, hil-cycles)");
    let cy_cmds = hil_cycles(&ws, rig.as_ref(), &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "hil-cycles-step", "HIL cycle-attribution step", &["R18.1"], cy_cmds);

    // ------------------------------------ 10b-cov. Structural coverage (R53)
    job.heading("Structural coverage: the Kernel's host tests instrumented, the Partitions' on-target reports (task 19.5, R53.3)");
    let cov_cmds = structural_coverage(&ws, &target_dir, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "coverage-step", "Structural coverage step", &["R53.3"], cov_cmds);

    // ------------------------------------------------ 10b'. App_Prover
    job.heading("App_Prover (Creusot, R46.3, R46.4)");
    let creusot_cmds = app_prover(&ws, &sandbox, &mut store, &manifest_id, creusot.as_ref(), &mut report);
    job.flush(&mut report, "proof-app-step", "App_Prover step", &["R46.2", "R46.3", "R46.4", "R35.2"], creusot_cmds);

    // -------------------------------------- 10b''. Aeneas evaluation (task 17.3)
    job.heading("Aeneas evaluation: the control law in Lean (R46.1, task 17.3)");
    let aeneas_cmds = aeneas_evaluation(&ws, &sandbox, &mut store, &manifest_id, &mut report);
    job.flush(&mut report, "proof-app-aeneas-step", "Aeneas step", &["R46.1"], aeneas_cmds);

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
    // R50.6: the criteria without an Evidence_Item in this store, and the
    // matrix of R50.5 as an artifact (the committed docs/traceability_matrix.md
    // is written by `cargo xtask trace` from the same sources).
    match crate::trace::build(root, &store.dir) {
        Ok(m) => {
            let text = crate::trace::render(&m, &ws.rel(&store.dir));
            let _ = store.write_artifact("traceability_matrix.md", &text);
            let missing = m.without_evidence();
            let early: Vec<&str> = missing.iter().filter(|c| c.phase.starts_with("P1") || c.phase.starts_with("P2")).map(|c| c.id.as_str()).collect();
            report.note(format!(
                "R50.6: {} of {} acceptance criteria have no Evidence_Item in this store ({} of Phase P1 or P2; traceability_matrix.md lists them with their tasks)",
                missing.len(),
                m.criteria.len(),
                early.len()
            ));
        }
        Err(e) => report.warn(format!("traceability matrix not built: {e}")),
    }
    job.flush(&mut report, "check-evidence", "Evidence_Items", &["R50.3", "R50.5", "R50.6"], vec!["cargo xtask trace".to_string()]);
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
            .arg(ws.root.join("profile").join("core_subset.toml"))
            // DD-11: the system crate of the Generator's binding (gen.rs).
            .args(["--system-crate", "rsk-system"]);
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

/// R46: the App_Prover (Creusot) over the proof workspace `proofs/creusot`,
/// which compiles the Partition crates' real sources with the `rsk` API's
/// trusted contracts (task 17.1). The prover is located like QEMU: when
/// `cargo creusot` is not installed the step warns and writes no item; when
/// it runs, `proof-app-creusot` records its version line and the pin of
/// toolchain/decision.toml, and passes only on `Proved (...)`.
/// Removes why3find's session state under the proof workspace and the
/// proof crates' build directory (all ignored by git), so that every run
/// proves from nothing (the hang itself was the sandbox denying why3's
/// local solver socket, `sandbox.rs`), and the translated files are emitted by
/// the compilation, so a fresh cargo cache beside a cleared `verif` gives
/// `cargo creusot` nothing to prove ("No files to prove", 2026-11-02, the
/// first job run after a hand run of the prover). A fresh build and proof
/// take about a minute.
pub(crate) fn creusot_session_clear(dir: &Path) {
    for name in ["verif", ".why3find", "target"] {
        let _ = std::fs::remove_dir_all(dir.join(name));
    }
}

/// Clears why3find's session (`.why3find`, and the `proof.json`,
/// `why3session.xml`, and `why3shapes.gz` under `verif`); with
/// `keep_translation` the `.coma` files stay, so that a second prover
/// set can be run on the same translation (`--only prove`), otherwise
/// `verif` goes too.
fn creusot_session_clear_keeping_translation(dir: &Path, keep_translation: bool) {
    let _ = std::fs::remove_dir_all(dir.join(".why3find"));
    if !keep_translation {
        let _ = std::fs::remove_dir_all(dir.join("verif"));
        return;
    }
    fn walk(d: &Path) {
        let Ok(entries) = std::fs::read_dir(d) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p);
            } else if matches!(p.file_name().and_then(|n| n.to_str()), Some("proof.json" | "why3session.xml" | "why3shapes.gz")) {
                let _ = std::fs::remove_file(&p);
            }
        }
    }
    walk(&dir.join("verif"));
}

/// The solvers that discharged the goals of the session under `verif`,
/// counted from why3find's `proof.json` files (`"prover": "<name>"` per
/// goal), for the Evidence_Item's note.
fn creusot_provers_used(dir: &Path) -> std::collections::BTreeMap<String, usize> {
    let mut out = std::collections::BTreeMap::new();
    fn walk(d: &Path, out: &mut std::collections::BTreeMap<String, usize>) {
        let Ok(entries) = std::fs::read_dir(d) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.file_name().and_then(|n| n.to_str()) == Some("proof.json") {
                let Ok(text) = std::fs::read_to_string(&p) else { continue };
                let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
                fn goals(v: &serde_json::Value, out: &mut std::collections::BTreeMap<String, usize>) {
                    match v {
                        serde_json::Value::Object(m) => {
                            if let Some(serde_json::Value::String(p)) = m.get("prover") {
                                *out.entry(p.clone()).or_default() += 1;
                            } else {
                                for x in m.values() {
                                    goals(x, out);
                                }
                            }
                        }
                        serde_json::Value::Array(a) => a.iter().for_each(|x| goals(x, out)),
                        _ => {}
                    }
                }
                if let Some(proofs) = v.get("proofs") {
                    goals(proofs, out);
                }
            }
        }
    }
    walk(&dir.join("verif"), &mut out);
    out
}

/// The HIL-configuration members of the proof workspace: every `members`
/// entry of `proofs/creusot/Cargo.toml` outside
/// `[workspace.metadata.creusot] default-members`, as (directory, package
/// name) pairs, in manifest order.
fn creusot_hil_members(dir: &Path) -> Vec<(String, String)> {
    let Ok(text) = std::fs::read_to_string(dir.join("Cargo.toml")) else { return vec![] };
    let Ok(table) = text.parse::<toml::Table>() else { return vec![] };
    let ws = table.get("workspace").and_then(|w| w.as_table());
    let members: Vec<String> = ws
        .and_then(|w| w.get("members"))
        .and_then(|m| m.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let defaults: Vec<String> = ws
        .and_then(|w| w.get("metadata"))
        .and_then(|m| m.get("creusot"))
        .and_then(|c| c.get("default-members"))
        .and_then(|d| d.as_array())
        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
        .unwrap_or_default();
    let mut out = Vec::new();
    for m in members {
        let Ok(manifest) = std::fs::read_to_string(dir.join(&m).join("Cargo.toml")) else { continue };
        let Ok(t) = manifest.parse::<toml::Table>() else { continue };
        let Some(name) = t.get("package").and_then(|p| p.get("name")).and_then(|n| n.as_str()) else { continue };
        if defaults.iter().any(|d| d == name) {
            continue;
        }
        out.push((m, name.to_string()));
    }
    out
}

#[cfg(test)]
mod creusot_members_tests {
    /// R46.3: the proof workspace's HIL-configuration members are the twenty-three (declaration variant, Partition features) pairs the rig builds (the measurement configurations' `measure` feature since task 19.2, the plain measurement and floating-point cycle configurations since tasks 19.4 and 19.3, Partition 1's eight since 2026-11-09), each outside the default members, named by directory and package.
    #[test]
    fn hil_members_are_the_twenty_three_configurations_outside_the_defaults() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let members = super::creusot_hil_members(&root.join("proofs").join("creusot"));
        let dirs: Vec<&str> = members.iter().map(|(d, _)| d.as_str()).collect();
        assert_eq!(dirs, ["part0-nrf52840", "part0-timing", "part0-stress", "part0-measure", "part0-response", "part0-isolation", "part0-cycles", "part0-cycles-fpu", "part1-nrf52840", "part1-timing", "part1-stress", "part1-measure", "part1-response", "part1-isolation", "part1-cycles", "part1-cycles-fpu", "part2-nrf52840", "part2-timing", "part2-response", "part2-stress", "part2-measure", "part2-isolation", "part2-cycles-fpu"]);
        assert!(members.iter().all(|(d, p)| *p == format!("p1-demo-{d}-proof")));
    }
}

fn app_prover(ws: &Workspace, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, creusot: Option<&Creusot>, report: &mut Report) -> Vec<String> {
    let dir = ws.root.join("proofs").join("creusot");
    if !dir.join("Cargo.toml").is_file() {
        report.note("App_Prover: no proof workspace at proofs/creusot");
        return vec![];
    }
    let Some(creusot) = creusot else {
        report.warn("cargo-creusot is not installed or not the pinned one: the App_Prover proofs were not run (toolchain/decision.toml [creusot] records the pinned installation)");
        return vec![];
    };
    let version = creusot.summary();
    let solvers = creusot.solvers.clone();
    // why3find keeps a session under proofs/creusot (`verif/`, `.why3find`)
    // and has deadlocked on it at 0 % CPU more than once (2026-11-02, after
    // a killed run and after a completed one): the session is cleared
    // before and after every run, any why3find left from an earlier run is
    // ended first, and the run is bounded (a fresh session proves the
    // demo's 14 files in seconds; the bound is generous).
    let _ = Command::new("pkill").args(["-f", "why3find"]).status();
    creusot_session_clear(&dir);
    // The runs: the QEMU pair (the workspace's default members, one
    // `cargo creusot`), then each HIL-configuration member by name
    // (`-p <package>`), since the declaration's variant features are
    // exclusive and one build of them all would unify them (task 17.1,
    // Verification_Plan section 7). The members come from the proof
    // workspace's manifest: every member outside `default-members`.
    // The runs: the QEMU pair (the workspace's default members, one
    // `cargo creusot`), then each HIL-configuration member by name
    // (`-p <package>`), since the declaration's variant features are
    // exclusive and one build of them all would unify them (task 17.1,
    // Verification_Plan section 7). The members come from the proof
    // workspace's manifest: every member outside `default-members`.
    //
    // Cross-solver confirmation (9.1, since 2026-11-06): every goal is
    // discharged twice, by a solver of each of two disjoint sets. The
    // first run compiles and proves with alt-ergo and z3 only
    // (`-P -cvc5,-cvc4` removes the other two from why3find.json's list);
    // the session is cleared but the translation kept, and the second run
    // (`--only prove --no-cache`) proves the same files with cvc5 and cvc4
    // only. The item passes when both runs of every configuration prove.
    const SET_A: [&str; 2] = ["--why3find-arg=-P", "--why3find-arg=-cvc5,-cvc4"];
    const SET_B: [&str; 2] = ["--why3find-arg=-P", "--why3find-arg=-alt-ergo,-z3"];
    let mut runs: Vec<(String, Vec<String>)> = vec![("the QEMU configuration".to_string(), vec![])];
    for (member, package) in creusot_hil_members(&dir) {
        runs.push((format!("`{member}`"), vec!["-p".to_string(), package]));
    }
    let run_once = |cmd: &Cmd| match cmd.output_timeout(Some(sandbox), 600, &["why3find", "cargo-creusot"]) {
        Ok(o) => {
            let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
            (o.status.success() && text.contains("Proved ("), text, false)
        }
        Err(e) => (false, format!("cannot run cargo creusot: {e}"), e.to_string().contains("did not complete")),
    };
    let verdict_of = |text: &str| {
        text.lines().rev().find(|l| l.contains("Proved (") || l.contains("proof failed") || l.starts_with("Error")).unwrap_or("no verdict line").trim().to_string()
    };
    let mut commands = Vec::new();
    let mut texts = Vec::new();
    let mut verdicts = Vec::new();
    let mut provers: std::collections::BTreeMap<String, usize> = std::collections::BTreeMap::new();
    let mut ok = true;
    for (label, select) in &runs {
        let mut cmd_a = Cmd::cargo().arg("creusot").args(select).args(SET_A).args(["--", "--offline"]).cwd(&dir);
        let mut cmd_b = Cmd::cargo().arg("creusot").args(select).args(["--only", "prove", "--no-cache"]).args(SET_B).args(["--", "--offline"]).cwd(&dir);
        let mut pair = Vec::new();
        for (set, cmd, keep_translation) in [("alt-ergo and z3", &mut cmd_a, false), ("cvc5 and cvc4", &mut cmd_b, true)] {
            // The session is cleared between runs (the translation too
            // before the first of a configuration); the build directory was
            // cleared once above, so every member's translation is fresh.
            let _ = Command::new("pkill").args(["-f", "why3find"]).status();
            creusot_session_clear_keeping_translation(&dir, keep_translation);
            let command = format!("(cd proofs/creusot && {})", cmd.display());
            let (mut run_ok, mut text, timed_out) = run_once(cmd);
            if timed_out {
                // The why3find hang (0 % CPU, no solver running) reached the bound
                // although the session was fresh (2026-11-02, with the HIL isolation
                // suite's job): ended, cleared, and run once more; a fresh run proves
                // the files in about a minute, so a second bound reached is the verdict.
                report.warn(format!("App_Prover: `cargo creusot` for {label} ({set}) reached its bound once (why3find hung); the session was cleared and the run repeated"));
                let _ = Command::new("pkill").args(["-f", "why3find"]).status();
                creusot_session_clear_keeping_translation(&dir, keep_translation);
                let (ok2, text2, _) = run_once(cmd);
                run_ok = ok2;
                text = format!("{text}\n--- second run after the bound ---\n{text2}");
            }
            for (prover, n) in creusot_provers_used(&dir) {
                *provers.entry(prover).or_default() += n;
            }
            pair.push(format!("{set}: {}", verdict_of(&text)));
            ok = ok && run_ok;
            texts.push(format!("=== {command}\n{text}"));
            commands.push(command);
        }
        verdicts.push(format!("{label}: {}", pair.join(", ")));
    }
    // Task 21.6 (R41.3, A-P8; Verification_Plan 5.15): the proofs that must
    // FAIL, in proofs/creusot-negative: each member a fixture body that
    // violates a generated obligation. The step passes only when Creusot
    // refuses every member; a member that proves is an error (the
    // obligation would be vacuous).
    let negative = ws.root.join("proofs").join("creusot-negative");
    if negative.join("Cargo.toml").is_file() {
        let _ = Command::new("pkill").args(["-f", "why3find"]).status();
        creusot_session_clear(&negative);
        let mut cmd = Cmd::cargo().arg("creusot").args(SET_A).args(["--", "--offline"]).cwd(&negative);
        let command = format!("(cd proofs/creusot-negative && {})", cmd.display());
        let (run_ok, text, _) = run_once(&mut cmd);
        let refused = !run_ok && !text.contains("Proved (");
        texts.push(format!("=== {command}\n{text}"));
        commands.push(command);
        creusot_session_clear(&negative);
        if refused {
            verdicts.push(format!("proofs that must fail (proofs/creusot-negative): refused as required ({})", verdict_of(&text)));
        } else {
            ok = false;
            verdicts.push("proofs that must fail (proofs/creusot-negative): PROVED, the obligation is vacuous".to_string());
        }
    }
    let prover_note = provers.iter().map(|(p, n)| format!("{p} {n}")).collect::<Vec<_>>().join(", ");
    verdicts.push(format!("goals discharged per solver: {prover_note}"));
    let text = texts.join("\n");
    creusot_session_clear(&dir);
    let _ = store.write_artifact("creusot.log", &text);
    let verdict = verdicts.join("; ");
    if ok {
        report.note(format!("R46.3: App_Prover: {verdict} ({version})"));
    } else {
        report.error(Rule::Step, None, format!("App_Prover: {verdict} (see creusot.log)"));
    }
    let mut item = Item::new(
        "proof-app-creusot",
        Kind::Proof,
        "App_Prover (Creusot): the Partition crates' Task bodies against the trusted contracts of the rsk API",
        Tool { name: "Creusot".to_string(), version: version.clone(), solvers },
        Verdict::from_bool(ok),
    )
    .covers(&["R46.2", "R46.3", "R46.4", "R35.2", "PR-19"]);
    item.command_lines = commands.clone();
    item.notes = vec![verdict, "every goal discharged by two solvers of disjoint sets (alt-ergo or z3, then cvc5 or cvc4; cross-solver confirmation, verification_plan.md 9.1); trusted: the contracts of rsk::Shared::lock, PerTask, Signal::raise, now, spin, dma_request, debug, mmio, and the hw_dt probes (TBR-08); the Generator's glue is excluded under cfg(creusot); the f32 step `lowpass` of part2 and the pointer-to-integer helper `address_of` of its isolation configuration are trusted (verification_plan.md section 7, R46.5)".to_string()];
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["creusot.log".to_string()];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    commands
}

/// Task 3.1's hardware half (Verification_Plan section 11, `hil-spike`): the
/// demo's nRF52840 variant flashed and run on the attached DK through
/// probe-rs (`hil.rs`). Skipped with a warning when no probe is attached.
/// Evidence_Item `hil-spike`: pass on `RSK-OK` and exit 0, with the console
/// as artifact; a HIL build carries the `debug-port` feature, so it is not
/// the Flight_Build binary the other items describe (its own Build_Manifest
/// entry is a follow-up).
fn hil_spike(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, builds: &[crate::hil::HilBuild], store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let spikes: Vec<&str> = ws.policy.qemu_spikes().collect();
    let Some(probe) = crate::hil::probe(rig) else {
        report.warn("no debug probe for the target (probe-rs list; the rig's target serial when hil-rig.toml exists, otherwise exactly one probe): the HIL spike was not run; the board's results stay hardware-pending (section 11)");
        return vec![];
    };
    let Some(serial) = crate::hil::target_serial(rig) else { return vec![] };
    let mut commands = Vec::new();
    for bin in spikes {
        let Some(build) = builds.iter().find(|b| b.bin == bin) else {
            report.error(Rule::Step, None, format!("HIL spike `{bin}`: no HIL build (see the Build_Manifest step)"));
            continue;
        };
        let run = crate::hil::run_build(build, &serial, 90);
        commands.push(run.command.clone());
        let log_name = format!("hil-{bin}.log");
        let _ = store.write_artifact(&log_name, &run.log);
        // A locked access port (nRF52840 APPROTECT after a reset without the
        // UICR setting that keeps it open) is the bench's state, not the
        // Kernel's: the run is inconclusive, and the operator unlocks the
        // chip (docs: Verification_Plan section 11).
        let locked = run.log.contains("Core is locked");
        let verdict = if run.ok {
            report.note(format!("HIL spike `{bin}` passed on {} ({}; {:.1}s)", crate::hil::CHIP, probe.probe, run.seconds));
            Verdict::Pass
        } else if locked {
            report.warn(format!("HIL spike `{bin}`: the board's access port is locked (APPROTECT); unlock it once with `probe-rs run --allow-erase-all` (see {log_name})"));
            Verdict::Inconclusive
        } else {
            report.error(Rule::Step, None, format!("HIL spike `{bin}` failed on {} (see {log_name})", crate::hil::CHIP));
            Verdict::Fail
        };
        let mut item = Item::new(
            &format!("hil-spike-{bin}"),
            Kind::Test,
            &format!("dispatch spike on the nRF52840-DK ({bin}, declaration variant {}, features {})", crate::hil::VARIANT, crate::hil::FEATURES),
            Tool { name: "probe-rs".to_string(), version: format!("{} ({})", probe.version, probe.probe), solvers: vec![] },
            verdict,
        )
        .covers(&["R7.7", "R11.1", "R16.4", "R12.1", "R12.6"]);
        item.command_lines = vec![run.command.clone()];
        item.notes = vec![
            "the HIL build enables `debug-port` (the access port stays open across resets) and is not the Flight_Build binary; the Kernel's semihosting console and exit are carried by probe-rs".to_string(),
            format!("HIL binary hash {} (Build_Manifest `hil_binaries`)", build.binary.hash),
        ];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    commands
}

/// The response suite (task 7.5, R14.11): the Application in its response
/// configuration (a HIL build with the `response` feature, 2 seconds) on
/// the rig, judged by `rsk_hil::response`. Skipped with a warning without
/// a rig or without its probes, like the self-test.
#[allow(clippy::too_many_arguments)]
fn hil_response(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, builds: &[crate::hil::HilBuild], target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let Some(rig) = rig else { return vec![] };
    let probes = match rsk_hil::probes::list() {
        Ok(p) => p,
        Err(e) => {
            report.warn(format!("HIL response suite: {e}; not run"));
            return vec![];
        }
    };
    if !probes.iter().any(|p| p.id == rig.target.serial) || !probes.iter().any(|p| p.id == rig.stimulus.serial) {
        report.warn("HIL response suite: the target or the stimulus board is not attached; not run".to_string());
        return vec![];
    }
    let Some(bin) = ws.policy.qemu_spikes().next() else {
        report.error(Rule::Step, None, "HIL response suite: no QEMU spike Application to build the response configuration of");
        return vec![];
    };
    let Some(stimulus) = rig.stimulus.bin.as_deref().and_then(|b| builds.iter().find(|x| x.bin == b)) else {
        report.error(Rule::Step, None, "HIL response suite: no HIL build of the stimulus binary");
        return vec![];
    };
    let features = format!("{},response", crate::hil::FEATURES);
    let dir = target_dir.join("hil").join("response");
    let Some(target) = crate::hil::build_with(ws, &bin, &features, &dir, Some(sandbox), report) else {
        return vec![];
    };
    let work = dir.join("run");
    let inputs = rsk_hil::response::Inputs {
        rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        work_dir: &work,
        timeout_secs: 90,
        tick_hz: crate::hil::TICK_HZ,
        samplerate_hz: 100_000,
        flash_stimulus: true,
    };
    let out = rsk_hil::response::run(&inputs);
    let text = rsk_hil::response::render(rig, &inputs, &out);
    let mut artifacts = Vec::new();
    if let Ok(name) = store.write_artifact("hil-response.md", &text) {
        artifacts.push(name);
    }
    if let Some(ev) = &out.events {
        if let Ok(name) = store.write_artifact("hil-response-events.txt", &ev.render(crate::hil::TICK_HZ)) {
            artifacts.push(name);
        }
    }
    if !out.trace_text.is_empty() {
        if let Ok(name) = store.write_artifact("hil-response-trace.txt", &out.trace_text) {
            artifacts.push(name);
        }
    }
    for st in &out.steps {
        if st.ok {
            report.note(format!("HIL response suite: {}: {}", st.name, st.detail));
        } else {
            report.error(Rule::Step, None, format!("HIL response suite: {}: {}", st.name, st.detail));
        }
    }
    let mut item = Item::new(
        "hil-response",
        Kind::Test,
        &format!("HIL response suite: one injected fault per response of R14.5 on the response configuration of {} ({}; target {}, stimulus {})", bin, target.binary.hash, rig.target.serial, rig.stimulus.serial),
        Tool { name: "rsk-hil".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
        Verdict::from_bool(out.ok()),
    )
    .covers(&["R14.11", "R14.5", "R14.9", "R18.4"]);
    let mut commands = vec![target.command.clone()];
    commands.extend(out.commands.iter().cloned());
    item.command_lines = commands.clone();
    item.notes = out.steps.iter().map(|s| format!("{}: {}: {}", s.name, if s.ok { "pass" } else { "fail" }, s.detail)).collect();
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = artifacts;
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    commands
}

/// The isolation suite (tasks 7.4 and 3.3): the isolation configuration
/// (a HIL build with the `isolation` feature: the `hw-dt` probes and the
/// matrix walkers) on the rig's target, judged by `rsk_hil::isolation`;
/// the same run's `DT_RECORD` judged by `rsk_hil::dt`. Evidence_Items
/// `hil-mpu-matrix` (R16.8), `hil-dma` (R17.7), and one `hil-dt-hw-NN`
/// per Hardware_Model rule (R49.3). Skipped with a warning without a rig
/// or its target.
fn hil_isolation(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let Some(rig) = rig else { return vec![] };
    let probes = match rsk_hil::probes::list() {
        Ok(p) => p,
        Err(e) => {
            report.warn(format!("HIL isolation suite: {e}; not run"));
            return vec![];
        }
    };
    if !probes.iter().any(|p| p.id == rig.target.serial) {
        report.warn("HIL isolation suite: the target board is not attached; not run".to_string());
        return vec![];
    }
    let Some(bin) = ws.policy.qemu_spikes().next() else {
        report.error(Rule::Step, None, "HIL isolation suite: no QEMU spike Application to build the isolation configuration of");
        return vec![];
    };
    let Some(model) = crate::hil::task_model(ws, &bin, "nrf52840_isolation") else {
        report.error(Rule::Step, None, "HIL isolation suite: no Task_Model for the isolation configuration");
        return vec![];
    };
    let features = format!("{},isolation", crate::hil::FEATURES);
    let dir = target_dir.join("hil").join("isolation");
    let Some(target) = crate::hil::build_with(ws, &bin, &features, &dir, Some(sandbox), report) else {
        return vec![];
    };
    let work = dir.join("run");
    let inputs = rsk_hil::isolation::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 60, tick_hz: crate::hil::TICK_HZ, model: &model };
    let out = rsk_hil::isolation::run(&inputs);
    let text = rsk_hil::isolation::render(rig, &inputs, &out, &target.binary.hash);
    let mut artifacts = Vec::new();
    if let Ok(name) = store.write_artifact("hil-isolation.md", &text) {
        artifacts.push(name);
    }
    if let Some(ev) = &out.events {
        if let Ok(name) = store.write_artifact("hil-isolation-events.txt", &ev.render(crate::hil::TICK_HZ)) {
            artifacts.push(name);
        }
    }
    let mut dt_artifacts = Vec::new();
    if let Some(dt) = &out.dt {
        let dt_text = rsk_hil::dt::render(dt, &out.dt_rows, &rig.target.serial, &target.binary.hash);
        if let Ok(name) = store.write_artifact("hil-hw-model-differential.md", &dt_text) {
            dt_artifacts.push(name);
        }
    }
    for st in &out.steps {
        if st.ok {
            report.note(format!("HIL isolation suite: {}: {}", st.name, st.detail));
        } else {
            report.error(Rule::Step, None, format!("HIL isolation suite: {}: {}", st.name, st.detail));
        }
    }
    let tool = Tool { name: "rsk-hil".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] };
    let mut commands = vec![target.command.clone()];
    commands.extend(out.commands.iter().cloned());
    let step_ok = |prefix: &str| out.steps.iter().filter(|s| s.name.starts_with(prefix)).all(|s| s.ok) && out.steps.iter().any(|s| s.name.starts_with(prefix)) && out.steps.iter().any(|s| s.name == "target run" && s.ok);
    let mut items = vec![
        (
            Item::new(
                "hil-mpu-matrix",
                Kind::Test,
                &format!("HIL MPU access matrix (R16.8): every protected class of every other Partition and of the Kernel attempted by one Task per Partition on the isolation configuration of {} ({}; target {})", bin, target.binary.hash, rig.target.serial),
                tool.clone(),
                Verdict::from_bool(step_ok("MPU access matrix") && step_ok("walkers' counters") && step_ok("Kernel events")),
            )
            .covers(&["R16.8", "R16.4", "R16.1", "R16.2", "R16.3", "R16.5", "R14.3"]),
            artifacts.clone(),
        ),
        (
            Item::new(
                "hil-dma",
                Kind::Test,
                &format!("HIL EasyDMA isolation (R17.7): refused requests before any register write, the accepted request's registers, and the stop before the re-initialization of a restarted Partition, on the isolation configuration of {} ({})", bin, target.binary.hash),
                tool.clone(),
                Verdict::from_bool(step_ok("EasyDMA cases") && step_ok("Partition restart stops")),
            )
            .covers(&["R17.7", "R17.2", "R17.6", "R17.1"]),
            artifacts.clone(),
        ),
    ];
    let verdicts = rsk_hil::dt::verdicts(&out.dt_rows);
    for (rule, name, v) in &verdicts {
        let id = format!("hil-dt-{}", rule.to_lowercase());
        let verdict = match v {
            rsk_hil::dt::Verdict::Match => Verdict::Pass,
            rsk_hil::dt::Verdict::Mismatch => Verdict::Fail,
            rsk_hil::dt::Verdict::NotTested => Verdict::Fail,
        };
        let mut item = Item::new(&id, Kind::Test, &format!("Hardware_Model differential test {rule} ({name}) on the nRF52840-DK, from the isolation configuration's run ({})", target.binary.hash), tool.clone(), verdict).covers(&["R49.3"]);
        item.notes = out.dt_rows.iter().filter(|r| r.rule == *rule).map(|r| format!("{}: predicted {}; observed {}: {}", r.what, r.predicted, r.observed, match r.verdict { rsk_hil::dt::Verdict::Match => "match", rsk_hil::dt::Verdict::Mismatch => "MISMATCH", rsk_hil::dt::Verdict::NotTested => "not tested (disposition recorded)" })).collect();
        items.push((item, dt_artifacts.clone()));
    }
    if out.dt.is_none() {
        report.error(Rule::Step, None, "HIL isolation suite: no differential-test record; the hil-dt items are failed");
    }
    for (mut item, arts) in items {
        item.command_lines = commands.clone();
        if item.notes.is_empty() {
            item.notes = out.steps.iter().map(|s| format!("{}: {}: {}", s.name, if s.ok { "pass" } else { "fail" }, s.detail)).collect();
        }
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = arts;
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    commands
}

/// The boot-check suite (task 7.2, R12.6): the HIL build with the
/// `boot-mutation` feature, one run per mutated check, judged by
/// `rsk_hil::boot_checks`. Evidence_Item `hil-boot-checks`. Skipped with a
/// warning without a rig or its target.
fn hil_boot_checks(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let Some(rig) = rig else { return vec![] };
    let probes = match rsk_hil::probes::list() {
        Ok(p) => p,
        Err(e) => {
            report.warn(format!("HIL boot-check suite: {e}; not run"));
            return vec![];
        }
    };
    if !probes.iter().any(|p| p.id == rig.target.serial) {
        report.warn("HIL boot-check suite: the target board is not attached; not run".to_string());
        return vec![];
    }
    let Some(bin) = ws.policy.qemu_spikes().next() else {
        report.error(Rule::Step, None, "HIL boot-check suite: no QEMU spike Application to build");
        return vec![];
    };
    let features = format!("{},boot-mutation", crate::hil::FEATURES);
    let dir = target_dir.join("hil").join("boot-checks");
    let Some(target) = crate::hil::build_with(ws, &bin, &features, &dir, Some(sandbox), report) else {
        return vec![];
    };
    let work = dir.join("run");
    let inputs = rsk_hil::boot_checks::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 40, counters: crate::hil::COUNTERS };
    let out = rsk_hil::boot_checks::run(&inputs);
    let text = rsk_hil::boot_checks::render(rig, &out, &target.binary.hash);
    let mut artifacts = Vec::new();
    if let Ok(name) = store.write_artifact("hil-boot-checks.md", &text) {
        artifacts.push(name);
    }
    for st in &out.steps {
        if st.ok {
            report.note(format!("HIL boot-check suite: {}: {}", st.name, st.detail));
        } else {
            report.error(Rule::Step, None, format!("HIL boot-check suite: {}: {}", st.name, st.detail));
        }
    }
    let mut item = Item::new(
        "hil-boot-checks",
        Kind::Test,
        &format!("HIL boot-check suite (R12.6): one run per check of R12.1, R12.3, and R12.5 made to fail on the HIL build of {} with the `boot-mutation` feature ({}; target {})", bin, target.binary.hash, rig.target.serial),
        Tool { name: "rsk-hil".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
        Verdict::from_bool(out.ok()),
    )
    .covers(&["R12.6", "R12.2", "R12.1", "R12.3"]);
    let mut commands = vec![target.command.clone()];
    commands.extend(out.commands.iter().cloned());
    item.command_lines = commands.clone();
    item.notes = out.steps.iter().map(|s| format!("{}: {}: {}", s.name, if s.ok { "pass" } else { "fail" }, s.detail)).collect();
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = artifacts;
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    commands
}

/// The cycle-attribution test (task 7.5's `hil-cycles`, R18.1, ASM-06):
/// the cycle configuration on the rig's target, judged by
/// `rsk_hil::cycles` from the Kernel's record. Evidence_Item `hil-cycles`.
fn hil_cycles(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    // The plain configuration (Δ_entry), then its floating-point variant
    // (task 19.3: Δ_fp_save and Δ_fp_restore, `hil-cycles-fpu`).
    let mut commands = hil_cycles_variant(ws, rig, target_dir, sandbox, store, manifest_id, report, false);
    commands.extend(hil_cycles_variant(ws, rig, target_dir, sandbox, store, manifest_id, report, true));
    commands
}

#[allow(clippy::too_many_arguments)]
fn hil_cycles_variant(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report, fpu: bool) -> Vec<String> {
    let (feature, variant, item_name, low, tolerance) = if fpu {
        ("cycles-fpu", "nrf52840_cycles_fpu", "hil-cycles-fpu", rsk_hil::cycles::LOW_FPU, rsk_hil::cycles::TOLERANCE_PERCENT_FPU)
    } else {
        ("cycles", "nrf52840_cycles", "hil-cycles", rsk_hil::cycles::LOW, rsk_hil::cycles::TOLERANCE_PERCENT)
    };
    let Some(rig) = rig else { return vec![] };
    let probes = match rsk_hil::probes::list() {
        Ok(p) => p,
        Err(e) => {
            report.warn(format!("HIL cycle-attribution test: {e}; not run"));
            return vec![];
        }
    };
    if !probes.iter().any(|p| p.id == rig.target.serial) {
        report.warn("HIL cycle-attribution test: the target board is not attached; not run".to_string());
        return vec![];
    }
    let Some(bin) = ws.policy.qemu_spikes().next() else {
        report.error(Rule::Step, None, "HIL cycle-attribution test: no QEMU spike Application to build");
        return vec![];
    };
    let features = format!("{},{feature}", crate::hil::FEATURES);
    let dir = target_dir.join("hil").join(if fpu { "cycles-fpu" } else { "cycles" });
    let Some(target) = crate::hil::build_with(ws, &bin, &features, &dir, Some(sandbox), report) else {
        return vec![];
    };
    let work = dir.join("run");
    let inputs = rsk_hil::cycles::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 40, tick_hz: crate::hil::TICK_HZ, cpu_hz: crate::hil::CPU_HZ, low, tolerance_percent: tolerance };
    let out = rsk_hil::cycles::run(&inputs);
    let text = rsk_hil::cycles::render(rig, &inputs, &out, &target.binary.hash);
    let mut artifacts = Vec::new();
    if let Ok(name) = store.write_artifact(&format!("{item_name}.md"), &text) {
        artifacts.push(name);
    }
    for st in &out.steps {
        if st.ok {
            report.note(format!("HIL cycle-attribution test: {}: {}", st.name, st.detail));
        } else {
            report.error(Rule::Step, None, format!("HIL cycle-attribution test: {}: {}", st.name, st.detail));
        }
    }
    let mut item = Item::new(
        item_name,
        Kind::Test,
        &format!("HIL cycle attribution (R18.1, ASM-06{}): the Budget accounting's executions of a preempted Task against an unpreempted one executing the same spin, on the {variant} configuration of {} ({}; target {})", if fpu { "; task 19.3, the FPU Task preempted: Δ_fp_save and Δ_fp_restore" } else { "" }, bin, target.binary.hash, rig.target.serial),
        Tool { name: "rsk-hil".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
        Verdict::from_bool(out.ok()),
    )
    .covers(&["R18.1"]);
    let mut commands = vec![target.command.clone()];
    commands.extend(out.commands.iter().cloned());
    item.command_lines = commands.clone();
    item.notes = out.steps.iter().map(|s| format!("{}: {}: {}", s.name, if s.ok { "pass" } else { "fail" }, s.detail)).collect();
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = artifacts;
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    commands
}

/// Task 7.1 (R34.8, R48.4): the HIL_Rig self-test (`rsk-hil`): the boards
/// by serial, the analyzer, the stimulus image flashed and read back, the
/// target image run and read back against the Build_Manifest, and the
/// capture agreeing with the console. Skipped with a warning without a
/// rig description or without the rig's probes.
fn hil_rig(ws: &Workspace, rig: Option<&rsk_hil::config::Rig>, builds: &[crate::hil::HilBuild], target_dir: &Path, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let Some(rig) = rig else { return vec![] };
    let probes = match rsk_hil::probes::list() {
        Ok(p) => p,
        Err(e) => {
            report.warn(format!("HIL_Rig: {e}; the self-test was not run"));
            return vec![];
        }
    };
    if !probes.iter().any(|p| p.id == rig.target.serial) || !probes.iter().any(|p| p.id == rig.stimulus.serial) {
        report.warn(format!(
            "HIL_Rig: the target ({}) or the stimulus ({}) board is not attached ({} probes seen); the self-test was not run",
            rig.target.serial,
            rig.stimulus.serial,
            probes.len()
        ));
        return vec![];
    }
    let targets: Vec<&crate::hil::HilBuild> = ws.policy.qemu_spikes().filter_map(|bin| builds.iter().find(|b| b.bin == bin)).collect();
    let Some(target) = targets.first() else {
        report.error(Rule::Step, None, "HIL_Rig: no HIL build of a QEMU spike Application to run on the target");
        return vec![];
    };
    let Some(stimulus) = rig.stimulus.bin.as_deref().and_then(|bin| builds.iter().find(|b| b.bin == bin)) else {
        report.error(Rule::Step, None, format!("HIL_Rig: no HIL build of the stimulus binary `{}`", rig.stimulus.bin.as_deref().unwrap_or("?")));
        return vec![];
    };
    let work = target_dir.join("hil").join("rig");
    let inputs = rsk_hil::selftest::Inputs {
        rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        target_hash: Some(&target.binary.hash),
        stimulus_hash: Some(&stimulus.binary.hash),
        work_dir: &work,
        timeout_secs: 90,
        tick_hz: crate::hil::TICK_HZ,
    };
    let out = rsk_hil::selftest::run(&inputs);
    let text = rsk_hil::selftest::render(rig, &out);
    let mut artifacts = Vec::new();
    if let Ok(name) = store.write_artifact("hil-rig-self-test.md", &text) {
        artifacts.push(name);
    }
    if !out.edges_text.is_empty() {
        if let Ok(name) = store.write_artifact("hil-rig-edges.txt", &out.edges_text) {
            artifacts.push(name);
        }
    }
    if !out.trace_text.is_empty() {
        if let Ok(name) = store.write_artifact("hil-rig-trace.txt", &out.trace_text) {
            artifacts.push(name);
        }
    }
    if !out.events_text.is_empty() {
        if let Ok(name) = store.write_artifact("hil-rig-events.txt", &out.events_text) {
            artifacts.push(name);
        }
    }
    for st in &out.steps {
        if st.ok {
            report.note(format!("HIL_Rig: {}: {}", st.name, st.detail));
        } else if out.locked {
            report.warn(format!("HIL_Rig: {}: {}", st.name, st.detail));
        } else {
            report.error(Rule::Step, None, format!("HIL_Rig: {}: {}", st.name, st.detail));
        }
    }
    let verdict = if out.ok() {
        Verdict::Pass
    } else if out.locked {
        Verdict::Inconclusive
    } else {
        Verdict::Fail
    };
    let mut item = Item::new(
        "hil-rig",
        Kind::Test,
        &format!("HIL_Rig self-test: target {}, stimulus {} ({}), analyzer {}", rig.target.serial, rig.stimulus.serial, stimulus.bin, rig.analyzer.driver),
        Tool { name: "rsk-hil".to_string(), version: format!("{} ({}; {})", env!("CARGO_PKG_VERSION"), out.probe_rs, out.sigrok), solvers: vec![] },
        verdict,
    )
    .covers(&["R34.8", "R48.4"]);
    item.command_lines = out.commands.clone();
    item.notes = out.steps.iter().map(|s| format!("{}: {}: {}", s.name, if s.ok { "pass" } else { "fail" }, s.detail)).collect();
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = artifacts;
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    out.commands
}

/// Task 17.3 (R46.1): the Aeneas evaluation. `proofs/aeneas/lean` holds the
/// Aeneas translation of `proofs/aeneas/controllaw` and the Lean proofs over
/// it; `lake build` there checks them. Skipped with a warning where `lake`
/// (elan) is absent; the translation is committed, so the step needs Lean
/// and the pinned Aeneas checkout's Lean library, not Charon or Aeneas.
fn aeneas_evaluation(ws: &Workspace, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let dir = ws.root.join("proofs").join("aeneas").join("lean");
    if !dir.join("lakefile.lean").is_file() {
        report.note("Aeneas: no Lean project at proofs/aeneas/lean");
        return vec![];
    }
    let home = std::env::var_os("HOME").map(PathBuf::from).unwrap_or_default();
    let candidates = ["lake".to_string(), home.join(".elan").join("bin").join("lake").to_string_lossy().into_owned()];
    let lake = candidates
        .iter()
        .find_map(|c| Cmd::new(c).arg("--version").cwd(&dir).output(None).ok().filter(|o| o.status.success()).map(|o| (c.clone(), String::from_utf8_lossy(&o.stdout).trim().to_string())));
    let Some((lake, lake_version)) = lake else {
        report.warn("lake (Lean) is not installed: the Aeneas evaluation's proofs were not checked (toolchain/decision.toml [aeneas] records the pinned installation)");
        return vec![];
    };
    let lean_version = Cmd::new(&lake).args(["env", "lean", "--version"]).cwd(&dir).output(None).ok().map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string()).unwrap_or_default();
    let cmd = Cmd::new(&lake).arg("build").cwd(&dir);
    let command = format!("(cd proofs/aeneas/lean && {})", cmd.display());
    let (ok, text) = match cmd.output(Some(sandbox)) {
        Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => (false, format!("cannot run lake: {e}")),
    };
    let _ = store.write_artifact("aeneas.log", &text);
    // Lean prints the name in quotes or, since 4.2x, in backticks.
    let sorries = text.matches("declaration uses 'sorry'").count() + text.matches("declaration uses `sorry`").count();
    let ok = ok && sorries == 0;
    if ok {
        report.note(format!("R46.1: Aeneas evaluation: the Lean proofs over the control law's translation and over the Analyzer's response-time fixed point (the Verus-erased build, verification_plan.md 7.2) and the response-time bound over a run of the Kernel's scheduler as extracted (7.3) check ({lean_version}; {lake_version})"));
    } else {
        report.error(Rule::Step, None, format!("Aeneas evaluation: lake build failed or left {sorries} sorry (see aeneas.log)"));
    }
    let mut item = Item::new(
        "proof-app-aeneas",
        Kind::Proof,
        "Aeneas evaluation (task 17.3): the Lean proofs over the Aeneas translation of the fixed-point PI control law (panic and overflow freedom, saturation, integral bound), and over the translation of the Analyzer's numeric chain as compiled without Verus (verification_plan.md 7.2: the fixed point, the terms of a Task's recurrence, one Task's verdict end to end, and the term derivation)",
        Tool { name: "Lean (Aeneas library)".to_string(), version: format!("{lean_version}; {lake_version}"), solvers: vec![] },
        Verdict::from_bool(ok),
    )
    .covers(&["R46.1", "R46.5"]);
    item.command_lines = vec![command.clone()];
    item.notes = vec!["evaluation only (verification_plan.md section 7.1): Aeneas is not the designated App_Prover; the translation (ControlLaw/Controllaw.lean) is generated by the pinned Charon and Aeneas and committed; the proofs are handwritten (ControlLaw/Proofs.lean)".to_string()];
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["aeneas.log".to_string()];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    vec![command]
}

/// R32.6, PAR-03: the UPPAAL cross-check corpus (`corpus.rs`): 50 generated
/// Task_Models analysed and, where `verifyta` is installed, cross-checked.
/// Evidence_Item `check-uppaal-corpus`: pass when no cross-check is a
/// discrepancy without a disposition, inconclusive when the verifier is
/// absent or a run hit its limits, fail on an undisposed discrepancy. The
/// committed report `docs/uppaal_corpus_report.md` must equal the
/// regenerated one.
fn uppaal_corpus(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let work = crate::corpus::work_dir(target_dir);
    let outcome = match crate::corpus::run(ws, target_dir, &work, Some(sandbox), report) {
        Ok(o) => o,
        Err(e) => {
            report.error(Rule::Step, None, format!("UPPAAL corpus: {e}"));
            return vec![];
        }
    };
    let _ = store.write_artifact("uppaal-corpus-report.md", &outcome.report);
    let committed = crate::corpus::report_path(&ws.root);
    match std::fs::read_to_string(&committed) {
        Ok(text) if text == outcome.report => {}
        Ok(_) => report.error(Rule::Step, Some(ws.rel(&committed)), "differs from the regenerated UPPAAL corpus report; run `cargo xtask uppaal-corpus`"),
        Err(_) => report.error(Rule::Step, Some(ws.rel(&committed)), "missing; run `cargo xtask uppaal-corpus`"),
    }
    let not_run = outcome.by_status.get("not_run").copied().unwrap_or(0);
    let inconclusive = outcome.by_status.get("inconclusive").copied().unwrap_or(0);
    let verdict = if !outcome.undisposed.is_empty() {
        report.error(Rule::Step, None, format!("UPPAAL corpus: discrepancy without a disposition in {} (R31.5, R32.6)", outcome.undisposed.join(", ")));
        Verdict::Fail
    } else if outcome.verifier.is_none() {
        report.warn(format!("UPPAAL corpus: verifyta is not installed; {} models exported, no cross-check run (ORQ-22): the item is inconclusive", outcome.rows.len()));
        Verdict::Inconclusive
    } else if not_run + inconclusive > 0 {
        report.warn(format!("UPPAAL corpus: {} of {} cross-checks not run or inconclusive (R31.9)", not_run + inconclusive, outcome.rows.len()));
        Verdict::Inconclusive
    } else {
        report.note(format!("R32.6: UPPAAL corpus: {} models, statuses {:?} ({})", outcome.rows.len(), outcome.by_status, outcome.verifier.clone().unwrap_or_default()));
        Verdict::Pass
    };
    let mut item = Item::new(
        "check-uppaal-corpus",
        Kind::Analysis,
        "UPPAAL cross-check corpus (PAR-03): generated Task_Models analysed with placeholder timing and cross-checked against their exported timed automata where verifyta is installed",
        Tool { name: "rsk-analyzer".to_string(), version: format!("{} ({})", env!("CARGO_PKG_VERSION"), outcome.verifier.clone().unwrap_or_else(|| "verifyta absent".into())), solvers: vec![] },
        verdict,
    )
    .covers(&["R32.6", "R31.4", "R31.5", "R31.6", "R31.9"]);
    item.command_lines = vec!["cargo xtask uppaal-corpus".to_string()];
    item.notes = vec![format!("{} models; cross-check statuses {:?}; dispositions in docs/uppaal_dispositions.toml", outcome.rows.len(), outcome.by_status)];
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["uppaal-corpus-report.md".to_string()];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    vec!["cargo xtask uppaal-corpus".to_string()]
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
        item.notes = vec!["placeholder timing: every WCET_Record equals the Task's Budget and every Kernel_Timing_Parameter is nominal; the result is a check of the pipeline and of the Budgets' schedulability, not timing evidence (R33.1); the HIL_Rig's measured Kernel_Timing_Parameters apply to the nRF52840 measurement configurations (`analysis-rta-*-measured`, task 19.3), not to this QEMU binary".to_string()];
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name, format!("rta-{}/report.json", binary.target_name), format!("rta-{}/report.txt", binary.target_name), format!("rta-{}/model.xml", binary.target_name)];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    if commands.is_empty() {
        report.note("Analyzer: no Flight_Build binary with a Task_Model and a Build_Manifest");
    }
    commands.extend(analyzer_measured_runs(ws, target_dir, sandbox, store, manifest_id, &manifest_file, report));
    commands
}

/// Task 19.3: the analyses that use the measured Kernel_Timing_Parameters.
/// The rig's records are bound to the HIL builds of the measurement
/// configurations (nRF52840, 64 MHz), so they apply to those
/// configurations' Task_Models, not to the QEMU Flight_Build's (R28.3,
/// R33.4: a record of another clock or binary is rejected). Each fresh
/// record (its binary among this manifest's `hil_binaries`) drives one
/// analysis of its configuration's model with placeholder WCET_Records
/// and the measured parameters over the nominal ones, as its own
/// Evidence_Item. While the WCET_Records are placeholders (every WCET the
/// Budget, the most pessimistic blocking term), a FAIL verdict is recorded
/// as a finding, not as a job violation; a rejected input or a tool error
/// is a violation. A stale record (R33.4) is named and not used.
fn analyzer_measured_runs(ws: &Workspace, target_dir: &Path, sandbox: &Sandbox, store: &mut Store, manifest_id: &str, manifest_file: &Path, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    let manifest_value: serde_json::Value = std::fs::read_to_string(manifest_file).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(serde_json::Value::Null);
    let (fresh, stale) = crate::hil::fresh_timing_records(&ws.root, &manifest_value);
    for s in &stale {
        report.note(format!("R33.4: stale Kernel_Timing_Parameter record not used: {s}"));
    }
    // The records of the further configurations (`RECORD_VARIANTS`, the cycle
    // test's Δ_entry) go into every analysis; their own configurations are
    // not analysed (their Tasks exist for the probes).
    let variant_of = |p: &Path| p.file_name().and_then(|f| f.to_str()).and_then(|f| f.strip_prefix("kernel_timing_")).and_then(|f| f.strip_suffix(".json")).map(str::to_string);
    let extra_records: Vec<PathBuf> = fresh.iter().filter(|r| variant_of(r).is_some_and(|v| crate::hil::RECORD_VARIANTS.iter().any(|(rv, _)| *rv == v))).cloned().collect();
    for record in &fresh {
        let Some(variant) = variant_of(record) else { continue };
        let variant = variant.as_str();
        if crate::hil::RECORD_VARIANTS.iter().any(|(rv, _)| *rv == variant) {
            continue;
        }
        let Some(bin) = ws.policy.qemu_spikes().next() else { continue };
        let Some(app_dir) = ws.packages.values().find(|p| p.name == bin).and_then(|p| p.dir.parent().map(Path::to_path_buf)) else { continue };
        let model = app_dir.join(format!("gen-{variant}")).join("task_model.json");
        if !model.is_file() {
            report.note(format!("task 19.3: no Task_Model for `{variant}` at {}: the record {} is not analysed", ws.rel(&model), ws.rel(record)));
            continue;
        }
        // The record's binary and its Generated_Config checksum, from the manifest.
        let hash = std::fs::read_to_string(record).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok()).map(|t| t.binary_hash).unwrap_or_default();
        let checksum = manifest_value
            .get("hil_binaries")
            .and_then(|b| b.as_array())
            .into_iter()
            .flatten()
            .find(|b| b.get("hash").and_then(|h| h.as_str()) == Some(hash.as_str()))
            .and_then(|b| b.get("config_checksum").cloned())
            .unwrap_or(serde_json::Value::Null);
        // Task 19.1: the configuration's measured WCET_Records, when fresh.
        let wcet_record = match crate::hil::fresh_wcet_records(&ws.root, &manifest_value, variant) {
            Ok(w) => w,
            Err(why) => {
                report.note(format!("R33.4: stale WCET_Record set not used: {why}"));
                None
            }
        };
        let out_dir = store.dir.join(format!("rta-{variant}-measured"));
        let _ = std::fs::create_dir_all(&out_dir);
        let mut value = manifest_value.clone();
        value["generated_config_checksum"] = checksum;
        let manifest_copy = out_dir.join("build-manifest.json");
        if let Err(e) = std::fs::write(&manifest_copy, serde_json::to_string_pretty(&value).unwrap_or_default()) {
            report.error(Rule::Evidence, None, format!("cannot write {}: {e}", ws.rel(&manifest_copy)));
            continue;
        }
        let cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
            .arg(ws.root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--")
            .arg("--model")
            .arg(&model)
            .arg("--manifest")
            .arg(&manifest_copy)
            .arg("--placeholders")
            .arg("--timing")
            .arg(record);
        let cmd = extra_records.iter().fold(cmd, |c, r| c.arg("--timing").arg(r));
        let cmd = match &wcet_record {
            Some(w) => cmd.arg("--wcet").arg(w),
            None => cmd,
        };
        let cmd = cmd.arg("--out").arg(&out_dir);
        commands.push(cmd.display());
        let (status, text) = match cmd.output(Some(sandbox)) {
            Ok(o) => (o.status.code().unwrap_or(3), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
            Err(e) => (3, format!("cannot run rsk-analyzer: {e}")),
        };
        let log_name = format!("rta-{variant}-measured.log");
        let _ = store.write_artifact(&log_name, &text);
        let measured_line = text.lines().find(|l| l.contains("MEASURED Kernel_Timing_Parameters")).map(|l| l.trim().to_string()).unwrap_or_default();
        let unschedulable: Vec<String> = text.lines().filter(|l| l.contains("NOT schedulable")).map(|l| l.trim().to_string()).collect();
        // A rejection under R28.4 (a measured WCET above the Task's Budget)
        // is a finding of a measurement configuration whose Jobs overrun by
        // design (the release-timing configuration's late `sensor` Jobs):
        // the Analyzer is right to reject the inputs, and the job records it.
        let budget_rejection = status == 2 && text.contains("[R28.4]");
        // A fault-injection configuration (the stress runs) misbehaves by
        // design: its Tasks exceed their Budgets and their deadlines on
        // purpose, R18.8's claim being part0's response times on the
        // board (11.4), so its analysis supplies the Kernel_Timing_Parameters
        // and the interference figures and makes no schedulability claim
        // (task 19.2, 2026-11-05): FAIL and R28.4 are recorded as
        // inconclusive, with the reason, not as the set's verdict.
        let fault_injection = crate::hil::MEASUREMENTS.iter().any(|m| m.variant == variant && m.stress);
        // The stress configuration's documented claim (11.14, 2026-11-07):
        // the Tasks of the highest Criticality_Level are schedulable, the
        // lower-criticality Tasks need not be (the configuration's overload
        // is its purpose, R18.7). The item passes on that claim.
        let protected: Vec<String> = crate::hil::task_model(ws, &bin, variant)
            .map(|m| {
                let top = m.partitions.iter().map(|p| p.level.clone()).min().unwrap_or_default();
                let ids: Vec<String> = m.partitions.iter().filter(|p| p.level == top).map(|p| p.id.clone()).collect();
                m.tasks.iter().filter(|t| ids.contains(&t.partition)).map(|t| format!("`{}`", t.id)).collect()
            })
            .unwrap_or_default();
        let protected_schedulable = fault_injection && status == 1 && !protected.is_empty() && !unschedulable.iter().any(|l| protected.iter().any(|t| l.contains(t.as_str())));
        let verdict = match status {
            0 => Verdict::Pass,
            1 if protected_schedulable => Verdict::Pass,
            1 if fault_injection => Verdict::Inconclusive,
            1 => Verdict::Fail,
            _ => Verdict::Inconclusive,
        };
        let wcet_used = match &wcet_record {
            Some(w) => format!("the measured WCET_Records of {} over placeholders for the rest", ws.rel(w)),
            None => "placeholder WCETs".to_string(),
        };
        match status {
            0 => report.note(format!("task 19.3: `{variant}`: schedulable with the measured Kernel_Timing_Parameters of {} and {wcet_used}; {measured_line}", ws.rel(record))),
            1 if protected_schedulable => report.note(format!(
                "task 19.3: `{variant}`: the protected Partition's Tasks ({}) schedulable with the measured Kernel_Timing_Parameters of {} and {wcet_used}, the lower-criticality Tasks not, as the fault-injection configuration claims (R18.7, 11.14): {}; {measured_line}",
                protected.join(", "),
                ws.rel(record),
                unschedulable.join(" | ")
            )),
            1 => report.note(format!(
                "task 19.3 FINDING: `{variant}`: NOT schedulable with the measured Kernel_Timing_Parameters of {} and {wcet_used}{}: {}; {measured_line}",
                ws.rel(record),
                if wcet_record.is_none() { " (every WCET the Budget: the blocking term is the whole Budget of the Critical_Section's Task); not a violation until the WCET_Records are measured (tasks 19.1, 19.2)" } else { "; not a violation while any WCET_Record is a placeholder (task 19.2)" },
                unschedulable.join(" | ")
            )),
            _ if budget_rejection => report.note(format!(
                "task 19.1 FINDING: `{variant}`: the Analyzer rejected the measured inputs under R28.4 (a measured WCET above its Task's Budget: the configuration's injected overruns); not a violation: {}",
                text.lines().find(|l| l.contains("[R28.4]")).map(|l| l.trim()).unwrap_or("")
            )),
            _ => report.error(Rule::Step, None, format!("Analyzer: `{variant}` with measured Kernel_Timing_Parameters: exit {status} (see {log_name})")),
        }
        let mut item = Item::new(
            &format!("analysis-rta-{variant}-measured"),
            Kind::Analysis,
            &format!("Analyzer: response-time analysis of the `{variant}` configuration with placeholder WCET_Records and the measured Kernel_Timing_Parameters of the HIL_Rig (task 19.3; not timing evidence while the WCET_Records are placeholders)"),
            Tool { name: "rsk-analyzer".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            verdict,
        )
        .covers(&["R13.2", "R13.3", "R28.3", "R29.1", "R29.3", "R33.4"]);
        if wcet_record.is_some() {
            item = item.covers(&["R34.3", "R34.6", "R34.7", "R28.2", "R29.2"]);
        }
        item.command_lines = vec![commands.last().cloned().unwrap_or_default()];
        item.notes = vec![format!(
            "measured J_rel and Δ_irq from {} (the rig's runs on the HIL build {} of this manifest, margin {} %, R34.6) over nominal placeholders for the other Kernel_Timing_Parameters, with {wcet_used} (a placeholder WCET is the Task's Budget): {}",
            ws.rel(record),
            hash,
            crate::hil::MARGIN_PERCENT,
            match status {
                0 => "schedulable".to_string(),
                1 if protected_schedulable => format!("the protected Partition's Tasks schedulable, the lower-criticality Tasks not, the fault-injection configuration's documented property (R18.7, 11.14): {}", unschedulable.join(" | ")),
                1 => format!("NOT schedulable under these inputs, a finding of task 19.3 pending measured WCET_Records: {}", unschedulable.join(" | ")),
                _ if budget_rejection => "REJECTED under R28.4: a measured WCET above its Task's Budget, the configuration's injected overruns (task 19.1)".to_string(),
                _ => format!("the Analyzer did not run to a verdict (exit {status})"),
            }
        )];
        if fault_injection {
            item.notes.insert(0, format!("fault-injection configuration `{variant}`: its Tasks exceed their Budgets and deadlines by design (R18.8), so this analysis supplies the Kernel_Timing_Parameters and interference figures and makes no schedulability claim; a FAIL or an R28.4 rejection is recorded as inconclusive (task 19.2, Verification_Plan 11.11)"));
        }
        if !stale.is_empty() {
            item.notes.push(format!("stale Kernel_Timing_Parameter records not used (R33.4): {}", stale.join("; ")));
        }
        item.build_manifest = Some(manifest_id.to_string());
        item.artifacts = vec![log_name, format!("rta-{variant}-measured/report.json"), format!("rta-{variant}-measured/report.txt")];
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
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
    checks::kernel_size::kernel_sources(ws, &cfg)
        .0
        .into_iter()
        .filter_map(|(_, f)| checks::kernel_size::count_file(&f, &cfg).ok().flatten())
        .map(|c| c.exec)
        .sum()
}

fn build_manifest(
    ws: &Workspace,
    target_dir: &Path,
    sandbox: &Sandbox,
    verus: Option<Verus>,
    kani: Option<Kani>,
    creusot: Option<Creusot>,
    hil_builds: &mut Vec<crate::hil::HilBuild>,
    report: &mut Report,
) -> Option<Manifest> {
    let mut manifest = match Manifest::collect(ws, verus, kani, creusot) {
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
    // Task 7.1: the HIL builds (the boards' images), each its own cargo
    // invocation under the job's target directory; the rig's read-back
    // check compares the loaded images against these hashes (R48.4).
    *hil_builds = crate::hil::build_all(ws, &target_dir.join("hil"), Some(sandbox), report);
    for b in hil_builds.iter() {
        report.note(format!("HIL build `{}`: {} over {} loadable bytes", b.binary.target_name, b.binary.hash, b.binary.loadable_bytes));
        manifest.hil_binaries.push(b.binary.clone());
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

/// Task 19.5 (R53.1, R53.3): the structural coverage reports the plan
/// selects (section 14): the Kernel crates' host tests run under the
/// coverage toolchain's source-level instrumentation with branch counters
/// (`coverage-kernel-host`, llvm-cov's totals over the Kernel sources;
/// skipped with a warning when the toolchain of `toolchain/decision.toml
/// [coverage]` is not installed), and the Partitions' on-target reports of
/// the measurement runs (`coverage-partitions-<variant>`, from the
/// committed `docs/hil/coverage_<variant>.json`: per crate and per Task
/// entry against the recorded threshold). Level A, B, and C code is the
/// three Partition crates; the Kernel's on-target coverage stays GAP-02.
fn structural_coverage(ws: &Workspace, target_dir: &Path, _sandbox: &Sandbox, store: &mut Store, manifest_id: &str, report: &mut Report) -> Vec<String> {
    let mut commands = Vec::new();
    // (a) The Kernel on the host.
    let mut host_report = Report::default();
    match crate::hil::coverage_tools(&mut host_report) {
        None => report.warn(format!("structural coverage: the coverage toolchain `{}` is not installed; the Kernel's host coverage was not measured (section 14)", crate::hil::COVERAGE_TOOLCHAIN)),
        Some(tools) => {
            let dir = target_dir.join("coverage-host");
            let _ = std::fs::create_dir_all(&dir);
            for f in std::fs::read_dir(&dir).into_iter().flatten().flatten() {
                if f.path().extension().and_then(|e| e.to_str()) == Some("profraw") {
                    let _ = std::fs::remove_file(f.path());
                }
            }
            let crates = ["rsk-logic", "rsk-kernel"];
            let mut cmd = Cmd::cargo_frozen("test").args(["--quiet"]);
            for c in crates {
                cmd = cmd.args(["-p", c]);
            }
            let cmd = cmd
                .arg("--manifest-path")
                .arg(ws.root.join("Cargo.toml"))
                .arg("--target-dir")
                .arg(&dir)
                .env("RUSTUP_TOOLCHAIN", crate::hil::COVERAGE_TOOLCHAIN)
                // The job pins CARGO_ENCODED_RUSTFLAGS empty (which cargo
                // takes over RUSTFLAGS), so the flags go in through it.
                .env("CARGO_ENCODED_RUSTFLAGS", "-C\x1finstrument-coverage\x1f-Z\x1fcoverage-options=branch")
                .env("LLVM_PROFILE_FILE", dir.join("%p-%m.profraw"));
            commands.push(format!("RUSTUP_TOOLCHAIN={} RUSTFLAGS='-C instrument-coverage -Z coverage-options=branch' {}", crate::hil::COVERAGE_TOOLCHAIN, cmd.display()));
            let tests_ok = match cmd.output(None) {
                Ok(o) if o.status.success() => true,
                Ok(o) => {
                    report.error(Rule::Step, None, format!("structural coverage: the Kernel crates' instrumented tests failed:\n{}", String::from_utf8_lossy(&o.stderr)));
                    false
                }
                Err(e) => {
                    report.error(Rule::Step, None, format!("structural coverage: cannot run cargo: {e}"));
                    false
                }
            };
            let profraws: Vec<PathBuf> = std::fs::read_dir(&dir).into_iter().flatten().flatten().map(|f| f.path()).filter(|p| p.extension().and_then(|e| e.to_str()) == Some("profraw")).collect();
            // The test binaries, wherever this cargo's layout puts them
            // (`debug/deps` or `debug/build/<crate>/<hash>/out`): the
            // crate's name, a hyphen, and sixteen hex digits, executable.
            fn test_binaries(dir: &Path, crates: &[&str], out: &mut Vec<PathBuf>) {
                for f in std::fs::read_dir(dir).into_iter().flatten().flatten() {
                    let p = f.path();
                    if p.is_dir() {
                        test_binaries(&p, crates, out);
                    } else if let Some(name) = p.file_name().and_then(|n| n.to_str()) {
                        let is_test = crates.iter().any(|c| {
                            let prefix = format!("{}-", c.replace('-', "_"));
                            name.strip_prefix(&prefix).is_some_and(|h| h.len() == 16 && h.chars().all(|ch| ch.is_ascii_hexdigit()))
                        });
                        if is_test {
                            out.push(p);
                        }
                    }
                }
            }
            let mut binaries = Vec::new();
            test_binaries(&dir.join("debug"), &crates, &mut binaries);
            let mut notes = Vec::new();
            let mut ok = tests_ok && !profraws.is_empty() && !binaries.is_empty();
            if ok {
                let profdata = dir.join("kernel-host.profdata");
                let merge = std::process::Command::new(&tools.profdata).arg("merge").arg("-o").arg(&profdata).args(&profraws).output();
                match merge {
                    Ok(o) if o.status.success() => {
                        let mut cov = std::process::Command::new(&tools.cov);
                        cov.arg("export").arg("-format=text").arg("-summary-only").arg("-instr-profile").arg(&profdata);
                        for b in &binaries {
                            cov.arg("-object").arg(b);
                        }
                        for c in crates {
                            cov.arg("-sources").arg(ws.root.join("crates").join(c).join("src"));
                        }
                        match cov.output() {
                            Ok(o) if o.status.success() => {
                                let _ = store.write_artifact("coverage-kernel-host.json", &String::from_utf8_lossy(&o.stdout));
                                match serde_json::from_slice::<serde_json::Value>(&o.stdout) {
                                    Ok(v) => {
                                        let totals = &v["data"][0]["totals"];
                                        let pct = |k: &str| (totals[k]["percent"].as_f64().unwrap_or(0.0), totals[k]["covered"].as_u64().unwrap_or(0), totals[k]["count"].as_u64().unwrap_or(0));
                                        let (l, b, f, r) = (pct("lines"), pct("branches"), pct("functions"), pct("regions"));
                                        notes.push(format!("the Kernel crates' host tests (rsk-logic, rsk-kernel; {} test binaries, {} profiles): lines {:.1} % ({} of {}), branches {:.1} % ({} of {}), functions {:.1} % ({} of {}), regions {:.1} % ({} of {}) of the Kernel sources, source-level instrumentation by {} (R53.1; the Verus proofs and Kani harnesses cover the logic beyond these tests, section 14)", binaries.len(), profraws.len(), l.0, l.1, l.2, b.0, b.1, b.2, f.0, f.1, f.2, r.0, r.1, r.2, crate::hil::COVERAGE_TOOLCHAIN));
                                        for file in v["data"][0]["files"].as_array().into_iter().flatten() {
                                            let name = file["filename"].as_str().unwrap_or("");
                                            let rel = name.strip_prefix(&format!("{}/", ws.root.display())).unwrap_or(name);
                                            let s = &file["summary"];
                                            notes.push(format!("{rel}: lines {:.1} %, branches {:.1} % ({} of {})", s["lines"]["percent"].as_f64().unwrap_or(0.0), s["branches"]["percent"].as_f64().unwrap_or(0.0), s["branches"]["covered"].as_u64().unwrap_or(0), s["branches"]["count"].as_u64().unwrap_or(0)));
                                        }
                                    }
                                    Err(e) => {
                                        ok = false;
                                        report.error(Rule::Step, None, format!("structural coverage: llvm-cov's export is not JSON: {e}"));
                                    }
                                }
                            }
                            Ok(o) => {
                                ok = false;
                                report.error(Rule::Step, None, format!("structural coverage: llvm-cov export failed: {}", String::from_utf8_lossy(&o.stderr).trim()));
                            }
                            Err(e) => {
                                ok = false;
                                report.error(Rule::Step, None, format!("structural coverage: cannot run llvm-cov: {e}"));
                            }
                        }
                    }
                    Ok(o) => {
                        ok = false;
                        report.error(Rule::Step, None, format!("structural coverage: llvm-profdata merge failed: {}", String::from_utf8_lossy(&o.stderr).trim()));
                    }
                    Err(e) => {
                        ok = false;
                        report.error(Rule::Step, None, format!("structural coverage: cannot run llvm-profdata: {e}"));
                    }
                }
            } else if tests_ok {
                report.error(Rule::Step, None, format!("structural coverage: no profile ({} files) or no test binary ({}) under {}", profraws.len(), binaries.len(), ws.rel(&dir)));
            }
            for n in &notes {
                report.note(format!("R53.3: {n}"));
            }
            let mut item = Item::new(
                "coverage-kernel-host",
                Kind::Measurement,
                "Structural coverage of the Kernel crates on the host (R53.1, R53.3): their requirements-based host tests under source-level instrumentation with branch counters, llvm-cov's totals over the Kernel sources (task 19.5; Verification_Plan section 14)",
                Tool { name: "llvm-cov".to_string(), version: crate::hil::COVERAGE_TOOLCHAIN.to_string(), solvers: vec![] },
                Verdict::from_bool(ok),
            )
            .covers(&["R53.1", "R53.3"]);
            item.command_lines = commands.clone();
            item.notes = notes;
            item.build_manifest = Some(manifest_id.to_string());
            item.artifacts = vec!["coverage-kernel-host.json".to_string()];
            if let Err(e) = store.write(&item) {
                report.error(Rule::Evidence, None, e);
            }
        }
    }
    // (b) The Kernel on the Target: the committed report of the Kernel
    // coverage build (`cargo xtask hil --coverage --kernel`), when any.
    let Some(bin) = ws.policy.qemu_spikes().next() else { return commands };
    for m in crate::hil::MEASUREMENTS.iter().filter(|m| m.kind == "coverage-kernel") {
        let path = crate::hil::kernel_coverage_record_path(&ws.root, m.variant);
        let Ok(text) = std::fs::read_to_string(&path) else {
            report.note(format!("R53.3: no on-target Kernel coverage report at {} (`cargo xtask hil --coverage --kernel`)", ws.rel(&path)));
            continue;
        };
        let k: rsk_hil::coverage::KernelReport = match serde_json::from_str(&text) {
            Ok(k) => k,
            Err(e) => {
                report.error(Rule::Evidence, None, format!("{}: not a Kernel coverage report: {e}", ws.rel(&path)));
                continue;
            }
        };
        let mut notes = vec![k.summary()];
        notes.extend(k.files.iter().map(|f| format!("{}: lines {} ({} of {}), branch outcomes {} ({} of {})", f.file, rsk_hil::coverage::KernelReport::pct(f.lines_covered, f.lines), f.lines_covered, f.lines, rsk_hil::coverage::KernelReport::pct(f.branches_covered, f.branches), f.branches_covered, f.branches)));
        for n in &notes {
            report.note(format!("R53.3 (Kernel, {}): {n}", m.variant));
        }
        let mut item = Item::new(
            "coverage-kernel-target",
            Kind::Measurement,
            &format!("Structural coverage of the Kernel on the Target (R53.1, R53.3): the Kernel crates instrumented in the Kernel coverage build `{}` (the enlarged-Kernel Target, the lean HIL build), its counters read over the probe after the plain measurement run, llvm-cov's totals over the Kernel sources; from the committed report {} (coverage build {})", m.variant, ws.rel(&path), k.binary_hash),
            Tool { name: "rsk-hil coverage (llvm-cov)".to_string(), version: crate::hil::COVERAGE_TOOLCHAIN.to_string(), solvers: vec![] },
            Verdict::from_bool(k.totals.lines > 0),
        )
        .covers(&["R53.1", "R53.3"]);
        item.notes = notes;
        item.build_manifest = Some(manifest_id.to_string());
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    // (c) The Partitions on the Target: the committed reports of the
    // measurement configurations and of the fault-injection suites
    // (`--coverage --response`, `--coverage --isolation`), whose inputs
    // reach the refusals, responses, and restarts the measurements do not.
    let mut variants: Vec<(&str, &str)> = crate::hil::MEASUREMENTS
        .iter()
        .filter(|m| m.duration_secs < 10 * 60 && m.kind != "coverage-kernel")
        .map(|m| (m.variant, match m.kind { "stress" => " --stress", "measure" => " --plain", _ => "" }))
        .collect();
    variants.push(("nrf52840_response", " --response"));
    variants.push(("nrf52840_isolation", " --isolation"));
    for (variant, flag) in variants {
        let path = crate::hil::coverage_record_path(&ws.root, variant);
        let Ok(text) = std::fs::read_to_string(&path) else {
            report.note(format!("R53.3: no on-target coverage report for `{}` at {} (`cargo xtask hil --coverage{flag}`)", variant, ws.rel(&path)));
            continue;
        };
        let rep: serde_json::Value = match serde_json::from_str(&text) {
            Ok(v) => v,
            Err(e) => {
                report.error(Rule::Evidence, None, format!("{}: not JSON: {e}", ws.rel(&path)));
                continue;
            }
        };
        let threshold = rep["threshold_percent"].as_u64().unwrap_or(0);
        let items: Vec<&serde_json::Value> = rep["items"].as_array().into_iter().flatten().collect();
        let ok = !items.is_empty() && items.iter().all(|i| i["percent"].as_u64().unwrap_or(0) >= threshold);
        let mut notes: Vec<String> = Vec::new();
        // Per crate (level A, B, C): the branch outcomes of every function of the crate.
        let mut per_crate: std::collections::BTreeMap<String, (u64, u64)> = std::collections::BTreeMap::new();
        for f in rep["functions"].as_array().into_iter().flatten() {
            let name = f["name"].as_str().unwrap_or("");
            let krate = name.split("::").next().unwrap_or("").to_string();
            let e = per_crate.entry(krate).or_insert((0, 0));
            e.0 += f["branches"].as_u64().unwrap_or(0);
            e.1 += f["covered"].as_u64().unwrap_or(0);
        }
        let model = crate::hil::task_model(ws, &bin, variant);
        for (krate, (b, c)) in &per_crate {
            let level = model.as_ref().and_then(|mm| mm.partitions.iter().find(|p| p.krate == *krate)).map(|p| p.level.clone()).unwrap_or_else(|| "?".into());
            notes.push(format!("{krate} (level {level}): {} of {} branch outcomes ({} %)", c, b, if *b == 0 { 100 } else { c * 100 / b }));
        }
        for i in items.iter().filter(|i| !i["item"].as_str().unwrap_or("").contains("#cs:")) {
            notes.push(format!("{}: {} % ({} of {} branch outcomes){}", i["item"].as_str().unwrap_or(""), i["percent"].as_u64().unwrap_or(0), i["covered"].as_u64().unwrap_or(0), i["branches"].as_u64().unwrap_or(0), if i["percent"].as_u64().unwrap_or(0) >= threshold { "" } else { " BELOW the threshold" }));
        }
        for n in &notes {
            report.note(format!("R53.3 ({}): {n}", variant));
        }
        let mut item = Item::new(
            &format!("coverage-partitions-{}", variant),
            Kind::Measurement,
            &format!("Structural coverage of the Partition code (levels A, B, C) on the Target under the `{}` configuration's inputs (R34.4, R53.3): the coverage build's branch outcomes per crate and per Task entry against the plan's {threshold} % threshold, from the committed report {} (coverage build {})", variant, ws.rel(&path), rep["binary_hash"].as_str().unwrap_or("?")),
            Tool { name: "rsk-hil coverage (llvm-cov)".to_string(), version: crate::hil::COVERAGE_TOOLCHAIN.to_string(), solvers: vec![] },
            Verdict::from_bool(ok),
        )
        .covers(&["R34.4", "R53.3"]);
        item.notes = notes;
        item.build_manifest = Some(manifest_id.to_string());
        if let Err(e) = store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }
    commands
}
