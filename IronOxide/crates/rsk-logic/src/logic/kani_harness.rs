//! Kani cross-check harnesses (R45.3; task 6.2): each runs a Kernel
//! operation's executable code on bounded nondeterministic states and
//! asserts the postcondition of its Verus specification, restated in Rust.
//! They check the erased executable code that Flight_Builds compile, with
//! the Verus ghost code removed, against the same contracts.
//!
//! Bounds (R45.8): the sizes and the configuration of the
//! Profile_Conformance_Suite's accepted Application, the P1 demo (7 Tasks,
//! 3 Resources, 3 Partitions, a lock stack of 6, 23 slots, an event log of
//! 32): `config` is its Generated_Config (`apps/p1-demo/gen/system_manifest.rs`),
//! and `cargo xtask check` fails when the two differ (checks::kani_bounds).
//! Every state the harnesses explore is therefore a state of the demo's
//! Kernel; the Verus proofs cover every size.
//!
//! Compiled only under `cfg(kani)`: not part of any Flight_Build, not
//! counted by PAR-01.
#![forbid(unsafe_code)]

use super::sched::{Sched, JOB_NONE, JOB_RUNNING, REL_NONE, REL_PENDING};
use super::{Config, PartitionCfg, ResourceCfg, TaskCfg, KIND_PERIODIC, KIND_SPORADIC, MIT_DEFER, RESP_END_JOB,
    RESP_RECORD_AND_CONTINUE, RESP_RECORD_ONLY, RESP_RESTART_PARTITION, RESP_STOP_PARTITION};

const NT: usize = 7;
const NR: usize = 3;
const NP: usize = 3;
const NL: usize = 6;
const NS: usize = 23;
const NE: usize = 32;

/// The P1 demo's Tasks, by index (the demo's Task order).
const CONTROL: u8 = 0;
const ECHO: u8 = 1;
const SENSOR: u8 = 2;
const LOG: u8 = 3;
const FILTER: u8 = 5;
const SAMPLER: u8 = 6;
/// The P1 demo's Resources.
const STATS: u8 = 2;

