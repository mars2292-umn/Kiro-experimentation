//! R42.1 (task 21.4): `Plain` cannot be derived for an enum (not every bit
//! pattern of its size is a value) nor for a struct without a `#[repr(C)]`
//! or `#[repr(transparent)]` layout; the derive names the rule.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

#[derive(Clone, Copy, rsk::Plain)]
#[repr(u8)]
pub enum Mode {
    Idle,
    Armed,
}

#[derive(Clone, Copy, rsk::Plain)]
pub struct Unlaid {
    pub seq: u32,
}

pub fn alpha(cx: rsk_ctx::Alpha) {
    cx.ping.store(rsk_roles::ping_a::State::Empty);
}

pub fn beta(cx: rsk_ctx::Beta) {
    cx.ping.store(rsk_roles::ping_b::State::Empty);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
