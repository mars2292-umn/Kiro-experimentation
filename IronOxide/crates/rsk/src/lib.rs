//! rsk: the Application API (design.md "Application API"; task 5.6):
//! Resource access through `Shared::lock`, the monotonic time base
//! `now()`, and the debug console. Context types with one proxy per
//! declared Resource are generated per Task by the Generator (task 10.5);
//! until then the P1 demo constructs `Shared` values by hand.
//!
//! Every operation is a safe wrapper over the Kernel's SVC services. The
//! closure of `lock` receives `&mut T` and nothing that can wait (PR-08);
//! `lock` takes `&mut self`, so a Job cannot enter the same Resource twice
//! (R8.10); the reference cannot leave the closure (R8.6). A Task body sees
//! only its generated Context: one `Shared` proxy per declared Resource and
//! one `Signal` per Release_Signal it may raise (PR-06, R8.7), and the Job
//! wrapper requires it to return `()` (no `async` body, PR-09). The
//! compile-fail fixtures of `xtask/fixtures/api-misuse` check each rule.
#![no_std]
#![forbid(unsafe_code)]

/// The App_Declaration macro (design.md "App_Declaration and generation
/// pipeline"): validates the declaration and exports the `rsk_system!` and
/// `rsk_partition_<name>!` manifests (R21 to R24).
pub use rsk_macros::app;
pub use rsk_kernel::arch::cm4::unsafe_module::ResourceCell;
use rsk_kernel::arch::cm4::unsafe_module::{hw, stubs};

/// The lock operation of a Resource proxy (design.md "Application API",
/// the shape RTIC uses): a lexically scoped Critical_Section whose
/// `&mut T` cannot outlive the closure (R8.6).
pub trait Mutex {
    type T;
    fn lock<R>(&mut self, f: impl FnOnce(&mut Self::T) -> R) -> R;
}

/// The proxy of one declared Resource in a Task's Context (R8.7).
pub struct Shared<'a, T: 'static> {
    cell: &'a ResourceCell<T>,
    id: u8,
}

impl<'a, T: 'static> Shared<'a, T> {
    /// Constructed by generated code only (PR-06): the Resource identifier
    /// is the one of Generated_Config.
    pub fn new(cell: &'a ResourceCell<T>, id: u8) -> Shared<'a, T> {
        Shared { cell, id }
    }

    /// Enters a Critical_Section on the Resource (R8.1), runs `f`, and
    /// exits it (R8.2). A refused lock means the type-system guarantees
    /// were bypassed; the Kernel has logged the misuse, and the Job panics
    /// into its Partition's fault response (PR-19).
    pub fn lock<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        if !stubs::svc_lock(self.id) {
            panic!("rsk: lock refused");
        }
        let r = self.cell.with(f);
        let _ = stubs::svc_unlock(self.id);
        r
    }
}

impl<'a, T: 'static> Mutex for Shared<'a, T> {
    type T = T;
    fn lock<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        Shared::lock(self, f)
    }
}

/// A Release_Signal a Task may raise (R10.5), held in its Context (PR-06).
pub struct Signal {
    id: u8,
}

impl Signal {
    /// Constructed by generated code only: the identifier is the one of
    /// Generated_Config's signal table.
    pub fn new(id: u8) -> Signal {
        Signal { id }
    }

    /// Raises the Release_Signal: a Release_Source event of the bound
    /// Sporadic_Task at the current instant (R10.1 to R10.3), attributed to
    /// this Partition (R10.7). Never waits (PR-08). Returns whether the
    /// Kernel took the event (a refusal is API misuse and a fault of the
    /// Partition; the Kernel decides what the event does).
    pub fn raise(&self) -> bool {
        stubs::svc_raise(self.id)
    }
}

/// The Kernel time base in ticks (PR-26).
pub fn now() -> u64 {
    stubs::svc_now()
}

/// Requests an EasyDMA transfer on `channel` (a channel index of the
/// calling Partition's peripherals, in the order of the summary) over
/// `buf` (DD-08, R17.1). The Kernel programs the peripheral's pointer and
/// count registers only when `buf` lies in the Partition's DMA-permitted
/// memory; otherwise the request is refused before any register is
/// written, the refusal is a spatial violation of the Partition (R17.2),
/// and `false` is returned (if the Partition's fault response lets the Job
/// continue).
pub fn dma_request(channel: u8, buf: &[u8]) -> bool {
    let len = if buf.len() > u32::MAX as usize { u32::MAX } else { buf.len() as u32 };
    stubs::svc_dma(channel, buf.as_ptr() as usize as u32, len)
}

/// Register access to the calling Partition's peripherals (R16.2): a
/// read or write of a word at `addr`. The MPU view confines the access to
/// the Partition's peripheral windows; any other address is a MemManage
/// fault of the Partition (R16.4). EasyDMA peripherals are read-only
/// here; their transfers go through [`dma_request`] (DD-08).
pub mod mmio {
    use rsk_kernel::arch::cm4::unsafe_module::hw;

    #[inline(always)]
    pub fn read(addr: u32) -> u32 {
        hw::peripheral_read(addr)
    }

    #[inline(always)]
    pub fn write(addr: u32, value: u32) {
        hw::peripheral_write(addr, value)
    }
}

/// Writes to the debug console (semihosting). Not for Flight_Builds
/// without a debugger (the Link_Checker reports it there).
pub fn debug(s: &[u8]) {
    hw::semihosting_write(s);
}

/// A bounded busy loop of `n` iterations, for the demo Tasks (PR-16: the
/// bound is the argument).
pub fn spin(n: u32) {
    let mut i = 0u32;
    while i < n {
        core::hint::black_box(i);
        i = i.wrapping_add(1);
    }
}