/// The P1 demo's Generated_Config, copied line for line from
/// `apps/p1-demo/gen/system_manifest.rs` (the `kani-bounds` check keeps
/// them equal). Priorities 7, 6, 5, 2, 4, 3, 1 in Task order: `control`
/// and `sensor` share `state` and `in_cs` (Ceiling 7) in part0, `echo`
/// and `pulse` are sporadic, `filter` and `sampler` share `stats`
/// (Ceiling 3) in part2.
fn config() -> Config<NT, NR, NP> {
    Config {
        tasks: [
            TaskCfg { priority: 7, partition: 0, kind: KIND_PERIODIC, offset: 0, period: 250000, deadline: 250000, budget: 25000, mit_policy: MIT_DEFER, fpu: false }, // part0.control
            TaskCfg { priority: 6, partition: 0, kind: KIND_SPORADIC, offset: 0, period: 500000, deadline: 500000, budget: 7500, mit_policy: MIT_DEFER, fpu: false }, // part0.echo
            TaskCfg { priority: 5, partition: 0, kind: KIND_PERIODIC, offset: 125000, period: 500000, deadline: 500000, budget: 217500, mit_policy: MIT_DEFER, fpu: false }, // part0.sensor
            TaskCfg { priority: 2, partition: 1, kind: KIND_PERIODIC, offset: 0, period: 1000000, deadline: 1000000, budget: 150000, mit_policy: MIT_DEFER, fpu: false }, // part1.log
            TaskCfg { priority: 4, partition: 1, kind: KIND_SPORADIC, offset: 0, period: 375000, deadline: 375000, budget: 12500, mit_policy: MIT_DEFER, fpu: false }, // part1.pulse
            TaskCfg { priority: 3, partition: 2, kind: KIND_PERIODIC, offset: 0, period: 625000, deadline: 625000, budget: 20000, mit_policy: MIT_DEFER, fpu: true }, // part2.filter
            TaskCfg { priority: 1, partition: 2, kind: KIND_PERIODIC, offset: 0, period: 2500000, deadline: 2500000, budget: 12500, mit_policy: MIT_DEFER, fpu: false }, // part2.sampler
        ],
        resources: [
            ResourceCfg { ceiling: 7, partition: 0 }, // part0.state
            ResourceCfg { ceiling: 7, partition: 0 }, // part0.in_cs
            ResourceCfg { ceiling: 3, partition: 2 }, // part2.stats
        ],
        access: [
            [true, true, false],
            [false, false, false],
            [true, true, false],
            [false, false, false],
            [false, false, false],
            [false, false, true],
            [false, false, true],
        ],
        partitions: [
            PartitionCfg { level: 0, fault_response: RESP_RESTART_PARTITION, overrun_response: RESP_RECORD_AND_CONTINUE, deadline_response: RESP_RECORD_ONLY, mit_threshold: 10 }, // part0
            PartitionCfg { level: 2, fault_response: RESP_STOP_PARTITION, overrun_response: RESP_END_JOB, deadline_response: RESP_RECORD_ONLY, mit_threshold: 10 }, // part1
            PartitionCfg { level: 1, fault_response: RESP_RESTART_PARTITION, overrun_response: RESP_END_JOB, deadline_response: RESP_RECORD_ONLY, mit_threshold: 10 }, // part2
        ],
        operating_duration: 5000000,
        slot_release: [0, 3, 6, 9, 12, 15, 18],
        slot_deadline: [1, 4, 7, 10, 13, 16, 19],
        slot_budget: [2, 5, 8, 11, 14, 17, 20],
        slot_count: NS as u8,
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
#[kani::unwind(8)]
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
/// by force while holding nothing, and the preempted Task resumes with
/// the ceiling restored to the maximum over the remaining locks; the
/// forced end of the holder releases its Resource. On the demo: `sampler`
/// holds `stats` (Ceiling 3), `log` (Priority 2) is blocked by the
/// Ceiling, `echo` (Priority 6) preempts.
#[kani::proof]
#[kani::unwind(8)]
fn forced_end_releases_and_resumes_lifo() {
    let cfg = config();
    let mut s = Sched::<NT, NR, NL>::new(&cfg);
    s.make_pending(&cfg, SAMPLER);
    s.start(&cfg, SAMPLER);
    assert!(s.lock(&cfg, STATS));
    assert!(s.ceiling == 3);
    s.make_pending(&cfg, LOG);
    s.make_pending(&cfg, ECHO);
    assert!(s.select(&cfg) == Some(ECHO));
    s.start(&cfg, ECHO);
    assert!(s.depth == 2 && s.stack[1] == ECHO);
    // `echo` may access nothing: every lock is refused.
    assert!(!s.lock(&cfg, 0) && !s.lock(&cfg, 1) && !s.lock(&cfg, STATS));
    // Forced end of the top Job releases nothing (it held nothing) and resumes `sampler`.
    let released = s.end_job(&cfg);
    assert!(released == 0 && s.depth == 1 && s.stack[0] == SAMPLER && s.job[ECHO as usize] == JOB_NONE);
    assert!(s.ceiling == 3);
    // Forced end of `sampler` releases `stats`; the ceiling returns to idle.
    let released = s.end_job(&cfg);
    assert!(released == 1 && s.depth == 0 && s.lock_len == 0 && s.ceiling == 0);
    assert!(s.release[LOG as usize] == REL_PENDING && s.release[ECHO as usize] == REL_NONE);
    // Now `log` is eligible.
    assert!(s.select(&cfg) == Some(LOG));
}

/// Selection never panics and respects the DD-03 tie order for any release
/// state (the demo's Priorities are distinct, so the tie clause holds
/// vacuously here; Verus proves it for every configuration).
#[kani::proof]
#[kani::unwind(8)]
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

// ------------------------------------------------------------------ intake

/// The demo under the given MIT policy for `echo` (sporadic, MIT 500,000
/// ticks, deadline 500,000): with deferral it keeps its release slot for
/// the deferral event, with discard it has none (R9.4).
fn sporadic_config(mit_policy: u8) -> Config<NT, NR, NP> {
    let mut cfg = config();
    cfg.tasks[ECHO as usize].mit_policy = mit_policy;
    if mit_policy == super::MIT_DISCARD {
        cfg.slot_release[ECHO as usize] = super::NO_SLOT;
    }
    cfg
}

/// Property 8 cross-check on `Kernel::intake` (R10.1, R10.2, R10.3, R10.7;
/// the harness design.md names): three events of one Sporadic_Task at
/// nondeterministic instants under a nondeterministic MIT policy.
///
/// 1. The first event is accepted: a Release pending and due now, the
///    previous accepted instant set, the source masked, no violation.
/// 2. A second event while that Release is still pending is an
///    MIT_Violation whatever its instant (R10.3): the Release and the
///    timing state are unchanged and one violation is counted against
///    the source Partition.
/// 3. After the Job ran and ended, a third event is accepted exactly when
///    one MIT has elapsed since the first; otherwise it is deferred until
///    `last_due + MIT` under the deferral policy or discarded under the
///    discard policy, in both cases counted as a violation and leaving the
///    previous accepted instant unchanged.
/// R9.9 with DD-06 (the HIL_Rig finding of 2026-11-01): an event too late
/// for its Job to complete within the operating duration makes no Release
/// and leaves the source masked, so that a level-triggered source no Job
/// will clear cannot re-enter the Kernel until the operating end; an event
/// in time is accepted as before.
#[kani::proof]
#[kani::unwind(25)]
fn intake_late_event_masks_source() {
    let cfg = sporadic_config(super::MIT_DEFER);
    assert!(cfg.validate());
    let mut k = super::kernel::Kernel::<NT, NR, NP, NL, NS, NE>::new(&cfg);
    k.start(&cfg);
    let t: u8 = ECHO;
    let n: u64 = kani::any();
    kani::assume(n <= cfg.operating_duration);
    k.intake(&cfg, t, 0, n);
    if n + cfg.tasks[1].deadline > cfg.operating_duration {
        assert!(k.sched.release[1] == REL_NONE);
        assert!(k.timing.masked[1]);
        assert!(!k.timing.has_last[1]);
        assert!(k.mit_violations[0] == 0);
    } else {
        assert!(k.sched.release[1] == REL_PENDING);
        assert!(k.timing.masked[1]);
    }
}

#[kani::proof]
#[kani::unwind(25)]
fn intake_outcomes_match_r10() {
    let policy: u8 = if kani::any() { super::MIT_DEFER } else { super::MIT_DISCARD };
    let cfg = sporadic_config(policy);
    assert!(cfg.validate());
    let mut k = super::kernel::Kernel::<NT, NR, NP, NL, NS, NE>::new(&cfg);
    k.start(&cfg);
    let t: u8 = ECHO;
    let sp: u8 = 0;
    let mit: u64 = cfg.tasks[1].period;

    // 1. Accepted (R10.1).
    let n1: u64 = kani::any();
    kani::assume(n1 <= 500_000);
    k.intake(&cfg, t, sp, n1);
    assert!(k.sched.release[1] == REL_PENDING);
    assert!(k.timing.has_last[1] && k.timing.last_due[1] == n1 && k.timing.due[1] == n1);
    assert!(k.timing.masked[1]);
    assert!(k.mit_violations[0] == 0);
    assert!(k.sched.release[0] == REL_NONE && k.sched.release[2] == REL_NONE);

    // 2. A second event while the Release is pending (R10.3).
    let n2: u64 = kani::any();
    kani::assume(n1 <= n2 && n2 <= 1_000_000);
    k.intake(&cfg, t, sp, n2);
    assert!(k.sched.release[1] == REL_PENDING);
    assert!(k.timing.has_last[1] && k.timing.last_due[1] == n1);
    assert!(k.mit_violations[0] == 1);

    // The Job runs and ends, so no Release of Task 1 is pending or deferred.
    assert!(k.select(&cfg) == Some(1));
    k.dispatch(&cfg, 1, 0);
    assert!(k.sched.depth == 1 && k.sched.stack[0] == 1 && k.sched.job[1] == JOB_RUNNING);
    k.job_return(&cfg, 0);
    assert!(k.sched.depth == 0 && k.sched.release[1] == REL_NONE);

    // 3. A third event: accepted, deferred, or discarded (R10.1, R10.2).
    let n3: u64 = kani::any();
    kani::assume(n2 <= n3 && n3 <= 1_000_000);
    k.intake(&cfg, t, sp, n3);
    if n3 >= n1 + mit {
        assert!(k.sched.release[1] == REL_PENDING);
        assert!(k.timing.last_due[1] == n3 && k.timing.due[1] == n3);
        assert!(k.mit_violations[0] == 1);
    } else if policy == super::MIT_DEFER {
        assert!(k.sched.release[1] == super::sched::REL_DEFERRED);
        assert!(k.timing.deferred_until[1] == n1 + mit);
        assert!(k.timing.last_due[1] == n1);
        assert!(k.mit_violations[0] == 2);
    } else {
        assert!(k.sched.release[1] == REL_NONE);
        assert!(k.timing.last_due[1] == n1);
        assert!(k.mit_violations[0] == 2);
    }
    // Other Tasks and Partitions are untouched throughout.
    assert!(k.sched.release[0] == REL_NONE && k.sched.release[2] == REL_NONE);
    assert!(k.mit_violations[1] == 0 && k.mit_violations[2] == 0);
}

/// R9.6 cross-check (KD-R9.6, the HIL_Rig finding of 2026-11-02, task
/// 7.5): a Periodic_Task with deadline = period whose Job runs past its
/// next nominal instant has its own deadline miss reported at that
/// instant, and the held Release's (R9.7) at the one after, although the
/// Task's release slot precedes its deadline slot in slot order (the
/// Generator's layout) and the Release that becomes due re-arms the
/// deadline slot. Before the fix the timer served the slots in slot order
/// and the Job's own miss was never reported. On the demo: `control`
/// (Priority 7, period 250,000, part0 RECORD_ONLY) runs on through its
/// instants; `sensor`'s Release at 125,000 is served meanwhile and stays
/// below it.
#[kani::proof]
#[kani::unwind(25)]
fn deadline_miss_of_the_running_job_is_reported_at_its_next_nominal_instant() {
    let cfg = config();
    assert!(cfg.validate());
    let c = CONTROL as usize;
    assert!(cfg.slot_release[c] < cfg.slot_deadline[c]);
    let mut k = super::kernel::Kernel::<NT, NR, NP, NL, NS, NE>::new(&cfg);
    k.start(&cfg);
    // Instant 0: the first Release of every offset-0 Periodic_Task;
    // `control`, the highest, starts and runs on.
    k.timer_event(&cfg, 0, 0);
    assert!(k.sched.release[c] == REL_PENDING && k.sched.release[LOG as usize] == REL_PENDING);
    assert!(k.sched.release[FILTER as usize] == REL_PENDING && k.sched.release[SAMPLER as usize] == REL_PENDING);
    assert!(k.sched.release[SENSOR as usize] == REL_NONE && k.sched.release[ECHO as usize] == REL_NONE);
    assert!(k.select(&cfg) == Some(CONTROL));
    k.dispatch(&cfg, CONTROL, 0);
    assert!(k.sched.depth == 1 && k.sched.stack[0] == CONTROL && k.sched.job[c] == JOB_RUNNING);
    let d = cfg.slot_deadline[c] as usize;
    assert!(k.slots.slots[d].armed && k.slots.slots[d].at == cfg.tasks[c].deadline);
    let logged = k.log.len;
    // The next nominal instant (= the running Job's absolute deadline): its
    // own miss is reported (RECORD_ONLY: the Job runs on), the Release is
    // held (R9.7) and the deadline slot holds the held Release's deadline.
    k.timer_event(&cfg, cfg.tasks[c].period, 0);
    assert!(k.sched.job[c] == JOB_RUNNING && k.sched.release[c] == REL_PENDING);
    assert!(k.slots.slots[d].armed && k.slots.slots[d].at == 2 * cfg.tasks[c].period);
    let mut misses = 0;
    let mut overlaps = 0;
    let mut i = logged;
    while i < k.log.len {
        let e = k.log.get(i);
        if e.kind == super::log::EV_DEADLINE_MISS && e.task == CONTROL && e.instant == cfg.tasks[c].period {
            misses += 1;
        }
        if e.kind == super::log::EV_RELEASE_OVERLAP && e.task == CONTROL {
            overlaps += 1;
        }
        i += 1;
    }
    assert!(misses == 1 && overlaps == 1);
    // `sensor`'s Release (125,000) was served; nothing is above `control`.
    assert!(k.sched.release[SENSOR as usize] == REL_PENDING);
    assert!(k.select(&cfg) == None);
    assert!(k.sched.depth == 1 && k.sched.stack[0] == CONTROL && k.sched.job[c] == JOB_RUNNING);
    // One period later: the held Release's deadline is missed, and the
    // nominal instant's Release is discarded (R9.8).
    let logged = k.log.len;
    k.timer_event(&cfg, 2 * cfg.tasks[c].period, 0);
    assert!(k.sched.job[c] == JOB_RUNNING && k.sched.release[c] == REL_PENDING);
    assert!(!k.slots.slots[d].armed);
    let mut misses = 0;
    let mut i = logged;
    while i < k.log.len {
        let e = k.log.get(i);
        if e.kind == super::log::EV_DEADLINE_MISS && e.task == CONTROL && e.instant == 2 * cfg.tasks[c].period {
            misses += 1;
        }
        i += 1;
    }
    assert!(misses == 1);
    assert!(k.mit_violations[0] == 1);
}

/// R18.1 cross-check (task 19.1, the Critical_Section measurements'
/// finding of 2026-11-02): the executing Job is charged the cycles it ran
/// at every timed event, not only at its Budget event. On the demo,
/// `sampler` (Priority 1, Budget 12,500, part2 END_JOB) alone at instant
/// 0, the other offset-0 Tasks moved to 300, 600, and 900: `sampler`'s
/// Job runs across `control`'s Release at 300 (nothing of its own due),
/// where its 40 cycles go to its account; at its Budget instant the
/// account reads 12,510 > 12,500: one Overrun, recorded once with the
/// full account, and END_JOB ends the Job.
#[kani::proof]
#[kani::unwind(25)]
fn running_cycles_are_charged_at_every_timed_event() {
    let mut cfg = config();
    cfg.tasks[CONTROL as usize].offset = 300;
    cfg.tasks[LOG as usize].offset = 600;
    cfg.tasks[FILTER as usize].offset = 900;
    assert!(cfg.validate());
    let s = SAMPLER as usize;
    let mut k = super::kernel::Kernel::<NT, NR, NP, NL, NS, NE>::new(&cfg);
    k.start(&cfg);
    k.timer_event(&cfg, 0, 0);
    assert!(k.select(&cfg) == Some(SAMPLER));
    k.dispatch(&cfg, SAMPLER, 0);
    assert!(k.budget.used[s] == 0);
    // `control`'s Release at 300: `sampler` ran 40 cycles since its
    // dispatch, charged on entry although none of its own events is due.
    k.timer_event(&cfg, 300, 40);
    assert!(k.budget.used[s] == 40);
    assert!(k.sched.release[CONTROL as usize] == REL_PENDING);
    assert!(k.sched.job[s] == JOB_RUNNING);
    // The Budget event at 12,500 (armed at the dispatch): 12,470 more
    // cycles; the account reads 12,510 > 12,500: one Overrun, END_JOB
    // ends the Job.
    let logged = k.log.len;
    k.timer_event(&cfg, cfg.tasks[s].budget as u64, 12_470);
    assert!(k.budget.used[s] == 12_510);
    let mut overruns = 0;
    let mut i = logged;
    while i < k.log.len {
        if k.log.get(i).kind == super::log::EV_OVERRUN && k.log.get(i).task == SAMPLER {
            overruns += 1;
        }
        i += 1;
    }
    assert!(overruns == 1);
    assert!(k.sched.job[s] == JOB_NONE);
}

/// Task 5.5 (R17.1, R17.2): the executable DMA buffer check agrees with a
/// direct recomputation of its specification for every layout of a 1 KiB
/// to 64 KiB Partition RAM and every buffer, as a cross-check of the Verus
/// proof `dma_permitted_exec` on the erased code.
#[kani::proof]
#[kani::unwind(33)]
fn dma_permitted_matches_recomputation() {
    use super::mpu::{Interval, PartitionLayout, MAX_WINDOWS};
    let size_log2: u8 = kani::any();
    kani::assume(size_log2 >= 10 && size_log2 <= 16);
    let size: u32 = 1u32 << size_log2;
    let base: u32 = kani::any();
    kani::assume(base % size == 0 && base >= 0x2000_0000 && base < 0x2004_0000);
    let stack: u32 = kani::any();
    kani::assume(stack > 0 && stack <= size);
    let srd: u8 = kani::any();
    let no_window = Interval { base: 0, size_log2: 5 };
    let pl = PartitionLayout {
        code: Interval { base: 0x0002_0000, size_log2: 15 },
        code_srd: 0,
        ram: Interval { base, size_log2 },
        ram_srd: srd,
        stack_size: stack,
        windows: [no_window; MAX_WINDOWS],
        window_count: 0,
        windows_ro: [false; MAX_WINDOWS],
    };
    let ptr: u32 = kani::any();
    let len: u32 = kani::any();
    kani::assume(len <= 0x2_0000);
    let got = pl.dma_permitted_exec(ptr, len);
    let p = ptr as u64;
    let l = len as u64;
    let b = base as u64;
    let sz = size as u64;
    let sub = sz / 8;
    let mut want = len > 0 && p >= b + stack as u64 && p + l <= b + sz;
    let mut s = 0u64;
    while s < 8 {
        let lo = b + s * sub;
        let hi = lo + sub;
        if lo < p + l && p < hi && (srd >> s) & 1 == 1 {
            want = false;
        }
        s += 1;
    }
    assert!(got == want);
}
