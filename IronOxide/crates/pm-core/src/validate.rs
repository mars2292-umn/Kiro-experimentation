//! Validation of a parameter set (PM19.1, PM19.3; design Property P5).
//!
//! A set travels as [`WORDS`] 32-bit words, one per row of Table 19-1 in the
//! table's order, each in the table's own unit (a rate in ppm, a delay in
//! ms, a width in µs). [`validate`] returns a [`Params`] exactly when every
//! word is a value the table lists and the set is one this stage delivers.
//!
//! Stage E1 (task 6.2) delivers the Modes Off, AOO, VOO, AAI and VVI with
//! hysteresis and rate smoothing Off. A set that names another Mode, or
//! turns either feature on, is refused whole: the step function does not
//! yet deliver it, and a set is never accepted to be delivered differently.
//! Tasks 11.6 and 19.5 widen this.
//!
//! No interactive limit of Table 19-5 applies in stage E1: each limit holds
//! "for the parameters its Mode uses", and by Table 19-4 the single-chamber
//! Modes use neither the URL, the MSR nor an AV delay, and hysteresis is Off.

use crate::mode::{Chambers, Mode, Response};
use crate::params::{interval_of_ppm, ActivityThreshold, Params};

// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// The number of words of a parameter set.
pub const WORDS: usize = 28;

/// The index of each parameter's word: the rows of Table 19-1 in order, the
/// atrial value before the ventricular one.
pub mod word {
    /// The Mode: its index in `Mode::PROGRAMMABLE`.
    pub const MODE: usize = 0;
    /// Lower Rate Limit, ppm.
    pub const LRL: usize = 1;
    /// Upper Rate Limit, ppm.
    pub const URL: usize = 2;
    /// Maximum Sensor Rate, ppm.
    pub const MSR: usize = 3;
    /// Fixed AV Delay, ms.
    pub const FIXED_AV: usize = 4;
    /// Dynamic AV Delay: 0 Off, 1 On.
    pub const DYNAMIC_AV: usize = 5;
    /// Minimum Dynamic AV Delay, ms.
    pub const MIN_DYNAMIC_AV: usize = 6;
    /// Sensed AV Delay Offset: 0 Off, or the ms by which the delay is shortened.
    pub const SENSED_AV_OFFSET: usize = 7;
    /// Atrial pulse amplitude, tenths of a volt; 0 Off.
    pub const A_AMPLITUDE: usize = 8;
    /// Ventricular pulse amplitude, tenths of a volt; 0 Off.
    pub const V_AMPLITUDE: usize = 9;
    /// Atrial pulse width, µs.
    pub const A_WIDTH: usize = 10;
    /// Ventricular pulse width, µs.
    pub const V_WIDTH: usize = 11;
    /// Atrial sensitivity, hundredths of a millivolt.
    pub const A_SENSITIVITY: usize = 12;
    /// Ventricular sensitivity, hundredths of a millivolt.
    pub const V_SENSITIVITY: usize = 13;
    /// Ventricular Refractory Period, ms.
    pub const VRP: usize = 14;
    /// Atrial Refractory Period, ms.
    pub const ARP: usize = 15;
    /// PVARP, ms.
    pub const PVARP: usize = 16;
    /// PVARP Extension, ms; 0 Off.
    pub const PVARP_EXTENSION: usize = 17;
    /// Hysteresis Rate Limit, ppm; 0 Off.
    pub const HRL: usize = 18;
    /// Rate Smoothing, percent; 0 Off.
    pub const RATE_SMOOTHING: usize = 19;
    /// ATR Mode: 0 Off, 1 On.
    pub const ATR: usize = 20;
    /// ATR Duration, cardiac cycles.
    pub const ATR_DURATION: usize = 21;
    /// ATR Fallback Time, minutes.
    pub const ATR_FALLBACK_TIME: usize = 22;
    /// Ventricular Blanking, ms.
    pub const V_BLANKING: usize = 23;
    /// Activity Threshold: 0 V-Low to 6 V-High.
    pub const ACTIVITY_THRESHOLD: usize = 24;
    /// Reaction Time, seconds.
    pub const REACTION_TIME: usize = 25;
    /// Response Factor.
    pub const RESPONSE_FACTOR: usize = 26;
    /// Recovery Time, minutes.
    pub const RECOVERY_TIME: usize = 27;
}

