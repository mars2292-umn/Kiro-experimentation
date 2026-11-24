//! The Partition `comms` of the PACEMAKER experiment (docs/pacemaker,
//! task 15.3; design PD-11): the Task `link`, which exchanges one frame of
//! 32 bytes with the DCM in every period of 50 ms over SPIM1.
//!
//! What is answered is `proto::answer`; this file is the transfer. A Job
//! of `link`:
//!
//! 1. takes back the two buffers of the transfer the Job before started
//!    (`rsk::DmaBuf::reclaim`; the transfer ended long before, 32 bytes at
//!    500 kbps take 0.5 ms), and decodes what was received if the
//!    transfer ended;
//! 2. writes the answer into the transmit buffer;
//! 3. lends both buffers to SPIM1 and starts the transfer, which runs
//!    after the Job has ended.
//!
//! SPIM1's registers are written through the Kernel (`rsk::dma_write`,
//! DD-08), its pointers and counts through `rsk::DmaBuf::lend`. The pins
//! (P0.28 clock, P0.29 out, P0.30 in) are named here in SPIM1's pin
//! selection; their direction is set by `therapy`, the owner of the GPIO
//! port (design, Table 1). There is no chip select: the SPIM has none and
//! the port is not this Partition's, so the bridge frames a transfer by
//! the clock's silence (`tools/pm-bridge`).
//!
//! On the QEMU Target the declaration has no `comms`, and this crate is
//! `proto` alone.
#![no_std]
#![forbid(unsafe_code)]

pub mod proto;

#[cfg(feature = "nrf52840")]
pacemaker_decl::rsk_partition_comms!();

pub use proto::Link;

/// The counts `link` keeps for the rig's probe, beside `proto::Link`'s.
#[cfg(feature = "nrf52840")]
mod spim {
    /// The window of SPIM1 (for reads; the Target table has the same).
    pub const SPIM1: u32 = 0x4000_4000;
    pub const TASKS_START: u32 = 0x010;
    pub const EVENTS_END: u32 = 0x118;
    pub const ENABLE: u32 = 0x500;
    pub const PSEL_SCK: u32 = 0x508;
    pub const PSEL_MOSI: u32 = 0x50C;
    pub const PSEL_MISO: u32 = 0x510;
    pub const FREQUENCY: u32 = 0x524;
    pub const CONFIG: u32 = 0x554;
    pub const ORC: u32 = 0x5C0;
    pub const SCK_PIN: u32 = 28;
    pub const MOSI_PIN: u32 = 29;
    pub const MISO_PIN: u32 = 30;
    pub const FREQUENCY_K500: u32 = 0x0800_0000;
    pub const ENABLE_SPIM: u32 = 7;
    /// SPIM1's channels in the system's DMA table: RXD, then TXD.
    pub const RX: usize = 0;
    pub const TX: usize = 1;
}

/// Takes the transfer kept in `held` back into `buf`. Whether the buffer's
/// bytes can be reached afterwards: nothing was lent, or the peripheral
/// reported the stop. A transfer that did not stop is kept for the next Job.
#[cfg(feature = "nrf52840")]
fn take_back(buf: &mut rsk::PerTask<'static, rsk::DmaBuf<32>>, held: &mut rsk::PerTask<'static, rsk::DmaHeld>) -> bool {
    let transfer = held.with(|h| h.take());
    let (free, again) = buf.with(|b| match transfer {
        None => (!b.is_lent(), None),
        Some(t) => match b.reclaim(t) {
            Ok(()) => (true, None),
            Err(rsk::DmaReclaimError::NotStopped(t)) | Err(rsk::DmaReclaimError::NotThisBuffer(t)) => (false, Some(t)),
        },
    });
    held.with(|h| h.put(again));
    free
}

/// The Task `link` (Table 2: periodic, 50 ms, Priority 2).
#[cfg(feature = "nrf52840")]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::link_pre(&cx)))]
pub fn link(mut cx: rsk_ctx::Link) {
    use spim::*;
    let ch = rsk_target::DMA[RX].index();
    // The first Job of an incarnation: the peripheral as this Task needs
    // it, whatever an incarnation before left running.
    let first = cx.started.with(|s| {
        let first = *s == 0;
        *s = 1;
        first
    });
    if first {
        let _ = rsk::dma_write(ch, ENABLE, 0);
        let _ = rsk::dma_write(ch, PSEL_SCK, SCK_PIN);
        let _ = rsk::dma_write(ch, PSEL_MOSI, MOSI_PIN);
        let _ = rsk::dma_write(ch, PSEL_MISO, MISO_PIN);
        let _ = rsk::dma_write(ch, FREQUENCY, FREQUENCY_K500);
        let _ = rsk::dma_write(ch, CONFIG, 0);
        let _ = rsk::dma_write(ch, ORC, 0);
    }
    // 1. The transfer of the Job before. Its end is read before the
    // buffers are taken back: taking back clears the event.
    //
    // A transfer that has ended is taken back with the peripheral
    // disabled. `reclaim` asks for a stop and waits for the STOPPED event,
    // and an SPIM that is enabled and idle gives none (measured on the
    // board, 2026-11-24: findings.md); disabled, it counts as stopped. A
    // transfer that has not ended is stopped with the peripheral enabled,
    // and answers.
    let ended = !first && rsk::mmio::read(SPIM1 + EVENTS_END) != 0;
    if ended {
        let _ = rsk::dma_write(ch, ENABLE, 0);
    }
    let rx_free = take_back(&mut cx.rx, &mut cx.rx_held);
    let tx_free = take_back(&mut cx.tx, &mut cx.tx_held);
    if !rx_free || !tx_free {
        // A transfer that does not stop: nothing is sent in this period.
        cx.stuck.with(|n| *n = n.saturating_add(1));
        return;
    }
    let got = if ended { cx.rx.with(|b| b.bytes().and_then(|bytes| pm_core::frame::decode(bytes))) } else { None };
    // 2. The answer.
    let frame = cx.proto.with(|l| proto::answer(l, got.as_ref()));
    let out = pm_core::frame::encode(&frame);
    cx.tx.with(|b| {
        if let Some(bytes) = b.bytes() {
            *bytes = out;
        }
    });
    // 3. The next transfer.
    let _ = rsk::dma_write(ch, ENABLE, ENABLE_SPIM);
    let tx = cx.tx.with(|b| b.lend(rsk_target::DMA[TX]));
    let rx = cx.rx.with(|b| b.lend(rsk_target::DMA[RX]));
    let lent = tx.is_some() && rx.is_some();
    cx.tx_held.with(|h| h.put(tx));
    cx.rx_held.with(|h| h.put(rx));
    if lent {
        let _ = rsk::dma_write(ch, EVENTS_END, 0);
        let _ = rsk::dma_write(ch, TASKS_START, 1);
        cx.transfers.with(|n| *n = n.wrapping_add(1));
    } else {
        cx.stuck.with(|n| *n = n.saturating_add(1));
    }
}
