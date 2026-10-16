//! A host crate that opts into Verus. The crate is only scanned, never
//! compiled.
#![forbid(unsafe_code)]

verus! {

/// Not listed: the gate rejects it.
pub proof fn helper() {
    admit();
}

} // verus!
