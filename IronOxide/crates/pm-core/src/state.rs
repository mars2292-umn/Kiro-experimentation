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
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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

/// For the App_Prover: the transitions of the State machine (design,
/// "Therapy State Machine"; Property P4). From a State the therapy may stay
/// in it or go: from Power-On Reset to Permanent (a set programmed),
/// Pace-Now or Magnet; from Permanent to Temporary, Pace-Now or Magnet; from
/// Temporary to Permanent, Pace-Now or Magnet; from Pace-Now to Permanent (a
/// set programmed) or, through it at the same event, to Magnet; from Magnet
/// to the State it interrupted, or to Pace-Now. Not: into Power-On Reset
/// from any State but Magnet, into Temporary from Power-On Reset or
/// Pace-Now.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn allowed(from: Therapy, to: Therapy) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! {
        from == to
            || match from {
                Therapy::PowerOnReset => to == Therapy::Permanent || to == Therapy::PaceNow || to == Therapy::Magnet,
                Therapy::Permanent => to == Therapy::Temporary || to == Therapy::PaceNow || to == Therapy::Magnet,
                Therapy::Temporary => to == Therapy::Permanent || to == Therapy::PaceNow || to == Therapy::Magnet,
                Therapy::PaceNow => to == Therapy::Permanent || to == Therapy::Magnet,
                Therapy::Magnet => true,
            }
    }
}

/// A Pace that is armed, or none.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Pending {
    /// Whether a Pace is pending. When false, `at` means nothing.
    pub armed: bool,
    /// Always zero (no padding byte: see `Params::reserved`).
    pub reserved: [u8; 3],
    /// Its instant.
    pub at: Micros,
}

impl Pending {
    /// No Pace pending.
    pub const NONE: Pending = Pending { armed: false, reserved: [0; 3], at: Micros(0) };

    /// A Pace pending at `at`.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.armed && result.at == at))]
    pub const fn at(at: Micros) -> Pending {
        Pending { armed: true, reserved: [0; 3], at }
    }
}

/// The timing state of one Chamber.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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
    /// Whether the last event is a Pace the step applied because a later
    /// event arrived first, and whose `Paced` event has not arrived yet
    /// (design, "Events in the order they happened").
    pub unconfirmed: bool,
    /// Noise tracking of the cycle (PM13.1): whether every service since the
    /// end of the refractory period reported edges closer than PM-PAR-12.
    pub noisy: bool,
    /// The instant of the last service of this Chamber's sense line, valid
    /// when `serviced`.
    pub serviced: bool,
    pub last_service: Micros,
    /// Always zero (no padding byte: see `Params::reserved`).
    pub reserved: [u8; 2],
}

/// The phase of the Atrial Tachycardia Response (PM14).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
#[repr(u8)]
pub enum AtrPhase {
    #[default]
    Idle = 0,
    Duration = 1,
    Fallback = 2,
}

/// The state of the Atrial Tachycardia Response.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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
    /// A permanent set the DCM programmed, waiting for the next timing
    /// event to come into force; `staged` means nothing when false.
    pub staged_valid: bool,
    pub staged: Params,
    /// Temporary pacing was started and not ended (PM6.4).
    pub temporary_wanted: bool,
    /// The magnet input and the link, as the last tick had them.
    pub magnet: bool,
    pub link_up: bool,
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
    /// Always zero (no padding byte: see `Params::reserved`).
    pub reserved: [u8; 2],
}

/// The timing state of a Chamber with no event yet: all zero.
pub const ZERO_CHAMBER: ChamberState = ChamberState {
    seen: false,
    last: Micros(0),
    last_paced: false,
    refractory_until: Micros(0),
    blanked: false,
    blanking_until: Micros(0),
    pending: Pending::NONE,
    unconfirmed: false,
    noisy: false,
    serviced: false,
    last_service: Micros(0),
    reserved: [0; 2],
};

impl State {
    /// The Power-On Reset State: every field zero.
    pub const POWER_ON_RESET: State = State {
        therapy: Therapy::PowerOnReset,
        before_magnet: Therapy::PowerOnReset,
        programmed: false,
        permanent: Params::ZERO,
        temporary: Params::ZERO,
        change_waiting: false,
        staged_valid: false,
        staged: Params::ZERO,
        temporary_wanted: false,
        magnet: false,
        link_up: false,
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
        reserved: [0; 2],
    };

    /// The parameters in force (PM6.2, PM6.3, PM6.5, PM7.1). The Magnet
    /// State paces with the parameters of the State it interrupted, at its
    /// own rate and asynchronously, which the step function applies.
    #[inline(always)]
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
    #[inline(always)]
    fn default() -> State {
        State::POWER_ON_RESET
    }
}
