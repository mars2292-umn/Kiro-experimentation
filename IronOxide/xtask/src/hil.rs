//! The HIL run of the P1 demo on the nRF52840-DK (task 3.1's hardware
//! half, `hil-spike` of Verification_Plan section 11): builds the
//! declaration variant `nrf52840` of the Flight_Build with the `debug-port`
//! feature (the access port stays open across resets, HIL builds only),
//! flashes and runs it through `probe-rs run`, which carries the Kernel's
//! semihosting console and exit, and judges the run by `RSK-OK` and the
//! exit code, as the QEMU spike does.
//!
//! One board, one probe: `probe-rs list` names it; without one the job's
//! step is skipped with a warning. `cargo xtask hil [--timeout <s>]` runs it
//! by hand and prints the console.

use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::sandbox::Sandbox;
use crate::workspace::Workspace;

/// probe-rs's chip name for the DK's part.
pub const CHIP: &str = "nRF52840_xxAA";
/// The declaration variant and the features of the HIL build.
pub const VARIANT: &str = "nrf52840";
pub const FEATURES: &str = "nrf52840,debug-port";

/// The probe-rs installation and the probe it sees.
pub struct Probe {
    pub version: String,
    pub probe: String,
}

/// Finds `probe-rs` and one attached debug probe.
pub fn probe() -> Option<Probe> {
    let version = Cmd::new("probe-rs").arg("--version").output(None).ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").trim().to_string())?;
    let list = Cmd::new("probe-rs").arg("list").output(None).ok().filter(|o| o.status.success()).map(|o| String::from_utf8_lossy(&o.stdout).into_owned())?;
    let probe = list.lines().find(|l| l.trim_start().starts_with("[0]:")).map(|l| l.trim().to_string())?;
    Some(Probe { version, probe })
}

/// The outcome of one run on the board.
pub struct Run {
    pub ok: bool,
    pub seconds: f64,
    pub log: String,
    pub command: String,
}

/// Where the HIL build goes (its own target directory: a different
/// feature set from the job's Flight_Build).
pub fn target_dir(root: &Path) -> PathBuf {
    root.join("target").join("xtask-hil")
}

/// Builds the HIL variant of `bin` for the Target; returns the ELF path.
pub fn build(ws: &Workspace, bin: &str, sandbox: Option<&Sandbox>, report: &mut Report) -> Option<(PathBuf, String)> {
    let target_dir = target_dir(&ws.root);
    let cmd = Cmd::cargo_frozen("build")
        .args(["--quiet", "-p", bin, "--release", "--target", &ws.policy.target, "--no-default-features", "--features", FEATURES, "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir);
    let command = cmd.display();
    match cmd.output(sandbox) {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            report.error(Rule::Step, None, format!("HIL build failed:\n{}", String::from_utf8_lossy(&o.stderr)));
            return None;
        }
        Err(e) => {
            report.error(Rule::Step, None, format!("cannot run cargo: {e}"));
            return None;
        }
    }
    Some((target_dir.join(&ws.policy.target).join("release").join(bin), command))
}

/// Flashes and runs `elf` on the attached board; the console is the log.
/// The run is killed after `timeout_secs`.
pub fn run_elf(elf: &Path, timeout_secs: u64) -> Run {
    let args = ["run", "--chip", CHIP, "--disable-progressbars", "--catch-hardfault"];
    let command = format!("probe-rs {} {}", args.join(" "), elf.display());
    let start = Instant::now();
    let mut child = match std::process::Command::new("probe-rs").args(args).arg(elf).stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn() {
        Ok(c) => c,
        Err(e) => return Run { ok: false, seconds: 0.0, log: format!("cannot run probe-rs: {e}"), command },
    };
    let mut timed_out = false;
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed().as_secs() >= timeout_secs => {
                let _ = child.kill();
                timed_out = true;
                break;
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(50)),
            Err(_) => break,
        }
    }
    let output = child.wait_with_output();
    let seconds = start.elapsed().as_secs_f64();
    let (status, mut log) = match output {
        Ok(o) => (o.status.code(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => (None, format!("probe-rs failed: {e}")),
    };
    // probe-rs colours its output; the log is kept plain.
    log = strip_ansi(&log);
    if timed_out {
        log.push_str(&format!("\n[killed after {timeout_secs} s]\n"));
    }
    // probe-rs reports the firmware's semihosting exit code as its own.
    let ok = !timed_out && status == Some(0) && log.contains("RSK-OK");
    Run { ok, seconds, log, command }
}

fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&d) = chars.peek() {
                    chars.next();
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}

/// The whole run for one binary: build, flash, run, judge.
pub fn run(ws: &Workspace, bin: &str, timeout_secs: u64, sandbox: Option<&Sandbox>, report: &mut Report) -> Option<Run> {
    let (elf, build_cmd) = build(ws, bin, sandbox, report)?;
    let mut run = run_elf(&elf, timeout_secs);
    run.command = format!("{build_cmd}; {}", run.command);
    Some(run)
}
