//! The isolation suite (task 7.4): the MPU access matrix of R16.8 and the
//! EasyDMA cases of R17.7 on the P1 demo's isolation configuration
//! (`nrf52840_isolation`, 3.6 seconds, every fault response END_JOB; the
//! rsk-kernel feature `hw-dt` for the probes), judged after the run from
//! the Kernel's event mirror (one MemManage or BusFault event, then an
//! END_JOB, per attempt; one `dma_rejected`, then an END_JOB, per refused
//! request), the fault record (the CFSR and address of each fault
//! matched with the attempt's address and class), the actions record
//! (the EasyDMA stop preceding the re-initialization of the restarted
//! Partition, R17.6), and the target's report (the walkers' counters,
//! SPIM0's registers read by the safe state, the probes' values). The
//! same run feeds the Hardware_Model differential tests (`dt`).
//!
//! The matrix is enumerated here exactly as `rsk::matrix` enumerates it
//! on the target, from the Task_Model's memory map: for the walker's
//! Partition `p`, against each other Partition `q` in index order: code
//! read, code write, code fetch (at `code + 0x100`), data read, data write
//! (at `ram + stack + 0x100`), stack write (at `ram + 0x10`), peripheral
//! read, peripheral write (the first window, skipped when `q` owns none);
//! then against the Kernel: RAM read and write (`kernel_ram + 0x100`),
//! flash read and fetch (`kernel_flash + 0x100`), PPB read and write
//! (NVIC_ISER0, 0xE000_E100), and a write to the word below `p`'s own
//! stack. The walkers are `part0.control`, `part1.log`, and
//! `part2.sampler` (after its seven EasyDMA cases).

use std::collections::BTreeMap;
use std::fmt::Write;
use std::path::Path;

use rsk_model::task_model::TaskModel;

use crate::board::{Run, Target};
use crate::config::Rig;
use crate::dt::{self, TaskSide};
use crate::events::{kind_name, EventRec, Events};
use crate::faults::{Fault, Faults};
use crate::release_timing::Step;

