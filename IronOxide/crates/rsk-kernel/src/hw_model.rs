//! Hardware_Model (R49; task 3.2): Verus specifications of the Target
//! behaviours that the Kernel_Proofs rely on, for the Arm Cortex-M4F r0p1
//! core of the nRF52840 (ASM-01).
//!
//! Every rule names the section that defines it:
//!
//! | Rule | Source |
//! |---|---|
//! | HW-01 Priority encoding, 3 implemented bits, PRIGROUP | Armv7-M ARM B1.5.4 (priority grouping), B3.2.5 (AIRCR.PRIGROUP); nRF52840 PS "NVIC" (3 priority bits, [RN-25]) |
//! | HW-02 Execution priority | Armv7-M ARM B1.5.4 "Execution priority and priority boosting" |
//! | HW-03 BASEPRI and BASEPRI_MAX | Armv7-M ARM B1.4.3 (BASEPRI), B5.2.3 (MSR BASEPRI_MAX) |
//! | HW-04 PRIMASK | Armv7-M ARM B1.4.3 |
//! | HW-05 NVIC arbitration among pending exceptions | Armv7-M ARM B1.5.4 ("the exception with the highest priority is taken; equal priority: lowest exception number") |
//! | HW-06 Exception entry stacks R0-R3, R12, LR, PC, xPSR (and S0-S15, FPSCR when FPCA) on the selected stack | Armv7-M ARM B1.5.6, B1.5.7 |
//! | HW-07 Exception return; NONBASETHRDENA | Armv7-M ARM B1.5.8 (exception return), B3.2.8 (CCR.NONBASETHRDENA) |
//! | HW-08 Handler mode is always privileged; Thread privilege is CONTROL.nPRIV | Armv7-M ARM B1.4.1, B1.4.4 [RN-30] |
//! | HW-09 MPU region match, overlap precedence, background map, subregions | Armv7-M ARM B3.5.1 to B3.5.5, B3.5.9 (SRD) |
//! | HW-10 Eager FP stacking (ASPEN = 1, LSPEN = 0) and FPDSCR defaults | Armv7-M ARM B1.5.7, B3.2.20 (FPCCR), B3.2.22 (FPDSCR) |
//! | HW-11 Precise BusFaults with DISDEFWBUF and Strongly-ordered windows | Cortex-M4 TRM 4.3.4 (ACTLR.DISDEFWBUF); Armv7-M ARM A3.5.5 (Strongly-ordered) |
//! | HW-12 TIMER compare semantics and the near-counter rule | nRF52840 PS 6.30 (TIMER, COMPARE); ASM-19 |
//! | HW-13 DWT CYCCNT | Armv7-M ARM C1.8.7 (DWT_CYCCNT), C1.8.8 (DWT_CTRL.CYCCNTENA); ASM-06 |
//! | HW-14 Tail-chaining and late arrival | Armv7-M ARM B1.5.12 |
//!
//! Errata (R49.4, ASM-01): Cortex-M4 r0p1 erratum 838869 ("store immediate
//! overlapping exception return operation might vector to incorrect
//! interrupt") applies to r0p0 and r0p1 and is worked around by a DSB
//! before every exception return (DD-01); erratum 752770 (VDIV/VSQRT with
//! a very short ISR) is excluded because Jobs never execute floating-point
//! divisions inside the Kernel level and eager stacking preserves the FP
//! state (DD-04). nRF52840 anomalies relevant to the Kernel: 219 (TWIM),
//! 223 (USBD), and 230 (RADIO) concern peripherals that Partitions own and
//! the MPU confines; none touches the TIMER, NVIC, or MPU behaviour used
//! here. The applicability of 838869 to r0p1 and the TIMER near-counter
//! rule are the two facts the P1 spike confirms on hardware (task 3.1).
//!
//! Behaviours deliberately not formalized (R49.6), with the reason no proof
//! depends on them: lazy FP stacking (disabled by the policy, DD-04);
//! bit-banding and unaligned-access semantics (Kernel code uses neither,
//! Link_Checker instruction scan); the SysTick timer (unused; the TIMER
//! peripheral is the time base); debug and trace behaviour other than
//! CYCCNT (ASM-12); cache behaviour (no cache, ASM-04); the exact timing
//! of late arrival (every stub reads the EXC_RETURN and PSP it was entered
//! with, so correctness does not depend on which exception was taken
//! first, only on HW-05 for the order).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use crate::logic::mpu::{Region, View, MPU_REGIONS};

