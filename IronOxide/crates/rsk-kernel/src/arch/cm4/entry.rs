//! The Kernel_Arch entry points inside `verus!` (task 5.7; GAP-10).
//!
//! Every entry point of `arch::cm4` (boot, start, dispatch entry, SVC,
//! timer, unwind, faults) is a verified *core* here, generic over three
//! traits:
//!
//! - [`KernelOps`] (`arch::bound`): the bound Kernel, whose specification
//!   ties every transition to the verified logic;
//! - [`KernelArch`]: the hardware. Its one mutating operation, `apply`,
//!   performs one [`Action`] (a register write, an NVIC pend or mask, an
//!   MPU view load, a TIMER compare, a frame write) and is specified as a
//!   step of the ghost [`HwModel`], the Kernel's view of the state it has
//!   written to the hardware. [`Cm4Hw`] implements it with four trusted
//!   functions (`#[verifier::external_body]`, listed in
//!   `docs/proof_assumptions.toml`): `apply`, which dispatches to the
//!   register-access functions of the Kernel_Unsafe_Module, and the three
//!   reads `now`, `cycles`, `boot_checks`;
//! - [`Tables`]: what the generated system crate knows (vector bindings,
//!   stack bounds, entry addresses, Target constants). `arch::cm4` adapts
//!   the `System` trait to it.
//!
//! The thin shells in `arch::cm4` (outside `verus!`) only fetch the three
//! objects, call the core, and enter the system safe state when the core
//! says so (`S::safe_state()` never returns, which `verus!` cannot express).
//!
//! What the cores prove, beyond absence of panics and overflow and the
//! preconditions of every `KernelOps` call (Verification_Plan section 4):
//!
//! - **A1**: after every transition that writes BASEPRI (`apply_core`,
//!   the dispatch path, SVC lock, the unwind), the model's BASEPRI is
//!   `hw_model::basepri_for(job_level, ceiling)` of the Kernel state
//!   (`basepri_matches`), or 0 in the safe state.
//! - **A3**: `boot_core` programs every dispatch vector at
//!   `encode_level(priority)` and the Kernel exceptions at level 0, with
//!   PRIGROUP all-preempt, NONBASETHRDENA clear, DISDEFWBUF and eager FP
//!   stacking set, the MPU, time base, and DWT enabled.
//! - **A5**: a dispatched Job returns to Thread mode on the PSP with
//!   CONTROL = 0b001 (unprivileged, no FP context) on an 8-byte-aligned
//!   frame inside its Partition's stack (`plan_frame`), with its
//!   Partition's view loaded (A6, together with `Bound::inv`).
//! - **A11**: `boot_core` calls `validate` before `init` before any other
//!   operation, so the invariant holds at the first transition.
//!
//! Not modelled: the effect of the NVIC taking an exception (the pending
//! set only records what the Kernel pended), PRIMASK inside the assembly
//! stubs (A2), the RBAR/RASR word encoding (A7, Kani), and the stub-level
//! register save and restore (Property 2).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::plan::{compare_target, find_level, plan_frame, ArchState, LevelRecord, MAX_INSTANT};
use crate::arch::bound::{encode_level, KernelOps};
use crate::hw_model;
use crate::logic::log::{EV_HARDFAULT, EV_PANIC, EV_UNBOUND_IRQ};
use crate::logic::mpu::View;

