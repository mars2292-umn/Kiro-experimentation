//! The verified parser of the Protocol text's tokens (2026-11-09): the
//! grammar as a reference specification (`sp_block`, the deterministic
//! parse of a token sequence), an executable parser refining it
//! (`parse_toks`), a printer (`print_toks`) whose output the grammar reads
//! back as the printed type (`lemma_parse_printed`), and the round trip
//! (`theorem_round_trip`: printing then parsing gives back the same AST).
//! The lexer (`parse::tokens`, characters to tokens, with keywords
//! classified) remains plain Rust: the one step before the grammar.

use vstd::prelude::*;

use crate::ast::{Global, Message};

verus! {

/// A token, keywords classified by the lexer so that the grammar never
/// compares strings.
pub enum TokE {
    Ident(String),
    /// The payload type text inside a label's parentheses, `None` for a signal.
    Paren(Option<String>),
    Colon,
    Arrow,
    Semi,
    LBrace,
    RBrace,
    KwEnd,
    KwContinue,
    KwRec,
    KwChoice,
    KwAt,
    KwOr,
}

impl TokE {
    pub open spec fn view(&self) -> TokS {
        match self {
            TokE::Ident(s) => TokS::Ident(s@),
            TokE::Paren(s) => TokS::Paren(payload_view(*s)),
            TokE::Colon => TokS::Colon,
            TokE::Arrow => TokS::Arrow,
            TokE::Semi => TokS::Semi,
            TokE::LBrace => TokS::LBrace,
            TokE::RBrace => TokS::RBrace,
            TokE::KwEnd => TokS::KwEnd,
            TokE::KwContinue => TokS::KwContinue,
            TokE::KwRec => TokS::KwRec,
            TokE::KwChoice => TokS::KwChoice,
            TokE::KwAt => TokS::KwAt,
            TokE::KwOr => TokS::KwOr,
        }
    }
}

/// The specification's token.
pub enum TokS {
    Ident(Seq<char>),
    Paren(Option<Seq<char>>),
    Colon,
    Arrow,
    Semi,
    LBrace,
    RBrace,
    KwEnd,
    KwContinue,
    KwRec,
    KwChoice,
    KwAt,
    KwOr,
}

pub open spec fn toks_view(v: Seq<TokE>) -> Seq<TokS> {
    v.map(|i: int, t: TokE| t@)
}

/// The specification's global type (the AST with sequences for lists).
pub enum GS {
    End,
    Msg { label: Seq<char>, payload: Option<Seq<char>>, from: Seq<char>, to: Seq<char>, rest: Box<GS> },
    Choice { at: Seq<char>, branches: Seq<GS> },
    Rec { name: Seq<char>, body: Box<GS> },
    Continue(Seq<char>),
}

pub open spec fn payload_view(p: Option<String>) -> Option<Seq<char>> {
    match p {
        Some(s) => Some(s@),
        None => None,
    }
}

/// The view of the parser's AST.
pub open spec fn gview(g: Global) -> GS
    decreases g, 0int,
{
    match g {
        Global::End => GS::End,
        Global::Msg(m, rest) => GS::Msg { label: m.label@, payload: payload_view(m.payload), from: m.from@, to: m.to@, rest: Box::new(gview(*rest)) },
        Global::Choice { at, branches } => GS::Choice { at: at@, branches: gview_list(branches@, 0) },
        Global::Rec { name, body } => GS::Rec { name: name@, body: Box::new(gview(*body)) },
        Global::Continue(n) => GS::Continue(n@),
    }
}

/// The views of the branches from `i`.
pub open spec fn gview_list(bs: Seq<Global>, i: int) -> Seq<GS>
    decreases bs, bs.len() - i,
{
    if 0 <= i < bs.len() { seq![gview(bs[i])].add(gview_list(bs, i + 1)) } else { Seq::<GS>::empty() }
}

/// The views are the elementwise views.
pub proof fn lemma_gview_list(bs: Seq<Global>, i: int)
    requires 0 <= i <= bs.len(),
    ensures gview_list(bs, i).len() == bs.len() - i, forall|k: int| 0 <= k < bs.len() - i ==> #[trigger] gview_list(bs, i)[k] == gview(bs[i + k]),
    decreases bs.len() - i,
{
    if i < bs.len() {
        lemma_gview_list(bs, i + 1);
        assert forall|k: int| 0 <= k < bs.len() - i implies #[trigger] gview_list(bs, i)[k] == gview(bs[i + k]) by {
            if k > 0 {
                assert(gview_list(bs, i)[k] == gview_list(bs, i + 1)[k - 1]);
            }
        }
    }
}

