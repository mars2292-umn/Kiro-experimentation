//! `cargo xtask`: the rsk Build_System command line.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use xtask::diag::{Report, Rule};
use xtask::sandbox::{probe_network, Mode, ProbeResult};
use xtask::verify::{self, Options, Proofs};

const USAGE: &str = "\
usage: cargo xtask <command>

commands:
  verify [--keep-target] [--network-sandbox required|off] [--proofs required|skip]
         [--evidence-dir <dir>]
      The verification job (CI entry point): static checks, network sandbox,
      Target and host builds, the Build_Manifest, post-build checks with the
      dependency report, the R58.4 probe fixture, the tests, the
      Kernel_Proofs (Verus), the Kani_Harnesses, and one Evidence_Item per
      step. --keep-target reuses target/xtask-verify instead of starting
      from an empty one. --network-sandbox off runs without network denial,
      which the job reports as a warning. --proofs skip runs without Verus
      and Kani (a warning; no proof evidence). --evidence-dir chooses where
      the manifest and the items are written (default
      target/xtask-verify/evidence). Verus is located through RSK_VERUS_DIR
      or a `cargo-verus` on PATH; Kani through `cargo kani`.
  check [--root <dir>] [--workspace-only]
      Static checks only. --workspace-only skips the repository-level checks
      (toolchain pin, CI workflows, alias), as the fixture tests do.
  rebuild-check
      R59.3: rebuild the Flight_Build binaries from a clean copy of the tree
      and compare their binary hashes with the build in target/xtask-verify.
  profile [--check]
      Check profile/profile.toml and profile/core_subset.toml against
      Requirements 1, 2, 4, 27.3, and 57; record the content hash of the
      current version; generate docs/profile.md and the profile_version.rs
      modules. --check only compares the generated files with the tree.
  confcheck-mutations <task_model.json> <flight-build.elf>
      R26.7: apply each Config_Checker mutation of docs/config_mutations.toml
      to a copy of the Task_Model or the binary and require rsk-confcheck
      to report the altered item.
  mutations [--check] [--only <id>]
      R52.2: apply each proof mutation of docs/proof_mutations.toml to a copy
      of the tree and require Verus to reject it with the registered
      diagnostic (a surviving mutant is a vacuous specification). --check
      only validates the register against the sources, without Verus.
  gen [--check]
  uppaal-corpus
  nightly
      The nightly HIL campaign (R50.1, Verification_Plan section 11): the
      verification job, then the hour-long release-timing and stress runs
      on the rig, each as its own command; the log and a summary go under
      target/xtask-nightly/<date>/. Needs the boards attached and the
      machine on AC power; xtask/launchd/com.ironoxide.nightly.plist
      schedules it at 02:00.
  hil [--bin <crate>] [--timeout <s>] [--spike-only | --release-timing [--short] [--defer-probe | --defer-nofix] | --stress [--short] | --response | --isolation | --boot-checks | --cycles | --coverage [--stress | --plain | --kernel | --response | --isolation]] [--lean]
      --lean: the HIL build without the trace ring and the trace pins
      (feature record-only; task 19.2, R34.7), its records written as
      `<variant>_lean` beside the instrumented ones, never used by the job.
      Run the Generator (rsk-gen) on every declaration crate of the
      workspace (policy `declaration = true`) and write the Task_Model,
      the linker script and memory map, the configuration summary and the
      manifests into the application's gen/ directory (R24.3, R25.1).
      --check only compares them with the tree (R24.4).
  qemu [--bin <name>] [--timeout <seconds>] [--flood]
      Task 3.1 dispatch spike: build the Flight_Build binary (default
      p1-demo) for the Target and run it on QEMU's mps2-an386 Cortex-M4
      board with semihosting; the exit status is the binary's semihosting
      exit code (0 = the demo's safe-state report found every expectation
      met). Output is printed and saved under target/qemu/.
  trace [--check] [--evidence-dir <dir>]
      Write docs/traceability_matrix.md: every acceptance criterion with its
      design, tasks, allocation rows, code, and Evidence_Items (R50.5), the
      criteria without an Evidence_Item (R50.6), and the per-criterion list
      of R43.5. The evidence store defaults to target/xtask-verify/evidence.
  conformance [--only <case id>]
      Run the Profile_Conformance_Suite (docs/conformance_suite.toml) against
      the P1 demo built into target/conformance (R3.6, R3.7, R3.11).
  probe-network
      Attempt a connection to 192.0.2.1:9 and report whether it was denied
      (exit status 0 if denied). Used inside the sandbox by `verify`.";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let (command, rest) = match args.split_first() {
        Some((command, rest)) => (command.as_str(), rest),
        None => ("help", &[][..]),
    };
    match command {
        "verify" => verify_command(rest),
        "check" => check_command(rest),
        "rebuild-check" => rebuild_command(),
        "profile" => profile_command(rest),
        "mutations" => mutations_command(rest),
        "confcheck-mutations" => confcheck_mutations_command(rest),
        "conformance" => conformance_command(rest),
        "trace" => trace_command(rest),
        "gen" => gen_command(rest),
        "uppaal-corpus" => uppaal_corpus_command(rest),
        "qemu" => qemu_command(rest),
        "hil" => hil_command(rest),
        "nightly" => nightly_command(rest),
        "probe-network" => match probe_network() {
            ProbeResult::Denied(detail) => {
                println!("denied: {detail}");
                ExitCode::SUCCESS
            }
            ProbeResult::NotDenied(detail) => {
                println!("not denied: {detail}");
                ExitCode::FAILURE
            }
        },
        "help" | "--help" | "-h" => {
            println!("{USAGE}");
            ExitCode::SUCCESS
        }
        other => {
            eprintln!("unknown command `{other}`\n\n{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn verify_command(args: &[String]) -> ExitCode {
    let mut options = Options {
        keep_target: false,
        sandbox: Mode::Required,
        proofs: Proofs::Required,
        evidence_dir: None,
    };
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--keep-target" => options.keep_target = true,
            "--network-sandbox" => match args.next().map(String::as_str) {
                Some("required") => options.sandbox = Mode::Required,
                Some("off") => options.sandbox = Mode::Off,
                _ => return usage_error("--network-sandbox takes `required` or `off`"),
            },
            "--proofs" => match args.next().map(String::as_str) {
                Some("required") => options.proofs = Proofs::Required,
                Some("skip") => options.proofs = Proofs::Skip,
                _ => return usage_error("--proofs takes `required` or `skip`"),
            },
            "--evidence-dir" => match args.next() {
                Some(dir) => options.evidence_dir = Some(PathBuf::from(dir)),
                None => return usage_error("--evidence-dir takes a directory"),
            },
            other => return usage_error(&format!("unknown option `{other}`")),
        }
    }
    if verify::run(&xtask::repository_root(), options) {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}

fn check_command(args: &[String]) -> ExitCode {
    let mut root = xtask::repository_root();
    let mut workspace_only = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--root" => match args.next() {
                Some(dir) => root = PathBuf::from(dir),
                None => return usage_error("--root takes a directory"),
            },
            "--workspace-only" => workspace_only = true,
            other => return usage_error(&format!("unknown option `{other}`")),
        }
    }
    let report = if workspace_only {
        xtask::check_workspace(&root)
    } else {
        xtask::check_repository(&root)
    };
    print_report(&report, "static checks passed")
}

fn rebuild_command() -> ExitCode {
    let root = xtask::repository_root();
    let mut report = Report::default();
    let ok = xtask::rebuild::run(&root, &root.join("target").join("xtask-verify"), &mut report);
    let code = print_report(&report, "rebuild check passed");
    if ok {
        code
    } else {
        ExitCode::FAILURE
    }
}

fn profile_command(args: &[String]) -> ExitCode {
    let check_only = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return usage_error("profile takes at most `--check`"),
    };
    let mut report = Report::default();
    let ok = xtask::profile_gen::generate(&xtask::repository_root(), check_only, &mut report);
    let code = print_report(&report, if check_only { "profile is current" } else { "profile generated" });
    if ok {
        code
    } else {
        ExitCode::FAILURE
    }
}

fn confcheck_mutations_command(args: &[String]) -> ExitCode {
    let [model, elf] = args else {
        return usage_error("confcheck-mutations takes <task_model.json> <flight-build.elf>");
    };
    let mut report = Report::default();
    let root = xtask::repository_root();
    let work = root.join("target").join("xtask-confcheck-mutations");
    let ok = match xtask::confcheck_mutations::run(&root, std::path::Path::new(model), std::path::Path::new(elf), &work, None, &mut report) {
        Ok(ok) => ok,
        Err(e) => {
            report.error(Rule::Step, None, e);
            false
        }
    };
    let code = print_report(&report, "every Config_Checker mutant was reported");
    if ok {
        code
    } else {
        ExitCode::FAILURE
    }
}

