//! The naked assembly entry points of DD-01 (Target only). Every function
//! is justified by review (asm cannot be checked by Kani) and exercised by
//! the QEMU dispatch spike and the HIL dispatch tests of task 3.1.
//!
//! Branches between stubs are always `b` (a Thumb `JUMP24` relocation):
//! `cbz`/`cbnz` have no relocation and a cross-section target assembles
//! silently to a `nop` (found by the P1 spike, see design.md DD-01).
//!
//! Register conventions: the stubs run in Handler mode on the MSP. The
//! Rust halves (`super::super::irq_entry`, `svc`, `epilogue`, `timer`,
//! `fault`) are generic over the system crate's `System` type; the entry
//! crate instantiates them through the `extern "C"` symbols
//! `rsk_irq_entry_rust`, `rsk_svc_rust`, `rsk_epilogue_rust`,
//! `rsk_timer_rust`, `rsk_memmanage_rust`, `rsk_busfault_rust`, and
//! `rsk_usagefault_rust`, which it defines (R24.6: the Kernel never
//! depends on the system crate directly).

use core::arch::naked_asm;

extern "C" {
    /// `irq_entry` of the system crate's `System`: (ipsr, psp, control,
    /// exc_return) -> plan (psp, control, exc_return, hop) through `plan_out`.
    fn rsk_irq_entry_rust(ipsr: u32, psp: u32, control: u32, exc_return: u32, plan_out: *mut [u32; 4]);
    /// `svc`: frame pointer -> ReturnPlan.
    fn rsk_svc_rust(frame: *mut [u32; 8]) -> u32;
    /// `epilogue`: level exception number -> (psp, control, exc_return,
    /// next level to unwind or 0) through `out`.
    fn rsk_epilogue_rust(level: u32, out: *mut [u32; 4]);
    /// `timer` -> ReturnPlan.
    fn rsk_timer_rust() -> u32;
    fn rsk_memmanage_rust(in_kernel: u32) -> u32;
    fn rsk_busfault_rust(in_kernel: u32) -> u32;
    fn rsk_usagefault_rust(in_kernel: u32) -> u32;
}

/// The shared handler of every interrupt vector: dispatch stub, intake, or
/// Kernel line (DD-01, DD-06).
///
/// Entry: exception frame already stacked on PSP (Job) or MSP (idle).
/// Steps: hold PRIMASK; push R3-R11 and LR (ten words, which keeps the
/// MSP 8-byte aligned for the call) and S16-S31 when the preempted context
/// has an active FP context (EXC_RETURN bit 4 clear); call the Rust half
/// with the plan's out pointer as the fifth (stack) argument; if it returns
/// a new PSP, install PSP and CONTROL and return to unprivileged Thread
/// mode (the pushes stay on the MSP as the level's record until the Job
/// ends and `rsk_unwind` pops them); otherwise pop the registers and return
/// as entered, or unwind the level of a Job that this handler ended.
// SAFETY (UJ-020): naked entry with a hand-written frame protocol: the
// pushes are matched by the pops of `rsk_unwind` (job end) or of the
// no-change path below; the Rust half never returns to a different stack.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_irq_entry() {
    naked_asm!(
        ".fpu fpv4-sp-d16",
        "cpsid i",
        "push {{r3-r11, lr}}",
        "tst lr, #0x10",
        "it eq",
        "vpusheq {{s16-s31}}",
        "mrs r0, ipsr",
        "mrs r1, psp",
        "mrs r2, control",
        "mov r3, lr",
        // EXC_RETURN survives the call in a callee-saved register (r8 is
        // restored from the stack by the pop below).
        "mov r8, lr",
        "sub sp, sp, #24",
        "add r4, sp, #8",
        "str r4, [sp, #0]",
        "bl {entry}",
        "ldr r0, [sp, #8]",
        "ldr r1, [sp, #12]",
        "ldr r2, [sp, #16]",
        "ldr r3, [sp, #20]",
        "add sp, sp, #24",
        "cbz r0, 1f",
        // Dispatch: install the Job's PSP and CONTROL, return to Thread mode.
        "msr psp, r0",
        "msr control, r1",
        "isb",
        "dsb",
        "cpsie i",
        "bx r2",
        // No dispatch: restore and return as entered, or unwind the level
        // of a Job that this handler ended (r3 = its exception number).
        "1:",
        "mov r12, r3",
        "tst r8, #0x10",
        "it eq",
        "vpopeq {{s16-s31}}",
        "pop {{r3-r11, lr}}",
        "cmp r12, #0",
        "bne 2f",
        "dsb",
        "cpsie i",
        "bx lr",
        "2:",
        "mov r0, r12",
        "b {unwind}",
        entry = sym rsk_irq_entry_rust,
        unwind = sym rsk_unwind,
    )
}

