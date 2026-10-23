//! The ghost history of the Kernel (task 4.11): a trace of the scheduling
//! transitions, carried by `Kernel` as a `Ghost<Seq<Step>>` and extended
//! by the entry point that performs each transition. It exists only in
//! the proofs (ghost erasure leaves nothing in a Flight_Build, ORQ-08),
//! so that the arguments of Verification_Plan section 5 (bounded blocking,
//! ceiling restore across preemptions, lazy release, MIT spacing over a
//! run, deadlock freedom; tasks 4.12 to 4.14) can be stated as theorems
//! over a run instead of per transition.
//!
//! Each step records the observed instant at which the Kernel performed
//! it (`Kernel::now`, monotone), the Task, one argument (the Resource of
//! a lock or unlock, the due instant of a Release, 1 for the end of a
//! killed Job), and a snapshot of the lock stack after the step: the
//! Resources held, bottom to top (`locks`), and the System_Ceiling
//! (`ceiling`). `history_wf` is the trace invariant that `Kernel::inv`
//! carries: every step names a Task and Resources of the configuration;
//! the instants never decrease and never exceed the Kernel's observed
//! instant; the ceiling of every step is the maximum Ceiling over its
//! lock snapshot; and consecutive snapshots are related by the step's
//! kind (`step_rel`): a lock pushes one entry, an unlock pops the top
//! one, the end of a Job (returned or forced) drops a suffix, and every
//! other step keeps the stack.
//!
//! Theorems over the history (task 4.13): `lemma_ceiling_restored`
//! (Verification_Plan argument 5.3, R8.2): after an unlock the
//! System_Ceiling equals its value before the matching lock, whatever
//! steps of other Jobs occurred between the two. The matching unlock of a
//! lock step `i` is the first later step `j` at which the stack is back
//! to its depth before `i`; between them the stack never goes below the
//! locked entry, which is what "matching" means on a LIFO stack.

use vstd::prelude::*;

use super::{Config, IDLE_CEILING};

verus! {

/// A Job started executing (`Kernel::dispatch`).
pub const ST_DISPATCH: u8 = 1;
/// The executing Job completed and returned (`Kernel::job_return`); `arg`
/// is 1 when the Job had been killed and the unwind ended it (R8.9).
pub const ST_END: u8 = 2;
/// The executing Job locked Resource `arg` (`Kernel::svc_lock`).
pub const ST_LOCK: u8 = 3;
/// The executing Job unlocked Resource `arg` (`Kernel::svc_unlock`).
pub const ST_UNLOCK: u8 = 4;
/// A Release of the Task became due at instant `arg` and pending: a
/// periodic nominal instant, a deferred Release firing, or an accepted
/// sporadic event (R9.2, R10.1, R10.2).
pub const ST_RELEASE: u8 = 5;
/// A Job in progress was killed by a Health_Monitor response (R14.5): the
/// executing one ended by force (`arg` 1), or a preempted one marked
/// killed for the unwind (`arg` 0; its lock entries stay, R8.9).
pub const ST_KILL: u8 = 6;

pub struct Step {
    pub kind: u8,
    pub task: int,
    /// The Kernel's observed instant when the step was performed.
    pub at: int,
    pub arg: int,
    /// The Resources held after the step, bottom to top (`Sched::lock_res`
    /// up to `lock_len`).
    pub locks: Seq<int>,
    /// The System_Ceiling after the step.
    pub ceiling: int,
}

/// The maximum Ceiling over a lock snapshot (`Sched::max_ceiling` on a
/// sequence; `IDLE_CEILING` when nothing is held).
pub open spec fn ceiling_of<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>, locks: Seq<int>) -> int
    decreases locks.len(),
{
    if locks.len() == 0 {
        IDLE_CEILING as int
    } else {
        let rest = ceiling_of(cfg, locks.drop_last());
        let c = cfg.ceiling(locks.last()) as int;
        if c > rest { c } else { rest }
    }
}

/// `b` keeps the first `n` entries of `a`.
pub open spec fn same_below(a: Seq<int>, b: Seq<int>, n: int) -> bool {
    forall|e: int| 0 <= e < n ==> #[trigger] b[e] == a[e]
}

