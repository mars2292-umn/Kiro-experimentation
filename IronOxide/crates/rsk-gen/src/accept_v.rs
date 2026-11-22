//! The acceptance conditions of a declaration that are conditions on
//! numbers, stated as one predicate and decided by a proven function
//! (2026-11-22). `validate` (`src/validate.rs`) produces the diagnostics
//! and is tested; this function is run on what `derive` produced from a
//! declaration `validate` accepted, and the Generator emits nothing when it
//! answers no. So whatever the validation code does, no Generated_Config
//! exists for a declaration outside `accepted`.
//!
//! Covered: for every Task, a Priority of 1 to 7 (PR-33), a Partition that
//! exists, a period or MIT of at least one tick, a deadline of at least one
//! tick and not later than the period or MIT (R22.1), a Budget of at least
//! one cycle; at least one Partition (PR-28); for every Endpoint, two to
//! four roles bound to Tasks that exist, a capacity of at least one message
//! (PR-14), and R42.4's edges: the Kernel accepts a rejected-message report
//! from role `r` about role `f` only where `f` is of another Partition of
//! lower Criticality_Level, and in the plain form (two roles, each
//! receiving the payload from the other) every such edge has a validation
//! function.
//!
//! Not covered: the rules about names and paths, the Target's peripherals
//! and interrupts, the Protocol form's per-label validation functions (the
//! labels a role receives come from the Protocol's text), and the Plain
//! bound of R42.1, which the compiler enforces on the emitted assertion.

use vstd::prelude::*;