verus! {

// ------------------------------------------------------------ HW-01, HW-02

/// Implemented NVIC priority bits on the nRF52840 (PAR-06, ASM-03).
pub const PRIO_BITS: u8 = 3;

/// The rsk priority map (DD-02): Kernel level 0 encodes as 0x00; a Task of
/// Priority p (1..=7) encodes as (8 - p) << 5. Lower values are more
/// urgent (HW-01).
pub open spec fn encode_level(level: int) -> int {
    (8 - level) * 32
}

/// The group priority of an exception, with PRIGROUP = 4 (all three
/// implemented bits are group bits; no sub-priority bits), is the
/// encoding itself.
pub open spec fn group_priority(level: int) -> int {
    encode_level(level)
}

/// BASEPRI for a System_Ceiling c: 0 (no masking) for the idle level,
/// otherwise the encoding of level c (HW-03).
pub open spec fn basepri_of_ceiling(c: int) -> int {
    if c == 0 { 0 } else { encode_level(c) }
}

/// HW-02: the execution priority is the minimum (most urgent) of the
/// active exceptions' group priorities, of BASEPRI when it is non-zero, and
/// of 0 when PRIMASK is set; 256 (less urgent than everything) when nothing
/// boosts it.
pub open spec fn execution_priority(active: Set<int>, basepri: int, primask: bool) -> int {
    let from_active = if active.is_empty() { 256 } else { min_of(active) };
    let from_basepri = if basepri == 0 { 256 } else { basepri };
    let from_primask = if primask { 0 } else { 256 };
    min3(from_active, from_basepri, from_primask)
}

pub open spec fn min3(a: int, b: int, c: int) -> int {
    if a <= b && a <= c { a } else if b <= c { b } else { c }
}

/// The minimum of a finite non-empty set of group priorities.
pub open spec fn min_of(s: Set<int>) -> int {
    choose|m: int| s.contains(m) && forall|x: int| s.contains(x) ==> m <= x
}

/// HW-05: a pending exception of group priority `g` is taken only when it
/// is strictly more urgent than the execution priority.
pub open spec fn nvic_takes(g: int, exec_priority: int) -> bool {
    g < exec_priority
}

/// The encoding is strictly decreasing in the level: more urgent levels
/// have smaller group priorities.
pub proof fn lemma_encoding_order(a: int, b: int)
    ensures a < b <==> encode_level(b) < encode_level(a),
{
}

/// Property 1 over the Hardware_Model (R7.1, R20.3): with BASEPRI set from
/// the System_Ceiling, the active exceptions being exactly the dispatch
/// vectors of the Jobs in progress, and PRIMASK clear, the NVIC takes a
/// pending Task vector of Priority p exactly when p exceeds the ceiling and
/// every Priority in progress.
pub proof fn lemma_nvic_takes_iff_eligible(in_progress: Set<int>, ceiling: int, p: int)
    requires
        1 <= p <= 7, 0 <= ceiling <= 7,
        forall|q: int| in_progress.contains(q) ==> 1 <= q <= 7,
    ensures
        nvic_takes(group_priority(p), execution_priority(in_progress.map(|q: int| group_priority(q)), basepri_of_ceiling(ceiling), false))
            <==> (p > ceiling && forall|q: int| in_progress.contains(q) ==> p > q),
{
    broadcast use vstd::set_lib::group_set_properties;
    broadcast use Set::lemma_map_contains;
    let active = in_progress.map(|q: int| group_priority(q));
    let from_basepri = if ceiling == 0 { 256 } else { encode_level(ceiling) };
    if in_progress.is_empty() {
        assert(active.is_empty());
    } else {
        assert(!active.is_empty()) by {
            let q = in_progress.choose();
            assert(active.contains(group_priority(q)));
        }
        // min_of(active) is attained by some in-progress level, and bounds all of them.
        let m = min_of(active);
        assert(active.contains(m) && forall|x: int| active.contains(x) ==> m <= x) by {
            // A finite non-empty set of integers has a minimum: witness by induction over the set.
            lemma_finite_set_has_min(active);
        }
        let qm = choose|q: int| in_progress.contains(q) && group_priority(q) == m;
        assert(in_progress.contains(qm) && group_priority(qm) == m);
        // group_priority(p) < m  <==>  p > qm, and qm is the highest in-progress level.
        lemma_encoding_order(qm, p);
        assert forall|q: int| in_progress.contains(q) implies q <= qm by {
            assert(active.contains(group_priority(q)));
            lemma_encoding_order(q, qm);
            lemma_encoding_order(qm, q);
        }
    }
    if ceiling > 0 {
        lemma_encoding_order(ceiling, p);
    }
}

/// Every finite non-empty set of integers has a minimum.
pub proof fn lemma_finite_set_has_min(s: Set<int>)
    requires !s.is_empty(),
    ensures s.contains(min_of(s)) && forall|x: int| s.contains(x) ==> min_of(s) <= x,
    decreases s.len(),
{
    broadcast use vstd::set_lib::group_set_properties;
    let e = s.choose();
    let rest = s.remove(e);
    if rest.is_empty() {
        assert(s =~= Set::<int>::empty().insert(e));
        assert(s.contains(e) && forall|x: int| s.contains(x) ==> e <= x);
    } else {
        lemma_finite_set_has_min(rest);
        let m = min_of(rest);
        let cand = if e <= m { e } else { m };
        assert(s.contains(cand) && forall|x: int| s.contains(x) ==> cand <= x);
    }
}

// ------------------------------------------------------------------ HW-03

/// HW-03: an MSR to BASEPRI_MAX raises BASEPRI only: the write takes effect
/// when BASEPRI is 0 or the new value is more urgent (smaller) than the
/// current one.
pub open spec fn basepri_max_write(current: int, value: int) -> int {
    if current == 0 || (value != 0 && value < current) { value } else { current }
}

/// The Kernel writes BASEPRI from the ceiling mirror with a plain MSR
/// BASEPRI (privileged, inside SVC), so both raising and restoring take
/// effect; BASEPRI_MAX is modelled for the RTIC comparison only.
pub proof fn lemma_basepri_max_only_raises(current: int, value: int)
    requires 0 <= current, 0 <= value,
    ensures basepri_max_write(current, value) == 0 || current == 0 || basepri_max_write(current, value) <= current,
{
}

// ------------------------------------------------------------ HW-07, HW-08

/// HW-07: an exception return always deactivates the returning exception
/// (the one IPSR names, B1.5.8 `ExceptionReturn`/`DeActivate`); a return
/// to Thread mode while *other* exceptions are still active is permitted
/// only when CCR.NONBASETHRDENA is 1, otherwise it is an INVPC UsageFault.
/// DD-01 (revised by the P1 spike, 2026-10-08) relies on the first
/// sentence only: a dispatch stub's return to Thread mode deactivates the
/// Job's vector, so no exception is active while a Job executes, the
/// Job's level is enforced by BASEPRI (HW-03, HW-05), and every return is
/// a return to the base level. NONBASETHRDENA stays 0.
pub open spec fn exception_return_ok(nonbasethrdena: bool, to_thread: bool, others_active: bool) -> bool {
    !(to_thread && others_active) || nonbasethrdena
}

/// HW-07 (deactivation): after the return, the returning exception is not
/// active. The Kernel's returns always happen with it as the only active
/// exception, so the resumed Thread-mode context runs at the base level.
pub open spec fn active_after_return(active_before: Set<int>, returning: int) -> Set<int> {
    active_before.remove(returning)
}

pub proof fn lemma_return_from_sole_active_reaches_base(active_before: Set<int>, returning: int)
    requires active_before =~= set![returning],
    ensures active_after_return(active_before, returning) =~= Set::<int>::empty(),
{
}

/// HW-08: Handler mode executes privileged; Thread mode is unprivileged
/// exactly when CONTROL.nPRIV is 1.
pub open spec fn privileged(handler_mode: bool, npriv: bool) -> bool {
    handler_mode || !npriv
}

// ------------------------------------------------------------------ HW-09

/// Access kinds for the MPU decision.
pub const ACCESS_READ: u8 = 0;
pub const ACCESS_WRITE: u8 = 1;
pub const ACCESS_FETCH: u8 = 2;

/// HW-09: the region that decides an access is the highest-numbered
/// enabled region that covers the address (B3.5.3).
pub open spec fn deciding_region(view: &View, a: int) -> Option<int> {
    if exists|r: int| 0 <= r < MPU_REGIONS && view.regions[r].covers(a) {
        Some(choose|r: int| 0 <= r < MPU_REGIONS && view.regions[r].covers(a)
            && forall|s: int| 0 <= s < MPU_REGIONS && view.regions[s].covers(a) ==> s <= r)
    } else {
        None
    }
}

/// HW-09: whether an unprivileged access of `kind` to `a` is granted
/// (PRIVDEFENA affects privileged accesses only, which use the default
/// map; Jobs are unprivileged, DD-01).
pub open spec fn mpu_grants_unprivileged(view: &View, a: int, kind: u8) -> bool {
    match deciding_region(view, a) {
        None => false,
        Some(r) => {
            let region = view.regions[r];
            if kind == ACCESS_WRITE { region.write }
            else if kind == ACCESS_FETCH { !region.xn }
            else { true }
        }
    }
}

/// Whatever the hardware grants to an unprivileged access, some enabled
/// region of the view covers the address with that permission, which is
/// the over-approximation `mpu::View` uses (Property 12 rests on it).
pub proof fn lemma_mpu_grant_covered(view: &View, a: int, kind: u8)
    ensures
        mpu_grants_unprivileged(view, a, kind) && kind == ACCESS_WRITE ==> view.writable(a),
        mpu_grants_unprivileged(view, a, kind) && kind == ACCESS_FETCH ==> view.executable(a),
        mpu_grants_unprivileged(view, a, kind) ==> view.readable(a),
{
    if mpu_grants_unprivileged(view, a, kind) {
        let r = deciding_region(view, a).unwrap();
        assert(exists|s: int| 0 <= s < MPU_REGIONS && view.regions[s].covers(a));
        // The chosen deciding region exists because a maximum index among finitely many covering regions exists.
        lemma_max_covering_region_exists(view, a);
        assert(0 <= r < MPU_REGIONS && view.regions[r].covers(a));
    }
}

/// Among the (at most eight) regions covering `a`, one has the greatest index.
pub proof fn lemma_max_covering_region_exists(view: &View, a: int)
    requires exists|r: int| 0 <= r < MPU_REGIONS && view.regions[r].covers(a),
    ensures exists|r: int| 0 <= r < MPU_REGIONS && view.regions[r].covers(a)
        && forall|s: int| 0 <= s < MPU_REGIONS && view.regions[s].covers(a) ==> s <= r,
{
    lemma_max_covering_below(view, a, MPU_REGIONS as int);
}

/// Induction on the number of regions considered.
proof fn lemma_max_covering_below(view: &View, a: int, n: int)
    requires 0 < n <= MPU_REGIONS, exists|r: int| 0 <= r < n && view.regions[r].covers(a),
    ensures exists|r: int| 0 <= r < n && view.regions[r].covers(a)
        && forall|s: int| 0 <= s < n && view.regions[s].covers(a) ==> s <= r,
    decreases n,
{
    if view.regions[n - 1].covers(a) {
        assert(0 <= n - 1 < n && view.regions[n - 1].covers(a)
            && forall|s: int| 0 <= s < n && view.regions[s].covers(a) ==> s <= n - 1);
    } else {
        let r0 = choose|r: int| 0 <= r < n && view.regions[r].covers(a);
        assert(r0 < n - 1);
        lemma_max_covering_below(view, a, n - 1);
        let r = choose|r: int| 0 <= r < n - 1 && view.regions[r].covers(a)
            && forall|s: int| 0 <= s < n - 1 && view.regions[s].covers(a) ==> s <= r;
        assert(forall|s: int| 0 <= s < n && view.regions[s].covers(a) ==> s <= r);
    }
}

// ------------------------------------------------------------------ HW-10

/// HW-10: the FP context control settings of the policy (DD-04): eager
/// stacking, no lazy stacking. With these settings an exception entered
/// while FPCA is set pushes the extended frame (26 words) and clears
/// FPCA; FPDSCR supplies the FPSCR of every newly created FP context.
pub const FPCCR_ASPEN: bool = true;
pub const FPCCR_LSPEN: bool = false;
/// Basic and extended exception frame sizes in bytes (B1.5.7).
pub const BASIC_FRAME_BYTES: u32 = 32;
pub const EXTENDED_FRAME_BYTES: u32 = 104;

pub open spec fn frame_bytes(fpca: bool) -> int {
    if fpca { EXTENDED_FRAME_BYTES as int } else { BASIC_FRAME_BYTES as int }
}

/// The frame pushed on exception entry: 8 words, or 26 with the FP
/// context, 8-byte aligned when CCR.STKALIGN is set (the default).
pub open spec fn entry_frame_size(fpca: bool, lspen: bool) -> int {
    if fpca && !lspen { EXTENDED_FRAME_BYTES as int } else if fpca { EXTENDED_FRAME_BYTES as int } else { BASIC_FRAME_BYTES as int }
}

// ------------------------------------------------------------------ HW-11

/// HW-11: a BusFault is precise when the write buffer is disabled for the
/// default memory map (ACTLR.DISDEFWBUF = 1) or the access targets
/// Strongly-ordered memory (DD-12).
pub open spec fn bus_fault_precise(disdefwbuf: bool, strongly_ordered: bool) -> bool {
    disdefwbuf || strongly_ordered
}

// ------------------------------------------------------------------ HW-12

/// HW-12 (ASM-19): a COMPARE event is guaranteed only when the compare
/// value is at least this many ticks ahead of the counter at the time of
/// the write; the Kernel never arms a compare closer than this and treats
/// a slot whose instant is closer as already due.
pub const MIN_COMPARE_DISTANCE: u64 = 2;

/// The 32-bit TIMER counter in 32-bit mode wraps at 2^32; the Kernel
/// extends it to 64 bits by counting wraps from the overflow compare
/// (DD-05). `now64` is the extended instant for a counter reading `low`
/// after `wraps` wraps.
pub open spec fn extended_instant(wraps: int, low: int) -> int {
    wraps * 0x1_0000_0000 + low
}

/// The compare value for a target instant: never closer than
/// MIN_COMPARE_DISTANCE to the current counter.
pub open spec fn compare_for(now: int, target: int) -> int {
    if target < now + MIN_COMPARE_DISTANCE { now + MIN_COMPARE_DISTANCE } else { target }
}

pub proof fn lemma_compare_never_too_close(now: int, target: int)
    ensures compare_for(now, target) >= now + MIN_COMPARE_DISTANCE, compare_for(now, target) >= target,
{
}

// ------------------------------------------------------------------ HW-13

/// HW-13: CYCCNT is a 32-bit core-cycle counter that wraps; the elapsed
/// cycles between two readings less than 2^32 cycles apart is their
/// wrapping difference.
pub open spec fn elapsed_cycles(start: int, end: int) -> int {
    if end >= start { end - start } else { end + 0x1_0000_0000 - start }
}

pub proof fn lemma_elapsed_in_range(start: int, end: int)
    requires 0 <= start < 0x1_0000_0000, 0 <= end < 0x1_0000_0000,
    ensures 0 <= elapsed_cycles(start, end) < 0x1_0000_0000,
{
}

// ------------------------------------------------------------------ HW-06, HW-14

/// HW-06: exception entry pushes the frame on the stack selected by the
/// mode: PSP for Thread mode when CONTROL.SPSEL is set (Jobs), MSP
/// otherwise (Kernel). HW-14: with tail-chaining, the stub of a chained
/// exception sees the frame of the original preempted context, and with
/// late arrival a higher-priority exception replaces the one being
/// entered; in both cases the stub's EXC_RETURN and PSP describe the
/// context that was actually preempted, which is all the dispatch logic
/// reads (design.md "Priority and privilege model").
pub open spec fn frame_on_psp(thread_mode: bool, spsel: bool) -> bool {
    thread_mode && spsel
}

} // verus!
