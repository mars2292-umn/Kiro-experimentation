//! PR-36 and R55.1: no `#![feature]` attribute in any crate that a
//! Flight_Build compiles, and no `RUSTC_BOOTSTRAP` set by a build script.
//!
//! Decision on `cfg_attr`-gated feature attributes:
//!
//! - Workspace crates: every `feature(...)` attribute is a violation,
//!   unconditional or inside any `cfg_attr`, whatever the condition.
//! - Third-party crates compiled by the Flight_Build (vendored, including
//!   proc-macros and build scripts): only a leading inner attribute of the
//!   unit's crate root can enable an unstable feature. rustc reads the
//!   feature set from the crate root alone; a `feature` attribute anywhere
//!   else (an inner attribute of a submodule file, or an outer attribute on
//!   an item) is inert and only produces the warning "the `#![feature]`
//!   attribute can only be used at the crate root" (checked on rustc
//!   1.95.0; vstd's `string.rs` and `std_specs/alloc.rs` are examples). Such
//!   placements are counted and noted, not rejected. At the crate root, an
//!   unconditional `#![feature]` is a violation. A `cfg_attr(P, feature(...))`
//!   is accepted only if `P` is false in the configuration the crate was
//!   actually compiled with (its Target or host `rustc --print cfg`, its
//!   enabled features, and its build script's cfgs, from Cargo's JSON
//!   messages; builds run with empty `RUSTFLAGS`). Typical accepted gates are
//!   `docsrs`, `verus_keep_ghost` (set only by the Verus_Toolchain), features
//!   such as serde_core's `unstable`, and cfgs that a build script sets only
//!   when a nightly probe compiles, such as proc-macro2's `proc_macro_span`.
//!   A condition that is true or that the evaluator cannot decide is a
//!   violation.
//!
//! The authoritative evidence remains that the build succeeds on the pinned
//! stable toolchain with `RUSTC_BOOTSTRAP` unset: stable rustc rejects any
//! active `#![feature]` with error E0554, and Cargo rejects a build script
//! that tries to set `RUSTC_BOOTSTRAP`.

use std::path::{Path, PathBuf};

use crate::cfg::Tri;
use crate::diag::{Report, Rule};
use crate::tokens::{feature_metas, leading_inner_attrs, lex_file, metas, render, Meta};
use crate::units::{CfgContext, Outcome, Unit};
use crate::workspace::{canonical, rust_files, Workspace};

fn describe(meta: &Meta) -> String {
    let args = render(meta.args.as_deref().unwrap_or(&[]));
    if meta.conditional() {
        let conds: Vec<String> = meta.conds.iter().map(|c| render(c)).collect();
        format!("`feature({args})` inside `cfg_attr({})`", conds.join(", "))
    } else {
        format!("`#![feature({args})]`")
    }
}

/// Workspace crates: any `feature(...)` attribute.
pub fn check_sources(ws: &Workspace, report: &mut Report) {
    let mut files = 0usize;
    let mut found = 0usize;
    for pkg in ws.member_packages() {
        for file in rust_files(&pkg.dir) {
            files += 1;
            let tokens = match lex_file(&file) {
                Ok(tokens) => tokens,
                Err(e) => {
                    report.error(Rule::NoUnstable, Some(ws.rel(&file)), e);
                    continue;
                }
            };
            for meta in feature_metas(&tokens) {
                found += 1;
                report.error(
                    Rule::NoUnstable,
                    Some(format!("{}:{}", ws.rel(&file), meta.line)),
                    format!("{} in workspace crate `{}`", describe(&meta), pkg.name),
                );
            }
        }
    }
    report.note(format!(
        "PR-36/R55.1: {files} workspace source files scanned, {found} feature attributes found"
    ));
}

/// The files to scan for a third-party unit.
fn unit_files(ws: &Workspace, unit: &Unit) -> Vec<PathBuf> {
    if unit.is_build_script() {
        return vec![unit.src_path.clone()];
    }
    match ws.packages.get(&unit.package_id) {
        Some(pkg)
            if pkg
                .lib()
                .is_some_and(|l| l.src_path == crate::workspace::canonical(&unit.src_path)) =>
        {
            pkg.lib_files()
        }
        _ => vec![unit.src_path.clone()],
    }
}

