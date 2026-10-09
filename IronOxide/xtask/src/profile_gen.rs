//! The Profile checker and generator (Component A; task 2.1):
//! `cargo xtask profile [--check]`.
//!
//! The Profile is written once, as `profile/profile.toml`. This module
//! checks it against Requirements 1, 2, 4, and 27.3 and against the
//! Core_Subset source `profile/core_subset.toml` (R57), then generates:
//!
//! - `docs/profile.md`, the Profile specification document (R1.5);
//! - `crates/rsk-kernel/src/profile_version.rs` and
//!   `crates/rsk-gen/src/profile_version.rs`, the `PROFILE_VERSION`
//!   constants that the Kernel exposes as linked symbols (R6.7) and that the
//!   Generator records in every Task_Model (R1.4).
//!
//! `--check` regenerates in memory and fails if a generated file differs
//! from the file on disk, so that the verification job rejects a stale
//! document. The checks are, by requirement:
//!
//! | Rule | Check |
//! |---|---|
//! | R1.1 | identifiers `PR-nn`, unique; retired identifiers listed and never reused; reserved identifiers not active |
//! | R1.2 | rule, guarantee, enforcement categories from {TS, MC, LN, LK, PO, RT} (primary first), analogue, and at least one Evidence_Item per listed category |
//! | R1.3 | an ASM identifier or a reason for every restriction that lists RT |
//! | R1.4 | version identifier; the change-log entry of the current version carries the content hash of the file; hashes never repeat |
//! | R1.5 | catalogue, Table 4-1, and disallowed-crate list present |
//! | R1.6 | cumulative change log, one entry per change with version, change, rationale, and requirements |
//! | R2.1 | PR-01 to PR-36 active |
//! | R2.2 | subjects and crate coverage per restriction |
//! | R2.3 | Evidence_Item names follow the kinds of R2.3 per category |
//! | R4.1 | exactly the 29 Table 4-1 rows, each classified Enforced, Not applicable by construction, or Deviation with the required text |
//! | R4.2 | Jorvik markers: Relaxed or Kept (deviation) for the seven Jorvik relaxations, Same elsewhere |
//! | R4.3, R4.6 | the Task_Dispatching_Policy row states the DD-03 order and cites R29.1 |
//! | R4.4 | the two Execution_Time rows are Deviations citing PR-24, PR-27, Requirement 18, and R29.3, R29.4 |
//! | R4.5 | two-way consistency between Table 2-1 analogues and Table 4-1 citations |
//! | R4.7 | the No_Abort_Statements and No_Task_Termination rows name the responses and cite R14.9, R18.4 |
//! | R27.3 | reserved inactive identifiers for the multi-core extension |
//! | R57.1, R57.5 | every Core_Subset item names its certification evidence or the substitute rsk evidence |

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml::{Table, Value};

use crate::diag::{Report, Rule};

pub const SOURCE: &str = "profile/profile.toml";
pub const CORE_SUBSET: &str = "profile/core_subset.toml";
pub const DOC: &str = "docs/profile.md";
pub const VERSION_FILES: &[(&str, &str)] = &[
    ("rsk-kernel", "crates/rsk-kernel/src/profile_version.rs"),
    ("rsk-gen", "crates/rsk-gen/src/profile_version.rs"),
];

pub const CATEGORIES: &[&str] = &["TS", "MC", "LN", "LK", "PO", "RT"];

/// The 3 pragmas and 26 restrictions of Ada RM D.13 (Ravenscar), R4.1.
pub const D13_ITEMS: &[(&str, &str)] = &[
    ("Task_Dispatching_Policy (FIFO_Within_Priorities)", "pragma"),
    ("Locking_Policy (Ceiling_Locking)", "pragma"),
    ("Detect_Blocking", "pragma"),
    ("No_Abort_Statements", "restriction"),
    ("No_Dynamic_Attachment", "restriction"),
    ("No_Dynamic_CPU_Assignment", "restriction"),
    ("No_Dynamic_Priorities", "restriction"),
    ("No_Implicit_Heap_Allocations", "restriction"),
    ("No_Local_Protected_Objects", "restriction"),
    ("No_Local_Timing_Events", "restriction"),
    ("No_Protected_Type_Allocators", "restriction"),
    ("No_Relative_Delay", "restriction"),
    ("No_Requeue_Statements", "restriction"),
    ("No_Select_Statements", "restriction"),
    ("No_Specific_Termination_Handlers", "restriction"),
    ("No_Task_Allocators", "restriction"),
    ("No_Task_Hierarchy", "restriction"),
    ("No_Task_Termination", "restriction"),
    ("Simple_Barriers", "restriction"),
    ("Max_Entry_Queue_Length => 1", "restriction"),
    ("Max_Protected_Entries => 1", "restriction"),
    ("Max_Task_Entries => 0", "restriction"),
    ("No_Dependence => Ada.Asynchronous_Task_Control", "restriction"),
    ("No_Dependence => Ada.Calendar", "restriction"),
    ("No_Dependence => Ada.Execution_Time.Group_Budgets", "restriction"),
    ("No_Dependence => Ada.Execution_Time.Timers", "restriction"),
    ("No_Dependence => Ada.Synchronous_Barriers", "restriction"),
    ("No_Dependence => Ada.Task_Attributes", "restriction"),
    ("No_Dependence => System.Multiprocessors.Dispatching_Domains", "restriction"),
];

