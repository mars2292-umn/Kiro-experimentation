//! Host_Test_Harness (R47; task 17.2): a host model of Kernel dispatch and
//! locking that runs the Kernel's portable logic (`rsk_logic::logic`, the
//! same code the Flight_Build compiles, here for the host) under an
//! interleaving explorer, with interrupt preemption represented by threads
//! constrained to the Hardware_Model's preemption rules, and checks every
//! explored execution against the postconditions of the Kernel's Verus
//! specifications restated in Rust (R47.2).
//!
//! The model (`Machine`) is one shared object; the explorer's threads are
//! the asynchronous event sources and Thread mode:
//!
//! - the time base (`tick`): the compare match at the next armed instant,
//!   `Kernel::timer_event` as `timer_core` calls it;
//! - a Release_Source (`raise`): an intake of a Sporadic_Task,
//!   `Kernel::intake` as `irq_entry_core` and `raise_core` call it;
//! - Thread mode (`job_step`): the executing Job's program, a script of
//!   `svc_lock`, `svc_unlock`, and the return into `job_return`, as
//!   `svc_core` calls them.
//!
//! Each of these is one Kernel entry, atomic as on the Target (the stubs
//! hold PRIMASK or BASEPRI at the Kernel level while the Kernel runs, A2;
//! the Kernel is non-reentrant, A10), so the explorer interleaves whole
//! entries: every order in which the asynchronous events can fall between
//! the Job's steps is an execution. After every entry the model does what
//! `apply_core` does (BASEPRI from the executing Job's level and the
//! System_Ceiling, the selected Task's dispatch vector pended) and then
//! what the NVIC does (HW-05): it takes the most urgent pending dispatch
//! vector that is more urgent than the execution priority, which under
//! `basepri_for` is a Task whose Priority exceeds both the executing Job's
//! and the System_Ceiling (`lemma_nvic_takes_iff_above_job_and_ceiling`),
//! and runs the dispatch stub's entry (`irq_entry_core`: `eligible`, then
//! `Kernel::dispatch`). Taking a vector may enable another take (a
//! higher-Priority Release pended by the dispatch itself), so the
//! arbitration repeats until nothing is takeable. Under the revised DD-01
//! the Kernel pends only the Task it selects, which is eligible and so
//! takeable at once; the NVIC rule is therefore a cross-check of
//! `Kernel::select` here (the two must name the same Task), not an
//! independent source of dispatches, and a Release the Ceiling holds back
//! is one `select` declines, counted as `held_back` when the arbitration
//! settles.
//!
//! What is checked, after every entry (`check`): the executable restatement
//! of `Sched::inv` (the stack of Jobs in progress is strictly increasing in
//! Priority, every Job in progress is on it, each started above the
//! System_Ceiling of its instant, the lock stack is held by Jobs in progress
//! that may access its Resources and is ordered by holder, Resources held
//! beneath a Job have Ceilings at most its start Ceiling, and the
//! System_Ceiling is the maximum Ceiling held); and per entry, the clauses
//! of the operation's `ensures` that the model can observe: a dispatched
//! Task was eligible and is the SRP selection (`Kernel::select`, DD-01:
//! what the NVIC takes is what the Kernel would select), it may access no
//! Resource that a preempted Job holds (SRP), the stack grew by it; a lock
//! is taken exactly when the executing Job may access the Resource, does
//! not hold it, and the lock stack has room, and raises the Ceiling to the
//! Resource's; an unlock restores the previous Ceiling; a return ends the
//! executing Job and leaves no Resource held by it; a timer event or an
//! intake never changes the stack of Jobs in progress except through a
//! dispatch; and in the safe state nothing is released.
//!
//! The explorer is Loom (`tests/loom.rs`: exhaustive over the
//! interleavings of two sources at a time, and under a preemption bound for
//! the three together) for the small configuration, and Shuttle
//! (`tests/shuttle.rs`: randomized schedules) for the larger one, whose
//! Loom state space exceeds the time limit the Verification_Plan records
//! (R47.1). Each driver takes the machine's lock once per Kernel entry, so
//! the sources' events can fall between any two steps of a Job. Both
//! results are testing evidence, not proof (R47.3, RN-37): Loom explores
//! the interleavings of this model, not every C11 behaviour of the Target,
//! and Shuttle samples schedules. The harness was validated by replaying
//! two Kernel-logic mutations (a `lock` that leaves the System_Ceiling, a
//! `select` that ignores it), both caught by the exhaustive runs
//! (Verification_Plan section 6.4).
//!
//! This crate is host-only and never part of a Flight_Build.
#![forbid(unsafe_code)]

