//! The multiparty progress theorem of R41.3 (task 21.7): every Protocol of
//! `n` roles that the acceptance conditions over the syntax admit
//! (`theory::acceptn`: closed, guarded, projectable onto every role with
//! the full merge) has no waiting set in any reachable configuration of
//! the asynchronous product of its projections (`theory::nreachable`,
//! `theory::ndeadlocked`).
//!
//! # Runtime global types
//!
//! A reachable configuration is described by a runtime global type `R`:
//! the syntax `G` with one more form, `Sent`, a communication whose
//! message is in transit (the sender has moved on, the receiver has not).
//! Its queue `rqueue` per ordered pair of roles is the sequence of
//! in-transit labels along the first branches (the well-formedness `rwf`
//! makes every branch agree). A step of the configuration is mirrored on
//! the runtime type by `rstep_send` and `rstep_recv`, which rewrite the
//! acting role's first pending node in every branch of every pending
//! choice the role is not part of.
//!
//! # The relation between a role and the runtime type
//!
//! A role that a pending choice does not involve has, as its local type,
//! the merge of the branches' projections; once the choice is taken the
//! runtime type keeps the taken branch alone while the role's type is
//! unchanged, and after further steps the branches of a still-pending
//! choice need not merge any more (the taken choices below it may have
//! left different continuations). So the invariant does not project the
//! runtime type: it relates the role's local type to the runtime type by
//! `fits`, a coinductive relation (a set of pairs closed under
//! `consistent`): the head of the local type is the role's next action at
//! the head of the runtime type, with the continuation pairs again in the
//! relation, and at a choice the role is not part of, the same local type
//! fits every branch. Initially the relation is "the local type refines
//! the merged projection" (`refines`: more labels in external choices),
//! which is consistent because the merge refines each of its arguments;
//! a step rewrites the relation along with the runtime type.
//!
//! # The theorem
//!
//! `minv(c, rt, n)`: `rt` is well formed, each role's type fits it, and
//! each channel is the queue of its pair. It holds initially with `rt` the
//! embedding of the Protocol, every step preserves it (`lemma_step`), and
//! under it no waiting set exists (`lemma_no_deadlock`): an awaiting
//! member of a waiting set is mentioned in `rt`, hence has a first pending
//! node along the first branches at some depth (every branch of a choice
//! agrees on whether it mentions a role); the role at the head of the
//! runtime type can move (the receiver of a `Sent`, the sender of a
//! `Comm`), and it is not a member, so the configuration after its move
//! has the same waiting set and a shallower node for the member; by
//! induction on the depth no such set exists.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use crate::theory::{
    acceptn, all_true, bs_distinct, bs_has_label, bs_len, bs_lookup, closed_at, closed_at_bs, lhead, lhead_n, llead_recs, ls_lookup, ls_remove, lsubst,
    lsubst_ls, lunfold, merge, merge_ls, nblocked, ndeadlock_set, ndeadlocked, ninit, nrecv, nrecv_enabled, nreachable, nrun, nsend, nstep, nterminal, projn,
    projn_bs, projn_merge, wfn, wfn_at, wfn_bs, Bs, G, L, Ls, NC,
};

verus! {

// ------------------------------------------------------------ refinement --

pub open spec fn lsize(l: L) -> nat
    decreases l,
{
    match l {
        L::End => 1,
        L::Var { .. } => 1,
        L::Rec { body } => 1 + lsize(*body),
        L::Send { ls, .. } => 1 + lssize(*ls),
        L::Recv { ls, .. } => 1 + lssize(*ls),
    }
}

pub open spec fn lssize(ls: Ls) -> nat
    decreases ls,
{
    match ls {
        Ls::Nil => 1,
        Ls::Cons { l, rest, .. } => 1 + lsize(l) + lssize(*rest),
    }
}

pub proof fn lemma_lookup_size(ls: Ls, label: int)
    requires ls_lookup(ls, label) is Some,
    ensures lsize(ls_lookup(ls, label).unwrap()) < lssize(ls),
    decreases ls,
{
    if let Ls::Cons { label: l, rest, .. } = ls {
        if l != label {
            lemma_lookup_size(*rest, label);
        }
    }
}

/// `m` refines `a`: `m` is `a` with possibly more labels in its external
/// choices (what the merge does to a role not involved in a choice); a
/// send keeps its labels (its continuations refined), an end or a variable
/// is refined by itself alone.
pub open spec fn refines(m: L, a: L) -> bool
    decreases lsize(m) + lsize(a),
{
    ||| m == a
    ||| (m is Rec && a is Rec && refines(*m->Rec_body, *a->Rec_body))
    ||| (m is Send && a is Send && m->Send_to == a->Send_to && same_labels(*m->Send_ls, *a->Send_ls) && refines_ls(*m->Send_ls, *a->Send_ls))
    ||| (m is Recv && a is Recv && m->Recv_from == a->Recv_from && refines_ls(*m->Recv_ls, *a->Recv_ls))
}

pub open spec fn same_labels(a: Ls, b: Ls) -> bool {
    forall|label: int| (#[trigger] ls_lookup(a, label)) is Some <==> ls_lookup(b, label) is Some
}

/// Every label of `a_ls` is in `m_ls` with a refining continuation.
pub open spec fn refines_ls(m_ls: Ls, a_ls: Ls) -> bool
    decreases lssize(m_ls) + lssize(a_ls),
    via refines_ls_decreases
{
    forall|label: int| ls_lookup(a_ls, label) is Some ==> #[trigger] ls_lookup(m_ls, label) is Some && refines(ls_lookup(m_ls, label).unwrap(), ls_lookup(a_ls, label).unwrap())
}

#[via_fn]
proof fn refines_ls_decreases(m_ls: Ls, a_ls: Ls) {
    assert forall|label: int| #![auto] ls_lookup(a_ls, label) is Some && ls_lookup(m_ls, label) is Some
        implies lsize(ls_lookup(m_ls, label).unwrap()) + lsize(ls_lookup(a_ls, label).unwrap()) < lssize(m_ls) + lssize(a_ls) by {
        lemma_lookup_size(m_ls, label);
        lemma_lookup_size(a_ls, label);
    }
}

pub proof fn lemma_refines_refl(a: L)
    ensures refines(a, a),
{
}

pub proof fn lemma_refines_ls_refl(ls: Ls)
    ensures refines_ls(ls, ls),
{
    assert forall|label: int| ls_lookup(ls, label) is Some implies #[trigger] ls_lookup(ls, label) is Some && refines(ls_lookup(ls, label).unwrap(), ls_lookup(ls, label).unwrap()) by {
    }
}

pub proof fn lemma_refines_trans(a: L, b: L, c: L)
    requires refines(a, b), refines(b, c),
    ensures refines(a, c),
    decreases lsize(a) + lsize(b) + lsize(c),
{
    if a == b || b == c {
    } else if a is Rec {
        lemma_refines_trans(*a->Rec_body, *b->Rec_body, *c->Rec_body);
    } else {
        let la = ls_of(a);
        let lb = ls_of(b);
        let lc = ls_of(c);
        assert(refines_ls(la, lb));
        assert(refines_ls(lb, lc));
        assert forall|label: int| ls_lookup(lc, label) is Some implies #[trigger] ls_lookup(la, label) is Some && refines(ls_lookup(la, label).unwrap(), ls_lookup(lc, label).unwrap()) by {
            assert(ls_lookup(lb, label) is Some && refines(ls_lookup(lb, label).unwrap(), ls_lookup(lc, label).unwrap()));
            assert(ls_lookup(la, label) is Some && refines(ls_lookup(la, label).unwrap(), ls_lookup(lb, label).unwrap()));
            lemma_lookup_size(la, label);
            lemma_lookup_size(lb, label);
            lemma_lookup_size(lc, label);
            lemma_refines_trans(ls_lookup(la, label).unwrap(), ls_lookup(lb, label).unwrap(), ls_lookup(lc, label).unwrap());
        }
        assert(refines_ls(la, lc));
    }
}

/// The branch list of a send or a receive (`Nil` otherwise).
pub open spec fn ls_of(l: L) -> Ls {
    match l {
        L::Send { ls, .. } => *ls,
        L::Recv { ls, .. } => *ls,
        _ => Ls::Nil,
    }
}

/// The kinds a refinement fixes.
pub proof fn lemma_refines_kind(m: L, a: L)
    requires refines(m, a),
    ensures
        a is Send ==> m is Send && m->Send_to == a->Send_to && same_labels(*m->Send_ls, *a->Send_ls) && refines_ls(*m->Send_ls, *a->Send_ls),
        a is End ==> m == a,
        a is Var ==> m == a,
        a is Recv ==> m is Recv && m->Recv_from == a->Recv_from && refines_ls(*m->Recv_ls, *a->Recv_ls),
        a is Rec ==> m is Rec && refines(*m->Rec_body, *a->Rec_body),
        m is Recv ==> a is Recv,
        m is Rec ==> a is Rec,
        m is End ==> a is End,
        m is Send ==> a is Send,
{
    if m == a {
        lemma_refines_ls_refl(ls_of(a));
    }
}

// --------------------------------------- refinement under substitution --

/// The lookup in a substituted list is the substituted lookup.
pub proof fn lemma_lsubst_lookup(ls: Ls, k: nat, by: L)
    ensures forall|label: int| #[trigger] ls_lookup(lsubst_ls(ls, k, by), label) == (match ls_lookup(ls, label) { Some(l) => Some(lsubst(l, k, by)), None => None::<L> }),
    decreases ls,
{
    match ls {
        Ls::Nil => {
            assert forall|label: int| #[trigger] ls_lookup(lsubst_ls(ls, k, by), label) == (match ls_lookup(ls, label) { Some(l) => Some(lsubst(l, k, by)), None => None::<L> }) by {
                assert(lsubst_ls(ls, k, by) == Ls::Nil);
            }
        }
        Ls::Cons { label: l0, l, rest } => {
            lemma_lsubst_lookup(*rest, k, by);
            assert forall|label: int| #[trigger] ls_lookup(lsubst_ls(ls, k, by), label) == (match ls_lookup(ls, label) { Some(l) => Some(lsubst(l, k, by)), None => None::<L> }) by {
                assert(lsubst_ls(ls, k, by) == Ls::Cons { label: l0, l: lsubst(l, k, by), rest: Box::new(lsubst_ls(*rest, k, by)) });
                if l0 != label {
                    assert(ls_lookup(lsubst_ls(ls, k, by), label) == ls_lookup(lsubst_ls(*rest, k, by), label));
                    assert(ls_lookup(ls, label) == ls_lookup(*rest, label));
                }
            }
        }
    }
}

pub proof fn lemma_refines_subst(m: L, a: L, k: nat, bm: L, ba: L)
    requires refines(m, a), refines(bm, ba),
    ensures refines(lsubst(m, k, bm), lsubst(a, k, ba)),
    decreases lsize(m) + lsize(a),
{
    if m == a {
        lemma_refines_subst_same(a, k, bm, ba);
    } else if a is Rec {
        lemma_refines_subst(*m->Rec_body, *a->Rec_body, k + 1, bm, ba);
    } else {
        let ml = ls_of(m);
        let al = ls_of(a);
        assert(refines_ls(ml, al));
        lemma_lsubst_lookup(ml, k, bm);
        lemma_lsubst_lookup(al, k, ba);
        if a is Recv {
            assert(lsubst(m, k, bm) == L::Recv { from: m->Recv_from, ls: Box::new(lsubst_ls(ml, k, bm)) });
            assert(lsubst(a, k, ba) == L::Recv { from: a->Recv_from, ls: Box::new(lsubst_ls(al, k, ba)) });
        } else {
            assert(lsubst(m, k, bm) == L::Send { to: m->Send_to, ls: Box::new(lsubst_ls(ml, k, bm)) });
            assert(lsubst(a, k, ba) == L::Send { to: a->Send_to, ls: Box::new(lsubst_ls(al, k, ba)) });
            assert(same_labels(lsubst_ls(ml, k, bm), lsubst_ls(al, k, ba)));
        }
        assert forall|label: int| ls_lookup(lsubst_ls(al, k, ba), label) is Some implies #[trigger] ls_lookup(lsubst_ls(ml, k, bm), label) is Some
            && refines(ls_lookup(lsubst_ls(ml, k, bm), label).unwrap(), ls_lookup(lsubst_ls(al, k, ba), label).unwrap()) by {
            assert(ls_lookup(al, label) is Some);
            assert(ls_lookup(ml, label) is Some && refines(ls_lookup(ml, label).unwrap(), ls_lookup(al, label).unwrap()));
            lemma_lookup_size(ml, label);
            lemma_lookup_size(al, label);
            lemma_refines_subst(ls_lookup(ml, label).unwrap(), ls_lookup(al, label).unwrap(), k, bm, ba);
        }
        assert(refines_ls(lsubst_ls(ml, k, bm), lsubst_ls(al, k, ba)));
    }
}

proof fn lemma_refines_subst_same(a: L, k: nat, bm: L, ba: L)
    requires refines(bm, ba),
    ensures refines(lsubst(a, k, bm), lsubst(a, k, ba)),
    decreases lsize(a),
{
    match a {
        L::Rec { body } => lemma_refines_subst_same(*body, k + 1, bm, ba),
        L::Send { to, ls } => {
            lemma_lsubst_lookup(*ls, k, bm);
            lemma_lsubst_lookup(*ls, k, ba);
            assert(lsubst(a, k, bm) == L::Send { to, ls: Box::new(lsubst_ls(*ls, k, bm)) });
            assert(lsubst(a, k, ba) == L::Send { to, ls: Box::new(lsubst_ls(*ls, k, ba)) });
            assert forall|label: int| ls_lookup(lsubst_ls(*ls, k, ba), label) is Some implies #[trigger] ls_lookup(lsubst_ls(*ls, k, bm), label) is Some
                && refines(ls_lookup(lsubst_ls(*ls, k, bm), label).unwrap(), ls_lookup(lsubst_ls(*ls, k, ba), label).unwrap()) by {
                assert(ls_lookup(*ls, label) is Some);
                lemma_lookup_size(*ls, label);
                lemma_refines_subst_same(ls_lookup(*ls, label).unwrap(), k, bm, ba);
            }
            assert(refines_ls(lsubst_ls(*ls, k, bm), lsubst_ls(*ls, k, ba)));
            assert(same_labels(lsubst_ls(*ls, k, bm), lsubst_ls(*ls, k, ba)));
        }
        L::Recv { from, ls } => {
            lemma_lsubst_lookup(*ls, k, bm);
            lemma_lsubst_lookup(*ls, k, ba);
            assert(lsubst(a, k, bm) == L::Recv { from, ls: Box::new(lsubst_ls(*ls, k, bm)) });
            assert(lsubst(a, k, ba) == L::Recv { from, ls: Box::new(lsubst_ls(*ls, k, ba)) });
            assert forall|label: int| ls_lookup(lsubst_ls(*ls, k, ba), label) is Some implies #[trigger] ls_lookup(lsubst_ls(*ls, k, bm), label) is Some
                && refines(ls_lookup(lsubst_ls(*ls, k, bm), label).unwrap(), ls_lookup(lsubst_ls(*ls, k, ba), label).unwrap()) by {
                assert(ls_lookup(*ls, label) is Some);
                lemma_lookup_size(*ls, label);
                lemma_refines_subst_same(ls_lookup(*ls, label).unwrap(), k, bm, ba);
            }
            assert(refines_ls(lsubst_ls(*ls, k, bm), lsubst_ls(*ls, k, ba)));
        }
        _ => {}
    }
}

/// The leading recursions of a refinement are those of the refined type.
proof fn lemma_refines_lead(m: L, a: L)
    requires refines(m, a),
    ensures llead_recs(m) == llead_recs(a),
    decreases lsize(m) + lsize(a),
{
    if m != a && a is Rec {
        lemma_refines_lead(*m->Rec_body, *a->Rec_body);
    }
}

/// Refinement is kept by unfolding and by head normalization.
pub proof fn lemma_refines_lhead(m: L, a: L)
    requires refines(m, a),
    ensures refines(lhead(m), lhead(a)),
{
    lemma_refines_lead(m, a);
    lemma_refines_lhead_n(m, a, llead_recs(a));
}

proof fn lemma_refines_lhead_n(m: L, a: L, n: nat)
    requires refines(m, a),
    ensures refines(lhead_n(m, n), lhead_n(a, n)),
    decreases n,
{
    if n > 0 && a is Rec {
        lemma_refines_kind(m, a);
        lemma_refines_subst(*m->Rec_body, *a->Rec_body, 0, m, a);
        lemma_refines_lhead_n(lunfold(m), lunfold(a), (n - 1) as nat);
    }
}

// ------------------------------------------------------------- the merge --

/// A lookup in `ls_remove` is the lookup for every other label.
proof fn lemma_remove_lookup(ls: Ls, label: int)
    ensures
        forall|lab: int| lab != label ==> #[trigger] ls_lookup(ls_remove(ls, label), lab) == ls_lookup(ls, lab),
        ls_lookup(ls_remove(ls, label), label) is None,
    decreases ls,
{
    match ls {
        Ls::Nil => {}
        Ls::Cons { label: l0, l: cont, rest } => {
            lemma_remove_lookup(*rest, label);
            if l0 == label {
                assert(ls_remove(ls, label) == ls_remove(*rest, label));
            } else {
                assert(ls_remove(ls, label) == Ls::Cons { label: l0, l: cont, rest: Box::new(ls_remove(*rest, label)) });
                assert forall|lab: int| lab != label implies #[trigger] ls_lookup(ls_remove(ls, label), lab) == ls_lookup(ls, lab) by {
                    if lab != l0 {
                        assert(ls_lookup(ls_remove(ls, label), lab) == ls_lookup(ls_remove(*rest, label), lab));
                    }
                }
            }
        }
    }
}

/// The merge refines each of its arguments.
pub proof fn lemma_merge_refines(a: L, b: L)
    requires merge(a, b) is Some,
    ensures refines(merge(a, b).unwrap(), a), refines(merge(a, b).unwrap(), b),
    decreases lsize(a) + lsize(b),
{
    if a == b {
    } else if a is Rec {
        lemma_merge_refines(*a->Rec_body, *b->Rec_body);
    } else {
        let la = *a->Recv_ls;
        let lb = *b->Recv_ls;
        lemma_merge_ls_refines(la, lb);
    }
}

/// The merge of two lists has every label of either, with a continuation
/// refining the one it came from.
proof fn lemma_merge_ls_refines(a: Ls, b: Ls)
    requires merge_ls(a, b) is Some,
    ensures refines_ls(merge_ls(a, b).unwrap(), a), refines_ls(merge_ls(a, b).unwrap(), b),
    decreases lssize(a) + lssize(b),
{
    let r = merge_ls(a, b).unwrap();
    match a {
        Ls::Nil => {
            lemma_refines_ls_refl(b);
            assert forall|label: int| ls_lookup(a, label) is Some implies #[trigger] ls_lookup(r, label) is Some && refines(ls_lookup(r, label).unwrap(), ls_lookup(a, label).unwrap()) by {}
        }
        Ls::Cons { label, l, rest } => {
            let b2 = ls_remove(b, label);
            lemma_remove_size(b, label);
            lemma_merge_ls_refines(*rest, b2);
            let r2 = merge_ls(*rest, b2).unwrap();
            let m = match ls_lookup(b, label) { Some(lb) => merge(l, lb).unwrap(), None => l };
            assert(r == Ls::Cons { label, l: m, rest: Box::new(r2) });
            if ls_lookup(b, label) is Some {
                lemma_lookup_size(b, label);
                lemma_merge_refines(l, ls_lookup(b, label).unwrap());
            }
            lemma_remove_lookup(b, label);
            assert forall|lab: int| ls_lookup(a, lab) is Some implies #[trigger] ls_lookup(r, lab) is Some && refines(ls_lookup(r, lab).unwrap(), ls_lookup(a, lab).unwrap()) by {
                if lab != label {
                    assert(ls_lookup(r, lab) == ls_lookup(r2, lab));
                    assert(ls_lookup(a, lab) == ls_lookup(*rest, lab));
                }
            }
            assert forall|lab: int| ls_lookup(b, lab) is Some implies #[trigger] ls_lookup(r, lab) is Some && refines(ls_lookup(r, lab).unwrap(), ls_lookup(b, lab).unwrap()) by {
                if lab != label {
                    assert(ls_lookup(r, lab) == ls_lookup(r2, lab));
                    assert(ls_lookup(b2, lab) == ls_lookup(b, lab));
                }
            }
        }
    }
}

proof fn lemma_remove_size(ls: Ls, label: int)
    ensures lssize(ls_remove(ls, label)) <= lssize(ls),
    decreases ls,
{
    if let Ls::Cons { rest, .. } = ls {
        lemma_remove_size(*rest, label);
    }
}

// ------------------------------------------------------- runtime types --

/// A runtime global type: the syntax with `Sent`, a communication whose
/// message `label` is in transit from `from` to `to`.
pub enum R {
    End,
    Comm { from: int, to: int, bs: Box<RBs> },
    Sent { from: int, to: int, label: int, cont: Box<R> },
    Rec { body: Box<R> },
    Var { n: nat },
}

pub enum RBs {
    Nil,
    Cons { label: int, g: R, rest: Box<RBs> },
}

pub open spec fn emb(g: G) -> R
    decreases g,
{
    match g {
        G::End => R::End,
        G::Var { n } => R::Var { n },
        G::Rec { body } => R::Rec { body: Box::new(emb(*body)) },
        G::Comm { from, to, bs } => R::Comm { from, to, bs: Box::new(emb_bs(*bs)) },
    }
}

pub open spec fn emb_bs(bs: Bs) -> RBs
    decreases bs,
{
    match bs {
        Bs::Nil => RBs::Nil,
        Bs::Cons { label, g, rest } => RBs::Cons { label, g: emb(g), rest: Box::new(emb_bs(*rest)) },
    }
}

pub open spec fn rbs_len(bs: RBs) -> nat
    decreases bs,
{
    match bs {
        RBs::Nil => 0,
        RBs::Cons { rest, .. } => 1 + rbs_len(*rest),
    }
}

pub open spec fn rbs_lookup(bs: RBs, label: int) -> Option<R>
    decreases bs,
{
    match bs {
        RBs::Nil => None,
        RBs::Cons { label: l, g, rest } => if l == label { Some(g) } else { rbs_lookup(*rest, label) },
    }
}

pub open spec fn rbs_distinct(bs: RBs) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { label, rest, .. } => rbs_lookup(*rest, label) is None && rbs_distinct(*rest),
    }
}

/// The first branch's continuation (`End` for an empty list, which
/// well-formedness excludes).
pub open spec fn rbs_first(bs: RBs) -> R {
    match bs {
        RBs::Nil => R::End,
        RBs::Cons { g, .. } => g,
    }
}

