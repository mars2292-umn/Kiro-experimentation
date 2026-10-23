//! The bound Kernel and the Kernel_Arch planning functions that are pure
//! computation over the Kernel state, inside `verus!` (GAP-10, task 8.1;
//! verification_plan.md section 4).
//!
//! Before this module existed, `Bound` and `KernelOps` were safe Rust
//! outside `verus!`, so Verus never checked that Kernel_Arch calls the
//! logic entry points only when their `requires` clauses hold (fact A9 of
//! the Verification_Plan). Here every `KernelOps` method carries a
//! specification, and Verus checks the implementation for `Bound`:
//!
//! - **A9**: every call into `Kernel` satisfies that entry point's
//!   `requires`, either because the trait requires `inv()` or because the
//!   method's own guard establishes the rest; a guard that fails to mirror
//!   a `requires` clause fails the proof.
//! - **A6** (views): `inv()` states that every Partition view is
//!   `Layout::view` of the validated layout (`is_view_of`), established by
//!   `init` and kept by every method.
//! - **A1** (computation): `basepri_now` returns exactly the
//!   Hardware_Model's `basepri_for(job_level, ceiling)`; `encode_level`,
//!   `basepri_for_ceiling` and `basepri_for` return the Hardware_Model's
//!   encodings. Writing the value to BASEPRI stays with Kernel_Arch
//!   (`arch::cm4`), outside the proof.
//! - **A4** (half): `eligible` decides exactly `Sched::eligible`, so the
//!   stale-pending-bit check at dispatch entry rejects exactly the Jobs
//!   the logic deems not eligible; `select` names only an eligible Job.
//! - **A11**: `init` is the only method that establishes `inv()`, from
//!   `valid()`, which `validate` establishes; every other method requires
//!   `inv()`. That the callers in `arch::cm4` respect this order remains
//!   by inspection, since they are outside `verus!`.
//!
//! What stays outside: `arch::cm4` (register writes, exception frames,
//! the level records), `System`, `Binding`, and the `static` initializers
//! `placeholder_kernel` and `placeholder_view`.
#![forbid(unsafe_code)]

use vstd::prelude::*;

use crate::hw_model;
use crate::logic::kernel::Kernel;
use crate::logic::mpu::{Layout, View};
use crate::logic::sched::{JOB_KILLED, JOB_NONE, REL_PENDING};
use crate::logic::{Config, KIND_SPORADIC, MAX_PRIORITY};

