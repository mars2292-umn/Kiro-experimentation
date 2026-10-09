//! The SRP core: preemption stack, lock stack, System_Ceiling, and the
//! transitions that the dispatch stubs and the SVC services drive
//! (Properties 1 to 6; R7.1 to R7.6, R8.1 to R8.5, R8.9, R8.10, R20.3).
//!
//! State model. Each Task has a release state (`release`: none, pending,
//! or deferred; PR-12: at most one Release per Task) and a job state (`job`:
//! none, running, or killed). Jobs in progress form a stack in start order;
//! the top is the executing Job (R7.5: last-in, first-out resumption). A
//! Task whose Job is in progress may hold a pending Release (R9.7); that
//! Job starts only after the previous one ends, which the Priority test of
//! R7.1 already implies, since a Job in progress has a Priority at most the
//! top's. The lock stack holds one entry per Resource currently held, in
//! lock order, each tagged with the stack index of the holding Job. The
//! System_Ceiling mirrors the maximum Ceiling over the lock stack
//! (design.md invariant 2); Kernel_Arch writes it to BASEPRI.
//!
//! Invariant `inv` (design.md "Ceiling-Protocol State Machine", invariants
//! 1 to 4):
//!
//! 1. Jobs on the stack have strictly increasing Priority, and each started
//!    with a Priority above the System_Ceiling of that instant
//!    (`start_ceiling`).
//! 2. `ceiling` equals the maximum Ceiling of the held Resources, or the
//!    idle level when none is held.
//! 3. No Resource appears twice in the lock stack; every holder is a Job in
//!    progress and a declared accessor; lock entries are ordered by holder
//!    (a Job's entries sit above those of the Jobs below it).
//! 4. Every held Resource of a Job below position `i` has a Ceiling at most
//!    `start_ceiling[i]`: the Job at `i` started above every Ceiling held
//!    beneath it. This is the fact from which mutual exclusion follows
//!    (`lemma_no_other_holder`): a Job never finds a Resource it may access
//!    held by another Job.
//!
//! Forced ends (R8.9, R14.5). `end_job` ends the executing Job and releases
//! its Resources at once. `kill` marks a preempted Job of a stopped or
//! restarted Partition; its frame cannot be removed from under the Jobs
//! above it, so it keeps its lock entries until the Jobs above complete and
//! Kernel_Arch unwinds to it, at which point `finish_top` releases them and
//! completes it. Until then no other Job can lock those Resources: every
//! accessor has a Priority at most their Ceiling, which the System_Ceiling
//! still covers, so the lazy release delays nothing that preemption by the
//! Jobs above would not delay anyway (argument recorded in the
//! Verification_Plan, task 8.1).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::{Config, IDLE_CEILING, MAX_PRIORITY};

verus! {

/// Release states (PR-12: one Release per Task).
pub const REL_NONE: u8 = 0;
pub const REL_PENDING: u8 = 1;
pub const REL_DEFERRED: u8 = 2;

/// Job states.
pub const JOB_NONE: u8 = 0;
/// In progress: on the preemption stack.
pub const JOB_RUNNING: u8 = 1;
/// In progress but ended by a Health_Monitor action; completes when
/// Kernel_Arch unwinds to it.
pub const JOB_KILLED: u8 = 2;

/// The scheduling state for `NT` Tasks, `NR` Resources, and a lock stack
/// of capacity `NL` (the Generator sets `NL` to the sum of the per-Task
/// nesting depths, R23.3, R23.4).
pub struct Sched<const NT: usize, const NR: usize, const NL: usize> {
    pub release: [u8; NT],
    pub job: [u8; NT],
    /// Jobs in progress, in start order (`stack[depth - 1]` executes).
    pub stack: [u8; NT],
    pub depth: usize,
    /// The System_Ceiling at the instant `stack[i]` started.
    pub start_ceiling: [u8; NT],
    /// The System_Ceiling (mirror of BASEPRI).
    pub ceiling: u8,
    pub lock_res: [u8; NL],
    /// Stack index of the holder of `lock_res[e]`.
    pub lock_depth: [usize; NL],
    pub lock_len: usize,
}

/// The maximum Ceiling over the first `n` lock entries, or the idle level.
pub open spec fn max_ceiling<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    cfg: &Config<NT, NR, NP>, lock_res: [u8; NL], n: int,
) -> u8
    decreases n,
{
    if n <= 0 {
        IDLE_CEILING
    } else {
        let rest = max_ceiling(cfg, lock_res, n - 1);
        let c = cfg.ceiling(lock_res[n - 1] as int);
        if c > rest { c } else { rest }
    }
}

impl<const NT: usize, const NR: usize, const NL: usize> Sched<NT, NR, NL> {
    pub open spec fn in_progress(&self, t: int) -> bool {
        self.job[t] != JOB_NONE
    }

    pub open spec fn on_stack(&self, t: int) -> bool {
        exists|i: int| 0 <= i < self.depth && self.stack[i] == t
    }

    pub open spec fn top(&self) -> int {
        self.stack[self.depth - 1] as int
    }