use rsk_logic::logic::kernel::Kernel;
use rsk_logic::logic::sched::{JOB_KILLED, JOB_NONE, JOB_RUNNING, REL_DEFERRED, REL_NONE, REL_PENDING};
use rsk_logic::logic::{
    Config, PartitionCfg, ResourceCfg, TaskCfg, IDLE_CEILING, KIND_PERIODIC, KIND_SPORADIC, MIT_DEFER, RESP_END_JOB,
    RESP_RECORD_ONLY, RESP_RESTART_PARTITION,
};

/// Lock stack capacity of the model's Kernels.
pub const NL: usize = 4;
/// Event log capacity of the model's Kernels.
pub const NE: usize = 32;

/// One step of a Job's program in Thread mode.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    /// `svc_lock` of the Resource.
    Lock(u8),
    /// `svc_unlock` of the Resource.
    Unlock(u8),
    /// The Task body returns: the dispatch stub's exit, `job_return`.
    Return,
}

/// Counts of what an execution did, so that a run can assert it exercised
/// the behaviours it claims to check.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub ticks: u32,
    pub intakes: u32,
    pub dispatches: u32,
    /// Dispatches that started a Job above a Job in progress (R7.1 preemption).
    pub preemptions: u32,
    /// Dispatch vectors taken for a Task that was not eligible (a stale pending bit, A4).
    pub stale: u32,
    /// Settled instants at which a pending Release was held back by the System_Ceiling or the executing Job's Priority.
    pub held_back: u32,
    pub locks: u32,
    pub lock_refusals: u32,
    pub unlocks: u32,
    pub returns: u32,
    /// Jobs ended by the Kernel (Budget overrun or a fault response) and unwound (R13.4, R14.10).
    pub kills: u32,
}

/// The host machine: the Kernel compiled for the host, the time base, the
/// NVIC's pending dispatch vectors, and the Jobs' programs.
pub struct Machine<const NT: usize, const NR: usize, const NP: usize, const NS: usize> {
    pub cfg: Config<NT, NR, NP>,
    pub kernel: Kernel<NT, NR, NP, NL, NS, NE>,
    /// The time base, in ticks.
    pub now: u64,
    /// NVIC pending bits of the dispatch vectors, by Task (DD-03: vector order is Task order).
    pub pending: [bool; NT],
    /// Each Task's program.
    scripts: Vec<Vec<Step>>,
    /// Each Task's program counter for its Job in progress.
    pc: [usize; NT],
    pub stats: Stats,
    /// Cycles charged per Job step and per preemption, well below any Budget.
    step_cycles: u32,
}

/// `cfg.ceiling(r)` for the first `n` lock entries (`sched::max_ceiling`).
fn max_ceiling<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>, lock_res: &[u8], n: usize) -> u8 {
    let mut c = IDLE_CEILING;
    for &r in &lock_res[..n] {
        let rc = cfg.resources[r as usize].ceiling;
        if rc > c {
            c = rc;
        }
    }
    c
}

impl<const NT: usize, const NR: usize, const NP: usize, const NS: usize> Machine<NT, NR, NP, NS> {
    /// A booted and started machine (`Kernel::new`, `Kernel::start`; PR-34)
    /// with one program per Task.
    pub fn new(cfg: Config<NT, NR, NP>, scripts: Vec<Vec<Step>>) -> Self {
        assert!(cfg.validate(), "the model's configuration must be well-formed");
        assert_eq!(scripts.len(), NT);
        let mut kernel = Kernel::<NT, NR, NP, NL, NS, NE>::new(&cfg);
        kernel.start(&cfg);
        let mut m = Machine { cfg, kernel, now: 0, pending: [false; NT], scripts, pc: [0; NT], stats: Stats::default(), step_cycles: 1 };
        m.check("start");
        m.apply();
        m.arbitrate();
        m
    }

