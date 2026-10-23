//! R8.6: the `&mut T` of a Critical_Section cannot outlive it. Returning it
//! from the closure needs a lifetime the closure's parameter does not have.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    let escaped: &mut u32 = cx.state.lock(|s| s);
    *escaped += 1;
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
