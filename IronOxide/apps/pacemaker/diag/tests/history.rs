//! Host tests of what `diag` keeps (docs/pacemaker tasks 22.1 and 22.3).
#![forbid(unsafe_code)]

use pacemaker_diag::history::*;
use pm_core::record::{self, aug, batch, kind, modifier};

fn rec(at: u32, k: u8, chamber: u8, since: u32) -> [u32; 3] {
    [at, record::word1(k, chamber, 0, 0), since]
}

fn batch_of(first: u32, dropped: u32, sensor: u32, records: &[[u32; 3]]) -> [u32; record::EVENTS_WORDS] {
    assert!(records.len() <= record::BATCH);
    let mut b = [0u32; record::EVENTS_WORDS];
    b[batch::COUNT] = records.len() as u32;
    b[batch::FIRST] = first;
    b[batch::DROPPED] = dropped;
    b[batch::SENSOR] = sensor;
    for (k, r) in records.iter().enumerate() {
        b[batch::RECORDS + 3 * k..batch::RECORDS + 3 * k + 3].copy_from_slice(r);
    }
    b
}

/// The all-zero value is the empty store (a Partition's state starts zeroed).
#[test]
fn the_empty_store_is_all_zero() {
    let h = History::new();
    assert_eq!(h, History::default());
    assert_eq!((h.batches, h.records, h.lost, h.dropped, h.pvcs, h.atr_episodes, h.markers_written, h.trend_written), (0, 0, 0, 0, 0, 0, 0, 0));
    assert!(h.histogram.iter().flatten().all(|c| *c == 0));
    assert_eq!((h.trend_duration, h.trend_what), (duration::CONTINUOUS, what::BOTH));
}

/// PM18.1: the bins of a rate histogram, on both sides of every edge.
#[test]
fn the_bins_of_a_rate() {
    assert_eq!(BINS, 17);
    // 60,000,000 / interval, rounded down, in bins of 10 ppm from 30.
    assert_eq!(bin(2_000_000), 1, "30 ppm");
    assert_eq!(bin(2_000_001), 0, "just below 30 ppm");
    assert_eq!(bin(u32::MAX - 1), 0);
    assert_eq!(bin(1_000_000), 4, "60 ppm");
    assert_eq!(bin(1_500_000), 2, "40 ppm");
    assert_eq!(bin(1_500_001), 1, "just below 40 ppm");
    assert_eq!(bin(342_857), 15, "175 ppm");
    assert_eq!(bin(333_334), 15, "179 ppm");
    assert_eq!(bin(333_333), 16, "180 ppm");
    assert_eq!(bin(1), 16);
    assert_eq!(bin(0), 16);
    for ppm in 30..180u32 {
        assert_eq!(bin(60_000_000 / ppm), ((ppm - 30) / 10) as usize + 1, "{ppm} ppm");
    }
}

/// PM18.1: each event kind into its histogram by the interval the record
/// carries; a Refractory sense and noise into none; PVCs and ATR episodes
/// counted.
#[test]
fn events_go_to_their_histograms_and_counts() {
    let mut h = History::new();
    h.take(&batch_of(
        0,
        0,
        0,
        &[
            rec(1_000, kind::PACE, 0, 1_000_000),
            rec(2_000, kind::PACE_FUSED, 1, 1_000_000),
            rec(3_000, kind::SENSE, 0, 800_000),
            rec(4_000, kind::SENSE, 1, 800_000),
            rec(5_000, kind::SENSE_REFRACTORY, 1, 100_000),
            rec(6_000, kind::NOISE, 0, 50_000),
            rec(7_000, kind::PVC, 1, 400_000),
            [8_000, record::word1(kind::AT, 0, 0, aug::ATR_DURATION), 300_000],
        ],
    ));
    assert_eq!((h.batches, h.records), (1, 8));
    assert_eq!(h.histogram[PACED_A][bin(1_000_000)], 1);
    assert_eq!(h.histogram[PACED_V][bin(1_000_000)], 1);
    assert_eq!(h.histogram[SENSED_A][bin(800_000)], 1);
    assert_eq!(h.histogram[SENSED_A][bin(300_000)], 1, "an AT is a sensed atrial event");
    assert_eq!(h.histogram[SENSED_V][bin(800_000)], 1);
    assert_eq!(h.histogram[SENSED_V][bin(400_000)], 1, "a PVC is a sensed ventricular event");
    assert_eq!(h.histogram.iter().flatten().sum::<u32>(), 6, "the Refractory sense and the noise are in no histogram");
    assert_eq!((h.pvcs, h.atr_episodes), (1, 1));
    // An event with no interval before it (the Chamber's first) is in the
    // Marker buffer and in no histogram.
    h.take(&batch_of(8, 0, 0, &[rec(9_000, kind::PACE, 1, record::NO_INTERVAL)]));
    assert_eq!(h.histogram.iter().flatten().sum::<u32>(), 6);
    assert_eq!(h.markers_written, 9);
}