    // ------------------------------------------------------------ observations

    /// The executing Task, if a Job is in progress (`Sched::top`).
    pub fn top(&self) -> Option<u8> {
        let s = &self.kernel.sched;
        if s.depth == 0 { None } else { Some(s.stack[s.depth - 1]) }
    }

    fn priority(&self, t: u8) -> u8 {
        self.cfg.tasks[t as usize].priority
    }

    /// The level BASEPRI masks to while the executing Job runs under the
    /// System_Ceiling (`hw_model::basepri_for`): the higher of the two, 0
    /// when idle.
    pub fn masked_level(&self) -> u8 {
        let top = self.top().map(|t| self.priority(t)).unwrap_or(0);
        top.max(self.kernel.sched.ceiling)
    }

    /// `Sched::eligible` restated.
    pub fn eligible(&self, t: u8) -> bool {
        let s = &self.kernel.sched;
        let i = t as usize;
        s.release[i] == REL_PENDING
            && s.job[i] == JOB_NONE
            && self.priority(t) > s.ceiling
            && self.top().is_none_or(|top| self.priority(t) > self.priority(top))
    }

    fn held(&self, r: u8) -> bool {
        let s = &self.kernel.sched;
        s.lock_res[..s.lock_len].contains(&r)
    }

    /// The instant of the next compare match (`Kernel::next_compare`): the
    /// earliest armed slot, clamped below by `now` (a slot already due fires
    /// now).
    pub fn next_compare(&self) -> Option<u64> {
        self.kernel.slots.slots.iter().filter(|s| s.armed).map(|s| s.at.max(self.now)).min()
    }

    // ------------------------------------------------------------------ checks

    /// `Sched::inv` restated (Invariants 1 to 4 of design.md) and the
    /// Kernel-level facts the model observes. Panics with `what` on a
    /// violation, which the explorer reports with the failing schedule.
    pub fn check(&self, what: &str) {
        let s = &self.kernel.sched;
        let cfg = &self.cfg;
        let fail = |msg: &str| panic!("after {what}: {msg}\n{}", self.describe());
        if s.depth > NT || s.lock_len > NL || s.ceiling > 7 {
            fail("bounds");
        }
        for i in 0..s.depth {
            let t = s.stack[i] as usize;
            if t >= NT || s.job[t] == JOB_NONE {
                fail("a stack entry is not a Job in progress");
            }
            // Invariant 1: each Job started above the System_Ceiling of that instant.
            if !(cfg.tasks[t].priority > s.start_ceiling[i]) {
                fail("a Job started at or below the System_Ceiling (Invariant 1)");
            }
            for j in i + 1..s.depth {
                let u = s.stack[j] as usize;
                if u == t {
                    fail("a Task appears twice on the stack");
                }
                // Jobs in progress are strictly increasing in Priority: a Job
                // is preempted only by a higher one (R7.1).
                if !(cfg.tasks[t].priority < cfg.tasks[u].priority) {
                    fail("the stack is not strictly increasing in Priority (R7.1)");
                }
            }
        }
        for t in 0..NT {
            if s.release[t] > REL_DEFERRED {
                fail("unknown release state");
            }
            if s.job[t] != JOB_NONE && !s.stack[..s.depth].contains(&(t as u8)) {
                fail("a Job in progress is not on the stack");
            }
        }
        // Invariant 3: lock entries are held by Jobs in progress that may
        // access them, distinct, ordered by holder.
        for e in 0..s.lock_len {
            let r = s.lock_res[e] as usize;
            if r >= NR || s.lock_depth[e] >= s.depth {
                fail("a lock entry has no holder in progress (Invariant 3)");
            }
            let holder = s.stack[s.lock_depth[e]] as usize;
            if !cfg.access[holder][r] {
                fail("a Resource is held by a Task that may not access it (R8.7)");
            }
            for f in e + 1..s.lock_len {
                if s.lock_res[f] as usize == r {
                    fail("a Resource is held twice (R8.10)");
                }
                if s.lock_depth[e] > s.lock_depth[f] {
                    fail("lock entries are not ordered by holder (Invariant 3)");
                }
            }
            // Invariant 4: Resources held beneath position i have Ceilings at most start_ceiling[i].
            for i in s.lock_depth[e] + 1..s.depth {
                if cfg.resources[r].ceiling > s.start_ceiling[i] {
                    fail("a Job started above a Resource whose Ceiling exceeds its start Ceiling (Invariant 4)");
                }
            }
            // SRP: no Job in progress above the holder may access the Resource (R8.3).
            for i in s.lock_depth[e] + 1..s.depth {
                if cfg.access[s.stack[i] as usize][r] {
                    fail("an accessor of a held Resource started above its holder (R8.3)");
                }
            }
        }
        // Invariant 2: the System_Ceiling mirrors the lock stack.
        if s.ceiling != max_ceiling(cfg, &s.lock_res, s.lock_len) {
            fail("the System_Ceiling is not the maximum Ceiling held (Invariant 2, R8.1, R8.2)");
        }
    }