/// How a step's lock snapshot follows from the previous one (`prev`).
pub open spec fn step_rel(prev: Seq<int>, s: Step) -> bool {
    if s.kind == ST_LOCK {
        // The locked Resource is pushed on top.
        &&& s.locks.len() == prev.len() + 1
        &&& same_below(prev, s.locks, prev.len() as int)
        &&& s.locks.last() == s.arg
    } else if s.kind == ST_UNLOCK {
        // The top entry, the unlocked Resource, is popped.
        &&& prev.len() >= 1
        &&& s.locks.len() == prev.len() - 1
        &&& same_below(prev, s.locks, s.locks.len() as int)
        &&& prev.last() == s.arg
    } else if s.kind == ST_END || s.kind == ST_KILL {
        // The ended Job's entries, a suffix, are released (none for a
        // Job marked killed, whose entries stay until the unwind).
        &&& s.locks.len() <= prev.len()
        &&& same_below(prev, s.locks, s.locks.len() as int)
    } else {
        // A dispatch or a Release leaves the lock stack as it is.
        &&& s.locks.len() == prev.len()
        &&& same_below(prev, s.locks, prev.len() as int)
    }
}

/// A step of a well-formed history over the configuration.
pub open spec fn step_wf<const NT: usize, const NR: usize, const NP: usize>(s: Step, cfg: &Config<NT, NR, NP>) -> bool {
    &&& ST_DISPATCH <= s.kind <= ST_KILL
    &&& 0 <= s.task < NT
    &&& 0 <= s.at
    &&& 0 <= s.arg
    &&& (s.kind == ST_LOCK || s.kind == ST_UNLOCK) ==> s.arg < NR
    &&& forall|e: int| 0 <= e < s.locks.len() ==> 0 <= #[trigger] s.locks[e] < NR
    &&& s.ceiling == ceiling_of(cfg, s.locks)
}

/// The lock snapshot before step `k` of `h`: the previous step's, or
/// nothing held before the first step (`Sched::new`).
pub open spec fn locks_before(h: Seq<Step>, k: int) -> Seq<int> {
    if k == 0 { Seq::<int>::empty() } else { h[k - 1].locks }
}

/// The trace invariant: well-formed steps, instants nondecreasing along
/// the history and never beyond the Kernel's observed instant `now`, and
/// every step's lock snapshot related to the previous one by its kind.
/// Opaque: the Kernel entry points carry it unopened (it changes only in
/// `record` and `observe`, which use the lemmas below); only the history
/// lemmas and theorems reveal it.
#[verifier::opaque]
pub open spec fn history_wf<const NT: usize, const NR: usize, const NP: usize>(h: Seq<Step>, cfg: &Config<NT, NR, NP>, now: int) -> bool {
    &&& forall|i: int| 0 <= i < h.len() ==> step_wf(#[trigger] h[i], cfg) && h[i].at <= now
    &&& forall|i: int, j: int| 0 <= i < j < h.len() ==> #[trigger] h[i].at <= #[trigger] h[j].at
    &&& forall|k: int| 0 <= k < h.len() ==> step_rel(locks_before(h, k), #[trigger] h[k])
}

/// The empty history is well formed at any instant.
pub proof fn lemma_history_empty<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>, now: int)
    requires 0 <= now,
    ensures history_wf(Seq::<Step>::empty(), cfg, now),
{
    reveal(history_wf);
}

/// A well-formed history stays well formed as the observed instant grows.
pub proof fn lemma_history_later<const NT: usize, const NR: usize, const NP: usize>(h: Seq<Step>, cfg: &Config<NT, NR, NP>, now: int, later: int)
    requires history_wf(h, cfg, now), now <= later,
    ensures history_wf(h, cfg, later),
{
    reveal(history_wf);
}

/// Appending a well-formed step performed at the observed instant, whose
/// snapshot follows from the last one, keeps the history well formed.
pub proof fn lemma_history_push<const NT: usize, const NR: usize, const NP: usize>(h: Seq<Step>, s: Step, cfg: &Config<NT, NR, NP>, now: int)
    requires history_wf(h, cfg, now), step_wf(s, cfg), s.at == now, step_rel(locks_before(h.push(s), h.len() as int), s),
    ensures history_wf(h.push(s), cfg, now),
{
    reveal(history_wf);
    let p = h.push(s);
    assert forall|i: int| 0 <= i < p.len() implies step_wf(#[trigger] p[i], cfg) && p[i].at <= now by {
        if i < h.len() {
            assert(p[i] == h[i]);
        } else {
            assert(p[i] == s);
        }
    }
    assert forall|i: int, j: int| 0 <= i < j < p.len() implies #[trigger] p[i].at <= #[trigger] p[j].at by {
        assert(p[i] == h[i]);
        if j < h.len() {
            assert(p[j] == h[j]);
        } else {
            assert(p[j] == s);
        }
    }
    assert forall|k: int| 0 <= k < p.len() implies step_rel(locks_before(p, k), #[trigger] p[k]) by {
        if k < h.len() {
            assert(p[k] == h[k]);
            if k > 0 {
                assert(p[k - 1] == h[k - 1]);
            }
            assert(locks_before(p, k) == locks_before(h, k));
        } else {
            assert(p[k] == s);
        }
    }
}

