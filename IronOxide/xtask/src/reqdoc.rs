//! `cargo xtask reqdoc`: `docs/requirements_evidence.md`, the plain-English
//! companion of the traceability matrix. One row per requirement.
//!
//! Two sources, kept apart so that neither drifts:
//!
//! - **Written by hand**, in `docs/requirements_evidence.toml`: the
//!   requirement in one sentence, how it is implemented, what each cited
//!   formal statement says, whether the statements are as strong as the
//!   requirement, and what remains.
//! - **Generated**, from the data the traceability matrix is generated from
//!   (`trace::build`: requirements.md, the plan's allocation table, the
//!   sources, the evidence store) and from the cited sources themselves: the
//!   identifiers, the criteria, the files and lines, the statements' names
//!   and tools, the Evidence_Items with their verdicts.
//!
//! Every cited statement (a Verus function's contract, a spec function's
//! body, a Lean theorem's statement, a Kani harness) is located in its
//! file and its text hashed, comments and layout aside. The hand-written
//! row records the hash it was written against (`reviewed`). A statement
//! that has changed since flags its row in the document and is an error of
//! `--check` and of the verification job: the plain-English reading must be
//! read again against the new text, and the new hash entered by hand
//! (`cargo xtask reqdoc --hashes` prints them).

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml::{Table, Value};

use crate::diag::{Report, Rule};
use crate::trace;

pub const SOURCE: &str = "docs/requirements_evidence.toml";
pub const OUTPUT: &str = "docs/requirements_evidence.md";

/// The kinds of evidence a row may claim besides its formal statements.
const OTHER_KINDS: &[(&str, &str)] = &[
    ("checked", "Checked"),
    ("tested-qemu", "Tested (QEMU)"),
    ("tested-rig", "Tested (rig)"),
    ("tested-host", "Tested (host)"),
    ("inspected", "Inspected"),
    ("assumed", "Assumed"),
];

/// The documents a reference of an `other` entry must be written in.
const REFERENCE_DOCS: &[&str] = &["docs/verification_plan.md", "docs/trust_base_register.toml", "docs/proof_assumptions.toml", "docs/profile.md", ".kiro/specs/rsk-ravenscar-profile/design.md", "crates/rsk-kernel/unsafe_justifications.toml"];

#[derive(Debug, Clone)]
pub struct Statement {
    pub file: String,
    pub name: String,
    /// `verus`, `lean`, `kani`.
    pub tool: String,
    /// Text the search starts after (to tell two items of one name apart).
    pub after: Option<String>,
    /// The body is part of the statement (a spec function, a definition).
    pub body: bool,
    pub says: String,
    pub reviewed: String,
}

#[derive(Debug, Clone)]
pub struct Other {
    pub kind: String,
    pub reference: String,
    pub note: String,
}

#[derive(Debug, Clone)]
pub struct Row {
    pub requirement: u32,
    pub sentence: String,
    pub implemented: String,
    pub strength: String,
    pub strength_note: String,
    pub closed: String,
    pub gaps: String,
    pub statements: Vec<Statement>,
    pub others: Vec<Other>,
}

/// A statement as found in its file.
#[derive(Debug, Clone)]
pub struct Found {
    pub line: usize,
    pub hash: String,
}

fn text_of(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).map(|s| s.split_whitespace().collect::<Vec<_>>().join(" ")).unwrap_or_default()
}

