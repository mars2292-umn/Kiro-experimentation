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
//! it (`Kernel::now`, monotone), the Task, and one argument: the Resource
//! of a lock or unlock, the due instant of a Release, 1 for the end of a
//! killed Job. `history_wf` is the trace invariant that `Kernel::inv`
//! carries: every step names a Task and, for locks, a Resource of the
//! configuration, and the instants never decrease and never exceed the
//! Kernel's observed instant.

use vstd::prelude::*;

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
/// killed for the unwind (`arg` 0).
pub const ST_KILL: u8 = 6;

pub struct Step {
    pub kind: u8,
    pub task: int,
    /// The Kernel's observed instant when the step was performed.
    pub at: int,
    pub arg: int,
}

/// A step of a well-formed history over `nt` Tasks and `nr` Resources.
pub open spec fn step_wf(s: Step, nt: int, nr: int) -> bool {
    &&& ST_DISPATCH <= s.kind <= ST_KILL
    &&& 0 <= s.task < nt
    &&& 0 <= s.at
    &&& 0 <= s.arg
    &&& (s.kind == ST_LOCK || s.kind == ST_UNLOCK) ==> s.arg < nr
}

/// The trace invariant: well-formed steps, instants nondecreasing along
/// the history and never beyond the Kernel's observed instant `now`.
pub open spec fn history_wf(h: Seq<Step>, nt: int, nr: int, now: int) -> bool {
    &&& forall|i: int| 0 <= i < h.len() ==> step_wf(#[trigger] h[i], nt, nr) && h[i].at <= now
    &&& forall|i: int, j: int| 0 <= i < j < h.len() ==> #[trigger] h[i].at <= #[trigger] h[j].at
}

/// The empty history is well formed at any instant.
pub proof fn lemma_history_empty(nt: int, nr: int, now: int)
    requires 0 <= now,
    ensures history_wf(Seq::<Step>::empty(), nt, nr, now),
{
}

/// A well-formed history stays well formed as the observed instant grows.
pub proof fn lemma_history_later(h: Seq<Step>, nt: int, nr: int, now: int, later: int)
    requires history_wf(h, nt, nr, now), now <= later,
    ensures history_wf(h, nt, nr, later),
{
}

/// Appending a well-formed step performed at the observed instant keeps
/// the history well formed.
pub proof fn lemma_history_push(h: Seq<Step>, s: Step, nt: int, nr: int, now: int)
    requires history_wf(h, nt, nr, now), step_wf(s, nt, nr), s.at == now,
    ensures history_wf(h.push(s), nt, nr, now),
{
    let p = h.push(s);
    assert forall|i: int| 0 <= i < p.len() implies step_wf(#[trigger] p[i], nt, nr) && p[i].at <= now by {
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
}

/// The history only grows: `h2` extends `h1` (every step of `h1` is a
/// step of `h2` at the same position). Every Kernel entry point ensures
/// this of the history it leaves against the one it found. Stated as a
/// plain universal so that the solver chains it across calls unaided.
pub open spec fn extends(h2: Seq<Step>, h1: Seq<Step>) -> bool {
    &&& h1.len() <= h2.len()
    &&& forall|i: int| 0 <= i < h1.len() ==> #[trigger] h2[i] == h1[i]
}

} // verus!
