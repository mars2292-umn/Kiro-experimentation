//! Host tests of the dynamic AV delay (task 11.2), the URL (task 11.3),
//! and PVARP, the PVC, PVARP Extension and ventricular blanking (task
//! 11.4), against hand-worked diagrams. The generated run of
//! `step_dual.rs` checks Properties P1, P3 and P8 and the limits of the
//! dynamic delay over generated sets. An integration test, so that the
//! crate itself names no `std` (rsk's check R6.1).
#![forbid(unsafe_code)]

use pm_core::*;

const A: Chamber = Chamber::A;
const V: Chamber = Chamber::V;

fn params(mode: Mode) -> Params {
    Params { mode, ..Params::NOMINAL }
}

fn tick(now: u32) -> Event {
    Event::Tick { now: Micros(now), activity: 0, magnet: false, battery: Battery::Bol, link_up: false }
}

fn sense(c: Chamber, at: u32) -> Event {
    Event::Sense { chamber: c, at: Micros(at), edges: 1 }
}

fn paced(c: Chamber, at: u32) -> Event {
    Event::Paced { chamber: c, at: Micros(at) }
}

fn at(us: u32) -> Pace {
    Pace::At { at: Micros(us), width: 400 }
}

fn sensed(at: u32) -> Option<(Marker, Micros)> {
    Some((Marker::Sense { refractory: false }, Micros(at)))
}

fn refractory(at: u32) -> Option<(Marker, Micros)> {
    Some((Marker::Sense { refractory: true }, Micros(at)))
}

fn pvc(at: u32) -> Option<(Marker, Micros)> {
    Some((Marker::Pvc, Micros(at)))
}

/// The start every diagram shares: a tick at 0, the atrial Pace at
/// 850,000 and the ventricular Pace at 1,000,000 (nominal set: E =
/// 1,000,000, Fixed AV Delay 150,000).
fn started(p: &Params) -> State {
    let (s, a) = step(p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (at(850_000), at(1_000_000)));
    let (s, _) = step(p, &s, paced(A, 850_000));
    let (s, a) = step(p, &s, paced(V, 1_000_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_850_000), at(2_000_000)));
    s
}

// Times in µs; the nominal set: LRL 60 ppm (E = 1,000,000), URL 120 ppm
// (500,000), Fixed AV Delay 150,000, Minimum Dynamic AV Delay 50,000,
// VRP 320,000, PVARP 250,000, Ventricular Blanking 40,000, widths 400.

/// The dynamic AV delay (PM11.4): the previous cardiac cycle's length
/// times 150,000 / 1,000,000, between 50,000 and 150,000; the fixed delay
/// until a cycle has been measured. The URL is 175 ppm here (342,857) so
/// that it does not enter.
///
/// ```text
/// VP 1,000,000   VS 1,600,000     VS 1,950,000     VS 2,280,000      AP 3,230,000  VP 3,280,000
///  (no cycle      cycle 600,000    cycle 350,000    cycle 330,000                   cycle 1,000,000
///   yet: 150,000)  AV 90,000        AV 52,500        49,500 -> 50,000                AV 150,000
/// AP at          2,510,000        2,897,500        3,230,000                       4,130,000
/// VP at          2,600,000        2,950,000        3,280,000                       4,280,000
/// ```
#[test]
fn the_dynamic_av_delay_follows_the_last_cycle_within_its_limits() {
    let p = Params { dynamic_av: true, url: interval_of_ppm(175), ..params(Mode::DDD) };
    let s = started(&p);
    assert!(!s.cycle_known, "one ventricular event is no cycle");

    let (s, a) = step(&p, &s, sense(V, 1_600_000));
    assert_eq!((s.cycle_known, s.last_cycle), (true, 600_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_510_000), at(2_600_000)));

    let (s, a) = step(&p, &s, sense(V, 1_950_000));
    assert_eq!(s.last_cycle, 350_000);
    assert_eq!((a.atrial, a.ventricular), (at(2_897_500), at(2_950_000)));

    // Below the minimum: the minimum.
    let (s, a) = step(&p, &s, sense(V, 2_280_000));
    assert_eq!(s.last_cycle, 330_000);
    assert_eq!((a.atrial, a.ventricular), (at(3_230_000), at(3_280_000)));

    // The atrial Pace starts that delay: the ventricular Pace is where it ends.
    let (s, a) = step(&p, &s, paced(A, 3_230_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    // A cycle of the LRL interval: the Fixed AV Delay exactly.
    let (s, a) = step(&p, &s, paced(V, 3_280_000));
    assert_eq!(s.last_cycle, 1_000_000);
    assert_eq!((a.atrial, a.ventricular), (at(4_130_000), at(4_280_000)));

    // A cycle longer than the LRL interval (the state says 2,000,000): the
    // delay is the maximum, not 300,000. A tracked Sense after the PVARP
    // (to 3,530,000) shows it.
    let long = State { last_cycle: 2_000_000, ..s };
    let (_, a) = step(&p, &long, sense(A, 3_600_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(3_750_000)));
    // A cycle of zero length: the minimum.
    let short = State { last_cycle: 0, ..s };
    let (_, a) = step(&p, &short, sense(A, 3_600_000));
    assert_eq!(a.ventricular, at(3_650_000));
}

/// With the delay fixed the last cycle changes nothing.
#[test]
fn the_fixed_av_delay_ignores_the_cycle() {
    let p = Params { url: interval_of_ppm(175), ..params(Mode::DDD) };
    let s = started(&p);
    let (s, a) = step(&p, &s, sense(V, 1_600_000));
    assert_eq!((s.cycle_known, s.last_cycle), (true, 600_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_450_000), at(2_600_000)));
}

