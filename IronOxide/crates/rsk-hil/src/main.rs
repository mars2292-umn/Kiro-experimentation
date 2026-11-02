//! `rsk-hil`: the HIL_Rig from the command line (task 7.1). The
//! verification job and `cargo xtask hil` call the library; this binary
//! is the operator's hand tool.
//!
//!     rsk-hil probes
//!     rsk-hil self-test --rig hil-rig.toml --target <elf> --stimulus <elf> --work <dir> [--timeout <s>] [--target-hash <sha256:..>] [--stimulus-hash <sha256:..>]
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use rsk_hil::config::Rig;
use rsk_hil::selftest::{self, Inputs};

const USAGE: &str = "usage:
  rsk-hil probes
  rsk-hil self-test --rig <hil-rig.toml> --target <elf> --stimulus <elf> --work <dir> [--timeout <s>] [--target-hash <h>] [--stimulus-hash <h>]";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("probes") => match rsk_hil::probes::list() {
            Ok(list) => {
                for p in list {
                    println!("{} ({})", p.id, p.name);
                }
                ExitCode::SUCCESS
            }
            Err(e) => {
                eprintln!("rsk-hil: {e}");
                ExitCode::FAILURE
            }
        },
        Some("self-test") => self_test(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            ExitCode::from(2)
        }
    }
}

fn self_test(args: &[String]) -> ExitCode {
    let mut rig = None;
    let mut target = None;
    let mut stimulus = None;
    let mut work = None;
    let mut timeout = 60u64;
    let mut target_hash = None;
    let mut stimulus_hash = None;
    let mut it = args.iter();
    while let Some(a) = it.next() {
        let value = |it: &mut std::slice::Iter<String>| it.next().cloned().ok_or_else(|| format!("{a} takes a value"));
        let r: Result<(), String> = (|| {
            match a.as_str() {
                "--rig" => rig = Some(PathBuf::from(value(&mut it)?)),
                "--target" => target = Some(PathBuf::from(value(&mut it)?)),
                "--stimulus" => stimulus = Some(PathBuf::from(value(&mut it)?)),
                "--work" => work = Some(PathBuf::from(value(&mut it)?)),
                "--timeout" => timeout = value(&mut it)?.parse().map_err(|_| "--timeout takes seconds".to_string())?,
                "--target-hash" => target_hash = Some(value(&mut it)?),
                "--stimulus-hash" => stimulus_hash = Some(value(&mut it)?),
                other => return Err(format!("unknown option `{other}`")),
            }
            Ok(())
        })();
        if let Err(e) = r {
            eprintln!("rsk-hil: {e}\n{USAGE}");
            return ExitCode::from(2);
        }
    }
    let (Some(rig_path), Some(target), Some(stimulus), Some(work)) = (rig, target, stimulus, work) else {
        eprintln!("rsk-hil: --rig, --target, --stimulus, and --work are required\n{USAGE}");
        return ExitCode::from(2);
    };
    let rig = match Rig::load(&rig_path) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rsk-hil: {e}");
            return ExitCode::FAILURE;
        }
    };
    let inputs = Inputs {
        rig: &rig,
        target_elf: &target,
        stimulus_elf: &stimulus,
        target_hash: target_hash.as_deref(),
        stimulus_hash: stimulus_hash.as_deref(),
        work_dir: &work,
        timeout_secs: timeout,
        tick_hz: 16_000_000,
    };
    let out = selftest::run(&inputs);
    let report = selftest::render(&rig, &out);
    print!("{report}");
    let _ = std::fs::write(work.join("self-test.md"), &report);
    let _ = std::fs::write(work.join("edges.txt"), &out.edges_text);
    let _ = std::fs::write(work.join("trace.txt"), &out.trace_text);
    if out.ok() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
