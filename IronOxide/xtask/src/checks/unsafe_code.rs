//! PR-20 and R6.3: `#![forbid(unsafe_code)]` placement and confinement of
//! `unsafe` code to the Kernel_Unsafe_Module.
//!
//! Rules, for every workspace crate:
//!
//! 1. Every crate root (library, binaries, tests, examples, benches, build
//!    script) applies an unconditional `#![forbid(unsafe_code)]`.
//! 2. Exception for a Kernel crate with an `unsafe-module`: its library root
//!    and every module on the path from the root to the Kernel_Unsafe_Module
//!    (the ancestors, such as `arch/mod.rs` and `arch/cm4/mod.rs`) cannot
//!    forbid the lint, because a forbidden lint cannot be re-allowed in a
//!    child module. They instead apply `#![deny(unsafe_code)]` and contain
//!    no unsafe code (rule 4), and every other module file of the library
//!    applies its own `#![forbid(unsafe_code)]`.
//! 3. Exception for the `unsafe-crate` (the entry crate): its library files
//!    are part of the Kernel_Unsafe_Module.
//! 4. Outside the Kernel_Unsafe_Module, no file contains the `unsafe`
//!    keyword, an unsafe attribute (`no_mangle`, `export_name`,
//!    `link_section`, `naked`, also as `unsafe(...)`), an assembly macro, an
//!    attribute that lowers `unsafe_code`, `include!`, or `#[path]`. The
//!    token scan includes macro bodies. The last two would bring in source
//!    text that the scan does not see.
//!
//! The compiler's `unsafe_code` lint remains the primary enforcement for the
//! forbidden modules; the scan adds the constructs that the lint misses (a
//! naked function passes `forbid(unsafe_code)` on rustc 1.95.0) and code in
//! macro bodies the compiler never expands.

use std::collections::BTreeSet;
use std::path::PathBuf;

use crate::diag::{Report, Rule};
use crate::policy::CratePolicy;
use crate::tokens::{
    has_lint_level, leading_inner_attrs, lex_file, source_inclusions, unsafe_constructs,
};
use crate::workspace::{canonical, rust_files, Package, Workspace};

pub fn check(ws: &Workspace, report: &mut Report) {
    let mut roots = 0usize;
    let mut scanned = 0usize;
    let mut unsafe_module_files = 0usize;
    for pkg in ws.member_packages() {
        let Some(policy) = ws.policy.crates.get(&pkg.name) else {
            continue; // reported by the policy check
        };
        let module = unsafe_module(ws, pkg, policy, report);
        unsafe_module_files += module.len();
        roots += check_roots(ws, pkg, policy, &module, report);
        if policy.unsafe_module.is_some() {
            check_kernel_modules(ws, pkg, &module, report);
        }
        for file in rust_files(&pkg.dir) {
            if module.contains(&file) {
                continue;
            }
            scanned += 1;
            let tokens = match lex_file(&file) {
                Ok(tokens) => tokens,
                Err(e) => {
                    report.error(Rule::UnsafeConfinement, Some(ws.rel(&file)), e);
                    continue;
                }
            };
            for finding in unsafe_constructs(&tokens) {
                report.error(
                    Rule::UnsafeConfinement,
                    Some(format!("{}:{}", ws.rel(&file), finding.line)),
                    format!("{} outside the Kernel_Unsafe_Module", finding.what),
                );
            }
            for finding in source_inclusions(&tokens) {
                report.error(
                    Rule::UnsafeConfinement,
                    Some(format!("{}:{}", ws.rel(&file), finding.line)),
                    format!(
                        "{} outside the Kernel_Unsafe_Module brings in source text that the unsafe-code \
                         scan does not cover",
                        finding.what
                    ),
                );
            }
        }
    }
    report.note(format!(
        "PR-20/R6.3: {roots} crate roots checked for forbid(unsafe_code); {scanned} files scanned; \
         {unsafe_module_files} files in the Kernel_Unsafe_Module"
    ));
}