/// The Jorvik relaxations of R4.2.
pub const JORVIK_RELAXATIONS: &[&str] = &[
    "No_Implicit_Heap_Allocations",
    "No_Relative_Delay",
    "Max_Entry_Queue_Length => 1",
    "Max_Protected_Entries => 1",
    "No_Dependence => Ada.Calendar",
    "No_Dependence => Ada.Synchronous_Barriers",
    "Simple_Barriers",
];

#[derive(Clone, Debug, Default)]
pub struct Restriction {
    pub id: String,
    pub rule: String,
    pub guarantee: String,
    pub enforcement: Vec<(String, String)>,
    pub analogue: Vec<String>,
    pub analogue_note: Option<String>,
    pub subjects: Vec<String>,
    pub crate_coverage: String,
    pub rt_assumption: Option<String>,
    pub rt_reason: Option<String>,
    pub evidence: BTreeMap<String, Vec<String>>,
}

#[derive(Clone, Debug, Default)]
pub struct Row {
    pub item: String,
    pub kind: String,
    pub classification: String,
    pub cites: Vec<String>,
    pub treatment: String,
    pub difference: Option<String>,
    pub rationale: Option<String>,
    pub constrained_by: Vec<String>,
    pub constraint_note: Option<String>,
    pub construct: Option<String>,
    pub jorvik: String,
    pub jorvik_cites: Vec<String>,
    pub jorvik_rationale: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct ChangeEntry {
    pub subject: String,
    pub change: String,
    pub rationale: String,
    pub requirements: Vec<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Change {
    pub version: String,
    pub date: String,
    pub content_hash: String,
    pub changes: Vec<ChangeEntry>,
}

#[derive(Clone, Debug, Default)]
pub struct CoreItem {
    pub path: String,
    pub scope: Vec<String>,
    pub certification: String,
    pub substitute: Option<String>,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Default)]
pub struct Profile {
    pub version: String,
    pub default_variant: String,
    pub variants: Vec<String>,
    pub requirements_draft: String,
    pub retired: Vec<String>,
    pub reserved: Vec<(String, String)>,
    pub disallowed: BTreeMap<String, String>,
    pub restrictions: Vec<Restriction>,
    pub d13: Vec<Row>,
    pub changes: Vec<Change>,
    pub core_subset: Vec<CoreItem>,
}

fn s(v: &Value, key: &str) -> String {
    v.get(key).and_then(Value::as_str).unwrap_or_default().to_string()
}

fn opt(v: &Value, key: &str) -> Option<String> {
    v.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|t| !t.is_empty())
        .map(str::to_string)
}

fn list(v: &Value, key: &str) -> Vec<String> {
    v.get(key)
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_string)
        .collect()
}

fn arrays<'a>(t: &'a Table, key: &str) -> impl Iterator<Item = &'a Value> {
    t.get(key).and_then(Value::as_array).into_iter().flatten()
}

