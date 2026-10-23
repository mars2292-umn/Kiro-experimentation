//! R3.4: compiler flags and `[lints]` tables that lower a Profile_Lint lint
//! below error, in the workspace configuration and in the environment.

use std::path::Path;

use crate::{Finding, Report};

/// The lints the Profile_Lint owns: Clippy's deny-lists (PR-10, PR-11,
/// PR-22, PR-35) and the groups that contain them.
pub const PROFILE_LINTS: &[&str] = &["clippy::disallowed_methods", "clippy::disallowed_types", "clippy::disallowed_macros", "clippy::all", "clippy::style", "warnings"];

pub fn is_profile_lint(name: &str) -> bool {
    let n: String = name.chars().filter(|c| !c.is_whitespace()).collect();
    PROFILE_LINTS.contains(&n.as_str())
}

/// `-A x`, `-Ax`, `--allow x`, `-W`, `--warn`, `--cap-lints` in a flag list.
pub fn lowering_flags(flags: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < flags.len() {
        let f = &flags[i];
        let (opt, arg) = if let Some(rest) = f.strip_prefix("-A").filter(|r| !r.is_empty() && !r.starts_with('-')) {
            ("-A", rest.to_string())
        } else if let Some(rest) = f.strip_prefix("-W").filter(|r| !r.is_empty() && !r.starts_with('-')) {
            ("-W", rest.to_string())
        } else if let Some(rest) = f.strip_prefix("--cap-lints=") {
            ("--cap-lints", rest.to_string())
        } else if matches!(f.as_str(), "-A" | "--allow" | "-W" | "--warn" | "--cap-lints") {
            i += 1;
            (f.as_str(), flags.get(i).cloned().unwrap_or_default())
        } else {
            i += 1;
            continue;
        };
        if opt == "--cap-lints" {
            if arg != "deny" && arg != "forbid" {
                out.push(format!("--cap-lints {arg}"));
            }
        } else if is_profile_lint(&arg) {
            out.push(format!("{opt} {arg}"));
        }
        i += 1;
    }
    out
}

/// The `rustflags` lists of a `.cargo/config.toml` text.
pub fn config_rustflags(text: &str) -> Vec<Vec<String>> {
    let Ok(table) = text.parse::<toml::Table>() else { return vec![] };
    let mut out = Vec::new();
    let list = |v: &toml::Value| -> Vec<String> {
        match v {
            toml::Value::String(s) => s.split_whitespace().map(String::from).collect(),
            toml::Value::Array(a) => a.iter().filter_map(|x| x.as_str().map(String::from)).collect(),
            _ => vec![],
        }
    };
    if let Some(v) = table.get("build").and_then(|b| b.get("rustflags")) {
        out.push(list(v));
    }
    if let Some(targets) = table.get("target").and_then(|t| t.as_table()) {
        for (_, t) in targets {
            if let Some(v) = t.get("rustflags") {
                out.push(list(v));
            }
        }
    }
    if let Some(env) = table.get("env").and_then(|e| e.as_table()) {
        for (k, v) in env {
            if k == "RUSTFLAGS" {
                out.push(list(v));
            }
        }
    }
    out
}

/// `[lints]` and `[workspace.lints]` entries of a Cargo.toml text that set a
/// Profile_Lint lint to `allow` or `warn`.
pub fn lints_table_lowerings(text: &str) -> Vec<String> {
    let Ok(table) = text.parse::<toml::Table>() else { return vec![] };
    let mut out = Vec::new();
    let mut tables = Vec::new();
    if let Some(l) = table.get("lints") {
        tables.push(l);
    }
    if let Some(l) = table.get("workspace").and_then(|w| w.get("lints")) {
        tables.push(l);
    }
    for l in tables {
        let Some(l) = l.as_table() else { continue };
        for (tool, entries) in l {
            let Some(entries) = entries.as_table() else { continue };
            for (lint, v) in entries {
                let level = match v {
                    toml::Value::String(s) => s.clone(),
                    toml::Value::Table(t) => t.get("level").and_then(|x| x.as_str()).unwrap_or("").to_string(),
                    _ => String::new(),
                };
                let name = if tool == "rust" { lint.clone() } else { format!("{tool}::{lint}") };
                if is_profile_lint(&name) && (level == "allow" || level == "warn") {
                    out.push(format!("[lints] {name} = \"{level}\""));
                }
            }
        }
    }
    out
}

