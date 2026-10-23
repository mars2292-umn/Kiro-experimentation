//! Bodies that follow every rule: this crate compiles.
#![no_std]
#![forbid(unsafe_code)]

use rsk::Mutex;

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    // Through the `Mutex` trait and through the inherent method.
    Mutex::lock(&mut cx.state, |s| *s += 1);
    let n = cx.other.lock(|o| {
        *o += 1;
        *o
    });
    if n > 3 {
        let _ = cx.tick.raise();
    }
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