pub fn parse(text: &str) -> Result<Vec<Row>, String> {
    let table: Table = text.parse().map_err(|e| format!("{SOURCE}: {e}"))?;
    let mut rows = Vec::new();
    for (n, r) in table.get("row").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let requirement = r.get("requirement").and_then(Value::as_integer).ok_or_else(|| format!("{SOURCE}: row {} has no `requirement` number", n + 1))? as u32;
        let mut statements = Vec::new();
        for s in r.get("statement").and_then(Value::as_array).into_iter().flatten() {
            statements.push(Statement {
                file: text_of(s, "file"),
                name: text_of(s, "name"),
                tool: text_of(s, "tool"),
                after: s.get("after").and_then(Value::as_str).map(str::to_string),
                body: s.get("body").and_then(Value::as_bool).unwrap_or(false),
                says: text_of(s, "says"),
                reviewed: text_of(s, "reviewed"),
            });
        }
        let mut others = Vec::new();
        for o in r.get("other").and_then(Value::as_array).into_iter().flatten() {
            others.push(Other { kind: text_of(o, "kind"), reference: text_of(o, "ref"), note: text_of(o, "note") });
        }
        rows.push(Row {
            requirement,
            sentence: text_of(r, "sentence"),
            implemented: text_of(r, "implemented"),
            strength: text_of(r, "strength"),
            strength_note: text_of(r, "strength_note"),
            closed: text_of(r, "closed"),
            gaps: text_of(r, "gaps"),
            statements,
            others,
        });
    }
    Ok(rows)
}

/// A line without its `//` or `--` comment (no string of the cited sources
/// holds either pair).
fn strip_comment<'a>(line: &'a str, lean: bool) -> &'a str {
    let mark = if lean { "--" } else { "//" };
    match line.find(mark) {
        Some(i) => &line[..i],
        None => line,
    }
}

fn is_word_at(line: &str, at: usize, len: usize) -> bool {
    let before = line[..at].chars().next_back();
    let after = line[at + len..].chars().next();
    let word = |c: char| c.is_alphanumeric() || c == '_' || c == '.' || c == '\'';
    !before.is_some_and(word) && !after.is_some_and(word)
}

/// The text of the statement `name` in `source`: for a Rust function, the
/// signature and the contract up to the body (with `body`, the body too);
/// for a Lean declaration, up to `:=` of a theorem (with `body`, up to the
/// next declaration). Returns the 1-based line and the text, comments
/// removed and whitespace collapsed.
pub fn statement_text(source: &str, name: &str, after: Option<&str>, body: bool, lean: bool) -> Option<(usize, String)> {
    let start_at = match after {
        Some(a) => source.find(a)?,
        None => 0,
    };
    let first_line = source[..start_at].matches('\n').count();
    let lines: Vec<&str> = source.lines().collect();
    let heads: &[&str] = if lean { &["theorem ", "def ", "inductive ", "structure ", "abbrev "] } else { &["fn "] };
    let mut start = None;
    'find: for (i, raw) in lines.iter().enumerate().skip(first_line) {
        let line = strip_comment(raw, lean);
        for head in heads {
            let needle = format!("{head}{name}");
            let mut from = 0;
            while let Some(p) = line[from..].find(&needle) {
                let at = from + p + head.len();
                let head_ok = from + p == 0 || !line[..from + p].chars().next_back().is_some_and(|c| c.is_alphanumeric() || c == '_');
                if head_ok && is_word_at(line, at, name.len()) {
                    start = Some(i);
                    break 'find;
                }
                from = at;
            }
        }
    }
    let start = start?;
    let mut out = String::new();
    if lean {
        let next_decl = |l: &str| ["theorem ", "def ", "inductive ", "structure ", "abbrev ", "/--", "/-!", "end ", "macro", "syntax ", "set_option ", "@[", "namespace ", "variable ", "open "].iter().any(|h| l.starts_with(h));
        for (i, raw) in lines.iter().enumerate().skip(start) {
            if i > start && next_decl(raw) {
                break;
            }
            let line = strip_comment(raw, true);
            if !body {
                if let Some(p) = line.find(":=") {
                    out.push_str(&line[..p]);
                    break;
                }
            }
            out.push_str(line);
            out.push(' ');
        }
    } else {
        let mut depth = 0i32;
        let mut opened = false;
        'lines: for (i, raw) in lines.iter().enumerate().skip(start) {
            let line = strip_comment(raw, false);
            if !body {
                // The body opens on a line of its own, or at the end of a
                // signature that has no contract.
                if i > start && line.trim() == "{" {
                    break;
                }
                if line.trim_end().ends_with('{') && !line.contains("requires") && !line.contains("ensures") {
                    out.push_str(line.trim_end().trim_end_matches('{'));
                    break;
                }
                out.push_str(line);
                out.push(' ');
                if line.trim_end().ends_with(';') {
                    break;
                }
                continue;
            }
            for c in line.chars() {
                out.push(c);
                if c == '{' {
                    depth += 1;
                    opened = true;
                } else if c == '}' {
                    depth -= 1;
                    if opened && depth == 0 {
                        break 'lines;
                    }
                }
            }
            out.push(' ');
        }
    }
    Some((start + 1, out.split_whitespace().collect::<Vec<_>>().join(" ")))
}