/// `cargo xtask conformance [--only <case id>]`: builds the suite's binary
/// for the Target into target/conformance and runs the
/// Profile_Conformance_Suite (R3.6, R3.7, R3.11), writing its Evidence_Items
/// to target/conformance/evidence.
fn conformance_command(args: &[String]) -> ExitCode {
    let mut only: Option<String> = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--only" => match args.next() {
                Some(id) => only = Some(id.clone()),
                None => return usage_error("--only takes a case id"),
            },
            other => return usage_error(&format!("unknown conformance option `{other}`")),
        }
    }
    let root = xtask::repository_root();
    let mut report = Report::default();
    let Some(ws) = xtask::workspace::Workspace::load(&root, &mut report) else {
        for d in &report.diagnostics {
            eprintln!("{d}");
        }
        return ExitCode::FAILURE;
    };
    let target_dir = root.join("target").join("conformance");
    let build = xtask::cmd::Cmd::cargo_frozen("build")
        .args(["--release", "--target", &ws.policy.target, "-p", "p1-demo"])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .env("CARGO_ENCODED_RUSTFLAGS", xtask::verify::target_rustflags(&root));
    println!("$ {}", build.display());
    if !build.status(None).unwrap_or(false) {
        eprintln!("the Target build failed");
        return ExitCode::FAILURE;
    }
    let revision = xtask::tools::Revision::of(&root);
    let profile_version = xtask::profile_gen::load(&root).map(|(p, _)| p.version).unwrap_or_default();
    let mut store = xtask::evidence::Store::new(&target_dir.join("evidence"), revision, &profile_version);
    let mut cx = xtask::conformance::Context { root: &root, ws: &ws, target_dir: &target_dir, sandbox: None, store: &mut store, manifest_id: None, only };
    let ok = match xtask::conformance::run(&mut cx, &mut report) {
        Ok((_, ok)) => ok,
        Err(e) => {
            eprintln!("conformance: {e}");
            false
        }
    };
    for n in &report.notes {
        println!("note: {n}");
    }
    for d in &report.diagnostics {
        println!("{d}");
    }
    println!("Evidence_Items in {}", target_dir.join("evidence").display());
    if ok && report.diagnostics.is_empty() { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

fn trace_command(args: &[String]) -> ExitCode {
    let mut check_only = false;
    let mut evidence_dir: Option<PathBuf> = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--evidence-dir" => match args.next() {
                Some(d) => evidence_dir = Some(PathBuf::from(d)),
                None => return usage_error("--evidence-dir takes a directory"),
            },
            other => return usage_error(&format!("unknown trace option `{other}`")),
        }
    }
    let root = xtask::repository_root();
    let evidence_dir = evidence_dir.unwrap_or_else(|| root.join("target").join("xtask-verify").join("evidence"));
    let mut report = Report::default();
    match xtask::trace::run(&root, &evidence_dir, check_only, &mut report) {
        Ok(_) => print_report(&report, if check_only { "traceability matrix is current" } else { "traceability matrix written" }),
        Err(e) => {
            eprintln!("trace: {e}");
            ExitCode::FAILURE
        }
    }
}

fn mutations_command(args: &[String]) -> ExitCode {
    let mut check_only = false;
    let mut only: Option<String> = None;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--check" => check_only = true,
            "--only" => match args.next() {
                Some(id) => only = Some(id.clone()),
                None => return usage_error("--only takes a mutation id"),
            },
            other => return usage_error(&format!("unknown mutations option `{other}`")),
        }
    }
    let mut report = Report::default();
    let root = xtask::repository_root();
    let ok = if check_only {
        xtask::mutations::check(&root, &mut report);
        report.diagnostics.is_empty()
    } else {
        xtask::mutations::run(&root, only.as_deref(), &mut report)
    };
    let code = print_report(&report, if check_only { "proof mutation register is current" } else { "every proof mutant was killed" });
    if ok {
        code
    } else {
        ExitCode::FAILURE
    }
}

fn gen_command(args: &[String]) -> ExitCode {
    let check_only = match args {
        [] => false,
        [flag] if flag == "--check" => true,
        _ => return usage_error("gen takes at most `--check`"),
    };
    let mut report = Report::default();
    let ok = xtask::gen::run(&xtask::repository_root(), check_only, &mut report);
    let code = print_report(&report, if check_only { "generated files are current" } else { "generated" });
    if ok {
        code
    } else {
        ExitCode::FAILURE
    }
}

/// `cargo xtask uppaal-corpus`: regenerates `docs/uppaal_corpus_report.md`
/// (R32.6, PAR-03), running UPPAAL's verifier where installed.
fn uppaal_corpus_command(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        return usage_error("uppaal-corpus takes no arguments");
    }
    let root = xtask::repository_root();
    let mut report = Report::default();
    let Some(ws) = xtask::workspace::Workspace::load(&root, &mut report) else {
        return print_report(&report, "corpus");
    };
    let target_dir = root.join("target").join("xtask-corpus");
    match xtask::corpus::run(&ws, &target_dir, &xtask::corpus::work_dir(&target_dir), None, &mut report) {
        Ok(outcome) => {
            let path = xtask::corpus::report_path(&root);
            if let Err(e) = std::fs::write(&path, &outcome.report) {
                report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", path.display()));
            } else {
                report.note(format!("{} models; statuses {:?}; report written to docs/uppaal_corpus_report.md", outcome.rows.len(), outcome.by_status));
            }
        }
        Err(e) => report.error(xtask::diag::Rule::Step, None, e),
    }
    print_report(&report, "UPPAAL corpus report written")
}

