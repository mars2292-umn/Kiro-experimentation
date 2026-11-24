//! The step function (design PD-05, PD-06).
//!
//! Stage E1 (task 6.3): the single-chamber cycle. A Chamber the Mode paces
//! has a Pace pending one escape interval after its last event; the escape
//! interval is the LRL's; a non-refractory Sense in a Mode whose response is
//! I starts the interval again, which inhibits the pending Pace by moving
//! it; a Sense inside the Chamber's refractory period changes nothing.
//!
//! Stage E2, first part (tasks 11.1 and 11.5): the ventricular-based cycle
//! of the Modes that pace the ventricle, and the triggered response.
//!
//! - **The ventricular-based cycle (PM9.2).** A ventricular event at `v`
//!   puts the ventricular Pace at `v + E` and, in a Mode that paces the
//!   atrium, the atrial Pace at `v + E - AV`. An atrial event starts an AV
//!   delay: after an atrial Pace the ventricular Pace is due one paced AV
//!   delay later, after a tracked atrial Sense (response D) one sensed AV
//!   delay later (the paced one less the Sensed AV Delay Offset, PM11.5),
//!   and in neither case later than `v + E`, where it already was: the LRL
//!   interval is never lengthened by an atrial event. An atrial Sense
//!   withdraws the pending atrial Pace in every Mode that senses the atrium;
//!   with response I (DDI) it does no more. Until the ventricular event that
//!   ends it, an AV delay makes every atrial Sense a Refractory sense
//!   (PM11.6).
//! - **The triggered response (PM5.4).** A non-refractory Sense puts a Pace
//!   of its Chamber at the Sense's own instant, which is already past: the
//!   hardware layer then arms it one `LEAD` after the present, the earliest
//!   it can, and reports that instant ([`rearmed`]); the Pace is then a Pace
//!   like any other.
//!
//! Stage E2, second part (tasks 11.2 to 11.4), all in the ventricular-based
//! cycle:
//!
//! - **The dynamic AV delay (PM11.4).** Where it is on, the paced AV delay
//!   of a cycle is the previous cardiac cycle's length (ventricular event to
//!   ventricular event) times the Fixed AV Delay over the LRL interval,
//!   kept between the Minimum Dynamic AV Delay and the Fixed AV Delay; the
//!   Fixed AV Delay until a cycle has been measured.
//! - **The URL (PM10.1, PM10.2).** The ventricular Pace an atrial event
//!   leads to is not put earlier than one URL interval after the last
//!   ventricular event: it is delayed to that instant, not dropped
//!   (Property P1). The LRL's own Pace is one escape interval after the
//!   event and so later still, the URL interval being no longer than the
//!   LRL's in an admitted parameter set (Table 19-5, task 11.6).
//! - **PVARP (PM12.3).** A ventricular event starts it: until it ends an
//!   atrial Sense is a Refractory sense. A ventricular Sense with no atrial
//!   event since the last ventricular event is a PVC; with PVARP Extension
//!   on, the PVARP it starts is longer by the extension, unless the cycle
//!   before was extended (PM12.4, PM12.5; Property P8).
//! - **Ventricular blanking (PM4.5).** In a Mode that paces the atrium and
//!   senses the ventricle, a ventricular Sense within the Ventricular
//!   Blanking interval after an atrial Pace is discarded: nothing is armed
//!   or withdrawn and no Marker is made.
//!
//! The step reads the Mode by its letters. What it does not deliver yet
//! (hysteresis, the sensor rate, the State machine) is the work of tasks
//! 16 and 19; `validate` admits only the Modes of stage E1 until task 11.6.
//! In the ventricular-based cycle the atrium's refractory periods are the
//! AV delay and the PVARP; the ARP is the single-chamber atrial Modes'
//! (PM12.2).
//!
//! **What is armed.** The arming the step returns is the difference the
//! event makes to the pending Paces, Chamber by Chamber: a new instant is
//! armed, a Pace no longer pending is withdrawn, an unchanged one is kept.
//! The difference is taken from the `accounted` state, so what accounting
//! itself changed is not armed by the event that happened to find it (a
//! Refractory sense arms nothing, Property P3). It is armed when the Pace
//! that was accounted is reported: a Chamber whose Pace has fired and is
//! not yet reported (`unconfirmed`) is left alone until then, since its
//! compares may not be written while its pulse is in progress, and its
//! `Paced` event arms it from the state; and the report of a ventricular
//! Pace that had been accounted arms the atrial Pace the ventricular event
//! made pending.

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

