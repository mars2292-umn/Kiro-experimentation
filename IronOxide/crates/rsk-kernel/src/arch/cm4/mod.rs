//! Kernel_Arch for the Cortex-M4F (DD-01, DD-02, DD-04, DD-07, DD-12; task
//! 5.1): the priority map, the dispatch of unprivileged Thread-mode Jobs
//! through their interrupt vectors, the SVC services, the unwind that
//! ends a Job, the timer and intake handlers, and the fault handlers.
//! Register access and assembly live in [`unsafe_module`]; everything here
//! is safe Rust that plans the hardware actions.
//!
//! Dispatch (DD-01, as revised by the P1 spike of 2026-10-08). Each Task
//! owns an interrupt vector whose NVIC priority encodes the Task's
//! Priority (HW-01); the NVIC pending bits are the ready queue and NVIC
//! arbitration picks the highest eligible level (HW-05,
//! `hw_model::lemma_nvic_takes_iff_eligible`). The vector's handler is the
//! shared dispatch stub (`unsafe_module::stubs::rsk_irq_entry`): it holds
//! PRIMASK, pushes R3-R11 and LR (and S16-S31 of a floating-point
//! context) on the MSP, and calls [`irq_entry`], which charges the
//! preempted Job, records the preempted context in the level record,
//! applies the Kernel transition, loads the Partition's MPU view, and
//! prepares the new Job's initial exception frame on the Partition stack.
//! The stub then returns to unprivileged Thread mode on the PSP. The
//! exception return deactivates the vector (an exception return always
//! deactivates the returning exception, HW-07), so while a Job executes no
//! exception is active, and the Job's level is enforced by BASEPRI: the
//! Kernel keeps BASEPRI at the level of the executing Job or the
//! System_Ceiling, whichever is higher (HW-03), which masks every dispatch
//! vector that the SRP start condition excludes and never the Kernel level
//! (`unsafe_module::kani::uj_010_basepri_never_masks_kernel_level`). A
//! stale pending bit (a Job suppressed after it was pended) is rejected at
//! entry by `eligible`.
//!
//! Job end (the unwind). A Job returns into the trampoline, which executes
//! the job-end SVC. The SVC handler (Kernel level) applies `job_return`
//! and names the ended Job's level; the handler's assembly pops its own
//! pushes and enters `rsk_unwind`, which calls [`epilogue`] for the level:
//! it restores the preempted context from the level record (PSP, CONTROL,
//! EXC_RETURN, the Partition view, BASEPRI); the assembly pops the
//! R3-R11/LR record that the dispatch stub left on the MSP and performs the
//! exception return with the preempted context's EXC_RETURN, which resumes
//! it. When the resumed Job was killed by the Health_Monitor, `epilogue`
//! ends it too and the unwind continues one level further down. The same
//! mechanism serves the timer, intake and fault handlers when a response
//! ends the executing Job.
//!
//! Stacks. Each Partition has one stack region at the bottom of its RAM;
//! the Jobs of a Partition nest last-in, first-out on it, so a Job's
//! initial frame goes below the live stack pointer of the deepest Job of
//! its Partition in progress (`floor`, updated from the preempted PSP at
//! every dispatch). The Kernel runs on the MSP in Kernel RAM.
// An ancestor of the Kernel_Unsafe_Module: `forbid` would propagate into it,
// so this module denies `unsafe_code` and contains none (R6.3).
#![deny(unsafe_code)]

pub mod unsafe_module;

use crate::arch::{Binding, KernelOps, System};
use crate::hw_model::MIN_COMPARE_DISTANCE;
use crate::logic::log::{EV_BUSFAULT, EV_HARDFAULT, EV_MEMMANAGE, EV_PANIC, EV_UNBOUND_IRQ, EV_USAGEFAULT};
pub use unsafe_module::KernelCell;
use unsafe_module::hw;

