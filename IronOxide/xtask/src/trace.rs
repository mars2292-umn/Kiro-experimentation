//! The traceability matrix (R50.5) and the report of acceptance criteria
//! without an Evidence_Item (R50.6), with the per-criterion list of R43.5
//! (task 8.4). `cargo xtask trace` writes `docs/traceability_matrix.md`
//! from:
//!
//! - every acceptance criterion `Rn.k` of requirements.md with the Phase of
//!   its requirement;
//! - the design sections and Properties of design.md that name it;
//! - the tasks of tasks.md that list it (`_Requirements:`), with their state;
//! - the rows of the Verification_Plan's property allocation table (section
//!   3) that cover it;
//! - the source files that cite it in a comment;
//! - the Evidence_Items of an evidence store whose `covers` names it, with
//!   their verdicts;
//! - the Profile's evidence names (`conf-PR-nn-cc`, `fi-PR-nn`, proof names)
//!   and whether the store holds them.
//!
//! The output carries no timestamp: identical inputs give an identical
//! matrix.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

pub const OUTPUT: &str = "docs/traceability_matrix.md";
const REQUIREMENTS: &str = ".kiro/specs/rsk-ravenscar-profile/requirements.md";
const DESIGN: &str = ".kiro/specs/rsk-ravenscar-profile/design.md";
const TASKS: &str = ".kiro/specs/rsk-ravenscar-profile/tasks.md";
const PLAN: &str = "docs/verification_plan.md";
const PROFILE: &str = "profile/profile.toml";

#[derive(Debug, Clone)]
pub struct Criterion {
    pub id: String,
    pub requirement: u32,
    pub title: String,
    pub phase: String,
}

#[derive(Debug, Default, Clone)]
pub struct Links {
    pub design: BTreeSet<String>,
    pub tasks: BTreeSet<(bool, String)>,
    pub properties: BTreeSet<String>,
    pub code: BTreeSet<String>,
    /// Evidence_Item id and verdict.
    pub evidence: BTreeSet<(String, String)>,
}

pub struct Matrix {
    pub criteria: Vec<Criterion>,
    pub links: BTreeMap<String, Links>,
    pub evidence_items: usize,
    /// Profile evidence names and whether the store has them.
    pub profile_evidence: Vec<(String, String, bool)>,
}

impl Matrix {
    pub fn without_evidence(&self) -> Vec<&Criterion> {
        self.criteria.iter().filter(|c| self.links.get(&c.id).is_none_or(|l| l.evidence.is_empty())).collect()
    }
    pub fn outside_allocation(&self) -> Vec<&Criterion> {
        self.criteria.iter().filter(|c| self.links.get(&c.id).is_none_or(|l| l.properties.is_empty())).collect()
    }
}

fn read(root: &Path, rel: &str) -> Result<String, String> {
    std::fs::read_to_string(root.join(rel)).map_err(|e| format!("{rel}: {e}"))
}

/// `Rn.k` identifiers in a text, with `Rn.a to Rn.b` ranges expanded.
pub fn criteria_in(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let b = text.as_bytes();
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'R' && i + 1 < b.len() && b[i + 1].is_ascii_digit() && (i == 0 || !b[i - 1].is_ascii_alphanumeric()) {
            let mut j = i + 1;
            while j < b.len() && b[j].is_ascii_digit() {
                j += 1;
            }
            if j < b.len() && b[j] == b'.' && j + 1 < b.len() && b[j + 1].is_ascii_digit() {
                let mut k = j + 1;
                while k < b.len() && b[k].is_ascii_digit() {
                    k += 1;
                }
                let req: u32 = text[i + 1..j].parse().unwrap_or(0);
                let from: u32 = text[j + 1..k].parse().unwrap_or(0);
                // A range `Rn.a to Rn.b`.
                let rest = &text[k..];
                let mut to = from;
                if let Some(r) = rest.strip_prefix(" to R") {
                    if let Some((rq, rk)) = r.split_once('.') {
                        let rk: String = rk.chars().take_while(|c| c.is_ascii_digit()).collect();
                        if rq.parse::<u32>().ok() == Some(req) {
                            if let Ok(t) = rk.parse::<u32>() {
                                to = t.max(from);
                            }
                        }
                    }
                }
                for n in from..=to {
                    out.insert(format!("R{req}.{n}"));
                }
                i = k;
                continue;
            }
        }
        i += 1;
    }
    out
}

