//! The 64-bit time base, the timed-event slot table, drift-free periodic
//! release, and MIT enforcement (Properties 7, 8, 9, 11; R9, R10, R19.1).
//!
//! Time. Instants are `u64` ticks of the Kernel time base (DD-05: a TIMER
//! at 16 MHz extended to 64 bits by Kernel_Arch). Every instant the Kernel
//! computes lies at or before the declared operating duration (R9.9), which
//! `Config::wf` bounds below 2^62, so sums of two instants and of an
//! instant and a period never overflow (R9.5: distinct instants have
//! distinct, ordered representations across every hardware counter wrap).
//!
//! Slots. Timed events live in a static table with one slot per declared
//! event (R9.4, PR-14). Property 11: the table refines a map from armed
//! slot to instant, and `next` returns a slot of minimum instant.
//!
//! Periodic release (Property 7). For each Periodic_Task the state holds
//! `k` and `nominal == O + k * T`, computed only from the offset and the
//! period (R9.3), never from the completion or effective release of earlier
//! Jobs, Overruns, or Health_Monitor responses.
//!
//! Sporadic release (Properties 8 and 9). `intake_decision` implements
//! R10.1 to R10.3 and `accept` records the due instant; consecutive
//! accepted Releases are at least one MIT apart. DD-06: an interrupt source
//! is masked from acceptance until both its MIT window and its Job have
//! ended, so the source costs at most one intake per MIT window (R19.1).
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::{Config, KIND_PERIODIC, KIND_SPORADIC, MIT_DEFER};