verus! {

// ------------------------------------------------------- priority encodings

/// NVIC priority encoding of an rsk level: level 0 (Kernel) is 0x00; a
/// Task of Priority p is (8 - p) << 5 (HW-01, DD-02). Mirrors
/// `hw_model::encode_level`, which the dispatch lemmas use.
pub fn encode_level(level: u8) -> (e: u8)
    requires level <= MAX_PRIORITY,
    ensures e as int == hw_model::encode_level(level as int),
{
    if level == 0 { 0 } else { (8 - level) * 32 }
}

/// BASEPRI for a System_Ceiling (HW-03): 0 for the idle level.
pub fn basepri_for_ceiling(c: u8) -> (b: u8)
    requires c <= MAX_PRIORITY,
    ensures b as int == hw_model::basepri_of_ceiling(c as int),
{
    if c == 0 { 0 } else { encode_level(c) }
}

/// BASEPRI while a Job of Priority `top` executes under System_Ceiling
/// `ceiling` (DD-01): the higher of the two levels, so that only the
/// dispatch vectors of Jobs that may preempt (strictly higher Priority
/// than both) and the Kernel level stay open.
pub fn basepri_for(top: u8, ceiling: u8) -> (b: u8)
    requires top <= MAX_PRIORITY, ceiling <= MAX_PRIORITY,
    ensures b as int == hw_model::basepri_for(top as int, ceiling as int),
{
    basepri_for_ceiling(if top > ceiling { top } else { ceiling })
}

// ------------------------------------------------------------------- CRC-32

/// CRC-32 (IEEE 802.3) of a byte stream, fed one byte at a time.
pub struct Crc32(pub u32);

impl Crc32 {
    pub fn new() -> Crc32 {
        Crc32(0xFFFF_FFFF)
    }

    pub fn byte(self, b: u8) -> Crc32 {
        let mut crc: u32 = self.0 ^ (b as u32);
        let mut i: usize = 0;
        while i < 8
            invariant i <= 8,
            decreases 8 - i,
        {
            let mask: u32 = if (crc & 1) == 1 { 0xFFFF_FFFF } else { 0 };
            crc = (crc >> 1) ^ (0xEDB8_8320u32 & mask);
            i += 1;
        }
        Crc32(crc)
    }

    /// Little-endian byte order, as `u32::to_le_bytes`.
    pub fn u32(self, v: u32) -> Crc32 {
        self.byte((v & 0xFF) as u8)
            .byte(((v >> 8) & 0xFF) as u8)
            .byte(((v >> 16) & 0xFF) as u8)
            .byte(((v >> 24) & 0xFF) as u8)
    }

    pub fn u64(self, v: u64) -> Crc32 {
        self.u32((v & 0xFFFF_FFFF) as u32).u32(((v >> 32) & 0xFFFF_FFFF) as u32)
    }

    pub fn finish(&self) -> u32 {
        !self.0
    }
}

/// The CRC-32 over the Generated_Config tables in memory (R12.3), in the
/// field order the Generator hashes (`rsk_gen::derive::config_bytes`) and
/// the Config_Checker recomputes (R26.4).
pub fn config_crc<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>) -> u32 {
    let mut c = Crc32::new().u32(NT as u32).u32(NR as u32).u32(NP as u32);
    let mut t: usize = 0;
    while t < NT
        invariant t <= NT,
        decreases NT - t,
    {
        let x = cfg.tasks[t];
        c = c.byte(x.priority).byte(x.partition).byte(x.kind).u64(x.offset).u64(x.period).u64(x.deadline)
            .u32(x.budget).byte(x.mit_policy).byte(if x.fpu { 1 } else { 0 });
        t += 1;
    }
    let mut r: usize = 0;
    while r < NR
        invariant r <= NR,
        decreases NR - r,
    {
        c = c.byte(cfg.resources[r].ceiling).byte(cfg.resources[r].partition);
        r += 1;
    }
    let mut t: usize = 0;
    while t < NT
        invariant t <= NT,
        decreases NT - t,
    {
        let mut r: usize = 0;
        while r < NR
            invariant r <= NR, t < NT,
            decreases NR - r,
        {
            c = c.byte(if cfg.access[t][r] { 1 } else { 0 });
            r += 1;
        }
        t += 1;
    }
    let mut p: usize = 0;
    while p < NP
        invariant p <= NP,
        decreases NP - p,
    {
        let x = cfg.partitions[p];
        c = c.byte(x.level).byte(x.fault_response).byte(x.overrun_response).byte(x.deadline_response).u32(x.mit_threshold);
        p += 1;
    }
    c = c.u64(cfg.operating_duration);
    let mut t: usize = 0;
    while t < NT
        invariant t <= NT,
        decreases NT - t,
    {
        c = c.byte(cfg.slot_release[t]);
        t += 1;
    }
    let mut t: usize = 0;
    while t < NT
        invariant t <= NT,
        decreases NT - t,
    {
        c = c.byte(cfg.slot_deadline[t]);
        t += 1;
    }
    let mut t: usize = 0;
    while t < NT
        invariant t <= NT,
        decreases NT - t,
    {
        c = c.byte(cfg.slot_budget[t]);
        t += 1;
    }
    c.byte(cfg.slot_count).finish()
}

/// The CRC-32 over the `LAYOUT` in memory (R12.3), in the field order the
/// Generator hashes (`rsk_gen::derive::layout_bytes`) and the Config_Checker
/// decodes (R26.4): the three Kernel intervals, NP, then per Partition code
/// and RAM (base, size_log2, srd), stack_size, window_count, and every
/// window slot (base, size_log2).
pub fn layout_crc<const NP: usize>(layout: &Layout<NP>) -> u32 {
    let mut c = Crc32::new();
    c = c.u32(layout.kernel_flash.base).byte(layout.kernel_flash.size_log2).byte(0);
    c = c.u32(layout.shared_code.base).byte(layout.shared_code.size_log2).byte(0);
    c = c.u32(layout.kernel_ram.base).byte(layout.kernel_ram.size_log2).byte(0);
    c = c.u32(NP as u32);
    let mut p: usize = 0;
    while p < NP
        invariant p <= NP,
        decreases NP - p,
    {
        let pl = layout.partitions[p];
        c = c.u32(pl.code.base).byte(pl.code.size_log2).byte(pl.code_srd);
        c = c.u32(pl.ram.base).byte(pl.ram.size_log2).byte(pl.ram_srd);
        c = c.u32(pl.stack_size).byte(pl.window_count);
        let mut w: usize = 0;
        while w < crate::logic::mpu::MAX_WINDOWS
            invariant w <= crate::logic::mpu::MAX_WINDOWS, p < NP,
            decreases crate::logic::mpu::MAX_WINDOWS - w,
        {
            c = c.u32(pl.windows[w].base).byte(pl.windows[w].size_log2);
            w += 1;
        }
        p += 1;
    }
    c.finish()
}

