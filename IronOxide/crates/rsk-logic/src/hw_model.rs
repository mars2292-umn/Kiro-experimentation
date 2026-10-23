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
//! | HW-04 PRIMASK; CPS and MSR change PRIMASK and FAULTMASK only when privileged | Armv7-M ARM B1.4.3 (PRIMASK, FAULTMASK), B5.2.1 (CPS), B5.2.3 (MSR) |
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
//! first, only on HW-05 for the order); FAULTMASK as an input of the
//! execution priority (HW-02: it is always clear, because nothing in the
//! Kernel sets it and a Job cannot, `lemma_job_cannot_set_mask`); the
//! fixed-priority exceptions Reset, NMI, and HardFault in HW-05
//! arbitration (they are more urgent than every configurable level and
//! lead to the system safe state, R14.4, so no proof depends on their
//! order); the NVIC enable bits as state (a `pending` set holds the
//! exceptions that are pending and enabled; the Kernel enables the
//! time-base TIMER, the dispatch vectors, and the system handlers at boot
//! and never disables them, and its own disabling of intake lines during
//! MIT windows and of unbound lines, R10.9 and R19.5, is a Kernel decision
//! that no proof treats as masking).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use crate::logic::mpu::{Layout, Region, View, MPU_REGIONS};
use crate::logic::sched::{Sched, JOB_NONE, REL_PENDING};
use crate::logic::Config;

verus! {

// ------------------------------------------------------------ HW-01, HW-02

/// Implemented NVIC priority bits on the nRF52840 (PAR-06, ASM-03).
pub const PRIO_BITS: u8 = 3;

/// The rsk priority map (DD-02): Kernel level 0 encodes as 0x00; a Task of
/// Priority p (1..=7) encodes as (8 - p) << 5. Lower values are more
/// urgent (HW-01). Mirrors `arch::cm4::encode_level`, which programs every
/// vector's NVIC priority.
pub open spec fn encode_level(level: int) -> int {
    if level == 0 { 0 } else { (8 - level) * 32 }
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

/// Over the Task levels 1..=7 the encoding is strictly decreasing: more
/// urgent levels have smaller group priorities. The Kernel level 0 is more
/// urgent than every Task level.
pub proof fn lemma_encoding_order(a: int, b: int)
    requires 1 <= a <= 7, 1 <= b <= 7,
    ensures
        a < b <==> encode_level(b) < encode_level(a),
        encode_level(0) < encode_level(a),
{
}

/// The start condition when the Jobs in progress keep their dispatch
/// vectors active (the original DD-01 form): with BASEPRI set from the
/// System_Ceiling, the active exceptions being exactly the dispatch
/// vectors of the Jobs in progress, and PRIMASK clear, the NVIC takes a
/// pending Task vector of Priority p exactly when p exceeds the ceiling and
/// every Priority in progress. Under the revised DD-01 no exception is
/// active while a Job executes; Property 1's hardware side is
/// `lemma_dispatch_takes_iff_eligible` and
/// `lemma_dispatch_selects_srp_choice` below.
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

// ------------------------------------- Property 1 under the revised DD-01

/// HW-03 under the revised DD-01: BASEPRI while the Job of Priority `top`
/// executes under System_Ceiling `ceiling` is the encoding of the higher of
/// the two levels, and 0 (no masking) when both are the idle level; `top`
/// is 0 when no Job is in progress. Mirrors `arch::cm4::basepri_for`
/// (`basepri_for_ceiling` of the maximum of the two).
pub open spec fn basepri_for(top: int, ceiling: int) -> int {
    basepri_of_ceiling(if top > ceiling { top } else { ceiling })
}

/// With no exception active and PRIMASK clear, BASEPRI from `basepri_for`
/// lets the NVIC take a pending Task vector of Priority p exactly when p
/// exceeds both the System_Ceiling and the executing Job's Priority
/// (HW-01, HW-02, HW-03).
pub proof fn lemma_nvic_takes_iff_above_job_and_ceiling(top: int, ceiling: int, p: int)
    requires 1 <= p <= 7, 0 <= top <= 7, 0 <= ceiling <= 7,
    ensures
        nvic_takes(group_priority(p), execution_priority(Set::<int>::empty(), basepri_for(top, ceiling), false))
            <==> (p > ceiling && p > top),
{
    assert(Set::<int>::empty().is_empty());
}

/// HW-05: the pending exception that the NVIC takes. Among the pending
/// exceptions whose group priority is more urgent than the execution
/// priority, it is the one with the most urgent (smallest) group priority,
/// and among equal group priorities the one with the lowest exception
/// number (B1.5.4). `group` maps an exception number to its group priority.
/// `pending` holds the exceptions that are pending and enabled (NVIC ISER,
/// SHCSR); a disabled line is never taken.
pub open spec fn nvic_selects(pending: Set<int>, group: spec_fn(int) -> int, exec_priority: int, n: int) -> bool {
    &&& pending.contains(n)
    &&& nvic_takes(group(n), exec_priority)
    &&& forall|m: int| #[trigger] pending.contains(m) && nvic_takes(group(m), exec_priority)
            ==> group(n) < group(m) || (group(n) == group(m) && n <= m)
}

/// HW-05 names at most one exception.
pub proof fn lemma_nvic_selects_unique(pending: Set<int>, group: spec_fn(int) -> int, exec_priority: int, n: int, m: int)
    requires nvic_selects(pending, group, exec_priority, n), nvic_selects(pending, group, exec_priority, m),
    ensures n == m,
{
}

/// The Priority level of the executing Job: 0 (idle) when no Job is in
/// progress. Mirrors `arch::cm4::basepri_now`.
pub open spec fn job_level<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>,
) -> int {
    if s.depth == 0 { 0 } else { cfg.priority(s.top()) as int }
}

/// The execution priority against which the NVIC arbitrates the pending
/// dispatch vectors wherever a Job may start (revised DD-01):
/// - no exception is active: every return from a dispatch stub or a
///   Kernel handler to Thread mode deactivates the returning exception
///   (HW-07), and a tail-chained exception is compared with the context
///   being returned to (HW-14);
/// - PRIMASK is clear: the stubs that set it clear it before their
///   exception return, and unprivileged Jobs cannot set it (HW-08);
/// - BASEPRI is `basepri_for` of the executing Job's level and the
///   System_Ceiling, which `arch::cm4::apply` writes after every
///   transition.
pub open spec fn dispatch_exec_priority<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>,
) -> int {
    execution_priority(Set::<int>::empty(), basepri_for(job_level(s, cfg), s.ceiling as int), false)
}

