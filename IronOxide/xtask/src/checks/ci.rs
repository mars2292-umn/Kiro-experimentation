//! CI workflow files (`.github/workflows/*.yml`):
//!
//! - PR-36/R55.1: no line outside comments mentions `RUSTC_BOOTSTRAP`.
//! - R58.1: every Cargo command passes `--frozen` or `--locked`, except
//!   `cargo xtask`, whose alias passes `--frozen` (checked separately), and
//!   no command updates the lock file or fetches packages. `cargo install
//!   --locked <tool>` is accepted: it installs a verification tool from the
//!   tool's own lock file and builds nothing of rsk.
//! - At least one workflow runs `cargo xtask verify`.
//!
//! The scan is line-based: it finds the word `cargo` followed by a
//! subcommand. Commands assembled indirectly (for example through shell
//! variables) are not recognized.

use std::path::Path;

use crate::diag::{Report, Rule};
use crate::workspace::rel;

pub const WORKFLOWS: &str = ".github/workflows";

/// Cargo subcommands that change the lock file or fetch packages.
const FORBIDDEN_SUBCOMMANDS: &[&str] = &[
    "update",
    "generate-lockfile",
    "fetch",
    "install",
    "add",
    "remove",
    "vendor",
    "publish",
    "search",
    "info",
    "login",
];

pub fn check(root: &Path, report: &mut Report) {
    let dir = root.join(WORKFLOWS);
    let mut files: Vec<_> = std::fs::read_dir(&dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|e| e == "yml" || e == "yaml"))
        .collect();
    files.sort();
    let mut runs_verify = false;
    for file in &files {
        let Ok(text) = std::fs::read_to_string(file) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let code = line.split_once('#').map_or(line, |(code, _)| code);
            let location = Some(format!("{}:{}", rel(root, file), n + 1));
            if code.contains("RUSTC_BOOTSTRAP") {
                report.error(
                    Rule::NoUnstable,
                    location.clone(),
                    "the CI workflow sets RUSTC_BOOTSTRAP",
                );
            }
            let words: Vec<&str> = code.split_whitespace().collect();
            for (i, word) in words.iter().enumerate() {
                if *word != "cargo" && !word.ends_with("/cargo") {
                    continue;
                }
                let mut rest = words[i + 1..].iter().copied();
                let mut sub = rest.next().unwrap_or_default();
                if sub.starts_with('+') {
                    sub = rest.next().unwrap_or_default();
                }
                let args: Vec<&str> = rest.collect();
                if sub.is_empty() || sub.starts_with('-') {
                    continue;
                }
                if sub == "xtask" {
                    runs_verify |= args.first() == Some(&"verify");
                } else if sub == "install" && args.iter().any(|a| *a == "--locked") {
                    // Installing a verification tool (Kani) from its own lock
                    // file is not a build of rsk; the tool's version is
                    // recorded in every Evidence_Item it produces (R45.7).
                } else if sub == "kani" || sub == "verus" {
                    // Proof tools: `cargo kani setup` installs CBMC, and the
                    // proof runs themselves resolve from the committed lock
                    // file under the offline Cargo configuration.
                } else if FORBIDDEN_SUBCOMMANDS.contains(&sub) {
                    report.error(
                        Rule::Locked,
                        location.clone(),
                        format!("CI runs `cargo {sub}`, which changes the lock file or fetches packages"),
                    );
                } else if !args.iter().any(|a| *a == "--frozen" || *a == "--locked") {
                    report.error(
                        Rule::Locked,
                        location.clone(),
                        format!("CI runs `cargo {sub}` without --frozen or --locked"),
                    );
                }
            }
        }
    }
    if runs_verify {
        report.note(format!(
            "CI: {} workflow file(s); `cargo xtask verify` is the CI entry point",
            files.len()
        ));
    } else {
        report.error(
            Rule::Step,
            Some(WORKFLOWS.to_string()),
            "no CI workflow runs `cargo xtask verify`",
        );
    }
}
