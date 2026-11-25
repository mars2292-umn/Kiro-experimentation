//! Validation of a parameter set (PM19.1, PM19.3; design Property P5).
//!
//! A set travels as [`WORDS`] 32-bit words, one per row of Table 19-1 in the
//! table's order, each in the table's own unit (a rate in ppm, a delay in
//! ms, a width in µs). [`validate`] returns a [`Params`] exactly when every
//! word is a value the table lists and the set is one this stage delivers.
//!
//! Since stage E4 (task 19.5) every Mode of Table 19-1 is delivered, with
//! hysteresis, Rate Smoothing and the Atrial Tachycardia Response: no set
//! is refused for its stage any more. Rate Smoothing is two words, Up and
//! Down (PM16.3); Down is the last word, after the table's rows.
//!
//! Each interactive limit of Table 19-5 holds "for the parameters its Mode
//! uses" (Table 19-3 as amended by Table 19-4).

use crate::mode::{Chambers, Mode, Response};
use crate::params::{interval_of_ppm, ActivityThreshold, Params};

// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// Property P5 for the App_Prover (task 12.3): the word sequences that are
/// admitted, written as a predicate over the sequence. It is Table 19-1 row
/// by row (`member`), Table 19-5 for the parameters the Mode uses
/// (`limits`). Until task 19.5 a third predicate, `stage`, refused what
/// the step did not deliver yet; every Mode and feature is delivered now. `validate`'s
/// contract is that it returns a parameter set exactly for these.
#[cfg(creusot)]
pub mod spec {
    use crate::mode::{hysteresis_spec, tracks_spec, Chambers, Mode};
    use creusot_std::logic::Seq;
    use creusot_std::macros::{logic, pearlite};
    use creusot_std::model::View;

    #[logic(open)]
    pub fn stepped(v: u32, lo: u32, hi: u32, step: u32) -> bool {
        pearlite! { lo <= v && v <= hi && (v@ - lo@) % step@ == 0 }
    }

    #[logic(open)]
    pub fn lrl(v: u32) -> bool {
        pearlite! { stepped(v, 30u32, 50u32, 5u32) || stepped(v, 50u32, 90u32, 1u32) || stepped(v, 90u32, 175u32, 5u32) }
    }

    #[logic(open)]
    pub fn amplitude(v: u32) -> bool {
        pearlite! { v@ == 0 || stepped(v, 5u32, 32u32, 1u32) || stepped(v, 35u32, 70u32, 5u32) }
    }

    #[logic(open)]
    pub fn width(v: u32) -> bool {
        pearlite! { v@ == 50 || stepped(v, 100u32, 1_900u32, 100u32) }
    }

    #[logic(open)]
    pub fn sensitivity(v: u32) -> bool {
        pearlite! { v@ == 25 || v@ == 50 || v@ == 75 || stepped(v, 100u32, 1_000u32, 50u32) }
    }

    #[logic(open)]
    pub fn atr_duration(v: u32) -> bool {
        pearlite! { v@ == 10 || stepped(v, 20u32, 80u32, 20u32) || stepped(v, 100u32, 2_000u32, 100u32) }
    }

    #[logic(open)]
    pub fn rate_smoothing(v: u32) -> bool {
        pearlite! { v@ == 0 || stepped(v, 3u32, 21u32, 3u32) || v@ == 25 }
    }

