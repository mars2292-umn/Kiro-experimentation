//! HIL differential tests of the Hardware_Model (task 3.3, R49.3; feature
//! `hw-dt`, HIL builds only, never a Flight_Build feature). Two privileged
//! phases of the boot write their observations into `DT_RECORD`, a word
//! array in `.rsk.noinit` the HIL_Rig reads over the probe after the run
//! and compares, rule by rule, with what `rsk_logic::hw_model` predicts
//! (`rsk-hil::dt`): the early phase runs before `boot_core` (the NVIC and
//! the MPU at their reset state, nothing else enabled) and the late phase
//! after it (the Kernel's own configuration read back, the time base
//! running). Two vectors the Application leaves unbound (IRQ 30 and 31,
//! `VECTOR_A` and `VECTOR_B`) and two SVC numbers the Kernel does not use
//! (14, 15) are the probes' handlers: while the record's active flag is
//! set, `arch::cm4::irq_entry` and `arch::cm4::svc` hand them to [`irq`]
//! and [`svc`] here before any Kernel logic runs and return as entered.
//! The Task-side probes ([`register_patterns`], [`register_patterns_fp`],
//! [`fpscr_read`], [`fetch`]) are inlined into the Partition's code (as
//! `hw::peripheral_read` is) and run unprivileged under its MPU view.
//!
//! The record (word indexes):
//!
//! | Words | Rule | Content |
//! |---|---|---|
//! | 0, 1, 2 | | magic `0x52534b44`, version 1, flags (bit 0 early done, bit 1 late done, bit 31 active) |
//! | 3 | HW-01 | IPR read-back after writing 0xFF (the implemented bits) |
//! | 4 | HW-01 | AIRCR.PRIGROUP at reset |
//! | 5 to 11 | HW-01 | IPR read-back of `encode_level(1)` to `encode_level(7)` |
//! | 12 to 15 | HW-03 | BASEPRI after `MSR BASEPRI 0x80`; after `MSR BASEPRI_MAX 0xA0` (less urgent: unchanged); after `MSR BASEPRI_MAX 0x60` (more urgent: taken); after `MSR BASEPRI 0` |
//! | 16 | HW-04 | PRIMASK read-back after CPSID I |
//! | 17, 18 | HW-04 | ISPR word 0 and the handler-log count with A (0xA0) and B (0x60) pended under PRIMASK |
//! | 19, 20 | HW-02, HW-03 | ISPR word 0 and the log count after CPSIE I with BASEPRI 0x80: B taken, A still pending |
//! | 21 | HW-03 | the log count after BASEPRI 0: A taken |
//! | 22, 23 | HW-07 | IABR word 0 and ICSR after the handlers returned (nothing active) |
//! | 24 | HW-04 | at the entry of SVC 15, issued from unprivileged Thread mode after a CPSID I there: the log count (low half: A, pended and held by BASEPRI, not yet taken) and PRIMASK (high half: 0, the CPSID I ignored) |
//! | 25, 26 | HW-08 | CONTROL as the handler of A read it (nPRIV set: Thread mode was unprivileged) and ICSR as that handler read it (a privileged access from Handler mode, VECTACTIVE 46) |
//! | 27 | HW-08 | CONTROL in Thread mode afterwards (the handler cleared nPRIV) |
//! | 28 to 35 | HW-06 | the SVC 14 exception frame: R0 to R3, R12, LR, PC, xPSR (R0 to R3 and R12 loaded with patterns before the SVC) |
//! | 36, 37 | HW-06 | the frame's address (8-byte aligned, on the MSP) and the address of the instruction after the SVC |
//! | 38 to 41 | HW-13 | two CYCCNT readings; two more taken a few dozen cycles after writing 0xFFFF_FFF0 (both wrapped past zero) |
//! | 42, 43 | HW-14 | cycles from the exit of one handler to the entry of the next: A then B pended together (tail-chained), and A pended again after returning to Thread mode |
//! | 44 | | the handler-log count |
//! | 45 to 108 | HW-05 | the handler log: 16 entries of (vector, CYCCNT at entry, CYCCNT at exit, CONTROL) |
//! | 110 to 133 | HW-12 | for compare distances 0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16 ticks: whether COMPARE[2] fired within the poll and the ticks elapsed |
//! | 134 to 139 | HW-10, HW-11, HW-07, HW-09, HW-01 | FPCCR, FPDSCR, ACTLR, CCR, MPU_CTRL, AIRCR.PRIGROUP after `boot_core` |
//!
//! The handler log's expected order: test 1 (BASEPRI mask) B then A;
//! test 2 (priorities 0xC0 and 0x40, A pended first) B then A; test 3
//! (equal priorities 0x80, B pended first) A then B, the lowest exception
//! number first; test 4 A twice; the unprivileged test A once.

