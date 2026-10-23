//! PR-09: a Task body is a synchronous function. The Job wrapper binds the
//! body's result to `()`; an `async fn` returns a future: E0308.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    cx.state.lock(|s| *s += 1);
}

pub async fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