    /// Table 19-1, a row a line, for the 27 words after the Mode, and
    /// Rate Smoothing Down, which has Rate Smoothing's row.
    #[logic(open)]
    pub fn member(w: Seq<u32>) -> bool {
        pearlite! {
            lrl(w[1])
                && stepped(w[2], 50u32, 175u32, 5u32)
                && stepped(w[3], 50u32, 175u32, 5u32)
                && stepped(w[4], 70u32, 300u32, 10u32)
                && w[5]@ <= 1
                && stepped(w[6], 30u32, 100u32, 10u32)
                && (w[7]@ == 0 || stepped(w[7], 10u32, 100u32, 10u32))
                && amplitude(w[8])
                && amplitude(w[9])
                && width(w[10])
                && width(w[11])
                && sensitivity(w[12])
                && sensitivity(w[13])
                && stepped(w[14], 150u32, 500u32, 10u32)
                && stepped(w[15], 150u32, 500u32, 10u32)
                && stepped(w[16], 150u32, 500u32, 10u32)
                && (w[17]@ == 0 || stepped(w[17], 50u32, 400u32, 50u32))
                && (w[18]@ == 0 || lrl(w[18]))
                && rate_smoothing(w[19])
                && w[20]@ <= 1
                && atr_duration(w[21])
                && stepped(w[22], 1u32, 5u32, 1u32)
                && stepped(w[23], 30u32, 60u32, 10u32)
                && w[24]@ <= 6
                && stepped(w[25], 10u32, 50u32, 10u32)
                && stepped(w[26], 1u32, 16u32, 1u32)
                && stepped(w[27], 2u32, 16u32, 1u32)
                && rate_smoothing(w[28])
        }
    }

    /// Table 19-5 for the Mode `m`: in the Modes that track, LRL ≤ URL,
    /// VRP < URL interval, PVARP + Fixed AV Delay ≤ URL interval and, with
    /// the Dynamic AV Delay On, the minimum ≤ the fixed delay; in the Mode
    /// that uses the offset, the fixed delay ≥ the offset + 30 ms; in a
    /// rate-adaptive Mode, LRL ≤ MSR, and MSR ≤ URL if the Mode also
    /// tracks; in a Mode that uses hysteresis, with hysteresis on, HRL ≤
    /// LRL.
    #[logic(open)]
    pub fn limits(m: Mode, w: Seq<u32>) -> bool {
        pearlite! {
            (tracks_spec(m) ==>
                w[1] <= w[2]
                && w[14]@ * 1_000 < 60_000_000 / w[2]@
                && (w[16]@ + w[4]@) * 1_000 <= 60_000_000 / w[2]@
                && (w[5]@ != 1 || w[6] <= w[4]))
            && (tracks_spec(m) && m.paces == Chambers::Both ==> w[4]@ >= w[7]@ + 30)
            && (m.adaptive ==> w[1] <= w[3])
            && (tracks_spec(m) && m.adaptive ==> w[3] <= w[2])
            && (hysteresis_spec(m) && w[18]@ != 0 ==> w[18] <= w[1])
        }
    }

    /// The admitted sequences.
    #[logic(open)]
    pub fn admitted(w: Seq<u32>) -> bool {
        pearlite! {
            w.len() == 29
                && w[0]@ < 19
                && member(w)
                && limits(Mode::PROGRAMMABLE@[w[0]@], w)
        }
    }
}

/// The number of words of a parameter set.
pub const WORDS: usize = 29;

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
    /// Rate Smoothing Up, percent; 0 Off.
    pub const RATE_SMOOTHING_UP: usize = 19;
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
    /// Rate Smoothing Down, percent; 0 Off. The source's table has one
    /// row for Rate Smoothing; the text has two parameters (PM16.3).
    pub const RATE_SMOOTHING_DOWN: usize = 28;
}

/// The nominal set of Table 19-1 as words (the Mode is DDD);
/// `Params::NOMINAL` is the same set as a value.
pub const NOMINAL_WORDS: [u32; WORDS] = [
    1, 60, 120, 120, 150, 0, 50, 0, 35, 35, 400, 400, 75, 250, 320, 250, 250, 0, 0, 0, 0, 20, 1, 40, 3, 30, 8, 5, 0,
];

/// `lo`, `lo + step`, ..., `hi`.
#[inline(always)]
#[allow(clippy::manual_is_multiple_of)] // not const on the workspace's toolchain floor
#[cfg_attr(creusot, creusot_std::macros::requires(step@ > 0))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> lo@ <= v@ && v@ <= hi@))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::stepped(v, lo, hi, step)))]
const fn stepped(v: u32, lo: u32, hi: u32, step: u32) -> bool {
    v >= lo && v <= hi && (v - lo) % step == 0
}

