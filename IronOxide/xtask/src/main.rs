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
  hil [--bin <crate>] [--timeout <s>] [--spike-only | --release-timing [--short] | --stress [--short] | --response | --isolation | --boot-checks | --cycles]
      Run the Generator (rsk-gen) on every declaration crate of the
      workspace (policy `declaration = true`) and write the Task_Model,
      the linker script and memory map, the configuration summary and the
      manifests into the application's gen/ directory (R24.3, R25.1).
      --check only compares them with the tree (R24.4).
  qemu [--bin <name>] [--timeout <seconds>]
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
    let mut short = false;
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
            "--short" => short = true,
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
    if release_timing || stress {
        return release_timing_command(&ws, &rig, &bin, &stimulus_bin, short, stress, &target_dir, report);
    }
    if response {
        return response_command(&ws, &rig, &bin, &stimulus_bin, &target_dir, report);
    }
    if isolation {
        return isolation_command(&ws, &rig, &bin, &target_dir, report);
    }
    if boot_checks {
        return boot_checks_command(&ws, &rig, &bin, &target_dir, report);
    }
    if cycles {
        return cycles_command(&ws, &rig, &bin, &target_dir, report);
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
fn release_timing_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, stimulus_bin: &str, short: bool, stress: bool, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let Some(measurement) = xtask::hil::MEASUREMENTS.iter().find(|m| m.stress == stress && (m.duration_secs < 10 * 60) == short) else {
        report.error(xtask::diag::Rule::Step, None, "no measurement configuration for this run".to_string());
        return print_report(&report, "hil");
    };
    let (feature, duration_secs, variant) = (measurement.feature, measurement.duration_secs, measurement.variant);
    let kind = if stress { "stress" } else { "release-timing" };
    let features = format!("{},{feature}", xtask::hil::FEATURES);
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
    let model = ws.root.join("apps").join(bin).join(format!("gen-{variant}")).join("task_model.json");
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
    };
    let out = rsk_hil::release_timing::run(&inputs);
    let text = rsk_hil::release_timing::render(rig, &inputs, &out);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let _ = std::fs::write(work.join("trace.txt"), rsk_hil::trace::render(&out.trace, xtask::hil::TICK_HZ));
    if let Some(ev) = &out.events {
        let _ = std::fs::write(work.join("events.txt"), ev.render(xtask::hil::TICK_HZ));
    }
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let name = format!("{}_{variant}.md", kind.replace('-', "_"));
    let _ = std::fs::write(record.join(&name), &text);
    report.note(format!("report written to {} and docs/hil/{name}", ws.rel(&work)));
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
    print_report(&report, &format!("{kind} run passed"))
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
fn response_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, stimulus_bin: &str, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let features = format!("{},response", xtask::hil::FEATURES);
    let dir = target_dir.join("response");
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
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
    let _ = std::fs::write(record.join("response_nrf52840_response.md"), &text);
    report.note(format!("report written to {} and docs/hil/response_nrf52840_response.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the response suite failed (report above)");
    }
    print_report(&report, "response suite passed")
}

/// `cargo xtask hil --isolation` (tasks 7.4 and 3.3): the isolation suite
/// over the isolation configuration (3.6 seconds; the MPU access matrix
/// of R16.8 and the EasyDMA cases of R17.7) and, from the same run, the
/// Hardware_Model differential tests of R49.3. The reports go under
/// target/xtask-hil/isolation and docs/hil/.
fn isolation_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let features = format!("{},isolation", xtask::hil::FEATURES);
    let dir = target_dir.join("isolation");
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
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
fn cycles_command(ws: &xtask::workspace::Workspace, rig: &rsk_hil::config::Rig, bin: &str, target_dir: &std::path::Path, mut report: Report) -> ExitCode {
    let features = format!("{},cycles", xtask::hil::FEATURES);
    let dir = target_dir.join("cycles");
    let Some(target) = xtask::hil::build_with(ws, bin, &features, &dir, None, &mut report) else {
        return print_report(&report, "hil");
    };
    report.note(format!("cycle configuration `nrf52840_cycles` ({features}): {} over {} loadable bytes", target.binary.hash, target.binary.loadable_bytes));
    let work = dir.join("run");
    let inputs = rsk_hil::cycles::Inputs { rig, target_elf: &target.elf, work_dir: &work, timeout_secs: 40, tick_hz: xtask::hil::TICK_HZ, cpu_hz: xtask::hil::CPU_HZ };
    let out = rsk_hil::cycles::run(&inputs);
    let text = rsk_hil::cycles::render(rig, &inputs, &out, &target.binary.hash);
    print!("{text}");
    let _ = std::fs::write(work.join("report.md"), &text);
    let record = ws.root.join("docs").join("hil");
    let _ = std::fs::create_dir_all(&record);
    let _ = std::fs::write(record.join("cycles_nrf52840_cycles.md"), &text);
    report.note(format!("report written to {} and docs/hil/cycles_nrf52840_cycles.md", ws.rel(&work)));
    if !out.ok() {
        report.error(xtask::diag::Rule::Step, None, "the cycle-attribution test failed (report above)");
    }
    print_report(&report, "cycle-attribution test passed")
}

fn qemu_command(args: &[String]) -> ExitCode {
    let mut bin = "p1-demo".to_string();
    let mut timeout = 60u64;
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
            other => return usage_error(&format!("unknown option `{other}`")),
        }
    }
    match xtask::qemu::run(&xtask::repository_root(), &bin, timeout) {
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
