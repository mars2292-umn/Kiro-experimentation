//! The terms of the response-time analysis, derived inside `verus!` (R29.2
//! to R29.4, R29.8, R29.9; task 15.2, 2026-11-06). `inputs::prepare`
//! resolves the Task_Model into the plain values this module takes (each
//! Task's Priority, Partition, Budget, kind, and its Partition's
//! Overrun_Response; each Critical_Section site with its holder, the
//! Ceiling of its Resource, and its WCET; the Kernel_Timing_Parameters)
//! and calls `derive_all`, whose postcondition is the requirement:
//!
//! - `spec_o` is O_i of R29.3: Δ_release + Δ_dispatch + Δ_complete + Δ_wake
//!   per Job; Δ_timer per timed event of a Job (`timed_events`: the release
//!   of a Periodic_Task, the deadline, the Budget, the MIT unmask of a
//!   Sporadic_Task and its deferral under the `defer` policy, and the
//!   Budget event of a lower Job that blocks it, the one Budget event of
//!   another Job that can fire while it waits; 2026-11-14);
//!   Δ_lock_in + Δ_lock_out per Critical_Section site;
//!   Δ_hm; Δ_entry (R13.6); Δ_fp_save + Δ_fp_restore when the set has an
//!   FPU-using Task; Δ_irq + Δ_stamp for a Sporadic_Task (R19.2); and,
//!   in a Partition that stops overrunning Jobs, what the entry that ends
//!   the Task's own overrunning Job (Δ_detect + Δ_enforce) takes beyond
//!   Δ_timer + Δ_hm + Δ_complete (`overrun_extra`, 2026-11-14);
//! - `spec_e` is E_j of R29.4: Budget_j + Δ_detect + Δ_enforce + L_j + O_j
//!   for a Task of a Partition that stops overrunning Jobs, Budget_j + O_j
//!   otherwise, with L_j the longest Critical_Section of the Task
//!   (`longest_cs`);
//! - `spec_b` is B_i of R29.2: the maximum of L_kernel, the Partition stop
//!   and restart work of R14.10, and, over every site of a lower-Priority
//!   Task on a Resource whose Ceiling is at least P_i (`blocks`), the
//!   site's WCET, or Budget_l + Δ_detect + Δ_enforce for a site of another
//!   Partition that stops overrunning Jobs (`candidate`, R18.7); the site
//!   returned is one that attains the maximum, or none when the Kernel
//!   term is the maximum;
//! - J_i is J_rel for every Task (Table 13-1);
//! - `terms_for` lists the terms of a Task's recurrence (`spec_terms`):
//!   every other Task of hep(i) by its Jobs, ⌈(w + J_j) / T_j⌉ · E_j
//!   (R29.1); every lower-Priority Task by the Kernel-level work of its
//!   arrivals, ⌈(w + J_j) / T_j⌉ · Δ_irq for a Sporadic_Task's
//!   Release_Source and ⌈(w + J_j) / T_j⌉ · Δ_timer for a Periodic_Task's
//!   release event (R19.2: the handler runs at the Kernel level and
//!   preempts every Task, where the Task's own Jobs preempt only those at
//!   or below its Priority; for a Task of hep(i) the cost is in O_j
//!   already), and by its other timed events, ⌈(w + J_j) / T_j⌉ ·
//!   `lower_events` · Δ_timer (the unmask and deferral events of a
//!   Sporadic_Task, and the deadline event unless the same analysis shows
//!   the Task schedulable: the slot is disarmed when the Job ends, so it
//!   fires only on a miss; the events of one kind are at least T_j apart;
//!   a lower Task's Budget event cannot fire while its Job is preempted,
//!   the Kernel disarms it; 2026-11-14); and the Kernel's housekeeping
//!   event, ⌈(w + J_rel) / T_hk⌉ · Δ_timer, on every Task. Each of these
//!   terms carries the release jitter (2026-11-14, found by the proof of
//!   Property 18 in Lean): an event's Kernel entry comes up to J_rel after
//!   its instant when a Kernel section is in progress then, so an event
//!   from before a window of length w can have its entry inside it, and
//!   the entries in the window number up to ⌈(w + J_rel) / T⌉, not
//!   ⌈w / T⌉;
//! - `ticks_to_cycles_down` and `ticks_to_cycles_up` are the rounding of
//!   R29.8 (periods, MITs, and deadlines down; execution times and jitter
//!   up);
//! - `utilization_exceeds_one` is the check of R29.9 on a fixed scale:
//!   Σ ⌈E_j · S / T_j⌉ > S with S = 10^9, which `lemma_scaled_term_over`
//!   shows never falls below the exact term (a set with Σ E_j / T_j > 1
//!   over the rationals, `utilization_over_one`, is always failed:
//!   `lemma_utilization_over_one_is_failed`; a set within 10^-9 per Task of full utilization
//!   may be failed early too, which the fixed point would have failed or
//!   passed by at most that pessimism).
//!
//! Every sum is checked for `u64` overflow (R29.6): `derive_all` returns
//! `None` exactly when some term of some Task exceeds `u64::MAX`, and
//! `prepare` reports the error instead of a result.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use crate::rta::ceil_div;
use crate::rta::{Outcome, Term};

verus! {

/// One Task as the term derivation sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TaskIn {
    pub priority: u8,
    /// The index of the Task's Partition (two Tasks share a Partition when
    /// the values are equal).
    pub partition: u32,
    /// C_i: the Budget in cycles.
    pub budget: u64,
    /// The Partition's Overrun_Response stops overrunning Jobs.
    pub stops_overruns: bool,
    /// A Periodic_Task (otherwise a Sporadic_Task).
    pub periodic: bool,
    /// A Sporadic_Task with the `defer` MIT policy (a deferral event).
    pub defer: bool,
    /// Task 21.5 (R40.2): the Kernel time of the Task's Endpoint operations
    /// per Job, Σ over its roles of the declared operations × the larger of
    /// Δ_send and Δ_recv, charged in O_i.
    pub endpoint_cost: u64,
}

/// One Critical_Section site: the Task that holds it, the Ceiling of its
/// Resource, and the site's WCET including the sections nested in it (R8.4).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Site {
    pub task: usize,
    pub ceiling: u8,
    pub wcet: u64,
    /// Task 21.5 (R40.2): an Endpoint send or receive, a Critical_Section
    /// at the Kernel level inside the SVC: it blocks like any site, costs
    /// no lock entry and exit of its own (its cost is `endpoint_cost`), and
    /// cannot be overrun-ended (R40.6), so its blocking is its WCET.
    pub endpoint: bool,
}

/// The Kernel_Timing_Parameters of Table 13-1 the terms use, in cycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Params {
    pub release: u64,
    pub dispatch: u64,
    pub complete: u64,
    pub wake: u64,
    pub timer: u64,
    pub fp_save: u64,
    pub fp_restore: u64,
    pub lock_in: u64,
    pub lock_out: u64,
    pub detect: u64,
    pub enforce: u64,
    pub hm: u64,
    pub irq: u64,
    pub stamp: u64,
    pub entry: u64,
    pub l_kernel: u64,
    pub j_rel: u64,
    pub restart: u64,
    pub stop: u64,
    /// Task 21.5 (R40.2): Δ_send and Δ_recv, the Endpoint operations.
    pub send: u64,
    pub recv: u64,
    /// The period of the Kernel's housekeeping event (the watchdog feed),
    /// in cycles (`rsk_logic::logic::kernel::HOUSEKEEPING_PERIOD` ticks).
    pub hk_period: u64,
}

/// The terms of one Task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Derived {
    /// The number of the Task's Critical_Section sites.
    pub sites: u64,
    /// L_i: the longest of them.
    pub longest_cs: u64,
    /// O_i (R29.3).
    pub o: u64,
    /// E_i (R29.4).
    pub e: u64,
    /// B_i (R29.2).
    pub b: u64,
    /// The site that attains B_i, or `None` when the Kernel term does.
    pub b_site: Option<usize>,
    /// J_i: J_rel.
    pub j: u64,
}

pub open spec fn u64_max() -> int {
    0xffff_ffff_ffff_ffff
}

// ------------------------------------------------------------------ specs

/// The timed events of R9.4 a Job of the Task costs, each a timer entry at
/// the Kernel level (2026-11-14, counted from the Kernel's slots): for a
/// Periodic_Task its release, its deadline, its Budget, and the Budget
/// event of a lower Job that blocks it (the Kernel disarms a preempted
/// Job's Budget event, so the only Budget event of another Job that can
/// fire while this one waits or executes is that of the Job holding the
/// blocking Critical_Section); for a Sporadic_Task its deadline, its
/// Budget, its MIT unmask and that blocking Job's event, plus its MIT
/// deferral under the `defer` policy. Until then the count was 3 (periodic
/// or `defer`) or 2.
pub open spec fn timed_events(t: TaskIn) -> int {
    if t.periodic { 4 } else if t.defer { 5 } else { 4 }
}