/// Well-formedness under a guard environment, as `theory::wfn_at`, a
/// `Sent` guarding like a communication.
pub open spec fn rwf_at(x: R, n: int, env: Seq<bool>) -> bool
    decreases x,
{
    match x {
        R::End => true,
        R::Var { n: v } => v < env.len() && env[env.len() - 1 - v],
        R::Rec { body } => rwf_at(*body, n, env.push(false)),
        R::Comm { from, to, bs } => {
            &&& 0 <= from < n
            &&& 0 <= to < n
            &&& from != to
            &&& rbs_len(*bs) >= 1
            &&& rbs_distinct(*bs)
            &&& rwf_bs(*bs, n, all_true(env))
        }
        R::Sent { from, to, cont, .. } => {
            &&& 0 <= from < n
            &&& 0 <= to < n
            &&& from != to
            &&& rwf_at(*cont, n, all_true(env))
        }
    }
}

pub open spec fn rwf_bs(bs: RBs, n: int, env: Seq<bool>) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => rwf_at(g, n, env) && rwf_bs(*rest, n, env),
    }
}

pub open spec fn rwf(x: R, n: int) -> bool {
    rwf_at(x, n, Seq::<bool>::empty())
}

pub open spec fn rclosed_at(x: R, m: nat) -> bool
    decreases x,
{
    match x {
        R::End => true,
        R::Var { n } => n < m,
        R::Rec { body } => rclosed_at(*body, m + 1),
        R::Comm { bs, .. } => rclosed_at_bs(*bs, m),
        R::Sent { cont, .. } => rclosed_at(*cont, m),
    }
}

pub open spec fn rclosed_at_bs(bs: RBs, m: nat) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => rclosed_at(g, m) && rclosed_at_bs(*rest, m),
    }
}

pub open spec fn rsubst(x: R, k: nat, by: R) -> R
    decreases x,
{
    match x {
        R::End => R::End,
        R::Var { n } => if n == k { by } else if n > k { R::Var { n: (n - 1) as nat } } else { R::Var { n } },
        R::Rec { body } => R::Rec { body: Box::new(rsubst(*body, k + 1, by)) },
        R::Comm { from, to, bs } => R::Comm { from, to, bs: Box::new(rsubst_bs(*bs, k, by)) },
        R::Sent { from, to, label, cont } => R::Sent { from, to, label, cont: Box::new(rsubst(*cont, k, by)) },
    }
}

pub open spec fn rsubst_bs(bs: RBs, k: nat, by: R) -> RBs
    decreases bs,
{
    match bs {
        RBs::Nil => RBs::Nil,
        RBs::Cons { label, g, rest } => RBs::Cons { label, g: rsubst(g, k, by), rest: Box::new(rsubst_bs(*rest, k, by)) },
    }
}

pub open spec fn runfold(x: R) -> R {
    match x {
        R::Rec { body } => rsubst(*body, 0, x),
        _ => x,
    }
}

pub open spec fn rlead_recs(x: R) -> nat
    decreases x,
{
    match x {
        R::Rec { body } => 1 + rlead_recs(*body),
        _ => 0,
    }
}

pub open spec fn rspine_ok(x: R) -> bool
    decreases x,
{
    match x {
        R::Rec { body } => rspine_ok(*body),
        R::Var { .. } => false,
        _ => true,
    }
}

/// The head normal form by at most `k` unfoldings.
pub open spec fn rhead_n(x: R, k: nat) -> R
    decreases k,
{
    if k == 0 || !(x is Rec) { x } else { rhead_n(runfold(x), (k - 1) as nat) }
}

pub open spec fn rhead(x: R) -> R {
    rhead_n(x, rlead_recs(x))
}

proof fn lemma_rwf_spine(x: R, n: int, env: Seq<bool>)
    requires rwf_at(x, n, env), forall|i: int| 0 <= i < env.len() ==> !env[i],
    ensures rspine_ok(x),
    decreases x,
{
    if let R::Rec { body } = x {
        lemma_rwf_spine(*body, n, env.push(false));
    }
}

proof fn lemma_rsubst_spine(x: R, k: nat, by: R)
    requires rspine_ok(x),
    ensures rspine_ok(rsubst(x, k, by)), rlead_recs(rsubst(x, k, by)) == rlead_recs(x),
    decreases x,
{
    if let R::Rec { body } = x {
        lemma_rsubst_spine(*body, k + 1, by);
    }
}

proof fn lemma_rwf_closed_at(x: R, n: int, env: Seq<bool>)
    requires rwf_at(x, n, env),
    ensures rclosed_at(x, env.len()),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_rwf_closed_at(*body, n, env.push(false)),
        R::Comm { bs, .. } => lemma_rwf_closed_at_bs(*bs, n, all_true(env)),
        R::Sent { cont, .. } => lemma_rwf_closed_at(*cont, n, all_true(env)),
        _ => {}
    }
}

proof fn lemma_rwf_closed_at_bs(bs: RBs, n: int, env: Seq<bool>)
    requires rwf_bs(bs, n, env),
    ensures rclosed_at_bs(bs, env.len()),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_rwf_closed_at(g, n, env);
        lemma_rwf_closed_at_bs(*rest, n, env);
    }
}

proof fn lemma_renv_swap(x: R, n: int, outer1: Seq<bool>, outer2: Seq<bool>, inner: Seq<bool>)
    requires rwf_at(x, n, outer1.add(inner)), rclosed_at(x, inner.len()),
    ensures rwf_at(x, n, outer2.add(inner)),
    decreases x,
{
    match x {
        R::End => {}
        R::Var { n: v } => {
            let e1 = outer1.add(inner);
            let e2 = outer2.add(inner);
            assert(e1[e1.len() - 1 - v] == inner[inner.len() - 1 - v]);
            assert(e2[e2.len() - 1 - v] == inner[inner.len() - 1 - v]);
        }
        R::Rec { body } => {
            assert(outer1.add(inner).push(false) =~= outer1.add(inner.push(false)));
            assert(outer2.add(inner).push(false) =~= outer2.add(inner.push(false)));
            lemma_renv_swap(*body, n, outer1, outer2, inner.push(false));
        }
        R::Comm { bs, .. } => {
            assert(all_true(outer1.add(inner)) =~= all_true(outer1).add(all_true(inner)));
            assert(all_true(outer2.add(inner)) =~= all_true(outer2).add(all_true(inner)));
            lemma_renv_swap_bs(*bs, n, all_true(outer1), all_true(outer2), all_true(inner));
        }
        R::Sent { cont, .. } => {
            assert(all_true(outer1.add(inner)) =~= all_true(outer1).add(all_true(inner)));
            assert(all_true(outer2.add(inner)) =~= all_true(outer2).add(all_true(inner)));
            lemma_renv_swap(*cont, n, all_true(outer1), all_true(outer2), all_true(inner));
        }
    }
}

proof fn lemma_renv_swap_bs(bs: RBs, n: int, outer1: Seq<bool>, outer2: Seq<bool>, inner: Seq<bool>)
    requires rwf_bs(bs, n, outer1.add(inner)), rclosed_at_bs(bs, inner.len()),
    ensures rwf_bs(bs, n, outer2.add(inner)),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_renv_swap(g, n, outer1, outer2, inner);
        lemma_renv_swap_bs(*rest, n, outer1, outer2, inner);
    }
}

proof fn lemma_rclosed_any_env(by: R, n: int, env: Seq<bool>)
    requires rwf(by, n),
    ensures rwf_at(by, n, env),
{
    lemma_rwf_closed_at(by, n, Seq::<bool>::empty());
    assert(Seq::<bool>::empty().add(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
    assert(env.add(Seq::<bool>::empty()) =~= env);
    lemma_renv_swap(by, n, Seq::<bool>::empty(), env, Seq::<bool>::empty());
}

proof fn lemma_rsubst_wf(x: R, n: int, e0: bool, rest: Seq<bool>, by: R)
    requires rwf_at(x, n, seq![e0].add(rest)), rwf(by, n),
    ensures rwf_at(rsubst(x, rest.len() as nat, by), n, rest),
    decreases x,
{
    let env = seq![e0].add(rest);
    match x {
        R::End => {}
        R::Var { n: v } => {
            if v == rest.len() {
                lemma_rclosed_any_env(by, n, rest);
            } else {
                assert(v < rest.len());
                assert(env[env.len() - 1 - v] == rest[rest.len() - 1 - v]);
            }
        }
        R::Rec { body } => {
            assert(env.push(false) =~= seq![e0].add(rest.push(false)));
            lemma_rsubst_wf(*body, n, e0, rest.push(false), by);
        }
        R::Comm { bs, .. } => {
            assert(all_true(env) =~= seq![true].add(all_true(rest)));
            lemma_rsubst_wf_bs(*bs, n, true, all_true(rest), by);
            lemma_rsubst_bs_shape(*bs, rest.len() as nat, by);
        }
        R::Sent { cont, .. } => {
            assert(all_true(env) =~= seq![true].add(all_true(rest)));
            lemma_rsubst_wf(*cont, n, true, all_true(rest), by);
        }
    }
}

proof fn lemma_rsubst_wf_bs(bs: RBs, n: int, e0: bool, rest: Seq<bool>, by: R)
    requires rwf_bs(bs, n, seq![e0].add(rest)), rwf(by, n),
    ensures rwf_bs(rsubst_bs(bs, rest.len() as nat, by), n, rest),
    decreases bs,
{
    if let RBs::Cons { g, rest: tail, .. } = bs {
        lemma_rsubst_wf(g, n, e0, rest, by);
        lemma_rsubst_wf_bs(*tail, n, e0, rest, by);
    }
}

/// Substitution keeps the labels of a branch list.
pub proof fn lemma_rsubst_bs_shape(bs: RBs, k: nat, by: R)
    ensures
        rbs_len(rsubst_bs(bs, k, by)) == rbs_len(bs),
        rbs_distinct(rsubst_bs(bs, k, by)) == rbs_distinct(bs),
        forall|label: int| #[trigger] rbs_lookup(rsubst_bs(bs, k, by), label) == (match rbs_lookup(bs, label) { Some(g) => Some(rsubst(g, k, by)), None => None::<R> }),
    decreases bs,
{
    match bs {
        RBs::Nil => {
            assert forall|label: int| #[trigger] rbs_lookup(rsubst_bs(bs, k, by), label) == (match rbs_lookup(bs, label) { Some(g) => Some(rsubst(g, k, by)), None => None::<R> }) by {
                assert(rsubst_bs(bs, k, by) == RBs::Nil);
            }
        }
        RBs::Cons { label: l0, g, rest } => {
            lemma_rsubst_bs_shape(*rest, k, by);
            assert(rsubst_bs(bs, k, by) == RBs::Cons { label: l0, g: rsubst(g, k, by), rest: Box::new(rsubst_bs(*rest, k, by)) });
            assert forall|label: int| #[trigger] rbs_lookup(rsubst_bs(bs, k, by), label) == (match rbs_lookup(bs, label) { Some(g) => Some(rsubst(g, k, by)), None => None::<R> }) by {
                if l0 != label {
                    assert(rbs_lookup(rsubst_bs(bs, k, by), label) == rbs_lookup(rsubst_bs(*rest, k, by), label));
                    assert(rbs_lookup(bs, label) == rbs_lookup(*rest, label));
                }
            }
        }
    }
}

/// Unfolding a closed well-formed recursion gives a closed well-formed
/// type with one recursion fewer at the top.
pub proof fn lemma_runfold_wf(x: R, n: int)
    requires rwf(x, n), x is Rec,
    ensures rwf(runfold(x), n), rlead_recs(runfold(x)) == rlead_recs(x) - 1,
{
    let body = *x->Rec_body;
    assert(Seq::<bool>::empty().push(false) =~= seq![false].add(Seq::<bool>::empty()));
    lemma_rsubst_wf(body, n, false, Seq::<bool>::empty(), x);
    lemma_rwf_spine(body, n, seq![false]);
    lemma_rsubst_spine(body, 0, x);
}

pub proof fn lemma_rhead_wf(x: R, n: int)
    requires rwf(x, n),
    ensures rwf(rhead(x), n), !(rhead(x) is Rec), !(rhead(x) is Var),
{
    lemma_rhead_n_wf(x, n, rlead_recs(x));
}

proof fn lemma_rhead_n_wf(x: R, n: int, k: nat)
    requires rwf(x, n), rlead_recs(x) == k,
    ensures rwf(rhead_n(x, k), n), !(rhead_n(x, k) is Rec), !(rhead_n(x, k) is Var),
    decreases k,
{
    if x is Rec {
        lemma_runfold_wf(x, n);
        lemma_rhead_n_wf(runfold(x), n, (k - 1) as nat);
    } else {
        lemma_rwf_spine(x, n, Seq::<bool>::empty());
    }
}

/// A branch of a closed well-formed communication is closed and well
/// formed.
pub proof fn lemma_rbranch_wf(bs: RBs, n: int, label: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rbs_lookup(bs, label) is Some,
    ensures rwf(rbs_lookup(bs, label).unwrap(), n),
    decreases bs,
{
    if let RBs::Cons { label: l, rest, .. } = bs {
        if l != label {
            lemma_rbranch_wf(*rest, n, label);
        }
    }
}

pub proof fn lemma_rcomm_branches_wf(x: R, n: int)
    requires rwf(x, n), x is Comm,
    ensures rwf_bs(*x->Comm_bs, n, Seq::<bool>::empty()),
{
    assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
}

pub proof fn lemma_rsent_cont_wf(x: R, n: int)
    requires rwf(x, n), x is Sent,
    ensures rwf(*x->Sent_cont, n),
{
    assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
}

// ---------------------------------------------------- the embedding --

pub proof fn lemma_emb_wf(g: G, n: int, env: Seq<bool>)
    ensures wfn_at(g, n, env) == rwf_at(emb(g), n, env),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_emb_wf(*body, n, env.push(false)),
        G::Comm { bs, .. } => {
            lemma_emb_wf_bs(*bs, n, all_true(env));
            lemma_emb_bs_shape(*bs);
        }
        _ => {}
    }
}

proof fn lemma_emb_wf_bs(bs: Bs, n: int, env: Seq<bool>)
    ensures wfn_bs(bs, n, env) == rwf_bs(emb_bs(bs), n, env),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_emb_wf(g, n, env);
        lemma_emb_wf_bs(*rest, n, env);
    }
}

pub proof fn lemma_emb_bs_shape(bs: Bs)
    ensures
        rbs_len(emb_bs(bs)) == bs_len(bs),
        rbs_distinct(emb_bs(bs)) == bs_distinct(bs),
        forall|label: int| #[trigger] rbs_lookup(emb_bs(bs), label) == (match bs_lookup(bs, label) { Some(g) => Some(emb(g)), None => None::<R> }),
        forall|label: int| #[trigger] bs_has_label(bs, label) == (bs_lookup(bs, label) is Some),
    decreases bs,
{
    match bs {
        Bs::Nil => {
            assert forall|label: int| #[trigger] rbs_lookup(emb_bs(bs), label) == (match bs_lookup(bs, label) { Some(g) => Some(emb(g)), None => None::<R> }) by {
                assert(emb_bs(bs) == RBs::Nil);
            }
        }
        Bs::Cons { label: l0, g, rest } => {
            lemma_emb_bs_shape(*rest);
            assert(emb_bs(bs) == RBs::Cons { label: l0, g: emb(g), rest: Box::new(emb_bs(*rest)) });
            assert forall|label: int| #[trigger] rbs_lookup(emb_bs(bs), label) == (match bs_lookup(bs, label) { Some(g) => Some(emb(g)), None => None::<R> }) by {
                if l0 != label {
                    assert(rbs_lookup(emb_bs(bs), label) == rbs_lookup(emb_bs(*rest), label));
                    assert(bs_lookup(bs, label) == bs_lookup(*rest, label));
                }
            }
            assert forall|label: int| #[trigger] bs_has_label(bs, label) == (bs_lookup(bs, label) is Some) by {
                if l0 != label {
                    assert(bs_has_label(bs, label) == bs_has_label(*rest, label));
                    assert(bs_lookup(bs, label) == bs_lookup(*rest, label));
                }
            }
        }
    }
}

// ------------------------------------------- the merged projection --

/// The projection of a runtime type onto role `u` with the full merge
/// (`theory::projn` on the syntax; a `Sent` is, for its receiver, the
/// external choice of the one label in transit, and for anyone else the
/// continuation). Used for the initial relation alone.
pub open spec fn rproj(x: R, u: int) -> Option<L>
    decreases x,
{
    match x {
        R::End => Some(L::End),
        R::Var { n } => Some(L::Var { n }),
        R::Rec { body } => match rproj(*body, u) {
            Some(l) => Some(L::Rec { body: Box::new(l) }),
            None => None,
        },
        R::Comm { from, to, bs } => {
            if u == from {
                match rproj_bs(*bs, u) {
                    Some(ls) => Some(L::Send { to, ls: Box::new(ls) }),
                    None => None,
                }
            } else if u == to {
                match rproj_bs(*bs, u) {
                    Some(ls) => Some(L::Recv { from, ls: Box::new(ls) }),
                    None => None,
                }
            } else {
                rproj_merge(*bs, u)
            }
        }
        R::Sent { from, to, label, cont } => match rproj(*cont, u) {
            None => None,
            Some(l) => if u == to { Some(L::Recv { from, ls: Box::new(Ls::Cons { label, l, rest: Box::new(Ls::Nil) }) }) } else { Some(l) },
        },
    }
}

pub open spec fn rproj_bs(bs: RBs, u: int) -> Option<Ls>
    decreases bs,
{
    match bs {
        RBs::Nil => Some(Ls::Nil),
        RBs::Cons { label, g, rest } => match (rproj(g, u), rproj_bs(*rest, u)) {
            (Some(l), Some(ls)) => Some(Ls::Cons { label, l, rest: Box::new(ls) }),
            _ => None,
        },
    }
}

pub open spec fn rproj_merge(bs: RBs, u: int) -> Option<L>
    decreases bs,
{
    match bs {
        RBs::Nil => None,
        RBs::Cons { g, rest, .. } => match rproj(g, u) {
            None => None,
            Some(l) => match *rest {
                RBs::Nil => Some(l),
                _ => match rproj_merge(*rest, u) {
                    Some(m) => merge(l, m),
                    None => None,
                },
            },
        },
    }
}

pub proof fn lemma_rproj_emb(g: G, u: int)
    ensures rproj(emb(g), u) == projn(g, u),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_rproj_emb(*body, u),
        G::Comm { bs, .. } => {
            lemma_rproj_emb_bs(*bs, u);
            lemma_rproj_emb_merge(*bs, u);
        }
        _ => {}
    }
}

proof fn lemma_rproj_emb_bs(bs: Bs, u: int)
    ensures rproj_bs(emb_bs(bs), u) == projn_bs(bs, u),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_rproj_emb(g, u);
        lemma_rproj_emb_bs(*rest, u);
    }
}

proof fn lemma_rproj_emb_merge(bs: Bs, u: int)
    ensures rproj_merge(emb_bs(bs), u) == projn_merge(bs, u),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_rproj_emb(g, u);
        lemma_rproj_emb_merge(*rest, u);
    }
}

/// The lookups of a projected branch list are the projections of the
/// branches.
pub proof fn lemma_rproj_bs_lookup(bs: RBs, u: int)
    requires rproj_bs(bs, u) is Some,
    ensures
        forall|label: int| #[trigger] rbs_lookup(bs, label) is Some ==> rproj(rbs_lookup(bs, label).unwrap(), u) is Some
            && ls_lookup(rproj_bs(bs, u).unwrap(), label) == Some(rproj(rbs_lookup(bs, label).unwrap(), u).unwrap()),
        forall|label: int| #[trigger] rbs_lookup(bs, label) is None ==> ls_lookup(rproj_bs(bs, u).unwrap(), label) is None,
    decreases bs,
{
    if let RBs::Cons { label: l0, g, rest } = bs {
        lemma_rproj_bs_lookup(*rest, u);
        let ls = rproj_bs(bs, u).unwrap();
        assert(ls == Ls::Cons { label: l0, l: rproj(g, u).unwrap(), rest: Box::new(rproj_bs(*rest, u).unwrap()) });
        assert forall|label: int| #[trigger] rbs_lookup(bs, label) is Some implies rproj(rbs_lookup(bs, label).unwrap(), u) is Some
            && ls_lookup(ls, label) == Some(rproj(rbs_lookup(bs, label).unwrap(), u).unwrap()) by {
            if l0 != label {
                assert(rbs_lookup(bs, label) == rbs_lookup(*rest, label));
                assert(ls_lookup(ls, label) == ls_lookup(rproj_bs(*rest, u).unwrap(), label));
            }
        }
        assert forall|label: int| #[trigger] rbs_lookup(bs, label) is None implies ls_lookup(ls, label) is None by {
            assert(rbs_lookup(bs, label) == rbs_lookup(*rest, label));
            assert(ls_lookup(ls, label) == ls_lookup(rproj_bs(*rest, u).unwrap(), label));
        }
    }
}

/// The merge of the branches' projections refines each branch's.
pub proof fn lemma_rproj_merge_refines(bs: RBs, u: int)
    requires rproj_merge(bs, u) is Some,
    ensures forall|label: int| #[trigger] rbs_lookup(bs, label) is Some ==> rproj(rbs_lookup(bs, label).unwrap(), u) is Some
        && refines(rproj_merge(bs, u).unwrap(), rproj(rbs_lookup(bs, label).unwrap(), u).unwrap()),
    decreases bs,
{
    if let RBs::Cons { label: l0, g, rest } = bs {
        let m = rproj_merge(bs, u).unwrap();
        let l = rproj(g, u).unwrap();
        if *rest is Nil {
            assert forall|label: int| #[trigger] rbs_lookup(bs, label) is Some implies rproj(rbs_lookup(bs, label).unwrap(), u) is Some
                && refines(m, rproj(rbs_lookup(bs, label).unwrap(), u).unwrap()) by {
                assert(rbs_lookup(*rest, label) is None);
                assert(label == l0);
                assert(m == l);
            }
        } else {
            let mr = rproj_merge(*rest, u).unwrap();
            lemma_rproj_merge_refines(*rest, u);
            lemma_merge_refines(l, mr);
            assert forall|label: int| #[trigger] rbs_lookup(bs, label) is Some implies rproj(rbs_lookup(bs, label).unwrap(), u) is Some
                && refines(m, rproj(rbs_lookup(bs, label).unwrap(), u).unwrap()) by {
                if l0 != label {
                    assert(rbs_lookup(bs, label) == rbs_lookup(*rest, label));
                    lemma_refines_trans(m, mr, rproj(rbs_lookup(bs, label).unwrap(), u).unwrap());
                }
            }
        }
    }
}

