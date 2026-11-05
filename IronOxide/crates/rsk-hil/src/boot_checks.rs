//! The boot-check suite (task 7.2, R12.6): one run per check of
//! Requirement 12 on a HIL build with the rsk-kernel feature
//! `boot-mutation`, whose mutation word (RAM above every region) the rig
//! writes over the probe before flashing and running the build. Each
//! mutation makes exactly one check fail: a recorded value differing from
//! the Target's (the CPUID, the implemented priority bits, the clock
//! source, the recorded `CONFIG` and `LAYOUT` checksums, the
//! interrupt-line count), the `Config` table corrupted in memory, a
//! register state the check rejects (PRIGROUP changed, the instruction
//! cache enabled), and the watchdog started with a 50 ms reload and never
//! fed (the reset lands inside the run; the next boot fails the watchdog
//! check and reports). The suite passes a case when the target
//! reaches the safe state without releasing any Task (every counter 0)
//! and its report records the failed check's identifier (the
//! `boot_check` event's detail bits) and nothing else. A final run with
//! the word 0 shows the unmutated build boots.
//!
//! Checks without a case, and why: the implementer and part-number
//! checks (bits 0 and 1) and the MPU region count (bit 2) compare the
//! Target against constants of the Kernel (a Cortex-M4 with 8 regions),
//! not against a recorded value, and the DWT (bit 3) and the FPU (bit
//! 12) cannot be removed from the silicon; the host test
//! `boot_checks_pass_on_the_expected_board_and_fail_one_bit_per_check`
//! covers those on simulated registers.

use std::fmt::Write;
use std::path::Path;

use crate::board::{Run, Target};
use crate::config::Rig;
use crate::release_timing::Step;

/// The mutation word's address (`rsk_kernel::arch::cm4::unsafe_module::BOOT_MUTATION_ADDR`):
/// the top of the nRF52840's RAM, outside every region of the layout and
/// of the flashing algorithm probe-rs runs, so that a write before the
/// flashing survives the flashing and the reset.
pub const MUTATION_ADDR: u32 = 0x2003_FFF0;

/// One case: the mutation word, the failed check's bit, and what it does.
#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub mutation: u32,
    pub bit: u32,
    pub what: &'static str,
    pub requirement: &'static str,
}

pub const CASES: &[Case] = &[
    Case { mutation: 1, bit: 8, what: "the recorded CPUID differs from the Target's (ASM-01)", requirement: "R12.1" },
    Case { mutation: 2, bit: 4, what: "the recorded priority bits (8) differ from the implemented 3 (ASM-03)", requirement: "R12.1" },
    Case { mutation: 3, bit: 10, what: "the recorded clock source (the RC oscillator) differs from the running crystal (ASM-05)", requirement: "R12.1" },
    Case { mutation: 4, bit: 9, what: "PRIGROUP changed from its reset value before the check (ASM-03)", requirement: "R12.1" },
    Case { mutation: 5, bit: 11, what: "the instruction cache enabled before the check (ASM-04)", requirement: "R12.1" },
    Case { mutation: 6, bit: 5, what: "the recorded CONFIG checksum differs from the tables'", requirement: "R12.3" },
    Case { mutation: 7, bit: 5, what: "the CONFIG table corrupted in memory (a Task's Budget changed)", requirement: "R12.3" },
    Case { mutation: 8, bit: 13, what: "the recorded LAYOUT checksum differs from the tables'", requirement: "R12.3" },
    Case { mutation: 9, bit: 6, what: "the recorded interrupt-line count (600) exceeds what the Kernel can serve", requirement: "R12.2" },
    Case { mutation: 10, bit: 7, what: "the watchdog started with a 50 ms reload and never fed: the reset lands inside the run and the next boot fails the watchdog check (ORQ-31)", requirement: "R12.2" },
];

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    pub target_elf: &'a Path,
    pub work_dir: &'a Path,
    pub timeout_secs: u64,
    /// The names of the per-Task counters the report prints.
    pub counters: &'a [&'a str],
}

#[derive(Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub runs: Vec<(u32, Run)>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }
}

/// The `boot_check` events of a report (kind 14) with their detail words.
pub fn boot_check_details(log: &str) -> Vec<u32> {
    let mut out = Vec::new();
    let mut lines = log.lines().map(str::trim).peekable();
    while let Some(l) = lines.next() {
        if l == "event=14" {
            for n in lines.by_ref() {
                if let Some(d) = n.strip_prefix("detail=") {
                    if let Ok(v) = d.trim().parse() {
                        out.push(v);
                    }
                    break;
                }
            }
        }
    }
    out
}

/// The kinds of every event of a report.
pub fn event_kinds(log: &str) -> Vec<u32> {
    log.lines().filter_map(|l| l.trim().strip_prefix("event=")).filter_map(|v| v.trim().parse().ok()).collect()
}