/// `Requirements 7.1, 7.2, 20.3` lists of the design's Properties.
fn validates_in(text: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for part in text.split("Validates: Requirements ").skip(1) {
        let list = part.split("**").next().unwrap_or("");
        for item in list.split(',') {
            let item = item.trim().trim_end_matches('.');
            if let Some((a, b)) = item.split_once('.') {
                if a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()) && !a.is_empty() && !b.is_empty() {
                    out.insert(format!("R{a}.{b}"));
                }
            }
        }
    }
    out
}

/// The criteria the Verification_Plan's section 9.5 lists as completely
/// covered by the named formal analyses (the `Complete` column of the rows
/// `| Verus, Kernel |` and `| Verus, Analyzer core |`), for the Verus
/// Evidence_Item's `covers` list (R52.5): the plan is the single source of
/// what the proofs are claimed to discharge.
pub fn plan_complete_criteria(plan_text: &str, analyses: &[&str]) -> Vec<String> {
    let mut out = BTreeSet::new();
    for line in plan_text.lines() {
        let cells: Vec<&str> = line.split('|').collect();
        if cells.len() < 4 {
            continue;
        }
        if analyses.contains(&cells[1].trim()) {
            out.extend(criteria_in(cells[2]));
        }
    }
    out.into_iter().collect()
}

/// The acceptance criteria and restrictions that the functions carrying
/// `attribute` (`#[test]`, `#[kani::proof]`) name in their doc comments
/// (`R12.2`) or in their names (`r28_5_...`, `..._pr20_r6_3`), across the
/// workspace sources outside `vendor` and `target`. A passing test or
/// harness that names a criterion is evidence for it (task 8.4 follow-up,
/// 2026-10-23); the Evidence_Item's `covers` list is derived from this.
pub fn named_by_tests(root: &Path, attribute: &str) -> (Vec<String>, usize) {
    let mut files = Vec::new();
    for d in ["crates", "xtask", "apps"] {
        collect(&root.join(d), &mut files);
    }
    files.sort();
    let mut out = BTreeSet::new();
    let mut functions = 0;
    for f in files {
        if f.extension().is_some_and(|x| x != "rs") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let lines: Vec<&str> = text.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            if line.trim() != attribute {
                continue;
            }
            // Doc comment lines above, skipping other attributes.
            let mut found = BTreeSet::new();
            let mut j = i;
            while j > 0 {
                j -= 1;
                let t = lines[j].trim();
                if t.starts_with("///") {
                    found.extend(criteria_in(t));
                    for pr in prs_in(t) {
                        found.insert(pr);
                    }
                } else if t.starts_with("#[") || t.is_empty() {
                    if t.is_empty() {
                        break;
                    }
                } else {
                    break;
                }
            }
            // The function name below, skipping other attributes.
            for k in i + 1..lines.len().min(i + 6) {
                let t = lines[k].trim();
                if let Some(rest) = t.strip_prefix("fn ").or_else(|| t.strip_prefix("pub fn ")) {
                    let name: String = rest.chars().take_while(|c| c.is_alphanumeric() || *c == '_').collect();
                    found.extend(criteria_in_name(&name));
                    break;
                }
            }
            if !found.is_empty() {
                functions += 1;
                out.extend(found);
            }
        }
    }
    (out.into_iter().collect(), functions)
}