/// PM18.2: a reset clears every histogram and both counts together, and
/// nothing else.
#[test]
fn a_reset_clears_the_histograms_together() {
    let mut h = History::new();
    h.take(&batch_of(0, 0, 0, &[rec(1, kind::PACE, 0, 1_000_000), rec(2, kind::PVC, 1, 500_000), [3, record::word1(kind::SENSE, 0, 0, aug::ATR_DURATION), 300_000]]));
    assert!(h.histogram.iter().flatten().any(|c| *c != 0));
    h.reset_histograms();
    assert!(h.histogram.iter().flatten().all(|c| *c == 0));
    assert_eq!((h.pvcs, h.atr_episodes, h.resets), (0, 0, 1));
    assert_eq!((h.records, h.markers_written), (3, 3), "the Marker buffer and the counts of records stay");
}

/// Task 22.1: a batch that did not arrive is seen by the sequence numbers
/// and counted; the Therapy_Partition's own drop count is kept as told; a
/// restart of the Therapy_Partition begins the numbers again without a loss.
#[test]
fn lost_batches_and_dropped_records_are_counted() {
    let mut h = History::new();
    h.take(&batch_of(100, 0, 0, &[rec(1, kind::PACE, 1, 1_000_000); 8]));
    assert_eq!((h.lost, h.next_seq), (0, 108), "the first batch begins anywhere");
    h.take(&batch_of(108, 0, 0, &[rec(2, kind::PACE, 1, 1_000_000); 3]));
    assert_eq!((h.lost, h.next_seq), (0, 111));
    // Five records never came; the Therapy_Partition says its ring lost five.
    h.take(&batch_of(116, 5, 0, &[rec(3, kind::PACE, 1, 1_000_000)]));
    assert_eq!((h.lost, h.dropped, h.next_seq), (5, 5, 117));
    // A batch of no record (the heartbeat) keeps the numbers.
    h.take(&batch_of(117, 5, 777, &[]));
    assert_eq!((h.lost, h.next_seq, h.sensor_interval, h.batches, h.records), (5, 117, 777, 4, 12));
    // The numbers wrap.
    let mut w = History::new();
    w.take(&batch_of(u32::MAX - 1, 0, 0, &[rec(1, kind::PACE, 1, 1_000_000); 4]));
    assert_eq!(w.next_seq, 2);
    w.take(&batch_of(2, 0, 0, &[rec(1, kind::PACE, 1, 1_000_000)]));
    assert_eq!(w.lost, 0);
    // A restart: the numbers begin again at 0 and nothing is counted lost.
    h.peer_changed();
    h.take(&batch_of(0, 0, 0, &[rec(4, kind::PACE, 1, 1_000_000)]));
    assert_eq!((h.lost, h.peer_changes, h.next_seq), (5, 1, 1));
    // A count above the batch's size is taken as the size.
    let mut bad = batch_of(1, 0, 0, &[rec(5, kind::PACE, 1, 1_000_000); 8]);
    bad[batch::COUNT] = 200;
    h.take(&bad);
    assert_eq!(h.next_seq, 9);
}

