//! R48 and R54: checks of the Trust_Base_Register and Gap_Register sources
//! (task 8.2). Both run with the repository checks, so `cargo xtask check`
//! and step 1 of `cargo xtask verify` apply them.
//!
//! Trust_Base_Register (`docs/trust_base_register.toml`, R48):
//! - every `[[entry]]` states what is trusted (`trusted_for`), why (`why`),
//!   the evidence that reduces the trust, the residual risk, and the
//!   requirements and Verification_Plan properties that depend on it
//!   (R48.2), and links its ASM and ORQ identifiers (R48.3);
//! - every item that R48.1 lists is mapped in `[[r48_1]]` to entries or
//!   `[[crate]]` entries that exist;
//! - every identifier resolves: requirements and criteria against
//!   requirements.md, ORQs against design.md, properties (`P-nn`) against
//!   the Verification_Plan's allocation table.
//!
//! The `[[crate]]` gate of the Flight_Build graph (R58.2, R58.5) is
//! `checks::deps`.
//!
//! Gap_Register (`docs/gap_register.toml`, R54):
//! - every DO-178C Annex A objective appears exactly once with its
//!   applicability, rsk status, and owner: prototype, applicant, or shared
//!   (R54.1, R54.4). The Level A and Level B tallies must equal the
//!   published counts; a Level C tally that differs from the published
//!   count is accepted only while some objective declares its Level C
//!   applicability unconfirmed (`confirm`);
//! - every partial or not-addressed objective or supplement item names a
//!   gap, and every addressed one names its evidence;
//! - the gaps that R54.2 requires exist and are not closed;
//! - every ORQ of design.md appears with the status design.md gives it, so
//!   a status change there fails this check until the Gap_Register is
//!   updated in the same change (R54.3); an ORQ that affects certification
//!   evidence and is not resolved names a gap;
//! - every identifier resolves, and every `GAP-nn` that the repository's
//!   documents cite exists.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml::{Table, Value};

use crate::diag::{Report, Rule};

pub const TRUST_BASE: &str = "docs/trust_base_register.toml";
pub const GAP_REGISTER: &str = "docs/gap_register.toml";
pub const REQUIREMENTS: &str = ".kiro/specs/rsk-ravenscar-profile/requirements.md";
pub const DESIGN: &str = ".kiro/specs/rsk-ravenscar-profile/design.md";
pub const VERIFICATION_PLAN: &str = "docs/verification_plan.md";

/// Documents whose `GAP-nn` citations must resolve.
const GAP_CITERS: &[&str] = &[
    "profile/core_subset.toml",
    "docs/kani_stubs.toml",
    "docs/trust_base_register.toml",
    "docs/verification_plan.md",
    ".kiro/specs/rsk-ravenscar-profile/design.md",
    ".kiro/specs/rsk-ravenscar-profile/tasks.md",
];

/// The items that R48.1 requires the Trust_Base_Register to list, as the
/// keys of its `[[r48_1]]` table.
pub const R48_1_ITEMS: &[&str] = &[
    "toolchain-front-end-and-llvm",
    "linker",
    "core-and-core-subset",
    "flight-proc-macros",
    "hal-and-pac",
    "hardware-model",
    "nvic-mpu-fpu-dwt-semantics",
    "silicon-errata",
    "verus-smt-vstd-assumptions",
    "kani-cbmc-sat-stubs",
    "app-prover",
    "uppaal",
    "analyzer",
    "wcet-harness",
    "static-wcet-analyzer",
    "config-checker",
    "link-checker",
    "profile-lint",
    "protocol-checker",
    "hil-rig-probe-flashing",
    "build-system",
];

/// DO-178C Annex A: the objective count of each table A-1 to A-10 (71 in
/// all).
pub const ANNEX_A: &[(u32, u32)] = &[(1, 7), (2, 7), (3, 7), (4, 13), (5, 9), (6, 5), (7, 9), (8, 6), (9, 5), (10, 3)];

/// Published objective counts per software level (DO-178C Annex A, as
/// summarized publicly). Level C is the lowest level of the
/// Profile_Conformance_Suite Application's Partitions (R54.1).
pub const PUBLISHED: &[(&str, usize)] = &[("A", 71), ("B", 69), ("C", 62)];

