//! Cargo configuration and environment: vendored sources (R58.1), offline
//! Cargo (R58.4), and `RUSTC_BOOTSTRAP` (PR-36/R55.1).
//!
//! Cargo merges the `.cargo/config.toml` files of the working directory and
//! all its parents, nearer files taking precedence, and then
//! `$CARGO_HOME/config.toml`. The checks read the same files in the same
//! order, so a fixture inside the repository inherits the repository's
//! configuration exactly as Cargo does.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::diag::{Report, Rule};
use crate::workspace::{canonical, rel};

/// One Cargo configuration file.
#[derive(Debug)]
pub struct ConfigFile {
    pub path: PathBuf,
    /// The directory that contains `.cargo/`; relative paths in the file
    /// are resolved against it.
    pub base: PathBuf,
    pub text: String,
    pub table: Result<Table, String>,
}

/// The configuration files Cargo reads for `root`, nearest first.
pub fn config_files(root: &Path) -> Vec<ConfigFile> {
    let cargo_home = std::env::var_os("CARGO_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".cargo")));
    let mut candidates: Vec<(PathBuf, PathBuf)> = canonical(root)
        .ancestors()
        .map(|d| (d.join(".cargo"), d.to_path_buf()))
        .collect();
    if let Some(home) = cargo_home {
        let base = home
            .parent()
            .map(Path::to_path_buf)
            .unwrap_or_else(|| home.clone());
        candidates.push((home, base));
    }
    let mut seen: Vec<PathBuf> = Vec::new();
    let mut files = Vec::new();
    for (dir, base) in candidates {
        let path = [dir.join("config.toml"), dir.join("config")]
            .into_iter()
            .find(|p| p.is_file());
        let Some(path) = path else { continue };
        let key = canonical(&path);
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let text = std::fs::read_to_string(&path).unwrap_or_default();
        let table = text.parse::<Table>().map_err(|e| e.to_string());
        files.push(ConfigFile {
            path,
            base,
            text,
            table,
        });
    }
    files
}

/// The nearest definition of a dotted key.
fn lookup<'a>(files: &'a [ConfigFile], keys: &[&str]) -> Option<(&'a ConfigFile, &'a Value)> {
    files.iter().find_map(|file| {
        let table = file.table.as_ref().ok()?;
        let (first, rest) = keys.split_first()?;
        let mut value = table.get(*first)?;
        for key in rest {
            value = value.as_table()?.get(*key)?;
        }
        Some((file, value))
    })
}

/// The vendored source directory that replaces crates.io, if configured.
pub fn vendor_dir(root: &Path) -> Option<PathBuf> {
    let files = config_files(root);
    let (_, name) = lookup(&files, &["source", "crates-io", "replace-with"])?;
    let name = name.as_str()?;
    let (file, dir) = lookup(&files, &["source", name, "directory"])?;
    Some(file.base.join(dir.as_str()?))
}

pub fn check(root: &Path, report: &mut Report) {
    let files = config_files(root);
    for file in &files {
        if let Err(e) = &file.table {
            report.error(
                Rule::Step,
                Some(rel(root, &file.path)),
                format!("cannot parse Cargo configuration: {e}"),
            );
        }
    }

    // R58.1: crates.io is replaced by an existing vendored directory.
    match lookup(&files, &["source", "crates-io", "replace-with"]) {
        Some((_, Value::String(name))) => match lookup(&files, &["source", name, "directory"]) {
            Some((file, Value::String(dir))) => {
                let path = file.base.join(dir);
                if path.is_dir() {
                    report.note(format!(
                        "R58.1: crates.io is replaced by the vendored directory source `{}` ({})",
                        rel(root, &path),
                        rel(root, &file.path)
                    ));
                } else {
                    report.error(
                        Rule::Locked,
                        Some(rel(root, &file.path)),
                        format!("the vendored source directory `{}` does not exist", path.display()),
                    );
                }
            }
            _ => report.error(
                Rule::Locked,
                None,
                format!("source `{name}`, which replaces crates.io, is not a vendored `directory` source"),
            ),
        },
        _ => report.error(
            Rule::Locked,
            None,
            "no Cargo configuration replaces crates.io with a vendored directory source \
             ([source.crates-io] replace-with)",
        ),
    }

    // R58.4: Cargo is configured offline.
    match lookup(&files, &["net", "offline"]) {
        Some((file, Value::Boolean(true))) => {
            report.note(format!(
                "R58.4: `net.offline = true` ({})",
                rel(root, &file.path)
            ));
        }
        Some((file, _)) => report.error(
            Rule::Offline,
            Some(rel(root, &file.path)),
            "`net.offline` is not `true` in the nearest Cargo configuration that sets it",
        ),
        None => report.error(
            Rule::Offline,
            None,
            "no Cargo configuration sets `[net] offline = true`",
        ),
    }

    // PR-36/R55.1: no configuration file mentions RUSTC_BOOTSTRAP, whether
    // in [env] or anywhere else (for example in a runner command).
    for file in &files {
        for (n, line) in file.text.lines().enumerate() {
            if !line.trim_start().starts_with('#') && line.contains("RUSTC_BOOTSTRAP") {
                report.error(
                    Rule::NoUnstable,
                    Some(format!("{}:{}", rel(root, &file.path), n + 1)),
                    "Cargo configuration sets RUSTC_BOOTSTRAP",
                );
            }
        }
    }
}

/// PR-36/R55.1: `RUSTC_BOOTSTRAP` is not set in the environment. The job
/// also removes it from every command it runs, so that its builds stay valid
/// stable builds, but its presence still fails the job.
pub fn check_environment(report: &mut Report) {
    if std::env::var_os("RUSTC_BOOTSTRAP").is_some() {
        report.error(
            Rule::NoUnstable,
            Some("environment".to_string()),
            "RUSTC_BOOTSTRAP is set in the environment of the verification job",
        );
    }
}

/// R58.1: the `xtask` alias that CI calls passes `--frozen` or `--locked`.
pub fn check_alias(root: &Path, report: &mut Report) {
    let files = config_files(root);
    match lookup(&files, &["alias", "xtask"]) {
        Some((file, value)) => {
            let words: Vec<String> = match value {
                Value::String(s) => s.split_whitespace().map(str::to_string).collect(),
                Value::Array(a) => a
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect(),
                _ => Vec::new(),
            };
            if !words.iter().any(|w| w == "--frozen" || w == "--locked") {
                report.error(
                    Rule::Locked,
                    Some(rel(root, &file.path)),
                    "the `xtask` alias does not pass --frozen or --locked",
                );
            }
        }
        None => report.error(Rule::Locked, None, "no `xtask` alias is configured"),
    }
}
