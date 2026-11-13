//! Kernel_Arch planning computations inside `verus!` (task 5.7, first
//! increment; GAP-10): the parts of `arch::cm4` that compute a value from
//! Kernel_Arch state without touching a register, with the properties the
//! Hardware_Model and the requirements ask of them.
//!
//! - [`plan_frame`]: where a new Job's initial exception frame goes. The
//!   frame is 8-byte aligned (HW-06, STKALIGN) and lies inside the
//!   Partition's stack region, at or above its bottom (R16.5: a Job never
//!   starts on a frame that already overflows its stack), and it is the
//!   highest such address below the current floor.
//! - [`compare_target`]: the TIMER compare value for a target instant,
//!   never closer than `MIN_COMPARE_DISTANCE` to the counter (HW-12, the
//!   near-counter rule, ASM-19): exactly `hw_model::compare_for`.
//! - [`find_level`]: the level record that names an exception number,
//!   as the unwind looks it up (the greatest matching level, 0 when none).
//! - [`LevelRecord`] and [`ArchState`]: the per-level records of the
//!   preempted contexts and the Partition stack floors, so that the
//!   functions above and, in later increments, the entry points can be
//!   specified over them.
//!
//! Register writes, exception frames in memory, and the `System` trait
//! stay in `arch::cm4` outside `verus!` until the register-access
//! functions of the Kernel_Unsafe_Module carry trusted specifications
//! (task 5.7, later increments).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use crate::hw_model::{self, MIN_COMPARE_DISTANCE};

verus! {

/// One EasyDMA channel of a Partition's peripheral (DD-08, R17): the
/// absolute addresses of its pointer register and (0 when the peripheral
/// has none) its count register, the peripheral's stop task registers, and
/// its list-mode registers (0 for none). Generated_Config lists one per
/// channel of every EasyDMA-capable peripheral a Partition owns.
#[derive(Clone, Copy)]
pub struct DmaCfg {
    pub partition: u8,
    pub ptr: u32,
    pub maxcnt: u32,
    pub stops: [u32; 2],
    pub lists: [u32; 2],
    /// The peripheral's SHORTS register (0 when it has none), cleared
    /// before the stop tasks (R17.6; task 19.2).
    pub shorts: u32,
}

/// One Release_Signal (R10.5): the Partition that declares it (the raising
/// Tasks' Partition, R10.7) and the index of the Sporadic_Task it releases
/// (0xFF when it is bound to none).
#[derive(Clone, Copy)]
pub struct SignalCfg {
    pub partition: u8,
    pub task: u8,
}

/// What the boot checks of R12.1 compare the hardware against: the values
/// Generated_Config records for the Target (ASM-01, ASM-03, ASM-05) and
/// whether any Task uses the FPU (R11.2).
#[derive(Clone, Copy)]
pub struct BootExpect {
    /// Implemented NVIC priority bits (ASM-03).
    pub priority_bits: u8,
    /// The full CPUID word: implementer, variant, part number, revision (ASM-01).
    pub cpuid: u32,
    /// Whether the high-frequency clock must run from the crystal (ASM-05).
    pub hfclk_xtal: bool,
    /// Whether any Task is FPU-using, so the FPU must be present (R12.1).
    pub fpu_needed: bool,
}

/// Basic exception frame size (HW-06).
pub const FRAME_BYTES: u32 = 32;

/// Instants stay below 2^62 ticks (R9.5, `Config::wf` bounds the operating
/// duration), so adding `MIN_COMPARE_DISTANCE` never overflows.
pub const MAX_INSTANT: u64 = 0x4000_0000_0000_0000;

/// The saved context of the Job that a level's stub preempted.
#[derive(Clone, Copy)]
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
    /// The Job this level's dispatch preempted, whose context the record
    /// holds; 0xFF when it preempted the idle activity. The unwind of this
    /// level resumes that context only while that Job is still the Kernel's
    /// executing Job (the audit's lead 2.1, 2026-11-13: one Kernel entry
    /// can end the Job of this level and the one it preempted).
    pub previous_task: u8,
}

