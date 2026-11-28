//! A function body as the processor can reach it, and the stack heights
//! of its instructions (R37.1; 2026-11-27).
//!
//! Until 2026-11-27 a body was every halfword of the symbol in address
//! order (`thumb::decode_function`). The table a `tbb`/`tbh` branches
//! through lies in the body after the instruction, and its bytes were
//! decoded as instructions: in the PACEMAKER's `therapy::event` the table
//! `02 b5 02 b5` was read as two `push {r1, lr}`. That only added to the
//! frame, but a table byte of 0xE8 or more in the high half of a halfword
//! starts a 32-bit instruction, which can take the first halfword of the
//! code after the table with it, so that a real stack-lowering instruction
//! there is not read.
//!
//! `body` follows control from the first instruction instead: the next
//! address unless the instruction stops, the targets of branches, the
//! entries of tables. It then proposes a height for every instruction (the
//! longest path of decrements from the entry, an increment credited only
//! outside an `it` block) and has the proposal checked by the verified
//! `flow_v::flow_ok`. Nothing here is trusted for the verdict: a body
//! whose walk or heights are wrong fails the check, and a function without
//! the certificate has no stack bound. What `flow_ok` takes as given is
//! listed in its module.

use std::collections::{BTreeMap, BTreeSet};

use crate::flow_v;
use crate::thumb::{self, Effect, Insn};

/// A body, its `it` marks and its heights, with the check's verdict.
#[derive(Debug, Clone)]
pub struct Body {
    /// The instructions reached, in address order.
    pub insns: Vec<Insn>,
    /// Whether each is under an `it` of the list.
    pub cond: Vec<bool>,
    /// The height proposed for each (empty when none was found).
    pub heights: Vec<i64>,
    /// `flow_v::flow_ok`'s answer for the above against the sum of the
    /// list's decrements.
    pub certified: bool,
    /// Why there is no certificate, as far as the walk can tell (for the
    /// report; the verdict is `certified`).
    pub why: Option<String>,
}

fn halfword(code: &[u8], base: u32, a: u32) -> Option<u32> {
    let off = a.checked_sub(base)? as usize;
    if off.checked_add(1)? < code.len() {
        Some(flow_v::hw(code, off))
    } else {
        None
    }
}

/// The addresses a table at `start` (entries of `sz` bytes, of the
/// instruction at `a`) names, read up to `limit`, the lowest target read
/// so far, or the first entry that is no forward target in the body.
/// Returns the targets and the address after the last entry read.
fn read_table(code: &[u8], base: u32, end: u32, a: u32, sz: u32, mut limit: u32) -> (Vec<u32>, u32) {
    let mut pos = a + 4;
    let mut targets = Vec::new();
    while pos + sz <= limit {
        let off = (pos - base) as usize;
        let e = if sz == 2 { flow_v::hw(code, off) } else { code[off] as u32 };
        let t = a as u64 + 4 + 2 * e as u64;
        if t < (pos + sz) as u64 || t >= end as u64 {
            break;
        }
        let t = t as u32;
        targets.push(t);
        if t < limit {
            limit = t;
        }
        pos += sz;
    }
    (targets, pos)
}