pub fn find(root: &Path, s: &Statement) -> Result<Found, String> {
    let source = std::fs::read_to_string(root.join(&s.file)).map_err(|e| format!("{}: {e}", s.file))?;
    let lean = s.file.ends_with(".lean");
    let (line, text) = statement_text(&source, &s.name, s.after.as_deref(), s.body, lean).ok_or_else(|| format!("{}: no statement `{}`", s.file, s.name))?;
    if s.tool == "kani" {
        let above = source.lines().take(line - 1).collect::<Vec<_>>();
        if !above.iter().rev().take(6).any(|l| l.contains("kani::proof")) {
            return Err(format!("{}:{line}: `{}` is cited as a Kani harness and has no `#[kani::proof]` above it", s.file, s.name));
        }
    }
    let digest = rsk_digest::sha256_hex(text.as_bytes());
    Ok(Found { line, hash: digest.trim_start_matches("sha256:")[..16].to_string() })
}

fn tool_name(tool: &str) -> &'static str {
    match tool {
        "verus" => "Verus",
        "lean" => "Lean",
        "kani" => "Kani",
        _ => "?",
    }
}

fn cell(s: &str) -> String {
    s.replace('|', "\\|").replace('\n', " ")
}

/// What the generator checked of the hand-written source.
#[derive(Default)]
pub struct Checked {
    /// (requirement, statement, file, hash now) of every cited statement
    /// whose text is not the one its row was written against.
    pub changed: Vec<(u32, String, String, String)>,
    pub errors: Vec<String>,
    pub statements: usize,
}