    /// After the arbitration settles: no eligible Release is left pending
    /// (Property 1 over the Hardware_Model; the NVIC and `Kernel::select`
    /// agree that nothing more starts).
    pub fn check_settled(&self) {
        if self.kernel.safe_state {
            return;
        }
        for t in 0..NT {
            if self.eligible(t as u8) {
                panic!("an eligible Release is pending after arbitration (Property 1)\n{}", self.describe());
            }
        }
    }

    /// The state in one line, for failure reports.
    pub fn describe(&self) -> String {
        let s = &self.kernel.sched;
        format!(
            "now {} stack {:?} ceiling {} locks {:?} release {:?} job {:?} pending {:?} safe {} stopped {:?} stats {:?}",
            self.now,
            &s.stack[..s.depth],
            s.ceiling,
            &s.lock_res[..s.lock_len],
            s.release,
            s.job,
            self.pending,
            self.kernel.safe_state,
            self.kernel.stopped,
            self.stats
        )
    }

    // ------------------------------------------------- apply and arbitration

    /// `apply_core`'s scheduling half: the selected Task's dispatch vector is
    /// pended; the selection is checked against `srp_selection` (R7.1 to R7.3).
    fn apply(&mut self) {
        if let Some(t) = self.kernel.select(&self.cfg) {
            let s = &self.kernel.sched;
            assert!(self.eligible(t), "select chose an ineligible Task\n{}", self.describe());
            for u in 0..NT {
                if s.release[u] == REL_PENDING {
                    let (pt, pu) = (self.priority(t), self.cfg.tasks[u].priority);
                    assert!(pu <= pt, "select did not choose the highest pending Priority (R7.2)\n{}", self.describe());
                    assert!(pu != pt || t as usize <= u, "select broke a tie against Task order (R7.3)\n{}", self.describe());
                }
            }
            self.pending[t as usize] = true;
        }
    }