/// The history only grows: `h2` extends `h1` (every step of `h1` is a
/// step of `h2` at the same position). Every Kernel entry point ensures
/// this of the history it leaves against the one it found. Stated as a
/// plain universal so that the solver chains it across calls unaided.
pub open spec fn extends(h2: Seq<Step>, h1: Seq<Step>) -> bool {
    &&& h1.len() <= h2.len()
    &&& forall|i: int| 0 <= i < h1.len() ==> #[trigger] h2[i] == h1[i]
}

// ------------------------------------------------------- ceiling_of lemmas

/// Snapshots equal entry by entry have the same ceiling.
pub proof fn lemma_ceiling_of_ext<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>, a: Seq<int>, b: Seq<int>)
    requires a.len() == b.len(), same_below(a, b, a.len() as int),
    ensures ceiling_of(cfg, a) == ceiling_of(cfg, b),
    decreases a.len(),
{
    if a.len() > 0 {
        assert(same_below(a.drop_last(), b.drop_last(), a.len() - 1)) by {
            assert forall|e: int| 0 <= e < a.len() - 1 implies #[trigger] b.drop_last()[e] == a.drop_last()[e] by {
                assert(b[e] == a[e]);
            }
        }
        lemma_ceiling_of_ext(cfg, a.drop_last(), b.drop_last());
        assert(a.last() == b.last());
    }
}

/// `ceiling_of` over the first `n` entries of a lock array is
/// `Sched::max_ceiling` over them: the bridge from the scheduler's
/// mirror (invariant 2) to the history's snapshot.
pub proof fn lemma_ceiling_of_array<const NT: usize, const NR: usize, const NP: usize, const NL: usize>(
    cfg: &Config<NT, NR, NP>, lock_res: [u8; NL], n: int,
)
    requires 0 <= n <= NL,
    ensures ceiling_of(cfg, Seq::new(n as nat, |e: int| lock_res[e] as int)) == super::sched::max_ceiling(cfg, lock_res, n) as int,
    decreases n,
{
    let s = Seq::new(n as nat, |e: int| lock_res[e] as int);
    if n > 0 {
        lemma_ceiling_of_array(cfg, lock_res, n - 1);
        let r = Seq::new((n - 1) as nat, |e: int| lock_res[e] as int);
        assert(s.drop_last() =~= r);
        assert(s.last() == lock_res[n - 1] as int);
    }
}

// --------------------------------------------- Theorem: ceiling restore (5.3)