/// The Lower Rate Limit's values: 30 to 50 by 5, 50 to 90 by 1, 90 to 175 by 5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 30 <= ppm@ && ppm@ <= 175))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::lrl(ppm)))]
const fn is_lrl(ppm: u32) -> bool {
    stepped(ppm, 30, 50, 5) || stepped(ppm, 50, 90, 1) || stepped(ppm, 90, 175, 5)
}

/// The regulated amplitudes: Off, 0.5 to 3.2 V by 0.1, 3.5 to 7.0 V by 0.5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 0 <= tenths@ && tenths@ <= 70))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::amplitude(tenths)))]
const fn is_amplitude(tenths: u32) -> bool {
    tenths == 0 || stepped(tenths, 5, 32, 1) || stepped(tenths, 35, 70, 5)
}

/// The pulse widths: 0.05 ms, 0.1 to 1.9 ms by 0.1.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 50 <= us@ && us@ <= 1900))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::width(us)))]
const fn is_width(us: u32) -> bool {
    us == 50 || stepped(us, 100, 1_900, 100)
}

/// The sensitivities: 0.25, 0.5, 0.75 mV, 1.0 to 10 mV by 0.5.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 25 <= hundredths@ && hundredths@ <= 1000))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::sensitivity(hundredths)))]
const fn is_sensitivity(hundredths: u32) -> bool {
    hundredths == 25 || hundredths == 50 || hundredths == 75 || stepped(hundredths, 100, 1_000, 50)
}

/// The ATR Durations: 10, 20 to 80 by 20, 100 to 2,000 by 100 cardiac cycles.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 10 <= cc@ && cc@ <= 2000))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::atr_duration(cc)))]
const fn is_atr_duration(cc: u32) -> bool {
    cc == 10 || stepped(cc, 20, 80, 20) || stepped(cc, 100, 2_000, 100)
}

/// The Rate Smoothing values: Off, 3 to 21 by 3, 25 percent.
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::ensures(result ==> 0 <= percent@ && percent@ <= 25))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::rate_smoothing(percent)))]
const fn is_rate_smoothing(percent: u32) -> bool {
    percent == 0 || stepped(percent, 3, 21, 3) || percent == 25
}

#[cfg_attr(creusot, creusot_std::macros::ensures((result != None) == (code@ <= 6)))]
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

/// Whether stage E2 delivers `m`: every programmable Mode at a fixed rate.
/// The rate-adaptive Modes are stage E4.
#[inline(always)]
#[must_use]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == !m.adaptive))]
pub const fn delivered_in_e2(m: Mode) -> bool {
    !m.adaptive
}

/// The interactive limits of Table 19-5 on the words `w`, each of which is
/// a value of its row of Table 19-1, for the Mode `m`. A limit binds where
/// the Mode uses every parameter it names (Table 19-3 as amended by Table
/// 19-4):
///
/// - LRL ≤ URL; VRP < URL interval; PVARP + Fixed AV Delay ≤ URL interval;
///   and, where the Dynamic AV Delay is On, Minimum Dynamic AV Delay ≤
///   Fixed AV Delay: in the Modes that track (VDD, DDD), the only ones for
///   which the Upper Rate Limit limits pacing and the delay can be dynamic;
/// - Fixed AV Delay less the Sensed AV Delay Offset ≥ 30 ms: in DDD, the
///   only fixed-rate Mode that uses the offset.
///
/// - LRL ≤ MSR: in the rate-adaptive Modes, the only ones that use the
///   Maximum Sensor Rate;
/// - MSR ≤ URL: in the rate-adaptive Modes that track (VDDR, DDDR), the
///   only ones that use both. This limit is not of Table 19-5: it is the
///   user's decision of 2026-11-25, so that no admitted set has a sensor
///   rate above the rate Property P1 holds pacing to (`step::url_floor`);
/// - HRL ≤ LRL, where hysteresis is on: in the Modes that use hysteresis
///   (AAI, VVI, DDD and their rate-adaptive forms).
///
/// ARP < URL interval binds nowhere: no Mode uses both
/// the ARP (AAT, AAI) and the Upper Rate Limit (the tracking Modes).
#[inline(always)]
#[cfg_attr(creusot, creusot_std::macros::requires(50 <= w[word::URL]@ && w[word::URL]@ <= 175))]
#[cfg_attr(creusot, creusot_std::macros::requires(w[word::VRP]@ <= 500 && w[word::PVARP]@ <= 500 && w[word::FIXED_AV]@ <= 300 && w[word::SENSED_AV_OFFSET]@ <= 100))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result == spec::limits(m, w@)))]
const fn limits_hold(m: Mode, w: &[u32; WORDS]) -> bool {
    let tracking = if m.tracks() {
        let url_interval = interval_of_ppm(w[word::URL]);
        w[word::LRL] <= w[word::URL]
            && w[word::VRP] * 1_000 < url_interval
            && (w[word::PVARP] + w[word::FIXED_AV]) * 1_000 <= url_interval
            && (w[word::DYNAMIC_AV] != 1 || w[word::MIN_DYNAMIC_AV] <= w[word::FIXED_AV])
    } else {
        true
    };
    let offset = !m.uses_sensed_av_offset() || w[word::FIXED_AV] >= w[word::SENSED_AV_OFFSET] + 30;
    let sensor = !m.adaptive || (w[word::LRL] <= w[word::MSR] && (!m.tracks() || w[word::MSR] <= w[word::URL]));
    let hysteresis = !m.uses_hysteresis() || w[word::HRL] == 0 || w[word::HRL] <= w[word::LRL];
    tracking && offset && sensor && hysteresis
}