/// `cargo xtask hil`: the HIL_Rig self-test (task 7.1): builds the HIL
/// variants of the demo and the stimulus Application, flashes the
/// stimulus board, runs the demo on the target with the analyzer
/// capturing, and prints the self-test report. `--spike-only` builds and
/// runs one binary on the target alone and prints its console (task 3.1's
/// hardware half); without `hil-rig.toml` that is all there is.
fn hil_command(args: &[String]) -> ExitCode {
    let mut bin = "p1-demo".to_string();
    let mut timeout = 60u64;
    let mut spike_only = false;
    let mut release_timing = false;
    let mut stress = false;
    let mut response = false;
    let mut isolation = false;
    let mut boot_checks = false;
    let mut cycles = false;
    let mut coverage = false;
    let mut lean = false;
    let mut short = false;
    let mut plain = false;
    let mut fpu = false;
    let mut kernel = false;
    let mut defer_probe = false;
    let mut defer_nofix = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => match args.next() {
                Some(b) => bin = b.clone(),
                None => return usage_error("--bin takes a crate name"),
            },
            "--timeout" => match args.next().and_then(|t| t.parse().ok()) {
                Some(t) => timeout = t,
                None => return usage_error("--timeout takes seconds"),
            },
            "--spike-only" => spike_only = true,
            "--release-timing" => release_timing = true,
            "--stress" => stress = true,
            "--response" => response = true,
            "--isolation" => isolation = true,
            "--boot-checks" => boot_checks = true,
            "--cycles" => cycles = true,
            "--coverage" => coverage = true,
            "--lean" => lean = true,
            "--short" => short = true,
            "--plain" => plain = true,
            "--fpu" => fpu = true,
            "--kernel" => kernel = true,
            "--defer-probe" => defer_probe = true,
            "--defer-nofix" => {
                defer_probe = true;
                defer_nofix = true;
            }
            other => return usage_error(&format!("unknown option `{other}`")),
        }
    }
    let root = xtask::repository_root();
    let mut report = Report::default();
    let Some(ws) = xtask::workspace::Workspace::load(&root, &mut report) else {
        return print_report(&report, "hil");
    };
    let rig = match xtask::hil::rig(&root) {
        Some(Ok(rig)) => Some(rig),
        Some(Err(e)) => {
            report.error(xtask::diag::Rule::Step, Some(rsk_hil::config::FILE.into()), e);
            return print_report(&report, "hil");
        }
        None => None,
    };
    let target_dir = xtask::hil::target_dir(&root);
    if spike_only || rig.is_none() {
        if rig.is_none() {
            report.note(format!("no {}: single-board run", rsk_hil::config::FILE));
        }
        let Some(probe) = xtask::hil::probe(rig.as_ref()) else {
            report.error(xtask::diag::Rule::Step, None, "no debug probe for the target: probe-rs is not installed, or `probe-rs list` shows none, several without a rig, or not the rig's target");
            return print_report(&report, "hil");
        };
        report.note(format!("{} ; {}", probe.version, probe.probe));
        let Some(build) = xtask::hil::build(&ws, &bin, &target_dir, None, &mut report) else {
            return print_report(&report, "hil");
        };
        let Some(serial) = xtask::hil::target_serial(rig.as_ref()) else {
            return print_report(&report, "hil");
        };
        let run = xtask::hil::run_build(&build, &serial, timeout);
        print!("{}", run.log);
        if run.ok {
            report.note(format!("HIL run of `{bin}` on {}: RSK-OK after {:.1}s", xtask::hil::CHIP, run.seconds));
        } else {
            report.error(xtask::diag::Rule::Step, None, format!("HIL run of `{bin}` failed after {:.1}s (console above)", run.seconds));
        }
        return print_report(&report, "HIL run passed");
    }
    let rig = rig.expect("a rig");
    let Some(stimulus_bin) = rig.stimulus.bin.clone() else {
        return print_report(&report, "hil");
    };
    let kind = if stress { "stress" } else if plain { "measure" } else if kernel && coverage { "coverage-kernel" } else { "release-timing" };
    if coverage && response {
        return response_command(&ws, &rig, &bin, &stimulus_bin, true, &target_dir, report);
    }
    if coverage && isolation {
        return isolation_command(&ws, &rig, &bin, true, &target_dir, report);
    }
    if coverage {
        return coverage_command(&ws, &rig, &bin, &stimulus_bin, kind, &target_dir, report);
    }
    if release_timing || stress || plain {
        let probe = if defer_nofix { Some("nofix") } else if defer_probe { Some("probe") } else { None };
        return release_timing_command(&ws, &rig, &bin, &stimulus_bin, kind, short || plain, lean, probe, &target_dir, report);
    }
    if response {
        return response_command(&ws, &rig, &bin, &stimulus_bin, false, &target_dir, report);
    }
    if isolation {
        return isolation_command(&ws, &rig, &bin, false, &target_dir, report);
    }
    if boot_checks {
        return boot_checks_command(&ws, &rig, &bin, &target_dir, report);
    }
    if cycles {
        return cycles_command(&ws, &rig, &bin, lean, fpu, &target_dir, report);
    }
    let Some(target) = xtask::hil::build(&ws, &bin, &target_dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    let Some(stimulus) = xtask::hil::build(&ws, &stimulus_bin, &target_dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    let work = target_dir.join("rig");
    let inputs = rsk_hil::selftest::Inputs {
        rig: &rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        target_hash: Some(&target.binary.hash),
        stimulus_hash: Some(&stimulus.binary.hash),
        work_dir: &work,
        timeout_secs: timeout,
        tick_hz: xtask::hil::TICK_HZ,
    };
    let out = rsk_hil::selftest::run(&inputs);
    let text = rsk_hil::selftest::render(&rig, &out);
    print!("{text}");
    let _ = std::fs::write(work.join("self-test.md"), &text);
    let _ = std::fs::write(work.join("edges.txt"), &out.edges_text);
    let _ = std::fs::write(work.join("trace.txt"), &out.trace_text);
    let _ = std::fs::write(work.join("events.txt"), &out.events_text);
    report.note(format!("report and edge list written under {}", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the HIL_Rig self-test failed (report above)");
    }
    print_report(&report, "HIL_Rig self-test passed")
}

/// `cargo xtask hil --release-timing [--short]`: the release-timing run of
/// task 7.3 (R9.10) over the Application's release-timing configuration
/// (`timing`: 61 minutes, PAR-05; `--short`: `timing-short`, 3 minutes,
/// the pipeline check). The report goes under target/xtask-hil/release-timing
/// and, for the record, docs/hil/.
///
/// `cargo xtask hil --stress [--short]` (task 7.5, R18.8, R19.3): the same
/// run over the stress configuration (`stress`: 61 minutes; `--short`: 3
/// minutes), the stimulus board built with its `storm` feature, and the
/// protected Partition's Jobs checked against the WCRTs the Analyzer
/// computes for the configuration's Task_Model (placeholder timing).
///
/// `--defer-probe` and `--defer-nofix` (Verification_Plan 5.8a): the
/// witness builds of the INVPC fix, the dispatch stub's return window
/// (the entry instruction and the return) widened to about 30 µs each,
/// with the fence and the deferral (`probe`: the run must record
/// deferrals and no fault) and without them (`nofix`: the windows fault as
/// the unfixed Kernel did, one INVPC per entry). Their
/// reports go to `docs/hil/findings/`; no record is written from them and
/// the timing bounds are not judged.
#[allow(clippy::too_many_arguments)]
fn release_timing_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, stimulus_bin: &str, kind: &str, short: bool, lean: bool, probe: Option<&str>, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let Some(measurement) = xtask::hil::MEASUREMENTS.iter().find(|m| m.kind == kind && (m.duration_secs < 10 * 60) == short) else {
        report.error(xtask::diag::Rule::Step, None, "no measurement configuration for this run".to_string());
        return print_report(&report, "hil");
    };
    let stress = measurement.stress;
    let (feature, duration_secs, variant_name) = (measurement.feature, measurement.duration_secs, measurement.variant);
    let probe_features = match probe {
        Some("nofix") => ",defer-probe,defer-nofix",
        Some(_) => ",defer-probe",
        None => "",
    };
    let features = format!("{},{feature}{probe_features}", xtask::hil::features_for(lean));
    // The lean run's reports and records carry the `_lean` suffix: beside
    // the instrumented ones, never taken by the job (task 19.2, R34.7).
    let variant_label = match probe {
        Some(p) => format!("{variant_name}_defer_{p}"),
        None if lean => format!("{variant_name}_lean"),
        None => variant_name.to_string(),
    };
    let variant = variant_label.as_str();
    let dir = target_dir.join(kind).join(variant);
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    let stimulus = if stress {
        xtask::hil::build_with(ws, stimulus_bin, &format!("{},storm", xtask::hil::FEATURES), &target_dir.join("stimulus-storm"), None, &mut report)
    } else {
        xtask::hil::build(ws, stimulus_bin, target_dir, None, &mut report)
    };
    let Some(stimulus) = stimulus else {
        return print_report(&report, "hil");
    };
    report.note(format!("{kind} configuration `{variant}` ({features}): {} over {} loadable bytes; {} s", target.binary.hash, target.binary.loadable_bytes, duration_secs));
    let model = ws.root.join("apps").join(bin).join(format!("gen-{variant_name}")).join("task_model.json");
    let stress_spec = if stress {
        match analyzer_wcrt(ws, &model, target.binary.config_checksum.as_deref(), &dir, &mut report) {
            Some((protected, timing_used)) => {
                for (t, name, r) in &protected {
                    report.note(format!("Analyzer WCRT of Task {t} ({name}): {r} cycles ({:.3} ms), {}", *r as f64 * 1000.0 / xtask::hil::CPU_HZ as f64, timing_used));
                }
                Some(rsk_hil::release_timing::Stress { protected, storm_task: 4, mit_ms: 15 })
            }
            None => return print_report(&report, "hil"),
        }
    } else {
        None
    };
    let work = dir.join("run");
    // Task 19.2: the Profile_Lint's loop-bound export, which the rig judges
    // the build's loop report against (R34.9, R35.4).
    let loop_bounds = xtask::hil::loop_bounds_export(ws, bin, &dir.join("loop-bounds.json"), &mut report);
    if loop_bounds.is_none() {
        return print_report(&report, "hil");
    }
    let inputs = rsk_hil::release_timing::Inputs {
        rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        work_dir: &work,
        duration_secs,
        grace_secs: 90,
        tick_hz: xtask::hil::TICK_HZ,
        cpu_hz: xtask::hil::CPU_HZ,
        samplerate_hz: 100_000,
        flash_stimulus: true,
        stress: stress_spec,
        loop_bounds: loop_bounds.as_deref(),
        configuration: variant,
        binary_hash: &target.binary.hash,
        timing_bounds: probe.is_none(),
        expect_deferrals: probe == Some("probe"),
    };
    let out = rsk_hil::release_timing::run(&inputs);
    let text = rsk_hil::release_timing::render(rig, &inputs, &out);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let _ = std::fs::write(work.join("trace.txt"), rsk_hil::trace::render(&out.trace, xtask::hil::TICK_HZ));
    if let Some(ev) = &out.events {
        let _ = std::fs::write(work.join("events.txt"), ev.render(xtask::hil::TICK_HZ));
    }
    if let Some(p) = probe {
        // The witness runs of 5.8a: the report and trace are findings, not records.
        let findings = ws.root.join("docs").join("hil").join("findings").join(format!("defer_{p}_{variant_name}"));
        let _ = std::fs::create_dir_all(&findings);
        let _ = std::fs::write(findings.join("report.md"), &text);
        let _ = std::fs::write(findings.join("trace.txt"), rsk_hil::trace::render(&out.trace, xtask::hil::TICK_HZ));
        report.note(format!("witness run `{variant}` ({features}): report and trace under {}", ws.rel(&findings)));
        if !out.ok() {
            report.error(xtask::diag::Rule::Step, None, format!("the {kind} witness run failed (report above)"));
            return print_report(&report, "hil");
        }
        return print_report(&report, &format!("{kind} witness run passed"));
    }
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let name = format!("{}_{variant}.md", kind.replace('-', "_"));
    let _ = std::fs::write(record.join(&name), &text);
    report.note(format!("report written to {} and docs/hil/{name}", ws.rel(&work)));
    // Task 19.2: the loop-iteration report of the run (R34.9, R35.4), kept
    // beside the records whatever the verdict (a failed run's is evidence
    // of what exceeded its bound).
    if let Some(l) = &out.loops {
        let path = xtask::hil::loop_record_path(&ws.root, variant);
        let _ = std::fs::write(work.join("loop_iterations.json"), l.print());
        match std::fs::write(&path, l.print()) {
            Ok(()) => report.note(format!("loop iterations written to {}: {}", ws.rel(&path), l.summary())),
            Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&path))),
        }
    }
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, format!("the {kind} run failed (report above)"));
        return print_report(&report, "hil");
    }
    // Task 19.3: the Kernel_Timing_Parameter record of the passed run
    // (J_rel; Δ_irq under the storm), bound to this HIL build, folded into
    // the committed record of the configuration.
    let task_model = std::fs::read_to_string(&model).ok().and_then(|t| rsk_model::read_task_model(&t).ok());
    let target_name = task_model.as_ref().map(|m| format!("{}/{}", m.target, m.core_revision)).unwrap_or_else(|| "nrf52840/?".to_string());
    let binding = rsk_hil::kernel_timing::Binding { binary_hash: &target.binary.hash, target: &target_name, icache: "off", configuration: variant };
    match rsk_hil::kernel_timing::record(&inputs, &out, &binding, xtask::hil::MARGIN_PERCENT) {
        Some(new) => {
            let _ = std::fs::write(work.join("kernel_timing.json"), rsk_model::print(&new));
            let path = xtask::hil::timing_record_path(&ws.root, variant);
            let kept = std::fs::read_to_string(&path).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok());
            let (folded, replaced) = rsk_hil::kernel_timing::fold(kept, new);
            match std::fs::write(&path, rsk_model::print(&folded)) {
                Ok(()) => {
                    let what: Vec<String> = folded.parameters.iter().map(|p| p.provenance()).collect();
                    report.note(format!(
                        "Kernel_Timing_Parameters {} {} (margin {} %, R34.6): {}",
                        if replaced { "written to" } else { "folded into" },
                        ws.rel(&path),
                        xtask::hil::MARGIN_PERCENT,
                        what.join("; ")
                    ));
                }
                Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&path))),
            }
        }
        None => report.note("no Kernel_Timing_Parameter record: the run recorded no Release".to_string()),
    }
    // Task 19.1: the WCET_Records of the run (Task entries from the Budget
    // accounting, Critical_Section sites), folded into the committed set.
    match task_model.as_ref().and_then(|m| rsk_hil::wcet_records::record(&inputs, &out, m, &binding, xtask::hil::MARGIN_PERCENT)) {
        Some(new) => {
            let _ = std::fs::write(work.join("wcet.json"), rsk_model::print(&new));
            let path = xtask::hil::wcet_record_path(&ws.root, variant);
            let kept = std::fs::read_to_string(&path).ok().and_then(|t| rsk_model::read_wcet_set(&t).ok());
            let (folded, replaced) = rsk_hil::wcet_records::fold(kept, new);
            match std::fs::write(&path, rsk_model::print(&folded)) {
                Ok(()) => {
                    let what: Vec<String> = folded.records.iter().map(|r| r.provenance()).collect();
                    report.note(format!(
                        "WCET_Records {} {} ({} items, margin {} %, R34.6): {}",
                        if replaced { "written to" } else { "folded into" },
                        ws.rel(&path),
                        folded.records.len(),
                        xtask::hil::MARGIN_PERCENT,
                        what.join("; ")
                    ));
                }
                Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&path))),
            }
        }
        None => report.note("no WCET_Record: the run recorded no Job execution (or no Task_Model)".to_string()),
    }
    // Task 19.4: the Analyzer on this run's records, and every Task's
    // observed response time against the WCRT it computes.
    let (ok, section) = measured_wcrt_step(ws, bin, variant_name, kind, &target, &model, &dir, &out, &mut report);
    let text = format!("{text}\n{section}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let _ = std::fs::write(record.join(&name), &text);
    print!("{section}");
    if !ok {
        report.error(xtask::diag::Rule::Step, None, "observed response times against the measured WCRTs (task 19.4): failed (report above)".to_string());
        return print_report(&report, "hil");
    }
    print_report(&report, &format!("{kind} run passed"))
}

