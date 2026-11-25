//! Host tests of the State machine in `step` (task 16.2; Property P4):
//! every transition, and every transition that must not happen. The step
//! is called as the Partition calls it, with the parameters the state has
//! in force. An integration test, so that the crate itself names no `std`.
#![forbid(unsafe_code)]

use pm_core::*;

const A: Chamber = Chamber::A;
const V: Chamber = Chamber::V;

fn go(s: &State, e: Event) -> (State, Arming) {
    let p = *s.params();
    step(&p, s, e)
}

fn tick(now: u32, magnet: bool, link_up: bool) -> Event {
    Event::Tick { now: Micros(now), activity: 0, magnet, battery: Battery::Bol, link_up }
}

fn sense(c: Chamber, at: u32) -> Event {
    Event::Sense { chamber: c, at: Micros(at), edges: 1 }
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

fn at(us: u32, width: u32) -> Pace {
    Pace::At { at: Micros(us), width }
}

/// The transitions the State machine has (design, "Therapy State Machine").
fn allowed(from: Therapy, to: Therapy) -> bool {
    use Therapy::*;
    from == to
        || matches!(
            (from, to),
            (PowerOnReset, Permanent | PaceNow | Magnet)
                | (Permanent, Temporary | PaceNow | Magnet)
                | (Temporary, Permanent | PaceNow | Magnet)
                | (PaceNow, Permanent | Magnet)
                | (Magnet, _)
        )
}

const POR_E: u32 = 60_000_000 / 65;

/// Power-On Reset (PM7.1, PM7.4): VVI at 65 ppm; left only by a programmed
/// set (at the next ventricular event), Pace-Now, or the magnet.
#[test]
fn power_on_reset_is_left_by_a_programmed_set_at_the_next_ventricular_event() {
    let por = State::POWER_ON_RESET;
    assert_eq!(por.params().lrl, POR_E);
    let (s, a) = go(&por, tick(0, false, false));
    assert_eq!(a.ventricular, at(POR_E, Params::POWER_ON_RESET.width[1]));
    assert_eq!(s.therapy, Therapy::PowerOnReset);

    // What does not end it: temporary pacing, its end, the link, a Sense.
    for e in [Event::Command(Command::StartTemporary(set(Mode::VOO, 100))), Event::Command(Command::EndTemporary), tick(20_000, false, true), sense(V, 100_000), sense(A, 100_000)] {
        let (t, _) = go(&s, e);
        assert_eq!(t.therapy, Therapy::PowerOnReset, "{e:?}");
        assert!(!t.change_waiting, "{e:?}");
        assert!(!t.temporary_wanted);
    }
    // Temporary pacing asked for with the link up is still refused.
    let (up, _) = go(&s, tick(20_000, false, true));
    let (t, _) = go(&up, Event::Command(Command::StartTemporary(set(Mode::VOO, 100))));
    assert!(!t.temporary_wanted && !t.change_waiting);

    // A programmed set: nothing changes until the ventricular event.
    let ddd = Params::NOMINAL;
    let (w, a) = go(&s, Event::Command(Command::Program(ddd)));
    assert_eq!(a, Arming::NONE);
    assert_eq!(w.therapy, Therapy::PowerOnReset);
    assert!(w.change_waiting && w.staged_valid && !w.programmed);
    assert_eq!(w.params().lrl, POR_E, "the Power-On Reset parameters are in force until then");
    // A tick, an atrial Sense (not sensed in VVI) and a Refractory sense
    // are not that event.
    let (w2, _) = go(&w, tick(40_000, false, false));
    assert_eq!(w2.therapy, Therapy::PowerOnReset);
    let (w2, _) = go(&w2, sense(A, 300_000));
    assert_eq!(w2.therapy, Therapy::PowerOnReset);
    // The Pace: Permanent from here, DDD at 60 ppm, both Chambers armed.
    let (t, a) = go(&w2, paced(V, POR_E));
    assert_eq!(t.therapy, Therapy::Permanent);
    assert!(t.programmed && !t.staged_valid && !t.change_waiting);
    assert_eq!(*t.params(), ddd);
    assert_eq!((a.atrial, a.ventricular), (at(POR_E + 850_000, 400), at(POR_E + 1_000_000, 400)));
    assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: false }, Micros(POR_E))));

    // The same at a ventricular Sense outside the VRP.
    let (t, a) = go(&w, sense(V, 500_000));
    assert_eq!(t.therapy, Therapy::Permanent);
    assert_eq!((a.atrial, a.ventricular), (at(1_350_000, 400), at(1_500_000, 400)));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: false }, Micros(500_000))));
    // And not at one inside it.
    let (r, _) = go(&w, paced(V, POR_E));
    let (r, _) = go(&r, Event::Command(Command::Program(set(Mode::VVI, 90))));
    let (r2, a) = go(&r, sense(V, POR_E + 100_000));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: true }, Micros(POR_E + 100_000))));
    assert_eq!(r2.params().mode, Mode::DDD);
    assert!(r2.change_waiting);
}

