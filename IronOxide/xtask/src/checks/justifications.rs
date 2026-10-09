//! PR-20 and R6.4: every `unsafe` block, `unsafe fn`, and `unsafe impl` (and,
//! conservatively, `unsafe trait` and `unsafe extern` block) that a
//! Flight_Build compiles in the Kernel_Unsafe_Module carries a justification
//! identifier, and every identifier links to a Kernel_Proofs proof or lemma,
//! a Kani harness, or a recorded review that states why neither is feasible.
//!
//! Convention:
//!
//! - A justification identifier has the form `UJ-nnn`. It is written in a
//!   comment on the line of the `unsafe` keyword or in the comment lines
//!   directly above the construct (attributes may sit between the comment
//!   and the construct), for example `// SAFETY (UJ-012): ...`.
//! - Each Kernel crate that contains `unsafe` constructs has a register
//!   `unsafe_justifications.toml` next to its `Cargo.toml`:
//!
//!   ```toml
//!   schema = 1
//!   [[justification]]
//!   id = "UJ-012"
//!   summary = "BASEPRI write in the lock service"
//!   kind = "kani"                 # "kani" | "verus" | "review"
//!   link = "uj_012_write_basepri" # kani: a #[kani::proof] fn; verus: a fn inside verus!
//!   # kind = "review" instead requires:
//!   # why_no_machine_check = "..."; reviewed_by = "..."; reviewed_on = "YYYY-MM-DD"
//!   ```
//!
//! - Links resolve against the sources of every Kernel crate: a `kani` link
//!   must name a function with a `#[kani::proof]` attribute; a `verus` link
//!   must name a function written inside `verus!` (a `proof fn` lemma or an
//!   executable function whose `ensures` is the proof).
//! - Constructs inside items gated by `#[cfg(kani)]` or `#[cfg(test)]` are
//!   not compiled by a Flight_Build and need no identifier (they are the
//!   harnesses themselves).

use std::collections::BTreeMap;
use std::path::Path;

use toml::{Table, Value};

use crate::checks::kernel_size::target_cfg;
use crate::checks::unsafe_code::unsafe_module;
use crate::diag::{Report, Rule};
use crate::tokens::{flight_unsafe_items, kani_proof_fns, lex_file, verus_fn_names};
use crate::workspace::Workspace;

pub const REGISTER: &str = "unsafe_justifications.toml";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Kani,
    Verus,
    Review,
}

#[derive(Clone, Debug)]
pub struct Justification {
    pub id: String,
    pub summary: String,
    pub kind: Kind,
    pub link: Option<String>,
    pub why_no_machine_check: Option<String>,
    pub reviewed_by: Option<String>,
    pub reviewed_on: Option<String>,
}

/// Whether `id` has the form `UJ-` followed by at least three digits.
pub fn is_identifier(id: &str) -> bool {
    id.strip_prefix("UJ-")
        .is_some_and(|d| d.len() >= 3 && d.bytes().all(|b| b.is_ascii_digit()))
}

/// The first `UJ-nnn` identifier in the comment part of a line.
fn marker_in_comment(line: &str) -> Option<String> {
    let comment = line.find("//").map(|i| &line[i..])?;
    marker_in(comment)
}

fn marker_in(text: &str) -> Option<String> {
    let mut rest = text;
    while let Some(i) = rest.find("UJ-") {
        let digits: String = rest[i + 3..]
            .chars()
            .take_while(|c| c.is_ascii_digit())
            .collect();
        if digits.len() >= 3 {
            return Some(format!("UJ-{digits}"));
        }
        rest = &rest[i + 3..];
    }
    None
}

/// The identifier that covers the construct on 1-based `line`: in a comment
/// on that line, or in the comment lines directly above it (blank lines and
/// attribute lines may intervene).
pub fn marker_for(lines: &[&str], line: usize) -> Option<String> {
    let idx = line.checked_sub(1)?;
    if let Some(id) = lines.get(idx).and_then(|l| marker_in_comment(l)) {
        return Some(id);
    }
    let mut i = idx;
    while i > 0 {
        i -= 1;
        let text = lines[i].trim_start();
        if text.is_empty() || text.starts_with("#[") || text.starts_with("#![") {
            continue;
        }
        if text.starts_with("//") {
            if let Some(id) = marker_in(text) {
                return Some(id);
            }
            continue;
        }
        if text.starts_with("/*") || text.starts_with('*') {
            if let Some(id) = marker_in(text) {
                return Some(id);
            }
            continue;
        }
        break;
    }
    None
}

