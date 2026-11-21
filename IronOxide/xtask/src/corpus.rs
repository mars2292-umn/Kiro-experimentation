//! The UPPAAL cross-check corpus (R32.6, PAR-03; task 16.2): at least 50
//! Task_Models of up to 8 Tasks each, generated deterministically from
//! synthetic App_Declarations by the Generator itself (so every model is
//! one the Generator accepts, with its derived values), analysed by the
//! Analyzer with placeholder timing (every WCET is the Budget) and, where
//! UPPAAL's `verifyta` is installed, cross-checked against the exported
//! timed automata (R31.4 to R31.6, R31.9). The outcome is one report,
//! `docs/uppaal_corpus_report.md` (`cargo xtask uppaal-corpus`), that lists
//! every model with its RTA verdict and cross-check status, and every
//! discrepancy with its disposition from `docs/uppaal_dispositions.toml`
//! (R32.6: the Verification_Plan records each discrepancy and its
//! disposition; the plan's section 8 points here).
//!
//! Since 2026-11-21 the verification job does not run the corpus (it took
//! about 14 minutes of every job, needs a licensed external tool whose
//! lease is renewed over the network the job's sandbox denies, and is a
//! sanity check beside the Analyzer's proofs, not the evidence for it). The
//! job's step "UPPAAL cross-check corpus" reads the committed report and
//! writes the Evidence_Item `check-uppaal-corpus` from it: `pass` when
//! every cross-check agrees or is pessimism, `fail` on a discrepancy
//! without a disposition, `inconclusive` when the verifier was not
//! installed, a run hit its limits, or the report's `Inputs` digest
//! ([`fingerprint`]) is not that of the tree, which means the sources the
//! corpus depends on changed after the report was written: run
//! `cargo xtask uppaal-corpus` to bring it up to date.
//!
//! The corpus is a seeded generator (no property-testing crate is vendored,
//! R58.1): the same 50 declarations on every run. Each has two Partitions
//! (levels A and C), 2 to 8 Tasks with Priorities 1 to 7 (ties broken by
//! R7.3 when 8 Tasks share 7 levels), periods from 5 ms to 200 ms,
//! deadlines equal to or below the period, Budgets chosen so that the
//! Budget utilization ranges from 20 % to 85 % (with the placeholder
//! Critical_Section WCETs, the analysed utilization from about 40 % to
//! above 100 %, so both PASS and FAIL verdicts occur), up to two Resources per Partition with random
//! accessor sets (SRP blocking), and about a third of the Tasks sporadic,
//! released by a Release_Signal that a periodic Task of the same Partition
//! raises (MIT-constrained generators in the export).

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::manifest::Manifest;
use crate::sandbox::Sandbox;
use crate::workspace::Workspace;

/// PAR-03: at least 50 models of up to 8 Tasks.
pub const MODELS: usize = 50;
/// PAR-03 proposes 8; the Target tables (nrf52840, qemu_mps2_an386) offer 7
/// free dispatch vectors, so 7 is the Generator's limit (DD-01).
pub const MAX_TASKS: usize = 7;
/// The verifier's limits per model (R31.9).
pub const VERIFIER_TIMEOUT_SECS: u64 = 120;
pub const VERIFIER_MEMORY_MIB: u64 = 4096;

/// A small deterministic generator (LCG), so the corpus is the same on every run.
struct Lcg(u64);

impl Lcg {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn pick<'a, T>(&mut self, items: &'a [T]) -> &'a T {
        &items[self.below(items.len() as u64) as usize]
    }
}

/// One generated model's declaration and the facts the report prints.
pub struct Generated {
    pub id: String,
    pub declaration: String,
    pub tasks: usize,
    pub sporadic: usize,
    pub resources: usize,
}