/// The gaps R54.2 requires, each with the identifiers its gap entry must
/// link. `R57` also matches any criterion of Requirement 57.
const R54_2: &[(&str, &[&str])] = &[
    ("compiler assurance beyond Ferrocene's DO-178C DAL C support", &["ORQ-04"]),
    ("Rust MC/DC tooling", &["ORQ-05"]),
    ("DO-330 qualification of Verus and Kani", &["ORQ-06"]),
    ("certification of `core` outside DO-178C", &["R57"]),
    ("measurement-based WCET", &["ORQ-13"]),
    ("robust partitioning evidence under the selected privilege model", &["ORQ-01", "R16.10"]),
    ("DMA isolation without an IOMMU", &["ORQ-03"]),
];

const OBJECTIVE_STATUS: &[&str] = &["addressed", "partial", "not-addressed", "not-applicable"];
const OWNERS: &[&str] = &["prototype", "applicant", "shared"];
const GAP_STATUS: &[&str] = &["open", "mitigated", "closed"];
const ORQ_STATUS: &[&str] = &["resolved", "partly-resolved", "deferred"];
const LEVELS: &[&str] = &["A", "B", "C"];

/// The identifiers defined across the specification and the registers.
#[derive(Debug, Default)]
pub struct Ids {
    pub requirements: BTreeSet<String>,
    pub restrictions: BTreeSet<String>,
    pub non_goals: BTreeSet<String>,
    pub assumptions: BTreeSet<String>,
    /// ORQ identifier to its status in design.md.
    pub orqs: BTreeMap<String, String>,
    pub properties: BTreeSet<String>,
    pub tbr: BTreeSet<String>,
    pub crates: BTreeSet<String>,
    pub gaps: BTreeSet<String>,
}

/// The kind of an identifier: its prefix, or `R` for a requirement or
/// acceptance criterion.
fn kind(id: &str) -> Option<&'static str> {
    for k in ["GAP", "ORQ", "ASM", "TBR", "PR", "NG", "P"] {
        if id.strip_prefix(k).is_some_and(|r| r.starts_with('-')) {
            return Some(k);
        }
    }
    if id.len() > 1 && id.starts_with('R') && id.as_bytes()[1].is_ascii_digit() {
        return Some("R");
    }
    None
}

impl Ids {
    /// Collects the identifiers of requirements.md, design.md, and the
    /// Verification_Plan.
    pub fn from_texts(requirements: &str, design: &str, plan: &str) -> Ids {
        let mut ids = Ids::default();
        let mut current: Option<String> = None;
        for line in requirements.lines() {
            if let Some(rest) = line.strip_prefix("### Requirement ") {
                let n: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if n.is_empty() {
                    current = None;
                } else {
                    ids.requirements.insert(format!("R{n}"));
                    current = Some(n);
                }
                continue;
            }
            if line.starts_with('#') {
                if !line.starts_with("####") {
                    current = None;
                }
                continue;
            }
            if let Some(n) = &current {
                let m: String = line.chars().take_while(char::is_ascii_digit).collect();
                if !m.is_empty() && line[m.len()..].starts_with(". ") {
                    ids.requirements.insert(format!("R{n}.{m}"));
                }
            }
            if let Some(id) = row_id(line, "PR-") {
                ids.restrictions.insert(id);
            }
            if let Some(id) = row_id(line, "ASM-") {
                ids.assumptions.insert(id);
            }
            if let Some(rest) = line.strip_prefix("- **NG-") {
                let n: String = rest.chars().take_while(char::is_ascii_digit).collect();
                if !n.is_empty() {
                    ids.non_goals.insert(format!("NG-{n}"));
                }
            }
        }
        for line in design.lines() {
            if let Some(id) = row_id(line, "ORQ-") {
                let text = line.split('|').nth(2).unwrap_or_default().trim();
                ids.orqs.insert(id, orq_status(text).unwrap_or("unknown").to_string());
            }
        }
        for line in plan.lines() {
            if let Some(id) = row_id(line, "P-") {
                ids.properties.insert(id);
            }
        }
        ids
    }

    /// `Some(true)` when `id` is defined, `Some(false)` when it is of a
    /// known kind but undefined, `None` for an unknown kind.
    fn known(&self, id: &str) -> Option<bool> {
        Some(match kind(id)? {
            "GAP" => self.gaps.contains(id),
            "ORQ" => self.orqs.contains_key(id),
            "ASM" => self.assumptions.contains(id),
            "TBR" => self.tbr.contains(id),
            "PR" => self.restrictions.contains(id),
            "NG" => self.non_goals.contains(id),
            "P" => self.properties.contains(id),
            _ => self.requirements.contains(id),
        })
    }
}

/// The identifier in the first cell of a Markdown table row, when it has
/// the form `<prefix><digits>`.
fn row_id(line: &str, prefix: &str) -> Option<String> {
    let cell = line.strip_prefix("| ")?.split(" |").next()?.trim();
    let digits = cell.strip_prefix(prefix)?;
    (!digits.is_empty() && digits.chars().all(|c| c.is_ascii_digit())).then(|| cell.to_string())
}