/// The Sensed AV Delay Offset shortens a dynamic delay too (PM11.5).
#[test]
fn the_offset_shortens_the_dynamic_delay() {
    let p = Params { dynamic_av: true, sensed_av_offset: 30_000, url: interval_of_ppm(175), ..params(Mode::DDD) };
    let s = started(&p);
    // Cycle 600,000: the paced delay is 90,000; the PVARP runs to 1,850,000.
    let (s, a) = step(&p, &s, sense(V, 1_600_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_510_000), at(2_600_000)));
    let (_, a) = step(&p, &s, sense(A, 2_000_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(2_060_000)));
}

/// The URL (PM10.1, PM10.2): an atrial rate of 150 a minute against a URL
/// of 120 ppm. A tracked Sense's ventricular Pace is delayed to one URL
/// interval after the last ventricular event, not dropped; a Sense in the
/// PVARP is not tracked. No two ventricular Paces are closer than 500,000.
///
/// ```text
/// AS   100,000        500,000           900,000      1,300,000      1,700,000
///      tracked        tracked           (PVARP to    tracked        tracked
///                     AS+AV = 650,000    1,000,000)  1,450,000      AS+AV = 1,850,000
///                     URL: 750,000                                  URL: 1,950,000
/// VP   250,000        750,000                        1,450,000      1,950,000
/// ```
#[test]
fn an_atrial_rate_above_the_url_is_tracked_no_faster_than_the_url() {
    let p = params(Mode::DDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    // Before any ventricular event there is no interval to keep.
    let (s, a) = step(&p, &s, sense(A, 100_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(250_000)));
    let (s, a) = step(&p, &s, paced(V, 250_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_100_000), at(1_250_000)));

    // The PVARP ends at 500,000: this Sense is tracked, and its Pace waits
    // for the URL interval.
    let (s, a) = step(&p, &s, sense(A, 500_000));
    assert_eq!(a.markers.atrial, sensed(500_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(750_000)));
    let (s, a) = step(&p, &s, paced(V, 750_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_600_000), at(1_750_000)));

    let (t, a) = step(&p, &s, sense(A, 900_000));
    assert_eq!(a.markers.atrial, refractory(900_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    assert_eq!(t, s);

    // AS + AV is past the URL interval's end (1,250,000): not delayed.
    let (s, a) = step(&p, &s, sense(A, 1_300_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(1_450_000)));
    let (s, a) = step(&p, &s, paced(V, 1_450_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_300_000), at(2_450_000)));

    // The PVARP ends at 1,700,000 exactly: tracked, and delayed again.
    let (_, a) = step(&p, &s, sense(A, 1_700_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(1_950_000)));
}

