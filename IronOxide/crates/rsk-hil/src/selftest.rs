//! The rig self-test (task 7.1's evidence, R34.8, R48.4): the boards
//! identified by serial, the analyzer seen, the stimulus image flashed and
//! read back, the stimulus board running, the target image run with the
//! console and read back, and the analyzer's capture agreeing with the
//! console: every stimulus pulse inside the run answered within the
//! deadline, as many answers as the target counted Releases.
//!
//! The run is unattended: no step waits for an operator, every command
//! has a bound, and a locked access port or an absent board is a verdict
//! of the step, not a hang.

use std::path::Path;
use std::time::Duration;

use crate::board::{Readback, Run, Target};
use crate::capture::{self, Analysis};
use crate::config::Rig;
use crate::probes;

/// What the self-test needs besides the rig.
pub struct Inputs<'a> {
    pub rig: &'a Rig,
    /// The Application under test, built for the rig (the HIL feature set).
    pub target_elf: &'a Path,
    /// The stimulus Application's binary.
    pub stimulus_elf: &'a Path,
    /// The Build_Manifest's hashes of the two, when the caller has them
    /// (R48.4 compares the read-back against the manifest).
    pub target_hash: Option<&'a str>,
    pub stimulus_hash: Option<&'a str>,
    /// Scratch space for the capture and the read-back files.
    pub work_dir: &'a Path,
    /// The bound on the target's run.
    pub timeout_secs: u64,
    /// The target's time base, for the trace's instants.
    pub tick_hz: u64,
}

/// One checked step.
#[derive(Clone, Debug)]
pub struct Step {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

/// The self-test's outcome.
#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub probe_rs: String,
    pub sigrok: String,
    pub probes: Vec<probes::Probe>,
    pub stimulus_readback: Option<Readback>,
    pub target_readback: Option<Readback>,
    pub run: Option<Run>,
    pub analysis: Option<Analysis>,
    /// The edge list of the capture (an evidence artifact).
    pub edges_text: String,
    /// The Kernel's trace of the run, rendered (an evidence artifact).
    pub trace_text: String,
    /// The target's access port is locked: inconclusive, not failed.
    pub locked: bool,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }

    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) -> bool {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
        ok
    }
}

