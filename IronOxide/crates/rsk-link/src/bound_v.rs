//! What the call graph and the stack bounds are computed from, verified
//! (R37.1, R37.2; 2026-11-26). `cert` checks the walk's result against the
//! nodes it is given; this module is the two ends of that: how a node comes
//! from the instructions the decoder read, and how the bounds come from the
//! walk's usages.
//!
//! - `frame_of`, `dynamic_of`, `direct_of`: a function's frame is the sum of
//!   the stack-pointer decrements the decoder read in its body (saturating),
//!   it is marked dynamic exactly when an instruction changes SP by an
//!   amount the decoder does not read, and its direct edges are exactly the
//!   targets of its `bl`s and of its branches that leave the body. The
//!   graph's construction (`graph::CallGraph::build`) takes these three from
//!   here. That the decoder reads each instruction as the processor
//!   executes it, and that the sum of a body's decrements bounds the
//!   stack-pointer's descent along a path, are in Lean
//!   (`Tools/LinkThumbArm.lean`, `Tools/LinkFrame.lean`).
//! - `msp_value`: the Kernel's main-stack bound is the largest of the three
//!   cases of R37.2 as `stack::msp_bound` documents them, never below the
//!   first, and refused (`None`) when a term does not fit 64 bits.
//! - `jobs_bound`: a Partition's bound is the sum over its Tasks of the
//!   wrapper's usage and the frames stacked for a Job.
//!
//! - `unread_of` (2026-11-27): the addresses of the instructions the
//!   decoder answered `SpUnread` for. A function with one has no bound.
//!
//! The body these are computed from is `flow::body`'s, whose walk and
//! stack heights are checked by `flow_v::flow_ok` (2026-11-27).
//!
//! What stays outside: the literal reads, the symbol table and the read-only tables the
//! indirect calls are resolved from, the walk itself (checked by `cert`),
//! and the text of the report.

use crate::thumb::{Effect, FpClass, Insn};
use vstd::prelude::*;