/// The Job that Property 1 says starts: eligible (R7.1), of the highest
/// Priority among the pending Releases (R7.2), and of the lowest Task
/// identifier among equal Priorities (R7.3, DD-03). These are the clauses
/// of `Sched::select`'s postcondition; `Kernel::select`, whose result
/// Kernel_Arch pends, ensures this predicate.
pub open spec fn srp_selection<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>, t: int,
) -> bool {
    &&& 0 <= t < NT
    &&& s.eligible(cfg, t)
    &&& forall|u: int| 0 <= u < NT && s.release[u] == REL_PENDING ==> cfg.priority(u) <= cfg.priority(t)
    &&& forall|u: int| 0 <= u < NT && s.release[u] == REL_PENDING && cfg.priority(u) == cfg.priority(t) ==> t <= u
}

/// DD-03 and HW-01: `vector(t)` is Task t's dispatch exception number,
/// assigned in Task-identifier (declaration) order, and its group priority
/// encodes the Task's Priority. The Generator assigns the vectors from the
/// Target's `dispatch_irqs` list in Task order (rsk-gen `derive`; the list
/// is strictly ascending for every Target, rsk-gen test
/// `dispatch_irqs_ascend_for_dd03`), and the Kernel programs each vector's
/// NVIC priority with `encode_level` at boot (`arch::cm4::boot`).
pub open spec fn dispatch_vectors_ok<const NT: usize, const NR: usize, const NP: usize>(
    cfg: &Config<NT, NR, NP>, vector: spec_fn(int) -> int, group: spec_fn(int) -> int,
) -> bool {
    &&& forall|t: int, u: int| 0 <= t < u < NT ==> #[trigger] vector(t) < #[trigger] vector(u)
    &&& forall|t: int| 0 <= t < NT ==> #[trigger] group(vector(t)) == group_priority(cfg.priority(t) as int)
}