// --------------------------------------------------------- distinctness --

pub open spec fn ldistinct(l: L) -> bool
    decreases l,
{
    match l {
        L::Rec { body } => ldistinct(*body),
        L::Send { ls, .. } => lsdistinct(*ls),
        L::Recv { ls, .. } => lsdistinct(*ls),
        _ => true,
    }
}

pub open spec fn lsdistinct(ls: Ls) -> bool
    decreases ls,
{
    match ls {
        Ls::Nil => true,
        Ls::Cons { label, l, rest } => ls_lookup(*rest, label) is None && ldistinct(l) && lsdistinct(*rest),
    }
}

/// Distinctness is the distinctness of the lookups.
proof fn lemma_lsdistinct_lookup(ls: Ls, label: int)
    requires lsdistinct(ls), ls_lookup(ls, label) is Some,
    ensures ldistinct(ls_lookup(ls, label).unwrap()),
    decreases ls,
{
    if let Ls::Cons { label: l0, rest, .. } = ls {
        if l0 != label {
            lemma_lsdistinct_lookup(*rest, label);
        }
    }
}

proof fn lemma_lsubst_distinct(l: L, k: nat, by: L)
    requires ldistinct(l), ldistinct(by),
    ensures ldistinct(lsubst(l, k, by)),
    decreases l,
{
    match l {
        L::Rec { body } => lemma_lsubst_distinct(*body, k + 1, by),
        L::Send { ls, .. } => lemma_lsubst_ls_distinct(*ls, k, by),
        L::Recv { ls, .. } => lemma_lsubst_ls_distinct(*ls, k, by),
        _ => {}
    }
}

proof fn lemma_lsubst_ls_distinct(ls: Ls, k: nat, by: L)
    requires lsdistinct(ls), ldistinct(by),
    ensures lsdistinct(lsubst_ls(ls, k, by)),
    decreases ls,
{
    if let Ls::Cons { label, l, rest } = ls {
        lemma_lsubst_distinct(l, k, by);
        lemma_lsubst_ls_distinct(*rest, k, by);
        lemma_lsubst_lookup(*rest, k, by);
    }
}

/// A label of the merge of two lists is a label of one of them.
proof fn lemma_merge_ls_labels(a: Ls, b: Ls)
    requires merge_ls(a, b) is Some,
    ensures forall|label: int| #[trigger] ls_lookup(merge_ls(a, b).unwrap(), label) is Some ==> ls_lookup(a, label) is Some || ls_lookup(b, label) is Some,
    decreases a,
{
    if let Ls::Cons { label: l0, l, rest } = a {
        let b2 = ls_remove(b, l0);
        lemma_merge_ls_labels(*rest, b2);
        lemma_remove_lookup(b, l0);
        let r = merge_ls(a, b).unwrap();
        let r2 = merge_ls(*rest, b2).unwrap();
        assert forall|label: int| #[trigger] ls_lookup(r, label) is Some implies ls_lookup(a, label) is Some || ls_lookup(b, label) is Some by {
            if label != l0 {
                assert(ls_lookup(r, label) == ls_lookup(r2, label));
            }
        }
    }
}

proof fn lemma_remove_distinct(ls: Ls, label: int)
    requires lsdistinct(ls),
    ensures lsdistinct(ls_remove(ls, label)),
    decreases ls,
{
    if let Ls::Cons { label: l0, rest, .. } = ls {
        lemma_remove_distinct(*rest, label);
        lemma_remove_lookup(*rest, label);
    }
}

pub proof fn lemma_merge_distinct(a: L, b: L)
    requires merge(a, b) is Some, ldistinct(a), ldistinct(b),
    ensures ldistinct(merge(a, b).unwrap()),
    decreases lsize(a) + lsize(b),
{
    if a == b {
    } else if a is Rec {
        lemma_merge_distinct(*a->Rec_body, *b->Rec_body);
    } else {
        lemma_merge_ls_distinct(*a->Recv_ls, *b->Recv_ls);
    }
}

proof fn lemma_merge_ls_distinct(a: Ls, b: Ls)
    requires merge_ls(a, b) is Some, lsdistinct(a), lsdistinct(b),
    ensures lsdistinct(merge_ls(a, b).unwrap()),
    decreases lssize(a) + lssize(b),
{
    if let Ls::Cons { label, l, rest } = a {
        let b2 = ls_remove(b, label);
        lemma_remove_size(b, label);
        lemma_remove_distinct(b, label);
        lemma_merge_ls_distinct(*rest, b2);
        lemma_merge_ls_labels(*rest, b2);
        lemma_remove_lookup(b, label);
        if ls_lookup(b, label) is Some {
            lemma_lookup_size(b, label);
            lemma_lsdistinct_lookup(b, label);
            lemma_merge_distinct(l, ls_lookup(b, label).unwrap());
        }
    }
}

// ------------------------------- the merge commutes with substitution --

proof fn lemma_remove_subst(ls: Ls, label: int, k: nat, by: L)
    ensures ls_remove(lsubst_ls(ls, k, by), label) == lsubst_ls(ls_remove(ls, label), k, by),
    decreases ls,
{
    if let Ls::Cons { rest, .. } = ls {
        lemma_remove_subst(*rest, label, k, by);
    }
}

/// Two distinct types that merge stay distinct under substitution (their
/// difference is in their labels or below equal labels, never a variable
/// against a term).
proof fn lemma_merge_neq_subst(a: L, b: L, k: nat, by: L)
    requires merge(a, b) is Some, a != b, ldistinct(a), ldistinct(b),
    ensures lsubst(a, k, by) != lsubst(b, k, by),
    decreases lsize(a) + lsize(b),
{
    if a is Rec {
        lemma_merge_neq_subst(*a->Rec_body, *b->Rec_body, k + 1, by);
    } else {
        assert(a is Recv && b is Recv && a->Recv_from == b->Recv_from);
        lemma_merge_ls_neq_subst(*a->Recv_ls, *b->Recv_ls, k, by);
        assert(lsubst(a, k, by) == L::Recv { from: a->Recv_from, ls: Box::new(lsubst_ls(*a->Recv_ls, k, by)) });
        assert(lsubst(b, k, by) == L::Recv { from: b->Recv_from, ls: Box::new(lsubst_ls(*b->Recv_ls, k, by)) });
    }
}

proof fn lemma_merge_ls_neq_subst(a: Ls, b: Ls, k: nat, by: L)
    requires merge_ls(a, b) is Some, a != b, lsdistinct(a), lsdistinct(b),
    ensures lsubst_ls(a, k, by) != lsubst_ls(b, k, by),
    decreases lssize(a) + lssize(b),
{
    match a {
        Ls::Nil => {}
        Ls::Cons { label, l, rest } => {
            match b {
                Ls::Nil => {}
                Ls::Cons { label: lb0, l: lbl, rest: lbrest } => {
                    if label == lb0 {
                        assert(ls_lookup(b, label) == Some(lbl));
                        if l != lbl {
                            lemma_merge_neq_subst(l, lbl, k, by);
                        } else {
                            assert(ls_remove(b, label) == ls_remove(*lbrest, label));
                            lemma_remove_absent(*lbrest, label);
                            lemma_merge_ls_neq_subst(*rest, *lbrest, k, by);
                        }
                    }
                }
            }
        }
    }
}

proof fn lemma_remove_absent(ls: Ls, label: int)
    requires ls_lookup(ls, label) is None,
    ensures ls_remove(ls, label) == ls,
    decreases ls,
{
    if let Ls::Cons { rest, .. } = ls {
        lemma_remove_absent(*rest, label);
    }
}

pub proof fn lemma_merge_subst(a: L, b: L, k: nat, by: L)
    requires merge(a, b) is Some, ldistinct(a), ldistinct(b),
    ensures merge(lsubst(a, k, by), lsubst(b, k, by)) == Some(lsubst(merge(a, b).unwrap(), k, by)),
    decreases lsize(a) + lsize(b),
{
    if a == b {
    } else {
        lemma_merge_neq_subst(a, b, k, by);
        if a is Rec {
            lemma_merge_subst(*a->Rec_body, *b->Rec_body, k + 1, by);
            assert(lsubst(a, k, by) == L::Rec { body: Box::new(lsubst(*a->Rec_body, k + 1, by)) });
            assert(lsubst(b, k, by) == L::Rec { body: Box::new(lsubst(*b->Rec_body, k + 1, by)) });
        } else {
            assert(a is Recv && b is Recv && a->Recv_from == b->Recv_from);
            let la = *a->Recv_ls;
            let lb = *b->Recv_ls;
            lemma_merge_ls_subst(la, lb, k, by);
            assert(lsubst(a, k, by) == L::Recv { from: a->Recv_from, ls: Box::new(lsubst_ls(la, k, by)) });
            assert(lsubst(b, k, by) == L::Recv { from: b->Recv_from, ls: Box::new(lsubst_ls(lb, k, by)) });
            assert(merge(a, b) == Some(L::Recv { from: a->Recv_from, ls: Box::new(merge_ls(la, lb).unwrap()) }));
        }
    }
}

proof fn lemma_merge_ls_subst(a: Ls, b: Ls, k: nat, by: L)
    requires merge_ls(a, b) is Some, lsdistinct(a), lsdistinct(b),
    ensures merge_ls(lsubst_ls(a, k, by), lsubst_ls(b, k, by)) == Some(lsubst_ls(merge_ls(a, b).unwrap(), k, by)),
    decreases lssize(a) + lssize(b),
{
    if let Ls::Cons { label, l, rest } = a {
        let b2 = ls_remove(b, label);
        lemma_remove_size(b, label);
        lemma_remove_distinct(b, label);
        lemma_lsubst_lookup(b, k, by);
        lemma_remove_subst(b, label, k, by);
        lemma_merge_ls_subst(*rest, b2, k, by);
        if ls_lookup(b, label) is Some {
            lemma_lookup_size(b, label);
            lemma_lsdistinct_lookup(b, label);
            lemma_merge_subst(l, ls_lookup(b, label).unwrap(), k, by);
        }
    }
}

// -------------------------------- the projection and the head normal form --

/// The projection of a well-formed runtime type has distinct labels.
pub proof fn lemma_rproj_distinct(x: R, n: int, env: Seq<bool>, u: int)
    requires rwf_at(x, n, env), rproj(x, u) is Some,
    ensures ldistinct(rproj(x, u).unwrap()),
    decreases x,
{
    match x {
        R::Rec { body } => {
            lemma_rproj_distinct(*body, n, env.push(false), u);
            assert(rproj(x, u) == Some(L::Rec { body: Box::new(rproj(*body, u).unwrap()) }));
        }
        R::Comm { from, to, bs } => {
            if u == from || u == to {
                lemma_rproj_bs_distinct(*bs, n, all_true(env), u);
                let ls = rproj_bs(*bs, u).unwrap();
                if u == from {
                    assert(rproj(x, u) == Some(L::Send { to, ls: Box::new(ls) }));
                } else {
                    assert(rproj(x, u) == Some(L::Recv { from, ls: Box::new(ls) }));
                }
            } else {
                lemma_rproj_merge_distinct(*bs, n, all_true(env), u);
            }
        }
        R::Sent { from, to, label, cont } => {
            lemma_rproj_distinct(*cont, n, all_true(env), u);
            let l = rproj(*cont, u).unwrap();
            if u == to {
                assert(rproj(x, u) == Some(L::Recv { from, ls: Box::new(Ls::Cons { label, l, rest: Box::new(Ls::Nil) }) }));
                assert(ls_lookup(Ls::Nil, label) is None);
                assert(lsdistinct(Ls::Nil));
                assert(lsdistinct(Ls::Cons { label, l, rest: Box::new(Ls::Nil) }));
            } else {
                assert(rproj(x, u) == Some(l));
            }
        }
        R::End => { assert(rproj(x, u) == Some(L::End)); }
        R::Var { n: v } => { assert(rproj(x, u) == Some(L::Var { n: v })); }
    }
}

proof fn lemma_rproj_bs_distinct(bs: RBs, n: int, env: Seq<bool>, u: int)
    requires rwf_bs(bs, n, env), rbs_distinct(bs), rproj_bs(bs, u) is Some,
    ensures lsdistinct(rproj_bs(bs, u).unwrap()),
    decreases bs,
{
    if let RBs::Cons { label, g, rest } = bs {
        lemma_rproj_distinct(g, n, env, u);
        lemma_rproj_bs_distinct(*rest, n, env, u);
        lemma_rproj_bs_lookup(*rest, u);
    }
}

proof fn lemma_rproj_merge_distinct(bs: RBs, n: int, env: Seq<bool>, u: int)
    requires rwf_bs(bs, n, env), rproj_merge(bs, u) is Some,
    ensures ldistinct(rproj_merge(bs, u).unwrap()),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_rproj_distinct(g, n, env, u);
        if !(*rest is Nil) {
            lemma_rproj_merge_distinct(*rest, n, env, u);
            lemma_merge_distinct(rproj(g, u).unwrap(), rproj_merge(*rest, u).unwrap());
        }
    }
}

/// Projection commutes with substitution.
pub proof fn lemma_rproj_subst(x: R, n: int, env: Seq<bool>, k: nat, by: R, u: int)
    requires rwf_at(x, n, env), rwf(by, n), rproj(x, u) is Some, rproj(by, u) is Some,
    ensures rproj(rsubst(x, k, by), u) == Some(lsubst(rproj(x, u).unwrap(), k, rproj(by, u).unwrap())),
    decreases x,
{
    let lby = rproj(by, u).unwrap();
    match x {
        R::Rec { body } => {
            lemma_rproj_subst(*body, n, env.push(false), k + 1, by, u);
            let lb = rproj(*body, u).unwrap();
            assert(rproj(x, u) == Some(L::Rec { body: Box::new(lb) }));
            assert(rsubst(x, k, by) == R::Rec { body: Box::new(rsubst(*body, k + 1, by)) });
            assert(rproj(rsubst(x, k, by), u) == Some(L::Rec { body: Box::new(lsubst(lb, k + 1, lby)) }));
            assert(rproj(rsubst(x, k, by), u) == Some(lsubst(rproj(x, u).unwrap(), k, lby)));
        }
        R::Comm { from, to, bs } => {
            assert(rsubst(x, k, by) == R::Comm { from, to, bs: Box::new(rsubst_bs(*bs, k, by)) });
            if u == from || u == to {
                lemma_rproj_bs_subst(*bs, n, all_true(env), k, by, u);
                let ls = rproj_bs(*bs, u).unwrap();
                if u == from {
                    assert(rproj(x, u) == Some(L::Send { to, ls: Box::new(ls) }));
                    assert(rproj(rsubst(x, k, by), u) == Some(L::Send { to, ls: Box::new(lsubst_ls(ls, k, lby)) }));
                } else {
                    assert(rproj(x, u) == Some(L::Recv { from, ls: Box::new(ls) }));
                    assert(rproj(rsubst(x, k, by), u) == Some(L::Recv { from, ls: Box::new(lsubst_ls(ls, k, lby)) }));
                }
            } else {
                lemma_rproj_merge_subst(*bs, n, all_true(env), k, by, u);
            }
            assert(rproj(rsubst(x, k, by), u) == Some(lsubst(rproj(x, u).unwrap(), k, lby)));
        }
        R::Sent { from, to, label, cont } => {
            lemma_rproj_subst(*cont, n, all_true(env), k, by, u);
            let l = rproj(*cont, u).unwrap();
            assert(rsubst(x, k, by) == R::Sent { from, to, label, cont: Box::new(rsubst(*cont, k, by)) });
            if u == to {
                assert(rproj(x, u) == Some(L::Recv { from, ls: Box::new(Ls::Cons { label, l, rest: Box::new(Ls::Nil) }) }));
                assert(rproj(rsubst(x, k, by), u) == Some(L::Recv { from, ls: Box::new(Ls::Cons { label, l: lsubst(l, k, lby), rest: Box::new(Ls::Nil) }) }));
                assert(lsubst_ls(Ls::Nil, k, lby) == Ls::Nil);
                assert(lsubst_ls(Ls::Cons { label, l, rest: Box::new(Ls::Nil) }, k, lby) == Ls::Cons { label, l: lsubst(l, k, lby), rest: Box::new(Ls::Nil) });
            } else {
                assert(rproj(x, u) == Some(l));
            }
            assert(rproj(rsubst(x, k, by), u) == Some(lsubst(rproj(x, u).unwrap(), k, lby)));
        }
        R::Var { n: v } => {
            assert(rproj(x, u) == Some(L::Var { n: v }));
            if v == k {
                assert(rsubst(x, k, by) == by);
                assert(lsubst(L::Var { n: v }, k, lby) == lby);
            } else if v > k {
                assert(rsubst(x, k, by) == R::Var { n: (v - 1) as nat });
                assert(lsubst(L::Var { n: v }, k, lby) == L::Var { n: (v - 1) as nat });
            } else {
                assert(rsubst(x, k, by) == x);
                assert(lsubst(L::Var { n: v }, k, lby) == L::Var { n: v });
            }
            assert(rproj(rsubst(x, k, by), u) == Some(lsubst(rproj(x, u).unwrap(), k, lby)));
        }
        R::End => { assert(rsubst(x, k, by) == R::End); assert(rproj(x, u) == Some(L::End)); }
    }
}

proof fn lemma_rproj_bs_subst(bs: RBs, n: int, env: Seq<bool>, k: nat, by: R, u: int)
    requires rwf_bs(bs, n, env), rwf(by, n), rproj_bs(bs, u) is Some, rproj(by, u) is Some,
    ensures rproj_bs(rsubst_bs(bs, k, by), u) == Some(lsubst_ls(rproj_bs(bs, u).unwrap(), k, rproj(by, u).unwrap())),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_rproj_subst(g, n, env, k, by, u);
        lemma_rproj_bs_subst(*rest, n, env, k, by, u);
    }
}

proof fn lemma_rproj_merge_subst(bs: RBs, n: int, env: Seq<bool>, k: nat, by: R, u: int)
    requires rwf_bs(bs, n, env), rwf(by, n), rproj_merge(bs, u) is Some, rproj(by, u) is Some,
    ensures rproj_merge(rsubst_bs(bs, k, by), u) == Some(lsubst(rproj_merge(bs, u).unwrap(), k, rproj(by, u).unwrap())),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_rproj_subst(g, n, env, k, by, u);
        if *rest is Nil {
            assert(rsubst_bs(*rest, k, by) is Nil);
        } else {
            lemma_rproj_merge_subst(*rest, n, env, k, by, u);
            lemma_rproj_distinct(g, n, env, u);
            lemma_rproj_merge_distinct(*rest, n, env, u);
            lemma_merge_subst(rproj(g, u).unwrap(), rproj_merge(*rest, u).unwrap(), k, rproj(by, u).unwrap());
            assert(!(rsubst_bs(*rest, k, by) is Nil));
        }
    }
}


// ------------------------------------------------ the canonical head --

/// The spine of a local type ends in an action or an end, never in a
/// variable (the projection of a vacuous recursion, `rec X { continue X }`
/// for a role the body never mentions, has a variable spine).
pub open spec fn lspine_ok(l: L) -> bool
    decreases l,
{
    match l {
        L::Rec { body } => lspine_ok(*body),
        L::Var { .. } => false,
        _ => true,
    }
}

/// The canonical form of a type with no action: the vacuous loop.
pub open spec fn stuck() -> L {
    L::Rec { body: Box::new(L::Var { n: 0 }) }
}

/// The head used by the invariant: the head normal form, with every
/// variable-spined type (no action: the semantics' `lhead` of it is a
/// recursion or a variable, neither a send, a receive, nor an end) taken
/// to the one canonical form.
pub open spec fn mhead(l: L) -> L {
    if lspine_ok(l) { lhead(l) } else { stuck() }
}

proof fn lemma_lsubst_spine(l: L, k: nat, by: L)
    requires lspine_ok(l),
    ensures lspine_ok(lsubst(l, k, by)), llead_recs(lsubst(l, k, by)) == llead_recs(l),
    decreases l,
{
    if let L::Rec { body } = l {
        lemma_lsubst_spine(*body, k + 1, by);
    }
}

proof fn lemma_lsubst_varspine(l: L, k: nat, by: L)
    requires !lspine_ok(l), !lspine_ok(by),
    ensures !lspine_ok(lsubst(l, k, by)),
    decreases l,
{
    if let L::Rec { body } = l {
        lemma_lsubst_varspine(*body, k + 1, by);
    }
}

/// `lhead_n` keeps a variable spine, so `lhead` of such a type is no action.
proof fn lemma_lhead_n_varspine(l: L, k: nat)
    requires !lspine_ok(l),
    ensures !lspine_ok(lhead_n(l, k)),
    decreases k,
{
    if k > 0 && l is Rec {
        lemma_lsubst_varspine(*l->Rec_body, 0, l);
        lemma_lhead_n_varspine(lunfold(l), (k - 1) as nat);
    }
}

/// The semantics' head and the invariant's agree on every action.
pub proof fn lemma_mhead_lhead(l: L)
    ensures
        lspine_ok(l) ==> mhead(l) == lhead(l),
        !lspine_ok(l) ==> mhead(l) == stuck() && !(lhead(l) is Send) && !(lhead(l) is Recv) && !(lhead(l) is End),
{
    if !lspine_ok(l) {
        lemma_lhead_n_varspine(l, llead_recs(l));
    }
}

/// The head of a non-recursion is itself.
pub proof fn lemma_mhead_flat(l: L)
    requires !(l is Rec), !(l is Var),
    ensures mhead(l) == l, lhead(l) == l,
{
}

/// Unfolding keeps the head.
pub proof fn lemma_mhead_unfold(l: L)
    requires l is Rec,
    ensures mhead(lunfold(l)) == mhead(l),
{
    let body = *l->Rec_body;
    if lspine_ok(l) {
        lemma_lsubst_spine(body, 0, l);
        assert(llead_recs(lunfold(l)) == llead_recs(l) - 1);
        assert(lhead(l) == lhead_n(lunfold(l), (llead_recs(l) - 1) as nat));
    } else {
        lemma_lsubst_varspine(body, 0, l);
    }
}

proof fn lemma_refines_spine(m: L, a: L)
    requires refines(m, a),
    ensures lspine_ok(m) == lspine_ok(a),
    decreases lsize(m) + lsize(a),
{
    if m != a && a is Rec {
        lemma_refines_spine(*m->Rec_body, *a->Rec_body);
    }
}

pub proof fn lemma_refines_mhead(m: L, a: L)
    requires refines(m, a),
    ensures refines(mhead(m), mhead(a)),
{
    lemma_refines_spine(m, a);
    if lspine_ok(a) {
        lemma_refines_lhead(m, a);
    }
}