/// Permanent (PM6.2): a newly programmed set is in force from the next
/// ventricular event, and the interval that event starts is the new one.
#[test]
fn a_set_programmed_in_the_permanent_state_is_in_force_from_the_next_event() {
    let s = permanent(set(Mode::VVI, 60));
    let (s, a) = go(&s, tick(0, false, false));
    assert_eq!(a.ventricular, at(1_000_000, 400));
    let (s, _) = go(&s, Event::Command(Command::Program(set(Mode::VVI, 90))));
    assert_eq!(s.params().lrl, 1_000_000);
    let (s, a) = go(&s, paced(V, 1_000_000));
    assert_eq!(s.therapy, Therapy::Permanent);
    assert_eq!(a.ventricular, at(1_666_666, 400));
    assert_eq!(s.params().lrl, 666_666);
}

/// Temporary (PM6.3, PM6.4): entered from Permanent with the link up; the
/// permanent parameters are untouched; left by the command, by the loss of
/// the link, and by Pace-Now.
#[test]
fn temporary_pacing_begins_and_ends() {
    let perm = set(Mode::VVI, 60);
    let temp = set(Mode::VOO, 120);
    let (s, _) = go(&permanent(perm), tick(0, false, true));

    // Without the link the command is without effect.
    let (down, _) = go(&permanent(perm), tick(0, false, false));
    let (t, _) = go(&down, Event::Command(Command::StartTemporary(temp)));
    assert!(!t.temporary_wanted && !t.change_waiting);

    let (w, a) = go(&s, Event::Command(Command::StartTemporary(temp)));
    assert_eq!(a, Arming::NONE);
    assert_eq!((w.therapy, w.change_waiting), (Therapy::Permanent, true));
    assert_eq!(*w.params(), perm);
    let (t, a) = go(&w, paced(V, 1_000_000));
    assert_eq!(t.therapy, Therapy::Temporary);
    assert_eq!(*t.params(), temp);
    assert_eq!(t.permanent, perm, "PM6.3");
    assert_eq!(a.ventricular, at(1_500_000, 400));
    // A second start while it runs is without effect.
    let (t2, _) = go(&t, Event::Command(Command::StartTemporary(set(Mode::VOO, 150))));
    assert_eq!(t2, t);

    // Ended by the command.
    let (e, _) = go(&t, Event::Command(Command::EndTemporary));
    assert_eq!((e.therapy, e.change_waiting), (Therapy::Temporary, true));
    let (e, a) = go(&e, paced(V, 1_500_000));
    assert_eq!(e.therapy, Therapy::Permanent);
    assert_eq!(*e.params(), perm);
    assert_eq!(a.ventricular, at(2_500_000, 400));

    // Ended by the loss of the link.
    let (l, _) = go(&t, tick(1_020_000, false, false));
    assert!(l.change_waiting);
    let (l, _) = go(&l, paced(V, 1_500_000));
    assert_eq!(l.therapy, Therapy::Permanent);
    assert!(!l.temporary_wanted);
    // The link coming back does not start it again.
    let (l, _) = go(&l, tick(1_520_000, false, true));
    assert!(!l.change_waiting);

    // Ended by Pace-Now, at once.
    let (n, _) = go(&t, Event::Command(Command::PaceNow));
    assert_eq!(n.therapy, Therapy::PaceNow);
    assert!(!n.temporary_wanted);
}

