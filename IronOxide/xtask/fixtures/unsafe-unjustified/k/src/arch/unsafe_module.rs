//! Four violations of PR-20/R6.4, one per construct.
#![allow(unsafe_code)]

pub struct Cell(u32);

pub fn no_marker(p: *const u32) -> u32 {
    unsafe { p.read_volatile() }
}

// UJ-009: this identifier is not in the register.
pub unsafe fn unregistered(p: *const u32) -> u32 {
    p.read_volatile()
}

// UJ-002: links to a Kani harness that does not exist.
unsafe impl Sync for Cell {}

// SAFETY (UJ-003): a review without the required statement.
pub unsafe fn review_without_why() {}
