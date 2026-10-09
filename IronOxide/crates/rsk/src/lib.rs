//! rsk: the Application API (design.md "Application API"; task 5.6):
//! Resource access through `Shared::lock`, the monotonic time base
//! `now()`, and the debug console. Context types with one proxy per
//! declared Resource are generated per Task by the Generator (task 10.5);
//! until then the P1 demo constructs `Shared` values by hand.
//!
//! Every operation is a safe wrapper over the Kernel's SVC services. The
//! closure of `lock` receives `&mut T` and nothing that can wait (PR-08);
//! `lock` takes `&mut self`, so a Job cannot enter the same Resource twice
//! (R8.10); the reference cannot leave the closure (R8.6).
#![no_std]
#![forbid(unsafe_code)]

/// The App_Declaration macro (design.md "App_Declaration and generation
/// pipeline"): validates the declaration and exports the `rsk_system!` and
/// `rsk_partition_<name>!` manifests (R21 to R24).
pub use rsk_macros::app;
pub use rsk_kernel::arch::cm4::unsafe_module::ResourceCell;
use rsk_kernel::arch::cm4::unsafe_module::{hw, stubs};

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

/// The Kernel time base in ticks (PR-26).
pub fn now() -> u64 {
    stubs::svc_now()
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
