//! Kernel_Unsafe_Module (R6.3, PR-20): the only part of `rsk-kernel` in
//! which `unsafe` code, unsafe attributes, and assembly are permitted.
//!
//! Every `unsafe` block, `unsafe fn`, and `unsafe impl` carries a
//! justification identifier `UJ-nnn` registered in
//! `crates/rsk-kernel/unsafe_justifications.toml` (R6.4), linked to a Kani
//! harness in [`kani`], to a Verus lemma, or to a recorded review.
//!
//! Layout:
//!
//! - [`hw`]: the register-access functions with trusted specifications
//!   (R44.5). On the Target they are volatile accesses to the addresses of
//!   the Armv7-M system control space and the Target's peripherals; on the
//!   host and under Kani they act on the simulated register file of
//!   [`sim`], which mirrors the Hardware_Model steps (R45.4; the stub list
//!   is `docs/kani_stubs.toml`).
//! - [`stubs`]: the naked assembly entry points (dispatch stub, SVC entry,
//!   epilogue, timer and fault entries, trampoline), Target only.
//! - [`kani`]: the harnesses linked from the justification identifiers.
//!
//! This module applies `#![allow(unsafe_code)]`; the crate root denies it.
#![allow(unsafe_code)]

pub mod hw;
#[cfg(any(kani, test, not(all(target_arch = "arm", target_os = "none"))))]
pub mod sim;
#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod stubs;
#[cfg(not(all(target_arch = "arm", target_os = "none")))]
pub mod stubs_host;
#[cfg(not(all(target_arch = "arm", target_os = "none")))]
pub use stubs_host as stubs;
#[cfg(kani)]
pub mod kani;

use core::cell::UnsafeCell;

use super::ArchState;

/// The Kernel static of the system crate: interior mutability for a value
/// that only Kernel-level or PRIMASK-held code touches (DD-02).
pub struct KernelCell<K> {
    inner: UnsafeCell<K>,
}

// SAFETY (UJ-003): every access goes through `get`, which the Kernel calls
// only from exception handlers at the Kernel NVIC level or from stubs with
// PRIMASK held; Kernel code is non-reentrant by DD-02, so no two accesses
// overlap. Jobs run unprivileged under the MPU and cannot reach Kernel RAM
// (Property 12), so no other code holds a reference. Review only: the
// argument is about the whole execution model, which no harness can state.
unsafe impl<K> Sync for KernelCell<K> {}

impl<K> KernelCell<K> {
    pub const fn new(value: K) -> KernelCell<K> {
        KernelCell { inner: UnsafeCell::new(value) }
    }

    /// The Kernel value. Called only at the Kernel level or with PRIMASK
    /// held (DD-02).
    pub fn get(&self) -> &mut K {
        // SAFETY (UJ-004): as UJ-003; the returned reference never outlives
        // the handler that took it, and handlers do not nest at the same
        // level (DD-02), so no two live references exist at once.
        unsafe { &mut *self.inner.get() }
    }
}

/// Storage of a Resource (design.md "Application API"): accessed only
/// inside a Critical_Section through `rsk::Shared::lock`, which holds the
/// Resource under the ceiling protocol.
pub struct ResourceCell<T> {
    inner: UnsafeCell<T>,
}

// SAFETY (UJ-050): the cell is accessed only inside a Critical_Section on
// its Resource (`with`, called by `rsk::Shared::lock` after the lock SVC
// succeeded), and the SRP makes Critical_Sections on one Resource mutually
// exclusive (Property 4, R8.3); the closure cannot re-enter (R8.10) or let
// the reference escape (R8.6). `T: Send` because the value moves between
// Jobs of different Priorities (RTIC's rule, RN-06). Review only: the
// argument is the Kernel proof plus the type-system rules.
unsafe impl<T: Send> Sync for ResourceCell<T> {}

impl<T> ResourceCell<T> {
    pub const fn new(value: T) -> ResourceCell<T> {
        ResourceCell { inner: UnsafeCell::new(value) }
    }