/// Parses the Profile source and the Core_Subset source.
pub fn load(root: &Path) -> Result<(Profile, String), String> {
    let text = std::fs::read_to_string(root.join(SOURCE)).map_err(|e| format!("cannot read {SOURCE}: {e}"))?;
    let table: Table = text.parse().map_err(|e| format!("cannot parse {SOURCE}: {e}"))?;
    let head = table.get("profile").and_then(Value::as_table).ok_or("no [profile] table")?;
    let head_v = Value::Table(head.clone());
    let mut profile = Profile {
        version: s(&head_v, "version"),
        default_variant: s(&head_v, "default_variant"),
        variants: list(&head_v, "variants"),
        requirements_draft: s(&head_v, "requirements_draft"),
        retired: list(&head_v, "retired"),
        ..Profile::default()
    };
    for r in arrays(&table, "reserved") {
        profile.reserved.push((s(r, "id"), s(r, "purpose")));
    }
    if let Some(d) = table.get("disallowed_crates").and_then(Value::as_table) {
        for (k, v) in d {
            profile
                .disallowed
                .insert(k.clone(), v.as_str().unwrap_or_default().to_string());
        }
    }
    for r in arrays(&table, "restriction") {
        let mut enforcement = Vec::new();
        for e in r.get("enforcement").and_then(Value::as_array).into_iter().flatten() {
            enforcement.push((s(e, "category"), s(e, "mechanism")));
        }
        let mut evidence = BTreeMap::new();
        if let Some(ev) = r.get("evidence").and_then(Value::as_table) {
            for (cat, items) in ev {
                evidence.insert(
                    cat.clone(),
                    items
                        .as_array()
                        .into_iter()
                        .flatten()
                        .filter_map(Value::as_str)
                        .map(str::to_string)
                        .collect(),
                );
            }
        }
        profile.restrictions.push(Restriction {
            id: s(r, "id"),
            rule: s(r, "rule"),
            guarantee: s(r, "guarantee"),
            enforcement,
            analogue: list(r, "analogue"),
            analogue_note: opt(r, "analogue_note"),
            subjects: list(r, "subjects"),
            crate_coverage: s(r, "crate_coverage"),
            rt_assumption: opt(r, "rt_assumption"),
            rt_reason: opt(r, "rt_reason"),
            evidence,
        });
    }
    for r in arrays(&table, "d13") {
        profile.d13.push(Row {
            item: s(r, "item"),
            kind: s(r, "kind"),
            classification: s(r, "classification"),
            cites: list(r, "cites"),
            treatment: s(r, "treatment"),
            difference: opt(r, "difference"),
            rationale: opt(r, "rationale"),
            constrained_by: list(r, "constrained_by"),
            constraint_note: opt(r, "constraint_note"),
            construct: opt(r, "construct"),
            jorvik: s(r, "jorvik"),
            jorvik_cites: list(r, "jorvik_cites"),
            jorvik_rationale: opt(r, "jorvik_rationale"),
        });
    }
    for c in arrays(&table, "change") {
        let mut changes = Vec::new();
        for e in c.get("changes").and_then(Value::as_array).into_iter().flatten() {
            changes.push(ChangeEntry {
                subject: s(e, "subject"),
                change: s(e, "change"),
                rationale: s(e, "rationale"),
                requirements: list(e, "requirements"),
            });
        }
        profile.changes.push(Change {
            version: s(c, "version"),
            date: s(c, "date"),
            content_hash: s(c, "content_hash"),
            changes,
        });
    }
    let core_text = std::fs::read_to_string(root.join(CORE_SUBSET))
        .map_err(|e| format!("cannot read {CORE_SUBSET}: {e}"))?;
    let core: Table = core_text.parse().map_err(|e| format!("cannot parse {CORE_SUBSET}: {e}"))?;
    for i in arrays(&core, "item") {
        profile.core_subset.push(CoreItem {
            path: s(i, "path"),
            scope: list(i, "scope"),
            certification: s(i, "certification"),
            substitute: opt(i, "substitute"),
            note: opt(i, "note"),
        });
    }
    Ok((profile, text))
}

/// The content hash of the source: SHA-256 over the text with every
/// `content_hash = "..."` value blanked, so that recording the hash does
/// not change it.
pub fn content_hash(text: &str) -> String {
    let mut blanked = String::with_capacity(text.len());
    for line in text.lines() {
        let trimmed = line.trim_start();
        if trimmed.starts_with("content_hash") && trimmed.contains('=') {
            let indent = &line[..line.len() - trimmed.len()];
            blanked.push_str(indent);
            blanked.push_str("content_hash = \"\"");
        } else {
            blanked.push_str(line);
        }
        blanked.push('\n');
    }
    rsk_digest::sha256_hex(blanked.as_bytes())
}

/// Replaces the `content_hash` of the newest change-log entry with `hash`.
pub fn with_recorded_hash(text: &str, hash: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut done = false;
    for line in text.lines() {
        let trimmed = line.trim_start();
        if !done && trimmed.starts_with("content_hash") && trimmed.contains('=') {
            let indent = &line[..line.len() - trimmed.len()];
            out.push_str(&format!("{indent}content_hash = \"{hash}\""));
            done = true;
        } else {
            out.push_str(line);
        }
        out.push('\n');
    }
    out
}

fn is_pr(id: &str) -> bool {
    id.strip_prefix("PR-")
        .is_some_and(|d| d.len() == 2 && d.bytes().all(|b| b.is_ascii_digit()))
}

fn mentions(text: &str, needle: &str) -> bool {
    text.contains(needle)
}