verus! {

/// Instants and durations stay below this bound (R9.5; `Config::wf`).
pub const INSTANT_BOUND: u64 = 0x4000_0000_0000_0000;

/// Slot kinds (R9.4).
pub const SLOT_FREE: u8 = 0;
pub const SLOT_RELEASE: u8 = 1;
pub const SLOT_DEADLINE: u8 = 2;
pub const SLOT_DEFER: u8 = 3;
pub const SLOT_BUDGET: u8 = 4;
pub const SLOT_HOUSEKEEPING: u8 = 5;
pub const SLOT_END: u8 = 6;

/// Intake decisions (R10.1 to R10.3).
pub const INTAKE_ACCEPT: u8 = 0;
pub const INTAKE_DEFER: u8 = 1;
pub const INTAKE_DISCARD: u8 = 2;

#[derive(Clone, Copy)]
pub struct Slot {
    pub armed: bool,
    pub at: u64,
    pub kind: u8,
    pub task: u8,
}

/// The timed-event table of `NS` slots.
pub struct Slots<const NS: usize> {
    pub slots: [Slot; NS],
}

impl<const NS: usize> Slots<NS> {
    /// Property 11: the table as a map from armed slot to its instant.
    pub open spec fn view(&self) -> Map<int, u64> {
        Map::new(
            vstd::set_lib::set_int_range(0, NS as int).filter(|i: int| self.slots[i].armed),
            |i: int| self.slots[i].at,
        )
    }

    pub open spec fn armed(&self, i: int) -> bool {
        self.slots[i].armed
    }

    pub fn new() -> (s: Self)
        ensures forall|i: int| 0 <= i < NS ==> !s.armed(i), s.view() =~= Map::<int, u64>::empty(),
    {
        let s = Slots { slots: [Slot { armed: false, at: 0, kind: SLOT_FREE, task: 0 }; NS] };
        proof {
            assert(s.view() =~= Map::<int, u64>::empty());
        }
        s
    }

    /// Arms slot `i` for instant `at`.
    pub fn arm(&mut self, i: usize, at: u64, kind: u8, task: u8)
        requires i < NS,
        ensures final(self).slots[i as int] == (Slot { armed: true, at, kind, task }),
            forall|j: int| 0 <= j < NS && j != i ==> final(self).slots[j] == old(self).slots[j],
            final(self).view() =~= old(self).view().insert(i as int, at),
    {
        self.slots[i] = Slot { armed: true, at, kind, task };
        proof {
            assert(self.view() =~= old(self).view().insert(i as int, at));
        }
    }

    /// Disarms slot `i`.
    pub fn disarm(&mut self, i: usize)
        requires i < NS,
        ensures !final(self).armed(i as int),
            forall|j: int| 0 <= j < NS && j != i ==> final(self).slots[j] == old(self).slots[j],
            final(self).view() =~= old(self).view().remove(i as int),
    {
        self.slots[i] = Slot { armed: false, at: 0, kind: SLOT_FREE, task: 0 };
        proof {
            assert(self.view() =~= old(self).view().remove(i as int));
        }
    }

    /// Property 11: the armed slot of minimum instant, ties to the lowest
    /// index, or `None` when nothing is armed.
    pub fn next(&self) -> (n: Option<usize>)
        ensures
            n.is_some() ==> n.unwrap() < NS && self.armed(n.unwrap() as int)
                && forall|j: int| 0 <= j < NS && self.armed(j) ==> self.slots[j].at >= self.slots[n.unwrap() as int].at
                && forall|j: int| 0 <= j < NS && self.armed(j) && self.slots[j].at == self.slots[n.unwrap() as int].at ==> n.unwrap() as int <= j,
            n.is_none() ==> forall|j: int| 0 <= j < NS ==> !self.armed(j),
    {
        let mut best: Option<usize> = None;
        let mut i: usize = 0;
        while i < NS
            invariant i <= NS,
                best.is_some() ==> best.unwrap() < i && self.armed(best.unwrap() as int),
                best.is_some() ==> forall|j: int| 0 <= j < i && self.armed(j) ==> self.slots[j].at >= self.slots[best.unwrap() as int].at,
                best.is_some() ==> forall|j: int| 0 <= j < i && self.armed(j) && self.slots[j].at == self.slots[best.unwrap() as int].at ==> best.unwrap() as int <= j,
                best.is_none() ==> forall|j: int| 0 <= j < i ==> !self.armed(j),
            decreases NS - i,
        {
            if self.slots[i].armed {
                match best {
                    None => { best = Some(i); }
                    Some(b) => {
                        if self.slots[i].at < self.slots[b].at {
                            best = Some(i);
                        }
                    }
                }
            }
            i = i + 1;
        }
        best
    }
}

/// Per-Task timing state.
pub struct Timing<const NT: usize> {
    /// Periodic: index of the next Release and its nominal instant O + k*T.
    pub k: [u64; NT],
    pub nominal: [u64; NT],
    /// Sporadic: whether a Release has been accepted since instant 0, and
    /// the instant the previous accepted Release became due (R10.1).
    pub has_last: [bool; NT],
    pub last_due: [u64; NT],
    /// Sporadic, deferral policy: the instant a deferred Release becomes due (R10.2).
    pub deferred_until: [u64; NT],
    /// DD-06: the interrupt source is masked until the MIT window and the Job end.
    pub masked: [bool; NT],
    /// The due instant of the current pending or deferred Release (for the deadline, R9.6).
    pub due: [u64; NT],
    /// The due instant of the Release whose Job is in progress.
    pub job_due: [u64; NT],
}

/// O + k * T as a mathematical integer.
pub open spec fn nominal_of<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>, t: int, k: int) -> int {
    cfg.tasks[t].offset + k * cfg.tasks[t].period
}

