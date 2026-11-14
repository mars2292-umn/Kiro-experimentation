//! rsk-analyzer: the schedulability Analyzer (Component D; tasks 15.1 to
//! 16.2). Inputs (R28) are validated and cross-checked, the response-time
//! analysis of R29 runs on `u64` cycle arithmetic with the fixed point
//! verified in Verus (R32.1, `rta`), the report of R30 is produced in JSON
//! and text, and the UPPAAL export of R31 is written next to the report.
#![forbid(unsafe_code)]
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports))]

pub mod inputs;
pub mod profile_version;
pub mod report;
pub mod rta;
pub mod terms;
pub mod uppaal;

use rsk_model::report::{ChainResult, CrossCheck, Inputs, Interference, Profile, Report, TaskResult};
use rsk_model::task_model::TaskModel;
use rsk_model::{KernelTiming, WcetSet};

pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Exit statuses (R30.4).
pub const EXIT_PASS: u8 = 0;
pub const EXIT_FAIL: u8 = 1;
pub const EXIT_REJECTED: u8 = 2;
pub const EXIT_INTERNAL: u8 = 3;

/// The result of one analysis: the report and the UPPAAL export.
pub struct Analysis {
    pub report: Report,
    pub uppaal_xml: String,
    pub uppaal_query: String,
    pub exit: u8,
}

/// Per-Task fixed-point results (used by the tests of R32 as well).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Wcrt {
    pub id: String,
    pub r: u64,
    pub schedulable: bool,
    pub overflow: bool,
    pub interference: Vec<Interference>,
}

/// The four inputs read and validated against their schemas (R28.1): each
/// violated rule is reported with its JSON path (R25.6). The texts are the
/// file contents; the names are for the messages.
pub struct Loaded {
    pub model: TaskModel,
    pub wcet: WcetSet,
    pub timing: KernelTiming,
    pub manifest: serde_json::Value,
}

pub fn load(model: &str, wcet: &str, timing: &str, manifest: &str) -> Result<Loaded, Vec<String>> {
    let mut rejected = Vec::new();
    let model = rsk_model::read_task_model(model).map_err(|e| rejected.push(format!("Task_Model: {e}"))).ok();
    let wcet = rsk_model::read_wcet_set(wcet).map_err(|e| rejected.push(format!("WCET_Records: {e}"))).ok();
    let timing = rsk_model::read_kernel_timing(timing).map_err(|e| rejected.push(format!("Kernel_Timing_Parameters: {e}"))).ok();
    let manifest = rsk_model::read_build_manifest(manifest).map_err(|e| rejected.push(format!("Build_Manifest: {e}"))).ok();
    match (model, wcet, timing, manifest) {
        (Some(model), Some(wcet), Some(timing), Some(manifest)) if rejected.is_empty() => Ok(Loaded { model, wcet, timing, manifest }),
        _ => Err(rejected),
    }
}

/// Runs the recurrence of R29.1 for every prepared Task (R29.5, R29.7,
/// R29.9), with the intake terms of R19.2.
pub fn response_times(tasks: &[inputs::AnalysedTask]) -> Vec<Wcrt> {
    let mut out = Vec::new();
    // The arithmetic of each Task's verdict is `terms::response_of`
    // (verified since 2026-11-14: the terms, the fixed point from
    // C + B + O, and R = J + w against the deadline); what follows it here
    // is the report's breakdown.
    let interferers = inputs::interferers(tasks);
    for (i, t) in tasks.iter().enumerate() {
        let (verdict, terms) = terms::response_of(&interferers, i, t.c, t.b, t.o, t.deadline_cycles);
        let (w, r, schedulable, overflow) = (verdict.w, verdict.r, verdict.schedulable, verdict.overflow);
        let interference = terms
            .iter()
            .map(|term| {
                let jobs = if overflow { 0 } else { (w.saturating_add(term.term.jitter)).div_ceil(term.term.period) };
                Interference {
                    task: if term.intake { format!("{} (intake, R19.2)", tasks[term.task].id) } else { tasks[term.task].id.clone() },
                    e_cycles: term.term.bound,
                    jobs,
                    cycles: jobs.saturating_mul(term.term.bound),
                }
            })
            .collect();
        out.push(Wcrt {
            id: t.id.clone(),
            r,
            schedulable,
            overflow,
            interference,
        });
    }
    out
}

