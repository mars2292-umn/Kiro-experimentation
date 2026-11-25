//! Host tests of the magnet and of the battery level in `step` (tasks 16.4
//! and 16.5; PM8.1 to PM8.4, PM17.7). The step is called as the Partition
//! calls it, with the parameters the state has in force.
#![forbid(unsafe_code)]

use pm_core::*;

const V: Chamber = Chamber::V;
const A: Chamber = Chamber::A;

fn go(s: &State, e: Event) -> (State, Arming) {
    let p = *s.params();
    step(&p, s, e)
}

fn tick(now: u32, magnet: bool, link_up: bool, battery: Battery) -> Event {
    Event::Tick { now: Micros(now), activity: 0, magnet, battery, link_up }
}

fn paced(c: Chamber, at: u32) -> Event {
    Event::Paced { chamber: c, at: Micros(at) }
}

fn set(mode: Mode, ppm: u32) -> Params {
    Params { mode, lrl: interval_of_ppm(ppm), ..Params::NOMINAL }
}

fn permanent(p: Params) -> State {
    State { therapy: Therapy::Permanent, programmed: true, permanent: p, ..State::POWER_ON_RESET }
}

/// The timing Chamber's next event: its pending Pace, delivered.
fn next_timing_event(s: &State) -> (State, Arming) {
    let c = s.params().mode.timing_chamber();
    let due = s.chamber[c.index()].pending.at.0;
    go(s, paced(c, due))
}

/// PM8.1: the Chambers the Mode paces, paced asynchronously; nothing where
/// it paces nothing. PM8.2: 100 ppm at BOL.
#[test]
fn the_magnet_state_paces_the_modes_chambers_asynchronously() {
    for (mode, magnet_mode) in [
        (Mode::AAI, Mode::AOO), (Mode::AAT, Mode::AOO), (Mode::AOO, Mode::AOO),
        (Mode::VVI, Mode::VOO), (Mode::VVT, Mode::VOO), (Mode::VOO, Mode::VOO), (Mode::VDD, Mode::VOO),
        (Mode::DDD, Mode::DOO), (Mode::DDI, Mode::DOO), (Mode::DOO, Mode::DOO),
    ] {
        let (s, _) = go(&permanent(set(mode, 60)), tick(0, false, false, Battery::Bol));
        let (s, _) = go(&s, tick(20_000, true, false, Battery::Bol));
        assert_eq!((s.therapy, s.change_waiting), (Therapy::Permanent, true), "{mode:?}: not before a timing event");
        assert_eq!(s.params().mode, mode);
        let (s, a) = next_timing_event(&s);
        assert_eq!(s.therapy, Therapy::Magnet, "{mode:?}");
        assert_eq!((s.params().mode, s.params().lrl), (magnet_mode, 600_000), "{mode:?}");
        // The event that entered the State armed the next Pace at the magnet's rate.
        let c = magnet_mode.timing_chamber();
        let from = s.chamber[c.index()].last.0;
        if magnet_mode != Mode::DOO {
            let pace = if c == A { a.atrial } else { a.ventricular };
            assert_eq!(pace, Pace::At { at: Micros(from + 600_000), width: 400 }, "{mode:?}");
        }
        // A Sense changes nothing under the magnet: the Mode senses nothing.
        let (t, a) = go(&s, Event::Sense { chamber: c, at: Micros(from + 300_000), edges: 1 });
        assert_eq!(a, Arming::NONE, "{mode:?}");
        assert_eq!(t.chamber[c.index()].pending, s.chamber[c.index()].pending);
    }
    // A sensing-only Mode: OOO, at the next tick (it has no timing event).
    let (s, _) = go(&permanent(set(Mode::OVO, 60)), tick(0, false, false, Battery::Bol));
    let (s, _) = go(&s, tick(20_000, true, false, Battery::Bol));
    assert_eq!((s.therapy, s.params().mode), (Therapy::Magnet, Mode::OFF));
}

