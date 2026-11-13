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
//!   written to the hardware. [`Cm4Hw`] implements it with five trusted
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

use crate::logic::kernel::HOUSEKEEPING_PERIOD;
use vstd::prelude::*;

use super::plan::{compare_target, find_level, plan_frame, ArchState, LevelRecord, MAX_INSTANT, BootExpect, DmaCfg, SignalCfg, EndpointCfg, MAX_ENDPOINTS, MAX_CHANNELS, MAX_ROLES, MAX_ROLE_SLOTS};
use crate::arch::bound::{encode_level, KernelOps};
use crate::hw_model;
use crate::logic::log::{EV_HARDFAULT, EV_PANIC, EV_UNBOUND_IRQ, EV_DMA_REJECTED, EV_API_MISUSE, EV_ENDPOINT_FULL, EV_PROTOCOL_VIOLATION, EV_MESSAGE_REJECTED};
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
/// EasyDMA request (DD-08, R17.2): r0 channel, r1 buffer address, r2 length; r0 out 0 accepted, 1 refused.
pub const SVC_DMA: u8 = 6;
/// Release_Signal raise (R10.5): r0 signal; r0 out 0 accepted as an event, 1 refused.
pub const SVC_RAISE: u8 = 7;
/// The Endpoint send and receive services (task 21.2; DD-09, R42.2): r0 is
/// the Endpoint index in its low byte and the role in bit 8, r1 the
/// message's address in the Partition, r2 its length; r0 returns
/// `ENDPOINT_OK`, `ENDPOINT_FULL` (send) or `ENDPOINT_EMPTY` (receive),
/// `ENDPOINT_REFUSED` (API misuse: a fault of the Partition), or
/// `ENDPOINT_PEER` (the peer-state change of R42.3, task 21.4).
pub const SVC_SEND: u8 = 8;
pub const SVC_RECV: u8 = 9;
pub const ENDPOINT_OK: u32 = 0;
pub const ENDPOINT_FULL: u32 = 1;
pub const ENDPOINT_EMPTY: u32 = 1;
pub const ENDPOINT_REFUSED: u32 = 2;
/// R42.3 (task 21.4): the other role's Partition was stopped or restarted
/// since this role's last operation; the Endpoint was re-initialized and
/// the operation not performed. Returned once, by the next operation.
pub const ENDPOINT_PEER: u32 = 3;
/// A Protocol violation reported by the Endpoint_Library (task 21.3; R41.2):
/// a role state dropped outside an end or recursion state, a Job that
/// returned with its RoleSlot empty, or a message with a label the role's
/// state does not expect. Recorded against the executing Task's Partition
/// (`EV_PROTOCOL_VIOLATION`), no response.
pub const SVC_VIOLATION: u8 = 10;
/// Not an SVC number: an SVC entry whose frame's return address does not
/// follow an SVC instruction (`unsafe_module::svc_number`): a stale,
/// late-arriving SVC of a killed Job, ignored by `arch::cm4::svc`.
pub const SVC_STALE: u8 = 0xFF;

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
/// Bit 7: the reset that led here was a watchdog reset (ORQ-31): the
/// system enters the safe state rather than running again.
pub const BOOT_FAIL_WATCHDOG_RESET: u32 = 128;
/// Bit 13: the `LAYOUT` in memory does not match the checksum Generated_Config recorded (R12.3).
pub const BOOT_FAIL_LAYOUT_CHECKSUM: u32 = 1 << 13;
/// Bit 14 is `hw::boot_checks`'s: a PPI channel enabled after the boot clear (DD-08).
/// Bit 15: an EasyDMA register the Kernel mediates lies outside the read-only
/// windows of its Partition (DD-08, R17.3; audit finding 1).
pub const BOOT_FAIL_DMA_WINDOW: u32 = 1 << 15;
/// `KernelArch::boot_info` bits.
pub const BOOT_INFO_DEBUGGER: u32 = 1;
pub const BOOT_INFO_WATCHDOG_RESET: u32 = 2;
/// The watchdog clock (nRF52840 WDT: 32.768 kHz).
pub const WATCHDOG_HZ: u64 = 32_768;
/// Watchdog timeout in housekeeping periods: the time-base handler runs
/// at least once per housekeeping period, so the handler stopping for
/// this many periods resets the Target.
pub const WATCHDOG_HOUSEKEEPING_PERIODS: u64 = 4;

/// The watchdog reload value for a time base of `tick_hz` ticks per
/// second: `WATCHDOG_HOUSEKEEPING_PERIODS` housekeeping periods in
/// watchdog ticks, saturated; the full range for a zero `tick_hz`.
pub fn watchdog_reload(tick_hz: u64) -> (r: u32)
    ensures tick_hz > 0 ==> r as int == (
        if (HOUSEKEEPING_PERIOD as int) * (WATCHDOG_HOUSEKEEPING_PERIODS as int) * (WATCHDOG_HZ as int) / (tick_hz as int) > 0xFFFF_FFFF { 0xFFFF_FFFF as int }
        else { (HOUSEKEEPING_PERIOD as int) * (WATCHDOG_HOUSEKEEPING_PERIODS as int) * (WATCHDOG_HZ as int) / (tick_hz as int) }),
{
    if tick_hz == 0 {
        return 0xFFFF_FFFF;
    }
    let ticks: u64 = HOUSEKEEPING_PERIOD * WATCHDOG_HOUSEKEEPING_PERIODS * WATCHDOG_HZ;
    let r = ticks / tick_hz;
    if r > 0xFFFF_FFFF { 0xFFFF_FFFF } else { r as u32 }
}

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
    /// Eager FP stacking (DD-04) with FPDSCR's flush-to-zero as declared.
    FpuEager(bool),
    KernelExceptionPriorities,
    MpuEnableBackground,
    ClearFaultStatus,
    /// NVIC ICPR: clear the pending state of exception `n` (an intake source
    /// unmasked after its MIT window drops the events of the window, DD-06).
    ClearPending(u16),
    /// EasyDMA (DD-08): write the pointer register and, when non-zero, the count register.
    DmaProgram(u32, u32, u32, u32),
    /// EasyDMA: trigger a stop task register (R17.6).
    DmaStop(u32),
    /// EasyDMA: clear a list-mode register (list mode forbidden, DD-08).
    DmaListOff(u32),
    /// EasyDMA: clear a peripheral's SHORTS register before its stop tasks
    /// (R17.6): a Partition's END_START shortcut restarted a stopped
    /// transfer on the rig (task 19.2, 2026-11-06).
    DmaShortsOff(u32),
    /// EasyDMA (DD-08, task 19.2): the Kernel-mediated write of one
    /// register of an EasyDMA-capable peripheral at (address, value);
    /// `dma_write_core` has refused every pointer, count, and list-mode
    /// register of the peripheral's channels.
    DmaWrite(u32, u32),
    /// PPI: disable every channel (Kernel-owned, DD-08).
    PpiDisableAll,
    /// R14.5: re-initializes the RAM image of a restarted or stopped
    /// Partition (its `.bss`/`.data` range of the linker's table zeroed;
    /// every Partition static is zero-initialized by construction, see
    /// `hw::partition_reinit`). Every Job of the Partition has ended or
    /// been killed when this is applied.
    ReinitPartition(u8),
    /// Start the Kernel-owned watchdog with a reload value (ORQ-31).
    WatchdogStart(u32),
    /// Feed the watchdog (every time-base event).
    WatchdogFeed,
    /// Endpoint message copy (task 21.2; DD-09, R42.2): `len` bytes from
    /// `src` to `dst`, both word-aligned and `len` a multiple of 4; the
    /// cores have checked the Partition's buffer against its RAM interval
    /// and the Kernel's slot against the Endpoint storage.
    Copy(u32, u32, u32),
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
    pub watchdog_running: bool,
    /// Fed since this model was created (one model per entry point).
    pub watchdog_fed: bool,
    /// The first word of every message copied into the Endpoint storage
    /// (A-P2, Verification_Plan 5.15): word address to word. The storage is
    /// Kernel-owned (R16.1), so `copy_in` is its only writer.
    pub store: Map<int, int>,
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
        Action::FpuEager(_) => HwModel { fpu_eager: true, ..h },
        Action::KernelExceptionPriorities => HwModel { kernel_exceptions_level0: true, ..h },
        Action::MpuEnableBackground => HwModel { mpu_enabled: true, ..h },
        Action::ClearFaultStatus => h,
        Action::ClearPending(n) => HwModel { pending: h.pending.remove(n as int), ..h },
        Action::DmaProgram(_, _, _, _) => h,
        Action::DmaStop(_) => h,
        Action::DmaListOff(_) => h,
        Action::DmaShortsOff(_) => h,
        Action::DmaWrite(_, _) => h,
        Action::PpiDisableAll => h,
        Action::ReinitPartition(_) => h,
        Action::Copy(_, _, _) => h,
        Action::WatchdogStart(_) => HwModel { watchdog_running: true, ..h },
        Action::WatchdogFeed => HwModel { watchdog_fed: true, ..h },
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
    /// The copy of a message into the Endpoint storage (`len` bytes from the
    /// Partition's `src` to the slot `dst`), returning the first word now
    /// at `dst`: the model records it as the slot's label (A-P2).
    fn copy_in(&mut self, dst: u32, src: u32, len: u32) -> (first: u32)
        ensures final(self).hw() == (HwModel { store: old(self).hw().store.insert(dst as int, first as int), ..old(self).hw() });
    /// The copy of a message out of the Endpoint storage (`len` bytes from
    /// the slot `src` to the Partition's `dst`), returning the first word at
    /// `src`, which is the recorded label: the storage is written by
    /// `copy_in` alone (R16.1, Kernel-owned).
    fn copy_out(&mut self, dst: u32, src: u32, len: u32) -> (first: u32)
        ensures
            final(self).hw() == old(self).hw(),
            old(self).hw().store.contains_key(src as int) ==> first as int == old(self).hw().store[src as int];
    /// The 64-bit time base (DD-05, HW-12).
    fn now(&self) -> u64;
    /// DWT CYCCNT (HW-13).
    fn cycles(&self) -> u32;
    /// The boot checks of R12.1 as a bit set of failures (0 when all pass).
    fn boot_checks(&self, expect: &BootExpect) -> u32;
    /// Boot information for the log: `BOOT_INFO_DEBUGGER`, `BOOT_INFO_WATCHDOG_RESET`.
    fn boot_info(&self) -> u32;
}