    /// Runs `f` on the value. Called only by `rsk::Shared::lock` while the
    /// Resource is held.
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY (UJ-051): as UJ-050; the reference lives only inside `f`.
        let value = unsafe { &mut *self.inner.get() };
        f(value)
    }

    /// A copy of the value for Kernel-level readers after every Job has
    /// ended (the safe-state report).
    pub fn peek(&self) -> T
    where
        T: Copy,
    {
        // SAFETY (UJ-052): called at the Kernel level once no Job is in
        // progress (safe state), so no Critical_Section is open.
        unsafe { *self.inner.get() }
    }

    /// Per-Task state that only one Task touches (PR-25: statically
    /// declared per-Task state): no lock is needed because no other Job
    /// accesses it. The Generator emits these accesses for declared
    /// per-Task state only; the demo uses them for its counters.
    #[inline(always)]
    pub fn with_unprivileged<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY (UJ-053): the cell is declared per-Task state (PR-25),
        // accessed by one Task's Jobs only, which never overlap (one Job
        // per Task in progress, Property 2); the reference lives inside `f`.
        let value = unsafe { &mut *self.inner.get() };
        f(value)
    }

    /// A copy of per-Task state or of a value another Task only ever
    /// reads as a hint (the demo's blocking indicator).
    #[inline(always)]
    pub fn peek_unprivileged(&self) -> T
    where
        T: Copy,
    {
        // SAFETY (UJ-054): a read of a `Copy` value; a torn read is
        // impossible for word-sized values on Armv7-M, and the caller uses
        // the value only as a hint.
        unsafe { *self.inner.get() }
    }
}

/// The Kernel_Arch state (level records, stack floors, switch point).
struct ArchCell(UnsafeCell<ArchState>);

// SAFETY (UJ-005): same argument as UJ-003; the state is touched only by
// Kernel-level handlers and PRIMASK-held stub code.
unsafe impl Sync for ArchCell {}

static ARCH_STATE: ArchCell = ArchCell(UnsafeCell::new(ArchState::new()));

/// The RAM trace ring (feature `trace-ram`): `TRACE_RING[0]` is the count
/// of entries written (the ring holds the last `TRACE_RING_LEN`), followed
/// by triples (tag | value, time-base low word, time-base high word). The
/// HIL_Rig reads it over the probe after a run. It lives in `.rsk.noinit`,
/// Kernel RAM the reset path leaves alone, so it survives the reset the
/// board takes when the probe detaches (the count is then stale on the
/// first power-up, which the modulo tolerates).
#[cfg(feature = "trace-ram")]
pub const TRACE_RING_LEN: usize = 512;
#[cfg(feature = "trace-ram")]
struct TraceCell(UnsafeCell<[u32; 1 + 3 * TRACE_RING_LEN]>);
// SAFETY (UJ-005a): as UJ-005; the ring is written only by Kernel-level
// handlers and PRIMASK-held stub code, and read only over the debug port.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for TraceCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[link_section = ".rsk.noinit"]
static TRACE_RING: TraceCell = TraceCell(UnsafeCell::new([0; 1 + 3 * TRACE_RING_LEN]));

