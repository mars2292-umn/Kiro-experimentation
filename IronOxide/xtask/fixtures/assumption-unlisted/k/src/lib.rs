//! A Kernel crate with one listed and one unlisted assumption construct
//! (R50.2). The crate is only scanned, never compiled.
#![no_std]
#![forbid(unsafe_code)]

verus! {

/// Listed in docs/proof_assumptions.toml: accepted.
pub proof fn listed_lemma() {
    assume(false);
}

/// Not listed: the gate rejects it.
#[verifier::external_body]
pub fn unlisted_register_read() -> u32 {
    0
}

} // verus!