/// The AV delay after an atrial Pace (PM11.2 to PM11.4): the Fixed AV
/// Delay; or, where the delay is dynamic and a cardiac cycle has been
/// measured, that cycle's length times the Fixed AV Delay over the LRL
/// interval, no shorter than the Minimum Dynamic AV Delay and no longer
/// than the Fixed AV Delay.
#[inline(always)]
const fn paced_av(p: &Params, st: &State) -> u32 {
    if !p.dynamic_av || !st.cycle_known || p.lrl == 0 {
        return p.fixed_av;
    }
    // In 64 bits: two 32-bit values' product has room.
    let scaled = st.last_cycle as u64 * p.fixed_av as u64 / p.lrl as u64;
    let lowest = if p.min_dynamic_av < p.fixed_av { p.min_dynamic_av } else { p.fixed_av };
    if scaled >= p.fixed_av as u64 {
        p.fixed_av
    } else if scaled <= lowest as u64 {
        lowest
    } else {
        scaled as u32
    }
}

/// The AV delay after a tracked atrial Sense: the paced one less the Sensed
/// AV Delay Offset (PM11.5).
#[inline(always)]
const fn sensed_av(p: &Params, st: &State) -> u32 {
    paced_av(p, st).saturating_sub(p.sensed_av_offset)
}

/// The escape interval of the atrium in the ventricular-based cycle: from a
/// ventricular event to the atrial Pace, so that the paced AV delay ends
/// one escape interval after the ventricular event.
#[inline(always)]
const fn atrial_escape(p: &Params, st: &State) -> u32 {
    escape(p).saturating_sub(paced_av(p, st))
}

/// Whether the cycle is the ventricular-based one: the Mode paces the
/// ventricle (PM9.2). Otherwise each paced Chamber is timed from its own
/// events (PM9.3).
#[inline(always)]
const fn ventricular_based(p: &Params) -> bool {
    p.mode.paces(Chamber::V)
}

/// Whether the Mode's two Chambers depend on each other: it paces the
/// ventricle and paces or senses the atrium.
#[inline(always)]
const fn coupled(p: &Params) -> bool {
    ventricular_based(p) && (p.mode.paces(Chamber::A) || p.mode.senses(Chamber::A))
}

/// Whether a ventricular Sense now would be a PVC: the Mode senses both
/// Chambers in the ventricular-based cycle, and no atrial event has come
/// since the last ventricular event.
#[inline(always)]
const fn is_pvc(p: &Params, st: &State) -> bool {
    ventricular_based(p) && p.mode.senses(Chamber::A) && st.chamber[Chamber::V.index()].seen && !st.atrial_since_ventricular
}

/// A ventricular event at `v`, paced or sensed (`pvc`: a Sense that is a
/// PVC): the Chamber's last event, the start of the VRP (PM12.1) and of
/// the next escape interval; the end of a cardiac cycle, whose length is
/// kept, of an AV delay and of a blanking interval; in the
/// ventricular-based cycle the start of the PVARP, extended after a PVC
/// unless the cycle before was, and, in a Mode that paces both Chambers,
/// the atrial Pace of the next cycle.
#[inline(always)]
fn ventricular_event(p: &Params, st: &mut State, v: Micros, paced: bool, pvc: bool) {
    let ch = &mut st.chamber[Chamber::V.index()];
    if ch.seen {
        st.last_cycle = v.elapsed_since(ch.last);
        st.cycle_known = true;
    }
    ch.seen = true;
    ch.last = v;
    ch.last_paced = paced;
    ch.refractory_until = v.plus(p.vrp);
    ch.blanked = false;
    ch.pending = if p.mode.paces(Chamber::V) { Pending::at(v.plus(escape(p))) } else { Pending::NONE };
    st.atrial_since_ventricular = false;
    if ventricular_based(p) {
        // Property P8: not in two successive cycles.
        let extend = pvc && p.pvarp_extension != 0 && !st.pvarp_extended;
        st.pvarp_extended_last_cycle = st.pvarp_extended;
        st.pvarp_extended = extend;
        let pvarp = if extend { p.pvarp.saturating_add(p.pvarp_extension) } else { p.pvarp };
        st.chamber[Chamber::A.index()].refractory_until = v.plus(pvarp);
        if p.mode.paces(Chamber::A) {
            let escape = atrial_escape(p, st);
            st.chamber[Chamber::A.index()].pending = Pending::at(v.plus(escape));
        }
    }
}