/// Trace one entry point with a tag and a value (feature `trace`).
#[inline(always)]
fn trace(tag: &[u8], v: u32) {
    #[cfg(feature = "trace")]
    {
        let mut buf = [0u8; 24];
        let mut n = 0;
        buf[n] = b'[';
        n += 1;
        for b in tag {
            buf[n] = *b;
            n += 1;
        }
        buf[n] = b':';
        n += 1;
        let hex = b"0123456789abcdef";
        let mut i = 0;
        while i < 8 {
            buf[n] = hex[((v >> (28 - 4 * i)) & 0xF) as usize];
            n += 1;
            i += 1;
        }
        buf[n] = b']';
        n += 1;
        hw::semihosting_write(&buf[..n]);
    }
    #[cfg(not(feature = "trace"))]
    {
        let _ = (tag, v);
    }
}

/// NVIC priority encoding of an rsk level: level 0 (Kernel) is 0x00; a
/// Task of Priority p is (8 - p) << 5 (HW-01, DD-02).
pub const fn encode_level(level: u8) -> u8 {
    if level == 0 { 0 } else { (8 - level) << 5 }
}

/// BASEPRI for a System_Ceiling (HW-03): 0 for the idle level.
pub const fn basepri_for_ceiling(c: u8) -> u8 {
    if c == 0 { 0 } else { encode_level(c) }
}

/// BASEPRI while a Job of Priority `top` executes under System_Ceiling
/// `ceiling` (DD-01): the higher of the two levels, so that only the
/// dispatch vectors of Jobs that may preempt (strictly higher Priority
/// than both) and the Kernel level stay open.
pub const fn basepri_for(top: u8, ceiling: u8) -> u8 {
    basepri_for_ceiling(if top > ceiling { top } else { ceiling })
}

/// The BASEPRI the Kernel state calls for: the executing Job's level or
/// the System_Ceiling.
fn basepri_now<K: KernelOps>(k: &K) -> u8 {
    let top = match k.top() {
        Some(t) => k.job_info(t).priority,
        None => 0,
    };
    basepri_for(top, k.ceiling())
}

/// SVC service numbers.
pub const SVC_LOCK: u8 = 1;
pub const SVC_UNLOCK: u8 = 2;
pub const SVC_JOB_END: u8 = 3;
pub const SVC_NOW: u8 = 4;
/// A panic in an unprivileged Job (the panic handler's SVC, PR-19).
pub const SVC_PANIC: u8 = 5;

/// EXC_RETURN values (HW-07).
pub const EXC_RETURN_HANDLER_MSP: u32 = 0xFFFF_FFF1;
pub const EXC_RETURN_THREAD_MSP: u32 = 0xFFFF_FFF9;
pub const EXC_RETURN_THREAD_PSP: u32 = 0xFFFF_FFFD;
/// xPSR with the Thumb bit for a new frame.
pub const XPSR_THUMB: u32 = 1 << 24;
/// Basic exception frame size (HW-06).
pub const FRAME_BYTES: u32 = 32;

/// The saved context of the Job that a level's stub preempted.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct LevelRecord {
    pub exception_number: u16,
    pub task: u8,
    pub partition: u8,
    pub psp: u32,
    pub control: u32,
    pub exc_return: u32,
    /// The Partition stack floor before this Job's frame was placed.
    pub floor_before: u32,
    /// The partition of the preempted Job whose view was loaded, 0xFF for the Kernel/idle.
    pub previous_partition: u8,
}

/// Per-level records (one Job in progress per level) and per-Partition
/// stack floors.
#[derive(Clone, Copy, Debug)]
pub struct ArchState {
    pub levels: [LevelRecord; 8],
    pub floor: [u32; 16],
    /// The loaded MPU view's Partition, 0xFF for the Kernel map.
    pub current_partition: u8,
    /// DWT CYCCNT at the last context switch.
    pub last_switch: u32,
    /// 64-bit extension of the time-base counter (DD-05).
    pub time_hi: u32,
    pub time_last_lo: u32,
    pub booted: bool,
}