/// `PR-nn` identifiers in a text.
fn prs_in(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    for (i, _) in text.match_indices("PR-") {
        let digits: String = text[i + 3..].chars().take_while(|c| c.is_ascii_digit()).collect();
        if digits.len() == 2 {
            out.push(format!("PR-{digits}"));
        }
    }
    out
}

/// `r28_5` -> R28.5 and `pr20` -> PR-20 in a snake-case function name.
pub fn criteria_in_name(name: &str) -> Vec<String> {
    let parts: Vec<&str> = name.split('_').collect();
    let mut out = Vec::new();
    let mut i = 0;
    while i < parts.len() {
        let p = parts[i];
        if let Some(n) = p.strip_prefix("pr") {
            if n.len() == 2 && n.chars().all(|c| c.is_ascii_digit()) {
                out.push(format!("PR-{n}"));
            }
        } else if let Some(n) = p.strip_prefix('r') {
            if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
                if let Some(k) = parts.get(i + 1) {
                    if !k.is_empty() && k.chars().all(|c| c.is_ascii_digit()) {
                        out.push(format!("R{n}.{k}"));
                        i += 1;
                    }
                }
            }
        }
        i += 1;
    }
    out
}

pub fn parse_requirements(text: &str) -> Vec<Criterion> {
    let mut out = Vec::new();
    let mut current: Option<(u32, String, String)> = None;
    let mut in_criteria = false;
    for line in text.lines() {
        if let Some(rest) = line.strip_prefix("### Requirement ") {
            let (num, title) = rest.split_once(':').unwrap_or((rest, ""));
            let title = title.trim();
            let (title, phase) = match title.rfind("(Phase ") {
                Some(k) => (title[..k].trim().to_string(), title[k + 7..].trim_end_matches(')').to_string()),
                None => (title.to_string(), "unstated".to_string()),
            };
            current = num.trim().parse().ok().map(|n| (n, title, phase));
            in_criteria = false;
            continue;
        }
        if line.starts_with("### ") || line.starts_with("## ") {
            current = None;
            in_criteria = false;
            continue;
        }
        if line.starts_with("#### Acceptance Criteria") {
            in_criteria = true;
            continue;
        }
        if !in_criteria {
            continue;
        }
        let Some((req, title, phase)) = &current else { continue };
        let Some((n, _)) = line.split_once(". ") else { continue };
        if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) {
            out.push(Criterion { id: format!("R{req}.{n}"), requirement: *req, title: title.clone(), phase: phase.clone() });
        }
    }
    out
}

fn design_links(text: &str, links: &mut BTreeMap<String, Links>) {
    let mut section = String::from("(preamble)");
    let mut property: Option<String> = None;
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("### ").or_else(|| line.strip_prefix("## ")) {
            section = h.trim().to_string();
            property = if h.starts_with("Property ") { Some(h.split(':').next().unwrap_or(h).trim().to_string()) } else { None };
        }
        let mut ids = criteria_in(line);
        if line.contains("Validates: Requirements") {
            ids.extend(validates_in(line));
        }
        for id in ids {
            let l = links.entry(id).or_default();
            l.design.insert(property.clone().unwrap_or_else(|| section.clone()));
        }
    }
}

fn task_links(text: &str, links: &mut BTreeMap<String, Links>) {
    let mut task: Option<(bool, String)> = None;
    for line in text.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("- [x] ") {
            task = Some((true, rest.split_whitespace().next().unwrap_or("").trim_end_matches('.').to_string()));
        } else if let Some(rest) = t.strip_prefix("- [ ] ") {
            task = Some((false, rest.split_whitespace().next().unwrap_or("").trim_end_matches('.').to_string()));
        } else if let Some(list) = t.strip_prefix("- _Requirements: ") {
            if let Some((done, id)) = &task {
                for item in list.trim_end_matches('_').split(',') {
                    let item = item.trim();
                    if let Some((a, b)) = item.split_once('.') {
                        if a.chars().all(|c| c.is_ascii_digit()) && b.chars().all(|c| c.is_ascii_digit()) {
                            links.entry(format!("R{a}.{b}")).or_default().tasks.insert((*done, id.clone()));
                        }
                    }
                }
            }
        }
    }
}

