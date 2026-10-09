//! Application-like crate.
#![no_std]
#![forbid(unsafe_code)]

pub fn sum(a: u32, b: u32) -> Option<u32> {
    k::logic::add(a, b)
}
