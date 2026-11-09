//! The general theorem of R41.3 (task 21.6 follow-up, 2026-11-08): every
//! Protocol the Protocol_Checker's structural conditions accept has
//! progress, for every run, with no enumeration of its configurations.
//!
//! This module states the Protocol syntax, well-formedness, and projection
//! as Verus specifications over the syntax and proves, for the two-role
//! fragment (every Endpoint of this Profile binds two roles), that no
//! reachable configuration of the product of the projections is
//! `progress::deadlocked`. The conclusion is the one `compose.rs` takes per
//! member, so the system theorem composes with it.
//!
//! # Syntax
//!
//! `G` is a global type over roles numbered from 0: `End`; `Comm` (a
//! message or labelled choice from one role to another, with a list of
//! branches each a label and a continuation; a message is a one-branch
//! `Comm`); `Rec` and `Var` (recursion in de Bruijn form: `Var(0)` is the
//! innermost `Rec`). `L` is a local type: `Send`, `Recv`, `Rec`, `Var`,
//! `End`. Branch lists are the cons lists `Bs` and `Ls`.
//!
//! # Acceptance conditions over the syntax (the checker's)
//!
//! `wf2(g)`: closed, every recursion variable under a message (guarded,
//! R38.5), every communication between the two distinct roles 0 and 1
//! with at least one branch and pairwise distinct labels. The parser's
//! separation of `Msg` from `Choice` (a choice has at least two branches)
//! is a presentation detail: a one-branch `Comm` is a message.
//!
//! # Semantics
//!
//! The semantics is that of `progress.rs`: the roles' automata over FIFO
//! channels. The link is `flattens(s, g, tb)`: a table `tb[r]` of
//! head-normal local types per role, entry 0 being the head of the
//! projection, such that the automaton `s` has exactly the transitions of
//! the table's branches. `explore::certify` builds such a table and
//! automaton (its verification against `flattens` is the remaining glue).
//!
//! # The invariant
//!
//! In every reachable configuration one role leads: there is a global type
//! `g0` (well formed) such that the lagging role's state is the head of
//! `g0`'s projection, the channel from the leader to the lagger holds a
//! sequence of labels `w` that `g0` can follow (each a message from the
//! leader to the lagger), the leader's state is the head of the projection
//! of the type reached by following `w`, and the other channel is empty
//! (`inv`). The initial configuration satisfies it with `g0 = g` and `w`
//! empty; every step preserves it (`lemma_step`); and under it no waiting
//! set exists (`lemma_no_deadlock`): with `w` non-empty the lagger can
//! receive `w[0]`; with `w` empty both roles stand at `g0`, whose head is
//! `End` (both finished) or a communication whose sender can send.

use vstd::prelude::*;

#[cfg(verus_keep_ghost)]
use crate::progress::{at, blocked, chan_of, deadlock_set, deadlocked, enabled, has_trans, initial, reachable, run, step, succ, terminal, wf_cfg, Cfg};
use crate::progress::{System, Trans};

verus! {

// ------------------------------------------------------------- syntax --

pub enum G {
    End,
    Comm { from: int, to: int, bs: Box<Bs> },
    Rec { body: Box<G> },
    Var { n: nat },
}

pub enum Bs {
    Nil,
    Cons { label: int, g: G, rest: Box<Bs> },
}

pub enum L {
    End,
    Send { to: int, ls: Box<Ls> },
    Recv { from: int, ls: Box<Ls> },
    Rec { body: Box<L> },
    Var { n: nat },
}

pub enum Ls {
    Nil,
    Cons { label: int, l: L, rest: Box<Ls> },
}

// ----------------------------------------------------- well-formedness --

pub open spec fn bs_len(bs: Bs) -> nat
    decreases bs,
{
    match bs {
        Bs::Nil => 0,
        Bs::Cons { rest, .. } => 1 + bs_len(*rest),
    }
}

pub open spec fn bs_has_label(bs: Bs, label: int) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => false,
        Bs::Cons { label: l, rest, .. } => l == label || bs_has_label(*rest, label),
    }
}

/// The branch with `label`, when there is one.
pub open spec fn bs_lookup(bs: Bs, label: int) -> Option<G>
    decreases bs,
{
    match bs {
        Bs::Nil => None,
        Bs::Cons { label: l, g, rest } => if l == label { Some(g) } else { bs_lookup(*rest, label) },
    }
}

pub open spec fn bs_distinct(bs: Bs) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => true,
        Bs::Cons { label, rest, .. } => !bs_has_label(*rest, label) && bs_distinct(*rest),
    }
}

/// The guard environment of a position in a type: one flag per enclosing
/// recursion, outermost first, true when a message lies between that
/// recursion and the position. Every recursion variable must be guarded
/// (R38.5: a `continue` with no message between it and its `rec`); this
/// is the standard guardedness, the checker's, and not the stricter "a
/// message since the nearest recursion" (the search of 2026-11-08 found
/// `rec X { D: d -> b; rec Y { continue X } }`, which the checker accepts
/// and the stricter rule refused).
pub open spec fn all_true(env: Seq<bool>) -> Seq<bool> {
    Seq::new(env.len(), |i: int| true)
}

/// Well-formedness of a two-role global type under the guard environment
/// `env`.
pub open spec fn wf2_at(g: G, env: Seq<bool>) -> bool
    decreases g,
{
    match g {
        G::End => true,
        G::Var { n } => n < env.len() && env[env.len() - 1 - n],
        G::Rec { body } => wf2_at(*body, env.push(false)),
        G::Comm { from, to, bs } => {
            &&& (from == 0 || from == 1)
            &&& (to == 0 || to == 1)
            &&& from != to
            &&& bs_len(*bs) >= 1
            &&& bs_distinct(*bs)
            &&& wf2_bs(*bs, all_true(env))
        }
    }
}

pub open spec fn wf2_bs(bs: Bs, env: Seq<bool>) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => true,
        Bs::Cons { g, rest, .. } => wf2_at(g, env) && wf2_bs(*rest, env),
    }
}

/// The checker's structural acceptance of a two-role Protocol: closed and
/// well formed.
pub open spec fn wf2(g: G) -> bool {
    wf2_at(g, Seq::<bool>::empty())
}

/// Every variable of `g` refers to one of the `m` innermost enclosing
/// recursions (none to a recursion outside them).
pub open spec fn closed_at(g: G, m: nat) -> bool
    decreases g,
{
    match g {
        G::End => true,
        G::Var { n } => n < m,
        G::Rec { body } => closed_at(*body, m + 1),
        G::Comm { bs, .. } => closed_at_bs(*bs, m),
    }
}

pub open spec fn closed_at_bs(bs: Bs, m: nat) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => true,
        Bs::Cons { g, rest, .. } => closed_at(g, m) && closed_at_bs(*rest, m),
    }
}

// --------------------------------------------------------- substitution --

/// `g` with `Var(k)` replaced by the closed type `by` and the variables
/// above `k` renumbered down (the body of `Rec` is substituted at `k + 1`).
pub open spec fn subst(g: G, k: nat, by: G) -> G
    decreases g,
{
    match g {
        G::End => G::End,
        G::Var { n } => if n == k { by } else if n > k { G::Var { n: (n - 1) as nat } } else { G::Var { n } },
        G::Rec { body } => G::Rec { body: Box::new(subst(*body, k + 1, by)) },
        G::Comm { from, to, bs } => G::Comm { from, to, bs: Box::new(subst_bs(*bs, k, by)) },
    }
}

pub open spec fn subst_bs(bs: Bs, k: nat, by: G) -> Bs
    decreases bs,
{
    match bs {
        Bs::Nil => Bs::Nil,
        Bs::Cons { label, g, rest } => Bs::Cons { label, g: subst(g, k, by), rest: Box::new(subst_bs(*rest, k, by)) },
    }
}

/// One unfolding of a recursion.
pub open spec fn unfold(g: G) -> G {
    match g {
        G::Rec { body } => subst(*body, 0, g),
        _ => g,
    }
}

/// The recursions at the top of a type.
pub open spec fn lead_recs(g: G) -> nat
    decreases g,
{
    match g {
        G::Rec { body } => 1 + lead_recs(*body),
        _ => 0,
    }
}

/// The spine of a type is recursions down to a communication or an end
/// (never a variable): what guardedness gives at the top.
pub open spec fn spine_ok(g: G) -> bool
    decreases g,
{
    match g {
        G::Rec { body } => spine_ok(*body),
        G::Var { .. } => false,
        _ => true,
    }
}

/// Under recursions none of which a message has guarded, the spine ends
/// in a communication or an end: a variable there would be unguarded.
proof fn lemma_wf_spine(g: G, env: Seq<bool>)
    requires wf2_at(g, env), forall|i: int| 0 <= i < env.len() ==> !env[i],
    ensures spine_ok(g),
    decreases g,
{
    if let G::Rec { body } = g {
        lemma_wf_spine(*body, env.push(false));
    }
}

proof fn lemma_subst_spine(g: G, k: nat, by: G)
    requires spine_ok(g),
    ensures spine_ok(subst(g, k, by)), lead_recs(subst(g, k, by)) == lead_recs(g),
    decreases g,
{
    if let G::Rec { body } = g {
        lemma_subst_spine(*body, k + 1, by);
    }
}

/// Well-formedness bounds every variable by the environment's depth.
proof fn lemma_wf_closed_at(g: G, env: Seq<bool>)
    requires wf2_at(g, env),
    ensures closed_at(g, env.len()),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_wf_closed_at(*body, env.push(false)),
        G::Comm { bs, .. } => lemma_wf_closed_at_bs(*bs, all_true(env)),
        _ => {}
    }
}

proof fn lemma_wf_closed_at_bs(bs: Bs, env: Seq<bool>)
    requires wf2_bs(bs, env),
    ensures closed_at_bs(bs, env.len()),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_wf_closed_at(g, env);
        lemma_wf_closed_at_bs(*rest, env);
    }
}

/// A type whose variables all point inside `inner` is well formed under
/// any outer environment.
proof fn lemma_env_swap(g: G, outer1: Seq<bool>, outer2: Seq<bool>, inner: Seq<bool>)
    requires wf2_at(g, outer1.add(inner)), closed_at(g, inner.len()),
    ensures wf2_at(g, outer2.add(inner)),
    decreases g,
{
    match g {
        G::End => {}
        G::Var { n } => {
            let e1 = outer1.add(inner);
            let e2 = outer2.add(inner);
            assert(e1[e1.len() - 1 - n] == inner[inner.len() - 1 - n]);
            assert(e2[e2.len() - 1 - n] == inner[inner.len() - 1 - n]);
        }
        G::Rec { body } => {
            assert(outer1.add(inner).push(false) =~= outer1.add(inner.push(false)));
            assert(outer2.add(inner).push(false) =~= outer2.add(inner.push(false)));
            lemma_env_swap(*body, outer1, outer2, inner.push(false));
        }
        G::Comm { bs, .. } => {
            assert(all_true(outer1.add(inner)) =~= all_true(outer1).add(all_true(inner)));
            assert(all_true(outer2.add(inner)) =~= all_true(outer2).add(all_true(inner)));
            lemma_env_swap_bs(*bs, all_true(outer1), all_true(outer2), all_true(inner));
        }
    }
}

proof fn lemma_env_swap_bs(bs: Bs, outer1: Seq<bool>, outer2: Seq<bool>, inner: Seq<bool>)
    requires wf2_bs(bs, outer1.add(inner)), closed_at_bs(bs, inner.len()),
    ensures wf2_bs(bs, outer2.add(inner)),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_env_swap(g, outer1, outer2, inner);
        lemma_env_swap_bs(*rest, outer1, outer2, inner);
    }
}