verus! {

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExFpClass(FpClass);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExEffect(Effect);

#[verifier::external_type_specification]
#[allow(dead_code)]
pub struct ExInsn(Insn);

/// The bytes instruction `e` takes from the stack, as the decoder read it.
pub open spec fn down(e: Effect) -> nat {
    match e {
        Effect::SpDown(n) => n as nat,
        _ => 0,
    }
}

/// The sum of the decrements of the first `k` instructions.
pub open spec fn down_sum(s: Seq<Insn>, k: int) -> nat
    decreases k,
{
    if k <= 0 {
        0
    } else {
        down_sum(s, k - 1) + down(s[k - 1].effect)
    }
}

/// The frame of a body: the sum of its decrements, saturating at the
/// largest 32-bit value (a frame that large fits no stack region).
pub fn frame_of(insns: &Vec<Insn>) -> (f: u32)
    ensures
        f as nat == if down_sum(insns@, insns@.len() as int) <= u32::MAX { down_sum(insns@, insns@.len() as int) } else { u32::MAX as nat },
{
    let mut f: u32 = 0;
    let mut i: usize = 0;
    while i < insns.len()
        invariant
            i <= insns.len(),
            f as nat == if down_sum(insns@, i as int) <= u32::MAX { down_sum(insns@, i as int) } else { u32::MAX as nat },
        decreases insns.len() - i,
    {
        match &insns[i].effect {
            Effect::SpDown(n) => {
                f = f.saturating_add(*n);
            }
            _ => {}
        }
        i += 1;
    }
    f
}

/// Whether a body changes SP by an amount the decoder does not read.
pub fn dynamic_of(insns: &Vec<Insn>) -> (d: bool)
    ensures d <==> exists|i: int| 0 <= i < insns@.len() && (#[trigger] insns@[i]).effect is SpDynamic,
{
    let mut d = false;
    let mut i: usize = 0;
    while i < insns.len()
        invariant
            i <= insns.len(),
            d <==> exists|j: int| 0 <= j < i && (#[trigger] insns@[j]).effect is SpDynamic,
        decreases insns.len() - i,
    {
        match &insns[i].effect {
            Effect::SpDynamic => {
                d = true;
            }
            _ => {}
        }
        i += 1;
    }
    d
}

/// Instruction `e` of the body `[address, end)` transfers control to
/// `t` outside the body, or calls `t`: an edge of the call graph.
pub open spec fn edge(e: Effect, address: u32, end: u32, t: u32) -> bool {
    match e {
        Effect::Call(x) => x == t,
        Effect::Branch(x) => x == t && (t < address || t >= end),
        Effect::CompareBranch(x) => x == t && (t < address || t >= end),
        _ => false,
    }
}

/// The edge instruction `e` of the body `[address, end)` makes, if it
/// makes one. (A function of its own since 2026-11-27, so that the loop of
/// `direct_of` has one place where it pushes: for its Lean translation.)
pub fn edge_of(e: &Effect, address: u32, end: u32) -> (r: Option<u32>)
    ensures forall|t: u32| #[trigger] edge(*e, address, end, t) <==> r == Some(t),
{
    match e {
        Effect::Call(t) => Some(*t),
        Effect::Branch(t) => if *t < address || *t >= end { Some(*t) } else { None },
        Effect::CompareBranch(t) => if *t < address || *t >= end { Some(*t) } else { None },
        _ => None,
    }
}

/// The direct edges of a body: exactly the targets of its calls and of its
/// branches that leave it.
pub fn direct_of(insns: &Vec<Insn>, address: u32, end: u32) -> (v: Vec<u32>)
    ensures
        forall|t: u32| v@.contains(t) <==> exists|i: int| 0 <= i < insns@.len() && edge((#[trigger] insns@[i]).effect, address, end, t),
{
    let mut v: Vec<u32> = Vec::new();
    let mut i: usize = 0;
    while i < insns.len()
        invariant
            i <= insns.len(),
            forall|t: u32| v@.contains(t) <==> exists|j: int| 0 <= j < i && edge((#[trigger] insns@[j]).effect, address, end, t),
        decreases insns.len() - i,
    {
        let ghost before = v@;
        let o = edge_of(&insns[i].effect, address, end);
        match o {
            Some(t) => {
                v.push(t);
                assert forall|u: u32| v@.contains(u) <==> exists|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u) by {
                    lemma_pushed(before, t, u);
                    if exists|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u) {
                        let j = choose|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u);
                        if j < i {
                            assert(before.contains(u));
                        }
                    }
                    if v@.contains(u) && !before.contains(u) {
                        assert(edge(insns@[i as int].effect, address, end, u));
                    }
                }
            }
            None => {
                assert forall|u: u32| v@.contains(u) <==> exists|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u) by {
                    if exists|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u) {
                        let j = choose|j: int| 0 <= j < i + 1 && edge((#[trigger] insns@[j]).effect, address, end, u);
                        assert(j < i);
                    }
                }
            }
        }
        i += 1;
    }
    v
}

/// What a sequence contains after a push.
proof fn lemma_pushed(s: Seq<u32>, t: u32, u: u32)
    ensures s.push(t).contains(u) <==> (s.contains(u) || u == t),
{
    if s.contains(u) {
        let k = choose|k: int| 0 <= k < s.len() && s[k] == u;
        assert(s.push(t)[k] == u);
    }
    if u == t {
        assert(s.push(t)[s.len() as int] == u);
    }
    if s.push(t).contains(u) {
        let k = choose|k: int| 0 <= k < s.push(t).len() && s.push(t)[k] == u;
        if k < s.len() {
            assert(s[k] == u);
        }
    }
}

/// The addresses of the instructions the decoder answered `SpUnread` for:
/// every one of the body, and no other address (2026-11-27; until then a
/// line of the graph's own loop).
pub fn unread_of(insns: &Vec<Insn>) -> (v: Vec<u32>)
    ensures
        forall|a: u32| v@.contains(a) <==> exists|i: int| 0 <= i < insns@.len() && (#[trigger] insns@[i]).effect is SpUnread && insns@[i].address == a,
{
    let mut v: Vec<u32> = Vec::new();
    let mut i: usize = 0;
    while i < insns.len()
        invariant
            i <= insns.len(),
            forall|a: u32| v@.contains(a) <==> exists|j: int| 0 <= j < i && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a,
        decreases insns.len() - i,
    {
        let ghost before = v@;
        match &insns[i].effect {
            Effect::SpUnread => {
                let at = insns[i].address;
                v.push(at);
                assert forall|a: u32| v@.contains(a) <==> exists|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a by {
                    lemma_pushed(before, at, a);
                    if exists|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a {
                        let j = choose|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a;
                        if j < i {
                            assert(before.contains(a));
                        }
                    }
                    if v@.contains(a) && !before.contains(a) {
                        assert(insns@[i as int].effect is SpUnread && insns@[i as int].address == a);
                    }
                }
            }
            _ => {
                assert forall|a: u32| v@.contains(a) <==> exists|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a by {
                    if exists|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a {
                        let j = choose|j: int| 0 <= j < i + 1 && (#[trigger] insns@[j]).effect is SpUnread && insns@[j].address == a;
                        assert(j < i);
                    }
                }
            }
        }
        i += 1;
    }
    v
}

// ------------------------------------------------------------ the bounds

/// Basic exception frame (HW-06).
pub const BASIC_FRAME: u32 = 32;

/// Extended frame with S0-S15 and FPSCR (HW-10, eager stacking).
pub const FP_FRAME: u32 = 104;

/// What the MSP bound is computed from.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct MspInputs {
    /// The worst usage over every vector-table entry, the reset included.
    pub worst: u64,
    /// The reset handler's usage.
    pub reset: u64,
    /// The worst usage over the handlers other than the reset.
    pub worst_handler: u64,
    /// The worse of the HardFault and NMI handlers' usages.
    pub fatal: u64,
    /// The number of Job levels (distinct Priorities).
    pub levels: u64,
    /// The bytes of one level record.
    pub record: u64,
    /// Whether a Task has a floating-point context.
    pub any_fp: bool,
}

/// R37.2, case 1 (the rule before 2026-11-26): the worst entry, one level
/// record per Job level, the frame of an exception taken from the idle loop.
pub open spec fn msp_before(i: MspInputs) -> int {
    i.worst + i.levels * i.record + BASIC_FRAME
}

/// Case 2: the start with a HardFault or NMI taken in it.
pub open spec fn msp_start(i: MspInputs) -> int {
    i.reset + BASIC_FRAME + i.fatal
}

/// Case 3: handlers nested: the worst handler, the level records, the idle
/// frame, a basic frame per Job level, and a HardFault or NMI in the
/// innermost handler (an extended frame when a Task has a floating-point
/// context) with the fatal handler's usage.
pub open spec fn msp_nested(i: MspInputs) -> int {
    i.worst_handler + i.levels * i.record + BASIC_FRAME + i.levels * BASIC_FRAME + (if i.any_fp { FP_FRAME as int } else { BASIC_FRAME as int }) + i.fatal
}

/// The bound: the largest of the three.
pub open spec fn msp_spec(i: MspInputs) -> int {
    let a = msp_before(i);
    let b = msp_start(i);
    let c = msp_nested(i);
    if a >= b && a >= c { a } else if b >= c { b } else { c }
}

/// The three cases and the bound, computed; `None` when a term does not
/// fit 64 bits (no region holds such a bound).
pub fn msp_value(i: &MspInputs) -> (r: Option<(u64, u64, u64, u64)>)
    ensures
        r is Some ==> r->Some_0.0 == msp_before(*i) && r->Some_0.1 == msp_start(*i) && r->Some_0.2 == msp_nested(*i)
            && r->Some_0.3 == msp_spec(*i) && r->Some_0.3 >= msp_before(*i),
        r is None ==> msp_spec(*i) > u64::MAX,
{
    let basic = BASIC_FRAME as u64;
    let fatal_frame: u64 = if i.any_fp { FP_FRAME as u64 } else { BASIC_FRAME as u64 };
    assert(i.levels * i.record >= 0 && i.levels * BASIC_FRAME >= 0) by (nonlinear_arith);
    let records = match i.levels.checked_mul(i.record) {
        Some(x) => x,
        None => return None,
    };
    let windows = match i.levels.checked_mul(basic) {
        Some(x) => x,
        None => return None,
    };
    let before = match i.worst.checked_add(records) {
        Some(x) => match x.checked_add(basic) {
            Some(y) => y,
            None => return None,
        },
        None => return None,
    };
    let start = match i.reset.checked_add(basic) {
        Some(x) => match x.checked_add(i.fatal) {
            Some(y) => y,
            None => return None,
        },
        None => return None,
    };
    let n1 = match i.worst_handler.checked_add(records) {
        Some(x) => x,
        None => return None,
    };
    let n2 = match n1.checked_add(basic) {
        Some(x) => x,
        None => return None,
    };
    let n3 = match n2.checked_add(windows) {
        Some(x) => x,
        None => return None,
    };
    let n4 = match n3.checked_add(fatal_frame) {
        Some(x) => x,
        None => return None,
    };
    let nested = match n4.checked_add(i.fatal) {
        Some(x) => x,
        None => return None,
    };
    let bound = if before >= start && before >= nested { before } else if start >= nested { start } else { nested };
    Some((before, start, nested, bound))
}

/// The sum of the first `k` Jobs' needs: the wrapper's usage and the
/// frames stacked for a Job.
pub open spec fn jobs_sum(jobs: Seq<(u32, u32)>, k: int) -> int
    decreases k,
{
    if k <= 0 {
        0
    } else {
        jobs_sum(jobs, k - 1) + jobs[k - 1].0 + jobs[k - 1].1
    }
}

/// A Partition's bound (R37.2): the sum over its Tasks, each given as
/// (wrapper usage, frame bytes); `None` when it does not fit 64 bits.
pub fn jobs_bound(jobs: &Vec<(u32, u32)>) -> (r: Option<u64>)
    ensures
        r is Some ==> r->Some_0 == jobs_sum(jobs@, jobs@.len() as int),
        r is None ==> jobs_sum(jobs@, jobs@.len() as int) > u64::MAX,
{
    let mut bound: u64 = 0;
    let mut k: usize = 0;
    while k < jobs.len()
        invariant
            k <= jobs.len(),
            bound == jobs_sum(jobs@, k as int),
        decreases jobs.len() - k,
    {
        let per_job = jobs[k].0 as u64 + jobs[k].1 as u64;
        bound = match bound.checked_add(per_job) {
            Some(x) => x,
            None => {
                proof {
                    lemma_jobs_sum_grows(jobs@, k as int + 1, jobs@.len() as int);
                }
                return None;
            }
        };
        k += 1;
    }
    Some(bound)
}

/// The sum does not fall as Jobs are added.
proof fn lemma_jobs_sum_grows(jobs: Seq<(u32, u32)>, a: int, b: int)
    requires 0 <= a <= b,
    ensures jobs_sum(jobs, a) <= jobs_sum(jobs, b),
    decreases b - a,
{
    if a < b {
        lemma_jobs_sum_grows(jobs, a, b - 1);
    }
}

} // verus!