// ------------------------------------------------------------ the bound Kernel

/// What Kernel_Arch needs of the executing Job to dispatch it (DD-01).
#[derive(Clone, Copy)]
pub struct JobInfo {
    pub task: u8,
    pub partition: u8,
    pub priority: u8,
    pub fpu: bool,
}

/// A Kernel instance bound to its configuration and Partition views.
pub struct Bound<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize> {
    pub cfg: Config<NT, NR, NP>,
    pub kernel: Kernel<NT, NR, NP, NL, NS, NE>,
    pub layout: Layout<NP>,
    pub views: [View; NP],
}

/// The const-erased operations of a bound Kernel. Every method but
/// `validate` and `init` requires the invariant `inv()`, which `init`
/// establishes from `valid()` and every mutating method preserves; so
/// Verus checks, for the implementation on `Bound`, that each call into
/// the verified logic satisfies that entry point's `requires` (A9).
pub trait KernelOps: Sized {
    /// The configuration is well formed, the Kernel state satisfies the
    /// logic's invariant, and the Partition views are those of the
    /// validated layout.
    spec fn inv(&self) -> bool;
    /// What `validate` establishes and `init` needs: the tables are well
    /// formed and the const parameters agree with them.
    spec fn valid(&self) -> bool;
    /// The number of Tasks.
    spec fn task_count_spec(&self) -> int;
    /// The executing Job's Priority, 0 when no Job is in progress
    /// (`hw_model::job_level`).
    spec fn top_level(&self) -> int;
    /// The System_Ceiling.
    spec fn ceiling_level(&self) -> int;
    /// `Sched::eligible` for Task `t` (R7.1).
    spec fn task_eligible(&self, t: int) -> bool;