/// The timed events of a lower-Priority Task's Job, beside its arrival,
/// that take a timer entry from a higher Job: the MIT unmask of a
/// Sporadic_Task and its deferral under `defer`, and the deadline event
/// when it can fire (`deadline_fires`: the Task is not shown schedulable).
/// The Budget event is not among them: the Kernel disarms it while the Job
/// is preempted.
pub open spec fn lower_events(t: TaskIn, deadline_fires: bool) -> int {
    (if t.periodic { 0int } else if t.defer { 2int } else { 1int }) + (if deadline_fires { 1int } else { 0int })
}

/// What the entry that ends the Task's own overrunning Job may take beyond
/// the terms of O_i it takes the place of (2026-11-14, found by the proof
/// of Property 18 in Lean). In a Partition whose Overrun_Response stops
/// overrunning Jobs, a Job that overruns has no Job end of its own: the
/// entry of its Budget event ends it, in up to Δ_detect + Δ_enforce
/// (R29.4), where O_i counts Δ_timer for the Budget event, Δ_hm for the
/// response and Δ_complete for the Job end. The excess, if any, is charged.
pub open spec fn overrun_extra(t: TaskIn, p: Params) -> int {
    let counted = p.timer as int + p.complete as int + p.hm as int;
    if t.stops_overruns && p.detect as int + p.enforce as int > counted {
        p.detect as int + p.enforce as int - counted
    } else {
        0
    }
}

/// O_i of R29.3 for a Task with `sites` Critical_Section sites.
pub open spec fn spec_o(t: TaskIn, p: Params, sites: int, any_fpu: bool) -> int {
    p.release as int + p.dispatch as int + p.complete as int + p.wake as int
        + p.timer as int * timed_events(t)
        + (p.lock_in as int + p.lock_out as int) * sites
        + p.hm as int + p.entry as int
        + (if any_fpu { p.fp_save as int + p.fp_restore as int } else { 0 })
        + (if t.periodic { 0 } else { p.irq as int + p.stamp as int })
        + t.endpoint_cost as int
        + overrun_extra(t, p)
}

/// E_j of R29.4.
pub open spec fn spec_e(t: TaskIn, p: Params, longest_cs: int, o: int) -> int {
    if t.stops_overruns {
        t.budget as int + p.detect as int + p.enforce as int + longest_cs + o
    } else {
        t.budget as int + o
    }
}

/// The number of Resource sites of Task `t` among the first `n` (the
/// Endpoint sites cost no lock entry and exit, task 21.5).
pub open spec fn site_count(sites: Seq<Site>, t: int, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { site_count(sites, t, n - 1) + (if sites[n - 1].task as int == t && !sites[n - 1].endpoint { 1int } else { 0int }) }
}

/// The longest site of Task `t` among the first `n` (0 without one).
pub open spec fn longest_cs(sites: Seq<Site>, t: int, n: int) -> int
    decreases n,
{
    if n <= 0 {
        0
    } else {
        let r = longest_cs(sites, t, n - 1);
        if sites[n - 1].task as int == t && sites[n - 1].wcet as int > r { sites[n - 1].wcet as int } else { r }
    }
}

/// Every site names a Task of the set.
pub open spec fn sites_ok(tasks: Seq<TaskIn>, sites: Seq<Site>) -> bool {
    forall|k: int| 0 <= k < sites.len() ==> (#[trigger] sites[k]).task < tasks.len()
}

/// Site `k` can block Task `i` (R29.2): its holder's Priority is lower than
/// P_i and its Resource's Ceiling is at least P_i.
pub open spec fn blocks(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, k: int) -> bool {
    tasks[sites[k].task as int].priority < tasks[i].priority && sites[k].ceiling >= tasks[i].priority
}

/// The blocking a site of a lower Task costs Task `i`: the site's WCET, or
/// Budget_l + Δ_detect + Δ_enforce for a Resource site of a Task of another
/// Partition that stops overrunning Jobs (R29.2, R18.7); an Endpoint site
/// is its WCET in every case (task 21.5: the operation runs to its end
/// inside the SVC, R40.6).
pub open spec fn candidate(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, k: int, p: Params) -> int {
    let l = tasks[sites[k].task as int];
    if !sites[k].endpoint && l.partition != tasks[i].partition && l.stops_overruns {
        l.budget as int + p.detect as int + p.enforce as int
    } else {
        sites[k].wcet as int
    }
}

/// The Kernel-level blocking: L_kernel or the Partition stop and restart
/// work of R14.10, whichever is larger.
pub open spec fn b_floor(p: Params) -> int {
    let work = if p.restart >= p.stop { p.restart as int } else { p.stop as int };
    if p.l_kernel as int >= work { p.l_kernel as int } else { work }
}

/// B_i over the first `n` sites.
pub open spec fn max_candidate(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, p: Params, n: int) -> int
    decreases n,
{
    if n <= 0 {
        b_floor(p)
    } else {
        let r = max_candidate(tasks, sites, i, p, n - 1);
        if blocks(tasks, sites, i, n - 1) && candidate(tasks, sites, i, n - 1, p) > r {
            candidate(tasks, sites, i, n - 1, p)
        } else {
            r
        }
    }
}

/// B_i of R29.2.
pub open spec fn spec_b(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, p: Params) -> int {
    max_candidate(tasks, sites, i, p, sites.len() as int)
}

/// The three terms of Task `i` fit `u64` (R29.6).
pub open spec fn terms_fit(tasks: Seq<TaskIn>, sites: Seq<Site>, p: Params, any_fpu: bool, i: int) -> bool {
    let o = spec_o(tasks[i], p, site_count(sites, i, sites.len() as int), any_fpu);
    o <= u64_max()
        && spec_e(tasks[i], p, longest_cs(sites, i, sites.len() as int), o) <= u64_max()
        && spec_b(tasks, sites, i, p) <= u64_max()
}

/// `d` holds the terms of Task `i` (the postcondition of `derive_all`).
pub open spec fn derived_ok(tasks: Seq<TaskIn>, sites: Seq<Site>, p: Params, any_fpu: bool, i: int, d: Derived) -> bool {
    let n = sites.len() as int;
    &&& d.sites as int == site_count(sites, i, n)
    &&& d.longest_cs as int == longest_cs(sites, i, n)
    &&& d.o as int == spec_o(tasks[i], p, d.sites as int, any_fpu)
    &&& d.e as int == spec_e(tasks[i], p, d.longest_cs as int, d.o as int)
    &&& d.b as int == spec_b(tasks, sites, i, p)
    &&& (d.b_site matches Some(k) ==> k < sites.len() && blocks(tasks, sites, i, k as int) && candidate(tasks, sites, i, k as int, p) == d.b as int)
    &&& (d.b_site.is_none() ==> d.b as int == b_floor(p))
    &&& d.j == p.j_rel
}

// ----------------------------------------------------------------- lemmas

/// B_i over a longer prefix is at least B_i over a shorter one.
proof fn lemma_max_candidate_monotone(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, p: Params, m: int, n: int)
    requires 0 <= m <= n,
    ensures max_candidate(tasks, sites, i, p, m) <= max_candidate(tasks, sites, i, p, n),
    decreases n - m,
{
    if m < n {
        lemma_max_candidate_monotone(tasks, sites, i, p, m, n - 1);
    }
}

/// B_i over a prefix is at least the Kernel term.
proof fn lemma_max_candidate_floor(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, p: Params, n: int)
    requires n >= 0,
    ensures max_candidate(tasks, sites, i, p, n) >= b_floor(p),
{
    lemma_max_candidate_monotone(tasks, sites, i, p, 0, n);
}

/// `⌈a / t⌉ · t ≥ a`: the scaled term of R29.9 never falls below the exact
/// one (the soundness of the early FAIL).
pub proof fn lemma_scaled_term_over(a: int, t: int)
    requires a >= 0, t > 0,
    ensures ceil_div(a, t) * t >= a,
{
    assert(((a + t - 1) / t) * t >= a) by (nonlinear_arith)
        requires a >= 0, t > 0;
}

/// `⌈a / t⌉ ≤ a` for `t ≥ 1`.
proof fn lemma_ceil_div_le(a: int, t: int)
    requires a >= 0, t > 0,
    ensures ceil_div(a, t) <= a,
{
    assert((a + t - 1) / t <= a) by (nonlinear_arith)
        requires a >= 0, t > 0;
}

// ------------------------------------------------------------------- exec

/// `a + b` in `u64`, `None` on overflow.
fn add(a: u64, b: u64) -> (r: Option<u64>)
    ensures
        r.is_some() <==> a as int + b as int <= u64_max(),
        r matches Some(v) ==> v as int == a as int + b as int,
{
    if a > u64::MAX - b { None } else { Some(a + b) }
}

/// `a · b` in `u64`, `None` on overflow.
fn mul(a: u64, b: u64) -> (r: Option<u64>)
    ensures
        r.is_some() <==> a as int * b as int <= u64_max(),
        r matches Some(v) ==> v as int == a as int * b as int,
{
    assert((a as int) * (b as int) < 0x1_0000_0000_0000_0000_0000_0000_0000_0000) by (nonlinear_arith)
        requires a < 0x1_0000_0000_0000_0000, b < 0x1_0000_0000_0000_0000;
    assert((a as int) * (b as int) >= 0) by (nonlinear_arith) requires a >= 0, b >= 0;
    let product: u128 = (a as u128) * (b as u128);
    if product > u64::MAX as u128 { None } else { Some(product as u64) }
}

/// The number and the longest of Task `t`'s sites.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SiteStats {
    pub count: u64,
    pub longest: u64,
}

