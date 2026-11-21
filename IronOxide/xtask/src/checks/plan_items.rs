//! R43 (2026-11-21): the property allocation table of the Verification_Plan
//! (section 3, the rows `| P-nn | ...`) names the functions, lemmas and
//! theorems that establish each property. The table is prose kept by hand,
//! and the requirements evidence document found it could fall behind the
//! code. This check ties the names to the sources: every item a row names
//! in backticks must be defined somewhere in the workspace's Rust or Lean
//! sources, so a renamed or deleted item fails the static checks until its
//! row is read again. It checks that the item exists, not that it says what
//! the row says: that is the evidence document's hashed statements.

use std::collections::BTreeSet;
use std::path::Path;

use crate::diag::{Report, Rule};

const PLAN: &str = "docs/verification_plan.md";

/// Keywords after which a name is a definition, in Rust and in Lean.
const DEFINING: [&str; 16] = ["fn", "struct", "enum", "trait", "const", "static", "type", "mod", "theorem", "lemma", "def", "abbrev", "structure", "inductive", "class", "macro_rules"];

fn is_ident_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// The names a source text defines: the identifier after a defining
/// keyword (with a Lean name's last segment), and every field or binding
/// written `name:`.
pub fn definitions(text: &str, out: &mut BTreeSet<String>) {
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    let mut previous = String::new();
    while i < chars.len() {
        if !is_ident_char(chars[i]) {
            i += 1;
            continue;
        }
        let start = i;
        while i < chars.len() && (is_ident_char(chars[i]) || (chars[i] == '.' && i + 1 < chars.len() && is_ident_char(chars[i + 1]) && DEFINING.contains(&previous.as_str()))) {
            i += 1;
        }
        let word: String = chars[start..i].iter().collect();
        if DEFINING.contains(&previous.as_str()) {
            if let Some(last) = word.rsplit('.').next() {
                out.insert(last.to_string());
            }
            out.insert(word.clone());
        } else if chars.get(i) == Some(&':') && chars.get(i + 1) != Some(&':') {
            out.insert(word.clone());
        }
        previous = word;
    }
}

fn collect(dir: &Path, out: &mut BTreeSet<String>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = e.file_name().to_string_lossy().into_owned();
        if p.is_dir() {
            if !matches!(name.as_str(), "target" | "vendor" | ".lake" | ".git") {
                collect(&p, out);
            }
        } else if name.ends_with(".rs") || name.ends_with(".lean") {
            if let Ok(text) = std::fs::read_to_string(&p) {
                definitions(&text, out);
            }
        }
    }
}

/// The items a table row names: the backticked texts that are a path of
/// identifiers (`a::b`, `A.b`, `name()`), reduced to their last segment.
/// A plain lower-case word without an underscore is a keyword, a feature
/// or a crate name, not an item, and is left out.
pub fn named_items(row: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (k, piece) in row.split('`').enumerate() {
        if k % 2 == 0 {
            continue;
        }
        let name = piece.strip_suffix("()").unwrap_or(piece);
        let segments: Vec<&str> = name.split("::").flat_map(|s| s.split('.')).collect();
        let well_formed = !segments.is_empty() && segments.iter().all(|s| !s.is_empty() && s.chars().all(is_ident_char) && !s.starts_with(|c: char| c.is_ascii_digit()));
        if !well_formed {
            continue;
        }
        let last = segments[segments.len() - 1];
        let is_item = segments.len() > 1 || last.contains('_') || last.starts_with(|c: char| c.is_ascii_uppercase());
        if is_item {
            out.push(last.to_string());
        }
    }
    out
}

/// The rows of the property allocation table: `| P-nn | ...`.
fn rows(text: &str) -> Vec<(String, &str)> {
    text.lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("| P-")?;
            let id: String = rest.chars().take_while(|c| c.is_ascii_digit()).collect();
            (id.len() == 2 && rest[2..].starts_with(" |")).then(|| (format!("P-{id}"), l))
        })
        .collect()
}

pub fn check(root: &Path, report: &mut Report) {
    let Ok(text) = std::fs::read_to_string(root.join(PLAN)) else {
        report.error(Rule::PlanTable, Some(PLAN.to_string()), "cannot read the Verification_Plan");
        return;
    };
    let mut defined = BTreeSet::new();
    for dir in ["crates", "apps", "proofs/aeneas/lean", "xtask/src", "xtask/tests"] {
        collect(&root.join(dir), &mut defined);
    }
    let rows = rows(&text);
    let mut named = 0usize;
    for (id, row) in &rows {
        for item in named_items(row) {
            named += 1;
            if !defined.contains(&item) {
                report.error(Rule::PlanTable, Some(PLAN.to_string()), format!("{id} names `{item}`, which no Rust or Lean source of the workspace defines: the row is behind the code"));
            }
        }
    }
    if rows.is_empty() {
        report.error(Rule::PlanTable, Some(PLAN.to_string()), "the property allocation table (rows `| P-nn |`) was not found");
    } else {
        report.note(format!("R43: the property allocation table names {named} items over {} rows, each defined in the sources", rows.len()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A row's backticked paths are items; keywords, feature names, file
    /// names and Evidence_Item identifiers are not.
    #[test]
    fn a_row_names_its_items_and_nothing_else() {
        let row = "| P-02 | Mutual exclusion | R8.3 | Verus: `lemma_no_other_holder`, `Sched::lock`, `Kernel.svc_lock()`, an `async` body, `fpu`, `rsk-link`, `ok == wf`, `Budget` | `proof-kernel-verus`; `test-workspace` |";
        assert_eq!(named_items(row), ["lemma_no_other_holder", "lock", "svc_lock", "Budget"]);
    }

    /// Definitions: Rust and Lean items, a Lean name's last segment, and
    /// fields.
    #[test]
    fn definitions_are_found_in_rust_and_lean() {
        let mut d = BTreeSet::new();
        definitions("pub fn lock(&mut self) {}\nproof fn lemma_a() {}\nstruct Budget { pub in_cs: u32 }\ntheorem Sched.Timer.served_chain : True := by trivial\nlet x = foo::bar;", &mut d);
        for name in ["lock", "lemma_a", "Budget", "in_cs", "served_chain", "Sched.Timer.served_chain"] {
            assert!(d.contains(name), "{name}: {d:?}");
        }
        assert!(!d.contains("bar") && !d.contains("foo"));
    }

    /// The table's rows are found by their identifier, and a name no source
    /// defines is reported with its row.
    #[test]
    fn a_name_without_a_definition_is_reported() {
        let root = std::env::temp_dir().join(format!("rsk-plan-items-{}", std::process::id()));
        std::fs::create_dir_all(root.join("docs")).unwrap();
        std::fs::create_dir_all(root.join("crates/k/src")).unwrap();
        std::fs::write(root.join("crates/k/src/lib.rs"), "pub fn present() {}\n").unwrap();
        std::fs::write(root.join(PLAN), "| P-01 | a | R1 | `present()` and `renamed_away` | x |\n| P-1 | not a row | `nothing_here` |\n").unwrap();
        let mut report = Report::default();
        check(&root, &mut report);
        let text: Vec<String> = report.diagnostics.iter().map(ToString::to_string).collect();
        assert!(text.iter().any(|t| t.contains("P-01 names `renamed_away`")), "{text:?}");
        assert!(!text.iter().any(|t| t.contains("present`") || t.contains("nothing_here")), "{text:?}");
        let _ = std::fs::remove_dir_all(&root);
    }
}