/// The status of an ORQ from the first words of its design.md disposition.
pub fn orq_status(text: &str) -> Option<&'static str> {
    if text.starts_with("Partly resolved") {
        Some("partly-resolved")
    } else if text.starts_with("Resolved") {
        Some("resolved")
    } else if text.starts_with("Deferred") {
        Some("deferred")
    } else {
        None
    }
}

fn text<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key)?.as_str().filter(|s| !s.trim().is_empty())
}

fn strs(v: &Value, key: &str) -> Option<Vec<String>> {
    v.get(key)?
        .as_array()
        .map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect())
}

/// Reports every identifier in `refs` that is of a kind outside `allowed`
/// or that does not resolve.
#[allow(clippy::too_many_arguments)]
fn check_refs(ids: &Ids, refs: &[String], allowed: &[&str], rule: Rule, file: &str, owner: &str, field: &str, report: &mut Report) {
    for r in refs {
        match (kind(r), ids.known(r)) {
            (Some(k), Some(true)) if allowed.contains(&k) => {}
            (Some(k), Some(false)) if allowed.contains(&k) => report.error(
                rule,
                Some(file.to_string()),
                format!("{owner}: `{field}` cites `{r}`, which is not defined"),
            ),
            _ => report.error(
                rule,
                Some(file.to_string()),
                format!("{owner}: `{field}` entry `{r}` is not one of: {}", allowed.join(", ")),
            ),
        }
    }
}

fn parse(text: &str, file: &str, rule: Rule, report: &mut Report) -> Option<Table> {
    match text.parse::<Table>() {
        Ok(t) => Some(t),
        Err(e) => {
            report.error(rule, Some(file.to_string()), format!("cannot parse: {e}"));
            None
        }
    }
}

fn header(table: &Table, file: &str, rule: Rule, report: &mut Report) {
    for key in ["schema", "version", "updated"] {
        if table.get(key).is_none() {
            report.error(rule, Some(file.to_string()), format!("lacks the top-level `{key}`"));
        }
    }
}

fn entries<'a>(table: &'a Table, key: &str) -> &'a [Value] {
    table.get(key).and_then(Value::as_array).map(Vec::as_slice).unwrap_or(&[])
}

// ------------------------------------------------------------------ R48

/// Collects the Trust_Base_Register's identifiers into `ids`.
pub fn collect_trust_base(table: &Table, ids: &mut Ids) {
    for e in entries(table, "entry") {
        if let Some(id) = text(e, "id") {
            ids.tbr.insert(id.to_string());
        }
    }
    for c in entries(table, "crate") {
        if let Some(name) = text(c, "name") {
            ids.crates.insert(name.to_string());
        }
    }
}

