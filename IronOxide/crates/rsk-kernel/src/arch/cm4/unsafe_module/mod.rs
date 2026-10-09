//! Kernel_Unsafe_Module (R6.3, PR-20): the only part of `rsk-kernel` in
//! which `unsafe` code, unsafe attributes, and assembly are permitted.
//!
//! Every `unsafe` block, `unsafe fn`, and `unsafe impl` carries a
//! justification identifier `UJ-nnn` registered in
//! `crates/rsk-kernel/unsafe_justifications.toml` (R6.4), linked to a Kani
//! harness in [`kani`], to a Verus lemma, or to a recorded review.
//!
//! Layout:
//!
//! - [`hw`]: the register-access functions with trusted specifications
//!   (R44.5). On the Target they are volatile accesses to the addresses of
//!   the Armv7-M system control space and the Target's peripherals; on the
//!   host and under Kani they act on the simulated register file of
//!   [`sim`], which mirrors the Hardware_Model steps (R45.4; the stub list
//!   is `docs/kani_stubs.toml`).
//! - [`stubs`]: the naked assembly entry points (dispatch stub, SVC entry,
//!   epilogue, timer and fault entries, trampoline), Target only.
//! - [`kani`]: the harnesses linked from the justification identifiers.
//!
//! This module applies `#![allow(unsafe_code)]`; the crate root denies it.
#![allow(unsafe_code)]

pub mod hw;
#[cfg(any(kani, test, not(all(target_arch = "arm", target_os = "none"))))]
pub mod sim;
#[cfg(all(target_arch = "arm", target_os = "none"))]
pub mod stubs;
#[cfg(not(all(target_arch = "arm", target_os = "none")))]
pub mod stubs_host;
#[cfg(not(all(target_arch = "arm", target_os = "none")))]
pub use stubs_host as stubs;
#[cfg(kani)]
pub mod kani;

use core::cell::UnsafeCell;

use super::ArchState;

/// The Kernel static of the system crate: interior mutability for a value
/// that only Kernel-level or PRIMASK-held code touches (DD-02).
pub struct KernelCell<K> {
    inner: UnsafeCell<K>,
}

// SAFETY (UJ-003): every access goes through `get`, which the Kernel calls
// only from exception handlers at the Kernel NVIC level or from stubs with
// PRIMASK held; Kernel code is non-reentrant by DD-02, so no two accesses
// overlap. Jobs run unprivileged under the MPU and cannot reach Kernel RAM
// (Property 12), so no other code holds a reference. Review only: the
// argument is about the whole execution model, which no harness can state.
unsafe impl<K> Sync for KernelCell<K> {}

impl<K> KernelCell<K> {
    pub const fn new(value: K) -> KernelCell<K> {
        KernelCell { inner: UnsafeCell::new(value) }
    }

    /// The Kernel value. Called only at the Kernel level or with PRIMASK
    /// held (DD-02).
    pub fn get(&self) -> &mut K {
        // SAFETY (UJ-004): as UJ-003; the returned reference never outlives
        // the handler that took it, and handlers do not nest at the same
        // level (DD-02), so no two live references exist at once.
        unsafe { &mut *self.inner.get() }
    }
}

/// Storage of a Resource (design.md "Application API"): accessed only
/// inside a Critical_Section through `rsk::Shared::lock`, which holds the
/// Resource under the ceiling protocol.
pub struct ResourceCell<T> {
    inner: UnsafeCell<T>,
}

// SAFETY (UJ-050): the cell is accessed only inside a Critical_Section on
// its Resource (`with`, called by `rsk::Shared::lock` after the lock SVC
// succeeded), and the SRP makes Critical_Sections on one Resource mutually
// exclusive (Property 4, R8.3); the closure cannot re-enter (R8.10) or let
// the reference escape (R8.6). `T: Send` because the value moves between
// Jobs of different Priorities (RTIC's rule, RN-06). Review only: the
// argument is the Kernel proof plus the type-system rules.
unsafe impl<T: Send> Sync for ResourceCell<T> {}

impl<T> ResourceCell<T> {
    pub const fn new(value: T) -> ResourceCell<T> {
        ResourceCell { inner: UnsafeCell::new(value) }
    }

