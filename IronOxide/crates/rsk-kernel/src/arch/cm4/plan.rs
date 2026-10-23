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
}

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
