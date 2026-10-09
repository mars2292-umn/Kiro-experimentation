//! Proof steps of the verification job: the Kernel_Proofs with Verus
//! (R44.6, R3.10), the assumption gate (R50.2), the PAR-01 cross-check with
//! Verus's `line_count`, and the Kani_Harnesses (R45.5, R45.7).
//!
//! Verus. `cargo-verus verify -p rsk-kernel` runs with the Verus release's
//! pinned rustc and Z3 (located by `tools::Verus`). The step passes only
//! when the verifier reports zero errors; its Evidence_Item records the
//! Verus version, the toolchain, and the Z3 version.
//!
//! Assumption gate. Every assumption construct in the Kernel crates
//! (`assume`, `admit`, `#[verifier::external_body]`,
//! `#[verifier::external_fn_specification]`, `assume_specification`,
//! `#[verifier::external]`, and the Kani stubs `#[kani::stub]`) must be
//! listed in `docs/proof_assumptions.toml` with its justification, or the
//! job fails naming the construct (R50.2). Assumptions in vstd itself are
//! covered by the Trust_Base_Register entry for vstd (TBR-04).
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
use crate::tokens::{for_each_level, is_ident, lex_file, metas, split_items};
use crate::tools::{Kani, Verus};
use crate::workspace::Workspace;

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

/// Scans the Kernel crates for assumption constructs.
pub fn find_assumptions(ws: &Workspace) -> Vec<Assumption> {
    let mut out = Vec::new();
    for name in ws.policy.kernel_crates() {
        let Some(pkg) = ws.package_named(name) else { continue };
        for file in pkg.lib_files() {
            let Ok(tokens) = lex_file(&file) else { continue };
            let rel = ws.rel(&file);
            for_each_level(&tokens, &mut |level| {
                for item in split_items(level) {
                    let fn_name = item.fn_name();
                    for attr in item.attrs.iter().filter(|a| !a.inner) {
                        for meta in metas(attr) {
                            if ASSUMPTION_ATTRS.contains(&meta.name.as_str()) {
                                out.push(Assumption {
                                    file: rel.clone(),
                                    line: meta.line,
                                    construct: format!("#[{}]", meta.name),
                                    item: fn_name.clone(),
                                });
                            }
                        }
                    }
                    for (i, tt) in item.tokens.iter().enumerate() {
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
                                    file: rel.clone(),
                                    line: crate::tokens::line(tt),
                                    construct: format!("{name}(...)"),
                                    item: fn_name.clone(),
                                });
                            }
                        }
                    }
                }
            });
        }
    }
    out.sort_by(|a, b| (&a.file, a.line).cmp(&(&b.file, b.line)));
    out.dedup();
    out
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
        "R50.2: {} assumption constructs in the Kernel crates, {} listed in {ASSUMPTIONS}",
        found.len(),
        listed.len()
    ));
    found
}

