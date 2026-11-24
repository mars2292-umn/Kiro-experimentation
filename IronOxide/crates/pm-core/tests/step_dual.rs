//! Host tests of the ventricular-based cycle (DOO, DDI, VDD, DDD; task
//! 11.1) and of the triggered response (AAT, VVT; task 11.5), against
//! hand-worked diagrams, and a generated run that holds a model of the
//! compares against the state. An integration test, so that the crate
//! itself names no `std` (rsk's check R6.1).
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

fn pace_marker(at: u32) -> Option<(Marker, Micros)> {
    Some((Marker::Pace { fused: false }, Micros(at)))
}

// Hand-worked diagrams. Times in µs; the nominal set: LRL 60 ppm
// (E = 1,000,000), Fixed AV Delay 150,000, no Sensed AV Delay Offset,
// VRP 320,000, ARP 250,000, widths 400.

/// DOO: both Chambers paced, nothing sensed (PM5.3, PM9.2, PM11.2).
///
/// ```text
/// tick 1,000   AP 851,000   VP 1,001,000   AP 1,851,000   VP 2,001,000
///   |-- E-AV ----|--- AV ----|              |--- AV -------|
///   |----------- E ----------|------------- E -------------|
/// ```
#[test]
fn doo_paces_the_atrium_one_av_delay_before_the_ventricle() {
    let p = params(Mode::DOO);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(1_000));
    assert_eq!((a.atrial, a.ventricular), (at(851_000), at(1_001_000)));
    assert_eq!(a.markers, Markers::NONE);

    // The atrial Pace: nothing to arm. The ventricular Pace is where the
    // AV delay ends already, and the atrium waits for the ventricular event.
    let (s, a) = step(&p, &s, paced(A, 851_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    assert_eq!(a.markers.atrial, pace_marker(851_000));
    assert!(!s.chamber[0].pending.armed);
    assert_eq!(s.chamber[1].pending, Pending::at(Micros(1_001_000)));

    // Senses are not sensed, in either Chamber, at any time.
    for c in [A, V] {
        let (t, a) = step(&p, &s, sense(c, 900_000));
        assert_eq!(a, Arming::NONE);
        assert_eq!(t, s);
    }
    // A tick during the AV delay arms nothing: the atrium has no Pace
    // pending until the ventricular event, and that is as it should be.
    let (t, a) = step(&p, &s, tick(900_000));
    assert_eq!(a, Arming::NONE);
    assert_eq!(t, s);

    // The ventricular Pace: both Chambers for the next cycle.
    let (s, a) = step(&p, &s, paced(V, 1_001_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_851_000), at(2_001_000)));
    assert_eq!(a.markers.ventricular, pace_marker(1_001_000));
    assert_eq!(a.markers.atrial, None);
    let _ = s;
}

/// An atrial Pace the hardware delivered late (moved by the lead) does not
/// lengthen the LRL interval: the ventricular Pace stays one escape
/// interval after the last ventricular event, and the AV delay is shorter
/// by the lead.
#[test]
fn a_late_atrial_pace_does_not_move_the_ventricular_pace() {
    let p = params(Mode::DOO);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(1_000));
    let (s, a) = step(&p, &s, paced(A, 851_500));
    assert_eq!(a.ventricular, Pace::Keep);
    assert_eq!(s.chamber[1].pending, Pending::at(Micros(1_001_000)));
}

