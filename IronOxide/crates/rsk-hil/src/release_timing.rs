//! The release-timing run (task 7.3, R9.10): the Application under test
//! in its release-timing configuration (every periodic offset 0, a
//! Critical_Section across nominal instants, injected late Jobs) runs for
//! its whole operating duration (PAR-05: the hour-long form, or the short
//! form for the pipeline) with the stimulus board driving the sporadic
//! source and the analyzer capturing at a low rate; afterwards the
//! Kernel's release-timing record, its trace ring, and the console are
//! read and judged:
//!
//! - every recorded nominal instant is `O + k*T` (the record's mismatch
//!   count is zero), and every recorded lag is below `J_rel`
//!   (`expect.j_rel_cycles`), over every Release of the run;
//! - the counter of the time base wrapped at least once (the run is
//!   longer than the 32-bit counter's period; the Kernel's trace shows
//!   the instants ordered across the wrap);
//! - the late Jobs produced the outcomes of R9.6 to R9.8 and nothing
//!   else did: the console's `RSK-OK` is the target's own verdict on its
//!   counts and events;
//! - the stimulus pulses were answered within the deadline throughout
//!   (the analyzer's loopback), and the Kernel trace shows no storm.

use std::path::Path;
use std::time::Duration;

use crate::board::{Run, Target};
use crate::capture::{self, Analysis, Timeline};
use crate::config::Rig;
use crate::stats::Stats;
use crate::trace::Entry;

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    /// The Application under test in its release-timing configuration.
    pub target_elf: &'a Path,
    pub stimulus_elf: &'a Path,
    pub work_dir: &'a Path,
    /// The operating duration of the configuration, in seconds, and the
    /// bound on the run beyond it.
    pub duration_secs: u64,
    pub grace_secs: u64,
    pub tick_hz: u64,
    pub cpu_hz: u64,
    /// The analyzer's rate for this run (low: an hour of samples).
    pub samplerate_hz: u32,
    /// Flash and start the stimulus board first (it may already run).
    pub flash_stimulus: bool,
    /// The stress configuration of the fault-injection suite (task 7.5,
    /// R18.8, R19.3): the lower-criticality Partitions misbehave and the
    /// stimulus board storms the source; the loopback step becomes the
    /// storm step, and the protected Partition's Jobs are checked against
    /// their computed WCRTs.
    pub stress: Option<Stress>,
    /// The Profile_Lint's `rsk-loop-bounds/1` export of the Application
    /// (task 19.2, R34.9, R35.4): the loop report is judged against it.
    pub loop_bounds: Option<&'a Path>,
    /// The configuration and the HIL build's hash, for the loop report.
    pub configuration: &'a str,
    pub binary_hash: &'a str,
    /// Whether the run's timing is judged against the exported bounds
    /// (J_rel, Δ_irq, Δ_stamp, the wire deadline): false for the Kernel
    /// coverage build, whose instrumented Kernel is no timing evidence
    /// (task 19.5); its figures are reported, not judged.
    pub timing_bounds: bool,
    /// The `defer-probe` witness build of Verification_Plan 5.8a (the
    /// dispatch stub's entry and return windows widened): the step
    /// "dispatch deferrals" then fails when the run deferred none.
    pub expect_deferrals: bool,
}

/// What the stress configuration expects (task 7.5).
#[derive(Clone, Debug, Default)]
pub struct Stress {
    /// The Tasks of the highest-criticality Partition (R18.7) with the
    /// WCRT the Analyzer computed for each, in CPU cycles.
    pub protected: Vec<(u8, String, u64)>,
    /// The Task whose source the stimulus board storms, and its MIT in ms.
    pub storm_task: u8,
    pub mit_ms: u64,
}

