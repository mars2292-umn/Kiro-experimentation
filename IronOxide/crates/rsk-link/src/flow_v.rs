//! The certificate of a function body's control flow and stack heights,
//! verified (R37.1; 2026-11-27).
//!
//! `flow::walk` (not verified) reads a body as the processor can reach it
//! from its first instruction and proposes a stack height for each
//! instruction. `flow_ok` checks the proposal against the body's bytes,
//! and what it accepts satisfies `FlowOk`:
//!
//! - the list is in address order, starts at the body's first address, and
//!   each instruction lies in the body with the length its first halfword
//!   gives;
//! - **closed**: wherever control passes from an instruction of the list
//!   and stays in the body, it passes to an instruction of the list: the
//!   next address unless the instruction stops (`hard`, and not under an
//!   `it`), the target of a branch, and every entry of the table of a
//!   `tbb`/`tbh [pc, Rm]` (the bytes from the instruction to the next one
//!   of the list are all read as entries);
//! - **heights**: the height of the first instruction is 0, and along
//!   every such passage the height of the instruction reached is at least
//!   the height of the one left plus what that one takes from the stack
//!   (an increment is credited only when the instruction is under no `it`
//!   of the list); every height plus the instruction's own decrement is
//!   within the frame.
//!
//! So a path from the first instruction that follows these passages stays
//! on the list and lowers SP by no more than the frame, whether or not it
//! passes an instruction twice (`Tools/LinkFlow.lean`). Until 2026-11-27
//! the body was every halfword in address order (an inline table was
//! decoded as instructions) and the frame model's premise was that no
//! instruction is executed twice.
//!
//! Taken as given: each `Insn` of the list is the decoder's answer for the
//! bytes at its address (`flow::walk` stores `thumb::decode`'s result);
//! the functions named in `noreturn` do not return; a call returns with SP
//! as it was; control that leaves the body is the call graph's.

use crate::thumb::{Effect, Insn};
use vstd::prelude::*;

