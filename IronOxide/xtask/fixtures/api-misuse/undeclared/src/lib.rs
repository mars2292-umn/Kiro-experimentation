//! R8.7 / PR-06: a Task sees only the Resources declared for it. `beta`
//! declares `state` only; `cx.other` is no field of its Context: E0609.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    cx.state.lock(|s| *s += 1);
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.other.lock(|o| *o = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