/// The nominal set of Table 19-1 as words. It names DDD, which stage E1
/// refuses; `Params::NOMINAL` is the same set as a value.
pub const NOMINAL_WORDS: [u32; WORDS] = [
    1, 60, 120, 120, 150, 0, 50, 0, 35, 35, 400, 400, 75, 250, 320, 250, 250, 0, 0, 0, 0, 20, 1, 40, 3, 30, 8, 5,
];

/// `lo`, `lo + step`, ..., `hi`.
#[inline(always)]
#[allow(clippy::manual_is_multiple_of)] // not const on the workspace's toolchain floor
#[cfg_attr(creusot, creusot_std::macros::requires(step@ > 0))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> lo@ <= v@ && v@ <= hi@))]
const fn stepped(v: u32, lo: u32, hi: u32, step: u32) -> bool {
    v >= lo && v <= hi && (v - lo) % step == 0
}

/// The Lower Rate Limit's values: 30 to 50 by 5, 50 to 90 by 1, 90 to 175 by 5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 30 <= ppm@ && ppm@ <= 175))]
const fn is_lrl(ppm: u32) -> bool {
    stepped(ppm, 30, 50, 5) || stepped(ppm, 50, 90, 1) || stepped(ppm, 90, 175, 5)
}

/// The regulated amplitudes: Off, 0.5 to 3.2 V by 0.1, 3.5 to 7.0 V by 0.5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 0 <= tenths@ && tenths@ <= 70))]
const fn is_amplitude(tenths: u32) -> bool {
    tenths == 0 || stepped(tenths, 5, 32, 1) || stepped(tenths, 35, 70, 5)
}

/// The pulse widths: 0.05 ms, 0.1 to 1.9 ms by 0.1.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 50 <= us@ && us@ <= 1900))]
const fn is_width(us: u32) -> bool {
    us == 50 || stepped(us, 100, 1_900, 100)
}

/// The sensitivities: 0.25, 0.5, 0.75 mV, 1.0 to 10 mV by 0.5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 25 <= hundredths@ && hundredths@ <= 1000))]
const fn is_sensitivity(hundredths: u32) -> bool {
    hundredths == 25 || hundredths == 50 || hundredths == 75 || stepped(hundredths, 100, 1_000, 50)
}

/// The ATR Durations: 10, 20 to 80 by 20, 100 to 2,000 by 100 cardiac cycles.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 10 <= cc@ && cc@ <= 2000))]
const fn is_atr_duration(cc: u32) -> bool {
    cc == 10 || stepped(cc, 20, 80, 20) || stepped(cc, 100, 2_000, 100)
}

/// The Rate Smoothing values: Off, 3 to 21 by 3, 25 percent.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 0 <= percent@ && percent@ <= 25))]
const fn is_rate_smoothing(percent: u32) -> bool {
    percent == 0 || stepped(percent, 3, 21, 3) || percent == 25
}

#[inline(always)]
const fn activity_threshold(code: u32) -> Option<ActivityThreshold> {
    Some(match code {
        0 => ActivityThreshold::VLow,
        1 => ActivityThreshold::Low,
        2 => ActivityThreshold::MedLow,
        3 => ActivityThreshold::Med,
        4 => ActivityThreshold::MedHigh,
        5 => ActivityThreshold::High,
        6 => ActivityThreshold::VHigh,
        _ => return None,
    })
}

/// Whether stage E1 delivers `m`: it paces at most one Chamber, senses only
/// that Chamber, and its response is none or inhibited, at a fixed rate.
#[inline(always)]
#[must_use]
pub const fn delivered_in_e1(m: Mode) -> bool {
    let single = matches!(
        (m.paces, m.senses),
        (Chambers::None | Chambers::A | Chambers::V, Chambers::None) | (Chambers::A, Chambers::A) | (Chambers::V, Chambers::V)
    );
    single && matches!(m.response, Response::None | Response::Inhibited) && !m.adaptive
}

