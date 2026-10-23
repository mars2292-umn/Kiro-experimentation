//! Portable Kernel state machine (Component B; design.md "Kernel logic and
//! Kernel_Arch"), written inside `verus!` so that the Kernel_Proofs verify
//! the very code that the Flight_Build compiles after ghost erasure
//! (ORQ-08).
//!
//! The logic is sequential (DD-02: Kernel code is non-reentrant) and single
//! core (R27.4: every proof below carries that assumption as the shape of
//! the state, one preemption stack and one System_Ceiling). It emits no
//! hardware effect itself: every transition returns, or leaves in the
//! state, what Kernel_Arch must apply to the hardware (DD-01 actions).
//!
//! Modules:
//!
//! - [`sched`]: the preemption stack, the lock stack, the System_Ceiling,
//!   and the transitions `start`, `complete`, `lock`, `unlock`, `end_job`,
//!   `kill` (Properties 1 to 6, R7, R8).
//! - [`time`]: the 64-bit time base, the timed-event slot table, periodic
//!   release arithmetic, MIT enforcement with deferral and discard, and the
//!   operating-duration limit (Properties 7, 8, 11; R9, R10).
//! - [`budget`]: Budget accounting and Overrun detection (Property 10, R18).
//! - [`log`]: the Health_Monitor event log ring with the saturating
//!   overflow counter (Property 11 part; R14.6, R14.7).
//! - [`mpu`]: the Partition MPU view computation (Property 12; R16.5,
//!   R16.6).
//!
//! Identifiers are plain integers (`u8` Task, Partition, and Resource
//! identifiers; `u64` instants in time-base ticks; `u32` CPU cycles) so
//! that the generated `Config` tables are constant data (R24.6) and the
//! specifications stay first order.
#![forbid(unsafe_code)]

use vstd::prelude::*;

pub mod budget;
#[cfg(kani)]
pub mod kani_harness;
pub mod kernel;
pub mod log;
pub mod mpu;
pub mod sched;
pub mod time;

