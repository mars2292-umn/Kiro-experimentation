//! Task 22.1 (PM18.8, PM22.4): the event ring and the batches `control`
//! sends over the Endpoint `events`.
#![forbid(unsafe_code)]

use pacemaker_therapy::{kind, validate_events, Record, Timing, RING};
use pm_core::record::{self, aug, batch, modifier};

fn rec(n: u32) -> Record {
    Record { kind: kind::PACE, chamber: 1, modifiers: modifier::SENSOR, augmentation: aug::ATR_FALLBACK, at: 1_000 + n, since: 500_000 + n }
}

fn numbers(b: &[u32; record::EVENTS_WORDS]) -> Vec<u32> {
    (0..b[batch::COUNT] as usize).map(|k| b[batch::RECORDS + 3 * k] - 1_000).collect()
}

/// A batch is the oldest records not yet handed over, eight at most, each
/// as its three words; they stay in the ring until the Endpoint has taken
/// them.
#[test]
fn a_batch_is_the_oldest_eight_records_and_leaves_them_in_the_ring() {
    let mut t = Timing::new();
    let empty = t.batch();
    assert_eq!(empty, [0; record::EVENTS_WORDS], "nothing written: a batch of no record");
    for n in 0..11 {
        t.push(rec(n));
    }
    let b = t.batch();
    assert_eq!((b[batch::COUNT], b[batch::FIRST], b[batch::DROPPED], b[batch::SENSOR]), (8, 0, 0, 0));
    assert_eq!(numbers(&b), (0..8).collect::<Vec<u32>>());
    let r = [b[batch::RECORDS], b[batch::RECORDS + 1], b[batch::RECORDS + 2]];
    assert_eq!(r, [1_000, record::word1(kind::PACE, 1, modifier::SENSOR, aug::ATR_FALLBACK), 500_000]);
    assert_eq!((record::kind_of(&r), record::chamber_of(&r), record::modifiers_of(&r), record::augmentation_of(&r)), (kind::PACE, 1, modifier::SENSOR, aug::ATR_FALLBACK));
    // Not taken (the Endpoint was full): the same batch again.
    assert_eq!(t.batch(), b);
    assert_eq!((t.drained, t.written), (0, 11));
    // Taken: the next batch is the three that are left, then none.
    t.delivered(b[batch::FIRST], b[batch::COUNT]);
    let b = t.batch();
    assert_eq!((b[batch::COUNT], b[batch::FIRST]), (3, 8));
    assert_eq!(numbers(&b), vec![8, 9, 10]);
    assert!(b[batch::RECORDS + 9..].iter().all(|w| *w == 0), "the words after the records are zero");
    t.delivered(8, 3);
    assert_eq!((t.batch()[batch::COUNT], t.batch()[batch::FIRST], t.drained), (0, 11, 11));
    // The sensor's interval travels in the batch once there is one.
    t.state.sensor_known = true;
    t.state.sensor_interval = 600_000;
    assert_eq!(t.batch()[batch::SENSOR], 600_000);
}

/// A ring full of records not handed over loses its oldest, and the loss
/// is counted and told in the batch; it never waits.
#[test]
fn an_overflow_loses_the_oldest_record_and_is_counted() {
    let mut t = Timing::new();
    for n in 0..RING as u32 {
        t.push(rec(n));
    }
    assert_eq!((t.counts.dropped, t.drained, t.written), (0, 0, RING as u32), "full, nothing lost yet");
    for n in RING as u32..RING as u32 + 9 {
        t.push(rec(n));
    }
    assert_eq!((t.counts.dropped, t.drained, t.written), (9, 9, RING as u32 + 9));
    let b = t.batch();
    assert_eq!((b[batch::COUNT], b[batch::FIRST], b[batch::DROPPED]), (8, 9, 9));
    assert_eq!(numbers(&b), (9..17).collect::<Vec<u32>>(), "the oldest that are left");
    // With the ring drained in time nothing is lost, however many records.
    let mut t = Timing::new();
    for n in 0..1_000u32 {
        t.push(rec(n));
        if n % 8 == 7 {
            let b = t.batch();
            assert_eq!(numbers(&b), (n - 7..=n).collect::<Vec<u32>>());
            t.delivered(b[batch::FIRST], b[batch::COUNT]);
        }
    }
    assert_eq!((t.counts.dropped, t.drained), (0, 1_000));
}

/// Records lost between a batch's making and its delivery: the number of
/// the oldest record never goes back.
#[test]
fn a_delivery_never_moves_the_ring_back() {
    let mut t = Timing::new();
    for n in 0..RING as u32 {
        t.push(rec(n));
    }
    let b = t.batch();
    // Twelve more before the Endpoint has answered: the ring loses 12, the
    // 8 of the batch among them.
    for n in 0..12 {
        t.push(rec(100 + n));
    }
    assert_eq!(t.drained, 12);
    t.delivered(b[batch::FIRST], b[batch::COUNT]);
    assert_eq!(t.drained, 12, "the batch's end, 8, is behind the ring's oldest record");
    // Four more only: the batch's end is ahead, and the ring moves to it.
    let mut t = Timing::new();
    for n in 0..RING as u32 {
        t.push(rec(n));
    }
    let b = t.batch();
    for n in 0..4 {
        t.push(rec(100 + n));
    }
    assert_eq!(t.drained, 4);
    t.delivered(b[batch::FIRST], b[batch::COUNT]);
    assert_eq!(t.drained, 8);
    // On a build with no `diag` the records are done with at once.
    t.discard();
    assert_eq!((t.drained, t.batch()[batch::COUNT]), (t.written, 0));
}

/// The channel of `events` toward `control` carries nothing.
#[test]
fn nothing_is_a_message_toward_control() {
    assert!(!validate_events(&[0; record::EVENTS_WORDS]));
    assert!(!validate_events(&[u32::MAX; record::EVENTS_WORDS]));
}