pub fn render(root: &Path, rows: &[Row], m: &trace::Matrix, evidence_dir_name: &str) -> (String, Checked) {
    let mut c = Checked::default();
    let refs: String = REFERENCE_DOCS.iter().filter_map(|d| std::fs::read_to_string(root.join(d)).ok()).collect::<Vec<_>>().join("\n");
    let mut s = String::new();
    s.push_str("<!-- @generated by `cargo xtask reqdoc` from docs/requirements_evidence.toml (the plain-English text, written by hand) and the data of the traceability matrix (the identifiers, files, statements and Evidence_Items). Edit the .toml, not this file. -->\n");
    s.push_str("# Requirements and their evidence, in plain English\n\n");
    s.push_str("A companion to `docs/traceability_matrix.md`. The matrix says which artifacts name a criterion; this says, for the requirements the paper's claims rest on, what the requirement asks, what does it, what the formal statements actually say, and whether that is as much as the requirement asks.\n\n");
    s.push_str("How to read it:\n\n");
    s.push_str("- The sentences are written by hand from the text of the cited statements. The identifiers, criteria, files, line numbers, statement names, tools and Evidence_Items are generated from the same data as the matrix.\n");
    s.push_str("- Every cited statement's text is hashed. A statement that has changed since its row was written is marked **CHANGED** below and fails `cargo xtask reqdoc --check` and the verification job, until the row is read again against the new text.\n");
    s.push_str("- \"Proven\" means a tool checked the statement against the code or the model it names. It does not mean the statement says what the requirement says: that is the fifth column.\n");
    s.push_str(&format!("- Evidence_Item verdicts are those of the store `{evidence_dir_name}` when this file was generated; an item not in that store is listed without a verdict.\n\n"));
    s.push_str("| Requirement | How it's implemented | Formal statement | Evidence | As strong as the requirement? | Gaps and assumptions |\n|---|---|---|---|---|---|\n");
    for row in rows {
        let criteria: Vec<&trace::Criterion> = m.criteria.iter().filter(|k| k.requirement == row.requirement).collect();
        if criteria.is_empty() {
            c.errors.push(format!("row R{}: requirements.md has no Requirement {}", row.requirement, row.requirement));
            continue;
        }
        let title = &criteria[0].title;
        let phase = &criteria[0].phase;
        let mut allocation: BTreeSet<String> = BTreeSet::new();
        let mut items: BTreeMap<String, String> = BTreeMap::new();
        let mut code: BTreeSet<String> = BTreeSet::new();
        for k in &criteria {
            if let Some(l) = m.links.get(&k.id) {
                allocation.extend(l.properties.iter().cloned());
                code.extend(l.code.iter().cloned());
                for (id, verdict) in &l.evidence {
                    items.insert(id.clone(), verdict.clone());
                }
            }
        }
        // Column 1.
        let col1 = format!(
            "**R{}** {} (R{}.1 to R{}.{}, Phase {}).<br>{}",
            row.requirement,
            title,
            row.requirement,
            row.requirement,
            criteria.len(),
            phase,
            row.sentence
        );
        // Column 3 and the statement files of column 2.
        let mut col3 = Vec::new();
        let mut files: BTreeSet<String> = BTreeSet::new();
        let mut tools: BTreeSet<&'static str> = BTreeSet::new();
        let mut row_changed = false;
        for st in &row.statements {
            c.statements += 1;
            if !matches!(st.tool.as_str(), "verus" | "lean" | "kani") {
                c.errors.push(format!("row R{}: statement `{}` has tool `{}` (verus, lean or kani)", row.requirement, st.name, st.tool));
            }
            match find(root, st) {
                Ok(f) => {
                    files.insert(st.file.clone());
                    tools.insert(tool_name(&st.tool));
                    let changed = f.hash != st.reviewed;
                    if changed {
                        row_changed = true;
                        c.changed.push((row.requirement, st.name.clone(), st.file.clone(), f.hash.clone()));
                    }
                    col3.push(format!(
                        "`{}` ({}, `{}:{}`){}: {}",
                        st.name,
                        tool_name(&st.tool),
                        st.file,
                        f.line,
                        if changed { " **CHANGED since this was written**" } else { "" },
                        st.says
                    ));
                }
                Err(e) => c.errors.push(format!("row R{}: {e}", row.requirement)),
            }
        }
        let cited = files.iter().filter(|f| code.contains(*f)).count();
        let col2 = format!(
            "{}<br>Statements in: {}.<br>{} source files cite a criterion of R{} in the matrix ({} of the files above among them).",
            row.implemented,
            files.iter().map(|f| format!("`{f}`")).collect::<Vec<_>>().join(", "),
            code.len(),
            row.requirement,
            cited
        );
        // Column 4.
        let mut col4 = Vec::new();
        if !tools.is_empty() {
            let per: Vec<String> = tools.iter().map(|t| format!("{t} ({})", row.statements.iter().filter(|s| tool_name(&s.tool) == *t).count())).collect();
            col4.push(format!("**Proven:** {} of the statements in column 3.", per.join(", ")));
        }
        for (kind, label) in OTHER_KINDS {
            let of_kind: Vec<&Other> = row.others.iter().filter(|o| o.kind == *kind).collect();
            if of_kind.is_empty() {
                continue;
            }
            let mut parts = Vec::new();
            for o in of_kind {
                if !refs.contains(&o.reference) {
                    c.errors.push(format!("row R{}: `{}` ({kind}) is written in none of {}", row.requirement, o.reference, REFERENCE_DOCS.join(", ")));
                }
                let verdict = m.links.values().flat_map(|l| l.evidence.iter()).find(|(id, _)| *id == o.reference).map(|(_, v)| format!(", {v} in the store")).unwrap_or_default();
                parts.push(format!("`{}`{}{}", o.reference, verdict, if o.note.is_empty() { String::new() } else { format!(" ({})", o.note) }));
            }
            col4.push(format!("**{label}:** {}.", parts.join("; ")));
        }
        for o in &row.others {
            if !OTHER_KINDS.iter().any(|(k, _)| *k == o.kind) {
                c.errors.push(format!("row R{}: evidence kind `{}` is not one of {}", row.requirement, o.kind, OTHER_KINDS.iter().map(|(k, _)| *k).collect::<Vec<_>>().join(", ")));
            }
        }
        col4.push(format!("Allocation rows: {}.", if allocation.is_empty() { "none".to_string() } else { allocation.iter().cloned().collect::<Vec<_>>().join(", ") }));
        col4.push(format!(
            "Evidence_Items naming a criterion of R{}: {}.",
            row.requirement,
            if items.is_empty() { "none in the store".to_string() } else { items.iter().map(|(id, v)| format!("`{id}` ({v})")).collect::<Vec<_>>().join(", ") }
        ));
        // Column 5.
        if !matches!(row.strength.as_str(), "yes" | "weaker" | "no") {
            c.errors.push(format!("row R{}: `strength` is `{}` (yes, weaker or no)", row.requirement, row.strength));
        }
        let mut col5 = format!("**{}.** {}", match row.strength.as_str() { "yes" => "Yes", "weaker" => "Weaker", _ => "No" }, row.strength_note);
        if row_changed {
            col5 = format!("**NOT CURRENT: a cited statement changed; the judgment below was made of the earlier text.**<br>{col5}");
        }
        if !row.closed.is_empty() {
            col5.push_str(&format!("<br>Closed since: {}", row.closed));
        }
        for (what, text) in [("sentence", &row.sentence), ("implemented", &row.implemented), ("strength_note", &row.strength_note), ("gaps", &row.gaps)] {
            if text.is_empty() {
                c.errors.push(format!("row R{}: `{what}` is empty", row.requirement));
            }
        }
        for st in &row.statements {
            if st.says.is_empty() {
                c.errors.push(format!("row R{}: statement `{}` has no `says`", row.requirement, st.name));
            }
        }
        s.push_str(&format!("| {} | {} | {} | {} | {} | {} |\n", cell(&col1), cell(&col2), cell(&col3.join("<br>")), cell(&col4.join("<br>")), cell(&col5), cell(&row.gaps)));
    }
    (s, c)
}

