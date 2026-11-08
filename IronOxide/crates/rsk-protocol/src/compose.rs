//! Progress of a whole system of Endpoints (R41.3 across Endpoints; the
//! A-P8 assumption of `progress.rs` made a theorem, 2026-11-08).
//!
//! # The model
//!
//! A system is a sequence of members (`Member`): each a Protocol-governed
//! Endpoint's automata (`progress::System`), the Task bound to each of its
//! roles, and a rank. A Task's Jobs may make an operation on one Endpoint
//! wait for a receive on another: the declaration states this as a
//! *gating* relation between Endpoints (`gates(e1, e2)`: the Task's
//! operations on `e2` may wait for its receives on `e1`), and the
//! Generator accepts it only when it is acyclic, which it certifies by the
//! ranks: every gating edge goes from a lower rank to a higher one.
//!
//! # The property
//!
//! A *stuck set* (`stuck_set`) is a set of Tasks with a Task awaiting a
//! message on some Endpoint, in which every Task's enabled operation on an
//! Endpoint is gated by one of its own awaiting roles on a lower Endpoint,
//! and every awaited message's sender is a Task of the set or a role that
//! has finished. Jobs never block, so this is the only way the system's
//! communication can stop: every Task of the set polls and finds nothing,
//! for ever.
//!
//! # The theorem
//!
//! `theorem_system_progress`: when no member's configuration is deadlocked
//! on its own (`progress::deadlocked`, which `progress::certify` establishes
//! for every reachable configuration of every accepted Protocol) and the
//! gating relation is ranked, no stuck set exists. Proof: take the
//! lowest-ranked Endpoint on which a Task of the set awaits; the roles of
//! the set's Tasks that await or have finished on it form a waiting set of
//! that Endpoint, because an awaited sender with an enabled operation
//! would be a Task of the set that is able on this Endpoint, so gated by an
//! awaiting role on a lower-ranked Endpoint, against the choice.
//! `theorem_system_r41_3` composes it with the per-Endpoint certificates
//! over every combination of reachable member configurations, which
//! includes every configuration the gated system reaches (gating only
//! removes interleavings; each member's steps are a run of its own).
//!
//! What remains outside this module is stated in the Verification_Plan
//! (5.15): that the Jobs gate their operations only as declared.

use vstd::prelude::*;

use crate::progress::System;
#[cfg(verus_keep_ghost)]
use crate::progress::{at, blocked, certificate, deadlock_set, deadlocked, enabled, has_trans, reachable, terminal, theorem_r41_3, wf_cfg, wf_sys, Cfg};

