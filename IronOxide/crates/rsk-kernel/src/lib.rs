//! rsk Kernel (Component B): the portable Kernel logic, the Hardware_Model
//! specifications, and the Kernel_Arch for the Cortex-M4F.
//!
//! Unsafe-code confinement (PR-20, R6.3): `unsafe` code and assembly may appear
//! only in `arch::cm4::unsafe_module`, which together with the `rsk-entry`
//! crate forms the Kernel_Unsafe_Module. A forbidden lint cannot be re-allowed
//! in a child module, so `#![forbid(unsafe_code)]` cannot sit at this crate
//! root. Instead the root contains no unsafe code and denies `unsafe_code`, and
//! every module outside the Kernel_Unsafe_Module applies its own
//! `#![forbid(unsafe_code)]`. `cargo xtask verify` checks this placement.
#![no_std]
#![deny(unsafe_code)]
// Stable builds erase the ghost code of `verus!`, which leaves the imports
// and parameters that only specifications use unused (ORQ-08).
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports, unused_variables))]

// The `verus!` macro and the vstd prelude must be visible at the crate root
// for Verus to run (every module imports the prelude itself).
#[allow(unused_imports)]
use vstd::prelude::*;

pub mod arch;
pub mod hw_model;
pub mod logic;
pub mod profile_version;