/// The SVC handler: locates the caller's frame (PSP for Jobs), calls the
/// Rust half, and either returns normally or, when the service ended the
/// executing Job, unwinds that Job's level.
// SAFETY (UJ-022): naked; the frame selection by EXC_RETURN bit 2 is
// HW-06; the pushes are popped before the unwind so that the MSP top is
// the ended level's record.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_svc_entry() {
    naked_asm!(
        "tst lr, #4",
        "ite eq",
        "mrseq r0, msp",
        "mrsne r0, psp",
        "push {{r4, lr}}",
        "bl {svc}",
        "pop {{r4, lr}}",
        "cbz r0, 1f",
        "b {unwind}",
        "1:",
        "dsb",
        "bx lr",
        svc = sym rsk_svc_rust,
        unwind = sym rsk_unwind,
    )
}

/// Ends the Job of a level and resumes the context it preempted (DD-01
/// job end). Entered from a Kernel-level handler (SVC, timer, intake,
/// fault) after that handler popped its own pushes, with r0 = the
/// exception number of the ended Job's level and the MSP top = the pushes
/// of that level's dispatch stub (R3-R11, LR, below them S16-S31 when the
/// preempted context had an FP context). The Rust half supplies the
/// preempted PSP, CONTROL and EXC_RETURN from the level record; the
/// registers are popped and the handler's exception return resumes the
/// preempted context (it deactivates this handler's exception, the only
/// active one, HW-07). When the resumed Job was killed, the Rust half
/// ends it too and names its level, and the loop unwinds one level more.
// SAFETY (UJ-023): naked; the pops match the pushes of `rsk_irq_entry`
// for the levels being unwound, which the LIFO job order guarantees
// (Property 2); the exception return value is the one the preempted
// context was entered with (HW-07).
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_unwind() {
    naked_asm!(
        ".fpu fpv4-sp-d16",
        "cpsid i",
        "1:",
        "sub sp, sp, #16",
        "mov r1, sp",
        "bl {epilogue}",
        "ldr r0, [sp, #0]",
        "ldr r1, [sp, #4]",
        "ldr r2, [sp, #8]",
        "ldr r12, [sp, #12]",
        "add sp, sp, #16",
        // The stub pushed r3-r11 and lr, then [s16-s31 if FP]: pop in reverse.
        "tst r2, #0x10",
        "it eq",
        "vpopeq {{s16-s31}}",
        "pop {{r3-r11, lr}}",
        "msr psp, r0",
        "msr control, r1",
        "isb",
        "cmp r12, #0",
        "bne 2f",
        "dsb",
        "cpsie i",
        "bx r2",
        // The resumed Job was killed: unwind its level as well.
        "2:",
        "mov r0, r12",
        "b 1b",
        epilogue = sym rsk_epilogue_rust,
    )
}

/// The Kernel-level timer handler: returns normally, or unwinds the level
/// of a Job that a response ended.
// SAFETY (UJ-025): as UJ-022.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_timer_entry() {
    naked_asm!(
        "push {{r4, lr}}",
        "bl {timer}",
        "pop {{r4, lr}}",
        "cbz r0, 1f",
        "b {unwind}",
        "1:",
        "dsb",
        "bx lr",
        timer = sym rsk_timer_rust,
        unwind = sym rsk_unwind,
    )
}

