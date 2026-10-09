//! Kernel_Arch: the architecture-specific part of the Kernel (R60.1), and
//! the contract between the Kernel and the generated system crate (DD-11).
//!
//! The portable logic (`crate::logic`) is generic over the table sizes. A
//! Flight_Build instantiates it once, in the system crate, as a
//! [`Bound`] value inside a [`KernelCell`] static; the system crate's
//! [`System`] implementation hands that static, the Task entry functions,
//! the vector bindings, and the Partition layout to Kernel_Arch. The entry
//! crate (`rsk-entry`) binds the vectors and calls the Kernel_Arch entry
//! points with the system crate's `System` type.
//!
//! [`KernelOps`] erases the const parameters so that Kernel_Arch needs no
//! generic const expressions; every method applies one transition of the
//! verified logic to the bound configuration.
// An ancestor of the Kernel_Unsafe_Module: `forbid` would propagate into it,
// so this module denies `unsafe_code` and contains none (R6.3).
#![deny(unsafe_code)]

pub mod cm4;

use crate::logic::kernel::Kernel;
use crate::logic::mpu::{Layout, View};
use crate::logic::sched::JOB_KILLED;
use crate::logic::Config;

/// A Kernel instance bound to its configuration and Partition views.
pub struct Bound<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize> {
    pub cfg: Config<NT, NR, NP>,
    pub kernel: Kernel<NT, NR, NP, NL, NS, NE>,
    pub layout: Layout<NP>,
    pub views: [View; NP],
}

/// A placeholder Kernel value for a `static` initializer: `KernelOps::init`
/// replaces it with `Kernel::new` (the verified constructor) at boot.
pub const fn placeholder_kernel<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize>(
) -> Kernel<NT, NR, NP, NL, NS, NE> {
    use crate::logic::budget::Budget;
    use crate::logic::log::{Event, Log};
    use crate::logic::sched::Sched;
    use crate::logic::time::{Slot, Slots, Timing, SLOT_FREE};
    Kernel {
        sched: Sched {
            release: [0; NT],
            job: [0; NT],
            stack: [0; NT],
            depth: 0,
            start_ceiling: [0; NT],
            ceiling: 0,
            lock_res: [0; NL],
            lock_depth: [0; NL],
            lock_len: 0,
        },
        timing: Timing {
            k: [0; NT],
            nominal: [0; NT],
            has_last: [false; NT],
            last_due: [0; NT],
            deferred_until: [0; NT],
            masked: [false; NT],
            due: [0; NT],
            job_due: [0; NT],
        },
        slots: Slots { slots: [Slot { armed: false, at: 0, kind: SLOT_FREE, task: 0 }; NS] },
        budget: Budget { used: [0; NT] },
        log: Log {
            entries: [Event { kind: 0, task: 0xFF, partition: 0xFF, prs: 0, instant: 0, detail: 0 }; NE],
            head: 0,
            len: 0,
            overflow: 0,
            seq: 0,
        },
        stopped: [false; NP],
        mit_violations: [0; NP],
        reinit_partition: [false; NP],
        stop_dma: [false; NP],
        safe_state: false,
        started: false,
        now: 0,
    }
}

/// A disabled region, for `static` initializers.
pub const DISABLED_REGION: crate::logic::mpu::Region = crate::logic::mpu::Region {
    enabled: false,
    interval: crate::logic::mpu::Interval { base: 0, size_log2: 5 },
    srd: 0,
    write: false,
    xn: true,
    device: false,
};

/// An all-disabled view, for `static` initializers.
pub const fn placeholder_view() -> View {
    View { regions: [DISABLED_REGION; crate::logic::mpu::MPU_REGIONS] }
}

/// What Kernel_Arch needs of the executing Job to dispatch it (DD-01).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct JobInfo {
    pub task: u8,
    pub partition: u8,
    pub priority: u8,
    pub fpu: bool,
}