/// Task 19.4 (R18.7, R29.5): the Analyzer run on the records this run
/// wrote (its Kernel_Timing_Parameters and WCET_Records, with the cycle
/// configurations' records when the job's manifest lists them), then each
/// Task's largest observed response time (the Kernel's record, Release
/// due to Job end) against the WCRT the Analyzer computed. A Task the
/// analysis finds unschedulable has no WCRT to hold; the step passes when
/// every schedulable Task observed stays within its WCRT and, for the
/// plain configuration, the analysis is a PASS; under the stress
/// configuration the Tasks of the highest Criticality_Level must be
/// schedulable and within (R18.7). Returns the verdict and the report's
/// section.
#[allow(clippy::too_many_arguments)]
fn measured_wcrt_step(ws: &xtask::workspace::Workspace, _bin: &str, variant: &str, kind: &str, target: &xtask::hil::HilBuild, model: &std::path::Path, dir: &std::path::Path, out: &rsk_hil::release_timing::Outcome, report: &mut Report) -> (bool, String) {
    let mut s = String::from("## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)\n\n");
    let fail = |s: &mut String, why: &str| {
        s.push_str(&format!("Not judged: {why}\n"));
        (false, s.clone())
    };
    let Some(st) = &out.stats else { return fail(&mut s, "the run has no record") };
    let timing = xtask::hil::timing_record_path(&ws.root, variant);
    let wcet = xtask::hil::wcet_record_path(&ws.root, variant);
    let bound_to_this = |path: &std::path::Path| -> bool {
        std::fs::read_to_string(path).ok().map(|t| t.contains(&target.binary.hash)).unwrap_or(false)
    };
    if !bound_to_this(&timing) || !bound_to_this(&wcet) {
        return fail(&mut s, "the records are not this build's");
    }
    let job_manifest = ws.root.join("target").join("xtask-verify").join("evidence").join("build-manifest.json");
    let manifest_value: serde_json::Value = std::fs::read_to_string(&job_manifest).ok().and_then(|t| serde_json::from_str(&t).ok()).unwrap_or(serde_json::Value::Null);
    let (fresh, _) = xtask::hil::fresh_timing_records(&ws.root, &manifest_value);
    let extra: Vec<std::path::PathBuf> = fresh
        .into_iter()
        .filter(|r| xtask::hil::RECORD_VARIANTS.iter().any(|(v, _)| r.file_name().and_then(|f| f.to_str()) == Some(&format!("kernel_timing_{v}.json"))))
        .collect();
    if manifest_value.is_null() {
        return fail(&mut s, "no Build_Manifest of the job (run `cargo xtask verify` first)");
    }
    let out_dir = dir.join("rta-measured");
    let _ = std::fs::create_dir_all(&out_dir);
    // The manifest copy is the job's with this run's build among its HIL
    // binaries (the records are bound to it, R33.4) and its Generated_Config
    // checksum (R28.8).
    let mut value = manifest_value.clone();
    value["generated_config_checksum"] = target.binary.config_checksum.clone().map(serde_json::Value::String).unwrap_or(serde_json::Value::Null);
    if let Some(list) = value.get_mut("hil_binaries").and_then(|b| b.as_array_mut()) {
        if !list.iter().any(|b| b.get("hash").and_then(|h| h.as_str()) == Some(target.binary.hash.as_str())) {
            list.push(serde_json::json!({
                "package": target.binary.package,
                "target": target.binary.target_name,
                "file": target.binary.path.file_name().and_then(|f| f.to_str()).unwrap_or("p1-demo"),
                "hash": target.binary.hash,
                "loadable_bytes": target.binary.loadable_bytes,
                "config_checksum": target.binary.config_checksum,
            }));
        }
    }
    let manifest_copy = out_dir.join("build-manifest.json");
    if std::fs::write(&manifest_copy, serde_json::to_string_pretty(&value).unwrap_or_default()).is_err() {
        return fail(&mut s, "cannot write the manifest copy");
    }
    let mut cmd = xtask::cmd::Cmd::cargo_frozen("run")
        .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(ws.root.join("target"))
        .arg("--")
        .arg("--model")
        .arg(model)
        .arg("--manifest")
        .arg(&manifest_copy)
        .arg("--placeholders")
        .arg("--timing")
        .arg(&timing);
    for r in &extra {
        cmd = cmd.arg("--timing").arg(r);
    }
    let cmd = cmd.arg("--wcet").arg(&wcet).arg("--out").arg(&out_dir);
    let (status, text) = match cmd.output(None) {
        Ok(o) => (o.status.code().unwrap_or(3), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => return fail(&mut s, &format!("cannot run the Analyzer: {e}")),
    };
    let measured_line = text.lines().find(|l| l.contains("MEASURED")).map(|l| l.trim().to_string()).unwrap_or_default();
    s.push_str(&format!("The Analyzer on this run's records ({}{}): exit {status}. {measured_line}\n\n", ws.rel(&timing), if extra.is_empty() { String::new() } else { format!(", with {}", extra.iter().map(|e| ws.rel(e)).collect::<Vec<_>>().join(", ")) }));
    if status == 2 {
        let why = text.lines().find(|l| l.contains("[R")).unwrap_or("").trim().to_string();
        // The release-timing configuration's injected 50 ms Jobs put a measured
        // WCET above the Budget by design (11.5, 11.6): its rejection under
        // R28.4 is the configuration's finding, not the run's failure.
        if kind == "release-timing" && why.contains("[R28.4]") {
            s.push_str(&format!("Not judged, as the configuration intends: {why} (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).\n"));
            report.note(format!("task 19.4: the release-timing configuration's measured analysis rejected under R28.4 by design: {why}"));
            return (true, s);
        }
        return fail(&mut s, &format!("the Analyzer rejected the inputs: {why}"));
    }
    let Ok(rep) = std::fs::read_to_string(out_dir.join("report.json")).map_err(|e| e.to_string()).and_then(|t| rsk_model::read_report(&t).map_err(|e| format!("{e:?}"))) else {
        return fail(&mut s, "the Analyzer wrote no report");
    };
    let top = rep.tasks.iter().map(|t| t.level.clone()).min().unwrap_or_default();
    let cycles_per_tick = xtask::hil::CPU_HZ / xtask::hil::TICK_HZ;
    s.push_str(&format!("Result **{}** (utilization {} %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |\n|---|---|---|---|---|---|---|\n", rep.result, rep.utilization_percent));
    let mut all_ok = true;
    for (i, t) in rep.tasks.iter().enumerate() {
        let ts = st.tasks.get(i);
        let (jobs, max_ticks) = ts.map(|x| (x.resp_count, x.resp_max_ticks as u64)).unwrap_or((0, 0));
        let observed = max_ticks * cycles_per_tick;
        let ms = |c: u64| c as f64 * 1000.0 / xtask::hil::CPU_HZ as f64;
        let verdict = if !t.schedulable {
            if t.level == top && kind == "stress" { all_ok = false; "NOT SCHEDULABLE (highest level)" } else { "not schedulable: no WCRT to hold" }
        } else if jobs == 0 {
            "no Job completed"
        } else if observed <= t.r_cycles {
            "within"
        } else {
            all_ok = false;
            "EXCEEDED"
        };
        s.push_str(&format!("| `{}` | {} | {:.3} | {:.3} | {:.3} | {} | {} |\n", t.id, t.level, ms(t.r_cycles), ms(t.deadline_cycles), ms(observed), jobs, verdict));
    }
    if kind == "measure" && rep.result != "PASS" {
        all_ok = false;
        s.push_str("\nThe plain configuration's analysis is not a PASS: the schedulability claim under measured timing does not hold for this set.\n");
    }
    s.push_str(&format!("\nVerdict: **{}**\n", if all_ok { "pass" } else { "FAIL" }));
    report.note(format!("task 19.4: the Analyzer on the measured records: {}; every observed response within its WCRT: {}", rep.result, if all_ok { "yes" } else { "NO" }));
    (all_ok, s)
}

/// `cargo xtask hil --coverage [--stress]` (task 19.2, R34.4, R53.3;
/// Verification_Plan 11.13): the short measurement configuration (the
/// release-timing one, or the stress one with `--stress`) built as the
/// coverage build and run on the rig under the same inputs as the
/// measurement run; afterwards the Partitions' branch counters are read
/// over the probe and the branch coverage of each Task entry (its body
/// and its crate's helpers) and Critical_Section site is judged against
/// the threshold the rig records, written to `docs/hil/coverage_<variant>.json`
/// and `.md`, and entered into the committed WCET_Records of the
/// configuration as their `coverage_branch_percent`. The run writes no
/// timing record: the instrumented bodies are not the measured binary.
fn coverage_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, stimulus_bin: &str, kind: &str, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let Some(measurement) = xtask::hil::MEASUREMENTS.iter().find(|m| m.kind == kind && m.duration_secs < 10 * 60) else {
        report.error(xtask::diag::Rule::Step, None, "no short measurement configuration for this run".to_string());
        return print_report(&report, "hil");
    };
    let stress = measurement.stress;
    let (feature, duration_secs, variant) = (measurement.feature, measurement.duration_secs, measurement.variant);
    let Some(tools) = xtask::hil::coverage_tools(&mut report) else {
        return print_report(&report, "hil");
    };
    // The Kernel coverage build (task 19.5): the HIL build with the Kernel
    // crates instrumented too, on the enlarged-Kernel Target (its RAM holds
    // the counters beside the records).
    let kernel = kind == "coverage-kernel";
    let features = format!("{},{feature},coverage", xtask::hil::FEATURES);
    let dir = target_dir.join("coverage").join(variant);
    let Some(target) = xtask::hil::coverage_build(ws, bin, &features, &dir, kernel, &mut report) else {
        return print_report(&report, "hil");
    };
    let stimulus = if stress {
        xtask::hil::build_with(ws, stimulus_bin, &format!("{},storm", xtask::hil::FEATURES), &target_dir.join("stimulus-storm"), None, &mut report)
    } else {
        xtask::hil::build(ws, stimulus_bin, target_dir, None, &mut report)
    };
    let Some(stimulus) = stimulus else {
        return print_report(&report, "hil");
    };
    report.note(format!("coverage build of `{variant}` ({features}; {}): {} over {} loadable bytes; {} s", xtask::hil::COVERAGE_TOOLCHAIN, target.binary.hash, target.binary.loadable_bytes, duration_secs));
    let model = ws.root.join("apps").join(bin).join(format!("gen-{variant}")).join("task_model.json");
    let stress_spec = if stress {
        match analyzer_wcrt(ws, &model, target.binary.config_checksum.as_deref(), &dir, &mut report) {
            Some((protected, _)) => Some(rsk_hil::release_timing::Stress { protected, storm_task: 4, mit_ms: 15 }),
            None => return print_report(&report, "hil"),
        }
    } else {
        None
    };
    let work = dir.join("run");
    let loop_bounds = xtask::hil::loop_bounds_export(ws, bin, &dir.join("loop-bounds.json"), &mut report);
    let inputs = rsk_hil::release_timing::Inputs {
        rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        work_dir: &work,
        duration_secs,
        grace_secs: 90,
        tick_hz: xtask::hil::TICK_HZ,
        cpu_hz: xtask::hil::CPU_HZ,
        samplerate_hz: 100_000,
        flash_stimulus: true,
        stress: stress_spec,
        loop_bounds: loop_bounds.as_deref(),
        configuration: variant,
        binary_hash: &target.binary.hash,
        timing_bounds: !kernel,
        expect_deferrals: false,
    };
    let out = rsk_hil::release_timing::run(&inputs);
    let mut text = rsk_hil::release_timing::render(rig, &inputs, &out);
    text = text.replacen("# HIL ", "# HIL coverage run (task 19.2, R34.4) over the ", 1);
    // The trace and events, as the measurement runs keep them (a failed
    // coverage run is diagnosed from them: the finding of 2026-11-07).
    let _ = std::fs::create_dir_all(&work);
    let _ = std::fs::write(work.join("trace.txt"), rsk_hil::trace::render(&out.trace, xtask::hil::TICK_HZ));
    if let Some(ev) = &out.events {
        let _ = std::fs::write(work.join("events.txt"), ev.render(xtask::hil::TICK_HZ));
    }
    // The counters, the raw profile, and the branch report.
    let board = rsk_hil::board::Target { chip: &rig.chip, serial: &rig.target.serial };
    let crates = xtask::hil::partition_crates(ws, bin);
    let mut sources: Vec<std::path::PathBuf> = crates.iter().map(|(_, _, s)| s.clone()).collect();
    if kernel {
        for c in xtask::hil::KERNEL_COVERAGE_CRATES {
            let dir = if *c == "rsk_system" { ws.root.join("apps").join(bin).join("system").join("src") } else { ws.root.join("crates").join(c.replace('_', "-")).join("src") };
            sources.push(dir);
        }
    }
    let task_model = std::fs::read_to_string(&model).ok().and_then(|t| rsk_model::read_task_model(&t).ok());
    let mut kernel_report: Option<rsk_hil::coverage::KernelReport> = None;
    let coverage = match (task_model.as_ref(), rsk_hil::coverage::collect(&board, &target.elf, &work)) {
        (Some(m), Ok((profraw, ranges))) => match rsk_hil::coverage::export(&tools, &target.elf, &profraw, &work, &sources) {
            Ok(export) => {
                if kernel {
                    // The Kernel's figures: the export over the Kernel sources alone.
                    let kernel_sources: Vec<std::path::PathBuf> = sources.iter().filter(|s| !s.starts_with(ws.root.join("apps").join(bin).join("part"))).cloned().collect();
                    match rsk_hil::coverage::export(&tools, &target.elf, &profraw, &work.join("kernel"), &kernel_sources) {
                        Ok(kexport) => kernel_report = Some(rsk_hil::coverage::kernel_report(&kexport, &ws.root.display().to_string(), variant, &target.binary.hash)),
                        Err(e) => report.error(xtask::diag::Rule::Step, None, format!("the Kernel's coverage export: {e}")),
                    }
                }
                let functions = rsk_hil::coverage::functions(&export);
                let tasks: Vec<(String, String, String, Vec<String>)> = m
                    .tasks
                    .iter()
                    .map(|t| {
                        let krate = m.partitions.iter().find(|p| p.id == t.partition).map(|p| p.krate.clone()).unwrap_or_default();
                        let body = t.id.rsplit('.').next().unwrap_or(&t.id).to_string();
                        (t.id.clone(), krate, body, t.cs_sites.clone())
                    })
                    .collect();
                let r = rsk_hil::coverage::report(functions, &tasks, variant, &target.binary.hash, rig.coverage_branch_percent);
                report.note(format!("counters read: {}", ranges.iter().map(|r| format!("Partition {} {} bytes", r.partition, r.end - r.start)).collect::<Vec<_>>().join(", ")));
                Ok(r)
            }
            Err(e) => Err(e),
        },
        (None, _) => Err(format!("no Task_Model at {}", ws.rel(&model))),
        (_, Err(e)) => Err(e),
    };
    match &coverage {
        Ok(r) => {
            text.push_str("\n## Branch coverage of the measurement inputs (R34.4)\n\n");
            text.push_str(&format!("{}\n\n", r.summary()));
            text.push_str(&r.render());
        }
        Err(e) => text.push_str(&format!("\n## Branch coverage of the measurement inputs (R34.4)\n\nNot collected: {e}\n")),
    }
    if let Some(k) = &kernel_report {
        text.push_str("\n## Structural coverage of the Kernel on the Target (task 19.5, R53.3)\n\n");
        text.push_str(&format!("{}\n\n", k.summary()));
        text.push_str(&k.render());
    }
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let _ = std::fs::write(record.join(format!("coverage_{variant}.md")), &text);
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the coverage run failed (report above): no coverage is taken from a failed run".to_string());
        return print_report(&report, "hil");
    }
    let r = match coverage {
        Ok(r) => r,
        Err(e) => {
            report.error(xtask::diag::Rule::Step, None, format!("branch coverage not collected: {e}"));
            return print_report(&report, "hil");
        }
    };
    let path = xtask::hil::coverage_record_path(&ws.root, variant);
    match std::fs::write(&path, r.print()) {
        Ok(()) => report.note(format!("branch coverage written to {}: {}", ws.rel(&path), r.summary())),
        Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&path))),
    }
    if kernel {
        let Some(k) = &kernel_report else {
            report.error(xtask::diag::Rule::Step, None, "the Kernel's coverage was not collected".to_string());
            return print_report(&report, "hil");
        };
        let kpath = xtask::hil::kernel_coverage_record_path(&ws.root, variant);
        match std::fs::write(&kpath, k.print()) {
            Ok(()) => report.note(format!("Kernel coverage written to {}: {}", ws.rel(&kpath), k.summary())),
            Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&kpath))),
        }
        return print_report(&report, "Kernel coverage run passed");
    }
    // The committed WCET_Records of the configuration take the figures
    // (R33.1's coverage field), each with a condition naming the build.
    let wcet = xtask::hil::wcet_record_path(&ws.root, variant);
    if let Some(mut set) = std::fs::read_to_string(&wcet).ok().and_then(|t| rsk_model::read_wcet_set(&t).ok()) {
        let mut entered = 0;
        for rec in &mut set.records {
            let Some(item) = r.items.iter().find(|i| i.item == rec.item) else { continue };
            if let Some(m) = &mut rec.measured {
                m.coverage_branch_percent = item.percent;
                rec.conditions.retain(|c| !c.starts_with("branch coverage "));
                rec.conditions.push(format!("branch coverage {} % of the inputs ({} of {} branch outcomes of the body and its crate's helpers) from the coverage build {} of this configuration (R34.4; docs/hil/coverage_{variant}.json)", item.percent, item.covered, item.branches, &target.binary.hash));
                entered += 1;
            }
        }
        match std::fs::write(&wcet, rsk_model::print(&set)) {
            Ok(()) => report.note(format!("coverage entered into {} records of {}", entered, ws.rel(&wcet))),
            Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&wcet))),
        }
    } else {
        report.note(format!("no committed WCET_Records at {} to carry the coverage (run the measurement first)", ws.rel(&wcet)));
    }
    if !r.ok() {
        report.error(xtask::diag::Rule::Step, None, format!("branch coverage below the {} % threshold (R34.4): {}", r.threshold_percent, r.below().iter().map(|i| format!("{} {} %", i.item, i.percent)).collect::<Vec<_>>().join(", ")));
        return print_report(&report, "hil");
    }
    print_report(&report, "coverage run passed")
}