#[derive(Clone, Debug)]
pub struct Step {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub run: Option<Run>,
    pub stats: Option<Stats>,
    pub trace: Vec<Entry>,
    pub events: Option<crate::events::Events>,
    /// The Kernel's fault record (`FAULT_RING`, feature `trace-ram`), read
    /// after every run since 2026-11-05 so that a fault event of the run
    /// carries its CFSR and address (the UsageFault of Task 0 at 148,400 ms
    /// in the timing-short run of that day had neither).
    pub faults: Option<crate::faults::Faults>,
    pub analysis: Option<Analysis>,
    pub timeline: Option<Timeline>,
    pub wraps: u64,
    /// The loop-iteration report (task 19.2; R34.9, R35.4).
    pub loops: Option<crate::loops::Report>,
    /// The masked span record (`ARCH_SPAN`, 11.13): the longest stretch
    /// in cycles during which a Kernel-level interrupt could not be taken,
    /// the spans counted, and the entry kind that closed the longest.
    pub span: Option<Span>,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Span {
    pub max_cycles: u32,
    pub count: u32,
    pub kind: u8,
    /// Spans by binary order of magnitude: bucket b holds those of
    /// 2^(b+6) to 2^(b+7)-1 cycles (the first everything below 2^7, the
    /// last everything from 2^21 on).
    pub histogram: Vec<u32>,
    /// Per closing entry kind (`stats::ENTRY_NAMES`): count and maximum.
    pub by_kind: Vec<(u32, u32)>,
}

impl Span {
    /// The histogram and the closing kinds as a table (cycles at `cpu_hz`).
    pub fn render(&self, cpu_hz: u64) -> String {
        let us = |c: u32| c as f64 * 1e6 / cpu_hz as f64;
        let mut s = String::from("| Span (cycles) | Spans |\n|---|---|\n");
        for (b, n) in self.histogram.iter().enumerate() {
            if *n == 0 {
                continue;
            }
            let lo: u64 = if b == 0 { 0 } else { 1u64 << (b + 6) };
            let label = if b + 1 == self.histogram.len() { format!("{lo} and above ({:.0} µs and above)", us(lo as u32)) } else { format!("{lo} to {} ({:.1} to {:.1} µs)", (1u64 << (b + 7)) - 1, us(lo as u32), us(((1u64 << (b + 7)) - 1) as u32)) };
            s.push_str(&format!("| {label} | {n} |\n"));
        }
        s.push_str("\n| Closing entry | Spans | Longest (cycles / µs) |\n|---|---|---|\n");
        for (k, (n, max)) in self.by_kind.iter().enumerate() {
            if *n == 0 {
                continue;
            }
            s.push_str(&format!("| {} | {n} | {max} / {:.1} |\n", crate::stats::ENTRY_NAMES.get(k).copied().unwrap_or("?"), us(*max)));
        }
        s
    }
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }

    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
    }
}

pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    let _ = std::fs::create_dir_all(inputs.work_dir);
    let scratch = inputs.work_dir.join("readback.bin");
    let stimulus = Target { chip: &rig.chip, serial: &rig.stimulus.serial };
    if inputs.flash_stimulus {
        match stimulus.flash(inputs.stimulus_elf).and_then(|f| stimulus.reset().map(|r| (f, r))) {
            Ok((f, r)) => {
                out.commands.push(f.command);
                out.commands.push(r.command);
                out.step("stimulus running", true, format!("flashed in {:.1} s and reset", f.seconds));
            }
            Err(e) => {
                out.step("stimulus running", false, e);
                return out;
            }
        }
        std::thread::sleep(Duration::from_millis(rig.expect.period_ms * 3));
    }
    let timeout = inputs.duration_secs + inputs.grace_secs;
    let analyzer = crate::config::Analyzer { samplerate_hz: inputs.samplerate_hz, ..rig.analyzer.clone() };
    let capture_path = inputs.work_dir.join("capture.bin");
    let cap = match capture::start(&analyzer, (timeout + 2) * 1000, &capture_path) {
        Ok(c) => c,
        Err(e) => {
            out.step("capture", false, e);
            return out;
        }
    };
    out.commands.push(cap.command.clone());
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    let run = target.run(inputs.target_elf, timeout);
    out.commands.push(run.command.clone());
    let samples = cap.stop();
    let pulses = run.value("pulse");
    let run_ok = run.ok;
    let detail = if run_ok {
        format!("RSK-OK after {:.0} s (operating duration {} s)", run.seconds, inputs.duration_secs)
    } else if run.timed_out {
        format!("no exit within {timeout} s")
    } else {
        format!("the run ended without RSK-OK after {:.0} s (console attached)", run.seconds)
    };
    // The EasyDMA traffic scenario of the measurement configurations (task
    // 19.2; R17.4, R34.5): the safe state counts the Jobs that found a
    // transfer ended since the last (`dma_end`, one per 100 ms Job while
    // SPIM0 chains 64-byte transfers at 8 Mbps) and the restarts after
    // which a transfer still ran once the Kernel had stopped the
    // peripheral (`dma_after_stop`, R17.6).
    if let Some(ended) = run.value("dma_end") {
        let after_stop = run.value("dma_after_stop").unwrap_or(0);
        let jobs = run.value("sampler").unwrap_or(0);
        let end_set = run.value("spim0_events_end").unwrap_or(0) == 1;
        // The counters are the last incarnation's of part2 (a restart
        // re-initializes them and the traffic restarts with its first
        // sampler Job): every Job after the first found a transfer ended,
        // and a transfer had ended at the end unless no Job counted one.
        let ok = after_stop == 0 && ended + 1 >= jobs && (ended > 0 || end_set || jobs <= 1);
        out.step(
            "EasyDMA traffic (R17.4)",
            ok,
            format!(
                "SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before {ended} of {jobs} sampler Jobs; transfers still running after the Kernel's stop at a restart: {after_stop}; ENABLE {}, SHORTS {:#x}, RXD.MAXCNT {}, TXD.MAXCNT {}, END {}, STOPPED {} at the end",
                run.value("spim0_enable").unwrap_or(0),
                run.value("spim0_shorts").unwrap_or(0),
                run.value("spim0_rxd_maxcnt").unwrap_or(0),
                run.value("spim0_txd_maxcnt").unwrap_or(0),
                run.value("spim0_events_end").unwrap_or(0),
                run.value("spim0_events_stopped").unwrap_or(0),
            ),
        );
    }
    // The loop-iteration report (task 19.2; R34.9, R35.4).
    if let Some(export) = inputs.loop_bounds {
        let value = |key: &str| run.value(key).map(|v| v as u64);
        match crate::loops::judge_file(export, &value, &rig.loops, inputs.configuration, inputs.binary_hash) {
            Ok(report) => {
                out.step("loop iterations (R34.9, R35.4)", report.ok(), report.summary());
                out.loops = Some(report);
            }
            Err(e) => out.step("loop iterations (R34.9, R35.4)", false, e),
        }
    }
    out.run = Some(run);
    out.step("target run", run_ok, detail);

    // The record of R9.10.
    match target.read_stats(inputs.target_elf, &scratch) {
        Ok(Some(st)) => {
            let max = st.max_cycles();
            let ok = st.nominal_mismatches == 0 && (max <= rig.expect.j_rel_cycles || !inputs.timing_bounds) && st.total() > 0;
            out.step(
                "release timing (R9.10)",
                ok,
                format!(
                    "{} periodic Releases recorded over {} Tasks; every nominal instant O + k*T: {}; lag max {} cycles ({:.1} µs) against J_rel {} cycles",
                    st.total(),
                    st.tasks.iter().filter(|t| t.count > 0).count(),
                    if st.nominal_mismatches == 0 { "yes" } else { "NO" },
                    max,
                    max as f64 * 1e6 / inputs.cpu_hz as f64,
                    rig.expect.j_rel_cycles
                ),
            );
            if let Some(spec) = &inputs.stress {
                // R18.8: every completed Job of the protected Partition
                // within the WCRT the Analyzer computed for its Task.
                let cycles_per_tick = inputs.cpu_hz / inputs.tick_hz;
                let mut all_ok = true;
                let mut parts = Vec::new();
                for (t, name, wcrt) in &spec.protected {
                    let ts = st.tasks.get(*t as usize);
                    let (jobs, max_ticks) = ts.map(|x| (x.resp_count, x.resp_max_ticks as u64)).unwrap_or((0, 0));
                    let max_cycles = max_ticks * cycles_per_tick;
                    let ok = jobs > 0 && max_cycles <= *wcrt;
                    all_ok &= ok;
                    parts.push(format!(
                        "{name}: {jobs} Jobs, response at most {:.3} ms ({} cycles) against WCRT {:.3} ms ({wcrt} cycles){}",
                        max_ticks as f64 * 1000.0 / inputs.tick_hz as f64,
                        max_cycles,
                        *wcrt as f64 * 1000.0 / inputs.cpu_hz as f64,
                        if ok { "" } else { " EXCEEDED" }
                    ));
                }
                out.step("WCRT of the protected Partition (R18.8)", all_ok, parts.join("; "));
                // R19.3: the stormed source costs one intake per MIT window
                // (DD-06), whatever its event rate: the intake count over
                // the run is at most ceil(t / MIT) plus the first.
                let windows = inputs.duration_secs * 1000 / spec.mit_ms + 1;
                let ts = st.tasks.get(spec.storm_task as usize);
                let (n, max, sum) = ts.map(|x| (x.intake_count as u64, x.intake_max_cycles, x.intake_sum_cycles)).unwrap_or((0, 0, 0));
                // R13.4: no observed run of the handler above the exported
                // bound of Δ_irq (the rig's `delta_irq_cycles`).
                let within_bound = max <= rig.expect.delta_irq_cycles;
                let ok = n > 0 && n <= windows + 1 && within_bound;
                out.step(
                    "interrupt storm (R19.3)",
                    ok,
                    format!(
                        "Task {}'s source stormed for {} s: {n} intakes against at most {windows} MIT windows of {} ms ({:.1} % of the windows); intake handler at most {max} cycles ({:.1} µs), mean {:.0} cycles: the measured Δ_irq{}; processor time {:.4} % of the run",
                        spec.storm_task,
                        inputs.duration_secs,
                        spec.mit_ms,
                        n as f64 * 100.0 / windows as f64,
                        max as f64 * 1e6 / inputs.cpu_hz as f64,
                        if n == 0 { 0.0 } else { sum as f64 / n as f64 },
                        if within_bound { format!(" (bound {} cycles, R13.4)", rig.expect.delta_irq_cycles) } else { format!(" ABOVE THE EXPORTED BOUND of {} cycles (R13.4)", rig.expect.delta_irq_cycles) },
                        sum as f64 * 100.0 / (inputs.duration_secs as f64 * inputs.cpu_hz as f64)
                    ),
                );
            }
            out.stats = Some(st);
        }
        Ok(None) => out.step("release timing (R9.10)", false, "the build has no release-timing record (feature `trace-ram`)"),
        Err(e) => out.step("release timing (R9.10)", false, e),
    }

    // The event mirror: every Health_Monitor event of the run (the log
    // holds 32; the late Jobs alone push hundreds over the hour), and the
    // trace freeze at the first one outside the run's expected set.
    match target.read_events(inputs.target_elf, &scratch) {
        Ok(Some(ev)) => {
            let unexpected = ev.unexpected();
            let mut detail = ev.summary(inputs.tick_hz);
            if !unexpected.is_empty() {
                detail.push_str("; unexpected: ");
                let shown: Vec<String> = unexpected
                    .iter()
                    .take(6)
                    .map(|e| format!("{} of Task {} at {:.3} ms (detail {:#x})", crate::events::kind_name(e.kind), e.task, e.instant as f64 * 1000.0 / inputs.tick_hz as f64, e.detail))
                    .collect();
                detail.push_str(&shown.join(", "));
            }
            let protected_events: Vec<String> = match &inputs.stress {
                Some(st) => ev
                    .run_entries()
                    .iter()
                    .filter(|e| st.protected.iter().any(|(t, _, _)| *t == e.task))
                    .take(6)
                    .map(|e| format!("{} of Task {} at {:.3} ms", crate::events::kind_name(e.kind), e.task, e.instant as f64 * 1000.0 / inputs.tick_hz as f64))
                    .collect(),
                None => Vec::new(),
            };
            if !protected_events.is_empty() {
                detail.push_str("; events of the protected Partition's Tasks: ");
                detail.push_str(&protected_events.join(", "));
            }
            out.step("Kernel events", unexpected.is_empty() && ev.frozen_in_run().is_none() && protected_events.is_empty(), detail);
            out.events = Some(ev);
        }
        Ok(None) => out.step("Kernel events", true, "the build has no event mirror"),
        Err(e) => out.step("Kernel events", false, e),
    }
    // The fault record: each MemManage, BusFault, or UsageFault the Kernel
    // took, with its CFSR and address, for the diagnosis of a fault event
    // (R14.3); informational, judged by the events above.
    match target.read_words(inputs.target_elf, "FAULT_RING", &scratch) {
        Ok(Some(w)) => match crate::faults::Faults::decode(&w) {
            Ok(f) => out.faults = Some(f),
            Err(e) => out.step("fault record", false, e),
        },
        Ok(None) => {}
        Err(e) => out.step("fault record", false, e),
    }

    // The trace: the storm check and the counter wraps (the time base is
    // 32 bits wide at `tick_hz`; a run longer than its period wraps).
    match target.read_trace(inputs.target_elf, &scratch) {
        Ok(Some(entries)) => {
            let storm = crate::trace::longest_storm(&entries);
            let storm_ok = storm.is_none_or(|(_, n, _)| n < 32);
            let wrap_period_s = (1u64 << 32) as f64 / inputs.tick_hz as f64;
            out.wraps = ((inputs.duration_secs as f64) / wrap_period_s) as u64;
            let last = entries.iter().rev().find(|e| e.tag == "tick").map(|e| e.ticks).unwrap_or(0);
            let seen_wraps = last >> 32;
            let wrap_ok = out.wraps == 0 || seen_wraps >= 1;
            out.step(
                "counter wraps",
                wrap_ok,
                format!(
                    "the 32-bit time base wraps every {wrap_period_s:.0} s: {} wraps in the run; the trace's last tick carries high word {seen_wraps} (instants ordered across the wrap, R9.5)",
                    out.wraps
                ),
            );
            out.step(
                "Kernel trace",
                storm_ok,
                format!(
                    "{}{}",
                    match storm {
                        Some((v, n, _)) => format!("{} entries; longest re-entry of one vector without a dispatch: vector {v}, {n} entries", entries.len()),
                        None => format!("{} entries; no interrupt re-entered without a dispatch", entries.len()),
                    },
                    match out.events.as_ref().and_then(|e| e.frozen_in_run()) {
                        Some(seq) => format!("; the ring is frozen at event {seq} and holds the entries leading to it"),
                        None => String::new(),
                    }
                ),
            );
            out.trace = entries;
        }
        Ok(None) => out.step("Kernel trace", true, "the build has no trace ring"),
        Err(e) => out.step("Kernel trace", false, e),
    }

    // The deferrals of 5.8a (the INVPC finding of 2026-11-06): the Kernel
    // counts every dispatch vector taken at another dispatch stub's entry
    // instruction and deferred (`ARCH_WINDOWS`; the actions record keeps
    // the last 32 actions, kind 3 with the vector). A count is a handled
    // occurrence (reported, never judged); the witness build must record
    // some, and its faults, if any, the fault record shows.
    match target.read_words(inputs.target_elf, "ARCH_ACTIONS", &scratch) {
        Ok(Some(words)) if words.len() >= 2 && words[0] == 0x5253_4b41 => {
            let n = words[1] as usize;
            let held = n.min(32);
            let hits: Vec<String> = (0..held).filter(|i| words.get(2 + 2 * i) == Some(&3)).map(|i| format!("{:#x}", words.get(3 + 2 * i).copied().unwrap_or(0))).collect();
            let counted = match target.read_words(inputs.target_elf, "ARCH_WINDOWS", &scratch) {
                Ok(Some(d)) if d.len() >= 2 && d[0] == 0x5253_4b41 => Some(d[1]),
                _ => None,
            };
            let total = counted.unwrap_or(hits.len() as u32);
            let ok = total > 0 || !inputs.expect_deferrals;
            let mut vectors: Vec<String> = hits.clone();
            vectors.sort();
            vectors.dedup();
            out.step("dispatch deferrals (5.8a)", ok, format!("{} entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx){}; {} actions recorded ({held} held, {} of them deferrals{}){}", total, if counted.is_none() { " (the record's held entries only)" } else { "" }, n, hits.len(), if vectors.is_empty() { String::new() } else { format!(": vectors {}", vectors.join(", ")) }, if inputs.expect_deferrals { if ok { "; the witness build's widened entry was entered and deferred" } else { "; the witness build's widened entry was never entered" } } else { "" }));
        }
        _ => {}
    }

    // The masked span (11.13; the basis of L_kernel since 2026-11-07).
    match target.read_words(inputs.target_elf, "ARCH_SPAN", &scratch) {
        Ok(Some(w)) if w.len() >= 6 && w[0] == 0x5253_4b41 => {
            let histogram: Vec<u32> = w.get(6..22).map(|h| h.to_vec()).unwrap_or_default();
            let by_kind: Vec<(u32, u32)> = w.get(22..54).map(|k| k.chunks(2).map(|c| (c[0], c[1])).collect()).unwrap_or_default();
            let span = Span { max_cycles: w[1], count: w[2], kind: w[5] as u8, histogram, by_kind };
            out.span = Some(span.clone());
            let name = crate::stats::ENTRY_NAMES.get(span.kind as usize).copied().unwrap_or("?");
            out.step(
                "masked span (L_kernel)",
                true,
                format!("the longest stretch without a Kernel-level interrupt taken: {} cycles ({:.1} µs) over {} spans, closed by a {name} entry; against the record's L_kernel {} cycles", span.max_cycles, span.max_cycles as f64 * 1e6 / inputs.cpu_hz as f64, span.count, rig.expect.l_kernel_cycles),
            );
        }
        _ => {}
    }

    // The analyzer over the whole run.
    match samples {
        Ok(samples) => {
            // Bursts a second apart: flashing and the reset on detach
            // are seconds; a late Job delays answers by tens of milliseconds.
            let gap = inputs.samplerate_hz as usize;
            let a = capture::analyze(&samples, inputs.samplerate_hz, rig.analyzer.stimulus_channel, rig.analyzer.response_channel, pulses.map(|p| p as usize), gap);
            let late = a.late(rig.expect.response_deadline_ms as f64 * 1000.0);
            if let Some(spec) = &inputs.stress {
                // Under the storm the mirror is quiet and the response line
                // toggles once per accepted Release: one per MIT window.
                let windows = inputs.duration_secs * 1000 / spec.mit_ms + 1;
                let edges = a.response_edges.saturating_sub(a.responses_before_run + a.responses_after_run);
                out.step(
                    "responses under the storm",
                    edges as u64 + 2 >= windows * 9 / 10 && edges as u64 <= windows + 2,
                    format!("{edges} response edges inside the run (target counted {}) against {windows} MIT windows; {} stimulus rises on the mirror", pulses.map(|p| p.to_string()).unwrap_or("?".into()), a.stimulus_rises),
                );
            } else {
            // A late answer on the wire: the Kernel's deadline runs from its
            // timestamp of the event (R9.6), the wire's from the edge, so an
            // answer later than the deadline by at most the stamp lag
            // (L_kernel's masking plus Δ_irq) with no deadline miss logged
            // by the Kernel is the stamp lag, not a miss (task 19.2); what
            // executed inside each late window is listed from the trace pins.
            // The exported Δ_stamp bound when the rig has one, else L_kernel
            // plus Δ_irq (its parts); the run's own largest lag is checked
            // against the bound too (R13.4).
            let stamp_bound = if rig.expect.delta_stamp_cycles > 0 { rig.expect.delta_stamp_cycles as u64 } else { rig.expect.l_kernel_cycles as u64 + rig.expect.delta_irq_cycles as u64 };
            let stamp_us = stamp_bound as f64 * 1e6 / inputs.cpu_hz as f64;
            let observed_stamp = out.stats.as_ref().map(|st| st.tasks.iter().map(|t| t.stamp_max_ticks as u64 * (inputs.cpu_hz / inputs.tick_hz)).max().unwrap_or(0)).unwrap_or(0);
            let stamp_within = rig.expect.delta_stamp_cycles == 0 || observed_stamp <= rig.expect.delta_stamp_cycles as u64;
            let deadline_us = rig.expect.response_deadline_ms as f64 * 1000.0;
            let kernel_misses = out.events.as_ref().map(|ev| ev.counts().iter().filter(|((k, t), _)| crate::events::kind_name(*k) == "deadline_miss" && *t == 4).map(|(_, n)| *n).sum::<usize>()).unwrap_or(0);
            let mut late_text = Vec::new();
            let mut late_within_stamp = true;
            for (s, r, l) in a.late_windows(deadline_us) {
                late_within_stamp &= l <= deadline_us + stamp_us;
                let what = if rig.analyzer.task_id_channels.is_empty() {
                    String::new()
                } else {
                    let w = capture::timeline(&samples, &rig.analyzer.task_id_channels, rig.analyzer.kernel_channel, s, r.min(samples.len()));
                    format!(": executing samples {:?}, none {}, Kernel-active {}", &w.samples[..7], w.samples[7], w.kernel_samples)
                };
                late_text.push(format!("{:.0} µs at {:.3} s{what}", l, s as f64 / inputs.samplerate_hz as f64));
            }
            let late_ok = late == 0 || (kernel_misses == 0 && late_within_stamp) || !inputs.timing_bounds;
            out.step(
                "loopback over the run",
                a.matched() >= rig.expect.min_matched && late_ok && a.unanswered_in_window == 0 && (stamp_within || !inputs.timing_bounds),
                format!(
                    "{} stimulus pulses answered (target counted {}; {} rises and {} response edges in the capture); latency {} to {} µs, median {} µs, {late} later than {} ms{}; {} unanswered inside the run{}",
                    a.matched(),
                    pulses.map(|p| p.to_string()).unwrap_or("?".into()),
                    a.stimulus_rises,
                    a.response_edges,
                    a.latency_min_us().map(|v| format!("{v:.0}")).unwrap_or("-".into()),
                    a.latency_max_us().map(|v| format!("{v:.0}")).unwrap_or("-".into()),
                    a.latency_median_us().map(|v| format!("{v:.0}")).unwrap_or("-".into()),
                    rig.expect.response_deadline_ms,
                    format!("{}; Δ_stamp observed {observed_stamp} cycles ({:.1} µs) against the exported bound {}", if late == 0 { String::new() } else { format!(" ({}; the Kernel logged {kernel_misses} deadline misses of Task 4; the stamp lag allowed beyond the deadline {stamp_us:.0} µs: {})", late_text.join(", "), if late_ok { "within it, the stamp lag and not a miss" } else { "EXCEEDED" }) }, observed_stamp as f64 * 1e6 / inputs.cpu_hz as f64, if rig.expect.delta_stamp_cycles > 0 { format!("{} cycles{}", rig.expect.delta_stamp_cycles, if stamp_within { "" } else { " EXCEEDED" }) } else { "(none recorded)".to_string() }),
                    a.unanswered_in_window,
                    if a.unanswered_at_s.is_empty() {
                        String::new()
                    } else {
                        let run0 = a.window_s.map(|(f, _)| f).unwrap_or(0.0);
                        format!(" (at {} s into the run)", a.unanswered_at_s.iter().map(|t| format!("{:.3}", t - run0)).collect::<Vec<_>>().join(", "))
                    }
                ),
            );
            }
            if !rig.analyzer.task_id_channels.is_empty() {
                let (from, to) = a.window_s.map(|(f, t)| ((f * inputs.samplerate_hz as f64) as usize, (t * inputs.samplerate_hz as f64) as usize)).unwrap_or((0, samples.len()));
                let tl = capture::timeline(&samples, &rig.analyzer.task_id_channels, rig.analyzer.kernel_channel, from, to);
                let busy: usize = tl.samples[..7].iter().sum();
                let total = (to - from).max(1);
                out.step(
                    "dispatch timeline",
                    true,
                    format!(
                        "Tasks executing {:.1} % of the run, the Kernel {:.1} % over {} entries; runs per Task {:?}",
                        busy as f64 * 100.0 / total as f64,
                        tl.kernel_samples as f64 * 100.0 / total as f64,
                        tl.kernel_entries,
                        &tl.runs[..7]
                    ),
                );
                // Task 19.1: the agent (the Budget accounting's largest Job
                // execution of Task 0, the highest Priority: nothing but the
                // Kernel's entries interrupts its runs) against the trace pin's
                // longest uninterrupted run, which includes those entries.
                if let Some(st) = &out.stats {
                    if let Some((agent, trace)) = crate::wcet_records::against_trace(st, tl.longest_run[0], inputs.samplerate_hz, inputs.cpu_hz, 0) {
                        let sample = inputs.cpu_hz / inputs.samplerate_hz as u64;
                        // Two samples of quantization, and a tenth for the
                        // stub's cycles the account carries at the Job's
                        // ends while the pins still show the previous state
                        // (0.4 % over the trace before the EasyDMA traffic
                        // scenario, 7.9 % under it on 2026-11-06: an
                        // observation of section 11.13).
                        let ok = (agent as u64) <= trace + trace / 10 + 2 * sample;
                        let jobs = st.tasks.first().map(|t| t.exec_count).unwrap_or(0);
                        out.step(
                            "agent against the trace (task 19.1)",
                            ok,
                            format!(
                                "Task 0: the Budget accounting's largest Job execution {agent} cycles ({:.3} ms, cycle-counter read cost {} cycles) against the trace pin's longest run {trace} cycles ({:.3} ms at {} samples/s; {} runs for {jobs} Jobs){}",
                                agent as f64 * 1000.0 / inputs.cpu_hz as f64,
                                st.calibration_cycles,
                                trace as f64 * 1000.0 / inputs.cpu_hz as f64,
                                inputs.samplerate_hz,
                                tl.runs[0],
                                if ok { if (agent as u64) > trace + 2 * sample { ": the agent exceeds the trace beyond quantization, within a tenth" } else { "" } } else { ": the agent exceeds the trace by more than a tenth" }
                            ),
                        );
                    }
                }
                out.timeline = Some(tl);
            }
            out.analysis = Some(a);
        }
        Err(e) => out.step("capture", false, e),
    }
    out
}

