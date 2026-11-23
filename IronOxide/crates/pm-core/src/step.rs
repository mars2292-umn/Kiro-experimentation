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

/// The escape interval in force: the LRL's (PM9.1).
#[inline(always)]
const fn escape(p: &Params) -> u32 {
    p.lrl
}

/// The refractory period an event of `c` starts: the ARP in the atrium, the
/// VRP in the ventricle (PM12.1, PM12.2).
#[inline(always)]
const fn refractory(p: &Params, c: Chamber) -> u32 {
    match c {
        Chamber::A => p.arp,
        Chamber::V => p.vrp,
    }
}

/// A Pace of `c` at `at`: the Chamber's last event, the start of its
/// refractory period and of the next escape interval (PM9.2, PM9.3).
#[inline(always)]
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
#[inline(always)]
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
#[inline(always)]
#[must_use]
pub fn accounted(p: &Params, s: &State, instant: Micros) -> State {
    let mut st = *s;
    account(p, &mut st, Chamber::A, instant);
    account(p, &mut st, Chamber::V, instant);
    st
}

#[inline(always)]
fn set_pace(a: &mut Arming, c: Chamber, pace: Pace) {
    match c {
        Chamber::A => a.atrial = pace,
        Chamber::V => a.ventricular = pace,
    }
}

#[inline(always)]
fn set_marker(a: &mut Arming, c: Chamber, marker: Marker, at: Micros) {
    match c {
        Chamber::A => a.markers.atrial = Some((marker, at)),
        Chamber::V => a.markers.ventricular = Some((marker, at)),
    }
}

/// The instruction that arms `c` as the state has it pending.
#[inline(always)]
fn arm_from(p: &Params, st: &State, c: Chamber) -> Pace {
    let pending = st.chamber[c.index()].pending;
    if pending.armed {
        Pace::At { at: pending.at, width: p.width[c.index()] }
    } else {
        Pace::Keep
    }
}

/// A tick for one Chamber: a Chamber the Mode paces has a Pace pending, the
/// first one escape interval from now; a Chamber it does not pace has none.
#[inline(always)]
fn tick(p: &Params, st: &mut State, out: &mut Arming, c: Chamber, now: Micros) {
    let ch = &mut st.chamber[c.index()];
    if p.mode.paces(c) {
        if !ch.pending.armed {
            ch.pending = Pending::at(now.plus(escape(p)));
            set_pace(out, c, Pace::At { at: ch.pending.at, width: p.width[c.index()] });
        }
    } else if ch.pending.armed {
        ch.pending = Pending::NONE;
        set_pace(out, c, Pace::Withdraw);
    }
}

/// One event applied to the state: the new state, and what to do with each
/// Chamber's compares. `p` is the parameter set in force.
///
/// Total, without side effect, allocation or loop over anything but the two
/// Chambers, in integer arithmetic.
#[inline(always)]
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
            tick(p, &mut st, &mut out, Chamber::A, now);
            tick(p, &mut st, &mut out, Chamber::V, now);
            (st, out)
        }
        // The State machine is stage E3 (task 16.2).
        Event::Command(_) => (*s, out),
    }
}
