//! rsk-analyzer: the schedulability Analyzer (Component D; tasks 15.1 to
//! 16.2). Inputs (R28) are validated and cross-checked, the response-time
//! analysis of R29 runs on `u64` cycle arithmetic with the fixed point
//! verified in Verus (R32.1, `rta`), the report of R30 is produced in JSON
//! and text, and the UPPAAL export of R31 is written next to the report.
#![forbid(unsafe_code)]
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports))]

pub mod inputs;
pub mod report;
pub mod rta;
pub mod uppaal;

use rsk_model::report::{CrossCheck, Inputs, Interference, Profile, Report, TaskResult};
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

/// Runs the recurrence of R29.1 for every prepared Task (R29.5, R29.7,
/// R29.9).
pub fn response_times(tasks: &[inputs::AnalysedTask]) -> Vec<Wcrt> {
    let mut out = Vec::new();
    for (i, t) in tasks.iter().enumerate() {
        let terms = inputs::terms_for(tasks, i);
        let base = t.c.saturating_add(t.b).saturating_add(t.o);
        let (w, schedulable, overflow) = if t.c.checked_add(t.b).and_then(|x| x.checked_add(t.o)).is_none() {
            (u64::MAX, false, true)
        } else {
            match rta::fixpoint(base, &terms, t.j, t.deadline_cycles) {
                rta::Outcome::Fixpoint(w) => (w, true, false),
                rta::Outcome::NotSchedulable(w) => (w, false, false),
                rta::Outcome::Overflow => (u64::MAX, false, true),
            }
        };
        let r = w.saturating_add(t.j);
        let interference = tasks
            .iter()
            .enumerate()
            .filter(|(j, u)| *j != i && u.priority >= t.priority)
            .map(|(_, u)| {
                let jobs = if overflow { 0 } else { (w.saturating_add(u.j)).div_ceil(u.period_cycles.max(1)) };
                Interference {
                    task: u.id.clone(),
                    e_cycles: u.e,
                    jobs,
                    cycles: jobs.saturating_mul(u.e),
                }
            })
            .collect();
        out.push(Wcrt {
            id: t.id.clone(),
            r,
            schedulable: schedulable && r <= t.deadline_cycles,
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
        prepared.omitted_terms.clone(),
        rejections,
        &prepared.placeholders,
    );
    Analysis {
        report,
        uppaal_xml,
        uppaal_query,
        exit: if all_ok { EXIT_PASS } else { EXIT_FAIL },
    }
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