/// The release-lag statistics (feature `trace-ram`; R9.10): for every
/// periodic Release the time-base handler makes effective, the lag from
/// the time base reaching the nominal instant to the Release becoming
/// pending, in CPU cycles: `(now - due) * (CPU_HZ / TICK_HZ)` at handler
/// entry plus the DWT cycles from entry to the point the Kernel logic has
/// returned. `RELEASE_STATS` is a word array in `.rsk.noinit`: a header
/// (magic `0x52534b53`, the Task slots, the nominal mismatches: due
/// instants that are not `O + k*T`), then per Task (8 slots) `count, max,
/// sum_lo, sum_hi, hist[16]` (bucket b counts lags below `2^b` cycles,
/// bucket 15 the rest), `resp_count, resp_max` (Jobs completed and the
/// largest response time, Release due to Job end, in ticks; R18.8), and
/// `intake_count, intake_max, intake_sum_lo, intake_sum_hi` (the intake
/// handler's runs for the Task's interrupt source and their cycles;
/// R19.3), and `exec_count, exec_max` (Jobs completed and the largest
/// execution of one Job in cycles, from the Budget accounting; task 19.1,
/// R34.7); then per (Task, Resource) `cs_count, cs_max` (Critical_Sections
/// closed and the largest execution of one, lock to unlock, from the
/// Budget accounting at the two SVCs, less the counter read cost) and the
/// consumed count at each open Critical_Section's lock; then a ring of
/// the last 256 Releases as `(task, due_lo, due_hi, lag)` after a write
/// index. The header's fourth word is the calibration of R34.3: the
/// smallest cycle count between two consecutive reads of DWT_CYCCNT,
/// measured at instant 0. The HIL_Rig reads it after a run.
#[cfg(feature = "trace-ram")]
pub const STATS_TASKS: usize = 8;
#[cfg(feature = "trace-ram")]
pub const STATS_RES: usize = 16;
#[cfg(feature = "trace-ram")]
pub const STATS_RING: usize = 256;
#[cfg(feature = "trace-ram")]
const STATS_HEADER: usize = 4;
#[cfg(feature = "trace-ram")]
const STATS_PER_TASK: usize = 28;
#[cfg(feature = "trace-ram")]
const STATS_CS: usize = STATS_HEADER + STATS_TASKS * STATS_PER_TASK;
#[cfg(feature = "trace-ram")]
const STATS_CS_OPEN: usize = STATS_CS + STATS_TASKS * STATS_RES * 2;
#[cfg(feature = "trace-ram")]
const STATS_RING_AT: usize = STATS_CS_OPEN + STATS_TASKS * STATS_RES;
#[cfg(feature = "trace-ram")]
const STATS_WORDS: usize = STATS_RING_AT + 1 + 4 * STATS_RING;
#[cfg(feature = "trace-ram")]
struct StatsCell(UnsafeCell<[u32; STATS_WORDS]>);
// SAFETY (UJ-005b): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for StatsCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[link_section = ".rsk.noinit"]
static RELEASE_STATS: StatsCell = StatsCell(UnsafeCell::new([0; STATS_WORDS]));

#[cfg(feature = "trace-ram")]
fn stats() -> &'static mut [u32; STATS_WORDS] {
    // SAFETY (UJ-006b): as UJ-006.
    let s = unsafe { &mut *RELEASE_STATS.0.get() };
    if s[0] != 0x5253_4b53 {
        *s = [0; STATS_WORDS];
        s[0] = 0x5253_4b53;
        s[1] = STATS_TASKS as u32;
    }
    s
}

/// Clears the statistics (at boot, so that a run's record is its own).
#[cfg(feature = "trace-ram")]
pub fn stats_reset() {
    let s = stats();
    *s = [0; STATS_WORDS];
    s[0] = 0x5253_4b53;
    s[1] = STATS_TASKS as u32;
}

