//! The Generator's derivation arithmetic, verified (2026-11-16): the
//! computations of R23 and R24 that are arithmetic, inside `verus!`, each
//! proven against a definition stated here from the requirement and free of
//! overflow for every input. `derive`, `convert` and `layout` (which read the
//! declaration, name things and report errors) call these and stay ordinary
//! Rust.
//!
//! The specifications are the Generator's own. The Config_Checker recomputes
//! the same value classes from specifications written separately in its own
//! crate (R26.1): nothing here is shared with it, so that a mistake in one
//! statement is not a mistake in both.
//!
//! | Value (requirement) | Function | Proven |
//! |---|---|---|
//! | Priority byte (DD-02) | `priority_byte` | `(8 - p) * 32` for a Task level 1 to 8, 0 for the Kernel's and for anything else; no underflow |
//! | Ceiling (R23.1) | `ceiling` | an upper bound of the accessors' Priorities that one of them attains; 0 for none |
//! | Timed-event slots (R9.4, R23.4) | `slot_plan` | every Task has a deadline and a Budget slot, and a release slot when asked; all slots given are different, below the count less the Kernel's two; the count is at most 253; `None` exactly when it would not be |
//! | Time conversion (R21.2, R29.8, Property 14) | `scale` | the result is the quotient rounded down, or up, as asked: within one unit of the exact value on the stated side, and exact when the flag says so; `None` exactly when the product or the result does not fit |
//! | Time conversion to the nearest (R21.2: a Task's offset; 2026-11-21) | `scale_nearest` | the result is within half a unit of the exact value, a half rounded upward; exact when the flag says unchanged |
//! | Region size (DD-07, R16.6) | `region_log2` | the smallest power of two, not below the minimum, that holds the bytes; `None` when none below 2^32 does |

use vstd::prelude::*;

