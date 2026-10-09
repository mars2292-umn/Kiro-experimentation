//! External tools of the verification job and their versions: the Verus
//! release (with its Z3), Kani (with its CBMC), and git.
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