/// Runs every check; diagnostics name the violated requirement.
pub fn check(profile: &Profile, text: &str, report: &mut Report) {
    let loc = || Some(SOURCE.to_string());
    let err = |report: &mut Report, req: &str, msg: String| {
        report.error(Rule::Profile, loc(), format!("{req}: {msg}"));
    };

    // R1.4, R1.5
    if profile.version.is_empty() {
        err(report, "R1.4", "the Profile has no version identifier".into());
    }
    if profile.restrictions.is_empty() {
        err(report, "R1.5", "no restriction catalogue".into());
    }
    if profile.d13.is_empty() {
        err(report, "R1.5", "no Table 4-1".into());
    }
    if profile.disallowed.is_empty() {
        err(report, "R1.5", "no disallowed-crate list (PR-35)".into());
    }

    // R1.1, R2.1
    let mut seen = BTreeSet::new();
    for r in &profile.restrictions {
        if !is_pr(&r.id) {
            err(report, "R1.1", format!("`{}` is not of the form PR-nn", r.id));
        }
        if !seen.insert(r.id.clone()) {
            err(report, "R1.1", format!("`{}` is defined twice", r.id));
        }
        if profile.retired.contains(&r.id) {
            err(report, "R1.1", format!("`{}` is both active and retired", r.id));
        }
        if profile.reserved.iter().any(|(id, _)| *id == r.id) {
            err(report, "R1.1", format!("`{}` is both active and reserved", r.id));
        }
    }
    let expected: BTreeSet<String> = (1..=36).map(|n| format!("PR-{n:02}")).collect();
    for id in expected.difference(&seen) {
        err(report, "R2.1", format!("`{id}` of Table 2-1 is not an active restriction"));
    }

    // R27.3
    let purposes: String = profile
        .reserved
        .iter()
        .map(|(_, p)| p.to_lowercase())
        .collect::<Vec<_>>()
        .join(" ");
    for (needle, what) in [
        ("core", "static Task-to-core assignment"),
        ("per-core srp", "per-core SRP"),
        ("endpoints", "cross-core sharing only through Endpoints or MSRP/MrsP"),
    ] {
        if !purposes.contains(needle) {
            err(report, "R27.3", format!("no reserved identifier for {what}"));
        }
    }
    for (id, _) in &profile.reserved {
        if !is_pr(id) {
            err(report, "R27.3", format!("reserved identifier `{id}` is not of the form PR-nn"));
        }
    }

    // R1.2, R1.3, R2.2, R2.3
    for r in &profile.restrictions {
        let id = &r.id;
        if r.rule.trim().is_empty() || r.guarantee.trim().is_empty() {
            err(report, "R1.2", format!("{id} lacks its rule or guarantee"));
        }
        if r.enforcement.is_empty() {
            err(report, "R1.2", format!("{id} lists no enforcement category"));
        }
        let cats: Vec<&str> = r.enforcement.iter().map(|(c, _)| c.as_str()).collect();
        for c in &cats {
            if !CATEGORIES.contains(c) {
                err(report, "R1.2", format!("{id} lists the unknown enforcement category `{c}`"));
            }
        }
        if r.analogue.is_empty() {
            err(report, "R1.2", format!("{id} names no Ada D.13 analogue (or the marker rsk-specific)"));
        }
        for a in &r.analogue {
            if a != "rsk-specific" && !D13_ITEMS.iter().any(|(item, _)| item == a) {
                err(report, "R1.2", format!("{id} names `{a}`, which is not a Table 4-1 item"));
            }
        }
        if r.analogue.len() > 1 && r.analogue.iter().any(|a| a == "rsk-specific") {
            err(report, "R1.2", format!("{id} mixes rsk-specific with Table 4-1 items"));
        }
        for c in &cats {
            match r.evidence.get(*c) {
                Some(items) if !items.is_empty() => {
                    for item in items {
                        let ok = match *c {
                            "TS" | "MC" | "LN" | "LK" => item == &format!("conf-{id}-{}", c.to_lowercase()),
                            "PO" => item.starts_with("proof-"),
                            "RT" => item == &format!("fi-{id}"),
                            _ => true,
                        };
                        if !ok {
                            err(
                                report,
                                "R2.3",
                                format!("{id}: Evidence_Item `{item}` is not of the kind R2.3 gives for {c}"),
                            );
                        }
                    }
                }
                _ => err(report, "R1.2", format!("{id} names no Evidence_Item for category {c}")),
            }
        }
        for c in r.evidence.keys() {
            if !cats.contains(&c.as_str()) {
                err(report, "R1.2", format!("{id} names evidence for {c}, a category it does not list"));
            }
        }
        if cats.contains(&"RT") && r.rt_assumption.is_none() && r.rt_reason.is_none() {
            err(report, "R1.3", format!("{id} lists RT but states neither an assumption nor a reason"));
        }
        if let Some(a) = &r.rt_assumption {
            if !a.starts_with("ASM-") {
                err(report, "R1.3", format!("{id}: `{a}` is not an ASM identifier"));
            }
        }
        if r.subjects.is_empty() || r.crate_coverage.trim().is_empty() {
            err(report, "R2.2", format!("{id} lacks subjects or crate coverage"));
        }
    }

    // R4.1
    let items: Vec<&str> = profile.d13.iter().map(|r| r.item.as_str()).collect();
    for (item, kind) in D13_ITEMS {
        match profile.d13.iter().filter(|r| r.item == *item).count() {
            1 => {
                let row = profile.d13.iter().find(|r| r.item == *item).expect("present");
                if row.kind != *kind {
                    err(report, "R4.1", format!("`{item}` is a {kind}, not a {}", row.kind));
                }
            }
            0 => err(report, "R4.1", format!("Table 4-1 lacks the row `{item}`")),
            n => err(report, "R4.1", format!("Table 4-1 has {n} rows for `{item}`")),
        }
    }
    for item in &items {
        if !D13_ITEMS.iter().any(|(i, _)| i == item) {
            err(report, "R4.1", format!("Table 4-1 has the extra row `{item}`"));
        }
    }
    let defined: BTreeSet<&str> = profile.restrictions.iter().map(|r| r.id.as_str()).collect();
    for row in &profile.d13 {
        let item = &row.item;
        match row.classification.as_str() {
            "Enforced" => {
                if row.cites.is_empty() {
                    err(report, "R4.1", format!("Enforced row `{item}` cites nothing"));
                }
            }
            "Not applicable by construction" => {
                if row.construct.is_none() || row.cites.is_empty() {
                    err(report, "R4.1", format!("row `{item}` must name the construct rsk does not provide and cite what prevents it"));
                }
            }
            "Deviation" => {
                if row.difference.is_none() || row.rationale.is_none() || row.constrained_by.is_empty() {
                    err(report, "R4.1", format!("Deviation row `{item}` must state the difference, the rationale, and the constraining requirements"));
                }
            }
            other => err(report, "R4.1", format!("row `{item}` has the classification `{other}`")),
        }
        for c in row.cites.iter().chain(&row.constrained_by).chain(&row.jorvik_cites) {
            if c.starts_with("PR-") && !defined.contains(c.as_str()) {
                err(report, "R4.5", format!("row `{item}` cites `{c}`, which Table 2-1 does not define"));
            }
        }
        // R4.5 forward: every PR whose analogue names the item is cited.
        for r in &profile.restrictions {
            if r.analogue.iter().any(|a| a == item) && !row.cites.contains(&r.id) {
                err(report, "R4.5", format!("row `{item}` does not cite {}, whose analogue names the item", r.id));
            }
        }
        // R4.5 backward: Enforced rows cite only PRs whose analogue names the item.
        if row.classification == "Enforced" {
            for c in row.cites.iter().filter(|c| c.starts_with("PR-")) {
                let names = profile
                    .restrictions
                    .iter()
                    .any(|r| r.id == *c && r.analogue.iter().any(|a| a == item));
                if !names {
                    err(report, "R4.5", format!("Enforced row `{item}` cites {c}, whose analogue does not name the item"));
                }
            }
        }
        // R4.2
        if JORVIK_RELAXATIONS.contains(&item.as_str()) {
            match row.jorvik.as_str() {
                "Relaxed" => {
                    if !row.jorvik_cites.iter().any(|c| c.starts_with("R5."))
                        || !row.jorvik_cites.iter().any(|c| c.starts_with("PR-"))
                    {
                        err(report, "R4.2", format!("Relaxed row `{item}` must cite an R5 criterion and a PR"));
                    }
                }
                "Kept (deviation)" => {
                    if !row.jorvik_cites.iter().any(|c| c.starts_with("PR-")) || row.jorvik_rationale.is_none() {
                        err(report, "R4.2", format!("Kept row `{item}` must cite the PR that keeps the treatment and state the rationale"));
                    }
                }
                other => err(report, "R4.2", format!("row `{item}` is a Jorvik relaxation but is marked `{other}`")),
            }
        } else if row.jorvik != "Same" {
            err(report, "R4.2", format!("row `{item}` is not a Jorvik relaxation but is marked `{}`", row.jorvik));
        }
    }
    let row = |item: &str| profile.d13.iter().find(|r| r.item == item);
    // R4.3, R4.6
    if let Some(r) = row("Task_Dispatching_Policy (FIFO_Within_Priorities)") {
        let diff = r.difference.clone().unwrap_or_default();
        if r.classification != "Deviation" || !mentions(&diff, "DD-03") || !r.cites.iter().any(|c| c == "R29.1") {
            err(report, "R4.6", "the Task_Dispatching_Policy row must be a Deviation stating the DD-03 order and citing R29.1".into());
        }
    }
    // R4.4
    for item in [
        "No_Dependence => Ada.Execution_Time.Group_Budgets",
        "No_Dependence => Ada.Execution_Time.Timers",
    ] {
        if let Some(r) = row(item) {
            let text = format!("{} {}", r.treatment, r.difference.clone().unwrap_or_default());
            let ok = r.classification == "Deviation"
                && r.cites.iter().any(|c| c == "PR-24")
                && r.cites.iter().any(|c| c == "PR-27")
                && r.cites.iter().any(|c| c == "R18")
                && ["Delta_timer", "Delta_detect", "Delta_enforce"].iter().all(|d| mentions(&text, d))
                && r.constrained_by.iter().any(|c| c == "R29.3")
                && r.constrained_by.iter().any(|c| c == "R29.4");
            if !ok {
                err(report, "R4.4", format!("row `{item}` must be a Deviation citing PR-24, PR-27, R18, the Table 13-1 costs, and R29.3/R29.4"));
            }
        }
    }
    // R4.7
    for item in ["No_Abort_Statements", "No_Task_Termination"] {
        if let Some(r) = row(item) {
            let diff = r.difference.clone().unwrap_or_default();
            let ok = r.classification == "Deviation"
                && r.cites.iter().any(|c| c == "PR-02")
                && mentions(&diff, "RESTART_PARTITION")
                && mentions(&diff, "STOP_PARTITION")
                && mentions(&diff, "SAFE_STATE")
                && r.constrained_by.iter().any(|c| c == "R14.9")
                && r.constrained_by.iter().any(|c| c == "R18.4");
            if !ok {
                err(report, "R4.7", format!("row `{item}` must be a Deviation citing PR-02, naming the responses, and constrained by R14.9 and R18.4"));
            }
        }
    }

    // R1.4, R1.6
    if profile.changes.is_empty() {
        err(report, "R1.6", "no change log".into());
    } else {
        let newest = &profile.changes[0];
        if newest.version != profile.version {
            err(report, "R1.6", format!("the newest change-log entry is for {} but the Profile version is {}", newest.version, profile.version));
        }
        let computed = content_hash(text);
        if newest.content_hash != computed {
            err(
                report,
                "R1.4",
                format!(
                    "the content hash recorded for version {} is `{}` but the file hashes to `{computed}`: the \
                     content changed, so bump the version, add a change-log entry, and run `cargo xtask profile` \
                     to record the hash",
                    newest.version, newest.content_hash
                ),
            );
        }
        let mut versions = BTreeSet::new();
        let mut hashes = BTreeSet::new();
        for c in &profile.changes {
            if !versions.insert(c.version.clone()) {
                err(report, "R1.4", format!("version {} appears twice in the change log", c.version));
            }
            if c.content_hash.starts_with("sha256:") && !hashes.insert(c.content_hash.clone()) {
                err(report, "R1.4", format!("version {} reuses the content hash of another version", c.version));
            }
            if c.date.is_empty() || c.changes.is_empty() {
                err(report, "R1.6", format!("change-log entry {} lacks a date or changes", c.version));
            }
            for e in &c.changes {
                if e.subject.is_empty() || e.change.is_empty() || e.rationale.is_empty() || e.requirements.is_empty() {
                    err(report, "R1.6", format!("a change of version {} lacks subject, change, rationale, or requirements", c.version));
                }
            }
        }
    }

    // R57.1, R57.5
    if profile.core_subset.is_empty() {
        err(report, "R57.1", "the Core_Subset is empty".into());
    }
    let mut paths = BTreeSet::new();
    for item in &profile.core_subset {
        if !item.path.starts_with("core::") {
            err(report, "R57.1", format!("Core_Subset item `{}` is not a `core` path", item.path));
        }
        if !paths.insert(item.path.clone()) {
            err(report, "R57.1", format!("Core_Subset item `{}` is listed twice", item.path));
        }
        if item.scope.is_empty() {
            err(report, "R57.1", format!("Core_Subset item `{}` has no scope", item.path));
        }
        if item.certification.trim().is_empty() {
            err(report, "R57.5", format!("Core_Subset item `{}` records no certification evidence", item.path));
        }
        if item.certification.to_lowercase().contains("none") && item.substitute.is_none() {
            err(report, "R57.5", format!("Core_Subset item `{}` has no certification evidence and no substitute rsk evidence", item.path));
        }
    }
}