/// The projection of the head normal form has the head of the projection.
pub proof fn lemma_mhead_rproj(x: R, n: int, u: int)
    requires rwf(x, n), rproj(x, u) is Some,
    ensures rproj(rhead(x), u) is Some, mhead(rproj(rhead(x), u).unwrap()) == mhead(rproj(x, u).unwrap()),
    decreases rlead_recs(x),
{
    if x is Rec {
        lemma_runfold_wf(x, n);
        let body = *x->Rec_body;
        let l = rproj(x, u).unwrap();
        assert(Seq::<bool>::empty().push(false) =~= seq![false]);
        lemma_rproj_subst(body, n, seq![false], 0, x, u);
        assert(rproj(runfold(x), u) == Some(lunfold(l)));
        lemma_mhead_unfold(l);
        lemma_mhead_rproj(runfold(x), n, u);
        assert(rhead_n(x, rlead_recs(x)) == rhead_n(runfold(x), rlead_recs(runfold(x))));
    }
}

// ---------------------------------------------- actions and reaching --

/// Every path of `x` (through recursions and every branch of every
/// choice) meets a node pending for role `u` (a communication it sends or
/// receives, or a message in transit to it) before an end or a variable.
pub open spec fn reaches(x: R, u: int) -> bool
    decreases x,
{
    match x {
        R::End => false,
        R::Var { .. } => false,
        R::Rec { body } => reaches(*body, u),
        R::Comm { from, to, bs } => from == u || to == u || reaches_bs(*bs, u),
        R::Sent { to, cont, .. } => to == u || reaches(*cont, u),
    }
}

/// Every branch reaches `u` (an empty list, which well-formedness
/// excludes, does not: it has no path).
pub open spec fn reaches_bs(bs: RBs, u: int) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => false,
        RBs::Cons { g, rest, .. } => reaches(g, u) && (*rest is Nil || reaches_bs(*rest, u)),
    }
}

/// A local type with no action.
pub open spec fn actionless(l: L) -> bool
    decreases l,
{
    match l {
        L::End => true,
        L::Var { .. } => true,
        L::Rec { body } => actionless(*body),
        _ => false,
    }
}

proof fn lemma_merge_actionless(a: L, b: L)
    requires merge(a, b) is Some, actionless(a) || actionless(b),
    ensures actionless(merge(a, b).unwrap()), actionless(a), actionless(b),
    decreases lsize(a) + lsize(b),
{
    if a != b {
        lemma_merge_actionless(*a->Rec_body, *b->Rec_body);
    }
}

/// A role whose projection has an action is reached on every path.
proof fn lemma_action_reaches(x: R, u: int)
    requires rproj(x, u) is Some, !actionless(rproj(x, u).unwrap()),
    ensures reaches(x, u),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_action_reaches(*body, u),
        R::Comm { from, to, bs } => {
            if u != from && u != to {
                lemma_action_reaches_merge(*bs, u);
            }
        }
        R::Sent { to, cont, .. } => {
            if u != to {
                lemma_action_reaches(*cont, u);
            }
        }
        _ => {}
    }
}

proof fn lemma_action_reaches_merge(bs: RBs, u: int)
    requires rproj_merge(bs, u) is Some, !actionless(rproj_merge(bs, u).unwrap()),
    ensures reaches_bs(bs, u),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        if *rest is Nil {
            assert(rproj_merge(bs, u) == rproj(g, u));
            lemma_action_reaches(g, u);
        } else {
            let l = rproj(g, u).unwrap();
            let m = rproj_merge(*rest, u).unwrap();
            assert(rproj_merge(bs, u) == merge(l, m));
            if actionless(l) || actionless(m) {
                lemma_merge_actionless(l, m);
                assert(false);
            }
            lemma_action_reaches(g, u);
            lemma_action_reaches_merge(*rest, u);
        }
    }
}

proof fn lemma_lsubst_actionless(l: L, k: nat, by: L)
    requires actionless(l), actionless(by),
    ensures actionless(lsubst(l, k, by)),
    decreases l,
{
    if let L::Rec { body } = l {
        lemma_lsubst_actionless(*body, k + 1, by);
    }
}

proof fn lemma_lhead_n_actionless(l: L, k: nat)
    requires actionless(l),
    ensures actionless(lhead_n(l, k)),
    decreases k,
{
    if k > 0 && l is Rec {
        lemma_lsubst_actionless(*l->Rec_body, 0, l);
        lemma_lhead_n_actionless(lunfold(l), (k - 1) as nat);
    }
}

/// The head of a type with no action is an end or the canonical loop.
pub proof fn lemma_actionless_mhead(l: L)
    requires actionless(l),
    ensures mhead(l) is End || mhead(l) == stuck(),
{
    if lspine_ok(l) {
        lemma_lhead_n_actionless(l, llead_recs(l));
        lemma_lhead_spine(l);
    }
}

/// The head normal form of a spine-ok type is not a recursion.
pub proof fn lemma_lhead_spine(l: L)
    requires lspine_ok(l),
    ensures !(lhead(l) is Rec), !(lhead(l) is Var), lspine_ok(lhead(l)),
{
    lemma_lhead_n_spine(l, llead_recs(l));
}

proof fn lemma_lhead_n_spine(l: L, k: nat)
    requires lspine_ok(l), llead_recs(l) == k,
    ensures !(lhead_n(l, k) is Rec), !(lhead_n(l, k) is Var), lspine_ok(lhead_n(l, k)),
    decreases k,
{
    if l is Rec {
        lemma_lsubst_spine(*l->Rec_body, 0, l);
        lemma_lhead_n_spine(lunfold(l), (k - 1) as nat);
    }
}

/// A head with an action, in the invariant's sense.
pub open spec fn acting(hm: L) -> bool {
    hm is Send || hm is Recv
}

// --------------------------------------------------------- the relation --

pub open spec fn labels_eq(ls: Ls, bs: RBs) -> bool {
    forall|label: int| (#[trigger] ls_lookup(ls, label)) is Some <==> rbs_lookup(bs, label) is Some
}

pub open spec fn labels_sup(ls: Ls, bs: RBs) -> bool {
    forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> ls_lookup(ls, label) is Some
}

/// The conditions on one pair of a consistent relation: the runtime type
/// is well formed; a role with an action is reached on every path of it;
/// and at its head the role's next action is the head's, with the
/// continuation pairs in the relation, or, at a choice the role is not
/// part of, the same local type is paired with every branch.
pub open spec fn node_ok(rel: ISet<(L, R)>, m: L, x: R, u: int, n: int) -> bool {
    let h = rhead(x);
    let hm = mhead(m);
    &&& rwf(x, n)
    &&& (acting(hm) ==> reaches(x, u))
    &&& match h {
        R::End => hm is End,
        R::Comm { from, to, bs } => {
            if u == from {
                &&& hm is Send
                &&& hm->Send_to == to
                &&& labels_eq(*hm->Send_ls, *bs)
                &&& forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some ==> rel.contains((ls_lookup(*hm->Send_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap()))
            } else if u == to {
                &&& hm is Recv
                &&& hm->Recv_from == from
                &&& labels_sup(*hm->Recv_ls, *bs)
                &&& forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some ==> rel.contains((ls_lookup(*hm->Recv_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap()))
            } else {
                forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some ==> rel.contains((m, rbs_lookup(*bs, label).unwrap()))
            }
        }
        R::Sent { from, to, label, cont } => {
            if u == to {
                &&& hm is Recv
                &&& hm->Recv_from == from
                &&& ls_lookup(*hm->Recv_ls, label) is Some
                &&& rel.contains((ls_lookup(*hm->Recv_ls, label).unwrap(), *cont))
            } else {
                rel.contains((m, *cont))
            }
        }
        _ => false,
    }
}

pub open spec fn consistent(rel: ISet<(L, R)>, u: int, n: int) -> bool {
    forall|m: L, x: R| #[trigger] rel.contains((m, x)) ==> node_ok(rel, m, x, u, n)
}

/// The local type `m` of role `u` fits the runtime type `x`.
pub open spec fn fits(m: L, x: R, u: int, n: int) -> bool {
    exists|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((m, x))
}

// ----------------------------------------------- the initial relation --

/// The relation of the initial configuration: the role's head refines the
/// head of the merged projection.
pub open spec fn rel0_member(m: L, x: R, u: int, n: int) -> bool {
    &&& rwf(x, n)
    &&& rproj(x, u) is Some
    &&& refines(mhead(m), mhead(rproj(x, u).unwrap()))
}

pub open spec fn rel0(u: int, n: int) -> ISet<(L, R)> {
    ISet::new(|p: (L, R)| rel0_member(p.0, p.1, u, n))
}

pub proof fn lemma_rel0_consistent(u: int, n: int)
    ensures consistent(rel0(u, n), u, n),
{
    let rel = rel0(u, n);
    assert forall|m: L, x: R| #[trigger] rel.contains((m, x)) implies node_ok(rel, m, x, u, n) by {
        lemma_rel0_node(m, x, u, n);
    }
}

proof fn lemma_rel0_node(m: L, x: R, u: int, n: int)
    requires rel0(u, n).contains((m, x)),
    ensures node_ok(rel0(u, n), m, x, u, n),
{
    let rel = rel0(u, n);
    let h = rhead(x);
    let hm = mhead(m);
    let l = rproj(x, u).unwrap();
    lemma_rhead_wf(x, n);
    lemma_mhead_rproj(x, n, u);
    let lh = rproj(h, u).unwrap();
    assert(refines(hm, mhead(lh)));
    lemma_refines_kind(hm, mhead(lh));
    if acting(hm) {
        if actionless(l) {
            lemma_actionless_mhead(l);
            lemma_refines_kind(hm, mhead(l));
            assert(false);
        }
        lemma_action_reaches(x, u);
    }
    match h {
        R::End => {
            assert(lh == L::End);
            lemma_mhead_flat(lh);
        }
        R::Comm { from, to, bs } => {
            lemma_rcomm_branches_wf(h, n);
            if u == from || u == to {
                let ls = rproj_bs(*bs, u).unwrap();
                lemma_mhead_flat(lh);
                lemma_rproj_bs_lookup(*bs, u);
                let hls = if u == from { *hm->Send_ls } else { *hm->Recv_ls };
                assert(refines_ls(hls, ls));
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel.contains((ls_lookup(hls, label).unwrap(), rbs_lookup(*bs, label).unwrap())) by {
                    let b = rbs_lookup(*bs, label).unwrap();
                    lemma_rbranch_wf(*bs, n, label);
                    assert(ls_lookup(ls, label) == Some(rproj(b, u).unwrap()));
                    assert(ls_lookup(hls, label) is Some && refines(ls_lookup(hls, label).unwrap(), rproj(b, u).unwrap()));
                    lemma_refines_mhead(ls_lookup(hls, label).unwrap(), rproj(b, u).unwrap());
                }
                if u == from {
                    assert(labels_eq(hls, *bs));
                } else {
                    assert(labels_sup(hls, *bs));
                }
            } else {
                lemma_rproj_merge_refines(*bs, u);
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel.contains((m, rbs_lookup(*bs, label).unwrap())) by {
                    let b = rbs_lookup(*bs, label).unwrap();
                    lemma_rbranch_wf(*bs, n, label);
                    lemma_refines_mhead(lh, rproj(b, u).unwrap());
                    lemma_refines_trans(hm, mhead(lh), mhead(rproj(b, u).unwrap()));
                }
            }
        }
        R::Sent { from, to, label, cont } => {
            lemma_rsent_cont_wf(h, n);
            let lc = rproj(*cont, u).unwrap();
            if u == to {
                assert(lh == L::Recv { from, ls: Box::new(Ls::Cons { label, l: lc, rest: Box::new(Ls::Nil) }) });
                lemma_mhead_flat(lh);
                let hls = *hm->Recv_ls;
                assert(ls_lookup(Ls::Cons { label, l: lc, rest: Box::new(Ls::Nil) }, label) == Some(lc));
                assert(ls_lookup(hls, label) is Some && refines(ls_lookup(hls, label).unwrap(), lc));
                lemma_refines_mhead(ls_lookup(hls, label).unwrap(), lc);
            } else {
                assert(lh == lc);
            }
        }
        _ => {}
    }
}

// ------------------------------------------- queues and the structure --

/// The labels in transit from `p` to `q`, along the first branches.
pub open spec fn rqueue(x: R, p: int, q: int) -> Seq<int>
    decreases x,
{
    match x {
        R::End => Seq::<int>::empty(),
        R::Var { .. } => Seq::<int>::empty(),
        R::Rec { body } => rqueue(*body, p, q),
        R::Comm { bs, .. } => rqueue_bs(*bs, p, q),
        R::Sent { from, to, label, cont } => (if from == p && to == q { seq![label] } else { Seq::<int>::empty() }).add(rqueue(*cont, p, q)),
    }
}

pub open spec fn rqueue_bs(bs: RBs, p: int, q: int) -> Seq<int>
    decreases bs,
{
    match bs {
        RBs::Nil => Seq::<int>::empty(),
        RBs::Cons { g, .. } => rqueue(g, p, q),
    }
}

pub open spec fn no_sent(x: R) -> bool
    decreases x,
{
    match x {
        R::Rec { body } => no_sent(*body),
        R::Comm { bs, .. } => no_sent_bs(*bs),
        R::Sent { .. } => false,
        _ => true,
    }
}

pub open spec fn no_sent_bs(bs: RBs) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => no_sent(g) && no_sent_bs(*rest),
    }
}

/// No message in transit from role `s`.
pub open spec fn no_sent_from(x: R, s: int) -> bool
    decreases x,
{
    match x {
        R::Rec { body } => no_sent_from(*body, s),
        R::Comm { bs, .. } => no_sent_from_bs(*bs, s),
        R::Sent { from, cont, .. } => from != s && no_sent_from(*cont, s),
        _ => true,
    }
}

pub open spec fn no_sent_from_bs(bs: RBs, s: int) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => no_sent_from(g, s) && no_sent_from_bs(*rest, s),
    }
}

/// Every branch has the queues of the first.
pub open spec fn queues_agree_bs(bs: RBs) -> bool {
    forall|label: int, p: int, q: int| rbs_lookup(bs, label) is Some ==> #[trigger] rqueue(rbs_lookup(bs, label).unwrap(), p, q) == rqueue_bs(bs, p, q)
}

/// The structure of a runtime type beyond well-formedness: no message in
/// transit inside a recursion (a step unfolds every recursion above it),
/// none from the sender of a pending communication below it (a role's
/// sends are in order), and the branches of every choice agreeing on
/// their queues (a step rewrites every branch alike).
pub open spec fn rgood(x: R) -> bool
    decreases x,
{
    match x {
        R::Rec { body } => no_sent(*body),
        R::Comm { from, bs, .. } => no_sent_from_bs(*bs, from) && queues_agree_bs(*bs) && rgood_bs(*bs),
        R::Sent { cont, .. } => rgood(*cont),
        _ => true,
    }
}

pub open spec fn rgood_bs(bs: RBs) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => rgood(g) && rgood_bs(*rest),
    }
}

proof fn lemma_no_sent_queue(x: R)
    requires no_sent(x),
    ensures forall|p: int, q: int| #[trigger] rqueue(x, p, q) == Seq::<int>::empty(),
    decreases x,
{
    match x {
        R::Rec { body } => {
            lemma_no_sent_queue(*body);
            assert forall|p: int, q: int| #[trigger] rqueue(x, p, q) == Seq::<int>::empty() by { assert(rqueue(x, p, q) == rqueue(*body, p, q)); }
        }
        R::Comm { bs, .. } => {
            lemma_no_sent_queue_bs(*bs);
            assert forall|p: int, q: int| #[trigger] rqueue(x, p, q) == Seq::<int>::empty() by { assert(rqueue(x, p, q) == rqueue_bs(*bs, p, q)); }
        }
        R::End => {
            assert forall|p: int, q: int| #[trigger] rqueue(x, p, q) == Seq::<int>::empty() by {}
        }
        R::Var { .. } => {
            assert forall|p: int, q: int| #[trigger] rqueue(x, p, q) == Seq::<int>::empty() by {}
        }
        R::Sent { .. } => {}
    }
}

proof fn lemma_no_sent_queue_bs(bs: RBs)
    requires no_sent_bs(bs),
    ensures forall|p: int, q: int| #[trigger] rqueue_bs(bs, p, q) == Seq::<int>::empty(),
    decreases bs,
{
    match bs {
        RBs::Nil => {
            assert forall|p: int, q: int| #[trigger] rqueue_bs(bs, p, q) == Seq::<int>::empty() by {}
        }
        RBs::Cons { g, .. } => {
            lemma_no_sent_queue(g);
            assert forall|p: int, q: int| #[trigger] rqueue_bs(bs, p, q) == Seq::<int>::empty() by { assert(rqueue_bs(bs, p, q) == rqueue(g, p, q)); }
        }
    }
}

proof fn lemma_no_sent_from_queue(x: R, s: int)
    requires no_sent_from(x, s),
    ensures forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty(),
    decreases x,
{
    match x {
        R::Rec { body } => {
            lemma_no_sent_from_queue(*body, s);
            assert forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty() by { assert(rqueue(x, s, q) == rqueue(*body, s, q)); }
        }
        R::Comm { bs, .. } => {
            lemma_no_sent_from_queue_bs(*bs, s);
            assert forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty() by { assert(rqueue(x, s, q) == rqueue_bs(*bs, s, q)); }
        }
        R::Sent { cont, .. } => {
            lemma_no_sent_from_queue(*cont, s);
            assert forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty() by {
                assert(rqueue(x, s, q) == Seq::<int>::empty().add(rqueue(*cont, s, q)));
                assert(Seq::<int>::empty().add(Seq::<int>::empty()) =~= Seq::<int>::empty());
            }
        }
        R::End => {
            assert forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty() by {}
        }
        R::Var { .. } => {
            assert forall|q: int| #[trigger] rqueue(x, s, q) == Seq::<int>::empty() by {}
        }
    }
}

proof fn lemma_no_sent_from_queue_bs(bs: RBs, s: int)
    requires no_sent_from_bs(bs, s),
    ensures forall|q: int| #[trigger] rqueue_bs(bs, s, q) == Seq::<int>::empty(),
    decreases bs,
{
    match bs {
        RBs::Nil => {
            assert forall|q: int| #[trigger] rqueue_bs(bs, s, q) == Seq::<int>::empty() by {}
        }
        RBs::Cons { g, .. } => {
            lemma_no_sent_from_queue(g, s);
            assert forall|q: int| #[trigger] rqueue_bs(bs, s, q) == Seq::<int>::empty() by { assert(rqueue_bs(bs, s, q) == rqueue(g, s, q)); }
        }
    }
}

proof fn lemma_no_sent_lookup(bs: RBs, label: int)
    requires no_sent_bs(bs), rbs_lookup(bs, label) is Some,
    ensures no_sent(rbs_lookup(bs, label).unwrap()),
    decreases bs,
{
    if let RBs::Cons { label: l0, rest, .. } = bs {
        if l0 != label {
            lemma_no_sent_lookup(*rest, label);
        }
    }
}

proof fn lemma_no_sent_rgood(x: R)
    requires no_sent(x),
    ensures rgood(x),
    decreases x,
{
    match x {
        R::Comm { from, bs, .. } => {
            lemma_no_sent_rgood_bs(*bs);
            lemma_no_sent_queue_bs(*bs);
            assert forall|label: int, p: int, q: int| rbs_lookup(*bs, label) is Some implies #[trigger] rqueue(rbs_lookup(*bs, label).unwrap(), p, q) == rqueue_bs(*bs, p, q) by {
                lemma_no_sent_lookup(*bs, label);
                lemma_no_sent_queue(rbs_lookup(*bs, label).unwrap());
            }
        }
        _ => {}
    }
}

proof fn lemma_no_sent_rgood_bs(bs: RBs)
    requires no_sent_bs(bs),
    ensures rgood_bs(bs), forall|s: int| #[trigger] no_sent_from_bs(bs, s),
    decreases bs,
{
    match bs {
        RBs::Nil => {}
        RBs::Cons { g, rest, .. } => {
            lemma_no_sent_rgood(g);
            lemma_no_sent_rgood_bs(*rest);
            lemma_no_sent_from_any(g);
            assert forall|s: int| #[trigger] no_sent_from_bs(bs, s) by {
                assert(no_sent_from(g, s));
                assert(no_sent_from_bs(*rest, s));
            }
        }
    }
}

proof fn lemma_no_sent_from_any(x: R)
    requires no_sent(x),
    ensures forall|s: int| #[trigger] no_sent_from(x, s),
    decreases x,
{
    match x {
        R::Rec { body } => {
            lemma_no_sent_from_any(*body);
            assert forall|s: int| #[trigger] no_sent_from(x, s) by { assert(no_sent_from(*body, s)); }
        }
        R::Comm { bs, .. } => {
            lemma_no_sent_from_any_bs(*bs);
            assert forall|s: int| #[trigger] no_sent_from(x, s) by { assert(no_sent_from_bs(*bs, s)); }
        }
        _ => {}
    }
}

proof fn lemma_no_sent_from_any_bs(bs: RBs)
    requires no_sent_bs(bs),
    ensures forall|s: int| #[trigger] no_sent_from_bs(bs, s),
    decreases bs,
{
    match bs {
        RBs::Nil => {}
        RBs::Cons { g, rest, .. } => {
            lemma_no_sent_from_any(g);
            lemma_no_sent_from_any_bs(*rest);
            assert forall|s: int| #[trigger] no_sent_from_bs(bs, s) by {
                assert(no_sent_from(g, s));
                assert(no_sent_from_bs(*rest, s));
            }
        }
    }
}

proof fn lemma_no_sent_subst(x: R, k: nat, by: R)
    requires no_sent(x), no_sent(by),
    ensures no_sent(rsubst(x, k, by)),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_no_sent_subst(*body, k + 1, by),
        R::Comm { bs, .. } => lemma_no_sent_subst_bs(*bs, k, by),
        _ => {}
    }
}

proof fn lemma_no_sent_subst_bs(bs: RBs, k: nat, by: R)
    requires no_sent_bs(bs), no_sent(by),
    ensures no_sent_bs(rsubst_bs(bs, k, by)),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_no_sent_subst(g, k, by);
        lemma_no_sent_subst_bs(*rest, k, by);
    }
}

