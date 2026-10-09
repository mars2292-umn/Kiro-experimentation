//! Forbids unsafe code, but hides an `unsafe` block in a macro body, where
//! a syntax-tree scan would not look.
#![forbid(unsafe_code)]

macro_rules! verus {
    ($($body:tt)*) => { $($body)* };
}

verus! {
    pub fn read(p: *const u32) -> u32 {
        unsafe { p.read_volatile() }
    }
}