    fn task_count(&self) -> (n: usize)
        requires self.inv(),
        ensures n as int == self.task_count_spec(), n <= 255;
    fn partition_count(&self) -> (n: usize)
        requires self.inv(),
        ensures n <= 255;
    /// Boot validation of the tables (R12.1, R12.3).
    fn validate(&self) -> (ok: bool)
        ensures ok ==> self.valid();
    /// CRC-32 over the configuration tables in memory (R12.3).
    fn config_checksum(&self) -> u32;
    /// CRC-32 over the `LAYOUT` in memory (R12.3, R26.4).
    fn layout_checksum(&self) -> u32;
    /// Replaces the `static` initializer's placeholder by the verified
    /// initial state and computes the Partition views (A6, A11).
    fn init(&mut self)
        requires old(self).valid(),
        ensures final(self).inv();
    fn start(&mut self)
        requires old(self).inv(),
        ensures final(self).inv();
    fn started(&self) -> bool;
    fn safe_state(&self) -> bool;
    fn job_info(&self, t: u8) -> (info: JobInfo)
        requires self.inv(),
        ensures info.task == t, info.priority <= MAX_PRIORITY;
    fn depth(&self) -> usize;
    fn top(&self) -> Option<u8>
        requires self.inv();
    /// The Priority of the executing Job, 0 when none.
    fn top_priority(&self) -> (p: u8)
        requires self.inv(),
        ensures p as int == self.top_level();
    /// The Task at stack position `i`.
    fn stack_at(&self, i: usize) -> u8;
    fn is_killed(&self, t: u8) -> bool;
    /// R7.1: whether Task `t` may start now; the dispatch stub rejects a
    /// stale pending bit with it (A4).
    fn eligible(&self, t: u8) -> (e: bool)
        requires self.inv(),
        ensures e == ((t as int) < self.task_count_spec() && self.task_eligible(t as int));
    fn dispatch(&mut self, t: u8, preempted_cycles: u32)
        requires old(self).inv(),
        ensures final(self).inv();
    fn job_return(&mut self, elapsed_cycles: u32)
        requires old(self).inv(),
        ensures final(self).inv();
    fn svc_lock(&mut self, r: u8) -> bool
        requires old(self).inv(),
        ensures final(self).inv();
    fn svc_unlock(&mut self, r: u8) -> bool
        requires old(self).inv(),
        ensures final(self).inv();
    fn ceiling(&self) -> (c: u8)
        requires self.inv(),
        ensures c as int == self.ceiling_level();
    /// The BASEPRI the Kernel state calls for (DD-01, HW-03): the executing
    /// Job's level or the System_Ceiling, whichever is higher (A1).
    fn basepri_now(&self) -> (b: u8)
        requires self.inv(),
        ensures b as int == hw_model::basepri_for(self.top_level(), self.ceiling_level());
    /// The Job to pend next (Property 1); only an eligible one (A4).
    fn select(&self) -> (sel: Option<u8>)
        requires self.inv(),
        ensures sel.is_some() ==> (sel.unwrap() as int) < self.task_count_spec() && self.task_eligible(sel.unwrap() as int);
    fn intake(&mut self, t: u8, source_partition: u8, now: u64)
        requires old(self).inv(),
        ensures final(self).inv();
    fn timer_event(&mut self, now: u64, running_cycles: u32)
        requires old(self).inv(),
        ensures final(self).inv();
    fn fault(&mut self, kind: u8, kernel: bool)
        requires old(self).inv(),
        ensures final(self).inv();
    fn next_compare(&self) -> Option<u64>
        requires self.inv();
    fn source_masked(&self, t: u8) -> bool
        requires self.inv();
    fn partition_view(&self, p: u8) -> &View
        requires self.inv();
    fn take_reinit(&mut self, p: u8) -> bool
        requires old(self).inv(),
        ensures final(self).inv(), final(self).top_level() == old(self).top_level(), final(self).ceiling_level() == old(self).ceiling_level();
    fn take_stop_dma(&mut self, p: u8) -> bool
        requires old(self).inv(),
        ensures final(self).inv(), final(self).top_level() == old(self).top_level(), final(self).ceiling_level() == old(self).ceiling_level();
    fn log_len(&self) -> usize
        requires self.inv();
    fn log_kind(&self, i: usize) -> u8
        requires self.inv();
    fn log_detail(&self, i: usize) -> u32
        requires self.inv();
    fn log_task(&self, i: usize) -> u8
        requires self.inv();
    fn log_instant(&self, i: usize) -> u64
        requires self.inv();
    /// R12.2: records the failed boot checks (bit set) before the safe state.
    fn boot_failed(&mut self, failed: u32)
        requires old(self).inv(),
        ensures final(self).inv();
    /// Logs the debug state and a watchdog reset at boot (ORQ-30, ORQ-31).
    fn boot_info(&mut self, debugger: bool, watchdog_reset: bool)
        requires old(self).inv(),
        ensures final(self).inv();
    /// Whether Partition `p` is stopped (R14.5): its interrupt sources stay disabled.
    fn partition_stopped(&self, p: u8) -> bool
        requires self.inv();
    /// R17.1: whether `[ptr, ptr + len)` is DMA-permitted memory of Partition `p`.
    spec fn dma_permitted_spec(&self, p: int, ptr: int, len: int) -> bool;
    fn dma_permitted(&self, p: u8, ptr: u32, len: u32) -> (ok: bool)
        requires self.inv(),
        ensures ok == ((p as int) < self.partition_count_spec() && self.dma_permitted_spec(p as int, ptr as int, len as int));
    spec fn partition_count_spec(&self) -> int;
}