/// The Endpoint table of a `Tables` as a specification: uninterpreted, the
/// generated constants (A-P2, Verification_Plan 5.15).
pub uninterp spec fn spec_endpoint_of<T: ?Sized>(t: &T, i: int) -> EndpointCfg;
/// The Endpoint storage base of a `Tables` as a specification.
pub uninterp spec fn spec_storage_base_of<T: ?Sized>(t: &T) -> int;

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
    fn layout_checksum(&self) -> u32;
    fn timer_vector(&self) -> u16;
    fn priority_bits(&self) -> u8;
    /// What the boot checks compare the hardware against (R12.1).
    fn boot_expect(&self) -> BootExpect;
    /// FPDSCR.FZ for every Job's floating-point context (erratum 839676).
    fn fp_flush_to_zero(&self) -> bool;
    /// Time-base ticks per second (R21.2), for the watchdog reload.
    fn tick_hz(&self) -> u64;
    /// The EasyDMA channels (DD-08).
    fn dma_count(&self) -> usize;
    fn dma(&self, i: usize) -> DmaCfg;
    /// The Endpoints (task 21.2) and their Kernel-owned storage (base, bytes).
    fn endpoint_count(&self) -> usize;
    /// The table as a specification (the generated crate's constants, a
    /// pure function of the index; trusted, as every `Tables` method).
    fn endpoint(&self, i: usize) -> (c: EndpointCfg)
        ensures c == spec_endpoint_of(self, i as int);
    fn endpoint_storage(&self) -> (r: (u32, u32))
        ensures r.0 as int == spec_storage_base_of(self);
    /// The Release_Signals (R10.5).
    fn signal_count(&self) -> usize;
    fn signal(&self, i: usize) -> SignalCfg;
}

// ------------------------------------------------ the Cortex-M4F hardware

/// The Cortex-M4F registers behind `KernelArch`. The ghost `model` is the
/// state the Kernel has written since this value was created (one value
/// per entry point); the five trusted functions below are the whole
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
            Action::LoadView(_, view) => {
                // Δ_view (R13.6, task 19.3): the view load's own cycles, inside
                // Δ_dispatch, recorded apart for review (HIL builds).
                #[cfg(feature = "trace-ram")]
                let start = hw::dwt_cyccnt();
                hw::mpu_load(&view);
                #[cfg(feature = "trace-ram")]
                super::unsafe_module::stats_entry(super::unsafe_module::ENTRY_VIEW, hw::dwt_cyccnt().wrapping_sub(start));
            }
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
            Action::FpuEager(fz) => hw::fpu_configure_eager(fz),
            Action::KernelExceptionPriorities => hw::set_kernel_exception_priorities(),
            Action::MpuEnableBackground => hw::mpu_enable_background(),
            Action::ClearFaultStatus => hw::clear_fault_status(),
            Action::ClearPending(n) => hw::nvic_clear_pending(n),
            Action::DmaProgram(ptr_reg, ptr, cnt_reg, len) => hw::dma_program(ptr_reg, ptr, cnt_reg, len),
            Action::DmaStop(reg) => {
                // Traced and recorded (HIL builds) so that the rig can see
                // the stop precede the Partition's re-initialization
                // (R17.6, task 7.4).
                super::trace(b"dmastop", reg);
                #[cfg(feature = "trace-ram")]
                super::unsafe_module::action_push(1, reg);
                hw::dma_stop(reg)
            }
            Action::DmaListOff(reg) => hw::dma_list_off(reg),
            Action::DmaShortsOff(reg) => hw::dma_list_off(reg),
            Action::DmaWrite(reg, value) => hw::dma_write(reg, value),
            Action::PpiDisableAll => hw::ppi_disable_all(),
            Action::ReinitPartition(p) => {
                super::trace(b"reinit", p as u32);
                #[cfg(feature = "trace-ram")]
                super::unsafe_module::action_push(2, p as u32);
                hw::partition_reinit(p)
            }
            Action::WatchdogStart(reload) => {
                // The boot-check suite's watchdog case (task 7.2): a 50 ms
                // reload, never fed, so that the reset lands inside the run
                // and the next boot fails the watchdog check.
                #[cfg(feature = "boot-mutation")]
                let reload = if super::unsafe_module::boot_mutation() == super::MUT_WATCHDOG { 1639 } else { reload };
                hw::watchdog_start(reload)
            }
            Action::WatchdogFeed => {
                // The boot-check suite's watchdog case (task 7.2): never fed,
                // so that the reset's next boot fails the watchdog check.
                #[cfg(feature = "boot-mutation")]
                if super::unsafe_module::boot_mutation() == super::MUT_WATCHDOG {
                    return;
                }
                hw::watchdog_feed()
            }
            Action::Copy(dst, src, len) => hw::copy_words(dst, src, len),
            Action::Trace(tag, v) => {
                // A window entry of 5.8a goes to the actions record too (kind
                // 3, the vector): the trace ring holds the last 512 entries
                // only, the record survives the run.
                #[cfg(feature = "trace-ram")]
                if tag == TR_WINDOW {
                    super::unsafe_module::action_push(3, v);
                }
                super::trace(super::trace_tag(tag), v)
            }
        }
    }

    /// Trusted as `apply` (UJ-056): the copy and one read of the word written.
    #[verifier::external_body]
    fn copy_in(&mut self, dst: u32, src: u32, len: u32) -> (first: u32) {
        super::unsafe_module::hw::copy_in_words(dst, src, len)
    }

    /// Trusted as `apply` (UJ-056): the copy and one read of the stored word.
    #[verifier::external_body]
    fn copy_out(&mut self, dst: u32, src: u32, len: u32) -> (first: u32) {
        super::unsafe_module::hw::copy_out_words(dst, src, len)
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
    fn boot_checks(&self, expect: &BootExpect) -> u32 {
        super::unsafe_module::hw::boot_checks(expect)
    }

    /// Trusted: the debug state and the reset reason (ORQ-30, ORQ-31).
    #[verifier::external_body]
    fn boot_info(&self) -> u32 {
        super::unsafe_module::hw::boot_info()
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
/// A dispatch vector taken at another dispatch stub's entry instruction and deferred (5.8a; 2026-11-06).
pub const TR_WINDOW: u8 = 17;
/// The deferral of 5.8a is in every build but the `defer-nofix` witness
/// (HIL only), which shows the open window faulting as the unfixed Kernel did.
#[cfg(not(feature = "defer-nofix"))]
pub const WINDOW_DEFER: bool = true;
#[cfg(feature = "defer-nofix")]
pub const WINDOW_DEFER: bool = false;

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
    &&& h.watchdog_running
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
#[verifier::rlimit(40)]
pub fn apply_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState) -> (safe: bool)
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
                let partition = k.job_info(t as u8).partition;
                if k.source_masked(t as u8) || k.partition_stopped(partition) {
                    // DD-06: masked for the rest of the MIT window; R14.5: a
                    // stopped Partition's sources stay disabled.
                    hw.apply(Action::Disable(v));
                } else {
                    // Events that arrived while masked belong to the window
                    // and are dropped (R19.1); a level still asserted pends
                    // again at once.
                    hw.apply(Action::ClearPending(v));
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
        // R17.6: a stopped or restarted Partition's EasyDMA transfers are
        // stopped before its memory is re-initialized (DD-08); then the
        // image (R14.5: every static of the Partition back to its initial
        // value; task 7.5 found the request acknowledged and nothing done,
        // so that a restarted Partition kept its state).
        if k.take_stop_dma(p as u8) {
            dma_stop_partition(hw, tables, p as u8);
            // R14.10 (tasks 21.2 and 21.4): the Partition's Endpoint roles
            // are re-initialized with it, on a stop as on a restart (the
            // DMA stop marks both): both channels of each of its Endpoints
            // are emptied and the other Partition's role is told (R42.3).
            endpoint_reinit(tables, state, p as u8);
        }
        if k.take_reinit(p as u8) {
            hw.apply(Action::ReinitPartition(p as u8));
        }
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
    // The debug state and the reset reason go to the log first (ORQ-30,
    // ORQ-31); a watchdog reset is then treated as a failed check.
    let info = hw.boot_info();
    k.boot_info(info & BOOT_INFO_DEBUGGER != 0, info & BOOT_INFO_WATCHDOG_RESET != 0);
    let expect = tables.boot_expect();
    // DD-08: PPI is Kernel-owned; every channel is cleared before the checks,
    // which read the enable register back (bit 14).
    hw.apply(Action::PpiDisableAll);
    let mut failed = hw.boot_checks(&expect);
    if k.config_checksum() != tables.config_checksum() {
        failed = failed | BOOT_FAIL_CHECKSUM;
    }
    if k.layout_checksum() != tables.layout_checksum() {
        failed = failed | BOOT_FAIL_LAYOUT_CHECKSUM;
    }
    if info & BOOT_INFO_WATCHDOG_RESET != 0 {
        failed = failed | BOOT_FAIL_WATCHDOG_RESET;
    }
    if !dma_windows_ok(k, tables) {
        failed = failed | BOOT_FAIL_DMA_WINDOW;
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
    dma_boot(k, hw, tables);
    hw.apply(Action::PrigroupAllPreempt);
    // No Job executes with an exception active (DD-01 as revised), so
    // NONBASETHRDENA stays at its reset value 0 and every exception return
    // to Thread mode is a return to the base level.
    hw.apply(Action::Nonbasethrdena(false));
    hw.apply(Action::Disdefwbuf(true));
    hw.apply(Action::FpuEager(tables.fp_flush_to_zero()));
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
    // The watchdog starts before the time base, so no window exists in
    // which the handler could stop unwatched (ORQ-31).
    hw.apply(Action::WatchdogStart(watchdog_reload(tables.tick_hz())));
    hw.apply(Action::TimebaseInit);
    hw.apply(Action::DwtEnable);
    state.booted = true;
    hw.apply(Action::Trace(TR_BOOTED, 1));
    true
}

/// Triggers the stop tasks of every EasyDMA channel of Partition `p`
/// (R17.6), each peripheral's SHORTS register cleared first: with an
/// END_START shortcut set, the END event the stop raises would start the
/// next transfer (seen on the rig under the traffic scenario of task
/// 19.2: one restart in about ninety left a transfer running after the
/// stop task).
fn dma_stop_partition<A: KernelArch, T: Tables>(hw: &mut A, tables: &T, p: u8)
    ensures
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
{
    let ghost h0 = hw.hw();
    let n = tables.dma_count();
    let mut i: usize = 0;
    while i < n
        invariant hw.hw().basepri == h0.basepri, hw.hw().view == h0.view, hw.hw().primask == h0.primask,
        decreases n - i,
    {
        let d = tables.dma(i);
        if d.partition == p {
            if d.shorts != 0 {
                hw.apply(Action::DmaShortsOff(d.shorts));
            }
            if d.stops[0] != 0 {
                hw.apply(Action::DmaStop(d.stops[0]));
            }
            if d.stops[1] != 0 {
                hw.apply(Action::DmaStop(d.stops[1]));
            }
        }
        i += 1;
    }
}

/// DD-08, R17.3 (audit finding 1, 2026-11-13): every pointer and count
/// register of every EasyDMA channel in Generated_Config lies in a window
/// that its Partition's view maps read-only, so that the registers are
/// reachable from a Job only through `dma_request_core`
/// (`Layout::lemma_ro_window_not_writable`: no view grants unprivileged write
/// access to such an address). A boot check (bit 15), since the channel table
/// is Generated_Config's and the windows are the layout's.
fn dma_windows_ok<K: KernelOps, T: Tables>(k: &K, tables: &T) -> (ok: bool)
    requires k.inv(),
{
    let n = tables.dma_count();
    let mut i: usize = 0;
    while i < n
        invariant k.inv(),
        decreases n - i,
    {
        let d = tables.dma(i);
        if !k.in_ro_window(d.partition, d.ptr) {
            return false;
        }
        if d.maxcnt != 0 && !k.in_ro_window(d.partition, d.maxcnt) {
            return false;
        }
        i += 1;
    }
    true
}

/// Clears every EasyDMA list-mode register at boot (DD-08: list mode
/// forbidden). The PPI channels are cleared by `boot_core` before its checks.
fn dma_boot<K: KernelOps, A: KernelArch, T: Tables>(k: &K, hw: &mut A, tables: &T)
    requires k.inv(),
{
    let n = tables.dma_count();
    let mut i: usize = 0;
    while i < n
        decreases n - i,
    {
        let d = tables.dma(i);
        if d.lists[0] != 0 {
            hw.apply(Action::DmaListOff(d.lists[0]));
        }
        if d.lists[1] != 0 {
            hw.apply(Action::DmaListOff(d.lists[1]));
        }
        i += 1;
    }
}

/// The EasyDMA mediation service (DD-08; R17.1, R17.2): channel `i` of
/// the executing Job's own Partition, with a buffer in that Partition's
/// DMA-permitted memory, is programmed; anything else is refused before
/// any register is written, and the refusal is a spatial violation of the
/// Partition (R14.3). Returns whether the request was accepted.
fn dma_request_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, i: usize, ptr: u32, len: u32) -> (ok: bool)
    requires old(k).inv(),
    ensures final(k).inv(),
        ok ==> *final(k) == *old(k),
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
{
    let accepted = if i < tables.dma_count() {
        let d = tables.dma(i);
        match k.top() {
            Some(t) => {
                let info = k.job_info(t);
                info.partition == d.partition && k.dma_permitted(d.partition, ptr, len)
            }
            None => false,
        }
    } else {
        false
    };
    if accepted {
        let d = tables.dma(i);
        hw.apply(Action::DmaProgram(d.ptr, ptr, d.maxcnt, len));
    } else {
        k.fault(EV_DMA_REJECTED, false);
    }
    accepted
}

/// The EasyDMA register-write service (DD-08; R17.3, Δ_dma_write; task
/// 19.2): a Partition reaches the registers of its EasyDMA-capable
/// peripherals, which its MPU view maps read-only, through the Kernel.
/// The write goes to `offset` within the window of channel `i`'s
/// peripheral when `i` is a channel of the executing Job's own Partition,
/// the offset is a word of the 4 KiB window, and the register is none of
/// the pointer, count, or list-mode registers of any channel of that
/// window (those are programmed by `dma_request_core` alone, after the
/// buffer check of R17.1; list mode stays off); anything else is refused
/// before any register is written and is a spatial violation of the
/// Partition (R17.2). Tasks (START, STOP), events, shortcuts, ENABLE,
/// FREQUENCY, CONFIG, and pin selection are what a Partition drives this
/// way. Returns whether the write was performed.
fn dma_write_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, i: usize, offset: u32, value: u32) -> (ok: bool)
    requires old(k).inv(),
    ensures final(k).inv(),
        ok ==> *final(k) == *old(k),
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
{
    let n = tables.dma_count();
    let mut accepted = false;
    let mut reg: u32 = 0;
    if i < n && offset < 0x1000 && offset % 4 == 0 {
        let d = tables.dma(i);
        let owned = match k.top() {
            Some(t) => {
                let info = k.job_info(t);
                info.partition == d.partition
            }
            None => false,
        };
        // The window: the 4 KiB block of the channel's pointer register.
        let window = d.ptr & 0xFFFF_F000;
        reg = window | offset;
        let mut mediated = false;
        let mut j: usize = 0;
        while j < n
            decreases n - j,
        {
            let e = tables.dma(j);
            if e.ptr & 0xFFFF_F000 == window {
                if reg == e.ptr || reg == e.maxcnt || reg == e.lists[0] || reg == e.lists[1] {
                    mediated = true;
                }
            }
            j += 1;
        }
        accepted = owned && d.ptr != 0 && !mediated;
    }
    if accepted {
        hw.apply(Action::DmaWrite(reg, value));
    } else {
        k.fault(EV_DMA_REJECTED, false);
    }
    accepted
}