/// DDD: an atrial Sense is tracked (PM5.6, PM11.1), a ventricular Sense
/// inhibits (PM5.5), and during an AV delay an atrial Sense is a Refractory
/// sense (PM11.6).
///
/// ```text
/// tick 0    AS 600,000   AS 700,000   VP 750,000         VS 1,200,000   VS 1,300,000
///            tracked      (refractory:  = AS + AV          inhibits       (refractory:
///                          AV delay)                       (VRP ended     VRP to
///                                                          1,070,000)     1,520,000)
/// armed:  AP 850,000   AP withdrawn             AP 1,600,000   AP 2,050,000
///         VP 1,000,000 VP 750,000               VP 1,750,000   VP 2,200,000
/// ```
#[test]
fn ddd_tracks_the_atrium_and_is_inhibited_by_the_ventricle() {
    let p = params(Mode::DDD);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (at(850_000), at(1_000_000)));

    let (s, a) = step(&p, &s, sense(A, 600_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(750_000)));
    assert_eq!(a.markers.atrial, sensed(600_000));
    assert_eq!(a.markers.ventricular, None);

    let (t, a) = step(&p, &s, sense(A, 700_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    assert_eq!(a.markers.atrial, refractory(700_000));
    assert_eq!(t, s, "Property P3");

    let (s, a) = step(&p, &s, paced(V, 750_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_600_000), at(1_750_000)));
    assert_eq!(a.markers.ventricular, pace_marker(750_000));

    let (s, a) = step(&p, &s, sense(V, 1_200_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_050_000), at(2_200_000)));
    assert_eq!(a.markers.ventricular, sensed(1_200_000));

    let (t, a) = step(&p, &s, sense(V, 1_300_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    assert_eq!(a.markers.ventricular, refractory(1_300_000));
    assert_eq!(t, s, "Property P3");

    // No atrial activity: the atrial Pace, then the AV delay. An atrial
    // Sense in it is a Refractory sense; a ventricular Sense in it (the
    // conducted beat) inhibits the ventricular Pace and starts the cycle.
    let (s, a) = step(&p, &s, paced(A, 2_050_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    let (t, a) = step(&p, &s, sense(A, 2_100_000));
    assert_eq!(a.markers.atrial, refractory(2_100_000));
    assert_eq!(t, s);
    let (s, a) = step(&p, &s, sense(V, 2_150_000));
    assert_eq!((a.atrial, a.ventricular), (at(3_000_000), at(3_150_000)));
    let _ = s;
}

/// The Sensed AV Delay Offset shortens the AV delay after a tracked Sense
/// and not after an atrial Pace (PM11.2, PM11.5).
#[test]
fn the_sensed_av_delay_is_the_paced_one_less_the_offset() {
    let p = Params { sensed_av_offset: 30_000, ..params(Mode::DDD) };
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    // The atrial Pace is one paced AV delay before the ventricular Pace.
    assert_eq!((a.atrial, a.ventricular), (at(850_000), at(1_000_000)));
    let (s, a) = step(&p, &s, sense(A, 600_000));
    assert_eq!(a.ventricular, at(720_000));
    let (s, a) = step(&p, &s, paced(V, 720_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_570_000), at(1_720_000)));
    // After the atrial Pace the delay is the paced one: nothing moves.
    let (_, a) = step(&p, &s, paced(A, 1_570_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
}

/// DDI: an atrial Sense inhibits the atrial Pace and is not tracked: the
/// ventricular Pace stays one escape interval after the last ventricular
/// event (PM5.5, PM9.2).
///
/// ```text
/// tick 0    AS 600,000      VP 1,000,000    AP 1,850,000   VP 2,000,000
///            AP withdrawn    (unmoved)
/// ```
#[test]
fn ddi_inhibits_the_atrial_pace_and_does_not_track() {
    let p = params(Mode::DDI);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (at(850_000), at(1_000_000)));

    let (s, a) = step(&p, &s, sense(A, 600_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, Pace::Keep));
    assert_eq!(a.markers.atrial, sensed(600_000));
    assert_eq!(s.chamber[1].pending, Pending::at(Micros(1_000_000)));

    // A second atrial Sense before the ventricular event changes nothing.
    let (t, a) = step(&p, &s, sense(A, 800_000));
    assert_eq!(a.markers.atrial, refractory(800_000));
    assert_eq!(t, s);

    let (s, a) = step(&p, &s, paced(V, 1_000_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_850_000), at(2_000_000)));
    let (s, a) = step(&p, &s, paced(A, 1_850_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    // A ventricular Sense inhibits as in DDD.
    let (_, a) = step(&p, &s, sense(V, 1_900_000));
    assert_eq!((a.atrial, a.ventricular), (at(2_750_000), at(2_900_000)));
}

/// VDD: the ventricle paced, the atrium sensed and tracked, never paced
/// (PM5.7). Without atrial activity it paces at the LRL; an atrial Sense
/// too late for its AV delay to end within the escape interval leaves the
/// ventricular Pace where the LRL has it (PM9.1).
///
/// ```text
/// tick 0   AS 400,000   VP 550,000        VP 1,550,000     AS 2,500,000   VP 2,550,000
///           tracked      = AS + AV         (LRL)            (AS + AV would be 2,650,000)
/// ```
#[test]
fn vdd_tracks_without_pacing_the_atrium_and_keeps_the_lrl() {
    let p = params(Mode::VDD);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(1_000_000)));
    assert!(!s.chamber[0].pending.armed);

    let (s, a) = step(&p, &s, sense(A, 400_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(550_000)));
    let (s, a) = step(&p, &s, paced(V, 550_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(1_550_000)));
    let (s, a) = step(&p, &s, paced(V, 1_550_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(2_550_000)));

    let (s, a) = step(&p, &s, sense(A, 2_500_000));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
    assert_eq!(a.markers.atrial, sensed(2_500_000));
    assert_eq!(s.chamber[1].pending, Pending::at(Micros(2_550_000)));
}

/// Both Paces fired before a Job ran: the tick accounts for the atrial
/// Pace, then for the ventricular Pace it led to, and arms nothing (their
/// `Paced` events will); each `Paced` event then arms its own Chamber from
/// the state, and the state is the one the events in order would have given.
///
/// And the case the generated run found (2026-11-23): a ventricular Pace
/// accounted by an event that arms nothing (a Sense the Mode does not
/// sense) leaves the next atrial Pace pending in the state and armed by
/// nothing, until the ventricular Pace's report arms it.
#[test]
fn two_paces_are_accounted_in_order_and_each_arms_itself_when_reported() {
    let p = params(Mode::DDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (s, a) = step(&p, &s, tick(1_000_500));
    assert_eq!(a, Arming::NONE);
    assert!(s.chamber[0].unconfirmed && s.chamber[1].unconfirmed);
    assert_eq!(s.chamber[0].pending, Pending::at(Micros(1_850_000)));
    assert_eq!(s.chamber[1].pending, Pending::at(Micros(2_000_000)));
    assert!(!s.atrial_since_ventricular);

    let (s, a) = step(&p, &s, paced(A, 850_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_850_000), Pace::Keep));
    assert_eq!(a.markers.atrial, pace_marker(850_000));
    // The report of a ventricular Pace that had been accounted arms the
    // atrial Pace it made pending: here a second time, at the same instant,
    // since the atrium's own report came first and armed it.
    let (s, a) = step(&p, &s, paced(V, 1_000_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_850_000), at(2_000_000)));
    assert!(!s.chamber[0].unconfirmed && !s.chamber[1].unconfirmed);

    // The found case: DOO, the ventricular Pace fired, a Sense comes first.
    let d = params(Mode::DOO);
    let (o, _) = step(&d, &State::POWER_ON_RESET, tick(0));
    let (o, _) = step(&d, &o, paced(A, 850_000));
    let (o, a) = step(&d, &o, sense(A, 1_000_200));
    assert_eq!(a, Arming::NONE);
    assert_eq!(o.chamber[0].pending, Pending::at(Micros(1_850_000)));
    let (_, a) = step(&d, &o, paced(V, 1_000_000));
    assert_eq!((a.atrial, a.ventricular), (at(1_850_000), at(2_000_000)));

    // The same events one at a time.
    let (o, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let (o, _) = step(&p, &o, paced(A, 850_000));
    let (o, _) = step(&p, &o, paced(V, 1_000_000));
    assert_eq!(s, o);
}

/// AAT: a non-refractory Sense puts a Pace of the Chamber at once (PM5.4).
/// The step gives the Sense's own instant; the hardware layer arms it one
/// lead after the present and says so; the Pace then starts the interval.
///
/// ```text
/// tick 0    AS 600,000 -> AP armed 600,700   AS 600,300    AP 600,700      AS 700,000
///                                             (ARP of AS)   next 1,600,700  (ARP of AP)
/// ```
#[test]
fn aat_paces_at_once_after_a_sense() {
    let p = params(Mode::AAT);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (at(1_000_000), Pace::Keep));

    let (s, a) = step(&p, &s, sense(A, 600_000));
    assert_eq!((a.atrial, a.ventricular), (at(600_000), Pace::Keep));
    assert_eq!(a.markers.atrial, sensed(600_000));
    // The hardware layer: armed at the present plus the lead.
    let s = rearmed(&s, A, Micros(600_700));
    assert_eq!(s.chamber[0].pending, Pending::at(Micros(600_700)));

    // A second Sense before the Pace is in the ARP the first one started,
    // and the Pace is not taken as delivered before its instant.
    let (t, a) = step(&p, &s, sense(A, 600_300));
    assert_eq!(a.markers.atrial, refractory(600_300));
    assert_eq!(a.atrial, Pace::Keep);
    assert_eq!(t, s);

    let (s, a) = step(&p, &s, paced(A, 600_700));
    assert_eq!(a.atrial, at(1_600_700));
    assert_eq!(a.markers.atrial, pace_marker(600_700));

    let (t, a) = step(&p, &s, sense(A, 700_000));
    assert_eq!(a.markers.atrial, refractory(700_000));
    assert_eq!(t, s);
    // The ventricle is not sensed.
    let (t, a) = step(&p, &s, sense(V, 900_000));
    assert_eq!(a, Arming::NONE);
    assert_eq!(t, s);
    // After the ARP (to 850,700) a Sense triggers again.
    let (_, a) = step(&p, &s, sense(A, 900_000));
    assert_eq!(a.atrial, at(900_000));
}

/// VVT: the same in the ventricle, with the VRP. Where the hardware layer's
/// report of the armed instant did not reach the state, the Pace's own
/// report restarts the interval from where it was delivered.
#[test]
fn vvt_paces_at_once_after_a_sense() {
    let p = params(Mode::VVT);
    let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(1_000_000)));
    let (s, a) = step(&p, &s, sense(V, 600_000));
    assert_eq!(a.ventricular, at(600_000));
    assert_eq!(a.markers.ventricular, sensed(600_000));

    // No `rearmed`: a tick takes the Pace as delivered at the Sense.
    let (s, a) = step(&p, &s, tick(600_500));
    assert_eq!(a, Arming::NONE);
    assert!(s.chamber[1].unconfirmed);
    let (s, a) = step(&p, &s, paced(V, 600_700));
    assert_eq!(a.ventricular, at(1_600_700));
    assert_eq!(a.markers.ventricular, pace_marker(600_700));
    // The VRP runs from the Pace: to 920,700.
    let (t, a) = step(&p, &s, sense(V, 900_000));
    assert_eq!(a.markers.ventricular, refractory(900_000));
    assert_eq!(t, s);
    let (_, a) = step(&p, &s, sense(V, 950_000));
    assert_eq!(a.ventricular, at(950_000));
}

