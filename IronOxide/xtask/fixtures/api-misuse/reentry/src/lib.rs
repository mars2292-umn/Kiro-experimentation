//! R8.10 / PR-07: a Job cannot enter the same Resource twice. `lock` takes
//! `&mut self`, so the nested lock is a second mutable borrow of `cx.state`
//! while the first is live: E0499.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    cx.state.lock(|s| {
        cx.state.lock(|t| *t = *s);
    });
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