fn plan_links(text: &str, links: &mut BTreeMap<String, Links>) {
    for line in text.lines() {
        if !line.starts_with("| P-") {
            continue;
        }
        let cells: Vec<&str> = line.split('|').collect();
        if cells.len() < 4 {
            continue;
        }
        let p = cells[1].trim().to_string();
        for id in criteria_in(cells[3]) {
            links.entry(id).or_default().properties.insert(p.clone());
        }
    }
}

fn code_links(root: &Path, links: &mut BTreeMap<String, Links>) {
    let mut files = Vec::new();
    for d in ["crates", "xtask/src", "apps", "profile"] {
        collect(&root.join(d), &mut files);
    }
    files.sort();
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let rel = f.strip_prefix(root).unwrap_or(&f).display().to_string();
        for id in criteria_in(&text) {
            links.entry(id).or_default().code.insert(rel.clone());
        }
    }
}

fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if p.is_dir() {
            if name != "target" && name != "vendor" && !name.starts_with('.') {
                collect(&p, out);
            }
        } else if matches!(p.extension().and_then(|x| x.to_str()), Some("rs") | Some("toml") | Some("x")) {
            out.push(p);
        }
    }
}

fn evidence_links(dir: &Path, links: &mut BTreeMap<String, Links>) -> (usize, BTreeSet<String>) {
    let mut ids = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir) else { return (0, ids) };
    let mut files: Vec<PathBuf> = entries.flatten().map(|e| e.path()).filter(|p| p.extension().is_some_and(|x| x == "json")).collect();
    files.sort();
    let mut n = 0;
    for f in files {
        let Ok(text) = std::fs::read_to_string(&f) else { continue };
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        if v.get("schema").and_then(|s| s.as_str()) != Some("rsk-evidence/1") {
            continue;
        }
        n += 1;
        let id = v.get("id").and_then(|s| s.as_str()).unwrap_or("").to_string();
        let verdict = v.get("verdict").and_then(|s| s.as_str()).unwrap_or("").to_string();
        ids.insert(id.clone());
        for c in v.get("covers").and_then(|c| c.as_array()).into_iter().flatten() {
            if let Some(c) = c.as_str() {
                links.entry(c.to_string()).or_default().evidence.insert((id.clone(), verdict.clone()));
            }
        }
    }
    (n, ids)
}

/// The Profile's evidence names per restriction and category.
fn profile_evidence(text: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(doc) = text.parse::<toml::Table>() else { return out };
    for r in doc.get("restriction").and_then(|v| v.as_array()).into_iter().flatten() {
        let id = r.get("id").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if let Some(ev) = r.get("evidence").and_then(|v| v.as_table()) {
            for (_, names) in ev {
                for n in names.as_array().into_iter().flatten() {
                    if let Some(n) = n.as_str() {
                        out.push((id.clone(), n.to_string()));
                    }
                }
            }
        }
    }
    out
}

pub fn build(root: &Path, evidence_dir: &Path) -> Result<Matrix, String> {
    let criteria = parse_requirements(&read(root, REQUIREMENTS)?);
    let mut links: BTreeMap<String, Links> = BTreeMap::new();
    design_links(&read(root, DESIGN)?, &mut links);
    task_links(&read(root, TASKS)?, &mut links);
    plan_links(&read(root, PLAN)?, &mut links);
    code_links(root, &mut links);
    let (evidence_items, item_ids) = evidence_links(evidence_dir, &mut links);
    // The Link_Checker and Profile_Lint logs stand for the Profile's LK and
    // LN evidence names (verification_plan.md section 1).
    let profile_evidence = profile_evidence(&read(root, PROFILE)?)
        .into_iter()
        .map(|(pr, name)| {
            let present = item_ids.contains(&name)
                || (name.ends_with("-lk") && item_ids.iter().any(|i| i.starts_with("analysis-stack-")))
                || (name.ends_with("-ln") && item_ids.contains("check-profile-lint"))
                || (name.starts_with("proof-kernel-") && item_ids.contains("proof-kernel-verus"));
            (pr, name, present)
        })
        .collect();
    Ok(Matrix { criteria, links, evidence_items, profile_evidence })
}

