//! The step function (design PD-05, PD-06).
//!
//! Stage E1 (task 6.3): the single-chamber cycle. A Chamber the Mode paces
//! has a Pace pending one escape interval after its last event; the escape
//! interval is the LRL's; a non-refractory Sense in a Mode whose response is
//! I starts the interval again, which inhibits the pending Pace by moving
//! it; a Sense inside the Chamber's refractory period changes nothing.
//!
//! The step reads the Mode by its letters. What stage E1 does not deliver
//! (a Mode that paces both Chambers or senses a Chamber it does not pace,
//! the responses T and D, blanking, PVARP, hysteresis, the sensor rate, the
//! State machine) is refused by `validate` and is the work of tasks 11, 16
//! and 19. Given such a Mode all the same, each paced Chamber is timed from
//! its own events and a response other than I leaves pending Paces alone.

use crate::event::{Arming, Event, Marker, Pace};
use crate::mode::{Chamber, Response};
use crate::params::Params;
use crate::state::{Pending, State};
use crate::time::Micros;

const CHAMBERS: [Chamber; 2] = [Chamber::A, Chamber::V];

/// The escape interval in force: the LRL's (PM9.1).
const fn escape(p: &Params) -> u32 {
    p.lrl
}

/// The refractory period an event of `c` starts: the ARP in the atrium, the
/// VRP in the ventricle (PM12.1, PM12.2).
const fn refractory(p: &Params, c: Chamber) -> u32 {
    match c {
        Chamber::A => p.arp,
        Chamber::V => p.vrp,
    }
}

/// A Pace of `c` at `at`: the Chamber's last event, the start of its
/// refractory period and of the next escape interval (PM9.2, PM9.3).
fn apply_pace(p: &Params, st: &mut State, c: Chamber, at: Micros) {
    let ch = &mut st.chamber[c.index()];
    ch.seen = true;
    ch.last = at;
    ch.last_paced = true;
    ch.refractory_until = at.plus(refractory(p, c));
    ch.pending = if p.mode.paces(c) { Pending::at(at.plus(escape(p))) } else { Pending::NONE };
}

/// A pending Pace whose instant is at or before `instant` has been
/// delivered: the armed compare fires without software. Apply it to `c`
/// and remember that its `Paced` event is still to come.
fn account(p: &Params, st: &mut State, c: Chamber, instant: Micros) {
    let pending = st.chamber[c.index()].pending;
    if pending.armed && instant.is_at_or_after(pending.at) {
        apply_pace(p, st, c, pending.at);
        st.chamber[c.index()].unconfirmed = true;
    }
}

/// The state with every pending Pace due at or before `instant` applied
/// (design, "Events in the order they happened"). Every event is applied to
/// this state, so Property P3 is stated against it.
#[must_use]
pub fn accounted(p: &Params, s: &State, instant: Micros) -> State {
    let mut st = *s;
    for c in CHAMBERS {
        account(p, &mut st, c, instant);
    }
    st
}

fn set_pace(a: &mut Arming, c: Chamber, pace: Pace) {
    match c {
        Chamber::A => a.atrial = pace,
        Chamber::V => a.ventricular = pace,
    }
}

fn set_marker(a: &mut Arming, c: Chamber, marker: Marker, at: Micros) {
    match c {
        Chamber::A => a.markers.atrial = Some((marker, at)),
        Chamber::V => a.markers.ventricular = Some((marker, at)),
    }
}

/// The instruction that arms `c` as the state has it pending.
fn arm_from(p: &Params, st: &State, c: Chamber) -> Pace {
    let pending = st.chamber[c.index()].pending;
    if pending.armed {
        Pace::At { at: pending.at, width: p.width[c.index()] }
    } else {
        Pace::Keep
    }
}

