//! Kernel-like crate: the root denies `unsafe_code` and contains no unsafe
//! code; every module outside `arch::unsafe_module` forbids it.
#![no_std]
#![deny(unsafe_code)]

pub mod arch;
pub mod logic;
