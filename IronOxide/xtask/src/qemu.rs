//! The QEMU runner of the dispatch spike (task 3.1): builds a Flight_Build
//! binary for the Target and runs it on `qemu-system-arm -M mps2-an386`
//! (Cortex-M4 with MPU and FPU) with semihosting enabled, so that the
//! binary's console output reaches the terminal and its semihosting exit
//! code becomes the run's status.
//!
//! QEMU is a pre-hardware check of the DD-01 mechanism (NVIC dispatch of
//! unprivileged Thread-mode Jobs, BASEPRI levels, the unwind at Job end,
//! MPU views, SVC services, the time base); it is not the Target, and its
//! DWT does not count cycles, so Budgets are not exercised. The HIL_Rig
//! runs the same binary class on the nRF52840-DK. The verification job runs
//! the spike binary on QEMU as a test step when QEMU is installed
//! (`cargo xtask verify`, step 10b).

use std::io::Read;
use std::path::Path;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use crate::cmd::Cmd;
use crate::diag::Report;
use crate::workspace::Workspace;

pub const MACHINE: &str = "mps2-an386";

/// The result of one QEMU run.
pub struct Run {
    /// The semihosting exit code.
    pub exit_code: u32,
    /// Console output (stdout then stderr).
    pub log: String,
    pub seconds: f64,
}

/// Builds `bin` for the Target in the release profile and runs it on QEMU.
/// Returns the semihosting exit code.
pub fn run(root: &Path, bin: &str, timeout_secs: u64) -> Result<u32, String> {
    let mut report = Report::default();
    let ws = Workspace::load(root, &mut report).ok_or_else(|| {
        report
            .diagnostics
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join("\n")
    })?;
    let target_dir = root.join("target").join("qemu");
    let build = Cmd::cargo_frozen("build")
        .args(["--release", "--target", &ws.policy.target, "-p", bin])
        .arg("--manifest-path")
        .arg(root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(&target_dir)
        .env("CARGO_ENCODED_RUSTFLAGS", crate::verify::target_rustflags(root));
    println!("$ {}", build.display());
    if !build.status(None).map_err(|e| e.to_string())? {
        return Err("the Target build failed".to_string());
    }
    let elf = target_dir.join(&ws.policy.target).join("release").join(bin);
    if !elf.is_file() {
        return Err(format!("no binary at {}", elf.display()));
    }
    let run = run_elf(&elf, timeout_secs)?;
    println!("--- QEMU output ---\n{}\n--- end ({:.1}s) ---", run.log, run.seconds);
    let log_dir = target_dir.join("logs");
    let _ = std::fs::create_dir_all(&log_dir);
    let _ = std::fs::write(log_dir.join(format!("{bin}.log")), &run.log);
    Ok(run.exit_code)
}

/// The QEMU executable, when installed.
pub fn locate() -> Option<std::path::PathBuf> {
    which("qemu-system-arm")
}

/// The QEMU version line (`QEMU emulator version x.y.z`), when installed.
pub fn version() -> Option<String> {
    let out = Command::new(locate()?).arg("--version").output().ok()?;
    let text = String::from_utf8_lossy(&out.stdout);
    let line = text.lines().next()?;
    Some(line.trim_start_matches("QEMU emulator version ").trim().to_string())
}

pub fn command_line(elf: &Path) -> String {
    format!(
        "qemu-system-arm -M {MACHINE} -cpu cortex-m4 -nographic -semihosting-config enable=on,target=native,userspace=on -kernel {}",
        elf.display()
    )
}

/// Runs an already-built ELF on QEMU and collects its console output and
/// semihosting exit code; the run is killed after `timeout_secs`.
pub fn run_elf(elf: &Path, timeout_secs: u64) -> Result<Run, String> {
    let qemu = locate().ok_or("qemu-system-arm is not installed (brew install qemu)")?;
    let mut cmd = Command::new(qemu);
    cmd.args(["-M", MACHINE, "-cpu", "cortex-m4", "-nographic", "-semihosting-config", "enable=on,target=native,userspace=on", "-kernel"])
        .arg(elf)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    println!("$ {}", command_line(elf));
    let start = Instant::now();
    let mut child = cmd.spawn().map_err(|e| format!("cannot start QEMU: {e}"))?;
    let mut stdout = child.stdout.take().expect("piped");
    let mut stderr = child.stderr.take().expect("piped");
    let reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stdout.read_to_end(&mut out);
        out
    });
    let err_reader = std::thread::spawn(move || {
        let mut out = Vec::new();
        let _ = stderr.read_to_end(&mut out);
        out
    });
    let status = loop {
        if let Some(status) = child.try_wait().map_err(|e| e.to_string())? {
            break Some(status);
        }
        if start.elapsed() > Duration::from_secs(timeout_secs) {
            let _ = child.kill();
            let _ = child.wait();
            break None;
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let out = reader.join().unwrap_or_default();
    let err = err_reader.join().unwrap_or_default();
    let mut log = String::from_utf8_lossy(&out).into_owned();
    let err_text = String::from_utf8_lossy(&err);
    if !err_text.trim().is_empty() {
        log.push('\n');
        log.push_str(&err_text);
    }
    match status {
        Some(status) => Ok(Run {
            exit_code: status.code().unwrap_or(1) as u32,
            log,
            seconds: start.elapsed().as_secs_f64(),
        }),
        None => Err(format!("QEMU did not exit within {timeout_secs}s (killed)")),
    }
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .map(|p| p.join(name))
        .find(|p| p.is_file())
}
