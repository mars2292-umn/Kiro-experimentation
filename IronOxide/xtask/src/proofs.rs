//! Proof steps of the verification job: the Kernel_Proofs with Verus
//! (R44.6, R3.10), the assumption gate (R50.2), the PAR-01 cross-check with
//! Verus's `line_count`, and the Kani_Harnesses (R45.5, R45.7).
//!
//! Verus. `cargo-verus verify -p rsk-kernel` runs with the Verus release's
//! pinned rustc and Z3 (located by `tools::Verus`). The step passes only
//! when the verifier reports zero errors; its Evidence_Item records the
//! Verus version, the toolchain, and the Z3 version. cargo-verus prints one
//! results line per crate it verifies, dependencies such as vstd first; the
//! count recorded for a package is the sum over its library and binary
//! targets (`package_verus_results`).
//!
//! Assumption gate. Every assumption construct in a verified crate (the
//! Kernel crates and every workspace crate that opts into Verus with a
//! `[package.metadata.verus]` table, such as the Analyzer): `assume`,
//! `admit`, `#[verifier::external_body]`,
//! `#[verifier::external_fn_specification]`, `assume_specification`,
//! `#[verifier::external]`, and the Kani stubs `#[kani::stub]`, must be
//! listed in `docs/proof_assumptions.toml` (the Trust_Base_Register's
//! assumption annex, TBR-04) with its justification, or the static checks
//! fail naming the construct (R50.2). The gate runs with the workspace
//! checks, so `cargo xtask check` and step 1 of the verification job both
//! apply it. A workspace without the register file has an empty register.
//! Assumptions in vstd itself are covered by the Trust_Base_Register entry
//! for vstd (TBR-04).
//!
//! Kani. `cargo kani -p rsk-kernel` runs every `#[kani::proof]` harness with
//! the pinned Kani and CBMC. A run passes only when Kani reports success
//! for every harness, which includes every unwinding assertion (R45.5).

use std::path::Path;

use toml::{Table, Value};

use crate::cmd::{stdout_of, Cmd};
use crate::diag::{Report, Rule};
use crate::evidence::{Item, Kind, Store, Tool, Verdict};
use crate::sandbox::Sandbox;
use crate::tokens::{is_ident, lex_file, metas, split_items};
use crate::tools::{Kani, Verus};
use crate::workspace::{Package, Workspace};

pub const ASSUMPTIONS: &str = "docs/proof_assumptions.toml";

/// Constructs that assume rather than prove.
const ASSUMPTION_ATTRS: &[&str] = &[
    "verifier::external_body",
    "verifier::external_fn_specification",
    "verifier::external_type_specification",
    "verifier::external",
    "verifier::assume_specification",
    "kani::stub",
    "kani::stub_verified",
];
const ASSUMPTION_CALLS: &[&str] = &["assume", "admit", "assume_specification"];

/// One assumption construct found in the sources.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assumption {
    pub file: String,
    pub line: usize,
    pub construct: String,
    /// The function the construct belongs to, when it can be determined.
    pub item: Option<String>,
}

/// Whether a package opts into Verus verification.
fn opts_into_verus(pkg: &Package) -> bool {
    std::fs::read_to_string(pkg.dir.join("Cargo.toml"))
        .map(|t| t.contains("[package.metadata.verus]"))
        .unwrap_or(false)
}

/// The crates whose proofs the verification job relies on: every Kernel
/// crate, plus every other workspace crate that opts into verification with
/// a `[package.metadata.verus]` table (the Analyzer's fixed-point core,
/// R32.1).
pub fn verified_crates(ws: &Workspace) -> Vec<String> {
    let mut names: Vec<String> = ws.policy.kernel_crates().map(str::to_string).collect();
    for pkg in ws.packages.values().filter(|p| ws.members.contains(&p.id)) {
        if opts_into_verus(pkg) && !names.contains(&pkg.name) {
            names.push(pkg.name.clone());
        }
    }
    names
}

