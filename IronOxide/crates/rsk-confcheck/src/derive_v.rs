//! The Config_Checker's recomputation core, verified (Verification_Plan
//! 5.19; 2026-11-13): the arithmetic of every R26.2 value class that is a
//! computation, inside `verus!`, each function proven against the
//! definition the requirement states and free of overflow for every input.
//! `recompute` (the reading of the Task_Model, the names, and the report)
//! calls these and stays ordinary Rust.
//!
//! The specifications are stated here, from the requirements, and not
//! imported: the Config_Checker shares nothing with the Generator or the
//! Kernel (R26.1), and that holds for its proofs too.
//!
//! | Value class (R26.2) | Function | Proven |
//! |---|---|---|
//! | Ceilings (R23.1) | `ceiling_of` | the result is the maximum of the accessors' Priorities: at least each, equal to one of them, 0 for none |
//! | Priority encoding (DD-02) | `encode_level` | `(8 - p) * 32` for 1 to 8, 0 otherwise; no underflow for any level |
//! | Priority map | `levels_of` | the Priorities in use, each once, strictly ascending |
//! | Lock-stack capacity (R23.4) | `sum_checked` | the sum of the nesting depths, or `None` exactly when it exceeds `u64` |
//! | Timer slots (R9.4) | `timer_slots`, `slot_index` | `3n + 2`; `3i + k`; `None` exactly on overflow |
//! | Regions (DD-07, R16) | `size_of`, `aligned`, `disjoint`, `stack_fits` | the size is `2^size_log2`; two regions are reported disjoint exactly when no address lies in both; no overflow for any base and size |

use vstd::prelude::*;