/// PM8.2: the rate shows the battery level; a level that changes under the
/// magnet changes the rate at the next timing event.
#[test]
fn the_magnet_rate_shows_the_battery_level() {
    for (battery, interval) in [(Battery::Bol, 600_000), (Battery::Ern, 666_666), (Battery::Ert, 705_882), (Battery::Erp, 705_882)] {
        let (s, _) = go(&permanent(set(Mode::VVI, 60)), tick(0, true, false, battery));
        let (s, a) = next_timing_event(&s);
        assert_eq!((s.therapy, s.params().lrl), (Therapy::Magnet, interval), "{battery:?}");
        assert_eq!(a.ventricular, Pace::At { at: Micros(s.chamber[1].last.0 + interval), width: 400 });
    }
    let (s, _) = go(&permanent(set(Mode::VVI, 60)), tick(0, true, false, Battery::Bol));
    let (s, _) = next_timing_event(&s);
    let now = s.chamber[1].last.0 + 20_000;
    let (s, _) = go(&s, tick(now, true, false, Battery::Ern));
    assert_eq!((s.params().lrl, s.change_waiting), (600_000, true), "not before a timing event");
    let (s, _) = next_timing_event(&s);
    assert_eq!(s.params().lrl, 666_666);
    // Released: the State before, with its own rate.
    let (s, _) = go(&s, tick(s.chamber[1].last.0 + 20_000, false, false, Battery::Ern));
    let (s, a) = next_timing_event(&s);
    assert_eq!((s.therapy, s.params().mode, s.params().lrl), (Therapy::Permanent, Mode::VVI, 1_000_000));
    assert_eq!(a.ventricular, Pace::At { at: Micros(s.chamber[1].last.0 + 1_000_000), width: 400 });
}

/// PM8.4: programmed Off, the magnet input is ignored; turned off under
/// the magnet, the Magnet State ends at the next timing event; turned on
/// again with the magnet still applied, it is entered again.
#[test]
fn the_magnet_response_programmed_off_ignores_the_magnet() {
    let (s, _) = go(&permanent(set(Mode::VVI, 60)), tick(0, false, false, Battery::Bol));
    let (s, a) = go(&s, Event::Command(Command::MagnetResponse(false)));
    assert_eq!(a, Arming::NONE);
    let (s, _) = go(&s, tick(20_000, true, false, Battery::Bol));
    assert!(!s.change_waiting);
    let (s, _) = next_timing_event(&s);
    assert_eq!((s.therapy, s.params().mode, s.params().lrl), (Therapy::Permanent, Mode::VVI, 1_000_000));

    let (s, _) = go(&s, Event::Command(Command::MagnetResponse(true)));
    assert!(s.change_waiting && s.therapy == Therapy::Permanent);
    let (s, _) = next_timing_event(&s);
    assert_eq!((s.therapy, s.before_magnet), (Therapy::Magnet, Therapy::Permanent));

    let (s, _) = go(&s, Event::Command(Command::MagnetResponse(false)));
    assert!(s.change_waiting && s.therapy == Therapy::Magnet);
    let (s, _) = next_timing_event(&s);
    assert_eq!((s.therapy, s.params().mode), (Therapy::Permanent, Mode::VVI));
}