pub fn site_stats(sites: &Vec<Site>, t: usize) -> (r: SiteStats)
    ensures
        r.count as int == site_count(sites@, t as int, sites.len() as int),
        r.longest as int == longest_cs(sites@, t as int, sites.len() as int),
{
    let mut count: u64 = 0;
    let mut longest: u64 = 0;
    let mut k: usize = 0;
    while k < sites.len()
        invariant
            k <= sites.len(),
            count as int == site_count(sites@, t as int, k as int),
            longest as int == longest_cs(sites@, t as int, k as int),
            count <= k,
        decreases sites.len() - k,
    {
        let s = sites[k];
        if s.task == t {
            if !s.endpoint {
                count = count + 1;
            }
            if s.wcet > longest {
                longest = s.wcet;
            }
        }
        k = k + 1;
    }
    SiteStats { count, longest }
}

/// `overrun_extra`, given `counted` = Δ_timer + Δ_complete + Δ_hm; `None`
/// when it exceeds `u64`.
fn overrun_extra_of(t: TaskIn, p: Params, counted: u64) -> (r: Option<u64>)
    requires
        counted as int == p.timer as int + p.complete as int + p.hm as int,
    ensures
        r.is_some() <==> overrun_extra(t, p) <= u64_max(),
        r matches Some(v) ==> v as int == overrun_extra(t, p),
{
    if !t.stops_overruns {
        return Some(0);
    }
    if p.detect >= counted {
        add(p.detect - counted, p.enforce)
    } else if p.enforce > counted - p.detect {
        Some(p.enforce - (counted - p.detect))
    } else {
        Some(0)
    }
}

/// O_i (R29.3) for a Task with `sites` Critical_Section sites; `None` when
/// it exceeds `u64` (R29.6).
pub fn overhead(t: TaskIn, p: Params, sites: u64, any_fpu: bool) -> (r: Option<u64>)
    ensures
        r.is_some() <==> spec_o(t, p, sites as int, any_fpu) <= u64_max(),
        r matches Some(v) ==> v as int == spec_o(t, p, sites as int, any_fpu),
{
    let events: u64 = if t.periodic { 4 } else if t.defer { 5 } else { 4 };
    // Each product as a value, so that the remaining reasoning is linear;
    // a term the Task does not carry is not summed (its parameters may be
    // anything).
    let timed = match mul(p.timer, events) { Some(v) => v, None => return None };
    let locks: u64 = if sites == 0 {
        assert((p.lock_in as int + p.lock_out as int) * sites as int == 0) by (nonlinear_arith) requires sites == 0;
        0
    } else {
        let lock_total = match add(p.lock_in, p.lock_out) {
            Some(v) => v,
            None => {
                assert((p.lock_in as int + p.lock_out as int) * sites as int >= p.lock_in as int + p.lock_out as int) by (nonlinear_arith)
                    requires sites >= 1, p.lock_in as int + p.lock_out as int >= 0;
                return None;
            }
        };
        match mul(lock_total, sites) { Some(v) => v, None => return None }
    };
    assert(locks as int == (p.lock_in as int + p.lock_out as int) * sites as int);
    let fp_cost: u64 = if any_fpu {
        match add(p.fp_save, p.fp_restore) { Some(v) => v, None => return None }
    } else {
        0
    };
    let irq_cost: u64 = if t.periodic {
        0
    } else {
        match add(p.irq, p.stamp) { Some(v) => v, None => return None }
    };
    let s1 = match add(p.release, p.dispatch) { Some(v) => v, None => return None };
    let s2 = match add(s1, p.complete) { Some(v) => v, None => return None };
    let s3 = match add(s2, p.wake) { Some(v) => v, None => return None };
    let s4 = match add(s3, timed) { Some(v) => v, None => return None };
    let s5 = match add(s4, locks) { Some(v) => v, None => return None };
    let s6 = match add(s5, p.hm) { Some(v) => v, None => return None };
    let s7 = match add(s6, p.entry) { Some(v) => v, None => return None };
    let s8 = match add(s7, fp_cost) { Some(v) => v, None => return None };
    let s9 = match add(s8, irq_cost) { Some(v) => v, None => return None };
    let s10 = match add(s9, t.endpoint_cost) { Some(v) => v, None => return None };
    // The entry that ends the Task's own overrunning Job (`overrun_extra`).
    // Δ_timer + Δ_complete + Δ_hm is within the sums above when they fit.
    assert(timed as int >= p.timer as int) by (nonlinear_arith)
        requires timed as int == p.timer as int * events as int, events >= 4;
    let tc = match add(p.timer, p.complete) { Some(v) => v, None => return None };
    let counted = match add(tc, p.hm) { Some(v) => v, None => return None };
    let extra = match overrun_extra_of(t, p, counted) { Some(v) => v, None => return None };
    let s11 = match add(s10, extra) { Some(v) => v, None => return None };
    Some(s11)
}

/// The sum of a sequence of terms (task 21.5, R41.4: an end-to-end bound).
pub open spec fn seq_sum(xs: Seq<u64>, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { seq_sum(xs, n - 1) + xs[n - 1] as int }
}

/// The checked sum of `xs` (R29.6: `None` when it exceeds `u64`).
pub fn checked_sum(xs: &Vec<u64>) -> (r: Option<u64>)
    ensures
        r.is_some() <==> seq_sum(xs@, xs.len() as int) <= u64_max(),
        r matches Some(v) ==> v as int == seq_sum(xs@, xs.len() as int),
{
    let mut sum: u64 = 0;
    let mut k: usize = 0;
    while k < xs.len()
        invariant
            k <= xs.len(),
            sum as int == seq_sum(xs@, k as int),
        decreases xs.len() - k,
    {
        sum = match add(sum, xs[k]) {
            Some(v) => v,
            None => {
                proof {
                    lemma_seq_sum_monotone(xs@, k as int + 1, xs.len() as int);
                }
                return None;
            }
        };
        k = k + 1;
    }
    Some(sum)
}

proof fn lemma_seq_sum_monotone(xs: Seq<u64>, m: int, n: int)
    requires 0 <= m <= n,
    ensures seq_sum(xs, m) <= seq_sum(xs, n),
    decreases n - m,
{
    if m < n {
        lemma_seq_sum_monotone(xs, m, n - 1);
    }
}

/// E_i (R29.4) from the Budget, L_i, and O_i; `None` when it exceeds `u64`.
pub fn interference_bound(t: TaskIn, p: Params, longest_cs: u64, o: u64) -> (r: Option<u64>)
    ensures
        r.is_some() <==> spec_e(t, p, longest_cs as int, o as int) <= u64_max(),
        r matches Some(v) ==> v as int == spec_e(t, p, longest_cs as int, o as int),
{
    if t.stops_overruns {
        let s1 = match add(t.budget, p.detect) { Some(v) => v, None => return None };
        let s2 = match add(s1, p.enforce) { Some(v) => v, None => return None };
        let s3 = match add(s2, longest_cs) { Some(v) => v, None => return None };
        add(s3, o)
    } else {
        add(t.budget, o)
    }
}

/// B_i (R29.2) and the site that attains it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Blocking {
    pub b: u64,
    pub site: Option<usize>,
}

pub fn blocking(tasks: &Vec<TaskIn>, sites: &Vec<Site>, i: usize, p: Params) -> (r: Option<Blocking>)
    requires
        i < tasks.len(),
        sites_ok(tasks@, sites@),
    ensures
        r.is_some() <==> spec_b(tasks@, sites@, i as int, p) <= u64_max(),
        r matches Some(bl) ==> {
            &&& bl.b as int == spec_b(tasks@, sites@, i as int, p)
            &&& (bl.site matches Some(k) ==> k < sites.len() && blocks(tasks@, sites@, i as int, k as int) && candidate(tasks@, sites@, i as int, k as int, p) == bl.b as int)
            &&& (bl.site.is_none() ==> bl.b as int == b_floor(p))
        },
{
    let ti = tasks[i];
    let work = if p.restart >= p.stop { p.restart } else { p.stop };
    let mut b: u64 = if p.l_kernel >= work { p.l_kernel } else { work };
    let mut site: Option<usize> = None;
    let mut k: usize = 0;
    while k < sites.len()
        invariant
            k <= sites.len(),
            i < tasks.len(),
            sites_ok(tasks@, sites@),
            ti == tasks@[i as int],
            b as int == max_candidate(tasks@, sites@, i as int, p, k as int),
            site matches Some(s) ==> s < k && blocks(tasks@, sites@, i as int, s as int) && candidate(tasks@, sites@, i as int, s as int, p) == b as int,
            site.is_none() ==> b as int == b_floor(p),
        decreases sites.len() - k,
    {
        let s = sites[k];
        let l = tasks[s.task];
        if l.priority < ti.priority && s.ceiling >= ti.priority {
            let c: u64 = if !s.endpoint && l.partition != ti.partition && l.stops_overruns {
                let s1 = match add(l.budget, p.detect) {
                    Some(v) => v,
                    None => {
                        proof {
                            lemma_max_candidate_monotone(tasks@, sites@, i as int, p, k as int + 1, sites.len() as int);
                        }
                        return None;
                    }
                };
                match add(s1, p.enforce) {
                    Some(v) => v,
                    None => {
                        proof {
                            lemma_max_candidate_monotone(tasks@, sites@, i as int, p, k as int + 1, sites.len() as int);
                        }
                        return None;
                    }
                }
            } else {
                s.wcet
            };
            assert(c as int == candidate(tasks@, sites@, i as int, k as int, p));
            if c > b {
                b = c;
                site = Some(k);
            }
        }
        k = k + 1;
    }
    Some(Blocking { b, site })
}