use core::cell::UnsafeCell;

use super::hw::{self, addr, mmio_read, mmio_write};

pub const WORDS: usize = 160;
pub const MAGIC: u32 = 0x5253_4b44;
pub const VERSION: u32 = 1;
/// IRQ 30 and 31 of the nRF52840 (exception numbers 46 and 47): unbound
/// in the P1 demo (its dispatch vectors are 36 to 41 and 56, the time
/// base 24, GPIOTE 22).
pub const VECTOR_A: u16 = 46;
pub const VECTOR_B: u16 = 47;
pub const SVC_FRAME: u8 = 14;
pub const SVC_UNMASK: u8 = 15;

pub const W_FLAGS: usize = 2;
pub const W_IPR_FF: usize = 3;
pub const W_PRIGROUP_RESET: usize = 4;
pub const W_IPR_LEVEL: usize = 5;
pub const W_BASEPRI: usize = 12;
pub const W_PRIMASK: usize = 16;
pub const W_ISPR_MASKED: usize = 17;
pub const W_LOG_MASKED: usize = 18;
pub const W_ISPR_BASEPRI: usize = 19;
pub const W_LOG_BASEPRI: usize = 20;
pub const W_LOG_UNMASKED: usize = 21;
pub const W_IABR: usize = 22;
pub const W_ICSR: usize = 23;
pub const W_LOG_AT_SVC15: usize = 24;
pub const W_CONTROL_HANDLER: usize = 25;
pub const W_ICSR_HANDLER: usize = 26;
pub const W_CONTROL_AFTER: usize = 27;
pub const W_FRAME: usize = 28;
pub const W_FRAME_ADDR: usize = 36;
pub const W_FRAME_PC: usize = 37;
pub const W_CYC: usize = 38;
pub const W_GAP_TAIL: usize = 42;
pub const W_GAP_SEPARATE: usize = 43;
pub const W_LOG_COUNT: usize = 44;
pub const W_LOG: usize = 45;
pub const LOG_LEN: usize = 16;
pub const W_COMPARE: usize = 110;
pub const COMPARE_DISTANCES: [u32; 12] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 10, 12, 16];
pub const W_FPCCR: usize = 134;
pub const W_FPDSCR: usize = 135;
pub const W_ACTLR: usize = 136;
pub const W_CCR: usize = 137;
pub const W_MPU_CTRL: usize = 138;
pub const W_PRIGROUP: usize = 139;

const FLAG_EARLY: u32 = 1;
const FLAG_LATE: u32 = 2;
const FLAG_ACTIVE: u32 = 1 << 31;

const NVIC_IABR: u32 = 0xE000_E300;

struct DtCell(UnsafeCell<[u32; WORDS]>);
// SAFETY (UJ-042): as UJ-005; written by the two boot phases (Thread
// mode, nothing else running) and by the probe handlers they trigger
// while the phase waits, read only over the debug port.
unsafe impl Sync for DtCell {}
#[no_mangle]
#[cfg_attr(all(target_arch = "arm", target_os = "none"), link_section = ".rsk.noinit")]
static DT_RECORD: DtCell = DtCell(UnsafeCell::new([0; WORDS]));

fn record() -> &'static mut [u32; WORDS] {
    // SAFETY (UJ-043): as UJ-006; the accesses are those of UJ-042.
    unsafe { &mut *DT_RECORD.0.get() }
}

/// Whether this boot is the reboot the board takes when the probe
/// detaches after a run (a lockup and a watchdog reset, RESETREAS.DOG):
/// the phases then leave the record of the run alone, as the other
/// no-init records do by resetting at instant 0 only, which that reboot
/// never reaches. Read without clearing; `boot_core` reads and clears it.
pub fn reboot_after_run() -> bool {
    #[cfg(not(feature = "board-qemu-mps2"))]
    {
        mmio_read(addr::NRF_RESETREAS) & (1 << 1) != 0
    }
    #[cfg(feature = "board-qemu-mps2")]
    {
        false
    }
}

/// Whether a differential-test phase is in progress (the probe vectors
/// and SVC numbers are then served here).
pub fn active() -> bool {
    record()[W_FLAGS] & FLAG_ACTIVE != 0
}