/// One event applied to the state: the new state, and what to do with each
/// Chamber's compares. `p` is the parameter set in force.
///
/// Total, without side effect, allocation or loop over anything but the two
/// Chambers, in integer arithmetic.
#[must_use]
pub fn step(p: &Params, s: &State, e: Event) -> (State, Arming) {
    let mut out = Arming::NONE;
    match e {
        Event::Paced { chamber, at } => {
            let mut st = *s;
            // The other Chamber's Pace, if it was due by now.
            account(p, &mut st, chamber.other(), at);
            let ch = st.chamber[chamber.index()];
            // A Pace the state no longer had pending at this instant: a
            // Sense moved or withdrew it too late for the compare (PD-04).
            let expected = ch.unconfirmed || (ch.pending.armed && at.is_at_or_after(ch.pending.at));
            // A confirmation of a Pace already applied restarts the interval
            // from the instant the hardware reports (the same instant, or
            // later by the lead), unless a Sense has started it again since.
            if !ch.unconfirmed || ch.last_paced {
                apply_pace(p, &mut st, chamber, at);
            }
            st.chamber[chamber.index()].unconfirmed = false;
            set_pace(&mut out, chamber, arm_from(p, &st, chamber));
            set_marker(&mut out, chamber, Marker::Pace { fused: !expected }, at);
            (st, out)
        }
        Event::Sense { chamber, at, edges: _ } => {
            let mut st = accounted(p, s, at);
            if !p.mode.senses(chamber) {
                // PM5.7: a Chamber the Mode does not sense.
                return (st, out);
            }
            let ch = &mut st.chamber[chamber.index()];
            if ch.seen && at.is_before(ch.refractory_until) {
                // PM4.4: a Refractory sense inhibits and triggers nothing.
                set_marker(&mut out, chamber, Marker::Sense { refractory: true }, at);
                return (st, out);
            }
            ch.seen = true;
            ch.last = at;
            ch.last_paced = false;
            ch.refractory_until = at.plus(refractory(p, chamber));
            if p.mode.paces(chamber) && matches!(p.mode.response, Response::Inhibited) {
                // PM5.5: the pending Pace is cancelled; the next one is due
                // one escape interval after this Sense.
                ch.pending = Pending::at(at.plus(escape(p)));
                set_pace(&mut out, chamber, Pace::At { at: ch.pending.at, width: p.width[chamber.index()] });
            }
            set_marker(&mut out, chamber, Marker::Sense { refractory: false }, at);
            (st, out)
        }
        Event::Tick { now, .. } => {
            let mut st = accounted(p, s, now);
            for c in CHAMBERS {
                let ch = &mut st.chamber[c.index()];
                if p.mode.paces(c) {
                    if !ch.pending.armed {
                        // Pacing begins: the first Pace one escape interval from now.
                        ch.pending = Pending::at(now.plus(escape(p)));
                        set_pace(&mut out, c, Pace::At { at: ch.pending.at, width: p.width[c.index()] });
                    }
                } else if ch.pending.armed {
                    ch.pending = Pending::NONE;
                    set_pace(&mut out, c, Pace::Withdraw);
                }
            }
            (st, out)
        }
        // The State machine is stage E3 (task 16.2).
        Event::Command(_) => (*s, out),
    }
}

#[cfg(test)]
mod tests {
    extern crate std;
    use super::*;
    use crate::event::{Battery, Markers};
    use crate::mode::Mode;
    use crate::validate::{validate, word, NOMINAL_WORDS};
    use std::vec::Vec;

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

    fn at(us: u32, width: u32) -> Pace {
        Pace::At { at: Micros(us), width }
    }

    fn marker(a: &Arming, c: Chamber) -> Option<(Marker, Micros)> {
        match c {
            Chamber::A => a.markers.atrial,
            Chamber::V => a.markers.ventricular,
        }
    }

    // Hand-worked diagrams. Times in µs; nominal set: LRL 60 ppm
    // (1,000,000), VRP 320,000, ARP 250,000, widths 400.