/// Judges one case from the report: the safe state reached, the failed
/// check's bit recorded (and, for the watchdog case, the reset logged),
/// every counter 0, no other event but the boot information.
pub fn judge(case: &Case, run: &Run, counters: &[&str]) -> (bool, String) {
    let details = boot_check_details(&run.log);
    let kinds = event_kinds(&run.log);
    let safe = run.log.contains("RSK-SAFE-STATE");
    let bit_ok = details.iter().any(|d| d & (1 << case.bit) != 0);
    let only_bit = details.iter().all(|d| *d & !(1 << case.bit) == 0);
    let released: Vec<String> = counters.iter().filter_map(|c| run.value(c).filter(|v| *v != 0).map(|v| format!("{c}={v}"))).collect();
    let watchdog_logged = case.mutation != 10 || kinds.contains(&15);
    // Expected events: debugger (23), watchdog reset (15) for that case, boot_check (14), safe_state (18).
    let unexpected: Vec<u32> = kinds.iter().copied().filter(|k| !matches!(k, 14 | 15 | 18 | 23)).collect();
    let ok = safe && bit_ok && only_bit && released.is_empty() && watchdog_logged && unexpected.is_empty();
    let detail = format!(
        "{}; boot_check detail {}; {}; {}{}{}",
        if safe { "safe state reached" } else { "NO SAFE STATE" },
        details.iter().map(|d| format!("{d:#x}")).collect::<Vec<_>>().join(","),
        if bit_ok { format!("bit {} recorded{}", case.bit, if only_bit { " alone" } else { " WITH OTHERS" }) } else { format!("bit {} NOT recorded", case.bit) },
        if released.is_empty() { "no Task released".to_string() } else { format!("TASKS RAN: {}", released.join(" ")) },
        if watchdog_logged { "" } else { "; watchdog reset NOT logged" },
        if unexpected.is_empty() { String::new() } else { format!("; unexpected events {unexpected:?}") }
    );
    (ok, detail)
}

pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    let _ = std::fs::create_dir_all(inputs.work_dir);
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    let addr = MUTATION_ADDR;
    for case in CASES {
        match target.write_word(addr, case.mutation) {
            Ok(c) => out.commands.push(c),
            Err(e) => {
                out.steps.push(Step { name: format!("case {}: {}", case.mutation, case.what), ok: false, detail: e });
                continue;
            }
        }
        // The watchdog case resets the chip mid-run: probe-rs must not halt
        // at the reset vector (its default), so that the next boot reports.
        let extra: &[&str] = if case.mutation == 10 { &["--no-catch-reset"] } else { &[] };
        let run = target.run_with(inputs.target_elf, inputs.timeout_secs, extra);
        out.commands.push(run.command.clone());
        let (ok, detail) = judge(case, &run, inputs.counters);
        out.steps.push(Step { name: format!("case {} ({}): {}", case.mutation, case.requirement, case.what), ok, detail });
        out.runs.push((case.mutation, run));
    }
    // The word back to 0: the unmutated build boots and runs.
    match target.write_word(addr, 0) {
        Ok(c) => {
            out.commands.push(c);
            let run = target.run(inputs.target_elf, inputs.timeout_secs);
            out.commands.push(run.command.clone());
            let ok = run.ok && boot_check_details(&run.log).is_empty();
            out.steps.push(Step { name: "no mutation: the build boots and runs (RSK-OK)".into(), ok, detail: if ok { format!("RSK-OK after {:.1} s", run.seconds) } else { "the unmutated build did not pass".into() } });
            out.runs.push((0, run));
        }
        Err(e) => out.steps.push(Step { name: "mutation word cleared".into(), ok: false, detail: e }),
    }
    out
}

pub fn render(rig: &Rig, out: &Outcome, binary: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# HIL boot-check suite: mismatched values and corrupted tables (task 7.2, R12.6)\n");
    let _ = writeln!(s, "Verdict: **{}**\n", if out.ok() { "pass" } else { "FAIL" });
    let _ = writeln!(s, "Target {}; binary {binary} (the HIL build with the `boot-mutation` feature; one run per case, the mutation word written over the probe before each).\n", rig.target.serial);
    let _ = writeln!(s, "## Cases\n\n| Case | Result | Detail |\n|---|---|---|");
    for st in &out.steps {
        let _ = writeln!(s, "| {} | {} | {} |", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|"));
    }
    let _ = writeln!(s, "\n## Target consoles\n");
    for (m, run) in &out.runs {
        let _ = writeln!(s, "### mutation {m}\n\n```\n{}\n```\n", run.log.trim_end());
    }
    if !out.commands.is_empty() {
        let _ = writeln!(s, "## Commands\n");
        for c in &out.commands {
            let _ = writeln!(s, "    {c}");
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(log: &str) -> Run {
        Run { ok: false, seconds: 1.0, log: log.into(), command: String::new(), timed_out: false }
    }

    #[test]
    fn a_failed_check_alone_with_no_task_released_passes() {
        let log = "RSK-SAFE-STATE\ncontrol=0\nsensor=0\nevents=3\nevent=23\n  task=255\n  at_ms=0\n  detail=0\nevent=14\n  task=255\n  at_ms=0\n  detail=256\nevent=18\n  task=255\n  at_ms=0\n  detail=0\nRSK-FAIL\n";
        let (ok, d) = judge(&CASES[0], &run(log), &["control", "sensor"]);
        assert!(ok, "{d}");
        let (ok, _) = judge(&CASES[1], &run(log), &["control", "sensor"]);
        assert!(!ok, "the wrong bit");
        let ran = log.replace("control=0", "control=3");
        let (ok, d) = judge(&CASES[0], &run(&ran), &["control", "sensor"]);
        assert!(!ok && d.contains("TASKS RAN"));
        let (ok, _) = judge(&CASES[9], &run(&log.replace("detail=256", "detail=128")), &["control"]);
        assert!(!ok, "the watchdog case wants the reset logged");
        let (ok, _) = judge(&CASES[9], &run(&log.replace("detail=256", "detail=128").replace("event=23", "event=15")), &["control"]);
        assert!(ok);
    }
}
