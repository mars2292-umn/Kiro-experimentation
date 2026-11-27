//! The stack-usage certificate (R37.1 to R37.3; 2026-11-22). The walk of
//! the call graph (`graph::CallGraph::visit`) is plain Rust and unverified.
//! Its result is checked here on every run by a verified function, and the
//! check implies the property the bounds rest on: along every chain of
//! calls from a bounded function, the frames add up to no more than the
//! usage the walk reported for it, with the panic path charged once.
//!
//! What stays outside: the frame of each function and its set of callees
//! (the disassembly and the call graph's construction, `thumb`, `graph`),
//! the frame model itself (the sum of a body's stack-pointer decrements),
//! and the per-region composition of `stack::bounds`.

use vstd::prelude::*;

verus! {

/// One function as the walk left it.
pub struct Node {
    /// The bytes its own body takes from the stack.
    pub frame: u32,
    /// The walk's usage from its entry, the callees it follows included.
    pub own: u32,
    /// The walk gave it a bound (no recursion, no unresolved call, no
    /// unbounded callee).
    pub bounded: bool,
    /// The walk found that it can reach a panic edge.
    pub panics: bool,
    /// The callees the walk follows.
    pub walked: Vec<usize>,
    /// Its panic edges: calls into the panic path from outside it, which
    /// the walk does not follow and charges once per entry point.
    pub cut: Vec<usize>,
}

/// A followed call from `i` to `c`: `c` is bounded, `i`'s usage covers its
/// own frame and `c`'s usage, and `i` can panic if `c` can.
pub open spec fn walk_ok(g: Seq<Node>, i: int, c: int) -> bool {
    &&& 0 <= c < g.len()
    &&& g[c].bounded
    &&& g[i].frame as int + g[c].own as int <= g[i].own as int
    &&& g[c].panics ==> g[i].panics
}

/// A panic edge from `i` to `c`: `i` is marked as able to panic, and the
/// panic path's charge covers `c`'s usage when `c` has a bound.
pub open spec fn cut_ok(g: Seq<Node>, i: int, c: int, panic_usage: int) -> bool {
    &&& 0 <= c < g.len()
    &&& g[i].panics
    &&& g[c].bounded ==> g[c].own as int <= panic_usage
}

pub open spec fn node_ok(g: Seq<Node>, i: int, panic_usage: int) -> bool {
    g[i].bounded ==> {
        &&& g[i].frame <= g[i].own
        &&& forall|k: int| 0 <= k < g[i].walked@.len() ==> walk_ok(g, i, #[trigger] g[i].walked@[k] as int)
        &&& forall|k: int| 0 <= k < g[i].cut@.len() ==> cut_ok(g, i, #[trigger] g[i].cut@[k] as int, panic_usage)
    }
}

/// The certificate: every bounded function's usage covers its frame and
/// each followed callee's usage, and its panic edges are accounted for.
pub open spec fn certified(g: Seq<Node>, panic_usage: int) -> bool {
    forall|i: int| 0 <= i < g.len() ==> #[trigger] node_ok(g, i, panic_usage)
}

/// Function `i` has a followed call to function `c`.
pub open spec fn follows(g: Seq<Node>, i: int, c: int) -> bool {
    exists|k: int| 0 <= k < g[i].walked@.len() && (#[trigger] g[i].walked@[k]) as int == c
}

/// Function `i` has a panic edge to function `c`.
pub open spec fn cuts(g: Seq<Node>, i: int, c: int) -> bool {
    exists|k: int| 0 <= k < g[i].cut@.len() && (#[trigger] g[i].cut@[k]) as int == c
}

/// A chain of followed calls: each function calls the next.
pub open spec fn walk_path(g: Seq<Node>, p: Seq<int>) -> bool {
    &&& p.len() > 0
    &&& forall|j: int| 0 <= j < p.len() ==> 0 <= #[trigger] p[j] < g.len()
    &&& forall|j: int| 1 <= j < p.len() ==> follows(g, p[j - 1], #[trigger] p[j])
}

/// The stack bytes a chain of calls holds at its deepest point: the sum of
/// its functions' frames.
pub open spec fn path_bytes(g: Seq<Node>, p: Seq<int>) -> int
    decreases p.len(),
{
    if p.len() == 0 {
        0
    } else {
        g[p[0]].frame as int + path_bytes(g, p.subrange(1, p.len() as int))
    }
}

/// R37.1: under the certificate, every chain of followed calls from a
/// bounded function takes no more stack than the usage reported for that
/// function; every function on it is bounded; and if its last function can
/// panic, the first is marked so.
pub proof fn theorem_walk_within_usage(g: Seq<Node>, panic_usage: int, p: Seq<int>)
    requires
        certified(g, panic_usage),
        walk_path(g, p),
        g[p[0]].bounded,
    ensures
        path_bytes(g, p) <= g[p[0]].own as int,
        g[p.last()].bounded,
        g[p.last()].panics ==> g[p[0]].panics,
    decreases p.len(),
{
    assert(node_ok(g, p[0], panic_usage));
    let q = p.subrange(1, p.len() as int);
    if p.len() == 1 {
        assert(q.len() == 0);
        assert(path_bytes(g, q) == 0);
    } else {
        let c = p[1];
        assert(follows(g, p[1 - 1], p[1]));
        let k = choose|k: int| 0 <= k < g[p[0]].walked@.len() && (#[trigger] g[p[0]].walked@[k]) as int == c;
        assert(walk_ok(g, p[0], g[p[0]].walked@[k] as int));
        assert(q[0] == c);
        assert(g[q[0]].bounded);
        assert(walk_path(g, q)) by {
            assert forall|j: int| 0 <= j < q.len() implies 0 <= #[trigger] q[j] < g.len() by {
                assert(q[j] == p[j + 1]);
            }
            assert forall|j: int| 1 <= j < q.len() implies follows(g, q[j - 1], #[trigger] q[j]) by {
                assert(q[j] == p[j + 1] && q[j - 1] == p[j + 1 - 1]);
                assert(follows(g, p[j + 1 - 1], p[j + 1]));
            }
        }
        theorem_walk_within_usage(g, panic_usage, q);
        assert(q.last() == p.last());
    }
}

/// R37.1 with the panic path: a chain of followed calls from a bounded
/// function, then one panic edge, then a chain of followed calls inside
/// the panic path takes no more stack than the function's usage plus the
/// panic path's charge, and the function is marked as able to panic (so
/// the charge is in its reported total).
pub proof fn theorem_one_panic_within_total(g: Seq<Node>, panic_usage: int, p1: Seq<int>, p2: Seq<int>)
    requires
        certified(g, panic_usage),
        walk_path(g, p1),
        g[p1[0]].bounded,
        walk_path(g, p2),
        cuts(g, p1.last(), p2[0]),
        g[p2[0]].bounded,
    ensures
        path_bytes(g, p1) + path_bytes(g, p2) <= g[p1[0]].own as int + panic_usage,
        g[p1[0]].panics,
{
    theorem_walk_within_usage(g, panic_usage, p1);
    let j = p1.last();
    assert(node_ok(g, j, panic_usage));
    let k = choose|k: int| 0 <= k < g[j].cut@.len() && (#[trigger] g[j].cut@[k]) as int == p2[0];
    assert(cut_ok(g, j, g[j].cut@[k] as int, panic_usage));
    theorem_walk_within_usage(g, panic_usage, p2);
}

/// The followed calls of function `i` (`walk_ok` for each). One loop, ended
/// by a flag: a function of its own since 2026-11-26, when the checker was
/// translated to Lean (`proofs/aeneas/lean/Tools/Link.lean`; Aeneas
/// translates neither a `return` inside a loop nor a loop nested in
/// another).
fn walked_check(g: &Vec<Node>, i: usize) -> (ok: bool)
    requires i < g@.len(),
    ensures ok ==> forall|b: int| 0 <= b < g@[i as int].walked@.len() ==> walk_ok(g@, i as int, #[trigger] g@[i as int].walked@[b] as int),
{
    let n = g.len();
    let node = &g[i];
    let m = node.walked.len();
    let mut ok = true;
    let mut k: usize = 0;
    while k < m && ok
        invariant
            k <= m, m == node.walked@.len(), i < n, n == g@.len(), *node == g@[i as int],
            forall|b: int| 0 <= b < k ==> walk_ok(g@, i as int, #[trigger] node.walked@[b] as int),
        decreases m - k + (if ok { 1int } else { 0int }),
    {
        let c = node.walked[k];
        if c >= n {
            ok = false;
        } else {
            let callee = &g[c];
            if !callee.bounded || node.frame as u64 + callee.own as u64 > node.own as u64 || (callee.panics && !node.panics) {
                ok = false;
            } else {
                k += 1;
            }
        }
    }
    ok
}

/// The panic edges of function `i` (`cut_ok` for each).
fn cut_check(g: &Vec<Node>, i: usize, panic_usage: u32) -> (ok: bool)
    requires i < g@.len(),
    ensures ok ==> forall|b: int| 0 <= b < g@[i as int].cut@.len() ==> cut_ok(g@, i as int, #[trigger] g@[i as int].cut@[b] as int, panic_usage as int),
{
    let n = g.len();
    let node = &g[i];
    let m = node.cut.len();
    let mut ok = true;
    let mut k: usize = 0;
    while k < m && ok
        invariant
            k <= m, m == node.cut@.len(), i < n, n == g@.len(), *node == g@[i as int],
            forall|b: int| 0 <= b < k ==> cut_ok(g@, i as int, #[trigger] node.cut@[b] as int, panic_usage as int),
        decreases m - k + (if ok { 1int } else { 0int }),
    {
        let c = node.cut[k];
        if c >= n || !node.panics {
            ok = false;
        } else {
            let callee = &g[c];
            if callee.bounded && callee.own > panic_usage {
                ok = false;
            } else {
                k += 1;
            }
        }
    }
    ok
}

/// Function `i` (`node_ok`).
fn node_check(g: &Vec<Node>, i: usize, panic_usage: u32) -> (ok: bool)
    requires i < g@.len(),
    ensures ok ==> node_ok(g@, i as int, panic_usage as int),
{
    let node = &g[i];
    if !node.bounded {
        true
    } else if node.frame > node.own {
        false
    } else if !walked_check(g, i) {
        false
    } else {
        cut_check(g, i, panic_usage)
    }
}

/// Checks the certificate: `true` only when it holds.
pub fn check(g: &Vec<Node>, panic_usage: u32) -> (r: bool)
    ensures r ==> certified(g@, panic_usage as int),
{
    let n = g.len();
    let mut ok = true;
    let mut i: usize = 0;
    while i < n && ok
        invariant
            i <= n, n == g@.len(),
            forall|a: int| 0 <= a < i ==> #[trigger] node_ok(g@, a, panic_usage as int),
        decreases n - i + (if ok { 1int } else { 0int }),
    {
        if node_check(g, i, panic_usage) {
            i += 1;
        } else {
            ok = false;
        }
    }
    ok
}

} // verus!
