//! The response suite (task 7.5, R14.11): the Application in its response
//! configuration (`nrf52840_response`, 2 seconds) injects, at fixed
//! instants, a fault that triggers each response of the closed sets of
//! R14.5 in a Partition declaring it; the rig reads the Kernel's event
//! mirror, the counts, and the capture afterwards and passes the run only
//! when the events are exactly the expected ones, in order and in their
//! windows, the effects on the faulting Partition are the declared ones
//! (a restarted Partition's state back at its initial values, a stopped
//! Partition's Tasks silent), and every other Partition ran unchanged
//! (R14.9: part0's counts exact to the instant of its own safe state,
//! part1's `pulse` answering throughout, before and after its restart
//! and after part2's stop).
//!
//! The schedule (milliseconds of the time base; Tasks control 0, echo 1,
//! sensor 2, log 3, pulse 4, filter 5, sampler 6; Partitions part0 (A),
//! part1 (C), part2 (B)):
//!
//! | Instant | Injection | Response | Events |
//! |---|---|---|---|
//! | 200 | control exceeds its 1 ms Budget | RECORD_AND_CONTINUE | overrun(0) |
//! | 250 | filter writes outside its view | END_JOB (fault) | memmanage(5), end_job(5) |
//! | 400 | log exceeds its 6 ms Budget (25 ms of busy wait: it is preempted by sensor's Critical_Section meanwhile) | END_JOB (Overrun) | overrun(3), end_job(3) |
//! | 500 | filter exceeds its Budget inside its Critical_Section | END_JOB escalated to RESTART_PARTITION (R18.4) | overrun(5), resource_released(5), partition_restart(part2) |
//! | 800 | log writes outside its view | RESTART_PARTITION | memmanage(3), partition_restart(part1) |
//! | 1305 | sensor runs 120 ms past its deadline | RECORD_AND_CONTINUE, RECORD_ONLY; part2 STOP_PARTITION on filter's miss; part1 RECORD_ONLY | overrun(2), deadline_miss(2, 3, 4, 5), partition_stop(part2), held and discarded Releases of 3, 4, 5, 6 |
//! | 1900 | control writes outside its view | SAFE_STATE | memmanage(0), safe_state |

use std::path::Path;
use std::time::Duration;

use crate::board::{Run, Target};
use crate::capture::{self, Analysis};
use crate::config::Rig;
use crate::events::{kind_name, Events};
use crate::release_timing::Step;
use crate::stats::Stats;

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    pub target_elf: &'a Path,
    pub stimulus_elf: &'a Path,
    pub work_dir: &'a Path,
    pub timeout_secs: u64,
    pub tick_hz: u64,
    pub samplerate_hz: u32,
    pub flash_stimulus: bool,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub run: Option<Run>,
    pub events: Option<Events>,
    pub stats: Option<Stats>,
    pub analysis: Option<Analysis>,
    pub trace_text: String,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }

    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
    }
}

const MEMMANAGE: u8 = 2;
const OVERRUN: u8 = 6;
const DEADLINE_MISS: u8 = 7;
const RELEASE_OVERLAP: u8 = 8;
const MIT_VIOLATION: u8 = 9;
const RESOURCE_RELEASED: u8 = 11;
const PARTITION_RESTART: u8 = 16;
const PARTITION_STOP: u8 = 17;
const SAFE_STATE: u8 = 18;
const END_JOB: u8 = 19;
const ANY: u8 = 0xFF;

/// One expected event: kind, Task (`ANY` for none), Partition (`ANY` for
/// the Kernel's), and the window of instants in milliseconds.
#[derive(Clone, Copy)]
pub struct Expected {
    pub kind: u8,
    pub task: u8,
    pub partition: u8,
    pub from_ms: u64,
    pub to_ms: u64,
    pub what: &'static str,
}

const fn ev(kind: u8, task: u8, partition: u8, from_ms: u64, to_ms: u64, what: &'static str) -> Expected {
    Expected { kind, task, partition, from_ms, to_ms, what }
}

