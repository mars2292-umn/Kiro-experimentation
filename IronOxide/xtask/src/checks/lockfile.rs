//! R58.1: build only from the committed lock file, with exact versions and
//! vendored sources.
//!
//! `Workspace::load` already ran `cargo metadata --frozen`, which fails when
//! the dependency graph needs a package that `Cargo.lock` does not list. This
//! module adds the rules Cargo does not enforce: every crates.io requirement
//! in a workspace crate is an exact `=x.y.z` requirement, path dependencies
//! stay inside the workspace, nothing comes from git or another registry,
//! and every registry package in the lock file is present in the vendored
//! directory with the checksum the lock file records.

use std::collections::BTreeSet;
use std::path::PathBuf;

use toml::{Table, Value};

use crate::checks::config::vendor_dir;
use crate::diag::{Report, Rule};
use crate::workspace::{canonical, Workspace};

/// The source id Cargo reports for crates.io dependencies.
pub const CRATES_IO: &str = "registry+https://github.com/rust-lang/crates.io-index";

/// Whether `req` pins exactly one version: `=MAJOR.MINOR.PATCH`, optionally
/// with a pre-release and build metadata.
pub fn is_exact_req(req: &str) -> bool {
    let Some(version) = req.trim().strip_prefix('=') else {
        return false;
    };
    let version = version.trim();
    let core = version.split_once('+').map_or(version, |(core, _)| core);
    let (numbers, pre) = match core.split_once('-') {
        Some((numbers, pre)) => (numbers, Some(pre)),
        None => (core, None),
    };
    let parts: Vec<&str> = numbers.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
        && pre.is_none_or(|p| {
            !p.is_empty()
                && p.bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'-')
        })
}

pub fn check(ws: &Workspace, report: &mut Report) {
    check_requirements(ws, report);
    check_vendored_lock_entries(ws, report);
}

fn check_requirements(ws: &Workspace, report: &mut Report) {
    let member_dirs: BTreeSet<PathBuf> = ws.member_packages().map(|p| p.dir.clone()).collect();
    let mut exact = 0usize;
    let mut registry = 0usize;
    for pkg in ws.member_packages() {
        let location = Some(format!("{}/Cargo.toml", ws.rel(&pkg.dir)));
        for dep in &pkg.dependencies {
            match (&dep.source, &dep.path) {
                (None, Some(path)) => {
                    if !member_dirs.contains(&canonical(path)) {
                        report.error(
                            Rule::Locked,
                            location.clone(),
                            format!(
                                "path dependency `{}` ({}) is not a workspace member, so it is neither \
                                 vendored nor covered by the workspace checks",
                                dep.name,
                                path.display()
                            ),
                        );
                    }
                }
                (Some(source), _) if source == CRATES_IO => {
                    registry += 1;
                    if is_exact_req(&dep.req) {
                        exact += 1;
                    } else {
                        report.error(
                            Rule::Locked,
                            location.clone(),
                            format!(
                                "dependency `{}` has the requirement `{}`; only exact `=x.y.z` requirements \
                                 are allowed",
                                dep.name, dep.req
                            ),
                        );
                    }
                }
                (Some(source), _) => report.error(
                    Rule::Locked,
                    location.clone(),
                    format!(
                        "dependency `{}` comes from `{source}`; only vendored crates.io packages and \
                         workspace paths are allowed",
                        dep.name
                    ),
                ),
                (None, None) => report.error(
                    Rule::Locked,
                    location.clone(),
                    format!("dependency `{}` has neither a source nor a path", dep.name),
                ),
            }
        }
    }
    report.note(format!(
        "R58.1: {registry} crates.io requirements in workspace crates, {exact} of them exact"
    ));
}

fn check_vendored_lock_entries(ws: &Workspace, report: &mut Report) {
    let location = Some("Cargo.lock".to_string());
    let text = match std::fs::read_to_string(ws.root.join("Cargo.lock")) {
        Ok(text) => text,
        Err(e) => {
            report.error(
                Rule::Locked,
                location,
                format!("cannot read the lock file: {e}"),
            );
            return;
        }
    };
    let lock: Table = match text.parse() {
        Ok(lock) => lock,
        Err(e) => {
            report.error(
                Rule::Locked,
                location,
                format!("cannot parse the lock file: {e}"),
            );
            return;
        }
    };
    let vendor = vendor_dir(&ws.root);
    let packages = lock
        .get("package")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut vendored = 0usize;
    for entry in &packages {
        let field = |key: &str| {
            entry
                .get(key)
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string()
        };
        let (name, version, source) = (field("name"), field("version"), field("source"));
        if source.is_empty() {
            continue; // a path package
        }
        if source != CRATES_IO {
            report.error(
                Rule::Locked,
                location.clone(),
                format!("lock entry `{name} {version}` comes from `{source}`, not from vendored crates.io"),
            );
            continue;
        }
        let checksum = field("checksum");
        if checksum.is_empty() {
            report.error(
                Rule::Locked,
                location.clone(),
                format!("lock entry `{name} {version}` has no checksum"),
            );
            continue;
        }
        let Some(vendor) = &vendor else {
            continue; // reported by the configuration check
        };
        let dir = [vendor.join(format!("{name}-{version}")), vendor.join(&name)]
            .into_iter()
            .find(|d| d.join(".cargo-checksum.json").is_file());
        let recorded = dir.as_ref().and_then(|d| {
            let json = std::fs::read_to_string(d.join(".cargo-checksum.json")).ok()?;
            let value: serde_json::Value = serde_json::from_str(&json).ok()?;
            value.get("package")?.as_str().map(str::to_string)
        });
        match recorded {
            Some(recorded) if recorded == checksum => vendored += 1,
            Some(_) => report.error(
                Rule::Locked,
                location.clone(),
                format!("vendored `{name} {version}` does not match the checksum in the lock file"),
            ),
            None => report.error(
                Rule::Locked,
                location.clone(),
                format!("lock entry `{name} {version}` is not present in the vendored sources"),
            ),
        }
    }
    report.note(format!(
        "R58.1: {} lock entries, {vendored} registry packages vendored with matching checksums",
        packages.len()
    ));
}

#[cfg(test)]
mod tests {
    use super::is_exact_req;

    #[test]
    fn accepts_only_exact_requirements() {
        for ok in ["=1.0.107", "= 1.2.3", "=1.1.7+spec-1.1.0", "=2.0.0-rc.1"] {
            assert!(is_exact_req(ok), "{ok}");
        }
        for bad in [
            "1.0.107",
            "^1.0",
            "~1.0.1",
            "=1.0",
            "*",
            ">=1, <2",
            "=1.0.0, <2",
            "=1.x.0",
            "",
        ] {
            assert!(!is_exact_req(bad), "{bad}");
        }
    }
}