pub enum Mode {
    Write,
    Check,
    Hashes,
}

/// The cited statements against the sources: the errors of the hand-written
/// file and the statements changed since their rows were written. Used by
/// the verification job, which does not judge the rendered file.
pub fn check_statements(root: &Path, report: &mut Report) -> Result<usize, String> {
    let rows = parse(&std::fs::read_to_string(root.join(SOURCE)).map_err(|e| format!("{SOURCE}: {e}"))?)?;
    let mut n = 0;
    for row in &rows {
        for st in &row.statements {
            n += 1;
            match find(root, st) {
                Ok(f) if f.hash != st.reviewed => report.error(
                    Rule::Evidence,
                    Some(format!("{}:{}", st.file, f.line)),
                    format!("`{}` has changed since row R{} of {OUTPUT} was written against it (hash {} then, {} now): read the row's plain-English text against the statement and enter the new hash in {SOURCE}", st.name, row.requirement, st.reviewed, f.hash),
                ),
                Ok(_) => {}
                Err(e) => report.error(Rule::Evidence, Some(SOURCE.into()), format!("row R{}: {e}", row.requirement)),
            }
        }
    }
    Ok(n)
}

pub fn run(root: &Path, evidence_dir: &Path, mode: Mode, report: &mut Report) -> Result<(), String> {
    let rows = parse(&std::fs::read_to_string(root.join(SOURCE)).map_err(|e| format!("{SOURCE}: {e}"))?)?;
    if let Mode::Hashes = mode {
        for row in &rows {
            for st in &row.statements {
                match find(root, st) {
                    Ok(f) => println!("R{} {} {}:{} {}{}", row.requirement, st.name, st.file, f.line, f.hash, if f.hash == st.reviewed { "" } else { "  <- differs from `reviewed`" }),
                    Err(e) => println!("R{} {}: {e}", row.requirement, st.name),
                }
            }
        }
        return Ok(());
    }
    let m = trace::build(root, evidence_dir)?;
    let name = evidence_dir.strip_prefix(root).unwrap_or(evidence_dir).display().to_string();
    let (text, checked) = render(root, &rows, &m, &name);
    for e in &checked.errors {
        report.error(Rule::Evidence, Some(SOURCE.into()), e.clone());
    }
    for (r, st, file, hash) in &checked.changed {
        let message = format!("`{st}` ({file}) has changed since row R{r} was written against it (hash now {hash}): read the row against the statement and enter the new hash");
        match mode {
            Mode::Check => report.error(Rule::Evidence, Some(SOURCE.into()), message),
            _ => report.warn(format!("{SOURCE}: {message}")),
        }
    }
    let path = root.join(OUTPUT);
    match mode {
        Mode::Check => {
            if std::fs::read_to_string(&path).unwrap_or_default() != text {
                report.error(Rule::Evidence, Some(OUTPUT.into()), "the document differs from its sources; run `cargo xtask reqdoc`");
            }
        }
        _ => {
            std::fs::write(&path, &text).map_err(|e| format!("{OUTPUT}: {e}"))?;
            report.note(format!("wrote {OUTPUT}"));
        }
    }
    report.note(format!("{} rows, {} cited statements, {} changed since their rows were written", rows.len(), checked.statements, checked.changed.len()));
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A Verus contract is the signature and the clauses up to the body; a comment or a layout change does not change it, a clause does.
    #[test]
    fn a_contract_is_found_and_only_its_text_counts() {
        let a = "impl K {\n    /// doc\n    pub fn intake(&mut self, t: u8)\n        requires old(self).inv(), // note\n        ensures final(self).inv(),\n    {\n        body();\n    }\n    fn intake_other() {}\n}\n";
        let (line, text) = statement_text(a, "intake", None, false, false).unwrap();
        assert_eq!(line, 3);
        assert_eq!(text, "pub fn intake(&mut self, t: u8) requires old(self).inv(), ensures final(self).inv(),");
        let b = a.replace("// note", "// another note").replace("requires old", "requires   old").replace("body();", "other();");
        assert_eq!(statement_text(&b, "intake", None, false, false).unwrap().1, text);
        let c = a.replace("ensures final(self).inv(),", "ensures true,");
        assert_ne!(statement_text(&c, "intake", None, false, false).unwrap().1, text);
        // With `body`, the body counts.
        assert_ne!(statement_text(&b, "intake", None, true, false).unwrap().1, statement_text(a, "intake", None, true, false).unwrap().1);
        // `after` tells two items of one name apart.
        let two = "fn f() {}\nmod m {\n    fn f(x: u8)\n        ensures x == x,\n    {\n    }\n}\n";
        assert_eq!(statement_text(two, "f", Some("mod m"), false, false).unwrap().0, 3);
        assert!(statement_text(two, "g", None, false, false).is_none());
    }

    /// A Lean theorem's statement ends at `:=`; a proof change does not change it.
    #[test]
    fn a_lean_statement_ends_at_its_proof() {
        let a = "theorem served_now {k k' : K}\n    (h : serve k = ok k') : -- the reading\n    k'.now = k.now := by\n  simp\n\ntheorem served_now_too : True := by trivial\n";
        let (line, text) = statement_text(a, "served_now", None, false, true).unwrap();
        assert_eq!(line, 1);
        assert_eq!(text, "theorem served_now {k k' : K} (h : serve k = ok k') : k'.now = k.now");
        assert_eq!(statement_text(&a.replace("simp", "omega"), "served_now", None, false, true).unwrap().1, text);
        assert_eq!(statement_text(a, "served_now_too", None, false, true).unwrap().0, 6);
    }

    /// The repository's document is generated from its sources without an error of the hand-written file.
    #[test]
    fn the_repository_source_parses_and_every_statement_is_found() {
        let root = crate::repository_root();
        let rows = parse(&std::fs::read_to_string(root.join(SOURCE)).unwrap()).unwrap();
        assert!(!rows.is_empty());
        for row in &rows {
            for st in &row.statements {
                find(&root, st).unwrap_or_else(|e| panic!("R{}: {e}", row.requirement));
            }
        }
    }
}
