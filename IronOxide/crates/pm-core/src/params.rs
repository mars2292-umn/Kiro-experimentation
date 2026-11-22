//! The programmable parameters (Table 19-1), as the step function uses them.

use crate::mode::Mode;

/// The interval of a rate of `ppm` pulses per minute, in microseconds,
/// rounded down: the rate delivered is then not below the programmed one,
/// and the error is under 1 µs (PM26.5). `ppm` must not be zero.
#[must_use]
pub const fn interval_of_ppm(ppm: u32) -> u32 {
    60_000_000 / ppm
}

/// The Activity Threshold (Table 19-1), lowest first.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
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
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
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

    /// The Pace-Now parameters (PM6.5): as Power-On Reset with a pulse width
    /// of 1.00 ms.
    pub const PACE_NOW: Params = Params { width: [400, 1_000], ..Params::POWER_ON_RESET };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_rate_s_interval_is_rounded_down() {
        assert_eq!(interval_of_ppm(60), 1_000_000);
        assert_eq!(interval_of_ppm(30), 2_000_000);
        assert_eq!(interval_of_ppm(65), 923_076);
        assert_eq!(interval_of_ppm(175), 342_857);
        // Rounded down: the interval times the rate never exceeds a minute,
        // and is less than one interval-rounding short of it.
        let mut ppm = 30;
        while ppm <= 175 {
            let i = interval_of_ppm(ppm);
            assert!(i * ppm <= 60_000_000);
            assert!((i + 1) * ppm > 60_000_000);
            ppm += 1;
        }
    }

    #[test]
    fn the_nominal_set_is_table_19_1() {
        let p = Params::NOMINAL;
        assert_eq!(p.mode, Mode::DDD);
        assert_eq!(p.lrl, 1_000_000);
        assert_eq!(p.url, 500_000);
        assert_eq!(p.fixed_av, 150_000);
        assert_eq!(p.width, [400, 400]);
        assert_eq!(p.vrp, 320_000);
        assert_eq!(p.arp, 250_000);
        assert_eq!(p.pvarp, 250_000);
        assert_eq!(p.v_blanking, 40_000);
        assert_eq!(p.activity_threshold, ActivityThreshold::default());
    }

    #[test]
    fn power_on_reset_and_pace_now_are_the_source_s() {
        let p = Params::POWER_ON_RESET;
        assert_eq!(p.mode, Mode::VVI);
        assert_eq!(p.lrl, 923_076);
        assert_eq!(p.width[1], 500);
        assert_eq!(p.amplitude[1], 50);
        assert_eq!(p.sensitivity[1], 150);
        assert_eq!(p.vrp, 320_000);
        let n = Params::PACE_NOW;
        assert_eq!(n.mode, Mode::VVI);
        assert_eq!(n.lrl, 923_076);
        assert_eq!(n.width[1], 1_000);
        assert_eq!(n.vrp, 320_000);
    }
}
