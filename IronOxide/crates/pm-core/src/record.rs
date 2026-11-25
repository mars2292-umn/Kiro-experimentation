//! The event record and the batch of the Endpoint `events` (docs/pacemaker
//! tasks 22.1 and 22.2; design, "Event record"; PM18.5, PM18.6, PM18.8):
//! what the Therapy_Partition's `control` sends to `diag`, as words.
//!
//! A record is three words:
//!
//! | Word | Content |
//! |---|---|
//! | 0 | The event's instant: the low 32 bits of the therapy's microseconds |
//! | 1 | The Marker ([`kind`]), the Chamber (0 the atrium, 1 the ventricle), the modifiers ([`modifier`]) and the augmentation Marker ([`aug`]), a byte each from the lowest |
//! | 2 | The microseconds since the Chamber's last non-refractory event before this one ([`NO_INTERVAL`]: there was none) |
//!
//! A batch is four words and up to [`BATCH`] records ([`batch`]).

/// The words of a record, the records of a batch, the words of a batch.
pub const RECORD_WORDS: usize = 3;
pub const BATCH: usize = 8;
pub const EVENTS_WORDS: usize = 28;
/// The index of a record's interval, and the value of "no event before".
pub const SINCE: usize = 2;
pub const NO_INTERVAL: u32 = u32::MAX;

/// The Markers of PM18.5 (the source's Table 5): what a record is of.
pub mod kind {
    pub const NONE: u8 = 0;
    /// AP, VP.
    pub const PACE: u8 = 1;
    /// AP, VP with a Sense captured within the inhibition window before it.
    pub const PACE_FUSED: u8 = 2;
    /// AS, VS.
    pub const SENSE: u8 = 3;
    /// (AS), (VS): a Refractory sense.
    pub const SENSE_REFRACTORY: u8 = 4;
    /// AT: an atrial Sense faster than the URL.
    pub const AT: u8 = 5;
    /// PVC.
    pub const PVC: u8 = 6;
    /// TN: noise.
    pub const NOISE: u8 = 7;
}

/// The modifiers of a Pace (PM18.5), as bits.
pub mod modifier {
    /// At the end of a hysteresis interval.
    pub const HYSTERESIS: u8 = 1;
    /// At the sensor's rate.
    pub const SENSOR: u8 = 2;
    /// Held by Rate Smoothing Up (the interval is longer than asked for).
    pub const SMOOTHING_UP: u8 = 4;
    /// Held by Rate Smoothing Down (the interval is shorter than asked for).
    pub const SMOOTHING_DOWN: u8 = 8;
}

/// The augmentation Markers (PM18.5).
pub mod aug {
    pub const NONE: u8 = 0;
    /// ATR-Dur: the onset is detected, ATR Duration begins.
    pub const ATR_DURATION: u8 = 1;
    /// ATR-FB: Fallback begins.
    pub const ATR_FALLBACK: u8 = 2;
    /// ATR-End: the tachycardia has ended.
    pub const ATR_END: u8 = 3;
    /// PVP→: the PVARP is extended after a PVC.
    pub const PVARP_EXTENDED: u8 = 4;
}

/// The words of a batch before its records.
pub mod batch {
    /// The records in the batch, 0 to [`super::BATCH`].
    pub const COUNT: usize = 0;
    /// The number of its first record among all the Therapy_Partition has
    /// written since its start (it wraps).
    pub const FIRST: usize = 1;
    /// The records the Therapy_Partition's ring has lost since its start.
    pub const DROPPED: usize = 2;
    /// The sensor's interval in microseconds; 0 while there is none.
    pub const SENSOR: usize = 3;
    /// The first record's first word.
    pub const RECORDS: usize = 4;
}

/// Word 1 of a record.
#[inline(always)]
#[must_use]
#[cfg_attr(creusot, creusot_std::macros::ensures(result@ == kind@ + chamber@ * 0x100 + modifiers@ * 0x1_0000 + augmentation@ * 0x100_0000))]
pub const fn word1(kind: u8, chamber: u8, modifiers: u8, augmentation: u8) -> u32 {
    kind as u32 + chamber as u32 * 0x100 + modifiers as u32 * 0x1_0000 + augmentation as u32 * 0x100_0000
}

/// The Marker of a record.
#[inline(always)]
#[must_use]
pub const fn kind_of(r: &[u32; RECORD_WORDS]) -> u8 {
    (r[1] % 0x100) as u8
}

/// The Chamber of a record.
#[inline(always)]
#[must_use]
pub const fn chamber_of(r: &[u32; RECORD_WORDS]) -> u8 {
    (r[1] / 0x100 % 0x100) as u8
}

