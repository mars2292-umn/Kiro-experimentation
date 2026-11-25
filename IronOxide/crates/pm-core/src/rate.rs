//! The arithmetic of the rate behaviour (stage E4; tasks 19.1 to 19.4), in
//! integers: no floating point is used anywhere in the therapy (PM15, PM16,
//! PM14.6).
//!
//! A rate is held in micro-pulses per minute (µppm): 60 ppm is 60,000,000.
//! An interval of i microseconds is the rate 6 × 10^13 / i, so a rate and
//! an interval convert into each other with an error under 1 µppm and
//! under 1 µs for every programmable value.

use crate::params::{ActivityThreshold, Params};

/// The period of `control`, in microseconds: the sensor rate moves one
/// step in each (design, Table 2).
pub const CONTROL_PERIOD_US: u32 = 20_000;
/// PM-PAR-12: sense edges of a Chamber no further apart than this,
/// throughout a cycle, are continuous noise.
pub const NOISE_INTERVAL_US: u32 = 100_000;
/// PM-PAR-13: the value of the detection counter at which the onset of an
/// atrial tachycardia is detected.
pub const ATR_COUNT: u8 = 8;
/// PM-PAR-14: the largest activity value.
pub const ACTIVITY_MAX: u32 = 255;

const MICRO_PER_MINUTE: u64 = 60_000_000_000_000;

/// The rate of an interval, in µppm (the largest a `u32` holds for an
/// interval too short to have one; 0 for no interval).
#[inline(always)]
#[must_use]
pub const fn rate_of(interval_us: u32) -> u32 {
    if interval_us == 0 {
        return 0;
    }
    let r = MICRO_PER_MINUTE / interval_us as u64;
    if r > u32::MAX as u64 {
        u32::MAX
    } else {
        r as u32
    }
}

/// The interval of a rate in µppm, in microseconds (the largest a `u32`
/// holds for a rate too low to have one).
#[inline(always)]
#[must_use]
pub const fn interval_of(rate_uppm: u32) -> u32 {
    if rate_uppm == 0 {
        return u32::MAX;
    }
    let i = MICRO_PER_MINUTE / rate_uppm as u64;
    if i > u32::MAX as u64 {
        u32::MAX
    } else {
        i as u32
    }
}

/// The activity value of an Activity Threshold: the seven thresholds are
/// evenly spaced over the range of the activity value (PM15.4): 31, 63, 95,
/// 127, 159, 191, 223.
#[inline(always)]
#[must_use]
pub const fn threshold(t: ActivityThreshold) -> u32 {
    let k = match t {
        ActivityThreshold::VLow => 1,
        ActivityThreshold::Low => 2,
        ActivityThreshold::MedLow => 3,
        ActivityThreshold::Med => 4,
        ActivityThreshold::MedHigh => 5,
        ActivityThreshold::High => 6,
        ActivityThreshold::VHigh => 7,
    };
    ACTIVITY_MAX * k / 8
}

/// The target rate for an activity value, in µppm (PM15.3, PM15.4): the
/// LRL at or below the Activity Threshold; above it the LRL plus
/// (MSR − LRL) × e × f / 16, where e is the activity above the threshold as
/// a fraction of the range above it and f the Response Factor; never above
/// the MSR, never below the LRL.
#[inline(always)]
#[must_use]
pub const fn target(p: &Params, activity: u8) -> u32 {
    let low = rate_of(p.lrl);
    let high = rate_of(p.msr);
    let thr = threshold(p.activity_threshold);
    let a = activity as u32;
    if high <= low || a <= thr {
        return low;
    }
    let factor = if p.response_factor > 16 { 16 } else { p.response_factor as u64 };
    let rise = (high - low) as u64 * (a - thr) as u64 * factor / ((ACTIVITY_MAX - thr) as u64 * 16);
    let t = low as u64 + rise;
    if t > high as u64 {
        high
    } else {
        t as u32
    }
}

/// The rate one period of `control` after `current`, in µppm (PM15.5,
/// PM15.6): toward the target by at most the step that takes the rate from
/// the LRL to the MSR in the Reaction Time when rising, and from the MSR to
/// the LRL in the Recovery Time when falling; within the LRL and the MSR
/// whatever `current` was.
#[inline(always)]
#[must_use]
pub const fn sensor_next(p: &Params, current: u32, activity: u8) -> u32 {
    let low = rate_of(p.lrl);
    let high = rate_of(p.msr);
    if high <= low {
        return low;
    }
    let cur = if current < low {
        low
    } else if current > high {
        high
    } else {
        current
    };
    let goal = target(p, activity);
    let span = (high - low) as u64 * CONTROL_PERIOD_US as u64;
    if goal > cur {
        let step = if p.reaction_time == 0 { (high - low) as u64 } else { span / p.reaction_time as u64 };
        let step = if step == 0 { 1 } else { step };
        let next = cur as u64 + step;
        if next > goal as u64 {
            goal
        } else {
            next as u32
        }
    } else {
        let step = if p.recovery_time == 0 { (high - low) as u64 } else { span / p.recovery_time as u64 };
        let step = if step == 0 { 1 } else { step };
        if (cur - goal) as u64 <= step {
            goal
        } else {
            cur - step as u32
        }
    }
}

/// The interval of the Fallback rate `elapsed_us` after Fallback began
/// (PM14.6): the rate falls in a straight line from that of `from_us`, the
/// cycle at which Fallback began, to the LRL over the Fallback Time, and
/// stays at the LRL after it. Never longer than the LRL interval.
#[inline(always)]
#[must_use]
pub const fn fallback_interval(p: &Params, from_us: u32, elapsed_us: u32) -> u32 {
    let low = rate_of(p.lrl);
    let start = rate_of(from_us);
    if start <= low || p.atr_fallback_time == 0 || elapsed_us >= p.atr_fallback_time {
        return p.lrl;
    }
    let fallen = (start - low) as u64 * elapsed_us as u64 / p.atr_fallback_time as u64;
    let rate = start - fallen as u32;
    let i = interval_of(rate);
    if i > p.lrl {
        p.lrl
    } else {
        i
    }
}

/// An interval kept within the Rate Smoothing percentages of the cycle
/// before it (PM16.3): not shorter than the cycle less the Up percentage
/// of it, not longer than the cycle plus the Down percentage of it. A
/// percentage of 0 is Rate Smoothing off in its direction.
#[inline(always)]
#[must_use]
pub const fn smoothed(interval_us: u32, last_cycle_us: u32, up: u8, down: u8) -> u32 {
    let up_pc = if up > 100 { 100 } else { up as u64 };
    let down_pc = if down > 100 { 100 } else { down as u64 };
    let lo = last_cycle_us as u64 - last_cycle_us as u64 * up_pc / 100;
    let hi = last_cycle_us as u64 + last_cycle_us as u64 * down_pc / 100;
    if up != 0 && (interval_us as u64) < lo {
        lo as u32
    } else if down != 0 && interval_us as u64 > hi {
        if hi > u32::MAX as u64 {
            u32::MAX
        } else {
            hi as u32
        }
    } else {
        interval_us
    }
}