verus! {

/// DD-02: the NVIC priority byte of a Task level with three implemented
/// priority bits; the Kernel level and any level beyond 8 encode as 0.
pub open spec fn spec_encode_level(level: int) -> int {
    if 1 <= level <= 8 { (8 - level) * 32 } else { 0 }
}

pub fn encode_level(level: u64) -> (e: u64)
    ensures e as int == spec_encode_level(level as int),
{
    if level == 0 || level > 8 { 0 } else { (8 - level) * 32 }
}

/// The maximum of a sequence, 0 for the empty one.
pub open spec fn spec_max(s: Seq<u64>) -> int
    decreases s.len(),
{
    if s.len() == 0 {
        0
    } else {
        let m = spec_max(s.drop_last());
        if s.last() as int > m { s.last() as int } else { m }
    }
}

/// The maximum is at least every element and, for a non-empty sequence, is
/// one of them.
pub proof fn lemma_max(s: Seq<u64>)
    ensures
        forall|i: int| 0 <= i < s.len() ==> (#[trigger] s[i]) as int <= spec_max(s),
        s.len() > 0 ==> exists|i: int| 0 <= i < s.len() && (#[trigger] s[i]) as int == spec_max(s),
        s.len() == 0 ==> spec_max(s) == 0,
    decreases s.len(),
{
    if s.len() > 0 {
        let t = s.drop_last();
        lemma_max(t);
        assert forall|i: int| 0 <= i < s.len() implies (#[trigger] s[i]) as int <= spec_max(s) by {
            if i < t.len() {
                assert(t[i] == s[i]);
            }
        }
        if s.last() as int > spec_max(t) || t.len() == 0 {
            assert(s[s.len() - 1] as int == spec_max(s));
        } else {
            let i = choose|i: int| 0 <= i < t.len() && (#[trigger] t[i]) as int == spec_max(t);
            assert(s[i] == t[i]);
        }
    }
}

/// R23.1: the Ceiling of a Resource is the maximum Priority over its
/// declared accessors (`prios`: the Priority of each).
pub fn ceiling_of(prios: &Vec<u64>) -> (c: u64)
    ensures
        c as int == spec_max(prios@),
        forall|i: int| 0 <= i < prios.len() ==> (#[trigger] prios[i]) <= c,
        prios.len() > 0 ==> exists|i: int| 0 <= i < prios.len() && (#[trigger] prios[i]) == c,
        prios.len() == 0 ==> c == 0,
{
    let mut c: u64 = 0;
    let mut i: usize = 0;
    while i < prios.len()
        invariant i <= prios.len(), c as int == spec_max(prios@.take(i as int)),
        decreases prios.len() - i,
    {
        proof {
            assert(prios@.take(i as int + 1).drop_last() =~= prios@.take(i as int));
        }
        if prios[i] > c {
            c = prios[i];
        }
        i += 1;
    }
    proof {
        assert(prios@.take(prios.len() as int) =~= prios@);
        lemma_max(prios@);
    }
    c
}

/// The sum of a sequence.
pub open spec fn spec_sum(s: Seq<u64>) -> int
    decreases s.len(),
{
    if s.len() == 0 { 0 } else { spec_sum(s.drop_last()) + s.last() as int }
}

/// R23.4: the lock stack holds every Task's nested sections at once: the
/// sum of the nesting depths. `None` exactly when the sum exceeds `u64`.
pub fn sum_checked(v: &Vec<u64>) -> (r: Option<u64>)
    ensures
        r is Some ==> r.unwrap() as int == spec_sum(v@),
        r is None ==> spec_sum(v@) > u64::MAX,
{
    let mut total: u64 = 0;
    let mut i: usize = 0;
    while i < v.len()
        invariant i <= v.len(), total as int == spec_sum(v@.take(i as int)),
        decreases v.len() - i,
    {
        proof {
            assert(v@.take(i as int + 1).drop_last() =~= v@.take(i as int));
        }
        if v[i] > u64::MAX - total {
            proof {
                lemma_sum_grows(v@, i as int + 1);
            }
            return None;
        }
        total = total + v[i];
        i += 1;
    }
    proof {
        assert(v@.take(v.len() as int) =~= v@);
    }
    Some(total)
}

/// A prefix's sum is at most the whole sequence's.
pub proof fn lemma_sum_grows(s: Seq<u64>, n: int)
    requires 0 <= n <= s.len(),
    ensures spec_sum(s.take(n)) <= spec_sum(s),
    decreases s.len() - n,
{
    if n < s.len() {
        assert(s.take(n + 1).drop_last() =~= s.take(n));
        lemma_sum_grows(s, n + 1);
    } else {
        assert(s.take(n) =~= s);
    }
}

/// R9.4: three timer slots per Task, the end of operation, and one more.
pub fn timer_slots(tasks: u64) -> (r: Option<u64>)
    ensures
        r is Some ==> r.unwrap() as int == 3 * tasks as int + 2,
        r is None ==> 3 * tasks as int + 2 > u64::MAX,
{
    if tasks > (u64::MAX - 2) / 3 { None } else { Some(3 * tasks + 2) }
}

/// The slot `k` (0 release, 1 deadline, 2 Budget) of Task `i`.
pub fn slot_index(i: u64, k: u64) -> (r: Option<u64>)
    requires k <= 2,
    ensures
        r is Some ==> r.unwrap() as int == 3 * i as int + k as int,
        r is None ==> 3 * i as int + k as int > u64::MAX,
{
    let v: u128 = 3 * (i as u128) + k as u128;
    if v > u64::MAX as u128 { None } else { Some(v as u64) }
}

/// Strictly ascending.
pub open spec fn ascending(s: Seq<u64>) -> bool {
    forall|i: int, j: int| 0 <= i < j < s.len() ==> s[i] < s[j]
}

/// The Priorities in use, each once, in ascending order (the keys of the
/// priority-to-hardware map).
pub fn levels_of(prios: &Vec<u64>) -> (out: Vec<u64>)
    ensures
        ascending(out@),
        forall|x: u64| out@.contains(x) <==> prios@.contains(x),
{
    let mut out: Vec<u64> = Vec::new();
    let mut k: usize = 0;
    while k < prios.len()
        invariant
            k <= prios.len(), ascending(out@),
            forall|x: u64| out@.contains(x) <==> prios@.take(k as int).contains(x),
        decreases prios.len() - k,
    {
        let p = prios[k];
        let mut pos: usize = 0;
        while pos < out.len() && out[pos] < p
            invariant pos <= out.len(), forall|i: int| 0 <= i < pos ==> out[i] < p,
            decreases out.len() - pos,
        {
            pos += 1;
        }
        let ghost before = out@;
        let ghost seen = prios@.take(k as int);
        let ghost seen1 = prios@.take(k as int + 1);
        proof {
            assert(seen1 =~= seen.push(p));
        }
        if pos == out.len() || out[pos] != p {
            out.insert(pos, p);
            proof {
                assert(out@ =~= before.insert(pos as int, p));
                assert forall|i: int, j: int| 0 <= i < j < out@.len() implies out@[i] < out@[j] by {
                    if j < pos {
                        assert(out@[i] == before[i] && out@[j] == before[j]);
                    } else if j == pos {
                        assert(out@[i] == before[i]);
                    } else if i < pos {
                        assert(out@[i] == before[i] && out@[j] == before[j - 1]);
                        assert(before[i] < p);
                        if (pos as int) < before.len() {
                            assert(before[pos as int] <= before[j - 1]) by {
                                if (pos as int) < j - 1 {
                                    assert(before[pos as int] < before[j - 1]);
                                }
                            }
                        }
                    } else if i == pos {
                        assert(out@[j] == before[j - 1]);
                        assert(before[pos as int] <= before[j - 1]) by {
                            if (pos as int) < j - 1 {
                                assert(before[pos as int] < before[j - 1]);
                            }
                        }
                    } else {
                        assert(out@[i] == before[i - 1] && out@[j] == before[j - 1]);
                    }
                }
                assert forall|x: u64| out@.contains(x) <==> seen1.contains(x) by {
                    if out@.contains(x) {
                        let i = choose|i: int| 0 <= i < out@.len() && out@[i] == x;
                        if i < pos {
                            assert(before[i] == x);
                            assert(before.contains(x));
                        } else if i > pos {
                            assert(before[i - 1] == x);
                            assert(before.contains(x));
                        }
                        if x != p {
                            assert(seen.contains(x));
                            let j = choose|j: int| 0 <= j < seen.len() && seen[j] == x;
                            assert(seen1[j] == x);
                        } else {
                            assert(seen1[k as int] == p);
                        }
                    }
                    if seen1.contains(x) {
                        let j = choose|j: int| 0 <= j < seen1.len() && seen1[j] == x;
                        if j < k {
                            assert(seen[j] == x);
                            assert(seen.contains(x));
                            assert(before.contains(x));
                            let i = choose|i: int| 0 <= i < before.len() && before[i] == x;
                            if i < pos {
                                assert(out@[i] == x);
                            } else {
                                assert(out@[i + 1] == x);
                            }
                        } else {
                            assert(out@[pos as int] == x);
                        }
                    }
                }
            }
        } else {
            proof {
                assert forall|x: u64| out@.contains(x) <==> seen1.contains(x) by {
                    if out@.contains(x) {
                        assert(seen.contains(x));
                        let j = choose|j: int| 0 <= j < seen.len() && seen[j] == x;
                        assert(seen1[j] == x);
                    }
                    if seen1.contains(x) {
                        let j = choose|j: int| 0 <= j < seen1.len() && seen1[j] == x;
                        if j < k {
                            assert(seen[j] == x);
                            assert(seen.contains(x));
                        } else {
                            assert(out@[pos as int] == x);
                        }
                    }
                }
            }
        }
        k += 1;
    }
    proof {
        assert(prios@.take(prios.len() as int) =~= prios@);
    }
    out
}

/// The size of a region of `2^size_log2` bytes (DD-07), for a size the
/// address space can hold.
pub open spec fn spec_size(size_log2: u8) -> int {
    (1u64 << (size_log2 as u64)) as int
}

/// Address `a` lies in the region at `base` of `2^size_log2` bytes.
pub open spec fn in_region(a: int, base: u64, size_log2: u8) -> bool {
    base as int <= a < base as int + spec_size(size_log2)
}

/// The region's size; `None` for a size beyond the 64-bit shift.
pub fn size_of(size_log2: u8) -> (s: Option<u64>)
    ensures
        size_log2 < 64 ==> s is Some && s.unwrap() as int == spec_size(size_log2) && s.unwrap() >= 1,
        size_log2 >= 64 ==> s is None,
{
    if size_log2 >= 64 {
        None
    } else {
        let s = 1u64 << (size_log2 as u64);
        proof {
            let n = size_log2 as u64;
            assert(n < 64 ==> (1u64 << n) >= 1) by (bit_vector);
        }
        Some(s)
    }
}

/// DD-07: an MPU region is a power of two of at least 32 bytes and at most
/// the address space, with its base a multiple of its size.
pub open spec fn spec_aligned(base: u64, size_log2: u8) -> bool {
    5 <= size_log2 <= 32 && base as int % spec_size(size_log2) == 0
}

pub fn aligned(base: u64, size_log2: u8) -> (ok: bool)
    ensures ok == spec_aligned(base, size_log2),
{
    if size_log2 < 5 || size_log2 > 32 {
        return false;
    }
    match size_of(size_log2) {
        Some(s) => base % s == 0,
        None => false,
    }
}

/// Two regions are disjoint: no address lies in both (R16.7). A region of
/// a size beyond the 64-bit shift is disjoint from nothing.
pub fn disjoint(a_base: u64, a_log2: u8, b_base: u64, b_log2: u8) -> (ok: bool)
    ensures
        a_log2 < 64 && b_log2 < 64 ==> (ok <==> forall|x: int| !(in_region(x, a_base, a_log2) && in_region(x, b_base, b_log2))),
        a_log2 >= 64 || b_log2 >= 64 ==> !ok,
{
    match (size_of(a_log2), size_of(b_log2)) {
        (Some(sa), Some(sb)) => {
            let ea = a_base as u128 + sa as u128;
            let eb = b_base as u128 + sb as u128;
            let ok = ea <= b_base as u128 || eb <= a_base as u128;
            proof {
                if !ok {
                    // The greater base lies in both.
                    let x = if a_base >= b_base { a_base as int } else { b_base as int };
                    assert(in_region(x, a_base, a_log2) && in_region(x, b_base, b_log2));
                }
            }
            ok
        }
        _ => false,
    }
}

/// R16.5: a Partition's stack is not empty and lies within its RAM region.
pub fn stack_fits(stack: u64, ram_log2: u8) -> (ok: bool)
    ensures ok <==> ram_log2 < 64 && 0 < stack as int <= spec_size(ram_log2),
{
    match size_of(ram_log2) {
        Some(s) => stack > 0 && stack <= s,
        None => false,
    }
}

} // verus!