/// The required events, in order.
pub const REQUIRED: &[Expected] = &[
    ev(OVERRUN, 0, 0, 200, 215, "control's Budget overrun, RECORD_AND_CONTINUE"),
    ev(MEMMANAGE, 5, 2, 250, 276, "filter's fault"),
    ev(END_JOB, 5, 2, 250, 276, "END_JOB as part2's fault response"),
    ev(OVERRUN, 3, 1, 400, 445, "log's Budget overrun"),
    ev(END_JOB, 3, 1, 400, 445, "END_JOB as part1's Overrun_Response"),
    ev(OVERRUN, 5, 2, 500, 526, "filter's Budget overrun inside its Critical_Section"),
    ev(PARTITION_RESTART, ANY, 2, 500, 526, "END_JOB escalated to RESTART_PARTITION (R18.4)"),
    ev(RESOURCE_RELEASED, 5, 2, 500, 526, "the Resource released by force (R8.9)"),
    ev(MEMMANAGE, 3, 1, 800, 845, "log's fault"),
    ev(PARTITION_RESTART, ANY, 1, 800, 845, "RESTART_PARTITION as part1's fault response"),
    // KD-R9.6 (found 2026-11-02, fixed): the running Job's own deadline
    // (1,325 ms) coincides with its Task's next nominal instant; the Kernel
    // now serves the due deadline events before the Releases of an instant,
    // so the Job's own miss is reported at 1,325 ms and the held Release's
    // (R9.7) at its deadline, 1,345 ms. Before the fix the held Release
    // re-armed the Task's single deadline slot over the unserved miss and
    // only the 1,345 ms miss was reported.
    ev(DEADLINE_MISS, 2, 0, 1320, 1335, "sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY"),
    ev(DEADLINE_MISS, 2, 0, 1340, 1350, "the deadline miss of sensor's held Release (R9.7), RECORD_ONLY"),
    ev(DEADLINE_MISS, 5, 2, 1345, 1360, "filter's deadline missed while pending (its Release of 1,325 ms)"),
    ev(PARTITION_STOP, ANY, 2, 1345, 1360, "STOP_PARTITION as part2's deadline-miss response"),
    ev(MEMMANAGE, 0, 0, 1900, 1915, "control's fault"),
    ev(SAFE_STATE, ANY, ANY, 1900, 1915, "SAFE_STATE as part0's fault response"),
];

/// Events required somewhere in their window, in no order against the
/// others: sensor's Budget overrun is charged in execution, which the
/// higher-Priority Tasks preempt, and its compare is dropped while sensor
/// is preempted (DD-10), so the overrun of the 120 ms Job of 1,305 ms is
/// recorded between 14 and 35 ms after the Job started on the bench
/// (1,319 ms and 1,338 ms observed), before or after the Job's own
/// deadline miss at 1,325 ms.
const REQUIRED_ANYWHERE: &[Expected] = &[
    ev(OVERRUN, 2, 0, 1305, 1345, "sensor's Budget overrun, RECORD_AND_CONTINUE"),
];

/// Events tolerated beside the required ones: the consequences of the
/// 120 ms Job of 1,305 ms on the lower-Priority Tasks (misses, held and
/// discarded Releases) and the two events the run ends with.
fn tolerated(e: &EventRecMs) -> bool {
    let ms = e.instant_ms;
    let hog = (1305..=1480).contains(&ms);
    match e.kind {
        DEADLINE_MISS | RELEASE_OVERLAP | MIT_VIOLATION => hog && matches!(e.task, 2 | 3 | 4 | 5 | 6),
        // RECORD_AND_CONTINUE re-arms the Budget event: the overrun is
        // recorded again as the Job runs on.
        OVERRUN => hog && e.task == 2,
        // Task 21.4 (R40.5): while sensor's 120 ms Job keeps `log` from
        // running, `control`'s telemetry sends find the channel of four
        // full from the fifth send on; each is the reported bound
        // violation of the sending Partition, by design.
        25 => hog && e.task == 0,
        23 => true, // debugger
        _ => false,
    }
}

/// The events of the run in milliseconds, with the Kernel's event index.
#[derive(Clone, Debug)]
pub struct EventRecMs {
    pub seq: u32,
    pub kind: u8,
    pub task: u8,
    pub partition: u8,
    pub instant_ms: u64,
}

impl EventRecMs {
    fn of(e: &crate::events::EventRec, tick_hz: u64) -> Self {
        EventRecMs { seq: e.seq, kind: e.kind, task: e.task, partition: e.partition, instant_ms: e.instant * 1000 / tick_hz }
    }
}