impl<const NT: usize> Timing<NT> {
    /// Property 7, state part: every Periodic_Task's recorded nominal
    /// instant is O + k*T and lies within the operating duration.
    pub open spec fn inv<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>) -> bool {
        &&& cfg.wf()
        &&& forall|t: int| 0 <= t < NT && cfg.tasks[t].kind == KIND_PERIODIC
            ==> (self.nominal[t] as int) == nominal_of(cfg, t, self.k[t] as int)
        &&& forall|t: int| 0 <= t < NT ==> self.nominal[t] <= cfg.operating_duration
        &&& forall|t: int| 0 <= t < NT ==> self.last_due[t] <= cfg.operating_duration
        &&& forall|t: int| 0 <= t < NT ==> self.deferred_until[t] <= cfg.operating_duration + cfg.tasks[t].period
    }

    pub fn new<const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>) -> (s: Self)
        requires cfg.wf(),
        ensures s.inv(cfg),
            forall|t: int| 0 <= t < NT ==> s.k[t] == 0 && s.nominal[t] == cfg.tasks[t].offset && !s.has_last[t] && !s.masked[t],
    {
        let mut s = Timing {
            k: [0; NT],
            nominal: [0; NT],
            has_last: [false; NT],
            last_due: [0; NT],
            deferred_until: [0; NT],
            masked: [false; NT],
            due: [0; NT],
            job_due: [0; NT],
        };
        let mut t: usize = 0;
        while t < NT
            invariant cfg.wf(), t <= NT,
                forall|u: int| 0 <= u < t ==> s.nominal[u] == cfg.tasks[u].offset,
                forall|u: int| 0 <= u < NT ==> s.k[u] == 0 && !s.has_last[u] && !s.masked[u] && s.last_due[u] == 0 && s.deferred_until[u] == 0,
                forall|u: int| t <= u < NT ==> s.nominal[u] == 0,
            decreases NT - t,
        {
            s.nominal[t] = cfg.tasks[t].offset;
            t = t + 1;
        }
        proof {
            assert forall|u: int| 0 <= u < NT implies (s.nominal[u] as int) == nominal_of(cfg, u, 0) by {
                assert(0 * cfg.tasks[u].period == 0) by (nonlinear_arith);
            }
        }
        s
    }

    /// Property 7: the next nominal instant of Periodic_Task `t`, computed
    /// from the offset and the period only (R9.1, R9.3, R9.5). Returns
    /// `None` when the next instant lies beyond the operating duration
    /// (R9.9: no Release beyond it becomes effective).
    pub fn advance_periodic<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8) -> (next: Option<u64>)
        requires old(self).inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_PERIODIC,
        ensures final(self).inv(cfg),
            next.is_some() ==> final(self).k[t as int] == old(self).k[t as int] + 1
                && (next.unwrap() as int) == nominal_of(cfg, t as int, old(self).k[t as int] as int + 1)
                && final(self).nominal[t as int] == next.unwrap()
                && next.unwrap() <= cfg.operating_duration,
            next.is_none() ==> nominal_of(cfg, t as int, old(self).k[t as int] as int + 1) > cfg.operating_duration
                && final(self).k == old(self).k && final(self).nominal == old(self).nominal,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).k[u] == old(self).k[u] && final(self).nominal[u] == old(self).nominal[u],
            final(self).has_last == old(self).has_last, final(self).last_due == old(self).last_due,
            final(self).masked == old(self).masked, final(self).due == old(self).due, final(self).job_due == old(self).job_due,
            final(self).deferred_until == old(self).deferred_until,
    {
        let period = cfg.tasks[t as usize].period;
        let n = self.nominal[t as usize];
        proof {
            assert(cfg.task_ok(t as int));
            // (k + 1) * T == k * T + T
            let k = self.k[t as int] as int;
            assert((k + 1) * (period as int) == k * (period as int) + (period as int)) by (nonlinear_arith);
        }
        match n.checked_add(period) {
            Some(next) if next <= cfg.operating_duration => {
                if self.k[t as usize] == u64::MAX {
                    // Unreachable: k * T <= duration < 2^62 with T >= 1 bounds k below 2^62.
                    proof {
                        assert((self.k[t as int] as int) * (period as int) >= self.k[t as int] as int) by (nonlinear_arith)
                            requires period >= 1, self.k[t as int] >= 0;
                        assert(false);
                    }
                    return None;
                }
                self.k[t as usize] = self.k[t as usize] + 1;
                self.nominal[t as usize] = next;
                Some(next)
            }
            _ => None,
        }
    }

    /// R10.1 to R10.3: the decision for a Release_Source event of `t` at
    /// `now`, given the Task's current release state (`REL_NONE` when no
    /// Release is pending or deferred).
    pub fn intake_decision<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: u8, release_none: bool, now: u64) -> (d: u8)
        requires self.inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_SPORADIC, now <= cfg.operating_duration,
        ensures
            !release_none ==> d == INTAKE_DISCARD,
            release_none && (!self.has_last[t as int] || now >= self.last_due[t as int] + cfg.tasks[t as int].period) ==> d == INTAKE_ACCEPT,
            release_none && self.has_last[t as int] && now < self.last_due[t as int] + cfg.tasks[t as int].period
                ==> d == (if cfg.tasks[t as int].mit_policy == MIT_DEFER { INTAKE_DEFER } else { INTAKE_DISCARD }),
    {
        if !release_none {
            return INTAKE_DISCARD;
        }
        let mit = cfg.tasks[t as usize].period;
        proof {
            assert(cfg.task_ok(t as int));
        }
        if !self.has_last[t as usize] || now >= self.last_due[t as usize] + mit {
            INTAKE_ACCEPT
        } else if cfg.tasks[t as usize].mit_policy == MIT_DEFER {
            INTAKE_DEFER
        } else {
            INTAKE_DISCARD
        }
    }

    /// Property 8 (MIT spacing): an accepted Release becomes due at `now`,
    /// at least one MIT after the previous accepted one; the source is
    /// masked for the MIT window (DD-06, Property 9).
    pub fn accept<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8, now: u64)
        requires old(self).inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_SPORADIC, now <= cfg.operating_duration,
            !old(self).has_last[t as int] || now >= old(self).last_due[t as int] + cfg.tasks[t as int].period,
        ensures final(self).inv(cfg),
            final(self).has_last[t as int], final(self).last_due[t as int] == now, final(self).due[t as int] == now,
            final(self).masked[t as int],
            old(self).has_last[t as int] ==> now >= old(self).last_due[t as int] + cfg.tasks[t as int].period,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).last_due[u] == old(self).last_due[u] && final(self).has_last[u] == old(self).has_last[u] && final(self).masked[u] == old(self).masked[u] && final(self).due[u] == old(self).due[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal,
            final(self).deferred_until == old(self).deferred_until, final(self).job_due == old(self).job_due,
    {
        self.has_last[t as usize] = true;
        self.last_due[t as usize] = now;
        self.due[t as usize] = now;
        self.masked[t as usize] = true;
    }

    /// R10.2, deferral: the Release becomes due one MIT after the previous
    /// accepted Release became due. Returns that instant.
    pub fn defer<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8) -> (at: u64)
        requires old(self).inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_SPORADIC, old(self).has_last[t as int],
        ensures final(self).inv(cfg),
            at == old(self).last_due[t as int] + cfg.tasks[t as int].period,
            final(self).deferred_until[t as int] == at, final(self).masked[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).deferred_until[u] == old(self).deferred_until[u] && final(self).masked[u] == old(self).masked[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal, final(self).has_last == old(self).has_last,
            final(self).last_due == old(self).last_due, final(self).due == old(self).due, final(self).job_due == old(self).job_due,
    {
        proof {
            assert(cfg.task_ok(t as int));
        }
        let at = self.last_due[t as usize] + cfg.tasks[t as usize].period;
        self.deferred_until[t as usize] = at;
        self.masked[t as usize] = true;
        at
    }

    /// The deferred Release of `t` becomes due at its deferred instant,
    /// which is then the previous accepted due instant for MIT purposes.
    pub fn deferred_due<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT, old(self).deferred_until[t as int] <= cfg.operating_duration,
        ensures final(self).inv(cfg),
            final(self).last_due[t as int] == old(self).deferred_until[t as int],
            final(self).due[t as int] == old(self).deferred_until[t as int],
            final(self).has_last[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).last_due[u] == old(self).last_due[u] && final(self).due[u] == old(self).due[u] && final(self).has_last[u] == old(self).has_last[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal, final(self).masked == old(self).masked,
            final(self).deferred_until == old(self).deferred_until, final(self).job_due == old(self).job_due,
    {
        let at = self.deferred_until[t as usize];
        self.last_due[t as usize] = at;
        self.due[t as usize] = at;
        self.has_last[t as usize] = true;
    }

    /// `deferred_due` with the instant clamped to the operating duration
    /// (a deferred Release beyond it never becomes effective, R9.9).
    pub fn deferred_due_clamped<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8, at: u64)
        requires old(self).inv(cfg), (t as int) < NT, at <= cfg.operating_duration,
        ensures final(self).inv(cfg),
            final(self).last_due[t as int] == at, final(self).due[t as int] == at, final(self).has_last[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).last_due[u] == old(self).last_due[u] && final(self).due[u] == old(self).due[u] && final(self).has_last[u] == old(self).has_last[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal, final(self).masked == old(self).masked,
            final(self).deferred_until == old(self).deferred_until, final(self).job_due == old(self).job_due,
    {
        self.last_due[t as usize] = at;
        self.due[t as usize] = at;
        self.has_last[t as usize] = true;
    }

    /// Records the due instant of a periodic Release (R9.1).
    pub fn periodic_due<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8, at: u64)
        requires old(self).inv(cfg), (t as int) < NT,
        ensures final(self).inv(cfg), final(self).due[t as int] == at,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).due[u] == old(self).due[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal, final(self).masked == old(self).masked,
            final(self).has_last == old(self).has_last, final(self).last_due == old(self).last_due,
            final(self).deferred_until == old(self).deferred_until, final(self).job_due == old(self).job_due,
    {
        self.due[t as usize] = at;
    }

    /// The Job for the current Release starts: its due instant is the one
    /// the deadline check refers to (R9.6).
    pub fn job_started<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires old(self).inv(cfg), (t as int) < NT,
        ensures final(self).inv(cfg), final(self).job_due[t as int] == old(self).due[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).job_due[u] == old(self).job_due[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal, final(self).masked == old(self).masked,
            final(self).has_last == old(self).has_last, final(self).last_due == old(self).last_due,
            final(self).deferred_until == old(self).deferred_until, final(self).due == old(self).due,
    {
        self.job_due[t as usize] = self.due[t as usize];
    }

    /// Property 9 (source masking): the source is unmasked only once its
    /// MIT window has elapsed and its Job has ended (DD-06). Returns
    /// whether it was unmasked.
    pub fn try_unmask<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8, now: u64, job_ended: bool) -> (unmasked: bool)
        requires old(self).inv(cfg), (t as int) < NT, cfg.tasks[t as int].kind == KIND_SPORADIC,
        ensures final(self).inv(cfg),
            unmasked == (job_ended && (!old(self).has_last[t as int] || now >= old(self).last_due[t as int] + cfg.tasks[t as int].period)),
            unmasked ==> !final(self).masked[t as int],
            !unmasked ==> final(self).masked == old(self).masked,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).masked[u] == old(self).masked[u],
            final(self).k == old(self).k, final(self).nominal == old(self).nominal,
            final(self).has_last == old(self).has_last, final(self).last_due == old(self).last_due,
            final(self).deferred_until == old(self).deferred_until, final(self).due == old(self).due, final(self).job_due == old(self).job_due,
    {
        proof {
            assert(cfg.task_ok(t as int));
        }
        let window_over = !self.has_last[t as usize] || now >= self.last_due[t as usize] + cfg.tasks[t as usize].period;
        if job_ended && window_over {
            self.masked[t as usize] = false;
            true
        } else {
            false
        }
    }

    /// Property 7, as a statement about every k: the nominal instant of
    /// Job k equals O + k*T whatever happened to earlier Jobs. The state
    /// records only (k, O + k*T), so every reachable value satisfies it.
    pub proof fn lemma_nominal_is_drift_free<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: int)
        requires self.inv(cfg), 0 <= t < NT, cfg.tasks[t].kind == KIND_PERIODIC,
        ensures (self.nominal[t] as int) == cfg.tasks[t].offset + (self.k[t] as int) * cfg.tasks[t].period,
    {
    }
}

} // verus!