/// The parameter set `words` describe, or `None` if the sequence is not
/// exactly [`WORDS`] long, a word is not a value of Table 19-1, or the set
/// is not one this stage delivers. A rate becomes its interval, rounded
/// down, so that the rate delivered is not below the programmed one.
#[inline(always)]
#[must_use]
pub fn validate(words: &[u32]) -> Option<Params> {
    let w: &[u32; WORDS] = words.try_into().ok()?;

    let code = w[word::MODE];
    if code >= Mode::PROGRAMMABLE.len() as u32 {
        return None;
    }
    let mode = Mode::PROGRAMMABLE[code as usize];
    let member = is_lrl(w[word::LRL])
        && stepped(w[word::URL], 50, 175, 5)
        && stepped(w[word::MSR], 50, 175, 5)
        && stepped(w[word::FIXED_AV], 70, 300, 10)
        && w[word::DYNAMIC_AV] <= 1
        && stepped(w[word::MIN_DYNAMIC_AV], 30, 100, 10)
        && (w[word::SENSED_AV_OFFSET] == 0 || stepped(w[word::SENSED_AV_OFFSET], 10, 100, 10))
        && is_amplitude(w[word::A_AMPLITUDE])
        && is_amplitude(w[word::V_AMPLITUDE])
        && is_width(w[word::A_WIDTH])
        && is_width(w[word::V_WIDTH])
        && is_sensitivity(w[word::A_SENSITIVITY])
        && is_sensitivity(w[word::V_SENSITIVITY])
        && stepped(w[word::VRP], 150, 500, 10)
        && stepped(w[word::ARP], 150, 500, 10)
        && stepped(w[word::PVARP], 150, 500, 10)
        && (w[word::PVARP_EXTENSION] == 0 || stepped(w[word::PVARP_EXTENSION], 50, 400, 50))
        && (w[word::HRL] == 0 || is_lrl(w[word::HRL]))
        && is_rate_smoothing(w[word::RATE_SMOOTHING])
        && w[word::ATR] <= 1
        && is_atr_duration(w[word::ATR_DURATION])
        && stepped(w[word::ATR_FALLBACK_TIME], 1, 5, 1)
        && stepped(w[word::V_BLANKING], 30, 60, 10)
        && stepped(w[word::REACTION_TIME], 10, 50, 10)
        && stepped(w[word::RESPONSE_FACTOR], 1, 16, 1)
        && stepped(w[word::RECOVERY_TIME], 2, 16, 1);
    if !member {
        return None;
    }
    let activity_threshold = activity_threshold(w[word::ACTIVITY_THRESHOLD])?;

    // What this stage delivers.
    if !delivered_in_e1(mode) || w[word::HRL] != 0 || w[word::RATE_SMOOTHING] != 0 {
        return None;
    }

    // Every word is now bounded by its table row, so the narrowing below
    // loses nothing: an amplitude is at most 70, a sensitivity 1,000, a
    // smoothing 25, a duration 2,000, a factor 16.
    Some(Params {
        mode,
        lrl: interval_of_ppm(w[word::LRL]),
        url: interval_of_ppm(w[word::URL]),
        msr: interval_of_ppm(w[word::MSR]),
        fixed_av: w[word::FIXED_AV] * 1_000,
        dynamic_av: w[word::DYNAMIC_AV] == 1,
        min_dynamic_av: w[word::MIN_DYNAMIC_AV] * 1_000,
        sensed_av_offset: w[word::SENSED_AV_OFFSET] * 1_000,
        amplitude: [w[word::A_AMPLITUDE] as u8, w[word::V_AMPLITUDE] as u8],
        width: [w[word::A_WIDTH], w[word::V_WIDTH]],
        sensitivity: [w[word::A_SENSITIVITY] as u16, w[word::V_SENSITIVITY] as u16],
        vrp: w[word::VRP] * 1_000,
        arp: w[word::ARP] * 1_000,
        pvarp: w[word::PVARP] * 1_000,
        pvarp_extension: w[word::PVARP_EXTENSION] * 1_000,
        hrl: 0,
        rate_smoothing: w[word::RATE_SMOOTHING] as u8,
        atr: w[word::ATR] == 1,
        atr_duration: w[word::ATR_DURATION] as u16,
        atr_fallback_time: w[word::ATR_FALLBACK_TIME] * 60_000_000,
        v_blanking: w[word::V_BLANKING] * 1_000,
        activity_threshold,
        reaction_time: w[word::REACTION_TIME] * 1_000_000,
        response_factor: w[word::RESPONSE_FACTOR] as u8,
        recovery_time: w[word::RECOVERY_TIME] * 60_000_000,
        reserved: [0; 3],
    })
}
