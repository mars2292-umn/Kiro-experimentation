//! Host tests of `pm-core`'s `state` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


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
        reserved,
    } = State::POWER_ON_RESET;
    assert_eq!(reserved, [0; 2]);
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
            unconfirmed,
            noisy,
            serviced,
            last_service,
            reserved,
        } = c;
        assert_eq!((reserved, pending.reserved), ([0; 2], [0; 3]));
        assert!(!seen && !last_paced && !blanked && !noisy && !serviced && !pending.armed && !unconfirmed);
        assert_eq!((last, refractory_until, blanking_until, pending.at, last_service), (Micros(0), Micros(0), Micros(0), Micros(0), Micros(0)));
    }
    for p in [permanent, temporary] {
        assert_eq!(p, Params::ZERO);
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
