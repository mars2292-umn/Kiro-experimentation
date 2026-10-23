//! P1 demo Partition 2 (`part2`, level B; task 14, R3.7): `filter`
//! (Priority 3, every 25 ms) keeps a floating-point state across Jobs, so
//! its Jobs carry an active FP context that the Kernel stacks and restores
//! (R11, PR-31; the `fpu` flag of the declaration), and `sampler`
//! (Priority 1, every 100 ms) requests an EasyDMA transfer of a buffer in
//! its own RAM through `rsk::dma_request` (DD-08, R17): the Kernel checks
//! the Partition's ownership of the channel and the buffer's permission and
//! programs the channel's registers (on QEMU, the GPIO0 stand-in).
#![no_std]
#![forbid(unsafe_code)]

use rsk::ResourceCell;

p1_demo_decl::rsk_partition_part2!();

/// Demo counters (plain statics of the Partition, read by the safe state).
pub static COUNT_FILTER: ResourceCell<u32> = ResourceCell::new(0);
pub static COUNT_SAMPLER: ResourceCell<u32> = ResourceCell::new(0);
/// Accepted EasyDMA requests.
pub static COUNT_DMA_OK: ResourceCell<u32> = ResourceCell::new(0);
/// The filter state in single precision: a first-order low-pass of a
/// constant input converges to it, which the safe state checks.
pub static FILTER_STATE: ResourceCell<f32> = ResourceCell::new(0.0);
/// The sampler's transfer buffer, in the Partition's RAM (DMA-permitted).
static SAMPLE_BUF: ResourceCell<[u8; 16]> = ResourceCell::new([0; 16]);

/// The channel index of CMSDK_GPIO0's stand-in channel in the system's DMA
/// table (the only EasyDMA channel of the declaration).
const DMA_CHANNEL: u8 = 0;

/// Task `filter`: y = y + (x - y) / 8 with x = 100.0, in `f32`.
pub fn filter(_cx: rsk_ctx::Filter) {
    FILTER_STATE.with_unprivileged(|y| {
        let x: f32 = 100.0;
        *y += (x - *y) * 0.125;
    });
    COUNT_FILTER.with_unprivileged(|c| *c += 1);
    rsk::debug(b"F");
}

/// Task `sampler`: fills its buffer and requests the transfer.
pub fn sampler(_cx: rsk_ctx::Sampler) {
    let accepted = SAMPLE_BUF.with_unprivileged(|buf| {
        let n = COUNT_SAMPLER.peek_unprivileged() as u8;
        buf[0] = n;
        rsk::dma_request(DMA_CHANNEL, &buf[..])
    });
    if accepted {
        COUNT_DMA_OK.with_unprivileged(|c| *c += 1);
    }
    COUNT_SAMPLER.with_unprivileged(|c| *c += 1);
    rsk::debug(b"G");
}