/// The size of a type: the measure of the printer's recursion.
pub open spec fn gsize(g: GS) -> nat
    decreases g, 0int,
{
    match g {
        GS::End => 1,
        GS::Continue(_) => 1,
        GS::Rec { body, .. } => 1 + gsize(*body),
        GS::Msg { rest, .. } => 1 + gsize(*rest),
        GS::Choice { branches, .. } => 1 + gsize_list(branches, 0),
    }
}

pub open spec fn gsize_list(bs: Seq<GS>, i: int) -> nat
    decreases bs, bs.len() - i,
{
    if 0 <= i < bs.len() { gsize(bs[i]) + gsize_list(bs, i + 1) } else { 0 }
}

proof fn lemma_gsize_branch(bs: Seq<GS>, i: int, k: int)
    requires 0 <= i <= k < bs.len(),
    ensures gsize(bs[k]) <= gsize_list(bs, i),
    decreases k - i,
{
    if i < k {
        lemma_gsize_branch(bs, i + 1, k);
    }
}

// ------------------------------------------------------------- the grammar --

pub open spec fn end_of_block(ts: Seq<TokS>, pos: int) -> bool {
    pos >= ts.len() || ts[pos] is RBrace
}

/// The deterministic parse of a block from `pos`: the type and the
/// position after it, or nothing.
pub open spec fn sp_block(ts: Seq<TokS>, pos: int) -> Option<(GS, int)>
    decreases ts.len() - pos, 1int,
{
    if !(0 <= pos < ts.len()) {
        None
    } else {
        match ts[pos] {
            TokS::KwEnd => {
                let p1 = if pos + 1 < ts.len() && ts[pos + 1] is Semi { pos + 2 } else { pos + 1 };
                if end_of_block(ts, p1) { Some((GS::End, p1)) } else { None }
            }
            TokS::KwContinue => {
                if pos + 1 < ts.len() && ts[pos + 1] is Ident {
                    let name = ts[pos + 1]->Ident_0;
                    let p1 = if pos + 2 < ts.len() && ts[pos + 2] is Semi { pos + 3 } else { pos + 2 };
                    if end_of_block(ts, p1) { Some((GS::Continue(name), p1)) } else { None }
                } else {
                    None
                }
            }
            TokS::KwRec => {
                if pos + 2 < ts.len() && ts[pos + 1] is Ident && ts[pos + 2] is LBrace {
                    let name = ts[pos + 1]->Ident_0;
                    match sp_block(ts, pos + 3) {
                        Some((body, p1)) => {
                            if p1 < ts.len() && ts[p1] is RBrace && end_of_block(ts, p1 + 1) {
                                Some((GS::Rec { name, body: Box::new(body) }, p1 + 1))
                            } else {
                                None
                            }
                        }
                        None => None,
                    }
                } else {
                    None
                }
            }
            TokS::KwChoice => {
                if pos + 2 < ts.len() && ts[pos + 1] is KwAt && ts[pos + 2] is Ident {
                    let at = ts[pos + 2]->Ident_0;
                    match sp_branches(ts, pos + 3) {
                        Some((bs, p1)) => if end_of_block(ts, p1) { Some((GS::Choice { at, branches: bs }, p1)) } else { None },
                        None => None,
                    }
                } else {
                    None
                }
            }
            TokS::Ident(label) => {
                if pos + 6 < ts.len() && ts[pos + 1] is Paren && ts[pos + 2] is Colon && ts[pos + 3] is Ident && ts[pos + 4] is Arrow && ts[pos + 5] is Ident && ts[pos + 6] is Semi {
                    let payload = ts[pos + 1]->Paren_0;
                    let from = ts[pos + 3]->Ident_0;
                    let to = ts[pos + 5]->Ident_0;
                    match sp_block(ts, pos + 7) {
                        Some((rest, p1)) => Some((GS::Msg { label, payload, from, to, rest: Box::new(rest) }, p1)),
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

/// The branches of a choice from `pos`: `{ block }` then, after `or`, more.
pub open spec fn sp_branches(ts: Seq<TokS>, pos: int) -> Option<(Seq<GS>, int)>
    decreases ts.len() - pos, 0int,
{
    if 0 <= pos < ts.len() && ts[pos] is LBrace {
        match sp_block(ts, pos + 1) {
            Some((b, p1)) => {
                if pos < p1 && p1 < ts.len() && ts[p1] is RBrace {
                    if p1 + 1 < ts.len() && ts[p1 + 1] is KwOr {
                        match sp_branches(ts, p1 + 2) {
                            Some((bs, p2)) => Some((seq![b].add(bs), p2)),
                            None => None,
                        }
                    } else {
                        Some((seq![b], p1 + 1))
                    }
                } else {
                    None
                }
            }
            None => None,
        }
    } else {
        None
    }
}

/// The grammar's parse of a whole token sequence.
pub open spec fn sp_parse(ts: Seq<TokS>) -> Option<GS> {
    match sp_block(ts, 0) {
        Some((g, p)) => if p == ts.len() { Some(g) } else { None },
        None => None,
    }
}

// ---------------------------------------------------------- the printer --

/// The canonical tokens of a type (what `render` prints).
pub open spec fn printed(g: GS) -> Seq<TokS>
    decreases g, 0int,
{
    match g {
        GS::End => seq![TokS::KwEnd],
        GS::Continue(n) => seq![TokS::KwContinue, TokS::Ident(n)],
        GS::Rec { name, body } => seq![TokS::KwRec, TokS::Ident(name), TokS::LBrace].add(printed(*body)).push(TokS::RBrace),
        GS::Choice { at, branches } => seq![TokS::KwChoice, TokS::KwAt, TokS::Ident(at)].add(printed_branches(branches, 0)),
        GS::Msg { label, payload, from, to, rest } => {
            seq![TokS::Ident(label), TokS::Paren(payload), TokS::Colon, TokS::Ident(from), TokS::Arrow, TokS::Ident(to), TokS::Semi].add(printed(*rest))
        }
    }
}

/// `{ b_i } or { b_i+1 } ...` from branch `i`.
pub open spec fn printed_branches(bs: Seq<GS>, i: int) -> Seq<TokS>
    decreases bs, bs.len() - i,
{
    if 0 <= i < bs.len() {
        let one = seq![TokS::LBrace].add(printed(bs[i])).push(TokS::RBrace);
        if i + 1 < bs.len() { one.push(TokS::KwOr).add(printed_branches(bs, i + 1)) } else { one }
    } else {
        Seq::<TokS>::empty()
    }
}

// -------------------------------------------------- the executable parser --

/// A token's kind, without its text.
fn is_kind(t: &TokE, k: u8) -> (r: bool)
    ensures r == match k {
        0 => t@ is Colon, 1 => t@ is Arrow, 2 => t@ is Semi, 3 => t@ is LBrace, 4 => t@ is RBrace,
        5 => t@ is KwEnd, 6 => t@ is KwContinue, 7 => t@ is KwRec, 8 => t@ is KwChoice, 9 => t@ is KwAt, 10 => t@ is KwOr,
        11 => t@ is Ident, 12 => t@ is Paren, _ => false,
    },
{
    match (t, k) {
        (TokE::Colon, 0) => true,
        (TokE::Arrow, 1) => true,
        (TokE::Semi, 2) => true,
        (TokE::LBrace, 3) => true,
        (TokE::RBrace, 4) => true,
        (TokE::KwEnd, 5) => true,
        (TokE::KwContinue, 6) => true,
        (TokE::KwRec, 7) => true,
        (TokE::KwChoice, 8) => true,
        (TokE::KwAt, 9) => true,
        (TokE::KwOr, 10) => true,
        (TokE::Ident(_), 11) => true,
        (TokE::Paren(_), 12) => true,
        _ => false,
    }
}

/// The identifier's text (a copy).
fn ident_of(t: &TokE) -> (s: String)
    requires t@ is Ident,
    ensures s@ == t@->Ident_0,
{
    match t {
        TokE::Ident(s) => s.clone(),
        _ => { assert(false); String::new() }
    }
}

fn paren_of(t: &TokE) -> (s: Option<String>)
    requires t@ is Paren,
    ensures payload_view(s) == t@->Paren_0,
{
    match t {
        TokE::Paren(Some(s)) => Some(s.clone()),
        TokE::Paren(None) => None,
        _ => { assert(false); None }
    }
}

fn at_kind(ts: &Vec<TokE>, pos: usize, k: u8) -> (r: bool)
    ensures r == (pos < ts.len() && match k {
        0 => toks_view(ts@)[pos as int] is Colon, 1 => toks_view(ts@)[pos as int] is Arrow, 2 => toks_view(ts@)[pos as int] is Semi,
        3 => toks_view(ts@)[pos as int] is LBrace, 4 => toks_view(ts@)[pos as int] is RBrace, 5 => toks_view(ts@)[pos as int] is KwEnd,
        6 => toks_view(ts@)[pos as int] is KwContinue, 7 => toks_view(ts@)[pos as int] is KwRec, 8 => toks_view(ts@)[pos as int] is KwChoice,
        9 => toks_view(ts@)[pos as int] is KwAt, 10 => toks_view(ts@)[pos as int] is KwOr, 11 => toks_view(ts@)[pos as int] is Ident,
        12 => toks_view(ts@)[pos as int] is Paren, _ => false,
    }),
{
    if pos < ts.len() {
        is_kind(&ts[pos], k)
    } else {
        false
    }
}

pub open spec fn eob(ts: Seq<TokE>, pos: int) -> bool {
    end_of_block(toks_view(ts), pos)
}

fn end_of_block_exec(ts: &Vec<TokE>, pos: usize) -> (r: bool)
    ensures r == eob(ts@, pos as int),
{
    pos >= ts.len() || at_kind(ts, pos, 4)
}

/// The executable parse of a block from `*pos`: refines `sp_block`, the
/// position advanced to the one after the block.
pub fn parse_block(ts: &Vec<TokE>, pos: &mut usize) -> (r: Option<Global>)
    ensures
        r is Some ==> sp_block(toks_view(ts@), *old(pos) as int) == Some((gview(r.unwrap()), *final(pos) as int)) && *final(pos) > *old(pos) && *final(pos) <= ts.len(),
        r is None ==> sp_block(toks_view(ts@), *old(pos) as int) is None,
    decreases ts.len() - *old(pos), 1int,
{
    let ghost tv = toks_view(ts@);
    assert(tv.len() == ts.len());
    if *pos >= ts.len() {
        return None;
    }
    let p0 = *pos;
    if at_kind(ts, p0, 5) {
        // end [;]
        assert(tv[p0 as int] is KwEnd);
        let p1 = if at_kind(ts, p0 + 1, 2) { p0 + 2 } else { p0 + 1 };
        if end_of_block_exec(ts, p1) {
            *pos = p1;
            assert(sp_block(tv, p0 as int) == Some((GS::End, p1 as int)));
            Some(Global::End)
        } else {
            assert(sp_block(tv, p0 as int) is None);
            None
        }
    } else if at_kind(ts, p0, 6) {
        // continue NAME [;]
        assert(tv[p0 as int] is KwContinue);
        if at_kind(ts, p0 + 1, 11) {
            let name = ident_of(&ts[p0 + 1]);
            let p1 = if at_kind(ts, p0 + 2, 2) { p0 + 3 } else { p0 + 2 };
            if end_of_block_exec(ts, p1) {
                *pos = p1;
                assert(sp_block(tv, p0 as int) == Some((GS::Continue(name@), p1 as int)));
                Some(Global::Continue(name))
            } else {
                assert(sp_block(tv, p0 as int) is None);
                None
            }
        } else {
            assert(sp_block(tv, p0 as int) is None);
            None
        }
    } else if at_kind(ts, p0, 7) {
        // rec NAME { block }
        assert(tv[p0 as int] is KwRec);
        if at_kind(ts, p0 + 1, 11) && at_kind(ts, p0 + 2, 3) {
            let name = ident_of(&ts[p0 + 1]);
            let mut p = p0 + 3;
            match parse_block(ts, &mut p) {
                Some(body) => {
                    assert(sp_block(tv, p0 as int + 3) == Some((gview(body), p as int)));
                    if at_kind(ts, p, 4) && end_of_block_exec(ts, p + 1) {
                        *pos = p + 1;
                        assert(sp_block(tv, p0 as int) == Some((GS::Rec { name: name@, body: Box::new(gview(body)) }, p as int + 1)));
                        Some(Global::Rec { name, body: Box::new(body) })
                    } else {
                        assert(sp_block(tv, p0 as int) is None);
                        None
                    }
                }
                None => {
                    assert(sp_block(tv, p0 as int + 3) is None);
                    assert(sp_block(tv, p0 as int) is None);
                    None
                }
            }
        } else {
            assert(sp_block(tv, p0 as int) is None);
            None
        }
    } else if at_kind(ts, p0, 8) {
        // choice at NAME { block } or { block } ...
        assert(tv[p0 as int] is KwChoice);
        if at_kind(ts, p0 + 1, 9) && at_kind(ts, p0 + 2, 11) {
            let at = ident_of(&ts[p0 + 2]);
            let mut p = p0 + 3;
            match parse_branches(ts, &mut p) {
                Some(bs) => {
                    assert(sp_branches(tv, p0 as int + 3) == Some((gview_list(bs@, 0), p as int)));
                    if end_of_block_exec(ts, p) {
                        *pos = p;
                        assert(sp_block(tv, p0 as int) == Some((GS::Choice { at: at@, branches: gview_list(bs@, 0) }, p as int)));
                        Some(Global::Choice { at, branches: bs })
                    } else {
                        assert(sp_block(tv, p0 as int) is None);
                        None
                    }
                }
                None => {
                    assert(sp_branches(tv, p0 as int + 3) is None);
                    assert(sp_block(tv, p0 as int) is None);
                    None
                }
            }
        } else {
            assert(sp_block(tv, p0 as int) is None);
            None
        }
    } else if at_kind(ts, p0, 11) {
        // LABEL(PAYLOAD): FROM -> TO; block
        assert(tv[p0 as int] is Ident);
        if at_kind(ts, p0 + 1, 12) && at_kind(ts, p0 + 2, 0) && at_kind(ts, p0 + 3, 11) && at_kind(ts, p0 + 4, 1) && at_kind(ts, p0 + 5, 11) && at_kind(ts, p0 + 6, 2) {
            let label = ident_of(&ts[p0]);
            let payload = paren_of(&ts[p0 + 1]);
            let from = ident_of(&ts[p0 + 3]);
            let to = ident_of(&ts[p0 + 5]);
            let mut p = p0 + 7;
            match parse_block(ts, &mut p) {
                Some(rest) => {
                    assert(sp_block(tv, p0 as int + 7) == Some((gview(rest), p as int)));
                    *pos = p;
                    assert(sp_block(tv, p0 as int) == Some((GS::Msg { label: label@, payload: payload_view(payload), from: from@, to: to@, rest: Box::new(gview(rest)) }, p as int)));
                    Some(Global::Msg(Message { label, payload, from, to }, Box::new(rest)))
                }
                None => {
                    assert(sp_block(tv, p0 as int + 7) is None);
                    assert(sp_block(tv, p0 as int) is None);
                    None
                }
            }
        } else {
            assert(sp_block(tv, p0 as int) is None);
            None
        }
    } else {
        assert(sp_block(tv, p0 as int) is None);
        None
    }
}

/// The executable parse of a choice's branches: refines `sp_branches`.
fn parse_branches(ts: &Vec<TokE>, pos: &mut usize) -> (r: Option<Vec<Global>>)
    ensures
        r is Some ==> sp_branches(toks_view(ts@), *old(pos) as int) == Some((gview_list(r.unwrap()@, 0), *final(pos) as int)) && *final(pos) > *old(pos) && *final(pos) <= ts.len(),
        r is None ==> sp_branches(toks_view(ts@), *old(pos) as int) is None,
    decreases ts.len() - *old(pos), 0int,
{
    let ghost tv = toks_view(ts@);
    assert(tv.len() == ts.len());
    let p0 = *pos;
    if !at_kind(ts, p0, 3) {
        assert(sp_branches(tv, p0 as int) is None);
        return None;
    }
    assert(tv[p0 as int] is LBrace);
    let mut p = p0 + 1;
    let b = match parse_block(ts, &mut p) {
        Some(b) => b,
        None => {
            assert(sp_block(tv, p0 as int + 1) is None);
            assert(sp_branches(tv, p0 as int) is None);
            return None;
        }
    };
    assert(sp_block(tv, p0 as int + 1) == Some((gview(b), p as int)));
    if !at_kind(ts, p, 4) {
        assert(sp_branches(tv, p0 as int) is None);
        return None;
    }
    if at_kind(ts, p + 1, 10) {
        let mut p2 = p + 2;
        match parse_branches(ts, &mut p2) {
            Some(mut bs) => {
                assert(sp_branches(tv, p as int + 2) == Some((gview_list(bs@, 0), p2 as int)));
                let ghost rest = bs@;
                bs.insert(0, b);
                proof {
                    lemma_gview_list(rest, 0);
                    lemma_gview_list(bs@, 0);
                    assert(bs@[0] == b);
                    assert forall|k: int| 0 <= k < rest.len() implies bs@[k + 1] == rest[k] by {}
                    assert(gview_list(bs@, 0) =~= seq![gview(b)].add(gview_list(rest, 0)));
                    assert(sp_branches(tv, p0 as int) == Some((seq![gview(b)].add(gview_list(rest, 0)), p2 as int)));
                }
                *pos = p2;
                Some(bs)
            }
            None => {
                assert(sp_branches(tv, p as int + 2) is None);
                assert(sp_branches(tv, p0 as int) is None);
                None
            }
        }
    } else {
        let mut bs: Vec<Global> = Vec::new();
        bs.push(b);
        proof {
            lemma_gview_list(bs@, 0);
            assert(gview_list(bs@, 0) =~= seq![gview(b)]);
            assert(sp_branches(tv, p0 as int) == Some((seq![gview(b)], p as int + 1)));
        }
        *pos = p + 1;
        Some(bs)
    }
}

/// The parse of a whole token sequence: refines `sp_parse`.
pub fn parse_toks(ts: &Vec<TokE>) -> (r: Option<Global>)
    ensures
        r is Some ==> sp_parse(toks_view(ts@)) == Some(gview(r.unwrap())),
        r is None ==> sp_parse(toks_view(ts@)) is None,
{
    let mut pos: usize = 0;
    match parse_block(ts, &mut pos) {
        Some(g) => if pos == ts.len() { Some(g) } else { None },
        None => None,
    }
}

// ---------------------------------------------------- the printer --

fn push_tok(out: &mut Vec<TokE>, t: TokE)
    ensures toks_view(final(out)@) == toks_view(old(out)@).push(t@),
{
    out.push(t);
    assert(toks_view(out@) =~= toks_view(old(out)@).push(t@));
}

/// The canonical tokens of a type (refines `printed`).
pub fn print_toks(g: &Global, out: &mut Vec<TokE>)
    ensures toks_view(final(out)@) == toks_view(old(out)@).add(printed(gview(*g))),
    decreases gsize(gview(*g)),
{
    match g {
        Global::End => {
            push_tok(out, TokE::KwEnd);
            assert(toks_view(out@) =~= toks_view(old(out)@).add(printed(gview(*g))));
        }
        Global::Continue(n) => {
            push_tok(out, TokE::KwContinue);
            push_tok(out, TokE::Ident(n.clone()));
            assert(toks_view(out@) =~= toks_view(old(out)@).add(printed(gview(*g))));
        }
        Global::Rec { name, body } => {
            push_tok(out, TokE::KwRec);
            push_tok(out, TokE::Ident(name.clone()));
            push_tok(out, TokE::LBrace);
            print_toks(body, out);
            push_tok(out, TokE::RBrace);
            assert(toks_view(out@) =~= toks_view(old(out)@).add(printed(gview(*g))));
        }
        Global::Choice { at, branches } => {
            push_tok(out, TokE::KwChoice);
            push_tok(out, TokE::KwAt);
            push_tok(out, TokE::Ident(at.clone()));
            let ghost head = toks_view(out@);
            let ghost bsv = gview_list(branches@, 0);
            proof { lemma_gview_list(branches@, 0); }
            let mut i: usize = 0;
            while i < branches.len()
                invariant
                    i <= branches.len(),
                    bsv == gview_list(branches@, 0), bsv.len() == branches.len(),
                    forall|k: int| 0 <= k < branches.len() ==> #[trigger] bsv[k] == gview(branches@[k]),
                    gview(*g) == (GS::Choice { at: at@, branches: bsv }),
                    toks_view(out@) == head.add(printed_prefix(bsv, i as int)),
                decreases branches.len() - i,
            {
                let ghost before = toks_view(out@);
                push_tok(out, TokE::LBrace);
                proof {
                    lemma_gsize_branch(bsv, 0, i as int);
                    assert(gsize(gview(branches@[i as int])) < gsize(gview(*g)));
                }
                print_toks(&branches[i], out);
                push_tok(out, TokE::RBrace);
                if i + 1 < branches.len() {
                    push_tok(out, TokE::KwOr);
                }
                proof {
                    assert(bsv[i as int] == gview(branches@[i as int]));
                    lemma_printed_prefix_step(bsv, i as int);
                    assert(toks_view(out@) =~= head.add(printed_prefix(bsv, i as int + 1)));
                }
                i += 1;
            }
            proof {
                lemma_printed_prefix_all(bsv);
                assert(toks_view(out@) =~= toks_view(old(out)@).add(printed(gview(*g))));
            }
        }
        Global::Msg(m, rest) => {
            push_tok(out, TokE::Ident(m.label.clone()));
            let p: Option<String> = match &m.payload { Some(s) => Some(s.clone()), None => None };
            push_tok(out, TokE::Paren(p));
            push_tok(out, TokE::Colon);
            push_tok(out, TokE::Ident(m.from.clone()));
            push_tok(out, TokE::Arrow);
            push_tok(out, TokE::Ident(m.to.clone()));
            push_tok(out, TokE::Semi);
            print_toks(rest, out);
            assert(toks_view(out@) =~= toks_view(old(out)@).add(printed(gview(*g))));
        }
    }
}

/// The tokens of the first `i` branches (the loop's invariant).
pub open spec fn printed_prefix(bs: Seq<GS>, i: int) -> Seq<TokS>
    decreases i,
{
    if i <= 0 || i > bs.len() {
        Seq::<TokS>::empty()
    } else {
        let one = seq![TokS::LBrace].add(printed(bs[i - 1])).push(TokS::RBrace);
        let one_or = if i < bs.len() { one.push(TokS::KwOr) } else { one };
        printed_prefix(bs, i - 1).add(one_or)
    }
}

proof fn lemma_printed_prefix_step(bs: Seq<GS>, i: int)
    requires 0 <= i < bs.len(),
    ensures printed_prefix(bs, i + 1) == printed_prefix(bs, i).add(
        if i + 1 < bs.len() { seq![TokS::LBrace].add(printed(bs[i])).push(TokS::RBrace).push(TokS::KwOr) } else { seq![TokS::LBrace].add(printed(bs[i])).push(TokS::RBrace) }),
{
}

/// The whole prefix is the branches' tokens.
proof fn lemma_printed_prefix_all(bs: Seq<GS>)
    ensures printed_prefix(bs, bs.len() as int) == printed_branches(bs, 0),
{
    lemma_printed_prefix_from(bs, 0);
}

proof fn lemma_printed_prefix_from(bs: Seq<GS>, i: int)
    requires 0 <= i <= bs.len(),
    ensures printed_prefix(bs, bs.len() as int) == printed_prefix(bs, i).add(printed_branches(bs, i)),
    decreases bs.len() - i,
{
    if i == bs.len() {
        assert(printed_branches(bs, i) =~= Seq::<TokS>::empty());
        assert(printed_prefix(bs, i).add(Seq::<TokS>::empty()) =~= printed_prefix(bs, i));
    } else {
        lemma_printed_prefix_from(bs, i + 1);
        lemma_printed_prefix_step(bs, i);
        assert(printed_prefix(bs, i).add(printed_branches(bs, i)) =~= printed_prefix(bs, i + 1).add(printed_branches(bs, i + 1)));
    }
}

/// The shape the grammar produces: every choice has a branch.
pub open spec fn gs_ok(g: GS) -> bool
    decreases g, 0int,
{
    match g {
        GS::Choice { branches, .. } => branches.len() >= 1 && gs_ok_list(branches, 0),
        GS::Rec { body, .. } => gs_ok(*body),
        GS::Msg { rest, .. } => gs_ok(*rest),
        _ => true,
    }
}

pub open spec fn gs_ok_list(bs: Seq<GS>, i: int) -> bool
    decreases bs, bs.len() - i,
{
    if 0 <= i < bs.len() { gs_ok(bs[i]) && gs_ok_list(bs, i + 1) } else { true }
}

// ------------------------------------------------------ the round trip --

/// An index into a subrange is the shifted index.
proof fn lemma_sub_index(ts: Seq<TokS>, pos: int, n: int, k: int)
    requires 0 <= pos, pos + n <= ts.len(), 0 <= k < n,
    ensures ts.subrange(pos, pos + n)[k] == ts[pos + k],
{
}

/// The tokens of a part of a printed type sit in the whole.
proof fn lemma_sub_part(ts: Seq<TokS>, pos: int, whole: Seq<TokS>, off: int, part: Seq<TokS>)
    requires
        0 <= pos, pos + whole.len() <= ts.len(), ts.subrange(pos, pos + whole.len()) == whole,
        0 <= off, off + part.len() <= whole.len(), forall|k: int| 0 <= k < part.len() ==> #[trigger] whole[off + k] == part[k],
    ensures ts.subrange(pos + off, pos + off + part.len()) == part,
{
    assert forall|k: int| 0 <= k < part.len() implies #[trigger] ts.subrange(pos + off, pos + off + part.len())[k] == part[k] by {
        lemma_sub_index(ts, pos, whole.len() as int, off + k);
    }
    assert(ts.subrange(pos + off, pos + off + part.len()) =~= part);
}

/// The grammar reads a printed type back, in any context that closes it
/// (the end of the tokens or a closing brace), as the type itself.
proof fn lemma_parse_printed(ts: Seq<TokS>, pos: int, g: GS)
    requires
        gs_ok(g), 0 <= pos, pos + printed(g).len() <= ts.len(),
        ts.subrange(pos, pos + printed(g).len()) == printed(g),
        end_of_block(ts, pos + printed(g).len()),
    ensures sp_block(ts, pos) == Some((g, pos + printed(g).len())),
    decreases g, 0int,
{
    let n = printed(g).len();
    let pg = printed(g);
    assert forall|k: int| 0 <= k < n implies #[trigger] ts[pos + k] == pg[k] by {
        lemma_sub_index(ts, pos, n as int, k);
    }
    match g {
        GS::End => {
            assert(ts[pos + 0] == pg[0]);
            assert(ts[pos] == TokS::KwEnd);
            if pos + 1 < ts.len() && ts[pos + 1] is Semi {
                // A semicolon after `end` closes nothing: the context is a brace or the end.
                assert(false);
            }
            assert(sp_block(ts, pos) == Some((g, pos + n)));
        }
        GS::Continue(name) => {
            assert(ts[pos + 0] == pg[0]);
            assert(ts[pos] == TokS::KwContinue);
            assert(ts[pos + 1] == TokS::Ident(name));
            if pos + 2 < ts.len() && ts[pos + 2] is Semi {
                assert(false);
            }
            assert(sp_block(ts, pos) == Some((g, pos + n)));
        }
        GS::Rec { name, body } => {
            let pb = printed(*body);
            assert(ts[pos + 0] == pg[0]);
            assert(ts[pos] == TokS::KwRec);
            assert(ts[pos + 1] == TokS::Ident(name));
            assert(ts[pos + 2] == TokS::LBrace);
            assert forall|k: int| 0 <= k < pb.len() implies #[trigger] pg[3 + k] == pb[k] by {}
            lemma_sub_part(ts, pos, pg, 3, pb);
            assert(ts[pos + (3 + pb.len())] == pg[(3 + pb.len()) as int]);
            assert(ts[pos + 3 + pb.len()] == TokS::RBrace);
            lemma_parse_printed(ts, pos + 3, *body);
            assert(sp_block(ts, pos) == Some((g, pos + n)));
        }
        GS::Choice { at, branches } => {
            assert(ts[pos + 0] == pg[0]);
            assert(ts[pos] == TokS::KwChoice);
            assert(ts[pos + 1] == TokS::KwAt);
            assert(ts[pos + 2] == TokS::Ident(at));
            let pbs = printed_branches(branches, 0);
            assert forall|k: int| 0 <= k < pbs.len() implies #[trigger] pg[3 + k] == pbs[k] by {}
            lemma_sub_part(ts, pos, pg, 3, pbs);
            lemma_parse_printed_branches(ts, pos + 3, branches, 0);
            assert(branches.subrange(0, branches.len() as int) =~= branches);
            assert(sp_block(ts, pos) == Some((g, pos + n)));
        }
        GS::Msg { label, payload, from, to, rest } => {
            let pr = printed(*rest);
            assert(ts[pos + 0] == pg[0]);
            assert(ts[pos] == TokS::Ident(label));
            assert(ts[pos + 1] == TokS::Paren(payload));
            assert(ts[pos + 2] == TokS::Colon);
            assert(ts[pos + 3] == TokS::Ident(from));
            assert(ts[pos + 4] == TokS::Arrow);
            assert(ts[pos + 5] == TokS::Ident(to));
            assert(ts[pos + 6] == TokS::Semi);
            assert forall|k: int| 0 <= k < pr.len() implies #[trigger] pg[7 + k] == pr[k] by {}
            lemma_sub_part(ts, pos, pg, 7, pr);
            lemma_parse_printed(ts, pos + 7, *rest);
            assert(sp_block(ts, pos) == Some((g, pos + n)));
        }
    }
}

proof fn lemma_parse_printed_branches(ts: Seq<TokS>, pos: int, bs: Seq<GS>, i: int)
    requires
        gs_ok_list(bs, i), 0 <= i < bs.len(), 0 <= pos, pos + printed_branches(bs, i).len() <= ts.len(),
        ts.subrange(pos, pos + printed_branches(bs, i).len()) == printed_branches(bs, i),
        end_of_block(ts, pos + printed_branches(bs, i).len()),
    ensures sp_branches(ts, pos) == Some((bs.subrange(i, bs.len() as int), pos + printed_branches(bs, i).len())),
    decreases bs, bs.len() - i,
{
    let pb = printed(bs[i]);
    let whole = printed_branches(bs, i);
    let n = whole.len();
    assert forall|k: int| 0 <= k < n implies #[trigger] ts[pos + k] == whole[k] by {
        lemma_sub_index(ts, pos, n as int, k);
    }
    assert(ts[pos + 0] == whole[0]);
    assert(ts[pos] == TokS::LBrace);
    assert forall|k: int| 0 <= k < pb.len() implies #[trigger] whole[1 + k] == pb[k] by {}
    lemma_sub_part(ts, pos, whole, 1, pb);
    assert(ts[pos + (1 + pb.len())] == whole[(1 + pb.len()) as int]);
    assert(ts[pos + 1 + pb.len()] == TokS::RBrace);
    lemma_parse_printed(ts, pos + 1, bs[i]);
    let p1 = pos + 1 + pb.len();
    if i + 1 < bs.len() {
        assert(ts[pos + (pb.len() + 2)] == whole[(pb.len() + 2) as int]);
        assert(ts[p1 + 1] == TokS::KwOr);
        let rest = printed_branches(bs, i + 1);
        assert forall|k: int| 0 <= k < rest.len() implies #[trigger] whole[pb.len() + 3 + k] == rest[k] by {}
        lemma_sub_part(ts, pos, whole, pb.len() as int + 3, rest);
        lemma_parse_printed_branches(ts, p1 + 2, bs, i + 1);
        assert(seq![bs[i]].add(bs.subrange(i + 1, bs.len() as int)) =~= bs.subrange(i, bs.len() as int));
    } else {
        assert(seq![bs[i]] =~= bs.subrange(i, bs.len() as int));
    }
}

/// Printing then parsing gives back the same type.
pub proof fn theorem_round_trip(g: GS)
    requires gs_ok(g),
    ensures sp_parse(printed(g)) == Some(g),
{
    let ts = printed(g);
    assert(ts.subrange(0, ts.len() as int) =~= ts);
    lemma_parse_printed(ts, 0, g);
}

/// The executable round trip: `parse_toks` of `print_toks` of a type is a
/// type with the same view.
pub fn round_trip(g: &Global) -> (r: Option<Global>)
    requires gs_ok(gview(*g)),
    ensures r is Some && gview(r.unwrap()) == gview(*g),
{
    let mut out: Vec<TokE> = Vec::new();
    print_toks(g, &mut out);
    proof {
        assert(toks_view(out@) =~= printed(gview(*g)));
        theorem_round_trip(gview(*g));
    }
    let r = parse_toks(&out);
    proof {
        if r is None {
            assert(false);
        }
    }
    r
}

} // verus!