/// The `i`-th declaration of the corpus.
pub fn declaration(i: usize) -> Generated {
    let mut r = Lcg(0x5eed_0000 + i as u64 * 7919);
    let n = 2 + r.below(MAX_TASKS as u64 - 1) as usize;
    let periods_ms: [u64; 8] = [5, 10, 20, 25, 40, 50, 100, 200];
    // Target Budget utilization in percent, spread so that both verdicts
    // occur: the placeholder timing charges each Critical_Section site a
    // Task's whole Budget, so a Task with Resources counts about twice.
    let util = [20u64, 30, 40, 50, 60, 70, 85][r.below(7) as usize];
    struct T {
        name: String,
        partition: usize,
        priority: u64,
        period: u64,
        deadline: u64,
        budget_us: u64,
        sporadic: bool,
        resources: Vec<String>,
    }
    let mut tasks: Vec<T> = Vec::new();
    for k in 0..n {
        let partition = if k == 0 { 0 } else if k == 1 { 1 } else { r.below(2) as usize };
        let period = *r.pick(&periods_ms);
        let deadline = if r.below(3) == 0 && period % 5 == 0 { period * 4 / 5 } else { period };
        // Budget: the Task's share of the target utilization, at least 100 us
        // and below the deadline.
        let budget_us = (period * 1000 * util / 100 / n as u64).clamp(100, deadline * 1000 - 100);
        // The first Task of each Partition is periodic (it raises the signals).
        let sporadic = k >= 2 && r.below(3) == 0;
        tasks.push(T { name: format!("t{k}"), partition, priority: 1 + r.below(7), period, deadline, budget_us, sporadic, resources: Vec::new() });
    }
    // Resources per Partition, each with at least one accessor.
    let mut resources: Vec<(usize, String)> = Vec::new();
    for p in 0..2 {
        let members: Vec<usize> = (0..n).filter(|&k| tasks[k].partition == p).collect();
        if members.is_empty() {
            continue;
        }
        for q in 0..r.below(3) as usize {
            let name = format!("r{p}{q}");
            let mut any = false;
            for &k in &members {
                if r.below(2) == 0 {
                    tasks[k].resources.push(name.clone());
                    any = true;
                }
            }
            if !any {
                tasks[members[0]].resources.push(name.clone());
            }
            resources.push((p, name));
        }
    }
    let mut s = String::new();
    s.push_str("target = nrf52840, profile = Ravenscar, operating_duration = 10.s(),\n");
    s.push_str("safe_state = crate::report_and_exit, log_capacity = 32;\n");
    let levels = ["A", "C"];
    for p in 0..2 {
        s.push_str(&format!(
            "partition part{p} (level = {}, crate = corpus_part{p}, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),\n                 fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {{\n    peripherals = [];\n",
            levels[p]
        ));
        for (_, name) in resources.iter().filter(|(q, _)| *q == p) {
            s.push_str(&format!("    resource {name}: u32 = 0;\n"));
        }
        let members: Vec<usize> = (0..n).filter(|&k| tasks[k].partition == p).collect();
        let sporadics: Vec<usize> = members.iter().copied().filter(|&k| tasks[k].sporadic).collect();
        for &k in &sporadics {
            s.push_str(&format!("    signal sig{k};\n"));
        }
        let raiser = members.iter().copied().find(|&k| !tasks[k].sporadic);
        for &k in &members {
            let t = &tasks[k];
            let res = if t.resources.is_empty() { String::new() } else { format!(", resources = [{}]", t.resources.join(", ")) };
            if t.sporadic {
                s.push_str(&format!(
                    "    task {} (sporadic, signal = sig{k}, mit = {}.ms(), deadline = {}.ms(), budget = {}.us(), priority = {}{res});\n",
                    t.name, t.period, t.deadline, t.budget_us, t.priority
                ));
            } else {
                let raises = if Some(k) == raiser && !sporadics.is_empty() {
                    format!(", raises = [{}]", sporadics.iter().map(|k| format!("sig{k}")).collect::<Vec<_>>().join(", "))
                } else {
                    String::new()
                };
                s.push_str(&format!(
                    "    task {} (periodic, period = {}.ms(), deadline = {}.ms(), budget = {}.us(), priority = {}{res}{raises});\n",
                    t.name, t.period, t.deadline, t.budget_us, t.priority
                ));
            }
        }
        s.push_str("}\n");
    }
    Generated {
        id: format!("m{i:02}"),
        declaration: s,
        tasks: n,
        sporadic: tasks.iter().filter(|t| t.sporadic).count(),
        resources: resources.len(),
    }
}