/// The Analyzer's WCRT of every Task of the highest-criticality Partition
/// (R18.7) in `model` under placeholder timing. The Analyzer requires a
/// Build_Manifest whose Generated_Config checksum is the model's (R28.8):
/// the job's manifest (`cargo xtask verify` writes it) is copied with the
/// checksum of the configuration's own HIL build, which is not a
/// Flight_Build and has no manifest of its own.
fn analyzer_wcrt(ws: &xtask::workspace::Workspace, model: &std::path::Path, config_checksum: Option<&str>, dir: &std::path::Path, report: &mut Report) -> Option<(Vec<(u8, String, u64)>, String)> {
    let job_manifest = ws.root.join("target").join("xtask-verify").join("evidence").join("build-manifest.json");
    if !job_manifest.is_file() {
        report.error(xtask::diag::Rule::Step, None, format!("no Build_Manifest at {}: run `cargo xtask verify` first (the Analyzer requires one)", ws.rel(&job_manifest)));
        return None;
    }
    if !model.is_file() {
        report.error(xtask::diag::Rule::Step, None, format!("no Task_Model at {}: run `cargo xtask gen`", ws.rel(model)));
        return None;
    }
    let out = dir.join("rta");
    let _ = std::fs::create_dir_all(&out);
    let manifest = out.join("build-manifest.json");
    let mut value: serde_json::Value = match std::fs::read_to_string(&job_manifest).ok().and_then(|t| serde_json::from_str(&t).ok()) {
        Some(v) => v,
        None => {
            report.error(xtask::diag::Rule::Step, None, format!("{} is not a JSON Build_Manifest", ws.rel(&job_manifest)));
            return None;
        }
    };
    value["generated_config_checksum"] = match config_checksum {
        Some(c) => serde_json::Value::String(c.to_string()),
        None => serde_json::Value::Null,
    };
    if let Err(e) = std::fs::write(&manifest, serde_json::to_string_pretty(&value).unwrap_or_default()) {
        report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&manifest)));
        return None;
    }
    // The bench criterion stays the placeholder analysis (every WCET the
    // Budget, nominal Kernel_Timing_Parameters): under the measured J_rel
    // (task 19.3) the placeholder analysis of the demo is not schedulable
    // (Verification_Plan 11.5), and the job's `analysis-rta-*-measured`
    // items carry that result.
    let timing_used = "placeholder timing".to_string();
    let cmd = xtask::cmd::Cmd::cargo_frozen("run")
        .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(ws.root.join("target"))
        .arg("--")
        .arg("--model")
        .arg(model)
        .arg("--manifest")
        .arg(&manifest)
        .arg("--placeholders")
        .arg("--out")
        .arg(&out);
    match cmd.output(None) {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            report.error(xtask::diag::Rule::Step, None, format!("the Analyzer rejected the configuration: {}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr)));
            return None;
        }
        Err(e) => {
            report.error(xtask::diag::Rule::Step, None, format!("cannot run the Analyzer: {e}"));
            return None;
        }
    }
    let text = std::fs::read_to_string(out.join("report.json")).ok()?;
    let rep = match rsk_model::read_report(&text) {
        Ok(r) => r,
        Err(e) => {
            report.error(xtask::diag::Rule::Step, None, format!("the Analyzer's report: {e:?}"));
            return None;
        }
    };
    // The Tasks of the highest Criticality_Level, in Task_Model order (the
    // Kernel's Task indices).
    let top = rep.tasks.iter().map(|t| t.level.clone()).min()?;
    Some((rep.tasks.iter().enumerate().filter(|(_, t)| t.level == top).map(|(i, t)| (i as u8, t.id.clone(), t.r_cycles)).collect(), timing_used))
}

