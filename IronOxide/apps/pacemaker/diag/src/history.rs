//! What `diag` keeps (docs/pacemaker, tasks 22.1 to 22.3; PM18.1 to PM18.3,
//! PM18.5, PM18.8): pure, with no register and no rsk in it, so the host
//! tests run the code the board runs.
//!
//! Everything is words in storage of a fixed size (PM18.8, PM30.1), and the
//! all-zero value is the empty store (a Partition's state starts zeroed).
//! What is kept and what is discarded when a store is full:
//!
//! | Store | Kept | Discarded when full |
//! |---|---|---|
//! | Rate histograms, counts | Every event since the last reset; a count stops at the largest a word holds | Nothing |
//! | Marker buffer | The last `MARKERS` records | The oldest record |
//! | Trend, Continuous | The last `TREND` samples, stamped at the end | The oldest sample |
//! | Trend, Fixed | The first `TREND` samples from its start, stamped at the beginning | Every later sample |
//!
//! A record the Therapy_Partition's ring lost is counted there and told
//! here (`dropped`); a batch that did not arrive is seen by its sequence
//! number and counted here (`lost`).

use pm_core::record::{self, aug, kind};

/// The bins of a rate histogram: below 30 ppm, fifteen of 10 ppm from 30
/// to 179 ppm, and 180 ppm or above (the programmable rates are 30 to
/// 175 ppm).
pub const BINS: usize = 17;
/// The histograms: paced atrial, sensed atrial, paced ventricular, sensed
/// ventricular (PM18.1).
pub const HISTOGRAMS: usize = 4;
pub const PACED_A: usize = 0;
pub const SENSED_A: usize = 1;
pub const PACED_V: usize = 2;
pub const SENSED_V: usize = 3;
/// The records of the Marker buffer.
pub const MARKERS: usize = 64;
/// The samples of the trend, and the Jobs of `history` between two (one
/// sample a second at a period of 100 ms: 128 s of trend).
pub const TREND: usize = 128;
pub const TREND_JOBS: u32 = 10;

/// The trend's recording duration (PM18.3). Zero, the value at start, is
/// Continuous.
pub mod duration {
    pub const CONTINUOUS: u32 = 0;
    pub const FIXED: u32 = 1;
}

/// What a trend sample holds (PM18.3: the pacing rate and the sensor data,
/// separately or together). Zero, the value at start, is both.
pub mod what {
    pub const BOTH: u32 = 0;
    pub const PACING_RATE: u32 = 1;
    pub const SENSOR: u32 = 2;
}

/// The fields are in the order written (`repr(C)`; every one is words, so
/// there is no padding): the rig reads the store through the debug probe.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[repr(C)]
pub struct History {
    /// Batches taken, records in them, and batches that did not arrive
    /// (by the sequence numbers: the records between).
    pub batches: u32,
    pub records: u32,
    pub lost: u32,
    /// The Therapy_Partition's own count of records its ring lost, as the
    /// last batch told it.
    pub dropped: u32,
    /// The sequence number the next batch's first record is to have, and
    /// 1 once a batch has come.
    pub next_seq: u32,
    pub seq_known: u32,
    /// Times the Endpoint reported that the Therapy_Partition had
    /// restarted (its sequence numbers begin again).
    pub peer_changes: u32,
    /// PM18.1: the counts of PVCs and of ATR episodes, and the resets.
    pub pvcs: u32,
    pub atr_episodes: u32,
    pub resets: u32,
    pub histogram: [[u32; BINS]; HISTOGRAMS],
    /// The Marker buffer: the last `MARKERS` records, `markers_written` in all.
    pub markers_written: u32,
    pub markers: [[u32; record::RECORD_WORDS]; MARKERS],
    /// The trend: its duration and content, the samples written, the
    /// stamp (milliseconds of the Kernel's time: of the first sample in
    /// Fixed, of the last in Continuous), and the samples: the pacing
    /// interval and the sensor interval, in microseconds (0: none).
    pub trend_duration: u32,
    pub trend_what: u32,
    pub trend_written: u32,
    pub trend_stamp_ms: u32,
    pub trend: [[u32; 2]; TREND],
    /// For the next sample: the interval the last Pace ended, the sensor
    /// interval the last batch told, and the Jobs since the last sample.
    pub pace_interval: u32,
    pub sensor_interval: u32,
    pub jobs: u32,
}

/// The bin of an interval in microseconds: of its rate, rounded down to a
/// whole pulse a minute.
#[must_use]
pub const fn bin(interval_us: u32) -> usize {
    if interval_us == 0 {
        return BINS - 1;
    }
    let ppm = 60_000_000 / interval_us;
    if ppm < 30 {
        0
    } else if ppm >= 180 {
        BINS - 1
    } else {
        ((ppm - 30) / 10) as usize + 1
    }
}

impl History {
    /// The empty store: every word zero.
    #[must_use]
    pub const fn new() -> History {
        History {
            batches: 0,
            records: 0,
            lost: 0,
            dropped: 0,
            next_seq: 0,
            seq_known: 0,
            peer_changes: 0,
            pvcs: 0,
            atr_episodes: 0,
            resets: 0,
            histogram: [[0; BINS]; HISTOGRAMS],
            markers_written: 0,
            markers: [[0; record::RECORD_WORDS]; MARKERS],
            trend_duration: 0,
            trend_what: 0,
            trend_written: 0,
            trend_stamp_ms: 0,
            trend: [[0; 2]; TREND],
            pace_interval: 0,
            sensor_interval: 0,
            jobs: 0,
        }
    }