/// Property P1's exception: a triggered Pace comes at once, inside the URL
/// interval too.
#[test]
fn a_triggered_pace_is_not_held_to_the_url() {
    let p = params(Mode::VVT);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, _) = step(&p, &s, paced(V, 1_000_000));
    // The VRP ends at 1,320,000; the URL interval at 1,500,000.
    let (_, a) = step(&p, &s, sense(V, 1_400_000));
    assert_eq!(a.ventricular, at(1_400_000));
}

/// PVARP, the PVC and PVARP Extension (PM12.3 to PM12.5; Property P8),
/// with an extension of 100,000.
///
/// ```text
/// VP 1,000,000    AS 1,200,000   VS 1,400,000    AS 1,700,000   VS 1,800,000    AS 2,100,000
///  PVARP to        (refractory)   PVC: PVARP      (refractory)   PVC again:      tracked
///  1,250,000                      extended to                    not extended,   VP at 2,300,000
///                                 1,750,000                      to 2,050,000    (URL)
///
/// VP 2,300,000    VS 2,700,000    AS 3,000,000
///  PVARP to        PVC: extended   (refractory: without the extension the
///  2,550,000       to 3,050,000     PVARP would have ended at 2,950,000)
/// ```
#[test]
fn a_pvc_extends_the_pvarp_in_at_most_one_cycle_of_two() {
    let p = Params { pvarp_extension: 100_000, ..params(Mode::DDD) };
    let s = started(&p);
    assert!(!s.pvarp_extended);

    let (t, a) = step(&p, &s, sense(A, 1_200_000));
    assert_eq!(a.markers.atrial, refractory(1_200_000));
    assert_eq!(t, s);

    // No atrial event since the ventricular Pace: a PVC.
    let (s, a) = step(&p, &s, sense(V, 1_400_000));
    assert_eq!(a.markers.ventricular, sensed(1_400_000));
    assert_eq!(a.markers.augmentation, pvc(1_400_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_250_000), at(2_400_000)));
    assert!(s.pvarp_extended && !s.pvarp_extended_last_cycle);
    let (t, a) = step(&p, &s, sense(A, 1_700_000));
    assert_eq!(a.markers.atrial, refractory(1_700_000));
    assert_eq!(t, s);

    // A Refractory sense is no atrial event: the next ventricular Sense is
    // a PVC as well, and its PVARP is the programmed one.
    let (s, a) = step(&p, &s, sense(V, 1_800_000));
    assert_eq!(a.markers.augmentation, pvc(1_800_000));
    assert!(!s.pvarp_extended && s.pvarp_extended_last_cycle);
    let (s, a) = step(&p, &s, sense(A, 2_100_000));
    assert_eq!(a.markers.atrial, sensed(2_100_000));
    assert_eq!(a.markers.augmentation, None);
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(2_300_000)));

    // A cycle without a PVC, then a PVC: extended again.
    let (s, a) = step(&p, &s, paced(V, 2_300_000));
    assert_eq!(a.markers.augmentation, None);
    assert!(!s.pvarp_extended && !s.pvarp_extended_last_cycle);
    let (s, a) = step(&p, &s, sense(V, 2_700_000));
    assert_eq!(a.markers.augmentation, pvc(2_700_000));
    assert!(s.pvarp_extended);
    let (t, a) = step(&p, &s, sense(A, 3_000_000));
    assert_eq!(a.markers.atrial, refractory(3_000_000));
    assert_eq!(t, s);
}

/// With PVARP Extension off a PVC is marked and the PVARP is the
/// programmed one. In a Mode that does not sense the atrium there is no
/// PVC: every ventricular Sense would be one.
#[test]
fn a_pvc_without_the_extension_and_in_a_single_chamber_mode() {
    let p = params(Mode::DDD);
    let s = started(&p);
    let (s, a) = step(&p, &s, sense(V, 1_400_000));
    assert_eq!(a.markers.augmentation, pvc(1_400_000));
    assert!(!s.pvarp_extended);
    // The PVARP ends at 1,650,000.
    let (_, a) = step(&p, &s, sense(A, 1_700_000));
    assert_eq!(a.markers.atrial, sensed(1_700_000));

    let p = params(Mode::VVI);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, _) = step(&p, &s, paced(V, 1_000_000));
    let (_, a) = step(&p, &s, sense(V, 1_400_000));
    assert_eq!(a.markers.ventricular, sensed(1_400_000));
    assert_eq!(a.markers.augmentation, None);
}