verus! {

/// The most urgent Task Priority (DD-02: NVIC levels 1 to 7 are Task
/// levels; level 0 is the Kernel's).
pub const MAX_PRIORITY: u8 = 7;

/// The idle System_Ceiling (no Resource held).
pub const IDLE_CEILING: u8 = 0;

/// Release kinds.
pub const KIND_PERIODIC: u8 = 0;
pub const KIND_SPORADIC: u8 = 1;

/// MIT policies of a Sporadic_Task (R10.2).
pub const MIT_DEFER: u8 = 0;
pub const MIT_DISCARD: u8 = 1;

/// The closed response sets (DD-10, R14.5).
pub const RESP_END_JOB: u8 = 0;
pub const RESP_RESTART_PARTITION: u8 = 1;
pub const RESP_STOP_PARTITION: u8 = 2;
pub const RESP_SAFE_STATE: u8 = 3;
pub const RESP_RECORD_ONLY: u8 = 4;
pub const RESP_RECORD_AND_CONTINUE: u8 = 5;

/// "No slot" in the slot tables.
pub const NO_SLOT: u8 = 0xFF;

/// The configuration of one Task (Generated_Config task table, R24.1).
#[derive(Clone, Copy)]
pub struct TaskCfg {
    /// 1..=MAX_PRIORITY; larger is more urgent.
    pub priority: u8,
    pub partition: u8,
    /// KIND_PERIODIC or KIND_SPORADIC.
    pub kind: u8,
    /// Offset O in ticks (periodic only).
    pub offset: u64,
    /// Period T or MIT in ticks.
    pub period: u64,
    /// Relative deadline D in ticks, 0 < D <= period.
    pub deadline: u64,
    /// Budget in CPU cycles.
    pub budget: u32,
    /// MIT_DEFER or MIT_DISCARD (sporadic only).
    pub mit_policy: u8,
    pub fpu: bool,
}

/// The configuration of one Resource.
#[derive(Clone, Copy)]
pub struct ResourceCfg {
    /// Ceiling(r): the maximum Priority of the declared accessors (R23.1).
    pub ceiling: u8,
    pub partition: u8,
}

/// The configuration of one Partition (R15.1).
#[derive(Clone, Copy)]
pub struct PartitionCfg {
    /// Criticality_Level: 0 = A, 1 = B, 2 = C, 3 = D, 4 = E.
    pub level: u8,
    pub fault_response: u8,
    pub overrun_response: u8,
    pub deadline_response: u8,
    pub mit_threshold: u32,
}

/// Generated_Config as the logic sees it: `NT` Tasks, `NR` Resources, `NP`
/// Partitions.
pub struct Config<const NT: usize, const NR: usize, const NP: usize> {
    pub tasks: [TaskCfg; NT],
    pub resources: [ResourceCfg; NR],
    /// `access[t][r]`: Task `t` is a declared accessor of Resource `r`
    /// (R8.7).
    pub access: [[bool; NR]; NT],
    pub partitions: [PartitionCfg; NP],
    /// The maximum continuous operating duration in ticks (R9.9), below
    /// 2^62 so that instant arithmetic never overflows (R9.5).
    pub operating_duration: u64,
    /// Timed-event slot of each Task's periodic release (periodic) or MIT
    /// deferral (sporadic with deferral), `NO_SLOT` otherwise (R9.4).
    pub slot_release: [u8; NT],
    /// Timed-event slot of each Task's deadline event (R9.4).
    pub slot_deadline: [u8; NT],
    /// Timed-event slot of each Task's Budget event (R9.4).
    pub slot_budget: [u8; NT],
    /// Number of slots: the declared events plus the end-of-operation and
    /// housekeeping slots (the last two).
    pub slot_count: u8,
}

impl<const NT: usize, const NR: usize, const NP: usize> Config<NT, NR, NP> {
    pub open spec fn priority(&self, t: int) -> u8 {
        self.tasks[t].priority
    }

    pub open spec fn ceiling(&self, r: int) -> u8 {
        self.resources[r].ceiling
    }

    pub open spec fn may_access(&self, t: int, r: int) -> bool {
        self.access[t][r]
    }

    /// One Task's declared values are in range (PR-27, R22.1).
    pub open spec fn task_ok(&self, t: int) -> bool {
        &&& 1 <= self.tasks[t].priority <= MAX_PRIORITY
        &&& (self.tasks[t].partition as int) < NP
        &&& (self.tasks[t].kind == KIND_PERIODIC || self.tasks[t].kind == KIND_SPORADIC)
        &&& self.tasks[t].period > 0
        &&& 0 < self.tasks[t].deadline <= self.tasks[t].period
        &&& self.tasks[t].budget > 0
        &&& (self.tasks[t].mit_policy == MIT_DEFER || self.tasks[t].mit_policy == MIT_DISCARD)
    }

    /// Ceiling(r) is the maximum Priority over the declared accessors
    /// (R23.1), every accessor belongs to the Resource's Partition (PR-30,
    /// R15.3), and the Resource has at least one accessor (R22.5).
    pub open spec fn resource_ok(&self, r: int) -> bool {
        &&& (self.resources[r].partition as int) < NP
        &&& 1 <= self.resources[r].ceiling <= MAX_PRIORITY
        &&& forall|t: int| 0 <= t < NT && self.access[t][r] ==> self.tasks[t].priority <= self.resources[r].ceiling
        &&& forall|t: int| 0 <= t < NT && self.access[t][r] ==> self.tasks[t].partition == self.resources[r].partition
        &&& exists|t: int| 0 <= t < NT && self.access[t][r] && self.tasks[t].priority == self.resources[r].ceiling
    }

    /// The responses of one Partition come from the closed sets of R14.5,
    /// and RECORD_AND_CONTINUE is declared only by a Partition of the
    /// highest Criticality_Level (R18.5, R18.6; level 0 is A).
    pub open spec fn partition_ok(&self, p: int) -> bool {
        let pc = self.partitions[p];
        &&& pc.level <= 4
        &&& (pc.fault_response == RESP_END_JOB || pc.fault_response == RESP_RESTART_PARTITION
            || pc.fault_response == RESP_STOP_PARTITION || pc.fault_response == RESP_SAFE_STATE)
        &&& (pc.overrun_response == RESP_END_JOB || pc.overrun_response == RESP_RESTART_PARTITION
            || pc.overrun_response == RESP_STOP_PARTITION || pc.overrun_response == RESP_SAFE_STATE
            || pc.overrun_response == RESP_RECORD_AND_CONTINUE)
        &&& (pc.deadline_response == RESP_RESTART_PARTITION || pc.deadline_response == RESP_STOP_PARTITION
            || pc.deadline_response == RESP_SAFE_STATE || pc.deadline_response == RESP_RECORD_ONLY)
        &&& (pc.overrun_response == RESP_RECORD_AND_CONTINUE
            ==> forall|q: int| 0 <= q < NP ==> self.partitions[q].level >= pc.level)
    }

    /// The slot tables: every Task has distinct deadline and Budget slots,
    /// every periodic Task and every deferring sporadic Task has a release
    /// slot, all below the two reserved slots (R9.4, R23.4).
    pub open spec fn slot_ok(&self, t: int) -> bool {
        &&& (self.slot_deadline[t] as int) < self.slot_count - 2
        &&& (self.slot_budget[t] as int) < self.slot_count - 2
        &&& self.slot_deadline[t] != self.slot_budget[t]
        &&& (self.tasks[t].kind == KIND_PERIODIC || self.tasks[t].mit_policy == MIT_DEFER)
            ==> (self.slot_release[t] as int) < self.slot_count - 2
                && self.slot_release[t] != self.slot_deadline[t] && self.slot_release[t] != self.slot_budget[t]
        &&& (self.tasks[t].kind == KIND_SPORADIC && self.tasks[t].mit_policy == MIT_DISCARD) ==> self.slot_release[t] == NO_SLOT
    }

    pub open spec fn slots_distinct(&self, t: int, u: int) -> bool {
        &&& self.slot_deadline[t] != self.slot_deadline[u]
        &&& self.slot_budget[t] != self.slot_budget[u]
        &&& self.slot_deadline[t] != self.slot_budget[u]
        &&& (self.slot_release[t] != NO_SLOT ==> self.slot_release[t] != self.slot_deadline[u]
            && self.slot_release[t] != self.slot_budget[u] && self.slot_release[t] != self.slot_release[u])
    }

    pub open spec fn slot_of_end(&self) -> int {
        self.slot_count - 2
    }

    pub open spec fn slot_of_housekeeping(&self) -> int {
        self.slot_count - 1
    }

    /// Well-formedness of the whole configuration: what the Generator
    /// guarantees (Requirements 21 to 23) and the boot check re-validates
    /// (R12.3). Every Kernel transition requires it.
    pub open spec fn wf(&self) -> bool {
        &&& 0 < NT <= 255
        &&& NR <= 255
        &&& 0 < NP <= 255
        &&& forall|t: int| 0 <= t < NT ==> self.task_ok(t)
        &&& forall|r: int| 0 <= r < NR ==> self.resource_ok(r)
        &&& forall|p: int| 0 <= p < NP ==> self.partition_ok(p)
        &&& 2 <= self.slot_count
        &&& forall|t: int| 0 <= t < NT ==> self.slot_ok(t)
        &&& forall|t: int, u: int| 0 <= t < NT && 0 <= u < NT && t != u ==> self.slots_distinct(t, u)
        &&& 0 < self.operating_duration < 0x4000_0000_0000_0000
        &&& forall|t: int| 0 <= t < NT ==> self.tasks[t].offset <= self.operating_duration
        &&& forall|t: int| 0 <= t < NT ==> self.tasks[t].period <= self.operating_duration
    }

    /// Executable validation of `wf` (R12.3, boot-time configuration
    /// validation of the tables): returns `true` exactly when the
    /// configuration is well formed.
    pub fn validate(&self) -> (ok: bool)
        ensures ok == self.wf(),
    {
        if !(0 < NT && NT <= 255 && NR <= 255 && 0 < NP && NP <= 255
            && 0 < self.operating_duration && self.operating_duration < 0x4000_0000_0000_0000)
        {
            return false;
        }
        let mut t: usize = 0;
        while t < NT
            invariant
                t <= NT,
                0 < NT <= 255, NR <= 255, 0 < NP <= 255, 0 < self.operating_duration < 0x4000_0000_0000_0000,
                forall|i: int| 0 <= i < t ==> self.task_ok(i),
                forall|i: int| 0 <= i < t ==> self.tasks[i].offset <= self.operating_duration,
                forall|i: int| 0 <= i < t ==> self.tasks[i].period <= self.operating_duration,
            decreases NT - t,
        {
            let task = self.tasks[t];
            let ok = 1 <= task.priority && task.priority <= MAX_PRIORITY
                && (task.partition as usize) < NP
                && (task.kind == KIND_PERIODIC || task.kind == KIND_SPORADIC)
                && task.period > 0
                && 0 < task.deadline && task.deadline <= task.period
                && task.budget > 0
                && (task.mit_policy == MIT_DEFER || task.mit_policy == MIT_DISCARD)
                && task.offset <= self.operating_duration
                && task.period <= self.operating_duration;
            if !ok {
                proof {
                    assert(!self.task_ok(t as int) || self.tasks[t as int].offset > self.operating_duration
                        || self.tasks[t as int].period > self.operating_duration);
                }
                return false;
            }
            t = t + 1;
        }
        let mut r: usize = 0;
        while r < NR
            invariant
                r <= NR,
                0 < NT <= 255, NR <= 255, 0 < NP <= 255, 0 < self.operating_duration < 0x4000_0000_0000_0000,
                forall|i: int| 0 <= i < NT ==> self.task_ok(i),
                forall|i: int| 0 <= i < NT ==> self.tasks[i].offset <= self.operating_duration,
                forall|i: int| 0 <= i < NT ==> self.tasks[i].period <= self.operating_duration,
                forall|i: int| 0 <= i < r ==> self.resource_ok(i),
            decreases NR - r,
        {
            let res = self.resources[r];
            if !((res.partition as usize) < NP && 1 <= res.ceiling && res.ceiling <= MAX_PRIORITY) {
                proof {
                    assert(!self.resource_ok(r as int));
                }
                return false;
            }
            // Every accessor is at or below the ceiling and in the
            // Resource's Partition; some accessor attains the ceiling.
            let mut t: usize = 0;
            let mut attained = false;
            while t < NT
                invariant
                    t <= NT, r < NR,
                    res == self.resources[r as int],
                    0 < NT <= 255, NR <= 255, 0 < NP <= 255,
                    forall|i: int| 0 <= i < NT ==> self.task_ok(i),
                    forall|i: int| 0 <= i < t && self.access[i][r as int] ==> self.tasks[i].priority <= self.resources[r as int].ceiling,
                    forall|i: int| 0 <= i < t && self.access[i][r as int] ==> self.tasks[i].partition == self.resources[r as int].partition,
                    attained == exists|i: int| 0 <= i < t && self.access[i][r as int] && self.tasks[i].priority == self.resources[r as int].ceiling,
                decreases NT - t,
            {
                if self.access[t][r] {
                    let task = self.tasks[t];
                    if task.priority > res.ceiling || task.partition != res.partition {
                        proof {
                            assert(!self.resource_ok(r as int));
                        }
                        return false;
                    }
                    if task.priority == res.ceiling {
                        attained = true;
                        proof {
                            assert(self.access[t as int][r as int] && self.tasks[t as int].priority == self.resources[r as int].ceiling);
                        }
                    }
                }
                proof {
                    // The existential over [0, t + 1) is the one over [0, t) or the witness t.
                    if !attained {
                        assert forall|i: int| 0 <= i < t + 1 implies !(self.access[i][r as int] && self.tasks[i].priority == self.resources[r as int].ceiling) by {
                            if i < t {
                                assert(!(exists|k: int| 0 <= k < t && self.access[k][r as int] && self.tasks[k].priority == self.resources[r as int].ceiling));
                            }
                        }
                    }
                }
                t = t + 1;
            }
            if !attained {
                proof {
                    assert(!self.resource_ok(r as int));
                }
                return false;
            }
            r = r + 1;
        }
        if !self.validate_partitions() {
            return false;
        }
        if !self.validate_slots() {
            return false;
        }
        true
    }

    /// Executable check of `partition_ok` for every Partition.
    pub fn validate_partitions(&self) -> (ok: bool)
        requires 0 < NP <= 255,
        ensures ok == forall|p: int| 0 <= p < NP ==> self.partition_ok(p),
    {
        let mut p: usize = 0;
        while p < NP
            invariant p <= NP, 0 < NP <= 255,
                forall|i: int| 0 <= i < p ==> self.partition_ok(i),
            decreases NP - p,
        {
            let pc = self.partitions[p];
            let fault_ok = pc.fault_response == RESP_END_JOB || pc.fault_response == RESP_RESTART_PARTITION
                || pc.fault_response == RESP_STOP_PARTITION || pc.fault_response == RESP_SAFE_STATE;
            let overrun_ok = pc.overrun_response == RESP_END_JOB || pc.overrun_response == RESP_RESTART_PARTITION
                || pc.overrun_response == RESP_STOP_PARTITION || pc.overrun_response == RESP_SAFE_STATE
                || pc.overrun_response == RESP_RECORD_AND_CONTINUE;
            let deadline_ok = pc.deadline_response == RESP_RESTART_PARTITION || pc.deadline_response == RESP_STOP_PARTITION
                || pc.deadline_response == RESP_SAFE_STATE || pc.deadline_response == RESP_RECORD_ONLY;
            if !(pc.level <= 4 && fault_ok && overrun_ok && deadline_ok) {
                proof {
                    assert(!self.partition_ok(p as int));
                }
                return false;
            }
            if pc.overrun_response == RESP_RECORD_AND_CONTINUE {
                // Every other Partition must be at the same or a lower Criticality_Level.
                let mut q: usize = 0;
                while q < NP
                    invariant q <= NP, p < NP, pc == self.partitions[p as int],
                        pc.overrun_response == RESP_RECORD_AND_CONTINUE,
                        forall|j: int| 0 <= j < q ==> self.partitions[j].level >= pc.level,
                    decreases NP - q,
                {
                    if self.partitions[q].level < pc.level {
                        proof {
                            assert(self.partitions[q as int].level < self.partitions[p as int].level);
                            assert(!self.partition_ok(p as int));
                        }
                        return false;
                    }
                    q = q + 1;
                }
            }
            proof {
                assert(self.partition_ok(p as int));
            }
            p = p + 1;
        }
        true
    }

    /// Executable check of the slot tables.
    pub fn validate_slots(&self) -> (ok: bool)
        requires 0 < NT <= 255,
        ensures ok == (2 <= self.slot_count
            && (forall|t: int| 0 <= t < NT ==> self.slot_ok(t))
            && (forall|t: int, u: int| 0 <= t < NT && 0 <= u < NT && t != u ==> self.slots_distinct(t, u))),
    {
        if self.slot_count < 2 {
            return false;
        }
        let limit = (self.slot_count - 2) as usize;
        let mut t: usize = 0;
        while t < NT
            invariant t <= NT, 0 < NT <= 255, self.slot_count >= 2, limit == self.slot_count - 2,
                forall|i: int| 0 <= i < t ==> self.slot_ok(i),
                forall|i: int, u: int| 0 <= i < t && 0 <= u < NT && i != u ==> self.slots_distinct(i, u),
            decreases NT - t,
        {
            let task = self.tasks[t];
            let d = self.slot_deadline[t];
            let b = self.slot_budget[t];
            let r = self.slot_release[t];
            let needs_release = task.kind == KIND_PERIODIC || task.mit_policy == MIT_DEFER;
            let discard = task.kind == KIND_SPORADIC && task.mit_policy == MIT_DISCARD;
            let ok = (d as usize) < limit && (b as usize) < limit && d != b
                && (!needs_release || ((r as usize) < limit && r != d && r != b))
                && (!discard || r == NO_SLOT);
            if !ok {
                proof {
                    assert(!self.slot_ok(t as int));
                }
                return false;
            }
            let mut u: usize = 0;
            while u < NT
                invariant u <= NT, t < NT, 0 < NT <= 255,
                    d == self.slot_deadline[t as int], b == self.slot_budget[t as int], r == self.slot_release[t as int],
                    forall|j: int| 0 <= j < u && j != t ==> self.slots_distinct(t as int, j),
                decreases NT - u,
            {
                if u != t {
                    let du = self.slot_deadline[u];
                    let bu = self.slot_budget[u];
                    let ru = self.slot_release[u];
                    let distinct = d != du && b != bu && d != bu
                        && (r == NO_SLOT || (r != du && r != bu && r != ru));
                    if !distinct {
                        proof {
                            assert(!self.slots_distinct(t as int, u as int));
                        }
                        return false;
                    }
                }
                u = u + 1;
            }
            t = t + 1;
        }
        true
    }
}

} // verus!