/// The handler half for the probe vectors: logs the entry and, for the
/// unprivileged test, restores Thread mode's privilege. `None` for any
/// other vector or outside a phase.
pub fn irq(ipsr: u32, control: u32) -> Option<crate::arch::cm4::entry::DispatchPlan> {
    if !active() {
        return None;
    }
    let v = ipsr & 0x1FF;
    if v != VECTOR_A as u32 && v != VECTOR_B as u32 {
        return None;
    }
    let entry = hw::dwt_cyccnt();
    let r = record();
    let n = r[W_LOG_COUNT] as usize;
    let slot = W_LOG + 4 * n;
    if n < LOG_LEN {
        r[slot] = v;
        r[slot + 1] = entry;
        r[slot + 3] = control;
        r[W_LOG_COUNT] = n as u32 + 1;
    }
    if control & 1 != 0 {
        // HW-08: Thread mode was unprivileged; this handler is privileged
        // (the SCS read succeeds) and hands the privilege back.
        r[W_CONTROL_HANDLER] = control;
        r[W_ICSR_HANDLER] = mmio_read(addr::ICSR);
        write_control(0);
    }
    if n < LOG_LEN {
        r[slot + 2] = hw::dwt_cyccnt();
    }
    Some(crate::arch::cm4::entry::DispatchPlan { psp: 0, control: 0, exc_return: 0, hop: 0 })
}

/// The handler half for the probe SVCs: 14 records the exception frame
/// (HW-06); 15 records PRIMASK and clears BASEPRI on behalf of the
/// unprivileged caller (HW-04, HW-08). `None` for any other number or outside a phase.
pub fn svc(number: u8, frame: &mut [u32; 8]) -> Option<u32> {
    if !active() {
        return None;
    }
    let r = record();
    match number {
        SVC_FRAME => {
            let mut i = 0;
            while i < 8 {
                r[W_FRAME + i] = frame[i];
                i += 1;
            }
            r[W_FRAME_ADDR] = frame.as_ptr() as usize as u32;
            Some(0)
        }
        SVC_UNMASK => {
            r[W_LOG_AT_SVC15] = r[W_LOG_COUNT] | (read_primask() << 16);
            hw::write_basepri(0);
            Some(0)
        }
        _ => None,
    }
}

fn ispr0() -> u32 {
    mmio_read(addr::NVIC_ISPR)
}

fn ipr(n: u16) -> u32 {
    let irq = n.saturating_sub(16) as u32;
    (mmio_read(addr::NVIC_IPR + (irq & !3)) >> (8 * (irq % 4))) & 0xFF
}

/// The DD-02 encoding (`arch::cm4::encode_level`): level `l` of 1 to 7
/// in the three implemented bits.
fn encode(level: u32) -> u8 {
    ((8 - level) << 5) as u8
}

/// Lets a pended probe exception be taken: barriers, then a few cycles.
fn settle() {
    hw::barrier();
    let mut i = 0;
    while i < 64 {
        nop();
        i += 1;
    }
    hw::barrier();
}

fn nop() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-044): a NOP has no effect.
        unsafe { core::arch::asm!("nop", options(nomem, nostack, preserves_flags)) }
    }
}

fn log_entry(i: usize) -> (u32, u32, u32) {
    let r = record();
    let s = W_LOG + 4 * i;
    (r[s], r[s + 1], r[s + 2])
}