/// The verified crates in dependency order: a crate comes after every
/// verified crate it depends on. cargo-verus verifies a dependency that
/// opts into Verus together with the crate that depends on it, and a later
/// run on the dependency alone is then fully cached and prints no results
/// line; verifying dependencies first gives every crate its own line.
pub fn verified_crates_in_dependency_order(ws: &Workspace) -> Vec<String> {
    let names = verified_crates(ws);
    let mut ordered: Vec<String> = Vec::new();
    let mut remaining = names.clone();
    while !remaining.is_empty() {
        let next = remaining.iter().position(|n| {
            ws.package_named(n).is_none_or(|pkg| {
                !pkg.dependencies.iter().any(|d| d.kind.is_none() && remaining.contains(&d.name) && d.name != *n)
            })
        });
        // A dependency cycle cannot occur among Cargo packages; take the first
        // crate if the search finds nothing, so the loop always ends.
        let i = next.unwrap_or(0);
        ordered.push(remaining.remove(i));
    }
    ordered
}

/// Scans the verified crates for assumption constructs.
pub fn find_assumptions(ws: &Workspace) -> Vec<Assumption> {
    let mut out = Vec::new();
    for name in verified_crates(ws) {
        let Some(pkg) = ws.package_named(&name) else { continue };
        for file in pkg.lib_files() {
            let Ok(tokens) = lex_file(&file) else { continue };
            let rel = ws.rel(&file);
            scan_level(&tokens, None, &rel, &mut out);
        }
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out.dedup();
    out
}

/// Scans one token level and the groups inside it. `enclosing` is the
/// function whose body the level belongs to, so that a construct inside a
/// body (`assume(...)` in a lemma) is attributed to that function: at the
/// body level no `fn` keyword is visible.
fn scan_level(tokens: &[proc_macro2::TokenTree], enclosing: Option<&str>, rel: &str, out: &mut Vec<Assumption>) {
    for item in split_items(tokens) {
        let fn_name = item.fn_name().or_else(|| enclosing.map(str::to_string));
        for attr in item.attrs.iter().filter(|a| !a.inner) {
            for meta in metas(attr) {
                if ASSUMPTION_ATTRS.contains(&meta.name.as_str()) {
                    out.push(Assumption {
                        file: rel.to_string(),
                        line: meta.line,
                        construct: format!("#[{}]", meta.name),
                        item: fn_name.clone(),
                    });
                }
            }
        }
        for (i, tt) in item.tokens.iter().enumerate() {
            if let proc_macro2::TokenTree::Group(g) = tt {
                let inner: Vec<proc_macro2::TokenTree> = g.stream().into_iter().collect();
                scan_level(&inner, fn_name.as_deref(), rel, out);
                continue;
            }
            let proc_macro2::TokenTree::Ident(id) = tt else { continue };
            let name = id.to_string();
            // `kani::assume(...)` constrains a harness input; it is not a Verus assumption.
            let kani_qualified = i >= 3
                && is_ident(&item.tokens[i - 3], "kani")
                && matches!(&item.tokens[i - 2], proc_macro2::TokenTree::Punct(p) if p.as_char() == ':')
                && matches!(&item.tokens[i - 1], proc_macro2::TokenTree::Punct(p) if p.as_char() == ':');
            if ASSUMPTION_CALLS.contains(&name.as_str()) && !kani_qualified {
                // `assume(...)`, `admit()`, or `assume_specification<...>[...]`.
                let next = item.tokens.get(i + 1);
                let is_call = matches!(next, Some(proc_macro2::TokenTree::Group(_)))
                    || matches!(next, Some(proc_macro2::TokenTree::Punct(p)) if p.as_char() == '<');
                if is_call && !is_ident(tt, "assume_specification") || name == "assume_specification" {
                    out.push(Assumption {
                        file: rel.to_string(),
                        line: crate::tokens::line(tt),
                        construct: format!("{name}(...)"),
                        item: fn_name.clone(),
                    });
                }
            }
        }
    }
}

/// R50.2: every assumption construct is listed with a justification.
pub fn check_assumptions(ws: &Workspace, report: &mut Report) -> Vec<Assumption> {
    let found = find_assumptions(ws);
    let path = ws.root.join(ASSUMPTIONS);
    let listed: Vec<(String, String, String)> = match std::fs::read_to_string(&path) {
        Ok(text) => match text.parse::<Table>() {
            Ok(table) => table
                .get("assumption")
                .and_then(Value::as_array)
                .into_iter()
                .flatten()
                .map(|a| {
                    let s = |k: &str| a.get(k).and_then(Value::as_str).unwrap_or_default().to_string();
                    (s("file"), s("construct"), s("item"))
                })
                .collect(),
            Err(e) => {
                report.error(Rule::Proof, Some(ASSUMPTIONS.to_string()), format!("cannot parse: {e}"));
                Vec::new()
            }
        },
        // No register file: no assumption is listed, so every construct found
        // is reported below.
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Vec::new(),
        Err(e) => {
            report.error(Rule::Proof, Some(ASSUMPTIONS.to_string()), format!("cannot read: {e}"));
            Vec::new()
        }
    };
    for a in &found {
        let item = a.item.clone().unwrap_or_default();
        let ok = listed
            .iter()
            .any(|(f, c, i)| *f == a.file && *c == a.construct && (*i == item || i.is_empty()));
        if !ok {
            report.error(
                Rule::Proof,
                Some(format!("{}:{}", a.file, a.line)),
                format!(
                    "assumption construct `{}`{} is not listed in {ASSUMPTIONS} (R50.2)",
                    a.construct,
                    a.item.as_ref().map_or(String::new(), |i| format!(" in `{i}`"))
                ),
            );
        }
    }
    for (f, c, i) in &listed {
        if !found.iter().any(|a| a.file == *f && a.construct == *c && (i.is_empty() || a.item.as_deref() == Some(i))) {
            report.warn(format!("{ASSUMPTIONS}: listed assumption `{c}` in {f} ({i}) no longer exists"));
        }
    }
    report.note(format!(
        "R50.2: {} assumption constructs in the verified crates ({}), {} listed in {ASSUMPTIONS}",
        found.len(),
        verified_crates(ws).join(", "),
        listed.len()
    ));
    found
}

/// Every "verification results:: N verified, M errors" line of a
/// cargo-verus run, in output order: one per crate the Verus driver
/// verified (dependencies such as vstd, then the package's own targets).
pub fn parse_verus_result_lines(output: &str) -> Vec<(u64, u64)> {
    output.lines().filter_map(parse_verus_line).collect()
}

fn parse_verus_line(line: &str) -> Option<(u64, u64)> {
    let rest = line.split("verification results::").nth(1)?.trim();
    let mut parts = rest.split(',');
    let verified = parts.next()?.trim().split_whitespace().next()?.parse().ok()?;
    let errors = parts.next()?.trim().split_whitespace().next()?.parse().ok()?;
    Some((verified, errors))
}

/// The results of a package's own targets, summed over its `targets`
/// library and binary targets. Every dependency of a package is a
/// dependency of its library (Cargo dependencies are per package, and
/// dev-dependencies are not built), so each dependency's line comes before
/// the library's; the binaries depend on the library, so their lines come
/// after it. The package's lines are therefore the last `targets` lines.
/// `None` when the run printed fewer lines than that, for example because
/// a failed crate stopped Cargo before the targets that depend on it.
pub fn package_verus_results(lines: &[(u64, u64)], targets: usize) -> Option<(u64, u64)> {
    if targets == 0 || lines.len() < targets {
        return None;
    }
    Some(
        lines[lines.len() - targets..]
            .iter()
            .fold((0, 0), |(v, e), &(lv, le)| (v + lv, e + le)),
    )
}

/// The targets of `pkg` that `cargo verus verify` checks: Cargo's default
/// targets, the library and every binary (tests, examples, and benches are
/// not built).
fn verus_target_count(pkg: &Package) -> usize {
    pkg.targets
        .iter()
        .filter(|t| t.is_lib() || t.kinds.iter().any(|k| k == "bin"))
        .count()
}

/// Runs the Kernel_Proofs. Returns whether they passed and writes the
/// Evidence_Item.
pub fn run_verus(
    ws: &Workspace,
    verus: &Verus,
    target_dir: &Path,
    sandbox: &Sandbox,
    store: &mut Store,
    manifest_id: &str,
    report: &mut Report,
) -> bool {
    let mut all_ok = true;
    let mut commands = Vec::new();
    let mut notes = Vec::new();
    let names = verified_crates_in_dependency_order(ws);
    for name in names.iter().map(String::as_str) {
        let Some(pkg) = ws.package_named(name) else { continue };
        if !pkg.dir.join("Cargo.toml").exists() {
            continue;
        }
        if !opts_into_verus(pkg) {
            notes.push(format!("{name}: no [package.metadata.verus] table; not verified"));
            continue;
        }
        let cmd = Cmd::new(verus.cargo_verus())
            .args(["verus", "verify", "-p", name, "--target-dir"])
            .arg(target_dir.join("verus"))
            .cwd(&ws.root);
        println!("   $ {}", cmd.display());
        commands.push(cmd.display());
        let out = match cmd.output(Some(sandbox)) {
            Ok(out) => out,
            Err(e) => {
                report.error(Rule::Proof, None, format!("cannot run cargo-verus: {e}"));
                all_ok = false;
                continue;
            }
        };
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let log_name = format!("verus-{name}.log");
        let _ = store.write_artifact(&log_name, &text);
        let lines = parse_verus_result_lines(&text);
        let targets = verus_target_count(pkg);
        // Errors in any verified crate, dependencies included, fail the step.
        let total_errors: u64 = lines.iter().map(|&(_, e)| e).sum();
        let package = package_verus_results(&lines, targets);
        if lines.is_empty() {
            all_ok = false;
            let first_error = text.lines().find(|l| l.starts_with("error")).unwrap_or("no verification results line");
            report.error(
                Rule::Proof,
                None,
                format!("cargo-verus did not report results for `{name}`: {first_error} (see {log_name})"),
            );
        } else if !out.status.success() || total_errors > 0 {
            // A failed crate stops Cargo before the crates that depend on
            // it, so the lines need not map to this package's targets:
            // report the totals of the run.
            all_ok = false;
            report.error(
                Rule::Proof,
                None,
                format!(
                    "Kernel_Proofs of `{name}` failed: {total_errors} errors over {} verified crates, exit {} (see {log_name})",
                    lines.len(),
                    out.status
                ),
            );
        } else if let Some((verified, _)) = package {
            notes.push(if targets > 1 {
                format!("{name}: {verified} verified, 0 errors (sum over {targets} library and binary targets)")
            } else {
                format!("{name}: {verified} verified, 0 errors")
            });
            let rule = if ws.policy.kernel_crates().any(|k| k == name) { "R44.6" } else { "R32.1" };
            report.note(format!("{rule}: {name}: {verified} items verified by Verus {} (Z3 {})", verus.version, verus.z3_version));
        } else {
            all_ok = false;
            report.error(
                Rule::Proof,
                None,
                format!(
                    "cargo-verus printed {} results lines for `{name}`, fewer than its {targets} library and binary \
                     targets, so its results cannot be attributed (see {log_name})",
                    lines.len()
                ),
            );
        }
    }
    let mut item = Item::new(
        "proof-kernel-verus",
        Kind::Proof,
        "Kernel_Proofs (Verus)",
        Tool {
            name: "Verus".to_string(),
            version: format!("{} ({})", verus.version, verus.toolchain),
            solvers: vec![("z3".to_string(), verus.z3_version.clone())],
        },
        Verdict::from_bool(all_ok),
    )
    .covers(&verus_covers(&ws.root).iter().map(String::as_str).collect::<Vec<_>>());
    item.command_lines = commands;
    item.notes = notes;
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = names.iter().map(|n| format!("verus-{n}.log")).collect();
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    all_ok
}

/// PAR-01 cross-check with Verus's `line_count` (notes only; the gate is
/// the Build_System counter, `checks::kernel_size`).
/// The criteria the Verus Evidence_Item covers (task 8.4 follow-up,
/// 2026-10-23): the `Complete` column of verification_plan.md section 9.5
/// for the Kernel and Analyzer-core rows, the criterion the item itself
/// discharges (R44.6: the proofs run in the job), and the restrictions
/// whose PO evidence the Profile names `proof-kernel-*`. Criteria the plan
/// lists as partial are not claimed.
pub fn verus_covers(root: &Path) -> Vec<String> {
    let plan = std::fs::read_to_string(root.join("docs/verification_plan.md")).unwrap_or_default();
    let mut out = crate::trace::plan_complete_criteria(&plan, &["Verus, Kernel", "Verus, Analyzer core"]);
    for fixed in ["R44.6", "PR-02", "PR-05", "PR-07", "PR-14", "PR-16", "PR-19", "PR-23", "PR-29", "PR-34"] {
        if !out.iter().any(|c| c == fixed) {
            out.push(fixed.to_string());
        }
    }
    out
}

/// Kani's optional checks per Kernel crate (Verification_Plan 6.1,
/// evaluated 2026-10-23 with Kani 0.68.0): rsk-logic passes every harness
/// with the invalid-value checks, the uninitialized-memory checks, and the
/// extra pointer checks; rsk-kernel passes with the extra pointer checks,
/// while `-Z valid-value-checks` crashes Kani's compiler on a `NonNull`
/// pattern type (an internal compiler error in `check_values.rs`) and
/// `-Z uninit-checks` reports two harnesses as failed because Kani cannot
/// determine the layout of `[MaybeUninit<u32>; 12]` and
/// `UnsafeCell<Option<u32>>` ("not supported", a tool limitation, not a
/// defect). The flags of each crate are the ones it passes with; the two
/// unusable ones are recorded in the plan, not silently dropped.
pub fn kani_extra_checks(crate_name: &str) -> &'static [&'static str] {
    match crate_name {
        "rsk-logic" => &["-Z", "valid-value-checks", "-Z", "uninit-checks", "-Z", "unstable-options", "--extra-pointer-checks"],
        "rsk-kernel" => &["-Z", "unstable-options", "--extra-pointer-checks"],
        _ => &[],
    }
}

