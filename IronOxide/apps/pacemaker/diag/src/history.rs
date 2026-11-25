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

use pm_core::record::{self, aug, data, kind, request};

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
    /// The Marker transmission (task 22.4): 1 while it runs, the number of
    /// the next record to send, and the records it had to pass over
    /// because the buffer had gone round before they were sent.
    pub stream: u32,
    pub stream_next: u32,
    pub stream_skipped: u32,
    /// Requests that named nothing this store has.
    pub unknown_requests: u32,
    /// PM18.7: the implant data, as the DCM supplied it (`implant` below).
    pub implant: [u32; record::IMPLANT_WORDS],
    /// PM17.1 to PM17.3, PM17.5: the measured data's stand-ins, as supplied
    /// (`measured` below). They are never measured.
    pub measured: [u32; record::MEASURED_WORDS],
    /// Always zero: the words are an even number.
    pub spare: u32,
}

/// The words of the implant data (PM18.7). The model and the serial
/// number are `link`'s to tell (the IDENTITY frame) and are not kept twice.
pub mod implant {
    /// Dates as days since 2000-01-01.
    pub const IMPLANT_DATE: usize = 0;
    pub const LEAD_IMPLANT_DATE: usize = 1;
    /// Bit 0 the atrial lead, bit 1 the ventricular: 1 bipolar, 0 unipolar.
    pub const LEAD_POLARITY: usize = 2;
    /// The pacing thresholds, in tenths of a volt at the pulse width of
    /// the test.
    pub const ATRIAL_THRESHOLD: usize = 3;
    pub const VENTRICULAR_THRESHOLD: usize = 4;
    /// The P and R wave amplitudes at implant, in tenths of a millivolt.
    pub const P_WAVE: usize = 5;
    pub const R_WAVE: usize = 6;
    /// The lead impedances at implant, in ohms.
    pub const ATRIAL_IMPEDANCE: usize = 7;
    pub const VENTRICULAR_IMPEDANCE: usize = 8;
    /// The patient's indications for pacing, as the DCM's code.
    pub const INDICATIONS: usize = 9;
}

/// The values of the measured data (PM17.1 to PM17.3): stand-ins the test
/// supplies, told with the mark `record::SIMULATED`.
pub mod measured {
    /// The battery voltage, in millivolts.
    pub const BATTERY_VOLTAGE: usize = 0;
    /// The P and R wave amplitudes, in tenths of a millivolt.
    pub const P_WAVE: usize = 1;
    pub const R_WAVE: usize = 2;
    /// The lead impedance measured (PM17.2), in ohms: the atrial lead's in
    /// the low half, the ventricular lead's in the high half.
    pub const LEAD_IMPEDANCE: usize = 3;
    /// The date of the last interrogation, as days since 2000-01-01.
    pub const LAST_INTERROGATION: usize = 4;
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
            stream: 0,
            stream_next: 0,
            stream_skipped: 0,
            unknown_requests: 0,
            implant: [0; record::IMPLANT_WORDS],
            measured: [0; record::MEASURED_WORDS],
            spare: 0,
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

impl History {
    /// One request of the Endpoint `request` at `now_ms` (task 22.4):
    /// what it changes, and the record that answers it, if it asks for one.
    pub fn request(&mut self, r: &[u32; record::REQUEST_WORDS], now_ms: u32) -> Option<[u32; record::DATA_WORDS]> {
        let (code, a, b) = (r[0], r[1], r[2]);
        if code == request::RESET_HISTOGRAMS {
            self.reset_histograms();
        } else if code == request::TREND {
            self.start_trend(a, b, now_ms);
        } else if code == request::MARKERS {
            self.stream = if a != 0 { 1 } else { 0 };
            // From the next record on: what is in the buffer was before the start.
            self.stream_next = self.markers_written;
        } else if code == request::SET_IMPLANT && (a as usize) < record::IMPLANT_WORDS {
            self.implant[a as usize] = b;
        } else if code == request::SET_MEASURED && (a as usize) < record::MEASURED_WORDS {
            self.measured[a as usize] = b;
        } else if code == request::READ && a < 0x100 && b < record::parts(a as u8) {
            return Some(self.read(a as u8, b as u8));
        } else {
            self.unknown_requests = self.unknown_requests.saturating_add(1);
        }
        None
    }