/// A Release_Signal raise (R10.5, R10.7): signal `i` of the executing Job's
/// own Partition is a Release_Source event of its bound Sporadic_Task,
/// attributed to that Partition; a signal of another Partition, an unbound
/// one, or an index beyond the table is API misuse (the Context exposes a
/// Job only its Partition's signals, PR-06) and a fault of the Partition.
fn raise_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, i: usize) -> (ok: bool)
    requires old(k).inv(),
    ensures final(k).inv(),
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
{
    let accepted = if i < tables.signal_count() {
        let sg = tables.signal(i);
        match k.top() {
            Some(t) => {
                let info = k.job_info(t);
                info.partition == sg.partition && (sg.task as usize) < k.task_count()
            }
            None => false,
        }
    } else {
        false
    };
    if accepted {
        let sg = tables.signal(i);
        let now = hw.now();
        k.intake(sg.task, sg.partition, now);
    } else {
        k.fault(EV_API_MISUSE, false);
    }
    accepted
}

/// Re-initializes the Endpoints of Partition `p`'s roles when `p` is
/// stopped or restarted (R14.10; R42.3, task 21.4): both channels of each
/// such Endpoint are emptied (what `p` had sent is of a state that no
/// longer exists; what it had not read went with its state), and the other
/// role, when it belongs to another Partition, is marked so that its next
/// operation reports the peer-state change and the two roles start over
/// from the Protocol's initial state (the restarted role's zeroed slot is
/// that state).
/// The `ArchState::fifos` index of the channel from role `from` to role
/// `to` of Endpoint `e` (task 21.7).
pub open spec fn spec_channel_index(e: int, from: int, to: int) -> int {
    e * (MAX_ROLES * MAX_ROLES) as int + from * MAX_ROLES as int + to
}

fn channel_index(e: usize, from: usize, to: usize) -> (r: usize)
    requires e < MAX_ENDPOINTS, from < MAX_ROLES, to < MAX_ROLES,
    ensures r as int == spec_channel_index(e as int, from as int, to as int), r < MAX_CHANNELS,
{
    assert(e * (MAX_ROLES * MAX_ROLES) <= (MAX_ENDPOINTS - 1) * (MAX_ROLES * MAX_ROLES)) by (nonlinear_arith)
        requires e < MAX_ENDPOINTS;
    assert(from * MAX_ROLES <= (MAX_ROLES - 1) * MAX_ROLES) by (nonlinear_arith)
        requires from < MAX_ROLES;
    e * (MAX_ROLES * MAX_ROLES) + from * MAX_ROLES + to
}

/// The peer-state flag of role `r` of Endpoint `e`.
fn role_slot(e: usize, r: usize) -> (i: usize)
    requires e < MAX_ENDPOINTS, r < MAX_ROLES,
    ensures i as int == e as int * MAX_ROLES as int + r as int, i < MAX_ROLE_SLOTS,
{
    assert(e * MAX_ROLES <= (MAX_ENDPOINTS - 1) * MAX_ROLES) by (nonlinear_arith)
        requires e < MAX_ENDPOINTS;
    e * MAX_ROLES + r
}

