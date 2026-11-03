//! WCET_Records from a rig run (task 19.1; R33.1, R34.3, R34.6, R34.7):
//! the Task-entry maxima from the Kernel's Budget accounting and the
//! Critical_Section maxima from the accounting at the lock and unlock
//! SVCs, less the calibrated cycle-counter read cost, each a measured
//! record with its runs, maximum and margin, bound to the HIL build that
//! produced it. The items are named as the Task_Model names them (the
//! Task id; `<task>#cs:<resource>` for a site), so that the Analyzer takes
//! them over its placeholders.

use rsk_model::task_model::TaskModel;
use rsk_model::wcet::{Measured, WcetRecord, WcetSet, WCET_SCHEMA};

use crate::kernel_timing::{bound, Binding};
use crate::release_timing::{Inputs, Outcome};
use crate::stats::Stats;

pub const TOOL: &str = "rsk-hil Budget-accounting record";

/// PAR-02: the executions a measured item needs; fewer are recorded with
/// the shortfall among the conditions.
pub const PAR_02_RUNS: u64 = 10_000;

/// The WCET_Records of a passed run. `None` when nothing was recorded.
pub fn record(inputs: &Inputs<'_>, out: &Outcome, model: &TaskModel, binding: &Binding<'_>, margin_percent: u64) -> Option<WcetSet> {
    let st = out.stats.as_ref()?;
    let target = format!("{}/{}", model.target, model.core_revision);
    let scenario = format!("{} ({} s)", binding.configuration, inputs.duration_secs);
    let mut records = Vec::new();
    let base_conditions = |what: &str, runs: u64| {
        let mut c = vec![
            what.to_string(),
            "HIL build (trace-ram, trace-pins): the instrumentation's cost is included (R34.7)".to_string(),
            format!("cycle-counter read cost {} cycles calibrated at instant 0 (R34.3)", st.calibration_cycles),
        ];
        if runs < PAR_02_RUNS {
            c.push(format!("{runs} executions, below PAR-02's {PAR_02_RUNS}"));
        }
        c
    };
    let make = |item: String, max: u32, runs: u64, what: &str| WcetRecord {
        item,
        wcet_cycles: bound(max as u64, margin_percent),
        method: "measured".into(),
        tool: TOOL.into(),
        binary_hash: binding.binary_hash.to_string(),
        target: target.clone(),
        clock_hz: inputs.cpu_hz,
        icache: binding.icache.to_string(),
        conditions: base_conditions(what, runs),
        measured: Some(Measured { runs, max_observed: max as u64, margin_percent, coverage_branch_percent: 0, scenarios: vec![scenario.clone()] }),
    };
    for (i, ts) in st.tasks.iter().enumerate() {
        let Some(task) = model.tasks.get(i) else { continue };
        if ts.exec_count > 0 {
            records.push(make(task.id.clone(), ts.exec_max_cycles, ts.exec_count as u64, "the Task entry: one Job's execution from the Budget accounting (R18.1)"));
        }
    }
    for c in &st.cs {
        let Some(task) = model.tasks.get(c.task as usize) else { continue };
        let Some(res) = model.resources.get(c.resource as usize) else { continue };
        let name = res.id.rsplit('.').next().unwrap_or(&res.id);
        let site = format!("{}#cs:{name}", task.id);
        if !task.cs_sites.contains(&site) {
            continue;
        }
        records.push(make(site, c.max_cycles, c.count as u64, "the Critical_Section: execution from the lock's return to the unlock's entry, from the Budget accounting"));
    }
    if records.is_empty() {
        return None;
    }
    Some(WcetSet { schema: WCET_SCHEMA.into(), records })
}