/// Pace-Now (PM6.5, PM6.7): entered at once from every State; what was
/// pending is withdrawn and the next tick arms the first pulse one Pace-Now
/// escape interval ahead; it lasts until a set is programmed.
#[test]
fn pace_now_is_entered_at_once_and_left_by_a_programmed_set() {
    let pn = Params::PACE_NOW;
    assert_eq!((pn.mode, pn.lrl, pn.width[1]), (Mode::VVI, POR_E, 1_000));
    let ddd = permanent(Params::NOMINAL);
    let (ddd, _) = go(&ddd, tick(0, false, true));
    // (Each Pace is reported in its turn, the atrial one first.)
    let (temp, _) = go(&ddd, Event::Command(Command::StartTemporary(set(Mode::VOO, 100))));
    let (temp, _) = go(&temp, paced(A, 850_000));
    let (temp, _) = go(&temp, paced(V, 1_000_000));
    let (magnet, _) = go(&ddd, tick(20_000, true, true));
    let (magnet, _) = go(&magnet, paced(A, 850_000));
    let (magnet, _) = go(&magnet, paced(V, 1_000_000));
    assert_eq!((temp.therapy, magnet.therapy), (Therapy::Temporary, Therapy::Magnet));
    let (por, _) = go(&State::POWER_ON_RESET, tick(0, false, false));

    for from in [por, ddd, temp, magnet] {
        let (s, a) = go(&from, Event::Command(Command::PaceNow));
        assert_eq!(s.therapy, Therapy::PaceNow, "from {:?}", from.therapy);
        assert_eq!(*s.params(), pn);
        assert!(!s.chamber[0].pending.armed && !s.chamber[1].pending.armed);
        assert!(!s.change_waiting && !s.temporary_wanted && !s.staged_valid);
        // Every Pace that was pending is withdrawn.
        for (k, pace) in [a.atrial, a.ventricular].into_iter().enumerate() {
            assert_eq!(pace, if from.chamber[k].pending.armed { Pace::Withdraw } else { Pace::Keep });
        }
        // The next tick: the first Pace-Now pulse, 1.00 ms wide.
        let (s, a) = go(&s, tick(2_000_000, s.magnet, s.link_up));
        assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(2_000_000 + POR_E, 1_000)));

        // It stays: the commands of temporary pacing, the magnet, the link.
        for e in [Event::Command(Command::PaceNow), Event::Command(Command::EndTemporary), Event::Command(Command::StartTemporary(set(Mode::VOO, 100))),
                  tick(2_020_000, true, true), tick(2_020_000, false, false), paced(V, 2_000_000 + POR_E), sense(V, 2_500_000)] {
            let (t, _) = go(&s, e);
            assert_eq!(t.therapy, Therapy::PaceNow, "{e:?}");
            let (t, _) = go(&t, paced(V, 2_000_000 + POR_E));
            assert_eq!(t.therapy, Therapy::PaceNow, "{e:?}, then a Pace");
        }
        // A programmed set ends it at the next ventricular event (PM6.7).
        let (t, _) = go(&s, Event::Command(Command::Program(set(Mode::VVI, 80))));
        assert_eq!(t.therapy, Therapy::PaceNow);
        // Where the magnet is still applied the Permanent State is entered
        // under it, in the same event.
        let (t, a) = go(&t, paced(V, 2_000_000 + POR_E));
        if s.magnet {
            assert_eq!((t.therapy, t.before_magnet), (Therapy::Magnet, Therapy::Permanent));
        } else {
            assert_eq!(t.therapy, Therapy::Permanent);
        }
        // The programmed set is the State's own. Under the magnet it is
        // paced asynchronously at the magnet's rate (task 16.4; until then
        // this test expected the set's own rate here too).
        assert_eq!(t.own().lrl, 750_000);
        let escape = if s.magnet { 600_000 } else { 750_000 };
        assert_eq!((t.params().lrl, t.params().mode), (escape, if s.magnet { Mode::VOO } else { Mode::VVI }));
        assert_eq!(a.ventricular, at(2_000_000 + POR_E + escape, 400));
    }
    // A Pace-Now command given before the programmed set is in force
    // drops the set.
    let (s, _) = go(&ddd, Event::Command(Command::Program(set(Mode::VVI, 80))));
    let (s, _) = go(&s, Event::Command(Command::PaceNow));
    let (s, _) = go(&s, tick(40_000, false, true));
    let (s, _) = go(&s, paced(V, 40_000 + POR_E));
    assert_eq!(s.therapy, Therapy::PaceNow);
}

