//! External tools of the verification job and their versions: the Verus
//! release (with its Z3), Kani (with its CBMC), Creusot (the App_Prover,
//! with Why3 and its solvers), and git.
//!
//! The Verus release is located through `RSK_VERUS_DIR` or, failing that,
//! the directory of a `cargo-verus` executable on `PATH`. Kani is the
//! `cargo kani` subcommand installed in `CARGO_HOME`. Versions are recorded
//! in every Evidence_Item (R44.6, R45.7, R50.3).

use std::path::{Path, PathBuf};

use crate::cmd::{stdout_of, Cmd};

/// The Verus release directory and the versions it reports.
#[derive(Clone, Debug)]
pub struct Verus {
    pub dir: PathBuf,
    pub version: String,
    pub toolchain: String,
    pub z3_version: String,
}

impl Verus {
    /// Locates the Verus release.
    pub fn locate() -> Result<Verus, String> {
        let dir = match std::env::var_os("RSK_VERUS_DIR") {
            Some(dir) => PathBuf::from(dir),
            None => {
                let path = std::env::var_os("PATH").unwrap_or_default();
                std::env::split_paths(&path)
                    .find(|p| p.join("cargo-verus").is_file() && p.join("verus").is_file())
                    .ok_or(
                        "no Verus release found: set RSK_VERUS_DIR to the unpacked release directory \
                         (it contains `verus`, `cargo-verus`, and `z3`) or put it on PATH",
                    )?
            }
        };
        let verus = dir.join("verus");
        if !verus.is_file() {
            return Err(format!("{} contains no `verus` executable", dir.display()));
        }
        let out = stdout_of(&Cmd::new(&verus).arg("--version"))?;
        let field = |key: &str| {
            out.lines()
                .find_map(|l| l.trim().strip_prefix(key))
                .map(|v| v.trim().to_string())
                .unwrap_or_default()
        };
        let version = field("Version:");
        let toolchain = field("Toolchain:");
        if version.is_empty() {
            return Err(format!("`{} --version` printed no version line", verus.display()));
        }
        let z3 = dir.join("z3");
        let z3_version = if z3.is_file() {
            stdout_of(&Cmd::new(&z3).arg("--version"))?.trim().to_string()
        } else {
            "unknown (no z3 in the release directory)".to_string()
        };
        Ok(Verus {
            dir,
            version,
            toolchain,
            z3_version,
        })
    }

    pub fn cargo_verus(&self) -> PathBuf {
        self.dir.join("cargo-verus")
    }

    pub fn line_count(&self) -> Option<PathBuf> {
        let p = self.dir.join("line_count");
        p.is_file().then_some(p)
    }
}

/// Kani and the CBMC it drives.
#[derive(Clone, Debug)]
pub struct Kani {
    pub version: String,
    pub cbmc_version: String,
}

impl Kani {
    /// Locates `cargo kani` through the default toolchain's Cargo (Kani
    /// installs itself as a Cargo subcommand and selects its own nightly).
    pub fn locate(root: &Path) -> Result<Kani, String> {
        let out = stdout_of(
            &Cmd::new("cargo")
                .args(["+stable", "kani", "--version"])
                .cwd(root),
        )
        .or_else(|_| stdout_of(&Cmd::new("cargo").args(["kani", "--version"]).cwd(root)))
        .map_err(|e| format!("Kani is not installed (`cargo kani --version` failed): {e}"))?;
        let mut version = String::new();
        let mut cbmc_version = String::new();
        for line in out.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("Kani Rust Verifier") {
                version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("CBMC") {
                cbmc_version = v.trim().to_string();
            }
        }
        if version.is_empty() {
            return Err(format!("unexpected `cargo kani --version` output: {}", out.trim()));
        }
        Ok(Kani {
            version,
            cbmc_version,
        })
    }
}

/// Creusot, the App_Prover (R46.2), as `cargo creusot version` reports it,
/// with the pin of `toolchain/decision.toml [creusot]`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Creusot {
    /// `cargo-creusot`'s version (`0.15.0-dev`).
    pub version: String,
    /// The rustc Creusot drives (`nightly-2026-08-03`).
    pub toolchain: String,
    pub why3: String,
    pub why3find: String,
    /// `(name, version)` of each back-end solver, in the order reported.
    pub solvers: Vec<(String, String)>,
    /// The pinned commit of the Creusot checkout (decision.toml), since a
    /// development build's version number does not identify it.
    pub pinned_commit: Option<String>,
}