verus! {

/// DD-02: the NVIC priority byte of a level, with three implemented bits.
pub open spec fn spec_priority_byte(level: int) -> int {
    if 1 <= level <= 8 { (8 - level) * 32 } else { 0 }
}

pub fn priority_byte(level: u8) -> (b: u8)
    ensures b as int == spec_priority_byte(level as int),
{
    if level >= 1 && level <= 8 { (8 - level) * 32 } else { 0 }
}

/// R23.1: `c` is the Ceiling of a Resource whose accessors have the
/// Priorities `p`: no accessor is above it, and one is at it.
pub open spec fn is_ceiling(p: Seq<u8>, c: u8) -> bool {
    &&& forall|i: int| 0 <= i < p.len() ==> #[trigger] p[i] <= c
    &&& (p.len() == 0 ==> c == 0)
    &&& (p.len() > 0 ==> exists|i: int| 0 <= i < p.len() && #[trigger] p[i] == c)
}

pub fn ceiling(p: &Vec<u8>) -> (c: u8)
    ensures is_ceiling(p@, c),
{
    let mut c: u8 = 0;
    let mut i: usize = 0;
    while i < p.len()
        invariant
            i <= p.len(),
            forall|j: int| 0 <= j < i ==> #[trigger] p@[j] <= c,
            i == 0 ==> c == 0,
            i > 0 ==> exists|j: int| 0 <= j < i && #[trigger] p@[j] == c,
        decreases p.len() - i,
    {
        if i == 0 || p[i] > c {
            c = p[i];
        }
        i += 1;
    }
    c
}

/// The slots of one Task: the release slot (`NO_SLOT` when the Task has
/// none), the deadline slot, the Budget slot.
pub struct Slots {
    pub release: u8,
    pub deadline: u8,
    pub budget: u8,
}

pub const NO_SLOT: u8 = 0xFF;

/// The slots of Task `t` that are real slots, as a set.
pub open spec fn slots_of(s: Slots) -> Set<int> {
    if s.release == NO_SLOT {
        set![s.deadline as int, s.budget as int]
    } else {
        set![s.release as int, s.deadline as int, s.budget as int]
    }
}

/// R9.4, R23.4: a plan gives every Task a deadline and a Budget slot, and
/// a release slot exactly when `wants[t]`; every slot is below `count - 2`
/// (the last two are the Kernel's: the end of the operating duration and
/// housekeeping); no slot is given twice, to one Task or to two.
pub open spec fn plan_ok(wants: Seq<bool>, plan: Seq<Slots>, count: int) -> bool {
    &&& plan.len() == wants.len()
    &&& 2 <= count <= 253
    &&& forall|t: int| 0 <= t < plan.len() ==> {
            &&& (#[trigger] plan[t]).deadline != plan[t].budget
            &&& (plan[t].deadline as int) < count - 2 && (plan[t].budget as int) < count - 2
            &&& (wants[t] ==> plan[t].release != NO_SLOT && (plan[t].release as int) < count - 2
                    && plan[t].release != plan[t].deadline && plan[t].release != plan[t].budget)
            &&& (!wants[t] ==> plan[t].release == NO_SLOT)
        }
    &&& forall|t: int, u: int| 0 <= t < plan.len() && 0 <= u < plan.len() && t != u
            ==> #[trigger] slots_of(plan[t]).disjoint(#[trigger] slots_of(plan[u]))
}

/// The number of slots the Tasks of `wants[..n]` take.
pub open spec fn taken(wants: Seq<bool>, n: int) -> int
    decreases n,
{
    if n <= 0 { 0 } else { taken(wants, n - 1) + (if wants[n - 1] { 3int } else { 2int }) }
}

pub fn slot_plan(wants: &Vec<bool>) -> (r: Option<(Vec<Slots>, u8)>)
    ensures
        r is Some ==> plan_ok(wants@, r.unwrap().0@, r.unwrap().1 as int)
            && r.unwrap().1 as int == taken(wants@, wants.len() as int) + 2,
        r is None ==> taken(wants@, wants.len() as int) + 2 > 253,
{
    let mut plan: Vec<Slots> = Vec::new();
    let mut next: u8 = 0;
    let mut t: usize = 0;
    while t < wants.len()
        invariant
            t <= wants.len(),
            plan.len() == t,
            next as int == taken(wants@, t as int),
            next <= 251,
            forall|j: int| 0 <= j < t ==> {
                &&& (#[trigger] plan@[j]).deadline != plan@[j].budget
                &&& plan@[j].deadline < next && plan@[j].budget < next
                &&& (wants@[j] ==> plan@[j].release != NO_SLOT && plan@[j].release < next
                        && plan@[j].release != plan@[j].deadline && plan@[j].release != plan@[j].budget)
                &&& (!wants@[j] ==> plan@[j].release == NO_SLOT)
            },
            forall|j: int, k: int| 0 <= j < t && 0 <= k < t && j != k
                ==> #[trigger] slots_of(plan@[j]).disjoint(#[trigger] slots_of(plan@[k])),
        decreases wants.len() - t,
    {
        let want = wants[t];
        let need: u8 = if want { 3 } else { 2 };
        if next > 251 - need {
            proof {
                // The Tasks so far, with this one, already pass the limit,
                // and the rest only add.
                lemma_taken_monotone(wants@, t as int + 1, wants.len() as int);
            }
            return None;
        }
        let release = if want { next } else { NO_SLOT };
        let base = if want { next + 1 } else { next };
        let s = Slots { release, deadline: base, budget: base + 1 };
        proof {
            let old_plan = plan@;
            assert forall|j: int| 0 <= j < t implies
                #[trigger] slots_of(old_plan[j]).disjoint(slots_of(s)) && slots_of(s).disjoint(slots_of(old_plan[j])) by {
                // Every slot of an earlier Task is below `next`; every slot
                // of this one is at or above it.
                assert(old_plan[j].deadline < next && old_plan[j].budget < next);
            }
        }
        plan.push(s);
        next = next + need;
        t += 1;
    }
    proof {
        assert(plan_ok(wants@, plan@, next as int + 2));
    }
    Some((plan, next + 2))
}

pub proof fn lemma_taken_monotone(wants: Seq<bool>, a: int, b: int)
    requires 0 <= a <= b,
    ensures taken(wants, a) <= taken(wants, b),
    decreases b - a,
{
    if a < b {
        lemma_taken_monotone(wants, a, b - 1);
    }
}

/// R21.2, R29.8 (Property 14): `x` is `value * num / den` rounded down
/// (`up` false) or up (`up` true).
pub open spec fn rounded(value: int, num: int, den: int, up: bool, x: int) -> bool {
    if up {
        x * den >= value * num && (x == 0 || (x - 1) * den < value * num)
    } else {
        x * den <= value * num && value * num < (x + 1) * den
    }
}

/// The conversion of a quantity by the ratio `num / den`. Returns the
/// rounded value and whether rounding changed it; `None` when the product
/// does not fit 128 bits or the result does not fit 64.
pub fn scale(value: u64, num: u128, den: u128, up: bool) -> (r: Option<(u64, bool)>)
    requires den > 0,
    ensures
        r is Some ==> rounded(value as int, num as int, den as int, up, r.unwrap().0 as int)
            && (r.unwrap().1 <==> (value as int * num as int) % (den as int) != 0),
        r is None ==> value as int * num as int > u128::MAX
            || (value as int * num as int) / (den as int) + 1 > u64::MAX,
{
    let v = match (value as u128).checked_mul(num) {
        Some(v) => v,
        None => return None,
    };
    let q = v / den;
    let rem = v % den;
    proof {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(v as int, den as int);
        assert(v as int == den as int * q as int + rem as int);
        assert(0 <= rem < den);
    }
    if q >= u64::MAX as u128 {
        return None;
    }
    let x: u128 = if up && rem != 0 { q + 1 } else { q };
    proof {
        let (vi, di, qi, ri, xi) = (v as int, den as int, q as int, rem as int, x as int);
        if up {
            if ri != 0 {
                assert(xi * di >= vi && (xi - 1) * di < vi) by (nonlinear_arith)
                    requires vi == di * qi + ri, 0 < ri < di, xi == qi + 1;
            } else {
                assert(xi * di >= vi && (xi == 0 || (xi - 1) * di < vi)) by (nonlinear_arith)
                    requires vi == di * qi + ri, ri == 0, di > 0, xi == qi;
            }
        } else {
            assert(xi * di <= vi && vi < (xi + 1) * di) by (nonlinear_arith)
                requires vi == di * qi + ri, 0 <= ri < di, xi == qi;
        }
    }
    Some((x as u64, rem != 0))
}

/// R21.2: `x` is `value * num / den` rounded to the nearest whole number,
/// a half upward: `x - 1/2 <= value * num / den < x + 1/2`, written without
/// division.
pub open spec fn rounded_nearest(value: int, num: int, den: int, x: int) -> bool {
    2 * (x * den) <= 2 * (value * num) + den && 2 * (value * num) + den < 2 * ((x + 1) * den)
}

/// The conversion of a quantity by the ratio `num / den`, rounded to the
/// nearest whole number, a half upward (R21.2: a Task's offset; verified
/// 2026-11-21, until then the one rounding of the Generator outside
/// `verus!`). Returns the rounded value and whether rounding changed it;
/// `None` when the product does not fit 128 bits or the result does not
/// fit 64.
pub fn scale_nearest(value: u64, num: u128, den: u128) -> (r: Option<(u64, bool)>)
    requires den > 0,
    ensures
        r is Some ==> rounded_nearest(value as int, num as int, den as int, r.unwrap().0 as int)
            && (r.unwrap().1 <==> (value as int * num as int) % (den as int) != 0),
        r is None ==> value as int * num as int > u128::MAX
            || (value as int * num as int) / (den as int) + 1 > u64::MAX,
{
    let v = match (value as u128).checked_mul(num) {
        Some(v) => v,
        None => return None,
    };
    let q = v / den;
    let rem = v % den;
    proof {
        vstd::arithmetic::div_mod::lemma_fundamental_div_mod(v as int, den as int);
        assert(v as int == den as int * q as int + rem as int);
        assert(0 <= rem < den);
    }
    // The remainder is at least half the divisor: `2 * rem >= den`, written
    // so that it cannot overflow.
    let half_up = rem >= den - rem;
    if q >= u64::MAX as u128 && (half_up || q > u64::MAX as u128) {
        return None;
    }
    let x: u128 = if half_up { q + 1 } else { q };
    proof {
        let (vi, di, qi, ri, xi) = (v as int, den as int, q as int, rem as int, x as int);
        if half_up {
            assert(2 * (xi * di) <= 2 * vi + di && 2 * vi + di < 2 * ((xi + 1) * di)) by (nonlinear_arith)
                requires vi == di * qi + ri, 0 <= ri < di, 2 * ri >= di, xi == qi + 1;
        } else {
            assert(2 * (xi * di) <= 2 * vi + di && 2 * vi + di < 2 * ((xi + 1) * di)) by (nonlinear_arith)
                requires vi == di * qi + ri, 0 <= ri < di, 2 * ri < di, xi == qi;
        }
    }
    Some((x as u64, rem != 0))
}

/// DD-07: two to the `k`.
pub open spec fn two_to(k: nat) -> int
    decreases k,
{
    if k == 0 { 1 } else { 2 * two_to((k - 1) as nat) }
}

/// The smallest region of a power-of-two size, of at least `2^min` bytes,
/// that holds `bytes`: its exponent. `None` when no exponent up to 31
/// does (an MPU region of the Target is at most 2^32 bytes; the layout
/// uses none that large).
pub fn region_log2(bytes: u64, min: u8) -> (r: Option<u8>)
    requires min <= 31,
    ensures
        r is Some ==> min <= r.unwrap() <= 31 && two_to(r.unwrap() as nat) >= bytes
            && (r.unwrap() > min ==> two_to((r.unwrap() - 1) as nat) < bytes),
        r is None ==> two_to(31) < bytes,
{
    let mut k: u8 = 0;
    let mut size: u64 = 1;
    // `size` is 2^k throughout; first up to the minimum, then until it holds the bytes.
    while k < min
        invariant k <= min, min <= 31, size as int == two_to(k as nat), size <= 0x8000_0000,
        decreases min - k,
    {
        proof {
            lemma_two_to_bound(k as nat);
        }
        size = size * 2;
        k += 1;
    }
    while size < bytes
        invariant
            min <= k <= 31, size as int == two_to(k as nat), size <= 0x8000_0000,
            k > min ==> two_to((k - 1) as nat) < bytes,
        decreases 31 - k,
    {
        if k == 31 {
            return None;
        }
        proof {
            lemma_two_to_bound(k as nat);
        }
        size = size * 2;
        k += 1;
    }
    Some(k)
}

/// 2^k is at most 2^30 for k below 31 (so doubling it fits).
pub proof fn lemma_two_to_bound(k: nat)
    requires k < 31,
    ensures two_to(k) <= 0x4000_0000, two_to(k + 1) == 2 * two_to(k),
    decreases 31 - k,
{
    reveal_with_fuel(two_to, 32);
}

} // verus!