/// The terms of every Task (R29.2 to R29.4); `None` exactly when a term of
/// some Task exceeds `u64` (R29.6).
pub fn derive_all(tasks: &Vec<TaskIn>, sites: &Vec<Site>, p: Params, any_fpu: bool) -> (r: Option<Vec<Derived>>)
    requires sites_ok(tasks@, sites@),
    ensures
        r matches Some(out) ==> out.len() == tasks.len()
            && forall|i: int| 0 <= i < tasks.len() ==> derived_ok(tasks@, sites@, p, any_fpu, i, #[trigger] out@[i]),
        r.is_none() ==> exists|i: int| 0 <= i < tasks.len() && !terms_fit(tasks@, sites@, p, any_fpu, i),
{
    let mut out: Vec<Derived> = Vec::new();
    let mut i: usize = 0;
    while i < tasks.len()
        invariant
            i <= tasks.len(),
            sites_ok(tasks@, sites@),
            out.len() == i,
            forall|j: int| 0 <= j < i ==> derived_ok(tasks@, sites@, p, any_fpu, j, #[trigger] out@[j]),
        decreases tasks.len() - i,
    {
        let t = tasks[i];
        let stats = site_stats(sites, i);
        let o = match overhead(t, p, stats.count, any_fpu) {
            Some(v) => v,
            None => {
                assert(!terms_fit(tasks@, sites@, p, any_fpu, i as int));
                return None;
            }
        };
        let e = match interference_bound(t, p, stats.longest, o) {
            Some(v) => v,
            None => {
                assert(!terms_fit(tasks@, sites@, p, any_fpu, i as int));
                return None;
            }
        };
        let bl = match blocking(tasks, sites, i, p) {
            Some(v) => v,
            None => {
                assert(!terms_fit(tasks@, sites@, p, any_fpu, i as int));
                return None;
            }
        };
        let d = Derived { sites: stats.count, longest_cs: stats.longest, o, e, b: bl.b, b_site: bl.site, j: p.j_rel };
        assert(derived_ok(tasks@, sites@, p, any_fpu, i as int, d));
        out.push(d);
        i = i + 1;
    }
    Some(out)
}

/// Ticks to cycles rounded down (R29.8: periods, MITs, deadlines); `None`
/// when the value exceeds `u64` (R29.6).
pub fn ticks_to_cycles_down(ticks: u64, tick_hz: u64, cpu_hz: u64) -> (r: Option<u64>)
    requires tick_hz > 0,
    ensures
        r.is_some() <==> (ticks as int * cpu_hz as int) / (tick_hz as int) <= u64_max(),
        r matches Some(v) ==> v as int == (ticks as int * cpu_hz as int) / (tick_hz as int),
{
    assert((ticks as int) * (cpu_hz as int) < 0x1_0000_0000_0000_0000_0000_0000_0000_0000) by (nonlinear_arith)
        requires ticks < 0x1_0000_0000_0000_0000, cpu_hz < 0x1_0000_0000_0000_0000;
    assert((ticks as int) * (cpu_hz as int) >= 0) by (nonlinear_arith) requires ticks >= 0, cpu_hz >= 0;
    let product: u128 = (ticks as u128) * (cpu_hz as u128);
    let q: u128 = product / (tick_hz as u128);
    if q > u64::MAX as u128 { None } else { Some(q as u64) }
}

/// Ticks to cycles rounded up (R29.8: execution times and jitter).
pub fn ticks_to_cycles_up(ticks: u64, tick_hz: u64, cpu_hz: u64) -> (r: Option<u64>)
    requires tick_hz > 0,
    ensures
        r.is_some() <==> ceil_div(ticks as int * cpu_hz as int, tick_hz as int) <= u64_max(),
        r matches Some(v) ==> v as int == ceil_div(ticks as int * cpu_hz as int, tick_hz as int),
{
    // (2^64 - 1)^2 + 2^64 - 2 < 2^128.
    assert((ticks as int) * (cpu_hz as int) + (tick_hz as int) - 1 < 0x1_0000_0000_0000_0000_0000_0000_0000_0000) by (nonlinear_arith)
        requires ticks < 0x1_0000_0000_0000_0000, cpu_hz < 0x1_0000_0000_0000_0000, tick_hz < 0x1_0000_0000_0000_0000;
    assert((ticks as int) * (cpu_hz as int) >= 0) by (nonlinear_arith) requires ticks >= 0, cpu_hz >= 0;
    let product: u128 = (ticks as u128) * (cpu_hz as u128);
    let q: u128 = (product + (tick_hz as u128 - 1)) / (tick_hz as u128);
    if q > u64::MAX as u128 { None } else { Some(q as u64) }
}

/// The load of one Task for R29.9: E_j and T_j in cycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Load {
    pub e: u64,
    pub period: u64,
}

/// The scale of the utilization check.
pub const SCALE: u64 = 1_000_000_000;

pub open spec fn loads_ok(loads: Seq<Load>) -> bool {
    forall|k: int| 0 <= k < loads.len() ==> (#[trigger] loads[k]).period > 0
}

/// Σ ⌈E_j · S / T_j⌉ over the first `n` loads.
pub open spec fn scaled_sum(loads: Seq<Load>, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { scaled_sum(loads, n - 1) + ceil_div(loads[n - 1].e as int * SCALE as int, loads[n - 1].period as int) }
}

/// The denominator of Σ E_j / T_j over the first `n` loads: Π T_j.
pub open spec fn util_den(loads: Seq<Load>, n: int) -> int
    decreases n,
{
    if n <= 0 { 1 } else { util_den(loads, n - 1) * loads[n - 1].period as int }
}

/// Its numerator over that denominator: Σ E_j · Π_{k ≠ j} T_k.
pub open spec fn util_num(loads: Seq<Load>, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { util_num(loads, n - 1) * loads[n - 1].period as int + loads[n - 1].e as int * util_den(loads, n - 1) }
}

/// Σ E_j / T_j > 1 over the rationals, as the inequality of the sum's
/// numerator and denominator (the denominator is positive).
pub open spec fn utilization_over_one(loads: Seq<Load>) -> bool {
    util_num(loads, loads.len() as int) > util_den(loads, loads.len() as int)
}

proof fn lemma_util_den_pos(loads: Seq<Load>, n: int)
    requires loads_ok(loads), 0 <= n <= loads.len(),
    ensures util_den(loads, n) > 0,
    decreases n,
{
    if n > 0 {
        lemma_util_den_pos(loads, n - 1);
        let d = util_den(loads, n - 1);
        let t = loads[n - 1].period as int;
        assert(d * t > 0) by (nonlinear_arith) requires d > 0, t > 0;
    }
}

/// The scaled sum is at or above the exact one: Σ ⌈E_j · S / T_j⌉ · Π T ≥
/// S · Σ E_j · Π_{k ≠ j} T_k.
proof fn lemma_scaled_sum_over(loads: Seq<Load>, n: int)
    requires loads_ok(loads), 0 <= n <= loads.len(),
    ensures scaled_sum(loads, n) * util_den(loads, n) >= SCALE as int * util_num(loads, n),
    decreases n,
{
    if n > 0 {
        lemma_scaled_sum_over(loads, n - 1);
        lemma_util_den_pos(loads, n - 1);
        let ss = scaled_sum(loads, n - 1);
        let d = util_den(loads, n - 1);
        let m = util_num(loads, n - 1);
        let t = loads[n - 1].period as int;
        let e = loads[n - 1].e as int;
        let sc = SCALE as int;
        let c = ceil_div(e * sc, t);
        assert(e * sc >= 0) by (nonlinear_arith) requires e >= 0, sc >= 0;
        lemma_scaled_term_over(e * sc, t);
        assert((ss + c) * (d * t) >= sc * (m * t + e * d)) by (nonlinear_arith)
            requires ss * d >= sc * m, c * t >= e * sc, d > 0, t > 0;
    }
}

/// R29.9 over the rationals: a Task set with Σ E_j / T_j > 1 is failed by
/// the scaled check (`utilization_exceeds_one` returns `true` for it).
pub proof fn lemma_utilization_over_one_is_failed(loads: Seq<Load>)
    requires loads_ok(loads), utilization_over_one(loads),
    ensures scaled_sum(loads, loads.len() as int) > SCALE as int,
{
    let n = loads.len() as int;
    lemma_scaled_sum_over(loads, n);
    lemma_util_den_pos(loads, n);
    let ss = scaled_sum(loads, n);
    let d = util_den(loads, n);
    let m = util_num(loads, n);
    let sc = SCALE as int;
    assert(ss > sc) by (nonlinear_arith)
        requires ss * d >= sc * m, m > d, d > 0, sc > 0;
}

/// R29.9: Σ E_j / T_j > 1 on the fixed scale, exact in `u128`.
pub fn utilization_exceeds_one(loads: &Vec<Load>) -> (r: bool)
    requires
        loads_ok(loads@),
        loads.len() < 0x1_0000_0000,
    ensures
        r == (scaled_sum(loads@, loads.len() as int) > SCALE as int),
        utilization_over_one(loads@) ==> r,
{
    proof {
        if utilization_over_one(loads@) {
            lemma_utilization_over_one_is_failed(loads@);
        }
    }
    // Each term is below 2^94: E < 2^64 and S < 2^30.
    let mut sum: u128 = 0;
    let mut k: usize = 0;
    while k < loads.len()
        invariant
            k <= loads.len(),
            loads_ok(loads@),
            loads.len() < 0x1_0000_0000,
            sum as int == scaled_sum(loads@, k as int),
            sum as int <= k as int * 0x4000_0000_0000_0000_0000_0000,
        decreases loads.len() - k,
    {
        let l = loads[k];
        assert(l.period > 0);
        assert((l.e as int) * (SCALE as int) < 0x4000_0000_0000_0000_0000_0000) by (nonlinear_arith)
            requires l.e < 0x1_0000_0000_0000_0000, SCALE < 0x4000_0000;
        assert((l.e as int) * (SCALE as int) >= 0) by (nonlinear_arith) requires l.e >= 0, SCALE >= 0;
        let scaled: u128 = (l.e as u128) * (SCALE as u128);
        let term: u128 = (scaled + (l.period as u128 - 1)) / (l.period as u128);
        proof {
            lemma_ceil_div_le((l.e as int) * (SCALE as int), l.period as int);
        }
        assert(term as int == ceil_div(l.e as int * SCALE as int, l.period as int));
        sum = sum + term;
        k = k + 1;
    }
    sum > SCALE as u128
}

/// One Task as an interferer of another's recurrence: its Priority, J_j,
/// T_j (period or MIT), E_j, the Kernel-level cost of one arrival (`intake`:
/// Δ_irq for a Sporadic_Task's Release_Source, Δ_timer for a
/// Periodic_Task's release event; R19.2) and the Kernel-level cost of a
/// Job's other timed events (`events`: `lower_events` · Δ_timer).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Interferer {
    pub priority: u8,
    pub jitter: u64,
    pub period: u64,
    pub bound: u64,
    pub intake: u64,
    pub events: u64,
}

/// The origin of a term: the Jobs of a Task (R29.1).
pub const TERM_JOBS: u8 = 0;
/// The Kernel-level work of a lower-Priority Task's arrivals (R19.2).
pub const TERM_INTAKE: u8 = 1;
/// The other timed events of a lower-Priority Task's Jobs (R19.2).
pub const TERM_EVENTS: u8 = 2;
/// The Kernel's housekeeping event (`task` is then the number of Tasks).
pub const TERM_HOUSEKEEPING: u8 = 3;

/// One term of a Task's recurrence with its origin (`kind`, and the Task
/// it comes from).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TermOf {
    pub task: usize,
    pub kind: u8,
    pub term: Term,
}