/// Fault entries: `in_kernel` is 1 when the fault was taken from Handler
/// mode (EXC_RETURN bit 3 clear) or with PRIMASK set (R14.3).
// SAFETY (UJ-026): as UJ-022.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_memmanage_entry() {
    naked_asm!(
        "push {{r4, lr}}",
        "mrs r0, primask",
        "tst lr, #8",
        "it eq",
        "moveq r0, #1",
        "bl {f}",
        "pop {{r4, lr}}",
        "cbz r0, 1f",
        "b {unwind}",
        "1:",
        "dsb",
        "bx lr",
        f = sym rsk_memmanage_rust,
        unwind = sym rsk_unwind,
    )
}

// SAFETY (UJ-027): as UJ-026.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_busfault_entry() {
    naked_asm!(
        "push {{r4, lr}}",
        "mrs r0, primask",
        "tst lr, #8",
        "it eq",
        "moveq r0, #1",
        "bl {f}",
        "pop {{r4, lr}}",
        "cbz r0, 1f",
        "b {unwind}",
        "1:",
        "dsb",
        "bx lr",
        f = sym rsk_busfault_rust,
        unwind = sym rsk_unwind,
    )
}

// SAFETY (UJ-028): as UJ-026.
#[unsafe(naked)]
#[no_mangle]
pub unsafe extern "C" fn rsk_usagefault_entry() {
    naked_asm!(
        "push {{r4, lr}}",
        "mrs r0, primask",
        "tst lr, #8",
        "it eq",
        "moveq r0, #1",
        "bl {f}",
        "pop {{r4, lr}}",
        "cbz r0, 1f",
        "b {unwind}",
        "1:",
        "dsb",
        "bx lr",
        f = sym rsk_usagefault_rust,
        unwind = sym rsk_unwind,
    )
}

/// The job-end trampoline: the return address of every Job entry. Executes
/// the job-end SVC from unprivileged Thread mode and never continues.
// SAFETY (UJ-029): executes in the shared code region, unprivileged; the
// SVC never returns to it (the handler unwinds the Job's level instead).
#[unsafe(naked)]
#[no_mangle]
#[link_section = ".text.rsk_shared.trampoline"]
pub unsafe extern "C" fn job_end_trampoline() {
    naked_asm!("svc #3", "1:", "b 1b")
}

/// SVC wrappers for the `rsk` API (unprivileged callers). Always inlined
/// into the calling crate so that the code executes from the caller's
/// region, never from Kernel flash.
#[inline(always)]
pub fn svc_lock(r: u8) -> bool {
    let out: u32;
    // SAFETY (UJ-030): the SVC instruction transfers to the Kernel; `r` is
    // passed in r0 and the result returned in r0 per the SVC frame protocol.
    unsafe { core::arch::asm!("svc #1", inout("r0") r as u32 => out, options(nostack)) };
    out != 0
}

#[inline(always)]
pub fn svc_unlock(r: u8) -> bool {
    let out: u32;
    // SAFETY (UJ-031): as UJ-030.
    unsafe { core::arch::asm!("svc #2", inout("r0") r as u32 => out, options(nostack)) };
    out != 0
}

#[inline(always)]
pub fn svc_now() -> u64 {
    let lo: u32;
    let hi: u32;
    // SAFETY (UJ-032): as UJ-030; the 64-bit instant comes back in r0:r1.
    unsafe { core::arch::asm!("svc #4", out("r0") lo, out("r1") hi, options(nostack)) };
    ((hi as u64) << 32) | lo as u64
}

/// The panic SVC of unprivileged Jobs (PR-19).
#[inline(always)]
pub fn svc_panic() {
    // SAFETY (UJ-034): as UJ-030; no result.
    unsafe { core::arch::asm!("svc #5", options(nostack)) };
}
