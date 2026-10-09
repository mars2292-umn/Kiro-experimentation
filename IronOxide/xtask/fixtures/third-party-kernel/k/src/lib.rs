//! A Kernel crate whose Target graph contains third-party crates.
#![no_std]
#![forbid(unsafe_code)]

pub use hashbrown::HashMap;

pub fn digits(n: u32) -> usize {
    itoa::Buffer::new().format(n).len()
}

pub fn has_zero(bytes: &[u8]) -> bool {
    memchr::memchr(0, bytes).is_some()
}