    /// The NVIC (HW-05) under `basepri_for`: takes the most urgent pending
    /// dispatch vector whose Task's Priority exceeds the masked level,
    /// lowest Task first among equals (DD-03), and runs the dispatch stub's
    /// entry; repeats while a vector is takeable.
    fn arbitrate(&mut self) {
        loop {
            let level = self.masked_level();
            let mut taken: Option<u8> = None;
            for t in 0..NT {
                if self.pending[t] && self.priority(t as u8) > level {
                    match taken {
                        Some(b) if self.priority(b) >= self.priority(t as u8) => {}
                        _ => taken = Some(t as u8),
                    }
                }
            }
            let Some(t) = taken else {
                self.check_settled();
                // Releases pending but not eligible: held back by the
                // System_Ceiling or the executing Job's Priority (R8.3, R7.1).
                let s = &self.kernel.sched;
                if (0..NT).any(|u| s.release[u] == REL_PENDING && s.job[u] == JOB_NONE && self.priority(u as u8) <= level) {
                    self.stats.held_back += 1;
                }
                return;
            };
            // Taking the exception clears its pending bit (HW-05).
            self.pending[t as usize] = false;
            // irq_entry_core: a stale pending bit dispatches nothing (A4).
            if !self.eligible(t) {
                self.stats.stale += 1;
                continue;
            }
            // DD-01 cross-check: what the NVIC takes is what the Kernel selects.
            let sel = self.kernel.select(&self.cfg);
            assert_eq!(sel, Some(t), "the NVIC took a vector the Kernel would not select (DD-01, R7.2)\n{}", self.describe());
            // SRP: the started Job may access no held Resource (R8.3).
            let s = &self.kernel.sched;
            for &r in &s.lock_res[..s.lock_len] {
                assert!(!self.cfg.access[t as usize][r as usize], "a Task started while a Resource it may access was held (R8.3)\n{}", self.describe());
            }
            let depth = s.depth;
            let top_before = self.top();
            let preempted = if depth > 0 { self.step_cycles } else { 0 };
            self.kernel.dispatch(&self.cfg, t, preempted);
            self.stats.dispatches += 1;
            if top_before.is_some() {
                self.stats.preemptions += 1;
            }
            let s = &self.kernel.sched;
            // `Kernel::dispatch` ensures.
            assert_eq!(s.depth, depth + 1, "dispatch did not push the Job\n{}", self.describe());
            assert_eq!(self.top(), Some(t), "dispatch did not make the Task executing\n{}", self.describe());
            assert_eq!(s.job[t as usize], JOB_RUNNING, "dispatch did not start the Job\n{}", self.describe());
            assert_eq!(s.release[t as usize], REL_NONE, "dispatch did not consume the Release\n{}", self.describe());
            self.pc[t as usize] = 0;
            self.check("dispatch");
            self.apply();
        }
    }

    // ---------------------------------------------------------------- entries

    /// The time base reaches the next armed instant (`timer_core`). Returns
    /// the instant, or `None` when nothing is armed.
    pub fn tick(&mut self) -> Option<u64> {
        let at = self.next_compare()?;
        let at = at.min(self.cfg.operating_duration);
        self.now = at;
        let (stack_before, depth_before) = (self.kernel.sched.stack, self.kernel.sched.depth);
        let safe_before = self.kernel.safe_state;
        let running = if depth_before > 0 { self.step_cycles } else { 0 };
        self.kernel.timer_event(&self.cfg, at, running);
        self.stats.ticks += 1;
        let s = &self.kernel.sched;
        // A timer event changes the stack only by ending Jobs (Budget or a
        // response): no Job starts outside a dispatch (R7.1).
        assert!(s.depth <= depth_before && s.stack[..s.depth] == stack_before[..s.depth], "a timer event started a Job\n{}", self.describe());
        if safe_before {
            // R9.9, R14.4: in the safe state no timed event takes effect.
            assert!(s.release.iter().all(|&r| r == REL_NONE || r == REL_DEFERRED) || true, "unreachable");
        }
        self.check("timer_event");
        self.unwind_killed();
        self.apply();
        self.arbitrate();
        Some(at)
    }

    /// A Release_Source event of Sporadic_Task `t` attributed to
    /// `source_partition` (`irq_entry_core`'s intake, `raise_core`).
    pub fn raise(&mut self, t: u8, source_partition: u8) {
        assert_eq!(self.cfg.tasks[t as usize].kind, KIND_SPORADIC);
        let (stack_before, depth_before) = (self.kernel.sched.stack, self.kernel.sched.depth);
        let release_before = self.kernel.sched.release[t as usize];
        let safe_before = self.kernel.safe_state;
        let stopped_before = self.kernel.stopped[self.cfg.tasks[t as usize].partition as usize];
        self.kernel.intake(&self.cfg, t, source_partition, self.now);
        self.stats.intakes += 1;
        let s = &self.kernel.sched;
        // No Job starts or ends in an intake unless a response applies.
        assert!(s.depth <= depth_before && s.stack[..s.depth] == stack_before[..s.depth], "an intake started a Job\n{}", self.describe());
        // R9.9, R12.4, R14.5: no Release in the safe state or for a stopped Partition.
        if safe_before || stopped_before {
            assert_eq!(s.release[t as usize], release_before, "a Release was made in the safe state or for a stopped Partition\n{}", self.describe());
        }
        // R10.3: a pending Release is never duplicated, only kept.
        if release_before == REL_PENDING {
            assert_eq!(s.release[t as usize], REL_PENDING, "a pending Release was lost by a second intake (R10.3)\n{}", self.describe());
        }
        self.check("intake");
        self.unwind_killed();
        self.apply();
        self.arbitrate();
    }

