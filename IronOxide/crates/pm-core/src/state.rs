//! What the step function carries from one event to the next.
//!
//! A Partition's statics are zero at start whatever initial value they
//! declare, so the all-zero value of every type here is its empty state, and
//! the all-zero [`State`] is the Power-On Reset State (PM30.4, design PD-08).
//! For that reason no field is an `Option`: a flag beside a value says
//! whether the value means anything.

use crate::params::Params;
use crate::time::Micros;

/// The therapy State (PM6.1). Exactly one holds: it is one field.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum Therapy {
    /// Power-On Reset: the State after a reset or a restart of the
    /// Therapy_Partition, until a parameter set is accepted.
    #[default]
    PowerOnReset = 0,
    Permanent = 1,
    Temporary = 2,
    PaceNow = 3,
    Magnet = 4,
}

/// A Pace that is armed, or none.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Pending {
    /// Whether a Pace is pending. When false, `at` means nothing.
    pub armed: bool,
    /// Its instant.
    pub at: Micros,
}

impl Pending {
    /// No Pace pending.
    pub const NONE: Pending = Pending { armed: false, at: Micros(0) };

    /// A Pace pending at `at`.
    #[must_use]
    pub const fn at(at: Micros) -> Pending {
        Pending { armed: true, at }
    }
}

/// The timing state of one Chamber.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct ChamberState {
    /// Whether the Chamber has had an event since the State began. When
    /// false, `last` and `refractory_until` mean nothing and no refractory
    /// period is in progress.
    pub seen: bool,
    /// The instant of the last paced or non-refractory sensed event.
    pub last: Micros,
    /// Whether that event was a Pace.
    pub last_paced: bool,
    /// The end of the refractory period that event started.
    pub refractory_until: Micros,
    /// The end of the blanking period in force, if `blanked`.
    pub blanked: bool,
    pub blanking_until: Micros,
    /// The pending Pace of this Chamber.
    pub pending: Pending,
    /// Noise tracking of the cycle (PM13.1): whether every service since the
    /// end of the refractory period reported edges closer than PM-PAR-12.
    pub noisy: bool,
    /// The instant of the last service of this Chamber's sense line, valid
    /// when `serviced`.
    pub serviced: bool,
    pub last_service: Micros,
}

/// The phase of the Atrial Tachycardia Response (PM14).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum AtrPhase {
    #[default]
    Idle = 0,
    Duration = 1,
    Fallback = 2,
}

/// The state of the Atrial Tachycardia Response.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Atr {
    pub phase: AtrPhase,
    /// The detection counter of PM14.1, 0 to PM-PAR-13.
    pub counter: u8,
    /// Cardiac cycles left of ATR Duration.
    pub cycles_left: u16,
    /// The instant Fallback began, valid in the Fallback phase.
    pub fallback_start: Micros,
}

/// Everything the step function carries. The all-zero value is the Power-On
/// Reset State with no event seen and nothing pending.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct State {
    /// The therapy State.
    pub therapy: Therapy,
    /// The State the Magnet State interrupted, to return to (PM8.3).
    pub before_magnet: Therapy,
    /// Whether a permanent parameter set has been accepted. When false,
    /// `permanent` means nothing and the Power-On Reset parameters are in force.
    pub programmed: bool,
    /// The permanent parameters, valid when `programmed`.
    pub permanent: Params,
    /// The temporary parameters, valid in the Temporary State.
    pub temporary: Params,
    /// A State or parameter change waiting for the next timing event.
    pub change_waiting: bool,
    /// Per Chamber, indexed by `Chamber::index`.
    pub chamber: [ChamberState; 2],
    /// The length of the last cardiac cycle, valid when `cycle_known`.
    pub cycle_known: bool,
    pub last_cycle: u32,
    /// Hysteresis: the escape interval in force is the HRL's (PM16.1).
    pub hysteresis_active: bool,
    /// Whether an atrial event occurred since the last ventricular event
    /// (a ventricular Sense without one is a PVC).
    pub atrial_since_ventricular: bool,
    /// Whether the PVARP in progress is the extended one, and whether the
    /// previous cycle's was (PM12.5: at most one cycle in two).
    pub pvarp_extended: bool,
    pub pvarp_extended_last_cycle: bool,
    /// The Atrial Tachycardia Response.
    pub atr: Atr,
    /// The sensor-indicated interval, valid when `sensor_known`.
    pub sensor_known: bool,
    pub sensor_interval: u32,
}

/// A parameter set of all zeros: the value of the parameter fields of a
/// zeroed [`State`]. Never in force; `programmed` guards it.
const ZERO_PARAMS: Params = Params {
    mode: crate::mode::Mode::OFF,
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
    activity_threshold: crate::params::ActivityThreshold::VLow,
    reaction_time: 0,
    response_factor: 0,
    recovery_time: 0,
};

const ZERO_CHAMBER: ChamberState = ChamberState {
    seen: false,
    last: Micros(0),
    last_paced: false,
    refractory_until: Micros(0),
    blanked: false,
    blanking_until: Micros(0),
    pending: Pending::NONE,
    noisy: false,
    serviced: false,
    last_service: Micros(0),
};

