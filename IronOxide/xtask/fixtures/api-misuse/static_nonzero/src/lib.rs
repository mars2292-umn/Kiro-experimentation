//! R14.5: a Partition's statics are zeroed memory at reset and after every
//! restart, whatever initial value they declare. A declared initial value
//! with a non-zero byte (`state alpha.level: u32 = 5;`) would never be
//! seen, so the generated cell refuses it at compile time
//! (`ResourceCell::for_static`).
#![no_std]
#![forbid(unsafe_code)]

api_static_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    cx.level.with(|l| *l = l.saturating_add(1));
}