/// The modifiers of a record.
#[inline(always)]
#[must_use]
pub const fn modifiers_of(r: &[u32; RECORD_WORDS]) -> u8 {
    (r[1] / 0x1_0000 % 0x100) as u8
}

/// The augmentation Marker of a record.
#[inline(always)]
#[must_use]
pub const fn augmentation_of(r: &[u32; RECORD_WORDS]) -> u8 {
    (r[1] / 0x100_0000) as u8
}

/// A request to `diag` over the Endpoint `request` (task 22.4): a code and
/// two values, as words.
pub const REQUEST_WORDS: usize = 3;

/// The codes of a request, with what the two values are.
pub mod request {
    /// PM18.2: every histogram and both counts cleared together.
    pub const RESET_HISTOGRAMS: u32 = 1;
    /// PM18.3: a trend begun now: the duration (0 Continuous, 1 Fixed) and
    /// the content (0 both, 1 the pacing rate, 2 the sensor data).
    pub const TREND: u32 = 2;
    /// One part of a record: the record's kind ([`super::data`]) and the part.
    pub const READ: u32 = 3;
    /// PM20.1: the Marker transmission started (1) or stopped (0).
    pub const MARKERS: u32 = 4;
    /// PM18.7, PM17.5: a word of the implant data, or a measured value's
    /// stand-in, supplied: its index and the value.
    pub const SET_IMPLANT: u32 = 5;
    pub const SET_MEASURED: u32 = 6;
}

/// A record of `diag`'s over the Endpoint `records`: word 0 the record's
/// kind and, above it, the part; then five words of data. Six words are
/// the payload of a link frame.
pub const DATA_WORDS: usize = 6;

/// The kinds of record, and the parts each has.
pub mod data {
    /// The four rate histograms, 17 bins in four parts of five:
    /// `HISTOGRAM` plus 0 paced atrial, 1 sensed atrial, 2 paced
    /// ventricular, 3 sensed ventricular.
    pub const HISTOGRAM: u8 = 0x10;
    /// Part 0: the PVCs, the ATR episodes, the records taken, the records
    /// the ring of `therapy` lost, the records of batches that did not
    /// arrive. Part 1: the resets, the batches, the restarts of `therapy`
    /// seen, the records written to the Marker buffer.
    pub const COUNTS: u8 = 0x20;
    /// Part 0: the duration, the content, the samples written, the stamp
    /// in milliseconds, the samples the trend holds. Parts 1 to 64: two
    /// samples each, oldest first, the pacing interval and the sensor's.
    pub const TREND: u8 = 0x30;
    /// One record of the Marker stream: its number, then its three words.
    pub const MARKER: u8 = 0x40;
    /// The implant data, ten words in two parts (PM18.7).
    pub const IMPLANT: u8 = 0x50;
    /// The measured data's stand-ins, six values in two parts; the first
    /// word of part 0 says they are simulated (PM17.5).
    pub const MEASURED: u8 = 0x60;
}

/// The words of the implant data and the values of the measured data.
pub const IMPLANT_WORDS: usize = 10;
pub const MEASURED_WORDS: usize = 5;
/// The first word of the measured data: the values are stand-ins.
pub const SIMULATED: u32 = 1;

/// The parts of a record of `kind`; 0 for a kind that is none.
#[inline(always)]
#[must_use]
pub const fn parts(kind: u8) -> u32 {
    if kind >= data::HISTOGRAM && kind < data::HISTOGRAM + 4 {
        4
    } else if kind == data::COUNTS || kind == data::IMPLANT || kind == data::MEASURED {
        2
    } else if kind == data::TREND {
        65
    } else if kind == data::MARKER {
        1
    } else {
        0
    }
}

/// Word 0 of a record of `kind`, part `part`.
#[inline(always)]
#[must_use]
#[cfg_attr(creusot, creusot_std::macros::ensures(result@ == kind@ + part@ * 0x100))]
pub const fn head(kind: u8, part: u8) -> u32 {
    kind as u32 + part as u32 * 0x100
}

/// Whether `m` is a record of the Endpoint `records`: `comms`'s
/// validation function (R42.4: from level D to level C; PM22.4). The kind
/// is one of [`data`], the part one the kind has, and nothing is above them.
#[inline(always)]
#[must_use]
pub const fn admitted(m: &[u32; DATA_WORDS]) -> bool {
    if m[0] > 0xFFFF {
        return false;
    }
    let kind = (m[0] % 0x100) as u8;
    let part = m[0] / 0x100;
    part < parts(kind)
}
