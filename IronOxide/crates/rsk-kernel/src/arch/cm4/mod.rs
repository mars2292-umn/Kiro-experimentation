//! Kernel_Arch for the Cortex-M4F (DD-01, DD-02, DD-04, DD-07, DD-12; task
//! 5.1): the priority map, the dispatch of unprivileged Thread-mode Jobs
//! through their interrupt vectors, the SVC services, the unwind that
//! ends a Job, the timer and intake handlers, and the fault handlers.
//! Register access and assembly live in [`unsafe_module`]; everything here
//! is safe Rust that plans the hardware actions.
//!
//! Dispatch (DD-01, as revised by the P1 spike of 2026-10-08). Each Task
//! owns an interrupt vector whose NVIC priority encodes the Task's
//! Priority (HW-01). After every Kernel transition, `apply` writes
//! BASEPRI and pends the vector of the Job that the verified
//! `Kernel::select` chooses; NVIC arbitration then takes exactly that
//! vector, whatever other dispatch vectors are pending (HW-05;
//! `hw_model::lemma_dispatch_takes_iff_eligible`,
//! `lemma_dispatch_selects_srp_choice`,
//! `lemma_dispatch_idle_when_none_eligible`). Those lemmas assume what
//! this module does: BASEPRI is [`basepri_for`] of the executing Job's
//! Priority and the System_Ceiling, PRIMASK is clear in Thread mode,
//! [`boot`] programs each vector's priority with [`encode_level`], and a
//! stale pending bit is rejected at entry (below). The vector's handler is the
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
//! (`hw_model::lemma_kernel_level_taken_in_every_state` for the model,
//! `unsafe_module::kani::uj_010_basepri_never_masks_kernel_level` for the
//! values this module computes). A
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

pub mod entry;
pub mod plan;
pub mod unsafe_module;

use core::marker::PhantomData;

pub use entry::{
    Action, Binding, Cm4Hw, DispatchPlan, HwModel, KernelArch, ReturnPlan, Tables, CONTROL_JOB, EXC_RETURN_HANDLER_MSP,
    EXC_RETURN_THREAD_MSP, EXC_RETURN_THREAD_PSP, SVC_JOB_END, SVC_LOCK, SVC_NOW, SVC_PANIC, SVC_UNLOCK, XPSR_THUMB,
};
pub use plan::{compare_target, find_level, plan_frame, ArchState, EndpointCfg, LevelRecord, FRAME_BYTES, MAX_INSTANT, MAX_ENDPOINTS};
pub use unsafe_module::KernelCell;

use crate::arch::{KernelOps, System};
use crate::logic::log::{EV_BUSFAULT, EV_HARDFAULT, EV_MEMMANAGE, EV_PANIC, EV_USAGEFAULT};
use unsafe_module::hw;