/// A closed well-formed type is well formed under any environment.
proof fn lemma_closed_any_env(by: G, env: Seq<bool>)
    requires wf2(by),
    ensures wf2_at(by, env),
{
    lemma_wf_closed_at(by, Seq::<bool>::empty());
    assert(Seq::<bool>::empty().add(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
    assert(env.add(Seq::<bool>::empty()) =~= env);
    lemma_env_swap(by, Seq::<bool>::empty(), env, Seq::<bool>::empty());
}

/// Substituting a closed well-formed type for the outermost recursion's
/// variable removes that recursion from the environment.
proof fn lemma_subst_wf(g: G, e0: bool, rest: Seq<bool>, by: G)
    requires wf2_at(g, seq![e0].add(rest)), wf2(by),
    ensures wf2_at(subst(g, rest.len() as nat, by), rest),
    decreases g,
{
    let env = seq![e0].add(rest);
    match g {
        G::End => {}
        G::Var { n } => {
            if n == rest.len() {
                lemma_closed_any_env(by, rest);
            } else {
                assert(n < rest.len());
                assert(env[env.len() - 1 - n] == rest[rest.len() - 1 - n]);
            }
        }
        G::Rec { body } => {
            assert(env.push(false) =~= seq![e0].add(rest.push(false)));
            lemma_subst_wf(*body, e0, rest.push(false), by);
        }
        G::Comm { from, to, bs } => {
            assert(all_true(env) =~= seq![true].add(all_true(rest)));
            lemma_subst_wf_bs(*bs, true, all_true(rest), by);
            lemma_subst_bs_shape(*bs, rest.len() as nat, by);
        }
    }
}

proof fn lemma_subst_wf_bs(bs: Bs, e0: bool, rest: Seq<bool>, by: G)
    requires wf2_bs(bs, seq![e0].add(rest)), wf2(by),
    ensures wf2_bs(subst_bs(bs, rest.len() as nat, by), rest),
    decreases bs,
{
    if let Bs::Cons { g, rest: tail, .. } = bs {
        lemma_subst_wf(g, e0, rest, by);
        lemma_subst_wf_bs(*tail, e0, rest, by);
    }
}

/// Substitution keeps the labels of a branch list.
proof fn lemma_subst_bs_shape(bs: Bs, k: nat, by: G)
    ensures
        bs_len(subst_bs(bs, k, by)) == bs_len(bs),
        bs_distinct(subst_bs(bs, k, by)) == bs_distinct(bs),
        forall|label: int| bs_has_label(subst_bs(bs, k, by), label) == bs_has_label(bs, label),
    decreases bs,
{
    if let Bs::Cons { label: l, g, rest } = bs {
        lemma_subst_bs_shape(*rest, k, by);
        assert forall|label: int| bs_has_label(subst_bs(bs, k, by), label) == bs_has_label(bs, label) by {
            assert(subst_bs(bs, k, by) == Bs::Cons { label: l, g: subst(g, k, by), rest: Box::new(subst_bs(*rest, k, by)) });
            assert(bs_has_label(subst_bs(bs, k, by), label) == (l == label || bs_has_label(subst_bs(*rest, k, by), label)));
            assert(bs_has_label(bs, label) == (l == label || bs_has_label(*rest, label)));
        }
    }
}

/// Unfolding a closed well-formed recursion gives a closed well-formed
/// type with one recursion fewer at the top.
pub proof fn lemma_unfold_wf(g: G)
    requires wf2(g), g is Rec,
    ensures wf2(unfold(g)), lead_recs(unfold(g)) == lead_recs(g) - 1,
{
    let body = *g->Rec_body;
    assert(Seq::<bool>::empty().push(false) =~= seq![false].add(Seq::<bool>::empty()));
    lemma_subst_wf(body, false, Seq::<bool>::empty(), g);
    lemma_wf_spine(body, seq![false]);
    lemma_subst_spine(body, 0, g);
}

// ---------------------------------------------------------------- head --

/// The head normal form: every leading recursion unfolded. Defined for
/// well-formed closed types (the unfolding count decreases).
pub open spec fn head(g: G) -> G
    decreases lead_recs(g),
    when wf2(g)
    via head_decreases
{
    if g is Rec {
        head(unfold(g))
    } else {
        g
    }
}

#[via_fn]
proof fn head_decreases(g: G) {
    if wf2(g) && g is Rec {
        lemma_unfold_wf(g);
    }
}

pub proof fn lemma_head_wf(g: G)
    requires wf2(g),
    ensures wf2(head(g)), !(head(g) is Rec), !(head(g) is Var),
    decreases lead_recs(g),
{
    if g is Rec {
        lemma_unfold_wf(g);
        lemma_head_wf(unfold(g));
    }
}

// ---------------------------------------------------------- projection --

pub open spec fn ls_lookup(ls: Ls, label: int) -> Option<L>
    decreases ls,
{
    match ls {
        Ls::Nil => None,
        Ls::Cons { label: l, l: cont, rest } => if l == label { Some(cont) } else { ls_lookup(*rest, label) },
    }
}

/// The projection of a two-role global type onto role `r` (R38.3): the
/// sender's side of a communication is an internal choice, the receiver's
/// an external one; no merge is needed, since both roles take part in
/// every communication (a role not involved gets `End`, a case
/// well-formedness excludes).
pub open spec fn proj(g: G, r: int) -> L
    decreases g,
{
    match g {
        G::End => L::End,
        G::Var { n } => L::Var { n },
        G::Rec { body } => L::Rec { body: Box::new(proj(*body, r)) },
        G::Comm { from, to, bs } => {
            if r == from {
                L::Send { to, ls: Box::new(proj_bs(*bs, r)) }
            } else if r == to {
                L::Recv { from, ls: Box::new(proj_bs(*bs, r)) }
            } else {
                L::End
            }
        }
    }
}

pub open spec fn proj_bs(bs: Bs, r: int) -> Ls
    decreases bs,
{
    match bs {
        Bs::Nil => Ls::Nil,
        Bs::Cons { label, g, rest } => Ls::Cons { label, l: proj(g, r), rest: Box::new(proj_bs(*rest, r)) },
    }
}

proof fn lemma_lookup_proj(bs: Bs, r: int, label: int)
    ensures
        bs_lookup(bs, label) is Some ==> ls_lookup(proj_bs(bs, r), label) == Some(proj(bs_lookup(bs, label).unwrap(), r)),
        bs_lookup(bs, label) is None ==> ls_lookup(proj_bs(bs, r), label) is None,
    decreases bs,
{
    if let Bs::Cons { rest, .. } = bs {
        lemma_lookup_proj(*rest, r, label);
    }
}

proof fn lemma_has_lookup(bs: Bs, label: int)
    ensures bs_has_label(bs, label) == (bs_lookup(bs, label) is Some),
    decreases bs,
{
    if let Bs::Cons { rest, .. } = bs {
        lemma_has_lookup(*rest, label);
    }
}

/// A branch of a closed well-formed communication is closed and well
/// formed (`all_true` of the empty environment is the empty environment).
proof fn lemma_branch_wf(bs: Bs, label: int)
    requires wf2_bs(bs, Seq::<bool>::empty()), bs_lookup(bs, label) is Some,
    ensures wf2(bs_lookup(bs, label).unwrap()),
    decreases bs,
{
    if let Bs::Cons { label: l, rest, .. } = bs {
        if l != label {
            lemma_branch_wf(*rest, label);
        }
    }
}

/// `wf2` of a communication gives its branches under the empty environment.
proof fn lemma_comm_branches_wf(g: G)
    requires wf2(g), g is Comm,
    ensures wf2_bs(*g->Comm_bs, Seq::<bool>::empty()),
{
    assert(all_true(Seq::<bool>::empty()) =~= Seq::<bool>::empty());
}

// --------------------------------------------- local substitution, head --

pub open spec fn lsubst(l: L, k: nat, by: L) -> L
    decreases l,
{
    match l {
        L::End => L::End,
        L::Var { n } => if n == k { by } else if n > k { L::Var { n: (n - 1) as nat } } else { L::Var { n } },
        L::Rec { body } => L::Rec { body: Box::new(lsubst(*body, k + 1, by)) },
        L::Send { to, ls } => L::Send { to, ls: Box::new(lsubst_ls(*ls, k, by)) },
        L::Recv { from, ls } => L::Recv { from, ls: Box::new(lsubst_ls(*ls, k, by)) },
    }
}

pub open spec fn lsubst_ls(ls: Ls, k: nat, by: L) -> Ls
    decreases ls,
{
    match ls {
        Ls::Nil => Ls::Nil,
        Ls::Cons { label, l, rest } => Ls::Cons { label, l: lsubst(l, k, by), rest: Box::new(lsubst_ls(*rest, k, by)) },
    }
}

pub open spec fn lunfold(l: L) -> L {
    match l {
        L::Rec { body } => lsubst(*body, 0, l),
        _ => l,
    }
}

pub open spec fn llead_recs(l: L) -> nat
    decreases l,
{
    match l {
        L::Rec { body } => 1 + llead_recs(*body),
        _ => 0,
    }
}

/// The head of a local type, by at most `n` unfoldings.
pub open spec fn lhead_n(l: L, n: nat) -> L
    decreases n,
{
    if n == 0 || !(l is Rec) { l } else { lhead_n(lunfold(l), (n - 1) as nat) }
}

/// The head normal form of a local type (for the projection of a
/// well-formed type, every leading recursion unfolded).
pub open spec fn lhead(l: L) -> L {
    lhead_n(l, llead_recs(l))
}

/// Projection commutes with substitution.
proof fn lemma_proj_subst(g: G, k: nat, by: G, r: int)
    ensures proj(subst(g, k, by), r) == lsubst(proj(g, r), k, proj(by, r)),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_proj_subst(*body, k + 1, by, r),
        G::Comm { bs, .. } => lemma_proj_subst_bs(*bs, k, by, r),
        _ => {}
    }
}

proof fn lemma_proj_subst_bs(bs: Bs, k: nat, by: G, r: int)
    ensures proj_bs(subst_bs(bs, k, by), r) == lsubst_ls(proj_bs(bs, r), k, proj(by, r)),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_proj_subst(g, k, by, r);
        lemma_proj_subst_bs(*rest, k, by, r);
    }
}

proof fn lemma_proj_lead(g: G, r: int)
    ensures llead_recs(proj(g, r)) == lead_recs(g),
    decreases g,
{
    if let G::Rec { body } = g {
        lemma_proj_lead(*body, r);
    }
}

/// The head of the projection is the projection of the head.
pub proof fn lemma_lhead_proj(g: G, r: int)
    requires wf2(g),
    ensures lhead(proj(g, r)) == proj(head(g), r),
    decreases lead_recs(g),
{
    lemma_proj_lead(g, r);
    if g is Rec {
        lemma_unfold_wf(g);
        let body = *g->Rec_body;
        lemma_proj_subst(body, 0, g, r);
        assert(lunfold(proj(g, r)) == proj(unfold(g), r));
        lemma_proj_lead(unfold(g), r);
        lemma_lhead_proj(unfold(g), r);
        assert(lhead_n(proj(g, r), llead_recs(proj(g, r))) == lhead_n(proj(unfold(g), r), llead_recs(proj(unfold(g), r))));
    }
}

// ------------------------------------------------------ local semantics --

/// A configuration of the two roles: each role's local type (not
/// necessarily in head normal form) and the two channels, `ch[r]` from
/// role `r` to the other, oldest label first.
pub ghost struct LC {
    pub l: Seq<L>,
    pub ch: Seq<Seq<int>>,
}

pub open spec fn lc_wf(c: LC) -> bool {
    c.l.len() == 2 && c.ch.len() == 2
}

pub open spec fn linit(g: G) -> LC {
    LC { l: seq![proj(g, 0), proj(g, 1)], ch: seq![Seq::<int>::empty(), Seq::<int>::empty()] }
}

pub open spec fn other(r: int) -> int {
    1 - r
}

/// Role `r` sends `label` and continues as `cont`.
pub open spec fn lsend(c: LC, r: int, label: int, cont: L) -> LC {
    LC { l: c.l.update(r, cont), ch: c.ch.update(r, c.ch[r].push(label)) }
}

/// Role `r` receives the oldest label of the other role's channel and
/// continues as `cont`.
pub open spec fn lrecv(c: LC, r: int, cont: L) -> LC {
    LC { l: c.l.update(r, cont), ch: c.ch.update(other(r), c.ch[other(r)].drop_first()) }
}

pub open spec fn recv_enabled(c: LC, r: int) -> bool {
    &&& lhead(c.l[r]) is Recv
    &&& lhead(c.l[r])->Recv_from == other(r)
    &&& c.ch[other(r)].len() > 0
    &&& ls_lookup(*lhead(c.l[r])->Recv_ls, c.ch[other(r)][0]) is Some
}

pub open spec fn lstep(c: LC, c2: LC) -> bool {
    exists|r: int, label: int|
        (r == 0 || r == 1) && (
            (lhead(c.l[r]) is Send && lhead(c.l[r])->Send_to == other(r)
                && ls_lookup(*lhead(c.l[r])->Send_ls, label) is Some
                && c2 == lsend(c, r, label, ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap()))
            || (recv_enabled(c, r) && label == c.ch[other(r)][0]
                && c2 == lrecv(c, r, ls_lookup(*lhead(c.l[r])->Recv_ls, label).unwrap()))
        )
}