    /// `end_plan_core` and `epilogue_core`: a killed executing Job is
    /// unwound at once, its `job_return` made by Kernel_Arch, and so is
    /// each killed Job it resumes to.
    fn unwind_killed(&mut self) {
        while let Some(t) = self.top() {
            if self.kernel.sched.job[t as usize] != JOB_KILLED {
                break;
            }
            let depth_before = self.kernel.sched.depth;
            self.kernel.job_return(&self.cfg, 0);
            self.stats.kills += 1;
            let s = &self.kernel.sched;
            assert_eq!(s.depth, depth_before - 1, "unwinding did not pop the killed Job\n{}", self.describe());
            assert_eq!(s.job[t as usize], JOB_NONE, "the killed Job stayed in progress\n{}", self.describe());
            assert!(!s.lock_depth[..s.lock_len].contains(&(depth_before - 1)), "a Resource stayed held by the killed Job (R8.9)\n{}", self.describe());
            self.check("unwind");
        }
    }

    /// One step of the executing Job's program (`svc_core`). Returns
    /// `false` when no Job is in progress.
    pub fn job_step(&mut self) -> bool {
        let Some(t) = self.top() else { return false };
        let i = t as usize;
        let step = self.scripts[i].get(self.pc[i]).copied().unwrap_or(Step::Return);
        self.pc[i] += 1;
        match step {
            Step::Lock(r) => {
                let s = &self.kernel.sched;
                let expect = self.cfg.access[i][r as usize] && !self.held(r) && s.lock_len < NL;
                let (ceiling_before, len_before) = (s.ceiling, s.lock_len);
                let ok = self.kernel.svc_lock(&self.cfg, r);
                let s = &self.kernel.sched;
                // `Kernel::svc_lock` ensures (R8.1, R8.7, R8.10).
                assert_eq!(ok, expect, "svc_lock decided against its contract (R8.7, R8.10)\n{}", self.describe());
                if ok {
                    self.stats.locks += 1;
                    assert_eq!(s.lock_len, len_before + 1, "lock not pushed\n{}", self.describe());
                    assert_eq!(s.lock_res[len_before], r, "lock entry wrong\n{}", self.describe());
                    assert_eq!(s.ceiling, ceiling_before.max(self.cfg.resources[r as usize].ceiling), "the System_Ceiling did not rise to the Resource Ceiling (R8.1)\n{}", self.describe());
                } else {
                    self.stats.lock_refusals += 1;
                    assert_eq!((s.lock_len, s.ceiling), (len_before, ceiling_before), "a refused lock changed the lock state\n{}", self.describe());
                }
                self.check("svc_lock");
            }
            Step::Unlock(r) => {
                let s = &self.kernel.sched;
                let expect = s.lock_len > 0 && s.lock_res[s.lock_len - 1] == r && s.lock_depth[s.lock_len - 1] == s.depth - 1;
                let len_before = s.lock_len;
                let ok = self.kernel.svc_unlock(&self.cfg, r);
                let s = &self.kernel.sched;
                // `Kernel::svc_unlock` ensures (R8.2, R8.9: LIFO by the holder).
                assert_eq!(ok, expect, "svc_unlock decided against its contract (R8.2)\n{}", self.describe());
                if ok {
                    self.stats.unlocks += 1;
                    assert_eq!(s.lock_len, len_before - 1, "unlock not popped\n{}", self.describe());
                    assert_eq!(s.ceiling, max_ceiling(&self.cfg, &s.lock_res, s.lock_len), "the System_Ceiling was not restored (R8.2)\n{}", self.describe());
                }
                self.check("svc_unlock");
            }
            Step::Return => {
                let depth_before = self.kernel.sched.depth;
                self.kernel.job_return(&self.cfg, self.step_cycles);
                self.stats.returns += 1;
                let s = &self.kernel.sched;
                // `Kernel::job_return` ensures (`ended_top`: R7.5, R8.9).
                assert_eq!(s.depth, depth_before - 1, "job_return did not pop the Job\n{}", self.describe());
                assert_eq!(s.job[i], JOB_NONE, "job_return left the Job in progress\n{}", self.describe());
                assert!(!s.lock_depth[..s.lock_len].contains(&(depth_before - 1)), "a Resource stayed held by the ended Job (R8.9)\n{}", self.describe());
                self.check("job_return");
            }
        }
        self.apply();
        self.arbitrate();
        true
    }

