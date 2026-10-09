//! R55.2 and R55.5: the pinned upstream rustc and the single edition.
//!
//! `toolchain/decision.toml` records the decision (Ferrocene 26.05.0, based
//! on Rust 1.95.0; edition 2021) with its sources and assumptions. The check
//! requires `rust-toolchain.toml`, the active rustc and Cargo, and every
//! workspace crate to agree with its `[pin]` table, and the active compiler
//! to be a stable release.

use std::path::Path;

use toml::{Table, Value};

use crate::cmd::{stdout_of, Cmd};
use crate::diag::{Report, Rule};
use crate::workspace::Workspace;

pub const DECISION: &str = "toolchain/decision.toml";
pub const TOOLCHAIN_FILE: &str = "rust-toolchain.toml";

/// The `[pin]` table of the decision record.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pin {
    pub upstream_rustc: String,
    pub edition: String,
    pub targets: Vec<String>,
}

fn read_table(root: &Path, file: &str, report: &mut Report) -> Option<Table> {
    let text = match std::fs::read_to_string(root.join(file)) {
        Ok(text) => text,
        Err(e) => {
            report.error(
                Rule::Toolchain,
                Some(file.to_string()),
                format!("cannot read: {e}"),
            );
            return None;
        }
    };
    match text.parse::<Table>() {
        Ok(table) => Some(table),
        Err(e) => {
            report.error(
                Rule::Toolchain,
                Some(file.to_string()),
                format!("cannot parse: {e}"),
            );
            None
        }
    }
}

fn strings(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

pub fn read_pin(root: &Path, report: &mut Report) -> Option<Pin> {
    let table = read_table(root, DECISION, report)?;
    let pin = table.get("pin").and_then(Value::as_table);
    let field = |key: &str| {
        pin.and_then(|p| p.get(key))
            .and_then(Value::as_str)
            .map(str::to_string)
    };
    match (field("upstream_rustc"), field("edition")) {
        (Some(upstream_rustc), Some(edition)) => Some(Pin {
            upstream_rustc,
            edition,
            targets: strings(pin.and_then(|p| p.get("targets"))),
        }),
        _ => {
            report.error(
                Rule::Toolchain,
                Some(DECISION.to_string()),
                "[pin] must define `upstream_rustc` and `edition`",
            );
            None
        }
    }
}

/// Whether `v` is a plain `x.y.z` release number.
fn is_release(v: &str) -> bool {
    let parts: Vec<&str> = v.split('.').collect();
    parts.len() == 3
        && parts
            .iter()
            .all(|p| !p.is_empty() && p.bytes().all(|b| b.is_ascii_digit()))
}

pub fn check(root: &Path, ws: Option<&Workspace>, report: &mut Report) {
    let Some(pin) = read_pin(root, report) else {
        return;
    };
    let location = Some(TOOLCHAIN_FILE.to_string());

    // rust-toolchain.toml pins exactly the recorded release and the Target.
    if let Some(table) = read_table(root, TOOLCHAIN_FILE, report) {
        let toolchain = table.get("toolchain").and_then(Value::as_table);
        let channel = toolchain
            .and_then(|t| t.get("channel"))
            .and_then(Value::as_str)
            .unwrap_or_default();
        if !is_release(channel) {
            report.error(
                Rule::Toolchain,
                location.clone(),
                format!("channel `{channel}` is not an exact stable release such as `1.95.0`"),
            );
        } else if channel != pin.upstream_rustc {
            report.error(
                Rule::Toolchain,
                location.clone(),
                format!(
                    "channel `{channel}` differs from the pinned upstream rustc `{}` in {DECISION}",
                    pin.upstream_rustc
                ),
            );
        }
        let targets = strings(toolchain.and_then(|t| t.get("targets")));
        for target in &pin.targets {
            if !targets.contains(target) {
                report.error(
                    Rule::Toolchain,
                    location.clone(),
                    format!("the toolchain does not install the Target `{target}`"),
                );
            }
        }
    }

    // The active compiler and Cargo are that stable release.
    match stdout_of(&Cmd::rustc().arg("-vV").cwd(root)) {
        Ok(out) => {
            let version_line = out.lines().next().unwrap_or_default().to_string();
            let release = out
                .lines()
                .find_map(|l| l.strip_prefix("release: "))
                .unwrap_or_default()
                .trim()
                .to_string();
            if release != pin.upstream_rustc || !is_release(&release) {
                report.error(
                    Rule::Toolchain,
                    Some("rustc".to_string()),
                    format!(
                        "the active compiler is `{version_line}`, not the stable release {} pinned in {DECISION}",
                        pin.upstream_rustc
                    ),
                );
            } else {
                report.note(format!(
                    "R55.2: active compiler `{version_line}` matches {TOOLCHAIN_FILE} and {DECISION}"
                ));
            }
        }
        Err(e) => report.error(Rule::Step, Some("rustc".to_string()), e),
    }
    match stdout_of(&Cmd::cargo().arg("-V").cwd(root)) {
        Ok(out) => {
            let version = out.split_whitespace().nth(1).unwrap_or_default();
            if version != pin.upstream_rustc {
                report.error(
                    Rule::Toolchain,
                    Some("cargo".to_string()),
                    format!(
                        "the active Cargo is `{}`, not {}",
                        out.trim(),
                        pin.upstream_rustc
                    ),
                );
            }
        }
        Err(e) => report.error(Rule::Step, Some("cargo".to_string()), e),
    }

    // R55.5: one edition for every workspace crate.
    if let Some(ws) = ws {
        let mut count = 0usize;
        for pkg in ws.member_packages() {
            count += 1;
            if pkg.edition != pin.edition {
                report.error(
                    Rule::Edition,
                    Some(format!("{}/Cargo.toml", ws.rel(&pkg.dir))),
                    format!(
                        "crate `{}` uses edition {}; every rsk crate uses edition {} ({DECISION})",
                        pkg.name, pkg.edition, pin.edition
                    ),
                );
            }
        }
        report.note(format!(
            "R55.5: {count} workspace crates checked for edition {}",
            pin.edition
        ));
    }
}
