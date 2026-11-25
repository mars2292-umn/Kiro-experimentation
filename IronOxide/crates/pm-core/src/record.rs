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
