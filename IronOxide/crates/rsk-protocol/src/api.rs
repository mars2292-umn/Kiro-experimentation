//! The operations of a role's states, from the verified flattening
//! (R38.4; 2026-11-21): the last verified step between a Protocol and the
//! typestate API the Generator emits for a role.
//!
//! `theory::flatten_two` and `multiparty::flatten_n` produce, per role, a
//! table of head-normal local types (entry 0 the head of the role's
//! projection) and the automaton's transitions, and prove `flattens` /
//! `flattens_n` of them. The Generator needs, per state, the kind of the
//! state (send, receive, end), the peer, the labels it offers in the
//! projection's order, and the state each label leads to. Until this
//! module that reading was plain Rust (a walk of the branch list and a
//! search of the transitions with a default of state 0).
//!
//! Here it is `role_ops`, proven to give `role_api`: the result has one
//! entry per state of the table, and entry `i` `offers` exactly what the
//! local type of state `i` allows:
//!
//! - its kind and its peer are the local type's;
//! - its operations are the local type's branches, label for label, in
//!   order, none added and none left out;
//! - each operation leads to the state whose local type is the head of
//!   that branch's continuation.
//!
//! `flatten_two_api` and `flatten_n_api` run the flattening and `role_ops`
//! for every role and carry both statements, so that the checker's plain
//! code calls one verified function and no precondition is left to it.
//! What stays outside: giving the labels and roles their names, and
//! writing the Rust text of the role module (`rsk-gen`, `roles.rs`), which
//! the compile-fail fixtures and the skeleton tests exercise.

use vstd::prelude::*;

use crate::multiparty;
use crate::progress::{System, Trans};
use crate::theory::{self, GE, LE, LsE};

#[cfg(verus_keep_ghost)]
use crate::multiparty::{flattens_n, tables_view};
#[cfg(verus_keep_ghost)]
use crate::theory::{flattens, lhead, ls_lookup, ls_of, tables, tbl_view, L, Ls};

verus! {

/// A state of a role in which the Protocol has ended for it.
pub const KIND_END: u8 = 0;
/// A state in which the role sends one of the labels to the peer.
pub const KIND_SEND: u8 = 1;
/// A state in which the role receives one of the labels from the peer.
pub const KIND_RECV: u8 = 2;

/// One operation of a state: the label sent or received and the role's
/// state after it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Op {
    pub label: u32,
    pub to: usize,
}

/// What a role's state offers: its kind, the peer of its operations (0 for
/// an end), and the operations in the order of the projection's branches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct StateOps {
    pub kind: u8,
    pub peer: usize,
    pub ops: Vec<Op>,
}

/// The labels of a branch list, in order.
pub open spec fn ls_labels(ls: Ls) -> Seq<int>
    decreases ls,
{
    match ls {
        Ls::Nil => Seq::empty(),
        Ls::Cons { label, l, rest } => seq![label] + ls_labels(*rest),
    }
}

/// R38.4 for one state: `st` offers exactly the operations the local type
/// `e` allows, each leading to the state of the table `tb` that is the
/// head of its continuation.
pub open spec fn offers(st: StateOps, tb: Seq<L>, e: L) -> bool {
    &&& match e {
        L::Send { to, .. } => st.kind == KIND_SEND && st.peer as int == to,
        L::Recv { from, .. } => st.kind == KIND_RECV && st.peer as int == from,
        _ => st.kind == KIND_END,
    }
    &&& st.ops@.len() == ls_labels(ls_of(e)).len()
    &&& forall|k: int| 0 <= k < st.ops@.len() ==> {
        let op = #[trigger] st.ops@[k];
        &&& op.label as int == ls_labels(ls_of(e))[k]
        &&& (op.to as int) < tb.len()
        &&& ls_lookup(ls_of(e), op.label as int) is Some
        &&& tb[op.to as int] == lhead(ls_lookup(ls_of(e), op.label as int).unwrap())
    }
}