/// Reads the register of one Kernel crate. Returns `None` when the file does
/// not exist.
pub fn read_register(dir: &Path, report: &mut Report, location: &str) -> Option<Vec<Justification>> {
    let path = dir.join(REGISTER);
    let text = std::fs::read_to_string(&path).ok()?;
    let table: Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            report.error(
                Rule::UnsafeJustification,
                Some(location.to_string()),
                format!("cannot parse the justification register: {e}"),
            );
            return Some(Vec::new());
        }
    };
    let mut out = Vec::new();
    for (n, entry) in table
        .get("justification")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .enumerate()
    {
        let field = |key: &str| {
            entry
                .get(key)
                .and_then(Value::as_str)
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string)
        };
        let Some(id) = field("id") else {
            report.error(
                Rule::UnsafeJustification,
                Some(location.to_string()),
                format!("justification entry {} has no `id`", n + 1),
            );
            continue;
        };
        if !is_identifier(&id) {
            report.error(
                Rule::UnsafeJustification,
                Some(location.to_string()),
                format!("justification `{id}` is not of the form UJ-nnn"),
            );
        }
        let kind = match field("kind").as_deref() {
            Some("kani") => Kind::Kani,
            Some("verus") => Kind::Verus,
            Some("review") => Kind::Review,
            other => {
                report.error(
                    Rule::UnsafeJustification,
                    Some(location.to_string()),
                    format!(
                        "justification `{id}` has kind {}; expected \"kani\", \"verus\", or \"review\"",
                        other.map_or("missing".to_string(), |k| format!("`{k}`"))
                    ),
                );
                continue;
            }
        };
        out.push(Justification {
            id,
            summary: field("summary").unwrap_or_default(),
            kind,
            link: field("link"),
            why_no_machine_check: field("why_no_machine_check"),
            reviewed_by: field("reviewed_by"),
            reviewed_on: field("reviewed_on"),
        });
    }
    Some(out)
}

