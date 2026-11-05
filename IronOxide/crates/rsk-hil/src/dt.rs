//! The Hardware_Model differential tests (task 3.3, R49.3): the Kernel's
//! `DT_RECORD` (rsk-kernel feature `hw-dt`: two privileged boot phases
//! probing the NVIC, BASEPRI, PRIMASK, CONTROL, the exception frame, the
//! DWT, and the TIMER compare rule) decoded and compared, rule by rule,
//! with what `rsk_logic::hw_model` predicts; the Task-side probes (the
//! register patterns and FPSCR of an FPU Task, the access matrix and the
//! BusFault record of the isolation suite) complete the rules the boot
//! phases cannot reach. One row per observation; one verdict per rule.
//! A rule's row that no test exercises carries its disposition (R49.3).

use std::fmt::Write;

pub const MAGIC: u32 = 0x5253_4b44;
pub const WORDS: usize = 160;

// The record's word indexes (`rsk_kernel::arch::cm4::unsafe_module::dt`).
const W_FLAGS: usize = 2;
const W_IPR_FF: usize = 3;
const W_PRIGROUP_RESET: usize = 4;
const W_IPR_LEVEL: usize = 5;
const W_BASEPRI: usize = 12;
const W_PRIMASK: usize = 16;
const W_ISPR_MASKED: usize = 17;
const W_LOG_MASKED: usize = 18;
const W_ISPR_BASEPRI: usize = 19;
const W_LOG_BASEPRI: usize = 20;
const W_LOG_UNMASKED: usize = 21;
const W_IABR: usize = 22;
const W_ICSR: usize = 23;
const W_LOG_AT_SVC15: usize = 24;
const W_CONTROL_HANDLER: usize = 25;
const W_ICSR_HANDLER: usize = 26;
const W_CONTROL_AFTER: usize = 27;
const W_FRAME: usize = 28;
const W_FRAME_ADDR: usize = 36;
const W_FRAME_PC: usize = 37;
const W_CYC: usize = 38;
const W_GAP_TAIL: usize = 42;
const W_GAP_SEPARATE: usize = 43;
const W_LOG_COUNT: usize = 44;
const W_LOG: usize = 45;
const LOG_LEN: usize = 16;
const W_COMPARE: usize = 110;
const COMPARE_DISTANCES: [u32; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16];
const W_FPCCR: usize = 134;
const W_FPDSCR: usize = 135;
const W_ACTLR: usize = 136;
const W_CCR: usize = 137;
const W_MPU_CTRL: usize = 138;
const W_PRIGROUP: usize = 139;
const VECTOR_A: u32 = 46;
const VECTOR_B: u32 = 47;

/// The Hardware_Model's constants the predictions use (`hw_model.rs`).
/// Revised from 2 to 16 by the first run of this test (2026-11-02): 0 to 7 ticks never fired, 8 and more always did.
pub const MIN_COMPARE_DISTANCE: u32 = 16;
pub const IMPLEMENTED_PRIORITY_BITS: u32 = 0xE0;
/// `encode_level` of DD-02 and `hw_model::encode_level`.
pub fn encode_level(level: u32) -> u32 {
    if level == 0 { 0 } else { (8 - level) << 5 }
}

/// The decoded record.
#[derive(Clone, Debug)]
pub struct Record {
    pub words: Vec<u32>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LogEntry {
    pub vector: u32,
    pub entry_cycles: u32,
    pub exit_cycles: u32,
    pub control: u32,
}

impl Record {
    pub fn decode(words: &[u32]) -> Result<Record, String> {
        if words.len() < WORDS || words[0] != MAGIC {
            return Err("DT_RECORD: no record (the magic word is absent: the build has no `hw-dt` feature or never ran)".into());
        }
        if words[W_FLAGS] & 1 == 0 || words[W_FLAGS] & 2 == 0 {
            return Err(format!("DT_RECORD: the boot phases did not complete (flags {:#x})", words[W_FLAGS]));
        }
        Ok(Record { words: words[..WORDS].to_vec() })
    }