/// The const-erased operations of a bound Kernel.
pub trait KernelOps {
    fn task_count(&self) -> usize;
    fn partition_count(&self) -> usize;
    fn validate(&self) -> bool;
    /// CRC-32 over the configuration tables in memory (R12.3).
    fn config_checksum(&self) -> u32;
    fn init(&mut self);
    fn start(&mut self);
    fn started(&self) -> bool;
    fn safe_state(&self) -> bool;
    fn job_info(&self, t: u8) -> JobInfo;
    fn depth(&self) -> usize;
    fn top(&self) -> Option<u8>;
    /// The Task at stack position `i`.
    fn stack_at(&self, i: usize) -> u8;
    fn is_killed(&self, t: u8) -> bool;
    fn eligible(&self, t: u8) -> bool;
    fn dispatch(&mut self, t: u8, preempted_cycles: u32);
    fn job_return(&mut self, elapsed_cycles: u32);
    fn svc_lock(&mut self, r: u8) -> bool;
    fn svc_unlock(&mut self, r: u8) -> bool;
    fn ceiling(&self) -> u8;
    fn select(&self) -> Option<u8>;
    fn intake(&mut self, t: u8, source_partition: u8, now: u64);
    fn timer_event(&mut self, now: u64, running_cycles: u32);
    fn fault(&mut self, kind: u8, kernel: bool);
    fn next_compare(&self) -> Option<u64>;
    fn source_masked(&self, t: u8) -> bool;
    fn view(&self, p: u8) -> &View;
    fn take_reinit(&mut self, p: u8) -> bool;
    fn take_stop_dma(&mut self, p: u8) -> bool;
    fn log_len(&self) -> usize;
    fn log_kind(&self, i: usize) -> u8;
    fn log_detail(&self, i: usize) -> u32;
    fn log_task(&self, i: usize) -> u8;
    fn log_instant(&self, i: usize) -> u64;
    /// R12.2: records the failed boot checks (bit set) before the safe state.
    fn boot_failed(&mut self, failed: u32);
}

