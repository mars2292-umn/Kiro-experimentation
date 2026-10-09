//! `cargo xtask`: the rsk Build_System command line.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use xtask::diag::Report;
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
  gen [--check]
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
        "gen" => gen_command(rest),
        "qemu" => qemu_command(rest),
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