fn md_escape(text: &str) -> String {
    text.replace('|', "\\|").replace('\n', " ")
}

/// Renders `docs/profile.md`.
pub fn render_markdown(p: &Profile) -> String {
    let mut out = String::new();
    out.push_str("<!-- @generated by `cargo xtask profile` from profile/profile.toml. Do not edit. -->\n");
    out.push_str("# The rsk Profile\n\n");
    out.push_str(&format!(
        "**Version {}** (requirements Draft {}). Default Profile_Variant: {}. Variants: {}.\n\n",
        p.version,
        p.requirements_draft,
        p.default_variant,
        p.variants.join(", ")
    ));
    out.push_str("The Profile is the rsk counterpart of `pragma Profile (Ravenscar)`: a numbered list of restrictions, each with the guarantee it supports, the enforcement categories that establish it (TS type system, MC macro, LN lint, LK linker or binary check, PO proof obligation, RT run-time monitor; the primary category first), its Ada RM D.13 analogue, the subjects and crates it covers, and the Evidence_Items that demonstrate its enforcement. The verification job checks this document's source against Requirements 1, 2, 4, 27.3, and 57 and regenerates this file.\n\n");
    out.push_str(&format!(
        "Retired identifiers: {}. Reserved identifiers (inactive in v1, R27.3): {}.\n\n",
        if p.retired.is_empty() { "none".to_string() } else { p.retired.join(", ") },
        p.reserved.iter().map(|(id, purpose)| format!("{id} ({purpose})")).collect::<Vec<_>>().join("; ")
    ));

    out.push_str("## Table 2-1: Restrictions\n\n");
    out.push_str("| ID | Rule | Guarantee | Enforcement (primary first) | Ada D.13 analogue | Subjects | Crate coverage | RT assumption or reason | Evidence_Items |\n|---|---|---|---|---|---|---|---|---|\n");
    for r in &p.restrictions {
        let enforcement = r
            .enforcement
            .iter()
            .map(|(c, m)| format!("{c} ({m})"))
            .collect::<Vec<_>>()
            .join("; ");
        let analogue = match &r.analogue_note {
            Some(n) => format!("{} ({n})", r.analogue.join(", ")),
            None => r.analogue.join(", "),
        };
        let rt = match (&r.rt_assumption, &r.rt_reason) {
            (Some(a), Some(reason)) => format!("{a}: {reason}"),
            (Some(a), None) => a.clone(),
            (None, Some(reason)) => reason.clone(),
            (None, None) => "n/a".to_string(),
        };
        let evidence = r
            .evidence
            .iter()
            .map(|(c, items)| format!("{c}: {}", items.join(", ")))
            .collect::<Vec<_>>()
            .join("; ");
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} | {} | {} | {} |\n",
            r.id,
            md_escape(&r.rule),
            md_escape(&r.guarantee),
            md_escape(&enforcement),
            md_escape(&analogue),
            r.subjects.join(", "),
            md_escape(&r.crate_coverage),
            md_escape(&rt),
            md_escape(&evidence)
        ));
    }

    out.push_str("\n## Disallowed crates (PR-35, R1.5)\n\n| Crate | Reason |\n|---|---|\n");
    for (k, v) in &p.disallowed {
        out.push_str(&format!("| `{k}` | {} |\n", md_escape(v)));
    }

    out.push_str("\n## Table 4-1: Ada RM D.13 correspondence (Requirement 4)\n\n");
    out.push_str("Classification as defined in R4.1; Jorvik Profile_Variant markers as defined in R4.2.\n\n");
    out.push_str("| Ada RM D.13 item (Ravenscar) | Kind | Classification | rsk treatment | Cites | Jorvik Profile_Variant |\n|---|---|---|---|---|---|\n");
    for r in &p.d13 {
        let mut treatment = r.treatment.clone();
        if let Some(c) = &r.construct {
            treatment = format!("Construct not provided: {c}. {treatment}");
        }
        if let Some(d) = &r.difference {
            treatment.push_str(&format!(" Difference: {d}"));
        }
        if let Some(ra) = &r.rationale {
            treatment.push_str(&format!(" Rationale: {ra}"));
        }
        if !r.constrained_by.is_empty() {
            treatment.push_str(&format!(" Constrained by {}.", r.constrained_by.join(", ")));
        }
        if let Some(n) = &r.constraint_note {
            treatment.push_str(&format!(" {n}"));
        }
        let jorvik = if r.jorvik == "Same" {
            "Same".to_string()
        } else {
            format!(
                "{} ({}): {}",
                r.jorvik,
                r.jorvik_cites.join(", "),
                r.jorvik_rationale.clone().unwrap_or_default()
            )
        };
        out.push_str(&format!(
            "| {} | {} | {} | {} | {} | {} |\n",
            md_escape(&r.item),
            r.kind,
            r.classification,
            md_escape(&treatment),
            r.cites.join(", "),
            md_escape(&jorvik)
        ));
    }

    out.push_str("\n## Core_Subset (PR-21, Requirement 57)\n\n");
    out.push_str("The allow-list of `core` items that the Kernel and Applications may use, with the certification evidence that covers each item or the rsk evidence that substitutes for it (R57.5). Scope `kernel-unsafe` means the item is permitted only inside the Kernel_Unsafe_Module.\n\n");
    out.push_str("| Item | Scope | Certification evidence | Substitute rsk evidence | Note |\n|---|---|---|---|---|\n");
    for i in &p.core_subset {
        out.push_str(&format!(
            "| `{}` | {} | {} | {} | {} |\n",
            i.path,
            i.scope.join(", "),
            md_escape(&i.certification),
            md_escape(i.substitute.as_deref().unwrap_or("")),
            md_escape(i.note.as_deref().unwrap_or(""))
        ));
    }

    out.push_str("\n## Change log (R1.6)\n\n");
    for c in &p.changes {
        out.push_str(&format!("### Version {} ({})\n\nContent hash: `{}`\n\n", c.version, c.date, c.content_hash));
        for e in &c.changes {
            out.push_str(&format!(
                "- **{}**: {} Rationale: {} Affects: {}.\n",
                md_escape(&e.subject),
                md_escape(&e.change),
                md_escape(&e.rationale),
                e.requirements.join(", ")
            ));
        }
        out.push('\n');
    }
    out
}

