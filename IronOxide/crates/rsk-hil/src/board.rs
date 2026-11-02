//! One board through probe-rs: flashing, reset, the read-back hash check
//! (R48.4), and the console run. Every command names the board by serial.

use std::path::Path;
use std::process::{Command, Stdio};
use std::time::Instant;

use crate::strip_ansi;

/// A board's probe-rs identity: the chip and the probe selector.
#[derive(Clone, Debug)]
pub struct Target<'a> {
    pub chip: &'a str,
    pub serial: &'a str,
}

impl Target<'_> {
    fn command(&self, sub: &str) -> Command {
        let mut c = Command::new("probe-rs");
        c.arg(sub).args(["--chip", self.chip, "--probe", self.serial]);
        c
    }

    fn display(&self, sub: &str, rest: &str) -> String {
        format!("probe-rs {sub} --chip {} --probe {} {rest}", self.chip, self.serial).trim_end().to_string()
    }

    /// Flashes `elf` (`probe-rs download`); the board is left as probe-rs
    /// leaves it (halted at reset), so a [`Target::reset`] follows when it
    /// must run.
    pub fn flash(&self, elf: &Path) -> Result<Step, String> {
        let start = Instant::now();
        let out = self.command("download").arg("--disable-progressbars").arg(elf).output().map_err(|e| format!("cannot run probe-rs: {e}"))?;
        let log = strip_ansi(&format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)));
        let command = self.display("download", &format!("--disable-progressbars {}", elf.display()));
        if !out.status.success() {
            return Err(format!("{command} failed:\n{log}"));
        }
        Ok(Step { command, log, seconds: start.elapsed().as_secs_f64() })
    }

    /// Resets the board and lets it run.
    pub fn reset(&self) -> Result<Step, String> {
        let start = Instant::now();
        let out = self.command("reset").output().map_err(|e| format!("cannot run probe-rs: {e}"))?;
        let log = strip_ansi(&format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)));
        let command = self.display("reset", "");
        if !out.status.success() {
            return Err(format!("{command} failed:\n{log}"));
        }
        Ok(Step { command, log, seconds: start.elapsed().as_secs_f64() })
    }

    /// Reads one range of the board's memory.
    pub fn read(&self, addr: u32, len: usize, scratch: &Path) -> Result<Vec<u8>, String> {
        let out = self
            .command("read")
            .args(["b8", &format!("{addr:#x}"), &len.to_string(), "-f", "binary", "-o"])
            .arg(scratch)
            .output()
            .map_err(|e| format!("cannot run probe-rs: {e}"))?;
        if !out.status.success() {
            return Err(format!("probe-rs read {addr:#x} ({len} bytes) failed: {}", strip_ansi(&String::from_utf8_lossy(&out.stderr)).trim()));
        }
        let bytes = std::fs::read(scratch).map_err(|e| format!("cannot read {}: {e}", scratch.display()))?;
        if bytes.len() != len {
            return Err(format!("probe-rs read {addr:#x}: {} bytes instead of {len}", bytes.len()));
        }
        Ok(bytes)
    }

    /// R48.4: reads back every loadable part of `elf` from the board and
    /// hashes the read-back image exactly as the Build_Manifest hashes the
    /// ELF (`rsk_elf::Elf::hash_input`), so the two hashes agree only when
    /// every byte agrees.
    pub fn read_back(&self, elf: &Path, scratch: &Path) -> Result<Readback, String> {
        let image = rsk_elf::Elf::read(elf).map_err(|e| format!("{}: {e}", elf.display()))?;
        let parts = image.loadable_image().map_err(|e| format!("{}: {e}", elf.display()))?;
        let elf_hash = rsk_digest::sha256_hex(&image.hash_input().map_err(|e| e.to_string())?);
        let mut input = Vec::new();
        let mut bytes = 0;
        let mut mismatches = Vec::new();
        for (addr, expected) in &parts {
            let got = self.read(*addr, expected.len(), scratch)?;
            if let Some(off) = got.iter().zip(expected).position(|(a, b)| a != b) {
                mismatches.push(*addr + off as u32);
            }
            bytes += got.len();
            input.extend_from_slice(&addr.to_le_bytes());
            input.extend_from_slice(&(got.len() as u32).to_le_bytes());
            input.extend_from_slice(&got);
        }
        let _ = std::fs::remove_file(scratch);
        Ok(Readback { segments: parts.len(), bytes, hash: rsk_digest::sha256_hex(&input), elf_hash, mismatches })
    }

    /// Reads the Kernel's RAM trace ring (rsk-kernel feature `trace-ram`,
    /// the `TRACE_RING` symbol of `elf`); `None` when the build has none.
    pub fn read_trace(&self, elf: &Path, scratch: &Path) -> Result<Option<Vec<crate::trace::Entry>>, String> {
        let image = rsk_elf::Elf::read(elf).map_err(|e| format!("{}: {e}", elf.display()))?;
        let symbols = image.symbols().map_err(|e| e.to_string())?;
        let Some(ring) = symbols.iter().find(|s| s.name == "TRACE_RING" && s.is_object() && s.size >= 16) else {
            return Ok(None);
        };
        let bytes = self.read(ring.address(), ring.size as usize, scratch)?;
        let _ = std::fs::remove_file(scratch);
        let words: Vec<u32> = bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]])).collect();
        Ok(Some(crate::trace::decode(&words)))
    }

    /// Flashes and runs `elf` with the console (`probe-rs run`, which
    /// carries the Kernel's semihosting output and exit code); the run is
    /// killed after `timeout_secs`. `ok` is a zero exit with `RSK-OK` on
    /// the console.
    pub fn run(&self, elf: &Path, timeout_secs: u64) -> Run {
        let args = ["--disable-progressbars", "--catch-hardfault"];
        let command = self.display("run", &format!("{} {}", args.join(" "), elf.display()));
        let start = Instant::now();
        let mut child = match self.command("run").args(args).arg(elf).stdout(Stdio::piped()).stderr(Stdio::piped()).spawn() {
            Ok(c) => c,
            Err(e) => return Run { ok: false, seconds: 0.0, log: format!("cannot run probe-rs: {e}"), command, timed_out: false },
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
        log = strip_ansi(&log);
        if timed_out {
            log.push_str(&format!("\n[killed after {timeout_secs} s]\n"));
        }
        let ok = !timed_out && status == Some(0) && log.contains("RSK-OK");
        Run { ok, seconds, log, command, timed_out }
    }
}