/// Head normalization keeps the structure and the queues.
pub proof fn lemma_rhead_good(x: R, n: int)
    requires rgood(x), rwf(x, n),
    ensures rgood(rhead(x)), forall|p: int, q: int| #[trigger] rqueue(rhead(x), p, q) == rqueue(x, p, q),
    decreases rlead_recs(x),
{
    if x is Rec {
        let body = *x->Rec_body;
        lemma_no_sent_subst(body, 0, x);
        lemma_no_sent_rgood(runfold(x));
        lemma_no_sent_queue(runfold(x));
        lemma_no_sent_queue(body);
        lemma_runfold_wf(x, n);
        lemma_rhead_good(runfold(x), n);
        assert(rhead_n(x, rlead_recs(x)) == rhead_n(runfold(x), rlead_recs(runfold(x))));
        assert forall|p: int, q: int| #[trigger] rqueue(rhead(x), p, q) == rqueue(x, p, q) by {
            assert(rqueue(x, p, q) == rqueue(body, p, q));
            assert(rqueue(rhead(x), p, q) == rqueue(runfold(x), p, q));
        }
    }
}

/// A branch of a good communication is good.
pub proof fn lemma_rgood_branch(bs: RBs, label: int)
    requires rgood_bs(bs), rbs_lookup(bs, label) is Some,
    ensures rgood(rbs_lookup(bs, label).unwrap()),
    decreases bs,
{
    if let RBs::Cons { label: l0, rest, .. } = bs {
        if l0 != label {
            lemma_rgood_branch(*rest, label);
        }
    }
}

pub proof fn lemma_no_sent_from_branch(bs: RBs, s: int, label: int)
    requires no_sent_from_bs(bs, s), rbs_lookup(bs, label) is Some,
    ensures no_sent_from(rbs_lookup(bs, label).unwrap(), s),
    decreases bs,
{
    if let RBs::Cons { label: l0, rest, .. } = bs {
        if l0 != label {
            lemma_no_sent_from_branch(*rest, s, label);
        }
    }
}

// ------------------------------------------------- paths and depths --

/// The first path (first branches) meets a node pending for `u`.
pub open spec fn first_hits(x: R, u: int) -> bool
    decreases x,
{
    match x {
        R::End => false,
        R::Var { .. } => false,
        R::Rec { body } => first_hits(*body, u),
        R::Comm { from, to, bs } => from == u || to == u || first_hits_bs(*bs, u),
        R::Sent { to, cont, .. } => to == u || first_hits(*cont, u),
    }
}

pub open spec fn first_hits_bs(bs: RBs, u: int) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => false,
        RBs::Cons { g, .. } => first_hits(g, u),
    }
}

/// The depth of the first pending node of `u` along the first path.
pub open spec fn fdepth(x: R, u: int) -> nat
    decreases x,
{
    match x {
        R::End => 0,
        R::Var { .. } => 0,
        R::Rec { body } => 1 + fdepth(*body, u),
        R::Comm { from, to, bs } => if from == u || to == u { 0 } else { 1 + fdepth_bs(*bs, u) },
        R::Sent { to, cont, .. } => if to == u { 0 } else { 1 + fdepth(*cont, u) },
    }
}

pub open spec fn fdepth_bs(bs: RBs, u: int) -> nat
    decreases bs,
{
    match bs {
        RBs::Nil => 0,
        RBs::Cons { g, .. } => fdepth(g, u),
    }
}

/// The greatest depth of a first pending node of `u` over every path.
pub open spec fn sdepth(x: R, u: int) -> nat
    decreases x,
{
    match x {
        R::End => 0,
        R::Var { .. } => 0,
        R::Rec { body } => 1 + sdepth(*body, u),
        R::Comm { from, to, bs } => if from == u || to == u { 0 } else { 1 + sdepth_bs(*bs, u) },
        R::Sent { to, cont, .. } => if to == u { 0 } else { 1 + sdepth(*cont, u) },
    }
}

pub open spec fn sdepth_bs(bs: RBs, u: int) -> nat
    decreases bs,
{
    match bs {
        RBs::Nil => 0,
        RBs::Cons { g, rest, .. } => if sdepth(g, u) >= sdepth_bs(*rest, u) { sdepth(g, u) } else { sdepth_bs(*rest, u) },
    }
}

proof fn lemma_sdepth_branch(bs: RBs, u: int, label: int)
    requires rbs_lookup(bs, label) is Some,
    ensures sdepth(rbs_lookup(bs, label).unwrap(), u) <= sdepth_bs(bs, u),
    decreases bs,
{
    if let RBs::Cons { label: l0, rest, .. } = bs {
        if l0 != label {
            lemma_sdepth_branch(*rest, u, label);
        }
    }
}

pub proof fn lemma_reaches_first(x: R, u: int)
    requires reaches(x, u),
    ensures first_hits(x, u),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_reaches_first(*body, u),
        R::Comm { from, to, bs } => {
            if from != u && to != u {
                lemma_reaches_first_bs(*bs, u);
            }
        }
        R::Sent { to, cont, .. } => {
            if to != u {
                lemma_reaches_first(*cont, u);
            }
        }
        _ => {}
    }
}

proof fn lemma_reaches_first_bs(bs: RBs, u: int)
    requires reaches_bs(bs, u),
    ensures first_hits_bs(bs, u),
    decreases bs,
{
    if let RBs::Cons { g, .. } = bs {
        lemma_reaches_first(g, u);
    }
}

pub proof fn lemma_reaches_branch(bs: RBs, u: int, label: int)
    requires reaches_bs(bs, u), rbs_lookup(bs, label) is Some,
    ensures reaches(rbs_lookup(bs, label).unwrap(), u),
    decreases bs,
{
    if let RBs::Cons { label: l0, rest, .. } = bs {
        if l0 != label {
            assert(rbs_lookup(*rest, label) is Some);
            assert(!(*rest is Nil));
            lemma_reaches_branch(*rest, u, label);
        }
    }
}

/// Substitution below the pending nodes changes neither the reaching nor
/// the depths.
proof fn lemma_reaches_subst(x: R, u: int, k: nat, by: R)
    requires reaches(x, u),
    ensures reaches(rsubst(x, k, by), u), sdepth(rsubst(x, k, by), u) == sdepth(x, u), fdepth(rsubst(x, k, by), u) == fdepth(x, u),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_reaches_subst(*body, u, k + 1, by),
        R::Comm { from, to, bs } => {
            if from != u && to != u {
                lemma_reaches_subst_bs(*bs, u, k, by);
            }
        }
        R::Sent { to, cont, .. } => {
            if to != u {
                lemma_reaches_subst(*cont, u, k, by);
            }
        }
        _ => {}
    }
}

proof fn lemma_reaches_subst_bs(bs: RBs, u: int, k: nat, by: R)
    requires reaches_bs(bs, u),
    ensures reaches_bs(rsubst_bs(bs, k, by), u), sdepth_bs(rsubst_bs(bs, k, by), u) == sdepth_bs(bs, u), fdepth_bs(rsubst_bs(bs, k, by), u) == fdepth_bs(bs, u),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_reaches_subst(g, u, k, by);
        if !(*rest is Nil) {
            lemma_reaches_subst_bs(*rest, u, k, by);
        }
    }
}

/// Head normalization keeps the reaching and does not deepen the nodes.
pub proof fn lemma_rhead_reaches(x: R, n: int, u: int)
    requires rwf(x, n), reaches(x, u),
    ensures reaches(rhead(x), u), sdepth(rhead(x), u) <= sdepth(x, u), fdepth(rhead(x), u) <= fdepth(x, u),
    decreases rlead_recs(x),
{
    if x is Rec {
        lemma_runfold_wf(x, n);
        lemma_reaches_subst(*x->Rec_body, u, 0, x);
        lemma_rhead_reaches(runfold(x), n, u);
        assert(rhead_n(x, rlead_recs(x)) == rhead_n(runfold(x), rlead_recs(runfold(x))));
    }
}

// --------------------------------------------------------- the steps --

/// `x2` is `x` after role `s` sent `l` to `t`: at every first pending node
/// of `s` along every path of the head normal form, the communication
/// becomes a message in transit with the chosen branch.
pub open spec fn rstep_send(x: R, x2: R, s: int, t: int, l: int) -> bool
    decreases x2,
{
    match rhead(x) {
        R::Comm { from, to, bs } => {
            if from == s {
                &&& to == t
                &&& rbs_lookup(*bs, l) is Some
                &&& x2 == R::Sent { from: s, to: t, label: l, cont: Box::new(rbs_lookup(*bs, l).unwrap()) }
            } else if to == s {
                false
            } else {
                &&& x2 is Comm
                &&& x2->Comm_from == from
                &&& x2->Comm_to == to
                &&& rstep_send_bs(*bs, *x2->Comm_bs, s, t, l)
            }
        }
        R::Sent { from, to, label, cont } => {
            &&& to != s
            &&& x2 is Sent
            &&& x2->Sent_from == from
            &&& x2->Sent_to == to
            &&& x2->Sent_label == label
            &&& rstep_send(*cont, *x2->Sent_cont, s, t, l)
        }
        _ => false,
    }
}

pub open spec fn rstep_send_bs(bs: RBs, bs2: RBs, s: int, t: int, l: int) -> bool
    decreases bs2,
{
    match bs2 {
        RBs::Nil => bs is Nil,
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            &&& bs is Cons
            &&& bs->Cons_label == label2
            &&& rstep_send(bs->Cons_g, g2, s, t, l)
            &&& rstep_send_bs(*bs->Cons_rest, *rest2, s, t, l)
        }
    }
}

/// `x2` is `x` after role `q` received `l` from `p`: at every first pending
/// node of `q` along every path of the head normal form, the message in
/// transit is consumed.
pub open spec fn rstep_recv(x: R, x2: R, q: int, p: int, l: int) -> bool
    decreases x2,
{
    match rhead(x) {
        R::Sent { from, to, label, cont } => {
            if to == q {
                from == p && label == l && x2 == *cont
            } else {
                &&& x2 is Sent
                &&& x2->Sent_from == from
                &&& x2->Sent_to == to
                &&& x2->Sent_label == label
                &&& rstep_recv(*cont, *x2->Sent_cont, q, p, l)
            }
        }
        R::Comm { from, to, bs } => {
            &&& from != q
            &&& to != q
            &&& x2 is Comm
            &&& x2->Comm_from == from
            &&& x2->Comm_to == to
            &&& rstep_recv_bs(*bs, *x2->Comm_bs, q, p, l)
        }
        _ => false,
    }
}

pub open spec fn rstep_recv_bs(bs: RBs, bs2: RBs, q: int, p: int, l: int) -> bool
    decreases bs2,
{
    match bs2 {
        RBs::Nil => bs is Nil,
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            &&& bs is Cons
            &&& bs->Cons_label == label2
            &&& rstep_recv(bs->Cons_g, g2, q, p, l)
            &&& rstep_recv_bs(*bs->Cons_rest, *rest2, q, p, l)
        }
    }
}

/// The stepped branch list has the labels of the original, each branch
/// stepped.
proof fn lemma_send_bs_labels(bs: RBs, bs2: RBs, s: int, t: int, l: int)
    requires rstep_send_bs(bs, bs2, s, t, l),
    ensures
        rbs_len(bs2) == rbs_len(bs),
        rbs_distinct(bs2) == rbs_distinct(bs),
        forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some <==> rbs_lookup(bs, label) is Some,
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rstep_send(rbs_lookup(bs, label).unwrap(), rbs_lookup(bs2, label).unwrap(), s, t, l),
    decreases bs2,
{
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            lemma_send_bs_labels(*bs->Cons_rest, *rest2, s, t, l);
            assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some <==> rbs_lookup(bs, label) is Some by {
                if label != label2 {
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                    assert(rbs_lookup(bs, label) == rbs_lookup(*bs->Cons_rest, label));
                }
            }
            assert forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some implies rstep_send(rbs_lookup(bs, label).unwrap(), rbs_lookup(bs2, label).unwrap(), s, t, l) by {
                if label != label2 {
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                    assert(rbs_lookup(bs, label) == rbs_lookup(*bs->Cons_rest, label));
                }
            }
        }
    }
}

proof fn lemma_recv_bs_labels(bs: RBs, bs2: RBs, q: int, p: int, l: int)
    requires rstep_recv_bs(bs, bs2, q, p, l),
    ensures
        rbs_len(bs2) == rbs_len(bs),
        rbs_distinct(bs2) == rbs_distinct(bs),
        forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some <==> rbs_lookup(bs, label) is Some,
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rstep_recv(rbs_lookup(bs, label).unwrap(), rbs_lookup(bs2, label).unwrap(), q, p, l),
    decreases bs2,
{
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            lemma_recv_bs_labels(*bs->Cons_rest, *rest2, q, p, l);
            assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some <==> rbs_lookup(bs, label) is Some by {
                if label != label2 {
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                    assert(rbs_lookup(bs, label) == rbs_lookup(*bs->Cons_rest, label));
                }
            }
            assert forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some implies rstep_recv(rbs_lookup(bs, label).unwrap(), rbs_lookup(bs2, label).unwrap(), q, p, l) by {
                if label != label2 {
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                    assert(rbs_lookup(bs, label) == rbs_lookup(*bs->Cons_rest, label));
                }
            }
        }
    }
}

// --------------------------------------------- the effects of a step --

/// A step keeps well-formedness and yields a head-normal type.
proof fn lemma_send_wf(x: R, x2: R, s: int, t: int, l: int, n: int)
    requires rwf(x, n), rstep_send(x, x2, s, t, l),
    ensures rwf(x2, n), !(x2 is Rec), !(x2 is Var), rhead(x2) == x2,
    decreases x2,
{
    lemma_rhead_wf(x, n);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            lemma_rcomm_branches_wf(h, n);
            if from == s {
                lemma_rbranch_wf(*bs, n, l);
                assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
            } else {
                let bs2 = *x2->Comm_bs;
                lemma_send_wf_bs(*bs, bs2, s, t, l, n);
                lemma_send_bs_labels(*bs, bs2, s, t, l);
                assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
            }
        }
        R::Sent { cont, .. } => {
            lemma_rsent_cont_wf(h, n);
            lemma_send_wf(*cont, *x2->Sent_cont, s, t, l, n);
            assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
        }
        _ => {}
    }
}

proof fn lemma_send_wf_bs(bs: RBs, bs2: RBs, s: int, t: int, l: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rstep_send_bs(bs, bs2, s, t, l),
    ensures rwf_bs(bs2, n, Seq::<bool>::empty()),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_send_wf(bs->Cons_g, g2, s, t, l, n);
        lemma_send_wf_bs(*bs->Cons_rest, *rest2, s, t, l, n);
    }
}

proof fn lemma_recv_wf(x: R, x2: R, q: int, p: int, l: int, n: int)
    requires rwf(x, n), rstep_recv(x, x2, q, p, l),
    ensures rwf(x2, n), (rhead(x) is Sent && rhead(x)->Sent_to == q) || (!(x2 is Rec) && !(x2 is Var) && rhead(x2) == x2),
    decreases x2,
{
    lemma_rhead_wf(x, n);
    let h = rhead(x);
    match h {
        R::Comm { bs, .. } => {
            lemma_rcomm_branches_wf(h, n);
            let bs2 = *x2->Comm_bs;
            lemma_recv_wf_bs(*bs, bs2, q, p, l, n);
            lemma_recv_bs_labels(*bs, bs2, q, p, l);
            assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
        }
        R::Sent { to, cont, .. } => {
            lemma_rsent_cont_wf(h, n);
            if to != q {
                lemma_recv_wf(*cont, *x2->Sent_cont, q, p, l, n);
                assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
            }
        }
        _ => {}
    }
}

proof fn lemma_recv_wf_bs(bs: RBs, bs2: RBs, q: int, p: int, l: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rstep_recv_bs(bs, bs2, q, p, l),
    ensures rwf_bs(bs2, n, Seq::<bool>::empty()),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_recv_wf(bs->Cons_g, g2, q, p, l, n);
        lemma_recv_wf_bs(*bs->Cons_rest, *rest2, q, p, l, n);
    }
}

/// The shape of a stepped branch list follows the original's.
proof fn lemma_send_bs_shape(bs: RBs, bs2: RBs, s: int, t: int, l: int)
    requires rstep_send_bs(bs, bs2, s, t, l),
    ensures
        bs is Cons <==> bs2 is Cons,
        bs is Cons ==> bs2->Cons_label == bs->Cons_label && rstep_send(bs->Cons_g, bs2->Cons_g, s, t, l) && rstep_send_bs(*bs->Cons_rest, *bs2->Cons_rest, s, t, l),
{
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { .. } => {}
    }
}

proof fn lemma_recv_bs_shape(bs: RBs, bs2: RBs, q: int, p: int, l: int)
    requires rstep_recv_bs(bs, bs2, q, p, l),
    ensures
        bs is Cons <==> bs2 is Cons,
        bs is Cons ==> bs2->Cons_label == bs->Cons_label && rstep_recv(bs->Cons_g, bs2->Cons_g, q, p, l) && rstep_recv_bs(*bs->Cons_rest, *bs2->Cons_rest, q, p, l),
{
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { .. } => {}
    }
}

/// A send keeps the structure, appends the label to its channel's queue,
/// leaves every other queue, and creates messages from the sender alone.
proof fn lemma_send_good(x: R, x2: R, s: int, t: int, l: int, n: int)
    requires rwf(x, n), rgood(x), rstep_send(x, x2, s, t, l),
    ensures
        rgood(x2),
        rqueue(x2, s, t) == rqueue(x, s, t).push(l),
        forall|p: int, q: int| !(p == s && q == t) ==> #[trigger] rqueue(x2, p, q) == rqueue(x, p, q),
        forall|y: int| y != s && #[trigger] no_sent_from(x, y) ==> no_sent_from(x2, y),
    decreases x2,
{
    lemma_rhead_wf(x, n);
    lemma_rhead_good(x, n);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            if from == s {
                let b = rbs_lookup(*bs, l).unwrap();
                lemma_rgood_branch(*bs, l);
                lemma_no_sent_from_branch(*bs, s, l);
                lemma_no_sent_from_queue(b, s);
                lemma_no_sent_from_queue_bs(*bs, s);
                assert(rqueue(h, s, t) == rqueue_bs(*bs, s, t));
                assert(rqueue(x2, s, t) == seq![l].add(rqueue(b, s, t)));
                assert(rqueue(x2, s, t) =~= rqueue(x, s, t).push(l));
                assert(rgood(h));
                assert(queues_agree_bs(*bs));
                assert forall|p: int, q: int| !(p == s && q == t) implies #[trigger] rqueue(x2, p, q) == rqueue(x, p, q) by {
                    assert(rqueue(x2, p, q) == Seq::<int>::empty().add(rqueue(b, p, q)));
                    assert(Seq::<int>::empty().add(rqueue(b, p, q)) =~= rqueue(b, p, q));
                    assert(rqueue(b, p, q) == rqueue_bs(*bs, p, q));
                    assert(rqueue(x, p, q) == rqueue(h, p, q));
                    assert(rqueue(h, p, q) == rqueue_bs(*bs, p, q));
                }
                assert forall|y: int| y != s && #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                    lemma_no_sent_from_rhead(x, y, n);
                    lemma_no_sent_from_branch(*bs, y, l);
                }
            } else {
                let bs2 = *x2->Comm_bs;
                lemma_send_bs_labels(*bs, bs2, s, t, l);
                lemma_rcomm_branches_wf(h, n);
                lemma_send_good_bs(*bs, bs2, s, t, l, n);
                // The first branch's queues: the stepped first branch.
                let first = rbs_first(*bs);
                assert(*bs is Cons);
                lemma_send_bs_shape(*bs, bs2, s, t, l);
                assert(bs2 is Cons);
                let label1 = bs->Cons_label;
                assert(bs2->Cons_label == label1);
                assert(rbs_lookup(*bs, label1) == Some(first));
                assert(rbs_lookup(bs2, label1) == Some(rbs_first(bs2)));
                assert(rstep_send(first, rbs_first(bs2), s, t, l));
                assert(x2 == R::Comm { from, to, bs: Box::new(bs2) });
                assert(rqueue(x2, s, t) == rqueue_bs(bs2, s, t));
                assert(rqueue_bs(bs2, s, t) == rqueue(rbs_first(bs2), s, t));
                assert(rqueue(h, s, t) == rqueue(first, s, t));
                assert(rqueue(x, s, t) == rqueue(h, s, t));
                assert(rgood(h));
                assert(queues_agree_bs(*bs));
                assert forall|p: int, q: int| !(p == s && q == t) implies #[trigger] rqueue(x2, p, q) == rqueue(x, p, q) by {
                    assert(rqueue(x2, p, q) == rqueue_bs(bs2, p, q));
                    assert(rqueue_bs(bs2, p, q) == rqueue(rbs_first(bs2), p, q));
                    assert(rqueue(h, p, q) == rqueue(first, p, q));
                    assert(rqueue(x, p, q) == rqueue(h, p, q));
                }
                // Agreement of the stepped branches.
                assert forall|label: int, p: int, q: int| rbs_lookup(bs2, label) is Some implies #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), p, q) == rqueue_bs(bs2, p, q) by {
                    let b = rbs_lookup(*bs, label).unwrap();
                    let b2 = rbs_lookup(bs2, label).unwrap();
                    assert(rqueue(b, p, q) == rqueue_bs(*bs, p, q));
                    assert(rqueue(first, p, q) == rqueue_bs(*bs, p, q));
                    assert(rqueue_bs(bs2, p, q) == rqueue(rbs_first(bs2), p, q));
                }
                assert forall|y: int| y != s && #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                    lemma_no_sent_from_rhead(x, y, n);
                    assert(no_sent_from(h, y));
                    assert(no_sent_from_bs(*bs, y));
                    assert(no_sent_from_bs(bs2, y));
                }
                assert(no_sent_from_bs(*bs, from));
                assert(no_sent_from_bs(bs2, from));
            }
        }
        R::Sent { from, to, label, cont } => {
            let cont2 = *x2->Sent_cont;
            lemma_rsent_cont_wf(h, n);
            lemma_send_good(*cont, cont2, s, t, l, n);
            let pre = if from == s && to == t { seq![label] } else { Seq::<int>::empty() };
            assert(rqueue(x2, s, t) == pre.add(rqueue(cont2, s, t)));
            assert(rqueue(h, s, t) == pre.add(rqueue(*cont, s, t)));
            assert(rqueue(x, s, t) == rqueue(h, s, t));
            assert(rqueue(x2, s, t) =~= rqueue(x, s, t).push(l));
            assert forall|p: int, q: int| !(p == s && q == t) implies #[trigger] rqueue(x2, p, q) == rqueue(x, p, q) by {
                let pre2 = if from == p && to == q { seq![label] } else { Seq::<int>::empty() };
                assert(rqueue(x2, p, q) == pre2.add(rqueue(cont2, p, q)));
                assert(rqueue(h, p, q) == pre2.add(rqueue(*cont, p, q)));
                assert(rqueue(x, p, q) == rqueue(h, p, q));
            }
            assert forall|y: int| y != s && #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                lemma_no_sent_from_rhead(x, y, n);
                assert(no_sent_from(h, y));
                assert(no_sent_from(*cont, y));
                assert(no_sent_from(cont2, y));
                assert(x2 == R::Sent { from, to, label, cont: Box::new(cont2) });
            }
        }
        _ => {}
    }
}