impl<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize> KernelOps
    for Bound<NT, NR, NP, NL, NS, NE>
{
    open spec fn inv(&self) -> bool {
        &&& self.cfg.wf()
        &&& self.kernel.inv(&self.cfg)
        &&& self.layout.wf()
        // A6: the views Kernel_Arch loads are the verified view function's
        // output for the validated layout (Property 12).
        &&& forall|p: int| 0 <= p < NP ==> self.layout.is_view_of(p, &self.views[p])
    }

    open spec fn valid(&self) -> bool {
        &&& self.cfg.wf()
        &&& (self.cfg.slot_count as int) == NS
        &&& NL <= 255
        &&& 0 < NE < 0x4000_0000
        &&& self.layout.wf()
    }

    open spec fn task_count_spec(&self) -> int {
        NT as int
    }

    open spec fn top_level(&self) -> int {
        hw_model::job_level(&self.kernel.sched, &self.cfg)
    }

    open spec fn ceiling_level(&self) -> int {
        self.kernel.sched.ceiling as int
    }

    open spec fn task_eligible(&self, t: int) -> bool {
        self.kernel.sched.eligible(&self.cfg, t)
    }

    open spec fn dma_permitted_spec(&self, p: int, ptr: int, len: int) -> bool {
        self.layout.partitions[p].dma_permitted(ptr, len)
    }

    open spec fn partition_count_spec(&self) -> int {
        NP as int
    }

    fn task_count(&self) -> (n: usize) {
        NT
    }

    fn partition_count(&self) -> (n: usize) {
        NP
    }

    fn config_checksum(&self) -> u32 {
        config_crc(&self.cfg)
    }

    fn layout_checksum(&self) -> u32 {
        layout_crc(&self.layout)
    }

    fn validate(&self) -> (ok: bool) {
        self.cfg.validate()
            && self.cfg.slot_count as usize == NS
            && NL <= 255
            && NE > 0
            && NE < 0x4000_0000
            && self.layout.validate()
    }

    fn init(&mut self) {
        self.kernel = Kernel::new(&self.cfg);
        let mut p: usize = 0;
        while p < NP
            invariant
                p <= NP,
                self.cfg == old(self).cfg, self.layout == old(self).layout,
                self.cfg.wf(), self.layout.wf(), self.kernel.inv(&self.cfg),
                forall|q: int| 0 <= q < p ==> self.layout.is_view_of(q, &self.views[q]),
            decreases NP - p,
        {
            let v = self.layout.view(p);
            self.views[p] = v;
            p += 1;
        }
    }

    fn start(&mut self) {
        if !self.kernel.started {
            self.kernel.start(&self.cfg);
        }
    }

    fn started(&self) -> bool {
        self.kernel.started
    }

    fn safe_state(&self) -> bool {
        self.kernel.safe_state
    }

    fn job_info(&self, t: u8) -> (info: JobInfo) {
        if (t as usize) < NT {
            proof {
                assert(self.cfg.task_ok(t as int));
            }
            let task = self.cfg.tasks[t as usize];
            JobInfo { task: t, partition: task.partition, priority: task.priority, fpu: task.fpu }
        } else {
            JobInfo { task: t, partition: 0, priority: 0, fpu: false }
        }
    }

    fn depth(&self) -> usize {
        self.kernel.sched.depth
    }

    fn top(&self) -> Option<u8> {
        if self.kernel.sched.depth == 0 {
            None
        } else {
            Some(self.kernel.sched.stack[self.kernel.sched.depth - 1])
        }
    }

    fn top_priority(&self) -> (p: u8) {
        let s = &self.kernel.sched;
        if s.depth == 0 {
            0
        } else {
            self.cfg.tasks[s.stack[s.depth - 1] as usize].priority
        }
    }

    fn stack_at(&self, i: usize) -> u8 {
        if i < NT { self.kernel.sched.stack[i] } else { 0 }
    }

    fn is_killed(&self, t: u8) -> bool {
        if (t as usize) < NT { self.kernel.sched.job[t as usize] == JOB_KILLED } else { false }
    }

    fn eligible(&self, t: u8) -> (e: bool) {
        if (t as usize) >= NT {
            return false;
        }
        let s = &self.kernel.sched;
        let p = self.cfg.tasks[t as usize].priority;
        s.release[t as usize] == REL_PENDING
            && s.job[t as usize] == JOB_NONE
            && p > s.ceiling
            && (s.depth == 0 || p > self.cfg.tasks[s.stack[s.depth - 1] as usize].priority)
    }

    fn dispatch(&mut self, t: u8, preempted_cycles: u32) {
        // The guards mirror `Kernel::dispatch`'s `requires` (A9); the stub
        // has already rejected a stale pending bit with `eligible`.
        if self.kernel.started && self.eligible(t) {
            self.kernel.dispatch(&self.cfg, t, preempted_cycles);
        }
    }

    fn job_return(&mut self, elapsed_cycles: u32) {
        if self.kernel.sched.depth > 0 {
            self.kernel.job_return(&self.cfg, elapsed_cycles);
        }
    }

    fn svc_lock(&mut self, r: u8) -> bool {
        if self.kernel.sched.depth == 0 || (r as usize) >= NR {
            return false;
        }
        self.kernel.svc_lock(&self.cfg, r)
    }

    fn svc_unlock(&mut self, r: u8) -> bool {
        if self.kernel.sched.depth == 0 {
            return false;
        }
        self.kernel.svc_unlock(&self.cfg, r)
    }

    fn ceiling(&self) -> (c: u8) {
        self.kernel.sched.ceiling
    }

    fn basepri_now(&self) -> (b: u8) {
        proof {
            if self.kernel.sched.depth > 0 {
                assert(self.cfg.task_ok(self.kernel.sched.top()));
            }
        }
        basepri_for(self.top_priority(), self.ceiling())
    }

    fn select(&self) -> (sel: Option<u8>) {
        self.kernel.select(&self.cfg)
    }

    fn intake(&mut self, t: u8, source_partition: u8, now: u64) {
        if (t as usize) < NT
            && (source_partition as usize) < NP
            && self.cfg.tasks[t as usize].kind == KIND_SPORADIC
            && now <= self.cfg.operating_duration
        {
            self.kernel.intake(&self.cfg, t, source_partition, now);
        }
    }

    fn timer_event(&mut self, now: u64, running_cycles: u32) {
        let now = if now > self.cfg.operating_duration { self.cfg.operating_duration } else { now };
        self.kernel.timer_event(&self.cfg, now, running_cycles);
    }

    fn fault(&mut self, kind: u8, kernel: bool) {
        self.kernel.fault(&self.cfg, kind, kernel);
    }

    fn next_compare(&self) -> Option<u64> {
        match self.kernel.slots.next() {
            Some(i) => Some(self.kernel.slots.slots[i].at),
            None => None,
        }
    }

    fn source_masked(&self, t: u8) -> bool {
        if (t as usize) < NT { self.kernel.timing.masked[t as usize] } else { false }
    }

    fn partition_view(&self, p: u8) -> &View {
        let i = if (p as usize) < NP { p as usize } else { 0 };
        &self.views[i]
    }

    fn take_reinit(&mut self, p: u8) -> bool {
        if (p as usize) >= NP {
            return false;
        }
        let v = self.kernel.reinit_partition[p as usize];
        if v {
            self.kernel.ack_reinit(&self.cfg, p);
        }
        v
    }

    fn take_stop_dma(&mut self, p: u8) -> bool {
        if (p as usize) >= NP {
            return false;
        }
        let v = self.kernel.stop_dma[p as usize];
        if v {
            self.kernel.ack_stop_dma(&self.cfg, p);
        }
        v
    }

    fn log_len(&self) -> usize {
        self.kernel.log.len
    }

    fn log_kind(&self, i: usize) -> u8 {
        if i < self.kernel.log.len { self.kernel.log.get(i).kind } else { 0 }
    }

    fn log_detail(&self, i: usize) -> u32 {
        if i < self.kernel.log.len { self.kernel.log.get(i).detail } else { 0 }
    }

    fn log_task(&self, i: usize) -> u8 {
        if i < self.kernel.log.len { self.kernel.log.get(i).task } else { 0xFF }
    }

    fn log_instant(&self, i: usize) -> u64 {
        if i < self.kernel.log.len { self.kernel.log.get(i).instant } else { 0 }
    }

    fn boot_failed(&mut self, failed: u32) {
        self.kernel.boot_check_failed(&self.cfg, failed);
    }

    fn boot_info(&mut self, debugger: bool, watchdog_reset: bool) {
        self.kernel.boot_info(&self.cfg, debugger, watchdog_reset);
    }

    fn partition_stopped(&self, p: u8) -> bool {
        if (p as usize) < NP { self.kernel.stopped[p as usize] } else { true }
    }

    fn dma_permitted(&self, p: u8, ptr: u32, len: u32) -> (ok: bool) {
        if (p as usize) >= NP {
            return false;
        }
        proof {
            assert(self.layout.partition_ok(p as int));
        }
        self.layout.partitions[p as usize].dma_permitted_exec(ptr, len)
    }
}

} // verus!
