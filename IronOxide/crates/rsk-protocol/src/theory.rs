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
use crate::progress::{at, blocked, chan_of, deadlock_set, deadlocked, enabled, has_trans, initial, reachable, run, step, succ, terminal, wf_cfg, Cfg, Trans};
use crate::progress::System;

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

/// Well-formedness of a two-role global type under `depth` enclosing
/// recursions, `guarded` when a message separates it from the nearest
/// enclosing `Rec`.
pub open spec fn wf2_at(g: G, depth: nat, guarded: bool) -> bool
    decreases g,
{
    match g {
        G::End => true,
        G::Var { n } => n < depth && guarded,
        G::Rec { body } => wf2_at(*body, depth + 1, false),
        G::Comm { from, to, bs } => {
            &&& (from == 0 || from == 1)
            &&& (to == 0 || to == 1)
            &&& from != to
            &&& bs_len(*bs) >= 1
            &&& bs_distinct(*bs)
            &&& wf2_bs(*bs, depth)
        }
    }
}

pub open spec fn wf2_bs(bs: Bs, depth: nat) -> bool
    decreases bs,
{
    match bs {
        Bs::Nil => true,
        Bs::Cons { g, rest, .. } => wf2_at(g, depth, true) && wf2_bs(*rest, depth),
    }
}

/// The checker's structural acceptance of a two-role Protocol: closed and
/// well formed.
pub open spec fn wf2(g: G) -> bool {
    wf2_at(g, 0, true)
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

/// Under a recursion, before any message, the spine ends in a
/// communication or an end: a variable there would be unguarded.
proof fn lemma_wf_spine(g: G, depth: nat)
    requires wf2_at(g, depth, false),
    ensures spine_ok(g),
    decreases g,
{
    if let G::Rec { body } = g {
        lemma_wf_spine(*body, depth + 1);
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

/// Substituting a closed well-formed type for the outermost variable
/// keeps well-formedness one level down.
proof fn lemma_subst_wf(g: G, depth: nat, guarded: bool, by: G)
    requires wf2_at(g, depth + 1, guarded), wf2_at(by, 0, true), spine_ok(by) || true,
    ensures wf2_at(subst(g, depth, by), depth, guarded),
    decreases g,
{
    match g {
        G::End => {}
        G::Var { n } => {
            if n == depth {
                lemma_wf_weaken(by, 0, depth, true);
            }
        }
        G::Rec { body } => {
            lemma_subst_wf(*body, depth + 1, false, by);
        }
        G::Comm { from, to, bs } => {
            lemma_subst_wf_bs(*bs, depth, by);
            lemma_subst_bs_shape(*bs, depth, by);
        }
    }
}

proof fn lemma_subst_wf_bs(bs: Bs, depth: nat, by: G)
    requires wf2_bs(bs, depth + 1), wf2_at(by, 0, true),
    ensures wf2_bs(subst_bs(bs, depth, by), depth),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_subst_wf(g, depth, true, by);
        lemma_subst_wf_bs(*rest, depth, by);
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

/// A type well formed under `d` recursions is well formed under more.
proof fn lemma_wf_weaken(g: G, d: nat, d2: nat, guarded: bool)
    requires wf2_at(g, d, guarded), d <= d2,
    ensures wf2_at(g, d2, guarded),
    decreases g,
{
    match g {
        G::Rec { body } => lemma_wf_weaken(*body, d + 1, d2 + 1, false),
        G::Comm { bs, .. } => lemma_wf_weaken_bs(*bs, d, d2),
        _ => {}
    }
}

proof fn lemma_wf_weaken_bs(bs: Bs, d: nat, d2: nat)
    requires wf2_bs(bs, d), d <= d2,
    ensures wf2_bs(bs, d2),
    decreases bs,
{
    if let Bs::Cons { g, rest, .. } = bs {
        lemma_wf_weaken(g, d, d2, true);
        lemma_wf_weaken_bs(*rest, d, d2);
    }
}

/// Unfolding a closed well-formed recursion gives a closed well-formed
/// type with one recursion fewer at the top.
pub proof fn lemma_unfold_wf(g: G)
    requires wf2(g), g is Rec,
    ensures wf2(unfold(g)), lead_recs(unfold(g)) == lead_recs(g) - 1,
{
    let body = *g->Rec_body;
    lemma_subst_wf(body, 0, false, g);
    lemma_wf_guarded_weaken(subst(body, 0, g), 0);
    lemma_wf_spine(body, 1);
    lemma_subst_spine(body, 0, g);
}

/// A type well formed unguarded is well formed guarded.
proof fn lemma_wf_guarded_weaken(g: G, d: nat)
    requires wf2_at(g, d, false),
    ensures wf2_at(g, d, true),
    decreases g,
{
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
/// formed.
proof fn lemma_branch_wf(bs: Bs, label: int)
    requires wf2_bs(bs, 0), bs_lookup(bs, label) is Some,
    ensures wf2(bs_lookup(bs, label).unwrap()),
    decreases bs,
{
    if let Bs::Cons { label: l, rest, .. } = bs {
        if l != label {
            lemma_branch_wf(*rest, label);
        }
    }
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

/// Recursions nest no deeper than this in a Protocol the check accepts
/// (the parser's input is a declaration of a few lines; the bound keeps
/// the depth counter from overflowing).
pub const MAX_REC_DEPTH: usize = 1 << 20;

/// The checker's structural acceptance of a two-role Protocol, decided:
/// `true` only when the specification `wf2_at` holds of the view (and
/// always when it holds with recursions nested below `MAX_REC_DEPTH`).
pub fn wf2_exec(g: &GE, depth: usize, guarded: bool) -> (r: bool)
    requires depth <= MAX_REC_DEPTH,
    ensures r ==> wf2_at(g.view(), depth as nat, guarded),
    decreases g,
{
    match g {
        GE::End => true,
        GE::Var { n } => *n < depth && guarded,
        GE::Rec { body } => depth < MAX_REC_DEPTH && wf2_exec(body, depth + 1, false),
        GE::Comm { from, to, bs } => {
            (*from == 0 || *from == 1) && (*to == 0 || *to == 1) && *from != *to && be_len(bs) >= 1 && be_distinct(bs) && wf2_bs_exec(bs, depth)
        }
    }
}

fn wf2_bs_exec(bs: &BE, depth: usize) -> (r: bool)
    requires depth <= MAX_REC_DEPTH,
    ensures r ==> wf2_bs(bs.view(), depth as nat),
    decreases bs,
{
    match bs {
        BE::Nil => true,
        BE::Cons { g, rest, .. } => wf2_exec(g, depth, true) && wf2_bs_exec(rest, depth),
    }
}

/// `check_protocol`'s hook (two roles): accepting establishes the
/// hypothesis of `theorem_two_role_progress` for the Protocol's view.
pub fn accept_two_role(g: &GE) -> (r: bool)
    ensures r ==> wf2(g.view()),
        r ==> forall|c: LC| lreachable(g.view(), c) ==> !ldeadlocked(c),
{
    let ok = wf2_exec(g, 0, true);
    proof {
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

} // verus!