/// An atrial event at `a`: a Pace, or a non-refractory Sense, which
/// `starts_av` says is tracked (response D). In the ventricular-based cycle
/// it ends the atrial escape interval and, when it starts an AV delay, puts
/// the ventricular Pace at the delay's end, or one URL interval after the
/// last ventricular event if that is later (PM10.2), unless the Pace is
/// already due sooner; an atrial Pace starts the Ventricular Blanking
/// interval in a Mode that senses the ventricle (PM4.5). In an atrial Mode
/// the event starts the ARP (PM12.2) and the next escape interval.
#[inline(always)]
fn atrial_event(p: &Params, st: &mut State, a: Micros, paced: bool, starts_av: bool) {
    let based = ventricular_based(p);
    let delay = if paced { paced_av(p, st) } else { sensed_av(p, st) };
    let ch = &mut st.chamber[Chamber::A.index()];
    ch.seen = true;
    ch.last = a;
    ch.last_paced = paced;
    if based {
        // No ARP: the AV delay and the PVARP, which the last ventricular
        // event started, are the atrium's refractory periods here.
        ch.pending = Pending::NONE;
        st.atrial_since_ventricular = true;
        let v = &mut st.chamber[Chamber::V.index()];
        if paced && p.mode.senses(Chamber::V) {
            v.blanked = true;
            v.blanking_until = a.plus(p.v_blanking);
        }
        if starts_av {
            let mut end = a.plus(delay);
            if v.seen {
                let earliest = v.last.plus(p.url);
                if end.is_before(earliest) {
                    end = earliest;
                }
            }
            if !v.pending.armed || end.is_before(v.pending.at) {
                v.pending = Pending::at(end);
            }
        }
    } else {
        ch.refractory_until = a.plus(p.arp);
        ch.pending = if p.mode.paces(Chamber::A) { Pending::at(a.plus(escape(p))) } else { Pending::NONE };
    }
}

