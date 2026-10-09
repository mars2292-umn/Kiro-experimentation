//! The fixture's Kernel_Unsafe_Module: `unsafe` code and assembly are
//! allowed here and nowhere else. Every construct cites a justification
//! identifier of `k/unsafe_justifications.toml` (PR-20/R6.4).
#![allow(unsafe_code)]

/// Reads a word through a raw pointer.
///
/// # Safety
///
/// `p` must be valid for reads of a `u32` and aligned.
// SAFETY (UJ-001): validity and alignment are the caller's precondition; the
// Kani harness `uj_001_read_valid` checks every aligned in-bounds pointer.
pub unsafe fn read(p: *const u32) -> u32 {
    // SAFETY (UJ-002): the caller guarantees that `p` is valid and aligned.
    unsafe { p.read_volatile() }
}

#[cfg(target_arch = "arm")]
pub fn nop() {
    // UJ-003: `nop` touches no memory and no register (review only).
    unsafe { core::arch::asm!("nop") }
}

/// Kani harnesses for this module's justification identifiers. They are not
/// compiled by a Flight_Build, so their own `unsafe` calls need no identifier.
#[cfg(kani)]
mod harness {
    #[kani::proof]
    fn uj_001_read_valid() {
        let word: u32 = kani::any();
        let got = unsafe { super::read(&word as *const u32) };
        assert_eq!(got, word);
    }
}