pub open spec fn lrun(g: G, path: Seq<LC>) -> bool {
    &&& path.len() > 0
    &&& path[0] == linit(g)
    &&& forall|i: int| 1 <= i < path.len() ==> lstep(path[i - 1], #[trigger] path[i])
}

pub open spec fn lreachable(g: G, c: LC) -> bool {
    exists|path: Seq<LC>| #[trigger] lrun(g, path) && path.last() == c
}

/// Role `r` awaits a message: its head is a receive that is not enabled.
pub open spec fn lblocked(c: LC, r: int) -> bool {
    lhead(c.l[r]) is Recv && !recv_enabled(c, r)
}

/// Role `r` has finished.
pub open spec fn lterminal(c: LC, r: int) -> bool {
    lhead(c.l[r]) is End
}

/// R41.3's waiting set over two roles: with two roles the only set whose
/// awaited senders lie inside it is both roles, so a deadlock is both
/// roles awaiting or finished, at least one awaiting (a wait on a
/// finished peer included, as in `progress::deadlock_set`).
pub open spec fn ldeadlocked(c: LC) -> bool {
    &&& (lblocked(c, 0) || lblocked(c, 1))
    &&& (lblocked(c, 0) || lterminal(c, 0))
    &&& (lblocked(c, 1) || lterminal(c, 1))
}

// ------------------------------------------------------- the invariant --

/// Following the labels `w` from `g`, each a message from `from` to `to`.
pub open spec fn follow(g: G, w: Seq<int>, from: int, to: int) -> Option<G>
    decreases w.len(),
{
    if !wf2(g) {
        None
    } else if w.len() == 0 {
        Some(g)
    } else {
        let h = head(g);
        if h is Comm && h->Comm_from == from && h->Comm_to == to {
            match bs_lookup(*h->Comm_bs, w[0]) {
                Some(cont) => follow(cont, w.drop_first(), from, to),
                None => None,
            }
        } else {
            None
        }
    }
}

/// The invariant of every reachable configuration: a leader, a global
/// type `g0` at the lagger's position, the leader ahead by the labels in
/// transit, which `g0` follows as messages from the leader to the lagger.
pub open spec fn inv_with(c: LC, g0: G, leader: int) -> bool {
    let lag = other(leader);
    &&& lc_wf(c)
    &&& (leader == 0 || leader == 1)
    &&& wf2(g0)
    &&& c.ch[lag].len() == 0
    &&& lhead(c.l[lag]) == lhead(proj(g0, lag))
    &&& follow(g0, c.ch[leader], leader, lag) is Some
    &&& lhead(c.l[leader]) == lhead(proj(follow(g0, c.ch[leader], leader, lag).unwrap(), leader))
}

pub open spec fn inv(c: LC) -> bool {
    exists|g0: G, leader: int| #[trigger] inv_with(c, g0, leader)
}

proof fn lemma_follow_wf(g: G, w: Seq<int>, from: int, to: int)
    requires follow(g, w, from, to) is Some,
    ensures wf2(follow(g, w, from, to).unwrap()),
    decreases w.len(),
{
    if w.len() > 0 {
        let h = head(g);
        lemma_head_wf(g);
        let cont = bs_lookup(*h->Comm_bs, w[0]).unwrap();
        lemma_comm_branches_wf(h);
        lemma_branch_wf(*h->Comm_bs, w[0]);
        lemma_follow_wf(cont, w.drop_first(), from, to);
    }
}

/// Following one more label after a communication at the end.
proof fn lemma_follow_push(g: G, w: Seq<int>, from: int, to: int, label: int)
    requires
        follow(g, w, from, to) is Some,
        head(follow(g, w, from, to).unwrap()) is Comm,
        head(follow(g, w, from, to).unwrap())->Comm_from == from,
        head(follow(g, w, from, to).unwrap())->Comm_to == to,
        bs_lookup(*head(follow(g, w, from, to).unwrap())->Comm_bs, label) is Some,
    ensures
        follow(g, w.push(label), from, to) == Some(bs_lookup(*head(follow(g, w, from, to).unwrap())->Comm_bs, label).unwrap()),
    decreases w.len(),
{
    lemma_follow_wf(g, w, from, to);
    let gk = follow(g, w, from, to).unwrap();
    lemma_head_wf(gk);
    lemma_comm_branches_wf(head(gk));
    lemma_branch_wf(*head(gk)->Comm_bs, label);
    if w.len() == 0 {
        assert(gk == g);
        assert(w.push(label)[0] == label);
        assert(w.push(label).drop_first() =~= Seq::<int>::empty());
        assert(w.push(label).len() == 1);
        let cont = bs_lookup(*head(g)->Comm_bs, label).unwrap();
        assert(follow(cont, Seq::<int>::empty(), from, to) == Some(cont));
    } else {
        let h = head(g);
        let cont = bs_lookup(*h->Comm_bs, w[0]).unwrap();
        assert(w.push(label)[0] == w[0]);
        assert(w.push(label).drop_first() =~= w.drop_first().push(label));
        lemma_follow_push(cont, w.drop_first(), from, to, label);
    }
}

/// The head of a role's position under the invariant, by the head of the
/// global type there.
proof fn lemma_position(g0: G, r: int)
    requires wf2(g0),
    ensures
        lhead(proj(g0, r)) == proj(head(g0), r),
        head(g0) is End ==> lhead(proj(g0, r)) is End,
        head(g0) is Comm && head(g0)->Comm_from == r ==> lhead(proj(g0, r)) is Send && lhead(proj(g0, r))->Send_to == head(g0)->Comm_to && *lhead(proj(g0, r))->Send_ls == proj_bs(*head(g0)->Comm_bs, r),
        head(g0) is Comm && head(g0)->Comm_to == r ==> lhead(proj(g0, r)) is Recv && lhead(proj(g0, r))->Recv_from == head(g0)->Comm_from && *lhead(proj(g0, r))->Recv_ls == proj_bs(*head(g0)->Comm_bs, r),
{
    lemma_lhead_proj(g0, r);
    lemma_head_wf(g0);
}

pub proof fn lemma_init(g: G)
    requires wf2(g),
    ensures inv(linit(g)),
{
    assert(follow(g, Seq::<int>::empty(), 0, 1) == Some(g));
    assert(inv_with(linit(g), g, 0));
}

pub proof fn lemma_step_inv(c: LC, c2: LC)
    requires inv(c), lstep(c, c2),
    ensures inv(c2),
{
    let (g0, leader) = choose|g0: G, leader: int| #[trigger] inv_with(c, g0, leader);
    let lag = other(leader);
    let w = c.ch[leader];
    let gk = follow(g0, w, leader, lag).unwrap();
    lemma_follow_wf(g0, w, leader, lag);
    lemma_head_wf(g0);
    lemma_head_wf(gk);
    lemma_position(g0, 0);
    lemma_position(g0, 1);
    lemma_position(gk, 0);
    lemma_position(gk, 1);
    if w.len() == 0 {
        assert(gk == g0);
    }
    let (r, label) = choose|r: int, label: int|
        (r == 0 || r == 1) && (
            (lhead(c.l[r]) is Send && lhead(c.l[r])->Send_to == other(r)
                && ls_lookup(*lhead(c.l[r])->Send_ls, label) is Some
                && c2 == lsend(c, r, label, ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap()))
            || (recv_enabled(c, r) && label == c.ch[other(r)][0]
                && c2 == lrecv(c, r, ls_lookup(*lhead(c.l[r])->Recv_ls, label).unwrap()))
        );
    if lhead(c.l[r]) is Send && lhead(c.l[r])->Send_to == other(r) && ls_lookup(*lhead(c.l[r])->Send_ls, label) is Some
        && c2 == lsend(c, r, label, ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap()) {
        let cont = ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap();
        if r == leader {
            // The leader goes one message further ahead.
            let h = head(gk);
            assert(h is Comm && h->Comm_from == leader && h->Comm_to == lag);
            lemma_lookup_proj(*h->Comm_bs, leader, label);
            let cont_g = bs_lookup(*h->Comm_bs, label).unwrap();
            assert(cont == proj(cont_g, leader));
            lemma_follow_push(g0, w, leader, lag, label);
            lemma_comm_branches_wf(h);
            lemma_branch_wf(*h->Comm_bs, label);
            assert(c2.ch[leader] == w.push(label));
            assert(c2.ch[lag] == c.ch[lag]);
            assert(c2.l[lag] == c.l[lag]);
            assert(c2.l[leader] == cont);
            assert(inv_with(c2, g0, leader));
        } else {
            // The lagger sends: nothing was in transit, so the roles swap.
            let h = head(g0);
            assert(h is Comm && h->Comm_from == lag && h->Comm_to == leader);
            if w.len() > 0 {
                assert(head(g0)->Comm_from == leader);
                assert(false);
            }
            assert(gk == g0);
            lemma_lookup_proj(*h->Comm_bs, lag, label);
            let cont_g = bs_lookup(*h->Comm_bs, label).unwrap();
            lemma_comm_branches_wf(h);
            lemma_branch_wf(*h->Comm_bs, label);
            assert(c2.ch[lag] == Seq::<int>::empty().push(label));
            assert(c2.ch[leader] == w);
            assert(follow(g0, Seq::<int>::empty(), lag, leader) == Some(g0));
            lemma_follow_push(g0, Seq::<int>::empty(), lag, leader, label);
            assert(Seq::<int>::empty().push(label) =~= c2.ch[lag]);
            assert(follow(g0, c2.ch[lag], lag, leader) == Some(cont_g));
            assert(c2.l[lag] == cont);
            assert(c2.l[leader] == c.l[leader]);
            assert(inv_with(c2, g0, lag));
        }
    } else {
        // A receive: only the lagger has anything to receive.
        assert(recv_enabled(c, r));
        if r == leader {
            assert(c.ch[lag].len() == 0);
            assert(false);
        }
        assert(r == lag);
        let h = head(g0);
        assert(w.len() > 0);
        assert(h is Comm && h->Comm_from == leader && h->Comm_to == lag);
        assert(label == w[0]);
        lemma_lookup_proj(*h->Comm_bs, lag, label);
        let cont_g = bs_lookup(*h->Comm_bs, label).unwrap();
        let cont = ls_lookup(*lhead(c.l[lag])->Recv_ls, label).unwrap();
        assert(cont == proj(cont_g, lag));
        lemma_comm_branches_wf(h);
        lemma_branch_wf(*h->Comm_bs, label);
        assert(follow(cont_g, w.drop_first(), leader, lag) == Some(gk));
        assert(c2.ch[leader] == w.drop_first());
        assert(c2.ch[lag] == c.ch[lag]);
        assert(c2.l[lag] == cont);
        assert(c2.l[leader] == c.l[leader]);
        assert(inv_with(c2, cont_g, leader));
    }
}

pub proof fn lemma_no_deadlock(c: LC)
    requires inv(c),
    ensures !ldeadlocked(c),
{
    let (g0, leader) = choose|g0: G, leader: int| #[trigger] inv_with(c, g0, leader);
    let lag = other(leader);
    let w = c.ch[leader];
    let gk = follow(g0, w, leader, lag).unwrap();
    lemma_follow_wf(g0, w, leader, lag);
    lemma_head_wf(g0);
    lemma_head_wf(gk);
    lemma_position(g0, 0);
    lemma_position(g0, 1);
    lemma_position(gk, 0);
    lemma_position(gk, 1);
    if w.len() > 0 {
        // The lagger can receive the oldest label in transit.
        let h = head(g0);
        assert(h is Comm && h->Comm_from == leader && h->Comm_to == lag);
        lemma_lookup_proj(*h->Comm_bs, lag, w[0]);
        assert(recv_enabled(c, lag));
        assert(!lblocked(c, lag) && !lterminal(c, lag));
    } else {
        // Both roles stand at `g0`: finished, or its sender can send.
        assert(gk == g0);
        let h = head(g0);
        if h is End {
            assert(!lblocked(c, 0) && !lblocked(c, 1));
        } else {
            let f = h->Comm_from;
            assert(f == 0 || f == 1);
            if f == leader {
                assert(lhead(c.l[f]) == lhead(proj(g0, f)));
            } else {
                assert(f == lag);
                assert(lhead(c.l[f]) == lhead(proj(g0, f)));
            }
            assert(lhead(c.l[f]) is Send);
            assert(!lblocked(c, f) && !lterminal(c, f));
        }
    }
}

proof fn lemma_run_inv(g: G, path: Seq<LC>)
    requires wf2(g), lrun(g, path),
    ensures inv(path.last()),
    decreases path.len(),
{
    if path.len() == 1 {
        lemma_init(g);
    } else {
        let prefix = path.drop_last();
        assert(lrun(g, prefix));
        lemma_run_inv(g, prefix);
        assert(lstep(prefix.last(), path.last()));
        lemma_step_inv(prefix.last(), path.last());
    }
}

/// No unexpected reception (2026-11-09): a role whose head is a receive
/// and whose channel from the other role is not empty accepts the oldest
/// label in it.
pub open spec fn lreception_ok(c: LC) -> bool {
    forall|r: int| (r == 0 || r == 1) && lhead(c.l[r]) is Recv && c.ch[other(r)].len() > 0
        ==> ls_lookup(*lhead(c.l[r])->Recv_ls, c.ch[other(r)][0]) is Some
}

pub proof fn lemma_reception(c: LC)
    requires inv(c),
    ensures lreception_ok(c),
{
    let (g0, leader) = choose|g0: G, leader: int| #[trigger] inv_with(c, g0, leader);
    let lag = other(leader);
    let w = c.ch[leader];
    lemma_follow_wf(g0, w, leader, lag);
    lemma_head_wf(g0);
    lemma_position(g0, lag);
    assert forall|r: int| (r == 0 || r == 1) && lhead(c.l[r]) is Recv && c.ch[other(r)].len() > 0
        implies ls_lookup(*lhead(c.l[r])->Recv_ls, c.ch[other(r)][0]) is Some by {
        // Only the lagger's channel from the leader can be non-empty.
        assert(r == lag);
        let h = head(g0);
        assert(h is Comm && h->Comm_from == leader && h->Comm_to == lag);
        lemma_lookup_proj(*h->Comm_bs, lag, w[0]);
    }
}

/// The two-role reception theorem: in every reachable configuration, the
/// oldest label of a channel a role awaits on is one it accepts.
pub proof fn theorem_two_role_reception(g: G)
    requires wf2(g),
    ensures forall|c: LC| lreachable(g, c) ==> lreception_ok(c),
{
    assert forall|c: LC| lreachable(g, c) implies lreception_ok(c) by {
        let path = choose|path: Seq<LC>| #[trigger] lrun(g, path) && path.last() == c;
        lemma_run_inv(g, path);
        lemma_reception(c);
    }
}

/// The general theorem for the two-role fragment: every Protocol the
/// structural acceptance conditions admit has progress in every run.
pub proof fn theorem_two_role_progress(g: G)
    requires wf2(g),
    ensures forall|c: LC| lreachable(g, c) ==> !ldeadlocked(c),
{
    assert forall|c: LC| lreachable(g, c) implies !ldeadlocked(c) by {
        let path = choose|path: Seq<LC>| #[trigger] lrun(g, path) && path.last() == c;
        lemma_run_inv(g, path);
        lemma_no_deadlock(c);
    }
}

// ------------------------------------- the executable acceptance check --

/// The executable global type the checker builds from the parsed Protocol
/// (roles and labels numbered, recursion in de Bruijn form); its view is
/// the specification's `G`.
pub enum GE {
    End,
    Comm { from: usize, to: usize, bs: Box<BE> },
    Rec { body: Box<GE> },
    Var { n: usize },
}

pub enum BE {
    Nil,
    Cons { label: u32, g: GE, rest: Box<BE> },
}

impl GE {
    pub open spec fn view(&self) -> G
        decreases self,
    {
        match self {
            GE::End => G::End,
            GE::Comm { from, to, bs } => G::Comm { from: *from as int, to: *to as int, bs: Box::new(bs.view()) },
            GE::Rec { body } => G::Rec { body: Box::new(body.view()) },
            GE::Var { n } => G::Var { n: *n as nat },
        }
    }
}

impl BE {
    pub open spec fn view(&self) -> Bs
        decreases self,
    {
        match self {
            BE::Nil => Bs::Nil,
            BE::Cons { label, g, rest } => Bs::Cons { label: *label as int, g: g.view(), rest: Box::new(rest.view()) },
        }
    }
}

fn be_has_label(bs: &BE, label: u32) -> (r: bool)
    ensures r == bs_has_label(bs.view(), label as int),
    decreases bs,
{
    match bs {
        BE::Nil => false,
        BE::Cons { label: l, rest, .. } => *l == label || be_has_label(rest, label),
    }
}

/// The branch count, saturating at `usize::MAX`.
fn be_len(bs: &BE) -> (r: usize)
    ensures r == bs_len(bs.view()) || (r == usize::MAX && bs_len(bs.view()) >= usize::MAX),
    decreases bs,
{
    match bs {
        BE::Nil => 0,
        BE::Cons { rest, .. } => {
            let n = be_len(rest);
            if n == usize::MAX { usize::MAX } else { n + 1 }
        }
    }
}

fn be_distinct(bs: &BE) -> (r: bool)
    ensures r == bs_distinct(bs.view()),
    decreases bs,
{
    match bs {
        BE::Nil => true,
        BE::Cons { label, rest, .. } => !be_has_label(rest, *label) && be_distinct(rest),
    }
}

/// The checker's structural acceptance of a two-role Protocol, decided
/// under the guard environment `env` (one flag per enclosing recursion,
/// outermost first): `true` exactly when `wf2_at` holds of the view.
pub fn wf2_exec(g: &GE, env: &mut Vec<bool>) -> (r: bool)
    ensures r == wf2_at(g.view(), old(env)@), final(env)@ == old(env)@,
    decreases g,
{
    match g {
        GE::End => true,
        GE::Var { n } => *n < env.len() && env[env.len() - 1 - *n],
        GE::Rec { body } => {
            env.push(false);
            let r = wf2_exec(body, env);
            env.pop();
            assert(env@ =~= old(env)@);
            r
        }
        GE::Comm { from, to, bs } => {
            let ok = (*from == 0 || *from == 1) && (*to == 0 || *to == 1) && *from != *to && be_len(bs) >= 1 && be_distinct(bs);
            if !ok {
                return false;
            }
            let mut guarded: Vec<bool> = Vec::new();
            let mut i = 0;
            while i < env.len()
                invariant i <= env.len(), guarded@ == Seq::new(i as nat, |j: int| true),
                decreases env.len() - i,
            {
                guarded.push(true);
                i += 1;
                assert(guarded@ =~= Seq::new(i as nat, |j: int| true));
            }
            assert(guarded@ =~= all_true(env@));
            wf2_bs_exec(bs, &mut guarded)
        }
    }
}

fn wf2_bs_exec(bs: &BE, env: &mut Vec<bool>) -> (r: bool)
    ensures r == wf2_bs(bs.view(), old(env)@), final(env)@ == old(env)@,
    decreases bs,
{
    match bs {
        BE::Nil => true,
        BE::Cons { g, rest, .. } => {
            let a = wf2_exec(g, env);
            let b = wf2_bs_exec(rest, env);
            a && b
        }
    }
}

/// `check_protocol`'s hook (two roles): accepting establishes the
/// hypothesis of `theorem_two_role_progress` for the Protocol's view.
pub fn accept_two_role(g: &GE) -> (r: bool)
    ensures r ==> wf2(g.view()),
        r ==> forall|c: LC| lreachable(g.view(), c) ==> !ldeadlocked(c),
{
    let mut env: Vec<bool> = Vec::new();
    let ok = wf2_exec(g, &mut env);
    proof {
        assert(env@ =~= Seq::<bool>::empty());
        if ok {
            theorem_two_role_progress(g.view());
        }
    }
    ok
}

// ------------------------------------------- the bridge to the automaton --

/// The branch list of a head-normal local type (`Nil` for `End`).
pub open spec fn ls_of(l: L) -> Ls {
    match l {
        L::Send { ls, .. } => *ls,
        L::Recv { ls, .. } => *ls,
        _ => Ls::Nil,
    }
}

/// A table entry: head normal, between the two roles, with at least one
/// branch when it communicates.
pub open spec fn entry_ok(l: L, r: int) -> bool {
    match l {
        L::End => true,
        L::Send { to, ls } => to == other(r) && *ls is Cons,
        L::Recv { from, ls } => from == other(r) && *ls is Cons,
        _ => false,
    }
}

pub open spec fn has_transition(s: System, t: Trans) -> bool {
    exists|i: int| 0 <= i < s.trans@.len() && s.trans@[i] == t
}

/// The automaton `s` is the flattening of the two-role Protocol `g`
/// through the state tables `tb` (one per role): entry 0 is the head of
/// the projection, every entry is head normal, and the transitions are
/// exactly the entries' branches, each leading to the entry equal to the
/// head of its continuation.
pub open spec fn flattens(s: System, g: G, tb: Seq<Seq<L>>) -> bool {
    &&& s.roles == 2
    &&& tb.len() == 2
    &&& forall|r: int| 0 <= r < 2 ==> (#[trigger] tb[r]).len() >= 1 && tb[r][0] == lhead(proj(g, r))
    &&& forall|r: int, i: int| 0 <= r < 2 && 0 <= i < tb[r].len() ==> entry_ok(#[trigger] tb[r][i], r)
    &&& forall|r: int, i: int, label: int| 0 <= r < 2 && 0 <= i < tb[r].len() && #[trigger] ls_lookup(ls_of(tb[r][i]), label) is Some
        ==> exists|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(ls_lookup(ls_of(tb[r][i]), label).unwrap())
            && has_transition(s, Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: other(r) as usize, label: label as u32, to: j as usize })
    &&& forall|k: int| 0 <= k < s.trans@.len() ==> {
        let t = #[trigger] s.trans@[k];
        &&& t.role < 2
        &&& (t.from as int) < tb[t.role as int].len()
        &&& (t.to as int) < tb[t.role as int].len()
        &&& t.peer == other(t.role as int)
        &&& (t.send ==> tb[t.role as int][t.from as int] is Send)
        &&& (!t.send ==> tb[t.role as int][t.from as int] is Recv)
        &&& ls_lookup(ls_of(tb[t.role as int][t.from as int]), t.label as int) is Some
        &&& tb[t.role as int][t.to as int] == lhead(ls_lookup(ls_of(tb[t.role as int][t.from as int]), t.label as int).unwrap())
    }
}

/// The local configuration an automaton configuration stands for.
pub open spec fn corresponds(tb: Seq<Seq<L>>, c: Cfg, lc: LC) -> bool {
    &&& lc_wf(lc)
    &&& wf_cfg(2, c)
    &&& (c.states[0] as int) < tb[0].len()
    &&& (c.states[1] as int) < tb[1].len()
    &&& lhead(lc.l[0]) == tb[0][c.states[0] as int]
    &&& lhead(lc.l[1]) == tb[1][c.states[1] as int]
    &&& lc.ch[0] == c.chans[1].map(|i: int, x: u32| x as int)
    &&& lc.ch[1] == c.chans[2].map(|i: int, x: u32| x as int)
}

proof fn lemma_entry_head(l: L)
    requires !(l is Rec),
    ensures lhead(l) == l,
{
}

proof fn lemma_bridge_step(s: System, g: G, tb: Seq<Seq<L>>, c: Cfg, c2: Cfg, lc: LC)
    requires flattens(s, g, tb), corresponds(tb, c, lc), step(s, c, c2),
    ensures exists|lc2: LC| lstep(lc, lc2) && corresponds(tb, c2, lc2),
{
    let k = choose|k: int| 0 <= k < s.trans@.len() && enabled(2, c, #[trigger] s.trans@[k]) && c2 == succ(2, c, s.trans@[k]);
    let t = s.trans@[k];
    let r = t.role as int;
    let i = t.from as int;
    let e = tb[r][i];
    let cont = ls_lookup(ls_of(e), t.label as int).unwrap();
    lemma_entry_head(e);
    assert(lhead(lc.l[r]) == e);
    assert(c.states[r] == t.from);
    let ch = chan_of(2, t);
    if t.send {
        assert(ch == r * 2 + other(r));
        let lc2 = lsend(lc, r, t.label as int, cont);
        assert(lhead(lc.l[r]) is Send && lhead(lc.l[r])->Send_to == other(r));
        assert(ls_lookup(*lhead(lc.l[r])->Send_ls, t.label as int) == Some(cont));
        assert(lstep(lc, lc2));
        // The correspondence after the step.
        assert(c2.states == c.states.update(r, t.to));
        assert(lc2.l[r] == cont);
        assert(lhead(cont) == tb[r][t.to as int]);
        assert(c2.chans == c.chans.update(ch, c.chans[ch].push(t.label)));
        if r == 0 {
            assert(ch == 1);
            assert(lc2.ch[0] =~= c2.chans[1].map(|i: int, x: u32| x as int));
            assert(lc2.ch[1] == c2.chans[2].map(|i: int, x: u32| x as int));
        } else {
            assert(ch == 2);
            assert(lc2.ch[1] =~= c2.chans[2].map(|i: int, x: u32| x as int));
            assert(lc2.ch[0] == c2.chans[1].map(|i: int, x: u32| x as int));
        }
        assert(corresponds(tb, c2, lc2));
    } else {
        assert(ch == other(r) * 2 + r);
        let lc2 = lrecv(lc, r, cont);
        assert(lhead(lc.l[r]) is Recv && lhead(lc.l[r])->Recv_from == other(r));
        assert(c.chans[ch].len() > 0 && c.chans[ch][0] == t.label);
        assert(lc.ch[other(r)][0] == t.label as int);
        assert(recv_enabled(lc, r));
        assert(lstep(lc, lc2));
        assert(lhead(cont) == tb[r][t.to as int]);
        if r == 0 {
            assert(ch == 2);
            assert(lc2.ch[1] =~= c2.chans[2].map(|i: int, x: u32| x as int));
            assert(lc2.ch[0] == c2.chans[1].map(|i: int, x: u32| x as int));
        } else {
            assert(ch == 1);
            assert(lc2.ch[0] =~= c2.chans[1].map(|i: int, x: u32| x as int));
            assert(lc2.ch[1] == c2.chans[2].map(|i: int, x: u32| x as int));
        }
        assert(corresponds(tb, c2, lc2));
    }
}

proof fn lemma_bridge_init(s: System, g: G, tb: Seq<Seq<L>>)
    requires flattens(s, g, tb),
    ensures corresponds(tb, initial(2), linit(g)),
{
    assert(initial(2).chans[1] =~= Seq::<u32>::empty());
    assert(initial(2).chans[2] =~= Seq::<u32>::empty());
    assert(linit(g).ch[0] =~= initial(2).chans[1].map(|i: int, x: u32| x as int));
    assert(linit(g).ch[1] =~= initial(2).chans[2].map(|i: int, x: u32| x as int));
}

/// Every automaton run has a local run beside it.
proof fn lemma_bridge_run(s: System, g: G, tb: Seq<Seq<L>>, path: Seq<Cfg>)
    requires flattens(s, g, tb), run(s, path),
    ensures exists|lc: LC| lreachable(g, lc) && corresponds(tb, path.last(), lc),
    decreases path.len(),
{
    if path.len() == 1 {
        lemma_bridge_init(s, g, tb);
        assert(lrun(g, seq![linit(g)]));
        assert(lreachable(g, linit(g)));
    } else {
        let prefix = path.drop_last();
        assert(run(s, prefix));
        lemma_bridge_run(s, g, tb, prefix);
        let lc = choose|lc: LC| lreachable(g, lc) && corresponds(tb, prefix.last(), lc);
        let lpath = choose|lpath: Seq<LC>| #[trigger] lrun(g, lpath) && lpath.last() == lc;
        assert(step(s, prefix.last(), path.last()));
        lemma_bridge_step(s, g, tb, prefix.last(), path.last(), lc);
        let lc2 = choose|lc2: LC| lstep(lc, lc2) && corresponds(tb, path.last(), lc2);
        let lpath2 = lpath.push(lc2);
        assert(lrun(g, lpath2));
        assert(lreachable(g, lc2));
    }
}

/// A waiting set of the automaton is a waiting set of the local types.
#[verifier::rlimit(60)]
proof fn lemma_bridge_deadlock(s: System, g: G, tb: Seq<Seq<L>>, c: Cfg, lc: LC)
    requires flattens(s, g, tb), corresponds(tb, c, lc), deadlocked(s, c),
    ensures ldeadlocked(lc),
{
    let set = choose|set: Seq<bool>| deadlock_set(s, c, set);
    let r0 = choose|r: int| 0 <= r < set.len() && set[r] && blocked(s, c, r);
    // Blocked and terminal roles of the automaton are those of the local types.
    assert forall|r: int| 0 <= r < 2 && blocked(s, c, r) implies lblocked(lc, r) && set[other(r)] || !set[r] by {
        if blocked(s, c, r) {
            let e = tb[r][c.states[r] as int];
            lemma_entry_head(e);
            let k = choose|k: int| 0 <= k < s.trans@.len() && (#[trigger] s.trans@[k]).role == r && at(c, s.trans@[k]);
            let t = s.trans@[k];
            assert(!t.send);
            assert(e is Recv);
            if recv_enabled(lc, r) {
                // The receive at the oldest label has a transition, enabled.
                let label = lc.ch[other(r)][0];
                let cont = ls_lookup(ls_of(e), label).unwrap();
                let j = choose|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(cont)
                    && has_transition(s, Trans { role: r as usize, from: c.states[r] as usize, send: false, peer: other(r) as usize, label: label as u32, to: j as usize });
                let t2 = Trans { role: r as usize, from: c.states[r] as usize, send: false, peer: other(r) as usize, label: label as u32, to: j as usize };
                let k2 = choose|k2: int| 0 <= k2 < s.trans@.len() && s.trans@[k2] == t2;
                let ch = chan_of(2, t2);
                assert(ch == other(r) * 2 + r);
                assert(c.chans[ch][0] == label as u32);
                assert(enabled(2, c, t2));
                assert(false);
            }
            assert(lblocked(lc, r));
            if set[r] {
                assert(set[t.peer as int]);
            }
        }
    }
    assert forall|r: int| 0 <= r < 2 && terminal(s, c, r) implies lterminal(lc, r) by {
        if terminal(s, c, r) {
            let e = tb[r][c.states[r] as int];
            lemma_entry_head(e);
            if !(e is End) {
                let ls = ls_of(e);
                let label = ls->Cons_label;
                assert(ls_lookup(ls, label) is Some);
                let j = choose|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(ls_lookup(ls_of(e), label).unwrap())
                    && has_transition(s, Trans { role: r as usize, from: c.states[r] as usize, send: e is Send, peer: other(r) as usize, label: label as u32, to: j as usize });
                let t2 = Trans { role: r as usize, from: c.states[r] as usize, send: e is Send, peer: other(r) as usize, label: label as u32, to: j as usize };
                let k2 = choose|k2: int| 0 <= k2 < s.trans@.len() && s.trans@[k2] == t2;
                assert(at(c, t2));
                assert(has_trans(s, c, r));
                assert(false);
            }
        }
    }
    assert(set[r0] && blocked(s, c, r0));
    assert(set[other(r0)]);
    assert(blocked(s, c, other(r0)) || terminal(s, c, other(r0)));
}

/// The two-role theorem for the flattened automaton: what `compose.rs`
/// takes per Endpoint, with no certificate.
pub proof fn theorem_two_role_automaton(s: System, g: G, tb: Seq<Seq<L>>)
    requires wf2(g), flattens(s, g, tb),
    ensures forall|c: Cfg| reachable(s, c) ==> !deadlocked(s, c),
{
    assert forall|c: Cfg| reachable(s, c) implies !deadlocked(s, c) by {
        let path = choose|path: Seq<Cfg>| #[trigger] run(s, path) && path.last() == c;
        lemma_bridge_run(s, g, tb, path);
        let lc = choose|lc: LC| lreachable(g, lc) && corresponds(tb, c, lc);
        theorem_two_role_progress(g);
        if deadlocked(s, c) {
            lemma_bridge_deadlock(s, g, tb, c, lc);
        }
    }
}

// ------------------------------ the conversion from the parsed Protocol --

/// The parser's types, made visible to the specifications (their fields
/// are public; nothing is assumed about them).
#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExGlobal(crate::ast::Global);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExMessage(crate::ast::Message);

/// The first position of `name` in `names`, or -1.
pub open spec fn index_of(names: Seq<Seq<char>>, name: Seq<char>) -> int
    decreases names.len(),
{
    if names.len() == 0 {
        -1
    } else if names[0] == name {
        0
    } else {
        let r = index_of(names.drop_first(), name);
        if r < 0 { -1 } else { r + 1 }
    }
}

/// The last position of `name` in `names`, or -1.
pub open spec fn last_index_of(names: Seq<Seq<char>>, name: Seq<char>) -> int
    decreases names.len(),
{
    if names.len() == 0 {
        -1
    } else if names.last() == name {
        names.len() - 1
    } else {
        last_index_of(names.drop_last(), name)
    }
}

/// A role's number: its position among the Endpoint's roles, or
/// `usize::MAX` for a name that is not one (which `wf2` then refuses).
pub open spec fn role_of(roles: Seq<Seq<char>>, name: Seq<char>) -> int {
    let i = index_of(roles, name);
    if i < 0 { usize::MAX as int } else { i }
}

/// A label's number: its position in the collected labels (every label
/// of the Protocol is collected, `labels_complete`, so the default is
/// never taken).
pub open spec fn label_of(labels: Seq<Seq<char>>, name: Seq<char>) -> int {
    let i = index_of(labels, name);
    if i < 0 { 0 } else { i }
}

/// A recursion variable in de Bruijn form: the innermost binder of its
/// name is 0; an unbound name (which well-formedness refuses) is out of
/// every range.
pub open spec fn rec_of(recs: Seq<Seq<char>>, name: Seq<char>) -> int {
    let i = last_index_of(recs, name);
    if i < 0 { usize::MAX as int - 1 } else { recs.len() - 1 - i }
}

/// The receiver of a choice: the receiver of its first message branch.
pub open spec fn choice_to(bs: Seq<crate::ast::Global>, roles: Seq<Seq<char>>) -> int
    decreases bs.len(),
{
    if bs.len() == 0 {
        usize::MAX as int
    } else {
        match bs[0] {
            crate::ast::Global::Msg(m, _) => role_of(roles, m.to@),
            _ => choice_to(bs.drop_first(), roles),
        }
    }
}

/// The theory's view of a parsed global type.
pub open spec fn gconv(g: crate::ast::Global, roles: Seq<Seq<char>>, labels: Seq<Seq<char>>, recs: Seq<Seq<char>>) -> G
    decreases g, 0int,
{
    match g {
        crate::ast::Global::End => G::End,
        crate::ast::Global::Continue(name) => G::Var { n: rec_of(recs, name@) as nat },
        crate::ast::Global::Msg(m, rest) => G::Comm {
            from: role_of(roles, m.from@),
            to: role_of(roles, m.to@),
            bs: Box::new(Bs::Cons { label: label_of(labels, m.label@), g: gconv(*rest, roles, labels, recs), rest: Box::new(Bs::Nil) }),
        },
        crate::ast::Global::Choice { at, branches } => G::Comm {
            from: role_of(roles, at@),
            to: choice_to(branches@, roles),
            bs: Box::new(bconv(branches@, 0, roles, labels, recs)),
        },
        crate::ast::Global::Rec { name, body } => G::Rec { body: Box::new(gconv(*body, roles, labels, recs.push(name@))) },
    }
}

/// The branches of a choice from index `i` on, each a message branch.
pub open spec fn bconv(bs: Seq<crate::ast::Global>, i: int, roles: Seq<Seq<char>>, labels: Seq<Seq<char>>, recs: Seq<Seq<char>>) -> Bs
    decreases bs, bs.len() - i,
{
    if i < 0 || i >= bs.len() {
        Bs::Nil
    } else {
        match bs[i] {
            crate::ast::Global::Msg(m, rest) => Bs::Cons {
                label: label_of(labels, m.label@),
                g: gconv(*rest, roles, labels, recs),
                rest: Box::new(bconv(bs, i + 1, roles, labels, recs)),
            },
            _ => bconv(bs, i + 1, roles, labels, recs),
        }
    }
}

/// `name` is a label of the Protocol.
pub open spec fn has_label_in(g: crate::ast::Global, name: Seq<char>) -> bool
    decreases g, 0int,
{
    match g {
        crate::ast::Global::Msg(m, rest) => m.label@ == name || has_label_in(*rest, name),
        crate::ast::Global::Choice { branches, .. } => has_label_in_bs(branches@, 0, name),
        crate::ast::Global::Rec { body, .. } => has_label_in(*body, name),
        _ => false,
    }
}

pub open spec fn has_label_in_bs(bs: Seq<crate::ast::Global>, i: int, name: Seq<char>) -> bool
    decreases bs, bs.len() - i,
{
    if i < 0 || i >= bs.len() {
        false
    } else {
        has_label_in(bs[i], name) || has_label_in_bs(bs, i + 1, name)
    }
}

pub open spec fn names_view(v: Seq<String>) -> Seq<Seq<char>> {
    v.map(|i: int, s: String| s@)
}

pub open spec fn strs_view(v: Seq<&str>) -> Seq<Seq<char>> {
    v.map(|i: int, s: &str| s@)
}

pub open spec fn distinct(names: Seq<Seq<char>>) -> bool {
    forall|i: int, j: int| 0 <= i < names.len() && 0 <= j < names.len() && i != j ==> names[i] != names[j]
}

/// Adds `name` if absent: the list stays distinct and gains the name.
fn push_name(out: &mut Vec<String>, name: &String)
    requires distinct(names_view(old(out)@)),
    ensures
        distinct(names_view(final(out)@)),
        index_of(names_view(final(out)@), name@) >= 0,
        forall|n: Seq<char>| index_of(names_view(old(out)@), n) >= 0 ==> index_of(names_view(final(out)@), n) >= 0,
{
    let mut i = 0;
    while i < out.len()
        invariant
            i <= out.len(),
            out@ == old(out)@,
            distinct(names_view(out@)),
            forall|j: int| 0 <= j < i ==> out@[j]@ != name@,
        decreases out.len() - i,
    {
        if out[i] == *name {
            proof { lemma_index_of_found(names_view(out@), name@, i as int); }
            return;
        }
        i += 1;
    }
    let ghost before = names_view(out@);
    out.push(name.clone());
    proof {
        assert(names_view(out@) =~= before.push(name@));
        lemma_push_distinct(before, name@);
        lemma_index_of_found(names_view(out@), name@, before.len() as int);
        assert forall|n: Seq<char>| index_of(before, n) >= 0 implies index_of(names_view(out@), n) >= 0 by {
            lemma_index_of_push(before, name@, n);
        }
    }
}

proof fn lemma_index_of_found(names: Seq<Seq<char>>, name: Seq<char>, i: int)
    requires 0 <= i < names.len(), names[i] == name,
    ensures index_of(names, name) >= 0,
    decreases names.len(),
{
    if names[0] != name {
        lemma_index_of_found(names.drop_first(), name, i - 1);
    }
}

proof fn lemma_index_of_bounds(names: Seq<Seq<char>>, name: Seq<char>)
    ensures
        index_of(names, name) >= 0 ==> index_of(names, name) < names.len() && names[index_of(names, name)] == name,
        index_of(names, name) < 0 ==> forall|i: int| 0 <= i < names.len() ==> names[i] != name,
    decreases names.len(),
{
    if names.len() > 0 && names[0] != name {
        lemma_index_of_bounds(names.drop_first(), name);
        let r = index_of(names.drop_first(), name);
        if r < 0 {
            assert forall|i: int| 0 <= i < names.len() implies names[i] != name by {
                if i > 0 {
                    assert(names.drop_first()[i - 1] == names[i]);
                }
            }
        } else {
            assert(names[r + 1] == names.drop_first()[r]);
        }
    }
}

proof fn lemma_index_of_push(names: Seq<Seq<char>>, added: Seq<char>, name: Seq<char>)
    requires index_of(names, name) >= 0,
    ensures index_of(names.push(added), name) >= 0,
{
    lemma_index_of_bounds(names, name);
    let i = index_of(names, name);
    assert(names.push(added)[i] == name);
    lemma_index_of_found(names.push(added), name, i);
}

proof fn lemma_push_distinct(names: Seq<Seq<char>>, name: Seq<char>)
    requires distinct(names), forall|j: int| 0 <= j < names.len() ==> names[j] != name,
    ensures distinct(names.push(name)),
{
}

/// Collects every label of the Protocol, each once (so the numbering by
/// position is injective: `distinct`), in order of first appearance.
fn collect_labels(g: &crate::ast::Global, out: &mut Vec<String>)
    requires distinct(names_view(old(out)@)),
    ensures
        distinct(names_view(final(out)@)),
        forall|n: Seq<char>| index_of(names_view(old(out)@), n) >= 0 ==> index_of(names_view(final(out)@), n) >= 0,
        forall|n: Seq<char>| has_label_in(*g, n) ==> index_of(names_view(final(out)@), n) >= 0,
    decreases g, 0int,
{
    match g {
        crate::ast::Global::End | crate::ast::Global::Continue(_) => {
            assert(forall|n: Seq<char>| !has_label_in(*g, n));
        }
        crate::ast::Global::Msg(m, rest) => {
            push_name(out, &m.label);
            let ghost mid = out@;
            collect_labels(rest, out);
            assert forall|n: Seq<char>| has_label_in(*g, n) implies index_of(names_view(out@), n) >= 0 by {
                assert(has_label_in(*g, n) == (m.label@ == n || has_label_in(**rest, n)));
                if m.label@ == n {
                    assert(index_of(names_view(mid), n) >= 0);
                }
            }
        }
        crate::ast::Global::Choice { branches, .. } => {
            collect_labels_bs(branches, 0, out);
            assert forall|n: Seq<char>| has_label_in(*g, n) implies index_of(names_view(out@), n) >= 0 by {
                assert(has_label_in(*g, n) == has_label_in_bs(branches@, 0, n));
            }
        }
        crate::ast::Global::Rec { body, .. } => {
            collect_labels(body, out);
            assert forall|n: Seq<char>| has_label_in(*g, n) implies index_of(names_view(out@), n) >= 0 by {
                assert(has_label_in(*g, n) == has_label_in(**body, n));
            }
        }
    }
}

fn collect_labels_bs(bs: &Vec<crate::ast::Global>, i: usize, out: &mut Vec<String>)
    requires distinct(names_view(old(out)@)), i <= bs.len(),
    ensures
        distinct(names_view(final(out)@)),
        forall|n: Seq<char>| index_of(names_view(old(out)@), n) >= 0 ==> index_of(names_view(final(out)@), n) >= 0,
        forall|n: Seq<char>| has_label_in_bs(bs@, i as int, n) ==> index_of(names_view(final(out)@), n) >= 0,
    decreases bs, bs.len() - i,
{
    if i < bs.len() {
        collect_labels(&bs[i], out);
        let ghost mid = out@;
        collect_labels_bs(bs, i + 1, out);
        assert forall|n: Seq<char>| has_label_in_bs(bs@, i as int, n) implies index_of(names_view(out@), n) >= 0 by {
            assert(has_label_in_bs(bs@, i as int, n) == (has_label_in(bs@[i as int], n) || has_label_in_bs(bs@, i as int + 1, n)));
            if has_label_in(bs@[i as int], n) {
                assert(index_of(names_view(mid), n) >= 0);
            }
        }
    } else {
        assert(forall|n: Seq<char>| !has_label_in_bs(bs@, i as int, n));
    }
}

fn role_index(roles: &[&str], name: &String) -> (r: usize)
    ensures r as int == role_of(strs_view(roles@), name@),
{
    let mut i = 0;
    while i < roles.len()
        invariant
            i <= roles.len(),
            forall|j: int| 0 <= j < i ==> roles@[j]@ != name@,
        decreases roles.len() - i,
    {
        if roles[i] == name.as_str() {
            proof { lemma_index_of_found(strs_view(roles@), name@, i as int); lemma_index_of_bounds(strs_view(roles@), name@); lemma_index_of_unique(strs_view(roles@), name@, i as int); }
            return i;
        }
        i += 1;
    }
    proof { lemma_index_of_bounds(strs_view(roles@), name@); }
    usize::MAX
}

/// The first position is the found one when nothing before it matches.
proof fn lemma_index_of_unique(names: Seq<Seq<char>>, name: Seq<char>, i: int)
    requires 0 <= i < names.len(), names[i] == name, forall|j: int| 0 <= j < i ==> names[j] != name,
    ensures index_of(names, name) == i,
    decreases names.len(),
{
    if i > 0 {
        lemma_index_of_unique(names.drop_first(), name, i - 1);
    }
}

fn label_index(labels: &Vec<String>, name: &String) -> (r: u32)
    requires labels.len() < u32::MAX as usize,
    ensures r as int == label_of(names_view(labels@), name@),
{
    let mut i = 0;
    while i < labels.len()
        invariant
            i <= labels.len(),
            labels.len() < u32::MAX as usize,
            forall|j: int| 0 <= j < i ==> labels@[j]@ != name@,
        decreases labels.len() - i,
    {
        if labels[i] == *name {
            proof { lemma_index_of_unique(names_view(labels@), name@, i as int); }
            return i as u32;
        }
        i += 1;
    }
    proof { lemma_index_of_bounds(names_view(labels@), name@); }
    0
}

fn rec_index(recs: &Vec<String>, name: &String) -> (r: usize)
    ensures r as int == rec_of(names_view(recs@), name@),
{
    let mut i = recs.len();
    while i > 0
        invariant
            i <= recs.len(),
            forall|j: int| i <= j < recs.len() ==> recs@[j]@ != name@,
        decreases i,
    {
        i -= 1;
        if recs[i] == *name {
            proof { lemma_last_index_of_unique(names_view(recs@), name@, i as int); }
            return recs.len() - 1 - i;
        }
    }
    proof { lemma_last_index_of_none(names_view(recs@), name@); }
    usize::MAX - 1
}

proof fn lemma_last_index_of_unique(names: Seq<Seq<char>>, name: Seq<char>, i: int)
    requires 0 <= i < names.len(), names[i] == name, forall|j: int| i < j < names.len() ==> names[j] != name,
    ensures last_index_of(names, name) == i,
    decreases names.len(),
{
    if i < names.len() - 1 {
        lemma_last_index_of_unique(names.drop_last(), name, i);
    }
}

proof fn lemma_last_index_of_none(names: Seq<Seq<char>>, name: Seq<char>)
    requires forall|j: int| 0 <= j < names.len() ==> names[j] != name,
    ensures last_index_of(names, name) < 0,
    decreases names.len(),
{
    if names.len() > 0 {
        lemma_last_index_of_none(names.drop_last(), name);
    }
}

fn convert(g: &crate::ast::Global, roles: &[&str], labels: &Vec<String>, recs: &mut Vec<String>) -> (r: GE)
    requires labels.len() < u32::MAX as usize,
    ensures r@ == gconv(*g, strs_view(roles@), names_view(labels@), names_view(old(recs)@)), final(recs)@ == old(recs)@,
    decreases g, 0int,
{
    let ghost rv = strs_view(roles@);
    let ghost lv = names_view(labels@);
    let ghost nv = names_view(recs@);
    match g {
        crate::ast::Global::End => {
            assert(gconv(*g, rv, lv, nv) == G::End);
            let r = GE::End;
            assert(r@ == gconv(*g, rv, lv, nv));
            r
        }
        crate::ast::Global::Continue(name) => {
            let n = rec_index(recs, name);
            let r = GE::Var { n };
            assert(r@ == G::Var { n: n as nat });
            assert(gconv(*g, rv, lv, nv) == G::Var { n: rec_of(nv, name@) as nat });
            assert(r@ == gconv(*g, rv, lv, nv));
            r
        }
        crate::ast::Global::Msg(m, rest) => {
            let cont = convert(rest, roles, labels, recs);
            let from = role_index(roles, &m.from);
            let to = role_index(roles, &m.to);
            let label = label_index(labels, &m.label);
            let r = GE::Comm { from, to, bs: Box::new(BE::Cons { label, g: cont, rest: Box::new(BE::Nil) }) };
            assert(gconv(*g, rv, lv, nv) == G::Comm { from: role_of(rv, m.from@), to: role_of(rv, m.to@), bs: Box::new(Bs::Cons { label: label_of(lv, m.label@), g: gconv(**rest, rv, lv, nv), rest: Box::new(Bs::Nil) }) });
            // The nested list needs its own unfolding of the view.
            assert((BE::Nil).view() == Bs::Nil);
            assert((BE::Cons { label, g: cont, rest: Box::new(BE::Nil) }).view() == Bs::Cons { label: label as int, g: cont@, rest: Box::new(Bs::Nil) });
            assert(r@ == G::Comm { from: from as int, to: to as int, bs: Box::new(Bs::Cons { label: label as int, g: cont@, rest: Box::new(Bs::Nil) }) });
            assert(r@ == gconv(*g, rv, lv, nv));
            r
        }
        crate::ast::Global::Choice { at, branches } => {
            let bs = convert_bs(branches, 0, roles, labels, recs);
            let to = choice_to_exec(branches, roles);
            let from = role_index(roles, at);
            let r = GE::Comm { from, to, bs: Box::new(bs) };
            assert(gconv(*g, rv, lv, nv) == G::Comm { from: role_of(rv, at@), to: choice_to(branches@, rv), bs: Box::new(bconv(branches@, 0, rv, lv, nv)) });
            assert(r@ == G::Comm { from: from as int, to: to as int, bs: Box::new(bs@) });
            assert(r@ == gconv(*g, rv, lv, nv));
            r
        }
        crate::ast::Global::Rec { name, body } => {
            let ghost before = recs@;
            recs.push(name.clone());
            assert(names_view(recs@) =~= names_view(before).push(name@));
            let b = convert(body, roles, labels, recs);
            recs.pop();
            assert(recs@ =~= before);
            assert(gconv(*g, rv, lv, nv) == G::Rec { body: Box::new(gconv(**body, rv, lv, nv.push(name@))) });
            let r = GE::Rec { body: Box::new(b) };
            assert(r@ == G::Rec { body: Box::new(b@) });
            assert(b@ == gconv(**body, rv, lv, nv.push(name@)));
            assert(r@ == gconv(*g, rv, lv, nv));
            r
        }
    }
}

fn convert_bs(bs: &Vec<crate::ast::Global>, i: usize, roles: &[&str], labels: &Vec<String>, recs: &mut Vec<String>) -> (r: BE)
    requires labels.len() < u32::MAX as usize, i <= bs.len(),
    ensures r@ == bconv(bs@, i as int, strs_view(roles@), names_view(labels@), names_view(old(recs)@)), final(recs)@ == old(recs)@,
    decreases bs, bs.len() - i,
{
    if i >= bs.len() {
        return BE::Nil;
    }
    match &bs[i] {
        crate::ast::Global::Msg(m, rest) => {
            let cont = convert(rest, roles, labels, recs);
            let tail = convert_bs(bs, i + 1, roles, labels, recs);
            BE::Cons { label: label_index(labels, &m.label), g: cont, rest: Box::new(tail) }
        }
        _ => convert_bs(bs, i + 1, roles, labels, recs),
    }
}

fn choice_to_exec(bs: &Vec<crate::ast::Global>, roles: &[&str]) -> (r: usize)
    ensures r as int == choice_to(bs@, strs_view(roles@)),
{
    let mut i = 0;
    assert(bs@.skip(0) =~= bs@);
    while i < bs.len()
        invariant
            i <= bs.len(),
            choice_to(bs@, strs_view(roles@)) == choice_to(bs@.skip(i as int), strs_view(roles@)),
        decreases bs.len() - i,
    {
        if let crate::ast::Global::Msg(m, _) = &bs[i] {
            assert(bs@.skip(i as int)[0] == bs@[i as int]);
            return role_index(roles, &m.to);
        }
        assert(bs@.skip(i as int).drop_first() =~= bs@.skip(i as int + 1));
        i += 1;
    }
    assert(bs@.skip(i as int).len() == 0);
    usize::MAX
}

/// The theory's type of a parsed Protocol: roles numbered by position in
/// `roles`, labels by first appearance (collected, distinct, complete),
/// recursion in de Bruijn form. Verified: the result is `gconv` of the
/// parsed type, so `accept_two_role` on it decides `wf2` of exactly that.
pub fn to_theory(global: &crate::ast::Global, roles: &[&str]) -> (r: (GE, Vec<String>))
    ensures
        distinct(names_view(r.1@)),
        forall|n: Seq<char>| has_label_in(*global, n) ==> index_of(names_view(r.1@), n) >= 0,
        r.1.len() < u32::MAX as usize ==> r.0@ == gconv(*global, strs_view(roles@), names_view(r.1@), Seq::<Seq<char>>::empty()),
{
    let mut labels: Vec<String> = Vec::new();
    collect_labels(global, &mut labels);
    if labels.len() >= u32::MAX as usize {
        return (GE::End, labels);
    }
    let mut recs: Vec<String> = Vec::new();
    assert(names_view(recs@) =~= Seq::<Seq<char>>::empty());
    let ge = convert(global, roles, &labels, &mut recs);
    (ge, labels)
}

// ------------------------------------------- the multiparty fragment (21.7) --

/// Well-formedness of a global type over `n` roles (task 21.7): closed,
/// every recursion variable guarded by a message since its own recursion,
/// every communication between two distinct roles below `n` with at
/// least one branch and pairwise distinct labels.
pub open spec fn wfn_at(g: G, n: int, env: Seq<bool>) -> bool
    decreases g,
{
    match g {
        G::End => true,
        G::Var { n: v } => v < env.len() && env[env.len() - 1 - v],
        G::Rec { body } => wfn_at(*body, n, env.push(false)),
        G::Comm { from, to, bs } => {
            &&& 0 <= from < n
            &&& 0 <= to < n
            &&& from != to
            &&& bs_len(*bs) >= 1
            &&& bs_distinct(*bs)
            &&& wfn_bs(*bs, n, all_true(env))
        }
    }
}

pub open spec fn wfn_bs(bs: Bs, n: int, env: Seq<bool>) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => true,
        Bs::Cons { g, rest, .. } => wfn_at(g, n, env) && wfn_bs(*rest, n, env),
    }
}

pub open spec fn wfn(g: G, n: int) -> bool {
    wfn_at(g, n, Seq::<bool>::empty())
}

/// The full merge of two local types (R38.3; Rumpsteak's and Scribble's):
/// equal types merge to themselves; two external choices from the same
/// role merge branch by branch, a label present in both continuing as the
/// merge of its continuations and a label present in one alone kept;
/// anything else does not merge.
pub open spec fn merge(a: L, b: L) -> Option<L>
    decreases a, b,
{
    if a == b {
        Some(a)
    } else {
        match (a, b) {
            (L::Rec { body: ba }, L::Rec { body: bb }) => match merge(*ba, *bb) {
                Some(m) => Some(L::Rec { body: Box::new(m) }),
                None => None,
            },
            (L::Recv { from: fa, ls: la }, L::Recv { from: fb, ls: lb }) => {
                if fa == fb {
                    match merge_ls(*la, *lb) {
                        Some(ls) => Some(L::Recv { from: fa, ls: Box::new(ls) }),
                        None => None,
                    }
                } else {
                    None
                }
            }
            _ => None,
        }
    }
}

/// The merge of two branch lists: the branches of `a`, each merged with
/// the branch of the same label in `b` when there is one, then the
/// branches of `b` whose labels `a` lacks.
pub open spec fn merge_ls(a: Ls, b: Ls) -> Option<Ls>
    decreases a, b,
{
    match a {
        Ls::Nil => Some(b),
        Ls::Cons { label, l, rest } => {
            let merged_l = match ls_lookup(b, label) {
                Some(lb) => merge(l, lb),
                None => Some(l),
            };
            match (merged_l, merge_ls(*rest, ls_remove(b, label))) {
                (Some(m), Some(r)) => Some(Ls::Cons { label, l: m, rest: Box::new(r) }),
                _ => None,
            }
        }
    }
}

pub open spec fn ls_remove(ls: Ls, label: int) -> Ls
    decreases ls,
{
    match ls {
        Ls::Nil => Ls::Nil,
        Ls::Cons { label: l, l: cont, rest } => if l == label { ls_remove(*rest, label) } else { Ls::Cons { label: l, l: cont, rest: Box::new(ls_remove(*rest, label)) } },
    }
}

/// A local type with no action (an end, a variable, or recursions of one).
pub open spec fn lactionless(l: L) -> bool
    decreases l,
{
    match l {
        L::End => true,
        L::Var { .. } => true,
        L::Rec { body } => lactionless(*body),
        _ => false,
    }
}

/// Variable `k` occurs free in `l`.
pub open spec fn lfree(l: L, k: nat) -> bool
    decreases l,
{
    match l {
        L::End => false,
        L::Var { n } => n == k,
        L::Rec { body } => lfree(*body, k + 1),
        L::Send { ls, .. } => lfree_ls(*ls, k),
        L::Recv { ls, .. } => lfree_ls(*ls, k),
    }
}

pub open spec fn lfree_ls(ls: Ls, k: nat) -> bool
    decreases ls,
{
    match ls {
        Ls::Nil => false,
        Ls::Cons { l, rest, .. } => lfree(l, k) || lfree_ls(*rest, k),
    }
}

/// The removal of binder `k` (which `l` does not use): every variable
/// above `k` renumbered down.
pub open spec fn lshift(l: L, k: nat) -> L
    decreases l,
{
    match l {
        L::End => L::End,
        L::Var { n } => if n > k { L::Var { n: (n - 1) as nat } } else { L::Var { n } },
        L::Rec { body } => L::Rec { body: Box::new(lshift(*body, k + 1)) },
        L::Send { to, ls } => L::Send { to, ls: Box::new(lshift_ls(*ls, k)) },
        L::Recv { from, ls } => L::Recv { from, ls: Box::new(lshift_ls(*ls, k)) },
    }
}

pub open spec fn lshift_ls(ls: Ls, k: nat) -> Ls
    decreases ls,
{
    match ls {
        Ls::Nil => Ls::Nil,
        Ls::Cons { label, l, rest } => Ls::Cons { label, l: lshift(l, k), rest: Box::new(lshift_ls(*rest, k)) },
    }
}

/// The projection of a recursion from the projection `l` of its body (the
/// Protocol_Checker's rule, `check::collapse_vacuous`, 2026-11-09): a body
/// the role takes no part in (its projection has no action) makes the
/// recursion vacuous, `continue` of it an end and `continue` of an
/// enclosing recursion the whole; a body whose projection uses the
/// variable keeps the recursion; a body whose projection does not use it
/// (an unused recursion, which R38.5 refuses since 2026-11-09, so never
/// reached from an accepted Protocol) drops the binder.
pub open spec fn close_rec(l: L) -> L {
    if lactionless(l) {
        match l {
            L::Var { n } => if n == 0 { L::End } else { L::Var { n: (n - 1) as nat } },
            _ => L::End,
        }
    } else if lfree(l, 0) {
        L::Rec { body: Box::new(l) }
    } else {
        lshift(l, 0)
    }
}

/// The projection onto role `r` with the full merge (R38.3): a role that
/// takes no part in a communication projects every branch and merges
/// them; `None` when the merge fails (the Protocol_Checker's rejection).
pub open spec fn projn(g: G, r: int) -> Option<L>
    decreases g,
{
    match g {
        G::End => Some(L::End),
        G::Var { n: v } => Some(L::Var { n: v }),
        G::Rec { body } => match projn(*body, r) {
            Some(l) => Some(close_rec(l)),
            None => None,
        },
        G::Comm { from, to, bs } => {
            if r == from {
                match projn_bs(*bs, r) {
                    Some(ls) => Some(L::Send { to, ls: Box::new(ls) }),
                    None => None,
                }
            } else if r == to {
                match projn_bs(*bs, r) {
                    Some(ls) => Some(L::Recv { from, ls: Box::new(ls) }),
                    None => None,
                }
            } else {
                projn_merge(*bs, r)
            }
        }
    }
}

pub open spec fn projn_bs(bs: Bs, r: int) -> Option<Ls>
    decreases bs,
{
    match bs {
        Bs::Nil => Some(Ls::Nil),
        Bs::Cons { label, g, rest } => match (projn(g, r), projn_bs(*rest, r)) {
            (Some(l), Some(ls)) => Some(Ls::Cons { label, l, rest: Box::new(ls) }),
            _ => None,
        },
    }
}

/// The merge of the projections of every branch (the role not involved).
pub open spec fn projn_merge(bs: Bs, r: int) -> Option<L>
    decreases bs,
{
    match bs {
        Bs::Nil => None,
        Bs::Cons { g, rest, .. } => match projn(g, r) {
            None => None,
            Some(l) => match *rest {
                Bs::Nil => Some(l),
                _ => match projn_merge(*rest, r) {
                    Some(m) => merge(l, m),
                    None => None,
                },
            },
        },
    }
}

/// The acceptance condition of an `n`-role Protocol over its syntax: well
/// formed and projectable onto every role.
pub open spec fn acceptn(g: G, n: int) -> bool {
    &&& n >= 2
    &&& wfn(g, n)
    &&& forall|r: int| 0 <= r < n ==> (#[trigger] projn(g, r)) is Some
}

/// An `n`-role configuration: every role's local type and the channel of
/// every ordered pair, `ch[from * n + to]`.
pub ghost struct NC {
    pub l: Seq<L>,
    pub ch: Seq<Seq<int>>,
}

pub open spec fn nc_wf(c: NC, n: int) -> bool {
    c.l.len() == n && c.ch.len() == n * n
}

pub open spec fn ninit(g: G, n: int) -> NC {
    NC { l: Seq::new(n as nat, |r: int| projn(g, r).unwrap()), ch: Seq::new((n * n) as nat, |i: int| Seq::<int>::empty()) }
}

pub open spec fn nsend(c: NC, n: int, r: int, to: int, label: int, cont: L) -> NC {
    NC { l: c.l.update(r, cont), ch: c.ch.update(r * n + to, c.ch[r * n + to].push(label)) }
}

pub open spec fn nrecv(c: NC, n: int, r: int, from: int, cont: L) -> NC {
    NC { l: c.l.update(r, cont), ch: c.ch.update(from * n + r, c.ch[from * n + r].drop_first()) }
}

pub open spec fn nrecv_enabled(c: NC, n: int, r: int) -> bool {
    let h = lhead(c.l[r]);
    &&& h is Recv
    &&& 0 <= h->Recv_from < n
    &&& c.ch[h->Recv_from * n + r].len() > 0
    &&& ls_lookup(*h->Recv_ls, c.ch[h->Recv_from * n + r][0]) is Some
}

pub open spec fn nstep(c: NC, n: int, c2: NC) -> bool {
    exists|r: int, label: int|
        0 <= r < n && (
            (lhead(c.l[r]) is Send && 0 <= lhead(c.l[r])->Send_to < n && lhead(c.l[r])->Send_to != r
                && ls_lookup(*lhead(c.l[r])->Send_ls, label) is Some
                && c2 == nsend(c, n, r, lhead(c.l[r])->Send_to, label, ls_lookup(*lhead(c.l[r])->Send_ls, label).unwrap()))
            || (nrecv_enabled(c, n, r) && label == c.ch[lhead(c.l[r])->Recv_from * n + r][0]
                && c2 == nrecv(c, n, r, lhead(c.l[r])->Recv_from, ls_lookup(*lhead(c.l[r])->Recv_ls, label).unwrap()))
        )
}

pub open spec fn nrun(g: G, n: int, path: Seq<NC>) -> bool {
    &&& path.len() > 0
    &&& path[0] == ninit(g, n)
    &&& forall|i: int| 1 <= i < path.len() ==> nstep(path[i - 1], n, #[trigger] path[i])
}

pub open spec fn nreachable(g: G, n: int, c: NC) -> bool {
    exists|path: Seq<NC>| #[trigger] nrun(g, n, path) && path.last() == c
}

pub open spec fn nblocked(c: NC, n: int, r: int) -> bool {
    lhead(c.l[r]) is Recv && !nrecv_enabled(c, n, r)
}

pub open spec fn nterminal(c: NC, r: int) -> bool {
    lhead(c.l[r]) is End
}

/// R41.3's waiting set over `n` roles (as `progress::deadlock_set`, over
/// local types): some member awaits, every member awaits or has finished,
/// and the sender each awaiting member waits for is a member.
pub open spec fn ndeadlock_set(c: NC, n: int, set: Seq<bool>) -> bool {
    &&& set.len() == n
    &&& exists|r: int| 0 <= r < n && set[r] && nblocked(c, n, r)
    &&& forall|r: int| 0 <= r < n && #[trigger] set[r] ==> nblocked(c, n, r) || nterminal(c, r)
    &&& forall|r: int| 0 <= r < n && #[trigger] set[r] && nblocked(c, n, r) ==> set[lhead(c.l[r])->Recv_from]
}

pub open spec fn ndeadlocked(c: NC, n: int) -> bool {
    exists|set: Seq<bool>| ndeadlock_set(c, n, set)
}

// -------------------------------- the verified flattening (two roles) --

/// The executable local type; its view is `L`.
pub enum LE {
    End,
    Send { to: usize, ls: Box<LsE> },
    Recv { from: usize, ls: Box<LsE> },
    Rec { body: Box<LE> },
    Var { n: usize },
}

pub enum LsE {
    Nil,
    Cons { label: u32, l: LE, rest: Box<LsE> },
}

impl LE {
    pub open spec fn view(&self) -> L
        decreases self,
    {
        match self {
            LE::End => L::End,
            LE::Send { to, ls } => L::Send { to: *to as int, ls: Box::new(ls.view()) },
            LE::Recv { from, ls } => L::Recv { from: *from as int, ls: Box::new(ls.view()) },
            LE::Rec { body } => L::Rec { body: Box::new(body.view()) },
            LE::Var { n } => L::Var { n: *n as nat },
        }
    }
}

impl LsE {
    pub open spec fn view(&self) -> Ls
        decreases self,
    {
        match self {
            LsE::Nil => Ls::Nil,
            LsE::Cons { label, l, rest } => Ls::Cons { label: *label as int, l: l.view(), rest: Box::new(rest.view()) },
        }
    }
}

/// The projection of an executable global type onto role `r` (two roles;
/// `proj`). A role the communication does not involve gives `End`, as
/// the specification does.
pub fn proj_exec(g: &GE, r: usize) -> (l: LE)
    ensures l@ == proj(g@, r as int),
    decreases g,
{
    match g {
        GE::End => LE::End,
        GE::Var { n } => LE::Var { n: *n },
        GE::Rec { body } => LE::Rec { body: Box::new(proj_exec(body, r)) },
        GE::Comm { from, to, bs } => {
            if r == *from {
                LE::Send { to: *to, ls: Box::new(proj_bs_exec(bs, r)) }
            } else if r == *to {
                LE::Recv { from: *from, ls: Box::new(proj_bs_exec(bs, r)) }
            } else {
                LE::End
            }
        }
    }
}

fn proj_bs_exec(bs: &BE, r: usize) -> (ls: LsE)
    ensures ls@ == proj_bs(bs@, r as int),
    decreases bs,
{
    match bs {
        BE::Nil => LsE::Nil,
        BE::Cons { label, g, rest } => LsE::Cons { label: *label, l: proj_exec(g, r), rest: Box::new(proj_bs_exec(rest, r)) },
    }
}

/// An executable type is a recursion exactly when its view is.
pub proof fn lemma_view_rec(l: &LE)
    ensures (l@ is Rec) == (l is Rec),
{
    match l {
        LE::End => {}
        LE::Var { .. } => {}
        LE::Rec { .. } => {}
        LE::Send { .. } => {}
        LE::Recv { .. } => {}
    }
}

/// A copy of an executable local type (the type is not `Clone` under
/// Verus's erasure rules for `Box`; the copy's view is the original's).
pub fn le_copy(l: &LE) -> (r: LE)
    ensures r@ == l@,
    decreases l,
{
    match l {
        LE::End => LE::End,
        LE::Var { n } => LE::Var { n: *n },
        LE::Rec { body } => LE::Rec { body: Box::new(le_copy(body)) },
        LE::Send { to, ls } => LE::Send { to: *to, ls: Box::new(lse_copy(ls)) },
        LE::Recv { from, ls } => LE::Recv { from: *from, ls: Box::new(lse_copy(ls)) },
    }
}

fn lse_copy(ls: &LsE) -> (r: LsE)
    ensures r@ == ls@,
    decreases ls,
{
    match ls {
        LsE::Nil => LsE::Nil,
        LsE::Cons { label, l, rest } => LsE::Cons { label: *label, l: le_copy(l), rest: Box::new(lse_copy(rest)) },
    }
}

/// `lsubst` on executable local types; `None` when a substitution index
/// would overflow (a nesting no Protocol reaches: the flattening then
/// refuses rather than computes a wrong term).
pub fn lsubst_exec(l: &LE, k: usize, by: &LE) -> (r: Option<LE>)
    ensures r is Some ==> r.unwrap()@ == lsubst(l@, k as nat, by@),
    decreases l,
{
    match l {
        LE::End => Some(LE::End),
        LE::Var { n } => {
            if *n == k { Some(le_copy(by)) } else if *n > k { Some(LE::Var { n: *n - 1 }) } else { Some(LE::Var { n: *n }) }
        }
        LE::Rec { body } => {
            if k == usize::MAX {
                return None;
            }
            match lsubst_exec(body, k + 1, by) {
                Some(b) => Some(LE::Rec { body: Box::new(b) }),
                None => None,
            }
        }
        LE::Send { to, ls } => match lsubst_ls_exec(ls, k, by) {
            Some(x) => Some(LE::Send { to: *to, ls: Box::new(x) }),
            None => None,
        },
        LE::Recv { from, ls } => match lsubst_ls_exec(ls, k, by) {
            Some(x) => Some(LE::Recv { from: *from, ls: Box::new(x) }),
            None => None,
        },
    }
}

fn lsubst_ls_exec(ls: &LsE, k: usize, by: &LE) -> (r: Option<LsE>)
    ensures r is Some ==> r.unwrap()@ == lsubst_ls(ls@, k as nat, by@),
    decreases ls,
{
    match ls {
        LsE::Nil => Some(LsE::Nil),
        LsE::Cons { label, l, rest } => match (lsubst_exec(l, k, by), lsubst_ls_exec(rest, k, by)) {
            (Some(a), Some(b)) => Some(LsE::Cons { label: *label, l: a, rest: Box::new(b) }),
            _ => None,
        },
    }
}

/// The head normal form of an executable local type (`lhead`), or `None`
/// when an unfolding would overflow an index.
pub fn lhead_exec(l: &LE) -> (r: Option<LE>)
    ensures r is Some ==> r.unwrap()@ == lhead(l@),
{
    let mut cur = le_copy(l);
    let mut n = llead_recs_exec(&cur);
    if n == usize::MAX {
        return None;
    }
    let ghost total = n as nat;
    assert(lhead(l@) == lhead_n(cur@, total));
    while n > 0
        invariant lhead_n(cur@, n as nat) == lhead(l@),
        decreases n,
    {
        proof { lemma_view_rec(&cur); }
        match &cur {
            LE::Rec { body } => {
                assert(cur@ == L::Rec { body: Box::new(body@) });
                let unfolded = match lsubst_exec(body, 0, &cur) {
                    Some(u) => u,
                    None => return None,
                };
                assert(unfolded@ == lsubst(body@, 0, cur@));
                assert(lunfold(cur@) == lsubst(body@, 0, cur@));
                assert(lhead_n(cur@, n as nat) == lhead_n(unfolded@, (n - 1) as nat));
                cur = unfolded;
            }
            _ => {
                assert(lhead_n(cur@, n as nat) == cur@);
                return Some(cur);
            }
        }
        n -= 1;
    }
    assert(lhead_n(cur@, 0) == cur@);
    Some(cur)
}

fn llead_recs_exec(l: &LE) -> (r: usize)
    ensures r as nat == llead_recs(l@) || (r == usize::MAX && llead_recs(l@) >= usize::MAX),
    decreases l,
{
    match l {
        LE::Rec { body } => {
            let m = llead_recs_exec(body);
            if m == usize::MAX { usize::MAX } else { m + 1 }
        }
        _ => 0,
    }
}

/// Structural equality of executable local types, deciding equality of
/// their views.
pub fn le_eq(a: &LE, b: &LE) -> (r: bool)
    ensures r == (a@ == b@),
    decreases a,
{
    match (a, b) {
        (LE::End, LE::End) => true,
        (LE::Var { n: x }, LE::Var { n: y }) => *x == *y,
        (LE::Rec { body: x }, LE::Rec { body: y }) => le_eq(x, y),
        (LE::Send { to: tx, ls: lx }, LE::Send { to: ty, ls: ly }) => *tx == *ty && lse_eq(lx, ly),
        (LE::Recv { from: fx, ls: lx }, LE::Recv { from: fy, ls: ly }) => *fx == *fy && lse_eq(lx, ly),
        _ => false,
    }
}

fn lse_eq(a: &LsE, b: &LsE) -> (r: bool)
    ensures r == (a@ == b@),
    decreases a,
{
    match (a, b) {
        (LsE::Nil, LsE::Nil) => true,
        (LsE::Cons { label: la, l: xa, rest: ra }, LsE::Cons { label: lb, l: xb, rest: rb }) => *la == *lb && le_eq(xa, xb) && lse_eq(ra, rb),
        _ => false,
    }
}

/// The view of a state table.
pub open spec fn tbl_view(v: Seq<LE>) -> Seq<L> {
    v.map(|i: int, l: LE| l@)
}

/// The most states a role's table may hold before the flattening refuses.
pub const MAX_STATES: usize = 4096;

/// Finds the entry equal to `h` in `tbl`, or appends it; the index.
pub fn intern(tbl: &mut Vec<LE>, h: LE) -> (j: Option<usize>)
    requires old(tbl).len() <= MAX_STATES,
    ensures
        j is Some ==> (j.unwrap() as int) < final(tbl).len() && tbl_view(final(tbl)@)[j.unwrap() as int] == h@,
        j is Some ==> final(tbl).len() <= MAX_STATES,
        j is None ==> final(tbl)@ == old(tbl)@,
        forall|i: int| 0 <= i < old(tbl).len() ==> #[trigger] tbl_view(final(tbl)@)[i] == tbl_view(old(tbl)@)[i],
        final(tbl).len() >= old(tbl).len(),
        j is Some && (j.unwrap() as int) >= old(tbl).len() ==> final(tbl).len() == old(tbl).len() + 1,
{
    let mut i = 0;
    while i < tbl.len()
        invariant i <= tbl.len(), tbl@ == old(tbl)@, tbl.len() <= MAX_STATES,
        decreases tbl.len() - i,
    {
        if le_eq(&tbl[i], &h) {
            assert(tbl_view(tbl@)[i as int] == tbl@[i as int]@);
            assert(tbl@[i as int]@ == h@);
            assert(tbl_view(tbl@)[i as int] == h@);
            return Some(i);
        }
        i += 1;
    }
    if tbl.len() >= MAX_STATES {
        return None;
    }
    let ghost before = tbl@;
    tbl.push(h);
    assert forall|k: int| 0 <= k < before.len() implies #[trigger] tbl_view(tbl@)[k] == tbl_view(before)[k] by {
        assert(tbl@[k] == before[k]);
    }
    assert(tbl_view(tbl@)[before.len() as int] == tbl@[before.len() as int]@);
    assert(tbl@[before.len() as int]@ == h@);
    assert(tbl_view(tbl@)[before.len() as int] == h@);
    assert(tbl.len() - 1 == before.len());
    Some(tbl.len() - 1)
}

/// The branches of a head-normal executable entry (`ls_of` of the view).
pub fn branches_of(e: &LE) -> (r: Option<&LsE>)
    ensures
        r is Some ==> r.unwrap()@ == ls_of(e@),
        r is None ==> (e@ is End || e@ is Rec || e@ is Var),
{
    match e {
        LE::Send { ls, .. } => Some(ls),
        LE::Recv { ls, .. } => Some(ls),
        _ => None,
    }
}

/// Whether a head-normal entry of role `r` is admissible (`entry_ok`).
fn entry_ok_exec(e: &LE, r: usize) -> (ok: bool)
    requires r < 2,
    ensures ok == entry_ok(e@, r as int),
{
    match e {
        LE::End => true,
        LE::Send { to, ls } => {
            proof { lemma_lse_cons_view(ls); }
            let cons = match &**ls { LsE::Cons { .. } => true, LsE::Nil => false };
            *to == 1 - r && cons
        }
        LE::Recv { from, ls } => {
            proof { lemma_lse_cons_view(ls); }
            let cons = match &**ls { LsE::Cons { .. } => true, LsE::Nil => false };
            *from == 1 - r && cons
        }
        _ => false,
    }
}

pub proof fn lemma_lse_cons_view(ls: &LsE)
    ensures (ls@ is Cons) == (ls is Cons),
{
    match ls {
        LsE::Nil => {}
        LsE::Cons { .. } => {}
    }
}

/// A transition list and two state tables with the properties `flattens`
/// asks of every entry processed so far.
pub open spec fn processed_ok(trans: Seq<Trans>, tb: Seq<Seq<L>>, r: int, upto: int) -> bool {
    forall|i: int, label: int| 0 <= i < upto && #[trigger] ls_lookup(ls_of(tb[r][i]), label) is Some
        ==> exists|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(ls_lookup(ls_of(tb[r][i]), label).unwrap())
            && has_transition_seq(trans, Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: other(r) as usize, label: label as u32, to: j as usize })
}

pub open spec fn has_transition_seq(trans: Seq<Trans>, t: Trans) -> bool {
    exists|i: int| 0 <= i < trans.len() && trans[i] == t
}

/// Every transition of the list is a branch of its entry (`flattens`'
/// last clause, over the tables' views).
pub open spec fn trans_ok(trans: Seq<Trans>, tb: Seq<Seq<L>>) -> bool {
    forall|k: int| 0 <= k < trans.len() ==> {
        let t = #[trigger] trans[k];
        &&& t.role < 2
        &&& (t.from as int) < tb[t.role as int].len()
        &&& (t.to as int) < tb[t.role as int].len()
        &&& t.peer == other(t.role as int)
        &&& (t.send ==> tb[t.role as int][t.from as int] is Send)
        &&& (t.send == false ==> tb[t.role as int][t.from as int] is Recv)
        &&& ls_lookup(ls_of(tb[t.role as int][t.from as int]), t.label as int) is Some
        &&& tb[t.role as int][t.to as int] == lhead(ls_lookup(ls_of(tb[t.role as int][t.from as int]), t.label as int).unwrap())
    }
}

/// The labels of an executable branch list.
pub fn labels_of(ls: &LsE) -> (r: Vec<u32>)
    ensures
        forall|label: int| ls_lookup(ls@, label) is Some <==> exists|i: int| 0 <= i < r@.len() && #[trigger] r@[i] as int == label,
        r@.len() == ls_len(ls@),
        forall|i: int| 0 <= i < r@.len() ==> #[trigger] r@[i] as int == ls_label_at(ls@, i),
{
    // In the branches' order (2026-11-09: the Generator numbers a role's
    // states by walking these labels, and the bodies name the states).
    let mut v: Vec<u32> = Vec::new();
    let mut cur: &LsE = ls;
    let ghost whole = ls@;
    while lse_is_cons(cur)
        invariant
            v@.len() + ls_len(cur@) == ls_len(whole),
            forall|i: int| 0 <= i < v@.len() ==> #[trigger] v@[i] as int == ls_label_at(whole, i),
            forall|i: int| 0 <= i < ls_len(cur@) ==> #[trigger] ls_label_at(cur@, i) == ls_label_at(whole, v@.len() as int + i),
            forall|label: int| ls_lookup(whole, label) is Some <==> (exists|i: int| 0 <= i < v@.len() && #[trigger] v@[i] as int == label) || ls_lookup(cur@, label) is Some,
        decreases cur,
    {
        match cur {
            LsE::Nil => { assert(false); }
            LsE::Cons { label, rest, .. } => {
                let ghost before = v@;
                let ghost cur_before = cur@;
                v.push(*label);
                proof {
                    assert(v@ == before.push(*label));
                    assert(ls_label_at(cur_before, 0) == *label as int);
                    assert forall|i: int| 0 <= i < v@.len() implies #[trigger] v@[i] as int == ls_label_at(whole, i) by {
                        if i == before.len() {
                            assert(ls_label_at(cur_before, 0) == ls_label_at(whole, before.len() as int));
                        }
                    }
                    assert forall|i: int| 0 <= i < ls_len(rest@) implies #[trigger] ls_label_at(rest@, i) == ls_label_at(whole, v@.len() as int + i) by {
                        assert(ls_label_at(cur_before, i + 1) == ls_label_at(rest@, i));
                        assert(ls_label_at(cur_before, i + 1) == ls_label_at(whole, before.len() as int + (i + 1)));
                    }
                    assert forall|lb: int| ls_lookup(whole, lb) is Some implies (exists|i: int| 0 <= i < v@.len() && #[trigger] v@[i] as int == lb) || ls_lookup(rest@, lb) is Some by {
                        if ls_lookup(cur_before, lb) is Some && !(ls_lookup(rest@, lb) is Some) {
                            assert(*label as int == lb);
                            assert(v@[before.len() as int] as int == lb);
                        } else if !(ls_lookup(cur_before, lb) is Some) {
                            let i = choose|i: int| 0 <= i < before.len() && #[trigger] before[i] as int == lb;
                            assert(v@[i] as int == lb);
                        }
                    }
                    assert forall|lb: int| ((exists|i: int| 0 <= i < v@.len() && #[trigger] v@[i] as int == lb) || ls_lookup(rest@, lb) is Some) implies ls_lookup(whole, lb) is Some by {
                        if ls_lookup(rest@, lb) is Some {
                            assert(ls_lookup(cur_before, lb) is Some);
                        } else {
                            let i = choose|i: int| 0 <= i < v@.len() && #[trigger] v@[i] as int == lb;
                            if i < before.len() {
                                assert(before[i] as int == lb);
                            } else {
                                assert(*label as int == lb);
                                assert(ls_lookup(cur_before, lb) is Some);
                            }
                        }
                    }
                }
                cur = rest;
            }
        }
    }
    proof {
        assert(cur@ is Nil);
        assert(ls_len(cur@) == 0);
        assert(forall|lb: int| !(ls_lookup(cur@, lb) is Some));
    }
    v
}

fn lse_is_cons(ls: &LsE) -> (r: bool)
    ensures r == (ls@ is Cons),
{
    match ls {
        LsE::Nil => false,
        LsE::Cons { .. } => true,
    }
}

/// The branch count of a local list.
pub open spec fn ls_len(ls: Ls) -> nat
    decreases ls,
{
    match ls {
        Ls::Nil => 0,
        Ls::Cons { rest, .. } => 1 + ls_len(*rest),
    }
}

/// The label of branch `i` of a local list.
pub open spec fn ls_label_at(ls: Ls, i: int) -> int
    decreases ls,
{
    match ls {
        Ls::Nil => 0,
        Ls::Cons { label, rest, .. } => if i == 0 { label } else { ls_label_at(*rest, i - 1) },
    }
}

/// The continuation of `label` in `ls` (`ls_lookup`).
pub fn lookup_exec<'a>(ls: &'a LsE, label: u32) -> (r: Option<&'a LE>)
    ensures
        r is Some ==> ls_lookup(ls@, label as int) == Some(r.unwrap()@),
        r is None ==> ls_lookup(ls@, label as int) is None,
    decreases ls,
{
    match ls {
        LsE::Nil => None,
        LsE::Cons { label: l, l: cont, rest } => if *l == label { Some(cont) } else { lookup_exec(rest, label) },
    }
}

pub proof fn lemma_has_trans_push(trans: Seq<Trans>, t: Trans, x: Trans)
    requires has_transition_seq(trans, x),
    ensures has_transition_seq(trans.push(t), x),
{
    let i = choose|i: int| 0 <= i < trans.len() && trans[i] == x;
    assert(trans.push(t)[i] == x);
}

/// `processed_ok` for one role's own table.
pub open spec fn role_ok(trans: Seq<Trans>, tbl: Seq<L>, r: int, upto: int) -> bool {
    forall|i: int, label: int| 0 <= i < upto && #[trigger] ls_lookup(ls_of(tbl[i]), label) is Some
        ==> exists|j: int| 0 <= j < tbl.len() && tbl[j] == lhead(ls_lookup(ls_of(tbl[i]), label).unwrap())
            && has_transition_seq(trans, Trans { role: r as usize, from: i as usize, send: tbl[i] is Send, peer: other(r) as usize, label: label as u32, to: j as usize })
}

/// The two tables as `flattens` sees them.
pub open spec fn tables(t0: Seq<LE>, t1: Seq<LE>) -> Seq<Seq<L>> {
    seq![tbl_view(t0), tbl_view(t1)]
}

/// Processes the branches of entry `i` of role `r`: a state for each
/// continuation's head, a transition for each label.
fn process_entry(entry: &LE, r: usize, i: usize, tbl: &mut Vec<LE>, trans: &mut Vec<Trans>, peer_tbl: &Vec<LE>) -> (ok: bool)
    requires
        r < 2, i < old(tbl).len(), old(tbl).len() <= MAX_STATES,
        entry@ == tbl_view(old(tbl)@)[i as int],
        entry_ok(entry@, r as int),
        trans_ok(old(trans)@, if r == 0 { tables(old(tbl)@, peer_tbl@) } else { tables(peer_tbl@, old(tbl)@) }),
    ensures
        final(tbl).len() >= old(tbl).len(), final(tbl).len() <= MAX_STATES,
        forall|k: int| 0 <= k < old(tbl).len() ==> #[trigger] tbl_view(final(tbl)@)[k] == tbl_view(old(tbl)@)[k],
        forall|x: Trans| has_transition_seq(old(trans)@, x) ==> has_transition_seq(final(trans)@, x),
        trans_ok(final(trans)@, if r == 0 { tables(final(tbl)@, peer_tbl@) } else { tables(peer_tbl@, final(tbl)@) }),
        ok ==> forall|label: int| #[trigger] ls_lookup(ls_of(entry@), label) is Some
            ==> exists|j: int| 0 <= j < final(tbl).len() && tbl_view(final(tbl)@)[j] == lhead(ls_lookup(ls_of(entry@), label).unwrap())
                && has_transition_seq(final(trans)@, Trans { role: r, from: i, send: entry@ is Send, peer: (1 - r) as usize, label: label as u32, to: j as usize }),
{
    let ls = match branches_of(entry) {
        None => {
            assert(forall|label: int| !(ls_lookup(ls_of(entry@), label) is Some));
            return true;
        }
        Some(ls) => ls,
    };
    let is_send = match entry { LE::Send { .. } => true, _ => false };
    proof { lemma_view_rec(entry); }
    assert(is_send == (entry@ is Send)) by {
        match entry {
            LE::End => {} LE::Var { .. } => {} LE::Rec { .. } => {} LE::Send { .. } => {} LE::Recv { .. } => {}
        }
    }
    let labels = labels_of(ls);
    let mut k = 0;
    while k < labels.len()
        invariant
            k <= labels.len(), r < 2, i < tbl.len(), tbl.len() <= MAX_STATES,
            ls@ == ls_of(entry@), entry_ok(entry@, r as int), is_send == (entry@ is Send),
            forall|label: int| ls_lookup(ls@, label) is Some <==> exists|q: int| 0 <= q < labels@.len() && #[trigger] labels@[q] as int == label,
            tbl.len() >= old(tbl).len(),
            forall|q: int| 0 <= q < old(tbl).len() ==> #[trigger] tbl_view(tbl@)[q] == tbl_view(old(tbl)@)[q],
            tbl_view(tbl@)[i as int] == entry@,
            forall|x: Trans| has_transition_seq(old(trans)@, x) ==> has_transition_seq(trans@, x),
            trans_ok(trans@, if r == 0 { tables(tbl@, peer_tbl@) } else { tables(peer_tbl@, tbl@) }),
            forall|q: int| 0 <= q < k ==> exists|j: int| 0 <= j < tbl.len() && tbl_view(tbl@)[j] == lhead(ls_lookup(ls@, #[trigger] labels@[q] as int).unwrap())
                && has_transition_seq(trans@, Trans { role: r, from: i, send: is_send, peer: (1 - r) as usize, label: labels@[q], to: j as usize }),
        decreases labels.len() - k,
    {
        let label = labels[k];
        assert(ls_lookup(ls@, label as int) is Some);
        let cont = match lookup_exec(ls, label) {
            Some(c) => c,
            None => { assert(false); return false; }
        };
        let h = match lhead_exec(cont) {
            Some(h) => h,
            None => return false,
        };
        let ghost tbl_before = tbl@;
        let ghost trans_before = trans@;
        let j = match intern(tbl, h) {
            Some(j) => j,
            None => return false,
        };
        let t = Trans { role: r, from: i, send: is_send, peer: 1 - r, label, to: j };
        trans.push(t);
        proof {
            assert forall|x: Trans| has_transition_seq(trans_before, x) implies has_transition_seq(trans@, x) by {
                lemma_has_trans_push(trans_before, t, x);
            }
            assert(trans@[trans@.len() - 1] == t);
            assert(has_transition_seq(trans@, t));
            // The new transition is a branch of its entry.
            assert(tbl_view(tbl@)[j as int] == lhead(ls_lookup(ls_of(entry@), label as int).unwrap()));
            assert(tbl_view(tbl@)[i as int] == entry@);
            // The old facts carry over the grown table and list.
            assert forall|q: int| 0 <= q < k implies exists|jj: int| 0 <= jj < tbl.len() && tbl_view(tbl@)[jj] == lhead(ls_lookup(ls@, #[trigger] labels@[q] as int).unwrap())
                && has_transition_seq(trans@, Trans { role: r, from: i, send: is_send, peer: (1 - r) as usize, label: labels@[q], to: jj as usize }) by {
                let jj = choose|jj: int| 0 <= jj < tbl_before.len() && tbl_view(tbl_before)[jj] == lhead(ls_lookup(ls@, labels@[q] as int).unwrap())
                    && has_transition_seq(trans_before, Trans { role: r, from: i, send: is_send, peer: (1 - r) as usize, label: labels@[q], to: jj as usize });
                assert(tbl_view(tbl@)[jj] == tbl_view(tbl_before)[jj]);
                lemma_has_trans_push(trans_before, t, Trans { role: r, from: i, send: is_send, peer: (1 - r) as usize, label: labels@[q], to: jj as usize });
            }
        }
        k += 1;
    }
    assert forall|label: int| #[trigger] ls_lookup(ls_of(entry@), label) is Some
        implies exists|j: int| 0 <= j < tbl.len() && tbl_view(tbl@)[j] == lhead(ls_lookup(ls_of(entry@), label).unwrap())
            && has_transition_seq(trans@, Trans { role: r, from: i, send: entry@ is Send, peer: (1 - r) as usize, label: label as u32, to: j as usize }) by {
        let q = choose|q: int| 0 <= q < labels@.len() && #[trigger] labels@[q] as int == label;
    }
    true
}

/// Builds role `r`'s state table and transitions: entry 0 the head of the
/// projection, every entry admissible and processed.
fn flatten_role(g: &GE, r: usize, tbl: &mut Vec<LE>, trans: &mut Vec<Trans>, peer_tbl: &Vec<LE>) -> (ok: bool)
    requires
        r < 2, old(tbl).len() == 0,
        trans_ok(old(trans)@, if r == 0 { tables(old(tbl)@, peer_tbl@) } else { tables(peer_tbl@, old(tbl)@) }),
    ensures
        final(tbl).len() <= MAX_STATES,
        forall|x: Trans| has_transition_seq(old(trans)@, x) ==> has_transition_seq(final(trans)@, x),
        trans_ok(final(trans)@, if r == 0 { tables(final(tbl)@, peer_tbl@) } else { tables(peer_tbl@, final(tbl)@) }),
        ok ==> final(tbl).len() >= 1 && tbl_view(final(tbl)@)[0] == lhead(proj(g@, r as int)),
        ok ==> forall|k: int| 0 <= k < final(tbl).len() ==> entry_ok(#[trigger] tbl_view(final(tbl)@)[k], r as int),
        ok ==> role_ok(final(trans)@, tbl_view(final(tbl)@), r as int, final(tbl).len() as int),
{
    let p = proj_exec(g, r);
    let h0 = match lhead_exec(&p) {
        Some(h) => h,
        None => return false,
    };
    tbl.push(h0);
    assert(tbl_view(tbl@)[0] == tbl@[0]@);
    let mut i = 0;
    while i < tbl.len()
        invariant
            r < 2, i <= tbl.len(), tbl.len() <= MAX_STATES, tbl.len() >= 1,
            tbl_view(tbl@)[0] == lhead(proj(g@, r as int)),
            forall|k: int| 0 <= k < i ==> entry_ok(#[trigger] tbl_view(tbl@)[k], r as int),
            role_ok(trans@, tbl_view(tbl@), r as int, i as int),
            forall|x: Trans| has_transition_seq(old(trans)@, x) ==> has_transition_seq(trans@, x),
            trans_ok(trans@, if r == 0 { tables(tbl@, peer_tbl@) } else { tables(peer_tbl@, tbl@) }),
        decreases MAX_STATES - i,
    {
        let entry = le_copy(&tbl[i]);
        assert(entry@ == tbl_view(tbl@)[i as int]);
        if !entry_ok_exec(&entry, r) {
            return false;
        }
        let ghost tbl_before = tbl@;
        let ghost trans_before = trans@;
        if !process_entry(&entry, r, i, tbl, trans, peer_tbl) {
            return false;
        }
        proof {
            // The table's prefix and the transitions carry the earlier facts.
            assert forall|k: int| 0 <= k < i + 1 implies entry_ok(#[trigger] tbl_view(tbl@)[k], r as int) by {
                assert(tbl_view(tbl@)[k] == tbl_view(tbl_before)[k]);
            }
            assert forall|k: int, label: int| 0 <= k < i + 1 && #[trigger] ls_lookup(ls_of(tbl_view(tbl@)[k]), label) is Some
                implies exists|j: int| 0 <= j < tbl.len() && tbl_view(tbl@)[j] == lhead(ls_lookup(ls_of(tbl_view(tbl@)[k]), label).unwrap())
                    && has_transition_seq(trans@, Trans { role: r, from: k as usize, send: tbl_view(tbl@)[k] is Send, peer: other(r as int) as usize, label: label as u32, to: j as usize }) by {
                assert(tbl_view(tbl@)[k] == tbl_view(tbl_before)[k]);
                if k < i {
                    let j = choose|j: int| 0 <= j < tbl_before.len() && tbl_view(tbl_before)[j] == lhead(ls_lookup(ls_of(tbl_view(tbl_before)[k]), label).unwrap())
                        && has_transition_seq(trans_before, Trans { role: r, from: k as usize, send: tbl_view(tbl_before)[k] is Send, peer: other(r as int) as usize, label: label as u32, to: j as usize });
                    assert(tbl_view(tbl@)[j] == tbl_view(tbl_before)[j]);
                } else {
                    assert(tbl_view(tbl@)[k] == entry@);
                    assert(other(r as int) == 1 - r as int);
                }
            }
        }
        i += 1;
    }
    true
}

/// The verified flattening of a two-role Protocol: the automaton and the
/// two state tables, related by `flattens`; `None` when the Protocol is
/// refused (a state beyond `MAX_STATES`, an inadmissible entry, an index
/// overflow), which the checker reports.
pub fn flatten_two(g: &GE, capacity: usize) -> (r: Option<(System, Vec<LE>, Vec<LE>)>)
    ensures r is Some ==> flattens(r.unwrap().0, g@, tables(r.unwrap().1@, r.unwrap().2@)),
{
    let mut t0: Vec<LE> = Vec::new();
    let mut t1: Vec<LE> = Vec::new();
    let mut trans: Vec<Trans> = Vec::new();
    if !flatten_role(g, 0, &mut t0, &mut trans, &t1) {
        return None;
    }
    if !flatten_role(g, 1, &mut t1, &mut trans, &t0) {
        return None;
    }
    let s = System { roles: 2, capacity, trans };
    proof {
        let tb = tables(t0@, t1@);
        assert(tb.len() == 2);
        assert(tb[0] == tbl_view(t0@));
        assert(tb[1] == tbl_view(t1@));
        // Role 0's facts survived role 1's additions: its table did not
        // change and the transitions only grew.
        assert forall|i: int, label: int| 0 <= i < tb[0].len() && #[trigger] ls_lookup(ls_of(tb[0][i]), label) is Some
            implies exists|j: int| 0 <= j < tb[0].len() && tb[0][j] == lhead(ls_lookup(ls_of(tb[0][i]), label).unwrap())
                && has_transition(s, Trans { role: 0, from: i as usize, send: tb[0][i] is Send, peer: other(0) as usize, label: label as u32, to: j as usize }) by {
            let j = choose|j: int| 0 <= j < tb[0].len() && tb[0][j] == lhead(ls_lookup(ls_of(tb[0][i]), label).unwrap())
                && has_transition_seq(s.trans@, Trans { role: 0, from: i as usize, send: tb[0][i] is Send, peer: other(0) as usize, label: label as u32, to: j as usize });
        }
        assert forall|i: int, label: int| 0 <= i < tb[1].len() && #[trigger] ls_lookup(ls_of(tb[1][i]), label) is Some
            implies exists|j: int| 0 <= j < tb[1].len() && tb[1][j] == lhead(ls_lookup(ls_of(tb[1][i]), label).unwrap())
                && has_transition(s, Trans { role: 1, from: i as usize, send: tb[1][i] is Send, peer: other(1) as usize, label: label as u32, to: j as usize }) by {
            let j = choose|j: int| 0 <= j < tb[1].len() && tb[1][j] == lhead(ls_lookup(ls_of(tb[1][i]), label).unwrap())
                && has_transition_seq(s.trans@, Trans { role: 1, from: i as usize, send: tb[1][i] is Send, peer: other(1) as usize, label: label as u32, to: j as usize });
        }
    }
    Some((s, t0, t1))
}

} // verus!