const MEMMANAGE: u8 = 2;
const BUSFAULT: u8 = 3;
const DMA_REJECTED: u8 = 13;
const PARTITION_RESTART: u8 = 16;
const END_JOB: u8 = 19;
pub const PPB: u32 = 0xE000_E100;
/// The EasyDMA cases of `sampler` before its matrix walk; the refused ones.
pub const DMA_CASES: u32 = 7;
pub const DMA_REFUSED: usize = 5;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Class {
    Read,
    Write,
    Fetch,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attempt {
    pub index: usize,
    pub what: String,
    /// `None`: skipped on the target (no address for the class).
    pub address: Option<u32>,
    pub class: Class,
    /// The fault kind the Kernel logs: MemManage, or BusFault for the PPB.
    pub kind: u8,
}

#[derive(Clone, Debug)]
pub struct Walker {
    pub task: u8,
    pub partition: u8,
    pub name: String,
    pub attempts: Vec<Attempt>,
}

fn region_base(r: &rsk_model::task_model::Region) -> u32 {
    r.base
}

/// The attempts of Partition `p`, in the target's order.
pub fn attempts(model: &TaskModel, p: usize) -> Vec<Attempt> {
    let mut out = Vec::new();
    let n = model.partitions.len();
    let mut i = 0usize;
    for q in 0..n {
        if q == p {
            continue;
        }
        let part = &model.partitions[q];
        let code = region_base(&part.mpu.code) + 0x100;
        let data = region_base(&part.mpu.ram) + part.regions.stack as u32 + 0x100;
        let stack = region_base(&part.mpu.ram) + 0x10;
        let window = part.mpu.windows.first().map(|w| w.base);
        let name = &part.id;
        let mut push = |what: String, address: Option<u32>, class: Class| {
            out.push(Attempt { index: i, what, address, class, kind: MEMMANAGE });
            i += 1;
        };
        push(format!("read {name}'s code"), Some(code), Class::Read);
        push(format!("write {name}'s code"), Some(code), Class::Write);
        push(format!("fetch from {name}'s code"), Some(code), Class::Fetch);
        push(format!("read {name}'s data"), Some(data), Class::Read);
        push(format!("write {name}'s data"), Some(data), Class::Write);
        push(format!("write into {name}'s stack"), Some(stack), Class::Write);
        push(format!("read {name}'s peripheral window"), window, Class::Read);
        push(format!("write {name}'s peripheral window"), window, Class::Write);
    }
    let kram = region_base(&model.layout.kernel_ram) + 0x100;
    let kflash = region_base(&model.layout.kernel_flash) + 0x100;
    let own = region_base(&model.partitions[p].mpu.ram).wrapping_sub(4);
    let kernel: [(String, u32, Class, u8); 7] = [
        ("read the Kernel's RAM".into(), kram, Class::Read, MEMMANAGE),
        ("write the Kernel's RAM".into(), kram, Class::Write, MEMMANAGE),
        ("read the Kernel's flash".into(), kflash, Class::Read, MEMMANAGE),
        ("fetch from the Kernel's flash".into(), kflash, Class::Fetch, MEMMANAGE),
        ("read the PPB (NVIC_ISER0)".into(), PPB, Class::Read, BUSFAULT),
        ("write the PPB (NVIC_ISER0)".into(), PPB, Class::Write, BUSFAULT),
        ("write the word below the own stack (R16.5)".into(), own, Class::Write, MEMMANAGE),
    ];
    for (what, address, class, kind) in kernel {
        out.push(Attempt { index: i, what, address: Some(address), class, kind });
        i += 1;
    }
    out
}

/// The three walkers of the isolation configuration.
pub fn walkers(model: &TaskModel) -> Vec<Walker> {
    ["part0.control", "part1.log", "part2.sampler"]
        .iter()
        .filter_map(|id| {
            let t = model.tasks.iter().position(|x| x.id == *id)?;
            let p = model.partitions.iter().position(|x| x.id == model.tasks[t].partition)?;
            Some(Walker { task: t as u8, partition: p as u8, name: id.to_string(), attempts: attempts(model, p) })
        })
        .collect()
}

/// Whether a fault-record entry is the fault an attempt predicts.
pub fn fault_matches(f: &Fault, a: &Attempt) -> bool {
    let Some(addr) = a.address else { return false };
    match (a.kind, a.class) {
        (BUSFAULT, _) => f.is_bus() && f.address == addr,
        (_, Class::Fetch) => f.iaccviol() && f.address == addr,
        _ => f.daccviol() && f.mmarvalid() && f.address == addr,
    }
}

/// Judges the matrix: per walker, the fault events of its Task in order
/// against its attempts, each followed by an END_JOB of the Task; the
/// fault record holding one entry per attempt (by address and class).
pub fn judge_matrix(walkers: &[Walker], events: &[EventRec], faults: &[Fault]) -> (bool, Vec<String>, TaskSide) {
    let mut ok = true;
    let mut notes = Vec::new();
    let mut side = TaskSide::default();
    let mut refused_total = 0usize;
    let mut stack_overflow_ok = true;
    for w in walkers {
        let expected: Vec<&Attempt> = w.attempts.iter().filter(|a| a.address.is_some()).collect();
        let fault_events: Vec<(usize, &EventRec)> = events.iter().enumerate().filter(|(_, e)| e.task == w.task && (e.kind == MEMMANAGE || e.kind == BUSFAULT)).collect();
        let kinds_ok = fault_events.len() == expected.len() && fault_events.iter().zip(&expected).all(|((_, e), a)| e.kind == a.kind);
        let ended = fault_events.iter().filter(|(i, _)| events.get(i + 1).is_some_and(|n| n.kind == END_JOB && n.task == w.task)).count();
        if !kinds_ok || ended != fault_events.len() {
            ok = false;
        }
        notes.push(format!(
            "{}: {} attempts ({} with an address), {} fault events ({} MemManage, {} BusFault), {} followed by END_JOB{}",
            w.name,
            w.attempts.len(),
            expected.len(),
            fault_events.len(),
            fault_events.iter().filter(|(_, e)| e.kind == MEMMANAGE).count(),
            fault_events.iter().filter(|(_, e)| e.kind == BUSFAULT).count(),
            ended,
            if kinds_ok { "" } else { "; SEQUENCE MISMATCH" }
        ));
        refused_total += expected.len();
    }
    // The fault record: one entry per attempt, by address and class, over
    // every walker (the record is global and the walkers interleave).
    let mut counts: BTreeMap<(u32, u8, u8), (usize, String)> = BTreeMap::new();
    for w in walkers {
        for a in w.attempts.iter().filter(|a| a.address.is_some()) {
            let e = counts.entry((a.address.unwrap(), a.kind, a.class as u8)).or_insert((0, a.what.clone()));
            e.0 += 1;
        }
    }
    let mut missing = Vec::new();
    for ((addr, kind, class), (n, what)) in &counts {
        let probe = Attempt { index: 0, what: what.clone(), address: Some(*addr), class: match class { 0 => Class::Read, 1 => Class::Write, _ => Class::Fetch }, kind: *kind };
        let got = faults.iter().filter(|f| fault_matches(f, &probe)).count();
        if got < *n {
            ok = false;
            missing.push(format!("{what} at {addr:#010x}: {got} of {n} faults recorded"));
            if what.contains("own stack") {
                stack_overflow_ok = false;
            }
        }
    }
    if missing.is_empty() {
        notes.push(format!("fault record: every one of the {} attempts has its fault with the attempted address and class (IACCVIOL for fetches, DACCVIOL with MMFAR for data, a BusFault for the PPB)", refused_total));
    } else {
        notes.push(format!("fault record: MISSING {}", missing.join("; ")));
    }
    let unexpected = faults.len().saturating_sub(refused_total);
    if unexpected > 0 {
        notes.push(format!("{unexpected} faults beyond the attempts' (the matrix is judged on its own; see the record)"));
    }
    side.ppb_attempts = walkers.len() * 2;
    side.ppb_faults = faults.iter().filter(|f| f.is_bus()).map(|f| (f.precise_bus(), f.imprecise_bus(), f.bfarvalid(), f.address)).collect();
    side.stack_overflow_refused = Some(stack_overflow_ok);
    side.matrix = Some((ok, notes.join(" | ")));
    (ok, notes, side)
}

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    pub target_elf: &'a Path,
    pub work_dir: &'a Path,
    pub timeout_secs: u64,
    pub tick_hz: u64,
    pub model: &'a TaskModel,
}

