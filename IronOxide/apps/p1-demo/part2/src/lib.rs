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

/// The Resource `stats` (declared `resource stats: crate::Stats`): DMA
/// transfers requested and accepted. Its declared invariant is
/// `ok <= total` (R46.4); `sampler` writes it and `filter` reads it.
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub total: u32,
    pub ok: u32,
}

impl Stats {
    /// The initial value; the App_Prover checks it satisfies the invariant
    /// from this postcondition.
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result.total@ == 0 && result.ok@ == 0))]
    pub const fn new() -> Stats {
        Stats { total: 0, ok: 0 }
    }

    /// Records one request and whether it was accepted (saturating).
    #[cfg_attr(creusot, rsk::creusot_std::macros::requires(self.ok@ <= self.total@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^self).ok@ <= (^self).total@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^self).total@ >= self.total@))]
    pub fn record(&mut self, accepted: bool) {
        self.total = self.total.saturating_add(1);
        if accepted {
            self.ok = self.ok.saturating_add(1);
        }
    }
}

/// y = y + (x - y) / 8 with x = 100.0, in `f32`: a first-order low-pass of
/// a constant input converges to it, which the safe state checks. Creusot's
/// floating-point support is partial (RN-10), so this function alone is
/// trusted under the App_Prover (task 17.1); the Task body is proven.
#[cfg_attr(creusot, rsk::creusot_std::macros::trusted)]
fn lowpass(y: &mut f32) {
    let x: f32 = 100.0;
    *y += (x - *y) * 0.125;
}

/// Task `filter`: steps the low-pass state and reads the transfer
/// statistics inside a Critical_Section it leaves unchanged.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::filter_pre(&cx)))]
pub fn filter(mut cx: rsk_ctx::Filter) {
    cx.y.with(|y| lowpass(y));
    #[cfg(creusot)]
    let read = #[rsk::creusot_std::macros::ensures(^s == *s && result == (*s).ok)] |s: &mut Stats| s.ok;
    #[cfg(not(creusot))]
    let read = |s: &mut Stats| s.ok;
    let ok = cx.stats.lock(read);
    // Known from the Resource invariant alone: no Job of this Task has seen
    // more accepted than requested transfers.
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(exists<t: Stats> rsk_spec::stats_inv(t) && t.ok == ok);
    let _ = ok;
    cx.count.with(|c| *c = c.saturating_add(1));
    // A byte array, not a byte-string literal: Creusot has no `b".."` yet.
    rsk::debug(&[b'F']);
}

/// Task `sampler`: fills its buffer (in the Partition's RAM, so
/// DMA-permitted), requests the transfer, and records the outcome in the
/// Resource `stats`, whose invariant the Critical_Section preserves.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::sampler_pre(&cx)))]
pub fn sampler(mut cx: rsk_ctx::Sampler) {
    #[cfg(creusot)]
    let before = rsk::creusot_std::macros::snapshot!(cx.count@);
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
    #[cfg(creusot)]
    let record = #[rsk::creusot_std::macros::requires(s.ok@ <= s.total@)]
    #[rsk::creusot_std::macros::ensures((^s).ok@ <= (^s).total@)]
    |s: &mut Stats| s.record(accepted);
    #[cfg(not(creusot))]
    let record = |s: &mut Stats| s.record(accepted);
    cx.stats.lock(record);
    cx.count.with(|c| *c = c.saturating_add(1));
    // Functional property (task 17.1): the Job advances the sampler count by one.
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(cx.count@@ >= (*before)@ && cx.count@@ <= (*before)@ + 1);
    rsk::debug(&[b'G']);
}