/// Judges the events of a run: every required event present in order and
/// in its window, nothing else but tolerated events.
pub fn judge_events(events: &[EventRecMs]) -> (bool, Vec<String>) {
    let mut notes = Vec::new();
    let mut ok = true;
    let mut pos = 0usize;
    for want in REQUIRED {
        let found = events[pos..].iter().position(|e| {
            e.kind == want.kind && (want.task == ANY || e.task == want.task) && (want.partition == ANY || e.partition == want.partition) && e.instant_ms >= want.from_ms && e.instant_ms <= want.to_ms
        });
        match found {
            Some(i) => {
                let e = &events[pos + i];
                notes.push(format!("{} at {} ms: {}", kind_name(want.kind), e.instant_ms, want.what));
                pos += i + 1;
            }
            None => {
                ok = false;
                notes.push(format!("MISSING {} of Task {} / Partition {} in [{}, {}] ms: {}", kind_name(want.kind), want.task, want.partition, want.from_ms, want.to_ms, want.what));
            }
        }
    }
    let matched: Vec<&EventRecMs> = {
        // The required ones again, to list the extras.
        let mut m = Vec::new();
        let mut pos = 0usize;
        for want in REQUIRED {
            if let Some(i) = events[pos..].iter().position(|e| {
                e.kind == want.kind && (want.task == ANY || e.task == want.task) && (want.partition == ANY || e.partition == want.partition) && e.instant_ms >= want.from_ms && e.instant_ms <= want.to_ms
            }) {
                m.push(&events[pos + i]);
                pos += i + 1;
            }
        }
        m
    };
    for want in REQUIRED_ANYWHERE {
        match events.iter().find(|e| {
            e.kind == want.kind && (want.task == ANY || e.task == want.task) && (want.partition == ANY || e.partition == want.partition) && e.instant_ms >= want.from_ms && e.instant_ms <= want.to_ms
        }) {
            Some(e) => notes.push(format!("{} at {} ms: {}", kind_name(want.kind), e.instant_ms, want.what)),
            None => {
                ok = false;
                notes.push(format!("MISSING {} of Task {} / Partition {} in [{}, {}] ms: {}", kind_name(want.kind), want.task, want.partition, want.from_ms, want.to_ms, want.what));
            }
        }
    }
    for e in events {
        if matched.iter().any(|m| m.seq == e.seq) || tolerated(e) {
            continue;
        }
        ok = false;
        notes.push(format!("UNEXPECTED {} of Task {} / Partition {} at {} ms", kind_name(e.kind), e.task, e.partition, e.instant_ms));
    }
    (ok, notes)
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
    let capture_path = inputs.work_dir.join("capture.bin");
    let cap = match capture::start(&rig.analyzer_at(inputs.samplerate_hz), (inputs.timeout_secs + 2) * 1000, &capture_path) {
        Ok(c) => c,
        Err(e) => {
            out.step("capture", false, e);
            return out;
        }
    };
    out.commands.push(cap.command.clone());
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    let run = target.run(inputs.target_elf, inputs.timeout_secs);
    out.commands.push(run.command.clone());
    let samples = cap.stop();
    let counts = |k: &str| run.value(k);
    let run_ok = run.ok;
    out.step(
        "target run",
        run_ok,
        if run_ok { format!("the report ran after {:.1} s (the safe state of 1.9 s, by design)", run.seconds) } else if run.timed_out { "no report within the timeout".to_string() } else { "the run ended without a report".to_string() },
    );
    let (control, echo, sensor, log, pulse, filter, sampler) = (counts("control"), counts("echo"), counts("sensor"), counts("log"), counts("pulse"), counts("filter"), counts("sampler"));
    let (telemetry_peer, pingpong_peer, pingpong_rejected, pingpong_reps) = (counts("telemetry_peer"), counts("pingpong_peer"), counts("pingpong_rejected"), counts("pingpong_reps"));
    out.run = Some(run);

    // The events.
    match target.read_events(inputs.target_elf, &scratch) {
        Ok(Some(ev)) => {
            let events: Vec<EventRecMs> = ev.run_entries().iter().map(|e| EventRecMs::of(e, inputs.tick_hz)).collect();
            let (ok, notes) = judge_events(&events);
            out.step("responses (R14.5, R14.11)", ok, format!("{} events; {}", events.len(), notes.join(" | ")));
            out.events = Some(ev);
        }
        Ok(None) => out.step("responses (R14.5, R14.11)", false, "the build has no event mirror"),
        Err(e) => out.step("responses (R14.5, R14.11)", false, e),
    }

    // The counts: the effects on the faulting Partitions and R14.9.
    let within = |v: Option<u32>, lo: u32, hi: u32| v.is_some_and(|v| v >= lo && v <= hi);
    // part0 to its own safe state at 1,900 ms: control 190 Jobs (the
    // 190th faults before it counts), echo one per three control Jobs,
    // sensor 95 (the 120 ms Job completes under RECORD_AND_CONTINUE).
    let part0_ok = within(control, 188, 190) && within(echo, 60, 64) && within(sensor, 89, 91);
    out.step(
        "part0 unchanged by the other Partitions' responses (R14.9)",
        part0_ok,
        format!("control {} (189 or 190 expected), echo {} (about 63), sensor {} (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s", fmt(control), fmt(echo), fmt(sensor)),
    );
    // part1 restarted at 800 ms: its counts are those since the restart
    // (log every 40 ms to 1.9 s, 27 Jobs less the two the 120 ms Job
    // held and discarded; pulse every 18 ms, about 61 less the hog's).
    let part1_ok = within(log, 22, 28) && within(pulse, 50, 62);
    out.step(
        "part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5)",
        part1_ok,
        format!("log {} (about 25 since the restart; 47 had it kept its state), pulse {} (about 56 since the restart)", fmt(log), fmt(pulse)),
    );
    // Task 21.4 (R42.3; the HIL Partition-restart case): part1's restart
    // re-initializes its Endpoint roles, and `control`'s next operation on
    // each of its two Endpoints to part1 reports the peer-state change,
    // once; the PingPong rounds go on after it (one per `log` Job before
    // and after the restart); no reply was rejected (R42.4).
    let peer_ok = within(telemetry_peer, 1, 1) && within(pingpong_peer, 1, 1) && within(pingpong_rejected, 0, 0) && within(pingpong_reps, 30, 47);
    out.step(
        "peer-state errors after part1's restart (R42.3, task 21.4)",
        peer_ok,
        format!("telemetry_peer {} (1 expected), pingpong_peer {} (1 expected), pingpong_rejected {} (0 expected), pingpong_reps {} (about 45: one round per log Job but the restart's)", fmt(telemetry_peer), fmt(pingpong_peer), fmt(pingpong_rejected), fmt(pingpong_reps)),
    );
    // part2 restarted at 500 ms and stopped at 1,325 ms: filter 33 Jobs
    // between (less the one that faulted at... none after 500), sampler 8.
    let part2_ok = within(filter, 30, 34) && within(sampler, 7, 9);
    out.step(
        "part2 restarted at 500 ms and stopped at 1,325 ms (R14.5)",
        part2_ok,
        format!("filter {} (about 32 between the restart and the stop; 76 had it run to the end), sampler {} (about 8)", fmt(filter), fmt(sampler)),
    );

    // The record: Jobs completed per Task (part2's none after the stop).
    match target.read_stats(inputs.target_elf, &scratch) {
        Ok(st) => out.stats = st,
        Err(e) => out.step("record", false, e),
    }
    match target.read_trace(inputs.target_elf, &scratch) {
        Ok(Some(entries)) => out.trace_text = crate::trace::render(&entries, inputs.tick_hz),
        _ => {}
    }

    // The capture: pulse answered before part1's restart, after it, and
    // after part2's stop (R14.9 on the one Task the analyzer can see).
    match samples {
        Ok(samples) => {
            // Every response edge (no windowing on the count: part1's restart
            // splits the run's responses in two); the phases are relative to
            // the first answered pulse, within a period of instant 0.
            let a = capture::analyze(&samples, inputs.samplerate_hz, rig.analyzer.stimulus_channel, rig.analyzer.response_channel, None, inputs.samplerate_hz as usize);
            let _ = pulse;
            let run0 = a.window_s.map(|(f, _)| f).unwrap_or(0.0);
            let count_in = |from_s: f64, to_s: f64| a.latencies_us.iter().filter(|(r, _)| { let t = *r as f64 / inputs.samplerate_hz as f64 - run0; t >= from_s && t < to_s }).count();
            let before_restart = count_in(0.0, 0.78);
            let between = count_in(0.86, 1.28);
            let after_stop = count_in(1.45, 1.88);
            let ok = before_restart >= 38 && between >= 20 && after_stop >= 20;
            out.step(
                "pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop)",
                ok,
                format!("{} answered pulses before part1's restart (of 43), {} between the restart and sensor's 120 ms Job (of 23), {} after part2's stop (of 23); {} matched in all, latency {} to {} µs", before_restart, between, after_stop, a.matched(), a.latency_min_us().map(|v| format!("{v:.0}")).unwrap_or("-".into()), a.latency_max_us().map(|v| format!("{v:.0}")).unwrap_or("-".into())),
            );
            out.analysis = Some(a);
        }
        Err(e) => out.step("capture", false, e),
    }
    out
}