/// Trace one entry point with a tag and a value (feature `trace`: the
/// semihosting console; feature `trace-ram`: the RAM ring the HIL_Rig
/// reads, as `tag << 24 | value & 0xFF_FFFF` with the time base).
#[inline(always)]
fn trace(tag: &[u8], v: u32) {
    #[cfg(feature = "trace-ram")]
    {
        let id = match tag {
            b"boot" => entry::TR_BOOT,
            b"invalid" => entry::TR_INVALID,
            b"bootfail" => entry::TR_BOOTFAIL,
            b"booted" => entry::TR_BOOTED,
            b"start" => entry::TR_START,
            b"irq" => entry::TR_IRQ,
            b"dispatch" => entry::TR_DISPATCH,
            b"tick" => entry::TR_TICK,
            b"svc" => entry::TR_SVC,
            b"epilogue" => entry::TR_EPILOGUE,
            b"fault" => entry::TR_FAULT,
            b"arm" => entry::TR_ARM,
            b"window" => entry::TR_WINDOW,
            b"cfsr" => 13,
            b"pc" => 14,
            b"dmastop" => 15,
            b"reinit" => 16,
            _ => 0,
        };
        // `now()` calls are the Tasks' busy loops: thousands per run, which
        // would flood the ring; the other SVCs and every other entry are kept.
        // Feature `record-only` (task 19.2, R34.7): the records without the
        // trace ring, to measure what the ring costs.
        #[cfg(not(feature = "record-only"))]
        if !(id == entry::TR_SVC && v == entry::SVC_NOW as u32) {
            unsafe_module::trace_ram_push((id as u32) << 24 | (v & 0x00FF_FFFF), hw::timebase_now());
        }
        #[cfg(feature = "record-only")]
        let _ = (id, v);
    }
    #[cfg(feature = "trace")]
    {
        let mut buf = [0u8; 24];
        let mut n = 0;
        buf[n] = b'[';
        n += 1;
        let mut t = 0;
        while t < tag.len() && t < 12 {
            buf[n] = tag[t];
            n += 1;
            t += 1;
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
    #[cfg(not(any(feature = "trace", feature = "trace-ram")))]
    {
        let _ = (tag, v);
    }
}

/// The trace tag of an `Action::Trace`.
fn trace_tag(tag: u8) -> &'static [u8] {
    match tag {
        entry::TR_BOOT => b"boot",
        entry::TR_INVALID => b"invalid",
        entry::TR_BOOTFAIL => b"bootfail",
        entry::TR_BOOTED => b"booted",
        entry::TR_START => b"start",
        entry::TR_IRQ => b"irq",
        entry::TR_DISPATCH => b"dispatch",
        entry::TR_TICK => b"tick",
        entry::TR_SVC => b"svc",
        entry::TR_EPILOGUE => b"epilogue",
        entry::TR_FAULT => b"fault",
        entry::TR_ARM => b"arm",
        entry::TR_WINDOW => b"window",
        _ => b"?",
    }
}

/// The priority encodings (HW-01, HW-03): proven against the
/// Hardware_Model in `arch::bound`.
pub use crate::arch::bound::{basepri_for, basepri_for_ceiling, encode_level};

/// The `System` trait of the generated system crate, seen as the
/// `Tables` the verified cores consume.
struct SysTables<S: System>(PhantomData<S>);

impl<S: System> Tables for SysTables<S> {
    fn binding(&self, n: u16) -> Binding {
        S::binding(n)
    }
    fn dispatch_vector(&self, t: u8) -> u16 {
        S::dispatch_vector(t)
    }
    fn intake_vector(&self, t: u8) -> Option<u16> {
        S::intake_vector(t)
    }
    fn stack_top(&self, p: u8) -> u32 {
        S::stack_top(p)
    }
    fn stack_bottom(&self, p: u8) -> u32 {
        S::stack_bottom(p)
    }
    fn task_entry_address(&self, t: u8) -> u32 {
        S::task_entry(t) as usize as u32
    }
    fn irq_count(&self) -> u16 {
        #[cfg(feature = "boot-mutation")]
        if unsafe_module::boot_mutation() == MUT_IRQ_COUNT {
            return 600;
        }
        S::IRQ_COUNT
    }
    fn config_checksum(&self) -> u32 {
        #[cfg(feature = "boot-mutation")]
        if unsafe_module::boot_mutation() == MUT_CONFIG_CHECKSUM {
            return S::CONFIG_CHECKSUM ^ 1;
        }
        S::CONFIG_CHECKSUM
    }
    fn layout_checksum(&self) -> u32 {
        #[cfg(feature = "boot-mutation")]
        if unsafe_module::boot_mutation() == MUT_LAYOUT_CHECKSUM {
            return S::LAYOUT_CHECKSUM ^ 1;
        }
        S::LAYOUT_CHECKSUM
    }
    fn timer_vector(&self) -> u16 {
        S::TIMER_VECTOR
    }
    fn priority_bits(&self) -> u8 {
        S::PRIORITY_BITS
    }
    fn boot_expect(&self) -> plan::BootExpect {
        #[allow(unused_mut)]
        let mut e = plan::BootExpect { priority_bits: S::PRIORITY_BITS, cpuid: S::CPUID, hfclk_xtal: S::HFCLK_XTAL, fpu_needed: S::FPU_NEEDED };
        #[cfg(feature = "boot-mutation")]
        match unsafe_module::boot_mutation() {
            MUT_CPUID => e.cpuid ^= 1,
            MUT_PRIORITY_BITS => e.priority_bits = 8,
            MUT_HFCLK => e.hfclk_xtal = !e.hfclk_xtal,
            _ => {}
        }
        e
    }
    fn tick_hz(&self) -> u64 {
        S::TICK_HZ
    }
    fn fp_flush_to_zero(&self) -> bool {
        S::FP_FLUSH_TO_ZERO
    }
    fn dma_count(&self) -> usize {
        S::DMA_COUNT
    }
    fn dma(&self, i: usize) -> plan::DmaCfg {
        S::dma(i)
    }
    fn signal_count(&self) -> usize {
        S::SIGNAL_COUNT
    }
    fn signal(&self, i: usize) -> plan::SignalCfg {
        S::signal(i)
    }
    fn endpoint_count(&self) -> usize {
        S::ENDPOINT_COUNT
    }
    fn endpoint(&self, i: usize) -> plan::EndpointCfg {
        S::endpoint(i)
    }
    fn endpoint_storage(&self) -> (u32, u32) {
        S::endpoint_storage()
    }
}

// The shells below are the only Kernel_Arch code outside `verus!` besides
// the Kernel_Unsafe_Module: each fetches the Kernel, the hardware, the
// tables, and the Kernel_Arch state, calls its verified core
// (`core.rs`), and enters the system safe state when the core says so.

/// Boot (R12.4): `false` when a boot check fails (R12.1, R12.2); the caller
/// enters the safe state.
/// The boot-check suite's mutations (feature `boot-mutation`, HIL builds
/// only; task 7.2, R12.6), selected by the word the rig writes into
/// `BOOT_MUTATION` before the run. Each makes exactly one check fail:
/// a recorded value that differs from the Target's (the CPUID, the
/// priority bits, the clock source, the two checksums, the interrupt-line
/// count), a table corrupted in memory (a Task's Budget in the Kernel's
/// `Config` copy, so the computed checksum differs from the recorded
/// one), a register state the check rejects (PRIGROUP, the instruction
/// cache enabled before the check), and the watchdog never fed (the
/// reset's next boot fails the watchdog check).
#[cfg(feature = "boot-mutation")]
pub const MUT_CPUID: u32 = 1;
#[cfg(feature = "boot-mutation")]
pub const MUT_PRIORITY_BITS: u32 = 2;
#[cfg(feature = "boot-mutation")]
pub const MUT_HFCLK: u32 = 3;
#[cfg(feature = "boot-mutation")]
pub const MUT_PRIGROUP: u32 = 4;
#[cfg(feature = "boot-mutation")]
pub const MUT_ICACHE: u32 = 5;
#[cfg(feature = "boot-mutation")]
pub const MUT_CONFIG_CHECKSUM: u32 = 6;
#[cfg(feature = "boot-mutation")]
pub const MUT_CONFIG_TABLE: u32 = 7;
#[cfg(feature = "boot-mutation")]
pub const MUT_LAYOUT_CHECKSUM: u32 = 8;
#[cfg(feature = "boot-mutation")]
pub const MUT_IRQ_COUNT: u32 = 9;
#[cfg(feature = "boot-mutation")]
pub const MUT_WATCHDOG: u32 = 10;

/// Applies the mutations that act before the checks: the register states
/// and the table corruption.
#[cfg(feature = "boot-mutation")]
fn boot_mutation<S: System>(k: &mut S::K) {
    match unsafe_module::boot_mutation() {
        MUT_PRIGROUP => {
            // AIRCR.PRIGROUP = 5 (VECTKEY 0x05FA): not the reset value the check requires.
            let v = hw::mmio_read(unsafe_module::hw::addr::AIRCR) & 0x0000_F8FF;
            hw::mmio_write(unsafe_module::hw::addr::AIRCR, (0x05FA << 16) | v | (5 << 8));
        }
        MUT_ICACHE => {
            #[cfg(not(feature = "board-qemu-mps2"))]
            hw::mmio_write(unsafe_module::hw::addr::NRF_ICACHECNF, 1);
        }
        MUT_CONFIG_TABLE => k.corrupt_config_for_test(),
        _ => {}
    }
}

pub fn boot<S: System>() -> bool {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    // The HIL differential tests' early phase (task 3.3): before the
    // Kernel configures anything.
    #[cfg(feature = "hw-dt")]
    unsafe_module::dt::early();
    #[cfg(feature = "boot-mutation")]
    boot_mutation::<S>(k);
    let ok = entry::boot_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state());
    mirror_events(k);
    ok
}