verus! {

/// One Endpoint of the system: its automata, the Task of each role, and
/// its rank in the gating order.
pub struct Member {
    pub sys: System,
    /// The Task bound to each role (indexed by role).
    pub tasks: Vec<usize>,
    pub rank: usize,
}

pub open spec fn member_wf(m: Member, ntasks: int) -> bool {
    &&& wf_sys(m.sys)
    &&& m.tasks@.len() == m.sys.roles
    &&& forall|r: int| 0 <= r < m.tasks@.len() ==> (#[trigger] m.tasks@[r]) < ntasks
}

/// The gating relation is ranked: every edge rises in rank (so the
/// relation is acyclic).
pub open spec fn ranked(ms: Seq<Member>, gates: spec_fn(int, int) -> bool) -> bool {
    forall|e1: int, e2: int| 0 <= e1 < ms.len() && 0 <= e2 < ms.len() && #[trigger] gates(e1, e2) ==> ms[e1].rank < ms[e2].rank
}

/// Task `t` has a role on member `e` that awaits a message.
pub open spec fn awaits_on(ms: Seq<Member>, cs: Seq<Cfg>, e: int, t: int) -> bool {
    exists|r: int| 0 <= r < ms[e].tasks@.len() && ms[e].tasks@[r] == t && #[trigger] blocked(ms[e].sys, cs[e], r)
}

/// Task `t` has a role on member `e` with an enabled operation.
pub open spec fn able_on(ms: Seq<Member>, cs: Seq<Cfg>, e: int, t: int) -> bool {
    exists|i: int| 0 <= i < ms[e].sys.trans@.len()
        && ms[e].tasks@[(#[trigger] ms[e].sys.trans@[i]).role as int] == t
        && enabled(ms[e].sys.roles as int, cs[e], ms[e].sys.trans@[i])
}

/// Task `t`'s operations on `e` are gated by one of its awaiting roles on
/// an Endpoint that gates `e`.
pub open spec fn gated(ms: Seq<Member>, cs: Seq<Cfg>, gates: spec_fn(int, int) -> bool, e: int, t: int) -> bool {
    exists|e2: int| 0 <= e2 < ms.len() && #[trigger] gates(e2, e) && awaits_on(ms, cs, e2, t)
}

/// R41.3 across Endpoints: a set of Tasks that can never communicate
/// again. `set` is indexed by Task.
pub open spec fn stuck_set(ms: Seq<Member>, cs: Seq<Cfg>, gates: spec_fn(int, int) -> bool, ntasks: int, set: Seq<bool>) -> bool {
    &&& set.len() == ntasks
    // Some Task of the set awaits a message on some Endpoint.
    &&& exists|t: int, e: int| 0 <= t < ntasks && 0 <= e < ms.len() && #[trigger] set[t] && #[trigger] awaits_on(ms, cs, e, t)
    // Every enabled operation of a Task of the set is gated.
    &&& forall|t: int, e: int| 0 <= t < ntasks && 0 <= e < ms.len() && set[t] && #[trigger] able_on(ms, cs, e, t) ==> gated(ms, cs, gates, e, t)
    // The sender of every message a Task of the set awaits is a Task of
    // the set, or a role that has finished.
    &&& forall|e: int, i: int| 0 <= e < ms.len() && 0 <= i < ms[e].sys.trans@.len()
        && set[ms[e].tasks@[(#[trigger] ms[e].sys.trans@[i]).role as int] as int]
        && blocked(ms[e].sys, cs[e], ms[e].sys.trans@[i].role as int)
        && at(cs[e], ms[e].sys.trans@[i])
        ==> set[ms[e].tasks@[ms[e].sys.trans@[i].peer as int] as int] || terminal(ms[e].sys, cs[e], ms[e].sys.trans@[i].peer as int)
}

pub open spec fn system_wf(ms: Seq<Member>, cs: Seq<Cfg>, ntasks: int) -> bool {
    &&& ms.len() == cs.len()
    &&& forall|e: int| 0 <= e < ms.len() ==> member_wf(#[trigger] ms[e], ntasks) && wf_cfg(ms[e].sys.roles as int, cs[e])
}

/// A lowest-ranked member among those satisfying `p` exists when some
/// member does (induction on the rank bound).
proof fn lemma_min_rank(ms: Seq<Member>, p: spec_fn(int) -> bool, bound: nat)
    requires
        exists|e: int| 0 <= e < ms.len() && #[trigger] p(e) && ms[e].rank < bound,
    ensures
        exists|e: int| 0 <= e < ms.len() && #[trigger] p(e) && forall|e2: int| 0 <= e2 < ms.len() && #[trigger] p(e2) ==> ms[e].rank <= ms[e2].rank,
    decreases bound,
{
    let e = choose|e: int| 0 <= e < ms.len() && #[trigger] p(e) && ms[e].rank < bound;
    if exists|e2: int| 0 <= e2 < ms.len() && #[trigger] p(e2) && ms[e2].rank < ms[e].rank {
        lemma_min_rank(ms, p, ms[e].rank as nat);
    } else {
        assert forall|e2: int| 0 <= e2 < ms.len() && #[trigger] p(e2) implies ms[e].rank <= ms[e2].rank by {}
    }
}

/// No stuck set when every member is deadlock-free on its own and the
/// gating relation is ranked.
pub proof fn theorem_system_progress(ms: Seq<Member>, cs: Seq<Cfg>, gates: spec_fn(int, int) -> bool, ntasks: int)
    requires
        system_wf(ms, cs, ntasks),
        ranked(ms, gates),
        forall|e: int| 0 <= e < ms.len() ==> !deadlocked(ms[e].sys, #[trigger] cs[e]),
    ensures
        forall|set: Seq<bool>| !stuck_set(ms, cs, gates, ntasks, set),
{
    assert forall|set: Seq<bool>| !stuck_set(ms, cs, gates, ntasks, set) by {
        if stuck_set(ms, cs, gates, ntasks, set) {
            // The Endpoints on which a Task of the set awaits, and the
            // lowest-ranked among them.
            let p = |e: int| exists|t: int| 0 <= t < ntasks && #[trigger] set[t] && awaits_on(ms, cs, e, t);
            let (t0, e0) = choose|t: int, e: int| 0 <= t < ntasks && 0 <= e < ms.len() && #[trigger] set[t] && #[trigger] awaits_on(ms, cs, e, t);
            assert(p(e0));
            lemma_min_rank(ms, p, ms[e0].rank as nat + 1);
            let e = choose|e: int| 0 <= e < ms.len() && #[trigger] p(e) && forall|e2: int| 0 <= e2 < ms.len() && #[trigger] p(e2) ==> ms[e].rank <= ms[e2].rank;
            let m = ms[e];
            let c = cs[e];
            // The waiting set of member `e`: its roles of the set's Tasks
            // that await, and every finished role.
            let roles = Seq::new(m.sys.roles as nat, |r: int| (set[m.tasks@[r] as int] && blocked(m.sys, c, r)) || terminal(m.sys, c, r));
            // Non-empty with a blocked member.
            let t = choose|t: int| 0 <= t < ntasks && #[trigger] set[t] && awaits_on(ms, cs, e, t);
            let r0 = choose|r: int| 0 <= r < m.tasks@.len() && m.tasks@[r] == t && #[trigger] blocked(m.sys, c, r);
            assert(roles[r0] && blocked(m.sys, c, r0));
            // Closed under the awaited senders.
            assert forall|i: int| 0 <= i < m.sys.trans@.len() && roles[(#[trigger] m.sys.trans@[i]).role as int] && at(c, m.sys.trans@[i])
                implies roles[m.sys.trans@[i].peer as int] by {
                let tr = m.sys.trans@[i];
                let r = tr.role as int;
                // A role with an operation is not finished, so it is a
                // blocked role of a Task of the set.
                assert(has_trans(m.sys, c, r));
                assert(set[m.tasks@[r] as int] && blocked(m.sys, c, r));
                let q = tr.peer as int;
                let u = m.tasks@[q] as int;
                // Its sender is a Task of the set, or finished.
                assert(set[u] || terminal(m.sys, c, q));
                if !terminal(m.sys, c, q) {
                    if !blocked(m.sys, c, q) {
                        // The sender has an enabled operation on `e`: it
                        // is able here, so gated by an awaiting role of
                        // its Task on a lower-ranked member, against the
                        // choice of `e`.
                        let j = choose|j: int| 0 <= j < m.sys.trans@.len() && (#[trigger] m.sys.trans@[j]).role == q && at(c, m.sys.trans@[j])
                            && (m.sys.trans@[j].send || enabled(m.sys.roles as int, c, m.sys.trans@[j]));
                        assert(enabled(m.sys.roles as int, c, m.sys.trans@[j])) by {
                            if m.sys.trans@[j].send {
                                crate::progress::lemma_chan_in_range(m.sys.roles as int, m.sys.trans@[j].role as int, m.sys.trans@[j].peer as int);
                            }
                        }
                        assert(able_on(ms, cs, e, u));
                        assert(gated(ms, cs, gates, e, u));
                        let e2 = choose|e2: int| 0 <= e2 < ms.len() && #[trigger] gates(e2, e) && awaits_on(ms, cs, e2, u);
                        assert(p(e2));
                        assert(ms[e].rank <= ms[e2].rank);
                        assert(ms[e2].rank < ms[e].rank);
                        assert(false);
                    }
                }
            }
            assert(deadlock_set(m.sys, c, roles));
            assert(false);
        }
    }
}

/// R41.3 for a whole system: with a certificate for every member and a
/// ranked gating relation, no combination of reachable member
/// configurations has a stuck set.
pub proof fn theorem_system_r41_3(ms: Seq<Member>, lists: Seq<Seq<Cfg>>, gates: spec_fn(int, int) -> bool, ntasks: int)
    requires
        ms.len() == lists.len(),
        forall|e: int| 0 <= e < ms.len() ==> member_wf(#[trigger] ms[e], ntasks) && certificate(ms[e].sys, lists[e]),
        ranked(ms, gates),
    ensures
        forall|cs: Seq<Cfg>| cs.len() == ms.len() && (forall|e: int| 0 <= e < ms.len() ==> wf_cfg(ms[e].sys.roles as int, cs[e]) && reachable(ms[e].sys, #[trigger] cs[e]))
            ==> forall|set: Seq<bool>| !stuck_set(ms, cs, gates, ntasks, set),
{
    assert forall|cs: Seq<Cfg>| cs.len() == ms.len() && (forall|e: int| 0 <= e < ms.len() ==> wf_cfg(ms[e].sys.roles as int, cs[e]) && reachable(ms[e].sys, #[trigger] cs[e]))
        implies forall|set: Seq<bool>| !stuck_set(ms, cs, gates, ntasks, set) by {
        assert forall|e: int| 0 <= e < ms.len() implies !deadlocked(ms[e].sys, #[trigger] cs[e]) by {
            theorem_r41_3(ms[e].sys, lists[e]);
        }
        theorem_system_progress(ms, cs, gates, ntasks);
    }
}

} // verus!