/// PM18.8: the Marker buffer keeps the last records and discards the oldest.
#[test]
fn the_marker_buffer_keeps_the_last_records() {
    let mut h = History::new();
    let mut seq = 0;
    for n in 0..(MARKERS as u32 + 10) {
        h.take(&batch_of(seq, 0, 0, &[[n, record::word1(kind::PACE, 1, modifier::SENSOR, 0), 500_000]]));
        seq += 1;
    }
    assert_eq!(h.markers_written, MARKERS as u32 + 10);
    // Slot k holds the record written last at k modulo the size.
    for k in 0..MARKERS as u32 {
        let want = if k < 10 { MARKERS as u32 + k } else { k };
        assert_eq!(h.markers[k as usize][0], want);
        assert_eq!(record::modifiers_of(&h.markers[k as usize]), modifier::SENSOR);
    }
}

/// PM18.3: the trend. Continuous keeps the latest samples and its stamp
/// is the last one's; Fixed keeps the first from its start and its stamp
/// is the start's; the pacing rate and the sensor data separately or
/// together.
#[test]
fn the_trend_continuous_and_fixed() {
    let mut h = History::new();
    // A sample every TREND_JOBS Jobs: the last paced interval and the
    // sensor interval of the last batch.
    h.take(&batch_of(0, 0, 600_000, &[rec(1, kind::PACE, 1, 750_000)]));
    for j in 1..TREND_JOBS {
        h.job(j * 100);
        assert_eq!(h.trend_written, 0);
    }
    h.job(1_000);
    assert_eq!((h.trend_written, h.trend[0], h.trend_stamp_ms), (1, [750_000, 600_000], 1_000));
    // Continuous: the buffer goes round, the stamp follows.
    for s in 1..(TREND as u32 + 5) {
        h.take(&batch_of(s, 0, 600_000 + s, &[rec(1, kind::PACE, 1, 750_000 + s)]));
        for j in 0..TREND_JOBS {
            h.job(1_000 + s * 1_000 + j);
        }
    }
    assert_eq!(h.trend_written, TREND as u32 + 5);
    assert_eq!(h.trend[4], [750_000 + TREND as u32 + 4, 600_000 + TREND as u32 + 4], "the oldest samples were discarded");
    assert_eq!(h.trend[5], [750_005, 600_005]);
    assert!(h.trend_stamp_ms > 1_000 + (TREND as u32 + 3) * 1_000);
    // Fixed, the pacing rate alone: from the start until full, stamped at the start.
    h.start_trend(duration::FIXED, what::PACING_RATE, 5_000_000);
    assert_eq!((h.trend_written, h.trend_stamp_ms), (0, 5_000_000));
    for s in 0..(TREND as u32 + 20) {
        h.take(&batch_of(1_000 + s, 0, 999, &[rec(1, kind::PACE, 1, 400_000 + s)]));
        for _ in 0..TREND_JOBS {
            h.job(6_000_000 + s);
        }
    }
    assert_eq!(h.trend_written, TREND as u32, "full: later samples are discarded");
    assert_eq!(h.trend[0], [400_000, 0], "the sensor data is not recorded");
    assert_eq!(h.trend[TREND - 1], [400_000 + TREND as u32 - 1, 0]);
    assert_eq!(h.trend_stamp_ms, 5_000_000);
    // The sensor data alone.
    h.start_trend(duration::CONTINUOUS, what::SENSOR, 7_000_000);
    for _ in 0..TREND_JOBS {
        h.job(7_000_500);
    }
    assert_eq!(h.trend[0], [0, 999]);
    // The pacing interval is the ventricle's once the ventricle is paced,
    // the atrium's before.
    let mut a = History::new();
    a.take(&batch_of(0, 0, 0, &[rec(1, kind::PACE, 0, 900_000)]));
    assert_eq!(a.pace_interval, 900_000);
    a.take(&batch_of(1, 0, 0, &[rec(2, kind::PACE, 1, 800_000), rec(3, kind::PACE, 0, 700_000)]));
    assert_eq!(a.pace_interval, 800_000);
}