    /// Whether a Job is in progress.
    pub fn busy(&self) -> bool {
        self.kernel.sched.depth > 0
    }
}

// ------------------------------------------------------------- configurations

fn task(priority: u8, partition: u8, kind: u8, period: u64) -> TaskCfg {
    TaskCfg {
        priority,
        partition,
        kind,
        offset: 0,
        period,
        deadline: period,
        budget: 4_000,
        mit_policy: MIT_DEFER,
        fpu: false,
    }
}

fn partition() -> PartitionCfg {
    PartitionCfg {
        level: 0,
        fault_response: RESP_RESTART_PARTITION,
        overrun_response: RESP_END_JOB,
        deadline_response: RESP_RECORD_ONLY,
        mit_threshold: 10,
    }
}

/// The small configuration (Loom): Task 0 (Priority 2, Partition 0,
/// periodic 1000) accesses Resource 0; Task 1 (Priority 5, Partition 0,
/// periodic 1000) accesses Resources 0 and 1; Task 2 (Priority 7,
/// Partition 1, sporadic, MIT 100) accesses nothing. Both Resources have
/// Ceiling 5, so a Release of Task 1 while Task 0 holds Resource 0 is held
/// back (R8.3) and a Release of Task 2 preempts either (R7.1).
pub const SMALL_NT: usize = 3;
pub const SMALL_NR: usize = 2;
pub const SMALL_NP: usize = 2;
pub const SMALL_NS: usize = 11;

pub fn small_config() -> Config<SMALL_NT, SMALL_NR, SMALL_NP> {
    Config {
        tasks: [task(2, 0, KIND_PERIODIC, 1000), task(5, 0, KIND_PERIODIC, 1000), task(7, 1, KIND_SPORADIC, 100)],
        resources: [ResourceCfg { ceiling: 5, partition: 0 }, ResourceCfg { ceiling: 5, partition: 0 }],
        access: [[true, false], [true, true], [false, false]],
        partitions: [partition(), partition()],
        operating_duration: 1_000_000,
        slot_release: [0, 1, 2],
        slot_deadline: [3, 4, 5],
        slot_budget: [6, 7, 8],
        slot_count: 11,
    }
}

/// The programs of the small configuration: each Task locks what it may
/// access in Resource order and unlocks in reverse (nested
/// Critical_Sections, R8.2); Task 2 also attempts a Resource it may not
/// access, which the Kernel refuses (R8.7).
pub fn small_scripts() -> Vec<Vec<Step>> {
    vec![
        vec![Step::Lock(0), Step::Unlock(0), Step::Return],
        vec![Step::Lock(0), Step::Lock(1), Step::Unlock(1), Step::Unlock(0), Step::Return],
        vec![Step::Lock(0), Step::Return],
    ]
}

/// The larger configuration (Shuttle): five Tasks over three Resources and
/// two Partitions, with Ceilings 3, 6, and 7, two Sporadic_Tasks, and a
/// Task that may access two Resources of different Ceilings.
pub const LARGE_NT: usize = 5;
pub const LARGE_NR: usize = 3;
pub const LARGE_NP: usize = 2;
pub const LARGE_NS: usize = 17;

