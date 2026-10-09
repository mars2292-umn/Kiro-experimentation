//! R6.1 (PR-15, PR-35): the Kernel consists of `#![no_std]` crates whose
//! dependency graphs for the Target, including transitive dependencies,
//! contain neither `std` nor `alloc`.
//!
//! The Kernel's Target graph is the closure of the Kernel crates over normal
//! dependencies in `cargo metadata --filter-platform <Target>`. Build
//! dependencies and proc-macros run on the host and are not linked, so they
//! are excluded here and covered by the PR-36 checks instead.
//!
//! A crate "uses `std` or `alloc`" if it is not `no_std` or if it declares
//! `extern crate std` or `extern crate alloc`. On a stable toolchain those
//! are the only ways to link either library, because neither is in the
//! extern prelude of a `no_std` crate. Workspace crates must state
//! `#![no_std]` unconditionally and may not declare either `extern crate` at
//! all. Third-party crates are judged by the configuration they were actually
//! compiled with for the Target, after the verification job's Target build.
//!
//! Limits: the Target build fails for `std` anyway, because no `std` is
//! shipped for `thumbv7em-none-eabihf`, but `alloc` is shipped, so the
//! explicit check above is what excludes it. `extern crate` items that a
//! macro generates and modules outside the library directory are not seen.
//! The link-level allocator-symbol check is task 12.3.

use std::collections::BTreeSet;

use crate::cfg::Tri;
use crate::diag::{Report, Rule};
use crate::tokens::{extern_crates, leading_inner_attrs, lex_file, no_std_metas, ItemCfg};
use crate::units::{CfgContext, Outcome};
use crate::workspace::{Package, Workspace};

const FORBIDDEN: &[&str] = &["std", "alloc"];

/// The package ids of the Kernel's Target graph.
pub fn kernel_graph(ws: &Workspace) -> BTreeSet<String> {
    ws.target_closure(ws.policy.kernel_crates())
}

/// Static part: every workspace crate in the Kernel's Target graph.
pub fn check(ws: &Workspace, report: &mut Report) {
    let graph = kernel_graph(ws);
    let mut third_party = Vec::new();
    for id in &graph {
        let Some(pkg) = ws.packages.get(id) else {
            continue;
        };
        if pkg.member {
            check_member(ws, pkg, report);
        } else {
            third_party.push(format!("{} {}", pkg.name, pkg.version));
        }
    }
    let names: Vec<&str> = graph
        .iter()
        .filter_map(|id| ws.packages.get(id))
        .map(|p| p.name.as_str())
        .collect();
    if graph.is_empty() {
        report.note("R6.1: the build policy names no Kernel crate");
    } else {
        report.note(format!(
            "R6.1: the Kernel's {} graph has {} crates ({}), {} of them third-party",
            ws.policy.target,
            graph.len(),
            names.join(", "),
            third_party.len()
        ));
    }
}

fn check_member(ws: &Workspace, pkg: &Package, report: &mut Report) {
    let Some(lib) = pkg.lib() else {
        report.error(
            Rule::KernelNoStd,
            Some(format!("{}/Cargo.toml", ws.rel(&pkg.dir))),
            format!(
                "crate `{}` in the Kernel's Target graph has no library target",
                pkg.name
            ),
        );
        return;
    };
    let location = Some(ws.rel(&lib.src_path));
    match lex_file(&lib.src_path) {
        Ok(tokens) => {
            let unconditional = no_std_metas(&leading_inner_attrs(&tokens))
                .iter()
                .any(|m| !m.conditional());
            if !unconditional {
                report.error(
                    Rule::KernelNoStd,
                    location,
                    format!(
                        "crate `{}` is in the Kernel's Target graph but its root lacks an unconditional \
                         `#![no_std]`",
                        pkg.name
                    ),
                );
            }
        }
        Err(e) => report.error(Rule::KernelNoStd, location, e),
    }
    for file in pkg.lib_files() {
        let tokens = match lex_file(&file) {
            Ok(tokens) => tokens,
            Err(e) => {
                report.error(Rule::KernelNoStd, Some(ws.rel(&file)), e);
                continue;
            }
        };
        for item in extern_crates(&tokens) {
            if FORBIDDEN.contains(&item.name.as_str()) {
                report.error(
                    Rule::KernelNoStd,
                    Some(format!("{}:{}", ws.rel(&file), item.line)),
                    format!(
                        "`extern crate {}` in crate `{}` of the Kernel's Target graph",
                        item.name, pkg.name
                    ),
                );
            }
        }
    }
}

/// Post-build part: third-party crates of the Kernel's Target graph, judged
/// by the configuration of their Target compilation unit.
pub fn check_third_party(ws: &Workspace, outcome: &Outcome, ctx: &CfgContext, report: &mut Report) {
    for id in kernel_graph(ws) {
        let Some(pkg) = ws.packages.get(&id) else {
            continue;
        };
        if pkg.member {
            continue;
        }
        let unit = outcome
            .units
            .iter()
            .find(|u| u.package_id == id && u.for_target && !u.is_build_script());
        let Some(unit) = unit else {
            report.error(
                Rule::KernelNoStd,
                None,
                format!(
                    "no Target compilation unit of `{}` was found in the Target build",
                    pkg.name
                ),
            );
            continue;
        };
        let cfg = ctx.for_unit(unit, &outcome.scripts);
        let Some(lib) = pkg.lib() else { continue };
        match lex_file(&lib.src_path) {
            Ok(tokens) => {
                let active = no_std_metas(&leading_inner_attrs(&tokens))
                    .iter()
                    .any(|m| cfg.eval_all(&m.conds) == Tri::True);
                if !active {
                    report.error(
                        Rule::KernelNoStd,
                        Some(ws.rel(&lib.src_path)),
                        format!(
                            "third-party crate `{} {}` in the Kernel's Target graph is not `no_std` in its \
                             Target configuration",
                            pkg.name, pkg.version
                        ),
                    );
                }
            }
            Err(e) => report.error(Rule::KernelNoStd, Some(ws.rel(&lib.src_path)), e),
        }
        for file in pkg.lib_files() {
            let Ok(tokens) = lex_file(&file) else {
                continue;
            };
            for item in extern_crates(&tokens) {
                if !FORBIDDEN.contains(&item.name.as_str()) {
                    continue;
                }
                let compiled = item.cfgs.iter().fold(Tri::True, |acc, c| {
                    acc.and(match c {
                        ItemCfg::Pred(p) => cfg.eval(p),
                        ItemCfg::Unknown => Tri::Unknown,
                    })
                });
                if compiled != Tri::False {
                    report.error(
                        Rule::KernelNoStd,
                        Some(format!("{}:{}", ws.rel(&file), item.line)),
                        format!(
                            "third-party crate `{} {}` in the Kernel's Target graph declares `extern crate {}`",
                            pkg.name, pkg.version, item.name
                        ),
                    );
                }
            }
        }
    }
}