/// Ventricular blanking (PM4.5): after an atrial Pace a ventricular Sense
/// within 40,000 is discarded, with no Marker; at 40,000 it counts. After
/// an atrial Sense there is no blanking.
///
/// ```text
/// AP 850,000   VS 880,000     VS 890,000
///               (discarded)    sensed: the cycle starts again
/// ```
#[test]
fn a_ventricular_sense_just_after_an_atrial_pace_is_discarded() {
    for mode in [Mode::DDD, Mode::DDI] {
        let p = params(mode);
        let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
        let (s, _) = step(&p, &s, paced(A, 850_000));
        let (t, a) = step(&p, &s, sense(V, 880_000));
        assert_eq!(a, Arming::NONE, "{mode:?}");
        assert_eq!(t, s, "{mode:?}: Property P3");
        let (t, a) = step(&p, &s, sense(V, 890_000));
        assert_eq!(a.markers.ventricular, sensed(890_000));
        assert_eq!(a.markers.augmentation, None, "an atrial Pace came before it");
        assert_eq!((a.atrial, a.ventricular), (at(1_740_000), at(1_890_000)));
        // The ventricular event ends the blanking.
        assert!(!t.chamber[1].blanked);
    }
    // After an atrial Sense: no blanking.
    let p = params(Mode::DDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, _) = step(&p, &s, sense(A, 600_000));
    let (_, a) = step(&p, &s, sense(V, 610_000));
    assert_eq!(a.markers.ventricular, sensed(610_000));
    // VDD paces no atrium and so never blanks.
    let p = params(Mode::VDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, _) = step(&p, &s, sense(A, 600_000));
    assert!(!s.chamber[1].blanked);
}

/// The allocation of Table 19-3 where the step reads a parameter (PM5.8):
/// the Dynamic AV Delay and PVARP Extension in the Modes that track (VDD,
/// DDD), the Sensed AV Delay Offset in DDD alone. Elsewhere a set that has
/// them programmed behaves as if it had not.
#[test]
fn a_parameter_is_read_only_in_the_modes_the_table_gives_it_to() {
    let all = |mode| Params { dynamic_av: true, sensed_av_offset: 30_000, pvarp_extension: 100_000, url: interval_of_ppm(175), ..params(mode) };

    // DDI: the delay stays fixed after a short cycle, and a PVC is marked
    // and extends nothing.
    let p = all(Mode::DDI);
    let s = started(&p);
    let (s, a) = step(&p, &s, sense(V, 1_600_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_450_000), at(2_600_000)));
    assert_eq!(a.markers.augmentation, pvc(1_600_000));
    assert!(!s.pvarp_extended);
    // DOO: the delay stays fixed (nothing is sensed; two paced cycles).
    let p = all(Mode::DOO);
    let s = started(&p);
    let (s, _) = step(&p, &s, paced(A, 1_850_000));
    let (_, a) = step(&p, &s, paced(V, 2_000_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_850_000), at(3_000_000)));

    // VDD: dynamic and extended, without the offset. Cycle 600,000: the
    // delay is 90,000; the PVC's PVARP runs to 1,950,000.
    let p = all(Mode::VDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, _) = step(&p, &s, paced(V, 1_000_000));
    let (s, a) = step(&p, &s, sense(V, 1_600_000));
    assert_eq!(a.ventricular, at(2_600_000));
    assert!(s.pvarp_extended);
    let (t, a) = step(&p, &s, sense(A, 1_900_000));
    assert_eq!(a.markers.atrial, refractory(1_900_000));
    assert_eq!(t, s);
    let (_, a) = step(&p, &s, sense(A, 2_000_000));
    assert_eq!(a.ventricular, at(2_090_000), "the paced delay: VDD has no offset");

    // DDD: all three (the offset is in `the_offset_shortens_the_dynamic_delay`).
    let p = all(Mode::DDD);
    let s = started(&p);
    let (s, _) = step(&p, &s, sense(V, 1_600_000));
    assert!(s.pvarp_extended);
    let (_, a) = step(&p, &s, sense(A, 2_000_000));
    assert_eq!(a.ventricular, at(2_060_000));
}