/// Whether Partition `p` holds a role of `cfg`.
fn partition_in(cfg: &EndpointCfg, p: u8) -> (r: bool)
{
    let mut i: usize = 0;
    let mut found = false;
    while i < MAX_ROLES
        invariant i <= MAX_ROLES,
        decreases MAX_ROLES - i,
    {
        if (i as u8) < cfg.roles && cfg.partitions[i] == p {
            found = true;
        }
        i += 1;
    }
    found
}

/// Empties every channel of Endpoint `e`.
fn clear_channels(state: &mut ArchState, e: usize)
    requires e < MAX_ENDPOINTS,
{
    assert(MAX_ROLES * MAX_ROLES == 16) by (compute);
    assert(MAX_CHANNELS == 128) by (compute);
    assert(e * 16 + 16 <= 128) by (nonlinear_arith)
        requires e < 8;
    let base = e * (MAX_ROLES * MAX_ROLES);
    let mut c: usize = 0;
    while c < MAX_ROLES * MAX_ROLES
        invariant c <= 16, base + 16 <= MAX_CHANNELS, MAX_ROLES * MAX_ROLES == 16,
        decreases MAX_ROLES * MAX_ROLES - c,
    {
        state.fifos[base + c].clear();
        // R42.4 (audit finding 2): no delivery of this Endpoint is left to report.
        state.delivered[base + c] = false;
        c += 1;
    }
}

fn endpoint_reinit<T: Tables>(tables: &T, state: &mut ArchState, p: u8) {
    let n = tables.endpoint_count();
    let mut e: usize = 0;
    while e < n && e < MAX_ENDPOINTS
        invariant e <= MAX_ENDPOINTS,
        decreases MAX_ENDPOINTS - e,
    {
        let cfg = tables.endpoint(e);
        if partition_in(&cfg, p) {
            clear_channels(state, e);
            // Every role of another Partition is marked (task 21.7: all
            // roles of a multiparty Endpoint start over together).
            let mut r: usize = 0;
            while r < MAX_ROLES
                invariant r <= MAX_ROLES, e < MAX_ENDPOINTS,
                decreases MAX_ROLES - r,
            {
                if (r as u8) < cfg.roles && cfg.partitions[r] != p {
                    state.peer_changed[role_slot(e, r)] = true;
                }
                r += 1;
            }
        }
        e += 1;
    }
}

/// The channel a send or receive operates on, as the specification of
/// the Protocol model sees it: the well-formed index state, or the empty
/// channel when the state in RAM fails its invariant (no Kernel path
/// produces that; the cores reset it rather than trust it).
pub open spec fn channel_before(f: crate::logic::endpoint::Fifo, cap: int) -> crate::logic::endpoint::Fifo {
    if f.wf(cap) { f } else { crate::logic::endpoint::Fifo::empty_spec() }
}

/// A send's channel step (task 21.6, goal 3 of the R41.3 link): the next
/// free slot of channel `ch` is appended to its view (the model's
/// `succ` for a send: `chans[ch].push(label)`, the label being the first
/// word the Kernel then copies into the slot), or nothing changes when
/// the channel is full (R40.5; the model's sends never meet a full
/// channel, `progress::within_capacity`). No other channel and no
/// peer-state flag changes.
fn channel_push(state: &mut ArchState, ch: usize, cap: u8) -> (slot: Option<u8>)
    requires ch < MAX_CHANNELS, cap >= 1,
    ensures
        final(state).fifos[ch as int].wf(cap as int),
        channel_before(old(state).fifos[ch as int], cap as int).wf(cap as int),
        slot.is_some() ==> channel_before(old(state).fifos[ch as int], cap as int).count < cap
            && final(state).fifos[ch as int].view(cap as int) == channel_before(old(state).fifos[ch as int], cap as int).view(cap as int).push(slot.unwrap() as int)
            && slot.unwrap() as int == (channel_before(old(state).fifos[ch as int], cap as int).head as int + channel_before(old(state).fifos[ch as int], cap as int).count as int) % (cap as int),
        slot.is_none() ==> channel_before(old(state).fifos[ch as int], cap as int).count == cap
            && final(state).fifos[ch as int] == channel_before(old(state).fifos[ch as int], cap as int),
        forall|c: int| 0 <= c < MAX_CHANNELS && c != ch ==> final(state).fifos[c] == old(state).fifos[c],
        final(state).peer_changed == old(state).peer_changed,
{
    if !state.fifos[ch].is_wf(cap) {
        state.fifos[ch].clear();
    }
    state.fifos[ch].push(cap)
}

/// A receive's channel step: the oldest slot of channel `ch` is removed
/// from the front of its view (the model's `succ` for a receive:
/// `chans[ch].drop_first()`), or nothing changes when the channel is
/// empty (R40.4; the model's receive is then not enabled). No other
/// channel and no peer-state flag changes.
fn channel_pop(state: &mut ArchState, ch: usize, cap: u8) -> (slot: Option<u8>)
    requires ch < MAX_CHANNELS, cap >= 1,
    ensures
        final(state).fifos[ch as int].wf(cap as int),
        channel_before(old(state).fifos[ch as int], cap as int).wf(cap as int),
        slot.is_some() ==> channel_before(old(state).fifos[ch as int], cap as int).count > 0
            && final(state).fifos[ch as int].view(cap as int) == channel_before(old(state).fifos[ch as int], cap as int).view(cap as int).drop_first()
            && slot.unwrap() as int == channel_before(old(state).fifos[ch as int], cap as int).view(cap as int)[0],
        slot.is_none() ==> channel_before(old(state).fifos[ch as int], cap as int).count == 0
            && final(state).fifos[ch as int] == channel_before(old(state).fifos[ch as int], cap as int),
        forall|c: int| 0 <= c < MAX_CHANNELS && c != ch ==> final(state).fifos[c] == old(state).fifos[c],
        final(state).peer_changed == old(state).peer_changed,
{
    if !state.fifos[ch].is_wf(cap) {
        state.fifos[ch].clear();
    }
    let ghost before = state.fifos[ch as int];
    let slot = state.fifos[ch].pop(cap);
    proof {
        if slot.is_some() {
            assert(before.view(cap as int)[0] == (before.head as int + 0) % (cap as int));
            assert((before.head as int) % (cap as int) == before.head as int) by (nonlinear_arith)
                requires 0 <= before.head as int, (before.head as int) < cap as int;
        }
    }
    slot
}