/// One Endpoint of Generated_Config (task 21.2; R38.1, R40.1): two roles,
/// each bound to a Task, and two directed channels of `capacity` messages
/// of `payload` bytes each (`payload` a multiple of 4), in Kernel-owned
/// storage at byte offset `storage` (channel `d`, the messages role `d`
/// sends, at `storage + d * capacity * payload`). `release[r]` says that a
/// message arriving for role `r` is a Release_Source event of its Task
/// (R40.3). An absent entry has `capacity` 0.
#[derive(Clone, Copy)]
pub struct EndpointCfg {
    pub capacity: u8,
    pub payload: u16,
    /// The number of roles, 2 to `MAX_ROLES` (task 21.7: multiparty
    /// Endpoints; one directed channel per ordered pair of distinct roles).
    pub roles: u8,
    pub tasks: [u8; MAX_ROLES],
    pub partitions: [u8; MAX_ROLES],
    pub release: [bool; MAX_ROLES],
    pub storage: u32,
    /// R42.4 (audit finding 2, 2026-11-13): `validates[r][f]` says that role
    /// `r` declares a validation function for a message it receives from
    /// role `f`, a role of another Partition of lower Criticality_Level (an
    /// upward edge). The Kernel accepts a rejection report of role `r`
    /// against role `f` only then, and only after a delivery
    /// (`ArchState::delivered`).
    pub validates: [[bool; MAX_ROLES]; MAX_ROLES],
}

/// The most roles an Endpoint may bind (task 21.7).
pub const MAX_ROLES: usize = 4;
/// The most Endpoints a configuration may declare (the Generator rejects
/// more). Each Endpoint owns `MAX_ROLES * MAX_ROLES` channel slots in
/// `ArchState::fifos` (channel `from * MAX_ROLES + to`; the diagonal and
/// the slots beyond the Endpoint's roles are unused) and `MAX_ROLES`
/// peer-state flags.
pub const MAX_ENDPOINTS: usize = 8;
pub const MAX_CHANNELS: usize = MAX_ENDPOINTS * MAX_ROLES * MAX_ROLES;
pub const MAX_ROLE_SLOTS: usize = MAX_ENDPOINTS * MAX_ROLES;

/// Per-level records (one Job in progress per level) and per-Partition
/// stack floors.
#[derive(Clone, Copy)]
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
    /// The first MemManage fault since boot (R14.3 diagnosis on the HIL
    /// bench): CFSR with bit 31 set as the record mark, and the faulting
    /// address (MMFAR when valid, else the stacked PC); zero until a fault.
    pub first_fault_cfsr: u32,
    /// The dispatch vectors deferred from a dispatch stub's entry
    /// instruction (Verification_Plan 5.8a): each disabled in the NVIC and
    /// re-enabled, pending, by the next entry taken from Thread mode. Zero
    /// is an empty slot.
    pub deferred: [u16; 8],
    /// The unwind a Kernel-level entry owed while a dispatch stub was at
    /// its entry instruction (5.8a): the ended Job's level, unwound by that
    /// stub once it has pushed its own record. Zero when none. A second
    /// entry in the same window keeps the first (topmost) level; the levels
    /// below it follow by the unwind's chain (audit finding 6).
    pub pending_unwind: u32,
    /// The index state of every Endpoint channel (task 21.2; Property 11):
    /// channel `2 * e + d` carries what role `d` of Endpoint `e` sends.
    pub fifos: [crate::logic::endpoint::Fifo; MAX_CHANNELS],
    /// R42.3 (task 21.4): entry `2 * e + r` is set when the other role of
    /// Endpoint `e`, in another Partition, was re-initialized by its
    /// Partition's stop or restart since role `r`'s last operation; the
    /// next send or receive of role `r` returns the peer-state error and
    /// clears it.
    pub peer_changed: [bool; MAX_ROLE_SLOTS],
    /// R42.4 (audit finding 2, 2026-11-13): the delivery latch of a
    /// rejection report. Entry `channel_index(e, from, to)` is set when the
    /// receive service delivers a message of that channel to role `to` on a
    /// validated upward edge (`EndpointCfg::validates`), and cleared when
    /// the Endpoint is re-initialized, which an accepted rejection report
    /// does. A report with the entry clear is API misuse of the caller:
    /// until this date any role could report a rejection against its peer
    /// at any time and impose the peer Partition's fault response.
    pub delivered: [bool; MAX_CHANNELS],
    pub first_fault_addr: u32,
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
                previous_task: 0xFF,
            }; 8],
            floor: [0; 16],
            current_partition: 0xFF,
            last_switch: 0,
            time_hi: 0,
            time_last_lo: 0,
            booted: false,
            deferred: [0; 8],
            pending_unwind: 0,
            fifos: [crate::logic::endpoint::Fifo { head: 0, count: 0 }; MAX_CHANNELS],
            peer_changed: [false; MAX_ROLE_SLOTS],
            delivered: [false; MAX_CHANNELS],
            first_fault_cfsr: 0,
            first_fault_addr: 0,
        }
    }
}