/// Runs the self-test. It stops at the first step that cannot continue
/// (no probe, no analyzer, a flash failure) and records it.
pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    let _ = std::fs::create_dir_all(inputs.work_dir);

    // 1. The tools and the probes, by serial.
    match probes::version() {
        Ok(v) => out.probe_rs = v,
        Err(e) => {
            out.step("probe-rs", false, e);
            return out;
        }
    }
    match probes::list() {
        Ok(list) => out.probes = list,
        Err(e) => {
            out.step("probe-rs list", false, e);
            return out;
        }
    }
    out.commands.push("probe-rs list".into());
    let mut missing = Vec::new();
    let mut seen = Vec::new();
    for board in rig.boards() {
        match out.probes.iter().find(|p| p.id == board.serial) {
            Some(p) => seen.push(format!("{} = {} ({}{})", board.role, p.id, p.name, if board.wired { "" } else { ", not wired" })),
            None if board.role == "target" || board.role == "stimulus" => missing.push(format!("{} ({})", board.role, board.serial)),
            None => seen.push(format!("{} = {} (absent, not required)", board.role, board.serial)),
        }
    }
    let unknown: Vec<String> = out.probes.iter().filter(|p| rig.boards().all(|b| b.serial != p.id)).map(|p| p.id.clone()).collect();
    let mut detail = seen.join("; ");
    if !unknown.is_empty() {
        detail.push_str(&format!("; not in the rig: {}", unknown.join(", ")));
    }
    if !missing.is_empty() {
        out.step("boards by serial", false, format!("missing {}; present: {detail}", missing.join(", ")));
        return out;
    }
    out.step("boards by serial", true, detail);

    // 2. The analyzer.
    match capture::version() {
        Ok(v) => out.sigrok = v,
        Err(e) => {
            out.step("sigrok-cli", false, e);
            return out;
        }
    }
    match capture::scan(&rig.analyzer.driver) {
        Ok(Some(line)) => {
            out.commands.push(format!("sigrok-cli -d {} --scan", rig.analyzer.driver));
            out.step("analyzer", true, format!("{line}; {} Hz; stimulus D{}, response D{}", rig.analyzer.samplerate_hz, rig.analyzer.stimulus_channel, rig.analyzer.response_channel));
        }
        Ok(None) => {
            out.step("analyzer", false, format!("sigrok-cli -d {} --scan shows no device", rig.analyzer.driver));
            return out;
        }
        Err(e) => {
            out.step("analyzer", false, e);
            return out;
        }
    }

    // 3. The stimulus board: flash, read back, run.
    let stimulus = Target { chip: &rig.chip, serial: &rig.stimulus.serial };
    let scratch = inputs.work_dir.join("readback.bin");
    match stimulus.flash(inputs.stimulus_elf) {
        Ok(s) => {
            out.commands.push(s.command);
            out.step("stimulus flashed", true, format!("{} in {:.1} s", inputs.stimulus_elf.display(), s.seconds));
        }
        Err(e) => {
            out.locked = e.contains("Core is locked");
            out.step("stimulus flashed", false, e);
            return out;
        }
    }
    match stimulus.read_back(inputs.stimulus_elf, &scratch) {
        Ok(rb) => {
            let ok = readback_step(&mut out, "stimulus read back", &rb, inputs.stimulus_hash);
            out.stimulus_readback = Some(rb);
            if !ok {
                return out;
            }
        }
        Err(e) => {
            out.step("stimulus read back", false, e);
            return out;
        }
    }
    match stimulus.reset() {
        Ok(s) => {
            out.commands.push(s.command);
            out.step("stimulus running", true, "reset; the stimulus Application runs until the rig resets it again");
        }
        Err(e) => {
            out.step("stimulus running", false, e);
            return out;
        }
    }
    // The stimulus board's first Job configures its pins; give it a period.
    std::thread::sleep(Duration::from_millis(rig.expect.period_ms * 3));

    // 4. The capture and the target's run. probe-rs `run` flashes, resets,
    // and carries the console; the capture starts first and covers the
    // flashing time, the run, and the exit.
    let capture_path = inputs.work_dir.join("capture.bin");
    let capture_ms = (inputs.timeout_secs + 2) * 1000;
    let cap = match capture::start(&rig.analyzer, capture_ms, &capture_path) {
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
    out.locked = run.locked();
    let pulses = run.value("pulse");
    let run_ok = run.ok;
    let run_detail = if run_ok {
        format!("RSK-OK in {:.1} s; pulse Releases counted by the target: {}", run.seconds, pulses.map(|p| p.to_string()).unwrap_or("?".into()))
    } else if run.locked() {
        "the target's access port is locked (APPROTECT): unlock it once with `probe-rs run --allow-erase-all`; the HIL build keeps it open afterwards".to_string()
    } else if run.timed_out {
        format!("killed after {} s without an exit", inputs.timeout_secs)
    } else {
        "the run did not end in RSK-OK (console attached)".to_string()
    };
    out.run = Some(run);
    out.step("target run", run_ok, run_detail);

    // 5. The target's image, read back after the run (probe-rs `run` is
    // the flashing step; flash is not written by the firmware).
    match target.read_back(inputs.target_elf, &scratch) {
        Ok(rb) => {
            readback_step(&mut out, "target read back", &rb, inputs.target_hash);
            out.target_readback = Some(rb);
        }
        Err(e) => {
            out.step("target read back", false, e);
        }
    }

    // 6. The Kernel's trace of the run (feature `trace-ram`): read back
    // after the run, and checked for an interrupt storm.
    match target.read_trace(inputs.target_elf, &scratch) {
        Ok(Some(entries)) => {
            out.trace_text = crate::trace::render(&entries, inputs.tick_hz);
            let storm = crate::trace::longest_storm(&entries);
            let ok = storm.is_none_or(|(_, n, _)| n < 32);
            out.step(
                "Kernel trace",
                ok,
                match storm {
                    Some((v, n, ticks)) => format!(
                        "{} entries; longest re-entry of one vector without a dispatch: vector {v}, {n} entries over {:.3} ms{}",
                        entries.len(),
                        ticks as f64 * 1000.0 / inputs.tick_hz as f64,
                        if ok { "" } else { " (an interrupt storm)" }
                    ),
                    None => format!("{} entries; no interrupt re-entered without a dispatch", entries.len()),
                },
            );
        }
        Ok(None) => {
            out.step("Kernel trace", true, "the build has no trace ring (feature `trace-ram`)");
        }
        Err(e) => {
            out.step("Kernel trace", false, e);
        }
    }

    // 7. The capture against the console.
    match samples {
        Ok(samples) => {
            // Edges of one run are a period apart; three periods separate bursts.
            let gap = (rig.expect.period_ms * 3 * rig.analyzer.samplerate_hz as u64 / 1000) as usize;
            let a = capture::analyze(&samples, rig.analyzer.samplerate_hz, rig.analyzer.stimulus_channel, rig.analyzer.response_channel, pulses.map(|p| p as usize), gap);
            out.edges_text = a.edges_text(&samples, rig.analyzer.stimulus_channel, rig.analyzer.response_channel);
            let deadline_us = rig.expect.response_deadline_ms as f64 * 1000.0;
            let period_ok = a.period_us.is_some_and(|(lo, hi)| {
                let nominal = rig.expect.period_ms as f64 * 1000.0;
                lo > nominal * 0.98 && hi < nominal * 1.02
            });
            let stimulus_detail = format!(
                "{} rises, period {} µs, pulse {} µs",
                a.stimulus_rises,
                a.period_us.map(|(lo, hi)| format!("{lo:.0} to {hi:.0}")).unwrap_or("none".into()),
                a.pulse_us.map(|(lo, hi)| format!("{lo:.1} to {hi:.1}")).unwrap_or("none".into())
            );
            out.step("stimulus seen by the analyzer", a.stimulus_rises >= 2 && period_ok, stimulus_detail);
            let late = a.late(deadline_us);
            let counted = pulses.map(|p| p as usize);
            let count_ok = counted.is_some_and(|c| c == a.matched());
            let loop_ok = a.matched() >= rig.expect.min_matched && late == 0 && a.unanswered_in_window == 0 && a.unmatched_responses == 0 && count_ok;
            out.step(
                "loopback",
                loop_ok && run_ok,
                format!(
                    "{} stimulus pulses answered (target counted {}); latency {} to {} µs, median {} µs, {late} later than {} ms; {} unanswered inside the run, {} responses without a stimulus, {} response edges before the run (flashing) and {} after it (the reset on detach); {} pulses before the run, {} after; capture {:.1} s",
                    a.matched(),
                    counted.map(|c| c.to_string()).unwrap_or("?".into()),
                    a.latency_min_us().map(|v| format!("{v:.1}")).unwrap_or("-".into()),
                    a.latency_max_us().map(|v| format!("{v:.1}")).unwrap_or("-".into()),
                    a.latency_median_us().map(|v| format!("{v:.1}")).unwrap_or("-".into()),
                    rig.expect.response_deadline_ms,
                    a.unanswered_in_window,
                    a.unmatched_responses,
                    a.responses_before_run,
                    a.responses_after_run,
                    a.stimulus_before,
                    a.stimulus_after,
                    a.samples as f64 / a.samplerate_hz as f64
                ),
            );
            out.analysis = Some(a);
        }
        Err(e) => {
            out.step("capture", false, e);
        }
    }
    out
}