impl ArchState {
    pub const fn new() -> ArchState {
        ArchState {
            levels: [LevelRecord {
                exception_number: 0,
                task: 0,
                partition: 0,
                psp: 0,
                control: 0,
                exc_return: 0,
                floor_before: 0,
                previous_partition: 0xFF,
            }; 8],
            floor: [0; 16],
            current_partition: 0xFF,
            last_switch: 0,
            time_hi: 0,
            time_last_lo: 0,
            booted: false,
        }
    }
}

/// The plan the dispatch stub executes after [`irq_entry`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DispatchPlan {
    /// The PSP of the new Job's frame, or 0 to return with the entry EXC_RETURN unchanged.
    pub psp: u32,
    /// CONTROL to install (nPRIV, FPCA cleared).
    pub control: u32,
    pub exc_return: u32,
    /// With `psp == 0`: the unwind plan when the handler ended the
    /// executing Job (its level's exception number), else 0.
    pub hop: ReturnPlan,
}

/// The plan after an SVC, timer or fault handler: 0 returns normally;
/// otherwise the exception number of the ended Job's level, which
/// `rsk_unwind` unwinds (the dispatch stub's pushes of that level are the
/// MSP top once the handler popped its own).
pub type ReturnPlan = u32;

/// Cycles consumed by the executing context since the last switch
/// (HW-13), updating the switch point.
fn charge_point(state: &mut ArchState) -> u32 {
    let now = hw::dwt_cyccnt();
    let elapsed = now.wrapping_sub(state.last_switch);
    state.last_switch = now;
    elapsed
}

/// Applies the Kernel's hardware-facing state after a transition: BASEPRI
/// from the System_Ceiling, the compare of the next timed event, the
/// pending bit of the selected Job's vector, source masks, and Partition
/// requests.
fn apply<S: System>(k: &mut S::K) {
    hw::write_basepri(basepri_now(k));
    if let Some(at) = k.next_compare() {
        arm_compare(at);
    }
    if let Some(t) = k.select() {
        hw::nvic_set_pending(S::dispatch_vector(t));
    }
    let mut t = 0u8;
    while (t as usize) < k.task_count() {
        if let Some(v) = S::intake_vector(t) {
            if k.source_masked(t) {
                hw::nvic_disable(v);
            } else {
                hw::nvic_enable(v);
            }
        }
        t = t.wrapping_add(1);
    }
    let mut p = 0u8;
    while (p as usize) < k.partition_count() {
        if k.take_stop_dma(p) {
            // EasyDMA stop (DD-08) is a Partition-specific driver action
            // performed by the system crate's re-initialization; the
            // request is recorded for it. Nothing else to do here.
        }
        if k.take_reinit(p) {
            // The Partition image is re-initialized by the Reset-like path
            // of the system crate on the next Release (task 10.5); the
            // request is consumed here so that the flag is cleared.
        }
        p = p.wrapping_add(1);
    }
    if k.safe_state() {
        hw::write_basepri(0);
        S::safe_state();
    }
}

/// Arms the time-base compare for `at`, never closer than
/// MIN_COMPARE_DISTANCE ticks (HW-12).
fn arm_compare(at: u64) {
    let now = hw::timebase_now();
    let target = if at < now + MIN_COMPARE_DISTANCE { now + MIN_COMPARE_DISTANCE } else { at };
    trace(b"arm", target as u32);
    hw::timebase_arm(target);
}