/// The early phase: before `boot_core`, in privileged Thread mode on the
/// MSP with every interrupt disabled and the MPU off.
pub fn early() {
    if reboot_after_run() {
        return;
    }
    let r = record();
    *r = [0; WORDS];
    r[0] = MAGIC;
    r[1] = VERSION;
    r[W_FLAGS] = FLAG_ACTIVE;
    // HW-13: the cycle counter (enabled here for the log's stamps; boot
    // enables and zeroes it again).
    hw::dwt_enable();
    r[W_CYC] = hw::dwt_cyccnt();
    r[W_CYC + 1] = hw::dwt_cyccnt();
    mmio_write(addr::DWT_CYCCNT, 0xFFFF_FFF0);
    settle();
    r[W_CYC + 2] = hw::dwt_cyccnt();
    r[W_CYC + 3] = hw::dwt_cyccnt();
    mmio_write(addr::DWT_CYCCNT, 0);
    // HW-01: the implemented priority bits and the encoding read back.
    hw::nvic_set_priority(VECTOR_A, 0xFF);
    r[W_IPR_FF] = ipr(VECTOR_A);
    r[W_PRIGROUP_RESET] = (mmio_read(addr::AIRCR) >> 8) & 7;
    let mut l = 1;
    while l <= 7 {
        hw::nvic_set_priority(VECTOR_A, encode(l));
        r[W_IPR_LEVEL + l as usize - 1] = ipr(VECTOR_A);
        l += 1;
    }
    // HW-03: BASEPRI and BASEPRI_MAX.
    hw::write_basepri(0x80);
    r[W_BASEPRI] = hw::read_basepri() as u32;
    basepri_max(0xA0);
    r[W_BASEPRI + 1] = hw::read_basepri() as u32;
    basepri_max(0x60);
    r[W_BASEPRI + 2] = hw::read_basepri() as u32;
    hw::write_basepri(0);
    r[W_BASEPRI + 3] = hw::read_basepri() as u32;
    // Test 1 (HW-02, HW-03, HW-04): A at 0xA0 and B at 0x60 pended under
    // PRIMASK; CPSIE I with BASEPRI 0x80 lets B through and holds A.
    hw::primask_set();
    r[W_PRIMASK] = read_primask();
    hw::nvic_set_priority(VECTOR_A, 0xA0);
    hw::nvic_set_priority(VECTOR_B, 0x60);
    hw::nvic_enable(VECTOR_A);
    hw::nvic_enable(VECTOR_B);
    hw::write_basepri(0x80);
    hw::nvic_set_pending(VECTOR_A);
    hw::nvic_set_pending(VECTOR_B);
    settle();
    r[W_ISPR_MASKED] = ispr0();
    r[W_LOG_MASKED] = r[W_LOG_COUNT];
    hw::primask_clear();
    settle();
    r[W_ISPR_BASEPRI] = ispr0();
    r[W_LOG_BASEPRI] = r[W_LOG_COUNT];
    hw::write_basepri(0);
    settle();
    r[W_LOG_UNMASKED] = r[W_LOG_COUNT];
    // HW-07: after the returns nothing is active.
    r[W_IABR] = mmio_read(NVIC_IABR);
    r[W_ICSR] = mmio_read(addr::ICSR);
    // Test 2 (HW-05): A at 0xC0 pended before B at 0x40; B is taken first.
    hw::primask_set();
    hw::nvic_set_priority(VECTOR_A, 0xC0);
    hw::nvic_set_priority(VECTOR_B, 0x40);
    hw::nvic_set_pending(VECTOR_A);
    hw::nvic_set_pending(VECTOR_B);
    settle();
    hw::primask_clear();
    settle();
    // Test 3 (HW-05, HW-14): equal priorities, B pended before A; A (the
    // lower exception number) is taken first and B tail-chains.
    hw::primask_set();
    hw::nvic_set_priority(VECTOR_A, 0x80);
    hw::nvic_set_priority(VECTOR_B, 0x80);
    hw::nvic_set_pending(VECTOR_B);
    hw::nvic_set_pending(VECTOR_A);
    settle();
    hw::primask_clear();
    settle();
    if r[W_LOG_COUNT] >= 6 {
        let (_, _, exit_a) = log_entry(4);
        let (_, entry_b, _) = log_entry(5);
        r[W_GAP_TAIL] = entry_b.wrapping_sub(exit_a);
    }
    // Test 4 (HW-14): A twice, the second pended after the first returned
    // to Thread mode: a full exception return and entry between them.
    hw::nvic_set_pending(VECTOR_A);
    settle();
    hw::nvic_set_pending(VECTOR_A);
    settle();
    if r[W_LOG_COUNT] >= 8 {
        let (_, _, exit_1) = log_entry(6);
        let (_, entry_2, _) = log_entry(7);
        r[W_GAP_SEPARATE] = entry_2.wrapping_sub(exit_1);
    }
    // HW-04, HW-08: A (0x80) pended and held by BASEPRI 0x80 (not PRIMASK:
    // an SVC issued at execution priority 0 escalates to HardFault, found
    // on the first run); Thread mode made unprivileged; its CPSID I is
    // ignored (were it not, the SVC would escalate); SVC 15 (privileged)
    // records PRIMASK and clears BASEPRI; A is taken and its handler
    // restores the privilege.
    hw::write_basepri(0x80);
    hw::nvic_set_pending(VECTOR_A);
    settle();
    r[W_CONTROL_AFTER] = unprivileged_window();
    settle();
    // HW-06: the exception frame of SVC 14.
    r[W_FRAME_PC] = svc_frame_probe();
    // Done: the probe vectors back to their reset state.
    hw::nvic_disable(VECTOR_A);
    hw::nvic_disable(VECTOR_B);
    hw::nvic_clear_pending(VECTOR_A);
    hw::nvic_clear_pending(VECTOR_B);
    hw::nvic_set_priority(VECTOR_A, 0);
    hw::nvic_set_priority(VECTOR_B, 0);
    hw::write_basepri(0);
    hw::primask_clear();
    r[W_FLAGS] = FLAG_EARLY;
}