/// `cargo xtask hil --response` (task 7.5, R14.11): the response suite
/// over the response configuration (2 seconds): one injected fault per
/// response of R14.5, judged from the event mirror, the counts, and the
/// capture. The report goes under target/xtask-hil/response and docs/hil/.
fn response_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, stimulus_bin: &str, coverage: bool, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    // `--coverage --response` (task 19.2 extended, R34.4, R53.3): the same
    // suite over the Partition crates instrumented, the counters read after
    // the run and the branch report written beside the measurement
    // configurations' (`docs/hil/coverage_nrf52840_response.{md,json}`).
    let features = format!("{},response{}", xtask::hil::FEATURES, if coverage { ",coverage" } else { "" });
    let dir = if coverage { target_dir.join("coverage").join("nrf52840_response") } else { target_dir.join("response") };
    let target = if coverage { xtask::hil::coverage_build(ws, bin, &features, &dir, false, &mut report) } else { xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) };
    let Some(target) = target else {
        return print_report(&report, "hil");
    };
    let Some(stimulus) = xtask::hil::build(ws, stimulus_bin, target_dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    report.note(format!("response configuration `nrf52840_response` ({features}): {} over {} loadable bytes", target.binary.hash, target.binary.loadable_bytes));
    let work = dir.join("run");
    let inputs = rsk_hil::response::Inputs {
        rig,
        target_elf: &target.elf,
        stimulus_elf: &stimulus.elf,
        work_dir: &work,
        // The report prints every event over semihosting, seconds each dozen.
        timeout_secs: 90,
        tick_hz: xtask::hil::TICK_HZ,
        samplerate_hz: 100_000,
        flash_stimulus: true,
    };
    let out = rsk_hil::response::run(&inputs);
    let text = rsk_hil::response::render(rig, &inputs, &out);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let _ = std::fs::write(work.join("trace.txt"), &out.trace_text);
    if let Some(ev) = &out.events {
        let _ = std::fs::write(work.join("events.txt"), ev.render(xtask::hil::TICK_HZ));
    }
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    if coverage {
        return suite_coverage(ws, rig, bin, "nrf52840_response", &target, &work, text, out.ok(), "the response suite", report);
    }
    let _ = std::fs::write(record.join("response_nrf52840_response.md"), &text);
    report.note(format!("report written to {} and docs/hil/response_nrf52840_response.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the response suite failed (report above)");
    }
    print_report(&report, "response suite passed")
}