/// The Task_Model JSON of a generated declaration, through the Generator.
pub fn task_model(g: &Generated) -> Result<String, String> {
    let source = format!("#![no_std]\nrsk::app! {{\n{}}}\nfn report_and_exit() -> ! {{ loop {{}} }}\n", g.declaration);
    let parsed = rsk_gen::syntax::parse_source(&source)?;
    let outputs = rsk_gen::outputs(&parsed.decl, "rsk-system").map_err(|errs| errs.iter().map(ToString::to_string).collect::<Vec<_>>().join("; "))?;
    Ok(outputs.task_model_json)
}

/// One model's row of the report.
#[derive(Clone, Debug)]
pub struct Row {
    pub id: String,
    pub tasks: usize,
    pub sporadic: usize,
    pub resources: usize,
    pub utilization: String,
    pub rta: String,
    pub status: String,
    pub detail: String,
}

/// The outcome of a corpus run.
pub struct Outcome {
    pub rows: Vec<Row>,
    pub by_status: BTreeMap<String, usize>,
    pub verifier: Option<String>,
    pub report: String,
    /// Discrepancies without a disposition.
    pub undisposed: Vec<String>,
}

/// Dispositions of recorded discrepancies: `[[discrepancy]] model = "m07"
/// disposition = "..."` in `docs/uppaal_dispositions.toml`.
fn dispositions(root: &Path) -> BTreeMap<String, String> {
    let mut out = BTreeMap::new();
    let Ok(text) = std::fs::read_to_string(root.join("docs").join("uppaal_dispositions.toml")) else { return out };
    let Ok(table) = text.parse::<toml::Table>() else { return out };
    for d in table.get("discrepancy").and_then(toml::Value::as_array).into_iter().flatten() {
        if let (Some(m), Some(disp)) = (d.get("model").and_then(toml::Value::as_str), d.get("disposition").and_then(toml::Value::as_str)) {
            out.insert(m.to_string(), disp.to_string());
        }
    }
    out
}

