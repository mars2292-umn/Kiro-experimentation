//! The response-time fixed point of R29.1 (Property 16, R32.1): for one
//! Task, `w = base + Σ_j ⌈(w + J_j) / T_j⌉ · E_j` with `base = C + B + O`,
//! iterated from `base` upwards. The loop is inside `verus!`: Verus proves
//! that it terminates, that `Ok(w)` is the least fixed point of the
//! recurrence (in unbounded integer arithmetic), that `NotSchedulable(w)`
//! is returned only once the iterate (plus the Task's own jitter) exceeds
//! the deadline and is then a lower bound on every fixed point (R29.7),
//! and that every arithmetic step is checked for `u64` overflow (R29.6;
//! `Overflow` is returned instead of a wrong result).
//!
//! Interferers are the Tasks of equal or higher Priority (R29.1 `hep(i)`,
//! `j ≠ i`); the caller builds the `Term` list.

use vstd::prelude::*;

verus! {

/// One interfering Task: its release jitter, its period or MIT, and its
/// interference bound E_j (R29.4), all in CPU cycles.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Term {
    pub jitter: u64,
    pub period: u64,
    pub bound: u64,
}

/// `⌈a / t⌉` for `t > 0`.
pub open spec fn ceil_div(a: int, t: int) -> int
    recommends t > 0,
{
    (a + t - 1) / t
}

/// The interference of the terms at iterate `w` (spec, unbounded).
pub open spec fn interference(terms: Seq<Term>, w: int) -> int
    decreases terms.len(),
{
    if terms.len() == 0 {
        0
    } else {
        let t = terms[0];
        ceil_div(w + t.jitter as int, t.period as int) * (t.bound as int) + interference(terms.skip(1), w)
    }
}

/// The recurrence `f(w) = base + interference(w)`.
pub open spec fn step(base: int, terms: Seq<Term>, w: int) -> int {
    base + interference(terms, w)
}

pub open spec fn is_fixpoint(base: int, terms: Seq<Term>, w: int) -> bool {
    step(base, terms, w) == w
}

pub open spec fn terms_ok(terms: Seq<Term>) -> bool {
    forall|i: int| 0 <= i < terms.len() ==> #[trigger] terms[i].period > 0
}

proof fn lemma_ceil_div_monotone(a: int, b: int, t: int)
    requires a <= b, t > 0,
    ensures ceil_div(a, t) <= ceil_div(b, t),
{
    assert((a + t - 1) / t <= (b + t - 1) / t) by (nonlinear_arith)
        requires a <= b, t > 0;
}

proof fn lemma_ceil_div_nonneg(a: int, t: int)
    requires a >= 0, t > 0,
    ensures ceil_div(a, t) >= 0,
{
    assert((a + t - 1) / t >= 0) by (nonlinear_arith)
        requires a >= 0, t > 0;
}

/// Monotonicity of the interference in `w` (the heart of Property 16).
proof fn lemma_interference_monotone(terms: Seq<Term>, w1: int, w2: int)
    requires terms_ok(terms), w1 <= w2,
    ensures interference(terms, w1) <= interference(terms, w2),
    decreases terms.len(),
{
    if terms.len() > 0 {
        let t = terms[0];
        lemma_ceil_div_monotone(w1 + t.jitter as int, w2 + t.jitter as int, t.period as int);
        assert(terms_ok(terms.skip(1))) by {
            assert forall|i: int| 0 <= i < terms.skip(1).len() implies #[trigger] terms.skip(1)[i].period > 0 by {
                assert(terms.skip(1)[i] == terms[i + 1]);
            }
        }
        lemma_interference_monotone(terms.skip(1), w1, w2);
        let c1 = ceil_div(w1 + t.jitter as int, t.period as int);
        let c2 = ceil_div(w2 + t.jitter as int, t.period as int);
        assert(c1 * (t.bound as int) <= c2 * (t.bound as int)) by (nonlinear_arith)
            requires c1 <= c2, t.bound as int >= 0;
    }
}

proof fn lemma_interference_nonneg(terms: Seq<Term>, w: int)
    requires terms_ok(terms), w >= 0,
    ensures interference(terms, w) >= 0,
    decreases terms.len(),
{
    if terms.len() > 0 {
        let t = terms[0];
        lemma_ceil_div_nonneg(w + t.jitter as int, t.period as int);
        assert(terms_ok(terms.skip(1))) by {
            assert forall|i: int| 0 <= i < terms.skip(1).len() implies #[trigger] terms.skip(1)[i].period > 0 by {
                assert(terms.skip(1)[i] == terms[i + 1]);
            }
        }
        lemma_interference_nonneg(terms.skip(1), w);
        let c = ceil_div(w + t.jitter as int, t.period as int);
        assert(c * (t.bound as int) >= 0) by (nonlinear_arith) requires c >= 0, t.bound as int >= 0;
    }
}

/// Every iterate from `base` stays at or below every fixed point.
proof fn lemma_iterate_below_fixpoint(base: int, terms: Seq<Term>, w: int, x: int)
    requires terms_ok(terms), w <= x, is_fixpoint(base, terms, x),
    ensures step(base, terms, w) <= x,
{
    lemma_interference_monotone(terms, w, x);
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// The least fixed point `w` (the response time is `jitter + w`).
    Fixpoint(u64),
    /// The iterate exceeded the deadline (R29.7): a lower bound on `w`.
    NotSchedulable(u64),
    /// `u64` arithmetic would have overflowed (R29.6).
    Overflow,
}