pub fn check_trust_base(table: &Table, ids: &Ids, report: &mut Report) {
    let rule = Rule::TrustBaseRegister;
    let file = TRUST_BASE;
    header(table, file, rule, report);
    let mut seen = BTreeSet::new();
    for e in entries(table, "entry") {
        let id = text(e, "id").unwrap_or("[[entry]] without `id`").to_string();
        let well_formed = id.strip_prefix("TBR-").is_some_and(|n| n.len() == 2 && n.chars().all(|c| c.is_ascii_digit()));
        if !well_formed {
            report.error(rule, Some(file.to_string()), format!("`{id}` is not an identifier of the form TBR-nn"));
        }
        if !seen.insert(id.clone()) {
            report.error(rule, Some(file.to_string()), format!("`{id}` is defined twice"));
        }
        for key in ["component", "trusted_for", "why", "evidence", "residual_risk"] {
            if text(e, key).is_none() {
                report.error(rule, Some(file.to_string()), format!("{id} lacks `{key}` (R48.2)"));
            }
        }
        match strs(e, "requirements") {
            Some(reqs) if !reqs.is_empty() => check_refs(ids, &reqs, &["R", "PR"], rule, file, &id, "requirements", report),
            _ => report.error(rule, Some(file.to_string()), format!("{id} names no dependent requirement (R48.2)")),
        }
        match strs(e, "properties") {
            Some(props) if !props.is_empty() => {
                let named: Vec<String> = props.into_iter().filter(|p| p != "all").collect();
                check_refs(ids, &named, &["P"], rule, file, &id, "properties", report);
            }
            _ => report.error(
                rule,
                Some(file.to_string()),
                format!("{id} names no dependent Verification_Plan property (R48.2)"),
            ),
        }
        match strs(e, "links") {
            Some(links) => check_refs(ids, &links, &["ASM", "ORQ"], rule, file, &id, "links", report),
            None => report.error(rule, Some(file.to_string()), format!("{id} lacks `links` (R48.3)")),
        }
    }
    for c in entries(table, "crate") {
        let name = format!("[[crate]] `{}`", text(c, "name").unwrap_or("?"));
        match strs(c, "requirements") {
            Some(reqs) => check_refs(ids, &reqs, &["R", "PR"], rule, file, &name, "requirements", report),
            None => report.error(rule, Some(file.to_string()), format!("{name} lacks `requirements` (R48.2)")),
        }
        match strs(c, "links") {
            Some(links) => check_refs(ids, &links, &["ASM", "ORQ"], rule, file, &name, "links", report),
            None => report.error(rule, Some(file.to_string()), format!("{name} lacks `links` (R48.3)")),
        }
    }
    // R48.1: every listed item is mapped to entries that exist.
    let mut mapped = BTreeSet::new();
    for m in entries(table, "r48_1") {
        let Some(item) = text(m, "item") else {
            report.error(rule, Some(file.to_string()), "an [[r48_1]] row lacks `item`");
            continue;
        };
        if !R48_1_ITEMS.contains(&item) {
            report.error(rule, Some(file.to_string()), format!("[[r48_1]] item `{item}` is not an R48.1 item"));
        }
        if !mapped.insert(item.to_string()) {
            report.error(rule, Some(file.to_string()), format!("[[r48_1]] item `{item}` is mapped twice"));
        }
        let targets = strs(m, "entries").unwrap_or_default();
        if targets.is_empty() {
            report.error(rule, Some(file.to_string()), format!("[[r48_1]] item `{item}` names no entry"));
        }
        for t in targets {
            let ok = match t.strip_prefix("crate:") {
                Some(name) => ids.crates.contains(name),
                None => ids.tbr.contains(&t),
            };
            if !ok {
                report.error(rule, Some(file.to_string()), format!("[[r48_1]] item `{item}` names `{t}`, which does not exist"));
            }
        }
    }
    for item in R48_1_ITEMS {
        if !mapped.contains(*item) {
            report.error(rule, Some(file.to_string()), format!("R48.1 item `{item}` is not mapped in [[r48_1]]"));
        }
    }
    report.note(format!(
        "R48: {} Trust_Base_Register entries and {} [[crate]] entries; {} of {} R48.1 items mapped",
        ids.tbr.len(),
        ids.crates.len(),
        mapped.len(),
        R48_1_ITEMS.len()
    ));
}

// ------------------------------------------------------------------ R54

/// Collects the Gap_Register's gap identifiers into `ids`.
pub fn collect_gaps(table: &Table, ids: &mut Ids) {
    for g in entries(table, "gap") {
        if let Some(id) = text(g, "id") {
            ids.gaps.insert(id.to_string());
        }
    }
}

/// The common fields of an objective or supplement item.
fn check_status_item(v: &Value, id: &str, ids: &Ids, report: &mut Report) {
    let rule = Rule::GapRegister;
    let file = GAP_REGISTER;
    if text(v, "title").is_none() {
        report.error(rule, Some(file.to_string()), format!("{id} lacks `title`"));
    }
    let status = text(v, "status").unwrap_or_default();
    if !OBJECTIVE_STATUS.contains(&status) {
        report.error(rule, Some(file.to_string()), format!("{id}: status `{status}` is not one of {}", OBJECTIVE_STATUS.join(", ")));
    }
    let owner = text(v, "owner").unwrap_or_default();
    if !OWNERS.contains(&owner) {
        report.error(rule, Some(file.to_string()), format!("{id}: owner `{owner}` is not one of {} (R54.4)", OWNERS.join(", ")));
    }
    let refs = strs(v, "refs").unwrap_or_default();
    check_refs(ids, &refs, &["GAP", "ORQ", "ASM", "TBR", "P", "R", "PR", "NG"], rule, file, id, "refs", report);
    let evidence = strs(v, "evidence").unwrap_or_default();
    match status {
        "partial" | "not-addressed" if !refs.iter().any(|r| r.starts_with("GAP-")) => report.error(
            rule,
            Some(file.to_string()),
            format!("{id} is {status} but names no gap (R54.1)"),
        ),
        "addressed" if evidence.is_empty() => {
            report.error(rule, Some(file.to_string()), format!("{id} is addressed but names no evidence (R54.1)"))
        }
        "not-applicable" if text(v, "note").is_none() => {
            report.error(rule, Some(file.to_string()), format!("{id} is not applicable but gives no reason in `note`"))
        }
        _ => {}
    }
}

