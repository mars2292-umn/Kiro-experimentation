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
#[cfg(not(feature = "quiet"))]
use rsk_kernel::arch::cm4::unsafe_module::hw;
use rsk_kernel::arch::cm4::unsafe_module::stubs;

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

/// The Kernel-mediated write of one register of an EasyDMA-capable
/// peripheral (DD-08, R17.3; task 19.2): `offset` is the register's offset
/// in the 4 KiB window of the peripheral of `channel` (a channel index of
/// the calling Partition's peripherals, as for [`dma_request`]). The
/// Kernel writes it only when the channel is the Partition's own and the
/// register is none of the pointer, count, or list-mode registers of the
/// peripheral's channels (those are [`dma_request`]'s); otherwise the
/// request is refused before any register is written, the refusal is a
/// spatial violation of the Partition (R17.2), and `false` is returned.
/// A Partition starts and stops its transfers, configures the peripheral,
/// and clears its events this way, since its MPU view maps the window
/// read-only.
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn dma_write(channel: u8, offset: u32, value: u32) -> bool {
    stubs::svc_dma_write(channel, offset, value)
}

/// Register access to the calling Partition's peripherals (R16.2): a
/// read or write of a word at `addr`. The MPU view confines the access to
/// the Partition's peripheral windows; any other address is a MemManage
/// fault of the Partition (R16.4). EasyDMA peripherals are read-only
/// here; their transfers go through [`dma_request`] and their other
/// registers through [`dma_write`] (DD-08).
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

/// The HIL differential tests' Task-side probes (task 3.3, R49.3;
/// feature `hw-dt`, HIL builds only): the register patterns a Job keeps
/// across preemptions (HW-06, HW-10, R7.5), the FPSCR a Job starts with
/// (HW-10), an instruction fetch at an address (the isolation suite's
/// fetch attempts, HW-09), and a DMA request by address (the isolation
/// suite's refused buffers, R17.2). Each is inlined into the Partition's
/// code and runs unprivileged under its MPU view.
#[cfg(feature = "hw-dt")]
pub mod hw_dt {
    use rsk_kernel::arch::cm4::unsafe_module::dt;

    /// Bit i set for the i-th of R2 to R5, R8, R10, R12 that changed over
    /// `iterations` of a spin (0: every register preserved).
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn register_patterns(iterations: u32) -> u32 {
        dt::register_patterns(iterations)
    }

    /// As [`register_patterns`], with S0 to S31 (bits 8 to 23: S0 to S15;
    /// bits 24 to 27: how many of S16 to S31 changed). FPU Tasks only.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn register_patterns_fp(iterations: u32) -> u32 {
        dt::register_patterns_fp(iterations)
    }

    /// FPSCR as the Job finds it. FPU Tasks only.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fpscr() -> u32 {
        dt::fpscr_read()
    }

    /// An instruction fetch at `addr`: a call, which the MPU refuses
    /// (IACCVIOL) when the address is outside the Partition's view.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fetch(addr: u32) {
        dt::fetch(addr)
    }

    /// An EasyDMA request over `len` bytes at `ptr` (any address: the
    /// Kernel decides, R17.1, R17.2), as [`super::dma_request`] over a slice.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn dma_request_raw(channel: u8, ptr: u32, len: u32) -> bool {
        super::stubs::svc_dma(channel, ptr, len)
    }
}