/// Every pending dispatch vector belongs to a Task whose Release is
/// pending. A stale pending bit (a Release suppressed after its vector was
/// pended, for example by STOP_PARTITION) violates this; the dispatch stub
/// rejects such an entry with `eligible`, taking the exception clears the
/// bit, and the NVIC arbitrates again on the stub's return. That
/// re-arbitration is a property of the entry path, outside this model.
pub open spec fn no_stale_dispatch<const NT: usize, const NR: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, pending: Set<int>, vector: spec_fn(int) -> int,
) -> bool {
    forall|n: int| #[trigger] pending.contains(n)
        ==> exists|t: int| 0 <= t < NT && #[trigger] vector(t) == n && s.release[t] == REL_PENDING
}

/// Property 1, start condition, over the Hardware_Model (R7.1, R20.3):
/// under the revised DD-01, the NVIC takes the dispatch vector of a Task
/// with a pending Release exactly when the Kernel logic deems the Task
/// eligible.
pub proof fn lemma_dispatch_takes_iff_eligible<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>, t: int,
)
    requires s.inv(cfg), 0 <= t < NT, s.release[t] == REL_PENDING,
    ensures
        nvic_takes(group_priority(cfg.priority(t) as int), dispatch_exec_priority(s, cfg)) <==> s.eligible(cfg, t),
{
    assert(cfg.task_ok(t));
    if s.depth > 0 {
        assert((s.stack[s.depth - 1] as int) < NT);
        assert(cfg.task_ok(s.top()));
    }
    lemma_nvic_takes_iff_above_job_and_ceiling(job_level(s, cfg), s.ceiling as int, cfg.priority(t) as int);
    if s.job[t] != JOB_NONE {
        // A Task whose Job is in progress is at or below the executing Job's level.
        s.lemma_in_progress_below_top(cfg, t);
    }
}

/// Property 1, selection, over the Hardware_Model (R7.1 to R7.3, R20.3):
/// once Kernel_Arch has pended the vector of the Job that the logic
/// selects, the NVIC's arbitration (HW-05) takes that vector and no other,
/// whatever other dispatch vectors are pending. Ties between equal
/// Priorities go to the lowest Task identifier on both sides (DD-03).
pub proof fn lemma_dispatch_selects_srp_choice<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>,
    vector: spec_fn(int) -> int, group: spec_fn(int) -> int, pending: Set<int>, t: int,
)
    requires
        s.inv(cfg),
        dispatch_vectors_ok(cfg, vector, group),
        no_stale_dispatch(s, pending, vector),
        srp_selection(s, cfg, t),
        pending.contains(vector(t)),
    ensures
        forall|n: int| nvic_selects(pending, group, dispatch_exec_priority(s, cfg), n) <==> n == vector(t),
{
    let exec = dispatch_exec_priority(s, cfg);
    // The selected Job's vector can be taken: the Job is eligible.
    lemma_dispatch_takes_iff_eligible(s, cfg, t);
    assert(group(vector(t)) == group_priority(cfg.priority(t) as int));
    // Every other pending vector belongs to a pending Release of no higher
    // Priority, and one of equal Priority has a higher Task identifier, so a
    // higher exception number.
    assert forall|m: int| #[trigger] pending.contains(m) && nvic_takes(group(m), exec)
        implies group(vector(t)) < group(m) || (group(vector(t)) == group(m) && vector(t) <= m) by {
        let u = choose|u: int| 0 <= u < NT && #[trigger] vector(u) == m && s.release[u] == REL_PENDING;
        assert(group(vector(u)) == group_priority(cfg.priority(u) as int));
        assert(cfg.priority(u) <= cfg.priority(t));
        assert(cfg.task_ok(u) && cfg.task_ok(t));
        lemma_encoding_order(cfg.priority(u) as int, cfg.priority(t) as int);
        if cfg.priority(u) == cfg.priority(t) && t < u {
            assert(vector(t) < vector(u));
        }
    }
    assert(nvic_selects(pending, group, exec, vector(t)));
    assert forall|n: int| nvic_selects(pending, group, exec, n) <==> n == vector(t) by {
        if nvic_selects(pending, group, exec, n) {
            lemma_nvic_selects_unique(pending, group, exec, n, vector(t));
        }
    }
}