#[derive(Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub run: Option<Run>,
    pub events: Option<Events>,
    pub faults: Option<Faults>,
    pub actions: Vec<(u32, u32)>,
    pub trace_text: String,
    pub dt: Option<dt::Record>,
    pub dt_rows: Vec<dt::Row>,
    pub walkers: Vec<Walker>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }
    /// The differential tests' verdict: no rule mismatched.
    pub fn dt_ok(&self) -> bool {
        self.dt.is_some() && !self.dt_rows.iter().any(|r| r.verdict == dt::Verdict::Mismatch)
    }
    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
    }
}

pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    out.walkers = walkers(inputs.model);
    let _ = std::fs::create_dir_all(inputs.work_dir);
    let scratch = inputs.work_dir.join("readback.bin");
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    let run = target.run(inputs.target_elf, inputs.timeout_secs);
    out.commands.push(run.command.clone());
    out.step("target run", run.ok, if run.ok { format!("the report ran after {:.1} s (operating duration 3.6 s)", run.seconds) } else if run.timed_out { "no report within the timeout".to_string() } else { "the run ended without a report".to_string() });
    let log = run.log.clone();
    out.run = Some(run);
    let v = |k: &str| Run { ok: true, seconds: 0.0, log: log.clone(), command: String::new(), timed_out: false }.value(k);
    let events = match target.read_events(inputs.target_elf, &scratch) {
        Ok(Some(ev)) => Some(ev),
        Ok(None) => { out.step("event mirror", false, "the build has no event mirror"); None }
        Err(e) => { out.step("event mirror", false, e); None }
    };
    let faults = match target.read_words(inputs.target_elf, "FAULT_RING", &scratch) {
        Ok(Some(w)) => match Faults::decode(&w) { Ok(f) => Some(f), Err(e) => { out.step("fault record", false, e); None } },
        Ok(None) => { out.step("fault record", false, "the build has no fault record"); None }
        Err(e) => { out.step("fault record", false, e); None }
    };
    let mut side = TaskSide::default();
    if let (Some(ev), Some(f)) = (&events, &faults) {
        let run_events: Vec<EventRec> = ev.run_entries().to_vec();
        let (ok, notes, s) = judge_matrix(&out.walkers, &run_events, &f.entries);
        side = s;
        out.step("MPU access matrix (R16.8, R16.4)", ok, notes.join(" | "));
        // The walkers' counters: every attempt made, none survived.
        let count0 = out.walkers.first().map(|w| w.attempts.len() as u32).unwrap_or(0);
        let cm = v("control_matrix"); let cs = v("control_survived"); let lm = v("log_matrix"); let ls = v("log_survived");
        let counters_ok = cm == Some(count0) && cs == Some(0) && lm == Some(count0) && ls == Some(0);
        out.step("walkers' counters (every attempt made, none returned)", counters_ok, format!("control {}/{} attempts, survived {}; log {}/{}, survived {}; sampler's counters are since part2's restart at 65 ms and judged by its events", fmt(cm), count0, fmt(cs), fmt(lm), count0, fmt(ls)));
        // The EasyDMA cases (R17.7).
        let sampler = out.walkers.iter().find(|w| w.name == "part2.sampler").map(|w| w.task);
        if let Some(t) = sampler {
            let rejected: Vec<usize> = run_events.iter().enumerate().filter(|(_, e)| e.kind == DMA_REJECTED && e.task == t).map(|(i, _)| i).collect();
            let ended = rejected.iter().filter(|i| run_events.get(*i + 1).is_some_and(|n| n.kind == END_JOB && n.task == t)).count();
            let ptr = v("sampler_dma_ptr");
            let p2 = inputs.model.partitions.iter().find(|p| p.id == "part2");
            let in_ram = match (ptr, p2) {
                (Some(ptr), Some(p)) => ptr >= p.mpu.ram.base + p.regions.stack as u32 && ptr + 16 <= p.mpu.ram.base + (1u32 << p.mpu.ram.size_log2),
                _ => false,
            };
            let regs_ok = v("spim0_rxd_ptr") == ptr && v("spim0_rxd_maxcnt") == Some(16) && v("spim0_txd_ptr") == Some(0) && v("spim0_txd_maxcnt") == Some(0)
                && v("spim0_rxd_list") == Some(0) && v("spim0_txd_list") == Some(0) && v("spim0_enable") == Some(0) && v("spim0_events_started") == Some(0) && v("spim0_events_end") == Some(0) && v("spim0_rxd_amount") == Some(0);
            let ok = rejected.len() == DMA_REFUSED && ended == DMA_REFUSED && in_ram && regs_ok && v("dma_ok") == Some(2) && v("sampler_dma_case").is_some_and(|c| c >= DMA_CASES + count0);
            out.step(
                "EasyDMA cases (R17.7, R17.1, R17.2)",
                ok,
                format!(
                    "{} refused requests (stack buffer, another Partition's RAM, Kernel RAM, a buffer past the RAM's end, an absent channel; {} expected), {} ended the Job; {} accepted (2 expected); the buffer at {} {} part2's RAM above its stack; SPIM0 after the run: RXD.PTR {} RXD.MAXCNT {} TXD.PTR {} TXD.MAXCNT {} RXD.LIST {} TXD.LIST {} ENABLE {} STARTED {} END {} RXD.AMOUNT {} (the accepted request's pointer and count, nothing else written, no transfer started); sampler at case {} ({} expected: every case and attempt made, then idle Jobs)",
                    rejected.len(), DMA_REFUSED, ended, fmt(v("dma_ok")), ptr.map(|p| format!("{p:#010x}")).unwrap_or("?".into()), if in_ram { "inside" } else { "OUTSIDE" },
                    fmt(v("spim0_rxd_ptr")), fmt(v("spim0_rxd_maxcnt")), fmt(v("spim0_txd_ptr")), fmt(v("spim0_txd_maxcnt")), fmt(v("spim0_rxd_list")), fmt(v("spim0_txd_list")), fmt(v("spim0_enable")), fmt(v("spim0_events_started")), fmt(v("spim0_events_end")), fmt(v("spim0_rxd_amount")),
                    fmt(v("sampler_dma_case")), format!("at least {}", DMA_CASES + count0)
                ),
            );
        }
        // R17.6: the restart of part2 stops its channels before the re-initialization.
        match target.read_words(inputs.target_elf, "ARCH_ACTIONS", &scratch) {
            Ok(Some(w)) if w.len() >= 2 && w[0] == 0x5253_4b41 => {
                let n = (w[1] as usize).min((w.len() - 2) / 2);
                out.actions = (0..n).map(|k| (w[2 + 2 * k], w[3 + 2 * k])).collect();
                let restart = run_events.iter().find(|e| e.kind == PARTITION_RESTART && e.partition == 2).map(|e| e.instant * 1000 / inputs.tick_hz);
                let stop = out.actions.iter().position(|(k, _)| *k == 1);
                let reinit = out.actions.iter().position(|(k, p)| *k == 2 && *p == 2);
                let stops_before = match (stop, reinit) { (Some(s), Some(r)) => s < r, _ => false };
                let stop_reg = out.actions.iter().find(|(k, _)| *k == 1).map(|(_, v)| format!("{v:#010x}")).unwrap_or("none".into());
                out.step("Partition restart stops its EasyDMA channels first (R17.6)", restart.is_some() && stops_before, format!("part2 restarted at {} ms; actions in order: {}; the stop task ({stop_reg}, SPIM0.TASKS_STOP) {} the re-initialization", restart.map(|m| m.to_string()).unwrap_or("never".into()), out.actions.iter().map(|(k, v)| if *k == 1 { format!("stop {v:#x}") } else { format!("reinit part{v}") }).collect::<Vec<_>>().join(", "), if stops_before { "precedes" } else { "DOES NOT PRECEDE" }));
            }
            Ok(_) => out.step("Partition restart stops its EasyDMA channels first (R17.6)", false, "the build has no actions record"),
            Err(e) => out.step("Partition restart stops its EasyDMA channels first (R17.6)", false, e),
        }
        // Every event accounted for: the trace never froze.
        let frozen = ev.frozen_in_run();
        out.step("Kernel events", frozen.is_none(), format!("{}{}", ev.summary(inputs.tick_hz), frozen.map(|s| format!("; the trace FROZE at event {s}")).unwrap_or_default()));
    }
    out.events = events;
    out.faults = faults;
    // The Task-side probes.
    side.filter_fpscr = v("filter_fpscr");
    side.patterns = match (v("filter_pattern_runs"), v("filter_pattern_fail"), v("filter_pattern_mask")) { (Some(r), Some(f), Some(m)) => Some((r, f, m)), _ => None };
    side.srd_used = inputs.model.partitions.iter().any(|p| p.mpu.code.srd != 0 || p.mpu.ram.srd != 0 || p.mpu.windows.iter().any(|w| w.srd != 0));
    side.fp_flush_to_zero = inputs.model.fp_flush_to_zero;
    if let Some((runs, fail, mask)) = side.patterns {
        out.step("register patterns across preemptions (HW-06, HW-10, R7.5)", runs > 0 && fail == 0, format!("{runs} probe runs of filter (FPU Task), {fail} with a changed register, mask {mask:#x}; FPSCR of the first Job {}", v("filter_fpscr").map(|f| format!("{f:#x}")).unwrap_or("?".into())));
    }
    // The differential tests.
    match target.read_words(inputs.target_elf, "DT_RECORD", &scratch) {
        Ok(Some(w)) => match dt::Record::decode(&w) {
            Ok(r) => {
                out.dt_rows = dt::judge(&r, &side);
                let v = dt::verdicts(&out.dt_rows);
                let mism: Vec<&str> = v.iter().filter(|(_, _, x)| *x == dt::Verdict::Mismatch).map(|(r, _, _)| *r).collect();
                out.step("Hardware_Model differential tests (R49.3)", mism.is_empty(), format!("{} rules matched, {} mismatched{}", v.iter().filter(|(_, _, x)| *x == dt::Verdict::Match).count(), mism.len(), if mism.is_empty() { String::new() } else { format!(": {}", mism.join(", ")) }));
                out.dt = Some(r);
            }
            Err(e) => out.step("Hardware_Model differential tests (R49.3)", false, e),
        },
        Ok(None) => out.step("Hardware_Model differential tests (R49.3)", false, "the build has no DT_RECORD (feature `hw-dt`)"),
        Err(e) => out.step("Hardware_Model differential tests (R49.3)", false, e),
    }
    if let Ok(Some(entries)) = target.read_trace(inputs.target_elf, &scratch) {
        out.trace_text = crate::trace::render(&entries, inputs.tick_hz);
    }
    out
}

