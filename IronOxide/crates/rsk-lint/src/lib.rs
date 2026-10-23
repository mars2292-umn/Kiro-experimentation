//! rsk-lint, the Profile_Lint (task 11.1): the source-level checks of the
//! restrictions whose enforcement lists LN, over the Kernel crates, the
//! Application crates, and the code the Generator emits (R2.2).
//!
//! Each violation names the PR identifier and the source span (R3.2). The
//! checks:
//!
//! - PR-09: no `async` function, `async` block, or `.await` (R20).
//! - PR-16: every `for`, `while`, and `loop` has a Loop_Bound_Annotation
//!   (`#[rsk::bounds('label = N)]` on the enclosing function, R35.1) or an
//!   iteration count derivable from compile-time constants: a `for` over a
//!   range of constants, or a `while` whose condition conjoins `i < N` with
//!   a constant `N`; inside `verus!` a loop carries `decreases` (R35.2);
//!   a bound is an integer constant or an expression over compile-time and
//!   Generated_Config constants (R35.5); every annotation is exported with
//!   its location and function (R35.3). A `loop {}` after a diverging call
//!   and the idle loop (`loop { wait_for_interrupt() }`, R7.6) are the two
//!   non-terminating loops the Profile admits, reported as such.
//! - PR-21: every explicit `core::` path (a `use` or an inline path) maps
//!   to a Core_Subset item of the file's scope (R57.2). Items the prelude
//!   brings in without a path, and methods resolved by type, are outside
//!   the source lint; the Link_Checker's symbol mapping covers them
//!   (R57.3).
//! - R3.4: no attribute lowers a Profile_Lint lint below error, and no
//!   compiler flag of the workspace configuration, the environment, or a
//!   `[lints]` table does.
//! - PR-05: no `static mut` and no interior-mutable static outside the
//!   Kernel. PR-17: no direct recursion. PR-18: no `dyn` type and no
//!   function-pointer type in Partition code.
//!
//! Dylint, which the design named, is a nightly-only tool that is not
//! vendored (R58.1); the lint parses the source with the vendored `syn`
//! instead. It is a verification aid (TQL-5) backed by the Link_Checker's
//! binary checks and the compiler (design.md, Profile_Lint).
#![forbid(unsafe_code)]

pub mod cfg;
pub mod config;
pub mod core_subset;
pub mod lints;
pub mod profile_version;
pub mod verus;

use std::path::{Path, PathBuf};

/// Which code a file is (R2.2; the scopes of the Core_Subset).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Scope {
    Kernel,
    KernelUnsafe,
    Application,
    /// Code the Generator emitted (an Application scope for PR-21; exempt
    /// from PR-18, which admits function pointers in generated tables).
    Generated,
}

impl Scope {
    pub fn core_scope(self) -> &'static str {
        match self {
            Scope::Kernel => "kernel",
            Scope::KernelUnsafe => "kernel-unsafe",
            Scope::Application | Scope::Generated => "application",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Finding {
    pub rule: &'static str,
    pub file: String,
    pub line: usize,
    pub message: String,
}

/// One loop of the covered code (R35.3).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LoopRecord {
    pub file: String,
    pub line: usize,
    pub function: String,
    pub label: Option<String>,
    /// `annotated`, `derived`, `verus`, or `exempt`.
    pub kind: &'static str,
    pub bound: Option<String>,
}

#[derive(Default)]
pub struct Options {
    pub kernel_dirs: Vec<PathBuf>,
    /// Path prefixes (files or directories) of the Kernel_Unsafe_Module.
    pub unsafe_paths: Vec<PathBuf>,
    pub app_dirs: Vec<PathBuf>,
    pub gen_dirs: Vec<PathBuf>,
    pub root: Option<PathBuf>,
    pub profile_toml: String,
    pub core_subset_toml: String,
    pub task_model_json: Option<String>,
}

#[derive(Default, Debug)]
pub struct Report {
    pub findings: Vec<Finding>,
    pub loops: Vec<LoopRecord>,
    pub notes: Vec<String>,
    pub files: usize,
}

impl Report {
    /// `rsk-loop-bounds/1`: every loop with its bound (R35.3).
    pub fn export(&self) -> String {
        let loops: Vec<serde_json::Value> = self
            .loops
            .iter()
            .map(|l| serde_json::json!({ "file": l.file, "line": l.line, "function": l.function, "label": l.label, "kind": l.kind, "bound": l.bound }))
            .collect();
        let v = serde_json::json!({
            "schema": "rsk-loop-bounds/1",
            "profile_version": profile_version::PROFILE_VERSION,
            "loops": loops,
            "violations": self.findings.iter().map(|f| serde_json::json!({ "rule": f.rule, "file": f.file, "line": f.line, "message": f.message })).collect::<Vec<_>>(),
        });
        serde_json::to_string_pretty(&v).unwrap_or_default() + "\n"
    }

    pub fn summary(&self) -> String {
        let annotated = self.loops.iter().filter(|l| l.kind == "annotated").count();
        let derived = self.loops.iter().filter(|l| l.kind == "derived").count();
        let verus = self.loops.iter().filter(|l| l.kind == "verus").count();
        let exempt = self.loops.iter().filter(|l| l.kind == "exempt").count();
        if self.findings.is_empty() {
            format!("{} files, {} loops bounded ({annotated} annotated, {derived} derived from constants, {verus} by `decreases`, {exempt} non-terminating by design), no violation", self.files, self.loops.len())
        } else {
            format!("{} files, {} violation(s)", self.files, self.findings.len())
        }
    }
}

/// The Rust sources under `dir`, skipping `target`, `tests`, `benches`,
/// and `examples` (not Flight_Build code).
fn sources(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = entries.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if !matches!(name, "target" | "tests" | "benches" | "examples" | "gen") {
                sources(&p, out);
            }
        } else if name.ends_with(".rs") {
            out.push(p);
        }
    }
}