/// Renders a `profile_version.rs` module.
pub fn render_version_rs(p: &Profile, crate_name: &str, hash: &str) -> String {
    let variants = p
        .variants
        .iter()
        .map(|v| format!("\"{v}\""))
        .collect::<Vec<_>>()
        .join(", ");
    format!(
        "// @generated by `cargo xtask profile` from profile/profile.toml. Do not edit.\n\
         //! The Profile version that `{crate_name}` implements (R1.4, R3.5, R6.7).\n\
         #![forbid(unsafe_code)]\n\n\
         /// The Profile version identifier (changes whenever the Profile changes, never reused).\n\
         pub const PROFILE_VERSION: &str = \"{}\";\n\
         /// The default Profile_Variant.\n\
         pub const PROFILE_DEFAULT_VARIANT: &str = \"{}\";\n\
         /// Every Profile_Variant the Profile defines.\n\
         pub const PROFILE_VARIANTS: &[&str] = &[{variants}];\n\
         /// SHA-256 of the Profile source at this version.\n\
         pub const PROFILE_CONTENT_HASH: &str = \"{hash}\";\n",
        p.version, p.default_variant
    )
}

/// Generates (or, with `check_only`, compares) every output. Returns whether
/// the Profile passed its checks and the outputs are current.
pub fn generate(root: &Path, check_only: bool, report: &mut Report) -> bool {
    let (profile, text) = match load(root) {
        Ok(v) => v,
        Err(e) => {
            report.error(Rule::Profile, Some(SOURCE.to_string()), e);
            return false;
        }
    };
    let hash = content_hash(&text);
    let text = if check_only {
        text
    } else {
        let recorded = with_recorded_hash(&text, &hash);
        if recorded != text {
            if let Err(e) = std::fs::write(root.join(SOURCE), &recorded) {
                report.error(Rule::Profile, Some(SOURCE.to_string()), format!("cannot record the content hash: {e}"));
                return false;
            }
            report.note(format!("recorded content hash {hash} for version {}", profile.version));
        }
        recorded
    };
    let (profile, _) = match load(root) {
        Ok(v) => v,
        Err(e) => {
            report.error(Rule::Profile, Some(SOURCE.to_string()), e);
            return false;
        }
    };
    check(&profile, &text, report);
    let mut outputs: Vec<(String, String)> = vec![(DOC.to_string(), render_markdown(&profile))];
    for (crate_name, path) in VERSION_FILES {
        outputs.push((path.to_string(), render_version_rs(&profile, crate_name, &hash)));
    }
    let mut current = true;
    for (path, content) in &outputs {
        let full = root.join(path);
        let existing = std::fs::read_to_string(&full).unwrap_or_default();
        if existing == *content {
            continue;
        }
        if check_only {
            current = false;
            report.error(
                Rule::Profile,
                Some(path.clone()),
                "is stale: run `cargo xtask profile` to regenerate it from profile/profile.toml",
            );
        } else {
            if let Some(dir) = full.parent() {
                let _ = std::fs::create_dir_all(dir);
            }
            match std::fs::write(&full, content) {
                Ok(()) => report.note(format!("wrote {path}")),
                Err(e) => report.error(Rule::Profile, Some(path.clone()), format!("cannot write: {e}")),
            }
        }
    }
    report.note(format!(
        "Profile {}: {} restrictions, {} Table 4-1 rows, {} disallowed crates, {} Core_Subset items, {} change-log entries; content hash {hash}",
        profile.version,
        profile.restrictions.len(),
        profile.d13.len(),
        profile.disallowed.len(),
        profile.core_subset.len(),
        profile.changes.len()
    ));
    current && !report.has_rule(Rule::Profile)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn content_hash_ignores_recorded_hashes() {
        let a = "x = 1\n[[change]]\ncontent_hash = \"\"\n";
        let b = "x = 1\n[[change]]\ncontent_hash = \"sha256:abc\"\n";
        assert_eq!(content_hash(a), content_hash(b));
        assert_ne!(content_hash(a), content_hash("x = 2\n[[change]]\ncontent_hash = \"\"\n"));
        let recorded = with_recorded_hash(a, "sha256:abc");
        assert_eq!(recorded, b);
    }
}