/// The late phase: after `boot_core`, before instant 0: the Kernel's
/// configuration read back and the TIMER compare rule (HW-12) on a
/// compare channel the time base does not use.
pub fn late() {
    if reboot_after_run() {
        return;
    }
    let r = record();
    r[W_FPCCR] = mmio_read(addr::FPCCR);
    r[W_FPDSCR] = mmio_read(addr::FPDSCR);
    r[W_ACTLR] = mmio_read(addr::ACTLR);
    r[W_CCR] = mmio_read(addr::CCR);
    r[W_MPU_CTRL] = mmio_read(addr::MPU_CTRL);
    r[W_PRIGROUP] = (mmio_read(addr::AIRCR) >> 8) & 7;
    #[cfg(not(feature = "board-qemu-mps2"))]
    {
        const TASKS_CAPTURE3: u32 = 0x04C;
        const EVENTS_COMPARE2: u32 = 0x148;
        const CC2: u32 = 0x548;
        const CC3: u32 = 0x54C;
        let t = addr::NRF_TIMER0;
        let mut k = 0;
        while k < COMPARE_DISTANCES.len() {
            let d = COMPARE_DISTANCES[k];
            mmio_write(t + EVENTS_COMPARE2, 0);
            mmio_write(t + TASKS_CAPTURE3, 1);
            let now = mmio_read(t + CC3);
            mmio_write(t + CC2, now.wrapping_add(d));
            let mut fired = 0;
            let mut i = 0;
            while i < 4096 {
                if mmio_read(t + EVENTS_COMPARE2) != 0 {
                    fired = 1;
                    break;
                }
                i += 1;
            }
            mmio_write(t + TASKS_CAPTURE3, 1);
            let then = mmio_read(t + CC3);
            r[W_COMPARE + 2 * k] = fired;
            r[W_COMPARE + 2 * k + 1] = then.wrapping_sub(now);
            k += 1;
        }
        mmio_write(t + EVENTS_COMPARE2, 0);
    }
    r[W_FLAGS] |= FLAG_LATE;
}

// --------------------------------------------------------- privileged asm

fn basepri_max(v: u8) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-044): privileged Thread mode of the boot phase; no
        // memory effect (HW-03).
        unsafe { core::arch::asm!("msr BASEPRI_MAX, {0}", in(reg) v as u32, options(nomem, nostack, preserves_flags)) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    let _ = v;
}

fn read_primask() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let v: u32;
        // SAFETY (UJ-044): a read.
        unsafe { core::arch::asm!("mrs {0}, PRIMASK", out(reg) v, options(nomem, nostack, preserves_flags)) }
        v
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        0
    }
}

fn write_control(v: u32) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-044): Handler mode; CONTROL.nPRIV for Thread mode
        // (HW-08); SPSEL writes are ignored in Handler mode.
        unsafe { core::arch::asm!("msr CONTROL, {0}", "isb", in(reg) v, options(nomem, nostack, preserves_flags)) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    let _ = v;
}

/// Thread mode unprivileged (CONTROL.nPRIV), a CPSID I there (ignored,
/// HW-04: PRIMASK stays clear, as SVC 15 records), SVC 15 (served
/// privileged: clears BASEPRI, after which the pended A is taken and its
/// handler clears nPRIV), then CONTROL read back.
fn unprivileged_window() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let c: u32;
        // SAFETY (UJ-045): the boot phase's own privilege transition; the
        // MPU is off (the default map admits unprivileged accesses to the
        // Kernel's flash and RAM), the stack is the MSP in both modes, and
        // the SVC and the probe handler restore every mode bit.
        unsafe {
            core::arch::asm!(
                "msr CONTROL, {np}",
                "isb",
                "cpsid i",
                "nop", "nop", "nop", "nop", "nop", "nop", "nop", "nop",
                "svc #15",
                "isb",
                "mrs {c}, CONTROL",
                np = in(reg) 1u32,
                c = out(reg) c,
                options(nostack),
            )
        }
        c
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        0
    }
}

/// R0 to R3 and R12 loaded with patterns, then SVC 14; returns the
/// address of the instruction after the SVC (the stacked PC, HW-06).
fn svc_frame_probe() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let lbl: u32;
        // SAFETY (UJ-045): the caller-saved registers are declared
        // clobbered; the SVC is served by `svc` above and returns.
        unsafe {
            core::arch::asm!(
                "movw r0, #0x1111", "movt r0, #0x1111",
                "movw r1, #0x2222", "movt r1, #0x2222",
                "movw r2, #0x3333", "movt r2, #0x3333",
                "movw r3, #0x4444", "movt r3, #0x4444",
                "movw r12, #0xcccc", "movt r12, #0xcccc",
                "adr {lbl}, 2f",
                "svc #14",
                "2:",
                lbl = out(reg) lbl,
                out("r0") _, out("r1") _, out("r2") _, out("r3") _, out("r12") _,
                options(nostack),
            )
        }
        lbl
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        0
    }
}