impl State {
    /// The Power-On Reset State: every field zero.
    pub const POWER_ON_RESET: State = State {
        therapy: Therapy::PowerOnReset,
        before_magnet: Therapy::PowerOnReset,
        programmed: false,
        permanent: ZERO_PARAMS,
        temporary: ZERO_PARAMS,
        change_waiting: false,
        chamber: [ZERO_CHAMBER, ZERO_CHAMBER],
        cycle_known: false,
        last_cycle: 0,
        hysteresis_active: false,
        atrial_since_ventricular: false,
        pvarp_extended: false,
        pvarp_extended_last_cycle: false,
        atr: Atr { phase: AtrPhase::Idle, counter: 0, cycles_left: 0, fallback_start: Micros(0) },
        sensor_known: false,
        sensor_interval: 0,
    };

    /// The parameters in force (PM6.2, PM6.3, PM6.5, PM7.1). The Magnet
    /// State paces with the parameters of the State it interrupted, at its
    /// own rate and asynchronously, which the step function applies.
    #[must_use]
    pub const fn params(&self) -> &Params {
        let therapy = match self.therapy {
            Therapy::Magnet => self.before_magnet,
            t => t,
        };
        match therapy {
            Therapy::PaceNow => &Params::PACE_NOW,
            Therapy::Temporary => &self.temporary,
            Therapy::Permanent if self.programmed => &self.permanent,
            _ => &Params::POWER_ON_RESET,
        }
    }
}

impl Default for State {
    fn default() -> State {
        State::POWER_ON_RESET
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mode::Mode;

    /// PM30.4: every enum's empty value is its zero discriminant, and every
    /// scalar of the Power-On Reset State is zero or false. Written field by
    /// field so that a field added later has to be added here.
    #[test]
    fn the_power_on_reset_state_is_all_zero() {
        let State {
            therapy,
            before_magnet,
            programmed,
            permanent,
            temporary,
            change_waiting,
            chamber,
            cycle_known,
            last_cycle,
            hysteresis_active,
            atrial_since_ventricular,
            pvarp_extended,
            pvarp_extended_last_cycle,
            atr,
            sensor_known,
            sensor_interval,
        } = State::POWER_ON_RESET;
        assert_eq!(therapy as u8, 0);
        assert_eq!(before_magnet as u8, 0);
        assert!(!programmed && !change_waiting && !cycle_known && !hysteresis_active);
        assert!(!atrial_since_ventricular && !pvarp_extended && !pvarp_extended_last_cycle && !sensor_known);
        assert_eq!((last_cycle, sensor_interval), (0, 0));
        assert_eq!(atr.phase as u8, 0);
        assert_eq!((atr.counter, atr.cycles_left, atr.fallback_start), (0, 0, Micros(0)));
        for c in chamber {
            let ChamberState {
                seen,
                last,
                last_paced,
                refractory_until,
                blanked,
                blanking_until,
                pending,
                noisy,
                serviced,
                last_service,
            } = c;
            assert!(!seen && !last_paced && !blanked && !noisy && !serviced && !pending.armed);
            assert_eq!((last, refractory_until, blanking_until, pending.at, last_service), (Micros(0), Micros(0), Micros(0), Micros(0), Micros(0)));
        }
        for p in [permanent, temporary] {
            assert_eq!(p, ZERO_PARAMS);
            assert_eq!(p.mode, Mode::default());
            assert_eq!(p.activity_threshold as u8, 0);
            assert_eq!(p.mode.paces as u8 + p.mode.senses as u8 + p.mode.response as u8, 0);
        }
        assert_eq!(State::default(), State::POWER_ON_RESET);
    }

    #[test]
    fn the_parameters_in_force_follow_the_state() {
        let mut s = State::POWER_ON_RESET;
        assert_eq!(*s.params(), Params::POWER_ON_RESET);

        s.permanent = Params::NOMINAL;
        s.programmed = true;
        s.therapy = Therapy::Permanent;
        assert_eq!(*s.params(), Params::NOMINAL);

        s.temporary = Params { mode: Mode::VOO, ..Params::NOMINAL };
        s.therapy = Therapy::Temporary;
        assert_eq!(s.params().mode, Mode::VOO);
        // The permanent set is untouched (PM6.3).
        assert_eq!(s.permanent, Params::NOMINAL);

        s.therapy = Therapy::PaceNow;
        assert_eq!(*s.params(), Params::PACE_NOW);

        s.before_magnet = Therapy::Permanent;
        s.therapy = Therapy::Magnet;
        assert_eq!(*s.params(), Params::NOMINAL);
        s.before_magnet = Therapy::PowerOnReset;
        assert_eq!(*s.params(), Params::POWER_ON_RESET);
    }

    /// A Permanent State without an accepted set cannot be reached; if it
    /// were, the zero parameters must not come into force.
    #[test]
    fn zero_parameters_are_never_in_force() {
        let mut s = State::POWER_ON_RESET;
        s.therapy = Therapy::Permanent;
        assert_eq!(*s.params(), Params::POWER_ON_RESET);
    }
}