fn readback_step(out: &mut Outcome, name: &str, rb: &Readback, manifest_hash: Option<&str>) -> bool {
    let manifest = match manifest_hash {
        Some(h) if h == rb.hash => ", equal to the Build_Manifest's".to_string(),
        Some(h) => format!(", NOT the Build_Manifest's {h}"),
        None => " (no Build_Manifest hash given)".to_string(),
    };
    let ok = rb.matches() && manifest_hash.is_none_or(|h| h == rb.hash);
    let detail = if rb.mismatches.is_empty() {
        format!("{} parts, {} bytes, read-back hash {}{manifest}", rb.segments, rb.bytes, rb.hash)
    } else {
        format!("{} parts, {} bytes; differs from the ELF at {}", rb.segments, rb.bytes, rb.mismatches.iter().map(|a| format!("{a:#x}")).collect::<Vec<_>>().join(", "))
    };
    out.step(name, ok, detail)
}

/// The rig self-test report (Markdown): the rig, the steps, the capture.
pub fn render(rig: &Rig, out: &Outcome) -> String {
    let mut s = String::new();
    s.push_str("# HIL_Rig self-test\n\n");
    s.push_str(&format!("Verdict: **{}**\n\n", if out.ok() { "pass" } else if out.locked { "inconclusive (access port locked)" } else { "FAIL" }));
    s.push_str(&format!("Tools: {}; {}\n\n", if out.probe_rs.is_empty() { "probe-rs absent" } else { &out.probe_rs }, if out.sigrok.is_empty() { "sigrok-cli absent" } else { &out.sigrok }));
    s.push_str("## Rig\n\n| Role | Probe | Board | Binary | Wired |\n|---|---|---|---|---|\n");
    for b in rig.boards() {
        s.push_str(&format!("| {} | `{}` | {} | {} | {} |\n", b.role, b.serial, b.board, b.bin.as_deref().unwrap_or("(the Application under test)"), if b.wired { "yes" } else { "no" }));
    }
    s.push_str(&format!("\nAnalyzer: {} at {} Hz; stimulus mirror on D{}, response on D{}.\n\n", rig.analyzer.driver, rig.analyzer.samplerate_hz, rig.analyzer.stimulus_channel, rig.analyzer.response_channel));
    if !rig.wiring.is_empty() {
        s.push_str("Wiring:\n\n");
        for (k, v) in &rig.wiring {
            s.push_str(&format!("- `{k}`: {v}\n"));
        }
        s.push('\n');
    }
    s.push_str("## Steps\n\n| Step | Result | Detail |\n|---|---|---|\n");
    for st in &out.steps {
        s.push_str(&format!("| {} | {} | {} |\n", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|").replace('\n', " ")));
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
