//! The composed Kernel state machine: the five entry points of the design
//! (dispatch stub entry and exit, SVC, timer event, peripheral intake, and
//! fault) over the verified components, plus the Health_Monitor's closed
//! response sets (DD-10; R9, R10, R14, R18).
//!
//! Every entry point requires and re-establishes `inv`: the invariants of
//! the scheduler (`Sched::inv`), the timing state (`Timing::inv`), and the
//! log, together with the slot-table validity of the configuration.
//!
//! The entry points also state their behaviour, lifted from the component
//! specifications: `dispatch`, `job_return`, `svc_lock`, `svc_unlock`, and
//! `select` carry the postconditions of `Sched::start`, `finish_top`,
//! `lock`, `unlock`, and `select`, with the Budget attribution of
//! Property 10 and the Budget and deadline events they arm or disarm;
//! `fault` states the safe state for a Kernel fault and, through
//! `response_applied`, the effect of each closed response on the faulting
//! Partition (R14.5) and that every response but SAFE_STATE leaves every
//! other Partition unchanged (R14.9, `others_unchanged`); `start`,
//! `enter_safe_state`, and the guards of `intake` and `timer_event` state
//! what they leave untouched.
//!
//! Property 8 (MIT enforcement). `inv` requires that a deferred Release
//! belongs to a Sporadic_Task under the deferral policy and becomes due
//! exactly one MIT after the previous accepted Release (R10.2). `intake`
//! states each outcome of R10.1 to R10.3: accepted (`accepted`), deferred
//! (`deferred`), or discarded, the last two with one MIT_Violation against
//! the source Partition (`violation_applied`, R10.7): below the threshold
//! the violation is counted and nothing else changes, so a discarded
//! event leaves the existing Release unchanged; past it the Partition's
//! fault response applies (R10.4). `periodic_release` states R9.8 the same
//! way, and `deferred_release` the deferred Release becoming due. Every
//! entry point preserves `mit_spaced`: the previous accepted due instant
//! of a Sporadic_Task is never forgotten and only moves at least one MIT
//! later, and only an accepted Release moves it, to its own due instant.
//! That two accepted Releases never become due less than one MIT apart
//! follows by induction over a run of entry points; stating it as one
//! theorem over the ghost history (`history`, task 4.14) is open.
//!
//! The ghost history (task 4.11). `history` records every scheduling
//! transition as a `Step` (`super::history`): dispatch, Job end, lock,
//! unlock, Release due, kill. `inv` carries `history_wf`; every entry
//! point extends the history (`history_extends`) and records exactly its
//! own transitions. The field is ghost: a Flight_Build carries nothing.
//!
//! Not stated here: the Releases that `timer_event` makes (Property 7's
//! link from slots to nominal instants) and the interrupt-source bound of
//! Property 9. The responses that `timer_event` and `intake` apply
//! (deadline miss, Overrun, MIT_Violation threshold) go through
//! `apply_response` and so satisfy R14.9 relative to the state in which
//! each is applied.
//! Kernel_Arch calls these functions with PRIMASK held or at the Kernel
//! level (DD-02), reads the resulting state (System_Ceiling, next compare
//! instant, source masks, Partition flags), and applies it to the hardware.
//!
//! Hardware actions that the logic cannot perform are recorded as flags
//! for Kernel_Arch: `reinit_partition` (RESTART_PARTITION: stop EasyDMA,
//! re-initialize the Partition image, re-run its Init), `stop_dma`, and
//! `safe_state` (apply the declared system safe state, stop every
//! Release).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::budget::Budget;
use super::log::{Event, Log, EV_BOOT_CHECK, EV_DEADLINE_MISS, EV_END_JOB, EV_MIT_VIOLATION, EV_OPERATING_END,
    EV_OVERRUN, EV_PARTITION_RESTART, EV_PARTITION_STOP, EV_RELEASE_OVERLAP, EV_RESOURCE_RELEASED, EV_SAFE_STATE,
    EV_API_MISUSE};
// Spec and proof items of `history` are named by path: ghost erasure removes them from the stable build.
use super::history::{Step, ST_DISPATCH, ST_END, ST_KILL, ST_LOCK, ST_RELEASE, ST_UNLOCK};
use super::sched::{Sched, JOB_KILLED, JOB_NONE, JOB_RUNNING, REL_DEFERRED, REL_NONE, REL_PENDING};
use super::time::{Slots, Timing, INTAKE_ACCEPT, INTAKE_DEFER, SLOT_BUDGET, SLOT_DEADLINE, SLOT_DEFER, SLOT_END,
    SLOT_HOUSEKEEPING, SLOT_RELEASE};
use super::{Config, KIND_PERIODIC, KIND_SPORADIC, MIT_DEFER, NO_SLOT, RESP_END_JOB, RESP_RECORD_AND_CONTINUE,
    RESP_RECORD_ONLY, RESP_RESTART_PARTITION, RESP_SAFE_STATE, RESP_STOP_PARTITION};