/// The branch coverage of a fault-injection suite's run (task 19.2
/// extended; R34.4, R53.3): the instrumented Partitions' counters read
/// over the probe after the suite, the raw profile exported, and each Task
/// entry's branch outcomes reported against the plan's threshold, as the
/// measurement configurations' coverage runs do. The report
/// (`docs/hil/coverage_<variant>.md`: the suite's report with the coverage
/// section) and the record (`coverage_<variant>.json`) are written
/// whatever the verdict; the command fails on a failed suite, on
/// uncollected coverage, or on a Task below the threshold. These suites
/// drive the refusals, the responses, and the restarts the measurement
/// configurations leave uncovered (section 14), so their figures are read
/// beside those, not in their place.
#[allow(clippy::too_many_arguments)]
fn suite_coverage(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, variant: &str, target: &xtask::hil::HilBuild, work: &std::path::Path, mut text: String, suite_ok: bool, suite: &str, mut report: Report) -> ExitCode {
    let Some(tools) = xtask::hil::coverage_tools(&mut report) else {
        return print_report(&report, "hil");
    };
    let board = rsk_hil::board::Target { chip: &rig.chip, serial: &rig.target.serial };
    let crates = xtask::hil::partition_crates(ws, bin);
    let sources: Vec<std::path::PathBuf> = crates.iter().map(|(_, _, s)| s.clone()).collect();
    let model = xtask::hil::task_model(ws, bin, variant);
    let coverage = match (model.as_ref(), rsk_hil::coverage::collect(&board, &target.elf, work)) {
        (Some(m), Ok((profraw, ranges))) => match rsk_hil::coverage::export(&tools, &target.elf, &profraw, work, &sources) {
            Ok(export) => {
                let functions = rsk_hil::coverage::functions(&export);
                let tasks: Vec<(String, String, String, Vec<String>)> = m
                    .tasks
                    .iter()
                    .map(|t| {
                        let krate = m.partitions.iter().find(|p| p.id == t.partition).map(|p| p.krate.clone()).unwrap_or_default();
                        let body = t.id.rsplit('.').next().unwrap_or(&t.id).to_string();
                        (t.id.clone(), krate, body, t.cs_sites.clone())
                    })
                    .collect();
                report.note(format!("counters read: {}", ranges.iter().map(|r| format!("Partition {} {} bytes", r.partition, r.end - r.start)).collect::<Vec<_>>().join(", ")));
                Ok(rsk_hil::coverage::report(functions, &tasks, variant, &target.binary.hash, rig.coverage_branch_percent))
            }
            Err(e) => Err(e),
        },
        (None, _) => Err(format!("no Task_Model for `{variant}` (run `cargo xtask gen`)")),
        (_, Err(e)) => Err(e),
    };
    text = text.replacen("# HIL ", "# HIL coverage run (R34.4, R53.3) over the ", 1);
    match &coverage {
        Ok(r) => {
            text.push_str("\n## Branch coverage of the Task entries under the suite (R34.4)\n\n");
            text.push_str(&format!("{}\n\n", r.summary()));
            text.push_str(&r.render());
        }
        Err(e) => text.push_str(&format!("\n## Branch coverage of the Task entries under the suite (R34.4)\n\nNot collected: {e}\n")),
    }
    let _ = std::fs::write(work.join("report.md"), &text);
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::write(record.join(format!("coverage_{variant}.md")), &text);
    report.note(format!("coverage report written to {} and docs/hil/coverage_{variant}.md", ws.rel(work)));
    if !suite_ok {
        report.error(xtask::diag::Rule::Step, None, format!("{suite} failed under instrumentation (report above): no coverage is taken from a failed run"));
        return print_report(&report, "hil");
    }
    let r = match coverage {
        Ok(r) => r,
        Err(e) => {
            report.error(xtask::diag::Rule::Step, None, format!("branch coverage not collected: {e}"));
            return print_report(&report, "hil");
        }
    };
    let path = xtask::hil::coverage_record_path(&ws.root, variant);
    match std::fs::write(&path, r.print()) {
        Ok(()) => report.note(format!("branch coverage written to {}: {}", ws.rel(&path), r.summary())),
        Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write {}: {e}", ws.rel(&path))),
    }
    if !r.ok() {
        report.error(xtask::diag::Rule::Step, None, format!("branch coverage below the {} % threshold (R34.4): {}", r.threshold_percent, r.below().iter().map(|i| format!("{} {} %", i.item, i.percent)).collect::<Vec<_>>().join(", ")));
        return print_report(&report, "hil");
    }
    print_report(&report, "coverage run passed")
}

/// `cargo xtask hil --isolation` (tasks 7.4 and 3.3): the isolation suite
/// over the isolation configuration (3.6 seconds; the MPU access matrix
/// of R16.8 and the EasyDMA cases of R17.7) and, from the same run, the
/// Hardware_Model differential tests of R49.3. The reports go under
/// target/xtask-hil/isolation and docs/hil/.
fn isolation_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, coverage: bool, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    // `--coverage --isolation`: the suite over the instrumented Partitions
    // (as `--coverage --response`; `docs/hil/coverage_nrf52840_isolation.{md,json}`).
    let features = format!("{},isolation{}", xtask::hil::FEATURES, if coverage { ",coverage" } else { "" });
    let dir = if coverage { target_dir.join("coverage").join("nrf52840_isolation") } else { target_dir.join("isolation") };
    let target = if coverage { xtask::hil::coverage_build(ws, bin, &features, &dir, false, &mut report) } else { xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) };
    let Some(target) = target else {
        return print_report(&report, "hil");
    };
    let Some(model) = xtask::hil::task_model(ws, bin, "nrf52840_isolation") else {
        report.error(xtask::diag::Rule::Step, None, "no Task_Model for the isolation configuration (run `cargo xtask gen`)");
        return print_report(&report, "hil");
    };
    report.note(format!("isolation configuration `nrf52840_isolation` ({features}): {} over {} loadable bytes", target.binary.hash, target.binary.loadable_bytes));
    let work = dir.join("run");
    let inputs = rsk_hil::isolation::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 60, tick_hz: xtask::hil::TICK_HZ, model: &model };
    let out = rsk_hil::isolation::run(&inputs);
    let text = rsk_hil::isolation::render(rig, &inputs, &out, &target.binary.hash);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let _ = std::fs::write(work.join("trace.txt"), &out.trace_text);
    if let Some(ev) = &out.events {
        let _ = std::fs::write(work.join("events.txt"), ev.render(xtask::hil::TICK_HZ));
    }
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    if coverage {
        return suite_coverage(ws, rig, bin, "nrf52840_isolation", &target, &work, text, out.ok(), "the isolation suite", report);
    }
    let _ = std::fs::write(record.join("isolation_nrf52840_isolation.md"), &text);
    if let Some(dt) = &out.dt {
        let dt_text = rsk_hil::dt::render(dt, &out.dt_rows, &rig.target.serial, &target.binary.hash);
        print!("{dt_text}");
        let _ = std::fs::write(work.join("hw_model_differential.md"), &dt_text);
        let _ = std::fs::write(record.join("hw_model_differential_nrf52840.md"), &dt_text);
    }
    report.note(format!("reports written to {} and docs/hil/isolation_nrf52840_isolation.md, docs/hil/hw_model_differential_nrf52840.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the isolation suite failed (report above)");
    }
    print_report(&report, "isolation suite passed")
}

/// `cargo xtask hil --boot-checks` (task 7.2, R12.6): the boot-check suite
/// over the HIL build with the `boot-mutation` feature: one run per
/// mutated check. The report goes under target/xtask-hil/boot-checks and
/// docs/hil/.
fn boot_checks_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let features = format!("{},boot-mutation", xtask::hil::FEATURES);
    let dir = target_dir.join("boot-checks");
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    report.note(format!("boot-check build ({features}): {} over {} loadable bytes", target.binary.hash, target.binary.loadable_bytes));
    let work = dir.join("run");
    let inputs = rsk_hil::boot_checks::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 40, counters: xtask::hil::COUNTERS };
    let out = rsk_hil::boot_checks::run(&inputs);
    let text = rsk_hil::boot_checks::render(rig, &out, &target.binary.hash);
    print!("{text}");
    let _ = std::fs::create_dir_all(&work);
    let _ = std::fs::write(work.join("report.md"), &text);
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let _ = std::fs::write(record.join("boot_checks_nrf52840.md"), &text);
    report.note(format!("report written to {} and docs/hil/boot_checks_nrf52840.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the boot-check suite failed (report above)");
    }
    print_report(&report, "boot-check suite passed")
}

