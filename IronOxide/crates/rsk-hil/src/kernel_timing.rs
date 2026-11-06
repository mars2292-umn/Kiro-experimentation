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

/// The Kernel_Timing_Parameters of Table 13-1 that the Kernel-entry table
/// measures (task 19.3): symbol, the entry kinds whose maxima add up to
/// it, and the condition text. Δ_complete is the Job-end SVC plus the
/// unwind's epilogue; Δ_hm the fault entry (a Health_Monitor response
/// other than an Overrun_Response); Δ_enforce the entry that recorded an
/// Overrun and applied the response; Δ_detect is the time-base handling
/// of the Budget compare, the timer entry.
pub const ENTRY_PARAMETERS: &[(&str, &[usize], &str)] = &[
    ("delta_dispatch", &[0], "one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry)"),
    ("delta_timer", &[1], "one time-base entry: every due timed event served (the Rust half)"),
    ("delta_detect", &[1], "the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry)"),
    ("delta_lock_in", &[3], "one lock SVC (the Rust half)"),
    ("delta_lock_out", &[4], "one unlock SVC (the Rust half)"),
    ("delta_complete", &[5, 10], "the Job-end SVC plus the unwind's epilogue (the Rust halves)"),
    ("delta_svc", &[6], "the now SVC, the service with the least work (the Rust half)"),
    ("delta_dma_write", &[8], "one EasyDMA request SVC, validation and the register writes (the Rust half)"),
    ("delta_hm", &[9], "one fault entry applying the Partition's fault response (the Rust half)"),
    ("delta_restart", &[11], "an entry that restarted a Partition (the Rust half, whole)"),
    ("delta_stop", &[12], "an entry that stopped a Partition (the Rust half, whole)"),
    ("delta_enforce", &[13], "an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole)"),
    ("delta_view", &[15], "one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6)"),
];

/// The parameters of `ENTRY_PARAMETERS` the record `st` measured, and
/// L_kernel: the longest Kernel entry of any kind (every entry runs with
/// preemption masked), the stub's portion being Δ_entry where the run
/// measured it (`delta_entry_cycles`).
pub fn entry_parameters(st: &crate::stats::Stats, scenario: &str, margin_percent: u64) -> Vec<KernelTimingParameter> {
    entry_parameters_with(st, scenario, margin_percent, None)
}

pub fn entry_parameters_with(st: &crate::stats::Stats, scenario: &str, margin_percent: u64, delta_entry_cycles: Option<u64>) -> Vec<KernelTimingParameter> {
    let instrumented = "HIL build (trace-ram, trace-pins): the instrumentation's cost is included".to_string();
    let mut out = Vec::new();
    // L_kernel: the longest Rust half of any entry kind but the view load
    // (inside a dispatch), plus the stub's cycles outside the charge points.
    let longest = st.entries.iter().enumerate().filter(|(k, _)| *k != 15).map(|(_, e)| e.max_cycles as u64).max().unwrap_or(0);
    if longest > 0 {
        let runs: u64 = st.entries.iter().map(|e| e.count as u64).sum();
        let max = longest + delta_entry_cycles.unwrap_or(0);
        out.push(KernelTimingParameter {
            symbol: "l_kernel".into(),
            cycles: bound(max, margin_percent),
            method: "measured".into(),
            tool: TOOL.into(),
            conditions: vec![
                format!("the longest Kernel entry of the run (every entry masks preemption, DD-01): the largest Rust half of any kind{} (task 19.3; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)", match delta_entry_cycles { Some(d) => format!(" plus the stub's measured {d} cycles outside the charge points (Δ_entry)"), None => " (the stub's cycles outside the charge points, Δ_entry, come from the cycle configuration's record)".to_string() }),
                instrumented.clone(),
            ],
            measured: Some(Measured { runs, max_observed: max, margin_percent, coverage_branch_percent: 0, scenarios: vec![scenario.to_string()] }),
        });
    }
    for (symbol, kinds, what) in ENTRY_PARAMETERS {
        let parts: Vec<&crate::stats::EntryStats> = kinds.iter().filter_map(|k| st.entries.get(*k)).collect();
        if parts.is_empty() || parts.iter().any(|e| e.count == 0) {
            continue;
        }
        let max: u64 = parts.iter().map(|e| e.max_cycles as u64).sum();
        let runs = parts.iter().map(|e| e.count as u64).min().unwrap_or(0);
        out.push(KernelTimingParameter {
            symbol: symbol.to_string(),
            cycles: bound(max, margin_percent),
            method: "measured".into(),
            tool: TOOL.into(),
            conditions: vec![format!("{what}, the Kernel's entry record (task 19.3)"), instrumented.clone()],
            measured: Some(Measured { runs, max_observed: max, margin_percent, coverage_branch_percent: 0, scenarios: vec![scenario.to_string()] }),
        });
    }
    out
}

/// A record from a record of any run: the entry parameters, plus the
/// parameters given (`extra`, such as Δ_entry from the cycle test).
pub fn record_from_stats(st: &crate::stats::Stats, binding: &Binding<'_>, cpu_hz: u64, scenario: &str, margin_percent: u64, extra: Vec<KernelTimingParameter>) -> Option<KernelTiming> {
    let delta_entry = extra.iter().find(|p| p.symbol == "delta_entry").and_then(|p| p.measured.as_ref()).map(|m| m.max_observed);
    let mut parameters = entry_parameters_with(st, scenario, margin_percent, delta_entry);
    parameters.extend(extra);
    if parameters.is_empty() {
        return None;
    }
    Some(KernelTiming { schema: KERNEL_TIMING_SCHEMA.into(), binary_hash: binding.binary_hash.to_string(), target: binding.target.to_string(), clock_hz: cpu_hz, icache: binding.icache.to_string(), parameters })
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
    parameters.extend(entry_parameters(st, &scenario, margin_percent));
    if parameters.is_empty() {
        return None;
    }
    // The interference scenarios of R34.5 as the run observed them (task
    // 19.2), after the configuration the records fold on.
    let extra: Vec<String> = crate::release_timing::scenarios(inputs, out).into_iter().skip(1).collect();
    for p in &mut parameters {
        if let Some(m) = &mut p.measured {
            for s in &extra {
                if !m.scenarios.contains(s) {
                    m.scenarios.push(s.clone());
                }
            }
        }
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