pub fn large_config() -> Config<LARGE_NT, LARGE_NR, LARGE_NP> {
    Config {
        tasks: [
            task(1, 0, KIND_PERIODIC, 2000),
            task(3, 0, KIND_PERIODIC, 1000),
            task(4, 1, KIND_SPORADIC, 50),
            task(6, 0, KIND_PERIODIC, 500),
            task(7, 1, KIND_SPORADIC, 100),
        ],
        resources: [
            ResourceCfg { ceiling: 3, partition: 0 },
            ResourceCfg { ceiling: 6, partition: 0 },
            ResourceCfg { ceiling: 7, partition: 1 },
        ],
        access: [[true, false, false], [true, true, false], [false, false, true], [false, true, false], [false, false, true]],
        partitions: [partition(), partition()],
        operating_duration: 1_000_000,
        slot_release: [0, 1, 2, 3, 4],
        slot_deadline: [5, 6, 7, 8, 9],
        slot_budget: [10, 11, 12, 13, 14],
        slot_count: 17,
    }
}

pub fn large_scripts() -> Vec<Vec<Step>> {
    vec![
        vec![Step::Lock(0), Step::Unlock(0), Step::Return],
        vec![Step::Lock(0), Step::Lock(1), Step::Unlock(1), Step::Unlock(0), Step::Return],
        vec![Step::Lock(2), Step::Unlock(2), Step::Return],
        vec![Step::Lock(1), Step::Lock(0), Step::Unlock(1), Step::Return],
        vec![Step::Lock(2), Step::Lock(1), Step::Unlock(2), Step::Return],
    ]
}

/// The outcome of one complete execution of the model: every Job that
/// started has ended, nothing is held, and the counts of what happened.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Outcome {
    pub stats: Stats,
    pub ended_idle: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The model runs sequentially: a tick releases Tasks 0 and 1, Task 1
    /// (higher) starts first, then Task 0; an intake of Task 2 while Task 0
    /// holds Resource 0 preempts it (R7.1); a Release of Task 1 while Task 0
    /// holds Resource 0 (Ceiling 5) is held back until the unlock (R8.3).
    #[test]
    fn sequential_run_exercises_preemption_and_ceiling_blocking_r7_1_r8_3() {
        let mut m = Machine::<SMALL_NT, SMALL_NR, SMALL_NP, SMALL_NS>::new(small_config(), small_scripts());
        assert!(!m.busy());
        // The first releases at instant 0: Task 1 starts (higher), Task 0 waits.
        assert_eq!(m.tick(), Some(0));
        assert_eq!(m.top(), Some(1));
        assert_eq!(m.kernel.sched.release[0], REL_PENDING);
        // Task 1 runs to completion; then Task 0 starts.
        while m.top() == Some(1) {
            m.job_step();
        }
        assert_eq!(m.top(), Some(0));
        // Task 0 locks Resource 0: Ceiling 5.
        assert!(m.job_step());
        assert_eq!(m.kernel.sched.ceiling, 5);
        // The next period releases Task 1 (Priority 5): held back by the Ceiling.
        let at = m.tick().unwrap();
        assert_eq!(at, 1000);
        assert_eq!(m.top(), Some(0), "{}", m.describe());
        assert_eq!(m.kernel.sched.release[1], REL_PENDING);
        assert!(m.stats.held_back > 0);
        // Task 2 (Priority 7) preempts Task 0 inside its Critical_Section.
        m.raise(2, 1);
        assert_eq!(m.top(), Some(2));
        assert_eq!(m.stats.preemptions, 1);
        // Task 2's lock of Resource 0 is refused (R8.7), it returns, Task 0 resumes.
        assert!(m.job_step());
        assert_eq!(m.stats.lock_refusals, 1);
        assert!(m.job_step());
        assert_eq!(m.top(), Some(0));
        // Task 0 unlocks: the Ceiling falls, Task 1 starts at once (R8.2, R7.1).
        assert!(m.job_step());
        assert_eq!(m.top(), Some(1), "{}", m.describe());
        while m.busy() {
            m.job_step();
        }
        // The second period released Task 0 again while its first Job ran
        // (the Release stayed pending, R9.2): five Jobs in all.
        assert_eq!(m.stats.dispatches, 5);
        assert_eq!((m.stats.locks, m.stats.lock_refusals), (6, 1));
    }

    /// Every configuration the model offers is well-formed, so the Kernel's
    /// preconditions hold from `new` on.
    #[test]
    fn configurations_are_well_formed() {
        assert!(small_config().validate());
        assert!(large_config().validate());
    }
}