/// PM17.7: at ERT and ERP the Mode in force is a single-chamber Mode
/// without rate adaptation, from the next timing event; the programmed
/// set is kept and is in force again when the level is.
#[test]
fn a_depleted_battery_leaves_a_single_chamber_mode() {
    for low in [Battery::Ert, Battery::Erp] {
        for (mode, left) in [
            (Mode::DDD, Mode::VVI), (Mode::DDI, Mode::VVI), (Mode::VDD, Mode::VVI), (Mode::DOO, Mode::VOO), (Mode::ODO, Mode::VVI),
            (Mode::DDD.rate_adaptive(), Mode::VVI), (Mode::DOO.rate_adaptive(), Mode::VOO),
            (Mode::VVI, Mode::VVI), (Mode::VVI.rate_adaptive(), Mode::VVI), (Mode::VOO, Mode::VOO), (Mode::VVT, Mode::VVT),
            (Mode::AAI, Mode::AAI), (Mode::AAI.rate_adaptive(), Mode::AAI), (Mode::AOO, Mode::AOO), (Mode::AAT, Mode::AAT),
            (Mode::OFF, Mode::OFF), (Mode::OVO, Mode::OVO), (Mode::OAO, Mode::OAO),
        ] {
            let p = set(mode, 60);
            let (s, _) = go(&permanent(p), tick(0, false, false, Battery::Bol));
            let (s, _) = go(&s, tick(20_000, false, false, low));
            let paces_nothing = matches!(mode.paces, Chambers::None);
            let s = if paces_nothing {
                s
            } else {
                assert_eq!(s.params().mode, mode, "{mode:?}: not before a timing event");
                assert_eq!(s.change_waiting, true, "{mode:?}");
                next_timing_event(&s).0
            };
            assert_eq!(s.therapy, Therapy::Permanent);
            assert_eq!(s.params().mode, left, "{mode:?} at {low:?}");
            assert_eq!(Params { mode, ..*s.params() }, p, "{mode:?}: nothing but the Mode");
            assert_eq!(s.permanent, p);
            // After the change a Pace is pending in every Chamber the Mode paces.
            assert_eq!(s.chamber[1].pending.armed, left.paces(V), "{mode:?}");
            assert_eq!(s.chamber[0].pending.armed, left.paces(A), "{mode:?}");
            // The level comes back.
            let now = s.chamber[left.timing_chamber().index()].last.0 + 20_000;
            let (s, _) = go(&s, tick(now, false, false, Battery::Bol));
            let s = if matches!(left.paces, Chambers::None) { s } else { next_timing_event(&s).0 };
            assert_eq!(s.params().mode, mode, "{mode:?} back at BOL");
        }
    }
    // ERN changes nothing but the magnet's rate.
    let (s, _) = go(&permanent(set(Mode::DDD, 60)), tick(0, false, false, Battery::Ern));
    let (s, _) = next_timing_event(&s);
    assert_eq!(s.params().mode, Mode::DDD);
}

/// PM17.7: no temporary pacing at ERT or ERP: the command is without
/// effect, and temporary pacing in progress ends at the next timing event.
#[test]
fn a_depleted_battery_ends_and_refuses_temporary_pacing() {
    let temp = set(Mode::VOO, 120);
    for low in [Battery::Ert, Battery::Erp] {
        let (s, _) = go(&permanent(set(Mode::VVI, 60)), tick(0, false, true, low));
        let (s, _) = go(&s, Event::Command(Command::StartTemporary(temp)));
        assert!(!s.temporary_wanted, "{low:?}");
        let (s, _) = next_timing_event(&s);
        assert_eq!(s.therapy, Therapy::Permanent);

        let (s, _) = go(&permanent(set(Mode::VVI, 60)), tick(0, false, true, Battery::Bol));
        let (s, _) = go(&s, Event::Command(Command::StartTemporary(temp)));
        let (s, _) = next_timing_event(&s);
        assert_eq!((s.therapy, s.params().mode), (Therapy::Temporary, Mode::VOO));
        let (s, _) = go(&s, tick(s.chamber[1].last.0 + 20_000, false, true, low));
        assert_eq!((s.therapy, s.change_waiting), (Therapy::Temporary, true));
        let (s, _) = next_timing_event(&s);
        assert_eq!((s.therapy, s.params().mode, s.temporary_wanted), (Therapy::Permanent, Mode::VVI, false), "{low:?}");
    }
}

/// The magnet over a depleted battery: the Mode the battery leaves, paced
/// asynchronously at 85 ppm.
#[test]
fn the_magnet_over_a_depleted_battery() {
    let (s, _) = go(&permanent(set(Mode::DDD, 60)), tick(0, true, false, Battery::Ert));
    let (s, _) = next_timing_event(&s);
    assert_eq!((s.therapy, s.params().mode, s.params().lrl), (Therapy::Magnet, Mode::VOO, 705_882));
    assert!(!s.chamber[0].pending.armed && s.chamber[1].pending.armed);
}
