//! Proof mutation fixtures (R52.2 model validation; verification_plan.md
//! section 9.2): `cargo xtask mutations [--check] [--only <id>]`.
//!
//! A Verus specification earns DO-333 credit only if it can fail: a
//! specification that every implementation satisfies (a vacuous one) proves
//! nothing. The register `docs/proof_mutations.toml` lists, for the
//! specifications that matter most, one source mutation each that breaks
//! the code or the Hardware_Model, and the text that the Verus failure must
//! then contain. This command copies the source tree, applies one mutation
//! at a time, runs `cargo-verus verify` on the mutated package, and fails
//! unless Verus rejects the mutant with the expected diagnostic. A mutant
//! that Verus accepts is reported as a vacuity finding against the named
//! specification.
//!
//! `--check` validates the register without Verus: every mutation names an
//! existing file in which `find` occurs exactly once (so the mutation stays
//! applicable as the code moves), and every `expect` is non-empty. The
//! repository checks (`cargo xtask check`, verification job step 1) run it.
//!
//! The copy lives in `target/xtask-mutations/tree` and the Verus target
//! directory in `target/xtask-mutations/verus`, so vstd is built once for
//! every mutation. Each run's output is written to
//! `target/xtask-mutations/<id>.log`.

use std::path::Path;

use toml::{Table, Value};

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::rebuild::copy_tree;
use crate::tools::Verus;

pub const MUTATIONS: &str = "docs/proof_mutations.toml";

/// One registered mutation.
#[derive(Clone, Debug)]
pub struct Mutation {
    pub id: String,
    /// The specification the mutation must trip (for the report).
    pub kills: String,
    /// Source file, relative to the repository root.
    pub file: String,
    /// Text that occurs exactly once in `file`.
    pub find: String,
    /// Its replacement.
    pub replace: String,
    /// Substrings that the Verus output must all contain.
    pub expect: Vec<String>,
    /// The package `cargo-verus verify -p` runs on.
    pub package: String,
}

fn field<'a>(t: &'a Table, key: &str, id: &str) -> Result<&'a str, String> {
    t.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{MUTATIONS}: mutation `{id}` lacks the string field `{key}`"))
}

/// Reads the register.
pub fn load(root: &Path) -> Result<Vec<Mutation>, String> {
    let path = root.join(MUTATIONS);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let table: Table = text.parse().map_err(|e| format!("{MUTATIONS}: {e}"))?;
    if table.get("schema").and_then(Value::as_integer) != Some(1) {
        return Err(format!("{MUTATIONS}: `schema` must be 1"));
    }
    let mut out = Vec::new();
    for (i, m) in table.get("mutation").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let m = m.as_table().ok_or_else(|| format!("{MUTATIONS}: mutation {i} is not a table"))?;
        let id = m.get("id").and_then(Value::as_str).ok_or_else(|| format!("{MUTATIONS}: mutation {i} has no `id`"))?;
        let expect: Vec<String> = m
            .get("expect")
            .and_then(Value::as_array)
            .map(|a| a.iter().filter_map(Value::as_str).map(str::to_string).collect())
            .unwrap_or_default();
        out.push(Mutation {
            id: id.to_string(),
            kills: field(m, "kills", id)?.to_string(),
            file: field(m, "file", id)?.to_string(),
            find: field(m, "find", id)?.to_string(),
            replace: field(m, "replace", id)?.to_string(),
            expect,
            package: m.get("package").and_then(Value::as_str).unwrap_or("rsk-kernel").to_string(),
        });
    }
    Ok(out)
}