verus! {

// ------------------------------------------------------------- constants

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
pub const XPSR_THUMB: u32 = 0x0100_0000;
/// CONTROL for a Job: nPRIV = 1 (unprivileged Thread mode), FPCA = 0.
pub const CONTROL_JOB: u32 = 1;
/// Partition stack floors kept by `ArchState` (PAR-07 bounds Partitions well below this).
pub const MAX_FLOORS: usize = 16;
/// The Armv7-M maximum of external interrupt lines.
pub const MAX_IRQS: u16 = 496;
/// Boot-check bit for a configuration checksum mismatch (R12.3).
pub const BOOT_FAIL_CHECKSUM: u32 = 32;
/// Boot-check bit for an interrupt-line count the Kernel_Arch cannot serve.
pub const BOOT_FAIL_IRQ_COUNT: u32 = 64;

/// How an exception vector is bound (Generated_Config `irq` table, R24.1).
#[derive(Clone, Copy)]
pub enum Binding {
    /// The dispatch vector of a Task (DD-01).
    Dispatch(u8),
    /// A peripheral interrupt that releases a Sporadic_Task; the Partition
    /// that owns the peripheral is attributed MIT_Violations (R10.7).
    Intake { task: u8, partition: u8 },
    /// A Kernel-owned interrupt line (the time-base timer).
    Kernel,
    /// Not bound: must stay disabled; an exception from it is a Kernel fault (R19.5).
    Unbound,
}

/// The plan the dispatch stub executes after `irq_entry`.
#[derive(Clone, Copy)]
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

// ------------------------------------------------------ hardware actions

/// One hardware effect of Kernel_Arch (DD-01 "actions"), applied by
/// `KernelArch::apply`.
#[derive(Clone, Copy)]
pub enum Action {
    /// MSR BASEPRI (HW-03).
    SetBasepri(u8),
    /// NVIC ISPR: pend exception `n` (HW-05).
    Pend(u16),
    /// NVIC ISER / ICER.
    Enable(u16),
    Disable(u16),
    /// NVIC IPR: the encoded priority of exception `n` (HW-01).
    SetPriority(u16, u8),
    /// TIMER compare for an instant (HW-12).
    ArmCompare(u64),
    TimebaseAck,
    TimebaseInit,
    DwtEnable,
    /// MPU RBAR/RASR for every region of a Partition's view (HW-09).
    LoadView(u8, View),
    FpuAccess(bool),
    /// The initial exception frame of a Job at (frame address, entry address) (HW-06).
    WriteFrame(u32, u32),
    PrimaskSet,
    PrimaskClear,
    PrigroupAllPreempt,
    Nonbasethrdena(bool),
    Disdefwbuf(bool),
    FpuEager,
    KernelExceptionPriorities,
    MpuEnableBackground,
    ClearFaultStatus,
    /// Debug trace (feature `trace` only): tag, value.
    Trace(u8, u32),
}

/// The Kernel's view of what it has written to the hardware: the ghost
/// state of `KernelArch`. `pending` and `enabled` record the Kernel's own
/// pends and masks; what the NVIC then takes is the Hardware_Model's
/// business (`hw_model::nvic_selects`).
pub struct HwModel {
    pub basepri: int,
    pub pending: Set<int>,
    pub enabled: Set<int>,
    pub priority: Map<int, int>,
    pub compare: Option<int>,
    /// The Partition whose view is loaded, `None` for the Kernel map.
    pub view: Option<int>,
    pub fpu_access: bool,
    pub primask: bool,
    pub prigroup_all_preempt: bool,
    pub nonbasethrdena: bool,
    pub disdefwbuf: bool,
    pub fpu_eager: bool,
    pub kernel_exceptions_level0: bool,
    pub mpu_enabled: bool,
    pub timebase_running: bool,
    pub dwt_enabled: bool,
}

/// The model's step for one action.
pub open spec fn step(h: HwModel, a: Action) -> HwModel {
    match a {
        Action::SetBasepri(v) => HwModel { basepri: v as int, ..h },
        Action::Pend(n) => HwModel { pending: h.pending.insert(n as int), ..h },
        Action::Enable(n) => HwModel { enabled: h.enabled.insert(n as int), ..h },
        Action::Disable(n) => HwModel { enabled: h.enabled.remove(n as int), ..h },
        Action::SetPriority(n, p) => HwModel { priority: h.priority.insert(n as int, p as int), ..h },
        Action::ArmCompare(at) => HwModel { compare: Some(at as int), ..h },
        Action::TimebaseAck => h,
        Action::TimebaseInit => HwModel { timebase_running: true, ..h },
        Action::DwtEnable => HwModel { dwt_enabled: true, ..h },
        Action::LoadView(p, _) => HwModel { view: Some(p as int), ..h },
        Action::FpuAccess(b) => HwModel { fpu_access: b, ..h },
        Action::WriteFrame(_, _) => h,
        Action::PrimaskSet => HwModel { primask: true, ..h },
        Action::PrimaskClear => HwModel { primask: false, ..h },
        Action::PrigroupAllPreempt => HwModel { prigroup_all_preempt: true, ..h },
        Action::Nonbasethrdena(b) => HwModel { nonbasethrdena: b, ..h },
        Action::Disdefwbuf(b) => HwModel { disdefwbuf: b, ..h },
        Action::FpuEager => HwModel { fpu_eager: true, ..h },
        Action::KernelExceptionPriorities => HwModel { kernel_exceptions_level0: true, ..h },
        Action::MpuEnableBackground => HwModel { mpu_enabled: true, ..h },
        Action::ClearFaultStatus => h,
        Action::Trace(_, _) => h,
    }
}

/// The hardware as Kernel_Arch drives it.
pub trait KernelArch: Sized {
    /// The ghost model of what has been written.
    spec fn hw(&self) -> HwModel;
    /// Performs one action; the model takes the corresponding step.
    fn apply(&mut self, a: Action)
        ensures final(self).hw() == step(old(self).hw(), a);
    /// The 64-bit time base (DD-05, HW-12).
    fn now(&self) -> u64;
    /// DWT CYCCNT (HW-13).
    fn cycles(&self) -> u32;
    /// The boot checks of R12.1 as a bit set of failures (0 when all pass).
    fn boot_checks(&self, priority_bits: u8) -> u32;
}

/// What the generated system crate provides (the `System` trait of
/// `arch`, adapted by `arch::cm4`).
pub trait Tables {
    fn binding(&self, n: u16) -> Binding;
    fn dispatch_vector(&self, t: u8) -> u16;
    fn intake_vector(&self, t: u8) -> Option<u16>;
    fn stack_top(&self, p: u8) -> u32;
    fn stack_bottom(&self, p: u8) -> u32;
    /// The address of Task `t`'s Job wrapper.
    fn task_entry_address(&self, t: u8) -> u32;
    fn irq_count(&self) -> u16;
    fn config_checksum(&self) -> u32;
    fn timer_vector(&self) -> u16;
    fn priority_bits(&self) -> u8;
}

// ------------------------------------------------ the Cortex-M4F hardware

/// The Cortex-M4F registers behind `KernelArch`. The ghost `model` is the
/// state the Kernel has written since this value was created (one value
/// per entry point); the four trusted functions below are the whole
/// register-facing surface of the planning code.
pub struct Cm4Hw {
    pub model: Ghost<HwModel>,
}

impl Cm4Hw {
    pub fn new() -> Cm4Hw {
        Cm4Hw { model: Ghost(arbitrary()) }
    }
}

impl KernelArch for Cm4Hw {
    open spec fn hw(&self) -> HwModel {
        self.model@
    }

    /// Trusted (docs/proof_assumptions.toml): each action is exactly one
    /// register-access function of the Kernel_Unsafe_Module, whose own
    /// justification identifier names its Kani harness or review.
    #[verifier::external_body]
    fn apply(&mut self, a: Action) {
        use super::unsafe_module::hw;
        match a {
            Action::SetBasepri(v) => hw::write_basepri(v),
            Action::Pend(n) => hw::nvic_set_pending(n),
            Action::Enable(n) => hw::nvic_enable(n),
            Action::Disable(n) => hw::nvic_disable(n),
            Action::SetPriority(n, p) => hw::nvic_set_priority(n, p),
            Action::ArmCompare(at) => hw::timebase_arm(at),
            Action::TimebaseAck => hw::timebase_ack(),
            Action::TimebaseInit => hw::timebase_init(),
            Action::DwtEnable => hw::dwt_enable(),
            Action::LoadView(_, view) => hw::mpu_load(&view),
            Action::FpuAccess(on) => hw::fpu_set_access(on),
            Action::WriteFrame(frame, entry) => super::unsafe_module::build_frame(
                frame,
                entry,
                super::unsafe_module::stubs::job_end_trampoline as *const () as usize as u32,
            ),
            Action::PrimaskSet => hw::primask_set(),
            Action::PrimaskClear => hw::primask_clear(),
            Action::PrigroupAllPreempt => hw::set_prigroup_all_preempt(),
            Action::Nonbasethrdena(on) => hw::set_nonbasethrdena(on),
            Action::Disdefwbuf(on) => hw::set_disdefwbuf(on),
            Action::FpuEager => hw::fpu_configure_eager(),
            Action::KernelExceptionPriorities => hw::set_kernel_exception_priorities(),
            Action::MpuEnableBackground => hw::mpu_enable_background(),
            Action::ClearFaultStatus => hw::clear_fault_status(),
            Action::Trace(tag, v) => super::trace(super::trace_tag(tag), v),
        }
    }

    /// Trusted: the extended TIMER counter (DD-05).
    #[verifier::external_body]
    fn now(&self) -> u64 {
        super::unsafe_module::hw::timebase_now()
    }

    /// Trusted: DWT CYCCNT (HW-13).
    #[verifier::external_body]
    fn cycles(&self) -> u32 {
        super::unsafe_module::hw::dwt_cyccnt()
    }

    /// Trusted: the hardware boot checks of R12.1.
    #[verifier::external_body]
    fn boot_checks(&self, priority_bits: u8) -> u32 {
        super::unsafe_module::hw::boot_checks(priority_bits)
    }
}

// ---------------------------------------------------------- the cores

/// Trace tags (`Action::Trace`).
pub const TR_BOOT: u8 = 1;
pub const TR_INVALID: u8 = 2;
pub const TR_BOOTFAIL: u8 = 3;
pub const TR_BOOTED: u8 = 4;
pub const TR_START: u8 = 5;
pub const TR_IRQ: u8 = 6;
pub const TR_DISPATCH: u8 = 7;
pub const TR_TICK: u8 = 8;
pub const TR_SVC: u8 = 9;
pub const TR_EPILOGUE: u8 = 10;
pub const TR_FAULT: u8 = 11;
pub const TR_ARM: u8 = 12;

/// A1: the model's BASEPRI is the Hardware_Model's value for the Kernel
/// state (DD-01, HW-03).
pub open spec fn basepri_matches<K: KernelOps, A: KernelArch>(k: &K, hw: &A) -> bool {
    hw.hw().basepri == hw_model::basepri_for(k.top_level(), k.ceiling_level())
}

/// The boot flags `boot_core` establishes (A3 and DD-02, DD-04, DD-07, DD-12).
pub open spec fn booted(h: HwModel) -> bool {
    &&& h.prigroup_all_preempt
    &&& !h.nonbasethrdena
    &&& h.disdefwbuf
    &&& h.fpu_eager
    &&& h.kernel_exceptions_level0
    &&& h.mpu_enabled
    &&& h.timebase_running
    &&& h.dwt_enabled
}

/// Cycles consumed by the executing context since the last switch
/// (HW-13), updating the switch point.
fn charge_point<A: KernelArch>(hw: &A, state: &mut ArchState) -> u32 {
    let now = hw.cycles();
    let elapsed = now.wrapping_sub(state.last_switch);
    state.last_switch = now;
    elapsed
}

/// Arms the time-base compare for `at`, never closer than
/// `MIN_COMPARE_DISTANCE` ticks (HW-12, `compare_target`). The time base
/// counts from instant 0 and the operating duration is below 2^62 (R9.5),
/// so the guard is never taken on the Target; it keeps the call total.
fn arm_compare<A: KernelArch>(hw: &mut A, at: u64)
    ensures final(hw).hw().basepri == old(hw).hw().basepri,
        final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
        final(hw).hw().compare.is_some(),
{
    let now = hw.now();
    let target = if now <= MAX_INSTANT { compare_target(now, at) } else { at };
    hw.apply(Action::ArmCompare(target));
}

/// Applies the Kernel's hardware-facing state after a transition: BASEPRI
/// from the executing Job's level and the System_Ceiling (A1), the compare
/// of the next timed event, the pending bit of the selected Job's vector
/// (A4: only the selection of `KernelOps::select`), the intake source
/// masks, and the Partition requests. Returns whether the Kernel is in the
/// safe state; the shell then enters `System::safe_state`.
pub fn apply_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T) -> (safe: bool)
    requires old(k).inv(),
    ensures
        final(k).inv(),
        final(k).top_level() == old(k).top_level(),
        final(k).ceiling_level() == old(k).ceiling_level(),
        !safe ==> basepri_matches(final(k), final(hw)),
        safe ==> final(hw).hw().basepri == 0,
        final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
{
    hw.apply(Action::SetBasepri(k.basepri_now()));
    match k.next_compare() {
        Some(at) => arm_compare(hw, at),
        None => {}
    }
    match k.select() {
        Some(t) => hw.apply(Action::Pend(tables.dispatch_vector(t))),
        None => {}
    }
    let ghost h1 = hw.hw();
    let ghost k1 = *k;
    let n = k.task_count();
    let mut t: usize = 0;
    while t < n
        invariant
            k.inv(), *k == k1, n <= 255,
            hw.hw().basepri == h1.basepri, hw.hw().view == h1.view, hw.hw().primask == h1.primask,
        decreases n - t,
    {
        match tables.intake_vector(t as u8) {
            Some(v) => {
                if k.source_masked(t as u8) {
                    hw.apply(Action::Disable(v));
                } else {
                    hw.apply(Action::Enable(v));
                }
            }
            None => {}
        }
        t += 1;
    }
    let np = k.partition_count();
    let mut p: usize = 0;
    while p < np
        invariant
            k.inv(), np <= 255,
            k.top_level() == k1.top_level(), k.ceiling_level() == k1.ceiling_level(),
            hw.hw().basepri == h1.basepri, hw.hw().view == h1.view, hw.hw().primask == h1.primask,
        decreases np - p,
    {
        // EasyDMA stop (DD-08) and the Partition image re-initialization
        // are performed by the system crate (tasks 5.5, 10.5); the requests
        // are consumed here so that the flags are cleared.
        let _ = k.take_stop_dma(p as u8);
        let _ = k.take_reinit(p as u8);
        p += 1;
    }
    if k.safe_state() {
        hw.apply(Action::SetBasepri(0));
        true
    } else {
        false
    }
}