/// A periodic Release of Task `t` became effective: due at `due` ticks
/// (nominal `offset + k * period`), observed `lag` cycles after it.
#[cfg(feature = "trace-ram")]
pub fn stats_release(t: u8, due: u64, offset: u64, period: u64, lag: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS {
        return;
    }
    if period == 0 || due < offset || (due - offset) % period != 0 {
        s[2] = s[2].saturating_add(1);
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK;
    s[v] = s[v].saturating_add(1);
    if lag > s[v + 1] {
        s[v + 1] = lag;
    }
    let sum = ((s[v + 3] as u64) << 32 | s[v + 2] as u64).saturating_add(lag as u64);
    s[v + 2] = sum as u32;
    s[v + 3] = (sum >> 32) as u32;
    let bucket = (32 - lag.leading_zeros()).min(15) as usize;
    s[v + 4 + bucket] = s[v + 4 + bucket].saturating_add(1);
    let ring = STATS_RING_AT;
    let n = s[ring] as usize;
    let slot = ring + 1 + 4 * (n % STATS_RING);
    s[slot] = t as u32;
    s[slot + 1] = due as u32;
    s[slot + 2] = (due >> 32) as u32;
    s[slot + 3] = lag;
    s[ring] = s[ring].wrapping_add(1);
}

/// A Job of Task `t` completed `response` ticks after its Release was due.
#[cfg(feature = "trace-ram")]
pub fn stats_response(t: u8, response: u64) {
    let s = stats();
    if t as usize >= STATS_TASKS {
        return;
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 20;
    s[v] = s[v].saturating_add(1);
    let r = if response > u32::MAX as u64 { u32::MAX } else { response as u32 };
    if r > s[v + 1] {
        s[v + 1] = r;
    }
}

/// The intake handler of Task `t`'s interrupt source ran for `cycles`.
#[cfg(feature = "trace-ram")]
pub fn stats_intake(t: u8, cycles: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS {
        return;
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 22;
    s[v] = s[v].saturating_add(1);
    if cycles > s[v + 1] {
        s[v + 1] = cycles;
    }
    let sum = ((s[v + 3] as u64) << 32 | s[v + 2] as u64).saturating_add(cycles as u64);
    s[v + 2] = sum as u32;
    s[v + 3] = (sum >> 32) as u32;
}

/// R34.3: calibrates the cycle-counter read at instant 0: the smallest
/// difference between two consecutive reads over 64 pairs (PR-16), kept
/// in the header; the Critical_Section measurements subtract it.
#[cfg(feature = "trace-ram")]
pub fn stats_calibrate() {
    let mut least = u32::MAX;
    let mut i = 0;
    while i < 64 {
        let a = hw::dwt_cyccnt();
        let b = hw::dwt_cyccnt();
        let d = b.wrapping_sub(a);
        if d < least {
            least = d;
        }
        i += 1;
    }
    let s = stats();
    s[3] = least;
}

/// A Job of Task `t` completed having executed `cycles` (task 19.1: the
/// Task-entry maximum from the Budget accounting, R34.7).
#[cfg(feature = "trace-ram")]
pub fn stats_exec(t: u8, cycles: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS {
        return;
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 26;
    s[v] = s[v].saturating_add(1);
    if cycles > s[v + 1] {
        s[v + 1] = cycles;
    }
}

/// Task `t` locked Resource `r` with `consumed` cycles of execution
/// charged to its Job so far (task 19.1).
#[cfg(feature = "trace-ram")]
pub fn stats_cs_open(t: u8, r: u8, consumed: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS || r as usize >= STATS_RES {
        return;
    }
    s[STATS_CS_OPEN + t as usize * STATS_RES + r as usize] = consumed;
}

/// Task `t` unlocked Resource `r` with `consumed` cycles charged to its
/// Job: the Critical_Section's execution is the difference since the
/// lock, less the counter read cost (R34.3).
#[cfg(feature = "trace-ram")]
pub fn stats_cs_close(t: u8, r: u8, consumed: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS || r as usize >= STATS_RES {
        return;
    }
    let opened = s[STATS_CS_OPEN + t as usize * STATS_RES + r as usize];
    let cycles = consumed.wrapping_sub(opened).saturating_sub(s[3]);
    let v = STATS_CS + (t as usize * STATS_RES + r as usize) * 2;
    s[v] = s[v].saturating_add(1);
    if cycles > s[v + 1] {
        s[v + 1] = cycles;
    }
}

/// Appends one trace entry, unless the trace is frozen (`events_push`).
/// Called only at the Kernel level or with PRIMASK held.
#[cfg(feature = "trace-ram")]
pub fn trace_ram_push(word: u32, now: u64) {
    if trace_frozen() {
        return;
    }
    // SAFETY (UJ-006a): as UJ-006.
    let ring = unsafe { &mut *TRACE_RING.0.get() };
    let n = ring[0] as usize;
    let slot = 1 + 3 * (n % TRACE_RING_LEN);
    ring[slot] = word;
    ring[slot + 1] = now as u32;
    ring[slot + 2] = (now >> 32) as u32;
    ring[0] = ring[0].wrapping_add(1);
}

/// The event mirror (feature `trace-ram`): every Health_Monitor event the
/// Kernel logic pushes, copied by Kernel_Arch after the entry that pushed
/// it (`arch::cm4::mirror_events`) into a ring of the last `EVENTS_LEN`
/// in `.rsk.noinit`, so that a run far longer than the 32-entry log can
/// be judged afterwards (the hour of PAR-05 overwrites the log many times
/// over); and the trace freeze: the first event outside the expected set
/// (`events_expect`: a kind set and a Task, the run's known events, with
/// EV_DEBUGGER, EV_OPERATING_END, and EV_SAFE_STATE always expected)
/// stops the trace ring, which then holds the entries leading to it.
/// Layout: header `magic 0x52534b45, capacity, written, seen_seq, frozen
/// (0, or 1 + the event's index in the mirror), expect_kinds (bit k = kind
/// k), expect_task (0xFF: any), 0`, then `(kind << 24 | task << 16 |
/// partition << 8, instant_lo, instant_hi, detail)` per event.
#[cfg(feature = "trace-ram")]
/// 512 entries: with the trace ring and the release-timing record the
/// no-init records must leave the Kernel stack its room in the 32 KiB of
/// Kernel RAM (a 1,024-entry mirror beside the record of task 19.1 left
/// it too little, 2026-11-02).
pub const EVENTS_LEN: usize = 512;
/// The most events one Kernel entry can add to the mirror: the log's capacity.
#[cfg(feature = "trace-ram")]
pub const EVENTS_PER_ENTRY: usize = 32;
#[cfg(feature = "trace-ram")]
const EVENTS_HEADER: usize = 8;
#[cfg(feature = "trace-ram")]
const EVENTS_WORDS: usize = EVENTS_HEADER + 4 * EVENTS_LEN;
#[cfg(feature = "trace-ram")]
const EVENTS_MAGIC: u32 = 0x5253_4b45;
#[cfg(feature = "trace-ram")]
struct EventsCell(UnsafeCell<[u32; EVENTS_WORDS]>);
// SAFETY (UJ-005c): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for EventsCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[link_section = ".rsk.noinit"]
static EVENT_MIRROR: EventsCell = EventsCell(UnsafeCell::new([0; EVENTS_WORDS]));

#[cfg(feature = "trace-ram")]
fn events() -> &'static mut [u32; EVENTS_WORDS] {
    // SAFETY (UJ-006c): as UJ-006.
    let e = unsafe { &mut *EVENT_MIRROR.0.get() };
    if e[0] != EVENTS_MAGIC {
        *e = [0; EVENTS_WORDS];
        e[0] = EVENTS_MAGIC;
        e[1] = EVENTS_LEN as u32;
        e[6] = 0xFF;
    }
    e
}

/// The expected events of the run: kinds in `kinds` (bit k = kind k) of
/// Task `task` (0xFF: any) do not freeze the trace. Kept across `events_reset`.
#[cfg(feature = "trace-ram")]
pub fn events_expect(kinds: u32, task: u8) {
    let e = events();
    e[5] = kinds;
    e[6] = task as u32;
}

/// Clears the mirror and the freeze (at boot, so that a run's record is
/// its own); the expectation stays.
#[cfg(feature = "trace-ram")]
pub fn events_reset() {
    let e = events();
    let (kinds, task) = (e[5], e[6]);
    *e = [0; EVENTS_WORDS];
    e[0] = EVENTS_MAGIC;
    e[1] = EVENTS_LEN as u32;
    e[5] = kinds;
    e[6] = task;
}

/// The sequence number the mirror has seen up to (the next event to copy).
#[cfg(feature = "trace-ram")]
pub fn events_seen_seq() -> u32 {
    events()[3]
}

/// Whether the trace ring is frozen (an unexpected event was mirrored).
#[cfg(feature = "trace-ram")]
pub fn trace_frozen() -> bool {
    events()[4] != 0
}

/// Mirrors event number `seq` of the log and freezes the trace at the
/// first unexpected one.
#[cfg(feature = "trace-ram")]
pub fn events_push(seq: u32, kind: u8, task: u8, partition: u8, instant: u64, detail: u32) {
    let e = events();
    let n = e[2] as usize;
    let slot = EVENTS_HEADER + 4 * (n % EVENTS_LEN);
    e[slot] = (kind as u32) << 24 | (task as u32) << 16 | (partition as u32) << 8;
    e[slot + 1] = instant as u32;
    e[slot + 2] = (instant >> 32) as u32;
    e[slot + 3] = detail;
    e[2] = e[2].wrapping_add(1);
    e[3] = seq.wrapping_add(1);
    let informational = kind == crate::logic::log::EV_DEBUGGER
        || kind == crate::logic::log::EV_OPERATING_END
        || kind == crate::logic::log::EV_SAFE_STATE;
    let expected = kind < 32 && (e[5] >> kind) & 1 == 1 && (e[6] == 0xFF || e[6] == task as u32);
    if e[4] == 0 && !informational && !expected {
        // The mirror's own index of the event (the log's sequence numbers
        // restart at the reboot on probe detach; the mirror's do not).
        e[4] = (n as u32).wrapping_add(1);
    }
}

/// The Kernel_Arch state. Called only at the Kernel level or with PRIMASK held.
pub fn arch_state() -> &'static mut ArchState {
    // SAFETY (UJ-006): as UJ-004.
    unsafe { &mut *ARCH_STATE.0.get() }
}

/// Extends a 32-bit counter reading to the 64-bit time base (HW-12,
/// DD-05): a reading below the previous one means the counter wrapped.
/// Readings happen at least once per housekeeping period, which is far
/// shorter than a wrap.
pub fn extend(low: u32) -> u64 {
    let s = arch_state();
    if low < s.time_last_lo {
        s.time_hi = s.time_hi.wrapping_add(1);
    }
    s.time_last_lo = low;
    ((s.time_hi as u64) << 32) | low as u64
}

/// The initial exception frame of a Job (HW-06): R0-R3 and R12 zero, LR
/// the job-end trampoline, PC the Job entry, xPSR with the Thumb bit.
pub fn frame_words(entry: u32, trampoline: u32) -> [u32; 8] {
    [0, 0, 0, 0, 0, trampoline | 1, entry & !1, super::XPSR_THUMB]
}

/// Writes the initial frame at `sp` (8-byte aligned, inside the Partition's
/// stack region: `irq_entry` checks `sp >= stack bottom` before calling).
pub fn build_frame(sp: u32, entry: u32, trampoline: u32) {
    let words = frame_words(entry, trampoline);
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-007): `sp` is 8-byte aligned and `sp..sp+32` lies inside
        // the Partition's stack region, which no other context uses at this
        // instant (the stack floor was just lowered under PRIMASK); the
        // region is ordinary RAM mapped read-write to the Kernel.
        let frame = unsafe { &mut *(sp as usize as *mut [u32; 8]) };
        *frame = words;
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        sim::record_frame(sp, words);
    }
}