verus! {

/// The bound on a height's magnitude.
pub const BIG: i64 = 0x100_0000_0000;

/// The halfword at offset `off` of the body, least significant byte first.
pub open spec fn hw_spec(code: Seq<u8>, off: int) -> u32 {
    (code[off] as u32) | ((code[off + 1] as u32) << 8u32)
}

pub fn hw(code: &[u8], off: usize) -> (r: u32)
    requires off + 1 < code@.len(),
    ensures r == hw_spec(code@, off as int),
{
    (code[off] as u32) | ((code[off + 1] as u32) << 8u32)
}

/// The length of the instruction whose first halfword is `h` (A5.1).
pub open spec fn len_spec(h: u32) -> u32 {
    if (h >> 11u32) >= 0x1D { 4 } else { 2 }
}

pub fn len_of(h: u32) -> (r: u32)
    ensures r == len_spec(h),
{
    if (h >> 11u32) >= 0x1D { 4 } else { 2 }
}

/// Instruction `x` lies in the body with the length its first halfword gives.
pub open spec fn placed(code: Seq<u8>, base: u32, x: Insn) -> bool {
    &&& base <= x.address
    &&& (x.len == 2 || x.len == 4)
    &&& x.address as int - base as int + x.len as int <= code.len()
    &&& x.len == len_spec(hw_spec(code, x.address as int - base as int))
}

pub fn placed_check(code: &[u8], base: u32, x: &Insn) -> (r: bool)
    ensures r == placed(code@, base, *x),
{
    if x.address < base {
        return false;
    }
    if x.len != 2 && x.len != 4 {
        return false;
    }
    let off = (x.address - base) as usize;
    if off > code.len() || x.len as usize > code.len() - off {
        return false;
    }
    x.len == len_of(hw(code, off))
}

/// The first and second halfwords of a placed instruction.
pub open spec fn h1s(code: Seq<u8>, base: u32, x: Insn) -> u32 {
    hw_spec(code, x.address as int - base as int)
}

pub open spec fn h2s(code: Seq<u8>, base: u32, x: Insn) -> u32 {
    if x.len == 4 { hw_spec(code, x.address as int - base as int + 2) } else { 0 }
}

pub fn halves(code: &[u8], base: u32, x: &Insn) -> (r: (u32, u32))
    requires placed(code@, base, *x),
    ensures r.0 == h1s(code@, base, *x), r.1 == h2s(code@, base, *x),
{
    let off = (x.address - base) as usize;
    let a = hw(code, off);
    let total = code.len();
    let b = if x.len == 4 && total - off >= 4 { hw(code, off + 2) } else { 0 };
    (a, b)
}

/// `it` (A7.7.38): 1011 1111 firstcond mask, the mask not 0000.
pub open spec fn is_it(x: Insn, h: u32) -> bool {
    x.len == 2 && h & 0xFF00 == 0xBF00 && h & 0xF != 0
}

/// The number of instructions an `it` with first halfword `h` governs.
pub open spec fn it_count(h: u32) -> int {
    if h & 1 != 0 { 4 } else if h & 2 != 0 { 3 } else if h & 4 != 0 { 2 } else { 1 }
}

/// The length of the instruction at address `q`, 2 when `q` has no
/// halfword in the body.
pub open spec fn len_at(code: Seq<u8>, base: u32, q: int) -> int {
    if base <= q && q - base + 1 < code.len() { len_spec(hw_spec(code, q - base)) as int } else { 2 }
}

/// The address of the instruction `m` places after the `it` at `p`.
pub open spec fn it_addr(code: Seq<u8>, base: u32, p: int, m: int) -> int
    decreases m,
{
    if m <= 0 {
        p + 2
    } else {
        let q = it_addr(code, base, p, m - 1);
        q + len_at(code, base, q)
    }
}

/// Instruction `j` of the list is the one `m` places after the `it` that
/// is instruction `p` of the list.
pub open spec fn governs(code: Seq<u8>, base: u32, insns: Seq<Insn>, p: int, m: int, j: int) -> bool {
    &&& 0 <= p < insns.len()
    &&& is_it(insns[p], h1s(code, base, insns[p]))
    &&& 0 <= m < it_count(h1s(code, base, insns[p]))
    &&& 0 <= j < insns.len()
    &&& insns[j].address as int == it_addr(code, base, insns[p].address as int, m)
}

/// Every instruction of the list under an `it` of the list is marked.
pub open spec fn cond_ok(code: Seq<u8>, base: u32, insns: Seq<Insn>, cond: Seq<bool>) -> bool {
    forall|p: int, m: int, j: int| #[trigger] governs(code, base, insns, p, m, j) ==> cond[j]
}

/// Control does not pass from `x` to the next address (when `x` is under
/// no `it`): a return, an indirect branch, a table branch, an unconditional
/// branch (`b` T2: 11100; `b.w` T4: 11110 ... | 10x1), `udf` (1101 1110),
/// or a call of a function that does not return.
pub open spec fn hard(x: Insn, h1: u32, h2: u32, noreturn: Seq<u32>) -> bool {
    match x.effect {
        Effect::Return => true,
        Effect::IndirectBranch(_) => true,
        Effect::SwitchTable => true,
        Effect::Branch(_) => if x.len == 2 { h1 & 0xF800 == 0xE000 } else { h1 & 0xF800 == 0xF000 && h2 & 0xD000 == 0x9000 },
        Effect::Other => x.len == 2 && h1 & 0xFF00 == 0xDE00,
        Effect::Call(c) => noreturn.contains(c),
        _ => false,
    }
}

fn has(v: &Vec<u32>, c: u32) -> (r: bool)
    ensures r == v@.contains(c),
{
    let mut i: usize = 0;
    while i < v.len()
        invariant
            i <= v.len(),
            forall|k: int| 0 <= k < i ==> v@[k] != c,
        decreases v.len() - i,
    {
        if v[i] == c {
            return true;
        }
        i += 1;
    }
    false
}

pub fn hard_of(x: &Insn, h1: u32, h2: u32, noreturn: &Vec<u32>) -> (r: bool)
    ensures r == hard(*x, h1, h2, noreturn@),
{
    match &x.effect {
        Effect::Return => true,
        Effect::IndirectBranch(_) => true,
        Effect::SwitchTable => true,
        Effect::Branch(_) => if x.len == 2 { h1 & 0xF800 == 0xE000 } else { h1 & 0xF800 == 0xF000 && h2 & 0xD000 == 0x9000 },
        Effect::Other => x.len == 2 && h1 & 0xFF00 == 0xDE00,
        Effect::Call(c) => has(noreturn, *c),
        _ => false,
    }
}

/// What instruction `e` adds to the height: its decrement, or minus its
/// increment when it is under no `it` (an instruction under an `it` may
/// not execute).
pub open spec fn delta(e: Effect, c: bool) -> int {
    match e {
        Effect::SpDown(n) => n as int,
        Effect::SpUp(n) => if c { 0 } else { -(n as int) },
        _ => 0,
    }
}

pub fn delta_of(e: &Effect, c: bool) -> (r: i64)
    ensures r as int == delta(*e, c),
{
    match e {
        Effect::SpDown(n) => *n as i64,
        Effect::SpUp(n) => if c { 0 } else { -(*n as i64) },
        _ => 0,
    }
}

/// The address `t` is an instruction of the list whose height is at least `need`.
pub open spec fn lands(insns: Seq<Insn>, d: Seq<i64>, t: int, need: int) -> bool {
    exists|j: int| 0 <= j < insns.len() && (#[trigger] insns[j]).address as int == t && d[j] as int >= need
}

fn lands_check(insns: &Vec<Insn>, d: &Vec<i64>, t: u32, need: i64, hint: usize) -> (r: bool)
    requires d@.len() == insns@.len(),
    ensures r ==> lands(insns@, d@, t as int, need as int),
{
    if hint < insns.len() && insns[hint].address == t && d[hint] >= need {
        return true;
    }
    let mut j: usize = 0;
    while j < insns.len()
        invariant
            j <= insns.len(),
            d@.len() == insns@.len(),
        decreases insns.len() - j,
    {
        if insns[j].address == t && d[j] >= need {
            return true;
        }
        j += 1;
    }
    false
}

/// The bytes of one entry of a table: 2 for `tbh` (bit 4 of the second
/// halfword), 1 for `tbb`.
pub open spec fn tsz(h2: u32) -> int {
    if h2 & 0x10 != 0 { 2 } else { 1 }
}

/// The entry at offset `off`.
pub open spec fn tentry(code: Seq<u8>, off: int, sz: int) -> int {
    if sz == 2 { hw_spec(code, off) as int } else { code[off] as int }
}

/// The address the entry at address `pos` of the table of the instruction
/// at `a` branches to: the instruction's address plus 4 plus twice the
/// entry (A7.7.185).
pub open spec fn ttarget(code: Seq<u8>, base: u32, a: int, sz: int, pos: int) -> int {
    a + 4 + 2 * tentry(code, pos - base, sz)
}

/// The byte that fills a `tbb` table of an odd number of entries to a
/// halfword: the last before `lim`, and no entry (it points below `lim`).
pub open spec fn tpad(code: Seq<u8>, base: u32, a: int, sz: int, lim: int, pos: int) -> bool {
    sz == 1 && pos + 1 == lim && ttarget(code, base, a, sz, pos) < lim
}

/// The entry at `pos` is the padding, or branches to an instruction of the
/// list in the body whose height is at least `need`.
pub open spec fn tslot_ok(code: Seq<u8>, base: u32, insns: Seq<Insn>, d: Seq<i64>, a: int, sz: int, lim: int, pos: int, need: int) -> bool {
    tpad(code, base, a, sz, lim, pos) || (ttarget(code, base, a, sz, pos) < base + code.len() && lands(insns, d, ttarget(code, base, a, sz, pos), need))
}

/// The table of the `tbb`/`tbh [pc, Rm]` that is instruction `i`: the
/// bytes from the instruction to `lim`, every one of them an entry.
pub open spec fn table_ok(code: Seq<u8>, base: u32, insns: Seq<Insn>, d: Seq<i64>, i: int, lim: int) -> bool {
    let x = insns[i];
    let a = x.address as int;
    let sz = tsz(h2s(code, base, x));
    &&& h1s(code, base, x) == 0xE8DF
    &&& x.len == 4
    &&& a + 4 <= lim <= base + code.len()
    &&& (lim - a - 4) % sz == 0
    &&& forall|pos: int| a + 4 <= pos < lim && (pos - a - 4) % sz == 0 ==> #[trigger] tslot_ok(code, base, insns, d, a, sz, lim, pos, d[i] as int)
}

fn table_check(code: &[u8], base: u32, insns: &Vec<Insn>, d: &Vec<i64>, i: usize, lim: u32) -> (r: bool)
    requires
        i < insns@.len(),
        d@.len() == insns@.len(),
        base as int + code@.len() <= u32::MAX,
        placed(code@, base, insns@[i as int]),
    ensures r ==> table_ok(code@, base, insns@, d@, i as int, lim as int),
{
    let x = &insns[i];
    let (h1, h2) = halves(code, base, x);
    if h1 != 0xE8DF || x.len != 4 {
        return false;
    }
    let a = x.address;
    let end: u64 = base as u64 + code.len() as u64;
    if (lim as u64) < a as u64 + 4 || lim as u64 > end {
        return false;
    }
    let wide = h2 & 0x10 != 0;
    let sz: u32 = if wide { 2 } else { 1 };
    if (lim - a - 4) % sz != 0 {
        return false;
    }
    let need = d[i];
    let mut pos: u32 = a + 4;
    while pos < lim
        invariant
            a as int + 4 <= pos <= lim,
            lim as int <= base as int + code@.len(),
            base as int + code@.len() <= u32::MAX,
            end == base as int + code@.len(),
            base <= a,
            sz as int == tsz(h2),
            sz == 1 || sz == 2,
            wide == (sz == 2),
            (lim - a - 4) % (sz as int) == 0,
            (pos - a - 4) % (sz as int) == 0,
            d@.len() == insns@.len(),
            forall|q: int| a as int + 4 <= q < pos && (q - a - 4) % (sz as int) == 0 ==> #[trigger] tslot_ok(code@, base, insns@, d@, a as int, sz as int, lim as int, q, need as int),
        decreases lim - pos,
    {
        let off = (pos - base) as usize;
        let e: u32 = if wide { hw(code, off) } else { code[off] as u32 };
        assert(e as int == tentry(code@, pos as int - base as int, sz as int));
        assert(e <= 0xFFFF) by {
            if wide {
                let lo = code@[off as int];
                let hi = code@[off as int + 1];
                assert(((lo as u32) | ((hi as u32) << 8u32)) <= 0xFFFF) by (bit_vector);
            }
        }
        let t: u64 = a as u64 + 4 + 2 * (e as u64);
        assert(t as int == ttarget(code@, base, a as int, sz as int, pos as int));
        let pad = !wide && pos + 1 == lim && t < lim as u64;
        if !pad {
            if t >= end {
                return false;
            }
            if !lands_check(insns, d, t as u32, need, insns.len()) {
                return false;
            }
        }
        assert(tslot_ok(code@, base, insns@, d@, a as int, sz as int, lim as int, pos as int, need as int));
        pos += sz;
    }
    true
}

/// A call as the last instruction of the body: control that returned
/// from it would leave the body at its end. Taken as not returning (the
/// compiler ends a body so only after a call that diverges).
pub open spec fn ends_in_call(x: Insn, end: int) -> bool {
    x.effect is Call && x.address as int + x.len as int == end
}

/// Instruction `i`: placed, its height within the bounds and the frame,
/// and every passage from it that stays in the body lands on the list
/// with the height it needs.
pub open spec fn node_ok(code: Seq<u8>, base: u32, insns: Seq<Insn>, cond: Seq<bool>, noreturn: Seq<u32>, d: Seq<i64>, frame: u32, i: int) -> bool {
    let x = insns[i];
    let end = base as int + code.len();
    let a = x.address as int;
    let need = d[i] as int + delta(x.effect, cond[i]);
    let lim = if i + 1 < insns.len() { insns[i + 1].address as int } else { end };
    &&& placed(code, base, x)
    &&& -BIG <= d[i] <= BIG
    &&& d[i] as int + crate::bound_v::down(x.effect) <= frame
    &&& (cond[i] || !hard(x, h1s(code, base, x), h2s(code, base, x), noreturn)) ==> (lands(insns, d, a + x.len, need) || ends_in_call(x, end))
    &&& match x.effect {
        Effect::Branch(t) => (base <= t && (t as int) < end) ==> lands(insns, d, t as int, need),
        Effect::CompareBranch(t) => (base <= t && (t as int) < end) ==> lands(insns, d, t as int, need),
        Effect::SwitchTable => table_ok(code, base, insns, d, i, lim),
        _ => true,
    }
}

fn node_check(code: &[u8], base: u32, insns: &Vec<Insn>, cond: &Vec<bool>, noreturn: &Vec<u32>, d: &Vec<i64>, frame: u32, i: usize) -> (r: bool)
    requires
        i < insns@.len(),
        d@.len() == insns@.len(),
        cond@.len() == insns@.len(),
        base as int + code@.len() <= u32::MAX,
    ensures r ==> node_ok(code@, base, insns@, cond@, noreturn@, d@, frame, i as int),
{
    let x = &insns[i];
    if !placed_check(code, base, x) {
        return false;
    }
    if d[i] < -BIG || d[i] > BIG {
        return false;
    }
    let dn: i64 = match &x.effect {
        Effect::SpDown(n) => *n as i64,
        _ => 0,
    };
    if d[i] + dn > frame as i64 {
        return false;
    }
    let end: u64 = base as u64 + code.len() as u64;
    let need: i64 = d[i] + delta_of(&x.effect, cond[i]);
    let (h1, h2) = halves(code, base, x);
    let next: u64 = x.address as u64 + x.len as u64;
    if cond[i] || !hard_of(x, h1, h2, noreturn) {
        let call_at_end = match &x.effect {
            Effect::Call(_) => next == end,
            _ => false,
        };
        if !call_at_end {
            if next > u32::MAX as u64 {
                return false;
            }
            if !lands_check(insns, d, next as u32, need, i + 1) {
                return false;
            }
        }
    }
    let lim: u32 = if i < insns.len() - 1 { insns[i + 1].address } else { end as u32 };
    match &x.effect {
        Effect::Branch(t) => {
            if base <= *t && (*t as u64) < end {
                if !lands_check(insns, d, *t, need, insns.len()) {
                    return false;
                }
            }
            true
        }
        Effect::CompareBranch(t) => {
            if base <= *t && (*t as u64) < end {
                if !lands_check(insns, d, *t, need, insns.len()) {
                    return false;
                }
            }
            true
        }
        Effect::SwitchTable => table_check(code, base, insns, d, i, lim),
        _ => true,
    }
}

/// Every instruction of the list at address `q` is marked.
fn marked(insns: &Vec<Insn>, cond: &Vec<bool>, q: u64) -> (r: bool)
    requires cond@.len() == insns@.len(),
    ensures r == (forall|j: int| 0 <= j < insns@.len() && (#[trigger] insns@[j]).address as int == q as int ==> cond@[j]),
{
    let mut j: usize = 0;
    while j < insns.len()
        invariant
            j <= insns.len(),
            cond@.len() == insns@.len(),
            forall|k: int| 0 <= k < j && (#[trigger] insns@[k]).address as int == q as int ==> cond@[k],
        decreases insns.len() - j,
    {
        if insns[j].address as u64 == q && !cond[j] {
            return false;
        }
        j += 1;
    }
    true
}

/// The certificate's statement.
pub open spec fn flow_spec(code: Seq<u8>, base: u32, insns: Seq<Insn>, cond: Seq<bool>, noreturn: Seq<u32>, d: Seq<i64>, frame: u32) -> bool {
    &&& insns.len() >= 1
    &&& cond.len() == insns.len()
    &&& d.len() == insns.len()
    &&& base as int + code.len() <= u32::MAX
    &&& insns[0].address == base
    &&& d[0] == 0
    &&& forall|i: int| 0 <= i < insns.len() - 1 ==> (#[trigger] insns[i]).address < insns[i + 1].address
    &&& cond_ok(code, base, insns, cond)
    &&& forall|i: int| 0 <= i < insns.len() ==> #[trigger] node_ok(code, base, insns, cond, noreturn, d, frame, i)
}

/// The list is in address order.
fn order_check(insns: &Vec<Insn>) -> (r: bool)
    requires insns@.len() >= 1,
    ensures r ==> forall|i: int| 0 <= i < insns@.len() - 1 ==> (#[trigger] insns@[i]).address < insns@[i + 1].address,
{
    let n = insns.len();
    let mut i: usize = 0;
    while i < n - 1
        invariant
            n == insns@.len(),
            n >= 1,
            i <= n - 1,
            forall|k: int| 0 <= k < i ==> (#[trigger] insns@[k]).address < insns@[k + 1].address,
        decreases n - i,
    {
        if insns[i].address >= insns[i + 1].address {
            return false;
        }
        i += 1;
    }
    true
}

/// Every instruction: placed, and its passages.
fn nodes_check(code: &[u8], base: u32, insns: &Vec<Insn>, cond: &Vec<bool>, noreturn: &Vec<u32>, d: &Vec<i64>, frame: u32) -> (r: bool)
    requires
        d@.len() == insns@.len(),
        cond@.len() == insns@.len(),
        base as int + code@.len() <= u32::MAX,
    ensures r ==> forall|i: int| 0 <= i < insns@.len() ==> #[trigger] node_ok(code@, base, insns@, cond@, noreturn@, d@, frame, i),
{
    let n = insns.len();
    let mut i: usize = 0;
    while i < n
        invariant
            n == insns@.len(),
            cond@.len() == n,
            d@.len() == n,
            base as int + code@.len() <= u32::MAX,
            i <= n,
            forall|k: int| 0 <= k < i ==> #[trigger] node_ok(code@, base, insns@, cond@, noreturn@, d@, frame, k),
        decreases n - i,
    {
        if !node_check(code, base, insns, cond, noreturn, d, frame, i) {
            return false;
        }
        i += 1;
    }
    true
}

/// The instructions under the `it` that is instruction `p` are marked.
fn it_check(code: &[u8], base: u32, insns: &Vec<Insn>, cond: &Vec<bool>, p: usize) -> (r: bool)
    requires
        p < insns@.len(),
        cond@.len() == insns@.len(),
        base as int + code@.len() <= u32::MAX,
        placed(code@, base, insns@[p as int]),
    ensures r ==> forall|m: int, j: int| #[trigger] governs(code@, base, insns@, p as int, m, j) ==> cond@[j],
{
    let x = &insns[p];
    let (h1, _) = halves(code, base, x);
    if !(x.len == 2 && h1 & 0xFF00 == 0xBF00 && h1 & 0xF != 0) {
        return true;
    }
    let count: usize = if h1 & 1 != 0 { 4 } else if h1 & 2 != 0 { 3 } else if h1 & 4 != 0 { 2 } else { 1 };
    let mut q: u64 = x.address as u64 + 2;
    let mut m: usize = 0;
    while m < count
        invariant
            cond@.len() == insns@.len(),
            base as int + code@.len() <= u32::MAX,
            p < insns@.len(),
            m <= count,
            count <= 4,
            count as int == it_count(h1s(code@, base, insns@[p as int])),
            q as int == it_addr(code@, base, insns@[p as int].address as int, m as int),
            q <= insns@[p as int].address as int + 2 + 4 * m,
            forall|mm: int, j: int| mm < m && #[trigger] governs(code@, base, insns@, p as int, mm, j) ==> cond@[j],
        decreases count - m,
    {
        if !marked(insns, cond, q) {
            return false;
        }
        let step: u64 = if q >= base as u64 && q - base as u64 + 1 < code.len() as u64 {
            len_of(hw(code, (q - base as u64) as usize)) as u64
        } else {
            2
        };
        assert(step as int == len_at(code@, base, q as int));
        assert(it_addr(code@, base, insns@[p as int].address as int, m as int + 1) == q as int + step as int);
        q += step;
        m += 1;
    }
    true
}

/// The marks: every instruction under an `it` of the list.
fn marks_check(code: &[u8], base: u32, insns: &Vec<Insn>, cond: &Vec<bool>) -> (r: bool)
    requires
        cond@.len() == insns@.len(),
        base as int + code@.len() <= u32::MAX,
        forall|k: int| 0 <= k < insns@.len() ==> placed(code@, base, #[trigger] insns@[k]),
    ensures r ==> cond_ok(code@, base, insns@, cond@),
{
    let n = insns.len();
    let mut p: usize = 0;
    while p < n
        invariant
            n == insns@.len(),
            cond@.len() == n,
            base as int + code@.len() <= u32::MAX,
            p <= n,
            forall|k: int| 0 <= k < n ==> placed(code@, base, #[trigger] insns@[k]),
            forall|pp: int, m: int, j: int| pp < p && #[trigger] governs(code@, base, insns@, pp, m, j) ==> cond@[j],
        decreases n - p,
    {
        if !it_check(code, base, insns, cond, p) {
            return false;
        }
        p += 1;
    }
    true
}

/// **The check.** `true` only when the list, the marks and the heights
/// satisfy `flow_spec` for the body's bytes.
pub fn flow_ok(code: &[u8], base: u32, insns: &Vec<Insn>, cond: &Vec<bool>, noreturn: &Vec<u32>, d: &Vec<i64>, frame: u32) -> (r: bool)
    ensures r ==> flow_spec(code@, base, insns@, cond@, noreturn@, d@, frame),
{
    let n = insns.len();
    if n < 1 || cond.len() != n || d.len() != n {
        return false;
    }
    if code.len() as u64 > u32::MAX as u64 || base as u64 + code.len() as u64 > u32::MAX as u64 {
        return false;
    }
    if insns[0].address != base || d[0] != 0 {
        return false;
    }
    if !order_check(insns) {
        return false;
    }
    if !nodes_check(code, base, insns, cond, noreturn, d, frame) {
        return false;
    }
    assert forall|k: int| 0 <= k < insns@.len() implies placed(code@, base, #[trigger] insns@[k]) by {
        assert(node_ok(code@, base, insns@, cond@, noreturn@, d@, frame, k));
    }
    marks_check(code, base, insns, cond)
}

} // verus!