/// Boot (R12.4: before any Release): validates the tables (R12.1, R12.3)
/// and initializes the Kernel before anything else touches it (A11), then
/// programs priority grouping, the Kernel-level priorities, NONBASETHRDENA,
/// precise BusFaults, eager FP stacking, every vector's priority (A3), the
/// Partition stack floors, the MPU background map, the time base, and the
/// DWT. Returns `false` when a check fails (R12.2); the shell then enters
/// the safe state.
pub fn boot_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState) -> (ok: bool)
    ensures
        ok ==> final(k).inv(),
        ok ==> booted(final(hw).hw()),
        ok ==> final(state).booted,
{
    hw.apply(Action::Trace(TR_BOOT, 0));
    if !k.validate() {
        hw.apply(Action::Trace(TR_INVALID, 0));
        return false;
    }
    k.init();
    let mut failed = hw.boot_checks(tables.priority_bits());
    if k.config_checksum() != tables.config_checksum() {
        failed = failed | BOOT_FAIL_CHECKSUM;
    }
    let irqs = tables.irq_count();
    if irqs > MAX_IRQS {
        // An interrupt-line count the Kernel_Arch cannot serve (Armv7-M
        // allows 496): a configuration failure like the checks above.
        failed = failed | BOOT_FAIL_IRQ_COUNT;
        hw.apply(Action::Trace(TR_BOOTFAIL, failed));
        k.boot_failed(failed);
        return false;
    }
    if failed != 0 {
        hw.apply(Action::Trace(TR_BOOTFAIL, failed));
        k.boot_failed(failed);
        return false;
    }
    hw.apply(Action::PrigroupAllPreempt);
    // No Job executes with an exception active (DD-01 as revised), so
    // NONBASETHRDENA stays at its reset value 0 and every exception return
    // to Thread mode is a return to the base level.
    hw.apply(Action::Nonbasethrdena(false));
    hw.apply(Action::Disdefwbuf(true));
    hw.apply(Action::FpuEager);
    hw.apply(Action::KernelExceptionPriorities);
    let ghost h1 = hw.hw();
    let end: u16 = 16 + irqs;
    let mut n: u16 = 16;
    while n < end
        invariant
            k.inv(), end <= 16 + MAX_IRQS,
            hw.hw().prigroup_all_preempt == h1.prigroup_all_preempt, hw.hw().nonbasethrdena == h1.nonbasethrdena,
            hw.hw().disdefwbuf == h1.disdefwbuf, hw.hw().fpu_eager == h1.fpu_eager,
            hw.hw().kernel_exceptions_level0 == h1.kernel_exceptions_level0,
            hw.hw().mpu_enabled == h1.mpu_enabled, hw.hw().timebase_running == h1.timebase_running,
            hw.hw().dwt_enabled == h1.dwt_enabled,
        decreases end - n,
    {
        hw.apply(Action::Disable(n));
        match tables.binding(n) {
            Binding::Dispatch(t) => {
                let p = k.job_info(t).priority;
                hw.apply(Action::SetPriority(n, encode_level(p)));
                hw.apply(Action::Enable(n));
            }
            Binding::Intake { task: _, partition: _ } => {
                hw.apply(Action::SetPriority(n, encode_level(0)));
                hw.apply(Action::Enable(n));
            }
            Binding::Kernel => {
                hw.apply(Action::SetPriority(n, encode_level(0)));
                hw.apply(Action::Enable(n));
            }
            Binding::Unbound => {}
        }
        n += 1;
    }
    let np = k.partition_count();
    let mut p: usize = 0;
    while p < np && p < MAX_FLOORS
        invariant p <= MAX_FLOORS,
        decreases MAX_FLOORS - p,
    {
        state.floor[p] = tables.stack_top(p as u8);
        p += 1;
    }
    hw.apply(Action::MpuEnableBackground);
    hw.apply(Action::TimebaseInit);
    hw.apply(Action::DwtEnable);
    state.booted = true;
    hw.apply(Action::Trace(TR_BOOTED, 1));
    true
}

