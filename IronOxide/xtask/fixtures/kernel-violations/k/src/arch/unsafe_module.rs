//! The fixture's Kernel_Unsafe_Module: nothing here is reported, because
//! every construct cites a valid justification (PR-20/R6.4).
#![allow(unsafe_code)]

/// # Safety
///
/// `p` must be valid for writes of a `u32` and aligned.
// SAFETY (UJ-001): reviewed; the fixture has no harness infrastructure.
pub unsafe fn write(p: *mut u32, value: u32) {
    // SAFETY (UJ-001): the caller guarantees that `p` is valid and aligned.
    unsafe { p.write_volatile(value) }
}