/// `--check`: the register is well formed and every mutation applies to the
/// current sources exactly once.
pub fn check(root: &Path, report: &mut Report) -> Vec<Mutation> {
    let mutations = match load(root) {
        Ok(m) => m,
        Err(e) => {
            report.error(Rule::Proof, Some(MUTATIONS.to_string()), e);
            return Vec::new();
        }
    };
    let mut ids = std::collections::BTreeSet::new();
    for m in &mutations {
        if !ids.insert(m.id.clone()) {
            report.error(Rule::Proof, Some(MUTATIONS.to_string()), format!("mutation `{}` is listed twice", m.id));
        }
        if m.expect.is_empty() || m.expect.iter().any(|e| e.trim().is_empty()) {
            report.error(Rule::Proof, Some(MUTATIONS.to_string()), format!("mutation `{}` names no expected diagnostic text", m.id));
        }
        if m.find == m.replace {
            report.error(Rule::Proof, Some(MUTATIONS.to_string()), format!("mutation `{}` changes nothing", m.id));
        }
        match std::fs::read_to_string(root.join(&m.file)) {
            Ok(text) => {
                let n = text.matches(m.find.as_str()).count();
                if n != 1 {
                    report.error(
                        Rule::Proof,
                        Some(m.file.clone()),
                        format!("mutation `{}`: `find` occurs {n} times in {} (must be exactly once)", m.id, m.file),
                    );
                }
            }
            Err(e) => report.error(Rule::Proof, Some(m.file.clone()), format!("mutation `{}`: cannot read {}: {e}", m.id, m.file)),
        }
    }
    report.note(format!("R52.2: {} proof mutations registered in {MUTATIONS}", mutations.len()));
    mutations
}

/// The verdict of one mutant.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Verus rejected the mutant with the expected diagnostic.
    Killed { first_error: String },
    /// Verus accepted the mutant: the specification is vacuous for it.
    Survived { verified: u64 },
    /// Verus failed, but not where the register says (a compile error, or
    /// another specification): the mutation needs revising.
    Unexpected { first_error: String, missing: String },
}

/// Classifies one cargo-verus output against the mutation's expectations.
pub fn classify(m: &Mutation, exit_ok: bool, text: &str) -> Verdict {
    let lines = crate::proofs::parse_verus_result_lines(text);
    let errors: u64 = lines.iter().map(|&(_, e)| e).sum();
    let verified: u64 = lines.iter().map(|&(v, _)| v).sum();
    let first_error = text
        .lines()
        .find(|l| l.starts_with("error"))
        .unwrap_or("(no error line)")
        .to_string();
    if exit_ok && errors == 0 {
        return Verdict::Survived { verified };
    }
    match m.expect.iter().find(|e| !text.contains(e.as_str())) {
        None => Verdict::Killed { first_error },
        Some(missing) => Verdict::Unexpected { first_error, missing: missing.clone() },
    }
}