pub open spec fn periods_ok(tasks: Seq<Interferer>) -> bool {
    forall|k: int| 0 <= k < tasks.len() ==> (#[trigger] tasks[k]).period > 0
}

/// The interference of Task `j`'s Jobs: ⌈(w + J_j) / T_j⌉ · E_j (R29.1).
pub open spec fn term_of_jobs(j: int, t: Interferer) -> TermOf {
    TermOf { task: j as usize, kind: TERM_JOBS, term: Term { jitter: t.jitter, period: t.period, bound: t.bound } }
}

/// The Kernel-level work of Task `j`'s arrivals: ⌈(w + J_j) / T_j⌉ ·
/// `intake` (R19.2), on a Task its Jobs cannot preempt. The jitter (since
/// 2026-11-14): the Kernel entry of an arrival comes up to J_rel after its
/// instant, when a Kernel section is in progress at that instant, so an
/// arrival from before the analysed interval can have its entry inside it.
pub open spec fn term_of_intake(j: int, t: Interferer) -> TermOf {
    TermOf { task: j as usize, kind: TERM_INTAKE, term: Term { jitter: t.jitter, period: t.period, bound: t.intake } }
}

/// The other timed events of Task `j`'s Jobs: ⌈(w + J_j) / T_j⌉ · `events`
/// (the events of one kind are at least T_j apart, and each entry comes up
/// to J_rel after its instant).
pub open spec fn term_of_events(j: int, t: Interferer) -> TermOf {
    TermOf { task: j as usize, kind: TERM_EVENTS, term: Term { jitter: t.jitter, period: t.period, bound: t.events } }
}

/// The housekeeping event's term, for a set of `n` Tasks.
pub open spec fn term_of_hk(n: int, hk: Term) -> TermOf {
    TermOf { task: n as usize, kind: TERM_HOUSEKEEPING, term: hk }
}

/// The terms a lower-Priority Task `j` adds: its arrivals' and its other
/// timed events', each when it has a cost.
pub open spec fn push_lower(prev: Seq<TermOf>, j: int, t: Interferer) -> Seq<TermOf> {
    let a = if t.intake > 0 { prev.push(term_of_intake(j, t)) } else { prev };
    if t.events > 0 { a.push(term_of_events(j, t)) } else { a }
}

/// The terms of Task `i`'s recurrence over the first `n` Tasks: every Task
/// of hep(i) other than `i` by its Jobs (R29.1), and every lower-Priority
/// Task by its Kernel-level work (R19.2, `push_lower`).
pub open spec fn spec_terms(tasks: Seq<Interferer>, i: int, n: int) -> Seq<TermOf>
    decreases n,
{
    if n <= 0 {
        Seq::empty()
    } else {
        let prev = spec_terms(tasks, i, n - 1);
        let j = n - 1;
        if j == i {
            prev
        } else if tasks[j].priority >= tasks[i].priority {
            prev.push(term_of_jobs(j, tasks[j]))
        } else {
            push_lower(prev, j, tasks[j])
        }
    }
}

/// The terms of Task `i`'s recurrence: those of the other Tasks, then the
/// housekeeping event's.
pub open spec fn spec_all_terms(tasks: Seq<Interferer>, i: int, hk: Term) -> Seq<TermOf> {
    spec_terms(tasks, i, tasks.len() as int).push(term_of_hk(tasks.len() as int, hk))
}

/// The terms of Task `i`'s recurrence (R29.1 with R19.2), each with its
/// origin, in Task order; every period positive (`rta::terms_ok`).
pub fn terms_for(tasks: &Vec<Interferer>, i: usize, hk: Term) -> (r: Vec<TermOf>)
    requires
        i < tasks.len(),
        periods_ok(tasks@),
        hk.period > 0,
    ensures
        r@ == spec_all_terms(tasks@, i as int, hk),
        forall|k: int| 0 <= k < r.len() ==> (#[trigger] r@[k]).term.period > 0,
{
    let mut out: Vec<TermOf> = Vec::new();
    let ti = tasks[i];
    let mut j: usize = 0;
    while j < tasks.len()
        invariant
            j <= tasks.len(),
            i < tasks.len(),
            periods_ok(tasks@),
            ti == tasks@[i as int],
            out@ == spec_terms(tasks@, i as int, j as int),
            forall|k: int| 0 <= k < out.len() ==> (#[trigger] out@[k]).term.period > 0,
        decreases tasks.len() - j,
    {
        let t = tasks[j];
        if j != i {
            if t.priority >= ti.priority {
                out.push(TermOf { task: j, kind: TERM_JOBS, term: Term { jitter: t.jitter, period: t.period, bound: t.bound } });
            } else {
                if t.intake > 0 {
                    out.push(TermOf { task: j, kind: TERM_INTAKE, term: Term { jitter: t.jitter, period: t.period, bound: t.intake } });
                }
                if t.events > 0 {
                    out.push(TermOf { task: j, kind: TERM_EVENTS, term: Term { jitter: t.jitter, period: t.period, bound: t.events } });
                }
            }
        }
        assert(out@ =~= spec_terms(tasks@, i as int, j as int + 1));
        j = j + 1;
    }
    out.push(TermOf { task: tasks.len(), kind: TERM_HOUSEKEEPING, term: hk });
    assert(out@ =~= spec_all_terms(tasks@, i as int, hk));
    out
}

/// The plain terms of a recurrence: the terms without their origins.
pub open spec fn plain(ts: Seq<TermOf>) -> Seq<Term> {
    Seq::new(ts.len(), |k: int| ts[k].term)
}

/// The verdict of the analysis for one Task (R29.5, R29.7): `w` is the
/// fixed point or the iterate that passed the deadline, `r = w + J` its
/// response time (saturated when the Task is not schedulable).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Verdict {
    pub w: u64,
    pub r: u64,
    pub schedulable: bool,
    pub overflow: bool,
}

/// The recurrence of Task `i` has a fixed point within the deadline.
pub open spec fn meets_deadline(base: int, ts: Seq<Term>, jitter: int, deadline: int) -> bool {
    exists|x: int| x >= 0 && #[trigger] crate::rta::is_fixpoint(base, ts, x) && jitter + x <= deadline
}

/// The analysis of one Task (R29.1 with R19.2, R29.5 to R29.7), composed:
/// the terms of Task `i` (`terms_for`), its recurrence from
/// `base = C + B + O` (`rta::fixpoint`), and the verdict. Until 2026-11-14
/// this composition was plain Rust in `response_times`, checked against a
/// reference implementation only.
///
/// A Task is reported schedulable exactly with its response time
/// `R = J + w` for the least fixed point `w`, and `R <= D`; a Task reported
/// not schedulable without an overflow has no fixed point within its
/// deadline; an overflow claims nothing (R29.6).
pub fn response_of(tasks: &Vec<Interferer>, i: usize, c: u64, b: u64, o: u64, deadline: u64, hk: Term) -> (v: (Verdict, Vec<TermOf>))
    requires
        i < tasks.len(),
        periods_ok(tasks@),
        hk.period > 0,
    ensures
        v.1@ == spec_all_terms(tasks@, i as int, hk),
        v.0.overflow ==> !v.0.schedulable,
        v.0.schedulable ==> c + b + o <= u64_max()
            && crate::rta::is_fixpoint(c + b + o, plain(v.1@), v.0.w as int)
            && (forall|x: int| x >= 0 && #[trigger] crate::rta::is_fixpoint(c + b + o, plain(v.1@), x) ==> v.0.w as int <= x)
            && v.0.r as int == tasks@[i as int].jitter as int + v.0.w as int
            && v.0.r <= deadline,
        !v.0.schedulable && !v.0.overflow
            ==> !meets_deadline(c + b + o, plain(v.1@), tasks@[i as int].jitter as int, deadline as int),
{
    let own_terms = terms_for(tasks, i, hk);
    let jitter = tasks[i].jitter;
    let mut plain_terms: Vec<Term> = Vec::new();
    let mut k: usize = 0;
    while k < own_terms.len()
        invariant
            k <= own_terms.len(),
            plain_terms.len() == k,
            forall|m: int| 0 <= m < k ==> (#[trigger] plain_terms@[m]) == own_terms@[m].term,
            forall|m: int| 0 <= m < own_terms.len() ==> (#[trigger] own_terms@[m]).term.period > 0,
        decreases own_terms.len() - k,
    {
        plain_terms.push(own_terms[k].term);
        k = k + 1;
    }
    assert(plain_terms@ =~= plain(own_terms@));
    assert(crate::rta::terms_ok(plain_terms@)) by {
        assert forall|m: int| 0 <= m < plain_terms@.len() implies (#[trigger] plain_terms@[m]).period > 0 by {
            assert(own_terms@[m].term.period > 0);
        }
    }
    let saturated = Verdict { w: u64::MAX, r: u64::MAX, schedulable: false, overflow: true };
    let cb = match add(c, b) {
        Some(x) => x,
        None => return (saturated, own_terms),
    };
    let base = match add(cb, o) {
        Some(x) => x,
        None => return (saturated, own_terms),
    };
    match crate::rta::fixpoint(base, &plain_terms, jitter, deadline) {
        Outcome::Fixpoint(w) => (Verdict { w, r: w + jitter, schedulable: true, overflow: false }, own_terms),
        Outcome::NotSchedulable(w) => {
            let r = if w > u64::MAX - jitter { u64::MAX } else { w + jitter };
            proof {
                // Every fixed point is at least `w`, which is already late.
                assert forall|x: int| x >= 0 && #[trigger] crate::rta::is_fixpoint(base as int, plain_terms@, x)
                    implies !(jitter as int + x <= deadline as int) by {}
            }
            (Verdict { w, r, schedulable: false, overflow: false }, own_terms)
        }
        Outcome::Overflow => (saturated, own_terms),
    }
}

/// A Task's period or MIT and its deadline, in cycles (R29.8).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Window {
    pub period: u64,
    pub deadline: u64,
}

/// Task `j` as an interferer of the others' recurrences, from the Task set
/// and its sites: its Priority, `J_rel`, its period (at least 1), `E_j`
/// (R29.4) and the intake cost of its Release_Source (R19.2).
///
/// `quiet`: the analysis shows Task `j` schedulable, so its deadline event
/// does not fire and is not charged to the Tasks above it.
pub open spec fn spec_interferer(tasks: Seq<TaskIn>, sites: Seq<Site>, p: Params, any_fpu: bool, windows: Seq<Window>, quiet: Seq<bool>, j: int) -> Interferer {
    let n = sites.len() as int;
    let o = spec_o(tasks[j], p, site_count(sites, j, n), any_fpu);
    Interferer {
        priority: tasks[j].priority,
        jitter: p.j_rel,
        period: if windows[j].period == 0 { 1 } else { windows[j].period },
        bound: spec_e(tasks[j], p, longest_cs(sites, j, n), o) as u64,
        intake: if tasks[j].periodic { p.timer } else { p.irq },
        events: (p.timer as int * lower_events(tasks[j], !quiet[j])) as u64,
    }
}

/// The housekeeping event's term: ⌈(w + J_rel) / T_hk⌉ · Δ_timer (its entry
/// comes up to J_rel after its instant, as every timed event's).
pub open spec fn spec_hk(p: Params) -> Term {
    Term { jitter: p.j_rel, period: if p.hk_period == 0 { 1 } else { p.hk_period }, bound: p.timer }
}

pub open spec fn spec_interferers(tasks: Seq<TaskIn>, sites: Seq<Site>, p: Params, any_fpu: bool, windows: Seq<Window>, quiet: Seq<bool>) -> Seq<Interferer> {
    Seq::new(tasks.len(), |j: int| spec_interferer(tasks, sites, p, any_fpu, windows, quiet, j))
}

/// The verdict `v` and the terms `ts` are those of Task `i` of the Task
/// set (R29.1 to R29.7, with R19.2): the recurrence is the one from
/// `C_i + B_i + O_i` over the terms of the Tasks' `E_j`, each term derived
/// from the Task set and its sites.
pub open spec fn verdict_ok(tasks: Seq<TaskIn>, sites: Seq<Site>, p: Params, any_fpu: bool, windows: Seq<Window>, quiet: Seq<bool>, i: int, v: Verdict, ts: Seq<TermOf>) -> bool {
    let its = spec_interferers(tasks, sites, p, any_fpu, windows, quiet);
    let base = tasks[i].budget as int + spec_b(tasks, sites, i, p)
        + spec_o(tasks[i], p, site_count(sites, i, sites.len() as int), any_fpu);
    &&& ts == spec_all_terms(its, i, spec_hk(p))
    &&& v.overflow ==> !v.schedulable
    &&& v.schedulable ==> crate::rta::is_fixpoint(base, plain(ts), v.w as int)
            && (forall|x: int| x >= 0 && #[trigger] crate::rta::is_fixpoint(base, plain(ts), x) ==> v.w as int <= x)
            && v.r as int == p.j_rel as int + v.w as int
            && v.r <= windows[i].deadline
    &&& !v.schedulable && !v.overflow ==> !meets_deadline(base, plain(ts), p.j_rel as int, windows[i].deadline as int)
}

/// `lower_events` of a Task, its deadline event left out when `quiet`.
fn lower_event_count(t: TaskIn, quiet: bool) -> (k: u64)
    ensures k as int == lower_events(t, !quiet), k <= 3,
{
    let base: u64 = if t.periodic { 0 } else if t.defer { 2 } else { 1 };
    if quiet { base } else { base + 1 }
}

/// One pass of the analysis: every Task's terms and verdict, with the
/// deadline event of each Task marked `quiet` left out of the recurrences
/// of the Tasks above it.
fn analyse_pass(tasks: &Vec<TaskIn>, sites: &Vec<Site>, p: Params, any_fpu: bool, windows: &Vec<Window>, derived: &Vec<Derived>, quiet: &Vec<bool>) -> (r: (Vec<Verdict>, Vec<Vec<TermOf>>))
    requires
        windows.len() == tasks.len(), derived.len() == tasks.len(), quiet.len() == tasks.len(),
        forall|i: int| 0 <= i < tasks.len() ==> derived_ok(tasks@, sites@, p, any_fpu, i, #[trigger] derived@[i]),
    ensures
        r.0.len() == tasks.len(), r.1.len() == tasks.len(),
        forall|i: int| 0 <= i < tasks.len() ==> verdict_ok(tasks@, sites@, p, any_fpu, windows@, quiet@, i, #[trigger] r.0@[i], r.1@[i]@),
{
    let n = tasks.len();
    let ghost spec_its = spec_interferers(tasks@, sites@, p, any_fpu, windows@, quiet@);
    let mut its: Vec<Interferer> = Vec::new();
    let mut j: usize = 0;
    while j < n
        invariant
            j <= n, n == tasks.len(), n == windows.len(), n == derived.len(), n == quiet.len(),
            its.len() == j,
            spec_its == spec_interferers(tasks@, sites@, p, any_fpu, windows@, quiet@),
            forall|i: int| 0 <= i < n ==> derived_ok(tasks@, sites@, p, any_fpu, i, #[trigger] derived@[i]),
            forall|m: int| 0 <= m < j ==> (#[trigger] its@[m]) == spec_its[m],
        decreases n - j,
    {
        let t = tasks[j];
        let d = derived[j];
        let w = windows[j];
        let q = quiet[j];
        proof {
            assert(derived_ok(tasks@, sites@, p, any_fpu, j as int, derived@[j as int]));
        }
        let period: u64 = if w.period == 0 { 1 } else { w.period };
        let intake: u64 = if t.periodic { p.timer } else { p.irq };
        let k: u64 = lower_event_count(t, q);
        proof {
            // Δ_timer · lower_events is within O_j, which fits.
            assert(p.timer as int * k as int <= spec_o(t, p, d.sites as int, any_fpu)) by (nonlinear_arith)
                requires k as int == lower_events(t, !q), lower_events(t, !q) <= timed_events(t), timed_events(t) >= 0,
                    spec_o(t, p, d.sites as int, any_fpu) >= p.timer as int * timed_events(t);
        }
        let events: u64 = p.timer * k;
        its.push(Interferer { priority: t.priority, jitter: d.j, period, bound: d.e, intake, events });
        j = j + 1;
    }
    assert(its@ =~= spec_its);
    assert(periods_ok(its@)) by {
        assert forall|k: int| 0 <= k < its@.len() implies (#[trigger] its@[k]).period > 0 by {
            assert(its@[k] == spec_its[k]);
        }
    }
    let mut verdicts: Vec<Verdict> = Vec::new();
    let mut all_terms: Vec<Vec<TermOf>> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n, n == tasks.len(), n == windows.len(), n == derived.len(), n == its.len(), n == quiet.len(),
            verdicts.len() == i, all_terms.len() == i,
            its@ == spec_interferers(tasks@, sites@, p, any_fpu, windows@, quiet@),
            periods_ok(its@),
            forall|k: int| 0 <= k < n ==> derived_ok(tasks@, sites@, p, any_fpu, k, #[trigger] derived@[k]),
            forall|k: int| 0 <= k < i ==> verdict_ok(tasks@, sites@, p, any_fpu, windows@, quiet@, k, #[trigger] verdicts@[k], all_terms@[k]@),
        decreases n - i,
    {
        let t = tasks[i];
        let d = derived[i];
        let w = windows[i];
        proof {
            assert(derived_ok(tasks@, sites@, p, any_fpu, i as int, derived@[i as int]));
            assert(its@[i as int] == spec_interferer(tasks@, sites@, p, any_fpu, windows@, quiet@, i as int));
        }
        let hk = Term { jitter: p.j_rel, period: if p.hk_period == 0 { 1 } else { p.hk_period }, bound: p.timer };
        let (verdict, own_terms) = response_of(&its, i, t.budget, d.b, d.o, w.deadline, hk);
        verdicts.push(verdict);
        all_terms.push(own_terms);
        i = i + 1;
    }
    (verdicts, all_terms)
}

/// The whole numeric analysis (R29.1 to R29.7 with R19.2), from the Task
/// set, its Critical_Section sites, the Kernel_Timing_Parameters and each
/// Task's period and deadline to every Task's terms and verdict: the term
/// derivation (`derive_all`), the Tasks as each other's interferers, and
/// each Task's recurrence and verdict (`response_of`).
///
/// The result's `quiet` marks the Tasks whose deadline event is left out of
/// the recurrences of the Tasks above them, and every Task so marked is
/// schedulable in this same result. That is sound by induction on the
/// Priority, lowest first: a Task's recurrence takes only lower Tasks'
/// events, a lower Task marked quiet meets its deadline, and the Kernel
/// disarms a Job's deadline event when the Job ends. The first pass marks
/// no Task; each further pass marks the Tasks the pass before found
/// schedulable, and is kept only if every marked Task is still schedulable
/// in it.
///
/// `None`: a term of some Task does not fit 64 bits (R29.6).
pub fn analyse_all(tasks: &Vec<TaskIn>, sites: &Vec<Site>, p: Params, any_fpu: bool, windows: &Vec<Window>) -> (r: Option<(Vec<Derived>, Vec<Verdict>, Vec<Vec<TermOf>>, Vec<bool>)>)
    requires
        sites_ok(tasks@, sites@),
        windows.len() == tasks.len(),
    ensures
        r matches Some(out) ==> out.0.len() == tasks.len() && out.1.len() == tasks.len() && out.2.len() == tasks.len()
            && out.3.len() == tasks.len()
            && (forall|i: int| 0 <= i < tasks.len() ==> derived_ok(tasks@, sites@, p, any_fpu, i, #[trigger] out.0@[i]))
            && (forall|i: int| 0 <= i < tasks.len() ==> verdict_ok(tasks@, sites@, p, any_fpu, windows@, out.3@, i, #[trigger] out.1@[i], out.2@[i]@))
            && (forall|i: int| 0 <= i < tasks.len() && #[trigger] out.3@[i] ==> out.1@[i].schedulable),
        r.is_none() ==> exists|i: int| 0 <= i < tasks.len() && !terms_fit(tasks@, sites@, p, any_fpu, i),
{
    let derived = match derive_all(tasks, sites, p, any_fpu) {
        Some(d) => d,
        None => return None,
    };
    let n = tasks.len();
    let mut quiet: Vec<bool> = Vec::new();
    let mut j: usize = 0;
    while j < n
        invariant j <= n, quiet.len() == j, forall|m: int| 0 <= m < j ==> !(#[trigger] quiet@[m]),
        decreases n - j,
    {
        quiet.push(false);
        j = j + 1;
    }
    let (mut verdicts, mut all_terms) = analyse_pass(tasks, sites, p, any_fpu, windows, &derived, &quiet);
    let mut pass: usize = 0;
    while pass < n
        invariant
            n == tasks.len(), n == windows.len(), n == derived.len(),
            quiet.len() == n, verdicts.len() == n, all_terms.len() == n,
            forall|i: int| 0 <= i < n ==> derived_ok(tasks@, sites@, p, any_fpu, i, #[trigger] derived@[i]),
            forall|i: int| 0 <= i < n ==> verdict_ok(tasks@, sites@, p, any_fpu, windows@, quiet@, i, #[trigger] verdicts@[i], all_terms@[i]@),
            forall|i: int| 0 <= i < n && #[trigger] quiet@[i] ==> verdicts@[i].schedulable,
        decreases n - pass,
    {
        // The Tasks this pass found schedulable; nothing to refine when
        // they are the ones already marked.
        let mut next: Vec<bool> = Vec::new();
        let mut same = true;
        let mut k: usize = 0;
        while k < n
            invariant k <= n, n == verdicts.len(), n == quiet.len(), next.len() == k,
            decreases n - k,
        {
            let s = verdicts[k].schedulable;
            if s != quiet[k] {
                same = false;
            }
            next.push(s);
            k = k + 1;
        }
        if same {
            break;
        }
        let (v2, t2) = analyse_pass(tasks, sites, p, any_fpu, windows, &derived, &next);
        // Kept only if every Task it marks is schedulable in it.
        let mut ok = true;
        let mut m: usize = 0;
        while m < n
            invariant m <= n, n == next.len(), n == v2.len(),
                ok ==> forall|i: int| 0 <= i < m && #[trigger] next@[i] ==> v2@[i].schedulable,
            decreases n - m,
        {
            if next[m] && !v2[m].schedulable {
                ok = false;
            }
            m = m + 1;
        }
        if !ok {
            break;
        }
        quiet = next;
        verdicts = v2;
        all_terms = t2;
        pass = pass + 1;
    }
    Some((derived, verdicts, all_terms, quiet))
}

/// What the statements of `analyse_all`, and Property 18 over them
/// (`proofs/aeneas/lean/Sched/KRun.lean`: `hsok`, `hwl`, `hbud`, `hprio`,
/// `hDT`), ask of the resolved inputs: every site names a Task, every Task
/// has a window, a positive Budget and a Priority above the idle level, and
/// no deadline is later than its period or MIT (2026-11-22).
pub open spec fn inputs_admissible(tasks: Seq<TaskIn>, sites: Seq<Site>, windows: Seq<Window>) -> bool {
    &&& sites_ok(tasks, sites)
    &&& windows.len() == tasks.len()
    &&& forall|i: int| 0 <= i < tasks.len() ==> task_admissible(#[trigger] tasks[i])
    &&& forall|i: int| 0 <= i < windows.len() ==> window_admissible(#[trigger] windows[i])
}

pub open spec fn task_admissible(t: TaskIn) -> bool {
    t.budget > 0 && t.priority > 0
}

pub open spec fn window_admissible(w: Window) -> bool {
    w.deadline <= w.period
}

/// The first input that is not admissible: `(1, k)` site `k` names no
/// Task, `(2, 0)` the windows are not one per Task, `(3, i)` Task `i` has
/// no Budget or no Priority, `(4, i)` the deadline of Task `i` is later
/// than its period or MIT. `None` exactly when the inputs are admissible:
/// the Analyzer refuses anything else before the analysis runs.
pub fn check_inputs(tasks: &Vec<TaskIn>, sites: &Vec<Site>, windows: &Vec<Window>) -> (r: Option<(u8, usize)>)
    ensures
        r.is_none() == inputs_admissible(tasks@, sites@, windows@),
        r matches Some(f) ==> match f.0 {
            1 => f.1 < sites.len() && sites@[f.1 as int].task >= tasks.len(),
            2 => windows.len() != tasks.len(),
            3 => f.1 < tasks.len() && !task_admissible(tasks@[f.1 as int]),
            4 => f.1 < windows.len() && !window_admissible(windows@[f.1 as int]),
            _ => false,
        },
{
    match first_stray_site(sites, tasks.len()) {
        Some(k) => return Some((1, k)),
        None => {}
    }
    if windows.len() != tasks.len() {
        return Some((2, 0));
    }
    match first_unfit_task(tasks) {
        Some(i) => return Some((3, i)),
        None => {}
    }
    match first_late_window(windows) {
        Some(j) => return Some((4, j)),
        None => {}
    }
    None
}

/// The first site that names no Task of a set of `n` Tasks, `None` when
/// each names one. (A loop of `check_inputs`, a function of its own since
/// 2026-11-26 for the translation to Lean, which takes a `return` in a
/// loop only when the loop ends its function.)
fn first_stray_site(sites: &Vec<Site>, n: usize) -> (r: Option<usize>)
    ensures
        r.is_none() ==> forall|m: int| 0 <= m < sites.len() ==> (#[trigger] sites@[m]).task < n,
        r matches Some(k) ==> k < sites.len() && sites@[k as int].task >= n,
{
    let mut k: usize = 0;
    while k < sites.len()
        invariant k <= sites.len(), forall|m: int| 0 <= m < k ==> (#[trigger] sites@[m]).task < n,
        decreases sites.len() - k,
    {
        if sites[k].task >= n {
            return Some(k);
        }
        k = k + 1;
    }
    None
}

/// The first Task with no Budget or no Priority, `None` when every Task
/// has both.
fn first_unfit_task(tasks: &Vec<TaskIn>) -> (r: Option<usize>)
    ensures
        r.is_none() ==> forall|m: int| 0 <= m < tasks.len() ==> task_admissible(#[trigger] tasks@[m]),
        r matches Some(i) ==> i < tasks.len() && !task_admissible(tasks@[i as int]),
{
    let mut i: usize = 0;
    while i < tasks.len()
        invariant i <= tasks.len(), forall|m: int| 0 <= m < i ==> task_admissible(#[trigger] tasks@[m]),
        decreases tasks.len() - i,
    {
        if tasks[i].budget == 0 || tasks[i].priority == 0 {
            return Some(i);
        }
        i = i + 1;
    }
    None
}

/// The first window whose deadline is later than its period or MIT,
/// `None` when there is none.
fn first_late_window(windows: &Vec<Window>) -> (r: Option<usize>)
    ensures
        r.is_none() ==> forall|m: int| 0 <= m < windows.len() ==> window_admissible(#[trigger] windows@[m]),
        r matches Some(j) ==> j < windows.len() && !window_admissible(windows@[j as int]),
{
    let mut j: usize = 0;
    while j < windows.len()
        invariant j <= windows.len(), forall|m: int| 0 <= m < j ==> window_admissible(#[trigger] windows@[m]),
        decreases windows.len() - j,
    {
        if windows[j].deadline > windows[j].period {
            return Some(j);
        }
        j = j + 1;
    }
    None
}

/// R40.2: an Endpoint operation of Task `task` as a Critical_Section site.
/// The Kernel executes it inside the SVC, so its Ceiling is above every
/// Priority, and it is marked as an Endpoint site (no lock entry and exit
/// of its own).
pub open spec fn endpoint_site(task: usize, wcet: u64) -> Site {
    Site { task, ceiling: u8::MAX, wcet, endpoint: true }
}

/// The two sites of one Endpoint role of Task `task`: a send of Δ_send and
/// a receive of Δ_recv.
pub fn endpoint_sites(task: usize, p: Params) -> (r: (Site, Site))
    ensures r.0 == endpoint_site(task, p.send), r.1 == endpoint_site(task, p.recv),
{
    (Site { task, ceiling: u8::MAX, wcet: p.send, endpoint: true }, Site { task, ceiling: u8::MAX, wcet: p.recv, endpoint: true })
}

/// **R40.2, the blocking term.** An Endpoint site blocks every Task of a
/// higher Priority than its holder: it is a candidate of that Task's B_i
/// whatever the Priorities are.
pub proof fn lemma_endpoint_site_blocks(tasks: Seq<TaskIn>, sites: Seq<Site>, i: int, k: int)
    requires
        0 <= k < sites.len(),
        sites[k] == endpoint_site(sites[k].task, sites[k].wcet),
        tasks[sites[k].task as int].priority < tasks[i].priority,
    ensures blocks(tasks, sites, i, k),
{
}

/// The larger of Δ_send and Δ_recv: what one declared Endpoint operation
/// of a Job costs at most.
pub open spec fn endpoint_op(p: Params) -> int {
    if p.send >= p.recv { p.send as int } else { p.recv as int }
}

/// **R40.2, the execution-time bound.** The Endpoint cost of a Task after
/// one more role with `ops` declared operations per Job: `acc` plus `ops`
/// times the larger of Δ_send and Δ_recv. `None` exactly when that does
/// not fit 64 bits (R29.6). The sum over a Task's roles is its
/// `endpoint_cost`, a summand of O_i (`spec_o`).
pub fn endpoint_cost_add(acc: u64, ops: u64, p: Params) -> (r: Option<u64>)
    ensures
        r.is_some() <==> acc as int + ops as int * endpoint_op(p) <= u64_max(),
        r matches Some(v) ==> v as int == acc as int + ops as int * endpoint_op(p),
{
    let op = if p.send >= p.recv { p.send } else { p.recv };
    assert((ops as int) * (op as int) >= 0) by (nonlinear_arith) requires ops >= 0, op >= 0;
    match mul(ops, op) {
        Some(c) => add(acc, c),
        None => None,
    }
}

} // verus!

#[cfg(test)]
mod admissible_tests {
    use super::*;

    fn task() -> TaskIn {
        TaskIn { priority: 2, partition: 0, budget: 100, stops_overruns: true, periodic: true, defer: false, endpoint_cost: 0 }
    }

    #[test]
    fn the_input_check_refuses_each_inadmissible_input_and_names_it() {
        let tasks = vec![task(), task()];
        let sites = vec![Site { task: 1, ceiling: 3, wcet: 10, endpoint: false }];
        let windows = vec![Window { period: 1000, deadline: 1000 }, Window { period: 1000, deadline: 500 }];
        assert_eq!(check_inputs(&tasks, &sites, &windows), None);
        assert_eq!(check_inputs(&tasks, &vec![Site { task: 2, ..sites[0] }], &windows), Some((1, 0)));
        assert_eq!(check_inputs(&tasks, &sites, &windows[..1].to_vec()), Some((2, 0)));
        assert_eq!(check_inputs(&vec![task(), TaskIn { budget: 0, ..task() }], &sites, &windows), Some((3, 1)));
        assert_eq!(check_inputs(&vec![TaskIn { priority: 0, ..task() }, task()], &sites, &windows), Some((3, 0)));
        assert_eq!(check_inputs(&tasks, &sites, &vec![windows[0], Window { period: 1000, deadline: 1001 }]), Some((4, 1)));
    }

    #[test]
    fn r40_2_an_endpoint_role_is_two_kernel_level_sites_and_its_operations_cost_the_larger_one() {
        let p = Params {
            release: 0, dispatch: 0, complete: 0, wake: 0, timer: 0, fp_save: 0, fp_restore: 0, lock_in: 0, lock_out: 0, detect: 0,
            enforce: 0, hm: 0, irq: 0, stamp: 0, entry: 0, l_kernel: 0, j_rel: 0, restart: 0, stop: 0, send: 700, recv: 900, hk_period: 1,
        };
        let (s, r) = endpoint_sites(3, p);
        assert_eq!(s, Site { task: 3, ceiling: u8::MAX, wcet: 700, endpoint: true });
        assert_eq!(r, Site { task: 3, ceiling: u8::MAX, wcet: 900, endpoint: true });
        assert_eq!(endpoint_cost_add(50, 4, p), Some(50 + 4 * 900));
        assert_eq!(endpoint_cost_add(u64::MAX, 1, p), None);
        assert_eq!(endpoint_cost_add(0, u64::MAX, p), None);
    }
}
