//! Kernel_Timing_Parameter records from a rig run (task 19.3; R13.2,
//! R13.4, R33.1, R34.6): the Kernel's own release-timing record gives
//! J_rel (the lag from a periodic nominal instant to the Release becoming
//! effective, over every Release of the run) and, in the stress
//! configuration, Δ_irq (one run of the intake handler for the stormed
//! source). Each measured parameter states the runs, the maximum observed
//! value and the margin; its bound is the maximum times `1 + m`, rounded
//! up (R34.6), and the run fails when an observed value exceeds the
//! exported bound of `hil-rig.toml` (R13.4, the release-timing and storm
//! steps). The record is bound to the HIL build that produced it
//! (`binary_hash`), which the Build_Manifest lists among its
//! `hil_binaries`; the Analyzer takes it from there (R33.4).

use rsk_model::wcet::{KernelTimingParameter, Measured, KERNEL_TIMING_SCHEMA};
use rsk_model::KernelTiming;

use crate::release_timing::{Inputs, Outcome};

pub const TOOL: &str = "rsk-hil release-timing record";

/// What the record is bound to: the HIL build's hash, the Target and its
/// core revision as the Task_Model names them, and the build's icache
/// setting.
pub struct Binding<'a> {
    pub binary_hash: &'a str,
    pub target: &'a str,
    pub icache: &'a str,
    /// The configuration (declaration variant) the run used.
    pub configuration: &'a str,
}

/// The margin-applied bound of R34.6: `max * (1 + m / 100)`, rounded up.
pub fn bound(max_observed: u64, margin_percent: u64) -> u64 {
    let scaled = max_observed as u128 * (100 + margin_percent as u128);
    u64::try_from(scaled.div_ceil(100)).unwrap_or(u64::MAX)
}

/// The record of a passed run: J_rel from every periodic Release, and
/// Δ_irq from the stormed source's intakes when the run was the stress
/// configuration. `None` when the run recorded nothing.
pub fn record(inputs: &Inputs<'_>, out: &Outcome, binding: &Binding<'_>, margin_percent: u64) -> Option<KernelTiming> {
    let st = out.stats.as_ref()?;
    let mut parameters = Vec::new();
    let releases = st.total() as u64;
    let scenario = format!("{} ({} s)", binding.configuration, inputs.duration_secs);
    let instrumented = "HIL build (trace-ram, trace-pins): the instrumentation's cost is included".to_string();
    if releases > 0 {
        let max = st.max_cycles() as u64;
        parameters.push(KernelTimingParameter {
            symbol: "j_rel".into(),
            cycles: bound(max, margin_percent),
            method: "measured".into(),
            tool: TOOL.into(),
            conditions: vec![
                "the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)".into(),
                instrumented.clone(),
            ],
            measured: Some(Measured {
                runs: releases,
                max_observed: max,
                margin_percent,
                coverage_branch_percent: 0,
                scenarios: vec![scenario.clone()],
            }),
        });
    }
    if let Some(spec) = &inputs.stress {
        if let Some(ts) = st.tasks.get(spec.storm_task as usize) {
            if ts.intake_count > 0 {
                let max = ts.intake_max_cycles as u64;
                parameters.push(KernelTimingParameter {
                    symbol: "delta_irq".into(),
                    cycles: bound(max, margin_percent),
                    method: "measured".into(),
                    tool: TOOL.into(),
                    conditions: vec![
                        format!("one run of the intake handler for Task {}'s source under the interrupt storm (R19.3), the Kernel's own record", spec.storm_task),
                        instrumented,
                    ],
                    measured: Some(Measured {
                        runs: ts.intake_count as u64,
                        max_observed: max,
                        margin_percent,
                        coverage_branch_percent: 0,
                        scenarios: vec![format!("{scenario}, interrupt storm")],
                    }),
                });
            }
        }
    }
    if parameters.is_empty() {
        return None;
    }
    Some(KernelTiming {
        schema: KERNEL_TIMING_SCHEMA.into(),
        binary_hash: binding.binary_hash.to_string(),
        target: binding.target.to_string(),
        clock_hz: inputs.cpu_hz,
        icache: binding.icache.to_string(),
        parameters,
    })
}

/// Folds a new run's record into the record kept for the same binary: per
/// symbol the runs add up, the maximum and the bound are the larger, and
/// the scenarios are joined. A record of another binary replaces the kept
/// one (the measurements of a binary no longer built are stale, R33.4).
/// Returns whether the kept record was replaced rather than extended.
pub fn fold(kept: Option<KernelTiming>, new: KernelTiming) -> (KernelTiming, bool) {
    let Some(mut kept) = kept else { return (new, true) };
    if kept.binary_hash != new.binary_hash || kept.clock_hz != new.clock_hz || kept.icache != new.icache || kept.target != new.target {
        return (new, true);
    }
    for p in new.parameters {
        match kept.parameters.iter_mut().find(|q| q.symbol == p.symbol) {
            Some(q) => match (&mut q.measured, &p.measured) {
                (Some(a), Some(b)) => {
                    a.runs = a.runs.saturating_add(b.runs);
                    a.max_observed = a.max_observed.max(b.max_observed);
                    a.margin_percent = a.margin_percent.max(b.margin_percent);
                    for s in &b.scenarios {
                        if !a.scenarios.contains(s) {
                            a.scenarios.push(s.clone());
                        }
                    }
                    q.cycles = q.cycles.max(p.cycles);
                }
                _ => {
                    if p.cycles >= q.cycles {
                        *q = p;
                    }
                }
            },
            None => kept.parameters.push(p),
        }
    }
    (kept, false)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn timing(hash: &str, runs: u64, max: u64) -> KernelTiming {
        KernelTiming {
            schema: KERNEL_TIMING_SCHEMA.into(),
            binary_hash: hash.into(),
            target: "nrf52840/r0p1".into(),
            clock_hz: 64_000_000,
            icache: "off".into(),
            parameters: vec![KernelTimingParameter {
                symbol: "j_rel".into(),
                cycles: bound(max, 25),
                method: "measured".into(),
                tool: TOOL.into(),
                conditions: vec![],
                measured: Some(Measured { runs, max_observed: max, margin_percent: 25, coverage_branch_percent: 0, scenarios: vec![format!("run of {max}")] }),
            }],
        }
    }

    #[test]
    fn the_bound_applies_the_margin_rounded_up() {
        assert_eq!(bound(8467, 25), 10584);
        assert_eq!(bound(1, 25), 2);
        assert_eq!(bound(0, 25), 0);
        assert_eq!(bound(100, 0), 100);
    }

    #[test]
    fn folding_runs_of_one_binary_adds_up_and_another_binary_replaces() {
        let (a, replaced) = fold(None, timing("sha256:a", 1000, 8000));
        assert!(replaced);
        let (b, replaced) = fold(Some(a), timing("sha256:a", 500, 8467));
        assert!(!replaced);
        let m = b.parameters[0].measured.as_ref().unwrap();
        assert_eq!((m.runs, m.max_observed), (1500, 8467));
        assert_eq!(b.parameters[0].cycles, 10584);
        assert_eq!(m.scenarios.len(), 2);
        let (c, replaced) = fold(Some(b), timing("sha256:b", 10, 100));
        assert!(replaced);
        assert_eq!(c.parameters[0].measured.as_ref().unwrap().runs, 10);
        // The record round-trips through the model's schema.
        let text = rsk_model::print(&c);
        assert_eq!(rsk_model::read_kernel_timing(&text).unwrap(), c);
    }
}