fn fmt(v: Option<u32>) -> String {
    v.map(|v| v.to_string()).unwrap_or("?".into())
}

/// The report (Markdown).
pub fn render(rig: &Rig, inputs: &Inputs<'_>, out: &Outcome) -> String {
    let mut s = String::new();
    s.push_str("# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)\n\n");
    s.push_str(&format!("Verdict: **{}**\n\n", if out.ok() { "pass" } else { "FAIL" }));
    s.push_str(&format!("Target {} with the stimulus board {}; analyzer {} at {} Hz.\n\n", rig.target.serial, rig.stimulus.serial, rig.analyzer.driver, inputs.samplerate_hz));
    s.push_str("## Steps\n\n| Step | Result | Detail |\n|---|---|---|\n");
    for st in &out.steps {
        s.push_str(&format!("| {} | {} | {} |\n", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|")));
    }
    if let Some(ev) = &out.events {
        s.push_str("\n## The Kernel's events\n\n| Seq | Instant (ms) | Kind | Task | Partition | Detail |\n|---|---|---|---|---|---|\n");
        for e in ev.run_entries() {
            s.push_str(&format!("| {} | {:.3} | {} | {} | {} | {:#x} |\n", e.seq, e.instant as f64 * 1000.0 / inputs.tick_hz as f64, kind_name(e.kind), if e.task == 0xFF { "-".to_string() } else { e.task.to_string() }, if e.partition == 0xFF { "-".to_string() } else { e.partition.to_string() }, e.detail));
        }
    }
    if let Some(st) = &out.stats {
        s.push_str("\n## The Kernel's record\n\n");
        s.push_str(&st.render(64_000_000, inputs.tick_hz));
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

#[cfg(test)]
mod tests {
    use super::*;

    fn e(seq: u32, kind: u8, task: u8, partition: u8, ms: u64) -> EventRecMs {
        EventRecMs { seq, kind, task, partition, instant_ms: ms }
    }

    #[test]
    fn the_expected_sequence_passes_and_an_extra_fails() {
        let mut events = vec![e(0, 23, 0xFF, 0xFF, 0)];
        let script: &[(u8, u8, u8, u64)] = &[
            (OVERRUN, 0, 0, 201), (MEMMANAGE, 5, 2, 250), (END_JOB, 5, 2, 250), (OVERRUN, 3, 1, 406), (END_JOB, 3, 1, 406),
            (OVERRUN, 5, 2, 500), (PARTITION_RESTART, 0xFF, 2, 500), (RESOURCE_RELEASED, 5, 2, 500),
            (MEMMANAGE, 3, 1, 800), (PARTITION_RESTART, 0xFF, 1, 800),
            (OVERRUN, 2, 0, 1313), (DEADLINE_MISS, 2, 0, 1325), (DEADLINE_MISS, 2, 0, 1345), (DEADLINE_MISS, 3, 1, 1320), (DEADLINE_MISS, 5, 2, 1350), (PARTITION_STOP, 0xFF, 2, 1350), (RELEASE_OVERLAP, 3, 1, 1360), (MEMMANAGE, 0, 0, 1900), (SAFE_STATE, 0xFF, 0xFF, 1900),
        ];
        for (i, (k, t, p, ms)) in script.iter().enumerate() {
            events.push(e(i as u32 + 1, *k, *t, *p, *ms));
        }
        let (ok, notes) = judge_events(&events);
        assert!(ok, "{notes:?}");
        // sensor's overrun after its own miss (1,338 ms observed) passes too;
        // none at all fails.
        let mut late = events.clone();
        let i = late.iter().position(|e| e.kind == OVERRUN && e.task == 2).unwrap();
        late[i].instant_ms = 1338;
        late.swap(i, i + 1);
        let (ok, notes) = judge_events(&late);
        assert!(ok, "{notes:?}");
        late.remove(i + 1);
        let (ok, notes) = judge_events(&late);
        assert!(!ok);
        assert!(notes.iter().any(|n| n.starts_with("MISSING overrun of Task 2")));
        events.push(e(99, OVERRUN, 0, 0, 1000));
        let (ok, notes) = judge_events(&events);
        assert!(!ok);
        assert!(notes.iter().any(|n| n.starts_with("UNEXPECTED overrun")));
        let (ok, notes) = judge_events(&events[..5]);
        assert!(!ok);
        assert!(notes.iter().any(|n| n.starts_with("MISSING")));
    }
}