/// Magnet (PM8.1, PM8.3): entered at the next ventricular event after the
/// magnet is applied, from Power-On Reset, Permanent and Temporary; it
/// remembers the State it interrupted and returns to it. A set programmed
/// under the magnet is the State returned to. Pace-Now is not interrupted.
#[test]
fn the_magnet_interrupts_and_returns() {
    let perm = set(Mode::VVI, 60);
    let (p, _) = go(&permanent(perm), tick(0, false, true));
    let (t, _) = go(&p, Event::Command(Command::StartTemporary(set(Mode::VOO, 120))));
    let (t, _) = go(&t, paced(V, 1_000_000));
    let (por, _) = go(&State::POWER_ON_RESET, tick(0, false, false));

    for from in [por, p, t] {
        let before = from.therapy;
        let link = from.link_up;
        let (s, a) = go(&from, tick(1_020_000, true, link));
        assert_eq!(a, Arming::NONE);
        assert_eq!((s.therapy, s.change_waiting), (before, true));
        let due = s.chamber[1].pending.at.0;
        let (s, _) = go(&s, paced(V, due));
        assert_eq!((s.therapy, s.before_magnet), (Therapy::Magnet, before));
        // The set of the State interrupted is kept (task 16.4: `own`; until
        // then `params` was that set), and paced asynchronously at 100 ppm.
        assert_eq!(*s.own(), *from.params(), "the parameters of the State interrupted");
        assert_eq!(*s.params(), Params { mode: Mode::VOO, lrl: 600_000, ..*from.params() });
        // Held while the magnet is.
        let (h, _) = go(&s, tick(due + 20_000, true, link));
        assert!(!h.change_waiting);
        // Released: back at the next ventricular event.
        let (r, _) = go(&s, tick(due + 20_000, false, link));
        assert_eq!((r.therapy, r.change_waiting), (Therapy::Magnet, true));
        let due = r.chamber[1].pending.at.0;
        let (r, _) = go(&r, paced(V, due));
        assert_eq!((r.therapy, r.before_magnet), (before, Therapy::PowerOnReset));
    }

    // Programmed under the magnet from Power-On Reset: Permanent on release.
    let (s, _) = go(&por, tick(20_000, true, false));
    let (s, _) = go(&s, paced(V, POR_E));
    let (s, _) = go(&s, Event::Command(Command::Program(perm)));
    let (s, _) = go(&s, paced(V, 2 * POR_E));
    assert_eq!((s.therapy, s.before_magnet), (Therapy::Magnet, Therapy::Permanent));
    assert_eq!(*s.own(), perm);
    let (s, _) = go(&s, tick(2 * POR_E + 20_000, false, false));
    let (s, _) = go(&s, paced(V, 2 * POR_E + 1_000_000));
    assert_eq!(s.therapy, Therapy::Permanent);

    // Temporary pacing whose link is lost under the magnet: Permanent on release.
    let (s, _) = go(&t, tick(1_020_000, true, true));
    let (s, _) = go(&s, paced(V, 1_500_000));
    assert_eq!((s.therapy, s.before_magnet), (Therapy::Magnet, Therapy::Temporary));
    let (s, _) = go(&s, tick(1_520_000, true, false));
    let (s, _) = go(&s, paced(V, 2_000_000));
    assert_eq!((s.therapy, s.before_magnet), (Therapy::Magnet, Therapy::Permanent));
}

/// A Mode that paces nothing has no timing event to wait for: the change
/// is in force at the next tick. And into such a Mode every pending Pace
/// is withdrawn.
#[test]
fn a_change_from_and_to_a_mode_that_paces_nothing() {
    let (s, _) = go(&permanent(Params::NOMINAL), tick(0, false, false));
    let (s, _) = go(&s, Event::Command(Command::Program(set(Mode::OFF, 60))));
    let (s, a) = go(&s, paced(V, 1_000_000));
    assert_eq!(s.params().mode, Mode::OFF);
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep), "the atrial Pace was spent and the ventricular one has just fired");
    assert!(!s.chamber[0].pending.armed && !s.chamber[1].pending.armed);
    let (s, a) = go(&s, tick(1_020_000, false, false));
    assert_eq!(a, Arming::NONE);

    let (s, _) = go(&s, Event::Command(Command::Program(set(Mode::VVI, 60))));
    assert!(s.change_waiting);
    let (s, a) = go(&s, tick(1_040_000, false, false));
    assert_eq!(s.params().mode, Mode::VVI);
    assert_eq!(a.ventricular, at(2_040_000, 400));
    assert!(!s.change_waiting);

    // From DDD with the atrial Pace still pending, at a ventricular Sense:
    // into VVI the atrial Pace is withdrawn.
    let (s, _) = go(&permanent(Params::NOMINAL), tick(0, false, false));
    let (s, _) = go(&s, Event::Command(Command::Program(set(Mode::VVI, 60))));
    let (s, a) = go(&s, sense(V, 500_000));
    assert_eq!(s.params().mode, Mode::VVI);
    assert_eq!((a.atrial, a.ventricular), (Pace::Withdraw, at(1_500_000, 400)));
}