    pub fn log(&self) -> Vec<LogEntry> {
        let n = (self.words[W_LOG_COUNT] as usize).min(LOG_LEN);
        (0..n)
            .map(|i| {
                let s = W_LOG + 4 * i;
                LogEntry { vector: self.words[s], entry_cycles: self.words[s + 1], exit_cycles: self.words[s + 2], control: self.words[s + 3] }
            })
            .collect()
    }
}

/// What the Task-side probes and the isolation suite saw, for the rules
/// the boot phases cannot exercise.
#[derive(Clone, Debug, Default)]
pub struct TaskSide {
    /// The access matrix's verdict and one-line detail (HW-09).
    pub matrix: Option<(bool, String)>,
    /// The stack-overflow write refused (HW-09, R16.5).
    pub stack_overflow_refused: Option<bool>,
    /// The PPB attempts' BusFaults: (precise, imprecise, BFAR valid, address) each (HW-08, HW-11).
    pub ppb_faults: Vec<(bool, bool, bool, u32)>,
    /// The number of PPB attempts made.
    pub ppb_attempts: usize,
    /// FPSCR as the FPU Task's first Job found it (HW-10).
    pub filter_fpscr: Option<u32>,
    /// The register-pattern probe: runs, failures, the union of mismatch masks (HW-06, HW-10).
    pub patterns: Option<(u32, u32, u32)>,
    /// Whether any MPU view uses subregion-disable bits (HW-09).
    pub srd_used: bool,
    /// The declaration's `fp_flush_to_zero` (FPDSCR.FZ expected set; HW-10).
    pub fp_flush_to_zero: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Match,
    Mismatch,
    /// Not exercised by any test: the row carries the disposition.
    NotTested,
}

#[derive(Clone, Debug)]
pub struct Row {
    pub rule: &'static str,
    pub what: String,
    pub predicted: String,
    pub observed: String,
    pub verdict: Verdict,
}

fn row(rule: &'static str, what: &str, predicted: impl Into<String>, observed: impl Into<String>, ok: bool) -> Row {
    Row { rule, what: what.to_string(), predicted: predicted.into(), observed: observed.into(), verdict: if ok { Verdict::Match } else { Verdict::Mismatch } }
}

fn untested(rule: &'static str, what: &str, predicted: impl Into<String>, disposition: impl Into<String>) -> Row {
    Row { rule, what: what.to_string(), predicted: predicted.into(), observed: disposition.into(), verdict: Verdict::NotTested }
}

/// The rules, in order, with their one-line names.
pub const RULES: [(&str, &str); 14] = [
    ("HW-01", "priority encoding, 3 implemented bits, PRIGROUP"),
    ("HW-02", "execution priority"),
    ("HW-03", "BASEPRI and BASEPRI_MAX"),
    ("HW-04", "PRIMASK; CPS and MSR only when privileged"),
    ("HW-05", "NVIC arbitration among pending exceptions"),
    ("HW-06", "exception entry stacks R0-R3, R12, LR, PC, xPSR on the selected stack"),
    ("HW-07", "exception return deactivates; NONBASETHRDENA"),
    ("HW-08", "Handler mode is privileged; Thread privilege is CONTROL.nPRIV"),
    ("HW-09", "MPU region match, overlap precedence, background map, subregions"),
    ("HW-10", "eager FP stacking and FPDSCR defaults"),
    ("HW-11", "precise BusFaults with DISDEFWBUF"),
    ("HW-12", "TIMER compare semantics and the near-counter rule"),
    ("HW-13", "DWT CYCCNT"),
    ("HW-14", "tail-chaining and late arrival"),
];

/// Every row of the differential tests.
pub fn judge(r: &Record, side: &TaskSide) -> Vec<Row> {
    let w = &r.words;
    let log = r.log();
    let hex = |v: u32| format!("{v:#x}");
    let mut rows = Vec::new();
    // HW-01
    rows.push(row("HW-01", "IPR read-back after writing 0xFF", hex(IMPLEMENTED_PRIORITY_BITS), hex(w[W_IPR_FF]), w[W_IPR_FF] == IMPLEMENTED_PRIORITY_BITS));
    rows.push(row("HW-01", "AIRCR.PRIGROUP at reset, then after the Kernel's boot (every implemented bit a group bit)", "0, then 4", format!("{}, then {}", w[W_PRIGROUP_RESET], w[W_PRIGROUP]), w[W_PRIGROUP_RESET] == 0 && w[W_PRIGROUP] == 4));
    let enc: Vec<String> = (1..=7).map(|l| hex(encode_level(l))).collect();
    let got: Vec<String> = (1..=7).map(|l| hex(w[W_IPR_LEVEL + l - 1])).collect();
    rows.push(row("HW-01", "IPR read-back of encode_level(1..7)", enc.join(" "), got.join(" "), (1..=7).all(|l| w[W_IPR_LEVEL + l - 1] == encode_level(l as u32))));
    // HW-02
    rows.push(row("HW-02", "with BASEPRI 0x80: B (0x60) taken, A (0xA0) held pending (ISPR bit 30 set, bit 31 clear)", "log 1 entry; ISPR bit 30 only", format!("log {} entries; ISPR {}", w[W_LOG_BASEPRI], hex(w[W_ISPR_BASEPRI])), w[W_LOG_BASEPRI] == 1 && w[W_ISPR_BASEPRI] & (3 << 30) == (1 << 30)));
    // HW-03
    rows.push(row("HW-03", "BASEPRI after MSR 0x80, MSR BASEPRI_MAX 0xA0, MSR BASEPRI_MAX 0x60, MSR 0", "0x80 0x80 0x60 0x0", format!("{} {} {} {}", hex(w[W_BASEPRI]), hex(w[W_BASEPRI + 1]), hex(w[W_BASEPRI + 2]), hex(w[W_BASEPRI + 3])), w[W_BASEPRI] == 0x80 && w[W_BASEPRI + 1] == 0x80 && w[W_BASEPRI + 2] == 0x60 && w[W_BASEPRI + 3] == 0));
    rows.push(row("HW-03", "BASEPRI 0 lets the held A through", "log 2 entries", format!("log {} entries", w[W_LOG_UNMASKED]), w[W_LOG_UNMASKED] == 2));
    // HW-04
    rows.push(row("HW-04", "PRIMASK read-back after CPSID I; both pended exceptions held (ISPR bits 30 and 31, log empty)", "1; ISPR bits 30 and 31; log 0", format!("{}; ISPR {}; log {}", w[W_PRIMASK], hex(w[W_ISPR_MASKED]), w[W_LOG_MASKED]), w[W_PRIMASK] == 1 && w[W_ISPR_MASKED] & (3 << 30) == (3 << 30) && w[W_LOG_MASKED] == 0));
    rows.push(row("HW-04", "CPSID I from unprivileged Thread mode ignored: PRIMASK still clear at the SVC that follows (an SVC under PRIMASK would escalate to HardFault), and A, held by BASEPRI, not yet taken", "PRIMASK 0; log 8 entries at SVC 15", format!("PRIMASK {}; log {} entries at SVC 15", w[W_LOG_AT_SVC15] >> 16, w[W_LOG_AT_SVC15] & 0xFFFF), w[W_LOG_AT_SVC15] == 8));
    rows.push(untested("HW-04", "MSR PRIMASK/BASEPRI from unprivileged Thread mode ignored", "no effect", "not tested on silicon: the Link_Checker excludes msr/mrs/cps from every Task's code (PR-22, R16.10), so no Job can execute them; the CPSIE I case above is the one the Kernel_Proofs rely on (`lemma_job_cannot_set_mask`)"));
    // HW-05
    let order: Vec<u32> = log.iter().map(|e| e.vector).collect();
    let want = [VECTOR_B, VECTOR_A, VECTOR_B, VECTOR_A, VECTOR_A, VECTOR_B, VECTOR_A, VECTOR_A, VECTOR_A];
    rows.push(row("HW-05", "handler order: test 1 (B more urgent) B A; test 2 (A 0xC0 pended first, B 0x40) B A; test 3 (equal, B pended first) A B (lowest number); test 4 A A; unprivileged test A", format!("{want:?}"), format!("{order:?}"), order == want));
    // HW-06
    let frame: Vec<String> = (0..8).map(|i| hex(w[W_FRAME + i])).collect();
    let patterns_ok = w[W_FRAME] == 0x1111_1111 && w[W_FRAME + 1] == 0x2222_2222 && w[W_FRAME + 2] == 0x3333_3333 && w[W_FRAME + 3] == 0x4444_4444 && w[W_FRAME + 4] == 0xCCCC_CCCC;
    rows.push(row("HW-06", "SVC frame R0-R3, R12 hold the patterns loaded before the SVC", "0x11111111 0x22222222 0x33333333 0x44444444 0xcccccccc", frame[..5].join(" "), patterns_ok));
    let pc_want = w[W_FRAME_PC].wrapping_sub(4);
    rows.push(row("HW-06", "stacked PC is the instruction after the SVC (the PC that instruction reads, less 4); xPSR.T set; frame 8-byte aligned (MSP)", format!("pc {}; T; aligned", hex(pc_want)), format!("pc {}; xpsr {}; frame at {}", frame[6], frame[7], hex(w[W_FRAME_ADDR])), w[W_FRAME + 6] == pc_want && w[W_FRAME + 7] & (1 << 24) != 0 && w[W_FRAME_ADDR] % 8 == 0));
    match side.patterns {
        Some((runs, fail, mask)) => rows.push(row("HW-06", "a Job's R2-R5, R8, R10, R12 preserved across preemptions (Property 2, R7.5): the FPU Task's pattern probe", "0 failures", format!("{runs} runs, {fail} failures, mask {}", hex(mask)), runs > 0 && fail == 0)),
        None => rows.push(untested("HW-06", "register patterns across preemptions", "0 failures", "no Task-side record in this run")),
    }
    // HW-07
    rows.push(row("HW-07", "after the handlers returned: IABR clear, ICSR.VECTACTIVE 0", "0; 0", format!("{}; {}", hex(w[W_IABR]), w[W_ICSR] & 0x1FF), w[W_IABR] == 0 && w[W_ICSR] & 0x1FF == 0));
    rows.push(row("HW-07", "CCR.NONBASETHRDENA after the Kernel's boot", "0", format!("{}", w[W_CCR] & 1), w[W_CCR] & 1 == 0));
    // HW-08
    rows.push(row("HW-08", "the handler of A read CONTROL.nPRIV set (Thread mode unprivileged), read ICSR with VECTACTIVE 46 (privileged in Handler mode), and Thread mode read CONTROL 0 afterwards", "nPRIV 1; VECTACTIVE 46; CONTROL 0", format!("CONTROL {}; VECTACTIVE {}; CONTROL {}", hex(w[W_CONTROL_HANDLER]), w[W_ICSR_HANDLER] & 0x1FF, hex(w[W_CONTROL_AFTER])), w[W_CONTROL_HANDLER] & 1 == 1 && w[W_ICSR_HANDLER] & 0x1FF == VECTOR_A && w[W_CONTROL_AFTER] & 1 == 0));
    if side.ppb_attempts > 0 {
        let all = side.ppb_faults.len() == side.ppb_attempts;
        rows.push(row("HW-08", "a Job's access to the Private Peripheral Bus faults (unprivileged Thread mode)", format!("{} BusFaults", side.ppb_attempts), format!("{} BusFaults", side.ppb_faults.len()), all));
    }
    // HW-09
    match &side.matrix {
        Some((ok, detail)) => rows.push(row("HW-09", "the access matrix (R16.8): every access outside the Partition's view refused", "every attempt a MemManage fault of the Partition", detail.clone(), *ok)),
        None => rows.push(untested("HW-09", "the access matrix", "every attempt refused", "no isolation suite in this run")),
    }
    match side.stack_overflow_refused {
        Some(ok) => rows.push(row("HW-09", "a write to the word below the Partition's stack refused (R16.5)", "MemManage", if ok { "MemManage" } else { "granted" }, ok)),
        None => {}
    }
    rows.push(row("HW-09", "MPU_CTRL after the Kernel's boot: ENABLE and PRIVDEFENA (background map for privileged code only)", "0x5", hex(w[W_MPU_CTRL]), w[W_MPU_CTRL] & 5 == 5));
    if side.srd_used {
        rows.push(row("HW-09", "subregion-disable bits", "a disabled subregion refuses", "exercised by the matrix", true));
    } else {
        rows.push(untested("HW-09", "subregion-disable bits and overlap precedence", "a disabled subregion refuses; the highest-numbered region decides", "not exercised: no view of this Application sets SRD bits (every region is a power-of-two region of exactly its size) and no view has two enabled regions covering one address; the encoding is checked at field level by the Kani harnesses `rasr_encoding_fields` and `rbar_encoding_fields`"));
    }
    // HW-10
    rows.push(row("HW-10", "FPCCR.ASPEN set, FPCCR.LSPEN clear (eager stacking)", "ASPEN 1, LSPEN 0", format!("FPCCR {}", hex(w[W_FPCCR])), w[W_FPCCR] >> 31 == 1 && (w[W_FPCCR] >> 30) & 1 == 0));
    let fpdscr_want: u32 = if side.fp_flush_to_zero { 1 << 24 } else { 0 };
    rows.push(row("HW-10", "FPDSCR as the declaration asks (FZ for `fp_flush_to_zero`, erratum 839676's workaround; the other defaults 0)", hex(fpdscr_want), hex(w[W_FPDSCR]), w[W_FPDSCR] == fpdscr_want));
    match side.filter_fpscr {
        Some(f) => rows.push(row("HW-10", "FPSCR of a new FP context equals FPDSCR", hex(w[W_FPDSCR]), hex(f), f == w[W_FPDSCR])),
        None => rows.push(untested("HW-10", "FPSCR of a new FP context", "FPDSCR", "no Task-side record in this run")),
    }
    match side.patterns {
        Some((runs, fail, mask)) => rows.push(row("HW-10", "S0-S31 preserved across preemptions (the hardware's S0-S15 under eager stacking, the stubs' S16-S31)", "0 failures", format!("{runs} runs, {fail} failures, mask {}", hex(mask)), runs > 0 && fail == 0)),
        None => {}
    }
    // HW-11
    rows.push(row("HW-11", "ACTLR.DISDEFWBUF after the Kernel's boot", "1", format!("{}", (w[W_ACTLR] >> 1) & 1), (w[W_ACTLR] >> 1) & 1 == 1));
    if side.ppb_attempts > 0 {
        let precise = side.ppb_faults.iter().all(|(p, i, v, _)| *p && !*i && *v);
        rows.push(row("HW-11", "every BusFault of the PPB attempts precise, with BFAR valid", format!("{} precise", side.ppb_attempts), format!("{} faults: {}", side.ppb_faults.len(), side.ppb_faults.iter().map(|(p, i, v, a)| format!("{}{}{} at {:#x}", if *p { "PRECISERR" } else { "-" }, if *i { "+IMPRECISERR" } else { "" }, if *v { "+BFARVALID" } else { "" }, a)).collect::<Vec<_>>().join(", ")), precise && side.ppb_faults.len() == side.ppb_attempts));
    } else {
        rows.push(untested("HW-11", "BusFaults precise", "PRECISERR", "no isolation suite in this run"));
    }
    // HW-12
    let mut obs = Vec::new();
    let mut ok12 = true;
    for (k, d) in COMPARE_DISTANCES.iter().enumerate() {
        let fired = w[W_COMPARE + 2 * k] == 1;
        let elapsed = w[W_COMPARE + 2 * k + 1];
        obs.push(format!("d={d}: {} after {elapsed} ticks", if fired { "fired" } else { "no event" }));
        if *d >= MIN_COMPARE_DISTANCE && !fired {
            ok12 = false;
        }
    }
    let smallest = COMPARE_DISTANCES.iter().enumerate().find(|(k, _)| w[W_COMPARE + 2 * k] == 1).map(|(_, d)| d.to_string()).unwrap_or("none".into());
    rows.push(row("HW-12", "COMPARE fires for every distance of MIN_COMPARE_DISTANCE (16, revised from 2 by this test's first run) ticks or more from the counter captured before the write; below it the event is not guaranteed (ASM-19)", "fired for every d >= 16; the model's former 2 refuted (0 to 7 never fire: the capture-to-write path is about 7 ticks)", format!("smallest distance that fired: {smallest} ticks; {}", obs.join("; ")), ok12));
    // HW-13
    let (c0, c1, c2, c3) = (w[W_CYC], w[W_CYC + 1], w[W_CYC + 2], w[W_CYC + 3]);
    let elapsed = |s: u32, e: u32| if e >= s { e - s } else { e.wrapping_sub(s) };
    rows.push(row("HW-13", "CYCCNT counts up between two readings", "second > first by a few cycles", format!("{} then {}", hex(c0), hex(c1)), c1 > c0 && c1 - c0 < 256));
    rows.push(row("HW-13", "CYCCNT wraps at 2^32: a few dozen cycles after writing 0xFFFF_FFF0, two readings have wrapped past zero, count up, and lie within the model's `elapsed_cycles` of the written value", "both below 0x2000; second > first; elapsed from 0xFFFF_FFF0 below 0x2000", format!("{} then {} (elapsed from the write {})", hex(c2), hex(c3), elapsed(0xFFFF_FFF0, c3)), c2 < 0x2000 && c3 > c2 && elapsed(0xFFFF_FFF0, c3) < 0x2000));
    // HW-14
    rows.push(row("HW-14", "tail-chaining: fewer cycles from one handler's exit to the next's entry when the next was already pending than when it is pended after the return", "tail-chained gap below the separate gap", format!("tail-chained {} cycles, separate {} cycles", w[W_GAP_TAIL], w[W_GAP_SEPARATE]), w[W_GAP_TAIL] > 0 && w[W_GAP_TAIL] < w[W_GAP_SEPARATE]));
    rows.push(untested("HW-14", "late arrival (a higher-priority exception replacing the one being entered)", "the stub sees the context actually preempted", "not driven deliberately: the timing window is a few cycles wide and no probe can place an exception in it; observed in the field by the response suite of task 7.5 (a Job's SVC overtaken by the time base, `SVC_STALE`), where the stub's EXC_RETURN and frame described the context the hardware had preempted, as HW-14 states"));
    rows
}

/// One verdict per rule: a mismatch in any row fails the rule; a rule
/// with no matching row is not tested.
pub fn verdicts(rows: &[Row]) -> Vec<(&'static str, &'static str, Verdict)> {
    RULES
        .iter()
        .map(|(rule, name)| {
            let mine: Vec<&Row> = rows.iter().filter(|r| r.rule == *rule).collect();
            let v = if mine.iter().any(|r| r.verdict == Verdict::Mismatch) {
                Verdict::Mismatch
            } else if mine.iter().any(|r| r.verdict == Verdict::Match) {
                Verdict::Match
            } else {
                Verdict::NotTested
            };
            (*rule, *name, v)
        })
        .collect()
}

pub fn render(r: &Record, rows: &[Row], target: &str, binary: &str) -> String {
    let mut s = String::new();
    let _ = writeln!(s, "# Hardware_Model differential tests on the nRF52840-DK (task 3.3, R49.3)\n");
    let v = verdicts(rows);
    let mism = v.iter().filter(|(_, _, x)| *x == Verdict::Mismatch).count();
    let _ = writeln!(s, "Verdict: **{}** ({} rules matched, {} mismatched, {} with untested parts)\n", if mism == 0 { "pass" } else { "FAIL" }, v.iter().filter(|(_, _, x)| *x == Verdict::Match).count(), mism, rows.iter().filter(|r| r.verdict == Verdict::NotTested).count());
    let _ = writeln!(s, "Target {target}; binary {binary}. The record `DT_RECORD` is written by the Kernel's two boot phases (feature `hw-dt`); the Task-side rows come from the isolation configuration's probes and the access matrix.\n");
    let _ = writeln!(s, "## Per rule\n\n| Rule | Behaviour | Verdict |\n|---|---|---|");
    for (rule, name, x) in &v {
        let _ = writeln!(s, "| {rule} | {name} | {} |", match x { Verdict::Match => "match", Verdict::Mismatch => "**MISMATCH**", Verdict::NotTested => "not tested" });
    }
    let _ = writeln!(s, "\n## Observations\n\n| Rule | Test | Predicted (Hardware_Model) | Observed (Target) | Result |\n|---|---|---|---|---|");
    for r in rows {
        let _ = writeln!(s, "| {} | {} | {} | {} | {} |", r.rule, r.what.replace('|', "\\|"), r.predicted.replace('|', "\\|"), r.observed.replace('|', "\\|"), match r.verdict { Verdict::Match => "match", Verdict::Mismatch => "**MISMATCH**", Verdict::NotTested => "not tested (disposition in the observed column)" });
    }
    let _ = writeln!(s, "\n## The handler log\n\n| # | Vector | CYCCNT at entry | at exit | CONTROL |\n|---|---|---|---|---|");
    for (i, e) in r.log().iter().enumerate() {
        let _ = writeln!(s, "| {i} | {} | {} | {} | {:#x} |", e.vector, e.entry_cycles, e.exit_cycles, e.control);
    }
    let _ = writeln!(s, "\n## The record\n\n```");
    for (i, chunk) in r.words.chunks(8).enumerate() {
        let _ = writeln!(s, "{:3}: {}", i * 8, chunk.iter().map(|w| format!("{w:08x}")).collect::<Vec<_>>().join(" "));
    }
    let _ = writeln!(s, "```");
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    fn good() -> Record {
        let mut w = vec![0u32; WORDS];
        w[0] = MAGIC; w[1] = 1; w[W_FLAGS] = 3;
        w[W_IPR_FF] = 0xE0; w[W_PRIGROUP_RESET] = 0; w[W_PRIGROUP] = 4;
        for l in 1..=7 { w[W_IPR_LEVEL + l - 1] = encode_level(l as u32); }
        w[W_BASEPRI] = 0x80; w[W_BASEPRI + 1] = 0x80; w[W_BASEPRI + 2] = 0x60; w[W_BASEPRI + 3] = 0;
        w[W_PRIMASK] = 1; w[W_ISPR_MASKED] = 3 << 30; w[W_LOG_MASKED] = 0;
        w[W_ISPR_BASEPRI] = 1 << 30; w[W_LOG_BASEPRI] = 1; w[W_LOG_UNMASKED] = 2;
        w[W_IABR] = 0; w[W_ICSR] = 0; w[W_LOG_AT_SVC15] = 8;
        w[W_CONTROL_HANDLER] = 1; w[W_ICSR_HANDLER] = 46; w[W_CONTROL_AFTER] = 0;
        w[W_FRAME] = 0x1111_1111; w[W_FRAME + 1] = 0x2222_2222; w[W_FRAME + 2] = 0x3333_3333; w[W_FRAME + 3] = 0x4444_4444; w[W_FRAME + 4] = 0xCCCC_CCCC;
        w[W_FRAME + 6] = 0x1234; w[W_FRAME_PC] = 0x1238; w[W_FRAME + 7] = 1 << 24; w[W_FRAME_ADDR] = 0x2000_7FC0;
        w[W_CYC] = 100; w[W_CYC + 1] = 110; w[W_CYC + 2] = 0x40; w[W_CYC + 3] = 0x44;
        w[W_GAP_TAIL] = 40; w[W_GAP_SEPARATE] = 70;
        let order = [47, 46, 47, 46, 46, 47, 46, 46, 46];
        w[W_LOG_COUNT] = order.len() as u32;
        for (i, v) in order.iter().enumerate() { w[W_LOG + 4 * i] = *v; }
        for k in 0..COMPARE_DISTANCES.len() { w[W_COMPARE + 2 * k] = if COMPARE_DISTANCES[k] >= 8 { 1 } else { 0 }; }
        w[W_FPCCR] = 1 << 31; w[W_FPDSCR] = 0; w[W_ACTLR] = 2; w[W_CCR] = 0x200; w[W_MPU_CTRL] = 5;
        Record::decode(&w).unwrap()
    }

    #[test]
    fn a_record_matching_the_model_passes_every_rule() {
        let r = good();
        let side = TaskSide { matrix: Some((true, "69 attempts refused".into())), stack_overflow_refused: Some(true), ppb_faults: vec![(true, false, true, 0xE000_E100); 6], ppb_attempts: 6, filter_fpscr: Some(0), patterns: Some((100, 0, 0)), srd_used: false, fp_flush_to_zero: false };
        let rows = judge(&r, &side);
        let v = verdicts(&rows);
        assert!(v.iter().all(|(_, _, x)| *x != Verdict::Mismatch), "{:?}", rows.iter().filter(|r| r.verdict == Verdict::Mismatch).collect::<Vec<_>>());
        assert_eq!(v.iter().filter(|(_, _, x)| *x == Verdict::Match).count(), 14);
    }

    #[test]
    fn a_wrong_arbitration_order_fails_hw_05_only() {
        let mut r = good();
        r.words[W_LOG + 4 * 4] = 47;
        r.words[W_LOG + 4 * 5] = 46;
        let rows = judge(&r, &TaskSide::default());
        let v = verdicts(&rows);
        assert_eq!(v.iter().filter(|(_, _, x)| *x == Verdict::Mismatch).map(|(r, _, _)| *r).collect::<Vec<_>>(), vec!["HW-05"]);
    }
}