/// A Pace of `c` at `at`.
#[inline(always)]
fn apply_pace(p: &Params, st: &mut State, c: Chamber, at: Micros) {
    match c {
        Chamber::A => atrial_event(p, st, at, true, true),
        Chamber::V => ventricular_event(p, st, at, true, false),
    }
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
/// this state, so Property P3 is stated against it. The atrium first: in
/// the ventricular-based cycle its Pace is the earlier of the two, and the
/// ventricular Pace it leads to may be due as well.
#[inline(always)]
#[must_use]
pub fn accounted(p: &Params, s: &State, instant: Micros) -> State {
    let mut st = *s;
    account(p, &mut st, Chamber::A, instant);
    account(p, &mut st, Chamber::V, instant);
    st
}

/// The hardware layer armed the pending Pace of `c` at `at` and not at the
/// instant the step gave (`Armed::Ok { moved: true, .. }`, or a pulse it
/// had to force): the state's pending instant becomes the armed one, so
/// that a Pace is accounted when it has fired and not before. A Chamber
/// with no Pace pending is left as it is.
#[inline(always)]
#[must_use]
pub fn rearmed(s: &State, c: Chamber, at: Micros) -> State {
    let mut st = *s;
    if st.chamber[c.index()].pending.armed {
        st.chamber[c.index()].pending = Pending::at(at);
    }
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

/// What to do with `c`'s compares so that they are as `after` has the
/// Chamber pending, given that they were as `before` had it (the module's
/// "What is armed").
#[inline(always)]
fn difference(p: &Params, before: &State, after: &State, c: Chamber) -> Pace {
    let (b, a) = (before.chamber[c.index()].pending, after.chamber[c.index()].pending);
    if after.chamber[c.index()].unconfirmed {
        Pace::Keep
    } else if a.armed {
        if b.armed && b.at.0 == a.at.0 {
            Pace::Keep
        } else {
            Pace::At { at: a.at, width: p.width[c.index()] }
        }
    } else if b.armed {
        Pace::Withdraw
    } else {
        Pace::Keep
    }
}

/// The arming of both Chambers from the states before and after.
#[inline(always)]
fn arm_difference(p: &Params, before: &State, after: &State, out: &mut Arming) {
    out.atrial = difference(p, before, after, Chamber::A);
    out.ventricular = difference(p, before, after, Chamber::V);
}

/// A tick: a Mode that paces has its cycle started if nothing is pending,
/// and a Chamber the Mode does not pace has nothing pending. In the
/// ventricular-based cycle the start is as after a ventricular event: the
/// ventricular Pace one escape interval from now and, if the Mode paces the
/// atrium, the atrial Pace one AV delay before it.
#[inline(always)]
fn tick(p: &Params, st: &mut State, now: Micros) {
    let paces_a = p.mode.paces(Chamber::A);
    if ventricular_based(p) {
        if !st.chamber[Chamber::V.index()].pending.armed {
            st.chamber[Chamber::V.index()].pending = Pending::at(now.plus(escape(p)));
            st.atrial_since_ventricular = false;
            if paces_a {
                let escape = atrial_escape(p, st);
                st.chamber[Chamber::A.index()].pending = Pending::at(now.plus(escape));
            }
        }
        if !paces_a {
            st.chamber[Chamber::A.index()].pending = Pending::NONE;
        }
    } else {
        st.chamber[Chamber::V.index()].pending = Pending::NONE;
        let a = &mut st.chamber[Chamber::A.index()];
        if paces_a {
            if !a.pending.armed {
                a.pending = Pending::at(now.plus(escape(p)));
            }
        } else {
            a.pending = Pending::NONE;
        }
    }
}

/// Whether a Sense of `c` at `at` falls in a refractory period of the
/// Chamber: the period its last event started (PM12.1, PM12.2); or, for the
/// atrium in the ventricular-based cycle, an AV delay in progress (PM11.6)
/// or the PVARP the last ventricular event started (PM12.3).
#[inline(always)]
fn refractory(p: &Params, st: &State, c: Chamber, at: Micros) -> bool {
    let ch = &st.chamber[c.index()];
    if matches!(c, Chamber::A) && ventricular_based(p) {
        st.atrial_since_ventricular || (st.chamber[Chamber::V.index()].seen && at.is_before(ch.refractory_until))
    } else {
        ch.seen && at.is_before(ch.refractory_until)
    }
}

/// Whether a ventricular Sense at `at` falls in the Ventricular Blanking
/// interval an atrial Pace started (PM4.5).
#[inline(always)]
fn blanked(st: &State, c: Chamber, at: Micros) -> bool {
    let ch = &st.chamber[c.index()];
    matches!(c, Chamber::V) && ch.blanked && at.is_before(ch.blanking_until)
}

/// One event applied to the state: the new state, and what to do with each
/// Chamber's compares. `p` is the parameter set in force.
///
/// Total, without side effect, allocation or loop, in integer arithmetic.
#[inline(always)]
#[must_use]
pub fn step(p: &Params, s: &State, e: Event) -> (State, Arming) {
    let mut out = Arming::NONE;
    match e {
        Event::Paced { chamber, at } => {
            let mut st = *s;
            // The other Chamber's Pace, if it was due by now.
            account(p, &mut st, chamber.other(), at);
            let before = st;
            let ch = st.chamber[chamber.index()];
            // A Pace the state no longer had pending at this instant: a
            // Sense moved or withdrew it too late for the compare (PD-04).
            let expected = ch.unconfirmed || (ch.pending.armed && at.is_at_or_after(ch.pending.at));
            if !ch.unconfirmed {
                apply_pace(p, &mut st, chamber, at);
            } else if ch.last_paced && at.0 != ch.last.0 && !coupled(p) {
                // A Pace already applied, reported at another instant than
                // the state had it pending (an arming the hardware layer
                // moved and `rearmed` was not told of): in a Mode of one
                // Chamber the interval restarts from the reported instant,
                // unless a Sense has started it again since. In a Mode of
                // both Chambers the other Chamber's instants hang on this
                // one and events of it may have come since, so the state
                // stands.
                apply_pace(p, &mut st, chamber, at);
            }
            st.chamber[chamber.index()].unconfirmed = false;
            arm_difference(p, &before, &st, &mut out);
            // The Chamber that paced is armed from the state: its compare
            // is spent whatever the state had pending before.
            set_pace(&mut out, chamber, arm_from(p, &st, chamber));
            // A ventricular Pace that had been accounted: the atrial Pace
            // it made pending was armed by nothing yet.
            if ch.unconfirmed && matches!(chamber, Chamber::V) && !st.chamber[Chamber::A.index()].unconfirmed {
                set_pace(&mut out, Chamber::A, arm_from(p, &st, Chamber::A));
            }
            set_marker(&mut out, chamber, Marker::Pace { fused: !expected }, at);
            (st, out)
        }
        Event::Sense { chamber, at, edges: _ } => {
            let before = accounted(p, s, at);
            let mut st = before;
            if !p.mode.senses(chamber) {
                // PM5.7: a Chamber the Mode does not sense.
                return (st, out);
            }
            if blanked(&st, chamber, at) {
                // PM4.5: discarded, without a Marker.
                return (st, out);
            }
            if refractory(p, &st, chamber, at) {
                // PM4.4, PM11.6: a Refractory sense inhibits and triggers
                // nothing.
                set_marker(&mut out, chamber, Marker::Sense { refractory: true }, at);
                return (st, out);
            }
            let paces = p.mode.paces(chamber);
            match (p.mode.response, chamber) {
                (Response::Triggered, _) if paces => {
                    // PM5.4: a Pace of the Chamber at once. The Sense is the
                    // Chamber's event until the Pace is, a moment later.
                    let period = match chamber {
                        Chamber::A => p.arp,
                        Chamber::V => p.vrp,
                    };
                    let ch = &mut st.chamber[chamber.index()];
                    ch.seen = true;
                    ch.last = at;
                    ch.last_paced = false;
                    ch.refractory_until = at.plus(period);
                    ch.pending = Pending::at(at);
                }
                (Response::Inhibited | Response::Tracked, Chamber::V) => {
                    // PM5.5: the pending ventricular Pace is cancelled; the
                    // cycle starts again from the Sense.
                    let pvc = is_pvc(p, &st);
                    ventricular_event(p, &mut st, at, false, pvc);
                    if pvc {
                        out.markers.augmentation = Some((Marker::Pvc, at));
                    }
                }
                (Response::Inhibited, Chamber::A) => {
                    // PM5.5: the pending atrial Pace is cancelled. In DDI
                    // the ventricular Pace stays where it was.
                    atrial_event(p, &mut st, at, false, false);
                }
                (Response::Tracked, Chamber::A) => {
                    // PM5.6: a ventricular Pace one AV delay after it.
                    atrial_event(p, &mut st, at, false, true);
                }
                _ => {
                    // A Mode that senses the Chamber and does not respond
                    // (the temporary Modes OAO, OVO, ODO): the event is kept.
                    let period = match chamber {
                        Chamber::A => p.arp,
                        Chamber::V => p.vrp,
                    };
                    let ch = &mut st.chamber[chamber.index()];
                    ch.seen = true;
                    ch.last = at;
                    ch.last_paced = false;
                    ch.refractory_until = at.plus(period);
                }
            }
            arm_difference(p, &before, &st, &mut out);
            set_marker(&mut out, chamber, Marker::Sense { refractory: false }, at);
            (st, out)
        }
        Event::Tick { now, .. } => {
            let before = accounted(p, s, now);
            let mut st = before;
            tick(p, &mut st, now);
            arm_difference(p, &before, &st, &mut out);
            (st, out)
        }
        // The State machine is stage E3 (task 16.2).
        Event::Command(_) => (*s, out),
    }
}