/// Branchwise: the stepped branches are good, with the sender's queue
/// extended and the others kept, and no new message from anyone else.
proof fn lemma_send_good_bs(bs: RBs, bs2: RBs, s: int, t: int, l: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rgood_bs(bs), rstep_send_bs(bs, bs2, s, t, l),
    ensures
        rgood_bs(bs2),
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rqueue(rbs_lookup(bs2, label).unwrap(), s, t) == rqueue(rbs_lookup(bs, label).unwrap(), s, t).push(l),
        forall|label: int, p: int, q: int| rbs_lookup(bs, label) is Some && !(p == s && q == t) ==> #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), p, q) == rqueue(rbs_lookup(bs, label).unwrap(), p, q),
        forall|y: int| y != s && #[trigger] no_sent_from_bs(bs, y) ==> no_sent_from_bs(bs2, y),
    decreases bs2,
{
    lemma_send_bs_labels(bs, bs2, s, t, l);
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            let g = bs->Cons_g;
            let rest = *bs->Cons_rest;
            lemma_send_good(g, g2, s, t, l, n);
            lemma_send_good_bs(rest, *rest2, s, t, l, n);
            assert forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some implies rqueue(rbs_lookup(bs2, label).unwrap(), s, t) == rqueue(rbs_lookup(bs, label).unwrap(), s, t).push(l) by {
                if label != label2 {
                    assert(rbs_lookup(bs, label) == rbs_lookup(rest, label));
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                }
            }
            assert forall|label: int, p: int, q: int| rbs_lookup(bs, label) is Some && !(p == s && q == t) implies #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), p, q) == rqueue(rbs_lookup(bs, label).unwrap(), p, q) by {
                if label != label2 {
                    assert(rbs_lookup(bs, label) == rbs_lookup(rest, label));
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                }
            }
            assert forall|y: int| y != s && #[trigger] no_sent_from_bs(bs, y) implies no_sent_from_bs(bs2, y) by {
                assert(no_sent_from(g, y));
                assert(no_sent_from_bs(rest, y));
            }
        }
    }
}

/// Head normalization keeps the absence of messages from a role.
proof fn lemma_no_sent_from_rhead(x: R, y: int, n: int)
    requires rwf(x, n), no_sent_from(x, y),
    ensures no_sent_from(rhead(x), y),
    decreases rlead_recs(x),
{
    if x is Rec {
        lemma_runfold_wf(x, n);
        lemma_no_sent_from_subst(*x->Rec_body, y, 0, x);
        lemma_no_sent_from_rhead(runfold(x), y, n);
        assert(rhead_n(x, rlead_recs(x)) == rhead_n(runfold(x), rlead_recs(runfold(x))));
    }
}

proof fn lemma_no_sent_from_subst(x: R, y: int, k: nat, by: R)
    requires no_sent_from(x, y), no_sent_from(by, y),
    ensures no_sent_from(rsubst(x, k, by), y),
    decreases x,
{
    match x {
        R::Rec { body } => lemma_no_sent_from_subst(*body, y, k + 1, by),
        R::Comm { bs, .. } => lemma_no_sent_from_subst_bs(*bs, y, k, by),
        R::Sent { cont, .. } => lemma_no_sent_from_subst(*cont, y, k, by),
        _ => {}
    }
}

proof fn lemma_no_sent_from_subst_bs(bs: RBs, y: int, k: nat, by: R)
    requires no_sent_from_bs(bs, y), no_sent_from(by, y),
    ensures no_sent_from_bs(rsubst_bs(bs, k, by), y),
    decreases bs,
{
    if let RBs::Cons { g, rest, .. } = bs {
        lemma_no_sent_from_subst(g, y, k, by);
        lemma_no_sent_from_subst_bs(*rest, y, k, by);
    }
}

/// A receive keeps the structure, drops the oldest label of its channel's
/// queue, leaves every other queue, and creates no message.
proof fn lemma_recv_good(x: R, x2: R, q: int, p: int, l: int, n: int)
    requires rwf(x, n), rgood(x), rstep_recv(x, x2, q, p, l),
    ensures
        rgood(x2),
        rqueue(x, p, q).len() > 0 && rqueue(x, p, q)[0] == l,
        rqueue(x2, p, q) == rqueue(x, p, q).drop_first(),
        forall|a: int, b: int| !(a == p && b == q) ==> #[trigger] rqueue(x2, a, b) == rqueue(x, a, b),
        forall|y: int| #[trigger] no_sent_from(x, y) ==> no_sent_from(x2, y),
    decreases x2,
{
    lemma_rhead_wf(x, n);
    lemma_rhead_good(x, n);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            let bs2 = *x2->Comm_bs;
            lemma_recv_bs_labels(*bs, bs2, q, p, l);
            lemma_rcomm_branches_wf(h, n);
            lemma_recv_good_bs(*bs, bs2, q, p, l, n);
            let first = rbs_first(*bs);
            assert(*bs is Cons);
            lemma_recv_bs_shape(*bs, bs2, q, p, l);
            assert(bs2 is Cons);
            let label1 = bs->Cons_label;
            assert(bs2->Cons_label == label1);
            assert(rbs_lookup(*bs, label1) == Some(first));
            assert(rbs_lookup(bs2, label1) == Some(rbs_first(bs2)));
            assert(rqueue(h, p, q) == rqueue(first, p, q));
            assert(rqueue(x, p, q) == rqueue(h, p, q));
            assert(x2 == R::Comm { from, to, bs: Box::new(bs2) });
            assert(rqueue(x2, p, q) == rqueue_bs(bs2, p, q));
            assert(rqueue_bs(bs2, p, q) == rqueue(rbs_first(bs2), p, q));
            assert(rgood(h));
            assert(queues_agree_bs(*bs));
            assert forall|a: int, b: int| !(a == p && b == q) implies #[trigger] rqueue(x2, a, b) == rqueue(x, a, b) by {
                assert(rqueue(x2, a, b) == rqueue_bs(bs2, a, b));
                assert(rqueue_bs(bs2, a, b) == rqueue(rbs_first(bs2), a, b));
                assert(rqueue(h, a, b) == rqueue(first, a, b));
                assert(rqueue(x, a, b) == rqueue(h, a, b));
            }
            assert forall|label: int, a: int, b: int| rbs_lookup(bs2, label) is Some implies #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), a, b) == rqueue_bs(bs2, a, b) by {
                let br = rbs_lookup(*bs, label).unwrap();
                assert(rqueue(br, a, b) == rqueue_bs(*bs, a, b));
                assert(rqueue(first, a, b) == rqueue_bs(*bs, a, b));
                assert(rqueue_bs(bs2, a, b) == rqueue(rbs_first(bs2), a, b));
            }
            assert forall|y: int| #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                lemma_no_sent_from_rhead(x, y, n);
                assert(no_sent_from(h, y));
                assert(no_sent_from_bs(*bs, y));
                assert(no_sent_from_bs(bs2, y));
            }
            assert(no_sent_from_bs(*bs, from));
            assert(no_sent_from_bs(bs2, from));
        }
        R::Sent { from, to, label, cont } => {
            lemma_rsent_cont_wf(h, n);
            if to == q {
                assert(rqueue(h, p, q) == seq![l].add(rqueue(*cont, p, q)));
                assert(rqueue(x, p, q) == rqueue(h, p, q));
                assert(rqueue(x2, p, q) =~= rqueue(x, p, q).drop_first());
                assert forall|a: int, b: int| !(a == p && b == q) implies #[trigger] rqueue(x2, a, b) == rqueue(x, a, b) by {
                    assert(rqueue(h, a, b) == Seq::<int>::empty().add(rqueue(*cont, a, b)));
                    assert(Seq::<int>::empty().add(rqueue(*cont, a, b)) =~= rqueue(*cont, a, b));
                    assert(rqueue(x, a, b) == rqueue(h, a, b));
                }
                assert forall|y: int| #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                    lemma_no_sent_from_rhead(x, y, n);
                    assert(no_sent_from(h, y));
                    assert(x2 == *cont);
                }
            } else {
                let cont2 = *x2->Sent_cont;
                lemma_recv_good(*cont, cont2, q, p, l, n);
                assert(rqueue(h, p, q) == Seq::<int>::empty().add(rqueue(*cont, p, q)));
                assert(Seq::<int>::empty().add(rqueue(*cont, p, q)) =~= rqueue(*cont, p, q));
                assert(rqueue(x2, p, q) == Seq::<int>::empty().add(rqueue(cont2, p, q)));
                assert(Seq::<int>::empty().add(rqueue(cont2, p, q)) =~= rqueue(cont2, p, q));
                assert(rqueue(x, p, q) == rqueue(h, p, q));
                assert forall|a: int, b: int| !(a == p && b == q) implies #[trigger] rqueue(x2, a, b) == rqueue(x, a, b) by {
                    assert(rqueue(x2, a, b) == (if from == a && to == b { seq![label] } else { Seq::<int>::empty() }).add(rqueue(cont2, a, b)));
                    assert(rqueue(h, a, b) == (if from == a && to == b { seq![label] } else { Seq::<int>::empty() }).add(rqueue(*cont, a, b)));
                    assert(rqueue(x, a, b) == rqueue(h, a, b));
                }
                assert forall|y: int| #[trigger] no_sent_from(x, y) implies no_sent_from(x2, y) by {
                    lemma_no_sent_from_rhead(x, y, n);
                    assert(no_sent_from(h, y));
                    assert(no_sent_from(*cont, y));
                    assert(no_sent_from(cont2, y));
                    assert(x2 == R::Sent { from, to, label, cont: Box::new(cont2) });
                }
            }
        }
        _ => {}
    }
}

proof fn lemma_recv_good_bs(bs: RBs, bs2: RBs, q: int, p: int, l: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rgood_bs(bs), rstep_recv_bs(bs, bs2, q, p, l),
    ensures
        rgood_bs(bs2),
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rqueue(rbs_lookup(bs, label).unwrap(), p, q).len() > 0
            && rqueue(rbs_lookup(bs, label).unwrap(), p, q)[0] == l
            && rqueue(rbs_lookup(bs2, label).unwrap(), p, q) == rqueue(rbs_lookup(bs, label).unwrap(), p, q).drop_first(),
        forall|label: int, a: int, b: int| rbs_lookup(bs, label) is Some && !(a == p && b == q) ==> #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), a, b) == rqueue(rbs_lookup(bs, label).unwrap(), a, b),
        forall|y: int| #[trigger] no_sent_from_bs(bs, y) ==> no_sent_from_bs(bs2, y),
    decreases bs2,
{
    lemma_recv_bs_labels(bs, bs2, q, p, l);
    match bs2 {
        RBs::Nil => {}
        RBs::Cons { label: label2, g: g2, rest: rest2 } => {
            let g = bs->Cons_g;
            let rest = *bs->Cons_rest;
            lemma_recv_good(g, g2, q, p, l, n);
            lemma_recv_good_bs(rest, *rest2, q, p, l, n);
            assert forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some implies rqueue(rbs_lookup(bs, label).unwrap(), p, q).len() > 0
                && rqueue(rbs_lookup(bs, label).unwrap(), p, q)[0] == l
                && rqueue(rbs_lookup(bs2, label).unwrap(), p, q) == rqueue(rbs_lookup(bs, label).unwrap(), p, q).drop_first() by {
                if label != label2 {
                    assert(rbs_lookup(bs, label) == rbs_lookup(rest, label));
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                }
            }
            assert forall|label: int, a: int, b: int| rbs_lookup(bs, label) is Some && !(a == p && b == q) implies #[trigger] rqueue(rbs_lookup(bs2, label).unwrap(), a, b) == rqueue(rbs_lookup(bs, label).unwrap(), a, b) by {
                if label != label2 {
                    assert(rbs_lookup(bs, label) == rbs_lookup(rest, label));
                    assert(rbs_lookup(bs2, label) == rbs_lookup(*rest2, label));
                }
            }
            assert forall|y: int| #[trigger] no_sent_from_bs(bs, y) implies no_sent_from_bs(bs2, y) by {
                assert(no_sent_from(g, y));
                assert(no_sent_from_bs(rest, y));
            }
        }
    }
}

/// A step by another role keeps the reaching of `u`.
proof fn lemma_send_reaches(x: R, x2: R, s: int, t: int, l: int, u: int, n: int)
    requires rwf(x, n), rstep_send(x, x2, s, t, l), u != s, reaches(x, u),
    ensures reaches(x2, u),
    decreases x2,
{
    lemma_rhead_wf(x, n);
    lemma_rhead_reaches(x, n, u);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            if from == s {
                if u != to {
                    lemma_reaches_branch(*bs, u, l);
                }
            } else if from != u && to != u {
                lemma_rcomm_branches_wf(h, n);
                lemma_send_reaches_bs(*bs, *x2->Comm_bs, s, t, l, u, n);
            }
        }
        R::Sent { to, cont, .. } => {
            if to != u {
                lemma_rsent_cont_wf(h, n);
                lemma_send_reaches(*cont, *x2->Sent_cont, s, t, l, u, n);
            }
        }
        _ => {}
    }
}

proof fn lemma_send_reaches_bs(bs: RBs, bs2: RBs, s: int, t: int, l: int, u: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rstep_send_bs(bs, bs2, s, t, l), u != s, reaches_bs(bs, u),
    ensures reaches_bs(bs2, u),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_send_reaches(bs->Cons_g, g2, s, t, l, u, n);
        if !(*rest2 is Nil) {
            lemma_send_bs_shape(*bs->Cons_rest, *rest2, s, t, l);
            assert(*bs->Cons_rest is Cons);
            assert(reaches_bs(*bs->Cons_rest, u));
            lemma_send_reaches_bs(*bs->Cons_rest, *rest2, s, t, l, u, n);
        }
    }
}

proof fn lemma_recv_reaches(x: R, x2: R, q: int, p: int, l: int, u: int, n: int)
    requires rwf(x, n), rstep_recv(x, x2, q, p, l), u != q, reaches(x, u),
    ensures reaches(x2, u),
    decreases x2,
{
    lemma_rhead_wf(x, n);
    lemma_rhead_reaches(x, n, u);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            if from != u && to != u {
                lemma_rcomm_branches_wf(h, n);
                lemma_recv_reaches_bs(*bs, *x2->Comm_bs, q, p, l, u, n);
            }
        }
        R::Sent { to, cont, .. } => {
            if to != q && to != u {
                lemma_rsent_cont_wf(h, n);
                lemma_recv_reaches(*cont, *x2->Sent_cont, q, p, l, u, n);
            }
        }
        _ => {}
    }
}

proof fn lemma_recv_reaches_bs(bs: RBs, bs2: RBs, q: int, p: int, l: int, u: int, n: int)
    requires rwf_bs(bs, n, Seq::<bool>::empty()), rstep_recv_bs(bs, bs2, q, p, l), u != q, reaches_bs(bs, u),
    ensures reaches_bs(bs2, u),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_recv_reaches(bs->Cons_g, g2, q, p, l, u, n);
        if !(*rest2 is Nil) {
            lemma_recv_bs_shape(*bs->Cons_rest, *rest2, q, p, l);
            assert(*bs->Cons_rest is Cons);
            assert(reaches_bs(*bs->Cons_rest, u));
            lemma_recv_reaches_bs(*bs->Cons_rest, *rest2, q, p, l, u, n);
        }
    }
}

// ---------------------------------------------- the existence of a step --

/// A node pending for `u` on every path, through distinct branches.
proof fn lemma_reaches_bs_from_lookups(bs: RBs, u: int)
    requires bs is Cons, rbs_distinct(bs), forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> reaches(rbs_lookup(bs, label).unwrap(), u),
    ensures reaches_bs(bs, u),
    decreases bs,
{
    if let RBs::Cons { label, g, rest } = bs {
        assert(rbs_lookup(bs, label) == Some(g));
        if !(*rest is Nil) {
            assert forall|lab: int| (#[trigger] rbs_lookup(*rest, lab)) is Some implies reaches(rbs_lookup(*rest, lab).unwrap(), u) by {
                assert(lab != label);
                assert(rbs_lookup(bs, lab) == rbs_lookup(*rest, lab));
            }
            lemma_reaches_bs_from_lookups(*rest, u);
        }
    }
}

/// A role whose head is a send of a label of its list can send it: the
/// runtime type has the step, in every branch of every pending choice.
proof fn lemma_send_exists(rel: ISet<(L, R)>, m: L, x: R, s: int, t: int, l: int, n: int)
    requires
        consistent(rel, s, n), rel.contains((m, x)),
        mhead(m) is Send, mhead(m)->Send_to == t, ls_lookup(*mhead(m)->Send_ls, l) is Some,
    ensures exists|x2: R| rstep_send(x, x2, s, t, l),
    decreases sdepth(x, s) + 1, 0nat,
{
    assert(node_ok(rel, m, x, s, n));
    let hm = mhead(m);
    lemma_rhead_wf(x, n);
    lemma_rhead_reaches(x, n, s);
    let h = rhead(x);
    match h {
        R::Comm { from, to, bs } => {
            if from == s {
                let x2 = R::Sent { from: s, to: t, label: l, cont: Box::new(rbs_lookup(*bs, l).unwrap()) };
                assert(rstep_send(x, x2, s, t, l));
            } else {
                assert(to != s);
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel.contains((m, rbs_lookup(*bs, label).unwrap())) && sdepth(rbs_lookup(*bs, label).unwrap(), s) < sdepth(x, s) by {
                    lemma_sdepth_branch(*bs, s, label);
                }
                lemma_send_exists_bs(rel, m, *bs, s, t, l, n, sdepth(x, s));
                let bs2 = choose|bs2: RBs| rstep_send_bs(*bs, bs2, s, t, l);
                let x2 = R::Comm { from, to, bs: Box::new(bs2) };
                assert(rstep_send(x, x2, s, t, l));
            }
        }
        R::Sent { from, to, label, cont } => {
            assert(to != s);
            assert(rel.contains((m, *cont)));
            lemma_send_exists(rel, m, *cont, s, t, l, n);
            let cont2 = choose|cont2: R| rstep_send(*cont, cont2, s, t, l);
            let x2 = R::Sent { from, to, label, cont: Box::new(cont2) };
            assert(rstep_send(x, x2, s, t, l));
        }
        _ => {}
    }
}

proof fn lemma_send_exists_bs(rel: ISet<(L, R)>, m: L, bs: RBs, s: int, t: int, l: int, n: int, d: nat)
    requires
        consistent(rel, s, n), rbs_distinct(bs),
        mhead(m) is Send, mhead(m)->Send_to == t, ls_lookup(*mhead(m)->Send_ls, l) is Some,
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rel.contains((m, rbs_lookup(bs, label).unwrap())) && sdepth(rbs_lookup(bs, label).unwrap(), s) < d,
    ensures exists|bs2: RBs| rstep_send_bs(bs, bs2, s, t, l),
    decreases d, rbs_len(bs),
{
    match bs {
        RBs::Nil => {
            assert(rstep_send_bs(bs, RBs::Nil, s, t, l));
        }
        RBs::Cons { label, g, rest } => {
            assert(rbs_lookup(bs, label) == Some(g));
            assert(rbs_len(bs) >= 1);
            lemma_send_exists(rel, m, g, s, t, l, n);
            let g2 = choose|g2: R| rstep_send(g, g2, s, t, l);
            assert forall|lab: int| (#[trigger] rbs_lookup(*rest, lab)) is Some implies rel.contains((m, rbs_lookup(*rest, lab).unwrap())) && sdepth(rbs_lookup(*rest, lab).unwrap(), s) < d by {
                assert(lab != label);
                assert(rbs_lookup(bs, lab) == rbs_lookup(*rest, lab));
            }
            lemma_send_exists_bs(rel, m, *rest, s, t, l, n, d);
            let rest2 = choose|rest2: RBs| rstep_send_bs(*rest, rest2, s, t, l);
            let bs2 = RBs::Cons { label, g: g2, rest: Box::new(rest2) };
            assert(rstep_send_bs(bs, bs2, s, t, l));
        }
    }
}

/// A role whose head is a receive from `p` with the oldest label of the
/// channel from `p` in its list can receive it.
proof fn lemma_recv_exists(rel: ISet<(L, R)>, m: L, x: R, q: int, p: int, l: int, n: int)
    requires
        consistent(rel, q, n), rel.contains((m, x)), rgood(x),
        mhead(m) is Recv, mhead(m)->Recv_from == p,
        rqueue(x, p, q).len() > 0, rqueue(x, p, q)[0] == l,
    ensures exists|x2: R| rstep_recv(x, x2, q, p, l),
    decreases sdepth(x, q) + 1, 0nat,
{
    assert(node_ok(rel, m, x, q, n));
    let hm = mhead(m);
    lemma_rhead_wf(x, n);
    lemma_rhead_good(x, n);
    lemma_rhead_reaches(x, n, q);
    let h = rhead(x);
    match h {
        R::Sent { from, to, label, cont } => {
            if to == q {
                assert(from == p);
                assert(rqueue(h, p, q) == seq![label].add(rqueue(*cont, p, q)));
                assert(label == l);
                assert(rstep_recv(x, *cont, q, p, l));
            } else {
                assert(rqueue(h, p, q) == Seq::<int>::empty().add(rqueue(*cont, p, q)));
                assert(Seq::<int>::empty().add(rqueue(*cont, p, q)) =~= rqueue(*cont, p, q));
                assert(rel.contains((m, *cont)));
                lemma_recv_exists(rel, m, *cont, q, p, l, n);
                let cont2 = choose|cont2: R| rstep_recv(*cont, cont2, q, p, l);
                let x2 = R::Sent { from, to, label, cont: Box::new(cont2) };
                assert(rstep_recv(x, x2, q, p, l));
            }
        }
        R::Comm { from, to, bs } => {
            if to == q {
                assert(from == p);
                assert(no_sent_from_bs(*bs, p));
                lemma_no_sent_from_queue_bs(*bs, p);
                assert(rqueue(h, p, q) == rqueue_bs(*bs, p, q));
                assert(false);
            } else if from == q {
                assert(false);
            } else {
                assert(queues_agree_bs(*bs));
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel.contains((m, rbs_lookup(*bs, label).unwrap()))
                    && sdepth(rbs_lookup(*bs, label).unwrap(), q) < sdepth(x, q)
                    && rgood(rbs_lookup(*bs, label).unwrap())
                    && rqueue(rbs_lookup(*bs, label).unwrap(), p, q) == rqueue(x, p, q) by {
                    lemma_sdepth_branch(*bs, q, label);
                    lemma_rgood_branch(*bs, label);
                    assert(rqueue(rbs_lookup(*bs, label).unwrap(), p, q) == rqueue_bs(*bs, p, q));
                    assert(rqueue(h, p, q) == rqueue_bs(*bs, p, q));
                }
                lemma_recv_exists_bs(rel, m, *bs, q, p, l, n, sdepth(x, q), rqueue(x, p, q));
                let bs2 = choose|bs2: RBs| rstep_recv_bs(*bs, bs2, q, p, l);
                let x2 = R::Comm { from, to, bs: Box::new(bs2) };
                assert(rstep_recv(x, x2, q, p, l));
            }
        }
        _ => {}
    }
}

