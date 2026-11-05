//! The cycle-attribution test (task 7.5's `hil-cycles`; R18.1, ASM-06):
//! on the P1 demo's cycle configuration (`nrf52840_cycles`, 2 seconds)
//! `control` (Priority 7, preempted by nothing but the Kernel's own
//! entries) and `log` (Priority 2, preempted by control, echo, sensor,
//! pulse, and the Kernel's work for them) execute the same fixed-iteration
//! spin in every Job. The Kernel's Budget accounting charges each Job the
//! DWT cycles it executed between its own charge points (dispatch,
//! preemption, resumption, every timed event, completion), so the record
//! of R34.7 (the executions per Task: count, minimum, maximum, mean) shows
//! the two Tasks' executions equal within the accounting's own overhead
//! while `log`'s response times (Release to completion) are many times
//! its execution. A Kernel that charged the preempting Jobs' or the
//! Kernel's cycles to the preempted Job would show `log`'s executions far
//! above `control`'s.

use std::fmt::Write;
use std::path::Path;

use crate::board::{Run, Target};
use crate::config::Rig;
use crate::release_timing::Step;
use crate::stats::Stats;

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    pub target_elf: &'a Path,
    pub work_dir: &'a Path,
    pub timeout_secs: u64,
    pub tick_hz: u64,
    pub cpu_hz: u64,
}

/// The Tasks compared: `control` (0) and `log` (3).
pub const HIGH: usize = 0;
pub const LOW: usize = 3;
/// The agreement required of the two mean executions, in percent. The
/// preempted Job is charged, at every Kernel entry that interrupts it,
/// the cycles between the exception entry and the handler's charge point
/// and between the last charge point and the return (the stub's pushes
/// and pops, the HIL build's trace and pins), some hundreds of cycles per
/// entry with the cache off; measured 2026-11-04: `log`'s Jobs, each
/// interrupted by about a dozen entries, 4,878 and 5,498 cycles (6.4 and 7.2 %) above
/// `control`'s. The tolerance covers that overhead; the difference itself
/// is reported as the attribution overhead.
pub const TOLERANCE_PERCENT: u64 = 10;

#[derive(Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub run: Option<Run>,
    pub stats: Option<Stats>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }
    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
    }
}

/// Judges the record: both Tasks ran enough Jobs; their mean executions
/// agree within the tolerance; each Task's executions are tight (the
/// spread below the tolerance of the mean); `log`'s response times exceed
/// its executions (it was preempted).
pub fn judge(st: &Stats, cpu_hz: u64, tick_hz: u64) -> Vec<(String, bool, String)> {
    let mut out = Vec::new();
    let (Some(h), Some(l)) = (st.tasks.get(HIGH), st.tasks.get(LOW)) else {
        out.push(("record".into(), false, "the record has no entry for the two Tasks".into()));
        return out;
    };
    out.push(("Jobs recorded".into(), h.exec_count >= 150 && l.exec_count >= 40, format!("control {} Jobs, log {} Jobs (at least 150 and 40 expected over 2 s)", h.exec_count, l.exec_count)));
    let (hm, lm) = (h.exec_mean_cycles(), l.exec_mean_cycles());
    let diff = (hm - lm).abs();
    let tol = hm * TOLERANCE_PERCENT as f64 / 100.0;
    out.push((
        "executions agree (R18.1: the preempted Job is charged its own cycles only)".into(),
        hm > 0.0 && diff <= tol,
        format!("control mean {hm:.0} cycles ({:.3} ms), log mean {lm:.0} cycles ({:.3} ms): difference {diff:.0} cycles ({:.2} %) against {TOLERANCE_PERCENT} %", hm * 1000.0 / cpu_hz as f64, lm * 1000.0 / cpu_hz as f64, if hm > 0.0 { diff * 100.0 / hm } else { 0.0 }),
    ));
    out.push((
        "attribution overhead (informative: the preempted Job's excess, the Kernel entries' cycles outside the charge points)".into(),
        true,
        format!("{:.0} cycles ({:.3} ms) per log Job over its {} Jobs; the Analyzer's switching parameters must cover it per preemption", lm - hm, (lm - hm) * 1000.0 / cpu_hz as f64, l.exec_count),
    ));
    let tight = |t: &crate::stats::TaskStats| t.exec_max_cycles as f64 - t.exec_min_cycles as f64 <= t.exec_mean_cycles() * 2.0 * TOLERANCE_PERCENT as f64 / 100.0;
    out.push((
        "executions tight (every Job of a Task within a narrow band)".into(),
        tight(h) && tight(l),
        format!("control min {} max {}, log min {} max {} cycles", h.exec_min_cycles, h.exec_max_cycles, l.exec_min_cycles, l.exec_max_cycles),
    ));
    let resp_cycles = l.resp_max_ticks as u64 * cpu_hz / tick_hz;
    out.push((
        "log preempted (its response times exceed its executions)".into(),
        resp_cycles > l.exec_max_cycles as u64 * 3,
        format!("log's largest response {} ticks ({} cycles) against its largest execution {} cycles; control's largest response {} ticks", l.resp_max_ticks, resp_cycles, l.exec_max_cycles, h.resp_max_ticks),
    ));
    out
}

pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    let _ = std::fs::create_dir_all(inputs.work_dir);
    let scratch = inputs.work_dir.join("readback.bin");
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    let run = target.run(inputs.target_elf, inputs.timeout_secs);
    out.commands.push(run.command.clone());
    out.step("target run", run.ok, if run.ok { format!("RSK-OK after {:.1} s (operating duration 2 s)", run.seconds) } else { "the run did not pass (console in the report)".to_string() });
    out.run = Some(run);
    match target.read_stats(inputs.target_elf, &scratch) {
        Ok(Some(st)) => {
            for (name, ok, detail) in judge(&st, inputs.cpu_hz, inputs.tick_hz) {
                out.step(&name, ok, detail);
            }
            out.stats = Some(st);
        }
        Ok(None) => out.step("record", false, "the build has no release-timing record"),
        Err(e) => out.step("record", false, e),
    }
    out
}

pub fn render(rig: &Rig, inputs: &Inputs<'_>, out: &Outcome, binary: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# HIL cycle attribution: the Budget accounting against the DWT (task 7.5, R18.1, ASM-06)\n");
    let _ = writeln!(s, "Verdict: **{}**\n", if out.ok() { "pass" } else { "FAIL" });
    let _ = writeln!(s, "Target {}; binary {binary} (the cycle configuration `nrf52840_cycles`: control and log execute the same {}-iteration spin per Job).\n", rig.target.serial, 8_000);
    let _ = writeln!(s, "## Steps\n\n| Step | Result | Detail |\n|---|---|---|");
    for st in &out.steps {
        let _ = writeln!(s, "| {} | {} | {} |", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|"));
    }
    if let Some(st) = &out.stats {
        let _ = writeln!(s, "\n## The Kernel's record\n\n{}", st.render(inputs.cpu_hz, inputs.tick_hz));
    }
    if !out.commands.is_empty() {
        let _ = writeln!(s, "\n## Commands\n");
        for c in &out.commands {
            let _ = writeln!(s, "    {c}");
        }
    }
    if let Some(run) = &out.run {
        let _ = writeln!(s, "\n## Target console\n\n```\n{}\n```", run.log.trim_end());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stats::TaskStats;

    fn task(count: u32, min: u32, max: u32, mean: u32, resp: u32) -> TaskStats {
        TaskStats { exec_count: count, exec_min_cycles: min, exec_max_cycles: max, exec_sum_cycles: mean as u64 * count as u64, resp_max_ticks: resp, ..Default::default() }
    }

    #[test]
    fn equal_executions_pass_and_a_charged_preemption_fails() {
        let mut st = Stats::default();
        st.tasks = vec![task(200, 39_800, 40_400, 40_000, 400), TaskStats::default(), TaskStats::default(), task(50, 40_000, 41_000, 40_600, 80_000)];
        let v = judge(&st, 64_000_000, 16_000_000);
        assert!(v.iter().all(|(_, ok, _)| *ok), "{v:?}");
        st.tasks[LOW] = task(50, 60_000, 120_000, 90_000, 80_000);
        let v = judge(&st, 64_000_000, 16_000_000);
        assert!(!v[1].1 && !v[3].1);
    }
}
