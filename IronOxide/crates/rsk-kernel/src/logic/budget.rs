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
//! the Kernel level, above every Ceiling), not a property of this module.
#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::Config;

verus! {

pub struct Budget<const NT: usize> {
    /// Cycles consumed by the current Job of each Task.
    pub used: [u32; NT],
}

impl<const NT: usize> Budget<NT> {
    pub fn new() -> (b: Self)
        ensures forall|t: int| 0 <= t < NT ==> b.used[t] == 0,
    {
        Budget { used: [0; NT] }
    }

    /// A new Job of `t` starts with nothing consumed.
    pub fn reset(&mut self, t: u8)
        requires (t as int) < NT,
        ensures final(self).used[t as int] == 0,
            forall|u: int| 0 <= u < NT && u != t ==> final(self).used[u] == old(self).used[u],
    {
        self.used[t as usize] = 0;
    }

    /// Property 10 (attribution): charges `cycles` to `t` alone,
    /// saturating so that an Overrun can never wrap into compliance.
    pub fn charge(&mut self, t: u8, cycles: u32)
        requires (t as int) < NT,
        ensures final(self).used[t as int] == (if old(self).used[t as int] as int + cycles as int > u32::MAX as int { u32::MAX } else { (old(self).used[t as int] + cycles) as u32 }),
            final(self).used[t as int] >= old(self).used[t as int],
            forall|u: int| 0 <= u < NT && u != t ==> final(self).used[u] == old(self).used[u],
    {
        let v = self.used[t as usize];
        self.used[t as usize] = v.saturating_add(cycles);
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
