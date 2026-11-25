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
//! 2. takes `control`'s status from the Endpoint `status`, writes the
//!    answer into the transmit buffer, and hands the Endpoint `program`
//!    the message for a frame it heard (task 16.1);
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

/// The validation function of the channel of `request` toward `link`,
/// which carries nothing: no message is one.
pub fn validate_nothing(_m: &[u32; pm_core::record::REQUEST_WORDS]) -> bool {
    false
}

/// The validation function of the Endpoint `records` (R42.4: from level D
/// to level C; PM22.4): exactly the records of `pm_core::record`.
pub fn validate_records(m: &[u32; pm_core::record::DATA_WORDS]) -> bool {
    pm_core::record::admitted(m)
}

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

/// The misbehaviour of task 17.2 (feature `misbehave`): from 10 s into the
/// run every Job of `link` faults, overruns its Budget, sends a parameter
/// set the validation function refuses, or floods the Endpoint `program`,
/// as the first letter of `PM_LINK_FAULT` says (`f`, `o`, `i`, `m`).
#[cfg(feature = "misbehave")]
mod misbehave {
    pub const KIND: u8 = env!("PM_LINK_FAULT").as_bytes()[0];
    pub const FROM_MS: u64 = 10_000;
    /// The messages of a flood: 16 times the Endpoint's capacity.
    pub const FLOOD: usize = 64;

    /// A busy wait of `ms` milliseconds by the time base, bounded in
    /// iterations too (PR-16).
    #[allow(unused_labels)]
    pub fn spin_ms(ms: u64) -> u32 {
        let start = rsk::now();
        let mut i = 0u32;
        'spin: while rsk::now().wrapping_sub(start) < super::rsk_target::ms(ms) && i < 100_000_000 {
            i += 1;
        }
        i
    }
}

/// The messages `link` takes from the Endpoint `status` in a Job (its `ops`).
#[cfg(feature = "nrf52840")]
const STATUS_OPS: usize = 2;

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
    #[cfg(feature = "misbehave")]
    if rsk::now() >= rsk_target::ms(misbehave::FROM_MS) {
        if misbehave::KIND == b'f' {
            // A write into the Kernel's RAM: a fault of this Partition.
            rsk::mmio::write(0x2000_0000, 1);
        } else if misbehave::KIND == b'o' {
            // 3 ms on a Budget of 2 ms.
            let _ = misbehave::spin_ms(3);
        } else if misbehave::KIND == b'i' {
            // A parameter set the validation function refuses (an LRL of
            // 31 ppm), as a `link` that did not check it would send: the
            // Kernel reports it against this Partition (R42.4).
            let mut set = pm_core::NOMINAL_WORDS;
            set[pm_core::word::LRL] = 31;
            let _ = cx.program.send(&pm_core::wire::message_with_set(pm_core::wire::code::PROGRAM, 0, &set));
        } else if misbehave::KIND == b'r' {
            // `record`: it is `diag` that misbehaves (task 22.4); `link`
            // goes on as ever.
        } else {
            // Messages the validation function admits, far more than the
            // Endpoint holds or `control` takes.
            let m = pm_core::wire::message(pm_core::wire::code::HEARD, 0);
            let mut k = 0;
            while k < misbehave::FLOOD {
                let _ = cx.program.send(&m);
                k += 1;
            }
        }
    }
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
    // 2. `control`'s status, when one has come (at most `STATUS_OPS` are
    // taken; the last is the one kept).
    let mut k = 0;
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(STATUS_OPS@ - k@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(k@ <= STATUS_OPS@))]
    while k < STATUS_OPS {
        k += 1;
        match cx.status.recv() {
            Ok(Some(w)) => cx.proto.with(|l| proto::status_in(l, &w)),
            Ok(None) => k = STATUS_OPS,
            Err(_) => {
                cx.proto.with(|l| proto::peer_changed(l));
                k = STATUS_OPS;
            }
        }
    }
    // 3. The answer, and the message to `control` for a frame heard.
    let frame = cx.proto.with(|l| proto::answer(l, got.as_ref()));
    if let Some(m) = cx.proto.with(|l| proto::take_forward(l)) {
        match cx.program.send(&m) {
            Ok(()) => {}
            Err(rsk::SendError::PeerChanged) => cx.proto.with(|l| {
                proto::not_forwarded(l);
                proto::peer_changed(l);
            }),
            Err(_) => cx.proto.with(|l| proto::not_forwarded(l)),
        }
    }
    // The request to `diag` for a frame that was one, and a record of
    // `diag`'s when the one before has been told (task 22.4). A record the
    // validation function refused never reaches here: the Kernel reports
    // it against `diag`. `diag` stopped or restarted is nothing to `link`.
    if let Some(r) = cx.proto.with(|l| proto::take_request(l)) {
        if cx.request.send(&r).is_err() {
            cx.proto.with(|l| proto::not_requested(l));
        }
    }
    if cx.proto.with(|l| proto::wants_record(l)) {
        if let Ok(Some(m)) = cx.records.recv() {
            cx.proto.with(|l| proto::record_in(l, &m));
        }
    }
    // The battery level again, when `therapy` has started again.
    if let Some(m) = cx.proto.with(|l| proto::take_resend(l)) {
        if cx.program.send(&m).is_err() {
            cx.proto.with(|l| proto::not_resent(l));
        }
    }
    let out = pm_core::frame::encode(&frame);
    cx.tx.with(|b| {
        if let Some(bytes) = b.bytes() {
            *bytes = out;
        }
    });
    // 4. The next transfer.
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