/// The criteria the Kani Evidence_Item covers: the harness requirements and
/// the criteria the harnesses name in their doc comments and names.
pub fn kani_covers(root: &Path) -> Vec<String> {
    let mut out: Vec<String> = ["R45.1", "R45.2", "R45.3", "R45.5", "R45.7", "R45.8"].iter().map(|s| s.to_string()).collect();
    for c in crate::trace::named_by_tests(root, "#[kani::proof]").0 {
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

pub fn line_count_cross_check(ws: &Workspace, verus: &Verus, own_total: usize, report: &mut Report) {
    let Some(tool) = verus.line_count() else {
        report.warn("the Verus release has no `line_count` tool; PAR-01 cross-check skipped");
        return;
    };
    let files: Vec<_> = crate::checks::kernel_size::kernel_sources(ws)
        .into_iter()
        .map(|(_, f)| f)
        .collect();
    let mut cmd = Cmd::new(&tool).args(["--json", "--no-external-by-default", "--delimiters-are-layout"]);
    for f in &files {
        cmd = cmd.arg(f);
    }
    match stdout_of(&cmd) {
        Ok(json) => {
            let value: serde_json::Value = serde_json::from_str(&json).unwrap_or_default();
            let total = value.get("total").cloned().unwrap_or_default();
            let get = |k: &str| total.get(k).and_then(serde_json::Value::as_u64).unwrap_or(0);
            let exec = get("exec");
            let proof = get("proof");
            let spec = get("spec");
            let trusted = get("trusted");
            report.note(format!(
                "R6.2/PAR-01 cross-check (Verus line_count): exec {exec}, proof {proof}, spec {spec}, trusted {trusted}; \
                 Build_System count {own_total}"
            ));
            if exec as usize > own_total {
                report.warn(format!(
                    "Verus line_count reports {exec} executable lines, more than the Build_System's {own_total}; \
                     review the counter's classification"
                ));
            }
        }
        Err(e) => report.warn(format!("line_count cross-check failed: {e}")),
    }
}

/// Runs the Kani_Harnesses. Returns whether every harness passed.
/// The verdict of one `cargo kani` run (R45.5): passing only when the
/// process succeeded, at least one harness was verified, no harness
/// failed, and no unwinding assertion failed (Kani reports a failed
/// unwinding assertion as a failed check, but the rule is stated on its
/// own so that a future Kani that demotes it to a warning still fails
/// here).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KaniVerdict {
    pub successes: usize,
    pub failures: usize,
    pub unwinding_failed: bool,
    pub ok: bool,
}

pub fn kani_verdict(exit_ok: bool, text: &str) -> KaniVerdict {
    let successes = text.matches("VERIFICATION:- SUCCESSFUL").count();
    let failures = text.matches("VERIFICATION:- FAILED").count();
    let unwinding_failed = text.lines().any(|l| l.contains("unwinding assertion") && (l.contains("FAILURE") || l.starts_with("Failed Checks:")))
        || text.contains("unwinding failures");
    KaniVerdict {
        successes,
        failures,
        unwinding_failed,
        ok: exit_ok && failures == 0 && successes > 0 && !unwinding_failed,
    }
}

pub fn run_kani(
    ws: &Workspace,
    kani: &Kani,
    target_dir: &Path,
    sandbox: &Sandbox,
    store: &mut Store,
    manifest_id: &str,
    report: &mut Report,
) -> bool {
    let mut all_ok = true;
    let mut commands = Vec::new();
    let mut notes = Vec::new();
    for name in ws.policy.kernel_crates() {
        let Some(pkg) = ws.package_named(name) else { continue };
        // Only crates that define harnesses are run.
        let has_harness = pkg
            .lib_files()
            .iter()
            .any(|f| lex_file(f).map(|t| !crate::tokens::kani_proof_fns(&t).is_empty()).unwrap_or(false));
        if !has_harness {
            notes.push(format!("{name}: no #[kani::proof] harness"));
            continue;
        }
        let mut cmd = Cmd::new("cargo")
            .args(["kani", "-p", name, "--target-dir"])
            .arg(target_dir.join("kani"));
        for flag in kani_extra_checks(name) {
            cmd = cmd.arg(flag);
        }
        let cmd = cmd
            .cwd(&ws.root);
        println!("   $ {}", cmd.display());
        commands.push(cmd.display());
        let out = match cmd.output(Some(sandbox)) {
            Ok(out) => out,
            Err(e) => {
                report.error(Rule::Kani, None, format!("cannot run cargo kani: {e}"));
                all_ok = false;
                continue;
            }
        };
        let text = format!(
            "{}\n{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        let log_name = format!("kani-{name}.log");
        let _ = store.write_artifact(&log_name, &text);
        let KaniVerdict { successes, failures, unwinding_failed, ok } = kani_verdict(out.status.success(), &text);
        if ok {
            notes.push(format!("{name}: {successes} harnesses successful"));
            report.note(format!(
                "R45.5: {name}: {successes} Kani harnesses passed with every check (Kani {}, CBMC {})",
                kani.version, kani.cbmc_version
            ));
        } else {
            all_ok = false;
            report.error(
                Rule::Kani,
                None,
                format!(
                    "Kani_Harnesses of `{name}` did not all pass: {successes} successful, {failures} failed{}, exit {} (see {log_name})",
                    if unwinding_failed { ", an unwinding assertion failed" } else { "" },
                    out.status
                ),
            );
        }
    }
    let mut item = Item::new(
        "proof-kernel-kani",
        Kind::Proof,
        "Kani_Harnesses",
        Tool {
            name: "Kani".to_string(),
            version: kani.version.clone(),
            solvers: vec![("cbmc".to_string(), kani.cbmc_version.clone())],
        },
        Verdict::from_bool(all_ok),
    )
    .covers(&kani_covers(&ws.root).iter().map(String::as_str).collect::<Vec<_>>());
    item.command_lines = commands;
    item.notes = notes;
    item.build_manifest = Some(manifest_id.to_string());
    item.artifacts = vec!["kani-rsk-kernel.log".to_string()];
    if let Err(e) = store.write(&item) {
        report.error(Rule::Evidence, None, e);
    }
    all_ok
}

/// Every justification identifier linked to a Kani harness or Verus proof
/// is backed by a passing run (R3.10, R6.4): when a proof step failed, the
/// PR identifiers whose obligations it discharges are named.
pub fn name_failed_obligations(verus_ok: bool, kani_ok: bool, report: &mut Report) {
    if !verus_ok {
        report.error(
            Rule::Proof,
            None,
            "the Kernel_Proofs failed, so the proof obligations of PR-02, PR-05, PR-07, PR-14, PR-16, PR-19, PR-20, \
             PR-23, PR-29, and PR-34 are not discharged (R3.10)",
        );
    }
    if !kani_ok {
        report.error(
            Rule::Kani,
            None,
            "the Kani_Harnesses failed, so the proof obligations of PR-19 and PR-20 (Kernel_Unsafe_Module) are not \
             discharged (R3.10)",
        );
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn kernel_crates_are_verified_dependencies_first() {
        let mut report = crate::diag::Report::default();
        let ws = crate::workspace::Workspace::load(&crate::repository_root(), &mut report).expect("workspace");
        let order = super::verified_crates_in_dependency_order(&ws);
        let pos = |n: &str| order.iter().position(|x| x == n).expect(n);
        assert!(pos("rsk-logic") < pos("rsk-kernel"), "{order:?}");
    }

    use super::*;

    #[test]
    fn parses_verus_summary() {
        assert_eq!(
            parse_verus_result_lines("x\nverification results:: 198 verified, 0 errors\n"),
            vec![(198, 0)]
        );
        assert_eq!(
            parse_verus_result_lines("verification results:: 3 verified, 1 errors\nerror: could not compile"),
            vec![(3, 1)]
        );
        assert_eq!(parse_verus_result_lines("nothing"), vec![]);
        assert_eq!(parse_verus_result_lines("verification results:: garbled"), vec![]);
    }

    #[test]
    fn package_results_exclude_dependencies_and_sum_the_targets() {
        // rsk-kernel: vstd, then the library.
        let kernel = parse_verus_result_lines(
            "verification results:: 1876 verified, 0 errors\nverification results:: 218 verified, 0 errors\n",
        );
        assert_eq!(package_verus_results(&kernel, 1), Some((218, 0)));
        // rsk-analyzer: vstd, then the library, then the binary. The last
        // line alone (the binary's 0) was the value reported before.
        let analyzer = parse_verus_result_lines(
            "verification results:: 2060 verified, 0 errors\n\
             verification results:: 17 verified, 0 errors\n\
             verification results:: 0 verified, 0 errors\n",
        );
        assert_eq!(package_verus_results(&analyzer, 2), Some((17, 0)));
        // A binary with proofs of its own is counted too.
        assert_eq!(package_verus_results(&[(2060, 0), (17, 0), (4, 0)], 2), Some((21, 0)));
        // Fewer lines than targets cannot be attributed.
        assert_eq!(package_verus_results(&[(17, 1)], 2), None);
        assert_eq!(package_verus_results(&[(17, 0)], 0), None);
    }

    #[test]
    fn scans_assumption_constructs() {
        let tokens = crate::tokens::lex_str(
            "verus! { #[verifier::external_body] pub fn read() -> u32 { 0 } pub proof fn l() { assume(false); admit(); } \
             pub fn ok() { let assume_me = 1; } }",
        )
        .unwrap();
        let mut out = Vec::new();
        scan_level(&tokens, None, "f.rs", &mut out);
        let found: Vec<(String, Option<String>)> = out.into_iter().map(|a| (a.construct, a.item)).collect();
        // Constructs inside a body are attributed to the enclosing function,
        // including a `proof fn`; `kani::assume` and an identifier that only
        // contains `assume` are not constructs.
        assert_eq!(
            found,
            vec![
                ("#[verifier::external_body]".to_string(), Some("read".to_string())),
                ("assume(...)".to_string(), Some("l".to_string())),
                ("admit(...)".to_string(), Some("l".to_string())),
            ]
        );
        let mut harness = Vec::new();
        scan_level(&crate::tokens::lex_str("fn h() { kani::assume(x < 3); }").unwrap(), None, "f.rs", &mut harness);
        assert!(harness.is_empty(), "{harness:?}");
    }

    #[test]
    fn a_failed_unwinding_assertion_fails_the_run() {
        let log = include_str!("../fixtures/kani-unwind/expected.log");
        let v = kani_verdict(false, log);
        assert_eq!((v.successes, v.failures, v.unwinding_failed, v.ok), (0, 1, true, false));
        // Even a Kani that exited 0 and printed SUCCESSFUL would be rejected.
        let forgiving = log.replace("VERIFICATION:- FAILED", "VERIFICATION:- SUCCESSFUL");
        assert!(!kani_verdict(true, &forgiving).ok);
        assert!(kani_verdict(true, "VERIFICATION:- SUCCESSFUL\nComplete - 1 successfully verified harnesses, 0 failures, 1 total.").ok);
        assert!(!kani_verdict(true, "nothing ran").ok);
    }
}