/// The parameter set `words` describe, or `None` if the sequence is not
/// exactly [`WORDS`] long, a word is not a value of Table 19-1, the set
/// or the set breaks an interactive limit of Table 19-5 ([`limits_hold`]).
/// A rate becomes its interval, rounded
/// down, so that the rate delivered is not below the programmed one.
///
/// Contract (task 12.3), Property P5: a parameter set is returned exactly
/// for the admitted sequences ([`spec::admitted`]).
#[cfg_attr(creusot, creusot_std::macros::ensures((result != None) == spec::admitted(words@)))]
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
        && is_rate_smoothing(w[word::RATE_SMOOTHING_UP])
        && w[word::ATR] <= 1
        && is_atr_duration(w[word::ATR_DURATION])
        && stepped(w[word::ATR_FALLBACK_TIME], 1, 5, 1)
        && stepped(w[word::V_BLANKING], 30, 60, 10)
        && stepped(w[word::REACTION_TIME], 10, 50, 10)
        && stepped(w[word::RESPONSE_FACTOR], 1, 16, 1)
        && stepped(w[word::RECOVERY_TIME], 2, 16, 1)
        && is_rate_smoothing(w[word::RATE_SMOOTHING_DOWN]);
    if !member {
        return None;
    }
    let activity_threshold = activity_threshold(w[word::ACTIVITY_THRESHOLD])?;

    // Table 19-5, for the parameters the Mode uses (PM19.3).
    if !limits_hold(mode, w) {
        return None;
    }

    // Every word is now bounded by its table row, so the narrowing below
    // loses nothing: an amplitude is at most 70, a sensitivity 1,000, a
    // smoothing 25, a duration 2,000, a factor 16. An HRL that is on is a
    // rate of at least 30 ppm.
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
        hrl: if w[word::HRL] == 0 { 0 } else { interval_of_ppm(w[word::HRL]) },
        rate_smoothing_up: w[word::RATE_SMOOTHING_UP] as u8,
        rate_smoothing_down: w[word::RATE_SMOOTHING_DOWN] as u8,
        atr: w[word::ATR] == 1,
        atr_duration: w[word::ATR_DURATION] as u16,
        atr_fallback_time: w[word::ATR_FALLBACK_TIME] * 60_000_000,
        v_blanking: w[word::V_BLANKING] * 1_000,
        activity_threshold,
        reaction_time: w[word::REACTION_TIME] * 1_000_000,
        response_factor: w[word::RESPONSE_FACTOR] as u8,
        recovery_time: w[word::RECOVERY_TIME] * 60_000_000,
        reserved: [0; 2],
    })
}