// --------------------------------------------------- Task-side probes

/// HW-10: FPSCR as the Job finds it (its new FP context takes FPDSCR's
/// value at the Job's first FP instruction). For FPU Tasks only (PR-31).
#[inline(always)]
pub fn fpscr_read() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let v: u32;
        // SAFETY (UJ-046): an FPSCR read, unprivileged; no memory effect.
        unsafe { core::arch::asm!("vmrs {0}, fpscr", out(reg) v, options(nomem, nostack, preserves_flags)) }
        v
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        0
    }
}

/// HW-06, Property 2 (R7.5): loads R2 to R5, R8, R10, and R12 with
/// patterns, spins `iterations` (so that the Kernel preempts and resumes
/// the Job meanwhile), and compares: bit i set for the i-th register that
/// changed. 0 when every register came back as it was.
#[inline(always)]
pub fn register_patterns(iterations: u32) -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let mask: u32;
        // SAFETY (UJ-046): every register the block touches is declared;
        // no memory access; the loop is bounded by `iterations`.
        unsafe {
            core::arch::asm!(
            "movw r2, #0x2222",
            "movt r2, #0x2222",
            "movw r3, #0x3333",
            "movt r3, #0x3333",
            "movw r4, #0x4444",
            "movt r4, #0x4444",
            "movw r5, #0x5555",
            "movt r5, #0x5555",
            "movw r8, #0x8888",
            "movt r8, #0x8888",
            "movw r10, #0xaaaa",
            "movt r10, #0xaaaa",
            "movw r12, #0xcccc",
            "movt r12, #0xcccc",
            "2:",
            "subs {n}, {n}, #1",
            "bne 2b",
            "mov r0, #0",
            "movw r1, #0x2222",
            "movt r1, #0x2222",
            "cmp r2, r1",
            "it ne",
            "orrne r0, r0, #0x1",
            "movw r1, #0x3333",
            "movt r1, #0x3333",
            "cmp r3, r1",
            "it ne",
            "orrne r0, r0, #0x2",
            "movw r1, #0x4444",
            "movt r1, #0x4444",
            "cmp r4, r1",
            "it ne",
            "orrne r0, r0, #0x4",
            "movw r1, #0x5555",
            "movt r1, #0x5555",
            "cmp r5, r1",
            "it ne",
            "orrne r0, r0, #0x8",
            "movw r1, #0x8888",
            "movt r1, #0x8888",
            "cmp r8, r1",
            "it ne",
            "orrne r0, r0, #0x10",
            "movw r1, #0xaaaa",
            "movt r1, #0xaaaa",
            "cmp r10, r1",
            "it ne",
            "orrne r0, r0, #0x20",
            "movw r1, #0xcccc",
            "movt r1, #0xcccc",
            "cmp r12, r1",
            "it ne",
            "orrne r0, r0, #0x40",
            n = inout(reg) iterations => _,
            out("r0") mask, out("r1") _, out("r2") _, out("r3") _, out("r4") _, out("r5") _,
            out("r8") _, out("r10") _, out("r12") _,
            options(nomem, nostack),
            )
        }
        mask
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        let _ = iterations;
        0
    }
}