/// The highest 8-byte-aligned address whose frame of `FRAME_BYTES` lies
/// below `floor`, if that address is not below `bottom` (the lowest address
/// of the Partition's stack region). `None` means the frame would overflow
/// the stack: the caller raises a Kernel fault (R16.5 backstop) instead of
/// writing below the region.
pub fn plan_frame(floor: u32, bottom: u32) -> (frame: Option<u32>)
    ensures
        frame.is_some() ==> frame.unwrap() % 8 == 0,
        frame.is_some() ==> bottom <= frame.unwrap(),
        frame.is_some() ==> frame.unwrap() + FRAME_BYTES <= floor,
        frame.is_some() ==> floor < frame.unwrap() + FRAME_BYTES + 8,
        frame.is_none() ==> floor < FRAME_BYTES || ((floor - FRAME_BYTES) / 8) * 8 < bottom,
{
    if floor < FRAME_BYTES {
        return None;
    }
    let top = floor - FRAME_BYTES;
    let f = (top / 8) * 8;
    proof {
        assert(f % 8 == 0) by (nonlinear_arith) requires f == (top / 8) * 8 {}
        assert(f <= top && top < f + 8) by (nonlinear_arith) requires f == (top / 8) * 8 {}
    }
    if f < bottom { None } else { Some(f) }
}

/// The compare value for target `at` while the counter reads `now`: never
/// closer than `MIN_COMPARE_DISTANCE` ticks (HW-12).
pub fn compare_target(now: u64, at: u64) -> (target: u64)
    requires now <= MAX_INSTANT,
    ensures
        target as int == hw_model::compare_for(now as int, at as int),
        target >= now + MIN_COMPARE_DISTANCE,
        target >= at,
{
    if at < now + MIN_COMPARE_DISTANCE { now + MIN_COMPARE_DISTANCE } else { at }
}

/// The greatest level 1..=7 whose record names exception number `n`, or 0
/// when none does (the unwind of `arch::cm4::epilogue`).
pub fn find_level(levels: &[LevelRecord; 8], n: u16) -> (level: usize)
    ensures
        level < 8,
        level != 0 ==> levels[level as int].exception_number == n,
        forall|i: int| level < i < 8 ==> levels[i].exception_number != n,
{
    let mut level: usize = 0;
    let mut i: usize = 1;
    while i < 8
        invariant
            1 <= i <= 8, level < i,
            level != 0 ==> levels[level as int].exception_number == n,
            forall|j: int| level < j < i ==> levels[j].exception_number != n,
        decreases 8 - i,
    {
        if levels[i].exception_number == n {
            level = i;
        }
        i += 1;
    }
    level
}

} // verus!