/// R38.4 for a role: one entry per state of its table, each offering what
/// the state's local type allows.
pub open spec fn role_api(out: Seq<StateOps>, tb: Seq<L>) -> bool {
    &&& out.len() == tb.len()
    &&& forall|i: int| 0 <= i < tb.len() ==> offers(#[trigger] out[i], tb, tb[i])
}

/// What `role_ops` needs of the transitions, for the role `r` with the
/// table `tb` (`flattens` and `flattens_n` both give it): every branch of
/// every entry has a transition, and every transition of the role is a
/// branch of its entry that leads to the head of the continuation.
pub open spec fn role_trans_ok(trans: Seq<Trans>, r: usize, tb: Seq<L>) -> bool {
    &&& forall|i: int, label: int| 0 <= i < tb.len() && #[trigger] ls_lookup(ls_of(tb[i]), label) is Some
        ==> exists|k: int| 0 <= k < trans.len() && (#[trigger] trans[k]).role == r && trans[k].from == i as usize && trans[k].label == label as u32
    &&& forall|k: int| 0 <= k < trans.len() && (#[trigger] trans[k]).role == r ==> {
        let t = trans[k];
        &&& (t.from as int) < tb.len()
        &&& (t.to as int) < tb.len()
        &&& ls_lookup(ls_of(tb[t.from as int]), t.label as int) is Some
        &&& tb[t.to as int] == lhead(ls_lookup(ls_of(tb[t.from as int]), t.label as int).unwrap())
    }
}

/// The two-role flattening gives `role_trans_ok` for each role.
pub proof fn lemma_flattens_role(s: System, g: theory::G, tb: Seq<Seq<L>>, r: int)
    requires flattens(s, g, tb), 0 <= r < 2,
    ensures role_trans_ok(s.trans@, r as usize, tb[r]),
{
    assert forall|i: int, label: int| 0 <= i < tb[r].len() && #[trigger] ls_lookup(ls_of(tb[r][i]), label) is Some
        implies exists|k: int| 0 <= k < s.trans@.len() && (#[trigger] s.trans@[k]).role == r as usize && s.trans@[k].from == i as usize && s.trans@[k].label == label as u32 by {
        let j = choose|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(ls_lookup(ls_of(tb[r][i]), label).unwrap())
            && theory::has_transition(s, Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: theory::other(r) as usize, label: label as u32, to: j as usize });
        let t = Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: theory::other(r) as usize, label: label as u32, to: j as usize };
        let k = choose|k: int| 0 <= k < s.trans@.len() && s.trans@[k] == t;
        assert(s.trans@[k].role == r as usize);
    }
}

/// The `n`-role flattening gives `role_trans_ok` for each role.
pub proof fn lemma_flattens_n_role(s: System, g: theory::G, n: int, tb: Seq<Seq<L>>, r: int)
    requires flattens_n(s, g, n, tb), 0 <= r < n,
    ensures role_trans_ok(s.trans@, r as usize, tb[r]),
{
    assert forall|i: int, label: int| 0 <= i < tb[r].len() && #[trigger] ls_lookup(ls_of(tb[r][i]), label) is Some
        implies exists|k: int| 0 <= k < s.trans@.len() && (#[trigger] s.trans@[k]).role == r as usize && s.trans@[k].from == i as usize && s.trans@[k].label == label as u32 by {
        assert(ls_of(tb[r][i]) == multiparty::ls_of(tb[r][i]));
        let j = choose|j: int| 0 <= j < tb[r].len() && tb[r][j] == lhead(ls_lookup(multiparty::ls_of(tb[r][i]), label).unwrap())
            && theory::has_transition(s, Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: multiparty::peer_of(tb[r][i]) as usize, label: label as u32, to: j as usize });
        let t = Trans { role: r as usize, from: i as usize, send: tb[r][i] is Send, peer: multiparty::peer_of(tb[r][i]) as usize, label: label as u32, to: j as usize };
        let k = choose|k: int| 0 <= k < s.trans@.len() && s.trans@[k] == t;
        assert(s.trans@[k].role == r as usize);
    }
    assert forall|k: int| 0 <= k < s.trans@.len() && (#[trigger] s.trans@[k]).role == r as usize implies {
        let t = s.trans@[k];
        &&& (t.from as int) < tb[r].len()
        &&& (t.to as int) < tb[r].len()
        &&& ls_lookup(ls_of(tb[r][t.from as int]), t.label as int) is Some
        &&& tb[r][t.to as int] == lhead(ls_lookup(ls_of(tb[r][t.from as int]), t.label as int).unwrap())
    } by {
        let t = s.trans@[k];
        assert(ls_of(tb[r][t.from as int]) == multiparty::ls_of(tb[r][t.from as int]));
    }
}