/// `⌈a / t⌉` in `u64`, `None` on overflow.
fn ceil_div_exec(a: u64, t: u64) -> (r: Option<u64>)
    requires t > 0,
    ensures r.is_some() ==> r.unwrap() as int == ceil_div(a as int, t as int),
{
    if a > u64::MAX - (t - 1) {
        return None;
    }
    Some((a + (t - 1)) / t)
}

/// The interference at `w` in `u64`; `None` on overflow. Mirrors the spec
/// term by term so that the postcondition follows the recursion.
fn interference_exec(terms: &Vec<Term>, from: usize, w: u64) -> (r: Option<u64>)
    requires terms_ok(terms@), from <= terms.len(),
    ensures r.is_some() ==> r.unwrap() as int == interference(terms@.skip(from as int), w as int),
    decreases terms.len() - from,
{
    if from == terms.len() {
        assert(terms@.skip(from as int).len() == 0);
        return Some(0);
    }
    let t = terms[from];
    assert(t.period > 0);
    if w > u64::MAX - t.jitter {
        return None;
    }
    let jobs = match ceil_div_exec(w + t.jitter, t.period) {
        Some(j) => j,
        None => return None,
    };
    assert((jobs as int) * (t.bound as int) < 0x1_0000_0000_0000_0000_0000_0000_0000_0000) by (nonlinear_arith)
        requires jobs < 0x1_0000_0000_0000_0000, t.bound < 0x1_0000_0000_0000_0000;
    let product: u128 = (jobs as u128) * (t.bound as u128);
    if product > u64::MAX as u128 {
        return None;
    }
    let product = product as u64;
    assert(terms_ok(terms@.skip(from as int + 1))) by {
        assert forall|i: int| 0 <= i < terms@.skip(from as int + 1).len() implies #[trigger] terms@.skip(from as int + 1)[i].period > 0 by {
            assert(terms@.skip(from as int + 1)[i] == terms@[from as int + 1 + i]);
        }
    }
    let rest = match interference_exec(terms, from + 1, w) {
        Some(r) => r,
        None => return None,
    };
    if product > u64::MAX - rest {
        return None;
    }
    assert(terms@.skip(from as int)[0] == t);
    assert(terms@.skip(from as int).skip(1) =~= terms@.skip(from as int + 1));
    Some(product + rest)
}

/// The fixed-point iteration (R29.1, R29.6, R29.7). `deadline` and
/// `jitter` are the Task's own (R = J + w must stay within D).
pub fn fixpoint(base: u64, terms: &Vec<Term>, jitter: u64, deadline: u64) -> (r: Outcome)
    requires terms_ok(terms@),
    ensures
        (r matches Outcome::Fixpoint(w) ==> is_fixpoint(base as int, terms@, w as int)
            && (forall|x: int| x >= 0 && is_fixpoint(base as int, terms@, x) ==> w as int <= x)
            && jitter as int + w as int <= deadline as int),
        (r matches Outcome::NotSchedulable(w) ==> jitter as int + w as int > deadline as int
            && (forall|x: int| x >= 0 && is_fixpoint(base as int, terms@, x) ==> w as int <= x)),
{
    let ghost tseq = terms@;
    // Every nonnegative fixed point is at least `base`.
    assert forall|x: int| x >= 0 && is_fixpoint(base as int, tseq, x) implies base as int <= x by {
        lemma_interference_nonneg(tseq, x);
    }
    let mut w: u64 = base;
    if jitter > deadline || w > deadline - jitter {
        return Outcome::NotSchedulable(w);
    }
    // The iterates from `base` never decrease: `base <= step(base)` because
    // the interference is nonnegative, and `w <= step(w)` is preserved by
    // the monotonicity of `step` (w <= step(w) ==> step(w) <= step(step(w))).
    assert(w as int <= step(base as int, tseq, w as int)) by {
        lemma_interference_nonneg(tseq, w as int);
    }
    loop
        invariant
            terms_ok(tseq), tseq == terms@,
            w >= base,
            w as int <= step(base as int, tseq, w as int),
            forall|x: int| x >= 0 && is_fixpoint(base as int, tseq, x) ==> w as int <= x,
            jitter as int + w as int <= deadline as int,
        decreases deadline - w,
    {
        let i = match interference_exec(terms, 0, w) {
            Some(i) => i,
            None => return Outcome::Overflow,
        };
        assert(tseq.skip(0) =~= tseq);
        if i > u64::MAX - base {
            return Outcome::Overflow;
        }
        let next = base + i;
        assert(next as int == step(base as int, tseq, w as int));
        assert(next >= w);
        if next == w {
            return Outcome::Fixpoint(w);
        }
        // next > w: next stays below every fixed point, and below its image.
        assert forall|x: int| x >= 0 && is_fixpoint(base as int, tseq, x) implies next as int <= x by {
            lemma_iterate_below_fixpoint(base as int, tseq, w as int, x);
        }
        assert(next as int <= step(base as int, tseq, next as int)) by {
            lemma_interference_monotone(tseq, w as int, next as int);
        }
        if next > deadline - jitter {
            return Outcome::NotSchedulable(next);
        }
        w = next;
    }
}

} // verus!
