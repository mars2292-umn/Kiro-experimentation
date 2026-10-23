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

pub mod bound;
pub mod cm4;

pub use bound::{config_crc, Bound, Crc32, JobInfo, KernelOps};
pub use cm4::entry::Binding;
pub use cm4::plan::{DmaCfg, SignalCfg};

use crate::logic::kernel::Kernel;
use crate::logic::mpu::View;

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
        history: vstd::prelude::Ghost::assume_new(),
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
    /// The Target's CPUID word (ASM-01; checked at boot, R12.1).
    const CPUID: u32;
    /// Whether the high-frequency clock must run from the crystal (ASM-05; checked at boot).
    const HFCLK_XTAL: bool;
    /// Whether any Task is FPU-using (R11.2; the FPU's presence is then checked at boot).
    const FPU_NEEDED: bool;
    /// Time-base ticks per second (R21.2), for the watchdog reload (ORQ-31).
    const TICK_HZ: u64;
    /// The EasyDMA channels of every Partition's peripherals (DD-08, R17).
    const DMA_COUNT: usize;
    fn dma(i: usize) -> DmaCfg;
    /// The Release_Signals (R10.5), in Generated_Config order.
    const SIGNAL_COUNT: usize;
    fn signal(i: usize) -> SignalCfg;
}