/// Runs the corpus: generates the models into `work`, analyses each with
/// the Analyzer (built from this workspace), and renders the report.
pub fn run(ws: &Workspace, target_dir: &Path, work: &Path, sandbox: Option<&Sandbox>, report: &mut Report) -> Result<Outcome, String> {
    std::fs::create_dir_all(work).map_err(|e| format!("cannot create {}: {e}", work.display()))?;
    // A Build_Manifest for the corpus: this revision and toolchain. Each
    // model gets its own copy naming a synthetic binary whose
    // Generated_Config checksum is the model's, which the Analyzer's
    // cross-checks (R28.3, R28.8) require; no such binary exists, which is
    // why the placeholder timing is no evidence.
    let manifest = Manifest::collect(ws, None, None, None)?;
    let manifest_json = manifest.to_json();
    let dispositions = dispositions(&ws.root);
    let mut rows = Vec::new();
    let mut by_status: BTreeMap<String, usize> = BTreeMap::new();
    let mut verifier: Option<String> = None;
    let mut undisposed = Vec::new();
    for i in 0..MODELS {
        let g = declaration(i);
        let dir = work.join(&g.id);
        std::fs::create_dir_all(&dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        std::fs::write(dir.join("declaration.rs"), &g.declaration).map_err(|e| e.to_string())?;
        let model = match task_model(&g) {
            Ok(m) => m,
            Err(e) => {
                report.error(Rule::Step, None, format!("UPPAAL corpus: the Generator rejected model {}: {e}", g.id));
                rows.push(Row { id: g.id.clone(), tasks: g.tasks, sporadic: g.sporadic, resources: g.resources, utilization: "—".into(), rta: "generator rejected".into(), status: "not_run".into(), detail: e });
                *by_status.entry("not_run".into()).or_default() += 1;
                continue;
            }
        };
        let model_path = dir.join("task_model.json");
        std::fs::write(&model_path, &model).map_err(|e| e.to_string())?;
        let checksum: serde_json::Value = serde_json::from_str::<serde_json::Value>(&model).ok().and_then(|m| m.get("config_checksum").cloned()).unwrap_or(serde_json::Value::Null);
        let mut per_model = manifest_json.clone();
        per_model["generated_config_checksum"] = checksum.clone();
        per_model["binaries"] = serde_json::json!([{
            "package": "uppaal-corpus", "target": g.id, "file": format!("{}.elf", g.id), "hash": format!("sha256:corpus-{}", g.id), "loadable_bytes": 0, "config_checksum": checksum
        }]);
        let manifest_path = dir.join("build-manifest.json");
        std::fs::write(&manifest_path, serde_json::to_string_pretty(&per_model).map_err(|e| e.to_string())? + "\n").map_err(|e| e.to_string())?;
        let cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
            .arg(ws.root.join("Cargo.toml"))
            .arg("--target-dir")
            .arg(target_dir)
            .arg("--")
            .arg("--model")
            .arg(&model_path)
            .arg("--manifest")
            .arg(&manifest_path)
            .arg("--placeholders")
            .arg("--out")
            .arg(&dir)
            .arg("--uppaal")
            .args(["--uppaal-timeout", &VERIFIER_TIMEOUT_SECS.to_string(), "--uppaal-memory", &VERIFIER_MEMORY_MIB.to_string()]);
        let output = cmd.output(sandbox).map_err(|e| format!("cannot run rsk-analyzer: {e}"))?;
        let text = format!("{}{}", String::from_utf8_lossy(&output.stdout), String::from_utf8_lossy(&output.stderr));
        let _ = std::fs::write(dir.join("analyzer.log"), &text);
        let json: serde_json::Value = std::fs::read_to_string(dir.join("report.json"))
            .ok()
            .and_then(|t| serde_json::from_str(&t).ok())
            .ok_or_else(|| format!("UPPAAL corpus: model {} produced no report (exit {:?}); see {}", g.id, output.status.code(), dir.join("analyzer.log").display()))?;
        let rta = json.get("result").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let status = json.pointer("/cross_check/status").and_then(|v| v.as_str()).unwrap_or("?").to_string();
        let detail = json.pointer("/cross_check/detail").and_then(|v| v.as_str()).unwrap_or("").to_string();
        if verifier.is_none() && status != "not_run" {
            // "verifyta <version>: ..." in the detail.
            verifier = detail.split(':').next().map(|v| v.trim().to_string());
        }
        let utilization = json.get("utilization_percent").map(|v| v.to_string()).unwrap_or_else(|| "—".into());
        if status == "discrepancy" && !dispositions.contains_key(&g.id) {
            undisposed.push(g.id.clone());
        }
        *by_status.entry(status.clone()).or_default() += 1;
        rows.push(Row { id: g.id.clone(), tasks: g.tasks, sporadic: g.sporadic, resources: g.resources, utilization, rta, status, detail: detail.lines().next().unwrap_or("").to_string() });
    }
    let report_text = render(&rows, &by_status, verifier.as_deref(), &dispositions, &fingerprint(&ws.root));
    Ok(Outcome { rows, by_status, verifier, report: report_text, undisposed })
}

/// The report (Markdown, deterministic: no timestamps).
pub fn render(rows: &[Row], by_status: &BTreeMap<String, usize>, verifier: Option<&str>, dispositions: &BTreeMap<String, String>, fingerprint: &str) -> String {
    let mut s = String::new();
    s.push_str("<!-- @generated by `cargo xtask uppaal-corpus` (xtask/src/corpus.rs). Do not edit; record dispositions in docs/uppaal_dispositions.toml. -->\n");
    s.push_str("# UPPAAL cross-check corpus (R32.6, PAR-03)\n\n");
    s.push_str(&format!(
        "{} Task_Models of 2 to {} Tasks (PAR-03 proposes up to 8; the Target tables offer 7 dispatch vectors), generated deterministically from synthetic App_Declarations by the Generator (`xtask/src/corpus.rs`), analysed with placeholder timing (every WCET equals the Budget; not timing evidence), and cross-checked against the exported UPPAAL model where `verifyta` is installed (R31.4 to R31.6, R31.9; limits {} s and {} MiB per model).\n\n",
        rows.len(),
        MAX_TASKS,
        VERIFIER_TIMEOUT_SECS,
        VERIFIER_MEMORY_MIB
    ));
    match verifier {
        Some(v) => s.push_str(&format!("Verifier: {v}.\n\n")),
        None => s.push_str("Verifier: not installed on the machine that produced this report. Every cross-check is `not_run`: the exports were written (`model.xml`, `model.q` per model under the run's directory) for an offline run, and the Evidence_Item of the corpus is `inconclusive` until a machine with UPPAAL runs the job (ORQ-22, R31.9). Nothing here is a cross-check result.\n\n"),
    }
    s.push_str(&format!("{INPUTS_PREFIX}{fingerprint}` (a digest of the sources this report depends on: the Analyzer, the Generator, the Task_Model crate, this corpus generator and the dispositions). The verification job does not run the corpus; it compares this digest with the tree's and reports the Evidence_Item `inconclusive` when they differ.\n\n"));
    s.push_str("## Summary\n\n| Cross-check status | Models |\n|---|---|\n");
    for (k, v) in by_status {
        s.push_str(&format!("| `{k}` | {v} |\n"));
    }
    let pass = rows.iter().filter(|r| r.rta == "PASS").count();
    let fail = rows.iter().filter(|r| r.rta == "FAIL").count();
    s.push_str(&format!("\nRTA verdicts: {pass} PASS, {fail} FAIL, {} other.\n\n", rows.len() - pass - fail));
    s.push_str("## Models\n\n| Model | Tasks | Sporadic | Resources | Utilization % | RTA | Cross-check | Detail |\n|---|---|---|---|---|---|---|---|\n");
    for r in rows {
        s.push_str(&format!("| {} | {} | {} | {} | {} | {} | `{}` | {} |\n", r.id, r.tasks, r.sporadic, r.resources, r.utilization, r.rta, r.status, r.detail.replace('|', "\\|")));
    }
    s.push_str("\n## Discrepancies and dispositions (R32.6)\n\n");
    let disc: Vec<&Row> = rows.iter().filter(|r| r.status == "discrepancy").collect();
    if disc.is_empty() {
        s.push_str("None reported in this run.\n");
    } else {
        s.push_str("| Model | Verifier detail | Disposition |\n|---|---|---|\n");
        for r in disc {
            let d = dispositions.get(&r.id).map(String::as_str).unwrap_or("**none recorded**: the trace requires confirmation (R31.5)");
            s.push_str(&format!("| {} | {} | {} |\n", r.id, r.detail.replace('|', "\\|"), d));
        }
    }
    s
}

/// How the report introduces its inputs' digest.
const INPUTS_PREFIX: &str = "Inputs: `";

/// A digest of the sources the report depends on: every Rust source of the
/// Analyzer, the Generator and the Task_Model crate, this file, and the
/// dispositions. The report carries it, and the verification job compares
/// it with the tree's (it does not run the corpus).
pub fn fingerprint(root: &Path) -> String {
    fn collect(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for e in entries.flatten() {
            let p = e.path();
            if p.is_dir() {
                collect(&p, out);
            } else if p.extension().is_some_and(|x| x == "rs") {
                out.push(p);
            }
        }
    }
    let mut files = Vec::new();
    for krate in ["rsk-analyzer", "rsk-gen", "rsk-model"] {
        collect(&root.join("crates").join(krate).join("src"), &mut files);
    }
    files.push(root.join("xtask").join("src").join("corpus.rs"));
    files.push(root.join("docs").join("uppaal_dispositions.toml"));
    files.sort();
    let mut data = Vec::new();
    for f in &files {
        data.extend_from_slice(f.strip_prefix(root).unwrap_or(f).to_string_lossy().as_bytes());
        data.push(0);
        data.extend_from_slice(&std::fs::read(f).unwrap_or_default());
        data.push(0);
    }
    rsk_digest::sha256_hex(&data)
}

/// What the verification job reads from the committed report.
#[derive(Debug, PartialEq, Eq)]
pub struct Committed {
    pub models: usize,
    pub by_status: BTreeMap<String, usize>,
    /// `None` when the report says the verifier was not installed.
    pub verifier: Option<String>,
    /// The `Inputs` digest; `None` for a report written before 2026-11-21.
    pub fingerprint: Option<String>,
    /// Discrepancies the report lists without a disposition.
    pub undisposed: Vec<String>,
}

/// Reads a report written by [`render`].
pub fn read_committed(text: &str) -> Committed {
    let mut by_status = BTreeMap::new();
    let mut verifier = None;
    let mut fingerprint = None;
    let mut undisposed = Vec::new();
    let mut section = "";
    for line in text.lines() {
        if let Some(h) = line.strip_prefix("## ") {
            section = if h.starts_with("Summary") { "summary" } else if h.starts_with("Discrepancies") { "dispositions" } else { "" };
            continue;
        }
        if let Some(v) = line.strip_prefix("Verifier: ") {
            if !v.starts_with("not installed") {
                verifier = Some(v.trim_end_matches('.').to_string());
            }
        }
        if let Some(rest) = line.strip_prefix(INPUTS_PREFIX) {
            fingerprint = rest.split('`').next().map(str::to_string);
        }
        let cells: Vec<&str> = line.trim_matches('|').split(" | ").map(str::trim).collect();
        if section == "summary" && cells.len() == 2 {
            if let (Some(status), Ok(n)) = (cells[0].strip_prefix('`').and_then(|c| c.strip_suffix('`')), cells[1].parse::<usize>()) {
                by_status.insert(status.to_string(), n);
            }
        }
        if section == "dispositions" && line.starts_with("| m") && line.contains("**none recorded**") {
            undisposed.push(cells[0].to_string());
        }
    }
    Committed { models: by_status.values().sum(), by_status, verifier, fingerprint, undisposed }
}

/// What verifyta prints when its licence lease does not cover the verifier.
pub const UPPAAL_UNLICENSED: &str = "License does not cover verifier";

/// The corpus models whose Analyzer report carries verifyta's licence
/// refusal. The lease is renewed over the network, so a run inside a
/// network sandbox cannot renew an expired one, and every cross-check then
/// ends with no verdict (2026-11-17).
pub fn unlicensed(work: &Path) -> usize {
    let Ok(dir) = std::fs::read_dir(work) else { return 0 };
    dir.flatten()
        .filter(|e| std::fs::read_to_string(e.path().join("report.json")).is_ok_and(|t| t.contains(UPPAAL_UNLICENSED)))
        .count()
}

/// Where the corpus works and where its report lives.
pub fn work_dir(target_dir: &Path) -> PathBuf {
    target_dir.join("uppaal-corpus")
}

pub fn report_path(root: &Path) -> PathBuf {
    root.join("docs").join("uppaal_corpus_report.md")
}

#[cfg(test)]
mod tests {
    use super::*;

    /// PAR-03: the corpus has at least 50 models of up to 8 Tasks, each a
    /// declaration the Generator accepts, deterministic across runs, with
    /// both Partitions populated, sporadic Tasks and Resources among them.
    #[test]
    fn the_corpus_is_deterministic_accepted_by_the_generator_and_within_par_03() {
        assert!(MODELS >= 50);
        let mut sporadic = 0;
        let mut resources = 0;
        let mut distinct = std::collections::BTreeSet::new();
        for i in 0..MODELS {
            let g = declaration(i);
            assert_eq!(g.declaration, declaration(i).declaration, "model {i} is not deterministic");
            assert!(g.tasks >= 2 && g.tasks <= MAX_TASKS, "model {i} has {} Tasks", g.tasks);
            let model = task_model(&g).unwrap_or_else(|e| panic!("model {i} rejected by the Generator: {e}\n{}", g.declaration));
            let parsed = rsk_model::read_task_model(&model).unwrap_or_else(|e| panic!("model {i}: {e}"));
            assert_eq!(parsed.tasks.len(), g.tasks);
            sporadic += g.sporadic;
            resources += g.resources;
            distinct.insert(g.declaration);
        }
        assert_eq!(distinct.len(), MODELS, "declarations repeat");
        assert!(sporadic > 10 && resources > 20, "sporadic {sporadic}, resources {resources}");
    }

    /// The corpus command counts the models whose report carries verifyta's
    /// licence refusal, and none where the reports carry verdicts.
    #[test]
    fn a_licence_refusal_in_a_corpus_report_is_counted() {
        let work = std::env::temp_dir().join(format!("rsk-uppaal-licence-{}", std::process::id()));
        for (name, text) in [("m00", "\"detail\": \"gave no verdict:\\nLicense does not cover verifier.\\n\""), ("m01", "\"detail\": \"agree\"")] {
            std::fs::create_dir_all(work.join(name)).unwrap();
            std::fs::write(work.join(name).join("report.json"), text).unwrap();
        }
        assert_eq!(unlicensed(&work), 1);
        std::fs::remove_file(work.join("m00").join("report.json")).unwrap();
        assert_eq!(unlicensed(&work), 0);
        let _ = std::fs::remove_dir_all(&work);
    }

    /// The verification job reads back what the report says: the counts,
    /// the verifier, the inputs' digest, and a discrepancy without a
    /// disposition.
    #[test]
    fn the_committed_report_is_read_back() {
        let row = |id: &str, status: &str| Row { id: id.into(), tasks: 2, sporadic: 0, resources: 0, utilization: "10".into(), rta: "PASS".into(), status: status.into(), detail: "d | e".into() };
        let rows = vec![row("m00", "agree"), row("m01", "discrepancy"), row("m02", "discrepancy"), row("m03", "inconclusive")];
        let mut by_status = BTreeMap::new();
        for r in &rows {
            *by_status.entry(r.status.clone()).or_default() += 1;
        }
        let dispositions: BTreeMap<String, String> = [("m01".to_string(), "spurious".to_string())].into();
        let text = render(&rows, &by_status, Some("verifyta UPPAAL 5.0.0"), &dispositions, "sha256:abc");
        let c = read_committed(&text);
        assert_eq!(c, Committed { models: 4, by_status, verifier: Some("verifyta UPPAAL 5.0.0".into()), fingerprint: Some("sha256:abc".into()), undisposed: vec!["m02".into()] });
        let none = read_committed(&render(&rows[..1], &BTreeMap::from([("not_run".to_string(), 1)]), None, &dispositions, "sha256:abc"));
        assert_eq!((none.verifier, none.models), (None, 1));
        assert_eq!(read_committed("# old report\n").fingerprint, None);
    }

    /// The digest changes with a source the report depends on and is the
    /// same for the same tree.
    #[test]
    fn the_inputs_digest_follows_the_sources() {
        let root = std::env::temp_dir().join(format!("rsk-corpus-fp-{}", std::process::id()));
        let src = root.join("crates").join("rsk-analyzer").join("src");
        std::fs::create_dir_all(&src).unwrap();
        std::fs::write(src.join("lib.rs"), "a").unwrap();
        let a = fingerprint(&root);
        assert_eq!(a, fingerprint(&root));
        std::fs::write(src.join("lib.rs"), "b").unwrap();
        assert_ne!(a, fingerprint(&root));
        let _ = std::fs::remove_dir_all(&root);
    }
}