fn fmt(v: Option<u32>) -> String {
    v.map(|v| v.to_string()).unwrap_or("?".into())
}

/// The isolation report (Markdown).
pub fn render(rig: &Rig, inputs: &Inputs<'_>, out: &Outcome, binary: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# HIL isolation suite: the MPU access matrix and the EasyDMA cases (task 7.4, R16.8, R17.7)\n");
    let _ = writeln!(s, "Verdict: **{}**\n", if out.ok() { "pass" } else { "FAIL" });
    let _ = writeln!(s, "Target {}; binary {binary} (the isolation configuration `nrf52840_isolation`).\n", rig.target.serial);
    let _ = writeln!(s, "## Steps\n\n| Step | Result | Detail |\n|---|---|---|");
    for st in &out.steps {
        let _ = writeln!(s, "| {} | {} | {} |", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('|', "\\|"));
    }
    let _ = writeln!(s, "\n## The access matrix\n");
    for w in &out.walkers {
        let _ = writeln!(s, "### {} (Task {}, Partition {})\n\n| # | Attempt | Address | Expected | Fault recorded |\n|---|---|---|---|---|", w.name, w.task, w.partition);
        for a in &w.attempts {
            let recorded = match (&out.faults, a.address) {
                (Some(f), Some(_)) => f.entries.iter().find(|x| fault_matches(x, a)).map(|x| x.describe()).unwrap_or("**none**".into()),
                _ => "-".into(),
            };
            let _ = writeln!(s, "| {} | {} | {} | {} | {} |", a.index, a.what, a.address.map(|x| format!("{x:#010x}")).unwrap_or("skipped (no window)".into()), if a.address.is_none() { "no attempt".to_string() } else { format!("{} of the Partition, then END_JOB", kind_name(a.kind)) }, recorded);
        }
        let _ = writeln!(s);
    }
    if let Some(ev) = &out.events {
        let _ = writeln!(s, "## The Kernel's events\n\n| Seq | Instant (ms) | Kind | Task | Partition | Detail |\n|---|---|---|---|---|---|");
        for e in ev.run_entries() {
            let _ = writeln!(s, "| {} | {:.3} | {} | {} | {} | {:#x} |", e.seq, e.instant as f64 * 1000.0 / inputs.tick_hz as f64, kind_name(e.kind), if e.task == 0xFF { "-".to_string() } else { e.task.to_string() }, if e.partition == 0xFF { "-".to_string() } else { e.partition.to_string() }, e.detail);
        }
    }
    if let Some(f) = &out.faults {
        let _ = writeln!(s, "\n## The fault record\n\n```\n{}```", f.render());
    }
    if !out.actions.is_empty() {
        let _ = writeln!(s, "\n## The actions record\n\n{}\n", out.actions.iter().map(|(k, v)| if *k == 1 { format!("stop {v:#010x}") } else { format!("reinit part{v}") }).collect::<Vec<_>>().join(", "));
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

    fn model() -> TaskModel {
        let text = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../../apps/p1-demo/gen-nrf52840_isolation/task_model.json")).unwrap();
        rsk_model::read_task_model(&text).unwrap()
    }

    #[test]
    fn the_matrix_follows_the_target_order_and_skips_part0_s_window() {
        let m = model();
        let w = walkers(&m);
        assert_eq!(w.len(), 3);
        assert_eq!(w[0].task, 0);
        assert_eq!(w[1].task, 3);
        assert_eq!(w[2].task, 6);
        let a = &w[1].attempts; // part1 against part0, part2, the Kernel
        assert_eq!(a.len(), 2 * 8 + 7);
        assert_eq!(a[0].address, Some(0x0002_0100));
        assert_eq!(a[2].class, Class::Fetch);
        assert_eq!(a[3].address, Some(0x2000_8000 + 4096 + 0x100));
        assert_eq!(a[5].address, Some(0x2000_8010));
        assert_eq!(a[6].address, None, "part0 owns no peripheral");
        assert_eq!(a[8 + 6].address, Some(0x4000_3000), "part2's SPIM0 window");
        assert_eq!(a[16].address, Some(0x2000_0100));
        assert_eq!(a[19].class, Class::Fetch);
        assert_eq!(a[20].kind, BUSFAULT);
        assert_eq!(a[22].address, Some(0x2001_0000 - 4));
    }

    #[test]
    fn the_judge_wants_one_fault_and_one_end_job_per_attempt() {
        let m = model();
        let w = walkers(&m);
        let mut events = Vec::new();
        let mut faults = Vec::new();
        let mut seq = 0;
        for walker in &w {
            for a in walker.attempts.iter().filter(|a| a.address.is_some()) {
                events.push(EventRec { seq, kind: a.kind, task: walker.task, partition: walker.partition, instant: seq as u64 * 1000, detail: 0 });
                events.push(EventRec { seq: seq + 1, kind: END_JOB, task: walker.task, partition: walker.partition, instant: seq as u64 * 1000, detail: 0 });
                seq += 2;
                let cfsr = match (a.kind, a.class) { (BUSFAULT, _) => 0x8200, (_, Class::Fetch) => 0x01, _ => 0x82 };
                faults.push(Fault { cfsr, address: a.address.unwrap() });
            }
        }
        let (ok, notes, side) = judge_matrix(&w, &events, &faults);
        assert!(ok, "{notes:?}");
        assert_eq!(side.ppb_faults.len(), 6);
        assert_eq!(side.stack_overflow_refused, Some(true));
        // One attempt that returned (no event, no fault) fails it.
        let dropped: Vec<EventRec> = events.iter().skip(2).cloned().collect();
        let (ok, _, _) = judge_matrix(&w, &dropped, &faults);
        assert!(!ok);
    }
}