proof fn lemma_recv_exists_bs(rel: ISet<(L, R)>, m: L, bs: RBs, q: int, p: int, l: int, n: int, d: nat, w: Seq<int>)
    requires
        consistent(rel, q, n), rbs_distinct(bs),
        mhead(m) is Recv, mhead(m)->Recv_from == p,
        w.len() > 0, w[0] == l,
        forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rel.contains((m, rbs_lookup(bs, label).unwrap()))
            && sdepth(rbs_lookup(bs, label).unwrap(), q) < d && rgood(rbs_lookup(bs, label).unwrap()) && rqueue(rbs_lookup(bs, label).unwrap(), p, q) == w,
    ensures exists|bs2: RBs| rstep_recv_bs(bs, bs2, q, p, l),
    decreases d, rbs_len(bs),
{
    match bs {
        RBs::Nil => {
            assert(rstep_recv_bs(bs, RBs::Nil, q, p, l));
        }
        RBs::Cons { label, g, rest } => {
            assert(rbs_lookup(bs, label) == Some(g));
            assert(rbs_len(bs) >= 1);
            lemma_recv_exists(rel, m, g, q, p, l, n);
            let g2 = choose|g2: R| rstep_recv(g, g2, q, p, l);
            assert forall|lab: int| (#[trigger] rbs_lookup(*rest, lab)) is Some implies rel.contains((m, rbs_lookup(*rest, lab).unwrap()))
                && sdepth(rbs_lookup(*rest, lab).unwrap(), q) < d && rgood(rbs_lookup(*rest, lab).unwrap()) && rqueue(rbs_lookup(*rest, lab).unwrap(), p, q) == w by {
                assert(lab != label);
                assert(rbs_lookup(bs, lab) == rbs_lookup(*rest, lab));
            }
            lemma_recv_exists_bs(rel, m, *rest, q, p, l, n, d, w);
            let rest2 = choose|rest2: RBs| rstep_recv_bs(*rest, rest2, q, p, l);
            let bs2 = RBs::Cons { label, g: g2, rest: Box::new(rest2) };
            assert(rstep_recv_bs(bs, bs2, q, p, l));
        }
    }
}

// --------------------------------------- the relation across a step --

/// The role's local type after the step: the actor continues with the
/// branch of the label, every other role keeps its type.
pub open spec fn after_act(m: L, u: int, actor: int, l: int) -> L {
    if u == actor { ls_lookup(ls_of(mhead(m)), l).unwrap() } else { m }
}

pub open spec fn rel_send_member(rel: ISet<(L, R)>, pr: (L, R), s: int, t: int, l: int, u: int) -> bool {
    ||| rel.contains(pr)
    ||| exists|m: L, x: R| #[trigger] rel.contains((m, x)) && rstep_send(x, pr.1, s, t, l) && pr.0 == after_act(m, u, s, l)
}

/// The relation of role `u` after `s` sent `l` to `t`: every pair, and
/// every stepped pair with the role's type after the step.
pub open spec fn rel_send(rel: ISet<(L, R)>, s: int, t: int, l: int, u: int) -> ISet<(L, R)> {
    ISet::new(|pr: (L, R)| rel_send_member(rel, pr, s, t, l, u))
}

pub open spec fn rel_recv_member(rel: ISet<(L, R)>, pr: (L, R), q: int, p: int, l: int, u: int) -> bool {
    ||| rel.contains(pr)
    ||| exists|m: L, x: R| #[trigger] rel.contains((m, x)) && rstep_recv(x, pr.1, q, p, l) && pr.0 == after_act(m, u, q, l)
}

pub open spec fn rel_recv(rel: ISet<(L, R)>, q: int, p: int, l: int, u: int) -> ISet<(L, R)> {
    ISet::new(|pr: (L, R)| rel_recv_member(rel, pr, q, p, l, u))
}

/// The conditions on a pair only grow with the relation.
proof fn lemma_node_ok_mono(rel: ISet<(L, R)>, rel2: ISet<(L, R)>, m: L, x: R, u: int, n: int)
    requires node_ok(rel, m, x, u, n), forall|pr: (L, R)| #[trigger] rel.contains(pr) ==> rel2.contains(pr),
    ensures node_ok(rel2, m, x, u, n),
{
    let h = rhead(x);
    let hm = mhead(m);
    match h {
        R::Comm { from, to, bs } => {
            if u == from {
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel2.contains((ls_lookup(*hm->Send_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap())) by {
                    assert(rel.contains((ls_lookup(*hm->Send_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap())));
                }
            } else if u == to {
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel2.contains((ls_lookup(*hm->Recv_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap())) by {
                    assert(rel.contains((ls_lookup(*hm->Recv_ls, label).unwrap(), rbs_lookup(*bs, label).unwrap())));
                }
            } else {
                assert forall|label: int| (#[trigger] rbs_lookup(*bs, label)) is Some implies rel2.contains((m, rbs_lookup(*bs, label).unwrap())) by {
                    assert(rel.contains((m, rbs_lookup(*bs, label).unwrap())));
                }
            }
        }
        R::Sent { to, cont, label, .. } => {
            if u == to {
                assert(rel.contains((ls_lookup(*hm->Recv_ls, label).unwrap(), *cont)));
            } else {
                assert(rel.contains((m, *cont)));
            }
        }
        _ => {}
    }
}

/// Every branch is paired with `m` in the relation.
pub open spec fn all_in(rel: ISet<(L, R)>, m: L, bs: RBs) -> bool
    decreases bs,
{
    match bs {
        RBs::Nil => true,
        RBs::Cons { g, rest, .. } => rel.contains((m, g)) && all_in(rel, m, *rest),
    }
}

proof fn lemma_all_in_from_lookups(rel: ISet<(L, R)>, m: L, bs: RBs)
    requires rbs_distinct(bs), forall|label: int| (#[trigger] rbs_lookup(bs, label)) is Some ==> rel.contains((m, rbs_lookup(bs, label).unwrap())),
    ensures all_in(rel, m, bs),
    decreases bs,
{
    if let RBs::Cons { label, g, rest } = bs {
        assert(rbs_lookup(bs, label) == Some(g));
        assert forall|lab: int| (#[trigger] rbs_lookup(*rest, lab)) is Some implies rel.contains((m, rbs_lookup(*rest, lab).unwrap())) by {
            assert(lab != label);
            assert(rbs_lookup(bs, lab) == rbs_lookup(*rest, lab));
        }
        lemma_all_in_from_lookups(rel, m, *rest);
    }
}

/// The actor's reaching after a send: its continuation, if it acts, is
/// reached on every path of the stepped type.
proof fn lemma_send_reaches_actor(rel: ISet<(L, R)>, m: L, x: R, x2: R, s: int, t: int, l: int, n: int)
    requires consistent(rel, s, n), rel.contains((m, x)), rstep_send(x, x2, s, t, l), acting(mhead(after_act(m, s, s, l))),
    ensures reaches(x2, s),
    decreases x2,
{
    assert(node_ok(rel, m, x, s, n));
    lemma_rhead_wf(x, n);
    lemma_send_wf(x, x2, s, t, l, n);
    let h = rhead(x);
    let m2 = after_act(m, s, s, l);
    match h {
        R::Comm { from, to, bs } => {
            if from == s {
                let b = rbs_lookup(*bs, l).unwrap();
                assert(rel.contains((m2, b)));
                assert(node_ok(rel, m2, b, s, n));
            } else {
                let bs2 = *x2->Comm_bs;
                lemma_all_in_from_lookups(rel, m, *bs);
                lemma_send_reaches_actor_bs(rel, m, *bs, bs2, s, t, l, n);
            }
        }
        R::Sent { cont, .. } => {
            assert(rel.contains((m, *cont)));
            lemma_send_reaches_actor(rel, m, *cont, *x2->Sent_cont, s, t, l, n);
        }
        _ => {}
    }
}

proof fn lemma_send_reaches_actor_bs(rel: ISet<(L, R)>, m: L, bs: RBs, bs2: RBs, s: int, t: int, l: int, n: int)
    requires consistent(rel, s, n), all_in(rel, m, bs), rstep_send_bs(bs, bs2, s, t, l), bs2 is Cons, acting(mhead(after_act(m, s, s, l))),
    ensures reaches_bs(bs2, s),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_send_reaches_actor(rel, m, bs->Cons_g, g2, s, t, l, n);
        if !(*rest2 is Nil) {
            lemma_send_reaches_actor_bs(rel, m, *bs->Cons_rest, *rest2, s, t, l, n);
        }
    }
}

proof fn lemma_recv_reaches_actor(rel: ISet<(L, R)>, m: L, x: R, x2: R, q: int, p: int, l: int, n: int)
    requires consistent(rel, q, n), rel.contains((m, x)), rstep_recv(x, x2, q, p, l), acting(mhead(after_act(m, q, q, l))),
    ensures reaches(x2, q),
    decreases x2,
{
    assert(node_ok(rel, m, x, q, n));
    lemma_rhead_wf(x, n);
    lemma_recv_wf(x, x2, q, p, l, n);
    let h = rhead(x);
    let m2 = after_act(m, q, q, l);
    match h {
        R::Sent { to, cont, .. } => {
            if to == q {
                assert(rel.contains((m2, *cont)));
                assert(node_ok(rel, m2, *cont, q, n));
            } else {
                assert(rel.contains((m, *cont)));
                lemma_recv_reaches_actor(rel, m, *cont, *x2->Sent_cont, q, p, l, n);
            }
        }
        R::Comm { bs, .. } => {
            let bs2 = *x2->Comm_bs;
            lemma_all_in_from_lookups(rel, m, *bs);
            lemma_recv_reaches_actor_bs(rel, m, *bs, bs2, q, p, l, n);
        }
        _ => {}
    }
}

proof fn lemma_recv_reaches_actor_bs(rel: ISet<(L, R)>, m: L, bs: RBs, bs2: RBs, q: int, p: int, l: int, n: int)
    requires consistent(rel, q, n), all_in(rel, m, bs), rstep_recv_bs(bs, bs2, q, p, l), bs2 is Cons, acting(mhead(after_act(m, q, q, l))),
    ensures reaches_bs(bs2, q),
    decreases bs2,
{
    if let RBs::Cons { g: g2, rest: rest2, .. } = bs2 {
        lemma_recv_reaches_actor(rel, m, bs->Cons_g, g2, q, p, l, n);
        if !(*rest2 is Nil) {
            lemma_recv_reaches_actor_bs(rel, m, *bs->Cons_rest, *rest2, q, p, l, n);
        }
    }
}

/// Membership of a stepped pair in the relation after a send.
proof fn lemma_rel_send_has(rel: ISet<(L, R)>, m: L, x: R, x2: R, s: int, t: int, l: int, u: int)
    requires rel.contains((m, x)), rstep_send(x, x2, s, t, l),
    ensures rel_send(rel, s, t, l, u).contains((after_act(m, u, s, l), x2)),
{
    let pr = (after_act(m, u, s, l), x2);
    assert(rel_send_member(rel, pr, s, t, l, u));
}

proof fn lemma_rel_recv_has(rel: ISet<(L, R)>, m: L, x: R, x2: R, q: int, p: int, l: int, u: int)
    requires rel.contains((m, x)), rstep_recv(x, x2, q, p, l),
    ensures rel_recv(rel, q, p, l, u).contains((after_act(m, u, q, l), x2)),
{
    let pr = (after_act(m, u, q, l), x2);
    assert(rel_recv_member(rel, pr, q, p, l, u));
}

/// A stepped pair satisfies the conditions in the relation after a send.
proof fn lemma_send_node(rel: ISet<(L, R)>, m: L, x: R, x2: R, s: int, t: int, l: int, u: int, n: int)
    requires consistent(rel, u, n), rel.contains((m, x)), rstep_send(x, x2, s, t, l), 0 <= u < n,
    ensures node_ok(rel_send(rel, s, t, l, u), after_act(m, u, s, l), x2, u, n),
{
    let rel2 = rel_send(rel, s, t, l, u);
    assert forall|pr: (L, R)| #[trigger] rel.contains(pr) implies rel2.contains(pr) by {
        assert(rel_send_member(rel, pr, s, t, l, u));
    }
    assert(node_ok(rel, m, x, u, n));
    lemma_rhead_wf(x, n);
    lemma_send_wf(x, x2, s, t, l, n);
    let h = rhead(x);
    let hm = mhead(m);
    let m2 = after_act(m, u, s, l);
    let hm2 = mhead(m2);
    match h {
        R::Comm { from, to, bs } => {
            if from == s {
                let b = rbs_lookup(*bs, l).unwrap();
                assert(x2 == R::Sent { from: s, to: t, label: l, cont: Box::new(b) });
                if u == s {
                    assert(rel.contains((m2, b)));
                    assert(node_ok(rel, m2, b, u, n));
                    assert(rel2.contains((m2, b)));
                } else if u == t {
                    assert(ls_lookup(*hm->Recv_ls, l) is Some);
                    assert(rel.contains((ls_lookup(*hm->Recv_ls, l).unwrap(), b)));
                    assert(rel2.contains((ls_lookup(*hm->Recv_ls, l).unwrap(), b)));
                } else {
                    assert(rel.contains((m, b)));
                    assert(node_ok(rel, m, b, u, n));
                    assert(rel2.contains((m, b)));
                }
            } else {
                let bs2 = *x2->Comm_bs;
                assert(x2 == R::Comm { from, to, bs: Box::new(bs2) });
                lemma_send_bs_labels(*bs, bs2, s, t, l);
                if u == from || u == to {
                    let hls = if u == from { *hm->Send_ls } else { *hm->Recv_ls };
                    assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some implies rel2.contains((ls_lookup(hls, label).unwrap(), rbs_lookup(bs2, label).unwrap())) by {
                        let b = rbs_lookup(*bs, label).unwrap();
                        let b2 = rbs_lookup(bs2, label).unwrap();
                        let mb = ls_lookup(hls, label).unwrap();
                        assert(rel.contains((mb, b)));
                        lemma_rel_send_has(rel, mb, b, b2, s, t, l, u);
                        assert(after_act(mb, u, s, l) == mb);
                    }
                    if u == from {
                        assert(labels_eq(hls, bs2));
                    } else {
                        assert(labels_sup(hls, bs2));
                    }
                } else {
                    assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some implies rel2.contains((m2, rbs_lookup(bs2, label).unwrap())) by {
                        let b = rbs_lookup(*bs, label).unwrap();
                        let b2 = rbs_lookup(bs2, label).unwrap();
                        assert(rel.contains((m, b)));
                        lemma_rel_send_has(rel, m, b, b2, s, t, l, u);
                    }
                    if acting(hm2) {
                        if u == s {
                            lemma_send_reaches_actor(rel, m, x, x2, s, t, l, n);
                        } else {
                            lemma_send_reaches(x, x2, s, t, l, u, n);
                        }
                    }
                }
            }
        }
        R::Sent { from, to, label, cont } => {
            let cont2 = *x2->Sent_cont;
            assert(x2 == R::Sent { from, to, label, cont: Box::new(cont2) });
            if u == to {
                let mb = ls_lookup(*hm->Recv_ls, label).unwrap();
                assert(rel.contains((mb, *cont)));
                lemma_rel_send_has(rel, mb, *cont, cont2, s, t, l, u);
                assert(after_act(mb, u, s, l) == mb);
            } else {
                assert(rel.contains((m, *cont)));
                lemma_rel_send_has(rel, m, *cont, cont2, s, t, l, u);
                if acting(hm2) {
                    if u == s {
                        lemma_send_reaches_actor(rel, m, x, x2, s, t, l, n);
                    } else {
                        lemma_send_reaches(x, x2, s, t, l, u, n);
                    }
                }
            }
        }
        _ => {}
    }
}

/// The relation after a send is consistent.
pub proof fn lemma_rel_send_consistent(rel: ISet<(L, R)>, s: int, t: int, l: int, u: int, n: int)
    requires consistent(rel, u, n), 0 <= u < n,
    ensures consistent(rel_send(rel, s, t, l, u), u, n),
{
    let rel2 = rel_send(rel, s, t, l, u);
    assert forall|pr: (L, R)| #[trigger] rel.contains(pr) implies rel2.contains(pr) by {
        assert(rel_send_member(rel, pr, s, t, l, u));
    }
    assert forall|m2: L, x2: R| #[trigger] rel2.contains((m2, x2)) implies node_ok(rel2, m2, x2, u, n) by {
        assert(rel_send_member(rel, (m2, x2), s, t, l, u));
        if rel.contains((m2, x2)) {
            lemma_node_ok_mono(rel, rel2, m2, x2, u, n);
        } else {
            let (m, x) = choose|m: L, x: R| #[trigger] rel.contains((m, x)) && rstep_send(x, x2, s, t, l) && m2 == after_act(m, u, s, l);
            lemma_send_node(rel, m, x, x2, s, t, l, u, n);
        }
    }
}

/// A stepped pair satisfies the conditions in the relation after a receive.
proof fn lemma_recv_node(rel: ISet<(L, R)>, m: L, x: R, x2: R, q: int, p: int, l: int, u: int, n: int)
    requires consistent(rel, u, n), rel.contains((m, x)), rstep_recv(x, x2, q, p, l), 0 <= u < n,
    ensures node_ok(rel_recv(rel, q, p, l, u), after_act(m, u, q, l), x2, u, n),
{
    let rel2 = rel_recv(rel, q, p, l, u);
    assert forall|pr: (L, R)| #[trigger] rel.contains(pr) implies rel2.contains(pr) by {
        assert(rel_recv_member(rel, pr, q, p, l, u));
    }
    assert(node_ok(rel, m, x, u, n));
    lemma_rhead_wf(x, n);
    lemma_recv_wf(x, x2, q, p, l, n);
    let h = rhead(x);
    let hm = mhead(m);
    let m2 = after_act(m, u, q, l);
    let hm2 = mhead(m2);
    match h {
        R::Sent { from, to, label, cont } => {
            if to == q {
                assert(x2 == *cont);
                if u == q {
                    assert(rel.contains((m2, *cont)));
                } else {
                    assert(rel.contains((m, *cont)));
                }
                assert(node_ok(rel, m2, x2, u, n));
                lemma_node_ok_mono(rel, rel2, m2, x2, u, n);
            } else {
                let cont2 = *x2->Sent_cont;
                assert(x2 == R::Sent { from, to, label, cont: Box::new(cont2) });
                if u == to {
                    let mb = ls_lookup(*hm->Recv_ls, label).unwrap();
                    assert(rel.contains((mb, *cont)));
                    lemma_rel_recv_has(rel, mb, *cont, cont2, q, p, l, u);
                    assert(after_act(mb, u, q, l) == mb);
                } else {
                    assert(rel.contains((m, *cont)));
                    lemma_rel_recv_has(rel, m, *cont, cont2, q, p, l, u);
                    if acting(hm2) {
                        if u == q {
                            lemma_recv_reaches_actor(rel, m, x, x2, q, p, l, n);
                        } else {
                            lemma_recv_reaches(x, x2, q, p, l, u, n);
                        }
                    }
                }
            }
        }
        R::Comm { from, to, bs } => {
            let bs2 = *x2->Comm_bs;
            assert(x2 == R::Comm { from, to, bs: Box::new(bs2) });
            lemma_recv_bs_labels(*bs, bs2, q, p, l);
            if u == from || u == to {
                let hls = if u == from { *hm->Send_ls } else { *hm->Recv_ls };
                assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some implies rel2.contains((ls_lookup(hls, label).unwrap(), rbs_lookup(bs2, label).unwrap())) by {
                    let b = rbs_lookup(*bs, label).unwrap();
                    let b2 = rbs_lookup(bs2, label).unwrap();
                    let mb = ls_lookup(hls, label).unwrap();
                    assert(rel.contains((mb, b)));
                    lemma_rel_recv_has(rel, mb, b, b2, q, p, l, u);
                    assert(after_act(mb, u, q, l) == mb);
                }
                if u == from {
                    assert(labels_eq(hls, bs2));
                } else {
                    assert(labels_sup(hls, bs2));
                }
            } else {
                assert forall|label: int| (#[trigger] rbs_lookup(bs2, label)) is Some implies rel2.contains((m2, rbs_lookup(bs2, label).unwrap())) by {
                    let b = rbs_lookup(*bs, label).unwrap();
                    let b2 = rbs_lookup(bs2, label).unwrap();
                    assert(rel.contains((m, b)));
                    lemma_rel_recv_has(rel, m, b, b2, q, p, l, u);
                }
                if acting(hm2) {
                    if u == q {
                        lemma_recv_reaches_actor(rel, m, x, x2, q, p, l, n);
                    } else {
                        lemma_recv_reaches(x, x2, q, p, l, u, n);
                    }
                }
            }
        }
        _ => {}
    }
}

pub proof fn lemma_rel_recv_consistent(rel: ISet<(L, R)>, q: int, p: int, l: int, u: int, n: int)
    requires consistent(rel, u, n), 0 <= u < n,
    ensures consistent(rel_recv(rel, q, p, l, u), u, n),
{
    let rel2 = rel_recv(rel, q, p, l, u);
    assert forall|pr: (L, R)| #[trigger] rel.contains(pr) implies rel2.contains(pr) by {
        assert(rel_recv_member(rel, pr, q, p, l, u));
    }
    assert forall|m2: L, x2: R| #[trigger] rel2.contains((m2, x2)) implies node_ok(rel2, m2, x2, u, n) by {
        assert(rel_recv_member(rel, (m2, x2), q, p, l, u));
        if rel.contains((m2, x2)) {
            lemma_node_ok_mono(rel, rel2, m2, x2, u, n);
        } else {
            let (m, x) = choose|m: L, x: R| #[trigger] rel.contains((m, x)) && rstep_recv(x, x2, q, p, l) && m2 == after_act(m, u, q, l);
            lemma_recv_node(rel, m, x, x2, q, p, l, u, n);
        }
    }
}

// ----------------------------------------------------- the invariant --