verus! {

/// PR identifier bits for log entries (R3.9).
pub const PRS_RELEASE: u64 = 1u64 << 12; // PR-12
pub const PRS_BUDGET: u64 = 1u64 << 27; // PR-27 (declared Budget)
pub const PRS_PANIC: u64 = 1u64 << 19; // PR-19

/// CPU cycles per time-base tick (64 MHz / 16 MHz, PAR-08, DD-05).
pub const CYCLES_PER_TICK: u64 = 4;

/// Ticks between housekeeping events (watchdog feed, DD "Watchdog").
pub const HOUSEKEEPING_PERIOD: u64 = 16_000_000 / 8;

pub struct Kernel<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize> {
    pub sched: Sched<NT, NR, NL>,
    pub timing: Timing<NT>,
    pub slots: Slots<NS>,
    pub budget: Budget<NT>,
    pub log: Log<NE>,
    pub stopped: [bool; NP],
    pub mit_violations: [u32; NP],
    /// Set by RESTART_PARTITION until Kernel_Arch has re-initialized the Partition.
    pub reinit_partition: [bool; NP],
    /// Set by a stop or restart until Kernel_Arch has stopped the Partition's EasyDMA transfers (R17.6).
    pub stop_dma: [bool; NP],
    /// The system safe state has been entered: no Release becomes effective (R9.9, R14.4).
    pub safe_state: bool,
    /// Releases enabled: Init and the checks of Requirement 12 are complete (PR-34).
    pub started: bool,
    /// The last observed time-base instant.
    pub now: u64,
    /// The ghost history of scheduling transitions (task 4.11).
    pub history: Ghost<Seq<Step>>,
}

impl<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize>
    Kernel<NT, NR, NP, NL, NS, NE>
{
    pub open spec fn inv(&self, cfg: &Config<NT, NR, NP>) -> bool {
        &&& self.inv_unrecorded(cfg)
        // The link from the last recorded lock snapshot to the scheduler
        // (task 4.13): every lock change is recorded by the entry point
        // that makes it.
        &&& self.locks_match()
    }

    // ------------------------------------------------- Property 8 (MIT enforcement)

    /// The instant an event at `now` is processed at: the observed instant
    /// never moves back (`observe`).
    pub open spec fn observed(was: u64, now: u64) -> int {
        if now > was { now as int } else { was as int }
    }

    /// Property 8, MIT spacing, one step (R10.1, R10.2): the instant at
    /// which a Sporadic_Task's previous accepted Release became due is never
    /// forgotten and moves only to an instant at least one MIT later. Every
    /// accepted Release sets it to its own due instant (`accepted`,
    /// `deferred_release`), so two accepted Releases of a Sporadic_Task never
    /// become due less than one MIT apart.
    pub open spec fn mit_spaced(&self, old: &Self, cfg: &Config<NT, NR, NP>) -> bool {
        forall|u: int| 0 <= u < NT && cfg.tasks[u].kind == KIND_SPORADIC && #[trigger] old.timing.has_last[u]
            ==> self.timing.has_last[u]
                && (self.timing.last_due[u] == old.timing.last_due[u]
                    || self.timing.last_due[u] as int >= old.timing.last_due[u] as int + cfg.tasks[u].period as int)
    }

    /// MIT spacing composes over consecutive steps.
    pub proof fn lemma_mit_spaced_trans(a: &Self, b: &Self, c: &Self, cfg: &Config<NT, NR, NP>)
        requires b.mit_spaced(a, cfg), c.mit_spaced(b, cfg),
        ensures c.mit_spaced(a, cfg),
    {
        assert forall|u: int| 0 <= u < NT && cfg.tasks[u].kind == KIND_SPORADIC && #[trigger] a.timing.has_last[u]
            implies c.timing.has_last[u]
                && (c.timing.last_due[u] == a.timing.last_due[u]
                    || c.timing.last_due[u] as int >= a.timing.last_due[u] as int + cfg.tasks[u].period as int) by {
            assert(b.timing.has_last[u]);
        }
    }

    /// The event of Task `t` at instant `n` is processed: the system runs,
    /// the Task's Partition is not stopped, and a Job released at `n` could
    /// complete within the operating duration (R9.9, R12.4, R14.5).
    pub open spec fn intake_active(&self, cfg: &Config<NT, NR, NP>, t: int, n: int) -> bool {
        &&& !self.safe_state && self.started
        &&& !self.stopped[cfg.tasks[t].partition as int]
        &&& n + cfg.tasks[t].deadline <= cfg.operating_duration
    }

    /// R10.1: no Release of `t` is pending or deferred, and either none has
    /// become due since instant 0 or at least one MIT has elapsed since the
    /// previous accepted one became due.
    pub open spec fn mit_accepts(&self, cfg: &Config<NT, NR, NP>, t: int, n: int) -> bool {
        &&& self.sched.release[t] == REL_NONE
        &&& (!self.timing.has_last[t] || n >= self.timing.last_due[t] as int + cfg.tasks[t].period as int)
    }

    /// R10.2: no Release of `t` is pending or deferred, the event comes less
    /// than one MIT after the previous accepted Release, and the Task's MIT
    /// policy defers.
    pub open spec fn mit_defers(&self, cfg: &Config<NT, NR, NP>, t: int, n: int) -> bool {
        &&& self.sched.release[t] == REL_NONE
        &&& !self.mit_accepts(cfg, t, n)
        &&& cfg.tasks[t].mit_policy == MIT_DEFER
    }

    /// R10.2: the deferral event of `t` makes its deferred Release due: the
    /// Partition is not stopped, the Release is still deferred, and its Job
    /// could complete within the operating duration (R9.9).
    pub open spec fn deferral_fires(&self, cfg: &Config<NT, NR, NP>, t: int) -> bool {
        &&& !self.stopped[cfg.tasks[t].partition as int]
        &&& self.sched.release[t] == REL_DEFERRED
        &&& self.timing.deferred_until[t] as int + cfg.tasks[t].deadline as int <= cfg.operating_duration as int
    }

    /// R9.8: the nominal instant of Periodic_Task `t` arrives while its
    /// previous Release is still pending (the Partition is not stopped, and
    /// the new Release's Job could complete within the operating duration).
    pub open spec fn periodic_discards(&self, cfg: &Config<NT, NR, NP>, t: int) -> bool {
        &&& cfg.tasks[t].kind == KIND_PERIODIC
        &&& !self.stopped[cfg.tasks[t].partition as int]
        &&& self.timing.nominal[t] as int + cfg.tasks[t].deadline as int <= cfg.operating_duration as int
        &&& self.sched.release[t] != REL_NONE
    }

    /// `self` is `before` except for the event log and the observed instant.
    pub open spec fn unchanged_but_log(&self, before: &Self) -> bool {
        &&& self.sched == before.sched && self.timing == before.timing
        &&& self.slots == before.slots && self.budget == before.budget
        &&& self.stopped == before.stopped && self.mit_violations == before.mit_violations
        &&& self.reinit_partition == before.reinit_partition && self.stop_dma == before.stop_dma
        &&& self.safe_state == before.safe_state && self.started == before.started
    }

    /// R10.1, R10.7: `self` is `old` with the event of Task `t` accepted at
    /// `n`: its Release is pending and due at `n`, which becomes the
    /// previous accepted due instant; its source is masked for the MIT
    /// window (DD-06) and its deadline event armed. No MIT_Violation is
    /// recorded and nothing else changes.
    pub open spec fn accepted(&self, old: &Self, cfg: &Config<NT, NR, NP>, t: int, n: int) -> bool {
        &&& self.sched.release[t] == REL_PENDING
        &&& forall|u: int| 0 <= u < NT && u != t ==> self.sched.release[u] == old.sched.release[u]
        &&& self.jobs_eq(old)
        &&& self.timing.has_last[t] && self.timing.last_due[t] as int == n && self.timing.due[t] as int == n
        &&& self.timing.masked[t]
        &&& forall|u: int| 0 <= u < NT && u != t ==> self.timing.has_last[u] == old.timing.has_last[u]
            && self.timing.last_due[u] == old.timing.last_due[u] && self.timing.due[u] == old.timing.due[u]
            && self.timing.masked[u] == old.timing.masked[u]
        &&& self.timing.k == old.timing.k && self.timing.nominal == old.timing.nominal
        &&& self.timing.deferred_until == old.timing.deferred_until && self.timing.job_due == old.timing.job_due
        &&& ({
                let s = self.slots.slots[cfg.slot_deadline[t] as int];
                s.armed && s.at as int == n + cfg.tasks[t].deadline && s.kind == SLOT_DEADLINE && s.task as int == t
            })
        &&& forall|j: int| 0 <= j < NS && j != cfg.slot_deadline[t] ==> self.slots.slots[j] == old.slots.slots[j]
        &&& self.budget == old.budget
        &&& self.stopped == old.stopped && self.mit_violations == old.mit_violations
        &&& self.reinit_partition == old.reinit_partition && self.stop_dma == old.stop_dma
        &&& self.safe_state == old.safe_state && self.started == old.started
    }

    /// R10.2: `self` is `old` with the event of Task `t` deferred: its
    /// Release is deferred until one MIT after the previous accepted Release
    /// became due, its source is masked, and its deferral event is armed for
    /// that instant (or for the end of the operating duration, beyond which
    /// no Release becomes effective, R9.9). The previous accepted due
    /// instant is unchanged.
    pub open spec fn deferred(&self, old: &Self, cfg: &Config<NT, NR, NP>, t: int) -> bool {
        &&& self.sched.release[t] == REL_DEFERRED
        &&& forall|u: int| 0 <= u < NT && u != t ==> self.sched.release[u] == old.sched.release[u]
        &&& self.jobs_eq(old)
        &&& self.timing.deferred_until[t] as int == old.timing.last_due[t] as int + cfg.tasks[t].period as int
        &&& self.timing.masked[t]
        &&& forall|u: int| 0 <= u < NT && u != t ==> self.timing.deferred_until[u] == old.timing.deferred_until[u]
            && self.timing.masked[u] == old.timing.masked[u]
        &&& self.timing.has_last == old.timing.has_last && self.timing.last_due == old.timing.last_due
        &&& self.timing.k == old.timing.k && self.timing.nominal == old.timing.nominal
        &&& self.timing.due == old.timing.due && self.timing.job_due == old.timing.job_due
        &&& ({
                let s = self.slots.slots[cfg.slot_release[t] as int];
                &&& s.armed && s.kind == SLOT_DEFER && s.task as int == t
                &&& s.at == (if self.timing.deferred_until[t] > cfg.operating_duration { cfg.operating_duration }
                    else { self.timing.deferred_until[t] })
            })
        &&& forall|j: int| 0 <= j < NS && j != cfg.slot_release[t] ==> self.slots.slots[j] == old.slots.slots[j]
        &&& self.budget == old.budget
        &&& self.stopped == old.stopped && self.mit_violations == old.mit_violations
        &&& self.reinit_partition == old.reinit_partition && self.stop_dma == old.stop_dma
        &&& self.safe_state == old.safe_state && self.started == old.started
    }

    /// A violation count incremented by one, saturating.
    pub open spec fn incremented(c: u32) -> u32 {
        super::budget::charged(c, 1)
    }

    /// `self` is `before` with the count of Partition `sp` set to `c`, and
    /// nothing else changed but the event log and the observed instant.
    pub open spec fn counted(&self, before: &Self, sp: int, c: u32) -> bool {
        &&& self.sched == before.sched && self.timing == before.timing
        &&& self.slots == before.slots && self.budget == before.budget
        &&& self.stopped == before.stopped
        &&& self.reinit_partition == before.reinit_partition && self.stop_dma == before.stop_dma
        &&& self.safe_state == before.safe_state && self.started == before.started
        &&& self.mit_violations[sp] == c
        &&& forall|q: int| 0 <= q < NP && q != sp ==> self.mit_violations[q] == before.mit_violations[q]
    }

    /// R10.3, R10.4, R10.7, R9.8: the outcome of one MIT_Violation of Task
    /// `t`, attributed to Partition `sp`, from state `before`. Up to the
    /// Partition's threshold the violation is counted and nothing else
    /// changes, so a discarded event leaves the existing Release unchanged.
    /// Past the threshold, the Partition's fault response is applied to the
    /// state with the violation counted (`response_applied`).
    pub open spec fn violation_applied(&self, before: &Self, cfg: &Config<NT, NR, NP>, sp: int, t: int) -> bool {
        if Self::incremented(before.mit_violations[sp]) <= cfg.partitions[sp].mit_threshold {
            self.counted(before, sp, Self::incremented(before.mit_violations[sp]))
        } else {
            exists|m: Self| #[trigger] m.counted(before, sp, Self::incremented(before.mit_violations[sp]))
                && self.response_applied(&m, cfg, sp, cfg.partitions[sp].fault_response, t)
        }
    }

    /// Every response keeps the timing state, MIT history included (R14.5).
    pub proof fn lemma_response_keeps_timing(&self, before: &Self, cfg: &Config<NT, NR, NP>, p: int, response: u8, t: int)
        requires self.response_applied(before, cfg, p, response, t),
        ensures self.timing == before.timing,
    {
    }

    /// So does every MIT_Violation, whether or not it trips the threshold.
    pub proof fn lemma_violation_keeps_timing(&self, before: &Self, cfg: &Config<NT, NR, NP>, sp: int, t: int)
        requires self.violation_applied(before, cfg, sp, t),
        ensures self.timing == before.timing,
    {
        if Self::incremented(before.mit_violations[sp]) > cfg.partitions[sp].mit_threshold {
            let m = choose|m: Self| #[trigger] m.counted(before, sp, Self::incremented(before.mit_violations[sp]))
                && self.response_applied(&m, cfg, sp, cfg.partitions[sp].fault_response, t);
            self.lemma_response_keeps_timing(&m, cfg, sp, cfg.partitions[sp].fault_response, t);
        }
    }

    /// A violation outcome stated against `a` holds against any `b` that
    /// differs from `a` only in the event log and the observed instant.
    pub proof fn lemma_violation_rebase(&self, a: &Self, b: &Self, cfg: &Config<NT, NR, NP>, sp: int, t: int)
        requires a.unchanged_but_log(b), self.violation_applied(a, cfg, sp, t),
        ensures self.violation_applied(b, cfg, sp, t),
    {
        assert(a.mit_violations[sp] == b.mit_violations[sp]);
        if Self::incremented(a.mit_violations[sp]) > cfg.partitions[sp].mit_threshold {
            let m = choose|m: Self| #[trigger] m.counted(a, sp, Self::incremented(a.mit_violations[sp]))
                && self.response_applied(&m, cfg, sp, cfg.partitions[sp].fault_response, t);
            assert(m.counted(b, sp, Self::incremented(b.mit_violations[sp])));
        }
    }

    // ------------------------------------------- specifications of the entry points

    /// The Kernel flags are those of `other`.
    pub open spec fn flags_eq(&self, other: &Self) -> bool {
        &&& self.stopped == other.stopped
        &&& self.mit_violations == other.mit_violations
        &&& self.reinit_partition == other.reinit_partition
        &&& self.stop_dma == other.stop_dma
        &&& self.safe_state == other.safe_state
        &&& self.started == other.started
        &&& self.now == other.now
    }

    /// No Release is pending or deferred.
    pub open spec fn no_release(&self) -> bool {
        forall|u: int| 0 <= u < NT ==> self.sched.release[u] == REL_NONE
    }

    /// The Jobs in progress and the lock stack are those of `other` (only
    /// the Release states may differ).
    pub open spec fn jobs_eq(&self, other: &Self) -> bool {
        &&& self.sched.job == other.sched.job
        &&& self.sched.stack == other.sched.stack
        &&& self.sched.depth == other.sched.depth
        &&& self.sched.start_ceiling == other.sched.start_ceiling
        &&& self.sched.ceiling == other.sched.ceiling
        &&& self.sched.lock_len == other.sched.lock_len
        &&& self.sched.lock_res == other.sched.lock_res
        &&& self.sched.lock_depth == other.sched.lock_depth
    }

    /// `Sched::end_job` lifted (Property 2 completion, Property 6, R8.9):
    /// the Job that executed in `old` has ended, the preempted Jobs keep
    /// their stack positions, the ended Job's lock entries are released and
    /// no others, the System_Ceiling is the maximum over the remaining
    /// entries, and every Release state is unchanged (R9.7).
    pub open spec fn ended_top(&self, old: &Self, cfg: &Config<NT, NR, NP>) -> bool {
        let t = old.sched.top();
        &&& self.sched.depth == old.sched.depth - 1
        &&& self.sched.job[t] == JOB_NONE
        &&& forall|u: int| 0 <= u < NT && u != t ==> self.sched.job[u] == old.sched.job[u]
        &&& self.sched.release == old.sched.release
        &&& forall|i: int| 0 <= i < self.sched.depth ==> self.sched.stack[i] == old.sched.stack[i]
        &&& self.sched.lock_len <= old.sched.lock_len
        &&& self.sched.lock_res == old.sched.lock_res
        &&& self.sched.lock_depth == old.sched.lock_depth
        &&& forall|e: int| 0 <= e < self.sched.lock_len ==> !old.sched.entry_of_top(e)
        &&& forall|e: int| self.sched.lock_len <= e < old.sched.lock_len ==> old.sched.entry_of_top(e)
        &&& self.sched.ceiling == super::sched::max_ceiling(cfg, old.sched.lock_res, self.sched.lock_len as int)
    }

    /// R9.1, R9.3: Periodic_Task `u`'s release slot is armed for its
    /// current nominal instant.
    pub open spec fn release_armed(&self, cfg: &Config<NT, NR, NP>, u: int) -> bool {
        let s = self.slots.slots[cfg.slot_release[u] as int];
        s.armed && s.at == self.timing.nominal[u] && s.kind == SLOT_RELEASE && s.task as int == u
    }

    /// R9.9: the end-of-operation slot is armed for the operating duration.
    pub open spec fn end_armed(&self, cfg: &Config<NT, NR, NP>) -> bool {
        let s = self.slots.slots[cfg.slot_of_end()];
        s.armed && s.at == cfg.operating_duration && s.kind == SLOT_END
    }

    /// R14.5: every Job of Partition `p` has ended or is marked to end when
    /// Kernel_Arch unwinds to it (`JOB_KILLED`), and no Release of `p` is
    /// pending or deferred.
    pub open spec fn partition_ended(&self, cfg: &Config<NT, NR, NP>, p: int) -> bool {
        forall|u: int| 0 <= u < NT && (cfg.tasks[u].partition as int) == p
            ==> self.sched.release[u] == REL_NONE && self.sched.job[u] != JOB_RUNNING
    }

    /// The Partition of the holder of lock entry `e`.
    pub open spec fn holder_partition(&self, cfg: &Config<NT, NR, NP>, e: int) -> int {
        cfg.tasks[self.sched.stack[self.sched.lock_depth[e] as int] as int].partition as int
    }

    /// Task `u` is as it was in `old`: its Release and Job states, its Budget
    /// account, its timing state (nominal instant, MIT history, deadline
    /// reference, source mask), and its release and deadline events. Its
    /// Budget event is unchanged too, unless `u`'s Job is the one that
    /// resumed because the executing Job ended (re-armed for its remaining
    /// Budget, as on every resumption).
    pub open spec fn task_unchanged(&self, old: &Self, cfg: &Config<NT, NR, NP>, u: int) -> bool {
        &&& self.sched.release[u] == old.sched.release[u]
        &&& self.sched.job[u] == old.sched.job[u]
        &&& self.budget.used[u] == old.budget.used[u]
        &&& self.timing.k[u] == old.timing.k[u] && self.timing.nominal[u] == old.timing.nominal[u]
        &&& self.timing.has_last[u] == old.timing.has_last[u] && self.timing.last_due[u] == old.timing.last_due[u]
        &&& self.timing.due[u] == old.timing.due[u] && self.timing.job_due[u] == old.timing.job_due[u]
        &&& self.timing.deferred_until[u] == old.timing.deferred_until[u] && self.timing.masked[u] == old.timing.masked[u]
        &&& self.slots.slots[cfg.slot_deadline[u] as int] == old.slots.slots[cfg.slot_deadline[u] as int]
        &&& (cfg.slot_release[u] != NO_SLOT
            ==> self.slots.slots[cfg.slot_release[u] as int] == old.slots.slots[cfg.slot_release[u] as int])
        &&& (self.sched.depth == old.sched.depth || self.sched.depth == 0 || u != self.sched.top()
            ==> self.slots.slots[cfg.slot_budget[u] as int] == old.slots.slots[cfg.slot_budget[u] as int])
    }

    /// R14.9: an action on Partition `p` leaves every other Partition as it
    /// was in `old`. "Unchanged" is per Partition, because the
    /// System_Ceiling is global: when `p` releases a Resource the ceiling
    /// falls, which can only let other Partitions' Jobs start sooner, so the
    /// ceiling itself is required only not to rise. What stays the same is
    /// what another Partition owns: every one of its Tasks
    /// (`task_unchanged`), the stack positions of its Jobs in progress, every
    /// lock entry its Jobs hold (Resource and holder), so every Ceiling it
    /// imposes is still in force (`lemma_others_ceilings_in_force`), and its
    /// Partition flags. Endpoint roles do not exist yet (Phase 4).
    pub open spec fn others_unchanged(&self, old: &Self, cfg: &Config<NT, NR, NP>, p: int) -> bool {
        &&& forall|u: int| 0 <= u < NT && (#[trigger] cfg.tasks[u].partition as int) != p ==> self.task_unchanged(old, cfg, u)
        &&& forall|i: int| 0 <= i < old.sched.depth && (cfg.tasks[old.sched.stack[i] as int].partition as int) != p
            ==> i < self.sched.depth && self.sched.stack[i] == old.sched.stack[i]
        &&& forall|e: int| 0 <= e < old.sched.lock_len && #[trigger] old.holder_partition(cfg, e) != p
            ==> e < self.sched.lock_len && self.sched.lock_res[e] == old.sched.lock_res[e]
                && self.sched.lock_depth[e] == old.sched.lock_depth[e]
        &&& forall|q: int| 0 <= q < NP && q != p ==> self.stopped[q] == old.stopped[q]
            && self.mit_violations[q] == old.mit_violations[q]
            && self.reinit_partition[q] == old.reinit_partition[q] && self.stop_dma[q] == old.stop_dma[q]
        &&& self.sched.ceiling <= old.sched.ceiling
    }

    /// R14.9, the global ceiling: after an action on Partition `p`, the
    /// System_Ceiling still covers the Ceiling of every Resource that
    /// another Partition held before it, so no Job that those Resources
    /// blocked starts.
    pub proof fn lemma_others_ceilings_in_force(&self, old: &Self, cfg: &Config<NT, NR, NP>, p: int)
        requires self.inv(cfg), self.others_unchanged(old, cfg, p),
        ensures forall|e: int| 0 <= e < old.sched.lock_len && #[trigger] old.holder_partition(cfg, e) != p
            ==> cfg.ceiling(old.sched.lock_res[e] as int) <= self.sched.ceiling,
    {
        assert forall|e: int| 0 <= e < old.sched.lock_len && #[trigger] old.holder_partition(cfg, e) != p
            implies cfg.ceiling(old.sched.lock_res[e] as int) <= self.sched.ceiling by {
            Sched::<NT, NR, NL>::lemma_entry_le_ceiling(cfg, self.sched.lock_res, self.sched.lock_len as int, e);
        }
    }

    /// An event record changes only the log, so a specification stated
    /// against the state after it (`mid`) holds against the state before it
    /// (`old`).
    pub proof fn lemma_same_but_log(&self, mid: &Self, old: &Self, cfg: &Config<NT, NR, NP>, p: int, response: u8, t: int)
        requires mid.sched == old.sched, mid.slots == old.slots, mid.budget == old.budget,
            mid.timing == old.timing, mid.flags_eq(old),
        ensures
            self.others_unchanged(mid, cfg, p) ==> self.others_unchanged(old, cfg, p),
            self.response_applied(mid, cfg, p, response, t) ==> self.response_applied(old, cfg, p, response, t),
    {
        self.lemma_others_same_but_log(mid, old, cfg, p);
        self.lemma_others_same_but_log(mid, old, cfg, cfg.tasks[t].partition as int);
    }

    proof fn lemma_others_same_but_log(&self, mid: &Self, old: &Self, cfg: &Config<NT, NR, NP>, p: int)
        requires mid.sched == old.sched, mid.slots == old.slots, mid.budget == old.budget,
            mid.timing == old.timing, mid.flags_eq(old),
        ensures self.others_unchanged(mid, cfg, p) ==> self.others_unchanged(old, cfg, p),
    {
        if self.others_unchanged(mid, cfg, p) {
            assert forall|u: int| 0 <= u < NT && (#[trigger] cfg.tasks[u].partition as int) != p
                implies self.task_unchanged(old, cfg, u) by {
                assert(self.task_unchanged(mid, cfg, u));
            }
            assert forall|e: int| 0 <= e < old.sched.lock_len && #[trigger] old.holder_partition(cfg, e) != p
                implies e < self.sched.lock_len && self.sched.lock_res[e] == old.sched.lock_res[e]
                    && self.sched.lock_depth[e] == old.sched.lock_depth[e] by {
                assert(mid.holder_partition(cfg, e) == old.holder_partition(cfg, e));
            }
        }
    }

    /// R9.4: distinct Tasks' timed-event slots are distinct, and every slot
    /// lies below the two reserved slots.
    proof fn lemma_slots_apart(cfg: &Config<NT, NR, NP>, t: int, u: int)
        requires cfg.wf(), 0 <= t < NT, 0 <= u < NT, t != u,
        ensures
            cfg.slot_deadline[u] != cfg.slot_deadline[t], cfg.slot_deadline[u] != cfg.slot_budget[t],
            cfg.slot_budget[u] != cfg.slot_deadline[t], cfg.slot_budget[u] != cfg.slot_budget[t],
            cfg.slot_release[u] != NO_SLOT ==> cfg.slot_release[u] != cfg.slot_deadline[t]
                && cfg.slot_release[u] != cfg.slot_budget[t] && cfg.slot_release[u] != cfg.slot_release[t],
            cfg.slot_release[t] != NO_SLOT ==> cfg.slot_release[t] != cfg.slot_deadline[u]
                && cfg.slot_release[t] != cfg.slot_budget[u],
            (cfg.slot_deadline[u] as int) < cfg.slot_count - 2, (cfg.slot_budget[u] as int) < cfg.slot_count - 2,
            cfg.slot_release[u] != NO_SLOT ==> (cfg.slot_release[u] as int) < cfg.slot_count - 2,
    {
        assert(cfg.slots_distinct(t, u));
        assert(cfg.slots_distinct(u, t));
        Self::lemma_slots_own(cfg, u);
    }

    /// R9.4: one Task's slots are distinct from each other.
    proof fn lemma_slots_own(cfg: &Config<NT, NR, NP>, u: int)
        requires cfg.wf(), 0 <= u < NT,
        ensures
            cfg.slot_deadline[u] != cfg.slot_budget[u],
            cfg.slot_release[u] != NO_SLOT ==> cfg.slot_release[u] != cfg.slot_budget[u]
                && cfg.slot_release[u] != cfg.slot_deadline[u] && (cfg.slot_release[u] as int) < cfg.slot_count - 2,
            (cfg.slot_deadline[u] as int) < cfg.slot_count - 2, (cfg.slot_budget[u] as int) < cfg.slot_count - 2,
    {
        assert(cfg.slot_ok(u));
        assert(cfg.task_ok(u));
    }

    /// DD-10, R14.5, R14.9: the effect of `response` on Partition `p` for an
    /// event of Task `t`, from state `old`, as `apply_response` applies it.
    /// Every response but SAFE_STATE (which stops the whole system, R14.4)
    /// leaves the other Partitions unchanged (`others_unchanged`).
    pub open spec fn response_applied(&self, old: &Self, cfg: &Config<NT, NR, NP>, p: int, response: u8, t: int) -> bool {
        if response == RESP_RECORD_ONLY || response == RESP_RECORD_AND_CONTINUE {
            // Logged by the caller; the Job continues.
            &&& self.sched == old.sched && self.flags_eq(old)
            &&& self.budget == old.budget && self.slots == old.slots && self.timing == old.timing
        } else if response == RESP_END_JOB && old.sched.depth > 0 && old.sched.top() == t && old.sched.top_holds_nothing() {
            &&& self.ended_top(old, cfg)
            &&& self.others_unchanged(old, cfg, cfg.tasks[t].partition as int)
            &&& self.timing == old.timing
        } else if response == RESP_END_JOB || response == RESP_RESTART_PARTITION {
            // END_JOB on a Job holding a Resource escalates (R18.4).
            &&& self.partition_ended(cfg, p)
            &&& self.reinit_partition[p] && self.stop_dma[p]
            &&& self.mit_violations[p] == 0
            &&& self.others_unchanged(old, cfg, p)
            // Releases resume at the next nominal instants; MIT history is kept.
            &&& self.timing == old.timing
        } else if response == RESP_STOP_PARTITION {
            &&& self.partition_ended(cfg, p) && self.stopped[p] && self.stop_dma[p]
            &&& self.others_unchanged(old, cfg, p)
            &&& self.timing == old.timing
        } else {
            self.safe_state && self.no_release() && self.timing == old.timing
        }
    }

    /// The history grew from `old`'s by this entry point's own steps only.
    pub open spec fn history_extends(&self, old: &Self) -> bool {
        super::history::extends(self.history@, old.history@)
    }

    /// The lock stack as a snapshot: the Resources held, bottom to top.
    pub open spec fn locks_now(&self) -> Seq<int> {
        Seq::new(self.sched.lock_len as nat, |e: int| self.sched.lock_res[e] as int)
    }

    /// The last step's lock snapshot (nothing before the first step).
    pub open spec fn locks_recorded(&self) -> Seq<int> {
        super::history::locks_before(self.history@, self.history@.len() as int)
    }

    /// The scheduler's lock stack is the last recorded snapshot: every
    /// lock change is recorded by the entry point that makes it.
    pub open spec fn locks_match(&self) -> bool {
        &&& self.locks_recorded().len() == self.sched.lock_len
        &&& forall|e: int| 0 <= e < self.sched.lock_len ==> #[trigger] self.locks_recorded()[e] == self.sched.lock_res[e]
    }

    /// `inv` without `locks_match`: the state between a lock change and
    /// the `record` that reports it.
    pub open spec fn inv_unrecorded(&self, cfg: &Config<NT, NR, NP>) -> bool {
        &&& self.sched.inv(cfg)
        &&& self.timing.inv(cfg)
        &&& self.log.wf()
        &&& (cfg.slot_count as int) == NS
        &&& self.now <= cfg.operating_duration
        &&& super::history::history_wf(self.history@, cfg, self.now as int)
        &&& forall|u: int| 0 <= u < NT && #[trigger] self.sched.release[u] == REL_DEFERRED
            ==> cfg.tasks[u].kind == KIND_SPORADIC && cfg.tasks[u].mit_policy == MIT_DEFER
                && self.timing.has_last[u]
                && self.timing.deferred_until[u] as int == self.timing.last_due[u] as int + cfg.tasks[u].period as int
    }

    /// The step `record` would append now.
    pub open spec fn step_now(&self, kind: u8, task: int, arg: int) -> Step {
        Step { kind, task, at: self.now as int, arg, locks: self.locks_now(), ceiling: self.sched.ceiling as int }
    }

    // The lock-snapshot relation `record` requires, from the scheduler
    // postconditions of the change since `before` (a state with
    // `locks_match`, the last one recorded). Kept out of the entry points
    // so that their own proofs stay within the resource limit.

    /// A Job end released a suffix (`Sched::end_job`, `finish_top`).
    pub proof fn lemma_rel_shrink(&self, before: &Self, kind: u8, task: int, arg: int)
        requires before.locks_match(), self.history@ == before.history@, kind == ST_END || kind == ST_KILL,
            self.sched.lock_len <= before.sched.lock_len, self.sched.lock_res == before.sched.lock_res,
        ensures super::history::step_rel(self.locks_recorded(), self.step_now(kind, task, arg)),
    {
        let s = self.step_now(kind, task, arg);
        assert(self.locks_recorded() == before.locks_recorded());
        assert forall|e: int| 0 <= e < s.locks.len() implies #[trigger] s.locks[e] == self.locks_recorded()[e] by {
            assert(s.locks[e] == self.sched.lock_res[e] as int);
            assert(before.locks_recorded()[e] == before.sched.lock_res[e] as int);
        }
    }

    /// A lock pushed `r` (`Sched::lock` with `ok`).
    pub proof fn lemma_rel_lock(&self, before: &Self, task: int, r: int)
        requires before.locks_match(), self.history@ == before.history@,
            self.sched.lock_len == before.sched.lock_len + 1,
            forall|e: int| 0 <= e < before.sched.lock_len ==> self.sched.lock_res[e] == before.sched.lock_res[e],
            self.sched.lock_res[before.sched.lock_len as int] as int == r,
        ensures super::history::step_rel(self.locks_recorded(), self.step_now(ST_LOCK, task, r)),
    {
        let s = self.step_now(ST_LOCK, task, r);
        assert(self.locks_recorded() == before.locks_recorded());
        assert forall|e: int| 0 <= e < before.sched.lock_len implies #[trigger] s.locks[e] == self.locks_recorded()[e] by {
            assert(s.locks[e] == self.sched.lock_res[e] as int);
            assert(before.locks_recorded()[e] == before.sched.lock_res[e] as int);
        }
        assert(s.locks.last() == self.sched.lock_res[before.sched.lock_len as int] as int);
    }

    /// An unlock popped `r` (`Sched::unlock` with `ok`).
    pub proof fn lemma_rel_unlock(&self, before: &Self, task: int, r: int)
        requires before.locks_match(), self.history@ == before.history@, before.sched.lock_len >= 1,
            self.sched.lock_len == before.sched.lock_len - 1, self.sched.lock_res == before.sched.lock_res,
            before.sched.lock_res[before.sched.lock_len - 1] as int == r,
        ensures super::history::step_rel(self.locks_recorded(), self.step_now(ST_UNLOCK, task, r)),
    {
        let s = self.step_now(ST_UNLOCK, task, r);
        assert(self.locks_recorded() == before.locks_recorded());
        assert forall|e: int| 0 <= e < s.locks.len() implies #[trigger] s.locks[e] == self.locks_recorded()[e] by {
            assert(s.locks[e] == self.sched.lock_res[e] as int);
            assert(before.locks_recorded()[e] == before.sched.lock_res[e] as int);
        }
        assert(self.locks_recorded().last() == before.sched.lock_res[before.sched.lock_len - 1] as int);
    }

    /// Appends one step, performed now, to the ghost history (task 4.11).
    /// Erased from the Flight_Build; the `Ghost` assignment has no
    /// executable effect.
    fn record(&mut self, cfg: &Config<NT, NR, NP>, kind: u8, task: Ghost<int>, arg: Ghost<int>)
        requires old(self).inv_unrecorded(cfg), ST_DISPATCH <= kind <= ST_KILL, 0 <= task@ < NT, 0 <= arg@,
            (kind == ST_LOCK || kind == ST_UNLOCK) ==> arg@ < NR,
            // The lock change since the last step is the one this kind reports.
            super::history::step_rel(old(self).locks_recorded(), old(self).step_now(kind, task@, arg@)),
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).history@ == old(self).history@.push(old(self).step_now(kind, task@, arg@)),
            final(self).sched == old(self).sched, final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).budget == old(self).budget, final(self).log == old(self).log, final(self).stopped == old(self).stopped,
            final(self).mit_violations == old(self).mit_violations, final(self).safe_state == old(self).safe_state,
            final(self).started == old(self).started, final(self).now == old(self).now,
            final(self).reinit_partition == old(self).reinit_partition, final(self).stop_dma == old(self).stop_dma,
    {
        let ghost s = self.step_now(kind, task@, arg@);
        proof {
            // The snapshot's Resources are the scheduler's (invariant 3) and
            // its ceiling the scheduler's mirror (invariant 2).
            super::history::lemma_ceiling_of_array(cfg, self.sched.lock_res, self.sched.lock_len as int);
            assert(super::history::step_wf(s, cfg));
            let p = self.history@.push(s);
            assert(super::history::locks_before(p, self.history@.len() as int) == self.locks_recorded());
            super::history::lemma_history_push(self.history@, s, cfg, self.now as int);
        }
        self.history = Ghost(self.history@.push(s));
    }

    /// Init: everything idle, no Release before `start` (PR-34, R12.4).
    pub fn new(cfg: &Config<NT, NR, NP>) -> (k: Self)
        requires cfg.wf(), NL <= 255, 0 < NE < 0x4000_0000, (cfg.slot_count as int) == NS,
        ensures k.inv(cfg), !k.started, !k.safe_state, k.sched.depth == 0, k.history@ == Seq::<Step>::empty(),
    {
        proof {
            super::history::lemma_history_empty(cfg, 0);
        }
        Kernel {
            sched: Sched::new(cfg),
            timing: Timing::new(cfg),
            slots: Slots::new(),
            budget: Budget::new(),
            log: Log::new(),
            stopped: [false; NP],
            mit_violations: [0; NP],
            reinit_partition: [false; NP],
            stop_dma: [false; NP],
            safe_state: false,
            started: false,
            now: 0,
            history: Ghost(Seq::<Step>::empty()),
        }
    }

    // ------------------------------------------------------------ helpers

    fn push_event(&mut self, cfg: &Config<NT, NR, NP>, kind: u8, task: u8, partition: u8, prs: u64, detail: u32)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).sched == old(self).sched, final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).budget == old(self).budget, final(self).stopped == old(self).stopped,
            final(self).mit_violations == old(self).mit_violations, final(self).safe_state == old(self).safe_state,
            final(self).started == old(self).started, final(self).now == old(self).now,
            final(self).reinit_partition == old(self).reinit_partition, final(self).stop_dma == old(self).stop_dma,
    {
        let instant = self.now;
        self.log.push(Event { kind, task, partition, prs, instant, detail });
    }

    /// Advances the observed instant (monotone; Kernel_Arch reads the time base).
    pub fn observe(&mut self, cfg: &Config<NT, NR, NP>, now: u64)
        requires old(self).inv(cfg), now <= cfg.operating_duration,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).now == (if now > old(self).now { now } else { old(self).now }),
            final(self).sched == old(self).sched, final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).started == old(self).started, final(self).safe_state == old(self).safe_state,
            final(self).budget == old(self).budget, final(self).stopped == old(self).stopped,
            final(self).mit_violations == old(self).mit_violations,
            final(self).reinit_partition == old(self).reinit_partition, final(self).stop_dma == old(self).stop_dma,
    {
        if now > self.now {
            proof {
                super::history::lemma_history_later(self.history@, cfg, self.now as int, now as int);
            }
            self.now = now;
        }
    }

    /// PR-34: Init and the checks of Requirement 12 are complete; instant 0
    /// is now. Arms the first release of every Periodic_Task, the
    /// end-of-operation slot, and the housekeeping slot.
    pub fn start(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), !old(self).started,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).started,
            // No Release is made here: Releases begin when their events fire (PR-34).
            final(self).sched == old(self).sched, final(self).timing == old(self).timing,
            final(self).budget == old(self).budget,
            final(self).safe_state == old(self).safe_state, final(self).stopped == old(self).stopped,
            final(self).now == old(self).now,
            // R9.1, R9.3: the first Release of every Periodic_Task is armed at its nominal instant.
            forall|u: int| 0 <= u < NT && cfg.tasks[u].kind == KIND_PERIODIC ==> final(self).release_armed(cfg, u),
            // R9.9: the end of the operating duration is armed.
            final(self).end_armed(cfg),
            final(self).mit_spaced(old(self), cfg),
    {
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), self.history_extends(old(self)), t <= NT, (cfg.slot_count as int) == NS, !self.started,
                self.sched == old(self).sched, self.timing == old(self).timing, self.budget == old(self).budget,
                self.flags_eq(old(self)),
                forall|u: int| #![trigger cfg.slot_release[u]]
                    0 <= u < t && cfg.tasks[u].kind == KIND_PERIODIC ==> self.release_armed(cfg, u),
            decreases NT - t,
        {
            proof {
                // Release slots of distinct Periodic_Tasks are distinct.
                assert forall|u: int| 0 <= u < t && cfg.tasks[u].kind == KIND_PERIODIC
                    implies (cfg.slot_release[u] as int) < NS && cfg.slot_release[u] != cfg.slot_release[t as int] by {
                    assert(cfg.slot_ok(u));
                    assert(cfg.slots_distinct(u, t as int));
                }
            }
            if cfg.tasks[t].kind == KIND_PERIODIC {
                proof {
                    assert(cfg.slot_ok(t as int));
                }
                let slot = cfg.slot_release[t] as usize;
                let at = self.timing.nominal[t];
                self.slots.arm(slot, at, SLOT_RELEASE, t as u8);
            }
            t = t + 1;
        }
        proof {
            // Release slots lie below the two reserved slots.
            assert forall|u: int| 0 <= u < NT && cfg.tasks[u].kind == KIND_PERIODIC
                implies (cfg.slot_release[u] as int) < cfg.slot_count - 2 by {
                assert(cfg.slot_ok(u));
            }
        }
        let end = (cfg.slot_count - 2) as usize;
        self.slots.arm(end, cfg.operating_duration, SLOT_END, 0xFF);
        let hk = (cfg.slot_count - 1) as usize;
        let hk_at = if HOUSEKEEPING_PERIOD < cfg.operating_duration { HOUSEKEEPING_PERIOD } else { cfg.operating_duration };
        self.slots.arm(hk, hk_at, SLOT_HOUSEKEEPING, 0xFF);
        self.started = true;
    }

    // ------------------------------------------------- dispatch stub entry and exit

    /// Dispatch stub entry: the NVIC took Task `t`'s vector, so `t` is
    /// eligible (Property 1 over the Hardware_Model); the preempted Job (if
    /// any) is charged the cycles it consumed since it last resumed. Arms
    /// the Budget event of `t`.
    pub fn dispatch(&mut self, cfg: &Config<NT, NR, NP>, t: u8, preempted_cycles: u32)
        requires old(self).inv(cfg), (t as int) < NT, old(self).sched.eligible(cfg, t as int), old(self).started,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // `Sched::start` lifted (Property 1 start, R7.1, R7.2).
            final(self).sched.depth == old(self).sched.depth + 1,
            final(self).sched.top() == t as int,
            final(self).sched.job[t as int] == JOB_RUNNING,
            final(self).sched.release[t as int] == REL_NONE,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).sched.job[u] == old(self).sched.job[u]
                && final(self).sched.release[u] == old(self).sched.release[u],
            forall|i: int| 0 <= i < old(self).sched.depth ==> final(self).sched.stack[i] == old(self).sched.stack[i],
            final(self).sched.ceiling == old(self).sched.ceiling,
            final(self).sched.lock_len == old(self).sched.lock_len,
            final(self).sched.lock_res == old(self).sched.lock_res,
            final(self).sched.lock_depth == old(self).sched.lock_depth,
            // Property 10 attribution (R18.1): the preempted Job is charged its
            // segment, the new Job starts from zero, no other account changes.
            final(self).budget.used[t as int] == 0,
            old(self).sched.depth > 0 ==> final(self).budget.used[old(self).sched.top()]
                == super::budget::charged(old(self).budget.used[old(self).sched.top()], preempted_cycles),
            forall|u: int| 0 <= u < NT && u != t && (old(self).sched.depth == 0 || u != old(self).sched.top())
                ==> final(self).budget.used[u] == old(self).budget.used[u],
            // R18.2: the new Job's Budget event is armed; R9.6: its deadline
            // check refers to the Release it serves.
            final(self).slots.armed(cfg.slot_budget[t as int] as int),
            final(self).timing.job_due[t as int] == old(self).timing.due[t as int],
            final(self).flags_eq(old(self)),
            final(self).timing.has_last == old(self).timing.has_last,
            final(self).timing.last_due == old(self).timing.last_due,
            final(self).mit_spaced(old(self), cfg),
    {
        proof {
            // The executing Job is in progress and `t` is not, so they differ.
            if self.sched.depth > 0 {
                assert(self.sched.in_progress(self.sched.top()));
                assert(self.sched.top() != t as int);
            }
        }
        if self.sched.depth > 0 {
            let prev = self.sched.stack[self.sched.depth - 1];
            self.budget.charge(prev, preempted_cycles);
        }
        self.sched.start(cfg, t);
        self.timing.job_started(cfg, t);
        self.budget.reset(t);
        self.arm_budget(cfg, t);
        self.record(cfg, ST_DISPATCH, Ghost(t as int), Ghost(0));
    }

    /// Arms the Budget event of the executing Job of `t` for its remaining Budget.
    fn arm_budget(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).sched == old(self).sched, final(self).timing == old(self).timing,
            final(self).started == old(self).started,
            final(self).budget == old(self).budget, final(self).flags_eq(old(self)),
            final(self).slots.armed(cfg.slot_budget[t as int] as int),
            forall|j: int| 0 <= j < NS && j != cfg.slot_budget[t as int] ==> final(self).slots.slots[j] == old(self).slots.slots[j],
    {
        proof {
            assert(cfg.slot_ok(t as int));
        }
        let remaining = self.budget.remaining(cfg, t) as u64;
        let ticks = (remaining + CYCLES_PER_TICK - 1) / CYCLES_PER_TICK;
        let at = if self.now + ticks > cfg.operating_duration { cfg.operating_duration } else { self.now + ticks };
        self.slots.arm(cfg.slot_budget[t as usize] as usize, at, SLOT_BUDGET, t);
    }

    /// Dispatch stub exit (the Task body returned, or Kernel_Arch unwound to
    /// a killed Job): charges the Job's last segment, completes it, disarms
    /// its Budget and deadline events, re-arms the Budget event of the
    /// resumed Job, and reports any Resources it still held (R8.9): as a
    /// forced release for a killed Job, as API misuse (PR-07) for a Job that
    /// returned holding one.
    pub fn job_return(&mut self, cfg: &Config<NT, NR, NP>, elapsed_cycles: u32)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // `Sched::finish_top` lifted (Property 2 completion, R7.5, R8.9).
            final(self).ended_top(old(self), cfg),
            old(self).sched.top_holds_nothing() ==> final(self).sched.lock_len == old(self).sched.lock_len
                && final(self).sched.ceiling == old(self).sched.ceiling,
            // Property 10 attribution (R18.1): only the ended Job is charged.
            final(self).budget.used[old(self).sched.top()]
                == super::budget::charged(old(self).budget.used[old(self).sched.top()], elapsed_cycles),
            forall|u: int| 0 <= u < NT && u != old(self).sched.top() ==> final(self).budget.used[u] == old(self).budget.used[u],
            // R9.6, R18.2: the ended Job's deadline and Budget events are
            // disarmed; the resumed Job's Budget event is armed.
            !final(self).slots.armed(cfg.slot_deadline[old(self).sched.top()] as int),
            !final(self).slots.armed(cfg.slot_budget[old(self).sched.top()] as int),
            final(self).sched.depth > 0 ==> final(self).slots.armed(cfg.slot_budget[final(self).sched.top()] as int),
            final(self).flags_eq(old(self)),
            final(self).timing.has_last == old(self).timing.has_last,
            final(self).timing.last_due == old(self).timing.last_due,
            final(self).mit_spaced(old(self), cfg),
    {
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.slot_ok(t as int));
        }
        self.budget.charge(t, elapsed_cycles);
        // Resources are released in both cases (R8.9), but the two causes are
        // logged apart: a killed Job's Resources are released by force
        // (EV_RESOURCE_RELEASED); a Job that returned to its Release_Point
        // still holding a Resource broke PR-07, which the type system should
        // have rejected, and is logged as API misuse (EV_API_MISUSE).
        let killed = self.sched.job[t as usize] == JOB_KILLED;
        let released = self.sched.finish_top(cfg);
        proof {
            self.lemma_rel_shrink(old(self), ST_END, t as int, if killed { 1int } else { 0int });
        }
        self.record(cfg, ST_END, Ghost(t as int), Ghost(if killed { 1int } else { 0int }));
        if released > 0 {
            let partition = cfg.tasks[t as usize].partition;
            let kind = if killed { EV_RESOURCE_RELEASED } else { EV_API_MISUSE };
            self.push_event(cfg, kind, t, partition, 1u64 << 7, released as u32);
        }
        self.slots.disarm(cfg.slot_budget[t as usize] as usize);
        self.slots.disarm(cfg.slot_deadline[t as usize] as usize);
        if cfg.tasks[t as usize].kind == KIND_SPORADIC {
            let now = self.now;
            let _ = self.timing.try_unmask(cfg, t, now, true);
        }
        if self.sched.depth > 0 {
            let resumed = self.sched.stack[self.sched.depth - 1];
            proof {
                // The resumed Job is not the ended one, so its Budget slot is not
                // one of the ended Job's slots.
                assert(old(self).sched.stack[self.sched.depth - 1] != old(self).sched.stack[old(self).sched.depth - 1]);
                assert(cfg.slots_distinct(resumed as int, t as int));
                assert(cfg.slots_distinct(t as int, resumed as int));
            }
            self.arm_budget(cfg, resumed);
        }
    }

    fn top_holds_nothing_exec(&self) -> (b: bool)
        requires self.sched.lock_len <= NL, self.sched.depth > 0,
        ensures b == self.sched.top_holds_nothing(),
    {
        let mut e: usize = 0;
        let mut all = true;
        while e < self.sched.lock_len
            invariant e <= self.sched.lock_len, self.sched.lock_len <= NL, self.sched.depth > 0,
                all == forall|f: int| 0 <= f < e ==> self.sched.lock_depth[f] < self.sched.depth - 1,
            decreases self.sched.lock_len - e,
        {
            if !(self.sched.lock_depth[e] < self.sched.depth - 1) {
                all = false;
            }
            e = e + 1;
        }
        all
    }

    // ------------------------------------------------------------ SVC services

    /// SVC lock (R8.1, R8.7, R8.10). Returns whether the lock was taken;
    /// Kernel_Arch then writes BASEPRI from `sched.ceiling`.
    pub fn svc_lock(&mut self, cfg: &Config<NT, NR, NP>, r: u8) -> (ok: bool)
        requires old(self).inv(cfg), old(self).sched.depth > 0, (r as int) < NR,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            ok == (cfg.may_access(old(self).sched.top(), r as int) && !old(self).sched.held(r as int) && old(self).sched.lock_len < NL),
            // `Sched::lock` lifted (Property 3 lock entry, R8.1; Property 4, R8.3).
            ok ==> final(self).sched.lock_len == old(self).sched.lock_len + 1
                && final(self).sched.lock_res[old(self).sched.lock_len as int] == r
                && final(self).sched.entry_of_top(old(self).sched.lock_len as int)
                && final(self).sched.ceiling == (if cfg.ceiling(r as int) > old(self).sched.ceiling { cfg.ceiling(r as int) } else { old(self).sched.ceiling }),
            ok ==> forall|e: int| 0 <= e < old(self).sched.lock_len
                ==> final(self).sched.lock_res[e] == old(self).sched.lock_res[e] && final(self).sched.lock_depth[e] == old(self).sched.lock_depth[e],
            !ok ==> final(self).sched.ceiling == old(self).sched.ceiling && final(self).sched.lock_len == old(self).sched.lock_len
                && final(self).sched.lock_res == old(self).sched.lock_res && final(self).sched.lock_depth == old(self).sched.lock_depth,
            cfg.may_access(old(self).sched.top(), r as int) ==> forall|e: int| 0 <= e < old(self).sched.lock_len
                && old(self).sched.lock_res[e] == r ==> old(self).sched.entry_of_top(e),
            final(self).sched.depth == old(self).sched.depth, final(self).sched.stack == old(self).sched.stack,
            final(self).sched.job == old(self).sched.job, final(self).sched.release == old(self).sched.release,
            final(self).sched.start_ceiling == old(self).sched.start_ceiling,
            final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).budget == old(self).budget, final(self).flags_eq(old(self)),
            final(self).mit_spaced(old(self), cfg),
    {
        let ok = self.sched.lock(cfg, r);
        if ok {
            proof {
                self.lemma_rel_lock(old(self), self.sched.top(), r as int);
            }
            self.record(cfg, ST_LOCK, Ghost(self.sched.top()), Ghost(r as int));
        }
        if !ok {
            let t = self.sched.stack[self.sched.depth - 1];
            self.push_event(cfg, EV_API_MISUSE, t, cfg.tasks[t as usize].partition, 1u64 << 7, r as u32);
        }
        ok
    }

    /// SVC unlock (R8.2, R8.6).
    pub fn svc_unlock(&mut self, cfg: &Config<NT, NR, NP>, r: u8) -> (ok: bool)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // `Sched::unlock` lifted (Property 3 lock exit, R8.2, R8.6).
            ok == (old(self).sched.lock_len > 0 && old(self).sched.lock_res[old(self).sched.lock_len - 1] == r
                && old(self).sched.entry_of_top(old(self).sched.lock_len - 1)),
            ok ==> final(self).sched.lock_len == old(self).sched.lock_len - 1
                && final(self).sched.ceiling == super::sched::max_ceiling(cfg, old(self).sched.lock_res, old(self).sched.lock_len - 1),
            !ok ==> final(self).sched.lock_len == old(self).sched.lock_len && final(self).sched.ceiling == old(self).sched.ceiling,
            final(self).sched.lock_res == old(self).sched.lock_res, final(self).sched.lock_depth == old(self).sched.lock_depth,
            final(self).sched.depth == old(self).sched.depth, final(self).sched.stack == old(self).sched.stack,
            final(self).sched.job == old(self).sched.job, final(self).sched.release == old(self).sched.release,
            final(self).sched.start_ceiling == old(self).sched.start_ceiling,
            final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).budget == old(self).budget, final(self).flags_eq(old(self)),
            final(self).mit_spaced(old(self), cfg),
    {
        let ok = self.sched.unlock(cfg, r);
        if ok {
            proof {
                self.lemma_rel_unlock(old(self), self.sched.top(), r as int);
            }
            self.record(cfg, ST_UNLOCK, Ghost(self.sched.top()), Ghost(r as int));
        }
        if !ok {
            let t = self.sched.stack[self.sched.depth - 1];
            self.push_event(cfg, EV_API_MISUSE, t, cfg.tasks[t as usize].partition, 1u64 << 7, r as u32);
        }
        ok
    }

    // --------------------------------------------------------- release events

    /// A Release_Source event of Sporadic_Task `t` (R10.1 to R10.3, R10.7):
    /// an interrupt intake (DD-06) or a Release_Signal raise (R10.5).
    /// `source_partition` is the Partition the violation is attributed to.
    pub fn intake(&mut self, cfg: &Config<NT, NR, NP>, t: u8, source_partition: u8, now: u64)
        requires old(self).inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_SPORADIC,
            (source_partition as int) < NP, now <= cfg.operating_duration,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // R9.9, R12.4, R14.5: an event in the safe state, before instant 0,
            // for a stopped Partition, or too late for its Job to complete
            // within the operating duration makes no Release.
            !old(self).intake_active(cfg, t as int, Self::observed(old(self).now, now))
                ==> final(self).unchanged_but_log(old(self)),
            // R10.1: the event is accepted as a Release due now, at least one
            // MIT after the previous accepted one.
            old(self).intake_active(cfg, t as int, Self::observed(old(self).now, now))
                && old(self).mit_accepts(cfg, t as int, Self::observed(old(self).now, now))
                ==> final(self).accepted(old(self), cfg, t as int, Self::observed(old(self).now, now)),
            // R10.2: the event is deferred until one MIT after the previous
            // accepted Release, and one MIT_Violation is recorded against the
            // source Partition (R10.7) from the deferred state.
            old(self).intake_active(cfg, t as int, Self::observed(old(self).now, now))
                && old(self).mit_defers(cfg, t as int, Self::observed(old(self).now, now))
                ==> exists|d: Self| #[trigger] d.deferred(old(self), cfg, t as int)
                    && final(self).violation_applied(&d, cfg, source_partition as int, t as int),
            // R10.2 under the discard policy, and R10.3 (a Release is already
            // pending or deferred): the event is discarded, the Task's timing
            // state is unchanged, and one MIT_Violation is recorded against
            // the source Partition; below the threshold the existing Release
            // is unchanged.
            old(self).intake_active(cfg, t as int, Self::observed(old(self).now, now))
                && !old(self).mit_accepts(cfg, t as int, Self::observed(old(self).now, now))
                && !old(self).mit_defers(cfg, t as int, Self::observed(old(self).now, now))
                ==> final(self).timing == old(self).timing
                    && final(self).violation_applied(old(self), cfg, source_partition as int, t as int),
            final(self).mit_spaced(old(self), cfg),
    {
        self.observe(cfg, now);
        let now = self.now;
        proof {
            assert(cfg.slot_ok(t as int));
            assert(cfg.task_ok(t as int));
        }
        let partition = cfg.tasks[t as usize].partition as usize;
        if self.safe_state || !self.started || self.stopped[partition] {
            return;
        }
        if now + cfg.tasks[t as usize].deadline > cfg.operating_duration {
            // R9.9: the Job could not complete within the operating duration.
            return;
        }
        let release_none = self.sched.release[t as usize] == REL_NONE;
        let d = self.timing.intake_decision(cfg, t, release_none, now);
        if d == INTAKE_ACCEPT {
            self.timing.accept(cfg, t, now);
            self.sched.make_pending(cfg, t);
            let deadline = now + cfg.tasks[t as usize].deadline;
            self.slots.arm(cfg.slot_deadline[t as usize] as usize, deadline, SLOT_DEADLINE, t);
            self.record(cfg, ST_RELEASE, Ghost(t as int), Ghost(now as int));
        } else {
            if d == INTAKE_DEFER && release_none {
                proof {
                    // Only the late-event case defers, so a previous Release exists.
                    if !self.timing.has_last[t as int] {
                        assert(d == INTAKE_ACCEPT);
                    }
                    assert(self.timing.has_last[t as int]);
                    assert(cfg.tasks[t as int].mit_policy == MIT_DEFER);
                }
                let at = self.timing.defer(cfg, t);
                let at = if at > cfg.operating_duration { cfg.operating_duration } else { at };
                self.sched.defer(cfg, t);
                self.slots.arm(cfg.slot_release[t as usize] as usize, at, SLOT_DEFER, t);
            }
            // The violation may trip the threshold and stop or restart the
            // Partition, which then suppresses the Release just deferred.
            let ghost before = *self;
            self.mit_violation(cfg, t, source_partition);
            proof {
                self.lemma_violation_keeps_timing(&before, cfg, source_partition as int, t as int);
                if d == INTAKE_DEFER && release_none {
                    assert(before.deferred(old(self), cfg, t as int));
                    assert(before.deferred(old(self), cfg, t as int)
                        && self.violation_applied(&before, cfg, source_partition as int, t as int));
                } else {
                    assert(before.unchanged_but_log(old(self)));
                    self.lemma_violation_rebase(&before, old(self), cfg, source_partition as int, t as int);
                }
            }
        }
    }

    /// Records an MIT_Violation against `partition` and applies the fault
    /// response when the threshold is exceeded (R10.4).
    fn mit_violation(&mut self, cfg: &Config<NT, NR, NP>, t: u8, partition: u8)
        requires old(self).inv(cfg), (t as int) < NT, (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).violation_applied(old(self), cfg, partition as int, t as int),
    {
        self.push_event(cfg, EV_MIT_VIOLATION, t, partition, PRS_RELEASE, 0);
        let count = self.mit_violations[partition as usize].saturating_add(1);
        self.mit_violations[partition as usize] = count;
        let ghost mid = *self;
        proof {
            assert(count == Self::incremented(old(self).mit_violations[partition as int]));
            assert(mid.counted(old(self), partition as int, count));
        }
        if count > cfg.partitions[partition as usize].mit_threshold {
            let response = cfg.partitions[partition as usize].fault_response;
            self.apply_response(cfg, partition, response, EV_MIT_VIOLATION, t);
            proof {
                assert(mid.counted(old(self), partition as int, Self::incremented(old(self).mit_violations[partition as int]))
                    && self.response_applied(&mid, cfg, partition as int, response, t as int));
            }
        }
    }

    // ------------------------------------------------------------ timer events

    /// The time base reached the armed compare instant: every due slot is
    /// served once, in slot order (bounded by `NS`, PR-16). Kernel_Arch then
    /// arms the compare at `slots.next()`.
    pub fn timer_event(&mut self, cfg: &Config<NT, NR, NP>, now: u64, running_cycles: u32)
        requires old(self).inv(cfg), now <= cfg.operating_duration,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // R9.9, R14.4: in the safe state no timed event takes effect.
            old(self).safe_state ==> final(self).sched == old(self).sched && final(self).slots == old(self).slots,
            // Property 8: deferred Releases that become due keep the MIT spacing.
            final(self).mit_spaced(old(self), cfg),
    {
        self.observe(cfg, now);
        if self.safe_state {
            return;
        }
        let mut i: usize = 0;
        while i < NS
            invariant self.inv(cfg), self.history_extends(old(self)), i <= NS, (cfg.slot_count as int) == NS,
                self.mit_spaced(old(self), cfg),
            decreases NS - i,
        {
            let slot = self.slots.slots[i];
            if slot.armed && slot.at <= self.now {
                let ghost before = *self;
                self.serve_slot(cfg, i, running_cycles);
                proof {
                    Self::lemma_mit_spaced_trans(old(self), &before, self, cfg);
                }
            }
            i = i + 1;
        }
    }

    fn serve_slot(&mut self, cfg: &Config<NT, NR, NP>, i: usize, running_cycles: u32)
        requires old(self).inv(cfg), i < NS, old(self).slots.slots[i as int].armed,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).mit_spaced(old(self), cfg),
    {
        let slot = self.slots.slots[i];
        let t = slot.task;
        if slot.kind == SLOT_END {
            self.slots.disarm(i);
            self.push_event(cfg, EV_OPERATING_END, 0xFF, 0xFF, 0, 0);
            self.enter_safe_state(cfg);
        } else if slot.kind == SLOT_HOUSEKEEPING {
            let next = if self.now + HOUSEKEEPING_PERIOD > cfg.operating_duration { cfg.operating_duration } else { self.now + HOUSEKEEPING_PERIOD };
            self.slots.arm(i, next, SLOT_HOUSEKEEPING, 0xFF);
        } else if (t as usize) < NT {
            if slot.kind == SLOT_RELEASE {
                self.periodic_release(cfg, t, i);
            } else if slot.kind == SLOT_DEFER {
                self.deferred_release(cfg, t, i);
            } else if slot.kind == SLOT_DEADLINE {
                self.deadline_miss(cfg, t, i);
            } else if slot.kind == SLOT_BUDGET {
                self.budget_event(cfg, t, i, running_cycles);
            } else {
                self.slots.disarm(i);
            }
        } else {
            self.slots.disarm(i);
        }
    }

    /// R9.1, R9.2, R9.7, R9.8: the nominal instant of Periodic_Task `t`
    /// arrived. The Release becomes pending unless one is already pending
    /// (discard with MIT_Violation), and the next nominal instant is armed.
    fn periodic_release(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // A periodic Release never touches the MIT history.
            final(self).timing.has_last == old(self).timing.has_last,
            final(self).timing.last_due == old(self).timing.last_due,
            final(self).timing.deferred_until == old(self).timing.deferred_until,
            // R9.8, R10.7: a periodic Release that becomes due while one is
            // pending is discarded, and one MIT_Violation is recorded against
            // the Task's own Partition; below the threshold the pending
            // Release is unchanged. Only the next nominal instant is armed
            // after it.
            old(self).periodic_discards(cfg, t as int) ==> exists|v: Self|
                #[trigger] v.violation_applied(old(self), cfg, cfg.tasks[t as int].partition as int, t as int)
                    && final(self).sched == v.sched && final(self).budget == v.budget && final(self).flags_eq(&v),
    {
        proof {
            assert(cfg.slot_ok(t as int));
            assert(cfg.task_ok(t as int));
        }
        let partition = cfg.tasks[t as usize].partition;
        let nominal = self.timing.nominal[t as usize];
        if cfg.tasks[t as usize].kind != KIND_PERIODIC {
            self.slots.disarm(i);
            return;
        }
        // R9.9: a Job whose absolute deadline lies beyond the operating
        // duration can never complete within it; its Release is not made.
        let within_duration = nominal + cfg.tasks[t as usize].deadline <= cfg.operating_duration;
        if !self.stopped[partition as usize] && within_duration {
            if self.sched.release[t as usize] == REL_NONE {
                if self.sched.job[t as usize] != JOB_NONE {
                    // R9.7: the previous Job has not completed; the Release waits.
                    self.push_event(cfg, EV_RELEASE_OVERLAP, t, partition, PRS_RELEASE, 0);
                }
                self.sched.make_pending(cfg, t);
                self.timing.periodic_due(cfg, t, nominal);
                let deadline = nominal + cfg.tasks[t as usize].deadline;
                self.slots.arm(cfg.slot_deadline[t as usize] as usize, deadline, SLOT_DEADLINE, t);
                self.record(cfg, ST_RELEASE, Ghost(t as int), Ghost(nominal as int));
            } else {
                // R9.8: a Release is already pending; discard this one.
                let ghost before = *self;
                self.mit_violation(cfg, t, partition);
                proof {
                    self.lemma_violation_keeps_timing(&before, cfg, partition as int, t as int);
                }
            }
        }
        let ghost v = *self;
        match self.timing.advance_periodic(cfg, t) {
            Some(next) => { self.slots.arm(i, next, SLOT_RELEASE, t); }
            None => { self.slots.disarm(i); }
        }
        proof {
            if old(self).periodic_discards(cfg, t as int) {
                assert(v.violation_applied(old(self), cfg, partition as int, t as int)
                    && self.sched == v.sched && self.budget == v.budget && self.flags_eq(&v));
            }
        }
    }

    /// R10.2: a deferred Release becomes due.
    fn deferred_release(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // R10.2: the deferred Release becomes due, and pending, exactly one
            // MIT after the previous accepted Release, which it then replaces
            // as the previous accepted due instant.
            old(self).deferral_fires(cfg, t as int) ==> final(self).sched.release[t as int] == REL_PENDING
                && final(self).timing.has_last[t as int]
                && final(self).timing.due[t as int] == old(self).timing.deferred_until[t as int]
                && final(self).timing.last_due[t as int] == old(self).timing.deferred_until[t as int]
                && final(self).timing.last_due[t as int] as int
                    == old(self).timing.last_due[t as int] as int + cfg.tasks[t as int].period as int,
            // R9.9: a deferred Release whose Job could not complete within the
            // operating duration is dropped.
            !old(self).stopped[cfg.tasks[t as int].partition as int]
                && old(self).sched.release[t as int] == REL_DEFERRED && !old(self).deferral_fires(cfg, t as int)
                ==> final(self).sched.release[t as int] == REL_NONE,
            // A deferral event of a stopped Partition, or for a Release no
            // longer deferred, changes nothing but its slot.
            old(self).stopped[cfg.tasks[t as int].partition as int] || old(self).sched.release[t as int] != REL_DEFERRED
                ==> final(self).sched == old(self).sched && final(self).timing == old(self).timing,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).sched.release[u] == old(self).sched.release[u],
            final(self).jobs_eq(old(self)), final(self).budget == old(self).budget, final(self).flags_eq(old(self)),
            final(self).mit_spaced(old(self), cfg),
    {
        proof {
            assert(cfg.slot_ok(t as int));
            assert(cfg.task_ok(t as int));
        }
        self.slots.disarm(i);
        let partition = cfg.tasks[t as usize].partition;
        if self.stopped[partition as usize] || self.sched.release[t as usize] != REL_DEFERRED {
            return;
        }
        let at = self.timing.deferred_until[t as usize];
        if at + cfg.tasks[t as usize].deadline > cfg.operating_duration {
            // R9.9: cannot complete within the operating duration.
            self.sched.clear_release(cfg, t);
            return;
        }
        self.timing.deferred_due_clamped(cfg, t, at);
        self.sched.make_pending(cfg, t);
        let deadline = at + cfg.tasks[t as usize].deadline;
        self.slots.arm(cfg.slot_deadline[t as usize] as usize, deadline, SLOT_DEADLINE, t);
        self.record(cfg, ST_RELEASE, Ghost(t as int), Ghost(at as int));
    }

    /// R9.6: the absolute deadline of a Release passed while its Job had
    /// not completed (the slot is disarmed on completion).
    fn deadline_miss(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).timing == old(self).timing,
    {
        self.slots.disarm(i);
        proof {
            assert(cfg.task_ok(t as int));
        }
        let partition = cfg.tasks[t as usize].partition;
        self.push_event(cfg, EV_DEADLINE_MISS, t, partition, PRS_BUDGET, 0);
        let ghost before = *self;
        let response = cfg.partitions[partition as usize].deadline_response;
        self.apply_response(cfg, partition, response, EV_DEADLINE_MISS, t);
        proof {
            self.lemma_response_keeps_timing(&before, cfg, partition as int, response, t as int);
        }
    }

    /// R18.2, R18.3: the Budget compare of the executing Job fired. The
    /// cycles consumed since it last resumed are charged; an Overrun
    /// triggers the Partition's Overrun_Response, otherwise the compare is
    /// re-armed for the remainder.
    fn budget_event(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize, running_cycles: u32)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).timing == old(self).timing,
    {
        self.slots.disarm(i);
        if self.sched.depth == 0 || self.sched.stack[self.sched.depth - 1] != t {
            return;
        }
        self.budget.charge(t, running_cycles);
        if self.budget.overrun(cfg, t) {
            proof {
                assert(cfg.task_ok(t as int));
            }
            let partition = cfg.tasks[t as usize].partition;
            self.push_event(cfg, EV_OVERRUN, t, partition, PRS_BUDGET, self.budget.used[t as usize]);
            let ghost before = *self;
            let response = cfg.partitions[partition as usize].overrun_response;
            self.apply_response(cfg, partition, response, EV_OVERRUN, t);
            proof {
                self.lemma_response_keeps_timing(&before, cfg, partition as int, response, t as int);
            }
        } else {
            self.arm_budget(cfg, t);
        }
    }

    // ------------------------------------------------------- Health_Monitor

    /// A fault attributed to the executing Job's Partition (R14.3), or to
    /// the Kernel when `kernel` is set (R14.4).
    pub fn fault(&mut self, cfg: &Config<NT, NR, NP>, kind: u8, kernel: bool)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // R14.4: a Kernel fault, or a fault with no Job executing, enters
            // the safe state.
            kernel || old(self).sched.depth == 0 ==> final(self).safe_state && final(self).no_release()
                && final(self).jobs_eq(old(self)),
            // R14.3, R14.5: otherwise the executing Job's Partition receives its
            // declared fault response.
            !kernel && old(self).sched.depth > 0 ==> final(self).response_applied(old(self), cfg,
                cfg.tasks[old(self).sched.top()].partition as int,
                cfg.partitions[cfg.tasks[old(self).sched.top()].partition as int].fault_response,
                old(self).sched.top()),
            // Every response keeps the MIT history (R14.5).
            final(self).timing == old(self).timing, final(self).mit_spaced(old(self), cfg),
    {
        if kernel || self.sched.depth == 0 {
            self.push_event(cfg, kind, 0xFF, 0xFF, PRS_PANIC, 0);
            self.enter_safe_state(cfg);
            return;
        }
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.task_ok(t as int));
        }
        let partition = cfg.tasks[t as usize].partition;
        self.push_event(cfg, kind, t, partition, PRS_PANIC, 0);
        let ghost mid = *self;
        let response = cfg.partitions[partition as usize].fault_response;
        self.apply_response(cfg, partition, response, kind, t);
        proof {
            self.lemma_same_but_log(&mid, old(self), cfg, partition as int, response, t as int);
            self.lemma_response_keeps_timing(&mid, cfg, partition as int, response, t as int);
        }
    }

    /// DD-10, R14.5: applies one response of the closed sets.
    fn apply_response(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, response: u8, cause: u8, t: u8)
        requires old(self).inv(cfg), (partition as int) < NP, (t as int) < NT,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).response_applied(old(self), cfg, partition as int, response, t as int),
    {
        if response == RESP_RECORD_ONLY || response == RESP_RECORD_AND_CONTINUE {
            return;
        }
        if response == RESP_END_JOB {
            // Ends the executing Job if it belongs to the Partition; escalates to
            // RESTART_PARTITION when the Job holds a Resource (R18.4).
            if self.sched.depth > 0 {
                let top = self.sched.stack[self.sched.depth - 1];
                if top == t {
                    let holds_nothing = self.top_holds_nothing_exec();
                    if holds_nothing {
                        self.push_event(cfg, EV_END_JOB, t, partition, 0, cause as u32);
                        let ghost mid = *self;
                        self.end_top(cfg);
                        proof {
                            self.lemma_others_same_but_log(&mid, old(self), cfg, cfg.tasks[t as int].partition as int);
                        }
                        return;
                    }
                }
            }
            self.restart_partition(cfg, partition, cause);
            return;
        }
        if response == RESP_RESTART_PARTITION {
            self.restart_partition(cfg, partition, cause);
            return;
        }
        if response == RESP_STOP_PARTITION {
            self.stop_partition(cfg, partition, cause);
            return;
        }
        self.enter_safe_state(cfg);
    }

    /// Ends the executing Job by force (R8.9) and resumes the preempted one.
    /// The first half of `end_top`: ends the executing Job by force,
    /// records the step, and disarms its events. Split from `end_top` so
    /// that the `others_unchanged` proof there and the lock-snapshot
    /// bookkeeping here (task 4.13) are separate solver queries.
    fn end_top_release(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).ended_top(old(self), cfg),
            final(self).flags_eq(old(self)),
            final(self).budget == old(self).budget, final(self).timing == old(self).timing,
            !final(self).slots.armed(cfg.slot_budget[old(self).sched.top()] as int),
            !final(self).slots.armed(cfg.slot_deadline[old(self).sched.top()] as int),
            forall|j: int| 0 <= j < NS && j != cfg.slot_budget[old(self).sched.top()] && j != cfg.slot_deadline[old(self).sched.top()]
                ==> final(self).slots.slots[j] == old(self).slots.slots[j],
    {
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.slot_ok(t as int));
            assert(cfg.task_ok(t as int));
        }
        let released = self.sched.end_job(cfg);
        proof {
            self.lemma_rel_shrink(old(self), ST_KILL, t as int, 1);
        }
        self.record(cfg, ST_KILL, Ghost(t as int), Ghost(1));
        if released > 0 {
            self.push_event(cfg, EV_RESOURCE_RELEASED, t, cfg.tasks[t as usize].partition, 1u64 << 7, released as u32);
        }
        self.slots.disarm(cfg.slot_budget[t as usize] as usize);
        self.slots.disarm(cfg.slot_deadline[t as usize] as usize);
    }

    fn end_top(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            // `Sched::end_job` lifted (Property 6, R8.9).
            final(self).ended_top(old(self), cfg),
            final(self).flags_eq(old(self)),
            final(self).budget == old(self).budget, final(self).timing == old(self).timing,
            // R14.9: only the ended Job's Partition is affected.
            final(self).others_unchanged(old(self), cfg, cfg.tasks[old(self).sched.top()].partition as int),
    {
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.slot_ok(t as int));
            assert(cfg.task_ok(t as int));
        }
        self.end_top_release(cfg);
        if self.sched.depth > 0 {
            let resumed = self.sched.stack[self.sched.depth - 1];
            proof {
                assert(cfg.task_ok(resumed as int));
            }
            self.arm_budget(cfg, resumed);
        }
        proof {
            let pt = cfg.tasks[t as int].partition as int;
            // Every other Partition's Task keeps its slots: the ones touched
            // here are t's Budget and deadline slots and the resumed Job's
            // Budget slot.
            assert forall|u: int| 0 <= u < NT && (cfg.tasks[u].partition as int) != pt
                implies #[trigger] self.task_unchanged(old(self), cfg, u) by {
                Self::lemma_slots_apart(cfg, t as int, u);
                Self::lemma_slots_own(cfg, u);
                if self.sched.depth > 0 {
                    let r = self.sched.top();
                    if r != u {
                        Self::lemma_slots_apart(cfg, r, u);
                    }
                }
            }
            // Another Partition's lock entries are not the ended Job's, so they
            // sit below the new top of the lock stack.
            assert forall|e: int| 0 <= e < old(self).sched.lock_len && #[trigger] old(self).holder_partition(cfg, e) != pt
                implies e < self.sched.lock_len by {
                assert(!old(self).sched.entry_of_top(e));
            }
            Sched::<NT, NR, NL>::lemma_max_ceiling_mono(cfg, old(self).sched.lock_res,
                self.sched.lock_len as int, old(self).sched.lock_len as int);
        }
    }

    /// Ends every Job of `partition`: the executing one at once, preempted
    /// ones when Kernel_Arch unwinds to them (`kill`); suppresses its
    /// Releases and disarms their deadline and deferral events. Periodic
    /// release events stay armed at the next nominal instants (R14.5); MIT
    /// history is kept. Other Partitions are untouched (R14.9,
    /// `others_unchanged`): only the Partition's own Jobs are ended or
    /// killed, and only its own Releases, events, and Budget accounts change.
    fn end_partition_jobs(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).partition_ended(cfg, partition as int),
            final(self).stop_dma[partition as int],
            // R14.9: every other Partition is as it was; MIT history is kept.
            final(self).others_unchanged(old(self), cfg, partition as int),
            final(self).timing == old(self).timing,
    {
        // The executing Job first, if it belongs to the Partition.
        if self.sched.depth > 0 {
            let top = self.sched.stack[self.sched.depth - 1];
            proof {
                assert(cfg.task_ok(top as int));
            }
            if cfg.tasks[top as usize].partition == partition {
                self.end_top(cfg);
            }
        }
        // Preempted Jobs of the Partition are marked killed.
        let mut i: usize = 0;
        while i < self.sched.depth
            invariant self.inv(cfg), self.history_extends(old(self)), i <= self.sched.depth, (partition as int) < NP,
                forall|j: int| 0 <= j < i && cfg.tasks[self.sched.stack[j] as int].partition == partition
                    ==> self.sched.job[self.sched.stack[j] as int] != JOB_RUNNING,
                self.others_unchanged(old(self), cfg, partition as int),
                self.timing == old(self).timing,
            decreases self.sched.depth - i,
        {
            let u = self.sched.stack[i];
            proof {
                // Stack entries are distinct, so killing entry i leaves the
                // Jobs below it as they were.
                assert forall|j: int| 0 <= j < i implies self.sched.stack[j] != u by {}
            }
            if cfg.tasks[u as usize].partition == partition && self.sched.job[u as usize] != JOB_NONE {
                self.sched.kill(cfg, i);
                self.record(cfg, ST_KILL, Ghost(u as int), Ghost(0));
            }
            i = i + 1;
        }
        proof {
            // Every Job of the Partition in progress is on the stack, so it was
            // reached by the loop.
            assert forall|u: int| 0 <= u < NT && cfg.tasks[u].partition == partition
                implies self.sched.job[u] != JOB_RUNNING by {
                if self.sched.job[u] != JOB_NONE {
                    assert(self.sched.in_progress(u));
                    assert(self.sched.on_stack(u));
                    let j = choose|j: int| 0 <= j < self.sched.depth && self.sched.stack[j] == u;
                    assert(cfg.tasks[self.sched.stack[j] as int].partition == partition);
                }
            }
        }
        // Releases and their events.
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), self.history_extends(old(self)), t <= NT, (cfg.slot_count as int) == NS, (partition as int) < NP,
                forall|u: int| 0 <= u < NT && cfg.tasks[u].partition == partition ==> self.sched.job[u] != JOB_RUNNING,
                forall|u: int| 0 <= u < t && cfg.tasks[u].partition == partition ==> self.sched.release[u] == REL_NONE,
                self.others_unchanged(old(self), cfg, partition as int),
                self.timing == old(self).timing,
            decreases NT - t,
        {
            if cfg.tasks[t].partition == partition {
                proof {
                    assert(cfg.slot_ok(t as int));
                    assert(cfg.task_ok(t as int));
                }
                self.sched.clear_release(cfg, t as u8);
                self.slots.disarm(cfg.slot_deadline[t] as usize);
                if cfg.tasks[t].kind == KIND_SPORADIC && cfg.tasks[t].mit_policy == MIT_DEFER {
                    self.slots.disarm(cfg.slot_release[t] as usize);
                }
                self.budget.reset(t as u8);
                proof {
                    // The slots disarmed here are t's, distinct from every other
                    // Partition's Tasks' slots.
                    assert forall|u: int| 0 <= u < NT && (cfg.tasks[u].partition as int) != partition as int
                        implies self.task_unchanged(old(self), cfg, u) by {
                        Self::lemma_slots_apart(cfg, t as int, u);
                    }
                }
            }
            t = t + 1;
        }
        self.stop_dma[partition as usize] = true;
    }

    fn restart_partition(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, cause: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)),
            final(self).partition_ended(cfg, partition as int),
            final(self).reinit_partition[partition as int], final(self).stop_dma[partition as int],
            final(self).mit_violations[partition as int] == 0,
            final(self).others_unchanged(old(self), cfg, partition as int),
            final(self).timing == old(self).timing,
    {
        self.push_event(cfg, EV_PARTITION_RESTART, 0xFF, partition, 0, cause as u32);
        let ghost mid = *self;
        self.end_partition_jobs(cfg, partition);
        self.mit_violations[partition as usize] = 0;
        self.reinit_partition[partition as usize] = true;
        proof {
            self.lemma_others_same_but_log(&mid, old(self), cfg, partition as int);
        }
    }

    fn stop_partition(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, cause: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).stopped[partition as int],
            final(self).partition_ended(cfg, partition as int), final(self).stop_dma[partition as int],
            final(self).others_unchanged(old(self), cfg, partition as int),
            final(self).timing == old(self).timing,
    {
        self.push_event(cfg, EV_PARTITION_STOP, 0xFF, partition, 0, cause as u32);
        let ghost mid = *self;
        self.end_partition_jobs(cfg, partition);
        self.stopped[partition as usize] = true;
        proof {
            self.lemma_others_same_but_log(&mid, old(self), cfg, partition as int);
        }
    }

    /// R9.9, R12.2, R14.4: the system safe state. No Release becomes
    /// effective afterwards; Kernel_Arch applies the declared outputs.
    pub fn enter_safe_state(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).safe_state,
            // No Release stays pending or deferred; the Jobs in progress and
            // the lock stack are untouched (Kernel_Arch applies the outputs).
            final(self).no_release(), final(self).jobs_eq(old(self)),
            final(self).started == old(self).started, final(self).stopped == old(self).stopped,
            final(self).budget == old(self).budget, final(self).slots == old(self).slots,
            final(self).timing == old(self).timing, final(self).mit_spaced(old(self), cfg),
    {
        if !self.safe_state {
            self.push_event(cfg, EV_SAFE_STATE, 0xFF, 0xFF, 0, 0);
        }
        self.safe_state = true;
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), self.history_extends(old(self)), t <= NT, self.safe_state,
                self.jobs_eq(old(self)),
                self.started == old(self).started, self.stopped == old(self).stopped,
                self.budget == old(self).budget, self.slots == old(self).slots,
                self.timing == old(self).timing,
                forall|u: int| 0 <= u < t ==> self.sched.release[u] == REL_NONE,
            decreases NT - t,
        {
            self.sched.clear_release(cfg, t as u8);
            t = t + 1;
        }
    }

    /// R12.2: a boot check failed; the identifiers (a bit set, `detail`)
    /// are logged and the safe state entered without any Release.
    pub fn boot_check_failed(&mut self, cfg: &Config<NT, NR, NP>, failed: u32)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), final(self).safe_state,
            final(self).no_release(), final(self).jobs_eq(old(self)),
            final(self).timing == old(self).timing, final(self).mit_spaced(old(self), cfg),
    {
        self.push_event(cfg, EV_BOOT_CHECK, 0xFF, 0xFF, 0, failed);
        self.enter_safe_state(cfg);
    }

    /// Kernel_Arch acknowledges a Partition re-initialization.
    pub fn ack_reinit(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), !final(self).reinit_partition[partition as int],
            final(self).timing == old(self).timing, final(self).mit_spaced(old(self), cfg),
            final(self).sched == old(self).sched,
    {
        self.reinit_partition[partition as usize] = false;
    }

    /// Kernel_Arch acknowledges that a Partition's EasyDMA transfers are stopped.
    pub fn ack_stop_dma(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).history_extends(old(self)), !final(self).stop_dma[partition as int],
            final(self).timing == old(self).timing, final(self).mit_spaced(old(self), cfg),
            final(self).sched == old(self).sched,
    {
        self.stop_dma[partition as usize] = false;
    }

    /// The selected Job to pend next (Property 1), for Kernel_Arch to pend
    /// its vector after a transition that may have made a Job eligible.
    /// The selection is the one that the NVIC then takes
    /// (`hw_model::lemma_dispatch_selects_srp_choice`).
    pub fn select(&self, cfg: &Config<NT, NR, NP>) -> (sel: Option<u8>)
        requires self.inv(cfg),
        ensures sel.is_some() ==> self.sched.eligible(cfg, sel.unwrap() as int),
            sel.is_some() ==> crate::hw_model::srp_selection(&self.sched, cfg, sel.unwrap() as int),
            // `Sched::select` lifted: no Job is selected only in the safe state,
            // before instant 0 (PR-34), or when no Job is eligible (R7.1).
            sel.is_some() ==> self.started && !self.safe_state,
            sel.is_none() ==> self.safe_state || !self.started
                || forall|t: int| 0 <= t < NT ==> !self.sched.eligible(cfg, t),
    {
        if self.safe_state || !self.started {
            return None;
        }
        self.sched.select(cfg)
    }
}

} // verus!
