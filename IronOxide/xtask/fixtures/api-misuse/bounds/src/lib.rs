//! PR-16 (R35.5): a Loop_Bound_Annotation whose bound is not a constant
//! expression, and one whose label names no loop, are compile errors of
//! the `#[rsk::bounds]` attribute naming PR-16.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

#[rsk::bounds('scan = n)]
pub fn alpha(mut cx: rsk_ctx::Alpha) {
    let n = 4;
    let mut i = 0;
    'scan: while i < n {
        i += 1;
    }
    cx.state.lock(|s| *s += i);
}

#[rsk::bounds('retry = 3)]
pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