/// Folds a run's records into the kept set of the same binary (per item:
/// the runs add up, the maximum and the bound are the larger, the
/// scenarios join); a set of another binary replaces the kept one.
pub fn fold(kept: Option<WcetSet>, new: WcetSet) -> (WcetSet, bool) {
    let Some(mut kept) = kept else { return (new, true) };
    let same = |a: &WcetRecord, b: &WcetRecord| a.binary_hash == b.binary_hash && a.clock_hz == b.clock_hz && a.icache == b.icache && a.target == b.target;
    if let (Some(a), Some(b)) = (kept.records.first(), new.records.first()) {
        if !same(a, b) {
            return (new, true);
        }
    }
    for r in new.records {
        match kept.records.iter_mut().find(|q| q.item == r.item && q.method == r.method) {
            Some(q) => match (&mut q.measured, &r.measured) {
                (Some(a), Some(b)) => {
                    a.runs = a.runs.saturating_add(b.runs);
                    a.max_observed = a.max_observed.max(b.max_observed);
                    a.margin_percent = a.margin_percent.max(b.margin_percent);
                    for s in &b.scenarios {
                        if !a.scenarios.contains(s) {
                            a.scenarios.push(s.clone());
                        }
                    }
                    q.wcet_cycles = q.wcet_cycles.max(r.wcet_cycles);
                    q.conditions = r.conditions.clone();
                    if a.runs >= PAR_02_RUNS {
                        q.conditions.retain(|c| !c.contains("below PAR-02"));
                    }
                }
                _ => {
                    if r.wcet_cycles >= q.wcet_cycles {
                        *q = r;
                    }
                }
            },
            None => kept.records.push(r),
        }
    }
    (kept, false)
}

/// The agent against the logic-analyser trace (task 19.1's evidence): for
/// Task `t`, the largest execution the Budget accounting recorded and the
/// longest uninterrupted run its trace pin showed, in cycles; the trace's
/// run is wall time and holds the Kernel's entries, so it bounds the
/// agent's figure from above when nothing preempts the Task.
pub fn against_trace(st: &Stats, longest_run_samples: usize, samplerate_hz: u32, cpu_hz: u64, t: usize) -> Option<(u32, u64)> {
    let ts = st.tasks.get(t)?;
    if ts.exec_count == 0 || longest_run_samples == 0 {
        return None;
    }
    let trace_cycles = longest_run_samples as u64 * cpu_hz / samplerate_hz as u64;
    Some((ts.exec_max_cycles, trace_cycles))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set(hash: &str, item: &str, runs: u64, max: u64) -> WcetSet {
        WcetSet {
            schema: WCET_SCHEMA.into(),
            records: vec![WcetRecord {
                item: item.into(),
                wcet_cycles: bound(max, 25),
                method: "measured".into(),
                tool: TOOL.into(),
                binary_hash: hash.into(),
                target: "nrf52840/r0p1".into(),
                clock_hz: 64_000_000,
                icache: "off".into(),
                conditions: if runs < PAR_02_RUNS { vec![format!("{runs} executions, below PAR-02's {PAR_02_RUNS}")] } else { vec![] },
                measured: Some(Measured { runs, max_observed: max, margin_percent: 25, coverage_branch_percent: 0, scenarios: vec![format!("run of {max}")] }),
            }],
        }
    }

    #[test]
    fn folding_adds_runs_and_drops_the_par_02_shortfall_once_met() {
        let (a, replaced) = fold(None, set("sha256:a", "part0.control", 6000, 19000));
        assert!(replaced);
        assert!(a.records[0].conditions.iter().any(|c| c.contains("below PAR-02")));
        let (b, replaced) = fold(Some(a), set("sha256:a", "part0.control", 6000, 19500));
        assert!(!replaced);
        let m = b.records[0].measured.as_ref().unwrap();
        assert_eq!((m.runs, m.max_observed), (12000, 19500));
        assert_eq!(b.records[0].wcet_cycles, bound(19500, 25));
        assert!(!b.records[0].conditions.iter().any(|c| c.contains("below PAR-02")));
        let (c, replaced) = fold(Some(b), set("sha256:b", "part0.control", 1, 1));
        assert!(replaced);
        assert_eq!(c.records[0].measured.as_ref().unwrap().runs, 1);
        let text = rsk_model::print(&c);
        assert_eq!(rsk_model::read_wcet_set(&text).unwrap(), c);
    }
}