pub fn check_workspace(root: &Path, files: &[std::path::PathBuf], report: &mut Report) {
    let config = root.join(".cargo").join("config.toml");
    if let Ok(text) = std::fs::read_to_string(&config) {
        for flags in config_rustflags(&text) {
            for f in lowering_flags(&flags) {
                report.findings.push(Finding { rule: "R3.4", file: ".cargo/config.toml".into(), line: 0, message: format!("`{f}` lowers a Profile_Lint lint (PR-10, PR-11, PR-22, PR-35)") });
            }
        }
    }
    let mut manifests = vec![root.join("Cargo.toml")];
    for f in files {
        let mut d = f.parent();
        while let Some(dir) = d {
            let m = dir.join("Cargo.toml");
            if m.is_file() {
                if !manifests.contains(&m) {
                    manifests.push(m);
                }
                break;
            }
            d = dir.parent();
        }
    }
    for m in manifests {
        if let Ok(text) = std::fs::read_to_string(&m) {
            for l in lints_table_lowerings(&text) {
                let name = m.strip_prefix(root).unwrap_or(&m).display().to_string();
                report.findings.push(Finding { rule: "R3.4", file: name, line: 0, message: format!("{l} lowers a Profile_Lint lint") });
            }
        }
    }
}

pub fn check_environment(report: &mut Report) {
    for var in ["RUSTFLAGS", "CARGO_BUILD_RUSTFLAGS", "CARGO_ENCODED_RUSTFLAGS"] {
        if let Ok(v) = std::env::var(var) {
            let flags: Vec<String> = if var == "CARGO_ENCODED_RUSTFLAGS" { v.split('\x1f').map(String::from).collect() } else { v.split_whitespace().map(String::from).collect() };
            for f in lowering_flags(&flags) {
                report.findings.push(Finding { rule: "R3.4", file: format!("${var}"), line: 0, message: format!("`{f}` in the environment lowers a Profile_Lint lint") });
            }
        }
    }
    for (k, v) in std::env::vars() {
        if k.starts_with("CARGO_TARGET_") && k.ends_with("_RUSTFLAGS") {
            for f in lowering_flags(&v.split_whitespace().map(String::from).collect::<Vec<_>>()) {
                report.findings.push(Finding { rule: "R3.4", file: format!("${k}"), line: 0, message: format!("`{f}` in the environment lowers a Profile_Lint lint") });
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flags_and_tables() {
        let f = |s: &str| lowering_flags(&s.split_whitespace().map(String::from).collect::<Vec<_>>());
        assert_eq!(f("--cap-lints warn"), vec!["--cap-lints warn"]);
        assert_eq!(f("--cap-lints=allow -D warnings"), vec!["--cap-lints allow"]);
        assert_eq!(f("-A clippy::disallowed_methods"), vec!["-A clippy::disallowed_methods"]);
        assert_eq!(f("-Aclippy::all"), vec!["-A clippy::all"]);
        assert_eq!(f("-W warnings"), vec!["-W warnings"]);
        assert!(f("-A dead_code -D clippy::disallowed_methods -C opt-level=3").is_empty());
        assert_eq!(config_rustflags("[build]\nrustflags = [\"--cap-lints\", \"warn\"]\n[target.thumbv7em-none-eabihf]\nrustflags = \"-A clippy::all\"\n"), vec![vec!["--cap-lints", "warn"], vec!["-A", "clippy::all"]]);
        assert_eq!(lints_table_lowerings("[lints.clippy]\ndisallowed_methods = \"allow\"\n[lints.rust]\nwarnings = { level = \"warn\" }\nunused = \"allow\"\n"), vec!["[lints] clippy::disallowed_methods = \"allow\"", "[lints] warnings = \"warn\""]);
        assert_eq!(lints_table_lowerings("[workspace.lints.clippy]\nall = \"warn\"\ndisallowed_types = \"deny\"\n"), vec!["[lints] clippy::all = \"warn\""]);
    }
}
