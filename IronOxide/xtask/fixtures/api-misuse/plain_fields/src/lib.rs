//! R42.1 (task 21.4): `Plain` cannot be derived for a struct with a field
//! that is not `Plain`: a reference (`&'static u32`) and a `bool` (not every
//! bit pattern is a value) are each a trait-bound error, E0277.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

#[derive(Clone, Copy, rsk::Plain)]
#[repr(C)]
pub struct Indirect {
    pub p: &'static u32,
}

#[derive(Clone, Copy, rsk::Plain)]
#[repr(C)]
pub struct Flagged {
    pub seq: u32,
    pub armed: bool,
}

pub fn alpha(cx: rsk_ctx::Alpha) {
    cx.ping.store(rsk_roles::ping_a::State::Empty);
}

pub fn beta(cx: rsk_ctx::Beta) {
    cx.ping.store(rsk_roles::ping_b::State::Empty);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