/// Feature `trace-ram`: the events the HIL_Rig's trace freeze treats as
/// expected (`unsafe_module::events_expect`): kinds in `kinds` (bit k =
/// kind k) of Task `task` (0xFF: any). Called by the entry crate before
/// `boot`; a no-op without the feature.
pub fn trace_expect(kinds: u32, task: u8, forbidden_partitions: u32) {
    #[cfg(feature = "trace-ram")]
    unsafe_module::events_expect(kinds, task, forbidden_partitions);
    #[cfg(not(feature = "trace-ram"))]
    let _ = (kinds, task, forbidden_partitions);
}

/// Feature `trace-ram`: copies the events the Kernel logic pushed during
/// this entry (the ones numbered from the mirror's seen sequence number
/// to the log's) into the event mirror, which freezes the trace ring at
/// the first unexpected one. Called after every `*_core` call.
#[inline(always)]
fn mirror_events<K: KernelOps>(k: &K) {
    #[cfg(feature = "trace-ram")]
    {
        let seq = k.log_seq();
        let new = seq.wrapping_sub(unsafe_module::events_seen_seq()) as usize;
        if new == 0 {
            return;
        }
        let len = k.log_len();
        // More new events than the log holds: the oldest are lost to the
        // mirror too (R14.7 counts them). The loop is bounded by the log's
        // capacity (PR-16).
        let new = if new > len { len } else { new };
        let mut i = len - new;
        let mut copied = 0;
        while i < len && copied < unsafe_module::EVENTS_PER_ENTRY {
            unsafe_module::events_push(
                seq.wrapping_sub((len - i) as u32),
                k.log_kind(i),
                k.log_task(i),
                k.log_partition(i),
                k.log_instant(i),
                k.log_detail(i),
            );
            i += 1;
            copied += 1;
        }
    }
    #[cfg(not(feature = "trace-ram"))]
    let _ = k;
}

/// Feature `trace-ram`, task 19.3: the kinds of the events the Kernel
/// logic pushed since `seq_before`, as a bit set (bit k = kind k).
#[cfg(feature = "trace-ram")]
fn pushed_kinds<K: KernelOps>(k: &K, seq_before: u32) -> u32 {
    let seq = k.log_seq();
    let new = seq.wrapping_sub(seq_before) as usize;
    let len = k.log_len();
    let new = if new > len { len } else { new };
    let mut bits = 0u32;
    let mut i = len - new;
    // Bounded by the log's capacity (PR-16), as `mirror_events` is.
    let mut seen = 0;
    while i < len && seen < unsafe_module::EVENTS_PER_ENTRY {
        let kind = k.log_kind(i);
        if kind < 32 {
            bits |= 1 << kind;
        }
        i += 1;
        seen += 1;
    }
    bits
}

/// Feature `trace-ram`, task 19.3: records a Kernel entry of `kind` that
/// began at `start` (DWT cycles) and, when the entry applied a response,
/// the response's kinds too (Table 13-1: Δ_restart, Δ_stop, Δ_enforce).
#[cfg(feature = "trace-ram")]
fn entry_record<K: KernelOps>(k: &K, kind: u8, start: u32, seq_before: u32) {
    let cycles = hw::dwt_cyccnt().wrapping_sub(start);
    unsafe_module::stats_entry(kind, cycles);
    let pushed = pushed_kinds(k, seq_before);
    if pushed & (1 << crate::logic::log::EV_PARTITION_RESTART) != 0 {
        unsafe_module::stats_entry(unsafe_module::ENTRY_RESTART, cycles);
    }
    if pushed & (1 << crate::logic::log::EV_PARTITION_STOP) != 0 {
        unsafe_module::stats_entry(unsafe_module::ENTRY_STOP, cycles);
    }
    if pushed & (1 << crate::logic::log::EV_OVERRUN) != 0 {
        unsafe_module::stats_entry(unsafe_module::ENTRY_ENFORCE, cycles);
    }
}

