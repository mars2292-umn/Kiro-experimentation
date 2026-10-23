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
pub use plan::{compare_target, find_level, plan_frame, ArchState, LevelRecord, FRAME_BYTES, MAX_INSTANT};
pub use unsafe_module::KernelCell;

use crate::arch::{KernelOps, System};
use crate::logic::log::{EV_BUSFAULT, EV_HARDFAULT, EV_MEMMANAGE, EV_PANIC, EV_USAGEFAULT};
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
        S::IRQ_COUNT
    }
    fn config_checksum(&self) -> u32 {
        S::CONFIG_CHECKSUM
    }
    fn timer_vector(&self) -> u16 {
        S::TIMER_VECTOR
    }
    fn priority_bits(&self) -> u8 {
        S::PRIORITY_BITS
    }
    fn boot_expect(&self) -> plan::BootExpect {
        plan::BootExpect { priority_bits: S::PRIORITY_BITS, cpuid: S::CPUID, hfclk_xtal: S::HFCLK_XTAL, fpu_needed: S::FPU_NEEDED }
    }
    fn tick_hz(&self) -> u64 {
        S::TICK_HZ
    }
    fn dma_count(&self) -> usize {
        S::DMA_COUNT
    }
    fn dma(&self, i: usize) -> plan::DmaCfg {
        S::dma(i)
    }
}

// The shells below are the only Kernel_Arch code outside `verus!` besides
// the Kernel_Unsafe_Module: each fetches the Kernel, the hardware, the
// tables, and the Kernel_Arch state, calls its verified core
// (`core.rs`), and enters the system safe state when the core says so.

/// Boot (R12.4): `false` when a boot check fails (R12.1, R12.2); the caller
/// enters the safe state.
pub fn boot<S: System>() -> bool {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    entry::boot_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state())
}

/// Instant 0: enables Releases (PR-34) and the compare of the first event.
pub fn start<S: System>() {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    if entry::start_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state()) {
        S::safe_state();
    }
}

/// The Rust half of the dispatch stub, with PRIMASK held.
pub fn irq_entry<S: System>(ipsr: u32, psp: u32, control: u32, exc_return: u32) -> DispatchPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let (plan, safe) = entry::irq_entry_core(
        k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), ipsr, psp, control, exc_return,
    );
    if safe {
        S::safe_state();
    }
    plan
}

/// The Kernel-level timer handler body.
pub fn timer<S: System>() -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let (hop, safe) = entry::timer_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state());
    if safe {
        S::safe_state();
    }
    hop
}

/// The Rust half of the SVC handler: `frame` points at the caller's
/// exception frame (R0-R3 at offsets 0 to 12, PC at 24).
pub fn svc<S: System>(frame: &mut [u32; 8]) -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let number = unsafe_module::svc_number(frame[6]);
    let (hop, safe) =
        entry::svc_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), frame, number);
    if safe {
        S::safe_state();
    }
    hop
}

/// The Rust half of the unwind of one level, named by its exception
/// number `n`: (PSP, CONTROL, EXC_RETURN, next), where `next` names one
/// more level to unwind when the resumed Job was killed, else 0.
pub fn epilogue<S: System>(n: u32) -> (u32, u32, u32, ReturnPlan) {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let (r, safe) = entry::epilogue_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), n);
    if safe {
        S::safe_state();
    }
    r
}

/// Fault handlers (R14.3).
pub fn fault<S: System>(kind: u8, in_kernel: bool) -> ReturnPlan {
    let k = S::kernel().get();
    let mut hw = Cm4Hw::new();
    let (hop, safe) =
        entry::fault_core(k, &mut hw, &SysTables::<S>(PhantomData), unsafe_module::arch_state(), kind, in_kernel);
    if safe {
        S::safe_state();
    }
    hop
}

pub fn mem_manage<S: System>(in_kernel: bool) -> ReturnPlan {
    Cm4Hw::new().apply(Action::ClearFaultStatus);
    fault::<S>(EV_MEMMANAGE, in_kernel)
}

pub fn bus_fault<S: System>(in_kernel: bool) -> ReturnPlan {
    Cm4Hw::new().apply(Action::ClearFaultStatus);
    fault::<S>(EV_BUSFAULT, in_kernel)
}

pub fn usage_fault<S: System>(in_kernel: bool) -> ReturnPlan {
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
/// its Partition; one in Kernel code is a Kernel fault.
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
    let _ = entry::apply_core(k, &mut hw, &SysTables::<S>(PhantomData));
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
        sim::write(hw::addr::NRF_RESETREAS, (1 << 1) | 1);
        assert_eq!(hw::boot_info(), hw::BOOT_INFO_DEBUGGER | hw::BOOT_INFO_WATCHDOG_RESET);
        assert_eq!(sim::read(hw::addr::NRF_RESETREAS), 1 << 1, "the DOG bit was written to clear it");
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