/// The files of the Kernel_Unsafe_Module in `pkg`.
pub(crate) fn unsafe_module(
    ws: &Workspace,
    pkg: &Package,
    policy: &CratePolicy,
    report: &mut Report,
) -> BTreeSet<PathBuf> {
    let mut files = BTreeSet::new();
    if policy.unsafe_crate {
        files.extend(pkg.lib_files());
    }
    if let Some(module) = &policy.unsafe_module {
        let base = pkg.dir.join(module);
        let file = base.with_extension("rs");
        let mod_rs = base.join("mod.rs");
        if !file.is_file() && !mod_rs.is_file() {
            report.error(
                Rule::Policy,
                Some(format!("{}/Cargo.toml", ws.rel(&pkg.dir))),
                format!("unsafe-module `{module}` names no module file ({module}.rs or {module}/mod.rs)"),
            );
        }
        if file.is_file() {
            files.insert(canonical(&file));
        }
        if base.is_dir() {
            files.extend(rust_files(&base));
        }
    }
    files
}

/// Rules 1 and 2 for the crate roots. Returns the number of roots checked.
fn check_roots(
    ws: &Workspace,
    pkg: &Package,
    policy: &CratePolicy,
    module: &BTreeSet<PathBuf>,
    report: &mut Report,
) -> usize {
    let mut checked = 0;
    for target in &pkg.targets {
        if module.contains(&target.src_path) {
            continue;
        }
        checked += 1;
        let location = Some(ws.rel(&target.src_path));
        let attrs = match lex_file(&target.src_path) {
            Ok(tokens) => leading_inner_attrs(&tokens),
            Err(e) => {
                report.error(Rule::UnsafeConfinement, location, e);
                continue;
            }
        };
        if policy.unsafe_module.is_some() && target.is_lib() {
            if !has_lint_level(&attrs, &["deny", "forbid"], "unsafe_code") {
                report.error(
                    Rule::UnsafeConfinement,
                    location,
                    "the Kernel crate root lacks an unconditional `#![deny(unsafe_code)]` (it cannot forbid \
                     the lint because its Kernel_Unsafe_Module must be able to allow it)",
                );
            }
        } else if !has_lint_level(&attrs, &["forbid"], "unsafe_code") {
            report.error(
                Rule::UnsafeConfinement,
                location,
                format!(
                    "crate root of `{}` ({} target `{}`) lacks an unconditional `#![forbid(unsafe_code)]`",
                    pkg.name,
                    target.kinds.join(","),
                    target.name
                ),
            );
        }
    }
    checked
}

/// Rule 2: every module file of a Kernel library outside the
/// Kernel_Unsafe_Module applies its own `#![forbid(unsafe_code)]`.
fn check_kernel_modules(
    ws: &Workspace,
    pkg: &Package,
    module: &BTreeSet<PathBuf>,
    report: &mut Report,
) {
    let Some(lib) = pkg.lib() else { return };
    // The module files on the path to the Kernel_Unsafe_Module (`mod.rs`
    // files of its ancestor directories) may only deny the lint.
    let module_root: Option<PathBuf> = module.iter().next().and_then(|f| {
        let unsafe_dir = f.parent()?;
        Some(unsafe_dir.to_path_buf())
    });
    for file in pkg.lib_files() {
        if file == lib.src_path || module.contains(&file) {
            continue;
        }
        let is_ancestor = module_root.as_ref().is_some_and(|root| {
            file.file_name().is_some_and(|n| n == "mod.rs")
                && file.parent().is_some_and(|dir| root.starts_with(dir))
        });
        let location = Some(ws.rel(&file));
        match lex_file(&file) {
            Ok(tokens) => {
                let attrs = leading_inner_attrs(&tokens);
                if is_ancestor {
                    if !has_lint_level(&attrs, &["deny", "forbid"], "unsafe_code") {
                        report.error(
                            Rule::UnsafeConfinement,
                            location,
                            "ancestor module of the Kernel_Unsafe_Module lacks `#![deny(unsafe_code)]`",
                        );
                    }
                } else if !has_lint_level(&attrs, &["forbid"], "unsafe_code") {
                    report.error(
                        Rule::UnsafeConfinement,
                        location,
                        "Kernel module outside the Kernel_Unsafe_Module lacks its own \
                         `#![forbid(unsafe_code)]`",
                    );
                }
            }
            Err(e) => report.error(Rule::UnsafeConfinement, location, e),
        }
    }
}
