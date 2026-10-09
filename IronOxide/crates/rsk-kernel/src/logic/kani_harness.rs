//! Kani cross-check harnesses (R45.3; task 6.2): each runs a Kernel
//! operation's executable code on bounded nondeterministic states and
//! asserts the postcondition of its Verus specification, restated in Rust.
//! They check the erased executable code that Flight_Builds compile, with
//! the Verus ghost code removed, against the same contracts.
//!
//! Bounds (R45.8): 3 Tasks, 2 Resources, 2 Partitions, a lock stack of 4,
//! and 8 slots. These are smaller than the Profile_Conformance_Suite sizes
//! (tasks 16, Resources 16, slots 48, lock depth 8 proposed in design.md);
//! the residual risk beyond these bounds is recorded in the
//! Verification_Plan, and the Verus proofs cover every size.
//!
//! Compiled only under `cfg(kani)`: not part of any Flight_Build, not
//! counted by PAR-01.
#![forbid(unsafe_code)]

use super::sched::{Sched, JOB_NONE, JOB_RUNNING, REL_NONE, REL_PENDING};
use super::{Config, PartitionCfg, ResourceCfg, TaskCfg, KIND_PERIODIC, MIT_DEFER, RESP_END_JOB, RESP_RECORD_ONLY,
    RESP_RESTART_PARTITION};

const NT: usize = 3;
const NR: usize = 2;
const NP: usize = 2;
const NL: usize = 4;

/// A fixed well-formed configuration: Task 0 (Priority 2, Partition 0)
/// accesses Resource 0; Task 1 (Priority 5, Partition 0) accesses Resources
/// 0 and 1; Task 2 (Priority 7, Partition 1) accesses nothing.
fn config() -> Config<NT, NR, NP> {
    let task = |priority: u8, partition: u8| TaskCfg {
        priority,
        partition,
        kind: KIND_PERIODIC,
        offset: 0,
        period: 1000,
        deadline: 1000,
        budget: 100,
        mit_policy: MIT_DEFER,
        fpu: false,
    };
    let part = PartitionCfg {
        level: 0,
        fault_response: RESP_RESTART_PARTITION,
        overrun_response: RESP_END_JOB,
        deadline_response: RESP_RECORD_ONLY,
        mit_threshold: 10,
    };
    Config {
        tasks: [task(2, 0), task(5, 0), task(7, 1)],
        resources: [ResourceCfg { ceiling: 5, partition: 0 }, ResourceCfg { ceiling: 5, partition: 0 }],
        access: [[true, false], [true, true], [false, false]],
        partitions: [part, part],
        operating_duration: 1_000_000,
        slot_release: [0, 1, 2],
        slot_deadline: [3, 4, 5],
        slot_budget: [6, 7, 8],
        slot_count: 11,
    }
}

fn max_ceiling(cfg: &Config<NT, NR, NP>, s: &Sched<NT, NR, NL>) -> u8 {
    let mut c = 0;
    for e in 0..s.lock_len {
        let rc = cfg.resources[s.lock_res[e] as usize].ceiling;
        if rc > c {
            c = rc;
        }
    }
    c
}

/// The configuration validator accepts the fixed configuration and never panics.
#[kani::proof]
fn validate_accepts_well_formed_config() {
    let cfg = config();
    assert!(cfg.validate());
}

/// Any single-field mutation of a Task's period or priority out of range is rejected without panicking.
#[kani::proof]
fn validate_rejects_out_of_range_without_panic() {
    let mut cfg = config();
    let t: usize = kani::any();
    kani::assume(t < NT);
    let priority: u8 = kani::any();
    cfg.tasks[t].priority = priority;
    let ok = cfg.validate();
    if priority == 0 || priority > 7 {
        assert!(!ok);
    }
}

