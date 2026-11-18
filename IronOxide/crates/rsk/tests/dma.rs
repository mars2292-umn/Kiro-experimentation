#![forbid(unsafe_code)]
//! R17.5 on the host: a buffer lent to an EasyDMA transfer gives out no
//! bytes until its transfer is taken back and the peripheral has reported
//! the stop. The supervisor calls and the registers are the host doubles
//! (a request is always accepted; a register reads what was last written).

use rsk::{DmaBuf, DmaChannel, DmaGuarded, DmaHeld, DmaReclaimError};

const WINDOW: u32 = 0x4000_3000;
const EVENTS_STOPPED: u32 = 0x104;
const ENABLE: u32 = 0x500;

/// The register doubles are one table: the tests take turns.
static TURN: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// R17.5: while a transfer has a buffer, the buffer gives out no bytes and
/// takes no second request; only its own transfer, taken back after the
/// peripheral's stop, releases it.
#[test]
fn a_lent_buffer_is_out_of_reach_until_its_transfer_is_taken_back_and_stopped() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut a: DmaBuf<16> = DmaBuf::new();
    let mut b: DmaBuf<16> = DmaBuf::new();
    a.bytes().expect("not lent")[0] = 7;
    let ta = a.lend(DmaChannel::generated(0, WINDOW)).expect("accepted");
    assert!(a.is_lent() && a.bytes().is_none());
    // Lent once: no second request while the first transfer has the bytes.
    assert!(a.lend(DmaChannel::generated(0, WINDOW)).is_none());
    // Another buffer's transfer does not release this one.
    let tb = b.lend(DmaChannel::generated(1, WINDOW)).expect("accepted");
    let tb = match a.reclaim(tb) {
        Err(DmaReclaimError::NotThisBuffer(t)) => t,
        _ => panic!("another buffer's transfer was taken"),
    };
    assert!(a.is_lent());
    // The peripheral enabled and not reporting STOPPED: the stop is
    // requested, the buffer stays lent, and the transfer comes back.
    rsk::mmio::write(WINDOW + ENABLE, 7);
    rsk::mmio::write(WINDOW + EVENTS_STOPPED, 0);
    let ta = match a.reclaim(ta) {
        Err(DmaReclaimError::NotStopped(t)) => t,
        _ => panic!("released without the STOPPED event"),
    };
    assert!(a.bytes().is_none());
    // Once it reports STOPPED the bytes are the Task's again, as written.
    rsk::mmio::write(WINDOW + EVENTS_STOPPED, 1);
    assert!(a.reclaim(ta).is_ok());
    assert_eq!(a.bytes().expect("released")[0], 7);
    // A peripheral that is not enabled runs no transfer.
    rsk::mmio::write(WINDOW + ENABLE, 0);
    rsk::mmio::write(WINDOW + EVENTS_STOPPED, 0);
    assert!(b.reclaim(tb).is_ok() && !b.is_lent());
    // A dropped transfer leaves its buffer lent.
    let t = b.lend(DmaChannel::generated(1, WINDOW)).expect("accepted");
    drop(t);
    assert!(b.is_lent() && b.bytes().is_none());
}

/// The guards: untouched and unwritten, written inside, a guard changed.
#[test]
fn the_footprint_verdicts() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut g: DmaGuarded<8> = DmaGuarded::new();
    // Before `arm` the guards are the zeroed memory's: every one "changed".
    assert_eq!(g.footprint(0xA5) & 0xFF, DmaGuarded::<8>::GUARD_WRITTEN);
    g.arm(0xA5);
    assert_eq!(g.footprint(0xA5), DmaGuarded::<8>::UNWRITTEN);
    g.buf.bytes().expect("not lent")[3] = 0;
    assert_eq!(g.footprint(0xA5), DmaGuarded::<8>::WRITTEN_INSIDE);
}

/// A Partition's statics start as zeroed memory (R14.5), whatever initial
/// value they were declared with. `DmaBuf::new` and `DmaHeld::none` are
/// written with every field zero (no niche, no non-zero default), so the
/// zeroed value and the declared one are the same: not lent, nothing held.
/// And a transfer kept in a holder comes out as it went in, without having
/// been stopped.
#[test]
fn a_new_buffer_is_free_and_a_new_holder_is_empty() {
    let _turn = TURN.lock().unwrap_or_else(|e| e.into_inner());
    let mut b: DmaBuf<16> = DmaBuf::new();
    let mut h: DmaHeld = DmaHeld::none();
    assert!(!b.is_lent() && !h.is_some() && h.take().is_none());
    h.put(b.lend(DmaChannel::generated(0, WINDOW)));
    assert!(h.is_some() && b.is_lent());
    let t = h.take().expect("kept");
    assert!(!h.is_some());
    rsk::mmio::write(WINDOW + ENABLE, 0);
    assert!(b.reclaim(t).is_ok() && !b.is_lent());
}
