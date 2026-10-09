//! The composed Kernel state machine: the five entry points of the design
//! (dispatch stub entry and exit, SVC, timer event, peripheral intake, and
//! fault) over the verified components, plus the Health_Monitor's closed
//! response sets (DD-10; R9, R10, R14, R18).
//!
//! Every entry point requires and re-establishes `inv`: the invariants of
//! the scheduler (`Sched::inv`), the timing state (`Timing::inv`), and the
//! log, together with the slot-table validity of the configuration.
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
use super::sched::{Sched, JOB_NONE, REL_DEFERRED, REL_NONE, REL_PENDING};
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
}

impl<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize>
    Kernel<NT, NR, NP, NL, NS, NE>
{
    pub open spec fn inv(&self, cfg: &Config<NT, NR, NP>) -> bool {
        &&& self.sched.inv(cfg)
        &&& self.timing.inv(cfg)
        &&& self.log.wf()
        &&& (cfg.slot_count as int) == NS
        &&& self.now <= cfg.operating_duration
    }

    /// Init: everything idle, no Release before `start` (PR-34, R12.4).
    pub fn new(cfg: &Config<NT, NR, NP>) -> (k: Self)
        requires cfg.wf(), NL <= 255, 0 < NE < 0x4000_0000, (cfg.slot_count as int) == NS,
        ensures k.inv(cfg), !k.started, !k.safe_state, k.sched.depth == 0,
    {
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
        }
    }

    // ------------------------------------------------------------ helpers

    fn push_event(&mut self, cfg: &Config<NT, NR, NP>, kind: u8, task: u8, partition: u8, prs: u64, detail: u32)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg),
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
        ensures final(self).inv(cfg), final(self).now == (if now > old(self).now { now } else { old(self).now }),
            final(self).sched == old(self).sched, final(self).timing == old(self).timing, final(self).slots == old(self).slots,
            final(self).started == old(self).started, final(self).safe_state == old(self).safe_state,
    {
        if now > self.now {
            self.now = now;
        }
    }

    /// PR-34: Init and the checks of Requirement 12 are complete; instant 0
    /// is now. Arms the first release of every Periodic_Task, the
    /// end-of-operation slot, and the housekeeping slot.
    pub fn start(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), !old(self).started,
        ensures final(self).inv(cfg), final(self).started,
    {
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), t <= NT, (cfg.slot_count as int) == NS,
            decreases NT - t,
        {
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
        ensures final(self).inv(cfg), final(self).sched.depth == old(self).sched.depth + 1,
            final(self).sched.top() == t as int,
    {
        if self.sched.depth > 0 {
            let prev = self.sched.stack[self.sched.depth - 1];
            self.budget.charge(prev, preempted_cycles);
        }
        self.sched.start(cfg, t);
        self.timing.job_started(cfg, t);
        self.budget.reset(t);
        self.arm_budget(cfg, t);
    }

    /// Arms the Budget event of the executing Job of `t` for its remaining Budget.
    fn arm_budget(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT,
        ensures final(self).inv(cfg), final(self).sched == old(self).sched, final(self).timing == old(self).timing,
            final(self).started == old(self).started,
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
    /// resumed Job, and reports Resources released by force (R8.9).
    pub fn job_return(&mut self, cfg: &Config<NT, NR, NP>, elapsed_cycles: u32)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).sched.depth == old(self).sched.depth - 1,
    {
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.slot_ok(t as int));
        }
        self.budget.charge(t, elapsed_cycles);
        let holds_nothing = self.top_holds_nothing_exec();
        let released = self.sched.finish_top(cfg);
        if released > 0 {
            let partition = cfg.tasks[t as usize].partition;
            let kind = if holds_nothing { EV_RESOURCE_RELEASED } else { EV_RESOURCE_RELEASED };
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
        ensures final(self).inv(cfg),
            ok == (cfg.may_access(old(self).sched.top(), r as int) && !old(self).sched.held(r as int) && old(self).sched.lock_len < NL),
    {
        let ok = self.sched.lock(cfg, r);
        if !ok {
            let t = self.sched.stack[self.sched.depth - 1];
            self.push_event(cfg, EV_API_MISUSE, t, cfg.tasks[t as usize].partition, 1u64 << 7, r as u32);
        }
        ok
    }

    /// SVC unlock (R8.2, R8.6).
    pub fn svc_unlock(&mut self, cfg: &Config<NT, NR, NP>, r: u8) -> (ok: bool)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg),
    {
        let ok = self.sched.unlock(cfg, r);
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
        ensures final(self).inv(cfg),
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
            self.mit_violation(cfg, t, source_partition);
        }
    }

    /// Records an MIT_Violation against `partition` and applies the fault
    /// response when the threshold is exceeded (R10.4).
    fn mit_violation(&mut self, cfg: &Config<NT, NR, NP>, t: u8, partition: u8)
        requires old(self).inv(cfg), (t as int) < NT, (partition as int) < NP,
        ensures final(self).inv(cfg),
    {
        self.push_event(cfg, EV_MIT_VIOLATION, t, partition, PRS_RELEASE, 0);
        let count = self.mit_violations[partition as usize].saturating_add(1);
        self.mit_violations[partition as usize] = count;
        if count > cfg.partitions[partition as usize].mit_threshold {
            let response = cfg.partitions[partition as usize].fault_response;
            self.apply_response(cfg, partition, response, EV_MIT_VIOLATION, t);
        }
    }

    // ------------------------------------------------------------ timer events

    /// The time base reached the armed compare instant: every due slot is
    /// served once, in slot order (bounded by `NS`, PR-16). Kernel_Arch then
    /// arms the compare at `slots.next()`.
    pub fn timer_event(&mut self, cfg: &Config<NT, NR, NP>, now: u64, running_cycles: u32)
        requires old(self).inv(cfg), now <= cfg.operating_duration,
        ensures final(self).inv(cfg),
    {
        self.observe(cfg, now);
        if self.safe_state {
            return;
        }
        let mut i: usize = 0;
        while i < NS
            invariant self.inv(cfg), i <= NS, (cfg.slot_count as int) == NS,
            decreases NS - i,
        {
            let slot = self.slots.slots[i];
            if slot.armed && slot.at <= self.now {
                self.serve_slot(cfg, i, running_cycles);
            }
            i = i + 1;
        }
    }

    fn serve_slot(&mut self, cfg: &Config<NT, NR, NP>, i: usize, running_cycles: u32)
        requires old(self).inv(cfg), i < NS, old(self).slots.slots[i as int].armed,
        ensures final(self).inv(cfg),
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
        ensures final(self).inv(cfg),
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
            } else {
                // R9.8: a Release is already pending; discard this one.
                self.mit_violation(cfg, t, partition);
            }
        }
        match self.timing.advance_periodic(cfg, t) {
            Some(next) => { self.slots.arm(i, next, SLOT_RELEASE, t); }
            None => { self.slots.disarm(i); }
        }
    }

    /// R10.2: a deferred Release becomes due.
    fn deferred_release(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg),
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
    }

    /// R9.6: the absolute deadline of a Release passed while its Job had
    /// not completed (the slot is disarmed on completion).
    fn deadline_miss(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg),
    {
        self.slots.disarm(i);
        proof {
            assert(cfg.task_ok(t as int));
        }
        let partition = cfg.tasks[t as usize].partition;
        self.push_event(cfg, EV_DEADLINE_MISS, t, partition, PRS_BUDGET, 0);
        let response = cfg.partitions[partition as usize].deadline_response;
        self.apply_response(cfg, partition, response, EV_DEADLINE_MISS, t);
    }

    /// R18.2, R18.3: the Budget compare of the executing Job fired. The
    /// cycles consumed since it last resumed are charged; an Overrun
    /// triggers the Partition's Overrun_Response, otherwise the compare is
    /// re-armed for the remainder.
    fn budget_event(&mut self, cfg: &Config<NT, NR, NP>, t: u8, i: usize, running_cycles: u32)
        requires old(self).inv(cfg), (t as int) < NT, i < NS,
        ensures final(self).inv(cfg),
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
            let response = cfg.partitions[partition as usize].overrun_response;
            self.apply_response(cfg, partition, response, EV_OVERRUN, t);
        } else {
            self.arm_budget(cfg, t);
        }
    }

    // ------------------------------------------------------- Health_Monitor

    /// A fault attributed to the executing Job's Partition (R14.3), or to
    /// the Kernel when `kernel` is set (R14.4).
    pub fn fault(&mut self, cfg: &Config<NT, NR, NP>, kind: u8, kernel: bool)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg),
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
        let response = cfg.partitions[partition as usize].fault_response;
        self.apply_response(cfg, partition, response, kind, t);
    }

    /// DD-10, R14.5: applies one response of the closed sets.
    fn apply_response(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, response: u8, cause: u8, t: u8)
        requires old(self).inv(cfg), (partition as int) < NP, (t as int) < NT,
        ensures final(self).inv(cfg),
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
                        self.end_top(cfg);
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
    fn end_top(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg), old(self).sched.depth > 0,
        ensures final(self).inv(cfg), final(self).sched.depth == old(self).sched.depth - 1,
    {
        let t = self.sched.stack[self.sched.depth - 1];
        proof {
            assert(cfg.slot_ok(t as int));
        }
        let released = self.sched.end_job(cfg);
        if released > 0 {
            self.push_event(cfg, EV_RESOURCE_RELEASED, t, cfg.tasks[t as usize].partition, 1u64 << 7, released as u32);
        }
        self.slots.disarm(cfg.slot_budget[t as usize] as usize);
        self.slots.disarm(cfg.slot_deadline[t as usize] as usize);
        if self.sched.depth > 0 {
            let resumed = self.sched.stack[self.sched.depth - 1];
            self.arm_budget(cfg, resumed);
        }
    }

    /// Ends every Job of `partition`: the executing one at once, preempted
    /// ones when Kernel_Arch unwinds to them (`kill`); suppresses its
    /// Releases and disarms their deadline and deferral events. Periodic
    /// release events stay armed at the next nominal instants (R14.5); MIT
    /// history is kept. Other Partitions are untouched (R14.9).
    fn end_partition_jobs(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg),
    {
        // The executing Job first, if it belongs to the Partition.
        if self.sched.depth > 0 {
            let top = self.sched.stack[self.sched.depth - 1];
            if cfg.tasks[top as usize].partition == partition {
                self.end_top(cfg);
            }
        }
        // Preempted Jobs of the Partition are marked killed.
        let mut i: usize = 0;
        while i < self.sched.depth
            invariant self.inv(cfg), i <= self.sched.depth,
            decreases self.sched.depth - i,
        {
            let u = self.sched.stack[i];
            if cfg.tasks[u as usize].partition == partition && self.sched.job[u as usize] != JOB_NONE {
                self.sched.kill(cfg, i);
            }
            i = i + 1;
        }
        // Releases and their events.
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), t <= NT, (cfg.slot_count as int) == NS,
            decreases NT - t,
        {
            if cfg.tasks[t].partition == partition {
                proof {
                    assert(cfg.slot_ok(t as int));
                }
                self.sched.clear_release(cfg, t as u8);
                self.slots.disarm(cfg.slot_deadline[t] as usize);
                if cfg.tasks[t].kind == KIND_SPORADIC && cfg.tasks[t].mit_policy == MIT_DEFER {
                    self.slots.disarm(cfg.slot_release[t] as usize);
                }
                self.budget.reset(t as u8);
            }
            t = t + 1;
        }
        self.stop_dma[partition as usize] = true;
    }

    fn restart_partition(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, cause: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg),
    {
        self.push_event(cfg, EV_PARTITION_RESTART, 0xFF, partition, 0, cause as u32);
        self.end_partition_jobs(cfg, partition);
        self.mit_violations[partition as usize] = 0;
        self.reinit_partition[partition as usize] = true;
    }

    fn stop_partition(&mut self, cfg: &Config<NT, NR, NP>, partition: u8, cause: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), final(self).stopped[partition as int],
    {
        self.push_event(cfg, EV_PARTITION_STOP, 0xFF, partition, 0, cause as u32);
        self.end_partition_jobs(cfg, partition);
        self.stopped[partition as usize] = true;
    }

    /// R9.9, R12.2, R14.4: the system safe state. No Release becomes
    /// effective afterwards; Kernel_Arch applies the declared outputs.
    pub fn enter_safe_state(&mut self, cfg: &Config<NT, NR, NP>)
        requires old(self).inv(cfg),
        ensures final(self).inv(cfg), final(self).safe_state,
    {
        if !self.safe_state {
            self.push_event(cfg, EV_SAFE_STATE, 0xFF, 0xFF, 0, 0);
        }
        self.safe_state = true;
        let mut t: usize = 0;
        while t < NT
            invariant self.inv(cfg), t <= NT, self.safe_state,
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
        ensures final(self).inv(cfg), final(self).safe_state,
    {
        self.push_event(cfg, EV_BOOT_CHECK, 0xFF, 0xFF, 0, failed);
        self.enter_safe_state(cfg);
    }

    /// Kernel_Arch acknowledges a Partition re-initialization.
    pub fn ack_reinit(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), !final(self).reinit_partition[partition as int],
    {
        self.reinit_partition[partition as usize] = false;
    }

    /// Kernel_Arch acknowledges that a Partition's EasyDMA transfers are stopped.
    pub fn ack_stop_dma(&mut self, cfg: &Config<NT, NR, NP>, partition: u8)
        requires old(self).inv(cfg), (partition as int) < NP,
        ensures final(self).inv(cfg), !final(self).stop_dma[partition as int],
    {
        self.stop_dma[partition as usize] = false;
    }

    /// The selected Job to pend next (Property 1), for Kernel_Arch to pend
    /// its vector after a transition that may have made a Job eligible.
    pub fn select(&self, cfg: &Config<NT, NR, NP>) -> (sel: Option<u8>)
        requires self.inv(cfg),
        ensures sel.is_some() ==> self.sched.eligible(cfg, sel.unwrap() as int),
    {
        if self.safe_state || !self.started {
            return None;
        }
        self.sched.select(cfg)
    }
}

} // verus!