    /// Part `part` of the record of `kind` (both checked by the caller).
    fn read(&self, kind: u8, part: u8) -> [u32; record::DATA_WORDS] {
        let mut m = [0u32; record::DATA_WORDS];
        m[0] = record::head(kind, part);
        let p = part as usize;
        if kind >= data::HISTOGRAM && kind < data::HISTOGRAM + 4 {
            let h = &self.histogram[(kind - data::HISTOGRAM) as usize % HISTOGRAMS];
            let mut k = 0;
            while k < 5 {
                if 5 * p + k < BINS {
                    m[1 + k] = h[5 * p + k];
                }
                k += 1;
            }
        } else if kind == data::COUNTS {
            if part == 0 {
                m[1] = self.pvcs;
                m[2] = self.atr_episodes;
                m[3] = self.records;
                m[4] = self.dropped;
                m[5] = self.lost;
            } else {
                m[1] = self.resets;
                m[2] = self.batches;
                m[3] = self.peer_changes;
                m[4] = self.markers_written;
            }
        } else if kind == data::TREND {
            if part == 0 {
                m[1] = self.trend_duration;
                m[2] = self.trend_what;
                m[3] = self.trend_written;
                m[4] = self.trend_stamp_ms;
                m[5] = TREND as u32;
            } else {
                // Two samples, oldest first: in Continuous the buffer has
                // gone round once it is full.
                let held = if self.trend_written as usize > TREND { TREND } else { self.trend_written as usize };
                let first = if self.trend_duration == duration::FIXED || self.trend_written as usize <= TREND { 0 } else { self.trend_written as usize % TREND };
                let mut k = 0;
                while k < 2 {
                    let n = 2 * (p - 1) + k;
                    if n < held {
                        let sample = self.trend[(first + n) % TREND];
                        m[1 + 2 * k] = sample[0];
                        m[2 + 2 * k] = sample[1];
                    }
                    k += 1;
                }
            }
        } else if kind == data::IMPLANT {
            let mut k = 0;
            while k < 5 {
                m[1 + k] = self.implant[(5 * p + k) % record::IMPLANT_WORDS];
                k += 1;
            }
        } else if kind == data::MEASURED {
            if part == 0 {
                // PM17.5: marked as simulated.
                m[1] = record::SIMULATED;
                m[2] = self.measured[measured::BATTERY_VOLTAGE];
                m[3] = self.measured[measured::P_WAVE];
                m[4] = self.measured[measured::R_WAVE];
                m[5] = self.measured[measured::LEAD_IMPEDANCE];
            } else {
                m[1] = self.measured[measured::LAST_INTERROGATION];
            }
        }
        m
    }

    /// The next record of the Marker transmission, if it runs and one is
    /// waiting: its number and its three words. Records the buffer lost
    /// before they could be sent are passed over and counted.
    pub fn next_marker(&mut self) -> Option<[u32; record::DATA_WORDS]> {
        if self.stream == 0 || self.stream_next == self.markers_written {
            return None;
        }
        let behind = self.markers_written.wrapping_sub(self.stream_next);
        if behind as usize > MARKERS {
            let over = behind - MARKERS as u32;
            self.stream_skipped = self.stream_skipped.saturating_add(over);
            self.stream_next = self.stream_next.wrapping_add(over);
        }
        let r = self.markers[self.stream_next as usize % MARKERS];
        let m = [record::head(data::MARKER, 0), self.stream_next, r[0], r[1], r[2], 0];
        self.stream_next = self.stream_next.wrapping_add(1);
        Some(m)
    }
}

impl Default for History {
    fn default() -> History {
        History::new()
    }
}