impl<const NT: usize, const NR: usize, const NP: usize, const NL: usize, const NS: usize, const NE: usize> KernelOps
    for Bound<NT, NR, NP, NL, NS, NE>
{
    fn task_count(&self) -> usize {
        NT
    }

    fn partition_count(&self) -> usize {
        NP
    }

    fn config_checksum(&self) -> u32 {
        config_crc(&self.cfg)
    }

    fn validate(&self) -> bool {
        self.cfg.validate()
            && self.cfg.slot_count as usize == NS
            && NL <= 255
            && NE > 0
            && NE < 0x4000_0000
            && self.layout.validate()
    }

    fn init(&mut self) {
        self.kernel = Kernel::new(&self.cfg);
        let mut p = 0;
        while p < NP {
            self.views[p] = self.layout.view(p);
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

    fn job_info(&self, t: u8) -> JobInfo {
        let task = self.cfg.tasks[t as usize];
        JobInfo {
            task: t,
            partition: task.partition,
            priority: task.priority,
            fpu: task.fpu,
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

    fn stack_at(&self, i: usize) -> u8 {
        self.kernel.sched.stack[i]
    }

    fn is_killed(&self, t: u8) -> bool {
        self.kernel.sched.job[t as usize] == JOB_KILLED
    }

    fn eligible(&self, t: u8) -> bool {
        let s = &self.kernel.sched;
        let p = self.cfg.tasks[t as usize].priority;
        s.release[t as usize] == crate::logic::sched::REL_PENDING
            && s.job[t as usize] == crate::logic::sched::JOB_NONE
            && p > s.ceiling
            && (s.depth == 0 || p > self.cfg.tasks[s.stack[s.depth - 1] as usize].priority)
    }

    fn dispatch(&mut self, t: u8, preempted_cycles: u32) {
        self.kernel.dispatch(&self.cfg, t, preempted_cycles);
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

    fn ceiling(&self) -> u8 {
        self.kernel.sched.ceiling
    }

    fn select(&self) -> Option<u8> {
        self.kernel.select(&self.cfg)
    }

    fn intake(&mut self, t: u8, source_partition: u8, now: u64) {
        if (t as usize) < NT
            && (source_partition as usize) < NP
            && self.cfg.tasks[t as usize].kind == crate::logic::KIND_SPORADIC
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
        self.kernel.slots.next().map(|i| self.kernel.slots.slots[i].at)
    }

    fn source_masked(&self, t: u8) -> bool {
        self.kernel.timing.masked[t as usize]
    }

    fn view(&self, p: u8) -> &View {
        &self.views[p as usize]
    }

    fn take_reinit(&mut self, p: u8) -> bool {
        let v = self.kernel.reinit_partition[p as usize];
        if v {
            self.kernel.ack_reinit(&self.cfg, p);
        }
        v
    }

    fn take_stop_dma(&mut self, p: u8) -> bool {
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
        self.kernel.log.get(i).kind
    }

    fn log_detail(&self, i: usize) -> u32 {
        self.kernel.log.get(i).detail
    }

    fn log_task(&self, i: usize) -> u8 {
        self.kernel.log.get(i).task
    }

    fn log_instant(&self, i: usize) -> u64 {
        self.kernel.log.get(i).instant
    }

    fn boot_failed(&mut self, failed: u32) {
        self.kernel.boot_check_failed(&self.cfg, failed);
    }
}

/// How an exception vector is bound (Generated_Config `irq` table, R24.1).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Binding {
    /// The dispatch vector of a Task (DD-01).
    Dispatch(u8),
    /// A peripheral interrupt that releases a Sporadic_Task; the Partition
    /// that owns the peripheral is attributed MIT_Violations (R10.7).
    Intake { task: u8, partition: u8 },
    /// A Kernel-owned interrupt line (the time-base timer).
    Kernel,
    /// Not bound: must stay disabled; an exception from it is a Kernel fault (R19.5).
    Unbound,
}

/// The system crate's contract with Kernel_Arch.
pub trait System: 'static {
    type K: KernelOps + 'static;
    /// The Kernel static. Kernel_Arch touches it only at the Kernel level
    /// or with PRIMASK held (DD-02), which is what makes the cell sound.
    fn kernel() -> &'static cm4::KernelCell<Self::K>;
    /// The Job entry function of Task `t` (the generated Job wrapper).
    fn task_entry(t: u8) -> fn();
    /// The binding of exception number `n` (16 + IRQ number).
    fn binding(n: u16) -> Binding;
    /// The exception number of Task `t`'s dispatch vector.
    fn dispatch_vector(t: u8) -> u16;
    /// The exception number of the interrupt that releases `t`, if any.
    fn intake_vector(t: u8) -> Option<u16>;
    /// The highest address of Partition `p`'s stack (the top of the stack
    /// region at the bottom of its RAM, R16.5).
    fn stack_top(p: u8) -> u32;
    /// The lowest address of Partition `p`'s stack region.
    fn stack_bottom(p: u8) -> u32;
    /// The declared system safe state (R14.4): applied by Kernel_Arch and
    /// never returned from.
    fn safe_state() -> !;
    /// The number of interrupt lines of the Target.
    const IRQ_COUNT: u16;
    /// CRC-32 over the Generated_Config tables, recorded by the Generator
    /// (R24.1) and checked against the tables in memory at boot (R12.3).
    const CONFIG_CHECKSUM: u32;
    /// The exception number of the time-base timer interrupt.
    const TIMER_VECTOR: u16;
    /// Implemented NVIC priority bits of the Target (ASM-03; checked at boot, R12.1).
    const PRIORITY_BITS: u8;
}

/// CRC-32 (IEEE 802.3) of a byte stream, fed one byte at a time; usable
/// in constant evaluation so that a system crate can record the checksum
/// of its own tables.
pub struct Crc32(u32);

impl Crc32 {
    pub const fn new() -> Crc32 {
        Crc32(0xFFFF_FFFF)
    }
    pub const fn byte(mut self, b: u8) -> Crc32 {
        let mut crc = self.0 ^ b as u32;
        let mut i = 0;
        while i < 8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
            i += 1;
        }
        self.0 = crc;
        self
    }
    pub const fn u32(self, v: u32) -> Crc32 {
        let b = v.to_le_bytes();
        self.byte(b[0]).byte(b[1]).byte(b[2]).byte(b[3])
    }
    pub const fn u64(self, v: u64) -> Crc32 {
        self.u32(v as u32).u32((v >> 32) as u32)
    }
    pub const fn finish(&self) -> u32 {
        !self.0
    }
}

impl Default for Crc32 {
    fn default() -> Crc32 {
        Crc32::new()
    }
}

/// The CRC-32 over the Generated_Config tables in memory (R12.3), in the
/// field order the Generator hashes (`rsk_gen::derive::config_bytes`) and
/// the Config_Checker recomputes (R26.4).
pub const fn config_crc<const NT: usize, const NR: usize, const NP: usize>(cfg: &Config<NT, NR, NP>) -> u32 {
    let mut c = Crc32::new().u32(NT as u32).u32(NR as u32).u32(NP as u32);
    let mut t = 0;
    while t < NT {
        let x = &cfg.tasks[t];
        c = c.byte(x.priority).byte(x.partition).byte(x.kind).u64(x.offset).u64(x.period).u64(x.deadline).u32(x.budget).byte(x.mit_policy).byte(x.fpu as u8);
        t += 1;
    }
    let mut r = 0;
    while r < NR {
        c = c.byte(cfg.resources[r].ceiling).byte(cfg.resources[r].partition);
        r += 1;
    }
    let mut t = 0;
    while t < NT {
        let mut r = 0;
        while r < NR {
            c = c.byte(cfg.access[t][r] as u8);
            r += 1;
        }
        t += 1;
    }
    let mut p = 0;
    while p < NP {
        let x = &cfg.partitions[p];
        c = c.byte(x.level).byte(x.fault_response).byte(x.overrun_response).byte(x.deadline_response).u32(x.mit_threshold);
        p += 1;
    }
    c = c.u64(cfg.operating_duration);
    let mut t = 0;
    while t < NT {
        c = c.byte(cfg.slot_release[t]);
        t += 1;
    }
    let mut t = 0;
    while t < NT {
        c = c.byte(cfg.slot_deadline[t]);
        t += 1;
    }
    let mut t = 0;
    while t < NT {
        c = c.byte(cfg.slot_budget[t]);
        t += 1;
    }
    c.byte(cfg.slot_count).finish()
}