/// Instant 0: enables Releases (PR-34) and the compare of the first event.
pub fn start<S: System>() {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    // The records are a run's own: cleared at instant 0, which the reboot
    // the board takes when the probe detaches (a failed boot check into
    // the safe state) never reaches, so the records of the run survive it.
    #[cfg(feature = "trace-ram")]
    unsafe_module::stats_reset();
    #[cfg(feature = "trace-ram")]
    unsafe_module::stats_calibrate();
    // The Δ_stamp probe (Table 13-1): the stimulus edge latched by the time base.
    #[cfg(feature = "trace-ram")]
    unsafe_module::hw::stamp_probe_init();
    #[cfg(feature = "trace-ram")]
    unsafe_module::events_reset();
    #[cfg(feature = "trace-ram")]
    unsafe_module::faults_reset();
    #[cfg(feature = "trace-ram")]
    unsafe_module::actions_reset();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_reset();
    // The coverage build (task 19.2): the Partitions' counters are a run's own.
    #[cfg(feature = "coverage")]
    unsafe_module::hw::coverage_reset();
    // The HIL differential tests' late phase (task 3.3): the Kernel's
    // configuration read back and the time base's compare rule, before
    // instant 0.
    #[cfg(feature = "hw-dt")]
    unsafe_module::dt::late();
    let safe = entry::start_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state());
    mirror_events(k);
    if safe {
        S::safe_state();
    }
}

/// The Rust half of the dispatch stub, with PRIMASK held.
pub fn irq_entry<S: System>(ipsr: u32, psp: u32, control: u32, exc_return: u32) -> DispatchPlan {
    // The HIL differential tests' probe vectors (task 3.3): served before
    // any Kernel logic while a test phase is in progress.
    #[cfg(feature = "hw-dt")]
    if let Some(plan) = unsafe_module::dt::irq(ipsr, control) {
        return plan;
    }
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    // The time-base vector is served here (`Binding::Kernel`), so the
    // release-timing record of R9.10 is taken around this call.
    #[cfg(feature = "trace-ram")]
    let (entry_now, entry_cycles, due_before) = {
        let mut due = [u64::MAX; unsafe_module::STATS_TASKS];
        let mut t = 0;
        while t < k.task_count() && t < unsafe_module::STATS_TASKS {
            due[t] = k.release_info(t as u8).1;
            t += 1;
        }
        (hw::timebase_now(), hw::dwt_cyccnt(), due)
    };
    #[cfg(feature = "trace-ram")]
    let (seq_before, top_before) = (k.log_seq(), k.top());
    let depth_before = k.depth();
    let (plan, safe) = entry::irq_entry_core(
        k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), ipsr, psp, control, exc_return,
    );
    svc_of_ended_job(k, depth_before);
    #[cfg(feature = "trace-ram")]
    {
        // R19.3: the cycles of this intake handler run, attributed to the
        // source's Task (the record the storm test reads).
        let binding = S::binding((ipsr & 0x1FF) as u16);
        if let crate::arch::Binding::Intake { task, .. } = binding {
            unsafe_module::stats_intake(task, hw::dwt_cyccnt().wrapping_sub(entry_cycles));
            // Δ_stamp: from the edge the probe latched to this entry's timestamp.
            unsafe_module::stats_stamp(task, hw::stamp_capture(), entry_now as u32);
        }
        // Task 19.3: the entry's cycles by kind, and the Job it interrupted.
        let kind = match binding {
            crate::arch::Binding::Dispatch(_) => unsafe_module::ENTRY_DISPATCH,
            crate::arch::Binding::Intake { .. } => unsafe_module::ENTRY_INTAKE,
            _ => unsafe_module::ENTRY_TIMER,
        };
        entry_record(k, kind, entry_cycles, seq_before);
        if let Some(t) = top_before {
            unsafe_module::stats_interrupted(t);
        }
    }
    #[cfg(feature = "trace-ram")]
    {
        // R9.10: every periodic Release this tick made effective (its due
        // instant moved; the Release may already be consumed by a dispatch
        // pended in the same tick), with the lag from its nominal instant
        // (now - due at entry, in cycles) plus the handler's own cycles.
        let handler_cycles = hw::dwt_cyccnt().wrapping_sub(entry_cycles);
        let mut t = 0;
        while t < k.task_count() && t < unsafe_module::STATS_TASKS {
            let (periodic, due, offset, period) = k.release_info(t as u8);
            if due != due_before[t] {
                if periodic {
                    let lag = (entry_now.saturating_sub(due) as u32).saturating_mul(hw::CYCLES_PER_TICK).saturating_add(handler_cycles);
                    unsafe_module::stats_release(t as u8, due, offset, period, lag);
                }
            }
            t += 1;
        }
    }
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, if plan.psp != 0 { unsafe_module::ENTRY_DISPATCH } else { unsafe_module::ENTRY_TIMER }, plan.hop != 0);
    pins_running(k.top());
    plan
}

/// The masked-span record's close (11.13): the span stays open when the
/// return will not reach a context in which a Kernel-level interrupt is
/// taken: an unwind follows (`chained`), or the highest pending exception
/// is a Kernel-level one (a fault, the SVC, the time base, an intake), or
/// a dispatch vector the resumed BASEPRI admits (it tail-chains into its
/// stub under PRIMASK). A pending dispatch vector BASEPRI masks, or
/// nothing pending, closes it: the Job resumes and an intake is taken.
#[cfg(feature = "trace-ram")]
fn span_close<S: System>(k: &S::K, kind: u8, chained: bool) {
    let chained = chained || {
        let v = hw::vectpending();
        v != 0
            && match S::binding(v) {
                crate::arch::Binding::Dispatch(t) => {
                    let b = k.basepri_now();
                    b == 0 || encode_level(k.job_info(t).priority) < b
                }
                _ => true,
            }
    };
    unsafe_module::span_leave(kind, chained);
}

/// Trace pins (feature `trace-pins`): a Kernel handler begins.
#[inline(always)]
fn pins_enter() {
    #[cfg(feature = "trace-pins")]
    hw::trace_pins::kernel_enter();
}