/// A label of the list is one the list's lookup finds.
pub proof fn lemma_label_found(ls: Ls, k: int)
    requires 0 <= k < ls_labels(ls).len(),
    ensures ls_lookup(ls, ls_labels(ls)[k]) is Some,
    decreases ls,
{
    match ls {
        Ls::Nil => {}
        Ls::Cons { label, l, rest } => {
            if k > 0 && label != ls_labels(ls)[k] {
                assert(ls_labels(ls)[k] == ls_labels(*rest)[k - 1]);
                lemma_label_found(*rest, k - 1);
            }
        }
    }
}

/// The state a transition of role `r` from state `i` on `label` leads to:
/// the first one in the list, or `None` when the list has none.
fn next_state(trans: &Vec<Trans>, r: usize, i: usize, label: u32) -> (j: Option<usize>)
    ensures
        match j {
            Some(j) => exists|k: int| 0 <= k < trans@.len() && (#[trigger] trans@[k]).role == r && trans@[k].from == i && trans@[k].label == label && trans@[k].to == j,
            None => forall|k: int| 0 <= k < trans@.len() ==> !((#[trigger] trans@[k]).role == r && trans@[k].from == i && trans@[k].label == label),
        },
{
    let mut k: usize = 0;
    while k < trans.len()
        invariant
            k <= trans@.len(),
            forall|m: int| 0 <= m < k ==> !((#[trigger] trans@[m]).role == r && trans@[m].from == i && trans@[m].label == label),
        decreases trans@.len() - k,
    {
        if trans[k].role == r && trans[k].from == i && trans[k].label == label {
            return Some(trans[k].to);
        }
        k += 1;
    }
    None
}

/// The operations of the branch list `list` of state `i` of role `r`, in
/// the list's order (`tbl` is read by the contract alone).
fn ops_of(list: &LsE, _tbl: &Vec<LE>, trans: &Vec<Trans>, r: usize, i: usize) -> (ops: Vec<Op>)
    requires
        i < _tbl@.len(),
        role_trans_ok(trans@, r, tbl_view(_tbl@)),
        list@ == ls_of(_tbl@[i as int]@),
    ensures
        ops@.len() == ls_labels(list@).len(),
        forall|k: int| 0 <= k < ops@.len() ==> {
            let op = #[trigger] ops@[k];
            &&& op.label as int == ls_labels(list@)[k]
            &&& (op.to as int) < tbl_view(_tbl@).len()
            &&& ls_lookup(list@, op.label as int) is Some
            &&& tbl_view(_tbl@)[op.to as int] == lhead(ls_lookup(list@, op.label as int).unwrap())
        },
{
    let ghost tb = tbl_view(_tbl@);
    let ghost e = _tbl@[i as int]@;
    assert(tb[i as int] == e);
    let mut ops: Vec<Op> = Vec::new();
    let mut cur: &LsE = list;
    loop
        invariant
            (i as int) < tb.len(),
            tb == tbl_view(_tbl@),
            tb[i as int] == e,
            list@ == ls_of(e),
            role_trans_ok(trans@, r, tb),
            ls_labels(ls_of(e)) == ops@.map(|k: int, op: Op| op.label as int) + ls_labels(cur@),
            forall|k: int| 0 <= k < ops@.len() ==> {
                let op = #[trigger] ops@[k];
                &&& (op.to as int) < tb.len()
                &&& ls_lookup(ls_of(e), op.label as int) is Some
                &&& tb[op.to as int] == lhead(ls_lookup(ls_of(e), op.label as int).unwrap())
            },
        ensures cur@ is Nil,
        decreases cur,
    {
        match cur {
            LsE::Nil => {
                break;
            }
            LsE::Cons { label, l: _, rest } => {
                let ghost done = ops@.map(|k: int, op: Op| op.label as int);
                proof {
                    // This label is the next of the entry's, so the lookup finds it.
                    assert(ls_labels(cur@) == seq![*label as int] + ls_labels(rest@));
                    assert(ls_labels(ls_of(e))[done.len() as int] == *label as int);
                    lemma_label_found(ls_of(e), done.len() as int);
                }
                let to = match next_state(trans, r, i, *label) {
                    Some(to) => to,
                    None => {
                        proof {
                            // A branch of the entry has a transition (`role_trans_ok`).
                            assert(ls_lookup(ls_of(tb[i as int]), *label as int) is Some);
                            let k = choose|k: int| 0 <= k < trans@.len() && (#[trigger] trans@[k]).role == r && trans@[k].from == (i as int) as usize && trans@[k].label == (*label as int) as u32;
                            assert(trans@[k].role == r && trans@[k].from == i && trans@[k].label == *label);
                            assert(false);
                        }
                        0
                    }
                };
                proof {
                    let k = choose|k: int| 0 <= k < trans@.len() && (#[trigger] trans@[k]).role == r && trans@[k].from == i && trans@[k].label == *label && trans@[k].to == to;
                    assert(trans@[k].role == r);
                }
                ops.push(Op { label: *label, to });
                proof {
                    let now = ops@.map(|k: int, op: Op| op.label as int);
                    assert(now == done + seq![*label as int]);
                    assert(done + seq![*label as int] + ls_labels(rest@) == done + (seq![*label as int] + ls_labels(rest@)));
                }
                cur = &**rest;
            }
        }
    }
    proof {
        assert(ls_labels(cur@) == Seq::<int>::empty());
        let labels = ops@.map(|k: int, op: Op| op.label as int);
        assert(ls_labels(ls_of(e)) == labels);
        assert forall|k: int| 0 <= k < ops@.len() implies (#[trigger] ops@[k]).label as int == ls_labels(ls_of(e))[k] by {
            assert(labels[k] == ops@[k].label as int);
        }
    }
    ops
}

/// The operations of state `i` of role `r`.
fn state_ops(tbl: &Vec<LE>, trans: &Vec<Trans>, r: usize, i: usize) -> (st: StateOps)
    requires
        i < tbl@.len(),
        role_trans_ok(trans@, r, tbl_view(tbl@)),
    ensures offers(st, tbl_view(tbl@), tbl@[i as int]@),
{
    assert(tbl_view(tbl@)[i as int] == tbl@[i as int]@);
    match &tbl[i] {
        LE::Send { to, ls } => {
            let ops = ops_of(ls, tbl, trans, r, i);
            StateOps { kind: KIND_SEND, peer: *to, ops }
        }
        LE::Recv { from, ls } => {
            let ops = ops_of(ls, tbl, trans, r, i);
            StateOps { kind: KIND_RECV, peer: *from, ops }
        }
        _ => StateOps { kind: KIND_END, peer: 0, ops: Vec::new() },
    }
}

/// R38.4: the operations of every state of role `r`, from its verified
/// table and the automaton's transitions.
pub fn role_ops(tbl: &Vec<LE>, trans: &Vec<Trans>, r: usize) -> (out: Vec<StateOps>)
    requires role_trans_ok(trans@, r, tbl_view(tbl@)),
    ensures role_api(out@, tbl_view(tbl@)),
{
    let mut out: Vec<StateOps> = Vec::new();
    let mut i: usize = 0;
    while i < tbl.len()
        invariant
            i <= tbl@.len(),
            out@.len() == i,
            role_trans_ok(trans@, r, tbl_view(tbl@)),
            forall|j: int| 0 <= j < i ==> offers(#[trigger] out@[j], tbl_view(tbl@), tbl_view(tbl@)[j]),
        decreases tbl@.len() - i,
    {
        let st = state_ops(tbl, trans, r, i);
        assert(tbl_view(tbl@)[i as int] == tbl@[i as int]@);
        out.push(st);
        i += 1;
    }
    out
}

/// The two-role flattening with each role's operations: `flattens` of the
/// automaton and some tables (entry 0 of a role the head of its
/// projection), and `role_api` of each role's operations over its table.
pub fn flatten_two_api(g: &GE, capacity: usize) -> (r: Option<(System, Vec<Vec<StateOps>>)>)
    ensures
        r is Some ==> exists|tb: Seq<Seq<L>>| {
            &&& flattens(r.unwrap().0, g@, tb)
            &&& r.unwrap().1@.len() == 2
            &&& forall|u: int| 0 <= u < 2 ==> role_api((#[trigger] r.unwrap().1@[u])@, tb[u])
        },
{
    let Some((sys, t0, t1)) = theory::flatten_two(g, capacity) else {
        return None;
    };
    let ghost tb = tables(t0@, t1@);
    proof {
        lemma_flattens_role(sys, g@, tb, 0);
        lemma_flattens_role(sys, g@, tb, 1);
        assert(tb[0] == tbl_view(t0@));
        assert(tb[1] == tbl_view(t1@));
    }
    let a0 = role_ops(&t0, &sys.trans, 0);
    let a1 = role_ops(&t1, &sys.trans, 1);
    let mut apis: Vec<Vec<StateOps>> = Vec::new();
    apis.push(a0);
    apis.push(a1);
    let r = Some((sys, apis));
    proof {
        assert(flattens(r.unwrap().0, g@, tb) && r.unwrap().1@.len() == 2 && forall|u: int| 0 <= u < 2 ==> role_api((#[trigger] r.unwrap().1@[u])@, tb[u]));
    }
    r
}

/// The `n`-role flattening with each role's operations (`flatten_two_api`
/// for `flattens_n`).
pub fn flatten_n_api(g: &GE, n: usize, capacity: usize) -> (r: Option<(System, Vec<Vec<StateOps>>)>)
    requires n >= 2,
    ensures
        r is Some ==> exists|tb: Seq<Seq<L>>| {
            &&& flattens_n(r.unwrap().0, g@, n as int, tb)
            &&& r.unwrap().1@.len() == n
            &&& forall|u: int| 0 <= u < n ==> role_api((#[trigger] r.unwrap().1@[u])@, tb[u])
        },
{
    let Some((sys, tbls)) = multiparty::flatten_n(g, n, capacity) else {
        return None;
    };
    let ghost tb = tables_view(tbls@);
    assert(tb.len() == n);
    let mut apis: Vec<Vec<StateOps>> = Vec::new();
    let mut u: usize = 0;
    while u < n
        invariant
            u <= n,
            apis@.len() == u,
            tb == tables_view(tbls@),
            tb.len() == n,
            tbls@.len() == n,
            flattens_n(sys, g@, n as int, tb),
            forall|v: int| 0 <= v < u ==> role_api((#[trigger] apis@[v])@, tb[v]),
        decreases n - u,
    {
        proof {
            lemma_flattens_n_role(sys, g@, n as int, tb, u as int);
            assert((u as int) as usize == u);
            assert(tb[u as int] == tbl_view(tbls@[u as int]@));
        }
        let a = role_ops(&tbls[u], &sys.trans, u);
        apis.push(a);
        u += 1;
    }
    let r = Some((sys, apis));
    proof {
        assert(flattens_n(r.unwrap().0, g@, n as int, tb) && r.unwrap().1@.len() == n && forall|u: int| 0 <= u < n ==> role_api((#[trigger] r.unwrap().1@[u])@, tb[u]));
    }
    r
}

} // verus!