    /// VOO: a Pace every LRL interval, whatever is sensed (PM5.3).
    ///
    /// ```text
    /// tick 1,000      VP 1,001,000      VP 2,001,000
    ///   |----- E -------|------- E -------|
    ///          VS 1,500,000 (not sensed)
    /// ```
    #[test]
    fn voo_paces_at_the_lrl_without_regard_to_senses() {
        let p = params(Mode::VOO);
        let (s, a) = step(&p, &State::POWER_ON_RESET, tick(1_000));
        assert_eq!(a.ventricular, at(1_001_000, 400));
        assert_eq!(a.atrial, Pace::Keep);
        assert_eq!(a.markers, Markers::NONE);

        let (s, a) = step(&p, &s, paced(Chamber::V, 1_001_000));
        assert_eq!(a.ventricular, at(2_001_000, 400));
        assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: false }, Micros(1_001_000))));
        assert_eq!(a.markers.atrial, None);

        let before = s;
        for c in [Chamber::A, Chamber::V] {
            let (s, a) = step(&p, &before, sense(c, 1_500_000));
            assert_eq!(a, Arming::NONE, "a Chamber VOO does not sense");
            assert_eq!(s, before);
        }
        // A later tick changes nothing: the Pace is pending.
        let (s, a) = step(&p, &before, tick(1_020_000));
        assert_eq!(a, Arming::NONE);
        assert_eq!(s, before);
    }

    /// AOO: the same in the atrium; the ventricle is never armed.
    #[test]
    fn aoo_paces_the_atrium_only() {
        let p = Params { width: [1_900, 50], ..params(Mode::AOO) };
        let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
        assert_eq!(a.atrial, at(1_000_000, 1_900));
        assert_eq!(a.ventricular, Pace::Keep);
        let (s, a) = step(&p, &s, paced(Chamber::A, 1_000_000));
        assert_eq!(a.atrial, at(2_000_000, 1_900));
        assert_eq!(a.ventricular, Pace::Keep);
        assert_eq!(a.markers.atrial, Some((Marker::Pace { fused: false }, Micros(1_000_000))));
        assert!(!s.chamber[Chamber::V.index()].pending.armed);
        let (_, a) = step(&p, &s, sense(Chamber::A, 1_600_000));
        assert_eq!(a, Arming::NONE);
    }

    /// VVI: inhibition and the VRP (PM5.5, PM12.1).
    ///
    /// ```text
    /// tick 0     VS 600,000   VS 800,000   VS 920,000          VP 1,920,000
    ///   |          |-- VRP 320,000 --|        |------ E ----------|
    ///   pending    pending      refractory    pending             pending
    ///   1,000,000  1,600,000    (kept)        1,920,000           2,920,000
    /// ```
    #[test]
    fn vvi_is_inhibited_by_a_sense_outside_the_vrp_only() {
        let p = params(Mode::VVI);
        let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
        assert_eq!(a.ventricular, at(1_000_000, 400));

        let (s, a) = step(&p, &s, sense(Chamber::V, 600_000));
        assert_eq!(a.ventricular, at(1_600_000, 400));
        assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: false }, Micros(600_000))));

        // Inside the VRP: nothing moves (Property P3).
        let before = s;
        let (s, a) = step(&p, &before, sense(Chamber::V, 800_000));
        assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
        assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: true }, Micros(800_000))));
        assert_eq!(s, before);
        let (s, a) = step(&p, &s, sense(Chamber::V, 919_999));
        assert_eq!(a.ventricular, Pace::Keep);
        assert_eq!(s, before);

        // The VRP ends at 920,000 exactly.
        let (s, a) = step(&p, &s, sense(Chamber::V, 920_000));
        assert_eq!(a.ventricular, at(1_920_000, 400));

        // No Sense for an LRL interval: the Pace, and the next interval from it.
        let (s, a) = step(&p, &s, paced(Chamber::V, 1_920_000));
        assert_eq!(a.ventricular, at(2_920_000, 400));
        assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: false }, Micros(1_920_000))));

        // A Pace starts the VRP as well.
        let (s2, a) = step(&p, &s, sense(Chamber::V, 2_239_999));
        assert_eq!(a.ventricular, Pace::Keep);
        assert_eq!(s2, s);
        // An atrial edge is not sensed in VVI.
        let (s2, a) = step(&p, &s, sense(Chamber::A, 2_500_000));
        assert_eq!(a, Arming::NONE);
        assert_eq!(s2, s);
    }

    /// AAI: the same in the atrium with the ARP (PM9.3, PM12.2).
    ///
    /// ```text
    /// tick 0     AS 700,000    AS 949,999     AS 950,000
    ///              |--- ARP 250,000 ---|
    ///   pending    pending      refractory     pending
    ///   1,000,000  1,700,000    (kept)         1,950,000
    /// ```
    #[test]
    fn aai_is_inhibited_by_a_sense_outside_the_arp_only() {
        let p = params(Mode::AAI);
        let (s, a) = step(&p, &State::POWER_ON_RESET, tick(0));
        assert_eq!(a.atrial, at(1_000_000, 400));
        assert_eq!(a.ventricular, Pace::Keep);
        let (s, a) = step(&p, &s, sense(Chamber::A, 700_000));
        assert_eq!(a.atrial, at(1_700_000, 400));
        let (s2, a) = step(&p, &s, sense(Chamber::A, 949_999));
        assert_eq!(a.atrial, Pace::Keep);
        assert_eq!(a.markers.atrial, Some((Marker::Sense { refractory: true }, Micros(949_999))));
        assert_eq!(s2, s);
        let (s, a) = step(&p, &s, sense(Chamber::A, 950_000));
        assert_eq!(a.atrial, at(1_950_000, 400));
        // A ventricular edge is not sensed in AAI.
        let (s2, a) = step(&p, &s, sense(Chamber::V, 1_000_000));
        assert_eq!(a, Arming::NONE);
        assert_eq!(s2, s);
    }

    /// The inhibition window (PD-04): a Sense 1 ms before the Pace moves the
    /// pending Pace, the compare fires all the same, and the `Paced` event
    /// that follows is recorded as a fusion with the next interval from the
    /// Pace.
    #[test]
    fn a_pace_the_sense_was_too_late_to_stop_is_a_fusion() {
        let p = params(Mode::VVI);
        let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
        let (s, a) = step(&p, &s, sense(Chamber::V, 999_000));
        assert_eq!(a.ventricular, at(1_999_000, 400));
        let (s, a) = step(&p, &s, paced(Chamber::V, 1_000_000));
        assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: true }, Micros(1_000_000))));
        assert_eq!(a.ventricular, at(2_000_000, 400));
        let ch = s.chamber[Chamber::V.index()];
        assert!(ch.last_paced && ch.last == Micros(1_000_000));
    }

    /// Events in the order they happened: a Sense whose instant is after a
    /// pending Pace's arrives before that Pace's `Paced` event. The step
    /// applies the Pace first, so the Sense is a Refractory sense, and the
    /// `Paced` event then only confirms and arms.
    #[test]
    fn a_pace_is_accounted_when_a_later_event_arrives_first() {
        let p = params(Mode::VVI);
        let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
        let (s, a) = step(&p, &s, sense(Chamber::V, 1_000_500));
        assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep));
        assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: true }, Micros(1_000_500))));
        let ch = s.chamber[Chamber::V.index()];
        assert!(ch.unconfirmed && ch.last_paced);
        assert_eq!(ch.last, Micros(1_000_000));
        assert_eq!(ch.pending, Pending::at(Micros(2_000_000)));

        let (s2, a) = step(&p, &s, paced(Chamber::V, 1_000_000));
        assert_eq!(a.ventricular, at(2_000_000, 400));
        assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: false }, Micros(1_000_000))));
        let mut confirmed = s;
        confirmed.chamber[Chamber::V.index()].unconfirmed = false;
        assert_eq!(s2, confirmed);
    }

    /// A tick after the Pace's instant accounts for it as well, and a Pace
    /// the hardware delivered later than asked (moved by the lead) restarts
    /// the interval from where it was delivered.
    #[test]
    fn a_tick_accounts_and_a_moved_pace_restarts_from_its_own_instant() {
        let p = params(Mode::VOO);
        let (s, _) = step(&p, &State::POWER_ON_RESET, tick(0));
        let (s, a) = step(&p, &s, tick(1_000_100));
        assert_eq!(a, Arming::NONE);
        assert_eq!(s.chamber[1].pending, Pending::at(Micros(2_000_000)));
        let (s, a) = step(&p, &s, paced(Chamber::V, 1_000_300));
        assert_eq!(a.ventricular, at(2_000_300, 400));
        assert_eq!(a.markers.ventricular, Some((Marker::Pace { fused: false }, Micros(1_000_300))));
        assert!(!s.chamber[1].unconfirmed);
    }

    /// Off: nothing is armed, and a Pace left pending by an earlier Mode is
    /// withdrawn at the next tick.
    #[test]
    fn off_arms_nothing_and_withdraws_what_was_pending() {
        let off = params(Mode::OFF);
        let (s, a) = step(&off, &State::POWER_ON_RESET, tick(0));
        assert_eq!(a, Arming::NONE);
        assert_eq!(s, State::POWER_ON_RESET);
        let (s, _) = step(&params(Mode::VVI), &s, tick(0));
        let (s, a) = step(&off, &s, tick(20_000));
        assert_eq!(a.ventricular, Pace::Withdraw);
        assert!(!s.chamber[1].pending.armed);
    }

    /// The instants are compared by difference: the cycle is the same across
    /// the counter's wrap.
    #[test]
    fn the_cycle_crosses_the_wrap() {
        let p = params(Mode::VVI);
        let t0 = u32::MAX - 400_000;
        let (s, a) = step(&p, &State::POWER_ON_RESET, tick(t0));
        assert_eq!(a.ventricular, at(t0.wrapping_add(1_000_000), 400));
        let (s, a) = step(&p, &s, sense(Chamber::V, t0.wrapping_add(500_000)));
        assert_eq!(a.ventricular, at(t0.wrapping_add(1_500_000), 400));
        let (_, a) = step(&p, &s, sense(Chamber::V, t0.wrapping_add(700_000)));
        assert_eq!(a.ventricular, Pace::Keep);
    }

    // Property-based tests: generated parameter sets and event sequences.

    /// A small deterministic generator (xorshift), so that a failure names
    /// its seed and the crate keeps no dependency.
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
        fn between(&mut self, lo: u64, hi: u64) -> u64 {
            lo + self.below(hi - lo + 1)
        }
    }

    /// A parameter set of a stage-E1 Mode, through `validate`.
    fn generated_params(r: &mut Rng) -> Params {
        let mut w = NOMINAL_WORDS;
        w[word::MODE] = [5, 6, 7, 8][r.below(4) as usize]; // AOO, AAI, VOO, VVI
        w[word::LRL] = [30, 45, 50, 60, 73, 90, 120, 175][r.below(8) as usize];
        w[word::VRP] = 150 + 10 * r.below(36) as u32;
        w[word::ARP] = 150 + 10 * r.below(36) as u32;
        w[word::A_WIDTH] = [50, 400, 1_900][r.below(3) as usize];
        w[word::V_WIDTH] = [50, 400, 1_900][r.below(3) as usize];
        validate(&w).expect("a set of Table 19-1")
    }

    /// The compares as the hardware layer keeps them: an armed start per
    /// Chamber, and a Pace that fired and is not yet reported, during which
    /// arming and withdrawing write nothing (`TooLate`).
    #[derive(Default)]
    struct Compares {
        armed: [Option<u64>; 2],
        unreported: [bool; 2],
    }

    struct Tally {
        steps: u64,
        paces: u64,
        fusions: u64,
        accounted: u64,
        refractory: u64,
        inhibitions: u64,
    }

    /// One generated run: a heart producing edges, the compares firing by
    /// themselves, every event delivered some time after its instant, a tick
    /// every 20 ms. After every step: Properties P2, P3 and P9, and the
    /// compares agree with the state. At the end: no gap between two events
    /// of a paced Chamber is longer than the escape interval (PM9.1).
    fn run(seed: u64, tally: &mut Tally) {
        let mut r = Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1);
        let p = generated_params(&mut r);
        let e = u64::from(escape(&p));
        // A base near the counter's wrap in half the runs.
        let base: u64 = if seed & 1 == 0 { u64::from(u32::MAX) - 3_000_000 } else { r.below(1 << 32) };
        let micros = |t: u64| Micros((base + t) as u32);
        let abs = |now: u64, m: Micros| (now as i64 + i64::from(m.since(micros(now)))) as u64;

        let mut s = State::POWER_ON_RESET;
        let mut hw = Compares::default();
        // Deliveries: (time of delivery, event).
        let mut queue: Vec<(u64, Event)> = Vec::new();
        let mut next_tick = r.below(20_000);
        let mut next_edge = [r.between(1_000, 1_500_000), r.between(1_000, 1_500_000)];
        let mut ticked = false;
        // The events of each Chamber as they happened: paces by the compare,
        // non-refractory Senses by the step's Marker.
        let mut happened: [Vec<u64>; 2] = [Vec::new(), Vec::new()];
        let mut first_tick = 0;
        let end = 30_000_000;

        loop {
            // The next thing in time: a compare firing, an edge, a tick, a delivery.
            let fire = CHAMBERS.iter().filter_map(|c| hw.armed[c.index()].map(|t| (t, *c))).min_by_key(|x| x.0);
            let edge = if next_edge[0] <= next_edge[1] { (next_edge[0], Chamber::A) } else { (next_edge[1], Chamber::V) };
            let delivery = queue.iter().enumerate().min_by_key(|(_, d)| d.0).map(|(i, d)| (d.0, i));
            let mut now = next_tick.min(edge.0);
            if let Some((t, _)) = fire {
                now = now.min(t);
            }
            if let Some((t, _)) = delivery {
                now = now.min(t);
            }
            if now > end {
                break;
            }

            if let Some((t, c)) = fire.filter(|f| f.0 == now) {
                // The compare fires: the pulse, then its `pace` Job.
                hw.armed[c.index()] = None;
                hw.unreported[c.index()] = true;
                happened[c.index()].push(t);
                tally.paces += 1;
                let delay = u64::from(p.width[c.index()]) + r.between(50, 300);
                queue.push((t + delay, Event::Paced { chamber: c, at: micros(t) }));
                continue;
            }
            if edge.0 == now {
                let c = edge.1;
                queue.push((now + r.between(100, 2_000), Event::Sense { chamber: c, at: micros(now), edges: 1 }));
                // The next edge: anywhere, or close around the armed Pace.
                next_edge[c.index()] = match hw.armed[c.index()] {
                    Some(t) if r.below(3) == 0 && t > now + 4_000 => t - 3_000 + r.below(6_000),
                    _ => now + r.between(150_000, 1_400_000),
                };
                continue;
            }
            let event = match delivery.filter(|d| d.0 == now) {
                Some((_, i)) => queue.swap_remove(i).1,
                None => {
                    next_tick += 20_000;
                    if !ticked {
                        ticked = true;
                        first_tick = now;
                    }
                    tick(micros(now).0)
                }
            };

            let before = s;
            let instant = match event {
                Event::Sense { at, .. } | Event::Paced { at, .. } => at,
                Event::Tick { now, .. } => now,
                Event::Command(_) => unreachable!(),
            };
            let settled = accounted(&p, &before, instant);
            if let Event::Paced { chamber, .. } = event {
                hw.unreported[chamber.index()] = false;
                if before.chamber[chamber.index()].unconfirmed {
                    tally.accounted += 1;
                }
            }
            let (after, arming) = step(&p, &before, event);
            s = after;
            tally.steps += 1;

            // The hardware layer applies the arming.
            for c in CHAMBERS {
                if hw.unreported[c.index()] {
                    continue; // TooLate: nothing is written
                }
                match arming.pace(c) {
                    Pace::Keep => {}
                    Pace::Withdraw => hw.armed[c.index()] = None,
                    Pace::At { at, width } => {
                        let t = abs(now, at);
                        assert!(t > now, "seed {seed}: armed in the past");
                        assert_eq!(width, p.width[c.index()], "seed {seed}");
                        hw.armed[c.index()] = Some(t);
                    }
                }
            }

            // Property P9, beyond the type: an event marks its own Chamber only.
            if let Event::Sense { chamber, .. } | Event::Paced { chamber, .. } = event {
                assert_eq!(marker(&arming, chamber.other()), None, "seed {seed}");
                assert_eq!(arming.markers.augmentation, None);
            }
            match (event, arming.markers) {
                (Event::Sense { chamber, .. }, m) => match marker(&Arming { markers: m, ..Arming::NONE }, chamber) {
                    Some((Marker::Sense { refractory: true }, _)) => {
                        // Property P3: nothing armed or withdrawn, and the
                        // state is the one the Sense found.
                        tally.refractory += 1;
                        assert_eq!((arming.atrial, arming.ventricular), (Pace::Keep, Pace::Keep), "seed {seed}");
                        assert_eq!(after, settled, "seed {seed}");
                    }
                    Some((Marker::Sense { refractory: false }, _)) => {
                        assert!(p.mode.senses(chamber));
                        happened[chamber.index()].push(abs(now, instant));
                        tally.inhibitions += 1;
                    }
                    None => assert!(!p.mode.senses(chamber), "seed {seed}"),
                    other => panic!("seed {seed}: {other:?}"),
                },
                (Event::Paced { chamber, .. }, m) => match marker(&Arming { markers: m, ..Arming::NONE }, chamber) {
                    Some((Marker::Pace { fused }, _)) => tally.fusions += u64::from(fused),
                    other => panic!("seed {seed}: {other:?}"),
                },
                (_, m) => assert_eq!(m, Markers::NONE),
            }

            // Property P2: once pacing has begun, a Pace of every paced
            // Chamber is pending no later than its last event plus the
            // escape interval; a Chamber not paced has none.
            for c in CHAMBERS {
                let ch = after.chamber[c.index()];
                if !p.mode.paces(c) {
                    assert!(!ch.pending.armed, "seed {seed}");
                    assert_eq!(hw.armed[c.index()], None);
                    continue;
                }
                if !ticked {
                    continue;
                }
                assert!(ch.pending.armed, "seed {seed}: no Pace pending");
                if ch.seen {
                    let d = ch.pending.at.since(ch.last);
                    assert!(d > 0 && d as u64 <= e, "seed {seed}: pending {d} after the last event");
                }
                // The compares agree with the state.
                if !hw.unreported[c.index()] {
                    assert_eq!(hw.armed[c.index()], Some(abs(now, ch.pending.at)), "seed {seed}");
                }
            }
        }

        // PM9.1 on what happened: from the first tick on, no paced Chamber
        // goes longer than the escape interval without an event.
        for c in CHAMBERS {
            if !p.mode.paces(c) {
                assert!(happened[c.index()].is_empty() || p.mode.senses(c));
                continue;
            }
            let mut events = happened[c.index()].clone();
            events.sort_unstable();
            let mut last = first_tick;
            for t in events.into_iter().filter(|t| *t >= first_tick) {
                assert!(t - last <= e, "seed {seed}: {} µs without an event", t - last);
                last = t;
            }
            assert!(end - last <= e + 2_000, "seed {seed}");
        }
    }

    #[test]
    fn generated_runs_keep_properties_p2_p3_and_p9() {
        let mut tally = Tally { steps: 0, paces: 0, fusions: 0, accounted: 0, refractory: 0, inhibitions: 0 };
        for seed in 1..=400 {
            run(seed, &mut tally);
        }
        // The runs reached every case the properties are about.
        assert!(tally.steps > 100_000, "{}", tally.steps);
        assert!(tally.paces > 1_000 && tally.inhibitions > 1_000 && tally.refractory > 100);
        assert!(tally.fusions > 10, "fusions {}", tally.fusions);
        assert!(tally.accounted > 10, "accounted {}", tally.accounted);
    }
}