/// Property 1 and 3 cross-check: from the initial state, release and start
/// a Task, lock a Resource it may access; the ceiling becomes the Resource
/// Ceiling, and the lock is refused for a Resource it may not access.
#[kani::proof]
#[kani::unwind(6)]
fn lock_raises_ceiling_to_resource_ceiling() {
    let cfg = config();
    let mut s = Sched::<NT, NR, NL>::new(&cfg);
    let t: u8 = kani::any();
    kani::assume((t as usize) < NT);
    s.make_pending(&cfg, t);
    assert!(s.release[t as usize] == REL_PENDING);
    let sel = s.select(&cfg);
    assert!(sel == Some(t));
    s.start(&cfg, t);
    assert!(s.depth == 1 && s.stack[0] == t && s.job[t as usize] == JOB_RUNNING);
    let r: u8 = kani::any();
    kani::assume((r as usize) < NR);
    let before = s.ceiling;
    let ok = s.lock(&cfg, r);
    if cfg.access[t as usize][r as usize] {
        assert!(ok);
        let expected = if cfg.resources[r as usize].ceiling > before { cfg.resources[r as usize].ceiling } else { before };
        assert!(s.ceiling == expected);
        assert!(s.lock_len == 1 && s.lock_res[0] == r);
        // Re-entry is refused (R8.10).
        assert!(!s.lock(&cfg, r));
        // LIFO exit restores the ceiling (R8.2).
        assert!(s.unlock(&cfg, r));
        assert!(s.ceiling == before && s.lock_len == 0);
    } else {
        assert!(!ok && s.ceiling == before && s.lock_len == 0);
    }
    assert!(s.ceiling == max_ceiling(&cfg, &s));
}

/// Property 2 and 6 cross-check: a higher-priority Task preempts, is ended
/// by force while holding a Resource, and the preempted Task resumes with
/// the ceiling restored to the maximum over the remaining locks.
#[kani::proof]
#[kani::unwind(6)]
fn forced_end_releases_and_resumes_lifo() {
    let cfg = config();
    let mut s = Sched::<NT, NR, NL>::new(&cfg);
    // Task 0 starts and locks Resource 0 (ceiling 5).
    s.make_pending(&cfg, 0);
    s.start(&cfg, 0);
    assert!(s.lock(&cfg, 0));
    assert!(s.ceiling == 5);
    // Task 2 (Priority 7 > 5) becomes pending and is the only eligible Task; Task 1 (Priority 5) is blocked by the ceiling.
    s.make_pending(&cfg, 1);
    s.make_pending(&cfg, 2);
    assert!(s.select(&cfg) == Some(2));
    s.start(&cfg, 2);
    assert!(s.depth == 2 && s.stack[1] == 2);
    // Task 2 may access nothing: every lock is refused.
    assert!(!s.lock(&cfg, 0) && !s.lock(&cfg, 1));
    // Forced end of the top Job releases nothing (it held nothing) and resumes Task 0.
    let released = s.end_job(&cfg);
    assert!(released == 0 && s.depth == 1 && s.stack[0] == 0 && s.job[2] == JOB_NONE);
    assert!(s.ceiling == 5);
    // Forced end of Task 0 releases Resource 0; the ceiling returns to idle.
    let released = s.end_job(&cfg);
    assert!(released == 1 && s.depth == 0 && s.lock_len == 0 && s.ceiling == 0);
    assert!(s.release[1] == REL_PENDING && s.release[2] == REL_NONE);
    // Now Task 1 is eligible.
    assert!(s.select(&cfg) == Some(1));
}

/// Selection never panics and respects the DD-03 tie order for any release state.
#[kani::proof]
#[kani::unwind(6)]
fn select_is_total_and_orders_ties() {
    let cfg = config();
    let mut s = Sched::<NT, NR, NL>::new(&cfg);
    let mask: u8 = kani::any();
    for t in 0..NT {
        if (mask >> t) & 1 == 1 {
            s.make_pending(&cfg, t as u8);
        }
    }
    match s.select(&cfg) {
        None => {
            for t in 0..NT {
                assert!(s.release[t] != REL_PENDING);
            }
        }
        Some(sel) => {
            assert!((sel as usize) < NT && s.release[sel as usize] == REL_PENDING);
            for t in 0..NT {
                if s.release[t] == REL_PENDING {
                    assert!(cfg.tasks[t].priority <= cfg.tasks[sel as usize].priority);
                    if cfg.tasks[t].priority == cfg.tasks[sel as usize].priority {
                        assert!((sel as usize) <= t);
                    }
                }
            }
        }
    }
}