/// HW-10 with HW-06: as [`register_patterns`], with S0 to S31 loaded as
/// well (the Job's FP context: S0 to S15 stacked by the hardware on every
/// exception entry under eager stacking, S16 to S31 by the Kernel's
/// stubs). Bits 0 to 6 the core registers, bits 8 to 23 S0 to S15, bits
/// 24 to 27 the count of S16 to S31 that changed (at most 15). For FPU
/// Tasks only (PR-31).
#[inline(always)]
pub fn register_patterns_fp(iterations: u32) -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let mask: u32;
        // SAFETY (UJ-046): as `register_patterns`, with the FP registers.
        unsafe {
            core::arch::asm!(
            "movw r2, #0x2222",
            "movt r2, #0x2222",
            "movw r3, #0x3333",
            "movt r3, #0x3333",
            "movw r4, #0x4444",
            "movt r4, #0x4444",
            "movw r5, #0x5555",
            "movt r5, #0x5555",
            "movw r8, #0x8888",
            "movt r8, #0x8888",
            "movw r10, #0xaaaa",
            "movt r10, #0xaaaa",
            "movw r12, #0xcccc",
            "movt r12, #0xcccc",
            "movw r1, #0x0",
            "movt r1, #0x5a00",
            "vmov s0, r1",
            "movw r1, #0x1",
            "movt r1, #0x5a01",
            "vmov s1, r1",
            "movw r1, #0x2",
            "movt r1, #0x5a02",
            "vmov s2, r1",
            "movw r1, #0x3",
            "movt r1, #0x5a03",
            "vmov s3, r1",
            "movw r1, #0x4",
            "movt r1, #0x5a04",
            "vmov s4, r1",
            "movw r1, #0x5",
            "movt r1, #0x5a05",
            "vmov s5, r1",
            "movw r1, #0x6",
            "movt r1, #0x5a06",
            "vmov s6, r1",
            "movw r1, #0x7",
            "movt r1, #0x5a07",
            "vmov s7, r1",
            "movw r1, #0x8",
            "movt r1, #0x5a08",
            "vmov s8, r1",
            "movw r1, #0x9",
            "movt r1, #0x5a09",
            "vmov s9, r1",
            "movw r1, #0xa",
            "movt r1, #0x5a0a",
            "vmov s10, r1",
            "movw r1, #0xb",
            "movt r1, #0x5a0b",
            "vmov s11, r1",
            "movw r1, #0xc",
            "movt r1, #0x5a0c",
            "vmov s12, r1",
            "movw r1, #0xd",
            "movt r1, #0x5a0d",
            "vmov s13, r1",
            "movw r1, #0xe",
            "movt r1, #0x5a0e",
            "vmov s14, r1",
            "movw r1, #0xf",
            "movt r1, #0x5a0f",
            "vmov s15, r1",
            "movw r1, #0x10",
            "movt r1, #0x5a10",
            "vmov s16, r1",
            "movw r1, #0x11",
            "movt r1, #0x5a11",
            "vmov s17, r1",
            "movw r1, #0x12",
            "movt r1, #0x5a12",
            "vmov s18, r1",
            "movw r1, #0x13",
            "movt r1, #0x5a13",
            "vmov s19, r1",
            "movw r1, #0x14",
            "movt r1, #0x5a14",
            "vmov s20, r1",
            "movw r1, #0x15",
            "movt r1, #0x5a15",
            "vmov s21, r1",
            "movw r1, #0x16",
            "movt r1, #0x5a16",
            "vmov s22, r1",
            "movw r1, #0x17",
            "movt r1, #0x5a17",
            "vmov s23, r1",
            "movw r1, #0x18",
            "movt r1, #0x5a18",
            "vmov s24, r1",
            "movw r1, #0x19",
            "movt r1, #0x5a19",
            "vmov s25, r1",
            "movw r1, #0x1a",
            "movt r1, #0x5a1a",
            "vmov s26, r1",
            "movw r1, #0x1b",
            "movt r1, #0x5a1b",
            "vmov s27, r1",
            "movw r1, #0x1c",
            "movt r1, #0x5a1c",
            "vmov s28, r1",
            "movw r1, #0x1d",
            "movt r1, #0x5a1d",
            "vmov s29, r1",
            "movw r1, #0x1e",
            "movt r1, #0x5a1e",
            "vmov s30, r1",
            "movw r1, #0x1f",
            "movt r1, #0x5a1f",
            "vmov s31, r1",
            "2:",
            "subs {n}, {n}, #1",
            "bne 2b",
            "mov r0, #0",
            "movw r1, #0x2222",
            "movt r1, #0x2222",
            "cmp r2, r1",
            "it ne",
            "orrne r0, r0, #0x1",
            "movw r1, #0x3333",
            "movt r1, #0x3333",
            "cmp r3, r1",
            "it ne",
            "orrne r0, r0, #0x2",
            "movw r1, #0x4444",
            "movt r1, #0x4444",
            "cmp r4, r1",
            "it ne",
            "orrne r0, r0, #0x4",
            "movw r1, #0x5555",
            "movt r1, #0x5555",
            "cmp r5, r1",
            "it ne",
            "orrne r0, r0, #0x8",
            "movw r1, #0x8888",
            "movt r1, #0x8888",
            "cmp r8, r1",
            "it ne",
            "orrne r0, r0, #0x10",
            "movw r1, #0xaaaa",
            "movt r1, #0xaaaa",
            "cmp r10, r1",
            "it ne",
            "orrne r0, r0, #0x20",
            "movw r1, #0xcccc",
            "movt r1, #0xcccc",
            "cmp r12, r1",
            "it ne",
            "orrne r0, r0, #0x40",
            "vmov r1, s0",
            "movw r9, #0x0",
            "movt r9, #0x5a00",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x100",
            "vmov r1, s1",
            "movw r9, #0x1",
            "movt r9, #0x5a01",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x200",
            "vmov r1, s2",
            "movw r9, #0x2",
            "movt r9, #0x5a02",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x400",
            "vmov r1, s3",
            "movw r9, #0x3",
            "movt r9, #0x5a03",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x800",
            "vmov r1, s4",
            "movw r9, #0x4",
            "movt r9, #0x5a04",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x1000",
            "vmov r1, s5",
            "movw r9, #0x5",
            "movt r9, #0x5a05",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x2000",
            "vmov r1, s6",
            "movw r9, #0x6",
            "movt r9, #0x5a06",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x4000",
            "vmov r1, s7",
            "movw r9, #0x7",
            "movt r9, #0x5a07",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x8000",
            "vmov r1, s8",
            "movw r9, #0x8",
            "movt r9, #0x5a08",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x10000",
            "vmov r1, s9",
            "movw r9, #0x9",
            "movt r9, #0x5a09",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x20000",
            "vmov r1, s10",
            "movw r9, #0xa",
            "movt r9, #0x5a0a",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x40000",
            "vmov r1, s11",
            "movw r9, #0xb",
            "movt r9, #0x5a0b",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x80000",
            "vmov r1, s12",
            "movw r9, #0xc",
            "movt r9, #0x5a0c",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x100000",
            "vmov r1, s13",
            "movw r9, #0xd",
            "movt r9, #0x5a0d",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x200000",
            "vmov r1, s14",
            "movw r9, #0xe",
            "movt r9, #0x5a0e",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x400000",
            "vmov r1, s15",
            "movw r9, #0xf",
            "movt r9, #0x5a0f",
            "cmp r1, r9",
            "it ne",
            "orrne r0, r0, #0x800000",
            "mov r11, #0",
            "vmov r1, s16",
            "movw r9, #0x10",
            "movt r9, #0x5a10",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s17",
            "movw r9, #0x11",
            "movt r9, #0x5a11",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s18",
            "movw r9, #0x12",
            "movt r9, #0x5a12",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s19",
            "movw r9, #0x13",
            "movt r9, #0x5a13",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s20",
            "movw r9, #0x14",
            "movt r9, #0x5a14",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s21",
            "movw r9, #0x15",
            "movt r9, #0x5a15",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s22",
            "movw r9, #0x16",
            "movt r9, #0x5a16",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s23",
            "movw r9, #0x17",
            "movt r9, #0x5a17",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s24",
            "movw r9, #0x18",
            "movt r9, #0x5a18",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s25",
            "movw r9, #0x19",
            "movt r9, #0x5a19",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s26",
            "movw r9, #0x1a",
            "movt r9, #0x5a1a",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s27",
            "movw r9, #0x1b",
            "movt r9, #0x5a1b",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s28",
            "movw r9, #0x1c",
            "movt r9, #0x5a1c",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s29",
            "movw r9, #0x1d",
            "movt r9, #0x5a1d",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s30",
            "movw r9, #0x1e",
            "movt r9, #0x5a1e",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "vmov r1, s31",
            "movw r9, #0x1f",
            "movt r9, #0x5a1f",
            "cmp r1, r9",
            "it ne",
            "addne r11, r11, #1",
            "cmp r11, #15",
            "it hi",
            "movhi r11, #15",
            "orr r0, r0, r11, lsl #24",
            n = inout(reg) iterations => _,
            out("r0") mask, out("r1") _, out("r2") _, out("r3") _, out("r4") _, out("r5") _,
            out("r8") _, out("r9") _, out("r10") _, out("r11") _, out("r12") _,
            out("s0") _, out("s1") _, out("s2") _, out("s3") _, out("s4") _, out("s5") _, out("s6") _, out("s7") _, out("s8") _, out("s9") _, out("s10") _, out("s11") _, out("s12") _, out("s13") _, out("s14") _, out("s15") _, out("s16") _, out("s17") _, out("s18") _, out("s19") _, out("s20") _, out("s21") _, out("s22") _, out("s23") _, out("s24") _, out("s25") _, out("s26") _, out("s27") _, out("s28") _, out("s29") _, out("s30") _, out("s31") _,
            options(nomem, nostack),
            )
        }
        mask
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        let _ = iterations;
        0
    }
}

/// HW-09 (R16.3): an instruction fetch at `addr` (a call), for the access
/// matrix of the isolation suite: the MPU refuses the fetch before any
/// instruction there executes (IACCVIOL), and the Partition's fault
/// response applies. Returns only if the fetch was granted, which the
/// suite records as a mismatch (the address is then executed as code,
/// which is the failure under test).
#[inline(always)]
pub fn fetch(addr: u32) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-047): a call into an address outside the Partition's
        // view; the MPU decision precedes any execution there. HIL builds
        // only.
        unsafe {
            core::arch::asm!(
                "blx {0}",
                in(reg) addr | 1,
                out("r0") _, out("r1") _, out("r2") _, out("r3") _, out("r12") _, out("lr") _,
                options(nostack),
            )
        }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    let _ = addr;
}