/// `rearmed` moves a pending Pace and nothing else.
#[test]
fn rearmed_changes_only_a_pending_instant() {
    let p = params(Mode::DDD);
    let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
    let t = rearmed(&s, V, Micros(1_000_500));
    assert_eq!(t.chamber[1].pending, Pending::at(Micros(1_000_500)));
    assert_eq!(State { chamber: [t.chamber[0], s.chamber[1]], ..t }, s);
    // Nothing pending: nothing changes.
    assert_eq!(rearmed(&State::POWER_ON_RESET, A, Micros(5)), State::POWER_ON_RESET);
}

// A generated run. The harness keeps what the compares would hold: an
// instant per Chamber, set by `Pace::At`, cleared by `Pace::Withdraw` and
// by firing. A Chamber whose Pace has fired and is not yet reported takes
// no arming (the hardware layer's `TooLate`).

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
}

struct Board {
    /// The present, for an arming at an instant already past.
    now: u32,
    /// Whether the Chamber's armed instant is one the driver moved.
    moved: [bool; 2],
    armed: [Option<u32>; 2],
    /// Fired and not reported: the instant.
    fired: [Option<u32>; 2],
}

impl Board {
    /// The arming applied as the driver applies it: an instant that is
    /// not ahead of the present is moved one lead (500 µs) after it, and
    /// the state is told (`rearmed`), as the Partition does.
    fn apply(&mut self, a: &Arming, s: &mut State) {
        for (k, pace) in [a.atrial, a.ventricular].into_iter().enumerate() {
            if self.fired[k].is_some() {
                continue;
            }
            match pace {
                Pace::Keep => {}
                Pace::Withdraw => self.armed[k] = None,
                Pace::At { at, .. } if at.0 > self.now => {
                    self.armed[k] = Some(at.0);
                    self.moved[k] = false;
                }
                Pace::At { .. } => {
                    let at = self.now + 500;
                    self.armed[k] = Some(at);
                    self.moved[k] = true;
                    *s = rearmed(s, if k == 0 { A } else { V }, Micros(at));
                }
            }
        }
    }
    /// The compares that fire by `now`, the atrium's first when both do
    /// (it is the earlier in every cycle the step makes).
    fn run_to(&mut self, now: u32) {
        self.now = now;
        for k in 0..2 {
            if let Some(x) = self.armed[k] {
                if x <= now {
                    self.armed[k] = None;
                    self.fired[k] = Some(x);
                }
            }
        }
    }
}