/// Boot: priority grouping, Kernel-level priorities, NONBASETHRDENA,
/// precise BusFaults, eager FP stacking, the MPU background map, the
/// time base, and the DWT (R12.4: before any Release). Returns `false`
/// when a boot check fails (R12.1, R12.2); the caller enters the safe
/// state.
pub fn boot<S: System>() -> bool {
    let cell = S::kernel();
    let k = cell.get();
    trace(b"boot", 0);
    if !k.validate() {
        trace(b"invalid", 0);
        return false;
    }
    k.init();
    let mut failed = hw::boot_checks(S::PRIORITY_BITS);
    // R12.3: the checksum of the tables in memory against the recorded one.
    if k.config_checksum() != S::CONFIG_CHECKSUM {
        failed |= 1 << 5;
    }
    if failed != 0 {
        trace(b"bootfail", failed);
        k.boot_failed(failed);
        return false;
    }
    trace(b"checks", 1);
    hw::set_prigroup_all_preempt();
    // No Job executes with an exception active (see the module doc), so
    // NONBASETHRDENA stays at its reset value 0 and every exception return
    // to Thread mode is a return to the base level.
    hw::set_nonbasethrdena(false);
    hw::set_disdefwbuf(true);
    hw::fpu_configure_eager();
    hw::set_kernel_exception_priorities();
    let irqs = S::IRQ_COUNT;
    let mut n: u16 = 16;
    while n < 16 + irqs {
        hw::nvic_disable(n);
        match S::binding(n) {
            Binding::Dispatch(t) => {
                let p = k.job_info(t).priority;
                hw::nvic_set_priority(n, encode_level(p));
                hw::nvic_enable(n);
            }
            Binding::Intake { .. } | Binding::Kernel => {
                hw::nvic_set_priority(n, encode_level(0));
                hw::nvic_enable(n);
            }
            Binding::Unbound => {}
        }
        n += 1;
    }
    let mut p = 0u8;
    while (p as usize) < k.partition_count() {
        unsafe_module::arch_state().floor[p as usize] = S::stack_top(p);
        p = p.wrapping_add(1);
    }
    hw::mpu_enable_background();
    trace(b"mpu", 1);
    hw::timebase_init();
    trace(b"timer", 1);
    hw::dwt_enable();
    unsafe_module::arch_state().booted = true;
    trace(b"booted", 1);
    true
}

/// Instant 0: enables Releases (PR-34) and the compare of the first event.
pub fn start<S: System>() {
    let k = S::kernel().get();
    hw::primask_set();
    unsafe_module::arch_state().last_switch = hw::dwt_cyccnt();
    k.start();
    apply::<S>(k);
    trace(b"start", k.next_compare().unwrap_or(0) as u32);
    hw::primask_clear();
}

/// The Rust half of the dispatch stub, with PRIMASK held.
pub fn irq_entry<S: System>(ipsr: u32, psp: u32, control: u32, exc_return: u32) -> DispatchPlan {
    let no_change = DispatchPlan { psp: 0, control, exc_return, hop: 0 };
    let n = (ipsr & 0x1FF) as u16;
    trace(b"irq", n as u32);
    let k = S::kernel().get();
    let state = unsafe_module::arch_state();
    match S::binding(n) {
        Binding::Dispatch(t) => {
            if !k.eligible(t) {
                // A stale pending bit (the Job was suppressed after it was
                // pended): nothing to dispatch.
                return no_change;
            }
            let preempted = charge_point(state);
            let previous_partition = state.current_partition;
            if previous_partition != 0xFF && exc_return & 0xC == 0xC {
                // The preempted context is a Job in Thread mode on the PSP:
                // its live stack pointer (below its stacked frame, HW-06)
                // is the floor of its Partition's stack from here on.
                state.floor[previous_partition as usize] = psp;
            }
            k.dispatch(t, preempted);
            let info = k.job_info(t);
            let level = info.priority as usize;
            let floor_before = state.floor[info.partition as usize];
            let frame = (floor_before - FRAME_BYTES) & !7;
            if frame < S::stack_bottom(info.partition) {
                // The Link_Checker's stack bound (R37.2) excludes this; the
                // backstop is a Kernel fault into the safe state (R16.5).
                k.fault(EV_HARDFAULT, true);
                apply::<S>(k);
                return no_change;
            }
            state.levels[level] = LevelRecord {
                exception_number: n,
                task: t,
                partition: info.partition,
                psp,
                control,
                exc_return,
                floor_before,
                previous_partition,
            };
            state.floor[info.partition as usize] = frame;
            let entry = S::task_entry(t) as usize as u32;
            unsafe_module::build_frame(frame, entry, unsafe_module::stubs::job_end_trampoline as *const () as usize as u32);
            hw::mpu_load(k.view(info.partition));
            state.current_partition = info.partition;
            hw::write_basepri(basepri_for(info.priority, k.ceiling()));
            if let Some(at) = k.next_compare() {
                arm_compare(at);
            }
            hw::fpu_set_access(info.fpu);
            trace(b"dispatch", (t as u32) << 24 | frame & 0x00FF_FFFF);
            DispatchPlan {
                psp: frame,
                // nPRIV = 1 (unprivileged Thread mode), FPCA = 0 (fresh FP context).
                control: 0b001,
                exc_return: EXC_RETURN_THREAD_PSP,
                hop: 0,
            }
        }
        Binding::Intake { task, partition } => {
            let now = hw::timebase_now();
            hw::nvic_disable(n);
            let top_before = k.top();
            k.intake(task, partition, now);
            apply::<S>(k);
            DispatchPlan { hop: end_plan::<S>(top_before), ..no_change }
        }
        Binding::Kernel => {
            let hop = if n == S::TIMER_VECTOR { timer::<S>() } else { 0 };
            DispatchPlan { hop, ..no_change }
        }
        Binding::Unbound => {
            hw::nvic_disable(n);
            let top_before = k.top();
            k.fault(EV_UNBOUND_IRQ, true);
            apply::<S>(k);
            DispatchPlan { hop: end_plan::<S>(top_before), ..no_change }
        }
    }
}

