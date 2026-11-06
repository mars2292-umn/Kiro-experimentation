//! Budget accounting and Overrun detection (Property 10; R18.1, R18.2).
//!
//! Kernel_Arch measures CPU cycles with the DWT cycle counter at every
//! dispatch, preemption, resumption, and completion and charges the
//! elapsed cycles to the Job that was executing (DD-01 stub work). The
//! logic here only accounts: `charge` adds cycles to one Task and no
//! other, so time spent in other Jobs or in Kernel work for other Tasks
//! is never charged to a Job (R18.1). An Overrun is `used > budget`
//! (R18.2); detection "whatever the System_Ceiling" is the DD-02 level
//! argument of the Hardware_Model (the Budget compare interrupt runs at
//! the Kernel level, which no BASEPRI value masks and a Job cannot mask:
//! `hw_model::lemma_kernel_level_never_masked`,
//! `lemma_kernel_level_taken_in_every_state`, `lemma_job_cannot_set_mask`,
//! `lemma_kernel_level_wins_arbitration`), not a property of this module.
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::Config;

verus! {

/// `used + cycles`, saturating at `u32::MAX`.
pub open spec fn charged(used: u32, cycles: u32) -> u32 {
    if used as int + cycles as int > u32::MAX as int { u32::MAX } else { (used + cycles) as u32 }
}

pub struct Budget<const NT: usize> {
    /// Cycles consumed by the current Job of each Task.
    pub used: [u32; NT],
    /// R18.2, R18.6 (2026-11-05): whether the current Job's Overrun has
    /// been reported. Set by the report, cleared with the account at the
    /// Task's next dispatch or its Partition's end: the Overrun of a Job
    /// is reported exactly once, at the first Budget event that finds the
    /// account above the Budget with the Job executing, whether the
    /// account crossed the Budget at that event or at an earlier charge
    /// point (the stress finding of Verification_Plan 11.6: a crossing at
    /// a preemption's charge, taken for an Overrun already reported, left
    /// a Job running on uncounted).
    pub reported: [bool; NT],
}

impl<const NT: usize> Budget<NT> {
    /// R18.2's bookkeeping: a reported Overrun is one the account shows.
    pub open spec fn inv<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>) -> bool {
        forall|t: int| 0 <= t < NT && #[trigger] self.reported[t] ==> self.used[t] > cfg.tasks[t].budget
    }

    pub fn new() -> (b: Self)
        ensures forall|t: int| 0 <= t < NT ==> b.used[t] == 0 && !b.reported[t],
    {
        Budget { used: [0; NT], reported: [false; NT] }
    }

    /// A new Job of `t` starts with nothing consumed and no Overrun reported.
    pub fn reset(&mut self, t: u8)
        requires (t as int) < NT,
        ensures final(self).used[t as int] == 0, !final(self).reported[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).used[u] == old(self).used[u] && final(self).reported[u] == old(self).reported[u],
    {
        self.used[t as usize] = 0;
        self.reported[t as usize] = false;
    }

    /// Property 10 (attribution): charges `cycles` to `t` alone,
    /// saturating so that an Overrun can never wrap into compliance.
    pub fn charge(&mut self, t: u8, cycles: u32)
        requires (t as int) < NT,
        ensures final(self).used[t as int] == charged(old(self).used[t as int], cycles),
            final(self).used[t as int] >= old(self).used[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).used[u] == old(self).used[u],
            final(self).reported == old(self).reported,
    {
        let v = self.used[t as usize];
        self.used[t as usize] = v.saturating_add(cycles);
    }

    /// R18.2: the Overrun of `t`'s Job is reported (once: `reported`).
    pub fn report<const NR: usize, const NP: usize>(&mut self, cfg: &Config<NT, NR, NP>, t: u8)
        requires (t as int) < NT, old(self).used[t as int] > cfg.tasks[t as int].budget,
        ensures final(self).reported[t as int], final(self).used == old(self).used,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).reported[u] == old(self).reported[u],
    {
        self.reported[t as usize] = true;
    }

    /// R18.1 over an entry point: an account changes from `old`'s only by
    /// a reset to 0 (a response that ended the Task's Partition, R14.5);
    /// nothing is charged to any Task.
    pub open spec fn only_reset_from(&self, old: &Self) -> bool {
        forall|u: int| 0 <= u < NT ==> #[trigger] self.used[u] == old.used[u] || self.used[u] == 0
    }

    /// `only_reset_from` is transitive.
    pub proof fn lemma_only_reset_trans(a: &Self, b: &Self, c: &Self)
        requires b.only_reset_from(a), c.only_reset_from(b),
        ensures c.only_reset_from(a),
    {
        assert forall|u: int| 0 <= u < NT implies #[trigger] c.used[u] == a.used[u] || c.used[u] == 0 by {
            assert(b.used[u] == a.used[u] || b.used[u] == 0);
            assert(c.used[u] == b.used[u] || c.used[u] == 0);
        }
    }

    /// R18.6 over a response: a reported Overrun stays reported unless the
    /// account was reset (the Task's Partition ended).
    pub open spec fn reported_kept(&self, old: &Self) -> bool {
        forall|u: int| 0 <= u < NT && #[trigger] old.reported[u] ==> self.reported[u] || self.used[u] == 0
    }

    /// `reported_kept` composes with `only_reset_from` (a reset account stays reset).
    pub proof fn lemma_reported_kept_trans(a: &Self, b: &Self, c: &Self)
        requires b.reported_kept(a), c.reported_kept(b), c.only_reset_from(b),
        ensures c.reported_kept(a),
    {
        assert forall|u: int| 0 <= u < NT && #[trigger] a.reported[u] implies c.reported[u] || c.used[u] == 0 by {
            assert(b.reported[u] || b.used[u] == 0);
            assert(c.used[u] == b.used[u] || c.used[u] == 0);
        }
    }

    /// R18.2: whether the Job of `t` has consumed more than its Budget.
    pub fn overrun<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: u8) -> (o: bool)
        requires (t as int) < NT,
        ensures o == (self.used[t as int] > cfg.tasks[t as int].budget),
    {
        self.used[t as usize] > cfg.tasks[t as usize].budget
    }

    /// The remaining Budget of `t`, which Kernel_Arch arms as the compare
    /// value of the Budget timer when the Job (re)starts executing.
    pub fn remaining<const NR: usize, const NP: usize>(&self, cfg: &Config<NT, NR, NP>, t: u8) -> (r: u32)
        requires (t as int) < NT,
        ensures r == (if self.used[t as int] >= cfg.tasks[t as int].budget { 0 } else { (cfg.tasks[t as int].budget - self.used[t as int]) as u32 }),
    {
        let b = cfg.tasks[t as usize].budget;
        let u = self.used[t as usize];
        if u >= b { 0 } else { b - u }
    }
}

} // verus!
