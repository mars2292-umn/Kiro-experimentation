//! The programmable parameters (Table 19-1), as the step function uses them.

use crate::mode::Mode;

// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// The interval of a rate of `ppm` pulses per minute, in microseconds,
/// rounded down: the rate delivered is then not below the programmed one,
/// and the error is under 1 µs (PM26.5). `ppm` must not be zero.
#[inline(always)]
#[must_use]
#[cfg_attr(creusot, creusot_std::macros::requires(ppm@ > 0))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result@ == 60_000_000 / ppm@))]
pub const fn interval_of_ppm(ppm: u32) -> u32 {
    60_000_000 / ppm
}

/// The Activity Threshold (Table 19-1), lowest first.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
#[repr(u8)]
pub enum ActivityThreshold {
    VLow = 0,
    Low = 1,
    MedLow = 2,
    #[default]
    Med = 3,
    MedHigh = 4,
    High = 5,
    VHigh = 6,
}

/// One parameter set. Every time is in microseconds; a rate is held as its
/// interval. Amplitudes and sensitivities are data on the Target (PM2.2):
/// they are stored and reported and change no behaviour.
///
/// A `Params` is only ever produced by `validate` (task 6.2) or is one of the
/// constants below, so the step function may rely on Table 19-1 and
/// Table 19-5 holding for it.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Params {
    pub mode: Mode,
    /// Lower Rate Limit, as its interval.
    pub lrl: u32,
    /// Upper Rate Limit, as its interval.
    pub url: u32,
    /// Maximum Sensor Rate, as its interval.
    pub msr: u32,
    /// Fixed AV Delay.
    pub fixed_av: u32,
    /// Dynamic AV Delay on.
    pub dynamic_av: bool,
    /// Minimum Dynamic AV Delay.
    pub min_dynamic_av: u32,
    /// Sensed AV Delay Offset: how much shorter the AV delay is after a
    /// tracked atrial Sense; 0 is Off.
    pub sensed_av_offset: u32,
    /// Pulse amplitude per Chamber, in tenths of a volt; 0 is Off. Data.
    pub amplitude: [u8; 2],
    /// Pulse width per Chamber.
    pub width: [u32; 2],
    /// Sensitivity per Chamber, in hundredths of a millivolt. Data.
    pub sensitivity: [u16; 2],
    /// Ventricular Refractory Period.
    pub vrp: u32,
    /// Atrial Refractory Period.
    pub arp: u32,
    /// Post-Ventricular Atrial Refractory Period.
    pub pvarp: u32,
    /// PVARP Extension; 0 is Off.
    pub pvarp_extension: u32,
    /// Hysteresis Rate Limit, as its interval; 0 is Off.
    pub hrl: u32,
    /// Rate Smoothing in percent; 0 is Off.
    pub rate_smoothing: u8,
    /// ATR Mode on.
    pub atr: bool,
    /// ATR Duration, in cardiac cycles.
    pub atr_duration: u16,
    /// ATR Fallback Time.
    pub atr_fallback_time: u32,
    /// Ventricular Blanking after an atrial Pace.
    pub v_blanking: u32,
    pub activity_threshold: ActivityThreshold,
    /// Reaction Time.
    pub reaction_time: u32,
    /// Response Factor, 1 to 16.
    pub response_factor: u8,
    /// Recovery Time.
    pub recovery_time: u32,
    /// Always zero. The fields narrower than a word add up to 17 bytes;
    /// these make the set a whole number of words with no padding byte, as
    /// a Partition static must be (rsk's check of R14.5 reads every byte).
    pub reserved: [u8; 3],
}

impl Params {
    /// The nominal values of Table 19-1.
    pub const NOMINAL: Params = Params {
        mode: Mode::DDD,
        lrl: interval_of_ppm(60),
        url: interval_of_ppm(120),
        msr: interval_of_ppm(120),
        fixed_av: 150_000,
        dynamic_av: false,
        min_dynamic_av: 50_000,
        sensed_av_offset: 0,
        amplitude: [35, 35],
        width: [400, 400],
        sensitivity: [75, 250],
        vrp: 320_000,
        arp: 250_000,
        pvarp: 250_000,
        pvarp_extension: 0,
        hrl: 0,
        rate_smoothing: 0,
        atr: false,
        atr_duration: 20,
        atr_fallback_time: 60_000_000,
        v_blanking: 40_000,
        activity_threshold: ActivityThreshold::Med,
        reaction_time: 30_000_000,
        response_factor: 8,
        recovery_time: 300_000_000,
        reserved: [0; 3],
    };

    /// The Power-On Reset parameters (PM7.1): VVI, an LRL of 65 ppm, a
    /// ventricular amplitude of 5.0 V, a pulse width of 0.5 ms, a VRP of
    /// 320 ms, a ventricular sensitivity of 1.5 mV. What VVI does not use
    /// keeps its nominal value.
    pub const POWER_ON_RESET: Params = Params {
        mode: Mode::VVI,
        lrl: interval_of_ppm(65),
        amplitude: [35, 50],
        width: [400, 500],
        sensitivity: [75, 150],
        vrp: 320_000,
        ..Params::NOMINAL
    };

    /// A parameter set of all zeros: the value of the parameter fields of a
    /// zeroed `State`. Never in force; `State::programmed` guards it.
    pub const ZERO: Params = Params {
        mode: Mode::OFF,
        lrl: 0,
        url: 0,
        msr: 0,
        fixed_av: 0,
        dynamic_av: false,
        min_dynamic_av: 0,
        sensed_av_offset: 0,
        amplitude: [0, 0],
        width: [0, 0],
        sensitivity: [0, 0],
        vrp: 0,
        arp: 0,
        pvarp: 0,
        pvarp_extension: 0,
        hrl: 0,
        rate_smoothing: 0,
        atr: false,
        atr_duration: 0,
        atr_fallback_time: 0,
        v_blanking: 0,
        activity_threshold: ActivityThreshold::VLow,
        reaction_time: 0,
        response_factor: 0,
        recovery_time: 0,
        reserved: [0; 3],
    };

    /// The Pace-Now parameters (PM6.5): as Power-On Reset with a pulse width
    /// of 1.00 ms.
    pub const PACE_NOW: Params = Params { width: [400, 1_000], ..Params::POWER_ON_RESET };
}
