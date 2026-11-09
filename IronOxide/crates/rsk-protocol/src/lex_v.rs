//! The verified lexer of the Protocol text (2026-11-09, the step before the
//! grammar of `parse_v`): the tokens a character sequence denotes as a
//! specification (`sp_lex`: identifiers with the keywords classified, the
//! payload text between a label's parentheses with its spaces normalized,
//! the punctuation), the executable lexer refining it (`lex_chars`), and
//! the composition with the grammar (`sp_parse_text`, `parse_chars`,
//! `parse_text`): the type the text denotes is the one `parse` returns.
//! The one unverified step left is `chars_of`, the standard library's
//! decoding of the `str` into its characters (an `external_body` with the
//! view as its contract, registered in `docs/proof_assumptions.toml`).
//!
//! Identifiers are ASCII (`_`, letters, then digits too), as the generated
//! role modules need them; the whitespace is Unicode's (`char::is_whitespace`).

use vstd::prelude::*;
#[cfg(verus_keep_ghost)]
use vstd::std_specs::char::is_white_space;

use crate::ast::Global;
use crate::parse_v::{parse_toks, TokE};
#[cfg(verus_keep_ghost)]
use crate::parse_v::{gview, sp_parse, toks_view, TokS, GS};

verus! {

// ------------------------------------------------------- character classes --

pub open spec fn ident_start(c: char) -> bool {
    c == '_' || (97 <= c as u32 && c as u32 <= 122) || (65 <= c as u32 && c as u32 <= 90)
}

pub open spec fn ident_char(c: char) -> bool {
    ident_start(c) || (48 <= c as u32 && c as u32 <= 57)
}

pub open spec fn opens(c: char) -> bool {
    c == '(' || c == '<' || c == '['
}

pub open spec fn closes(c: char) -> bool {
    c == ')' || c == '>' || c == ']'
}

fn ident_start_x(c: char) -> (b: bool)
    ensures b == ident_start(c),
{
    let u = c as u32;
    c == '_' || (97 <= u && u <= 122) || (65 <= u && u <= 90)
}

fn ident_char_x(c: char) -> (b: bool)
    ensures b == ident_char(c),
{
    let u = c as u32;
    ident_start_x(c) || (48 <= u && u <= 57)
}

fn opens_x(c: char) -> (b: bool)
    ensures b == opens(c),
{
    c == '(' || c == '<' || c == '['
}

fn closes_x(c: char) -> (b: bool)
    ensures b == closes(c),
{
    c == ')' || c == '>' || c == ']'
}

// ----------------------------------------------------------- the lexemes --

/// The end of the identifier run from `i`.
pub open spec fn ident_end(cs: Seq<char>, i: int) -> int
    decreases cs.len() - i,
{
    if 0 <= i < cs.len() && ident_char(cs[i]) { ident_end(cs, i + 1) } else { i }
}

pub proof fn lemma_ident_end_bounds(cs: Seq<char>, i: int)
    requires 0 <= i <= cs.len(),
    ensures
        i <= ident_end(cs, i) <= cs.len(),
        i < cs.len() && ident_char(cs[i]) ==> ident_end(cs, i) > i,
    decreases cs.len() - i,
{
    if i < cs.len() && ident_char(cs[i]) {
        lemma_ident_end_bounds(cs, i + 1);
    }
}

/// The position after the parenthesis closing a payload opened before
/// `i` (`depth` the parentheses open so far; `(`, `<`, `[` open and `)`,
/// `>`, `]` close, so that a generic type's brackets balance), or nothing
/// when the text ends first.
pub open spec fn paren_end(cs: Seq<char>, i: int, depth: int) -> Option<int>
    decreases cs.len() - i,
{
    if depth == 0 {
        Some(i)
    } else if !(0 <= i < cs.len()) {
        None
    } else {
        let c = cs[i];
        let d = if opens(c) { depth + 1 } else if closes(c) { depth - 1 } else { depth };
        paren_end(cs, i + 1, d)
    }
}

pub proof fn lemma_paren_end_bounds(cs: Seq<char>, i: int, depth: int)
    requires 0 <= i <= cs.len(),
    ensures
        paren_end(cs, i, depth) is Some ==> i <= paren_end(cs, i, depth).unwrap() <= cs.len(),
        paren_end(cs, i, depth) is Some && depth != 0 ==> i < paren_end(cs, i, depth).unwrap(),
    decreases cs.len() - i,
{
    if depth != 0 && i < cs.len() {
        let c = cs[i];
        let d = if opens(c) { depth + 1 } else if closes(c) { depth - 1 } else { depth };
        lemma_paren_end_bounds(cs, i + 1, d);
    }
}

/// The keyword a word is, else the identifier.
pub open spec fn kw(s: Seq<char>) -> TokS {
    if s == seq!['e', 'n', 'd'] {
        TokS::KwEnd
    } else if s == seq!['c', 'o', 'n', 't', 'i', 'n', 'u', 'e'] {
        TokS::KwContinue
    } else if s == seq!['r', 'e', 'c'] {
        TokS::KwRec
    } else if s == seq!['c', 'h', 'o', 'i', 'c', 'e'] {
        TokS::KwChoice
    } else if s == seq!['a', 't'] {
        TokS::KwAt
    } else if s == seq!['o', 'r'] {
        TokS::KwOr
    } else {
        TokS::Ident(s)
    }
}

// ------------------------------------------- the payload text normalized --

/// Runs of whitespace become one space, none at either end: `out` the text
/// so far, `pend` whether a space is owed before the next character.
pub open spec fn collapse(s: Seq<char>, i: int, out: Seq<char>, pend: bool) -> Seq<char>
    decreases s.len() - i,
{
    if !(0 <= i < s.len()) {
        out
    } else if is_white_space(s[i]) {
        collapse(s, i + 1, out, out.len() > 0)
    } else {
        collapse(s, i + 1, if pend { out.push(' ').push(s[i]) } else { out.push(s[i]) }, false)
    }
}

/// The spaces beside a path separator `::` are dropped (`app :: Fix` reads
/// as `app::Fix`).
pub open spec fn strip(t: Seq<char>, i: int, out: Seq<char>) -> Seq<char>
    decreases t.len() - i,
{
    if !(0 <= i < t.len()) {
        out
    } else if i + 1 < t.len() && t[i] == ':' && t[i + 1] == ':' {
        let o = if out.len() > 0 && out.last() == ' ' { out.drop_last() } else { out };
        let j = if i + 2 < t.len() && t[i + 2] == ' ' { i + 3 } else { i + 2 };
        strip(t, j, o.push(':').push(':'))
    } else {
        strip(t, i + 1, out.push(t[i]))
    }
}

pub open spec fn normalize(s: Seq<char>) -> Seq<char> {
    strip(collapse(s, 0, Seq::<char>::empty(), false), 0, Seq::<char>::empty())
}

// ------------------------------------------------------------ the lexer --

pub open spec fn cons(t: TokS, r: Option<Seq<TokS>>) -> Option<Seq<TokS>> {
    match r {
        Some(r) => Some(seq![t].add(r)),
        None => None,
    }
}

/// The tokens of the text from `i`, or nothing when a character fits no
/// token (a `-` not followed by `>`, an unclosed payload, anything else).
pub open spec fn sp_lex_from(cs: Seq<char>, i: int) -> Option<Seq<TokS>>
    decreases cs.len() - i,
    via lex_decreases
{
    if !(0 <= i < cs.len()) {
        Some(Seq::<TokS>::empty())
    } else {
        let c = cs[i];
        if is_white_space(c) {
            sp_lex_from(cs, i + 1)
        } else if ident_start(c) {
            let j = ident_end(cs, i);
            cons(kw(cs.subrange(i, j)), sp_lex_from(cs, j))
        } else if c == '(' {
            match paren_end(cs, i + 1, 1) {
                Some(j) => {
                    let inner = normalize(cs.subrange(i + 1, j - 1));
                    cons(TokS::Paren(if inner.len() == 0 { None } else { Some(inner) }), sp_lex_from(cs, j))
                }
                None => None,
            }
        } else if c == ':' {
            cons(TokS::Colon, sp_lex_from(cs, i + 1))
        } else if c == '-' {
            if i + 1 < cs.len() && cs[i + 1] == '>' { cons(TokS::Arrow, sp_lex_from(cs, i + 2)) } else { None }
        } else if c == ';' {
            cons(TokS::Semi, sp_lex_from(cs, i + 1))
        } else if c == '{' {
            cons(TokS::LBrace, sp_lex_from(cs, i + 1))
        } else if c == '}' {
            cons(TokS::RBrace, sp_lex_from(cs, i + 1))
        } else {
            None
        }
    }
}

#[via_fn]
proof fn lex_decreases(cs: Seq<char>, i: int) {
    if 0 <= i < cs.len() {
        lemma_ident_end_bounds(cs, i);
        lemma_paren_end_bounds(cs, i + 1, 1);
    }
}

/// The tokens a text denotes.
pub open spec fn sp_lex(cs: Seq<char>) -> Option<Seq<TokS>> {
    sp_lex_from(cs, 0)
}

/// The type a text denotes: its tokens read by the grammar.
pub open spec fn sp_parse_text(cs: Seq<char>) -> Option<GS> {
    match sp_lex(cs) {
        Some(ts) => sp_parse(ts),
        None => None,
    }
}

pub open spec fn cons_all(ts: Seq<TokS>, r: Option<Seq<TokS>>) -> Option<Seq<TokS>> {
    match r {
        Some(r) => Some(ts.add(r)),
        None => None,
    }
}

fn ident_end_x(cs: &Vec<char>, i: usize) -> (j: usize)
    requires i <= cs.len(),
    ensures j == ident_end(cs@, i as int), i <= j <= cs.len(),
{
    let mut j = i;
    while j < cs.len() && ident_char_x(cs[j])
        invariant i <= j <= cs.len(), ident_end(cs@, i as int) == ident_end(cs@, j as int),
        decreases cs.len() - j,
    {
        j += 1;
    }
    j
}

fn paren_end_x(cs: &Vec<char>, start: usize) -> (r: Option<usize>)
    requires start <= cs.len(),
    ensures
        r is Some ==> paren_end(cs@, start as int, 1) == Some(r.unwrap() as int) && start < r.unwrap() <= cs.len(),
        r is None ==> paren_end(cs@, start as int, 1) is None,
{
    let mut j = start;
    // The depth is counted in a type the text's length cannot overflow.
    let mut depth: u128 = 1;
    while j < cs.len() && depth > 0
        invariant
            start <= j <= cs.len(),
            depth <= 1 + (j - start),
            j == start ==> depth == 1,
            paren_end(cs@, start as int, 1) == paren_end(cs@, j as int, depth as int),
        decreases cs.len() - j,
    {
        let c = cs[j];
        if opens_x(c) {
            depth += 1;
        } else if closes_x(c) {
            depth -= 1;
        }
        j += 1;
    }
    if depth == 0 { Some(j) } else { None }
}

/// The word as a token: a keyword, else the identifier (`s` the word).
fn classify_ident(v: &Vec<char>, s: String) -> (t: TokE)
    requires s@ == v@,
    ensures t@ == kw(v@),
{
    let n = v.len();
    if n == 3 && v[0] == 'e' && v[1] == 'n' && v[2] == 'd' {
        proof { assert(v@ =~= seq!['e', 'n', 'd']); }
        TokE::KwEnd
    } else if n == 8 && v[0] == 'c' && v[1] == 'o' && v[2] == 'n' && v[3] == 't' && v[4] == 'i' && v[5] == 'n' && v[6] == 'u' && v[7] == 'e' {
        proof { assert(v@ =~= seq!['c', 'o', 'n', 't', 'i', 'n', 'u', 'e']); }
        TokE::KwContinue
    } else if n == 3 && v[0] == 'r' && v[1] == 'e' && v[2] == 'c' {
        proof { assert(v@ =~= seq!['r', 'e', 'c']); }
        TokE::KwRec
    } else if n == 6 && v[0] == 'c' && v[1] == 'h' && v[2] == 'o' && v[3] == 'i' && v[4] == 'c' && v[5] == 'e' {
        proof { assert(v@ =~= seq!['c', 'h', 'o', 'i', 'c', 'e']); }
        TokE::KwChoice
    } else if n == 2 && v[0] == 'a' && v[1] == 't' {
        proof { assert(v@ =~= seq!['a', 't']); }
        TokE::KwAt
    } else if n == 2 && v[0] == 'o' && v[1] == 'r' {
        proof { assert(v@ =~= seq!['o', 'r']); }
        TokE::KwOr
    } else {
        TokE::Ident(s)
    }
}

/// The characters `cs[a..b]` as a vector.
fn slice_of(cs: &Vec<char>, a: usize, b: usize) -> (v: Vec<char>)
    requires a <= b <= cs.len(),
    ensures v@ == cs@.subrange(a as int, b as int),
{
    let mut v: Vec<char> = Vec::new();
    let mut k = a;
    while k < b
        invariant a <= k <= b <= cs.len(), v@ == cs@.subrange(a as int, k as int),
        decreases b - k,
    {
        v.push(cs[k]);
        k += 1;
        proof { assert(v@ =~= cs@.subrange(a as int, k as int)); }
    }
    v
}

/// The normalized payload text.
fn normalize_x(s: &Vec<char>) -> (r: String)
    ensures r@ == normalize(s@),
{
    let mut t: Vec<char> = Vec::new();
    let mut pend = false;
    let mut i: usize = 0;
    while i < s.len()
        invariant i <= s.len(), collapse(s@, 0, Seq::<char>::empty(), false) == collapse(s@, i as int, t@, pend),
        decreases s.len() - i,
    {
        let ghost i0 = i as int;
        let ghost t_old = t@;
        let ghost pend_old = pend;
        let c = s[i];
        if c.is_whitespace() {
            pend = t.len() > 0;
        } else {
            if pend {
                t.push(' ');
            }
            t.push(c);
            pend = false;
        }
        i += 1;
        proof {
            assert(collapse(s@, i0, t_old, pend_old) == collapse(s@, i0 + 1, t@, pend));
        }
    }
    let mut out = String::new();
    let mut j: usize = 0;
    while j < t.len()
        invariant j <= t.len(), strip(t@, 0, Seq::<char>::empty()) == strip(t@, j as int, out@),
        decreases t.len() - j,
    {
        let ghost j0 = j as int;
        let ghost before = out@;
        if j + 1 < t.len() && t[j] == ':' && t[j + 1] == ':' {
            let _dropped = if !out.is_empty() {
                match out.pop() {
                    Some(' ') => true,
                    Some(c) => { out.push(c); false }
                    None => false,
                }
            } else {
                false
            };
            proof {
                if before.len() > 0 && before.last() == ' ' {
                    assert(_dropped && out@ == before.drop_last());
                } else {
                    assert(!_dropped);
                    assert(out@ =~= before);
                }
            }
            out.push(':');
            out.push(':');
            if j + 2 < t.len() && t[j + 2] == ' ' { j += 3; } else { j += 2; }
            proof { assert(strip(t@, j0, before) == strip(t@, j as int, out@)); }
        } else {
            out.push(t[j]);
            j += 1;
            proof { assert(strip(t@, j0, before) == strip(t@, j as int, out@)); }
        }
    }
    out
}

/// The tokens of a text, refining `sp_lex`.
pub fn lex_chars(cs: &Vec<char>) -> (r: Option<Vec<TokE>>)
    ensures
        r is Some ==> sp_lex(cs@) == Some(toks_view(r.unwrap()@)),
        r is None ==> sp_lex(cs@) is None,
{
    let mut out: Vec<TokE> = Vec::new();
    let mut i: usize = 0;
    while i < cs.len()
        invariant i <= cs.len(), sp_lex_from(cs@, 0) == cons_all(toks_view(out@), sp_lex_from(cs@, i as int)),
        decreases cs.len() - i,
    {
        let ghost before = toks_view(out@);
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if ident_start_x(c) {
            let j = ident_end_x(cs, i);
            proof { lemma_ident_end_bounds(cs@, i as int); }
            let word = slice_of(cs, i, j);
            let mut s = String::new();
            let mut k: usize = 0;
            while k < word.len()
                invariant k <= word.len(), s@ == word@.subrange(0, k as int),
                decreases word.len() - k,
            {
                s.push(word[k]);
                k += 1;
                proof { assert(s@ =~= word@.subrange(0, k as int)); }
            }
            proof { assert(s@ =~= word@); }
            let t = classify_ident(&word, s);
            out.push(t);
            proof { assert(toks_view(out@) =~= before.push(t@)); lemma_cons_all(before, t@, sp_lex_from(cs@, j as int)); }
            i = j;
        } else if c == '(' {
            match paren_end_x(cs, i + 1) {
                Some(j) => {
                    let inner = slice_of(cs, i + 1, j - 1);
                    let norm = normalize_x(&inner);
                    let t = if norm.is_empty() { TokE::Paren(None) } else { TokE::Paren(Some(norm)) };
                    out.push(t);
                    proof { assert(toks_view(out@) =~= before.push(t@)); lemma_cons_all(before, t@, sp_lex_from(cs@, j as int)); }
                    i = j;
                }
                None => {
                    proof { assert(sp_lex_from(cs@, i as int) is None); }
                    return None;
                }
            }
        } else if c == ':' {
            out.push(TokE::Colon);
            proof { assert(toks_view(out@) =~= before.push(TokS::Colon)); lemma_cons_all(before, TokS::Colon, sp_lex_from(cs@, i + 1)); }
            i += 1;
        } else if c == '-' {
            if i + 1 < cs.len() && cs[i + 1] == '>' {
                out.push(TokE::Arrow);
                proof { assert(toks_view(out@) =~= before.push(TokS::Arrow)); lemma_cons_all(before, TokS::Arrow, sp_lex_from(cs@, i + 2)); }
                i += 2;
            } else {
                proof { assert(sp_lex_from(cs@, i as int) is None); }
                return None;
            }
        } else if c == ';' {
            out.push(TokE::Semi);
            proof { assert(toks_view(out@) =~= before.push(TokS::Semi)); lemma_cons_all(before, TokS::Semi, sp_lex_from(cs@, i + 1)); }
            i += 1;
        } else if c == '{' {
            out.push(TokE::LBrace);
            proof { assert(toks_view(out@) =~= before.push(TokS::LBrace)); lemma_cons_all(before, TokS::LBrace, sp_lex_from(cs@, i + 1)); }
            i += 1;
        } else if c == '}' {
            out.push(TokE::RBrace);
            proof { assert(toks_view(out@) =~= before.push(TokS::RBrace)); lemma_cons_all(before, TokS::RBrace, sp_lex_from(cs@, i + 1)); }
            i += 1;
        } else {
            proof { assert(sp_lex_from(cs@, i as int) is None); }
            return None;
        }
    }
    proof { assert(toks_view(out@).add(Seq::<TokS>::empty()) =~= toks_view(out@)); }
    Some(out)
}

/// `ts ++ (t :: r)` is `(ts ++ [t]) ++ r`.
proof fn lemma_cons_all(ts: Seq<TokS>, t: TokS, r: Option<Seq<TokS>>)
    ensures cons_all(ts, cons(t, r)) == cons_all(ts.push(t), r),
{
    if let Some(r) = r {
        assert(ts.add(seq![t].add(r)) =~= ts.push(t).add(r));
    }
}

/// The type the characters denote, refining `sp_parse_text`.
pub fn parse_chars(cs: &Vec<char>) -> (r: Option<Global>)
    ensures
        r is Some ==> sp_parse_text(cs@) == Some(gview(r.unwrap())),
        r is None ==> sp_parse_text(cs@) is None,
{
    match lex_chars(cs) {
        Some(ts) => parse_toks(&ts),
        None => None,
    }
}

/// The standard library's decoding of the text into its characters: the
/// one step outside the proof (A-P2 of the parser; `docs/proof_assumptions.toml`).
#[verifier::external_body]
pub fn chars_of(text: &str) -> (v: Vec<char>)
    ensures v@ == text@,
{
    text.chars().collect()
}

/// The type the text denotes.
pub fn parse_text(text: &str) -> (r: Option<Global>)
    ensures
        r is Some ==> sp_parse_text(text@) == Some(gview(r.unwrap())),
        r is None ==> sp_parse_text(text@) is None,
{
    let cs = chars_of(text);
    parse_chars(&cs)
}

} // verus!