/// The whole analysis (R28 to R31). `names` are the input file names or
/// hashes recorded in the report (R30.2).
pub fn analyze(
    model: &TaskModel,
    wcet: &WcetSet,
    timing: &KernelTiming,
    manifest: &serde_json::Value,
    names: Inputs,
    options: &inputs::Options,
) -> Analysis {
    let profile = Profile {
        version: model.profile.version.clone(),
        variant: model.profile.variant.clone(),
    };
    let prepared = match inputs::prepare(model, wcet, timing, manifest, options) {
        Ok(p) => p,
        Err(rejections) => {
            let report = report::build(
                "REJECTED",
                names,
                profile,
                VERSION,
                vec![],
                0,
                CrossCheck {
                    status: "not_run".into(),
                    detail: "input rejected".into(),
                },
                vec![],
                rejections.iter().map(ToString::to_string).collect(),
                &[],
                &[],
                &[],
                vec![],
            );
            return Analysis {
                report,
                uppaal_xml: String::new(),
                uppaal_query: String::new(),
                exit: EXIT_REJECTED,
            };
        }
    };
    let utilization_percent = inputs::utilization_percent(&prepared.tasks);
    let over_utilized = inputs::utilization_exceeds_one(&prepared.tasks);
    let wcrts = if over_utilized { Vec::new() } else { response_times(&prepared.tasks) };
    let mut all_ok = !over_utilized;
    let mut rejections = Vec::new();
    if over_utilized {
        rejections.push(format!("[R29.9] Σ E_j/T_j exceeds 1 ({utilization_percent}%): FAIL without fixed-point iteration"));
    }
    let mut task_results = Vec::new();
    for (i, t) in prepared.tasks.iter().enumerate() {
        let (r, schedulable, interference) = match wcrts.get(i) {
            Some(w) => {
                if w.overflow {
                    rejections.push(format!("[R29.6] arithmetic overflow while analysing `{}`: no result", t.id));
                    all_ok = false;
                }
                (w.r, w.schedulable && !w.overflow, w.interference.clone())
            }
            None => (0, false, vec![]),
        };
        if !schedulable {
            all_ok = false;
        }
        task_results.push(TaskResult {
            id: t.id.clone(),
            partition: t.partition.clone(),
            level: t.level.clone(),
            priority: t.priority,
            period_cycles: t.period_cycles,
            deadline_cycles: t.deadline_cycles,
            c_cycles: t.c,
            b_cycles: t.b,
            b_source: t.b_source.clone(),
            o_cycles: t.o,
            j_cycles: t.j,
            interference,
            r_cycles: r,
            schedulable,
            slack_cycles: if schedulable { t.deadline_cycles - r } else { 0 },
            wcet_provenance: t.wcet_provenance.clone(),
        });
    }
    // Task 21.5 (R41.4): the end-to-end latency bound of each declared
    // chain: Δ_stamp when the chain starts at an interrupt event, then for
    // each Task its WCRT, preceded for a Periodic_Task after the first by
    // its period (it samples the Endpoint once per period) and for a
    // Sporadic_Task released by the hop's Endpoint under `defer` by its MIT
    // (an arrival inside the window is released at the window's end); a
    // Sporadic_Task released otherwise, or discarding arrivals, leaves the
    // chain unbounded. PASS only when every Task of the chain is
    // schedulable and the bound is within the deadline.
    let mut chain_results = Vec::new();
    for c in &prepared.chains {
        let mut named: Vec<(String, u64)> = Vec::new();
        let mut unbounded: Option<String> = None;
        let mut every_task_schedulable = true;
        for (pos, &ti) in c.tasks.iter().enumerate() {
            let t = &prepared.tasks[ti];
            let (r, sched) = wcrts.get(ti).map(|w| (w.r, w.schedulable && !w.overflow)).unwrap_or((0, false));
            if !sched {
                every_task_schedulable = false;
            }
            if pos == 0 {
                if t.source_interrupt {
                    named.push((format!("Δ_stamp (the chain starts at the interrupt event that releases `{}`)", t.id), prepared.stamp_cycles));
                }
            } else {
                let e = &c.endpoints[pos - 1];
                if t.sporadic {
                    match &t.source_endpoint {
                        Some(x) if x == e => {
                            if t.defer {
                                named.push((format!("MIT of `{}` (`defer`: an arrival inside the MIT window is released at its end, R10.2)", t.id), t.period_cycles));
                            } else if unbounded.is_none() {
                                unbounded = Some(format!("`{}` discards arrivals inside its MIT (R10.8 keeps the message for a later Release the chain does not bound)", t.id));
                            }
                        }
                        _ => {
                            if unbounded.is_none() {
                                unbounded = Some(format!("`{}` is not released by its role of Endpoint `{e}`, so when it reads the message is not bounded by the chain", t.id));
                            }
                        }
                    }
                } else {
                    named.push((format!("T of `{}` (a Periodic_Task samples the Endpoint once per period)", t.id), t.period_cycles));
                }
            }
            named.push((format!("R of `{}`{}", t.id, if sched { "" } else { " (NOT schedulable: a lower bound)" }), r));
        }
        let values: Vec<u64> = named.iter().map(|(_, v)| *v).collect();
        let (bound, result) = match (unbounded, terms::checked_sum(&values)) {
            (Some(why), _) => {
                named.push((why, 0));
                (u64::MAX, "UNBOUNDED")
            }
            (None, None) => {
                named.push(("the sum of the terms exceeds u64 (R29.6)".to_string(), 0));
                (u64::MAX, "UNBOUNDED")
            }
            (None, Some(b)) => (b, if every_task_schedulable && b <= c.deadline_cycles { "PASS" } else { "FAIL" }),
        };
        if result != "PASS" {
            all_ok = false;
        }
        chain_results.push(ChainResult {
            id: c.id.clone(),
            tasks: c.tasks.iter().map(|&i| prepared.tasks[i].id.clone()).collect(),
            bound_cycles: bound,
            deadline_cycles: c.deadline_cycles,
            result: result.to_string(),
            terms: named.iter().map(|(n, v)| if *v == 0 && n.contains("not ") || n.contains("exceeds u64") || n.contains("discards") { n.clone() } else { format!("{n}: {v} cycles") }).collect(),
        });
    }
    let (uppaal_xml, uppaal_query) = uppaal::export(&prepared.tasks);
    let result = if all_ok { "PASS" } else { "FAIL" };
    let report = report::build(
        result,
        names,
        profile,
        VERSION,
        task_results,
        utilization_percent,
        CrossCheck {
            status: "not_run".into(),
            detail: "run the UPPAAL cross-check with --uppaal".into(),
        },
        // The RTA's omitted terms and the UPPAAL export's approximations (R31.2).
        prepared.omitted_terms.iter().cloned().chain(uppaal::omissions()).collect(),
        rejections,
        &prepared.placeholders,
        &prepared.measured_parameters,
        &unprotected_partitions(model),
        chain_results,
    );
    Analysis {
        report,
        uppaal_xml,
        uppaal_query,
        exit: if all_ok { EXIT_PASS } else { EXIT_FAIL },
    }
}