fn short(set: &BTreeSet<String>, max: usize) -> String {
    let v: Vec<&String> = set.iter().collect();
    if v.is_empty() {
        return "—".into();
    }
    let mut s: Vec<String> = v.iter().take(max).map(|x| x.to_string()).collect();
    if v.len() > max {
        s.push(format!("+{}", v.len() - max));
    }
    s.join("; ")
}

pub fn render(m: &Matrix, evidence_dir_name: &str) -> String {
    let mut s = String::new();
    s.push_str("<!-- @generated by `cargo xtask trace` from requirements.md, design.md, tasks.md, docs/verification_plan.md, the sources, and the evidence store. Do not edit. -->\n");
    s.push_str("# Traceability matrix (R50.5, R50.6, R43.5)\n\n");
    let with_ev = m.criteria.len() - m.without_evidence().len();
    let in_alloc = m.criteria.len() - m.outside_allocation().len();
    s.push_str(&format!(
        "{} acceptance criteria in {} requirements; {} with at least one Evidence_Item, {} without; {} covered by a row of the property allocation table, {} outside it. Evidence store: `{evidence_dir_name}` ({} Evidence_Items).\n\n",
        m.criteria.len(),
        m.criteria.iter().map(|c| c.requirement).collect::<BTreeSet<_>>().len(),
        with_ev,
        m.without_evidence().len(),
        in_alloc,
        m.outside_allocation().len(),
        m.evidence_items
    ));
    // Per Phase.
    s.push_str("## Coverage per Phase\n\n| Phase | Criteria | With Evidence_Item | Without | In the allocation table |\n|---|---|---|---|---|\n");
    let mut phases: BTreeMap<&str, (usize, usize, usize)> = BTreeMap::new();
    for c in &m.criteria {
        let e = phases.entry(c.phase.as_str()).or_default();
        e.0 += 1;
        let l = m.links.get(&c.id);
        if l.is_some_and(|l| !l.evidence.is_empty()) {
            e.1 += 1;
        }
        if l.is_some_and(|l| !l.properties.is_empty()) {
            e.2 += 1;
        }
    }
    for (p, (n, e, a)) in &phases {
        s.push_str(&format!("| {p} | {n} | {e} | {} | {a} |\n", n - e));
    }
    // The matrix.
    s.push_str("\n## Matrix\n\nOne row per criterion: the Phase of its requirement; the rows of the property allocation table (verification_plan.md section 3) that cover it; the design sections or Properties that cite it; the tasks that list it (✓ done, ○ open); the source files that cite it; the Evidence_Items whose `covers` names it, with their verdicts.\n");
    let mut by_req: BTreeMap<u32, Vec<&Criterion>> = BTreeMap::new();
    for c in &m.criteria {
        by_req.entry(c.requirement).or_default().push(c);
    }
    for (req, cs) in &by_req {
        let first = cs[0];
        s.push_str(&format!("\n### Requirement {req}: {} (Phase {})\n\n| Criterion | Allocation | Design | Tasks | Code | Evidence_Items |\n|---|---|---|---|---|---|\n", first.title, first.phase));
        for c in cs {
            let l = m.links.get(&c.id).cloned().unwrap_or_default();
            let tasks: BTreeSet<String> = l.tasks.iter().map(|(d, t)| format!("{}{t}", if *d { "✓" } else { "○" })).collect();
            let ev: BTreeSet<String> = l.evidence.iter().map(|(i, v)| if v == "pass" { i.clone() } else { format!("{i} ({v})") }).collect();
            s.push_str(&format!("| {} | {} | {} | {} | {} | {} |\n", c.id, short(&l.properties, 4), short(&l.design, 3), short(&tasks, 4), short(&l.code, 3), short(&ev, 5)));
        }
    }
    // R50.6.
    s.push_str("\n## Criteria without an Evidence_Item (R50.6)\n\nGrouped by Phase. A criterion of a later Phase awaits that Phase; a criterion of Phase P1 or P2 names the tasks that will produce its evidence (○ open, ✓ done but producing no item, which is the document-and-review kind of evidence or a check folded into another item).\n");
    let mut missing: BTreeMap<&str, Vec<&Criterion>> = BTreeMap::new();
    for c in m.without_evidence() {
        missing.entry(c.phase.as_str()).or_default().push(c);
    }
    for (p, cs) in &missing {
        s.push_str(&format!("\n### Phase {p}: {} criteria\n\n| Criterion | Tasks | Design |\n|---|---|---|\n", cs.len()));
        for c in cs {
            let l = m.links.get(&c.id).cloned().unwrap_or_default();
            let tasks: BTreeSet<String> = l.tasks.iter().map(|(d, t)| format!("{}{t}", if *d { "✓" } else { "○" })).collect();
            s.push_str(&format!("| {} | {} | {} |\n", c.id, short(&tasks, 4), short(&l.design, 2)));
        }
    }
    // R43.5.
    s.push_str("\n## Criteria outside the property allocation table (R43.5)\n\nEach criterion that no row of verification_plan.md section 3 covers, with the Phase in which its evidence is due and the tasks that list it; a criterion with an Evidence_Item but no row is verified by a check, a test, or a document rather than by a property (section 12 of the plan gives the groups).\n\n| Criterion | Phase | Tasks | Evidence_Items |\n|---|---|---|---|\n");
    for c in m.outside_allocation() {
        let l = m.links.get(&c.id).cloned().unwrap_or_default();
        let tasks: BTreeSet<String> = l.tasks.iter().map(|(d, t)| format!("{}{t}", if *d { "✓" } else { "○" })).collect();
        let ev: BTreeSet<String> = l.evidence.iter().map(|(i, _)| i.clone()).collect();
        s.push_str(&format!("| {} | {} | {} | {} |\n", c.id, c.phase, short(&tasks, 3), short(&ev, 3)));
    }
    // Profile evidence names.
    s.push_str("\n## Profile evidence names\n\nThe Evidence_Item names the Profile's restrictions promise (profile.toml `evidence`), and whether this evidence store holds them. LK names stand for the Link_Checker's item (`analysis-stack-<binary>`), LN names for `check-profile-lint`, and `proof-kernel-*` names for `proof-kernel-verus` (verification_plan.md section 1).\n\n| Restriction | Evidence name | Present |\n|---|---|---|\n");
    for (pr, name, present) in &m.profile_evidence {
        s.push_str(&format!("| {pr} | `{name}` | {} |\n", if *present { "yes" } else { "no" }));
    }
    s
}