/// One probe-rs command that completed.
#[derive(Clone, Debug)]
pub struct Step {
    pub command: String,
    pub log: String,
    pub seconds: f64,
}

/// The read-back of a loaded image.
#[derive(Clone, Debug)]
pub struct Readback {
    pub segments: usize,
    pub bytes: usize,
    /// `sha256:` over the read-back image in the Build_Manifest's form.
    pub hash: String,
    /// The same hash computed from the ELF on the host.
    pub elf_hash: String,
    /// The first differing address of each part that differs.
    pub mismatches: Vec<u32>,
}

impl Readback {
    pub fn matches(&self) -> bool {
        self.mismatches.is_empty() && self.hash == self.elf_hash
    }
}

/// The outcome of one console run.
#[derive(Clone, Debug)]
pub struct Run {
    pub ok: bool,
    pub seconds: f64,
    pub log: String,
    pub command: String,
    pub timed_out: bool,
}

impl Run {
    /// The value of a `key=value` line of the safe-state report.
    pub fn value(&self, key: &str) -> Option<u32> {
        self.log.lines().find_map(|l| l.trim().strip_prefix(key).and_then(|r| r.strip_prefix('=')).and_then(|v| v.trim().parse().ok()))
    }

    /// The board's access port is locked (nRF52840 APPROTECT after a reset
    /// without the setting that keeps it open): the bench's state, not the
    /// Kernel's.
    pub fn locked(&self) -> bool {
        self.log.contains("Core is locked")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reads_report_values() {
        let run = Run { ok: true, seconds: 1.0, log: "RSK-SAFE-STATE\ncontrol=20\npulse=10\nRSK-OK\n".into(), command: String::new(), timed_out: false };
        assert_eq!(run.value("pulse"), Some(10));
        assert_eq!(run.value("control"), Some(20));
        assert_eq!(run.value("echo"), None);
        assert!(!run.locked());
    }
}