/// R18.9: when more than one Partition has the highest Criticality_Level
/// (A before B before C) and any of them declares an Overrun_Response that
/// lets an overrunning Job continue, those Partitions, each marked
/// `(continues)` or `(stops)`, are not protected from one another by
/// R18.7; the report must say so.
pub fn unprotected_partitions(model: &TaskModel) -> Vec<String> {
    let Some(top) = model.partitions.iter().map(|p| p.level.as_str()).min() else { return vec![] };
    let highest: Vec<&rsk_model::task_model::Partition> = model.partitions.iter().filter(|p| p.level == top).collect();
    if highest.len() < 2 || !highest.iter().any(|p| p.overrun == "RECORD_AND_CONTINUE") {
        return vec![];
    }
    highest.iter().map(|p| format!("{} ({})", p.id, if p.overrun == "RECORD_AND_CONTINUE" { "continues" } else { "stops" })).collect()
}

/// Placeholder timing inputs for a binary without measurements (pre-
/// hardware development): one `placeholder` WCET_Record per Task entry and
/// Critical_Section site of the model, each equal to the Task's Budget
/// (a Critical_Section cannot outlast its Job's Budget, so this is the
/// conservative choice), and every Kernel_Timing_Parameter at a nominal
/// value. The Analyzer accepts them only with `allow_placeholders`, and
/// the report states that it is not timing evidence.
pub mod placeholders {
    use rsk_model::task_model::TaskModel;
    use rsk_model::wcet::{KernelTimingParameter, Measured, KERNEL_TIMING_SYMBOLS};
    use rsk_model::{KernelTiming, WcetRecord, WcetSet};

    pub const TOOL: &str = "rsk-analyzer placeholders";

    /// Task 19.1: the measured records of `measured` replace the placeholder
    /// records of their items (a placeholder is the Task's Budget, never a
    /// bound to keep); an item without a placeholder gains the record.
    /// Returns the number of items taken.
    pub fn overlay_wcet(set: &mut WcetSet, measured: &WcetSet) -> usize {
        let mut taken = 0;
        for r in &measured.records {
            if r.method == "placeholder" {
                continue;
            }
            set.records.retain(|q| !(q.item == r.item && q.method == "placeholder"));
            set.records.push(r.clone());
            taken += 1;
        }
        taken
    }