/// Trace pins: a Kernel handler returns to `task`.
#[inline(always)]
fn pins_running(task: Option<u8>) {
    #[cfg(feature = "trace-pins")]
    hw::trace_pins::running(task);
    #[cfg(not(feature = "trace-pins"))]
    let _ = task;
}

/// The Kernel-level timer handler body.
pub fn timer<S: System>() -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    #[cfg(feature = "trace-ram")]
    let (start, seq_before, top_before) = (hw::dwt_cyccnt(), k.log_seq(), k.top());
    let depth_before = k.depth();
    let (hop, safe) = entry::timer_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state());
    svc_of_ended_job(k, depth_before);
    #[cfg(feature = "trace-ram")]
    {
        entry_record(k, unsafe_module::ENTRY_TIMER, start, seq_before);
        if let Some(t) = top_before {
            unsafe_module::stats_interrupted(t);
        }
    }
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, unsafe_module::ENTRY_TIMER, hop != 0);
    pins_running(k.top());
    hop
}

/// Audit finding 4 (2026-11-13): a Kernel entry other than the SVC handler
/// that ended a Job (fewer Jobs in progress than at its start; a dispatch
/// only adds one) drops a pending SVCall, which can only be the ended
/// executing Job's (`hw::svc_clear_pending`).
fn svc_of_ended_job<K: KernelOps>(k: &K, depth_before: usize) {
    if k.depth() < depth_before {
        hw::svc_clear_pending();
    }
}

/// R16.5 (the audit's findings 8 and 9 and contested finding D,
/// 2026-11-13): the stack check of an SVC taken from a Job, before anything
/// is read or written through the process stack pointer `psp`, the SVC
/// instruction's number included. `None`: the frame lies inside the
/// Partition's stack region and the SVC is served. Otherwise the Job has a
/// stack violation: the Partition's fault response is applied, the stacking
/// fault pending behind this handler (an SVC issued with less than a
/// frame's room: the entry could not stack) is discarded, and the plan
/// unwinds the ended Job. With no Job executing the frame is the Kernel's
/// own (the idle activity's) and is not checked.
pub fn svc_frame_check<S: System>(psp: u32) -> Option<ReturnPlan> {
    let state = unsafe_module::arch_state();
    let tables = SysTables::<S>(PhantomData);
    if state.current_partition == 0xFF || entry::psp_in_stack(&tables, state, psp) {
        return None;
    }
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    #[cfg(feature = "trace-ram")]
    let (start, seq_before) = (hw::dwt_cyccnt(), k.log_seq());
    let (hop, safe) = entry::stack_violation_core(k, &mut hw, &tables, state);
    hw::stacking_fault_discard();
    #[cfg(feature = "trace-ram")]
    entry_record(k, unsafe_module::ENTRY_FAULT, start, seq_before);
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, unsafe_module::ENTRY_FAULT, hop != 0);
    pins_running(k.top());
    Some(hop)
}

/// The stacked PC of a faulting context for the fault record (R14.3), or 0
/// when it cannot be read safely (the audit's contested finding A,
/// 2026-11-13): the frame at `frame` was not stacked (CFSR.MSTKERR: the
/// fault is this entry's own stacking failure), or it is on the process
/// stack and not inside the stack region of the executing Job's Partition.
fn fault_pc<S: System>(cfsr: u32, frame: u32, exc_return: u32) -> u32 {
    if cfsr & (1 << 4) != 0 {
        return 0;
    }
    if exc_return & 4 != 0 {
        let state = unsafe_module::arch_state();
        if !entry::psp_in_stack(&SysTables::<S>(PhantomData), state, frame) {
            return 0;
        }
    }
    unsafe_module::frame_pc(frame)
}