/// Re-initializes one Endpoint after a rejected message (task 21.6,
/// 2026-11-08; the A-P7 assumption of the R41.3 proof made a Kernel
/// rule): both channels are emptied and both roles are marked, so that
/// each role's next operation reports the peer-state change and returns
/// the role to the Protocol's initial state (R42.3). The rejected message
/// was consumed without a Protocol step; after this the Endpoint is in the
/// initial configuration, which the proof covers, whatever the sending
/// Partition's fault response.
fn endpoint_reinit_one(state: &mut ArchState, e: usize)
    requires e < MAX_ENDPOINTS,
    ensures
        // The Endpoint's channels (indices `e * 16 .. (e + 1) * 16`) are
        // empty, its roles' flags set, everything else unchanged.
        forall|i: int| e as int * (MAX_ROLES * MAX_ROLES) as int <= i < (e as int + 1) * (MAX_ROLES * MAX_ROLES) as int ==> #[trigger] final(state).fifos[i] == crate::logic::endpoint::Fifo::empty_spec(),
        forall|i: int| e as int * MAX_ROLES as int <= i < (e as int + 1) * MAX_ROLES as int ==> #[trigger] final(state).peer_changed[i],
        forall|i: int| 0 <= i < MAX_CHANNELS && !(e as int * (MAX_ROLES * MAX_ROLES) as int <= i < (e as int + 1) * (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] final(state).fifos[i] == old(state).fifos[i],
        forall|i: int| 0 <= i < MAX_ROLE_SLOTS && !(e as int * MAX_ROLES as int <= i < (e as int + 1) * MAX_ROLES as int) ==> #[trigger] final(state).peer_changed[i] == old(state).peer_changed[i],
        // R42.4 (audit finding 2): the Endpoint's delivery latches are clear, the others unchanged.
        forall|i: int| e as int * (MAX_ROLES * MAX_ROLES) as int <= i < (e as int + 1) * (MAX_ROLES * MAX_ROLES) as int ==> !(#[trigger] final(state).delivered[i]),
        forall|i: int| 0 <= i < MAX_CHANNELS && !(e as int * (MAX_ROLES * MAX_ROLES) as int <= i < (e as int + 1) * (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] final(state).delivered[i] == old(state).delivered[i],
{
    let ghost before = *state;
    assert(e * (MAX_ROLES * MAX_ROLES) <= (MAX_ENDPOINTS - 1) * (MAX_ROLES * MAX_ROLES)) by (nonlinear_arith)
        requires e < MAX_ENDPOINTS;
    assert(e * MAX_ROLES <= (MAX_ENDPOINTS - 1) * MAX_ROLES) by (nonlinear_arith)
        requires e < MAX_ENDPOINTS;
    let base = e * (MAX_ROLES * MAX_ROLES);
    let mut c: usize = 0;
    while c < MAX_ROLES * MAX_ROLES
        invariant
            c <= MAX_ROLES * MAX_ROLES, e < MAX_ENDPOINTS, base == e * (MAX_ROLES * MAX_ROLES), base <= (MAX_ENDPOINTS - 1) * (MAX_ROLES * MAX_ROLES),
            forall|i: int| base as int <= i < base as int + c ==> #[trigger] state.fifos[i] == crate::logic::endpoint::Fifo::empty_spec(),
            forall|i: int| 0 <= i < MAX_CHANNELS && !(base as int <= i < base as int + c) ==> #[trigger] state.fifos[i] == before.fifos[i],
            forall|i: int| base as int <= i < base as int + c ==> !(#[trigger] state.delivered[i]),
            forall|i: int| 0 <= i < MAX_CHANNELS && !(base as int <= i < base as int + c) ==> #[trigger] state.delivered[i] == before.delivered[i],
            state.peer_changed == before.peer_changed,
        decreases MAX_ROLES * MAX_ROLES - c,
    {
        state.fifos[base + c].clear();
        state.delivered[base + c] = false;
        c += 1;
    }
    let rbase = e * MAX_ROLES;
    let mut r: usize = 0;
    while r < MAX_ROLES
        invariant
            r <= MAX_ROLES, e < MAX_ENDPOINTS, base == e * (MAX_ROLES * MAX_ROLES), rbase == e * MAX_ROLES, rbase <= (MAX_ENDPOINTS - 1) * MAX_ROLES, base <= (MAX_ENDPOINTS - 1) * (MAX_ROLES * MAX_ROLES),
            forall|i: int| base as int <= i < base as int + (MAX_ROLES * MAX_ROLES) as int ==> #[trigger] state.fifos[i] == crate::logic::endpoint::Fifo::empty_spec(),
            forall|i: int| 0 <= i < MAX_CHANNELS && !(base as int <= i < base as int + (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] state.fifos[i] == before.fifos[i],
            forall|i: int| base as int <= i < base as int + (MAX_ROLES * MAX_ROLES) as int ==> !(#[trigger] state.delivered[i]),
            forall|i: int| 0 <= i < MAX_CHANNELS && !(base as int <= i < base as int + (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] state.delivered[i] == before.delivered[i],
            forall|i: int| rbase as int <= i < rbase as int + r ==> #[trigger] state.peer_changed[i],
            forall|i: int| 0 <= i < MAX_ROLE_SLOTS && !(rbase as int <= i < rbase as int + r) ==> #[trigger] state.peer_changed[i] == before.peer_changed[i],
        decreases MAX_ROLES - r,
    {
        state.peer_changed[rbase + r] = true;
        r += 1;
    }
}

/// The Endpoint_Library's reports (`SVC_VIOLATION`; tasks 21.3 and 21.4).
/// `arg` = 2 | e << 8 | role << 16: a Protocol violation of role `role` of
/// Endpoint `e` (R41.2), recorded with no response and the Endpoint
/// re-initialized. `arg` = 1 | e << 8 | role << 16: a message that role
/// `role` of Endpoint `e`, bound to the executing Task, received and its
/// validation function rejected (R42.4): the fault of the sending
/// Partition, recorded against the sending Task, whose Partition receives
/// its fault response, and the Endpoint re-initialized (`endpoint_reinit_one`,
/// task 21.6). A report naming a role the executing Task is not bound to,
/// or an unknown form, is API misuse of the caller.
fn violation_core<K: KernelOps, T: Tables>(k: &mut K, tables: &T, state: &mut ArchState, arg: u32)
    requires old(k).inv(),
    ensures final(k).inv(),
{
    let t = match k.top() {
        Some(t) => t,
        None => return,
    };
    // Task 21.7, the review's finding 14(f): `arg` = 2 | e << 8 | role << 16
    // reports a Protocol violation of role `role` of Endpoint `e` (a role
    // value dropped mid-Protocol, an unexpected label, a slot left empty;
    // R41.2): recorded against the executing Task with no response, and the
    // Endpoint re-initialized, so that every role starts the Protocol over
    // (A-P6) instead of one role staying blocked for ever.
    if arg & 0xFF == 2 {
        let e = ((arg >> 8) & 0xFF) as usize;
        let role = ((arg >> 16) & 0xF) as usize;
        if (arg >> 20) != 0 || e >= tables.endpoint_count() || e >= MAX_ENDPOINTS || role >= MAX_ROLES {
            k.fault(EV_API_MISUSE, false);
            return;
        }
        let cfg = tables.endpoint(e);
        if (role as u8) >= cfg.roles || cfg.tasks[role] != t {
            k.fault(EV_API_MISUSE, false);
            return;
        }
        k.record_event(EV_PROTOCOL_VIOLATION, t);
        endpoint_reinit_one(state, e);
        return;
    }
    // Task 21.7: `arg` = 1 | e << 8 | role << 16 | from << 20, `from` the
    // role that sent the rejected message.
    let e = ((arg >> 8) & 0xFF) as usize;
    let role = ((arg >> 16) & 0xF) as usize;
    let from = ((arg >> 20) & 0xF) as usize;
    if arg & 0xFF != 1 || (arg >> 24) != 0 || e >= tables.endpoint_count() || e >= MAX_ENDPOINTS || role >= MAX_ROLES || from >= MAX_ROLES {
        k.fault(EV_API_MISUSE, false);
        return;
    }
    let cfg = tables.endpoint(e);
    if (role as u8) >= cfg.roles || (from as u8) >= cfg.roles || from == role || cfg.tasks[role] != t {
        k.fault(EV_API_MISUSE, false);
        return;
    }
    rejection_core(k, state, &cfg, e, role, from);
}

/// R42.4 (audit finding 2, 2026-11-13): whether the Kernel accepts a
/// rejection report of role `role` of Endpoint `e` against role `from`: the
/// role declares a validation function for what it receives from `from`
/// over an upward Criticality_Level edge, and a message of that channel was
/// delivered to it since the Endpoint's last re-initialization.
pub open spec fn spec_rejection_permitted(cfg: EndpointCfg, state: ArchState, e: int, role: int, from: int) -> bool {
    cfg.validates[role][from] && state.delivered[spec_channel_index(e, from, role)]
}

/// The rejected-message report of a role bound to the executing Task
/// (R42.4). Permitted (`spec_rejection_permitted`): the fault of the sending
/// Partition, recorded against the sending Task, and the Endpoint
/// re-initialized, its delivery latches with it, so that one delivery
/// permits one report. Not permitted (no validated upward edge from `from`,
/// or nothing delivered): API misuse of the caller, and no Endpoint or
/// latch state changes. Until 2026-11-13 the report was accepted from any
/// role bound to the caller, so that a Partition of lower Criticality_Level
/// could impose a higher one's fault response with one call.
fn rejection_core<K: KernelOps>(k: &mut K, state: &mut ArchState, cfg: &EndpointCfg, e: usize, role: usize, from: usize)
    requires old(k).inv(), e < MAX_ENDPOINTS, role < MAX_ROLES, from < MAX_ROLES,
    ensures final(k).inv(),
        !spec_rejection_permitted(*cfg, *old(state), e as int, role as int, from as int) ==> *final(state) == *old(state),
        spec_rejection_permitted(*cfg, *old(state), e as int, role as int, from as int) ==>
            forall|i: int| e as int * (MAX_ROLES * MAX_ROLES) as int <= i < (e as int + 1) * (MAX_ROLES * MAX_ROLES) as int ==> !(#[trigger] final(state).delivered[i]),
{
    let ch = channel_index(e, from, role);
    if !(cfg.validates[role][from] && state.delivered[ch]) {
        k.fault(EV_API_MISUSE, false);
        return;
    }
    k.fault_of(EV_MESSAGE_REJECTED, cfg.tasks[from]);
    endpoint_reinit_one(state, e);
}

/// The checks shared by both Endpoint services (task 21.2): the Endpoint
/// exists, the role is one of its two, the executing Job is the role's
/// Task, the length is the Endpoint's payload, and the message lies
/// word-aligned in the Partition's own RAM (R42.2; the Kernel copies under
/// its own view and must never reach beyond the caller's memory). Returns
/// the Endpoint index, the role, and its configuration.
fn endpoint_args<K: KernelOps, T: Tables>(k: &K, tables: &T, arg: u32, ptr: u32, len: u32) -> (r: Option<(usize, usize, usize, EndpointCfg)>)
    requires k.inv(),
    ensures r.is_some() ==> r.unwrap().0 < MAX_ENDPOINTS && r.unwrap().1 < MAX_ROLES && r.unwrap().2 < MAX_ROLES && r.unwrap().1 != r.unwrap().2
        && (r.unwrap().1 as int) < r.unwrap().3.roles as int && (r.unwrap().2 as int) < r.unwrap().3.roles as int && r.unwrap().3.roles as int <= MAX_ROLES as int
        && r.unwrap().3.capacity >= 1 && r.unwrap().3.payload >= 4 && r.unwrap().3.payload % 4 == 0
        && r.unwrap().0 as int == (arg & 0xFF) as int && r.unwrap().1 as int == ((arg >> 8) & 0xF) as int && r.unwrap().2 as int == ((arg >> 12) & 0xF) as int
        && r.unwrap().3 == spec_endpoint_of(tables, (arg & 0xFF) as int),
{
    // Task 21.7: `arg` = e | role << 8 | peer << 12 (the role performing
    // the operation, and the role it sends to or receives from).
    let e = (arg & 0xFF) as usize;
    let role = ((arg >> 8) & 0xF) as usize;
    let peer = ((arg >> 12) & 0xF) as usize;
    if (arg >> 16) != 0 || e >= tables.endpoint_count() || e >= MAX_ENDPOINTS || role >= MAX_ROLES || peer >= MAX_ROLES || role == peer {
        return None;
    }
    let cfg = tables.endpoint(e);
    if (cfg.roles as usize) > MAX_ROLES || (role as u8) >= cfg.roles || (peer as u8) >= cfg.roles {
        return None;
    }
    let ok = match k.top() {
        Some(t) => {
            t == cfg.tasks[role]
                && cfg.capacity >= 1
                && cfg.payload >= 4
                && cfg.payload % 4 == 0
                && len == cfg.payload as u32
                && ptr % 4 == 0
                && k.message_permitted(cfg.partitions[role], ptr, len)
        }
        None => false,
    };
    if ok { Some((e, role, peer, cfg)) } else { None }
}

/// The Kernel storage address of slot `slot` of channel `d` of `cfg`, when
/// the slot's bytes lie inside the Endpoint storage; `None` otherwise (a
/// Generated_Config the Generator never emits: the backstop is a refusal).
/// The address of slot `slot` of channel `d` of an Endpoint (A-P2).
pub open spec fn slot_addr(base: int, cfg: EndpointCfg, d: int, slot: int) -> int {
    base + cfg.storage as int + (d * cfg.capacity as int + slot) * cfg.payload as int
}

/// The labels in transit on a channel: the stored first word of each
/// occupied slot, oldest first (the model's channel of R41.3, A-P2).
pub open spec fn channel_labels(h: HwModel, base: int, cfg: EndpointCfg, d: int, f: crate::logic::endpoint::Fifo) -> Seq<int> {
    f.view(cfg.capacity as int).map(|i: int, slot: int| h.store[slot_addr(base, cfg, d, slot)])
}

/// Distinct slots of a channel have distinct addresses (the payload is at
/// least a word).
proof fn lemma_slot_addr_distinct(base: int, cfg: EndpointCfg, d: int, s1: int, s2: int)
    requires cfg.payload >= 1, 0 <= s1, 0 <= s2, s1 != s2,
    ensures slot_addr(base, cfg, d, s1) != slot_addr(base, cfg, d, s2),
{
    assert((d * cfg.capacity as int + s1) * cfg.payload as int != (d * cfg.capacity as int + s2) * cfg.payload as int) by (nonlinear_arith)
        requires cfg.payload >= 1, s1 != s2;
}

/// The next free slot of a channel is none of the occupied ones.
proof fn lemma_next_slot_fresh(f: crate::logic::endpoint::Fifo, cap: int)
    requires f.wf(cap), (f.count as int) < cap,
    ensures forall|i: int| 0 <= i < f.count ==> #[trigger] f.view(cap)[i] != (f.head as int + f.count as int) % cap,
{
    assert forall|i: int| 0 <= i < f.count implies #[trigger] f.view(cap)[i] != (f.head as int + f.count as int) % cap by {
        let h = f.head as int;
        let c = f.count as int;
        assert(f.view(cap)[i] == (h + i) % cap);
        assert((h + i) % cap != (h + c) % cap) by (nonlinear_arith)
            requires 0 <= i < c, c < cap, 0 <= h < cap;
    }
}

fn endpoint_slot<T: Tables>(tables: &T, cfg: &EndpointCfg, d: usize, slot: u8) -> (a: Option<u32>)
    requires d < MAX_ROLES * MAX_ROLES,
    ensures a is Some ==> a.unwrap() as int == slot_addr(spec_storage_base_of(tables), *cfg, d as int, slot as int),
{
    let (base, total) = tables.endpoint_storage();
    let payload = cfg.payload as u64;
    let capacity = cfg.capacity as u64;
    let slot64 = slot as u64;
    let d64 = d as u64;
    // Every product below is of an 8-bit by a 16-bit quantity: under 2^24;
    // the channel count per Endpoint is at most 16 (task 21.7).
    assert(capacity * payload <= 255 * 65535) by (nonlinear_arith)
        requires capacity <= 255, payload <= 65535;
    assert(slot64 * payload <= 255 * 65535) by (nonlinear_arith)
        requires slot64 <= 255, payload <= 65535;
    let per_channel: u64 = capacity * payload;
    assert(d64 * per_channel <= 16 * 255 * 65535) by (nonlinear_arith)
        requires d64 < 16, per_channel <= 255 * 65535;
    let channel_off: u64 = d64 * per_channel;
    let off: u64 = cfg.storage as u64 + channel_off + slot64 * payload;
    proof {
        assert((d64 as int) * ((capacity as int) * (payload as int)) + (slot64 as int) * (payload as int) == ((d64 as int) * (capacity as int) + slot64 as int) * (payload as int)) by (nonlinear_arith);
    }
    if off + payload <= total as u64 && base as u64 + off + payload <= 0xFFFF_FFFF {
        Some((base as u64 + off) as u32)
    } else {
        None
    }
}

/// The send service (task 21.2; R40.4, R40.5, R42.2): takes the next free
/// slot of the role's outgoing channel, copies the message from the
/// Partition into it, and, when the arriving message is the receiving
/// Task's Release_Source, raises the Release attributed to the sending
/// Partition (R40.3; MIT enforcement may discard the event, never the
/// message, R10.8). A full channel returns `ENDPOINT_FULL` with the event
/// recorded against the sender (R40.5); misuse is a fault of the Partition.
/// The whole operation runs inside the SVC at the Kernel level with PRIMASK
/// held, so no Job end can interrupt it (R40.6).
/// An accepted send's effect on its channel (A-P2): the decoded Endpoint,
/// role, and peer of `arg`; the channel's slots gain the next free slot
/// and its label sequence gains the stored first word.
pub open spec fn send_labels_ok<T: Tables>(tables: &T, s0: ArchState, s1: ArchState, h0: HwModel, h1: HwModel, arg: u32) -> bool {
    let e = (arg & 0xFF) as int;
    let role = ((arg >> 8) & 0xF) as int;
    let peer = ((arg >> 12) & 0xF) as int;
    let cfg = spec_endpoint_of(tables, e);
    let base = spec_storage_base_of(tables);
    let ch = spec_channel_index(e, role, peer);
    let d = role * cfg.roles as int + peer;
    let cap = cfg.capacity as int;
    let before = channel_before(s0.fifos[ch], cap);
    &&& (before.count as int) < cap
    &&& exists|slot: int| #![auto] 0 <= slot < cap
        && s1.fifos[ch].view(cap) == before.view(cap).push(slot)
        && channel_labels(h1, base, cfg, d, s1.fifos[ch]) == channel_labels(h0, base, cfg, d, before).push(h1.store[slot_addr(base, cfg, d, slot)])
        && h1.store == h0.store.insert(slot_addr(base, cfg, d, slot), h1.store[slot_addr(base, cfg, d, slot)])
}

/// An accepted receive's effect on its channel (A-P2): the oldest slot
/// and the oldest label leave, and the oldest label is the first word
/// delivered (when the slot was written by a send, which every occupied
/// slot was).
pub open spec fn recv_labels_ok<T: Tables>(tables: &T, s0: ArchState, s1: ArchState, h0: HwModel, h1: HwModel, arg: u32, delivered: int) -> bool {
    let e = (arg & 0xFF) as int;
    let role = ((arg >> 8) & 0xF) as int;
    let from = ((arg >> 12) & 0xF) as int;
    let cfg = spec_endpoint_of(tables, e);
    let base = spec_storage_base_of(tables);
    let ch = spec_channel_index(e, from, role);
    let d = from * cfg.roles as int + role;
    let cap = cfg.capacity as int;
    let before = channel_before(s0.fifos[ch], cap);
    &&& (before.count as int) > 0
    &&& h1 == h0
    &&& s1.fifos[ch].view(cap) == before.view(cap).drop_first()
    &&& channel_labels(h1, base, cfg, d, s1.fifos[ch]) == channel_labels(h0, base, cfg, d, before).drop_first()
    &&& (h0.store.contains_key(slot_addr(base, cfg, d, before.view(cap)[0])) ==> delivered == channel_labels(h0, base, cfg, d, before)[0])
}

/// The label sequence after a push: the old labels, then the stored word.
proof fn lemma_send_labels(base: int, cfg: EndpointCfg, d: int, before: crate::logic::endpoint::Fifo, after: crate::logic::endpoint::Fifo, slot: int, h0: HwModel, h1: HwModel, first: int)
    requires
        cfg.payload >= 1, before.wf(cfg.capacity as int), (before.count as int) < cfg.capacity as int,
        after.view(cfg.capacity as int) == before.view(cfg.capacity as int).push(slot),
        slot == (before.head as int + before.count as int) % cfg.capacity as int,
        h1 == (HwModel { store: h0.store.insert(slot_addr(base, cfg, d, slot), first), ..h0 }),
    ensures
        channel_labels(h1, base, cfg, d, after) == channel_labels(h0, base, cfg, d, before).push(h1.store[slot_addr(base, cfg, d, slot)]),
        h1.store == h0.store.insert(slot_addr(base, cfg, d, slot), h1.store[slot_addr(base, cfg, d, slot)]),
{
    let cap = cfg.capacity as int;
    lemma_next_slot_fresh(before, cap);
    let la = channel_labels(h1, base, cfg, d, after);
    let lb = channel_labels(h0, base, cfg, d, before);
    assert forall|i: int| 0 <= i < before.count implies la[i] == lb[i] by {
        let s = before.view(cap)[i];
        lemma_slot_addr_distinct(base, cfg, d, s, slot);
    }
    assert(la =~= lb.push(h1.store[slot_addr(base, cfg, d, slot)]));
}

/// The label sequence after a pop: the old labels without the first.
proof fn lemma_recv_labels(base: int, cfg: EndpointCfg, d: int, before: crate::logic::endpoint::Fifo, after: crate::logic::endpoint::Fifo, slot: int, h: HwModel, first: int)
    requires
        (before.count as int) > 0,
        after.view(cfg.capacity as int) == before.view(cfg.capacity as int).drop_first(),
        slot == before.view(cfg.capacity as int)[0],
        h.store.contains_key(slot_addr(base, cfg, d, slot)) ==> first == h.store[slot_addr(base, cfg, d, slot)],
    ensures
        channel_labels(h, base, cfg, d, after) == channel_labels(h, base, cfg, d, before).drop_first(),
        h.store.contains_key(slot_addr(base, cfg, d, before.view(cfg.capacity as int)[0])) ==> first == channel_labels(h, base, cfg, d, before)[0],
{
    let cap = cfg.capacity as int;
    assert(channel_labels(h, base, cfg, d, after) =~= channel_labels(h, base, cfg, d, before).drop_first());
}

fn endpoint_send_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState, arg: u32, ptr: u32, len: u32) -> (code: u32)
    requires old(k).inv(),
    ensures final(k).inv(),
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
        // A-P2 (2026-11-09): an accepted send appends the message's first
        // word, its label, to the channel's label sequence.
        code == ENDPOINT_OK ==> send_labels_ok(tables, *old(state), *final(state), old(hw).hw(), final(hw).hw(), arg),
{
    let (e, role, peer, cfg) = match endpoint_args(k, tables, arg, ptr, len) {
        Some(a) => a,
        None => {
            k.fault(EV_API_MISUSE, false);
            return ENDPOINT_REFUSED;
        }
    };
    let rs = role_slot(e, role);
    if state.peer_changed[rs] {
        state.peer_changed[rs] = false;
        return ENDPOINT_PEER;
    }
    // The channel from `role` to `peer` (task 21.7): its storage is the
    // dense channel `role * roles + peer` of the Endpoint.
    let ch = channel_index(e, role, peer);
    let slot = match channel_push(state, ch, cfg.capacity) {
        Some(s) => s,
        None => {
            let t = cfg.tasks[role];
            k.record_event(EV_ENDPOINT_FULL, t);
            return ENDPOINT_FULL;
        }
    };
    assert(role * (cfg.roles as usize) <= (MAX_ROLES - 1) * MAX_ROLES) by (nonlinear_arith)
        requires role < MAX_ROLES, (cfg.roles as usize) <= MAX_ROLES;
    let d = role * (cfg.roles as usize) + peer;
    let ghost fifo_after = state.fifos[ch as int];
    let ghost before = channel_before(old(state).fifos[ch as int], cfg.capacity as int);
    match endpoint_slot(tables, &cfg, d, slot) {
        Some(dst) => {
            let first = hw.copy_in(dst, ptr, len);
            proof {
                let base = spec_storage_base_of(tables);
                let cap = cfg.capacity as int;
                lemma_send_labels(base, cfg, d as int, before, fifo_after, slot as int, old(hw).hw(), hw.hw(), first as int);
                let sl0 = slot as int;
                assert(0 <= sl0 && sl0 < cap) by (nonlinear_arith)
                    requires sl0 == (before.head as int + before.count as int) % cap, cap >= 1;
                assert(spec_channel_index(e as int, role as int, peer as int) == ch as int);
                assert(send_labels_ok(tables, *old(state), *state, old(hw).hw(), hw.hw(), arg)) by {
                    let sl = slot as int;
                    assert(0 <= sl && sl < cap
                        && state.fifos[ch as int].view(cap) == before.view(cap).push(sl)
                        && channel_labels(hw.hw(), base, cfg, d as int, state.fifos[ch as int]) == channel_labels(old(hw).hw(), base, cfg, d as int, before).push(hw.hw().store[slot_addr(base, cfg, d as int, sl)])
                        && hw.hw().store == old(hw).hw().store.insert(slot_addr(base, cfg, d as int, sl), hw.hw().store[slot_addr(base, cfg, d as int, sl)]));
                }
            }
        }
        None => {
            k.fault(EV_API_MISUSE, false);
            return ENDPOINT_REFUSED;
        }
    }
    if cfg.release[peer] {
        let now = hw.now();
        k.intake(cfg.tasks[peer], cfg.partitions[role], now);
    }
    ENDPOINT_OK
}

/// The receive service (task 21.2; R40.4, R42.2): takes the oldest message
/// of the role's incoming channel, the other role's outgoing one, and
/// copies it into the Partition; an empty channel returns `ENDPOINT_EMPTY`.
fn endpoint_recv_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState, arg: u32, ptr: u32, len: u32) -> (r: (u32, Ghost<int>))
    requires old(k).inv(),
    ensures final(k).inv(),
        final(hw).hw().basepri == old(hw).hw().basepri, final(hw).hw().view == old(hw).hw().view,
        final(hw).hw().primask == old(hw).hw().primask,
        // A-P2 (2026-11-09): an accepted receive takes the oldest label of
        // the channel's label sequence and delivers that word first.
        r.0 == ENDPOINT_OK ==> recv_labels_ok(tables, *old(state), *final(state), old(hw).hw(), final(hw).hw(), arg, r.1@),
{
    let (e, role, from, cfg) = match endpoint_args(k, tables, arg, ptr, len) {
        Some(a) => a,
        None => {
            k.fault(EV_API_MISUSE, false);
            return (ENDPOINT_REFUSED, Ghost(0));
        }
    };
    let rs = role_slot(e, role);
    if state.peer_changed[rs] {
        state.peer_changed[rs] = false;
        return (ENDPOINT_PEER, Ghost(0));
    }
    // The channel from `from` to `role` (task 21.7).
    let ch = channel_index(e, from, role);
    let slot = match channel_pop(state, ch, cfg.capacity) {
        Some(s) => s,
        None => return (ENDPOINT_EMPTY, Ghost(0)),
    };
    assert(from * (cfg.roles as usize) <= (MAX_ROLES - 1) * MAX_ROLES) by (nonlinear_arith)
        requires from < MAX_ROLES, (cfg.roles as usize) <= MAX_ROLES;
    let d = from * (cfg.roles as usize) + role;
    let ghost fifo_after = state.fifos[ch as int];
    let ghost before = channel_before(old(state).fifos[ch as int], cfg.capacity as int);
    match endpoint_slot(tables, &cfg, d, slot) {
        Some(src) => {
            let first = hw.copy_out(ptr, src, len);
            // R42.4 (audit finding 2): a delivery over a validated upward
            // edge permits one rejection report of this role against `from`.
            if cfg.validates[role][from] {
                state.delivered[ch] = true;
            }
            proof {
                let base = spec_storage_base_of(tables);
                lemma_recv_labels(base, cfg, d as int, before, fifo_after, slot as int, hw.hw(), first as int);
                assert(spec_channel_index(e as int, from as int, role as int) == ch as int);
                assert(recv_labels_ok(tables, *old(state), *state, old(hw).hw(), hw.hw(), arg, first as int));
            }
            (ENDPOINT_OK, Ghost(first as int))
        }
        None => {
            k.fault(EV_API_MISUSE, false);
            (ENDPOINT_REFUSED, Ghost(0))
        }
    }
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
    let safe = apply_core(k, hw, tables, state);
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
    // The vectors deferred from a dispatch stub's entry (5.8a) are
    // re-enabled, pending, by the entry that was preempted there, which is
    // the one taken from Thread mode (EXC_RETURN bit 3 set); PRIMASK holds
    // them until its return, after which the NVIC takes them from the base
    // level in priority order.
    if WINDOW_DEFER && exc_return & 0x8 != 0 {
        let ghost h0 = hw.hw();
        let mut i: usize = 0;
        while i < 8
            invariant
                k.inv(), i <= 8,
                hw.hw().basepri == h0.basepri, hw.hw().view == h0.view, hw.hw().primask == h0.primask,
            decreases 8 - i,
        {
            let v = state.deferred[i];
            if v != 0 {
                hw.apply(Action::Enable(v));
                hw.apply(Action::Pend(v));
                state.deferred[i] = 0;
            }
            i += 1;
        }
    }
    let r: (DispatchPlan, bool) = match tables.binding(n) {
        Binding::Dispatch(t) => {
            // The unwind a Kernel-level entry owed while this stub sat at
            // its entry instruction (below): this stub has pushed its record
            // over the ended Job's, so it pops its own and unwinds that one
            // (`hop`), and re-pends its vector to be taken again from the
            // resumed context, where it dispatches.
            if WINDOW_DEFER && exc_return & 0x8 != 0 && state.pending_unwind != 0 {
                let h = state.pending_unwind;
                state.pending_unwind = 0;
                hw.apply(Action::Pend(n));
                return (DispatchPlan { psp: 0, control, exc_return, hop: h }, false);
            }
            // A dispatch vector taken while another handler is active: the
            // preempted context is a handler (EXC_RETURN bit 3 clear), which
            // can only be a dispatch stub of a lower level at its entry
            // instruction, before its `cpsid i` (every other handler runs at
            // the Kernel level, above every dispatch vector, and the return
            // window is closed by FAULTMASK). A dispatch from here would
            // return to Thread mode with that exception still active, which
            // DD-01's invariant forbids and the processor refuses with
            // NONBASETHRDENA clear (UFSR.INVPC: the finding of 2026-11-06,
            // Verification_Plan 5.8a). The vector is deferred: disabled in
            // the NVIC and recorded, so that the preempted stub runs its own
            // entry under PRIMASK, re-enables and re-pends it (above), and
            // returns, after which the NVIC takes it from Thread mode. A
            // re-pend without the disable is a livelock (the witness run of
            // 2026-11-07: the vector is taken again before the stub's next
            // instruction). The backstop, with every slot taken, is a Kernel
            // fault into the safe state (R16.5).
            if WINDOW_DEFER && exc_return & 0x8 == 0 {
                hw.apply(Action::Trace(TR_WINDOW, n as u32));
                hw.apply(Action::Disable(n));
                let mut slot: usize = 8;
                let mut i: usize = 0;
                while i < 8
                    invariant i <= 8, slot <= 8,
                    decreases 8 - i,
                {
                    if slot == 8 && (state.deferred[i] == 0 || state.deferred[i] == n) {
                        slot = i;
                    }
                    i += 1;
                }
                if slot < 8 {
                    state.deferred[slot] = n;
                    return (no_change, false);
                }
                k.fault(EV_HARDFAULT, true);
                let safe = apply_core(k, hw, tables, state);
                return (no_change, safe);
            }
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
            // The Job whose context this dispatch preempts (lead 2.1).
            let previous_task = match k.top() {
                Some(u) => u,
                None => 0xFF,
            };
            k.dispatch(t, preempted);
            let info = k.job_info(t);
            let level = info.priority as usize;
            if (info.partition as usize) >= MAX_FLOORS {
                // More Partitions than `ArchState` keeps floors for: a
                // configuration the Generator rejects; the backstop is a
                // Kernel fault into the safe state.
                k.fault(EV_HARDFAULT, true);
                let safe = apply_core(k, hw, tables, state);
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
                    let safe = apply_core(k, hw, tables, state);
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
                previous_task,
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
            // The frame is 8-byte aligned (`plan_frame`); the Task index
            // rides in its low bits for the trace.
            hw.apply(Action::Trace(TR_DISPATCH, frame | (t as u32 & 7)));
            (DispatchPlan { psp: frame, control: CONTROL_JOB, exc_return: EXC_RETURN_THREAD_PSP, hop: 0 }, false)
        }
        Binding::Intake { task, partition } => {
            let now = hw.now();
            hw.apply(Action::Disable(n));
            let top_before = k.top();
            k.intake(task, partition, now);
            let safe = apply_core(k, hw, tables, state);
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
            let safe = apply_core(k, hw, tables, state);
            let hop = end_plan_core(k, state, top_before);
            (DispatchPlan { psp: 0, control, exc_return, hop }, safe)
        }
    };
    // A Kernel-level entry that ended a Job while a dispatch stub was at its
    // entry instruction (EXC_RETURN bit 3 clear: that stub is active and
    // has pushed nothing yet) cannot unwind from here: the MSP top is this
    // entry's own frame, not the ended Job's level record, which lies
    // below the preempted stub's pushes to come. The unwind is owed to that
    // stub (above), which performs it after its own pushes.
    if WINDOW_DEFER && r.0.psp == 0 && r.0.hop != 0 && exc_return & 0x8 == 0 && !r.1 {
        hw.apply(Action::Trace(TR_WINDOW, 0x10000 | r.0.hop));
        // Audit finding 6 (2026-11-13): a second Kernel-level entry in the
        // same window (tail-chained before the stub's `cpsid i`) that ends
        // the Job the first one resumed owes a deeper level's unwind. The
        // first owed level is the topmost record and stays the one the stub
        // unwinds; the deeper one follows from it by `epilogue_core`'s chain
        // (`LevelRecord::previous_task`: the Job that level preempted is no
        // longer the executing Job). Until this date the second entry
        // overwrote the slot and the stub unwound the deeper level over the
        // topmost record.
        if state.pending_unwind == 0 {
            state.pending_unwind = r.0.hop;
        }
        return (DispatchPlan { psp: 0, control, exc_return, hop: 0 }, false);
    }
    r
}

/// The Kernel-level timer handler body. Returns the unwind plan when the
/// executing Job was ended by a response, and the safe-state flag.
pub fn timer_core<K: KernelOps, A: KernelArch, T: Tables>(k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState) -> (r: (ReturnPlan, bool))
    requires old(k).inv(),
    ensures
        final(k).inv(),
        !r.1 ==> basepri_matches(final(k), final(hw)),
        r.1 ==> final(hw).hw().basepri == 0,
        // Every time-base event feeds the watchdog (ORQ-31).
        final(hw).hw().watchdog_fed,
{
    hw.apply(Action::TimebaseAck);
    let now = hw.now();
    hw.apply(Action::Trace(TR_TICK, now as u32));
    let running = charge_point(hw, state);
    let top_before = k.top();
    k.timer_event(now, running);
    let safe = apply_core(k, hw, tables, state);
    let hop = end_plan_core(k, state, top_before);
    // Fed last: a handler that did not complete does not feed (ORQ-31).
    hw.apply(Action::WatchdogFeed);
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
        let safe = apply_core(k, hw, tables, state);
        (0, safe)
    } else if number == SVC_NOW {
        let now = hw.now();
        frame[0] = now as u32;
        frame[1] = (now >> 32) as u32;
        (0, false)
    } else if number == SVC_DMA {
        // The executing Job is read before the request, as for the panic
        // SVC below: a refusal applies the Partition's fault response
        // (R17.2), and when that response ends the Job the end plan must
        // unwind its level. Read after the refusal, the plan was 0 and the
        // stub returned into the ended Job, which ran on with the Kernel
        // holding no Job for it (the HIL isolation suite's first refused
        // request, 2026-11-02, task 7.4: the next SVC was served with no
        // executing Job and the Kernel entered the safe state).
        // Bit 31 of r0 selects the register write of DD-08 (channel in the
        // low byte, r1 the register offset, r2 the value; task 19.2).
        let top_before = k.top();
        let ok = if frame[0] & 0x8000_0000 != 0 {
            dma_write_core(k, hw, tables, (frame[0] & 0xFF) as usize, frame[1], frame[2])
        } else {
            dma_request_core(k, hw, tables, frame[0] as usize, frame[1], frame[2])
        };
        frame[0] = if ok { 0 } else { 1 };
        if ok {
            (0, false)
        } else {
            let safe = apply_core(k, hw, tables, state);
            let hop = end_plan_core(k, state, top_before);
            (hop, safe)
        }
    } else if number == SVC_RAISE {
        // As for the EasyDMA request: the executing Job is read before the
        // raise, whose refusal applies the fault response (the same defect
        // by inspection, fixed with it; never reached on the bench, since
        // the demo's raises are all accepted).
        let top_before = k.top();
        let ok = raise_core(k, hw, tables, frame[0] as usize);
        frame[0] = if ok { 0 } else { 1 };
        // The released Task is pended if eligible; a refusal applied the
        // Partition's fault response, which may have ended the Job.
        let safe = apply_core(k, hw, tables, state);
        let hop = end_plan_core(k, state, top_before);
        (hop, safe)
    } else if number == SVC_VIOLATION {
        // Task 21.3 (R41.2): the Endpoint_Library's report of a Protocol
        // violation, recorded against the executing Task; a Job that ends
        // without storing its role state is reported by its wrapper after
        // the body, still inside the Job. Task 21.4 (R42.4): the report of
        // a rejected message, a fault of the sending Partition whose
        // response may stop or restart it (never the executing Job's own
        // Partition unless the sender is its own), hence the plan.
        let top_before = k.top();
        violation_core(k, tables, state, frame[0]);
        frame[0] = 0;
        let safe = apply_core(k, hw, tables, state);
        let hop = end_plan_core(k, state, top_before);
        (hop, safe)
    } else if number == SVC_SEND || number == SVC_RECV {
        // Task 21.2: as for the raise, the executing Job is read before the
        // service; a refusal applies the fault response, and a send may
        // release the receiving Task or, through the MIT policy's response
        // to the sending Partition, end the Job.
        let top_before = k.top();
        let code = if number == SVC_SEND {
            endpoint_send_core(k, hw, tables, state, frame[0], frame[1], frame[2])
        } else {
            endpoint_recv_core(k, hw, tables, state, frame[0], frame[1], frame[2]).0
        };
        frame[0] = code;
        if number == SVC_RECV && code != ENDPOINT_REFUSED {
            (0, false)
        } else {
            let safe = apply_core(k, hw, tables, state);
            let hop = end_plan_core(k, state, top_before);
            (hop, safe)
        }
    } else if number == SVC_JOB_END {
        let elapsed = charge_point(hw, state);
        let top_before = k.top();
        k.job_return(elapsed);
        let safe = apply_core(k, hw, tables, state);
        let hop = end_plan_core(k, state, top_before);
        (hop, safe)
    } else {
        // SVC_PANIC and anything else: a fault of the executing Job's
        // Partition (PR-19, R14.2). The executing Job is read before the
        // response so that the end plan unwinds its level when the
        // response ended it (fi-PR-19, 2026-10-23: read after it, the plan
        // was 0, the stub returned into the ended Job, and the next
        // dispatch took a UsageFault).
        let top_before = k.top();
        k.fault(EV_PANIC, false);
        let safe = apply_core(k, hw, tables, state);
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
// Near Z3's default resource limit since `HwModel` gained the watchdog
// fields (task 5.3), and over 20 since `ArchState` gained the deferred
// vectors (5.8a, 2026-11-07); the limit is raised rather than the proof split.
#[verifier::rlimit(40)]
#[verifier::rlimit(80)]
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
    // The audit's lead 2.1 (2026-11-13): the context this level preempted
    // is resumed only while its Job is still the executing Job. One Kernel
    // entry can end two Jobs outright (the Kani witness
    // `one_timer_event_ends_two_jobs_when_the_resumed_job_crossed_its_budget_at_a_preemption`:
    // a Budget event ends the executing Job, the resumed Job's Budget event
    // is armed with no remainder and served in the same pass); the Job this
    // level preempted is then no longer the top, and its level is unwound
    // as well. Until this date the unwind returned into that Job's context,
    // which ran on with the Kernel holding no Job for it.
    let resumed_ended = rec.previous_task != 0xFF && match k.top() {
        Some(t) => t != rec.previous_task,
        None => true,
    };
    if resumed_ended {
        let l = k.job_info(rec.previous_task).priority as usize;
        let next = state.levels[l].exception_number as u32;
        hw.apply(Action::SetBasepri(k.basepri_now()));
        return ((rec.psp, rec.control, rec.exc_return, next), false);
    }
    // A killed resumed Job ends now and its level is unwound as well.
    match k.top() {
        Some(t) => {
            if k.is_killed(t) {
                k.job_return(0);
                let safe = apply_core(k, hw, tables, state);
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
    k: &mut K, hw: &mut A, tables: &T, state: &mut ArchState, kind: u8, in_kernel: bool,
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
    let safe = apply_core(k, hw, tables, state);
    let hop = end_plan_core(k, state, top_before);
    (hop, safe)
}

} // verus!