/// Third-party crates compiled by the Flight_Build.
pub fn check_flight_units(
    ws: &Workspace,
    outcome: &Outcome,
    ctx: &CfgContext,
    report: &mut Report,
) {
    let mut third_party = 0usize;
    let mut inactive = Vec::new();
    let mut inert = 0usize;
    for unit in &outcome.units {
        let Some(pkg) = ws.packages.get(&unit.package_id) else {
            continue;
        };
        if pkg.member {
            continue; // covered by check_sources
        }
        third_party += 1;
        let cfg = ctx.for_unit(unit, &outcome.scripts);
        let root = canonical(&unit.src_path);
        for file in unit_files(ws, unit) {
            let tokens = match lex_file(&file) {
                Ok(tokens) => tokens,
                Err(e) => {
                    report.error(Rule::NoUnstable, Some(ws.rel(&file)), e);
                    continue;
                }
            };
            let everywhere = feature_metas(&tokens).len();
            let at_root: Vec<Meta> = if canonical(&file) == root {
                leading_inner_attrs(&tokens)
                    .iter()
                    .flat_map(metas)
                    .filter(|m| m.name == "feature" && m.args.is_some())
                    .collect()
            } else {
                Vec::new()
            };
            inert += everywhere - at_root.len();
            for meta in at_root {
                let location = Some(format!("{}:{}", ws.rel(&file), meta.line));
                let verdict = if meta.conditional() {
                    cfg.eval_all(&meta.conds)
                } else {
                    Tri::True
                };
                match verdict {
                    Tri::False => {
                        inactive.push(format!("{} {}: {}", pkg.name, pkg.version, describe(&meta)))
                    }
                    Tri::True => report.error(
                        Rule::NoUnstable,
                        location,
                        format!(
                            "{} is active in Flight_Build crate `{} {}`",
                            describe(&meta),
                            pkg.name,
                            pkg.version
                        ),
                    ),
                    Tri::Unknown => report.error(
                        Rule::NoUnstable,
                        location,
                        format!(
                            "{} in Flight_Build crate `{} {}` cannot be shown inactive",
                            describe(&meta),
                            pkg.name,
                            pkg.version
                        ),
                    ),
                }
            }
        }
    }
    report.note(format!(
        "PR-36/R55.1: the Target build (Flight_Build roots and other Target crates) compiled {} units, \
         {third_party} of them third-party; {} inactive cfg_attr feature gates at crate roots; \
         {inert} inert feature attributes outside crate roots (ignored by rustc)",
        outcome.units.len(),
        inactive.len()
    ));
    for gate in inactive {
        report.note(format!("  inactive: {gate}"));
    }
    for script in &outcome.scripts {
        for (key, _) in &script.env {
            if key == "RUSTC_BOOTSTRAP" {
                report.error(
                    Rule::NoUnstable,
                    None,
                    format!(
                        "the build script of `{}` sets RUSTC_BOOTSTRAP",
                        script.package_id
                    ),
                );
            }
        }
    }
}

/// Every build-script `output` file under `target_dir`: none may set
/// `RUSTC_BOOTSTRAP` through `cargo:rustc-env` or `cargo::rustc-env`.
pub fn check_build_script_outputs(ws: &Workspace, target_dir: &Path, report: &mut Report) {
    let mut outputs = Vec::new();
    find_outputs(target_dir, 0, &mut outputs);
    for output in &outputs {
        let Ok(text) = std::fs::read_to_string(output) else {
            continue;
        };
        for (n, line) in text.lines().enumerate() {
            let directive = line
                .strip_prefix("cargo::")
                .or_else(|| line.strip_prefix("cargo:"));
            if directive.is_some_and(|d| d.starts_with("rustc-env=RUSTC_BOOTSTRAP")) {
                report.error(
                    Rule::NoUnstable,
                    Some(format!("{}:{}", ws.rel(output), n + 1)),
                    "a build script sets RUSTC_BOOTSTRAP",
                );
            }
        }
    }
    report.note(format!(
        "PR-36/R55.1: {} build-script outputs scanned; none sets RUSTC_BOOTSTRAP",
        outputs.len()
    ));
}

fn find_outputs(dir: &Path, depth: usize, out: &mut Vec<PathBuf>) {
    if depth > 6 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            find_outputs(&path, depth + 1, out);
        } else if path.file_name().is_some_and(|n| n == "output")
            && path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .is_some_and(|n| n == "build")
        {
            out.push(path);
        }
    }
}