/// The Rust half of the SVC handler: `frame` points at the caller's
/// exception frame (R0-R3 at offsets 0 to 12, PC at 24).
pub fn svc<S: System>(frame: &mut [u32; 8]) -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let number = unsafe_module::svc_number(frame[6]);
    // The HIL differential tests' probe SVCs (task 3.3).
    #[cfg(feature = "hw-dt")]
    if let Some(plan) = unsafe_module::dt::svc(number, frame) {
        return plan;
    }
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    // A late-arriving SVC of a killed Job (found by the fault-injection
    // suite, task 7.5): a Job executes `SVC` and, during the exception
    // entry, a higher-priority Kernel exception (the time base) arrives and
    // is served first (late arrival, Armv7-M B1.5.11), kills the Job
    // (Budget overrun, END_JOB) and unwinds its level; the pending SVC
    // then tail-chains onto the resumed context, whose frame is not the
    // SVC's (the idle activity's return address follows a WFI). The SVC's
    // request died with its Job: the entry is ignored and the context
    // resumes. Before this guard the handler read the number from the
    // resumed frame (48, the low byte of WFI) and the Kernel's panic
    // response took the Target to the safe state.
    if number == entry::SVC_STALE {
        trace(b"svc", entry::SVC_STALE as u32);
        #[cfg(feature = "trace-ram")]
        span_close::<S>(k, unsafe_module::ENTRY_PANIC, false);
        pins_running(k.top());
        return 0;
    }
    // R18.8: the response time of the ending Job, from its Release's due
    // instant to now (the record the fault-injection suites read).
    #[cfg(feature = "trace-ram")]
    let ending = if number == entry::SVC_JOB_END { k.top().map(|t| (t, k.job_due(t))) } else { None };
    // Task 19.1: the execution charged to the executing Job so far, the
    // Budget account plus the cycles since the last charge point, at a lock
    // or unlock (the Resource is the call's argument, before the result
    // replaces it).
    #[cfg(feature = "trace-ram")]
    let section = if number == entry::SVC_LOCK || number == entry::SVC_UNLOCK { k.top().map(|t| (t, frame[0] as u8, consumed_now(k, t))) } else { None };
    #[cfg(feature = "trace-ram")]
    let (start, seq_before) = (hw::dwt_cyccnt(), k.log_seq());
    let (hop, safe) =
        entry::svc_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), frame, number);
    #[cfg(feature = "trace-ram")]
    {
        let kind = match number {
            entry::SVC_LOCK => unsafe_module::ENTRY_LOCK,
            entry::SVC_UNLOCK => unsafe_module::ENTRY_UNLOCK,
            entry::SVC_JOB_END => unsafe_module::ENTRY_JOB_END,
            entry::SVC_NOW => unsafe_module::ENTRY_NOW,
            entry::SVC_RAISE => unsafe_module::ENTRY_RAISE,
            entry::SVC_DMA => unsafe_module::ENTRY_DMA,
            entry::SVC_SEND => unsafe_module::ENTRY_SEND,
            entry::SVC_RECV => unsafe_module::ENTRY_RECV,
            // A violation report is a signal to the Health_Monitor, like a raise.
            entry::SVC_VIOLATION => unsafe_module::ENTRY_RAISE,
            _ => unsafe_module::ENTRY_PANIC,
        };
        entry_record(k, kind, start, seq_before);
    }
    #[cfg(feature = "trace-ram")]
    if let Some((t, due)) = ending {
        unsafe_module::stats_response(t, hw::timebase_now().saturating_sub(due));
        // The Job's execution, charged in full by `job_return`. A completion
        // above the Budget under a response that ends overrunning Jobs is
        // the diagnostic event of task 19.1: the trace freezes at it.
        let used = k.budget_used(t);
        let over = used > k.budget_of(t);
        unsafe_module::stats_exec(t, used, over);
        if over && !k.overrun_continues(t) {
            unsafe_module::events_push_diag(unsafe_module::EV_DIAG_EXEC_OVER, t, k.partition_of(t), hw::timebase_now(), used);
        }
    }
    #[cfg(feature = "trace-ram")]
    if let Some((t, r, at_entry)) = section {
        if frame[0] == 1 {
            if number == entry::SVC_LOCK {
                // From the lock's return: the lock's own work is not the
                // Critical_Section's.
                unsafe_module::stats_cs_open(t, r, consumed_now(k, t));
            } else {
                unsafe_module::stats_cs_close(t, r, at_entry);
            }
        }
    }
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, unsafe_module::ENTRY_JOB_END, hop != 0);
    pins_running(k.top());
    hop
}

/// The execution charged to the Job of `t` so far: the Budget account
/// plus the DWT cycles since the last charge point (task 19.1).
#[cfg(feature = "trace-ram")]
fn consumed_now<K: crate::arch::bound::KernelOps>(k: &K, t: u8) -> u32 {
    k.budget_used(t).wrapping_add(hw::dwt_cyccnt().wrapping_sub(unsafe_module::arch_state().last_switch))
}

/// The Rust half of the unwind of one level, named by its exception
/// number `n`: (PSP, CONTROL, EXC_RETURN, next), where `next` names one
/// more level to unwind when the resumed Job was killed, else 0.
pub fn epilogue<S: System>(n: u32) -> (u32, u32, u32, ReturnPlan) {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    #[cfg(feature = "trace-ram")]
    let (start, seq_before) = (hw::dwt_cyccnt(), k.log_seq());
    let (r, safe) = entry::epilogue_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), n);
    #[cfg(feature = "trace-ram")]
    entry_record(k, unsafe_module::ENTRY_EPILOGUE, start, seq_before);
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, unsafe_module::ENTRY_EPILOGUE, r.3 != 0);
    pins_running(k.top());
    r
}

/// Fault handlers (R14.3).
pub fn fault<S: System>(kind: u8, in_kernel: bool) -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    pins_enter();
    #[cfg(feature = "trace-ram")]
    unsafe_module::span_enter();
    #[cfg(feature = "trace-ram")]
    let (start, seq_before) = (hw::dwt_cyccnt(), k.log_seq());
    let depth_before = k.depth();
    let (hop, safe) =
        entry::fault_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), kind, in_kernel);
    svc_of_ended_job(k, depth_before);
    #[cfg(feature = "trace-ram")]
    entry_record(k, unsafe_module::ENTRY_FAULT, start, seq_before);
    mirror_events(k);
    if safe {
        S::safe_state();
    }
    #[cfg(feature = "trace-ram")]
    span_close::<S>(k, unsafe_module::ENTRY_FAULT, hop != 0);
    pins_running(k.top());
    hop
}

/// The first MemManage fault since boot (`ArchState::first_fault_*`): CFSR
/// with bit 31 set as the record mark, and the faulting address (MMFAR when
/// valid, else the stacked PC of the faulting context), for the HIL report
/// (R14.3 diagnosis; the event log keeps the kind, task, and instant).
pub fn first_memmanage() -> (u32, u32) {
    let state = unsafe_module::arch_state();
    (state.first_fault_cfsr, state.first_fault_addr)
}

pub fn mem_manage<S: System>(in_kernel: bool, frame: u32, exc_return: u32) -> ReturnPlan {
    {
        let (cfsr, mmfar) = hw::fault_status();
        let pc = fault_pc::<S>(cfsr, frame, exc_return);
        let state = unsafe_module::arch_state();
        if state.first_fault_cfsr == 0 {
            state.first_fault_cfsr = cfsr | 0x8000_0000;
            state.first_fault_addr = if cfsr & (1 << 7) != 0 { mmfar } else { pc };
        }
        trace(b"cfsr", cfsr);
        trace(b"pc", pc);
        #[cfg(feature = "trace-ram")]
        unsafe_module::fault_push(cfsr, if cfsr & (1 << 7) != 0 { mmfar } else { pc });
    }
    Cm4Hw::new().apply(Action::ClearFaultStatus);
    fault::<S>(EV_MEMMANAGE, in_kernel)
}