/// Applies the mutations one at a time to a copy of the tree and runs Verus
/// on each. Returns whether every mutant was killed as expected.
pub fn run(root: &Path, only: Option<&str>, report: &mut Report) -> bool {
    let mutations = check(root, report);
    if report.diagnostics.iter().any(|d| d.rule == Rule::Proof) {
        return false;
    }
    let verus = match Verus::locate() {
        Ok(v) => v,
        Err(e) => {
            report.error(Rule::Proof, None, e);
            return false;
        }
    };
    let selected: Vec<&Mutation> = mutations.iter().filter(|m| only.is_none_or(|o| o == m.id)).collect();
    if selected.is_empty() {
        report.error(Rule::Proof, None, format!("no mutation named `{}`", only.unwrap_or("")));
        return false;
    }
    let work = root.join("target").join("xtask-mutations");
    let tree = work.join("tree");
    let target_dir = work.join("verus");
    if tree.exists() {
        if let Err(e) = std::fs::remove_dir_all(&tree) {
            report.error(Rule::Proof, None, format!("cannot remove {}: {e}", tree.display()));
            return false;
        }
    }
    if let Err(e) = copy_tree(root, &tree) {
        report.error(Rule::Proof, None, e);
        return false;
    }
    let _ = std::fs::create_dir_all(&work);
    let mut all_ok = true;
    for m in selected {
        let original = root.join(&m.file);
        let mutated = tree.join(&m.file);
        let text = match std::fs::read_to_string(&original) {
            Ok(t) => t,
            Err(e) => {
                report.error(Rule::Proof, Some(m.file.clone()), format!("cannot read: {e}"));
                all_ok = false;
                continue;
            }
        };
        let mutant = text.replacen(m.find.as_str(), m.replace.as_str(), 1);
        if let Err(e) = std::fs::write(&mutated, &mutant) {
            report.error(Rule::Proof, Some(m.file.clone()), format!("cannot write the mutant: {e}"));
            all_ok = false;
            continue;
        }
        let cmd = Cmd::new(verus.cargo_verus())
            .args(["verus", "verify", "-p", &m.package, "--target-dir"])
            .arg(&target_dir)
            .cwd(&tree);
        println!("   mutation `{}` ({}): $ {}", m.id, m.kills, cmd.display());
        let verdict = match cmd.output(None) {
            Ok(out) => {
                let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
                let _ = std::fs::write(work.join(format!("{}.log", m.id)), &text);
                classify(m, out.status.success(), &text)
            }
            Err(e) => {
                report.error(Rule::Proof, None, format!("cannot run cargo-verus: {e}"));
                all_ok = false;
                let _ = std::fs::write(&mutated, &text);
                continue;
            }
        };
        // Restore the file so that the next mutation starts from the original.
        let _ = std::fs::write(&mutated, &text);
        match verdict {
            Verdict::Killed { first_error } => {
                report.note(format!("mutation `{}` killed by {}: {first_error}", m.id, m.kills));
            }
            Verdict::Survived { verified } => {
                all_ok = false;
                report.error(
                    Rule::Proof,
                    Some(m.file.clone()),
                    format!(
                        "mutation `{}` survived: Verus verified {verified} items with 0 errors, so {} does not exclude it (R52.2 vacuity)",
                        m.id, m.kills
                    ),
                );
            }
            Verdict::Unexpected { first_error, missing } => {
                all_ok = false;
                report.error(
                    Rule::Proof,
                    Some(m.file.clone()),
                    format!(
                        "mutation `{}` failed Verus, but the output lacks `{missing}` (first error: {first_error}); revise the mutation or its `expect` (see target/xtask-mutations/{}.log)",
                        m.id, m.id
                    ),
                );
            }
        }
    }
    all_ok
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mutation(expect: &[&str]) -> Mutation {
        Mutation {
            id: "m".into(),
            kills: "k".into(),
            file: "f".into(),
            find: "a".into(),
            replace: "b".into(),
            expect: expect.iter().map(|s| s.to_string()).collect(),
            package: "rsk-kernel".into(),
        }
    }

    /// R52.2: a mutant that Verus accepts is a surviving mutant (a vacuity finding against the specification).
    #[test]
    fn a_passing_run_is_a_surviving_mutant() {
        let text = "verification results:: 10 verified, 0 errors\n";
        assert_eq!(classify(&mutation(&["x"]), true, text), Verdict::Survived { verified: 10 });
    }

    /// R52.2: a mutant that Verus rejects with the expected diagnostic text is killed.
    #[test]
    fn a_failing_run_with_the_expected_text_kills_the_mutant() {
        let text = "error: postcondition not satisfied\n  --> a.rs:1:1\nensures basepri_for\nverification results:: 9 verified, 1 errors\n";
        assert!(matches!(classify(&mutation(&["basepri_for", "a.rs"]), false, text), Verdict::Killed { .. }));
    }

    /// R52.2: a mutant that Verus rejects for another reason than the expected one is reported as unexpected, not as killed.
    #[test]
    fn a_failing_run_without_the_expected_text_is_unexpected() {
        let text = "error: expected `{`\nverification results:: 0 verified, 0 errors\n";
        assert_eq!(
            classify(&mutation(&["basepri_for"]), false, text),
            Verdict::Unexpected { first_error: "error: expected `{`".into(), missing: "basepri_for".into() }
        );
    }

    /// R52.2: every mutation of `docs/proof_mutations.toml` applies to the current sources (its `find` occurs exactly once in an existing file).
    #[test]
    fn the_register_applies_to_the_current_sources() {
        let root = crate::repository_root();
        let mut report = Report::default();
        let mutations = check(&root, &mut report);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        assert!(mutations.len() >= 7);
    }
}