/// Instant 0: enables Releases (PR-34) and the compare of the first event,
/// under PRIMASK. Returns whether the Kernel entered the safe state (the
/// shell then enters `System::safe_state` with PRIMASK still set).
pub fn start_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState) -> (safe: bool)
    requires old(k).inv(),
    ensures
        final(k).inv(),
        !safe ==> basepri_matches(final(k), final(hw)) && !final(hw).hw().primask,
{
    hw.apply(Action::PrimaskSet);
    state.last_switch = hw.cycles();
    k.start();
    let safe = apply_core(k, hw, tables);
    hw.apply(Action::Trace(TR_START, 0));
    if !safe {
        hw.apply(Action::PrimaskClear);
    }
    safe
}

/// After a transition that may have ended the executing Job: the unwind
/// plan naming its level's exception number, or 0.
fn end_plan_core<K: KernelOps>(k: &K, state: &ArchState, top_before: Option<u8>) -> ReturnPlan
    requires k.inv(),
{
    match top_before {
        Some(t) => {
            let still = match k.top() {
                Some(u) => u == t,
                None => false,
            };
            if still {
                0
            } else {
                let level = k.job_info(t).priority as usize;
                state.levels[level].exception_number as u32
            }
        }
        None => 0,
    }
}

/// The Rust half of the dispatch stub, with PRIMASK held. Returns the plan
/// and whether the Kernel entered the safe state.
pub fn irq_entry_core<K: KernelOps, A: KernelArch, T: Tables>(
    k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState,
    ipsr: u32, psp: u32, control: u32, exc_return: u32,
) -> (r: (DispatchPlan, bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        // A dispatch (a non-zero frame address) returns to unprivileged
        // Thread mode on the PSP (A5), on an 8-byte-aligned frame (HW-06),
        // with the Job's Partition view loaded (A6) and BASEPRI at the
        // Job's level or the System_Ceiling (A1).
        r.0.psp != 0 ==> r.0.control == CONTROL_JOB && r.0.exc_return == EXC_RETURN_THREAD_PSP
            && r.0.psp % 8 == 0 && r.0.hop == 0 && !r.1
            && final(hw).hw().view.is_some() && basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
{
    let no_change = DispatchPlan { psp: 0, control, exc_return, hop: 0 };
    let n = (ipsr & 0x1FF) as u16;
    hw.apply(Action::Trace(TR_IRQ, n as u32));
    match tables.binding(n) {
        Binding::Dispatch(t) => {
            if !k.eligible(t) {
                // A stale pending bit (the Job was suppressed after it was
                // pended): nothing to dispatch (A4).
                return (no_change, false);
            }
            let preempted = charge_point(hw, state);
            let previous_partition = state.current_partition;
            if previous_partition != 0xFF && (previous_partition as usize) < MAX_FLOORS && (exc_return & 0xC) == 0xC {
                // The preempted context is a Job in Thread mode on the PSP:
                // its live stack pointer (below its stacked frame, HW-06)
                // is the floor of its Partition's stack from here on.
                state.floor[previous_partition as usize] = psp;
            }
            k.dispatch(t, preempted);
            let info = k.job_info(t);
            let level = info.priority as usize;
            if (info.partition as usize) >= MAX_FLOORS {
                // More Partitions than `ArchState` keeps floors for: a
                // configuration the Generator rejects; the backstop is a
                // Kernel fault into the safe state.
                k.fault(EV_HARDFAULT, true);
                let safe = apply_core(k, hw, tables);
                return (no_change, safe);
            }
            let floor_before = state.floor[info.partition as usize];
            // The frame is 8-byte aligned and inside the Partition's stack
            // region (`plan_frame`); the Link_Checker's stack bound (R37.2)
            // excludes the overflow case, and the backstop is a Kernel fault
            // into the safe state (R16.5).
            let frame = match plan_frame(floor_before, tables.stack_bottom(info.partition)) {
                Some(frame) => frame,
                None => {
                    k.fault(EV_HARDFAULT, true);
                    let safe = apply_core(k, hw, tables);
                    return (no_change, safe);
                }
            };
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
            hw.apply(Action::WriteFrame(frame, tables.task_entry_address(t)));
            let view = *k.partition_view(info.partition);
            hw.apply(Action::LoadView(info.partition, view));
            state.current_partition = info.partition;
            // The dispatched Job is the executing one, so its level or the
            // System_Ceiling is `basepri_now` (A1).
            hw.apply(Action::SetBasepri(k.basepri_now()));
            match k.next_compare() {
                Some(at) => arm_compare(hw, at),
                None => {}
            }
            hw.apply(Action::FpuAccess(info.fpu));
            hw.apply(Action::Trace(TR_DISPATCH, frame));
            (DispatchPlan { psp: frame, control: CONTROL_JOB, exc_return: EXC_RETURN_THREAD_PSP, hop: 0 }, false)
        }
        Binding::Intake { task, partition } => {
            let now = hw.now();
            hw.apply(Action::Disable(n));
            let top_before = k.top();
            k.intake(task, partition, now);
            let safe = apply_core(k, hw, tables);
            let hop = end_plan_core(k, state, top_before);
            (DispatchPlan { psp: 0, control, exc_return, hop }, safe)
        }
        Binding::Kernel => {
            if n == tables.timer_vector() {
                let (hop, safe) = timer_core(k, hw, tables, state);
                (DispatchPlan { psp: 0, control, exc_return, hop }, safe)
            } else {
                (no_change, false)
            }
        }
        Binding::Unbound => {
            hw.apply(Action::Disable(n));
            let top_before = k.top();
            k.fault(EV_UNBOUND_IRQ, true);
            let safe = apply_core(k, hw, tables);
            let hop = end_plan_core(k, state, top_before);
            (DispatchPlan { psp: 0, control, exc_return, hop }, safe)
        }
    }
}

/// The Kernel-level timer handler body. Returns the unwind plan when the
/// executing Job was ended by a response, and the safe-state flag.
pub fn timer_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState) -> (r: (ReturnPlan, bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        !r.1 ==> basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
{
    hw.apply(Action::TimebaseAck);
    let now = hw.now();
    hw.apply(Action::Trace(TR_TICK, now as u32));
    let running = charge_point(hw, state);
    let top_before = k.top();
    k.timer_event(now, running);
    let safe = apply_core(k, hw, tables);
    let hop = end_plan_core(k, state, top_before);
    (hop, safe)
}

/// The Rust half of the SVC handler: `frame` is the caller's exception
/// frame (R0-R3 at words 0 to 3, PC at word 6); `number` the SVC number
/// the shell read from the instruction. Lock and unlock leave BASEPRI at
/// the executing Job's level or the (new) System_Ceiling (A1).
pub fn svc_core<K: KernelOps, A: KernelArch, T: Tables>(
    k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState, frame: &mut [u32; 8], number: u8,
) -> (r: (ReturnPlan, bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        number == SVC_LOCK || number == SVC_UNLOCK ==> !r.1 ==> basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
{
    hw.apply(Action::Trace(TR_SVC, number as u32));
    if number == SVC_LOCK {
        let ok = k.svc_lock(frame[0] as u8);
        frame[0] = if ok { 1 } else { 0 };
        hw.apply(Action::SetBasepri(k.basepri_now()));
        (0, false)
    } else if number == SVC_UNLOCK {
        let ok = k.svc_unlock(frame[0] as u8);
        frame[0] = if ok { 1 } else { 0 };
        let safe = apply_core(k, hw, tables);
        (0, safe)
    } else if number == SVC_NOW {
        let now = hw.now();
        frame[0] = now as u32;
        frame[1] = (now >> 32) as u32;
        (0, false)
    } else if number == SVC_JOB_END {
        let elapsed = charge_point(hw, state);
        let top_before = k.top();
        k.job_return(elapsed);
        let safe = apply_core(k, hw, tables);
        let hop = end_plan_core(k, state, top_before);
        (hop, safe)
    } else {
        // SVC_PANIC and anything else: a fault of the executing Job's
        // Partition (PR-19, R14.2).
        k.fault(EV_PANIC, false);
        let safe = apply_core(k, hw, tables);
        let top_before = k.top();
        let hop = end_plan_core(k, state, top_before);
        (hop, safe)
    }
}

/// The Rust half of the unwind of one level, named by its exception
/// number `n`: restores the preempted context recorded for the level and
/// returns (PSP, CONTROL, EXC_RETURN, next), where `next` names one more
/// level to unwind when the resumed Job was killed, else 0; and the
/// safe-state flag. BASEPRI ends at the resumed Job's level or the
/// System_Ceiling (A1), and the resumed Job's Partition view is loaded (A6).
pub fn epilogue_core<K: KernelOps, A: KernelArch, T: Tables>(
    k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState, n: u32,
) -> (r: ((u32, u32, u32, ReturnPlan), bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        !r.1 ==> basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
{
    let n = (n & 0x1FF) as u16;
    hw.apply(Action::Trace(TR_EPILOGUE, n as u32));
    let level = find_level(&state.levels, n);
    let rec = state.levels[level];
    if (rec.partition as usize) < MAX_FLOORS {
        state.floor[rec.partition as usize] = rec.floor_before;
    }
    state.last_switch = hw.cycles();
    // Restore the preempted Job's view.
    if rec.previous_partition != 0xFF {
        let view = *k.partition_view(rec.previous_partition);
        hw.apply(Action::LoadView(rec.previous_partition, view));
        let d = k.depth();
        let idx = if d == 0 { 0 } else { d - 1 };
        hw.apply(Action::FpuAccess(k.job_info(k.stack_at(idx)).fpu));
    }
    state.current_partition = rec.previous_partition;
    // A killed resumed Job ends now and its level is unwound as well.
    match k.top() {
        Some(t) => {
            if k.is_killed(t) {
                k.job_return(0);
                let safe = apply_core(k, hw, tables);
                let l = k.job_info(t).priority as usize;
                let next = state.levels[l].exception_number as u32;
                return ((rec.psp, rec.control, rec.exc_return, next), safe);
            }
        }
        None => {}
    }
    hw.apply(Action::SetBasepri(k.basepri_now()));
    ((rec.psp, rec.control, rec.exc_return, 0), false)
}

/// Fault handlers (R14.3): a fault while a Job executes is attributed to
/// its Partition, a fault at the Kernel level or with PRIMASK held, or any
/// HardFault, to the Kernel.
pub fn fault_core<K: KernelOps, A: KernelArch, T: Tables>(
    k: &mut K, hw: &mut A, tables: &T, state: &ArchState, kind: u8, in_kernel: bool,
) -> (r: (ReturnPlan, bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        !r.1 ==> basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
{
    hw.apply(Action::Trace(TR_FAULT, kind as u32));
    let kernel = in_kernel || kind == EV_HARDFAULT || k.depth() == 0;
    let top_before = k.top();
    k.fault(kind, kernel);
    let safe = apply_core(k, hw, tables);
    let hop = end_plan_core(k, state, top_before);
    (hop, safe)
}

} // verus!