pub fn bus_fault<S: System>(in_kernel: bool) -> ReturnPlan {
    #[cfg(feature = "trace-ram")]
    {
        // BFAR (0xE000_ED38) when CFSR.BFARVALID (bit 15).
        let (cfsr, _) = hw::fault_status();
        let bfar = if cfsr & (1 << 15) != 0 { hw::mmio_read(0xE000_ED38) } else { 0 };
        unsafe_module::fault_push(cfsr, bfar);
    }
    Cm4Hw::new().apply(Action::ClearFaultStatus);
    fault::<S>(EV_BUSFAULT, in_kernel)
}

pub fn usage_fault<S: System>(in_kernel: bool) -> ReturnPlan {
    #[cfg(feature = "trace-ram")]
    {
        let (cfsr, _) = hw::fault_status();
        unsafe_module::fault_push(cfsr, 0);
        // The INVPC diagnostic (2026-11-06, Verification_Plan section 13):
        // the processor's exception state and the faulting frame, as
        // tagged entries of the fault record (`0xA1xx_xxxx`, the rig names
        // them): ICSR, SHCSR, CCR, NVIC_IABR0, PSP, MSP, and the stacked
        // xPSR, PC, and LR of the frame the return used (on the PSP when
        // the fault is a Job's, the MSP otherwise).
        let (psp, msp) = hw::stack_pointers();
        let frame = if in_kernel { msp } else { psp };
        let diag = |tag: u32, value: u32| unsafe_module::fault_push(0xA100_0000 | tag, value);
        diag(1, hw::mmio_read(unsafe_module::hw::addr::ICSR));
        diag(2, hw::mmio_read(unsafe_module::hw::addr::SHCSR));
        diag(3, hw::mmio_read(unsafe_module::hw::addr::CCR));
        diag(4, hw::mmio_read(0xE000_E300));
        diag(5, psp);
        diag(6, msp);
        diag(7, hw::mmio_read(frame.wrapping_add(28)));
        diag(8, hw::mmio_read(frame.wrapping_add(24)));
        diag(9, hw::mmio_read(frame.wrapping_add(20)));
    }
    Cm4Hw::new().apply(Action::ClearFaultStatus);
    fault::<S>(EV_USAGEFAULT, in_kernel)
}

pub fn hard_fault<S: System>() -> ! {
    trace(b"hardfault", hw::mmio_read(unsafe_module::hw::addr::CFSR));
    trace(b"hfsr", hw::mmio_read(0xE000_ED2C));
    trace(b"mmfar", hw::mmio_read(0xE000_ED34));
    let k = S::kernel().get();
    k.fault(EV_HARDFAULT, true);
    S::safe_state()
}

/// The panic handler body (PR-19, R14.2): a panic in a Job is a fault of
/// its Partition; one in Kernel code is a Kernel fault. This function is
/// Kernel code (Kernel flash): the panic handler in the shared region
/// (rsk-entry) issues the panic SVC itself for a Job and calls here only
/// in Handler mode. Found by the fi-PR-19 fault injection of the
/// Profile_Conformance_Suite (2026-10-23): a Job's panic that reached this
/// function faulted with MemManage before the SVC.
pub fn panic<S: System>(in_kernel: bool) -> ! {
    if !in_kernel {
        // Unprivileged: report through the SVC; the Kernel ends the Job and
        // never returns here (two-step return), so the loop is unreachable.
        unsafe_module::stubs::svc_panic();
        loop {}
    }
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    k.fault(EV_PANIC, true);
    let _ = entry::apply_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state());
    S::safe_state()
}

#[cfg(test)]
mod tests {
    use super::*;
    use core::sync::atomic::{AtomicBool, Ordering};
    use unsafe_module::{hw, sim};

    /// The simulated register file is one static; tests that use it run
    /// one at a time (a spin lock: the Kernel crate links no `std`, R6.1).
    static SIM_BUSY: AtomicBool = AtomicBool::new(false);

    struct SimGuard;

    impl Drop for SimGuard {
        fn drop(&mut self) {
            SIM_BUSY.store(false, Ordering::Release);
        }
    }