pub fn check(ws: &Workspace, report: &mut Report) {
    let cfg = target_cfg(ws);

    // Link targets across every Kernel crate.
    let mut harnesses: Vec<String> = Vec::new();
    let mut verus_fns: Vec<String> = Vec::new();
    for name in ws.policy.kernel_crates() {
        let Some(pkg) = ws.package_named(name) else { continue };
        for file in pkg.lib_files() {
            if let Ok(tokens) = lex_file(&file) {
                harnesses.extend(kani_proof_fns(&tokens));
                verus_fns.extend(verus_fn_names(&tokens));
            }
        }
    }

    let mut constructs = 0usize;
    let mut by_kind: BTreeMap<&str, usize> = BTreeMap::new();
    for pkg in ws.member_packages() {
        let Some(policy) = ws.policy.crates.get(&pkg.name) else { continue };
        if policy.unsafe_module.is_none() && !policy.unsafe_crate {
            continue;
        }
        let module = unsafe_module(ws, pkg, policy, report);
        let register_location = format!("{}/{REGISTER}", ws.rel(&pkg.dir));
        let register = read_register(&pkg.dir, report, &register_location);
        let mut entries: BTreeMap<String, Justification> = BTreeMap::new();
        for j in register.clone().unwrap_or_default() {
            if entries.insert(j.id.clone(), j.clone()).is_some() {
                report.error(
                    Rule::UnsafeJustification,
                    Some(register_location.clone()),
                    format!("justification `{}` is defined twice", j.id),
                );
            }
        }
        let mut used: BTreeMap<String, usize> = BTreeMap::new();
        let mut crate_constructs = 0usize;
        for file in &module {
            let tokens = match lex_file(file) {
                Ok(tokens) => tokens,
                Err(e) => {
                    report.error(Rule::UnsafeJustification, Some(ws.rel(file)), e);
                    continue;
                }
            };
            let text = std::fs::read_to_string(file).unwrap_or_default();
            let lines: Vec<&str> = text.lines().collect();
            for item in flight_unsafe_items(&tokens, &cfg) {
                crate_constructs += 1;
                let location = Some(format!("{}:{}", ws.rel(file), item.line));
                let what = match &item.name {
                    Some(name) => format!("{} `{name}`", item.kind.describe()),
                    None => item.kind.describe().to_string(),
                };
                let Some(id) = marker_for(&lines, item.line) else {
                    report.error(
                        Rule::UnsafeJustification,
                        location,
                        format!("{what} lacks a justification identifier (UJ-nnn in a comment on or above it)"),
                    );
                    continue;
                };
                *used.entry(id.clone()).or_default() += 1;
                let Some(entry) = entries.get(&id) else {
                    report.error(
                        Rule::UnsafeJustification,
                        location,
                        format!("{what} cites `{id}`, which {register_location} does not define"),
                    );
                    continue;
                };
                match entry.kind {
                    Kind::Kani => match &entry.link {
                        Some(link) if harnesses.contains(link) => {
                            *by_kind.entry("Kani harness").or_default() += 1;
                        }
                        Some(link) => report.error(
                            Rule::UnsafeJustification,
                            location,
                            format!(
                                "`{id}` ({what}) links to the Kani harness `{link}`, but no `#[kani::proof]` \
                                 function of that name exists in the Kernel crates"
                            ),
                        ),
                        None => report.error(
                            Rule::UnsafeJustification,
                            location,
                            format!("`{id}` ({what}) has kind \"kani\" but no `link`"),
                        ),
                    },
                    Kind::Verus => match &entry.link {
                        Some(link) if verus_fns.contains(link) => {
                            *by_kind.entry("Verus proof or lemma").or_default() += 1;
                        }
                        Some(link) => report.error(
                            Rule::UnsafeJustification,
                            location,
                            format!(
                                "`{id}` ({what}) links to the Verus proof `{link}`, but no function of that name \
                                 exists inside `verus!` in the Kernel crates"
                            ),
                        ),
                        None => report.error(
                            Rule::UnsafeJustification,
                            location,
                            format!("`{id}` ({what}) has kind \"verus\" but no `link`"),
                        ),
                    },
                    Kind::Review => {
                        if entry.why_no_machine_check.is_none() {
                            report.error(
                                Rule::UnsafeJustification,
                                location,
                                format!(
                                    "`{id}` ({what}) is justified by review only but its entry lacks the \
                                     `why_no_machine_check` statement that R6.4 requires"
                                ),
                            );
                        } else if entry.reviewed_by.is_none() || entry.reviewed_on.is_none() {
                            report.error(
                                Rule::UnsafeJustification,
                                location,
                                format!("`{id}` ({what}) is a review without `reviewed_by` and `reviewed_on`"),
                            );
                        } else {
                            *by_kind.entry("recorded review").or_default() += 1;
                        }
                    }
                }
            }
        }
        if crate_constructs > 0 && register.is_none() {
            report.error(
                Rule::UnsafeJustification,
                Some(register_location.clone()),
                format!(
                    "crate `{}` has {crate_constructs} unsafe construct(s) but no justification register",
                    pkg.name
                ),
            );
        }
        for id in entries.keys() {
            if !used.contains_key(id) {
                report.warn(format!(
                    "{register_location}: justification `{id}` is cited by no unsafe construct"
                ));
            }
        }
        constructs += crate_constructs;
    }
    let kinds: Vec<String> = by_kind.iter().map(|(k, n)| format!("{n} by {k}")).collect();
    report.note(format!(
        "PR-20/R6.4: {constructs} unsafe constructs in the Kernel_Unsafe_Module{}",
        if kinds.is_empty() {
            String::new()
        } else {
            format!(" ({})", kinds.join(", "))
        }
    ));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_markers_on_the_line_or_in_the_comment_block_above() {
        let src = "\
/// Reads a word.
///
/// # Safety
// SAFETY (UJ-001): the caller guarantees validity.
#[inline]
pub unsafe fn read(p: *const u32) -> u32 {
    unsafe { p.read_volatile() } // UJ-002
}

fn other() {}
unsafe fn no_marker() {}
";
        let lines: Vec<&str> = src.lines().collect();
        assert_eq!(marker_for(&lines, 6).as_deref(), Some("UJ-001"));
        assert_eq!(marker_for(&lines, 7).as_deref(), Some("UJ-002"));
        assert_eq!(marker_for(&lines, 11), None);
        assert!(is_identifier("UJ-012"));
        assert!(!is_identifier("UJ-1"));
        assert!(!is_identifier("uj-001"));
    }
}