/// Parses "verification results:: N verified, M errors".
pub fn parse_verus_results(output: &str) -> Option<(u64, u64)> {
    let line = output.lines().rev().find(|l| l.contains("verification results::"))?;
    let rest = line.split("verification results::").nth(1)?.trim();
    let mut parts = rest.split(',');
    let verified = parts.next()?.trim().split_whitespace().next()?.parse().ok()?;
    let errors = parts.next()?.trim().split_whitespace().next()?.parse().ok()?;
    Some((verified, errors))
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
    // Every Kernel crate, plus every other workspace crate that opts into
    // verification with a `[package.metadata.verus]` table (the Analyzer's
    // fixed-point core, R32.1).
    let mut names: Vec<String> = ws.policy.kernel_crates().map(str::to_string).collect();
    for pkg in ws.packages.values().filter(|p| ws.members.contains(&p.id)) {
        let opted = std::fs::read_to_string(pkg.dir.join("Cargo.toml"))
            .map(|t| t.contains("[package.metadata.verus]"))
            .unwrap_or(false);
        if opted && !names.contains(&pkg.name) {
            names.push(pkg.name.clone());
        }
    }
    for name in names.iter().map(String::as_str) {
        let Some(pkg) = ws.package_named(name) else { continue };
        if !pkg.dir.join("Cargo.toml").exists() {
            continue;
        }
        let verifies = std::fs::read_to_string(pkg.dir.join("Cargo.toml"))
            .map(|t| t.contains("[package.metadata.verus]"))
            .unwrap_or(false);
        if !verifies {
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
        match parse_verus_results(&text) {
            Some((verified, errors)) if out.status.success() && errors == 0 => {
                notes.push(format!("{name}: {verified} verified, 0 errors"));
                let rule = if ws.policy.kernel_crates().any(|k| k == name) { "R44.6" } else { "R32.1" };
                report.note(format!("{rule}: {name}: {verified} items verified by Verus {} (Z3 {})", verus.version, verus.z3_version));
            }
            Some((verified, errors)) => {
                all_ok = false;
                report.error(
                    Rule::Proof,
                    None,
                    format!("Kernel_Proofs of `{name}` failed: {verified} verified, {errors} errors (see {log_name})"),
                );
            }
            None => {
                all_ok = false;
                let first_error = text.lines().find(|l| l.starts_with("error")).unwrap_or("no verification results line");
                report.error(
                    Rule::Proof,
                    None,
                    format!("cargo-verus did not report results for `{name}`: {first_error} (see {log_name})"),
                );
            }
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
    .covers(&[
        "R7.1", "R7.2", "R7.3", "R7.5", "R7.6", "R8.1", "R8.2", "R8.3", "R8.4", "R8.5", "R8.9", "R8.10", "R9.1", "R9.3",
        "R9.4", "R9.5", "R9.7", "R9.8", "R10.1", "R10.2", "R10.3", "R10.7", "R14.6", "R14.7", "R16.5", "R16.6", "R18.1",
        "R19.1", "R20.3", "R44.1", "R44.2", "R44.3", "R44.4", "R44.6", "R44.8", "R49.1", "R49.5", "PR-02", "PR-07",
        "PR-14", "PR-16", "PR-23", "PR-34",
    ]);
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
        let cmd = Cmd::new("cargo")
            .args(["kani", "-p", name, "--target-dir"])
            .arg(target_dir.join("kani"))
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
    .covers(&["R45.1", "R45.2", "R45.3", "R45.5", "R45.7", "R45.8"]);
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
    use super::*;

    #[test]
    fn parses_verus_summary() {
        assert_eq!(
            parse_verus_results("x\nverification results:: 198 verified, 0 errors\n"),
            Some((198, 0))
        );
        assert_eq!(
            parse_verus_results("verification results:: 3 verified, 1 errors\nerror: could not compile"),
            Some((3, 1))
        );
        assert_eq!(parse_verus_results("nothing"), None);
    }

    #[test]
    fn scans_assumption_constructs() {
        let tokens = crate::tokens::lex_str(
            "verus! { #[verifier::external_body] pub fn read() -> u32 { 0 } pub proof fn l() { assume(false); admit(); } \
             pub fn ok() { let assume_me = 1; } }",
        )
        .unwrap();
        let mut found = Vec::new();
        for_each_level(&tokens, &mut |level| {
            for item in split_items(level) {
                for attr in item.attrs.iter().filter(|a| !a.inner) {
                    for meta in metas(attr) {
                        if ASSUMPTION_ATTRS.contains(&meta.name.as_str()) {
                            found.push(format!("#[{}]", meta.name));
                        }
                    }
                }
                for (i, tt) in item.tokens.iter().enumerate() {
                    if let proc_macro2::TokenTree::Ident(id) = tt {
                        let name = id.to_string();
                        if ASSUMPTION_CALLS.contains(&name.as_str())
                            && matches!(item.tokens.get(i + 1), Some(proc_macro2::TokenTree::Group(_)))
                        {
                            found.push(format!("{name}(...)"));
                        }
                    }
                }
            }
        });
        assert_eq!(found, vec!["#[verifier::external_body]", "assume(...)", "admit(...)"]);
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