/// In a Mode of the atrium alone the timing event is the atrial one (PM9.3).
#[test]
fn in_an_atrial_mode_the_change_waits_for_an_atrial_event() {
    let (s, a) = go(&permanent(set(Mode::AAI, 60)), tick(0, false, false));
    assert_eq!(a.atrial, at(1_000_000, 400));
    let (s, _) = go(&s, Event::Command(Command::Program(set(Mode::VVI, 60))));
    // A ventricular Sense is not sensed in AAI and changes nothing.
    let (s, _) = go(&s, sense(V, 400_000));
    assert_eq!(s.params().mode, Mode::AAI);
    // The atrial Pace: VVI from here, one escape interval ahead.
    let (s, a) = go(&s, paced(A, 1_000_000));
    assert_eq!(s.params().mode, Mode::VVI);
    assert_eq!((a.atrial, a.ventricular), (Pace::Keep, at(2_000_000, 400)));
}

struct Rng(u64);

impl Rng {
    fn below(&mut self, n: u64) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0 % n
    }
}

/// Property P4 over generated runs: commands, magnet and link changes and
/// events of the heart in any order. Every step's State is the State
/// before or one the State machine allows from it; exactly the transitions
/// of the design occur and no other; and in a Mode that paces, with every
/// Pace reported, a Pace is pending after every tick.
#[test]
fn generated_runs_make_only_the_transitions_of_the_state_machine() {
    use std::collections::BTreeSet;
    let sets = [set(Mode::VVI, 60), set(Mode::VOO, 100), Params::NOMINAL, set(Mode::AAI, 70), set(Mode::OFF, 60), set(Mode::VDD, 60)];
    let mut seen: BTreeSet<(u8, u8)> = BTreeSet::new();
    for seed in 1..=300u64 {
        let mut r = Rng(seed * 0x9E37_79B9 | 1);
        let mut s = State::POWER_ON_RESET;
        let mut now = 0u32;
        let (mut magnet, mut link) = (false, false);
        for _ in 0..400 {
            now += 1 + r.below(300_000) as u32;
            let e = match r.below(12) {
                0 => Event::Command(Command::Program(sets[r.below(6) as usize])),
                1 => Event::Command(Command::StartTemporary(sets[r.below(6) as usize])),
                2 => Event::Command(Command::EndTemporary),
                3 => {
                    if r.below(4) == 0 { Event::Command(Command::PaceNow) } else { tick(now, magnet, link) }
                }
                4 => {
                    magnet = !magnet;
                    tick(now, magnet, link)
                }
                5 => {
                    link = !link;
                    tick(now, magnet, link)
                }
                6 | 7 => sense(if r.below(2) == 0 { A } else { V }, now),
                _ => {
                    // A Pace that is due, if one is; else a tick.
                    let due = [A, V].into_iter().find(|c| {
                        let p = s.chamber[c.index()].pending;
                        p.armed && p.at.0 <= now
                    });
                    match due {
                        Some(c) => paced(c, s.chamber[c.index()].pending.at.0),
                        None => tick(now, magnet, link),
                    }
                }
            };
            let from = s.therapy;
            let (t, _) = go(&s, e);
            assert!(allowed(from, t.therapy), "seed {seed}: {from:?} to {:?} at {e:?}", t.therapy);
            if from != t.therapy {
                seen.insert((from as u8, t.therapy as u8));
            }
            // Exactly one State, and the Magnet State remembers a State
            // that is not itself; no other State remembers one.
            if t.therapy == Therapy::Magnet {
                assert_ne!(t.before_magnet, Therapy::Magnet);
            } else {
                assert_eq!(t.before_magnet, Therapy::PowerOnReset);
            }
            // The permanent parameters change only by a programmed set
            // coming into force (PM6.3).
            if t.permanent != s.permanent {
                assert!(s.staged_valid && !t.staged_valid && t.permanent == s.staged);
            }
            if let Event::Tick { .. } = e {
                let p = *t.params();
                if !t.change_waiting || p.mode.paces(V) || p.mode.paces(A) {
                    if p.mode.paces(V) {
                        assert!(t.chamber[1].pending.armed, "seed {seed}");
                    } else if p.mode.paces(A) {
                        assert!(t.chamber[0].pending.armed, "seed {seed}");
                    }
                }
            }
            s = t;
        }
    }
    // Every transition of the design was made, and (by the assertion
    // above) no other.
    let all: BTreeSet<(u8, u8)> = (0..5u8).flat_map(|a| (0..5u8).map(move |b| (a, b))).filter(|(a, b)| a != b).filter(|(a, b)| {
        let t = |x: u8| [Therapy::PowerOnReset, Therapy::Permanent, Therapy::Temporary, Therapy::PaceNow, Therapy::Magnet][x as usize];
        allowed(t(*a), t(*b))
    }).collect();
    assert_eq!(all.len(), 15);
    assert_eq!(seen, all, "transitions made and not made");
}