fn display(root: Option<&Path>, p: &Path) -> String {
    match root.and_then(|r| p.strip_prefix(r).ok()) {
        Some(rel) => rel.display().to_string(),
        None => p.display().to_string(),
    }
}

/// Runs every check over the given crates and configuration.
pub fn run(opts: &Options) -> Result<Report, String> {
    let items = core_subset::parse(&opts.core_subset_toml)?;
    let mut report = Report::default();
    let root = opts.root.as_deref();
    let mut work: Vec<(PathBuf, Scope)> = Vec::new();
    for d in &opts.kernel_dirs {
        let mut files = Vec::new();
        sources(d, &mut files);
        for f in files {
            let unsafe_ = opts.unsafe_paths.iter().any(|u| f.starts_with(u) || f.with_extension("") == *u);
            work.push((f, if unsafe_ { Scope::KernelUnsafe } else { Scope::Kernel }));
        }
    }
    for d in &opts.app_dirs {
        let mut files = Vec::new();
        sources(d, &mut files);
        work.extend(files.into_iter().map(|f| (f, Scope::Application)));
    }
    for d in &opts.gen_dirs {
        let Ok(entries) = std::fs::read_dir(d) else { continue };
        let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|e| e == "rs")).collect();
        files.sort();
        work.extend(files.into_iter().map(|f| (f, Scope::Generated)));
    }
    // A file whose `mod x;` declaration is gated off for the Flight_Build
    // (`cfg(test)`, `cfg(kani)`, host-only) is outside the lint, with
    // everything under it.
    let mut excluded: Vec<PathBuf> = Vec::new();
    for (path, _) in &work {
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        if let Ok(file) = syn::parse_file(&text) {
            for m in lints::gated_modules(&file) {
                let base = match path.file_name().and_then(|n| n.to_str()) {
                    Some("mod.rs") | Some("lib.rs") | Some("main.rs") => path.parent().map(Path::to_path_buf),
                    _ => Some(path.with_extension("")),
                };
                if let Some(base) = base {
                    excluded.push(base.join(format!("{m}.rs")));
                    excluded.push(base.join(&m));
                }
            }
        }
    }
    let mut skipped = 0;
    for (path, scope) in &work {
        if excluded.iter().any(|e| path == e || path.starts_with(e)) {
            skipped += 1;
            continue;
        }
        let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
        let name = display(root, path);
        lint_source(&name, &text, *scope, &items, &mut report)?;
        report.files += 1;
    }
    if skipped > 0 {
        report.notes.push(format!("{skipped} files are declared under a `mod` that the Flight_Build's cfg excludes (test, kani, host-only) and are not linted"));
    }
    if let Some(root) = root {
        config::check_workspace(root, &work.iter().map(|(p, _)| p.clone()).collect::<Vec<_>>(), &mut report);
    }
    config::check_environment(&mut report);
    if let Some(model) = &opts.task_model_json {
        check_profile_version(model, &mut report);
    }
    let scopes = |s: Scope| work.iter().filter(|(_, x)| *x == s).count();
    report.notes.push(format!(
        "Profile {} ({}): {} Kernel files ({} in the Kernel_Unsafe_Module), {} Application files, {} generated files",
        profile_version::PROFILE_VERSION,
        profile_version::PROFILE_DEFAULT_VARIANT,
        scopes(Scope::Kernel) + scopes(Scope::KernelUnsafe),
        scopes(Scope::KernelUnsafe),
        scopes(Scope::Application),
        scopes(Scope::Generated)
    ));
    Ok(report)
}

/// Lints one source text (the unit the fixtures exercise).
pub fn lint_source(name: &str, text: &str, scope: Scope, items: &[core_subset::Item], report: &mut Report) -> Result<(), String> {
    let file = syn::parse_file(text).map_err(|e| format!("{name}: cannot parse: {e} (line {})", e.span().start().line))?;
    let mut linter = lints::Linter::new(name, scope, items);
    linter.run(&file);
    report.findings.extend(linter.findings);
    report.loops.extend(linter.loops);
    Ok(())
}

/// R3.5: the Profile version the lint implements is the Task_Model's.
fn check_profile_version(model_json: &str, report: &mut Report) {
    let found = serde_json::from_str::<serde_json::Value>(model_json)
        .ok()
        .and_then(|v| v.get("profile").and_then(|p| p.get("version")).and_then(|x| x.as_str().map(String::from)));
    match found {
        Some(v) if v == profile_version::PROFILE_VERSION => {}
        Some(v) => report.findings.push(Finding { rule: "R3.5", file: "task_model.json".into(), line: 0, message: format!("the Task_Model records Profile {v}; this Profile_Lint implements Profile {}", profile_version::PROFILE_VERSION) }),
        None => report.findings.push(Finding { rule: "R3.5", file: "task_model.json".into(), line: 0, message: "the Task_Model records no Profile version".into() }),
    }
}