impl Creusot {
    /// Locates `cargo creusot` (run in `dir`, the proof workspace) and reads
    /// the pin. A reported version or toolchain other than the pinned one is
    /// an error: the Evidence_Items would record an installation the plan
    /// did not evaluate (R46.2).
    pub fn locate(root: &Path, dir: &Path) -> Result<Creusot, String> {
        let out = stdout_of(&Cmd::cargo().args(["creusot", "version"]).cwd(dir))
            .map_err(|e| format!("cargo-creusot is not installed (`cargo creusot version` failed): {e}"))?;
        let mut c = Creusot {
            version: String::new(),
            toolchain: String::new(),
            why3: String::new(),
            why3find: String::new(),
            solvers: Vec::new(),
            pinned_commit: None,
        };
        for line in out.lines() {
            let line = line.trim();
            if let Some(v) = line.strip_prefix("cargo-creusot") {
                c.version = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Rust toolchain") {
                c.toolchain = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("Why3 platform, version") {
                c.why3 = v.trim().to_string();
            } else if let Some(v) = line.strip_prefix("why3find") {
                c.why3find = v.trim().trim_start_matches('v').to_string();
            } else if let Some((name, v)) = line.split_once(' ') {
                if ["alt-ergo", "z3", "cvc4", "cvc5"].contains(&name) && !c.solvers.iter().any(|(n, _)| n == name) {
                    // cvc5 prints a multi-line banner that cargo creusot
                    // reports as "cvc5 bad CVC5 version: cvc5 1.4.1 [git ..";
                    // the version is the first word that looks like one.
                    let v = v
                        .split(" [")
                        .next()
                        .unwrap_or(v)
                        .split_whitespace()
                        .find(|w| w.chars().next().is_some_and(|ch| ch.is_ascii_digit()) && w.contains('.'))
                        .unwrap_or("")
                        .to_string();
                    c.solvers.push((name.to_string(), v));
                }
            }
        }
        if c.version.is_empty() {
            return Err(format!("unexpected `cargo creusot version` output: {}", out.trim()));
        }
        let pin = std::fs::read_to_string(root.join("toolchain/decision.toml"))
            .ok()
            .and_then(|t| t.parse::<toml::Table>().ok())
            .and_then(|t| t.get("creusot").and_then(toml::Value::as_table).cloned());
        if let Some(pin) = pin {
            let s = |k: &str| pin.get(k).and_then(toml::Value::as_str).map(str::to_string);
            c.pinned_commit = s("commit");
            if let Some(v) = s("version").filter(|v| *v != c.version) {
                return Err(format!("Creusot {} is installed but toolchain/decision.toml [creusot] pins {v} (R46.2)", c.version));
            }
            if let Some(v) = s("pinned_rustc").filter(|v| *v != c.toolchain) {
                return Err(format!("Creusot drives {} but toolchain/decision.toml [creusot] pins {v} (R46.2)", c.toolchain));
            }
        }
        Ok(c)
    }

    /// One line for an Evidence_Item's tool version.
    pub fn summary(&self) -> String {
        let mut parts = vec![format!("cargo-creusot {}", self.version)];
        if let Some(c) = &self.pinned_commit {
            parts.push(format!("commit {}", &c[..c.len().min(9)]));
        }
        parts.push(format!("Rust toolchain {}", self.toolchain));
        parts.push(format!("Why3 {}", self.why3));
        parts.push(format!("why3find {}", self.why3find));
        parts.extend(self.solvers.iter().map(|(n, v)| format!("{n} {v}")));
        parts.join("; ")
    }
}

/// The git revision of the working tree and whether it has local changes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Revision {
    pub commit: String,
    pub dirty: bool,
    /// `true` when `root` is inside a git repository and tracked.
    pub tracked: bool,
}

impl Revision {
    pub fn of(root: &Path) -> Revision {
        let commit = stdout_of(&Cmd::new("git").args(["rev-parse", "HEAD"]).cwd(root))
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string());
        let status = stdout_of(&Cmd::new("git").args(["status", "--porcelain", "--", "."]).cwd(root))
            .unwrap_or_default();
        let tracked = stdout_of(&Cmd::new("git").args(["ls-files", "--", "."]).cwd(root))
            .map(|s| !s.trim().is_empty())
            .unwrap_or(false);
        Revision {
            commit,
            dirty: !status.trim().is_empty(),
            tracked,
        }
    }
}

/// The current time as an RFC 3339 UTC timestamp, without any dependency.
pub fn now_rfc3339() -> String {
    let secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let days = secs / 86_400;
    let rem = secs % 86_400;
    let (y, m, d) = civil_from_days(days as i64);
    format!(
        "{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z",
        rem / 3600,
        (rem % 3600) / 60,
        rem % 60
    )
}

/// Howard Hinnant's days-to-civil algorithm.
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn civil_dates() {
        assert_eq!(civil_from_days(0), (1970, 1, 1));
        assert_eq!(civil_from_days(19_000), (2022, 1, 8));
        assert_eq!(civil_from_days(20_734), (2026, 10, 8));
    }
}