    fn sim_lock() -> SimGuard {
        while SIM_BUSY.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            core::hint::spin_loop();
        }
        SimGuard
    }

    fn expected_board() -> plan::BootExpect {
        plan::BootExpect { priority_bits: 3, cpuid: 0x410F_C241, hfclk_xtal: true, fpu_needed: true }
    }

    /// The simulated registers of an nRF52840-DK as the boot checks expect
    /// them (the simulator models 3 implemented priority bits).
    fn simulate_expected_board() {
        sim::reset();
        sim::write(hw::addr::CPUID, 0x410F_C241);
        sim::write(hw::addr::MPU_TYPE, 8 << 8);
        sim::write(hw::addr::DWT_CTRL, 0);
        sim::write(hw::addr::AIRCR, 0);
        sim::write(hw::addr::NRF_HFCLKSTAT, (1 << 16) | 1);
        sim::write(hw::addr::NRF_ICACHECNF, 0);
        sim::write(hw::addr::NRF_PPI_CHEN, 0);
        sim::write(hw::addr::MVFR0, 0x1011_0021);
    }

    /// R12.1, task 5.2: every boot check passes on the expected board and
    /// each one alone fails with its own bit (host test of the boot logic;
    /// the HIL boot-check suite of task 7.2 repeats this on the Target).
    #[test]
    #[cfg(not(feature = "board-qemu-mps2"))]
    fn boot_checks_pass_on_the_expected_board_and_fail_one_bit_per_check() {
        let _sim = sim_lock();
        let expect = expected_board();
        simulate_expected_board();
        assert_eq!(hw::boot_checks(&expect), 0);

        simulate_expected_board();
        sim::write(hw::addr::CPUID, 0x410F_C240);
        assert_eq!(hw::boot_checks(&expect), 1 << 8, "core revision (ASM-01)");

        simulate_expected_board();
        sim::write(hw::addr::CPUID, 0x410F_C271);
        assert_eq!(hw::boot_checks(&expect), (1 << 1) | (1 << 8), "part number");

        simulate_expected_board();
        sim::write(hw::addr::MPU_TYPE, 0);
        assert_eq!(hw::boot_checks(&expect), 1 << 2, "MPU regions (ASM-02)");

        simulate_expected_board();
        sim::write(hw::addr::DWT_CTRL, 1 << 25);
        assert_eq!(hw::boot_checks(&expect), 1 << 3, "DWT cycle counter (ASM-06)");

        simulate_expected_board();
        assert_eq!(hw::boot_checks(&plan::BootExpect { priority_bits: 8, ..expect }), 1 << 4, "priority bits (ASM-03)");

        simulate_expected_board();
        sim::write(hw::addr::AIRCR, 5 << 8);
        assert_eq!(hw::boot_checks(&expect), 1 << 9, "PRIGROUP (ASM-03)");

        simulate_expected_board();
        sim::write(hw::addr::NRF_HFCLKSTAT, 1 << 16);
        assert_eq!(hw::boot_checks(&expect), 1 << 10, "HFCLK from the RC oscillator (ASM-05)");
        sim::write(hw::addr::NRF_HFCLKSTAT, 1);
        assert_eq!(hw::boot_checks(&expect), 1 << 10, "HFCLK not running (ASM-05)");

        simulate_expected_board();
        sim::write(hw::addr::NRF_ICACHECNF, 1);
        assert_eq!(hw::boot_checks(&expect), 1 << 11, "instruction cache enabled (ASM-04)");

        simulate_expected_board();
        sim::write(hw::addr::NRF_PPI_CHEN, 1 << 3);
        assert_eq!(hw::boot_checks(&expect), 1 << 14, "a PPI channel enabled after the boot clear (DD-08)");

        simulate_expected_board();
        sim::write(hw::addr::MVFR0, 0);
        assert_eq!(hw::boot_checks(&expect), 1 << 12, "FPU absent while a Task uses it");
        assert_eq!(hw::boot_checks(&plan::BootExpect { fpu_needed: false, ..expect }), 0, "no FPU needed");
    }

    /// ORQ-30, ORQ-31, task 5.2: the debug state and a watchdog reset are
    /// reported, and the reset reason is cleared for the next boot (the
    /// simulator is a plain store, so the clear shows as the written bit).
    #[test]
    #[cfg(not(feature = "board-qemu-mps2"))]
    fn boot_info_reports_the_debugger_and_a_watchdog_reset_and_clears_the_reason() {
        let _sim = sim_lock();
        sim::reset();
        assert_eq!(hw::boot_info(), 0);
        sim::write(hw::addr::DHCSR, 1);
        assert_eq!(hw::boot_info(), hw::BOOT_INFO_DEBUGGER);
        sim::write(hw::addr::NRF_RESETREAS, (1 << 1) | (1 << 2));
        assert_eq!(hw::boot_info(), hw::BOOT_INFO_DEBUGGER | hw::BOOT_INFO_WATCHDOG_RESET);
        assert_eq!(sim::read(hw::addr::NRF_RESETREAS), (1 << 1) | (1 << 2), "every set bit was written to clear it");
        // nRF52840 anomaly 136 (task 3.4): with RESETPIN set the other bits
        // are not trusted; the register is still cleared.
        sim::write(hw::addr::NRF_RESETREAS, (1 << 1) | 1);
        assert_eq!(hw::boot_info(), hw::BOOT_INFO_DEBUGGER, "DOG beside RESETPIN is ignored (anomaly 136)");
        assert_eq!(sim::read(hw::addr::NRF_RESETREAS), (1 << 1) | 1);
    }

    /// ORQ-31, task 5.3: the watchdog is started with the reload value,
    /// one reload register, running in sleep and paused under a debugger,
    /// and fed with the reload request.
    #[test]
    fn watchdog_start_and_feed_write_the_wdt_registers() {
        let _sim = sim_lock();
        sim::reset();
        hw::watchdog_start(16_384);
        let w = hw::addr::NRF_WDT;
        assert_eq!(sim::read(w + hw::wdt::CRV), 16_384);
        assert_eq!(sim::read(w + hw::wdt::RREN), 1);
        assert_eq!(sim::read(w + hw::wdt::CONFIG), 1);
        assert_eq!(sim::read(w + hw::wdt::TASKS_START), 1);
        hw::watchdog_feed();
        assert_eq!(sim::read(w + hw::wdt::RR0), hw::wdt::RELOAD);
    }

    /// The reload value covers four housekeeping periods of the time base
    /// (the handler runs at least once per period), saturating for slow
    /// or absent time bases.
    #[test]
    fn watchdog_reload_covers_four_housekeeping_periods() {
        assert_eq!(entry::watchdog_reload(16_000_000), 16_384);
        assert_eq!(entry::watchdog_reload(25_000_000), 10_485);
        assert_eq!(entry::watchdog_reload(0), u32::MAX);
        assert_eq!(entry::watchdog_reload(1), u32::MAX);
    }

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