verus! {

/// A Task as the acceptance conditions see it.
pub struct TaskA {
    pub priority: u8,
    pub partition: usize,
    /// Period or MIT, in ticks.
    pub period: u64,
    pub deadline: u64,
    pub budget: u32,
}

/// An Endpoint: whether it is of the plain form, its capacity, the global
/// Task index of each role, and `validates[r][f]` (the edges on which the
/// Kernel accepts a rejection report from role `r` about role `f`).
pub struct EndpointA {
    pub plain: bool,
    pub capacity: u32,
    pub roles: Vec<usize>,
    pub validates: Vec<Vec<bool>>,
}

pub open spec fn task_ok(t: TaskA, partitions: int) -> bool {
    &&& 1 <= t.priority <= 7
    &&& t.partition < partitions
    &&& t.period >= 1
    &&& 1 <= t.deadline <= t.period
    &&& t.budget >= 1
}

/// Task `f` is of another Partition than Task `r`, of lower
/// Criticality_Level (a larger level code: A is 0).
pub open spec fn upward(tasks: Seq<TaskA>, levels: Seq<u8>, r: int, f: int) -> bool {
    tasks[r].partition != tasks[f].partition
        && levels[tasks[r].partition as int] < levels[tasks[f].partition as int]
}

pub open spec fn role_ok(tasks: Seq<TaskA>, e: EndpointA, r: int) -> bool {
    e.roles@[r] < tasks.len() && e.validates@[r]@.len() == e.roles@.len()
}

/// R42.4 on one ordered pair of roles.
pub open spec fn edge_ok(tasks: Seq<TaskA>, levels: Seq<u8>, e: EndpointA, r: int, f: int) -> bool {
    let up = upward(tasks, levels, e.roles@[r] as int, e.roles@[f] as int);
    &&& e.validates@[r]@[f] ==> up
    &&& e.plain && r != f && up ==> e.validates@[r]@[f]
}

pub open spec fn endpoint_ok(tasks: Seq<TaskA>, levels: Seq<u8>, e: EndpointA) -> bool {
    &&& 2 <= e.roles@.len() <= 4
    &&& e.plain ==> e.roles@.len() == 2
    &&& e.capacity >= 1
    &&& e.validates@.len() == e.roles@.len()
    &&& forall|r: int| 0 <= r < e.roles@.len() ==> #[trigger] role_ok(tasks, e, r)
    &&& forall|r: int, f: int| 0 <= r < e.roles@.len() && 0 <= f < e.roles@.len() ==> #[trigger] edge_ok(tasks, levels, e, r, f)
}

/// **The acceptance conditions.**
pub open spec fn accepted(tasks: Seq<TaskA>, levels: Seq<u8>, endpoints: Seq<EndpointA>) -> bool {
    &&& levels.len() >= 1
    &&& forall|i: int| 0 <= i < tasks.len() ==> task_ok(#[trigger] tasks[i], levels.len() as int)
    &&& forall|i: int| 0 <= i < endpoints.len() ==> endpoint_ok(tasks, levels, #[trigger] endpoints[i])
}

fn endpoint_accept(tasks: &Vec<TaskA>, levels: &Vec<u8>, e: &EndpointA) -> (ok: bool)
    requires forall|i: int| 0 <= i < tasks.len() ==> task_ok(#[trigger] tasks@[i], levels.len() as int),
    ensures ok == endpoint_ok(tasks@, levels@, *e),
{
    let n = e.roles.len();
    if n < 2 || n > 4 || (e.plain && n != 2) || e.capacity < 1 || e.validates.len() != n {
        return false;
    }
    let mut r: usize = 0;
    while r < n
        invariant
            r <= n, n == e.roles@.len(), e.validates@.len() == n,
            forall|m: int| 0 <= m < r ==> #[trigger] role_ok(tasks@, *e, m),
        decreases n - r,
    {
        if e.roles[r] >= tasks.len() || e.validates[r].len() != n {
            assert(!role_ok(tasks@, *e, r as int));
            return false;
        }
        r = r + 1;
    }
    let mut r: usize = 0;
    while r < n
        invariant
            r <= n, n == e.roles@.len(), e.validates@.len() == n,
            forall|m: int| 0 <= m < n ==> #[trigger] role_ok(tasks@, *e, m),
            forall|i: int| 0 <= i < tasks.len() ==> task_ok(#[trigger] tasks@[i], levels.len() as int),
            forall|a: int, b: int| 0 <= a < r && 0 <= b < n ==> #[trigger] edge_ok(tasks@, levels@, *e, a, b),
        decreases n - r,
    {
        let mut f: usize = 0;
        while f < n
            invariant
                r < n, f <= n, n == e.roles@.len(), e.validates@.len() == n,
                forall|m: int| 0 <= m < n ==> #[trigger] role_ok(tasks@, *e, m),
                forall|i: int| 0 <= i < tasks.len() ==> task_ok(#[trigger] tasks@[i], levels.len() as int),
                forall|b: int| 0 <= b < f ==> #[trigger] edge_ok(tasks@, levels@, *e, r as int, b),
            decreases n - f,
        {
            assert(role_ok(tasks@, *e, r as int));
            assert(role_ok(tasks@, *e, f as int));
            let tr = e.roles[r];
            let tf = e.roles[f];
            assert(task_ok(tasks@[tr as int], levels.len() as int));
            assert(task_ok(tasks@[tf as int], levels.len() as int));
            let up = tasks[tr].partition != tasks[tf].partition && levels[tasks[tr].partition] < levels[tasks[tf].partition];
            let v = e.validates[r][f];
            if (v && !up) || (e.plain && r != f && up && !v) {
                assert(!edge_ok(tasks@, levels@, *e, r as int, f as int));
                return false;
            }
            f = f + 1;
        }
        r = r + 1;
    }
    true
}

/// Decides `accepted`.
pub fn accept(tasks: &Vec<TaskA>, levels: &Vec<u8>, endpoints: &Vec<EndpointA>) -> (ok: bool)
    ensures ok == accepted(tasks@, levels@, endpoints@),
{
    if levels.len() < 1 {
        return false;
    }
    let mut i: usize = 0;
    while i < tasks.len()
        invariant i <= tasks.len(), forall|m: int| 0 <= m < i ==> task_ok(#[trigger] tasks@[m], levels.len() as int),
        decreases tasks.len() - i,
    {
        let t = &tasks[i];
        if !(1 <= t.priority && t.priority <= 7 && t.partition < levels.len() && t.period >= 1 && 1 <= t.deadline && t.deadline <= t.period && t.budget >= 1) {
            assert(!task_ok(tasks@[i as int], levels.len() as int));
            return false;
        }
        i = i + 1;
    }
    let mut j: usize = 0;
    while j < endpoints.len()
        invariant
            j <= endpoints.len(),
            forall|m: int| 0 <= m < tasks.len() ==> task_ok(#[trigger] tasks@[m], levels.len() as int),
            forall|m: int| 0 <= m < j ==> endpoint_ok(tasks@, levels@, #[trigger] endpoints@[m]),
        decreases endpoints.len() - j,
    {
        if !endpoint_accept(tasks, levels, &endpoints[j]) {
            assert(!endpoint_ok(tasks@, levels@, endpoints@[j as int]));
            return false;
        }
        j = j + 1;
    }
    true
}

} // verus!

/// The image of what `derive` produced that `accept` decides on.
pub fn image(g: &crate::derive::Generated) -> (Vec<TaskA>, Vec<u8>, Vec<EndpointA>) {
    let tasks = g.tasks.iter().map(|t| TaskA { priority: t.priority, partition: t.partition, period: t.period_ticks, deadline: t.deadline_ticks, budget: t.budget_cycles }).collect();
    let levels = g.partitions.iter().map(|p| p.level.code()).collect();
    let endpoints = g
        .endpoints
        .iter()
        .map(|e| EndpointA { plain: e.protocol.is_none(), capacity: e.capacity, roles: e.roles.iter().map(|(_, t)| *t).collect(), validates: e.validates.clone() })
        .collect();
    (tasks, levels, endpoints)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn task(priority: u8, partition: usize) -> TaskA {
        TaskA { priority, partition, period: 10, deadline: 10, budget: 5 }
    }

    /// Two Partitions, A (level 0) and C (level 2), one Task each, and a
    /// plain Endpoint between them: the level A role validates what it
    /// receives from the level C role.
    fn base() -> (Vec<TaskA>, Vec<u8>, Vec<EndpointA>) {
        (
            vec![task(2, 0), task(1, 1)],
            vec![0, 2],
            vec![EndpointA { plain: true, capacity: 1, roles: vec![0, 1], validates: vec![vec![false, true], vec![false, false]] }],
        )
    }

    #[test]
    fn the_acceptance_check_accepts_the_base_and_refuses_each_altered_condition() {
        let (t, l, e) = base();
        assert!(accept(&t, &l, &e));
        let alter = |f: &dyn Fn(&mut Vec<TaskA>, &mut Vec<u8>, &mut Vec<EndpointA>)| {
            let (mut t, mut l, mut e) = base();
            f(&mut t, &mut l, &mut e);
            accept(&t, &l, &e)
        };
        // R22.1: a deadline later than the period, no deadline, no period, no Budget.
        assert!(!alter(&|t, _, _| t[0].deadline = 11));
        assert!(!alter(&|t, _, _| t[0].deadline = 0));
        assert!(!alter(&|t, _, _| t[1].period = 0));
        assert!(!alter(&|t, _, _| t[1].budget = 0));
        // PR-33: a Priority of 0 or above 7; a Partition that does not exist; no Partition.
        assert!(!alter(&|t, _, _| t[0].priority = 0));
        assert!(!alter(&|t, _, _| t[0].priority = 8));
        assert!(!alter(&|t, _, _| t[0].partition = 2));
        assert!(!alter(&|t, l, e| {
            t.clear();
            l.clear();
            e.clear();
        }));
        // R42.4: the upward edge without a validation function; a report
        // accepted on an edge that is not upward; a role naming no Task.
        assert!(!alter(&|_, _, e| e[0].validates[0][1] = false));
        assert!(!alter(&|_, _, e| e[0].validates[1][0] = true));
        assert!(!alter(&|_, _, e| e[0].validates[0][0] = true));
        assert!(!alter(&|_, _, e| e[0].roles[1] = 2));
        // PR-14: no capacity; one role; five roles; a plain Endpoint of three.
        assert!(!alter(&|_, _, e| e[0].capacity = 0));
        assert!(!alter(&|_, _, e| {
            e[0].roles.pop();
        }));
        assert!(!alter(&|_, _, e| {
            e[0].roles = vec![0, 1, 0];
            e[0].validates = vec![vec![false; 3]; 3];
        }));
        // The same Partition on both sides needs no validation function.
        assert!(alter(&|t, _, e| {
            t[1].partition = 0;
            e[0].validates[0][1] = false;
        }));
    }
}