    pub fn make(model: &TaskModel, binary_hash: &str, icache: &str) -> (WcetSet, KernelTiming) {
        let target = format!("{}/{}", model.target, model.core_revision);
        let record = |item: String, cycles: u64| WcetRecord {
            item,
            wcet_cycles: cycles,
            method: "placeholder".into(),
            tool: TOOL.into(),
            binary_hash: binary_hash.to_string(),
            target: target.clone(),
            clock_hz: model.clock.cpu_hz,
            icache: icache.to_string(),
            conditions: vec!["placeholder: no measurement".into()],
            measured: None::<Measured>,
        };
        let mut records = Vec::new();
        for t in &model.tasks {
            records.push(record(t.id.clone(), t.budget_cycles as u64));
            for site in &t.cs_sites {
                records.push(record(site.clone(), t.budget_cycles as u64));
            }
        }
        let timing = KernelTiming {
            schema: rsk_model::wcet::KERNEL_TIMING_SCHEMA.into(),
            binary_hash: binary_hash.to_string(),
            target,
            clock_hz: model.clock.cpu_hz,
            icache: icache.to_string(),
            parameters: KERNEL_TIMING_SYMBOLS
                .iter()
                .map(|s| KernelTimingParameter {
                    symbol: s.to_string(),
                    cycles: match *s {
                        "l_kernel" => 2000,
                        "j_rel" => 500,
                        "delta_restart" | "delta_stop" => 5000,
                        _ => 300,
                    },
                    method: "placeholder".into(),
                    tool: TOOL.into(),
                    conditions: vec!["placeholder: nominal value".into()],
                    measured: None,
                })
                .collect(),
        };
        (
            WcetSet {
                schema: rsk_model::wcet::WCET_SCHEMA.into(),
                records,
            },
            timing,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");

    fn manifest_for(model: &TaskModel) -> serde_json::Value {
        serde_json::json!({
            "schema": "rsk-build-manifest/1",
            "generated_config_checksum": model.config_checksum,
            "build": {"icache": "off"},
            "binaries": [{"hash": "sha256:demo", "package": "p1-demo"}]
        })
    }

    /// R30.4, R30.1, R29.1: exit 0 only on PASS and a distinct status for
    /// rejected input; the JSON report conforms to its schema and round-trips;
    /// the lower-Priority Task suffers interference from the higher one's Jobs.
    #[test]
    fn the_demo_passes_with_placeholders_and_is_rejected_without() {
        let model = rsk_model::read_task_model(DEMO).unwrap();
        let (wcet, timing) = placeholders::make(&model, "sha256:demo", "off");
        let names = Inputs { task_model: "m".into(), wcet_records: "w".into(), kernel_timing: "k".into(), build_manifest: "b".into() };
        let strict = analyze(&model, &wcet, &timing, &manifest_for(&model), names.clone(), &inputs::Options::default());
        assert_eq!(strict.exit, EXIT_REJECTED);
        assert_eq!(strict.report.result, "REJECTED");
        let a = analyze(&model, &wcet, &timing, &manifest_for(&model), names, &inputs::Options { allow_placeholders: true, prefer_method: None });
        assert_eq!(a.exit, EXIT_PASS, "{}", report::render_text(&a.report));
        assert!(a.report.statements.iter().any(|s| s.contains("PLACEHOLDER")));
        // control (P7): R = J + C + B + O; sensor (P5) suffers interference from control.
        let control = &a.report.tasks[0];
        let sensor = &a.report.tasks[1];
        assert!(control.r_cycles < sensor.r_cycles);
        assert!(sensor.interference.iter().any(|i| i.task == "part0.control" && i.jobs >= 2));
        // The report round-trips through its schema.
        let json = report::render_json(&a.report);
        assert_eq!(rsk_model::read_report(&json).unwrap(), a.report);
        assert_eq!(rsk_model::read_report(&report::render_json(&a.report)).unwrap(), a.report);
    }

    #[test]
    fn a_checksum_mismatch_is_rejected_citing_r28_8() {
        let model = rsk_model::read_task_model(DEMO).unwrap();
        let (wcet, timing) = placeholders::make(&model, "sha256:demo", "off");
        let mut manifest = manifest_for(&model);
        manifest["generated_config_checksum"] = serde_json::json!("0xdeadbeef");
        let names = Inputs { task_model: "m".into(), wcet_records: "w".into(), kernel_timing: "k".into(), build_manifest: "b".into() };
        let a = analyze(&model, &wcet, &timing, &manifest, names, &inputs::Options { allow_placeholders: true, prefer_method: None });
        assert_eq!(a.exit, EXIT_REJECTED);
        assert!(a.report.rejections.iter().any(|r| r.contains("R28.8")), "{:?}", a.report.rejections);
    }
}