/// The invariant of a reachable configuration: a well-formed, good runtime
/// type that every role's type fits and whose queues are the channels.
pub open spec fn minv(c: NC, x: R, n: int) -> bool {
    &&& n >= 2
    &&& c.l.len() == n
    &&& c.ch.len() == n * n
    &&& rwf(x, n)
    &&& rgood(x)
    &&& forall|u: int| 0 <= u < n ==> #[trigger] fits(c.l[u], x, u, n)
    &&& forall|p: int, q: int| 0 <= p < n && 0 <= q < n ==> c.ch[p * n + q] == #[trigger] rqueue(x, p, q)
}

proof fn lemma_index(p: int, q: int, a: int, b: int, n: int)
    requires 0 <= p < n, 0 <= q < n, 0 <= a < n, 0 <= b < n,
    ensures 0 <= p * n + q < n * n, (p * n + q == a * n + b) ==> (p == a && q == b),
{
    assert(0 <= p * n + q < n * n) by (nonlinear_arith)
        requires 0 <= p < n, 0 <= q < n;
    if p * n + q == a * n + b {
        assert(p == a && q == b) by (nonlinear_arith)
            requires 0 <= p < n, 0 <= q < n, 0 <= a < n, 0 <= b < n, p * n + q == a * n + b;
    }
}

proof fn lemma_emb_no_sent(g: G)
    ensures no_sent(emb(g)),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_emb_no_sent(*body),
        G::Comm { bs, .. } => lemma_emb_no_sent_bs(*bs),
        _ => {}
    }
}

proof fn lemma_emb_no_sent_bs(bs: Bs)
    ensures no_sent_bs(emb_bs(bs)),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_emb_no_sent(g);
        lemma_emb_no_sent_bs(*rest);
    }
}

/// The initial configuration satisfies the invariant with the embedded
/// Protocol.
pub proof fn lemma_init(g: G, n: int)
    requires acceptn(g, n),
    ensures minv(ninit(g, n), emb(g), n),
{
    let x = emb(g);
    let c = ninit(g, n);
    lemma_emb_wf(g, n, Seq::<bool>::empty());
    lemma_emb_no_sent(g);
    lemma_no_sent_rgood(x);
    lemma_no_sent_queue(x);
    assert forall|u: int| 0 <= u < n implies #[trigger] fits(c.l[u], x, u, n) by {
        lemma_rproj_emb(g, u);
        lemma_rel0_consistent(u, n);
        assert(c.l[u] == projn(g, u).unwrap());
        lemma_refines_refl(mhead(projn(g, u).unwrap()));
        assert(rel0_member(c.l[u], x, u, n));
        assert(rel0(u, n).contains((c.l[u], x)));
    }
    assert forall|p: int, q: int| 0 <= p < n && 0 <= q < n implies c.ch[p * n + q] == #[trigger] rqueue(x, p, q) by {
        lemma_index(p, q, p, q, n);
    }
}

/// The head of a role's type in the semantics, under the invariant: an
/// action of `lhead` is the same action of `mhead`.
proof fn lemma_heads(m: L)
    ensures
        lhead(m) is Send || lhead(m) is Recv || lhead(m) is End ==> mhead(m) == lhead(m),
        mhead(m) is Send || mhead(m) is Recv || mhead(m) is End ==> mhead(m) == lhead(m),
{
    lemma_mhead_lhead(m);
}

/// A send by role `s` keeps the invariant, with the stepped runtime type.
proof fn lemma_send_minv(c: NC, x: R, n: int, s: int, label: int, x2: R)
    requires
        minv(c, x, n), 0 <= s < n,
        lhead(c.l[s]) is Send, 0 <= lhead(c.l[s])->Send_to < n, lhead(c.l[s])->Send_to != s,
        ls_lookup(*lhead(c.l[s])->Send_ls, label) is Some,
        rstep_send(x, x2, s, lhead(c.l[s])->Send_to, label),
    ensures minv(nsend(c, n, s, lhead(c.l[s])->Send_to, label, ls_lookup(*lhead(c.l[s])->Send_ls, label).unwrap()), x2, n),
{
    let t = lhead(c.l[s])->Send_to;
    let cont = ls_lookup(*lhead(c.l[s])->Send_ls, label).unwrap();
    let c2 = nsend(c, n, s, t, label, cont);
    lemma_heads(c.l[s]);
    lemma_send_wf(x, x2, s, t, label, n);
    lemma_send_good(x, x2, s, t, label, n);
    lemma_index(s, t, s, t, n);
    assert forall|u: int| 0 <= u < n implies #[trigger] fits(c2.l[u], x2, u, n) by {
        assert(fits(c.l[u], x, u, n));
        assert(fits(c.l[u], x, u, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((c.l[u], x));
        lemma_rel_send_consistent(rel, s, t, label, u, n);
        lemma_rel_send_has(rel, c.l[u], x, x2, s, t, label, u);
        assert(c2.l[u] == after_act(c.l[u], u, s, label));
    }
    assert forall|p: int, q: int| 0 <= p < n && 0 <= q < n implies c2.ch[p * n + q] == #[trigger] rqueue(x2, p, q) by {
        lemma_index(p, q, s, t, n);
        if p == s && q == t {
            assert(c2.ch[p * n + q] == c.ch[s * n + t].push(label));
        } else {
            assert(p * n + q != s * n + t);
            assert(c2.ch[p * n + q] == c.ch[p * n + q]);
        }
    }
}

/// A receive by role `q` keeps the invariant, with the stepped runtime type.
proof fn lemma_recv_minv(c: NC, x: R, n: int, q: int, label: int, x2: R)
    requires
        minv(c, x, n), 0 <= q < n,
        nrecv_enabled(c, n, q), label == c.ch[lhead(c.l[q])->Recv_from * n + q][0],
        rstep_recv(x, x2, q, lhead(c.l[q])->Recv_from, label),
    ensures minv(nrecv(c, n, q, lhead(c.l[q])->Recv_from, ls_lookup(*lhead(c.l[q])->Recv_ls, label).unwrap()), x2, n),
{
    let p = lhead(c.l[q])->Recv_from;
    let cont = ls_lookup(*lhead(c.l[q])->Recv_ls, label).unwrap();
    let c2 = nrecv(c, n, q, p, cont);
    lemma_heads(c.l[q]);
    lemma_recv_wf(x, x2, q, p, label, n);
    lemma_recv_good(x, x2, q, p, label, n);
    lemma_index(p, q, p, q, n);
    assert forall|u: int| 0 <= u < n implies #[trigger] fits(c2.l[u], x2, u, n) by {
        assert(fits(c.l[u], x, u, n));
        assert(fits(c.l[u], x, u, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((c.l[u], x));
        lemma_rel_recv_consistent(rel, q, p, label, u, n);
        lemma_rel_recv_has(rel, c.l[u], x, x2, q, p, label, u);
        assert(c2.l[u] == after_act(c.l[u], u, q, label));
    }
    assert forall|a: int, b: int| 0 <= a < n && 0 <= b < n implies c2.ch[a * n + b] == #[trigger] rqueue(x2, a, b) by {
        lemma_index(a, b, p, q, n);
        if a == p && b == q {
            assert(c2.ch[a * n + b] == c.ch[p * n + q].drop_first());
        } else {
            assert(a * n + b != p * n + q);
            assert(c2.ch[a * n + b] == c.ch[a * n + b]);
        }
    }
}

/// Every step of the configuration has a step of the runtime type that
/// keeps the invariant.
pub proof fn lemma_step(c: NC, x: R, n: int, c2: NC)
    requires minv(c, x, n), nstep(c, n, c2),
    ensures exists|x2: R| minv(c2, x2, n),
{
    let (r, label) = choose|r: int, label: int|
        0 <= r < n && (
            (lhead(c.l[r]) is Send && 0 <= lhead(c.l[r])->Send_to < n && lhead(c.l[r])->Send_to != r
                && ls_lookup(*lhead(c.l[r])->Send_ls, label) is Some
                && c2 == nsend(c, n, r, lhead(c.l[r])->Send_to, label, ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap()))
            || (nrecv_enabled(c, n, r) && label == c.ch[lhead(c.l[r])->Recv_from * n + r][0]
                && c2 == nrecv(c, n, r, lhead(c.l[r])->Recv_from, ls_lookup(*lhead(c.l[r])->Recv_ls, label).unwrap()))
        );
    let m = c.l[r];
    lemma_heads(m);
    assert(fits(c.l[r], x, r, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, r, n) && rel.contains((m, x));
    if lhead(m) is Send && 0 <= lhead(m)->Send_to < n && lhead(m)->Send_to != r
        && ls_lookup(*lhead(m)->Send_ls, label) is Some
        && c2 == nsend(c, n, r, lhead(m)->Send_to, label, ls_lookup(*lhead(m)->Send_ls, label).unwrap()) {
        let t = lhead(m)->Send_to;
        lemma_send_exists(rel, m, x, r, t, label, n);
        let x2 = choose|x2: R| rstep_send(x, x2, r, t, label);
        lemma_send_minv(c, x, n, r, label, x2);
    } else {
        let p = lhead(m)->Recv_from;
        lemma_index(p, r, p, r, n);
        lemma_recv_exists(rel, m, x, r, p, label, n);
        let x2 = choose|x2: R| rstep_recv(x, x2, r, p, label);
        lemma_recv_minv(c, x, n, r, label, x2);
    }
}

// ---------------------------------------------------- no waiting set --

/// A role whose head is a send is neither awaiting nor finished.
proof fn lemma_sender_free(c: NC, x: R, n: int, u: int)
    requires minv(c, x, n), 0 <= u < n, rhead(x) is Comm, rhead(x)->Comm_from == u,
    ensures !nblocked(c, n, u), !nterminal(c, u), lhead(c.l[u]) is Send, lhead(c.l[u])->Send_to == rhead(x)->Comm_to,
        labels_eq(*lhead(c.l[u])->Send_ls, *rhead(x)->Comm_bs),
{
    assert(fits(c.l[u], x, u, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((c.l[u], x));
    assert(node_ok(rel, c.l[u], x, u, n));
    let hm = mhead(c.l[u]);
    assert(hm is Send && hm->Send_to == rhead(x)->Comm_to && labels_eq(*hm->Send_ls, *rhead(x)->Comm_bs));
    lemma_heads(c.l[u]);
    assert(lhead(c.l[u]) == hm);
}

/// The receiver of a pending communication has a receive from its sender
/// with every label of it.
proof fn lemma_receiver_head(c: NC, x: R, n: int, u: int)
    requires minv(c, x, n), 0 <= u < n, rhead(x) is Comm, rhead(x)->Comm_to == u,
    ensures lhead(c.l[u]) is Recv, lhead(c.l[u])->Recv_from == rhead(x)->Comm_from, labels_sup(*lhead(c.l[u])->Recv_ls, *rhead(x)->Comm_bs),
{
    assert(fits(c.l[u], x, u, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((c.l[u], x));
    assert(node_ok(rel, c.l[u], x, u, n));
    lemma_rhead_wf(x, n);
    assert(rhead(x)->Comm_from != u);
    let hm = mhead(c.l[u]);
    assert(hm is Recv && hm->Recv_from == rhead(x)->Comm_from && labels_sup(*hm->Recv_ls, *rhead(x)->Comm_bs));
    lemma_heads(c.l[u]);
    assert(lhead(c.l[u]) == hm);
}

/// The receiver of a message in transit at the head can receive it.
proof fn lemma_transit_receiver(c: NC, x: R, n: int, u: int)
    requires minv(c, x, n), 0 <= u < n, rhead(x) is Sent, rhead(x)->Sent_to == u,
    ensures nrecv_enabled(c, n, u), !nblocked(c, n, u), !nterminal(c, u),
        lhead(c.l[u])->Recv_from == rhead(x)->Sent_from, c.ch[rhead(x)->Sent_from * n + u][0] == rhead(x)->Sent_label,
{
    let h = rhead(x);
    let p = h->Sent_from;
    assert(fits(c.l[u], x, u, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, u, n) && rel.contains((c.l[u], x));
    assert(node_ok(rel, c.l[u], x, u, n));
    let hm = mhead(c.l[u]);
    assert(hm is Recv && hm->Recv_from == p && ls_lookup(*hm->Recv_ls, h->Sent_label) is Some);
    lemma_heads(c.l[u]);
    assert(lhead(c.l[u]) == hm);
    lemma_rhead_wf(x, n);
    lemma_rhead_good(x, n);
    lemma_index(p, u, p, u, n);
    assert(rqueue(h, p, u) == seq![h->Sent_label].add(rqueue(*h->Sent_cont, p, u)));
    assert(c.ch[p * n + u] == rqueue(x, p, u));
    assert(c.ch[p * n + u][0] == h->Sent_label);
}

/// A role's awaiting and finishing are unchanged by a step that touches
/// neither its type nor the channel it awaits.
proof fn lemma_status_same(c: NC, c2: NC, n: int, u: int, a: int, b: int)
    requires
        c.l.len() == n, c.ch.len() == n * n, c2.l.len() == n, c2.ch.len() == n * n,
        0 <= u < n, 0 <= a < n, 0 <= b < n, u != b,
        c2.l[u] == c.l[u],
        forall|i: int| 0 <= i < n * n && i != a * n + b ==> #[trigger] c2.ch[i] == c.ch[i],
    ensures nblocked(c2, n, u) == nblocked(c, n, u), nterminal(c2, u) == nterminal(c, u),
        nblocked(c, n, u) ==> lhead(c2.l[u])->Recv_from == lhead(c.l[u])->Recv_from,
{
    let hm = lhead(c.l[u]);
    if hm is Recv && 0 <= hm->Recv_from < n {
        let p = hm->Recv_from;
        lemma_index(p, u, a, b, n);
        assert(c2.ch[p * n + u] == c.ch[p * n + u]);
    }
}

/// An awaiting member of a waiting set at first-path depth `d` of the
/// runtime type: impossible, by induction on `d`.
proof fn lemma_blocked_depth(c: NC, x: R, n: int, set: Seq<bool>, r: int, d: nat)
    requires minv(c, x, n), ndeadlock_set(c, n, set), 0 <= r < n, set[r], nblocked(c, n, r), fdepth(x, r) == d,
    ensures false,
    decreases d,
{
    let m = c.l[r];
    assert(fits(c.l[r], x, r, n));
    let rel = choose|rel: ISet<(L, R)>| #[trigger] consistent(rel, r, n) && rel.contains((m, x));
    assert(node_ok(rel, m, x, r, n));
    lemma_heads(m);
    assert(reaches(x, r));
    lemma_rhead_wf(x, n);
    lemma_rhead_good(x, n);
    lemma_rhead_reaches(x, n, r);
    let h = rhead(x);
    let p0 = lhead(m)->Recv_from;
    match h {
        R::End => {}
        R::Comm { from, to, bs } => {
            if from == r {
                lemma_sender_free(c, x, n, r);
            } else if to == r {
                lemma_receiver_head(c, x, n, r);
                assert(set[from]);
                lemma_sender_free(c, x, n, from);
            } else {
                // The sender at the head sends its first label, the receiver takes it.
                let p = from;
                let q = to;
                lemma_sender_free(c, x, n, p);
                lemma_receiver_head(c, x, n, q);
                assert(!set[p]);
                if set[q] {
                    assert(nblocked(c, n, q));
                    assert(set[p]);
                    assert(false);
                }
                assert(*bs is Cons);
                let l1 = bs->Cons_label;
                let first = bs->Cons_g;
                assert(rbs_lookup(*bs, l1) == Some(first));
                let ms = *lhead(c.l[p])->Send_ls;
                assert(ls_lookup(ms, l1) is Some);
                let cont_p = ls_lookup(ms, l1).unwrap();
                let c1 = nsend(c, n, p, q, l1, cont_p);
                let x1 = R::Sent { from: p, to: q, label: l1, cont: Box::new(first) };
                assert(rstep_send(x, x1, p, q, l1));
                lemma_send_minv(c, x, n, p, l1, x1);
                assert(minv(c1, x1, n));
                // The channel from p to q was empty: the receiver's head label is l1.
                lemma_no_sent_from_queue_bs(*bs, p);
                assert(rqueue(h, p, q) == rqueue_bs(*bs, p, q));
                lemma_index(p, q, p, q, n);
                assert(c.ch[p * n + q] == Seq::<int>::empty());
                assert(c1.ch[p * n + q] =~= seq![l1]);
                assert(c1.l[q] == c.l[q]);
                let mq = *lhead(c.l[q])->Recv_ls;
                assert(ls_lookup(mq, l1) is Some);
                assert(nrecv_enabled(c1, n, q));
                let cont_q = ls_lookup(mq, l1).unwrap();
                let c2 = nrecv(c1, n, q, p, cont_q);
                assert(rhead(x1) == x1);
                assert(rstep_recv(x1, first, q, p, l1));
                lemma_recv_minv(c1, x1, n, q, l1, first);
                assert(minv(c2, first, n));
                // The waiting set is one of c2.
                assert(c2.l.len() == n && c2.ch.len() == n * n);
                assert forall|u: int| 0 <= u < n && u != p && u != q implies c2.l[u] == c.l[u]
                    && nblocked(c2, n, u) == nblocked(c, n, u) && nterminal(c2, u) == nterminal(c, u)
                    && (nblocked(c, n, u) ==> lhead(c2.l[u])->Recv_from == lhead(c.l[u])->Recv_from) by {
                    assert forall|i: int| 0 <= i < n * n && i != p * n + q implies #[trigger] c2.ch[i] == c.ch[i] by {}
                    lemma_status_same(c, c2, n, u, p, q);
                }
                assert(r != p && r != q);
                assert(c2.l[r] == c.l[r]);
                assert forall|i: int| 0 <= i < n * n && i != p * n + q implies #[trigger] c2.ch[i] == c.ch[i] by {}
                assert(c2.l.len() == n && c2.ch.len() == n * n);
                lemma_status_same(c, c2, n, r, p, q);
                assert(nblocked(c2, n, r));
                assert forall|u: int| 0 <= u < n && #[trigger] set[u] implies nblocked(c2, n, u) || nterminal(c2, u) by {
                    assert(u != p && u != q);
                    assert(c2.l[u] == c.l[u]);
                }
                assert forall|u: int| 0 <= u < n && #[trigger] set[u] && nblocked(c2, n, u) implies set[lhead(c2.l[u])->Recv_from] by {
                    assert(u != p && u != q);
                    assert(c2.l[u] == c.l[u]);
                    assert(nblocked(c, n, u));
                }
                assert(ndeadlock_set(c2, n, set));
                assert(fdepth(h, r) == 1 + fdepth_bs(*bs, r));
                assert(fdepth_bs(*bs, r) == fdepth(first, r));
                lemma_blocked_depth(c2, first, n, set, r, fdepth(first, r));
            }
        }
        R::Sent { from, to, label, cont } => {
            if to == r {
                lemma_transit_receiver(c, x, n, r);
            } else {
                let p = from;
                let q = to;
                lemma_transit_receiver(c, x, n, q);
                assert(!set[q]);
                let mq = *lhead(c.l[q])->Recv_ls;
                let cont_q = ls_lookup(mq, label).unwrap();
                let c2 = nrecv(c, n, q, p, cont_q);
                assert(rstep_recv(x, *cont, q, p, label));
                lemma_recv_minv(c, x, n, q, label, *cont);
                assert(minv(c2, *cont, n));
                lemma_index(p, q, p, q, n);
                assert forall|u: int| 0 <= u < n && u != q implies c2.l[u] == c.l[u]
                    && nblocked(c2, n, u) == nblocked(c, n, u) && nterminal(c2, u) == nterminal(c, u)
                    && (nblocked(c, n, u) ==> lhead(c2.l[u])->Recv_from == lhead(c.l[u])->Recv_from) by {
                    assert forall|i: int| 0 <= i < n * n && i != p * n + q implies #[trigger] c2.ch[i] == c.ch[i] by {}
                    lemma_status_same(c, c2, n, u, p, q);
                }
                assert(r != q);
                assert(c2.l[r] == c.l[r]);
                assert forall|i: int| 0 <= i < n * n && i != p * n + q implies #[trigger] c2.ch[i] == c.ch[i] by {}
                assert(c2.l.len() == n && c2.ch.len() == n * n);
                lemma_status_same(c, c2, n, r, p, q);
                assert(nblocked(c2, n, r));
                assert forall|u: int| 0 <= u < n && #[trigger] set[u] implies nblocked(c2, n, u) || nterminal(c2, u) by {
                    assert(u != q);
                    assert(c2.l[u] == c.l[u]);
                }
                assert forall|u: int| 0 <= u < n && #[trigger] set[u] && nblocked(c2, n, u) implies set[lhead(c2.l[u])->Recv_from] by {
                    assert(u != q);
                    assert(c2.l[u] == c.l[u]);
                    assert(nblocked(c, n, u));
                }
                assert(ndeadlock_set(c2, n, set));
                assert(fdepth(h, r) == 1 + fdepth(*cont, r));
                lemma_blocked_depth(c2, *cont, n, set, r, fdepth(*cont, r));
            }
        }
        _ => {}
    }
}

/// Under the invariant no waiting set exists.
pub proof fn lemma_no_deadlock(c: NC, x: R, n: int)
    requires minv(c, x, n),
    ensures !ndeadlocked(c, n),
{
    if ndeadlocked(c, n) {
        let set = choose|set: Seq<bool>| ndeadlock_set(c, n, set);
        let r = choose|r: int| 0 <= r < n && set[r] && nblocked(c, n, r);
        lemma_blocked_depth(c, x, n, set, r, fdepth(x, r));
    }
}

proof fn lemma_run_minv(g: G, n: int, path: Seq<NC>)
    requires acceptn(g, n), nrun(g, n, path),
    ensures exists|x: R| minv(path.last(), x, n),
    decreases path.len(),
{
    if path.len() == 1 {
        lemma_init(g, n);
        assert(path.last() == ninit(g, n));
        assert(minv(path.last(), emb(g), n));
    } else {
        let prefix = path.drop_last();
        assert(nrun(g, n, prefix));
        lemma_run_minv(g, n, prefix);
        let x = choose|x: R| minv(prefix.last(), x, n);
        assert(nstep(prefix.last(), n, path.last()));
        lemma_step(prefix.last(), x, n, path.last());
    }
}

/// The multiparty progress theorem (R41.3, task 21.7): every Protocol of
/// `n` roles the acceptance conditions over the syntax admit has no
/// waiting set in any reachable configuration of the asynchronous product
/// of its projections.
pub proof fn theorem_multiparty_progress(g: G, n: int)
    requires acceptn(g, n),
    ensures forall|c: NC| nreachable(g, n, c) ==> !ndeadlocked(c, n),
{
    assert forall|c: NC| nreachable(g, n, c) implies !ndeadlocked(c, n) by {
        let path = choose|path: Seq<NC>| #[trigger] nrun(g, n, path) && path.last() == c;
        lemma_run_minv(g, n, path);
        let x = choose|x: R| minv(c, x, n);
        lemma_no_deadlock(c, x, n);
    }
}

} // verus!
