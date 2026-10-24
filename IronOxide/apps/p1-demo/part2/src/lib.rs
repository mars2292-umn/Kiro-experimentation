//! P1 demo Partition 2 (`part2`, level B; task 14, R3.7): `filter`
//! (Priority 3, every 25 ms) keeps a floating-point state across Jobs, so
//! its Jobs carry an active FP context that the Kernel stacks and restores
//! (R11, PR-31; the `fpu` flag of the declaration), and `sampler`
//! (Priority 1, every 100 ms) requests an EasyDMA transfer of a buffer in
//! its own RAM through `rsk::dma_request` (DD-08, R17): the Kernel checks
//! the Partition's ownership of the channel and the buffer's permission and
//! programs the channel's registers (on QEMU, the GPIO0 stand-in). Every
//! value that outlives a Job is declared per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part2!();

/// The channel index of CMSDK_GPIO0's stand-in channel in the system's DMA
/// table (the only EasyDMA channel of the declaration).
const DMA_CHANNEL: u8 = 0;

/// Task `filter`: y = y + (x - y) / 8 with x = 100.0, in `f32`; a
/// first-order low-pass of a constant input converges to it, which the
/// safe state checks.
// Creusot's floating-point support is partial (RN-10), so this body is
// trusted under the App_Prover (task 17.1); `sampler` below is proven.
#[cfg_attr(creusot, rsk::creusot_std::macros::trusted)]
pub fn filter(mut cx: rsk_ctx::Filter) {
    cx.y.with(|y| {
        let x: f32 = 100.0;
        *y += (x - *y) * 0.125;
    });
    cx.count.with(|c| *c = c.saturating_add(1));
    // A byte array, not a byte-string literal: Creusot has no `b".."` yet.
    rsk::debug(&[b'F']);
}

/// Task `sampler`: fills its buffer (in the Partition's RAM, so
/// DMA-permitted) and requests the transfer.
pub fn sampler(mut cx: rsk_ctx::Sampler) {
    // The low byte of the counter: a reduction before the cast, so that the
    // cast is lossless (the App_Prover checks every `as`).
    let n = (cx.count.get() % 256) as u8;
    let accepted = cx.buf.with(|buf| {
        buf[0] = n;
        // An unsizing coercion, not `Index<RangeFull>`, which Creusot has no
        // contract for on arrays.
        let bytes: &[u8] = &*buf;
        rsk::dma_request(DMA_CHANNEL, bytes)
    });
    if accepted {
        cx.dma_ok.with(|c| *c = c.saturating_add(1));
    }
    cx.count.with(|c| *c = c.saturating_add(1));
    rsk::debug(&[b'G']);
}