/// The Kernel-level timer handler body (also reached through `irq_entry`
/// when the timer line is a dispatch-stub vector). Returns the two-step
/// return plan when the executing Job was ended by a response.
pub fn timer<S: System>() -> ReturnPlan {
    let k = S::kernel().get();
    let state = unsafe_module::arch_state();
    hw::timebase_ack();
    let now = hw::timebase_now();
    trace(b"tick", now as u32);
    let running = charge_point(state);
    let top_before = k.top();
    k.timer_event(now, running);
    apply::<S>(k);
    end_plan::<S>(top_before)
}

/// After a transition that may have ended the executing Job: the unwind
/// plan naming its level's exception number, or 0.
fn end_plan<S: System>(top_before: Option<u8>) -> ReturnPlan {
    let k = S::kernel().get();
    let state = unsafe_module::arch_state();
    match top_before {
        Some(t) if k.top() != Some(t) => {
            let level = k.job_info(t).priority as usize;
            state.levels[level].exception_number as u32
        }
        _ => 0,
    }
}

/// The Rust half of the SVC handler: `frame` points at the caller's
/// exception frame (R0-R3 at offsets 0 to 12, PC at 24).
pub fn svc<S: System>(frame: &mut [u32; 8]) -> ReturnPlan {
    let k = S::kernel().get();
    let state = unsafe_module::arch_state();
    let number = unsafe_module::svc_number(frame[6]);
    trace(b"svc", number as u32);
    match number {
        SVC_LOCK => {
            let ok = k.svc_lock(frame[0] as u8);
            frame[0] = ok as u32;
            hw::write_basepri(basepri_now(k));
            0
        }
        SVC_UNLOCK => {
            let ok = k.svc_unlock(frame[0] as u8);
            frame[0] = ok as u32;
            apply::<S>(k);
            0
        }
        SVC_NOW => {
            let now = hw::timebase_now();
            frame[0] = now as u32;
            frame[1] = (now >> 32) as u32;
            0
        }
        SVC_JOB_END => {
            let elapsed = charge_point(state);
            let top_before = k.top();
            k.job_return(elapsed);
            apply::<S>(k);
            end_plan::<S>(top_before)
        }
        _ => {
            k.fault(EV_PANIC, false);
            apply::<S>(k);
            let top_before = k.top();
            end_plan::<S>(top_before)
        }
    }
}