/// Between steps `i` and `k` the stack never went below depth `n`
/// (every step after `i` up to `k` has at least `n` entries), so the
/// first `n` entries of step `i`'s snapshot are still in place at `k`.
pub proof fn lemma_prefix_kept<const NT: usize, const NR: usize, const NP: usize>(h: Seq<Step>, cfg: &Config<NT, NR, NP>, now: int, i: int, k: int, n: int)
    requires history_wf(h, cfg, now), 0 <= i <= k < h.len(), 0 <= n <= h[i].locks.len(),
        forall|m: int| i < m <= k ==> #[trigger] h[m].locks.len() >= n,
    ensures same_below(h[i].locks, h[k].locks, n),
    decreases k - i,
{
    reveal(history_wf);
    if k > i {
        lemma_prefix_kept(h, cfg, now, i, k - 1, n);
        let prev = h[k - 1].locks;
        let s = h[k];
        assert(step_rel(locks_before(h, k), s));
        assert(locks_before(h, k) == prev);
        assert(prev.len() >= n);
        assert(s.locks.len() >= n);
        // Every kind keeps the entries below the new length, which is at
        // least `n`: a lock keeps all of `prev`, an unlock and a Job end
        // keep the first `s.locks.len()`, the rest keep `prev` as it is.
        if s.kind == ST_LOCK {
            assert(same_below(prev, s.locks, prev.len() as int));
        } else if s.kind == ST_UNLOCK {
            assert(same_below(prev, s.locks, s.locks.len() as int));
        } else if s.kind == ST_END || s.kind == ST_KILL {
            assert(same_below(prev, s.locks, s.locks.len() as int));
        } else {
            assert(same_below(prev, s.locks, prev.len() as int));
        }
        assert forall|e: int| 0 <= e < n implies #[trigger] s.locks[e] == h[i].locks[e] by {
            assert(prev[e] == h[i].locks[e]);
            assert(s.locks[e] == prev[e]);
        }
    }
}

/// Verification_Plan argument 5.3 (R8.2, Property 3), over the history:
/// step `i` is a lock, step `j` its matching unlock (the stack is back to
/// the depth before `i`, and between them it never went below the locked
/// entry), so the System_Ceiling after `j` is what it was before `i`,
/// whatever other Jobs locked, unlocked, started, or ended in between.
pub proof fn lemma_ceiling_restored<const NT: usize, const NR: usize, const NP: usize>(h: Seq<Step>, cfg: &Config<NT, NR, NP>, now: int, i: int, j: int)
    requires history_wf(h, cfg, now), 0 <= i < j < h.len(),
        h[i].kind == ST_LOCK, h[j].kind == ST_UNLOCK,
        h[j].locks.len() == h[i].locks.len() - 1,
        forall|m: int| i < m < j ==> #[trigger] h[m].locks.len() >= h[i].locks.len(),
    ensures
        // The stack after the unlock is the stack before the lock.
        h[j].locks.len() == locks_before(h, i).len(),
        same_below(locks_before(h, i), h[j].locks, h[j].locks.len() as int),
        // So the ceiling is restored.
        h[j].ceiling == ceiling_of(cfg, locks_before(h, i)),
        h[j].ceiling == (if i == 0 { IDLE_CEILING as int } else { h[i - 1].ceiling }),
{
    reveal(history_wf);
    let n = h[i].locks.len() as int;
    let before = locks_before(h, i);
    assert(step_rel(before, h[i]));
    assert(before.len() == n - 1);
    // Up to the step before the unlock, the locked entry and everything
    // below it are in place.
    lemma_prefix_kept(h, cfg, now, i, j - 1, n);
    assert(step_rel(locks_before(h, j), h[j]));
    assert(locks_before(h, j) == h[j - 1].locks);
    assert(h[j - 1].locks.len() == n);
    assert forall|e: int| 0 <= e < n - 1 implies #[trigger] h[j].locks[e] == before[e] by {
        assert(h[j].locks[e] == h[j - 1].locks[e]);
        assert(h[j - 1].locks[e] == h[i].locks[e]);
        assert(h[i].locks[e] == before[e]);
    }
    lemma_ceiling_of_ext(cfg, before, h[j].locks);
    assert(h[j].ceiling == ceiling_of(cfg, h[j].locks));
    if i > 0 {
        assert(h[i - 1].ceiling == ceiling_of(cfg, h[i - 1].locks));
    }
}

} // verus!