    /// Runs `f` on the value. Called only by `rsk::Shared::lock` while the
    /// Resource is held.
    pub fn with<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY (UJ-051): as UJ-050; the reference lives only inside `f`.
        let value = unsafe { &mut *self.inner.get() };
        f(value)
    }

    /// A copy of the value for Kernel-level readers after every Job has
    /// ended (the safe-state report).
    pub fn peek(&self) -> T
    where
        T: Copy,
    {
        // SAFETY (UJ-052): called at the Kernel level once no Job is in
        // progress (safe state), so no Critical_Section is open.
        unsafe { *self.inner.get() }
    }

    /// Per-Task state that only one Task touches (PR-25: statically
    /// declared per-Task state): no lock is needed because no other Job
    /// accesses it. The Generator emits these accesses for declared
    /// per-Task state only; the demo uses them for its counters.
    #[inline(always)]
    pub fn with_unprivileged<R>(&self, f: impl FnOnce(&mut T) -> R) -> R {
        // SAFETY (UJ-053): the cell is declared per-Task state (PR-25),
        // accessed by one Task's Jobs only, which never overlap (one Job
        // per Task in progress, Property 2); the reference lives inside `f`.
        let value = unsafe { &mut *self.inner.get() };
        f(value)
    }

    /// A copy of per-Task state or of a value another Task only ever
    /// reads as a hint (the demo's blocking indicator).
    #[inline(always)]
    pub fn peek_unprivileged(&self) -> T
    where
        T: Copy,
    {
        // SAFETY (UJ-054): a read of a `Copy` value; a torn read is
        // impossible for word-sized values on Armv7-M, and the caller uses
        // the value only as a hint.
        unsafe { *self.inner.get() }
    }
}

/// The Kernel_Arch state (level records, stack floors, switch point).
struct ArchCell(UnsafeCell<ArchState>);

// SAFETY (UJ-005): same argument as UJ-003; the state is touched only by
// Kernel-level handlers and PRIMASK-held stub code.
unsafe impl Sync for ArchCell {}

static ARCH_STATE: ArchCell = ArchCell(UnsafeCell::new(ArchState::new()));

/// The Kernel_Arch state. Called only at the Kernel level or with PRIMASK held.
pub fn arch_state() -> &'static mut ArchState {
    // SAFETY (UJ-006): as UJ-004.
    unsafe { &mut *ARCH_STATE.0.get() }
}

/// Extends a 32-bit counter reading to the 64-bit time base (HW-12,
/// DD-05): a reading below the previous one means the counter wrapped.
/// Readings happen at least once per housekeeping period, which is far
/// shorter than a wrap.
pub fn extend(low: u32) -> u64 {
    let s = arch_state();
    if low < s.time_last_lo {
        s.time_hi = s.time_hi.wrapping_add(1);
    }
    s.time_last_lo = low;
    ((s.time_hi as u64) << 32) | low as u64
}

/// The initial exception frame of a Job (HW-06): R0-R3 and R12 zero, LR
/// the job-end trampoline, PC the Job entry, xPSR with the Thumb bit.
pub fn frame_words(entry: u32, trampoline: u32) -> [u32; 8] {
    [0, 0, 0, 0, 0, trampoline | 1, entry & !1, super::XPSR_THUMB]
}

/// Writes the initial frame at `sp` (8-byte aligned, inside the Partition's
/// stack region: `irq_entry` checks `sp >= stack bottom` before calling).
pub fn build_frame(sp: u32, entry: u32, trampoline: u32) {
    let words = frame_words(entry, trampoline);
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-007): `sp` is 8-byte aligned and `sp..sp+32` lies inside
        // the Partition's stack region, which no other context uses at this
        // instant (the stack floor was just lowered under PRIMASK); the
        // region is ordinary RAM mapped read-write to the Kernel.
        let frame = unsafe { &mut *(sp as usize as *mut [u32; 8]) };
        *frame = words;
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        sim::record_frame(sp, words);
    }
}

/// The SVC number of the instruction before the stacked return address
/// (Thumb `SVC #imm8`: the low byte of the halfword at PC - 2).
pub fn svc_number(stacked_pc: u32) -> u8 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-008): `stacked_pc` is the return address stacked by an
        // SVC exception taken from Thumb code (HW-06), so `stacked_pc - 2`
        // is the address of the SVC instruction, in executable memory.
        let halfword = unsafe { core::ptr::read_volatile((stacked_pc - 2) as usize as *const u16) };
        (halfword & 0xFF) as u8
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        sim::svc_number_at(stacked_pc)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_layout_matches_hw06() {
        let f = frame_words(0x0000_1000, 0x0000_2000);
        assert_eq!(f[5], 0x0000_2001, "LR carries the Thumb bit");
        assert_eq!(f[6], 0x0000_1000, "PC has bit 0 clear");
        assert_eq!(f[7], 1 << 24, "xPSR Thumb bit");
        assert_eq!(&f[0..5], &[0, 0, 0, 0, 0]);
    }
}