    pub open spec fn held(&self, r: int) -> bool {
        exists|e: int| 0 <= e < self.lock_len && self.lock_res[e] == r
    }

    /// Whether lock entry `e` belongs to the executing Job.
    pub open spec fn entry_of_top(&self, e: int) -> bool {
        self.lock_depth[e] == self.depth - 1
    }

    pub open spec fn top_holds_nothing(&self) -> bool {
        forall|e: int| 0 <= e < self.lock_len ==> self.lock_depth[e] < self.depth - 1
    }

    /// R7.1: a released Job is eligible while its Priority is strictly
    /// above the System_Ceiling and above every Job in progress (the top of
    /// the stack has the highest Priority, invariant 1). A Task whose
    /// previous Job is still in progress is never eligible (R9.7); the
    /// Priority test implies it, and `job == JOB_NONE` states it.
    pub open spec fn eligible<const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: int) -> bool {
        &&& self.release[t] == REL_PENDING
        &&& self.job[t] == JOB_NONE
        &&& cfg.priority(t) > self.ceiling
        &&& (self.depth == 0 || cfg.priority(t) > cfg.priority(self.top()))
    }

    pub open spec fn inv<const NP: usize>(&self, cfg: &Config<NT, NR, NP>) -> bool {
        &&& cfg.wf()
        &&& NL <= 255
        &&& self.depth <= NT
        &&& self.lock_len <= NL
        &&& self.ceiling <= MAX_PRIORITY
        // Stack entries are valid, in progress, distinct, strictly increasing in Priority.
        &&& forall|i: int| 0 <= i < self.depth ==> (self.stack[i] as int) < NT
        &&& forall|i: int| 0 <= i < self.depth ==> self.in_progress(self.stack[i] as int)
        &&& forall|i: int, j: int| 0 <= i < j < self.depth ==> self.stack[i] != self.stack[j]
        &&& forall|i: int, j: int| 0 <= i < j < self.depth ==> cfg.priority(self.stack[i] as int) < cfg.priority(self.stack[j] as int)
        // Every Job in progress is on the stack; every state value is a known one.
        &&& forall|t: int| 0 <= t < NT ==> self.release[t] <= REL_DEFERRED
        &&& forall|t: int| 0 <= t < NT ==> self.job[t] <= JOB_KILLED
        &&& forall|t: int| 0 <= t < NT && self.in_progress(t) ==> self.on_stack(t)
        // Invariant 1: each Job started above the System_Ceiling of that instant.
        &&& forall|i: int| 0 <= i < self.depth ==> self.start_ceiling[i] <= MAX_PRIORITY
        &&& forall|i: int| 0 <= i < self.depth ==> cfg.priority(self.stack[i] as int) > self.start_ceiling[i]
        // Invariant 3: lock entries are valid, distinct, held by Jobs in progress that may access them, ordered by holder.
        &&& forall|e: int| 0 <= e < self.lock_len ==> (self.lock_res[e] as int) < NR
        &&& forall|e: int| 0 <= e < self.lock_len ==> self.lock_depth[e] < self.depth
        &&& forall|e: int| 0 <= e < self.lock_len ==> cfg.may_access(self.stack[self.lock_depth[e] as int] as int, self.lock_res[e] as int)
        &&& forall|e: int, f: int| 0 <= e < f < self.lock_len ==> self.lock_res[e] != self.lock_res[f]
        &&& forall|e: int, f: int| 0 <= e < f < self.lock_len ==> self.lock_depth[e] <= self.lock_depth[f]
        // Invariant 4: Resources held beneath position i have Ceilings at most start_ceiling[i].
        &&& forall|i: int, e: int| 0 <= i < self.depth && 0 <= e < self.lock_len && self.lock_depth[e] < i
                ==> cfg.ceiling(self.lock_res[e] as int) <= self.start_ceiling[i]
        // Invariant 2: the System_Ceiling mirrors the lock stack.
        &&& self.ceiling == max_ceiling(cfg, self.lock_res, self.lock_len as int)
    }

    /// The initial state: no Job in progress, no Release, nothing held
    /// (PR-34: Releases begin only after Init).
    pub fn new<const NP: usize>(cfg: &Config<NT, NR, NP>) -> (s: Self)
        requires cfg.wf(), NL <= 255,
        ensures s.inv(cfg), s.depth == 0, s.lock_len == 0, s.ceiling == IDLE_CEILING,
            forall|t: int| 0 <= t < NT ==> s.release[t] == REL_NONE && s.job[t] == JOB_NONE,
    {
        Sched {
            release: [REL_NONE; NT],
            job: [JOB_NONE; NT],
            stack: [0; NT],
            depth: 0,
            start_ceiling: [0; NT],
            ceiling: IDLE_CEILING,
            lock_res: [0; NL],
            lock_depth: [0; NL],
            lock_len: 0,
        }
    }

    // ----------------------------------------------------------------- lemmas

    /// Every lock entry's Ceiling is at most the mirror.
    pub proof fn lemma_entry_le_ceiling<const NP: usize>(cfg: &Config<NT, NR, NP>, lock_res: [u8; NL], n: int, e: int)
        requires 0 <= e < n,
        ensures cfg.ceiling(lock_res[e] as int) <= max_ceiling(cfg, lock_res, n),
        decreases n,
    {
        if e < n - 1 {
            Self::lemma_entry_le_ceiling(cfg, lock_res, n - 1, e);
        }
    }

    /// The mirror never exceeds MAX_PRIORITY when every entry is a valid Resource.
    pub proof fn lemma_max_ceiling_bound<const NP: usize>(cfg: &Config<NT, NR, NP>, lock_res: [u8; NL], n: int)
        requires 0 <= n, cfg.wf(), forall|e: int| 0 <= e < n ==> (lock_res[e] as int) < NR,
        ensures max_ceiling(cfg, lock_res, n) <= MAX_PRIORITY,
        decreases n,
    {
        if n > 0 {
            Self::lemma_max_ceiling_bound(cfg, lock_res, n - 1);
            assert(cfg.resource_ok(lock_res[n - 1] as int));
        }
    }

    /// The mirror is attained by some entry, or is idle when there is none.
    pub proof fn lemma_ceiling_attained<const NP: usize>(cfg: &Config<NT, NR, NP>, lock_res: [u8; NL], n: int)
        requires 0 <= n,
        ensures n == 0 ==> max_ceiling(cfg, lock_res, n) == IDLE_CEILING,
            n > 0 ==> exists|e: int| 0 <= e < n && cfg.ceiling(lock_res[e] as int) == max_ceiling(cfg, lock_res, n),
        decreases n,
    {
        if n > 0 {
            Self::lemma_ceiling_attained(cfg, lock_res, n - 1);
            let c = cfg.ceiling(lock_res[n - 1] as int);
            let rest = max_ceiling(cfg, lock_res, n - 1);
            if c > rest || n - 1 == 0 {
                assert(max_ceiling(cfg, lock_res, n) == c);
                assert(0 <= n - 1 < n && cfg.ceiling(lock_res[n - 1] as int) == max_ceiling(cfg, lock_res, n));
            } else {
                let e = choose|e: int| 0 <= e < n - 1 && cfg.ceiling(lock_res[e] as int) == rest;
                assert(max_ceiling(cfg, lock_res, n) == rest);
                assert(0 <= e < n && cfg.ceiling(lock_res[e] as int) == max_ceiling(cfg, lock_res, n));
            }
        }
    }

    /// `max_ceiling` depends only on the first `n` entries.
    pub proof fn lemma_max_ceiling_prefix<const NP: usize>(cfg: &Config<NT, NR, NP>, a: [u8; NL], b: [u8; NL], n: int)
        requires 0 <= n, forall|i: int| 0 <= i < n ==> a[i] == b[i],
        ensures max_ceiling(cfg, a, n) == max_ceiling(cfg, b, n),
        decreases n,
    {
        if n > 0 {
            Self::lemma_max_ceiling_prefix(cfg, a, b, n - 1);
        }
    }

    /// Jobs in progress stay on the stack when the stack is unchanged and
    /// no Job became in progress.
    pub proof fn lemma_on_stack_from(&self, pre: &Self)
        requires pre.depth == self.depth, pre.stack == self.stack,
            forall|u: int| 0 <= u < NT && self.in_progress(u) ==> pre.in_progress(u),
            forall|u: int| 0 <= u < NT && pre.in_progress(u) ==> pre.on_stack(u),
        ensures forall|u: int| 0 <= u < NT && self.in_progress(u) ==> self.on_stack(u),
    {
        assert forall|u: int| 0 <= u < NT && self.in_progress(u) implies self.on_stack(u) by {
            assert(pre.on_stack(u));
            let i = choose|i: int| 0 <= i < pre.depth && pre.stack[i] == u;
            assert(self.stack[i] == u);
        }
    }

    /// A Task with a Job in progress has a Priority at most the top's.
    pub proof fn lemma_in_progress_below_top<const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: int)
        requires self.inv(cfg), 0 <= t < NT, self.in_progress(t),
        ensures self.depth > 0, cfg.priority(t) <= cfg.priority(self.top()),
    {
        assert(self.on_stack(t));
        let i = choose|i: int| 0 <= i < self.depth && self.stack[i] == t;
        if i < self.depth - 1 {
            assert(cfg.priority(self.stack[i] as int) < cfg.priority(self.top()));
        }
    }

    /// Property 4 (mutual exclusion): under the invariant, a Resource that
    /// the executing Job may access is held, if at all, by that Job itself.
    pub proof fn lemma_no_other_holder<const NP: usize>(&self, cfg: &Config<NT, NR, NP>, r: int, e: int)
        requires self.inv(cfg), self.depth > 0, 0 <= r < NR,
            cfg.may_access(self.top(), r),
            0 <= e < self.lock_len, self.lock_res[e] == r,
        ensures self.entry_of_top(e),
    {
        let i = self.depth - 1;
        if self.lock_depth[e] < i {
            // Invariant 4: ceiling(r) <= start_ceiling[i] < priority(top) <= ceiling(r).
            assert(cfg.ceiling(r) <= self.start_ceiling[i]);
            assert(cfg.priority(self.top()) > self.start_ceiling[i]);
            assert(cfg.resource_ok(r));
            assert(cfg.priority(self.top()) <= cfg.ceiling(r));
            assert(false);
        }
    }

    /// Pigeonhole: `n` pairwise distinct values below `n` include every value below `n`.
    proof fn lemma_distinct_full_stack_covers(stack: [u8; NT], n: int, t: int)
        requires n == NT, 0 <= t < n,
            forall|i: int| 0 <= i < n ==> (stack[i] as int) < n,
            forall|i: int, j: int| 0 <= i < j < n ==> stack[i] != stack[j],
        ensures exists|i: int| 0 <= i < n && stack[i] == t,
    {
        broadcast use vstd::set_lib::group_set_properties;
        broadcast use Set::lemma_map_contains;
        let indices = vstd::set_lib::set_int_range(0, n);
        vstd::set_lib::lemma_int_range(0, n);
        let f = |i: int| stack[i] as int;
        let values = indices.map(f);
        assert(indices.injective_on(f)) by {
            assert forall|a: int, b: int| indices.contains(a) && indices.contains(b) && f(a) == f(b) implies a == b by {
                if a < b {
                    assert(stack[a] != stack[b]);
                } else if b < a {
                    assert(stack[b] != stack[a]);
                }
            }
        }
        vstd::set_lib::lemma_map_size(indices, values, f);
        assert(values.subset_of(indices)) by {
            assert forall|v: int| values.contains(v) implies indices.contains(v) by {
                let i = choose|i: int| indices.contains(i) && f(i) == v;
                assert((stack[i] as int) < n);
            }
        }
        vstd::set_lib::lemma_subset_equality(values, indices);
        assert(indices.contains(t));
        assert(values.contains(t));
        let i = choose|i: int| indices.contains(i) && f(i) == t;
        assert(0 <= i < n && stack[i] == t);
    }

    // ---------------------------------------------------------- transitions

    /// A Release of `t` becomes due and pending (R9.2, R10.1, R10.2): from
    /// no Release, or from a deferred one whose instant arrived.
    pub fn make_pending<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT, old(self).release[t as int] != REL_PENDING,
        ensures final(self).inv(cfg),
            final(self).release[t as int] == REL_PENDING,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).release[u] == old(self).release[u],
            final(self).job == old(self).job, final(self).stack == old(self).stack, final(self).depth == old(self).depth,
            final(self).ceiling == old(self).ceiling, final(self).lock_len == old(self).lock_len,
            final(self).lock_res == old(self).lock_res, final(self).lock_depth == old(self).lock_depth,
            final(self).start_ceiling == old(self).start_ceiling,
    {
        self.release[t as usize] = REL_PENDING;
        proof {
            self.lemma_on_stack_from(old(self));
        }
    }

    /// A Release of `t` is deferred until one MIT after the previous
    /// accepted Release became due (R10.2, deferral policy).
    pub fn defer<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT, old(self).release[t as int] == REL_NONE,
        ensures final(self).inv(cfg),
            final(self).release[t as int] == REL_DEFERRED,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).release[u] == old(self).release[u],
            final(self).job == old(self).job, final(self).stack == old(self).stack, final(self).depth == old(self).depth,
            final(self).ceiling == old(self).ceiling, final(self).lock_len == old(self).lock_len,
    {
        self.release[t as usize] = REL_DEFERRED;
        proof {
            self.lemma_on_stack_from(old(self));
        }
    }

    /// A Partition stop or restart suppresses the Release of `t` (R14.5).
    pub fn clear_release<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT,
        ensures final(self).inv(cfg),
            final(self).release[t as int] == REL_NONE,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).release[u] == old(self).release[u],
            final(self).job == old(self).job, final(self).stack == old(self).stack, final(self).depth == old(self).depth,
            final(self).ceiling == old(self).ceiling, final(self).lock_len == old(self).lock_len,
    {
        self.release[t as usize] = REL_NONE;
        proof {
            self.lemma_on_stack_from(old(self));
        }
    }

    /// Property 1 (selection): the pending Task of highest Priority, with
    /// ties broken by the lowest Task identifier (DD-03: exception-number
    /// order assigned in declaration order), if it is eligible. When it is
    /// not, no pending Task is (R7.1 compares Priorities only, and a pending
    /// Task whose Job is in progress sits on the stack below the top).
    pub fn select<const NP: usize>(&self, cfg: &Config<NT, NR, NP>) -> (sel: Option<u8>)
        requires self.inv(cfg),
        ensures
            sel.is_some() ==> (sel.unwrap() as int) < NT && self.eligible(cfg, sel.unwrap() as int),
            sel.is_some() ==> forall|t: int| 0 <= t < NT && self.release[t] == REL_PENDING ==> cfg.priority(t) <= cfg.priority(sel.unwrap() as int),
            sel.is_some() ==> forall|t: int| 0 <= t < NT && self.release[t] == REL_PENDING && cfg.priority(t) == cfg.priority(sel.unwrap() as int) ==> sel.unwrap() as int <= t,
            sel.is_none() ==> forall|t: int| 0 <= t < NT ==> !self.eligible(cfg, t),
    {
        let mut best: Option<u8> = None;
        let mut t: usize = 0;
        while t < NT
            invariant
                self.inv(cfg), t <= NT,
                best.is_some() ==> (best.unwrap() as int) < t && self.release[best.unwrap() as int] == REL_PENDING,
                best.is_some() ==> forall|u: int| 0 <= u < t && self.release[u] == REL_PENDING ==> cfg.priority(u) <= cfg.priority(best.unwrap() as int),
                best.is_some() ==> forall|u: int| 0 <= u < t && self.release[u] == REL_PENDING && cfg.priority(u) == cfg.priority(best.unwrap() as int) ==> best.unwrap() as int <= u,
                best.is_none() ==> forall|u: int| 0 <= u < t ==> self.release[u] != REL_PENDING,
            decreases NT - t,
        {
            if self.release[t] == REL_PENDING {
                match best {
                    None => { best = Some(t as u8); }
                    Some(b) => {
                        if cfg.tasks[t].priority > cfg.tasks[b as usize].priority {
                            best = Some(t as u8);
                        }
                    }
                }
            }
            t = t + 1;
        }
        match best {
            None => None,
            Some(b) => {
                let p = cfg.tasks[b as usize].priority;
                let above_top = if self.depth == 0 {
                    true
                } else {
                    p > cfg.tasks[self.stack[self.depth - 1] as usize].priority
                };
                let free = self.job[b as usize] == JOB_NONE;
                proof {
                    if !free {
                        self.lemma_in_progress_below_top(cfg, b as int);
                    }
                    if !(p > self.ceiling && above_top && free) {
                        assert forall|u: int| 0 <= u < NT implies !self.eligible(cfg, u) by {
                            if self.release[u] == REL_PENDING && self.job[u] == JOB_NONE {
                                assert(cfg.priority(u) <= p);
                            }
                        }
                    }
                }
                if p > self.ceiling && above_top && free {
                    Some(b)
                } else {
                    None
                }
            }
        }
    }

    /// Property 1 (start): the selected Job starts; it is pushed with the
    /// current System_Ceiling as its `start_ceiling` (R7.1, R7.2, R20.3).
    pub fn start<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT, old(self).eligible(cfg, t as int),
        ensures final(self).inv(cfg),
            final(self).depth == old(self).depth + 1,
            final(self).top() == t as int,
            final(self).job[t as int] == JOB_RUNNING,
            final(self).release[t as int] == REL_NONE,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).job[u] == old(self).job[u] && final(self).release[u] == old(self).release[u],
            final(self).ceiling == old(self).ceiling,
            final(self).lock_len == old(self).lock_len,
            final(self).lock_res == old(self).lock_res,
            final(self).lock_depth == old(self).lock_depth,
            forall|i: int| 0 <= i < old(self).depth ==> final(self).stack[i] == old(self).stack[i],
    {
        let d = self.depth;
        proof {
            // The Task has no Job in progress, so it is not on the stack.
            assert forall|i: int| 0 <= i < d implies self.stack[i] != t by {
                assert(self.in_progress(self.stack[i] as int));
            }
            if d == NT {
                Self::lemma_distinct_full_stack_covers(self.stack, NT as int, t as int);
                assert(false);
            }
            // Every Job on the stack has a Priority below the top's, which is below t's.
            assert forall|i: int| 0 <= i < d implies cfg.priority(self.stack[i] as int) < cfg.priority(t as int) by {
                if i < d - 1 {
                    assert(cfg.priority(self.stack[i] as int) < cfg.priority(self.stack[d - 1] as int));
                }
            }
        }
        self.stack[d] = t;
        self.start_ceiling[d] = self.ceiling;
        self.job[t as usize] = JOB_RUNNING;
        self.release[t as usize] = REL_NONE;
        self.depth = d + 1;
        proof {
            // Invariant 4 for the new top: every held Resource's Ceiling <= ceiling mirror = start_ceiling[d].
            assert forall|e: int| 0 <= e < self.lock_len implies cfg.ceiling(self.lock_res[e] as int) <= self.start_ceiling[d as int] by {
                Self::lemma_entry_le_ceiling(cfg, self.lock_res, self.lock_len as int, e);
            }
            // Every Job in progress is on the stack: t at d, the others where they were.
            assert forall|u: int| 0 <= u < NT && self.in_progress(u) implies self.on_stack(u) by {
                if u == t as int {
                    assert(self.stack[d as int] == t);
                } else {
                    assert(old(self).in_progress(u));
                    let i = choose|i: int| 0 <= i < old(self).depth && old(self).stack[i] == u;
                    assert(self.stack[i] == u);
                }
            }
        }
    }

    /// Property 2 (completion): the executing Job, which holds nothing,
    /// ends; the most recently preempted Job (if any) becomes the top with
    /// the System_Ceiling unchanged (R7.5). A pending Release of the Task
    /// stays pending (R9.7).
    pub fn complete<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), old(self).depth > 0, old(self).top_holds_nothing(),
        ensures final(self).inv(cfg),
            final(self).depth == old(self).depth - 1,
            final(self).job[old(self).top()] == JOB_NONE,
            forall|u: int| 0 <= u < NT && u != old(self).top() ==> final(self).job[u] == old(self).job[u],
            final(self).release == old(self).release,
            final(self).ceiling == old(self).ceiling,
            final(self).lock_len == old(self).lock_len,
            final(self).lock_res == old(self).lock_res,
            forall|i: int| 0 <= i < final(self).depth ==> final(self).stack[i] == old(self).stack[i],
    {
        let d = self.depth - 1;
        let t = self.stack[d];
        self.job[t as usize] = JOB_NONE;
        self.depth = d;
        proof {
            assert forall|u: int| 0 <= u < NT && self.in_progress(u) implies self.on_stack(u) by {
                assert(u != t as int);
                assert(self.job[u] == old(self).job[u]);
                assert(old(self).in_progress(u));
                assert(old(self).on_stack(u));
                let i = choose|i: int| 0 <= i < old(self).depth && old(self).stack[i] == u;
                assert(i != d as int);
                assert(self.stack[i] == u);
            }
        }
    }

    /// Property 3 (lock entry): the executing Job locks `r`. Returns `false`
    /// without change when the Job is not a declared accessor (R8.7, checked
    /// again in the SVC), already holds `r` (re-entry, R8.10), or the lock
    /// stack is full (impossible for a Generator-sized `NL`). Otherwise the
    /// System_Ceiling becomes `max(ceiling, Ceiling(r))` (R8.1) and the
    /// entry finds `r` free (R8.3): no other Job holds it.
    pub fn lock<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, r: u8) -> (ok: bool)
        requires old(self).inv(cfg), old(self).depth > 0, (r as int) < NR,
        ensures final(self).inv(cfg),
            ok == (cfg.may_access(old(self).top(), r as int) && !old(self).held(r as int) && old(self).lock_len < NL),
            ok ==> final(self).lock_len == old(self).lock_len + 1
                && final(self).lock_res[old(self).lock_len as int] == r
                && final(self).entry_of_top(old(self).lock_len as int)
                && final(self).ceiling == (if cfg.ceiling(r as int) > old(self).ceiling { cfg.ceiling(r as int) } else { old(self).ceiling }),
            !ok ==> final(self).ceiling == old(self).ceiling && final(self).lock_len == old(self).lock_len,
            // R8.3: a declared accessor that does not hold r finds it free of every other Job.
            cfg.may_access(old(self).top(), r as int) ==> forall|e: int| 0 <= e < old(self).lock_len && old(self).lock_res[e] == r ==> old(self).entry_of_top(e),
            final(self).depth == old(self).depth,
            final(self).stack == old(self).stack,
            final(self).job == old(self).job,
            final(self).release == old(self).release,
            final(self).start_ceiling == old(self).start_ceiling,
    {
        let d = self.depth - 1;
        let t = self.stack[d];
        proof {
            if cfg.may_access(old(self).top(), r as int) {
                assert forall|e: int| 0 <= e < old(self).lock_len && old(self).lock_res[e] == r implies old(self).entry_of_top(e) by {
                    Self::lemma_no_other_holder(old(self), cfg, r as int, e);
                }
            }
        }
        if !cfg.access[t as usize][r as usize] {
            return false;
        }
        // Re-entry check: scan the lock stack (R8.10).
        let mut e: usize = 0;
        let mut reentry = false;
        while e < self.lock_len
            invariant self.inv(cfg), e <= self.lock_len,
                reentry == exists|f: int| 0 <= f < e && self.lock_res[f] == r,
            decreases self.lock_len - e,
        {
            if self.lock_res[e] == r {
                reentry = true;
            }
            e = e + 1;
        }
        if reentry {
            return false;
        }
        if self.lock_len >= NL {
            return false;
        }
        let n = self.lock_len;
        let c = cfg.resources[r as usize].ceiling;
        self.lock_res[n] = r;
        self.lock_depth[n] = d;
        self.lock_len = n + 1;
        if c > self.ceiling {
            self.ceiling = c;
        }
        proof {
            assert(max_ceiling(cfg, self.lock_res, n as int) == max_ceiling(cfg, old(self).lock_res, n as int)) by {
                Self::lemma_max_ceiling_prefix(cfg, old(self).lock_res, self.lock_res, n as int);
            }
            assert(max_ceiling(cfg, self.lock_res, n as int + 1) == (if c > old(self).ceiling { c } else { old(self).ceiling }));
            assert(cfg.resource_ok(r as int));
            assert(self.ceiling <= MAX_PRIORITY);
            assert forall|i: int, e: int| 0 <= i < self.depth && 0 <= e < self.lock_len && self.lock_depth[e] < i
                implies cfg.ceiling(self.lock_res[e] as int) <= self.start_ceiling[i] by {
                if e < n {
                    assert(old(self).lock_depth[e] < i);
                } else {
                    assert(self.lock_depth[e] == d);
                    assert(false);
                }
            }
            self.lemma_on_stack_from(old(self));
        }
        true
    }

    /// Property 3 (lock exit): the executing Job releases `r`, which must be
    /// its most recent lock (last-in, first-out, R8.6). The System_Ceiling
    /// returns to the maximum over the remaining entries, which is the value
    /// before the matching entry (R8.2). Returns `false` without change
    /// otherwise.
    pub fn unlock<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, r: u8) -> (ok: bool)
        requires old(self).inv(cfg), old(self).depth > 0,
        ensures final(self).inv(cfg),
            ok == (old(self).lock_len > 0 && old(self).lock_res[old(self).lock_len - 1] == r && old(self).entry_of_top(old(self).lock_len - 1)),
            ok ==> final(self).lock_len == old(self).lock_len - 1
                && final(self).ceiling == max_ceiling(cfg, old(self).lock_res, old(self).lock_len - 1),
            !ok ==> final(self).lock_len == old(self).lock_len && final(self).ceiling == old(self).ceiling,
            final(self).depth == old(self).depth,
            final(self).stack == old(self).stack,
            final(self).job == old(self).job,
            final(self).release == old(self).release,
            final(self).start_ceiling == old(self).start_ceiling,
            final(self).lock_res == old(self).lock_res,
            final(self).lock_depth == old(self).lock_depth,
    {
        if self.lock_len == 0 {
            return false;
        }
        let n = self.lock_len - 1;
        if self.lock_res[n] != r || self.lock_depth[n] != self.depth - 1 {
            return false;
        }
        self.lock_len = n;
        self.ceiling = self.recompute_ceiling(cfg, n);
        proof {
            Self::lemma_max_ceiling_bound(cfg, self.lock_res, n as int);
            self.lemma_on_stack_from(old(self));
        }
        true
    }

    /// The maximum Ceiling over the first `n` entries, computed by a bounded
    /// loop (PR-16: `n <= NL`).
    pub fn recompute_ceiling<const NP: usize>(&self, cfg: &Config<NT, NR, NP>, n: usize) -> (c: u8)
        requires cfg.wf(), n <= NL, forall|e: int| 0 <= e < n ==> (self.lock_res[e] as int) < NR,
        ensures c == max_ceiling(cfg, self.lock_res, n as int),
    {
        let mut c: u8 = IDLE_CEILING;
        let mut e: usize = 0;
        while e < n
            invariant n <= NL, cfg.wf(), e <= n,
                forall|f: int| 0 <= f < n ==> (self.lock_res[f] as int) < NR,
                c == max_ceiling(cfg, self.lock_res, e as int),
            decreases n - e,
        {
            let rc = cfg.resources[self.lock_res[e] as usize].ceiling;
            if rc > c {
                c = rc;
            }
            e = e + 1;
        }
        c
    }

    /// Property 6 (forced end): the executing Job ends now, whatever it
    /// holds; every one of its lock entries is released and the
    /// System_Ceiling becomes the maximum Ceiling over the Resources still
    /// held by the Jobs beneath it (R8.9). Returns the number of released
    /// Resources, which the Health_Monitor reports.
    pub fn end_job<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>) -> (released: usize)
        requires old(self).inv(cfg), old(self).depth > 0,
        ensures final(self).inv(cfg),
            final(self).depth == old(self).depth - 1,
            final(self).job[old(self).top()] == JOB_NONE,
            forall|u: int| 0 <= u < NT && u != old(self).top() ==> final(self).job[u] == old(self).job[u],
            final(self).release == old(self).release,
            released <= old(self).lock_len,
            final(self).lock_len == old(self).lock_len - released,
            final(self).lock_res == old(self).lock_res,
            forall|e: int| 0 <= e < final(self).lock_len ==> !old(self).entry_of_top(e),
            forall|e: int| final(self).lock_len <= e < old(self).lock_len ==> old(self).entry_of_top(e),
            final(self).ceiling == max_ceiling(cfg, old(self).lock_res, final(self).lock_len as int),
            forall|i: int| 0 <= i < final(self).depth ==> final(self).stack[i] == old(self).stack[i],
    {
        let d = self.depth - 1;
        // The executing Job's entries are the topmost ones (ordered by holder).
        let mut n = self.lock_len;
        while n > 0 && self.lock_depth[n - 1] == d
            invariant self.inv(cfg), self.depth == old(self).depth, d == self.depth - 1,
                n <= self.lock_len,
                forall|e: int| n <= e < self.lock_len ==> self.lock_depth[e] == d,
            decreases n,
        {
            n = n - 1;
        }
        proof {
            // Below n, no entry belongs to the top: entries are ordered by holder.
            assert forall|e: int| 0 <= e < n implies self.lock_depth[e] < d by {
                if n > 0 {
                    assert(self.lock_depth[n - 1] != d);
                    assert(self.lock_depth[e] <= self.lock_depth[n - 1]);
                }
            }
        }
        let released = self.lock_len - n;
        self.lock_len = n;
        self.ceiling = self.recompute_ceiling(cfg, n);
        proof {
            Self::lemma_max_ceiling_bound(cfg, self.lock_res, n as int);
            self.lemma_on_stack_from(old(self));
            assert(self.top_holds_nothing());
        }
        self.complete(cfg);
        released
    }

    /// Marks the preempted Job at stack position `i` (not the top) as ended
    /// by a Health_Monitor action; it completes, releasing its Resources,
    /// when Kernel_Arch unwinds to it (`finish_top`). Jobs above keep
    /// running (R14.9).
    pub fn kill<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, i: usize)
        requires old(self).inv(cfg), i < old(self).depth,
        ensures final(self).inv(cfg),
            final(self).job[old(self).stack[i as int] as int] == JOB_KILLED,
            forall|u: int| 0 <= u < NT && u != old(self).stack[i as int] ==> final(self).job[u] == old(self).job[u],
            final(self).release == old(self).release,
            final(self).depth == old(self).depth, final(self).stack == old(self).stack,
            final(self).ceiling == old(self).ceiling, final(self).lock_len == old(self).lock_len,
    {
        let t = self.stack[i];
        self.job[t as usize] = JOB_KILLED;
        proof {
            assert forall|u: int| 0 <= u < NT && self.in_progress(u) implies self.on_stack(u) by {
                if u == t as int {
                    assert(0 <= i < self.depth && self.stack[i as int] == u);
                } else {
                    assert(self.job[u] == old(self).job[u]);
                    assert(old(self).in_progress(u));
                    assert(old(self).on_stack(u));
                    let j = choose|j: int| 0 <= j < old(self).depth && old(self).stack[j] == u;
                    assert(self.stack[j] == u);
                }
            }
        }
    }

    /// The executing Job returns to its Release_Point, or Kernel_Arch
    /// unwinds to a killed Job: its remaining lock entries (none for a
    /// well-behaved return, PR-07) are released and it completes. Returns
    /// the number of Resources released (non-zero only for a killed Job, or
    /// for a Job whose API use the type system failed to reject, which the
    /// Health_Monitor records as a Kernel fault).
    pub fn finish_top<const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>) -> (released: usize)
        requires old(self).inv(cfg), old(self).depth > 0,
        ensures final(self).inv(cfg),
            final(self).depth == old(self).depth - 1,
            final(self).job[old(self).top()] == JOB_NONE,
            final(self).release == old(self).release,
            old(self).top_holds_nothing() ==> released == 0 && final(self).ceiling == old(self).ceiling,
    {
        let released = self.end_job(cfg);
        proof {
            if old(self).top_holds_nothing() {
                if released > 0 {
                    assert(old(self).entry_of_top(old(self).lock_len - 1));
                    assert(false);
                }
                assert(self.lock_len == old(self).lock_len);
            }
        }
        released
    }

    /// Property 5 (single-section blocking, state part): a pending Job that
    /// is not eligible only because of the System_Ceiling is blocked by a
    /// Resource held by a Job of lower Priority whose Ceiling is at least
    /// the pending Job's Priority. With the SRP trace argument (one such
    /// section per Release, since the blocking Job cannot lock a second
    /// such Resource after the pending Job's Release without first
    /// finishing the first: it would have to start a new Job, which R7.1
    /// forbids), this bounds the blocking to one Critical_Section (R8.4).
    pub proof fn lemma_blocked_by_one_holder<const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: int)
        requires self.inv(cfg), 0 <= t < NT, self.release[t] == REL_PENDING, self.job[t] == JOB_NONE,
            cfg.priority(t) <= self.ceiling,
            self.depth == 0 || cfg.priority(t) > cfg.priority(self.top()),
        ensures exists|e: int| 0 <= e < self.lock_len
            && cfg.ceiling(self.lock_res[e] as int) >= cfg.priority(t)
            && cfg.priority(self.stack[self.lock_depth[e] as int] as int) < cfg.priority(t),
    {
        Self::lemma_ceiling_attained(cfg, self.lock_res, self.lock_len as int);
        assert(cfg.task_ok(t));
        if self.lock_len == 0 {
            assert(self.ceiling == IDLE_CEILING);
            assert(cfg.priority(t) >= 1);
            assert(false);
        }
        let e = choose|e: int| 0 <= e < self.lock_len && cfg.ceiling(self.lock_res[e] as int) == self.ceiling;
        let i = self.lock_depth[e] as int;
        assert(i < self.depth);
        if i < self.depth - 1 {
            assert(cfg.priority(self.stack[i] as int) < cfg.priority(self.top()));
        }
        assert(cfg.priority(self.stack[i] as int) < cfg.priority(t));
    }
}

} // verus!
