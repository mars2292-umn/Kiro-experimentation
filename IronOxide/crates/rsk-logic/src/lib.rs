//! rsk Kernel proof core (Component B, design.md "Kernel logic and
//! Kernel_Arch"): the portable Kernel logic and the Hardware_Model
//! specifications, every item inside `verus!`.
//!
//! This crate contains no `unsafe` code and touches no hardware; the
//! Kernel_Arch (`rsk-kernel::arch`) binds it to the Cortex-M4F and
//! re-exports it as `rsk_kernel::logic` and `rsk_kernel::hw_model`, so the
//! two crates together are the Kernel of PR-20 and R6.1. The split keeps
//! the logic proofs and the Kernel_Arch proofs in separate Verus crates,
//! so neither side's SMT queries see the other's declarations.
#![no_std]
#![forbid(unsafe_code)]
// Stable builds erase the ghost code of `verus!`, which leaves the imports
// and parameters that only specifications use unused (ORQ-08).
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports, unused_variables))]

// The `verus!` macro and the vstd prelude must be visible at the crate root
// for Verus to run (every module imports the prelude itself).
#[allow(unused_imports)]
use vstd::prelude::*;

pub mod hw_model;
pub mod logic;