    /// One batch of the Endpoint `events`.
    pub fn take(&mut self, batch: &[u32; record::EVENTS_WORDS]) {
        let count = if batch[record::batch::COUNT] as usize > record::BATCH { record::BATCH } else { batch[record::batch::COUNT] as usize };
        let first = batch[record::batch::FIRST];
        if self.seq_known != 0 && first != self.next_seq {
            // Records between that never came (the difference of the
            // sequence numbers, which wrap).
            self.lost = self.lost.saturating_add(first.wrapping_sub(self.next_seq));
        }
        self.seq_known = 1;
        self.next_seq = first.wrapping_add(count as u32);
        self.batches = self.batches.saturating_add(1);
        self.dropped = batch[record::batch::DROPPED];
        self.sensor_interval = batch[record::batch::SENSOR];
        let mut k = 0;
        while k < record::BATCH {
            if k < count {
                let at = record::batch::RECORDS + record::RECORD_WORDS * k;
                self.record([batch[at], batch[at + 1], batch[at + 2]]);
            }
            k += 1;
        }
    }

    /// The Endpoint reported that the Therapy_Partition restarted: its
    /// sequence numbers begin again, and no loss is counted across it.
    pub fn peer_changed(&mut self) {
        self.peer_changes = self.peer_changes.saturating_add(1);
        self.seq_known = 0;
    }

    /// One event record: into the Marker buffer, and into its histogram
    /// and the counts if it is an event they count. A Refractory sense and
    /// noise are Markers and no event of a histogram.
    fn record(&mut self, r: [u32; record::RECORD_WORDS]) {
        self.records = self.records.saturating_add(1);
        self.markers[self.markers_written as usize % MARKERS] = r;
        self.markers_written = self.markers_written.wrapping_add(1);
        let (k, chamber, augmentation, since) = (record::kind_of(&r), record::chamber_of(&r), record::augmentation_of(&r), r[record::SINCE]);
        let paced = k == kind::PACE || k == kind::PACE_FUSED;
        let sensed = k == kind::SENSE || k == kind::AT || k == kind::PVC;
        if k == kind::PVC {
            self.pvcs = self.pvcs.saturating_add(1);
        }
        if augmentation == aug::ATR_DURATION {
            self.atr_episodes = self.atr_episodes.saturating_add(1);
        }
        if (paced || sensed) && chamber < 2 && since != record::NO_INTERVAL {
            let h = if chamber == 0 { if paced { PACED_A } else { SENSED_A } } else if paced { PACED_V } else { SENSED_V };
            let b = bin(since);
            self.histogram[h][b] = self.histogram[h][b].saturating_add(1);
        }
        // The pacing interval of the trend: the ventricle's, or the
        // atrium's while the ventricle is not paced.
        if paced && since != record::NO_INTERVAL && (chamber == 1 || self.histogram[PACED_V] == [0; BINS]) {
            self.pace_interval = since;
        }
    }

    /// PM18.2: every histogram and both counts cleared together.
    pub fn reset_histograms(&mut self) {
        self.histogram = [[0; BINS]; HISTOGRAMS];
        self.pvcs = 0;
        self.atr_episodes = 0;
        self.resets = self.resets.saturating_add(1);
    }

    /// PM18.3: a trend begun now, Fixed or Continuous, of the pacing rate,
    /// the sensor data, or both.
    pub fn start_trend(&mut self, duration: u32, what: u32, now_ms: u32) {
        self.trend_duration = if duration == duration::FIXED { duration::FIXED } else { duration::CONTINUOUS };
        self.trend_what = if what == what::PACING_RATE || what == what::SENSOR { what } else { what::BOTH };
        self.trend_written = 0;
        self.trend_stamp_ms = now_ms;
        self.trend = [[0; 2]; TREND];
        self.jobs = 0;
    }

    /// One Job of `history` at `now_ms`: every `TREND_JOBS` Jobs a sample.
    pub fn job(&mut self, now_ms: u32) {
        self.jobs = self.jobs.saturating_add(1);
        if self.jobs < TREND_JOBS {
            return;
        }
        self.jobs = 0;
        let sample = [
            if self.trend_what == what::SENSOR { 0 } else { self.pace_interval },
            if self.trend_what == what::PACING_RATE { 0 } else { self.sensor_interval },
        ];
        if self.trend_duration == duration::FIXED {
            // From its start until the storage is full; the stamp is the start's.
            if (self.trend_written as usize) < TREND {
                self.trend[self.trend_written as usize] = sample;
                self.trend_written += 1;
            }
        } else {
            // The latest samples; the stamp is the last one's.
            self.trend[self.trend_written as usize % TREND] = sample;
            self.trend_written = self.trend_written.wrapping_add(1);
            self.trend_stamp_ms = now_ms;
        }
    }
}

impl Default for History {
    fn default() -> History {
        History::new()
    }
}