/// Property 1, no start (R7.1): when no Job is eligible, the NVIC takes no
/// pending dispatch vector.
pub proof fn lemma_dispatch_idle_when_none_eligible<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>,
    vector: spec_fn(int) -> int, group: spec_fn(int) -> int, pending: Set<int>,
)
    requires
        s.inv(cfg),
        dispatch_vectors_ok(cfg, vector, group),
        no_stale_dispatch(s, pending, vector),
        forall|t: int| 0 <= t < NT ==> !s.eligible(cfg, t),
    ensures
        forall|n: int| !nvic_selects(pending, group, dispatch_exec_priority(s, cfg), n),
{
    let exec = dispatch_exec_priority(s, cfg);
    assert forall|n: int| !nvic_selects(pending, group, exec, n) by {
        if nvic_selects(pending, group, exec, n) {
            let u = choose|u: int| 0 <= u < NT && #[trigger] vector(u) == n && s.release[u] == REL_PENDING;
            assert(group(vector(u)) == group_priority(cfg.priority(u) as int));
            lemma_dispatch_takes_iff_eligible(s, cfg, u);
            assert(!s.eligible(cfg, u));
        }
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

// ------------------------- Property 10, level part: the Kernel is never masked

/// HW-04 with HW-08: CPS and MSR change PRIMASK and FAULTMASK only when
/// executed privileged; unprivileged, they leave the mask unchanged
/// (Armv7-M ARM B5.2.1 CPS and B5.2.3 MSR, both guarded by
/// `CurrentModeIsPrivileged()`). `set` is the value the instruction would
/// write. MSR BASEPRI is guarded the same way, but no BASEPRI value masks
/// the Kernel level (`lemma_kernel_level_never_masked`), so it needs no
/// rule here.
pub open spec fn mask_write(privileged: bool, current: bool, set: bool) -> bool {
    if privileged { set } else { current }
}

/// A Job (Thread mode with CONTROL.nPRIV = 1, DD-01) cannot set PRIMASK or
/// FAULTMASK. FAULTMASK, which `execution_priority` leaves out, is never
/// set by the Kernel either, so it never masks the Kernel level.
pub proof fn lemma_job_cannot_set_mask(current: bool, set: bool)
    ensures mask_write(privileged(false, true), current, set) == current,
{
}

/// Property 10, level part (DD-02, R18.2): a pending, enabled exception at
/// the Kernel level (group priority 0x00) is taken whatever BASEPRI holds
/// and whatever Task-level exceptions are active. Only PRIMASK, or an
/// active Kernel-level exception (the Kernel itself running, non-reentrant
/// by DD-02), delays it. The Kernel level carries the time-base TIMER that
/// serves the Budget events, SVC, MemManage, BusFault, UsageFault, and
/// peripheral intake (`arch::cm4::boot` programs them with
/// `encode_level(0)` and `set_kernel_exception_priorities`). The TIMER and
/// the system handlers stay enabled after boot; the Kernel disables only
/// intake lines, during MIT windows (R10.9), and unbound lines (R19.5),
/// which is MIT enforcement, not masking of the Budget events.
pub proof fn lemma_kernel_level_never_masked(active: Set<int>, basepri: int)
    requires
        0 <= basepri,
        forall|g: int| active.contains(g) ==> g > group_priority(0),
    ensures
        nvic_takes(group_priority(0), execution_priority(active, basepri, false)),
{
    if !active.is_empty() {
        lemma_finite_set_has_min(active);
    }
}

/// Property 10, level part, in every reachable state (R18.2): while a Job
/// executes, under any System_Ceiling up to its maximum, and while no Job
/// is in progress, the NVIC takes a pending Kernel-level exception. The
/// execution priority is the one of the revised DD-01
/// (`dispatch_exec_priority`): no exception active, PRIMASK clear (a Job
/// cannot set it, `lemma_job_cannot_set_mask`), BASEPRI from `basepri_for`.
pub proof fn lemma_kernel_level_taken_in_every_state<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    s: &Sched<NT, NR, NL>, cfg: &Config<NT, NR, NP>,
)
    requires s.inv(cfg),
    ensures nvic_takes(group_priority(0), dispatch_exec_priority(s, cfg)),
{
    if s.depth > 0 {
        assert((s.stack[s.depth - 1] as int) < NT);
        assert(cfg.task_ok(s.top()));
    }
    lemma_kernel_level_never_masked(Set::<int>::empty(), basepri_for(job_level(s, cfg), s.ceiling as int));
}

/// HW-05 with DD-02: while a Kernel-level exception is pending and can be
/// taken, the NVIC takes a Kernel-level exception, never a Task vector.
/// `pending` holds configurable exceptions (group priorities 0 to 255);
/// HardFault and NMI, fixed above every configurable level, are Kernel
/// handlers as well (R14.4).
pub proof fn lemma_kernel_level_wins_arbitration(
    pending: Set<int>, group: spec_fn(int) -> int, exec_priority: int, k: int, n: int,
)
    requires
        forall|m: int| pending.contains(m) ==> group(m) >= group_priority(0),
        pending.contains(k),
        group(k) == group_priority(0),
        nvic_takes(group(k), exec_priority),
        nvic_selects(pending, group, exec_priority, n),
    ensures
        group(n) == group_priority(0),
{
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
/// the over-approximation `mpu::View` uses (Property 12 rests on it:
/// `lemma_hw_partition_isolation`, `lemma_hw_write_isolation`).
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

// ------------------------------------- Property 12 over the Hardware_Model

/// The addresses Partition `q` owns in the layout: its code, its RAM
/// (which holds its stack and Resource storage), and its peripheral
/// windows. The shared code is owned by no Partition.
pub open spec fn partition_owns<const NP: usize>(layout: &Layout<NP>, q: int, a: int) -> bool {
    let ql = layout.partitions[q];
    ql.code.contains(a) || ql.ram.contains(a) || ql.in_window(a)
}

/// Property 12 over the Hardware_Model (R16.1, R16.3, R16.5, R16.6,
/// R44.4): for a layout that passes the boot validation
/// (`Layout::validate` ensures `wf`) and the view that `Layout::view`
/// computes for Partition `p`, the MPU's own decision for an unprivileged
/// access of any kind (HW-09: highest-numbered covering region, subregion
/// disables, no background region for unprivileged accesses):
/// - grants only addresses in `p`'s code, RAM, peripheral windows, or the
///   shared code; writes only to its RAM or windows; instruction fetches
///   only from its code or the shared code;
/// - grants nothing in the Kernel's flash or RAM;
/// - grants nothing that another Partition owns;
/// - refuses a write to the word below `p`'s stack, so a stack overflow
///   faults before it modifies memory outside the stack region (R16.5);
/// - is computed from regions that each satisfy the Armv7-M size and
///   alignment rules.
/// The proof composes `lemma_mpu_grant_covered` (the hardware grant implies
/// the view's over-approximation) with the view lemmas of `logic::mpu`.
/// Outside this model, and holding by inspection of Kernel_Arch: that it
/// loads these views (computed once with `Layout::view`, after
/// `arch::cm4::boot` has checked `validate`); that the RBAR/RASR words it
/// writes encode the view (Kani harnesses `rasr_encoding_fields`,
/// `rbar_encoding_fields`); and that the MPU is enabled while Jobs run
/// (`hw::mpu_enable_background` at boot). Kernel-
/// owned peripherals and the PPB are not intervals of the layout, so they
/// are excluded only through containment, as long as no window of `p`
/// covers them (the Generator, PR-29, and the Config_Checker, R16.7).
pub proof fn lemma_hw_partition_isolation<const NP: usize>(layout: &Layout<NP>, p: int, vp: &View, a: int, kind: u8)
    requires layout.wf(), 0 <= p < NP, layout.is_view_of(p, vp),
    ensures
        mpu_grants_unprivileged(vp, a, kind) ==> layout.shared_code.contains(a)
            || layout.partitions[p].code.contains(a) || layout.partitions[p].ram.contains(a)
            || layout.partitions[p].in_window(a),
        mpu_grants_unprivileged(vp, a, ACCESS_WRITE) ==> layout.partitions[p].ram.contains(a)
            || layout.partitions[p].in_window(a),
        mpu_grants_unprivileged(vp, a, ACCESS_FETCH) ==> layout.shared_code.contains(a)
            || layout.partitions[p].code.contains(a),
        mpu_grants_unprivileged(vp, a, kind) ==> !layout.kernel_flash.contains(a) && !layout.kernel_ram.contains(a),
        forall|q: int| 0 <= q < NP && q != p
            ==> (mpu_grants_unprivileged(vp, a, kind) ==> !#[trigger] partition_owns(layout, q, a)),
        !mpu_grants_unprivileged(vp, layout.partitions[p].ram.base as int - 1, ACCESS_WRITE),
        forall|i: int| 0 <= i < MPU_REGIONS && #[trigger] vp.regions[i].enabled ==> vp.regions[i].interval.rules_ok(),
{
    let pl = layout.partitions[p];
    assert(layout.partition_ok(p));
    // The hardware grant implies the view's over-approximation, which the
    // view lemmas bound.
    lemma_mpu_grant_covered(vp, a, kind);
    lemma_mpu_grant_covered(vp, a, ACCESS_WRITE);
    lemma_mpu_grant_covered(vp, a, ACCESS_FETCH);
    layout.lemma_view_within_layout(p, vp, a);
    if mpu_grants_unprivileged(vp, a, kind) {
        // Kernel memory: every interval of p and the shared code are
        // disjoint from the Kernel's flash and RAM.
        if pl.in_window(a) {
            let w = choose|w: int| 0 <= w < pl.window_count && pl.windows[w].contains(a);
            assert(pl.windows[w].disjoint(&layout.kernel_flash) && pl.windows[w].disjoint(&layout.kernel_ram));
        }
        // Another Partition's memory and peripherals: p's intervals and the
        // shared code are disjoint from everything q owns.
        assert forall|q: int| 0 <= q < NP && q != p implies !#[trigger] partition_owns(layout, q, a) by {
            assert(layout.pair_ok(p, q));
            assert(layout.partition_ok(q));
            let ql = layout.partitions[q];
            if partition_owns(layout, q, a) {
                if ql.in_window(a) {
                    let w = choose|w: int| 0 <= w < ql.window_count && ql.windows[w].contains(a);
                    assert(ql.windows[w].disjoint(&layout.shared_code));
                    assert(ql.windows[w].disjoint(&pl.code) && ql.windows[w].disjoint(&pl.ram));
                    if pl.in_window(a) {
                        let v = choose|v: int| 0 <= v < pl.window_count && pl.windows[v].contains(a);
                        assert(pl.windows[v].disjoint(&ql.windows[w]));
                    }
                } else {
                    assert(ql.code.disjoint(&layout.shared_code) && ql.ram.disjoint(&layout.shared_code));
                    if pl.in_window(a) {
                        let v = choose|v: int| 0 <= v < pl.window_count && pl.windows[v].contains(a);
                        assert(pl.windows[v].disjoint(&ql.code) && pl.windows[v].disjoint(&ql.ram));
                    }
                }
                assert(false);
            }
        }
    }
    // Stack overflow: the word below the stack is writable in no region of
    // the view, so the hardware refuses the write.
    layout.lemma_stack_overflow_faults(p, vp);
    lemma_mpu_grant_covered(vp, pl.ram.base as int - 1, ACCESS_WRITE);
    // Region rules: every enabled region is an interval of the layout.
    assert forall|i: int| 0 <= i < MPU_REGIONS && #[trigger] vp.regions[i].enabled implies vp.regions[i].interval.rules_ok() by {
        assert(vp.regions[i] == layout.region_spec(p, i));
        if 3 <= i < 3 + pl.window_count {
            assert(pl.windows[i - 3].rules_ok());
        }
    }
}

/// Property 12 over the Hardware_Model, pairwise (R16.1, R16.6): no address
/// that the MPU lets one Partition write is accessible to another Partition
/// in any way. Composes `lemma_mpu_grant_covered` with
/// `Layout::lemma_views_disjoint`.
pub proof fn lemma_hw_write_isolation<const NP: usize>(
    layout: &Layout<NP>, p: int, q: int, vp: &View, vq: &View, a: int, kind: u8,
)
    requires
        layout.wf(), 0 <= p < NP, 0 <= q < NP, p != q,
        layout.is_view_of(p, vp), layout.is_view_of(q, vq),
    ensures
        !(mpu_grants_unprivileged(vp, a, ACCESS_WRITE) && mpu_grants_unprivileged(vq, a, kind)),
{
    lemma_mpu_grant_covered(vp, a, ACCESS_WRITE);
    lemma_mpu_grant_covered(vq, a, kind);
    layout.lemma_views_disjoint(p, q, vp, vq, a);
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