pub fn check_gap_register(table: &Table, ids: &Ids, report: &mut Report) {
    let rule = Rule::GapRegister;
    let file = GAP_REGISTER;
    header(table, file, rule, report);

    // R54.1: every Annex A objective, once.
    let expected: BTreeSet<String> = ANNEX_A
        .iter()
        .flat_map(|&(t, n)| (1..=n).map(move |o| format!("A-{t}.{o}")))
        .collect();
    let mut seen = BTreeSet::new();
    let mut tally: BTreeMap<&str, usize> = BTreeMap::new();
    let mut confirm_c = 0usize;
    let mut status_count: BTreeMap<String, usize> = BTreeMap::new();
    for o in entries(table, "objective") {
        let id = text(o, "id").unwrap_or("[[objective]] without `id`").to_string();
        if !expected.contains(&id) {
            report.error(rule, Some(file.to_string()), format!("`{id}` is not a DO-178C Annex A objective (tables A-1 to A-10)"));
        }
        if !seen.insert(id.clone()) {
            report.error(rule, Some(file.to_string()), format!("`{id}` is listed twice"));
        }
        let applies = strs(o, "applies").unwrap_or_default();
        if !applies.iter().any(|l| l == "A") {
            report.error(rule, Some(file.to_string()), format!("{id}: every Annex A objective applies at Level A"));
        }
        for level in &applies {
            match LEVELS.iter().find(|l| **l == level.as_str()) {
                Some(l) => *tally.entry(*l).or_default() += 1,
                None => report.error(rule, Some(file.to_string()), format!("{id}: level `{level}` is not one of A, B, C")),
            }
        }
        if strs(o, "confirm").unwrap_or_default().iter().any(|l| l == "C") {
            confirm_c += 1;
        }
        *status_count.entry(text(o, "status").unwrap_or_default().to_string()).or_default() += 1;
        check_status_item(o, &id, ids, report);
    }
    for missing in expected.difference(&seen) {
        report.error(rule, Some(file.to_string()), format!("objective {missing} is missing (R54.1)"));
    }
    for &(level, published) in PUBLISHED {
        let found = tally.get(level).copied().unwrap_or(0);
        if found != published {
            if level == "C" && confirm_c > 0 {
                report.note(format!(
                    "R54.1: {found} objectives marked applicable at Level C against {published} published; \
                     {confirm_c} have unconfirmed Level C applicability"
                ));
            } else {
                report.error(
                    rule,
                    Some(file.to_string()),
                    format!("{found} objectives apply at Level {level}; DO-178C has {published} (R54.1)"),
                );
            }
        }
    }

    for s in entries(table, "supplement") {
        let id = text(s, "id").unwrap_or("[[supplement]] without `id`").to_string();
        if !matches!(text(s, "standard"), Some("DO-330" | "DO-333")) {
            report.error(rule, Some(file.to_string()), format!("{id}: `standard` must be DO-330 or DO-333"));
        }
        check_status_item(s, &id, ids, report);
    }

    // The gaps.
    let mut gap_ids = BTreeSet::new();
    let mut open = 0usize;
    for g in entries(table, "gap") {
        let id = text(g, "id").unwrap_or("[[gap]] without `id`").to_string();
        let well_formed = id.strip_prefix("GAP-").is_some_and(|n| n.len() == 2 && n.chars().all(|c| c.is_ascii_digit()));
        if !well_formed {
            report.error(rule, Some(file.to_string()), format!("`{id}` is not an identifier of the form GAP-nn"));
        }
        if !gap_ids.insert(id.clone()) {
            report.error(rule, Some(file.to_string()), format!("`{id}` is defined twice"));
        }
        for key in ["title", "description", "plan"] {
            if text(g, key).is_none() {
                report.error(rule, Some(file.to_string()), format!("{id} lacks `{key}`"));
            }
        }
        let status = text(g, "status").unwrap_or_default();
        if !GAP_STATUS.contains(&status) {
            report.error(rule, Some(file.to_string()), format!("{id}: status `{status}` is not one of {}", GAP_STATUS.join(", ")));
        }
        if status != "closed" {
            open += 1;
        }
        let owner = text(g, "owner").unwrap_or_default();
        if !OWNERS.contains(&owner) {
            report.error(rule, Some(file.to_string()), format!("{id}: owner `{owner}` is not one of {} (R54.4)", OWNERS.join(", ")));
        }
        let links = strs(g, "links").unwrap_or_default();
        check_refs(ids, &links, &["ORQ", "ASM", "TBR", "P", "R", "PR", "NG"], rule, file, &id, "links", report);
    }
    // R54.2: the required gaps exist and are not closed.
    for (what, needed) in R54_2 {
        let present = entries(table, "gap").iter().any(|g| {
            let links = strs(g, "links").unwrap_or_default();
            text(g, "status") != Some("closed")
                && needed.iter().all(|n| {
                    links
                        .iter()
                        .any(|l| l == n || (*n == "R57" && l.starts_with("R57.")))
                })
        });
        if !present {
            report.error(
                rule,
                Some(file.to_string()),
                format!("no open gap records {what} (R54.2: a gap linking {})", needed.join(" and ")),
            );
        }
    }

    // R54.3: every ORQ, with the status design.md gives it.
    let mut orqs_seen = BTreeSet::new();
    for q in entries(table, "orq") {
        let id = text(q, "id").unwrap_or("[[orq]] without `id`").to_string();
        if !orqs_seen.insert(id.clone()) {
            report.error(rule, Some(file.to_string()), format!("`{id}` is listed twice"));
        }
        let status = text(q, "status").unwrap_or_default();
        if !ORQ_STATUS.contains(&status) {
            report.error(rule, Some(file.to_string()), format!("{id}: status `{status}` is not one of {}", ORQ_STATUS.join(", ")));
        }
        match ids.orqs.get(&id) {
            None => report.error(rule, Some(file.to_string()), format!("{id} is not an ORQ of design.md")),
            Some(design) if design != status => report.error(
                rule,
                Some(file.to_string()),
                format!(
                    "{id}: design.md gives the status `{design}`, the Gap_Register `{status}`; \
                     update both in the same change (R54.3)"
                ),
            ),
            Some(_) => {}
        }
        let gaps = strs(q, "gaps").unwrap_or_default();
        check_refs(ids, &gaps, &["GAP"], rule, file, &id, "gaps", report);
        let certification = q.get("certification").and_then(Value::as_bool);
        match certification {
            None => report.error(rule, Some(file.to_string()), format!("{id} lacks the boolean `certification`")),
            Some(true) if status != "resolved" && gaps.is_empty() => report.error(
                rule,
                Some(file.to_string()),
                format!("{id} affects certification evidence and is {status}, but names no gap"),
            ),
            Some(_) => {}
        }
        if text(q, "note").is_none() {
            report.error(rule, Some(file.to_string()), format!("{id} lacks `note`"));
        }
    }
    for (id, status) in &ids.orqs {
        if status == "unknown" {
            report.error(
                rule,
                Some(DESIGN.to_string()),
                format!("{id}: the disposition does not start with Resolved, Partly resolved, or Deferred"),
            );
        }
        if !orqs_seen.contains(id) {
            report.error(
                rule,
                Some(file.to_string()),
                format!("{id} (design.md: {status}) is not in the Gap_Register (R54.3)"),
            );
        }
    }

    let statuses: Vec<String> = status_count.iter().map(|(s, n)| format!("{n} {s}")).collect();
    report.note(format!(
        "R54: {} Annex A objectives ({}); {} supplement items; {} gaps, {open} not closed; {} ORQs mirrored from design.md",
        seen.len(),
        statuses.join(", "),
        entries(table, "supplement").len(),
        gap_ids.len(),
        orqs_seen.len()
    ));
}