/// The Rust half of the unwind of one level, named by its exception
/// number `n`: restores the preempted context recorded for the level and
/// returns (PSP, CONTROL, EXC_RETURN, next), where `next` names one more
/// level to unwind when the resumed Job was killed, else 0.
pub fn epilogue<S: System>(n: u32) -> (u32, u32, u32, ReturnPlan) {
    let k = S::kernel().get();
    let state = unsafe_module::arch_state();
    let n = (n & 0x1FF) as u16;
    trace(b"epilogue", n as u32);
    // Find the level record of this vector.
    let mut level = 0usize;
    let mut i = 1usize;
    while i < 8 {
        if state.levels[i].exception_number == n {
            level = i;
        }
        i += 1;
    }
    let rec = state.levels[level];
    state.floor[rec.partition as usize] = rec.floor_before;
    state.last_switch = hw::dwt_cyccnt();
    // Restore the preempted Job's view.
    if rec.previous_partition != 0xFF {
        hw::mpu_load(k.view(rec.previous_partition));
        hw::fpu_set_access(k.job_info(k.stack_at(k.depth().saturating_sub(1))).fpu);
    }
    state.current_partition = rec.previous_partition;
    // A killed resumed Job ends now and its level is unwound as well.
    if let Some(t) = k.top() {
        if k.is_killed(t) {
            k.job_return(0);
            apply::<S>(k);
            let l = k.job_info(t).priority as usize;
            let next = state.levels[l].exception_number as u32;
            return (rec.psp, rec.control, rec.exc_return, next);
        }
    }
    hw::write_basepri(basepri_now(k));
    (rec.psp, rec.control, rec.exc_return, 0)
}

/// Fault handlers (R14.3): a fault while a Job executes is attributed to
/// its Partition, a fault at the Kernel level or with PRIMASK held, or any
/// HardFault, to the Kernel.
pub fn fault<S: System>(kind: u8, in_kernel: bool) -> ReturnPlan {
    trace(b"fault", (kind as u32) << 8 | in_kernel as u32);
    let k = S::kernel().get();
    let kernel = in_kernel || kind == EV_HARDFAULT || k.depth() == 0;
    let top_before = k.top();
    k.fault(kind, kernel);
    apply::<S>(k);
    end_plan::<S>(top_before)
}

pub fn mem_manage<S: System>(in_kernel: bool) -> ReturnPlan {
    hw::clear_fault_status();
    fault::<S>(EV_MEMMANAGE, in_kernel)
}

pub fn bus_fault<S: System>(in_kernel: bool) -> ReturnPlan {
    hw::clear_fault_status();
    fault::<S>(EV_BUSFAULT, in_kernel)
}

pub fn usage_fault<S: System>(in_kernel: bool) -> ReturnPlan {
    hw::clear_fault_status();
    fault::<S>(EV_USAGEFAULT, in_kernel)
}

pub fn hard_fault<S: System>() -> ! {
    trace(b"hardfault", hw::mmio_read(unsafe_module::hw::addr::CFSR));
    let k = S::kernel().get();
    k.fault(EV_HARDFAULT, true);
    S::safe_state()
}

/// The panic handler body (PR-19, R14.2): a panic in a Job is a fault of
/// its Partition; one in Kernel code is a Kernel fault.
pub fn panic<S: System>(in_kernel: bool) -> ! {
    if !in_kernel {
        // Unprivileged: report through the SVC; the Kernel ends the Job and
        // never returns here (two-step return), so the loop is unreachable.
        unsafe_module::stubs::svc_panic();
        loop {}
    }
    let k = S::kernel().get();
    k.fault(EV_PANIC, true);
    apply::<S>(k);
    S::safe_state()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn priority_encoding_matches_the_hardware_model() {
        assert_eq!(encode_level(0), 0);
        assert_eq!(encode_level(7), 1 << 5);
        assert_eq!(encode_level(1), 7 << 5);
        for a in 1..=7u8 {
            for b in 1..=7u8 {
                assert_eq!(a < b, encode_level(b) < encode_level(a));
            }
        }
        assert_eq!(basepri_for_ceiling(0), 0);
        assert_eq!(basepri_for_ceiling(5), encode_level(5));
    }
}