/// One walk from the entry. `hints` are addresses known to be code from an
/// earlier round: a table does not extend over one. Returns the
/// instructions by address, the addresses under an `it`, and the tables
/// read as `(first entry, end of the entries read)`.
#[allow(clippy::type_complexity)]
fn walk_once(code: &[u8], base: u32, literal: &dyn Fn(u32) -> Option<u32>, noreturn: &Vec<u32>, hints: &BTreeSet<u32>) -> (BTreeMap<u32, (Insn, bool)>, BTreeSet<u32>, Vec<(u32, u32)>) {
    let end = base + code.len() as u32;
    let mut seen: BTreeMap<u32, (Insn, bool)> = BTreeMap::new();
    let mut covered: BTreeSet<u32> = BTreeSet::new();
    let mut tables = Vec::new();
    let mut work = vec![base];
    while let Some(addr) = work.pop() {
        if addr < base || addr >= end || end - addr < 2 || seen.contains_key(&addr) {
            continue;
        }
        let Some(h1) = halfword(code, base, addr) else { continue };
        let Some(insn) = thumb::decode(code, base, addr, literal) else { continue };
        let h2 = if insn.len == 4 { halfword(code, base, addr + 2).unwrap_or(0) } else { 0 };
        if insn.len == 2 && h1 & 0xFF00 == 0xBF00 && h1 & 0xF != 0 {
            let count = if h1 & 1 != 0 { 4 } else if h1 & 2 != 0 { 3 } else if h1 & 4 != 0 { 2 } else { 1 };
            let mut q = addr + 2;
            for _ in 0..count {
                covered.insert(q);
                // An instruction walked before its `it` was: it does not stop after all.
                if let Some((x, true)) = seen.get(&q) {
                    work.push(q + x.len);
                }
                q += match halfword(code, base, q) {
                    Some(h) => flow_v::len_of(h),
                    None => 2,
                };
            }
        }
        match &insn.effect {
            Effect::Branch(t) | Effect::CompareBranch(t) => work.push(*t),
            Effect::SwitchTable if h1 == 0xE8DF => {
                let sz = if h2 & 0x10 != 0 { 2 } else { 1 };
                let limit = hints.range(addr + 4..).next().copied().unwrap_or(end).min(end);
                let (targets, stop) = read_table(code, base, end, addr, sz, limit);
                work.extend(targets);
                tables.push((addr + 4, stop));
            }
            _ => {}
        }
        let hard = flow_v::hard_of(&insn, h1, h2, noreturn);
        if !hard || covered.contains(&addr) {
            work.push(addr + insn.len);
        }
        seen.insert(addr, (insn, hard));
    }
    (seen, covered, tables)
}