/// Every `GAP-nn` cited in `text` resolves.
pub fn check_gap_citations(file: &str, text: &str, ids: &Ids, report: &mut Report) {
    for (n, line) in text.lines().enumerate() {
        let mut rest = line;
        while let Some(at) = rest.find("GAP-") {
            let tail = &rest[at + 4..];
            let digits: String = tail.chars().take_while(char::is_ascii_digit).collect();
            if digits.len() == 2 {
                let id = format!("GAP-{digits}");
                if !ids.gaps.contains(&id) {
                    report.error(
                        Rule::GapRegister,
                        Some(format!("{file}:{}", n + 1)),
                        format!("cites `{id}`, which the Gap_Register does not define"),
                    );
                }
            }
            rest = tail;
        }
    }
}

fn read(root: &Path, file: &str, rule: Rule, report: &mut Report) -> Option<String> {
    match std::fs::read_to_string(root.join(file)) {
        Ok(t) => Some(t),
        Err(e) => {
            report.error(rule, Some(file.to_string()), format!("cannot read: {e}"));
            None
        }
    }
}

/// Runs both register checks on the repository at `root`.
pub fn check(root: &Path, report: &mut Report) {
    let (Some(requirements), Some(design), Some(plan)) = (
        read(root, REQUIREMENTS, Rule::GapRegister, report),
        read(root, DESIGN, Rule::GapRegister, report),
        read(root, VERIFICATION_PLAN, Rule::GapRegister, report),
    ) else {
        return;
    };
    let mut ids = Ids::from_texts(&requirements, &design, &plan);
    let tbr = read(root, TRUST_BASE, Rule::TrustBaseRegister, report)
        .and_then(|t| parse(&t, TRUST_BASE, Rule::TrustBaseRegister, report));
    let gap = read(root, GAP_REGISTER, Rule::GapRegister, report).and_then(|t| parse(&t, GAP_REGISTER, Rule::GapRegister, report));
    if let Some(t) = &tbr {
        collect_trust_base(t, &mut ids);
    }
    if let Some(g) = &gap {
        collect_gaps(g, &mut ids);
    }
    if let Some(t) = &tbr {
        check_trust_base(t, &ids, report);
    }
    if let Some(g) = &gap {
        check_gap_register(g, &ids, report);
        for file in GAP_CITERS {
            if let Ok(text) = std::fs::read_to_string(root.join(file)) {
                check_gap_citations(file, &text, &ids, report);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const REQUIREMENTS_TEXT: &str = "\
## Requirements
### Requirement 1: One
#### Acceptance Criteria
1. THE thing SHALL work.
2. THE other SHALL too.
### Requirement 57: Core
#### Acceptance Criteria
4. THE gap SHALL be recorded.
## Tables
| PR-01 | rule |
| ASM-01 | assumption |
- **NG-01** non-goal
";

    const DESIGN_TEXT: &str = "\
| ORQ-01 | Resolved by the spike | x |
| ORQ-02 | Partly resolved: half | y |
| ORQ-03 | Deferred to P3 | z |
";

    const PLAN_TEXT: &str = "| P-01 | Data-race freedom | x |\n";

    fn ids() -> Ids {
        Ids::from_texts(REQUIREMENTS_TEXT, DESIGN_TEXT, PLAN_TEXT)
    }

    /// A Gap_Register with every Annex A objective addressed at levels A to
    /// C except A-7.5 and A-7.9 (A only) and A-7.6 (A and B), and with
    /// `extra` appended.
    fn gap_register(extra: &str) -> String {
        let mut s = String::from("schema = 1\nversion = \"1\"\nupdated = \"2026-10-14\"\n");
        for &(t, n) in ANNEX_A {
            for o in 1..=n {
                let applies = match (t, o) {
                    (7, 5) | (7, 9) => "[\"A\"]",
                    (7, 6) => "[\"A\", \"B\"]",
                    _ => "[\"A\", \"B\", \"C\"]",
                };
                s.push_str(&format!(
                    "[[objective]]\nid = \"A-{t}.{o}\"\ntitle = \"t\"\napplies = {applies}\nstatus = \"addressed\"\n\
                     owner = \"prototype\"\nevidence = [\"e\"]\nrefs = []\nnote = \"n\"\n"
                ));
            }
        }
        s.push_str(extra);
        s
    }

    const GAPS: &str = r#"
[[gap]]
id = "GAP-01"
title = "t"
status = "open"
owner = "shared"
links = ["ORQ-01", "R1.1"]
description = "d"
plan = "p"
"#;

    fn run(gap: &str) -> Report {
        let table: Table = gap.parse().unwrap();
        let mut ids = ids();
        collect_gaps(&table, &mut ids);
        let mut report = Report::default();
        check_gap_register(&table, &ids, &mut report);
        report
    }

    fn messages(report: &Report) -> String {
        report.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n")
    }

    #[test]
    fn parses_identifiers_and_orq_statuses() {
        let ids = ids();
        for r in ["R1", "R1.1", "R1.2", "R57", "R57.4"] {
            assert!(ids.requirements.contains(r), "{r}");
        }
        assert!(!ids.requirements.contains("R1.3"));
        assert!(ids.restrictions.contains("PR-01") && ids.assumptions.contains("ASM-01") && ids.non_goals.contains("NG-01"));
        assert_eq!(ids.orqs.get("ORQ-01").map(String::as_str), Some("resolved"));
        assert_eq!(ids.orqs.get("ORQ-02").map(String::as_str), Some("partly-resolved"));
        assert_eq!(ids.orqs.get("ORQ-03").map(String::as_str), Some("deferred"));
        assert!(ids.properties.contains("P-01"));
        assert_eq!(kind("PR-01"), Some("PR"));
        assert_eq!(kind("P-01"), Some("P"));
        assert_eq!(kind("R16.10"), Some("R"));
        assert_eq!(kind("KS-01"), None);
    }

    #[test]
    fn an_orq_status_change_without_a_register_update_is_reported_naming_r54_3() {
        let orqs = r#"
[[orq]]
id = "ORQ-01"
status = "resolved"
certification = true
gaps = []
note = "n"
[[orq]]
id = "ORQ-02"
status = "deferred"
certification = true
gaps = ["GAP-01"]
note = "n"
"#;
        let text = messages(&run(&gap_register(&format!("{GAPS}{orqs}"))));
        assert!(text.contains("ORQ-02: design.md gives the status `partly-resolved`, the Gap_Register `deferred`"), "{text}");
        assert!(text.contains("ORQ-03 (design.md: deferred) is not in the Gap_Register (R54.3)"), "{text}");
    }

    #[test]
    fn shortfalls_without_gaps_and_missing_r54_2_gaps_are_reported() {
        let mut gap = gap_register(GAPS);
        gap = gap.replacen(
            "id = \"A-1.1\"\ntitle = \"t\"\napplies = [\"A\", \"B\", \"C\"]\nstatus = \"addressed\"",
            "id = \"A-1.1\"\ntitle = \"t\"\napplies = [\"A\", \"B\", \"C\"]\nstatus = \"partial\"",
            1,
        );
        let text = messages(&run(&gap));
        assert!(text.contains("A-1.1 is partial but names no gap (R54.1)"), "{text}");
        assert!(text.contains("no open gap records Rust MC/DC tooling"), "{text}");
        // GAP-01 links ORQ-01 but not R16.10, so the partitioning gap is missing too.
        assert!(text.contains("robust partitioning evidence"), "{text}");
    }

    #[test]
    fn level_tallies_must_match_the_published_counts() {
        // The base register marks 68 objectives at Level C; making A-7.5 apply
        // at Level B too gives 70 at Level B against 69.
        let gap = gap_register(GAPS).replacen("applies = [\"A\"]", "applies = [\"A\", \"B\"]", 1);
        let text = messages(&run(&gap));
        assert!(text.contains("70 objectives apply at Level B; DO-178C has 69 (R54.1)"), "{text}");
        assert!(text.contains("68 objectives apply at Level C; DO-178C has 62"), "{text}");
        // Declaring Level C unconfirmed turns the Level C mismatch into a note.
        let confirmed = gap_register(GAPS).replacen("note = \"n\"", "note = \"n\"\nconfirm = [\"C\"]", 1);
        let report = run(&confirmed);
        assert!(!messages(&report).contains("Level C"), "{}", messages(&report));
        assert!(report.notes.iter().any(|n| n.contains("unconfirmed Level C applicability")), "{:#?}", report.notes);
    }

    #[test]
    fn undefined_references_and_gap_citations_are_reported() {
        let gap = gap_register(&GAPS.replace("\"R1.1\"", "\"R1.9\""));
        let text = messages(&run(&gap));
        assert!(text.contains("GAP-01: `links` cites `R1.9`, which is not defined"), "{text}");
        let mut report = Report::default();
        let mut ids = ids();
        ids.gaps.insert("GAP-01".to_string());
        check_gap_citations("doc.md", "see GAP-01 and GAP-02", &ids, &mut report);
        assert_eq!(report.diagnostics.len(), 1, "{}", messages(&report));
        assert!(messages(&report).contains("doc.md:1: cites `GAP-02`"));
    }

    #[test]
    fn trust_base_entries_need_every_r48_2_field_and_every_r48_1_item() {
        let tbr: Table = r#"
schema = 1
version = "1"
updated = "2026-10-14"
[[entry]]
id = "TBR-01"
component = "c"
trusted_for = "t"
evidence = "e"
residual_risk = "r"
requirements = ["R1.1"]
properties = ["P-01"]
links = ["ASM-01", "ORQ-09"]
[[r48_1]]
item = "linker"
entries = ["TBR-01", "crate:missing"]
"#
        .parse()
        .unwrap();
        let mut ids = ids();
        collect_trust_base(&tbr, &mut ids);
        let mut report = Report::default();
        check_trust_base(&tbr, &ids, &mut report);
        let text = messages(&report);
        assert!(text.contains("TBR-01 lacks `why` (R48.2)"), "{text}");
        assert!(text.contains("TBR-01: `links` cites `ORQ-09`, which is not defined"), "{text}");
        assert!(text.contains("[[r48_1]] item `linker` names `crate:missing`, which does not exist"), "{text}");
        assert!(text.contains("R48.1 item `build-system` is not mapped in [[r48_1]]"), "{text}");
    }
}
