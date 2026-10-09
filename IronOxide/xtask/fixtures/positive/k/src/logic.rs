//! Safe logic written inside a macro body, as the Kernel logic will be
//! inside `verus!`.
#![forbid(unsafe_code)]

macro_rules! verus {
    ($($body:tt)*) => { $($body)* };
}

verus! {
    pub fn add(a: u32, b: u32) -> Option<u32> {
        a.checked_add(b)
    }

    /// Stands in for the lemma that `UJ-002` links to.
    pub fn uj_002_lemma_read_volatile() {}
}
