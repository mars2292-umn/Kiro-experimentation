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
/// The HIL differential tests of the Hardware_Model (task 3.3; HIL builds only).
#[cfg(feature = "hw-dt")]
pub mod dt;

/// The boot-check suite's mutation word (feature `boot-mutation`, HIL
/// builds only; task 7.2, R12.6): a word of RAM outside every region of
/// the memory map (the top of the nRF52840's 256 KiB, above the 128 KiB
/// the layout uses) that the HIL_Rig writes over the probe before a run,
/// read by the trusted table accessors and the boot
/// (`arch::cm4::boot_mutation`) to make one check of R12.1, R12.3, or
/// R12.5 fail. 0 is no mutation. Outside the Kernel's RAM on purpose: the
/// flashing algorithm probe-rs runs before the reset occupies the start
/// of RAM, where a no-init word of the Kernel's was overwritten (seen on
/// the first run of the suite, 2026-11-04); a word at the top of RAM
/// survives the flashing and the reset. Never in a Flight_Build.
#[cfg(feature = "boot-mutation")]
pub const BOOT_MUTATION_ADDR: u32 = 0x2003_FFF0;

/// The mutation the rig requested for this boot.
#[cfg(feature = "boot-mutation")]
pub fn boot_mutation() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none", not(feature = "board-qemu-mps2")))]
    {
        // SAFETY (UJ-048): a volatile word read of RAM outside every region
        // of the layout, written only over the debug port. HIL builds only.
        unsafe { core::ptr::read_volatile(BOOT_MUTATION_ADDR as usize as *const u32) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none", not(feature = "board-qemu-mps2"))))]
    {
        0
    }
}

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
    #[cfg_attr(feature = "coverage", coverage(off))]
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
    #[cfg_attr(feature = "coverage", coverage(off))]
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
/// R34.7), `exec_over, exec_hist[16]`, and `exec_min, exec_sum_lo,
/// exec_sum_hi` (the smallest execution and the sum, task 7.5's cycle
/// test), `interrupted` (the entries that interrupted the Task's Jobs), and
/// `stamp_count, stamp_max, stamp_last` (the lag in ticks from the
/// stimulus edge the Δ_stamp probe latched to the intake handler's own
/// timestamp, counted once per latched edge; Table 13-1); then per (Task, Resource) `cs_count, cs_max` (Critical_Sections
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
const STATS_PER_TASK: usize = 52;
#[cfg(feature = "trace-ram")]
const STATS_CS: usize = STATS_HEADER + STATS_TASKS * STATS_PER_TASK;
#[cfg(feature = "trace-ram")]
const STATS_CS_OPEN: usize = STATS_CS + STATS_TASKS * STATS_RES * 2;
#[cfg(feature = "trace-ram")]
/// The Kernel-entry table (task 19.3, Table 13-1): per entry kind
/// (`ENTRY_*`) `count, max, sum_lo, sum_hi` of the cycles from the Rust
/// half's entry to its return (the stub's own cycles outside are the
/// interrupted Job's, `ENTRY_KINDS` apart: Δ_entry, measured by the cycle
/// test as the preempted Job's excess over the interruptions counted per
/// Task at word 48 of its record).
#[cfg(feature = "trace-ram")]
pub const ENTRY_KINDS: usize = 18;
#[cfg(feature = "trace-ram")]
const STATS_ENTRY: usize = STATS_CS_OPEN + STATS_TASKS * STATS_RES;
#[cfg(feature = "trace-ram")]
const STATS_RING_AT: usize = STATS_ENTRY + 4 * ENTRY_KINDS;
#[cfg(feature = "trace-ram")]
pub const ENTRY_DISPATCH: u8 = 0;
#[cfg(feature = "trace-ram")]
pub const ENTRY_TIMER: u8 = 1;
#[cfg(feature = "trace-ram")]
pub const ENTRY_INTAKE: u8 = 2;
#[cfg(feature = "trace-ram")]
pub const ENTRY_LOCK: u8 = 3;
#[cfg(feature = "trace-ram")]
pub const ENTRY_UNLOCK: u8 = 4;
#[cfg(feature = "trace-ram")]
pub const ENTRY_JOB_END: u8 = 5;
#[cfg(feature = "trace-ram")]
pub const ENTRY_NOW: u8 = 6;
#[cfg(feature = "trace-ram")]
pub const ENTRY_RAISE: u8 = 7;
#[cfg(feature = "trace-ram")]
pub const ENTRY_DMA: u8 = 8;
#[cfg(feature = "trace-ram")]
pub const ENTRY_FAULT: u8 = 9;
#[cfg(feature = "trace-ram")]
pub const ENTRY_EPILOGUE: u8 = 10;
#[cfg(feature = "trace-ram")]
pub const ENTRY_RESTART: u8 = 11;
#[cfg(feature = "trace-ram")]
pub const ENTRY_STOP: u8 = 12;
#[cfg(feature = "trace-ram")]
pub const ENTRY_ENFORCE: u8 = 13;
#[cfg(feature = "trace-ram")]
pub const ENTRY_PANIC: u8 = 14;
/// One MPU view load inside a dispatch (Δ_view, R13.6; task 19.3).
#[cfg(feature = "trace-ram")]
pub const ENTRY_VIEW: u8 = 15;
/// The Endpoint send and receive SVCs (task 21.2; R39.3).
#[cfg(feature = "trace-ram")]
pub const ENTRY_SEND: u8 = 16;
#[cfg(feature = "trace-ram")]
pub const ENTRY_RECV: u8 = 17;
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
/// Δ_stamp (Table 13-1): the intake handler for Task `t`'s source ran
/// with the time base at `now` (low word, ticks) after the probe latched
/// the edge at `captured` (TIMER0 CC[3]); recorded once per latched edge
/// (a second intake without a new edge, under the storm's masking, does
/// not count), and only when the lag is below a second (a stale capture
/// from before a wrap is not a lag).
#[cfg(feature = "trace-ram")]
pub fn stats_stamp(t: u8, captured: u32, now: u32) {
    let s = stats();
    if t as usize >= STATS_TASKS || captured == 0 {
        return;
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 49;
    if s[v + 2] == captured {
        return;
    }
    s[v + 2] = captured;
    let lag = now.wrapping_sub(captured);
    if lag >= 16_000_000 {
        return;
    }
    s[v] = s[v].saturating_add(1);
    if lag > s[v + 1] {
        s[v + 1] = lag;
    }
}

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
/// Task-entry maximum from the Budget accounting, R34.7), `over` its
/// Budget or not: the count, the maximum, the completions above the
/// Budget, and a log2 histogram of the executions (bucket b counts
/// executions below `2^b` cycles, bucket 15 the rest).
#[cfg(feature = "trace-ram")]
pub fn stats_exec(t: u8, cycles: u32, over: bool) {
    let s = stats();
    if t as usize >= STATS_TASKS {
        return;
    }
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 26;
    s[v] = s[v].saturating_add(1);
    if cycles > s[v + 1] {
        s[v + 1] = cycles;
    }
    if over {
        s[v + 2] = s[v + 2].saturating_add(1);
    }
    let bucket = (32 - cycles.leading_zeros()).min(15) as usize;
    s[v + 3 + bucket] = s[v + 3 + bucket].saturating_add(1);
    // The smallest execution and the sum (task 7.5's cycle test, R18.1):
    // the mean and the spread of a Task's executions.
    if s[v] == 1 || cycles < s[v + 19] {
        s[v + 19] = cycles;
    }
    let sum = ((s[v + 21] as u64) << 32 | s[v + 20] as u64).saturating_add(cycles as u64);
    s[v + 20] = sum as u32;
    s[v + 21] = (sum >> 32) as u32;
}

/// Diagnostic event kind of a HIL build: a Job completed above its Budget
/// under an Overrun_Response that ends overrunning Jobs (R18.2). Never in
/// a run's expected kinds, so the trace freezes at it (task 19.1).
#[cfg(feature = "trace-ram")]
pub const EV_DIAG_EXEC_OVER: u8 = 24;

/// Mirrors a diagnostic event of the HIL build itself (not of the
/// Kernel's log): it takes a mirror slot and leaves the mirror's view of
/// the log (`seen_seq`) alone, so the log's next event is still
/// mirrored; the trace freezes at it as at any unexpected kind.
#[cfg(feature = "trace-ram")]
pub fn events_push_diag(kind: u8, task: u8, partition: u8, instant: u64, detail: u32) {
    let e = events();
    if e[4] != 0 {
        // Frozen: the mirror holds the events that led to the freeze.
        return;
    }
    let n = e[2] as usize;
    let slot = EVENTS_HEADER + 4 * (n % EVENTS_LEN);
    e[slot] = (kind as u32) << 24 | (task as u32) << 16 | (partition as u32) << 8;
    e[slot + 1] = instant as u32;
    e[slot + 2] = (instant >> 32) as u32;
    e[slot + 3] = detail;
    e[2] = e[2].wrapping_add(1);
    if e[4] == 0 {
        e[4] = (n as u32).wrapping_add(1);
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

/// Records one Kernel entry of `kind` that took `cycles` (task 19.3).
#[cfg(feature = "trace-ram")]
pub fn stats_entry(kind: u8, cycles: u32) {
    if kind as usize >= ENTRY_KINDS {
        return;
    }
    let s = stats();
    let v = STATS_ENTRY + 4 * kind as usize;
    s[v] = s[v].saturating_add(1);
    if cycles > s[v + 1] {
        s[v + 1] = cycles;
    }
    let sum = ((s[v + 3] as u64) << 32 | s[v + 2] as u64).saturating_add(cycles as u64);
    s[v + 2] = sum as u32;
    s[v + 3] = (sum >> 32) as u32;
}

/// Counts one Kernel entry that interrupted the executing Job of `t`.
#[cfg(feature = "trace-ram")]
pub fn stats_interrupted(t: u8) {
    if t as usize >= STATS_TASKS {
        return;
    }
    let s = stats();
    let v = STATS_HEADER + t as usize * STATS_PER_TASK + 48;
    s[v] = s[v].saturating_add(1);
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

/// The fault record (feature `trace-ram`; task 7.4, R16.8, R49.3): for
/// every MemManage, BusFault, and UsageFault the Kernel takes, the CFSR
/// and the faulting address (MMFAR or BFAR when valid, else the stacked
/// PC of the faulting context), in order, so that the HIL_Rig can match
/// each fault event of the mirror with the access that caused it (the
/// isolation suite's access matrix) and judge its class (precise or
/// imprecise, HW-11). `FAULT_RING`: magic `0x52534b46`, count, then
/// `(cfsr, address)` per fault, the last `FAULTS_LEN` kept.
#[cfg(feature = "trace-ram")]
pub const FAULTS_LEN: usize = 128;
#[cfg(feature = "trace-ram")]
const FAULTS_WORDS: usize = 2 + 2 * FAULTS_LEN;
#[cfg(feature = "trace-ram")]
const FAULTS_MAGIC: u32 = 0x5253_4b46;
#[cfg(feature = "trace-ram")]
struct FaultsCell(UnsafeCell<[u32; FAULTS_WORDS]>);
// SAFETY (UJ-005d): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for FaultsCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[cfg_attr(all(target_arch = "arm", target_os = "none"), link_section = ".rsk.noinit")]
static FAULT_RING: FaultsCell = FaultsCell(UnsafeCell::new([0; FAULTS_WORDS]));

#[cfg(feature = "trace-ram")]
fn faults() -> &'static mut [u32; FAULTS_WORDS] {
    // SAFETY (UJ-006d): as UJ-006.
    let f = unsafe { &mut *FAULT_RING.0.get() };
    if f[0] != FAULTS_MAGIC {
        *f = [0; FAULTS_WORDS];
        f[0] = FAULTS_MAGIC;
    }
    f
}

/// Clears the fault record (at instant 0, so that a run's record is its own).
#[cfg(feature = "trace-ram")]
pub fn faults_reset() {
    let f = faults();
    *f = [0; FAULTS_WORDS];
    f[0] = FAULTS_MAGIC;
}

/// Appends one fault's status and address.
#[cfg(feature = "trace-ram")]
pub fn fault_push(cfsr: u32, address: u32) {
    let f = faults();
    let n = f[1] as usize;
    let s = 2 + 2 * (n % FAULTS_LEN);
    f[s] = cfsr;
    f[s + 1] = address;
    f[1] = f[1].wrapping_add(1);
}

/// The actions record (feature `trace-ram`; task 7.4, R17.6): the EasyDMA
/// stop tasks the Kernel triggers and the Partition re-initializations it
/// performs, in order, so that the HIL_Rig can see every stop of a
/// restarted Partition's channels precede the re-initialization of its
/// memory. `ARCH_ACTIONS`: magic `0x52534b41`, count, then `(kind, value)`
/// per action (kind 1: a stop task, value its register; kind 2: a
/// re-initialization, value the Partition), the last `ACTIONS_LEN` kept.
#[cfg(feature = "trace-ram")]
pub const ACTIONS_LEN: usize = 32;
#[cfg(feature = "trace-ram")]
const ACTIONS_WORDS: usize = 2 + 2 * ACTIONS_LEN;
#[cfg(feature = "trace-ram")]
const ACTIONS_MAGIC: u32 = 0x5253_4b41;
#[cfg(feature = "trace-ram")]
struct ActionsCell(UnsafeCell<[u32; ACTIONS_WORDS]>);
// SAFETY (UJ-005e): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for ActionsCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[cfg_attr(all(target_arch = "arm", target_os = "none"), link_section = ".rsk.noinit")]
static ARCH_ACTIONS: ActionsCell = ActionsCell(UnsafeCell::new([0; ACTIONS_WORDS]));

/// The count of the dispatch vectors taken inside another stub's return
/// window (Verification_Plan 5.8a; actions of kind 3), kept apart from the
/// record because the record holds the last `ACTIONS_LEN` actions only.
/// `ARCH_WINDOWS`: magic, count.
#[cfg(feature = "trace-ram")]
struct WindowsCell(UnsafeCell<[u32; 2]>);
// SAFETY (UJ-005f): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for WindowsCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[cfg_attr(all(target_arch = "arm", target_os = "none"), link_section = ".rsk.noinit")]
static ARCH_WINDOWS: WindowsCell = WindowsCell(UnsafeCell::new([0; 2]));

#[cfg(feature = "trace-ram")]
fn windows() -> &'static mut [u32; 2] {
    // SAFETY (UJ-006f): as UJ-006.
    let d = unsafe { &mut *ARCH_WINDOWS.0.get() };
    if d[0] != ACTIONS_MAGIC {
        *d = [ACTIONS_MAGIC, 0];
    }
    d
}

/// The masked span (Verification_Plan 11.13, 2026-11-07; the basis of
/// L_kernel): the longest stretch during which a Kernel-level interrupt
/// cannot be taken, measured end to end from the first Rust half of a
/// Kernel entry to the last one that returns to Thread mode with nothing
/// pending that will tail-chain: a handler whose return is followed by a
/// pending Kernel-level exception, a dispatch vector the resumed BASEPRI
/// admits, or its own unwind (`rsk_unwind`'s epilogues) keeps the span
/// open. `ARCH_SPAN`: magic, max cycles, spans counted, the span's start,
/// open (1) or closed, the entry kind that closed the longest span; then
/// a histogram of the spans' lengths by their binary order of magnitude
/// (`SPAN_BUCKETS` buckets from 2^6 cycles, the last one open-ended) and,
/// per closing entry kind (`ENTRY_KINDS`), the spans' count and maximum.
#[cfg(feature = "trace-ram")]
pub const SPAN_BUCKETS: usize = 16;
#[cfg(feature = "trace-ram")]
const SPAN_HEADER: usize = 6;
#[cfg(feature = "trace-ram")]
const SPAN_WORDS: usize = SPAN_HEADER + SPAN_BUCKETS + 2 * ENTRY_KINDS;
#[cfg(feature = "trace-ram")]
struct SpanCell(UnsafeCell<[u32; SPAN_WORDS]>);
// SAFETY (UJ-005g): as UJ-005.
#[cfg(feature = "trace-ram")]
unsafe impl Sync for SpanCell {}
#[cfg(feature = "trace-ram")]
#[no_mangle]
#[cfg_attr(all(target_arch = "arm", target_os = "none"), link_section = ".rsk.noinit")]
static ARCH_SPAN: SpanCell = SpanCell(UnsafeCell::new([0; SPAN_WORDS]));

#[cfg(feature = "trace-ram")]
fn span() -> &'static mut [u32; SPAN_WORDS] {
    // SAFETY (UJ-006g): as UJ-006.
    let s = unsafe { &mut *ARCH_SPAN.0.get() };
    if s[0] != ACTIONS_MAGIC {
        *s = [0; SPAN_WORDS];
        s[0] = ACTIONS_MAGIC;
    }
    s
}

/// Clears the masked-span record (at instant 0).
#[cfg(feature = "trace-ram")]
pub fn span_reset() {
    let s = span();
    *s = [0; SPAN_WORDS];
    s[0] = ACTIONS_MAGIC;
}

/// A Kernel entry's Rust half begins: opens a span unless one is open
/// (a tail-chained or unwinding continuation).
#[cfg(feature = "trace-ram")]
pub fn span_enter() {
    let now = hw::dwt_cyccnt();
    let s = span();
    if s[4] == 0 {
        s[3] = now;
        s[4] = 1;
    }
}

/// A Kernel entry's Rust half ends: closes the span unless `chained` (the
/// return does not reach a Thread-mode context in which a Kernel-level
/// interrupt is taken).
#[cfg(feature = "trace-ram")]
pub fn span_leave(kind: u8, chained: bool) {
    let now = hw::dwt_cyccnt();
    let s = span();
    if chained || s[4] == 0 {
        return;
    }
    let len = now.wrapping_sub(s[3]);
    s[4] = 0;
    s[2] = s[2].saturating_add(1);
    if len > s[1] {
        s[1] = len;
        s[5] = kind as u32;
    }
    // The histogram: bucket b holds the spans of 2^(b+6) to 2^(b+7)-1
    // cycles, the first everything below 2^7, the last everything from
    // 2^21 on.
    let order = 32 - len.leading_zeros() as usize;
    let b = order.saturating_sub(7).min(SPAN_BUCKETS - 1);
    s[SPAN_HEADER + b] = s[SPAN_HEADER + b].saturating_add(1);
    if (kind as usize) < ENTRY_KINDS {
        let k = SPAN_HEADER + SPAN_BUCKETS + 2 * kind as usize;
        s[k] = s[k].saturating_add(1);
        if len > s[k + 1] {
            s[k + 1] = len;
        }
    }
}

#[cfg(feature = "trace-ram")]
fn actions() -> &'static mut [u32; ACTIONS_WORDS] {
    // SAFETY (UJ-006e): as UJ-006.
    let a = unsafe { &mut *ARCH_ACTIONS.0.get() };
    if a[0] != ACTIONS_MAGIC {
        *a = [0; ACTIONS_WORDS];
        a[0] = ACTIONS_MAGIC;
    }
    a
}

/// Clears the actions record (at instant 0).
#[cfg(feature = "trace-ram")]
pub fn actions_reset() {
    let a = actions();
    *a = [0; ACTIONS_WORDS];
    a[0] = ACTIONS_MAGIC;
    *windows() = [ACTIONS_MAGIC, 0];
}

/// Appends one action (a window entry, kind 3, is counted too).
#[cfg(feature = "trace-ram")]
pub fn action_push(kind: u32, value: u32) {
    if kind == 3 {
        let d = windows();
        d[1] = d[1].wrapping_add(1);
    }
    let a = actions();
    let n = a[1] as usize;
    let s = 2 + 2 * (n % ACTIONS_LEN);
    a[s] = kind;
    a[s + 1] = value;
    a[1] = a[1].wrapping_add(1);
}

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
pub fn events_expect(kinds: u32, task: u8, forbidden_partitions: u32) {
    let e = events();
    e[5] = kinds;
    e[6] = task as u32;
    e[7] = forbidden_partitions;
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
    if e[4] != 0 {
        // Frozen (2026-11-05): the mirror keeps the events that led to the
        // freeze, as the trace ring does, instead of overwriting them; the
        // log's sequence stays followed so no entry re-copies old events.
        e[3] = seq.wrapping_add(1);
        return;
    }
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
    // An event attributed to a forbidden Partition (bit p of the header's
    // eighth word) freezes whatever its kind: a Partition restart names no
    // Task, so a Task-based expectation never saw one (task 19.1).
    let forbidden = partition < 32 && (e[7] >> partition) & 1 == 1;
    if e[4] == 0 && !informational && (!expected || forbidden) {
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
