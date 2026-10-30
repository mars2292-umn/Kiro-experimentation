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
/// The App_Prover's specification library (task 17.1), for Partition code
/// compiled under Creusot (`cfg(creusot)`): `#[cfg_attr(creusot,
/// rsk::creusot_std::macros::trusted)]` and the like. Absent from every
/// Flight_Build (the `creusot` feature is off).
#[cfg(feature = "creusot")]
pub use creusot_std;
#[cfg(feature = "creusot")]
#[allow(unused_imports)]
use creusot_std::{invariant::inv, std::ops::FnOnceExt};

/// The App_Prover's logical models (task 17.1): a `PerTask<T>` is viewed
/// as its current value, which `with`, `get`, and `set` relate exactly
/// (only this Task's Jobs touch it); a `Shared<T>` is viewed as the
/// Resource's invariant, a predicate on `T` (R46.4): what a Critical_Section
/// finds is any value satisfying it, and what it leaves must satisfy it.
/// Other Tasks change the value between Critical_Sections, so no stronger
/// model is sound for a Resource.
#[cfg(feature = "creusot")]
mod spec {
    use creusot_std::logic::Mapping;
    use creusot_std::prelude::*;

    impl<'a, T: 'static> View for super::PerTask<'a, T> {
        type ViewTy = T;
        #[logic(opaque)]
        fn view(self) -> T {
            dead
        }
    }

    impl<'a, T: 'static> View for super::Shared<'a, T> {
        type ViewTy = Mapping<T, bool>;
        #[logic(opaque)]
        fn view(self) -> Mapping<T, bool> {
            dead
        }
    }
}
/// `#[rsk::bounds('label = N)]`: the Loop_Bound_Annotation of a function's
/// labelled loops (R35, PR-16).
pub use rsk_macros::bounds;
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
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(cell: &'a ResourceCell<T>, id: u8) -> Shared<'a, T> {
        Shared { cell, id }
    }

    /// Enters a Critical_Section on the Resource (R8.1), runs `f`, and
    /// exits it (R8.2). A refused lock means the type-system guarantees
    /// were bypassed; the Kernel has logged the misuse, and the Job panics
    /// into its Partition's fault response (PR-19).
    // App_Prover contract (task 17.1, trusted: the body is the Kernel's SVC
    // pair, R8.1 to R8.3): the closure runs once on the Resource's value and
    // its result is returned; the closure must be ready for any value.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T> inv(v) && (*self)@[*v] ==> f.precondition((v,))))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T, r: R> inv(v) && (*self)@[*v] && f.postcondition_once((v,), r) ==> (*self)@[^v]))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures((^self)@ == (*self)@))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(exists<v: &mut T> inv(v) && (*self)@[*v] && (*self)@[^v] && f.postcondition_once((v,), result)))]
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
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    fn lock<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        Shared::lock(self, f)
    }
}

/// Per-Task state (PR-25): declared `state <task>.<name>: T = init;` in
/// the App_Declaration and held in the Task's Context. Only this Task's
/// Jobs touch the cell, and one Job of a Task is in progress at a time
/// (Property 2), so no lock is needed; the Generator emits the cell, so
/// Partition code declares no interior-mutable `static` (PR-05).
pub struct PerTask<'a, T: 'static> {
    cell: &'a ResourceCell<T>,
}

impl<'a, T: 'static> PerTask<'a, T> {
    /// Constructed by generated code only.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(cell: &'a ResourceCell<T>) -> PerTask<'a, T> {
        PerTask { cell }
    }

    /// Runs `f` on the state. App_Prover contract (task 17.1, trusted): the
    /// closure runs once on the state and its result is returned.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T> inv(v) && *v == (*self)@ ==> f.precondition((v,))))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(exists<v: &mut T> inv(v) && *v == (*self)@ && ^v == (^self)@ && f.postcondition_once((v,), result)))]
    pub fn with<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        self.cell.with_unprivileged(f)
    }

    /// A copy of the state.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result == self@))]
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.cell.peek_unprivileged()
    }

    /// Replaces the state.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures((^self)@ == value))]
    pub fn set(&mut self, value: T) {
        self.with(|v| *v = value)
    }
}

/// A Release_Signal a Task may raise (R10.5), held in its Context (PR-06).
pub struct Signal {
    id: u8,
}

impl Signal {
    /// Constructed by generated code only: the identifier is the one of
    /// Generated_Config's signal table.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(id: u8) -> Signal {
        Signal { id }
    }

    /// Raises the Release_Signal: a Release_Source event of the bound
    /// Sporadic_Task at the current instant (R10.1 to R10.3), attributed to
    /// this Partition (R10.7). Never waits (PR-08). Returns whether the
    /// Kernel took the event (a refusal is API misuse and a fault of the
    /// Partition; the Kernel decides what the event does).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn raise(&self) -> bool {
        stubs::svc_raise(self.id)
    }
}

/// The Kernel time base in ticks (PR-26).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
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
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
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
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn read(addr: u32) -> u32 {
        hw::peripheral_read(addr)
    }

    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn write(addr: u32, value: u32) {
        hw::peripheral_write(addr, value)
    }
}

/// Writes to the debug console (semihosting). Not for Flight_Builds
/// without a debugger (the Link_Checker reports it there).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn debug(s: &[u8]) {
    #[cfg(not(feature = "quiet"))]
    hw::semihosting_write(s);
    #[cfg(feature = "quiet")]
    let _ = s;
}

/// A bounded busy loop of `N` iterations, for the demo Tasks (PR-16: the
/// bound is the constant argument, which the Profile_Lint derives).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn spin<const N: u32>() {
    let mut i = 0u32;
    while i < N {
        core::hint::black_box(i);
        i = i.wrapping_add(1);
    }
}
