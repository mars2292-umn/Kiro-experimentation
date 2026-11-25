//! The Partition `diag` of the PACEMAKER experiment (docs/pacemaker,
//! tasks 22.1 to 22.3; design PD-07): the Task `history`, which every
//! 100 ms takes the batches of event records the Therapy_Partition's
//! `control` sent over the Endpoint `events` and keeps what `history.rs`
//! says: the rate histograms, the counts, the Marker buffer and the trend.
//!
//! It is at Criticality_Level D and owns no peripheral. Nothing it does
//! reaches the Therapy_Partition: the Endpoint's channel toward `control`
//! is never written, and `control`'s validation function for it accepts
//! nothing.
//!
//! On the QEMU Target the declaration has no `diag`, and this crate is
//! `history` alone.
#![no_std]
#![forbid(unsafe_code)]

pub mod history;

#[cfg(feature = "nrf52840")]
pacemaker_decl::rsk_partition_diag!();

pub use history::History;

/// The batches `history` takes in a Job (the Endpoint's `ops`): `control`
/// sends at most one in each of its periods, five of which are one of
/// `history`'s, and the Endpoint holds two (finding B44).
pub const BATCHES: usize = 2;

/// Task `history` (periodic, 100 ms): the batches of `events`; one request
/// of `link`'s; one record for `link`, the answer to the request or else
/// the next record of the Marker transmission (the Endpoint `records`
/// holds two and `link` takes one in each of its periods, two in one of
/// this Task's: one a Job never finds it full); and the trend's sample.
#[cfg(feature = "nrf52840")]
pub fn history(mut cx: rsk_ctx::History) {
    let mut k = 0;
    while k < BATCHES {
        k += 1;
        match cx.events.recv() {
            Ok(Some(batch)) => cx.store.with(|h| h.take(&batch)),
            Ok(None) => k = BATCHES,
            // The Therapy_Partition restarted: its numbers begin again.
            Err(_) => {
                cx.store.with(|h| h.peer_changed());
                k = BATCHES;
            }
        }
    }
    let now_ms = (rsk::now() / rsk_target::ms(1)) as u32;
    let answer = match cx.request.recv() {
        Ok(Some(r)) => cx.store.with(|h| h.request(&r, now_ms)),
        _ => None,
    };
    #[cfg(feature = "misbehave")]
    let answer = if env!("PM_LINK_FAULT").as_bytes()[0] == b'r' && rsk::now() >= rsk_target::ms(10_000) {
        // A record `comms`'s validation function refuses (a kind that is
        // none): the Kernel reports it against this Partition (R42.4).
        Some([0x99, 0, 0, 0, 0, 0])
    } else {
        answer
    };
    let record = match answer {
        Some(m) => Some(m),
        None => cx.store.with(|h| h.next_marker()),
    };
    if let Some(m) = record {
        let _ = cx.records.send(&m);
    }
    cx.store.with(|h| h.job(now_ms));
}