/// `cargo xtask hil --cycles` (task 7.5's `hil-cycles`, R18.1): the
/// cycle-attribution test on the cycle configuration (2 seconds). The
/// report goes under target/xtask-hil/cycles and docs/hil/.
fn cycles_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, lean: bool, fpu: bool, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    // `--fpu` (task 19.3): the floating-point variant, `filter` the preempted
    // spinner, which measures Δ_fp_save and Δ_fp_restore against the plain
    // configuration's Δ_entry.
    let base_variant = if fpu { "nrf52840_cycles_fpu" } else { "nrf52840_cycles" };
    let features = format!("{},{}", xtask::hil::features_for(lean), if fpu { "cycles-fpu" } else { "cycles" });
    let dir = target_dir.join(format!("{}{}", if fpu { "cycles-fpu" } else { "cycles" }, if lean { "-lean" } else { "" }));
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    report.note(format!("cycle configuration `{base_variant}` ({features}): {} over {} loadable bytes", target.binary.hash, target.binary.loadable_bytes));
    let work = dir.join("run");
    let inputs = rsk_hil::cycles::Inputs {
        rig,
        target_elf: &target.elf,
        work_dir: &work,
        timeout_secs: 40,
        tick_hz: xtask::hil::TICK_HZ,
        cpu_hz: xtask::hil::CPU_HZ,
        low: if fpu { rsk_hil::cycles::LOW_FPU } else { rsk_hil::cycles::LOW },
        tolerance_percent: if fpu { rsk_hil::cycles::TOLERANCE_PERCENT_FPU } else { rsk_hil::cycles::TOLERANCE_PERCENT },
    };
    let out = rsk_hil::cycles::run(&inputs);
    let text = rsk_hil::cycles::render(rig, &inputs, &out, &target.binary.hash);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let suffix = if lean { "_lean" } else { "" };
    let _ = std::fs::write(record.join(format!("cycles_{base_variant}{suffix}.md")), &text);
    report.note(format!("report written to {} and docs/hil/cycles_{base_variant}{suffix}.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the cycle-attribution test failed (report above)");
        return print_report(&report, "hil");
    }
    // The Kernel_Timing_Parameter record (task 19.3): Δ_entry and the entry
    // kinds, folded into the kept record of the same binary.
    let variant_label = format!("{base_variant}{suffix}");
    let variant = variant_label.as_str();
    let model = xtask::hil::task_model(ws, bin, base_variant);
    let target_name = model.as_ref().map(|m| format!("{}/{}", m.target, m.core_revision)).unwrap_or_else(|| "nrf52840/?".to_string());
    let binding = rsk_hil::kernel_timing::Binding { binary_hash: &target.binary.hash, target: &target_name, icache: "off", configuration: variant };
    if let Some(st) = &out.stats {
        let extra: Vec<_> = if fpu {
            // Δ_entry of the plain configuration's committed record, before the margin.
            let plain = std::fs::read_to_string(xtask::hil::timing_record_path(&ws.root, &format!("nrf52840_cycles{suffix}"))).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok());
            let delta_entry = plain.as_ref().and_then(|p| p.parameters.iter().find(|q| q.symbol == "delta_entry")).and_then(|q| q.measured.as_ref()).map(|m| m.max_observed);
            match delta_entry.and_then(|d| rsk_hil::cycles::delta_fp(st, d, xtask::hil::MARGIN_PERCENT)) {
                Some(v) => {
                    report.note(format!("Δ_fp_save and Δ_fp_restore from filter's excess per entry less Δ_entry {} of the plain configuration: {}", delta_entry.unwrap_or(0), v.iter().map(|p| format!("{} {} (observed {})", p.symbol, p.cycles, p.measured.as_ref().map(|m| m.max_observed).unwrap_or(0))).collect::<Vec<_>>().join(", ")));
                    v
                }
                None => {
                    report.error(xtask::diag::Rule::Step, None, format!("Δ_fp: no plain cycle record with Δ_entry ({delta_entry:?}; run `cargo xtask hil --cycles` first) or no interruption counts ({:?})", rsk_hil::cycles::per_entry_excess(st, rsk_hil::cycles::LOW_FPU)));
                    return print_report(&report, "hil");
                }
            }
        } else {
            rsk_hil::cycles::delta_entry(st, xtask::hil::MARGIN_PERCENT).into_iter().collect()
        };
        if let Some(new) = rsk_hil::kernel_timing::record_from_stats(st, &binding, xtask::hil::CPU_HZ, &format!("{base_variant} (2 s)"), xtask::hil::MARGIN_PERCENT, extra) {
            let path = xtask::hil::timing_record_path(&ws.root, variant);
            let kept = std::fs::read_to_string(&path).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok());
            let (folded, replaced) = rsk_hil::kernel_timing::fold(kept, new);
            match serde_json::to_string_pretty(&folded) {
                Ok(text) => {
                    let _ = std::fs::write(&path, format!("{text}\n"));
                    report.note(format!("Kernel_Timing_Parameter record {} {} ({} parameters: {})", ws.rel(&path), if replaced { "written" } else { "extended" }, folded.parameters.len(), folded.parameters.iter().map(|p| format!("{} {}", p.symbol, p.cycles)).collect::<Vec<_>>().join(", ")));
                }
                Err(e) => report.error(xtask::diag::Rule::Step, None, format!("cannot write the record: {e}")),
            }
        }
    }
    print_report(&report, "cycle-attribution test passed")
}

/// `cargo xtask nightly` (R50.1): the whole campaign on the head of the
/// main branch, one command after another, each with its own log; a step
/// that fails does not stop the next. The summary names every step's
/// verdict and log, so that a morning read suffices.
fn nightly_command(args: &[String]) -> ExitCode {
    if !args.is_empty() {
        return usage_error("nightly takes no options");
    }
    let root = xtask::repository_root();
    let stamp = {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_secs()).unwrap_or(0);
        // Calendar date from the epoch (UTC), no dependency.
        let days = secs / 86_400;
        let (y, m, d) = civil_from_days(days as i64);
        format!("{y:04}-{m:02}-{d:02}")
    };
    let dir = root.join("target").join("xtask-nightly").join(&stamp);
    let _ = std::fs::create_dir_all(&dir);
    // The job first (its Build_Manifest is what the measurement runs' own
    // analyses key the cycle records on), then the records' refresh in the
    // order a Kernel change requires (the cycle configurations' Δ_entry
    // and Δ_fp, the short configurations, the plain one), the hour-long
    // campaigns, the coverage runs (task 19.2, 19.4, 19.5; the two
    // fault-injection suites' since 2026-11-07), and the job again on the
    // refreshed records.
    let steps: [(&str, &[&str]); 15] = [
        ("verify", &["verify"]),
        ("cycles", &["hil", "--cycles"]),
        ("cycles-fpu", &["hil", "--cycles", "--fpu"]),
        ("timing-short", &["hil", "--release-timing", "--short"]),
        ("stress-short", &["hil", "--stress", "--short"]),
        ("measure", &["hil", "--plain"]),
        ("release-timing", &["hil", "--release-timing"]),
        ("stress", &["hil", "--stress"]),
        ("coverage-timing", &["hil", "--coverage"]),
        ("coverage-stress", &["hil", "--coverage", "--stress"]),
        ("coverage-plain", &["hil", "--coverage", "--plain"]),
        ("coverage-kernel", &["hil", "--coverage", "--kernel"]),
        ("coverage-response", &["hil", "--coverage", "--response"]),
        ("coverage-isolation", &["hil", "--coverage", "--isolation"]),
        ("verify-final", &["verify"]),
    ];
    let mut summary = format!("# Nightly HIL campaign {stamp}\n\n| Step | Verdict | Log | Minutes |\n|---|---|---|---|\n");
    let mut all_ok = true;
    for (name, argv) in steps {
        let log = dir.join(format!("{name}.log"));
        let start = std::time::Instant::now();
        let status = std::process::Command::new(std::env::current_exe().unwrap_or_else(|_| "xtask".into()))
            .args(argv)
            .current_dir(&root)
            .stdout(std::fs::File::create(&log).map(std::process::Stdio::from).unwrap_or_else(|_| std::process::Stdio::null()))
            .stderr(std::process::Stdio::inherit())
            .status();
        let ok = status.map(|s| s.success()).unwrap_or(false);
        all_ok &= ok;
        summary.push_str(&format!("| {name} | {} | {} | {:.0} |\n", if ok { "pass" } else { "FAIL" }, log.display(), start.elapsed().as_secs_f64() / 60.0));
        println!("nightly: {name}: {}", if ok { "pass" } else { "FAIL" });
    }
    let _ = std::fs::write(dir.join("summary.md"), &summary);
    let _ = std::fs::write(root.join("target").join("xtask-nightly").join("latest.md"), &summary);
    print!("{summary}");
    if all_ok { ExitCode::SUCCESS } else { ExitCode::FAILURE }
}

/// Proleptic Gregorian date from days since 1970-01-01 (Howard Hinnant's algorithm).
fn civil_from_days(z: i64) -> (i64, u32, u32) {
    let z = z + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = (z - era * 146_097) as u64;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe as i64 + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as u32;
    let m = if mp < 10 { mp + 3 } else { mp - 9 } as u32;
    (if m <= 2 { y + 1 } else { y }, m, d)
}

fn qemu_command(args: &[String]) -> ExitCode {
    let mut bin = "p1-demo".to_string();
    let mut timeout = 60u64;
    let mut flood = false;
    let mut args = args.iter();
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--bin" => match args.next() {
                Some(b) => bin = b.clone(),
                None => return usage_error("--bin takes a crate name"),
            },
            // The flood configuration (task 21.6, A-P7): the demo built
            // `--no-default-features --features qemu-flood`.
            "--flood" => flood = true,
            "--timeout" => match args.next().and_then(|t| t.parse().ok()) {
                Some(t) => timeout = t,
                None => return usage_error("--timeout takes seconds"),
            },
            other => return usage_error(&format!("unknown option `{other}`")),
        }
    }
    let features = if flood { Some("qemu-flood") } else { None };
    match xtask::qemu::run_with(&xtask::repository_root(), &bin, timeout, features) {
        Ok(code) => ExitCode::from(code.min(255) as u8),
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}

fn print_report(report: &Report, success: &str) -> ExitCode {
    for note in &report.notes {
        println!("note: {note}");
    }
    for warning in &report.warnings {
        println!("warning: {warning}");
    }
    for diagnostic in &report.diagnostics {
        println!("{diagnostic}");
    }
    if report.diagnostics.is_empty() {
        println!("{success}");
        ExitCode::SUCCESS
    } else {
        println!("{} violation(s)", report.diagnostics.len());
        ExitCode::FAILURE
    }
}

fn usage_error(message: &str) -> ExitCode {
    eprintln!("{message}\n\n{USAGE}");
    ExitCode::from(2)
}