/// Writes the matrix to `docs/traceability_matrix.md` (or checks it is
/// current) and returns the counts (criteria, without evidence).
pub fn run(root: &Path, evidence_dir: &Path, check_only: bool, report: &mut crate::diag::Report) -> Result<(usize, usize), String> {
    let m = build(root, evidence_dir)?;
    let name = evidence_dir.strip_prefix(root).unwrap_or(evidence_dir).display().to_string();
    let text = render(&m, &name);
    let path = root.join(OUTPUT);
    if check_only {
        let current = std::fs::read_to_string(&path).unwrap_or_default();
        if current != text {
            report.error(crate::diag::Rule::Evidence, Some(OUTPUT.into()), "the traceability matrix differs from the sources and the evidence store; run `cargo xtask trace`");
        }
    } else {
        std::fs::write(&path, &text).map_err(|e| format!("{OUTPUT}: {e}"))?;
        report.note(format!("wrote {OUTPUT}"));
    }
    let missing = m.without_evidence().len();
    report.note(format!(
        "R50.5/R50.6: {} criteria, {} with an Evidence_Item, {} without ({} of Phase P1 or P2), {} outside the allocation table (R43.5); {} of {} Profile evidence names present",
        m.criteria.len(),
        m.criteria.len() - missing,
        missing,
        m.without_evidence().iter().filter(|c| c.phase.starts_with("P1") || c.phase.starts_with("P2")).count(),
        m.outside_allocation().len(),
        m.profile_evidence.iter().filter(|(_, _, p)| *p).count(),
        m.profile_evidence.len()
    ));
    Ok((m.criteria.len(), missing))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn criteria_and_ranges_are_parsed() {
        let ids = criteria_in("covers R16.1 to R16.3, R8.4 and PR-05 (R29.2); not RX or R1.");
        assert_eq!(ids.into_iter().collect::<Vec<_>>(), vec!["R16.1", "R16.2", "R16.3", "R29.2", "R8.4"]);
        let v = validates_in("**Validates: Requirements 29.1, 29.6, 29.7, 32.1** · Evidence: Verus");
        assert_eq!(v.len(), 4);
        let reqs = parse_requirements("### Requirement 3: Enforcement (Phase P2)\n\ntext\n\n#### Acceptance Criteria\n\n1. THE a.\n2. IF b.\n\n### Requirement 4: Other (Phase P1 plan)\n#### Acceptance Criteria\n1. x\n");
        assert_eq!(reqs.iter().map(|c| (c.id.as_str(), c.phase.as_str())).collect::<Vec<_>>(), vec![("R3.1", "P2"), ("R3.2", "P2"), ("R4.1", "P1 plan")]);
    }

    /// R50.5: the matrix links criteria to the tests and harnesses that name them, by function name and by doc comment.
    #[test]
    fn criteria_named_by_tests_are_found_in_names_and_doc_comments() {
        assert_eq!(criteria_in_name("kernel_violations_are_rejected_naming_r6_1_and_pr20_r6_3"), vec!["R6.1", "PR-20", "R6.3"]);
        assert_eq!(criteria_in_name("r28_5_a_task_on_another_core_is_rejected_citing_ng_01"), vec!["R28.5"]);
        assert!(criteria_in_name("r32_1_fixpoint_agrees").contains(&"R32.1".to_string()));
        assert!(criteria_in_name("rta_r_and_x").is_empty());
        let root = crate::repository_root();
        let (tests, functions) = named_by_tests(&root, "#[test]");
        assert!(functions > 40 && tests.contains(&"R28.5".to_string()) && tests.contains(&"R12.1".to_string()), "{functions} functions: {tests:?}");
        let (harnesses, _) = named_by_tests(&root, "#[kani::proof]");
        assert!(!harnesses.is_empty(), "{harnesses:?}");
    }

    /// R50.5: the repository's matrix has a row for every acceptance criterion of requirements.md, and every criterion a task lists exists there.
    #[test]
    fn the_repository_matrix_covers_every_criterion_of_the_requirements() {
        let root = crate::repository_root();
        let m = build(&root, &root.join("target/xtask-verify/evidence")).unwrap();
        assert!(m.criteria.len() > 350, "{}", m.criteria.len());
        // Every criterion a task lists exists in requirements.md.
        let ids: BTreeSet<&str> = m.criteria.iter().map(|c| c.id.as_str()).collect();
        let dangling: Vec<&String> = m.links.iter().filter(|(id, l)| !l.tasks.is_empty() && id.starts_with('R') && !ids.contains(id.as_str())).map(|(id, _)| id).collect();
        assert!(dangling.is_empty(), "tasks.md lists criteria that requirements.md does not define: {dangling:?}");
    }
}