/// The SVC number of the instruction before the stacked return address
/// (Thumb `SVC #imm8`: the low byte of the halfword at PC - 2), or
/// `SVC_STALE` when that halfword is not an SVC instruction: the entry is
/// then a late-arriving SVC of a Job that the exception which preempted
/// its entry killed (see `arch::cm4::svc`).
pub fn svc_number(stacked_pc: u32) -> u8 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-008): `stacked_pc` is the return address stacked by an
        // exception taken from Thumb code (HW-06), so `stacked_pc - 2` is
        // the address of the last instruction executed, in executable
        // memory (the SVC instruction when the frame is the SVC's own).
        let halfword = unsafe { core::ptr::read_volatile((stacked_pc - 2) as usize as *const u16) };
        if halfword >> 8 != 0xDF {
            return super::entry::SVC_STALE;
        }
        (halfword & 0xFF) as u8
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        sim::svc_number_at(stacked_pc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_layout_matches_hw06() {
        let f = frame_words(0x0000_1000, 0x0000_2000);
        assert_eq!(f[5], 0x0000_2001, "LR carries the Thumb bit");
        assert_eq!(f[6], 0x0000_1000, "PC has bit 0 clear");
        assert_eq!(f[7], 1 << 24, "xPSR Thumb bit");
        assert_eq!(&f[0..5], &[0, 0, 0, 0, 0]);
    }
}