/// What a generated run met.
#[derive(Default, Debug, Clone, Copy)]
struct Met {
    paces: u32,
    tracked: u32,
    refractory: u32,
    blanked: u32,
    pvcs: u32,
    extended: u32,
    delayed_to_the_url: u32,
}

fn run(mode: Mode, seed: u64, events: usize) -> Met {
    // The set varies with the seed: the offset, the dynamic AV delay, the
    // URL (never a longer interval than the LRL's) and PVARP Extension.
    let p = Params {
        sensed_av_offset: if seed % 3 == 0 { 30_000 } else { 0 },
        dynamic_av: seed % 2 == 0,
        url: [interval_of_ppm(120), interval_of_ppm(90), interval_of_ppm(175)][(seed % 5 % 3) as usize],
        pvarp_extension: if seed % 7 < 4 { 100_000 } else { 0 },
        ..params(mode)
    };
    let mut met = Met::default();
    let e = p.lrl;
    let mut r = Rng(seed | 1);
    let mut board = Board { now: 0, moved: [false; 2], armed: [None; 2], fired: [None; 2] };
    let (mut s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
    board.apply(&a, &mut s);
    let mut now = 0u32;
    let mut paces = 0;
    // The last ventricular event the harness saw, for Property P2.
    let mut last_v: Option<u32> = None;
    for _ in 0..events {
        // A fast heart in some runs, a slow one (more Paces) in others.
        now += 1 + r.below(if seed % 4 < 2 { 250_000 } else { 900_000 }) as u32;
        board.run_to(now);
        // Most of the time the Paces are reported before the next event,
        // the earlier first; sometimes the event comes first.
        let report_first = r.below(4) != 0;
        let report = |s: &mut State, board: &mut Board, paces: &mut u32, last_v: &mut Option<u32>| {
            let mut order = [0usize, 1];
            if let (Some(x), Some(y)) = (board.fired[0], board.fired[1]) {
                if y < x {
                    order = [1, 0];
                }
            }
            for k in order {
                if let Some(x) = board.fired[k].take() {
                    let c = if k == 0 { A } else { V };
                    let (t, a) = step(&p, s, paced(c, x));
                    assert!(matches!(a.markers.atrial.or(a.markers.ventricular), Some((Marker::Pace { fused: false }, _))), "{mode:?} seed {seed}: a Pace the state did not expect");
                    *s = t;
                    board.apply(&a, s);
                    *paces += 1;
                    if k == 1 {
                        // A report may come after a later ventricular Sense.
                        *last_v = Some(last_v.map_or(x, |l| l.max(x)));
                    }
                }
            }
        };
        if report_first {
            report(&mut s, &mut board, &mut paces, &mut last_v);
        }
        let ev = match r.below(5) {
            0 | 1 => sense(A, now),
            2 | 3 => sense(V, now),
            _ => tick(now),
        };
        let before = accounted(&p, &s, Micros(now));
        let (t, a) = step(&p, &s, ev);
        // Property P1: a ventricular Pace the step arms is no earlier than
        // one URL interval after the last ventricular event (the Modes here
        // have no triggered response).
        if let (Pace::At { at, .. }, Some(l)) = (a.ventricular, last_v) {
            let l = if matches!(a.markers.ventricular, Some((Marker::Sense { refractory: false }, _))) { now } else { l };
            assert!(at.0 >= l + p.url, "{mode:?} seed {seed}: a ventricular Pace at {} within the URL interval of the event at {l}", at.0);
            if at.0 == l + p.url && p.url < p.lrl {
                met.delayed_to_the_url += 1;
            }
        }
        // Property P8: the PVARP is extended in at most one of two
        // successive cycles; and only after a PVC, with the extension on.
        if t.pvarp_extended && !before.pvarp_extended {
            assert!(matches!(a.markers.augmentation, Some((Marker::Pvc, _))) && p.pvarp_extension != 0 && p.mode.tracks());
            met.extended += 1;
        }
        assert!(!(t.pvarp_extended && t.pvarp_extended_last_cycle), "{mode:?} seed {seed}: Property P8");
        if matches!(a.markers.augmentation, Some((Marker::Pvc, _))) {
            met.pvcs += 1;
        }
        if let Event::Sense { chamber, .. } = ev {
            let m = match chamber {
                Chamber::A => a.markers.atrial,
                Chamber::V => a.markers.ventricular,
            };
            match m {
                Some((Marker::Sense { refractory: true }, _)) => {
                    // Property P3.
                    assert_eq!(t, before, "{mode:?} seed {seed}");
                    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
                    met.refractory += 1;
                }
                Some((Marker::Sense { refractory: false }, _)) => {
                    if matches!(chamber, Chamber::V) {
                        last_v = Some(now);
                    } else if matches!(p.mode.response, Response::Tracked) {
                        met.tracked += 1;
                    }
                }
                None if p.mode.senses(chamber) => {
                    // A ventricular Sense in the blanking interval after an
                    // atrial Pace: discarded (PM4.5; Property P3).
                    let v = before.chamber[1];
                    assert!(matches!(chamber, Chamber::V) && v.blanked && now < v.blanking_until.0, "{mode:?} seed {seed}");
                    assert_eq!(t, before);
                    assert_eq!(a, Arming::NONE);
                    met.blanked += 1;
                }
                None => {}
                other => panic!("{other:?}"),
            }
        }
        s = t;
        board.apply(&a, &mut s);
        if !report_first {
            report(&mut s, &mut board, &mut paces, &mut last_v);
        }

        // With every Pace reported, the compares are what the state has
        // pending, in both Chambers.
        for k in 0..2 {
            let pending = s.chamber[k].pending;
            assert_eq!(board.armed[k], pending.armed.then_some(pending.at.0), "{mode:?} seed {seed} at {now}: Chamber {k}");
            assert!(!s.chamber[k].unconfirmed);
        }
        // Property P2 for the ventricle: a Pace is pending, no later than
        // one escape interval after the last ventricular event.
        let v = s.chamber[1].pending;
        assert!(v.armed, "{mode:?} seed {seed}");
        if let Some(l) = last_v {
            assert!(v.at.0 <= l + e, "{mode:?} seed {seed}: ventricular Pace at {} after the event at {l}", v.at.0);
            assert!(v.at.0 > l);
        }
        // The atrium has a Pace pending exactly while no atrial event has
        // come since the last ventricular event, in a Mode that paces it,
        // and one AV delay before the end of the escape interval.
        let a_pending = s.chamber[0].pending;
        assert_eq!(a_pending.armed, p.mode.paces(A) && !s.atrial_since_ventricular, "{mode:?} seed {seed}");
        if a_pending.armed && !board.moved[0] {
            if let Some(l) = last_v {
                // One paced AV delay before the end of the escape interval:
                // the fixed delay, or a dynamic one within its limits.
                let av = l + e - a_pending.at.0;
                if p.dynamic_av && p.mode.tracks() {
                    assert!((p.min_dynamic_av..=p.fixed_av).contains(&av), "{mode:?} seed {seed}: AV delay {av}");
                } else {
                    assert_eq!(av, p.fixed_av);
                }
            }
        }
    }
    met.paces = paces;
    met
}

#[test]
fn generated_runs_keep_the_compares_and_the_state_together() {
    let mut totals = [Met::default(); 4];
    for (m, mode) in [Mode::DOO, Mode::DDI, Mode::VDD, Mode::DDD].into_iter().enumerate() {
        for seed in 1..=200u64 {
            let r = run(mode, seed * 7919, 400);
            let t = &mut totals[m];
            t.paces += r.paces;
            t.tracked += r.tracked;
            t.refractory += r.refractory;
            t.blanked += r.blanked;
            t.pvcs += r.pvcs;
            t.extended += r.extended;
            t.delayed_to_the_url += r.delayed_to_the_url;
        }
    }
    let [doo, ddi, vdd, ddd] = totals;
    // Every Mode paced; the tracking Modes tracked, and had Paces delayed
    // to the URL; the sensing Modes had Refractory senses, PVCs and
    // extended PVARPs; the Modes that pace the atrium and sense the
    // ventricle had blanked Senses; DOO sensed nothing.
    assert!(totals.iter().all(|t| t.paces > 1_000), "{totals:?}");
    assert!(vdd.tracked > 1_000 && ddd.tracked > 1_000, "{totals:?}");
    assert!(vdd.delayed_to_the_url > 100 && ddd.delayed_to_the_url > 100, "{totals:?}");
    assert_eq!((doo.tracked, doo.refractory, doo.blanked, doo.pvcs), (0, 0, 0, 0));
    assert_eq!((ddi.tracked, vdd.blanked), (0, 0));
    assert!(ddi.refractory > 1_000 && vdd.refractory > 1_000 && ddd.refractory > 1_000, "{totals:?}");
    assert!(ddi.blanked > 20 && ddd.blanked > 20, "{totals:?}");
    assert!(ddi.pvcs > 500 && vdd.pvcs > 500 && ddd.pvcs > 500, "{totals:?}");
    // PVARP Extension is the tracking Modes' (Table 19-3): DDI marks its
    // PVCs and extends nothing.
    assert!(vdd.extended > 100 && ddd.extended > 100, "{totals:?}");
    assert_eq!(ddi.extended, 0);
}