/// The access matrix of the HIL isolation suite (task 7.4, R16.8;
/// feature `hw-dt`, HIL builds only): for the executing Partition, one
/// access attempt per protected class of every other Partition and of
/// the Kernel, enumerated from the generated memory map
/// (`rsk_target::layout`) in a fixed order the rig mirrors
/// (`rsk-hil::isolation`). Against each other Partition `q`, in order:
/// a read, a write, and a fetch of its code; a read and a write of its
/// data (above its stack); a write into its stack; a read and a write of
/// its first peripheral window (skipped when it owns none). Then against
/// the Kernel: a read and a write of its RAM, a read and a fetch of its
/// flash, a read and a write of the Private Peripheral Bus (the NVIC), a
/// read and a write of the first Kernel-owned peripheral's window after
/// the PPB (R16.2; skipped when the Target has none), and a write to the
/// word below this Partition's own stack (R16.5).
/// Every attempt the MPU refuses is a MemManage fault of the Partition
/// (R16.4), the PPB attempts a BusFault (unprivileged, HW-08, HW-11);
/// an attempt that returns is recorded by the caller as a mismatch.
/// Endpoint storage has no attempt: no Endpoint exists before Phase 4.
#[cfg(feature = "hw-dt")]
pub mod matrix {
    /// The memory map, from `rsk_target::layout`.
    pub struct Layout<'a> {
        pub this: usize,
        pub code: &'a [(u32, u32)],
        pub ram: &'a [(u32, u32)],
        pub stack: &'a [u32],
        pub windows: &'a [[(u32, u32); 5]],
        pub kernel_flash: (u32, u32),
        pub kernel_ram: (u32, u32),
        /// The Kernel's peripheral intervals, the PPB first (R16.2).
        pub kernel_windows: &'a [(u32, u32)],
    }

    /// Attempts per other Partition and against the Kernel.
    pub const PER_PARTITION: usize = 8;
    pub const KERNEL: usize = 9;
    /// The private peripheral bus address attempted (NVIC_ISER0).
    pub const PPB: u32 = 0xE000_E100;

    /// What an attempt did. No `PartialEq`: under the App_Prover a derived
    /// equality needs Creusot's `DeepModel` (the finding of 2026-10-24);
    /// callers match on the variant.
    #[derive(Clone, Copy)]
    pub enum Outcome {
        /// No address for the class (the Partition owns no peripheral).
        Skipped,
        /// The access returned: the MPU granted it (a mismatch).
        Survived,
    }

    /// A map with one entry per Partition in every table and `this` among
    /// them (App_Prover, task 17.1: what `target` and `perform` require).
    #[cfg(feature = "creusot")]
    #[allow(unused_imports)] // used by the logic function below under creusot-rustc only
    use creusot_std::{macros::{logic, pearlite}, model::View};

    #[cfg(feature = "creusot")]
    #[logic(open)]
    pub fn wf(l: &Layout<'_>) -> bool {
        pearlite! {
            l.code@.len() >= 1 && l.ram@.len() == l.code@.len() && l.stack@.len() == l.code@.len()
                && l.windows@.len() == l.code@.len() && l.this@ < l.code@.len()
        }
    }

    /// The number of attempts of this Partition.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result@ == (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn count(l: &Layout<'_>) -> usize {
        (l.code.len() - 1) * PER_PARTITION + KERNEL
    }

    /// The address and class of attempt `i`: `(address, class)` with the
    /// class 0 read, 1 write, 2 fetch; `None` when skipped.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(i@ < (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn target(l: &Layout<'_>, i: usize) -> Option<(u32, u8)> {
        let others = l.code.len() - 1;
        if i < others * PER_PARTITION {
            let mut q = i / PER_PARTITION;
            if q >= l.this {
                q += 1;
            }
            let k = i % PER_PARTITION;
            let code = l.code[q].0 + 0x100;
            let data = l.ram[q].0 + l.stack[q] + 0x100;
            let stack = l.ram[q].0 + 0x10;
            let window = l.windows[q][0];
            match k {
                0 => Some((code, 0)),
                1 => Some((code, 1)),
                2 => Some((code, 2)),
                3 => Some((data, 0)),
                4 => Some((data, 1)),
                5 => Some((stack, 1)),
                6 => if window.1 == 0 { None } else { Some((window.0, 0)) },
                _ => if window.1 == 0 { None } else { Some((window.0, 1)) },
            }
        } else {
            match i - others * PER_PARTITION {
                0 => Some((l.kernel_ram.0 + 0x100, 0)),
                1 => Some((l.kernel_ram.0 + 0x100, 1)),
                2 => Some((l.kernel_flash.0 + 0x100, 0)),
                3 => Some((l.kernel_flash.0 + 0x100, 2)),
                4 => Some((PPB, 0)),
                5 => Some((PPB, 1)),
                6 => l.kernel_windows.get(1).map(|w| (w.0 + 0x100, 0)),
                7 => l.kernel_windows.get(1).map(|w| (w.0 + 0x100, 1)),
                8 => Some((l.ram[l.this].0.wrapping_sub(4), 1)),
                _ => None,
            }
        }
    }

    /// Performs attempt `i`. Returns only when the access was skipped or
    /// granted: a refused access ends the Job (the Partition's fault
    /// response) and never returns here.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(i@ < (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn perform(l: &Layout<'_>, i: usize) -> Outcome {
        match target(l, i) {
            None => Outcome::Skipped,
            Some((a, 0)) => {
                let _ = super::mmio::read(a);
                Outcome::Survived
            }
            Some((a, 1)) => {
                super::mmio::write(a, 0);
                Outcome::Survived
            }
            Some((a, _)) => {
                super::hw_dt::fetch(a);
                Outcome::Survived
            }
        }
    }
}