/// The body of the function at `base` with the bytes `code`.
pub fn body(code: &[u8], base: u32, literal: &dyn Fn(u32) -> Option<u32>, noreturn: &Vec<u32>) -> Body {
    if code.len() < 2 || base as u64 + code.len() as u64 > u32::MAX as u64 {
        return Body { insns: vec![], cond: vec![], heights: vec![], certified: false, why: Some("no instruction in the body".to_string()) };
    }
    let end = base + code.len() as u32;
    // A table read over an address another path reaches as code was read
    // too far: walk again with that address as its limit.
    let mut hints: BTreeSet<u32> = BTreeSet::new();
    let (mut seen, mut covered, mut tables) = walk_once(code, base, literal, noreturn, &hints);
    for _ in 0..64 {
        let over: Vec<u32> = tables.iter().filter_map(|(start, stop)| seen.range(*start..*stop).next().map(|(a, _)| *a)).collect();
        if over.is_empty() {
            break;
        }
        hints.extend(over);
        (seen, covered, tables) = walk_once(code, base, literal, noreturn, &hints);
    }
    let mut insns = Vec::with_capacity(seen.len());
    let mut cond = Vec::with_capacity(seen.len());
    let mut hard = Vec::with_capacity(seen.len());
    for (a, (insn, h)) in seen {
        cond.push(covered.contains(&a));
        hard.push(h);
        insns.push(insn);
    }
    let n = insns.len();
    let index: BTreeMap<u32, usize> = insns.iter().enumerate().map(|(i, x)| (x.address, i)).collect();
    // The passages `flow_v::node_ok` asks for, as edges with what the
    // instruction left adds to the height.
    let mut notes: Vec<String> = Vec::new();
    let mut edges: Vec<(usize, usize, i64)> = Vec::new();
    for (i, x) in insns.iter().enumerate() {
        let w = flow_v::delta_of(&x.effect, cond[i]);
        let pass = |t: u64, what: &str, edges: &mut Vec<(usize, usize, i64)>, notes: &mut Vec<String>| match u32::try_from(t).ok().and_then(|t| index.get(&t)) {
            Some(&j) => edges.push((i, j, w)),
            None => notes.push(format!("{what} of the instruction at {:#010x} goes to {t:#010x}, which is not an instruction the walk reached", x.address)),
        };
        let next = x.address as u64 + x.len as u64;
        if (cond[i] || !hard[i]) && !(matches!(x.effect, Effect::Call(_)) && next == end as u64) {
            pass(next, "the next address", &mut edges, &mut notes);
        }
        match &x.effect {
            Effect::Branch(t) | Effect::CompareBranch(t) if *t >= base && *t < end => pass(*t as u64, "the branch", &mut edges, &mut notes),
            Effect::SwitchTable => {
                let lim = if i + 1 < n { insns[i + 1].address } else { end };
                let (h1, h2) = (halfword(code, base, x.address).unwrap_or(0), halfword(code, base, x.address + 2).unwrap_or(0));
                let sz = if h2 & 0x10 != 0 { 2u32 } else { 1 };
                if h1 != 0xE8DF {
                    notes.push(format!("the table branch at {:#010x} is not through a table at the PC", x.address));
                } else if lim < x.address + 4 || (lim - x.address - 4) % sz != 0 {
                    notes.push(format!("the table of the branch at {:#010x} does not end at the next instruction", x.address));
                } else {
                    let mut pos = x.address + 4;
                    while pos < lim {
                        let off = (pos - base) as usize;
                        let e = if sz == 2 { flow_v::hw(code, off) } else { code[off] as u32 };
                        let t = x.address as u64 + 4 + 2 * e as u64;
                        if !(sz == 1 && pos + 1 == lim && t < lim as u64) {
                            if t >= end as u64 {
                                notes.push(format!("an entry of the table of the branch at {:#010x} goes to {t:#010x}, outside the body", x.address));
                            } else {
                                pass(t, "a table entry", &mut edges, &mut notes);
                            }
                        }
                        pos += sz;
                    }
                }
            }
            _ => {}
        }
    }
    // The longest path of weights from the entry; every other instruction
    // starts from a floor no path reaches.
    const FLOOR: i64 = -(1 << 36);
    let mut d = vec![FLOOR; n];
    d[0] = 0;
    let mut rounds = 0usize;
    loop {
        let mut changed = false;
        for &(i, j, w) in &edges {
            if d[i] + w > d[j] {
                d[j] = d[i] + w;
                changed = true;
            }
        }
        if !changed {
            break;
        }
        rounds += 1;
        if rounds > n + 1 {
            notes.push("a loop of the body lowers SP more than it raises it: no height fits".to_string());
            break;
        }
    }
    let frame = crate::bound_v::frame_of(&insns);
    let certified = flow_v::flow_ok(code, base, &insns, &cond, noreturn, &d, frame);
    let why = if certified { None } else { Some(notes.into_iter().next().unwrap_or_else(|| "the walk's heights do not pass the check".to_string())) };
    Body { insns, cond, heights: d, certified, why }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn none(_: u32) -> Option<u32> {
        None
    }

    fn of(words: &[u16]) -> Body {
        let code: Vec<u8> = words.iter().flat_map(|w| w.to_le_bytes()).collect();
        body(&code, 0x1000, &none, &vec![])
    }

    /// A prologue, a body and a return: one height per instruction.
    #[test]
    fn straight_line_body_is_certified() {
        // push {r4, lr}; sub sp, #8; add sp, #8; pop {r4, pc}
        let b = of(&[0xB510, 0xB082, 0xB002, 0xBD10]);
        assert!(b.certified, "{:?}", b.why);
        assert_eq!(b.heights, vec![0, 8, 16, 8]);
    }

    /// The unwind stub's shape: a loop that lowers SP and raises it by the
    /// same amount each time round has a certificate.
    #[test]
    fn a_balanced_loop_is_certified() {
        // sub sp, #16; add sp, #16; cbz r0, +0 (to the return); b back; bx lr
        let b = of(&[0xB084, 0xB004, 0xB100, 0xE7FB, 0x4770]);
        assert!(b.certified, "{:?}", b.why);
    }

    /// A loop that lowers SP each time round has none.
    #[test]
    fn a_loop_that_lowers_sp_is_refused() {
        // sub sp, #16; cbz r0, +0; b back; bx lr
        let b = of(&[0xB084, 0xB100, 0xE7FC, 0x4770]);
        assert!(!b.certified);
        assert!(b.why.unwrap().contains("lowers SP more than it raises it"));
    }

    /// An increment under an `it` is not credited: the loop below raises
    /// SP only when the condition holds.
    #[test]
    fn a_conditional_increment_does_not_balance_a_loop() {
        // sub sp, #16; it eq; addeq sp, #16; cbz r0, +0; b back; bx lr
        let b = of(&[0xB084, 0xBF08, 0xB004, 0xB100, 0xE7FA, 0x4770]);
        assert_eq!(b.cond, vec![false, false, true, false, false, false]);
        assert!(!b.certified);
    }

    /// The table of a `tbb` is not decoded: the PACEMAKER's `02 b5 02 b5`
    /// (two `push {r1, lr}` when read as instructions).
    #[test]
    fn an_inline_table_is_data() {
        // tbb [pc, r1]; .byte 2, 3, 2, 3; bx lr; bx lr
        let b = of(&[0xE8DF, 0xF001, 0x0302, 0x0302, 0x4770, 0x4770]);
        assert!(b.certified, "{:?}", b.why);
        let at: Vec<u32> = b.insns.iter().map(|x| x.address).collect();
        assert_eq!(at, vec![0x1000, 0x1008, 0x100A]);
        assert_eq!(crate::bound_v::frame_of(&b.insns), 0);
        let old = thumb::decode_function(&[0xDF, 0xE8, 0x01, 0xF0, 0x02, 0xB5, 0x02, 0xB5, 0x70, 0x47], 0x1000, &none);
        assert_eq!(crate::bound_v::frame_of(&old), 16);
    }

    /// A table of an odd number of entries is filled to a halfword.
    #[test]
    fn a_table_of_three_entries_and_its_padding() {
        // tbb [pc, r1]; .byte 2, 3, 2, pad 0; bx lr; bx lr
        let b = of(&[0xE8DF, 0xF001, 0x0302, 0x0002, 0x4770, 0x4770]);
        assert!(b.certified, "{:?}", b.why);
    }

    /// A branch to an address the walk cannot decode is refused by the check.
    #[test]
    fn a_branch_into_the_middle_of_nothing_is_refused() {
        // b +0x7fe (in a body of 6 bytes: outside, so no edge); then a body whose
        // table names an address past its end.
        let b = of(&[0xE8DF, 0xF001, 0x7F7F, 0x4770]);
        assert!(!b.certified);
    }

    /// Code after a call of a function that does not return is not reached.
    #[test]
    fn a_call_that_does_not_return_stops_the_walk() {
        // bl 0x2000; .word (data)
        let code: Vec<u8> = [0xF000u16, 0xFFFE, 0xB5FF, 0xB5FF].iter().flat_map(|w| w.to_le_bytes()).collect();
        let b = body(&code, 0x1000, &none, &vec![0x2000]);
        assert!(b.certified, "{:?}", b.why);
        assert_eq!(b.insns.len(), 1);
        let b = body(&code, 0x1000, &none, &vec![]);
        assert_eq!(b.insns.len(), 3);
    }

    /// A survey over built binaries (`RSK_FLOW_ELFS`, colon-separated): the
    /// functions without a certificate and the frames that differ from the
    /// walk in address order. Not a check; run by hand.
    #[test]
    #[ignore]
    fn survey() {
        let list = std::env::var("RSK_FLOW_ELFS").unwrap_or_default();
        for path in list.split(':').filter(|p| !p.is_empty()) {
            let elf = rsk_elf::Elf::parse(std::fs::read(path).unwrap()).unwrap();
            let literal = |a: u32| -> Option<u32> { elf.read_at(a, 4).ok().map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])) };
            let mut funcs: Vec<(String, u32, u32)> = elf.symbols().unwrap().iter().filter(|s| s.is_func() && s.size > 0).map(|s| (s.name.clone(), s.address(), s.size)).collect();
            funcs.sort_by_key(|f| f.1);
            funcs.dedup_by_key(|f| f.1);
            let noreturn: Vec<u32> = funcs.iter().filter(|f| crate::graph::CallGraph::is_panic_path(&f.0)).map(|f| f.1).collect();
            let (mut refused, mut changed, mut ends) = (0, 0, 0);
            for (name, address, size) in &funcs {
                let code = elf.read_at(*address, *size as usize).unwrap();
                let b = body(&code, *address, &literal, &noreturn);
                let old = thumb::decode_function(&code, *address, &literal);
                let (f_old, f_new) = (crate::bound_v::frame_of(&old), crate::bound_v::frame_of(&b.insns));
                if !b.certified {
                    refused += 1;
                    println!("  REFUSED {name}: {:?}", b.why);
                }
                if f_old != f_new {
                    changed += 1;
                    println!("  FRAME {name}: {f_old} -> {f_new}");
                }
                if let Some(x) = b.insns.last() {
                    if let Effect::Call(c) = x.effect {
                        if x.address + x.len == address + size && !noreturn.contains(&c) {
                            ends += 1;
                            let callee = funcs.iter().find(|f| f.1 == c).map(|f| f.0.as_str()).unwrap_or("?");
                            println!("  ENDCALL {name} -> {callee}");
                        }
                    }
                }
            }
            println!("{path}: {} functions, refused {refused}, frames changed {changed}, ending in a call {ends}", funcs.len());
        }
    }
}