/// The interference scenarios of the run as the records state them
/// (R33.1, R34.5): the configuration and its duration first (the records
/// fold on it), then the EasyDMA traffic (R17.4) as the run observed it,
/// the floating-point context preservation triggered and not triggered,
/// and the preemption rate.
pub fn scenarios(inputs: &Inputs<'_>, out: &Outcome) -> Vec<String> {
    let mut v = vec![format!("{} ({} s)", inputs.configuration, inputs.duration_secs)];
    let run = out.run.as_ref();
    match run.and_then(|r| r.value("dma_end")) {
        Some(ended) => v.push(format!(
            "worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut ({ended} transfer-ended observations, {} still running after a stop)",
            run.and_then(|r| r.value("dma_after_stop")).unwrap_or(0)
        )),
        None => v.push("EasyDMA traffic: none (the build has no traffic scenario)".to_string()),
    }
    v.push("floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs".to_string());
    v.push(format!(
        "preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT{}",
        if inputs.stress.is_some() { ", the stormed source at the stimulus board's highest rate" } else { "" }
    ));
    v
}

/// The report (Markdown).
pub fn render(rig: &Rig, inputs: &Inputs<'_>, out: &Outcome) -> String {
    let mut s = String::new();
    if inputs.stress.is_some() {
        s.push_str("# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)\n\n");
    } else {
        s.push_str("# HIL release-timing run (task 7.3, R9.10)\n\n");
    }
    s.push_str(&format!("Verdict: **{}**\n\n", if out.ok() { "pass" } else { "FAIL" }));
    s.push_str(&format!(
        "Operating duration {} s on the target {} with the stimulus board {}; analyzer {} at {} Hz; J_rel bound {} cycles.\n\n",
        inputs.duration_secs, rig.target.serial, rig.stimulus.serial, rig.analyzer.driver, inputs.samplerate_hz, rig.expect.j_rel_cycles
    ));
    s.push_str("## Steps\n\n| Step | Result | Detail |\n|---|---|---|\n");
    for st in &out.steps {
        s.push_str(&format!("| {} | {} | {} |\n", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|")));
    }
    if let Some(l) = &out.loops {
        s.push_str("\n## Loop iterations (R34.9, R35.4)\n\n");
        s.push_str(&l.render());
    }
    if let Some(st) = &out.stats {
        s.push_str("\n## The Kernel's release-timing record\n\n");
        s.push_str(&st.render(inputs.cpu_hz, inputs.tick_hz));
    }
    if let Some(sp) = &out.span {
        s.push_str("\n## The Kernel's masked spans (11.13)\n\n");
        s.push_str(&format!("Every stretch without a Kernel-level interrupt taken, {} of them, the longest {} cycles ({:.1} µs), closed by a {} entry:\n\n", sp.count, sp.max_cycles, sp.max_cycles as f64 * 1e6 / inputs.cpu_hz as f64, crate::stats::ENTRY_NAMES.get(sp.kind as usize).copied().unwrap_or("?")));
        s.push_str(&sp.render(inputs.cpu_hz));
    }
    if let Some(f) = &out.faults {
        if f.written > 0 {
            s.push_str("\n## The Kernel's fault record\n\n");
            s.push_str("Each fault the Kernel took (CFSR, and MMFAR, BFAR, or the stacked PC), oldest first:\n\n```\n");
            s.push_str(&f.render());
            s.push_str("```\n");
        }
    }
    if let Some(ev) = &out.events {
        s.push_str("\n## The Kernel's events\n\n");
        s.push_str(&ev.summary(inputs.tick_hz));
        s.push('\n');
        let unexpected = ev.unexpected();
        if !unexpected.is_empty() {
            s.push_str("\n| Seq | Instant (ms) | Kind | Task | Partition | Detail |\n|---|---|---|---|---|---|\n");
            for e in unexpected.iter().take(32) {
                s.push_str(&format!("| {} | {:.3} | {} | {} | {} | {:#x} |\n", e.seq, e.instant as f64 * 1000.0 / inputs.tick_hz as f64, crate::events::kind_name(e.kind), e.task, e.partition, e.detail));
            }
        }
    }
    if let Some(tl) = &out.timeline {
        s.push_str("\n## Dispatch timeline (trace pins)\n\n| Task | Executing (samples) | Runs | Longest run (ms) |\n|---|---|---|---|\n");
        for t in 0..8 {
            if tl.samples[t] > 0 {
                s.push_str(&format!("| {} | {} | {} | {:.3} |\n", if t == 7 { "none".to_string() } else { t.to_string() }, tl.samples[t], tl.runs[t], tl.longest_run[t] as f64 * 1000.0 / inputs.samplerate_hz as f64));
            }
        }
    }
    if !out.commands.is_empty() {
        s.push_str("\n## Commands\n\n");
        for c in &out.commands {
            s.push_str(&format!("    {c}\n"));
        }
    }
    if let Some(run) = &out.run {
        s.push_str("\n## Target console\n\n```\n");
        s.push_str(run.log.trim_end());
        s.push_str("\n```\n");
    }
    s
}
