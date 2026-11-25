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
//! The step reads the Mode by its letters, and reads a parameter only in
//! the Modes Table 19-3, as amended by Table 19-4, gives it to (PM5.8): the
//! Dynamic AV Delay and PVARP Extension in the Modes that track, the
//! Sensed AV Delay Offset in DDD. What it does not deliver yet (hysteresis,
//! the sensor rate, the State machine) is the work of tasks 16 and 19.
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

use crate::event::{Arming, Battery, Command, Event, Marker, Pace};
#[cfg(creusot)]
#[allow(unused_imports)]
use crate::mode::Chambers;
use crate::mode::{Chamber, Response};
// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

// The contracts (tasks 12.1 and 12.2). Each helper keeps what the step's
// contract states, so that the App_Prover can carry it through:
//
// - Property P4: the therapy State is not changed (the State machine is
//   stage E3; until then no event changes it).
// - Property P8: the PVARP of the present cycle and of the one before are
//   not both extended.
// - Property P2, as far as it is stated without ordering instants on a
//   counter that wraps: in a Mode that paces the ventricle a ventricular
//   Pace stays pending once it is, and a tick makes it so; after a
//   ventricular event it is pending at exactly the event plus the escape
//   interval; likewise the atrial Pace of a Mode that paces the atrium
//   alone.
use crate::mode::Mode;
use crate::params::{interval_of_ppm, Params};
#[cfg(creusot)]
#[allow(unused_imports)]
use crate::state::allowed;
use crate::rate::{fallback_interval, sensor_next, smoothed, interval_of, ATR_COUNT, NOISE_INTERVAL_US};
use crate::state::{AtrPhase, Pending, State, Therapy};
use crate::time::Micros;

/// Property P1 as a statement about a state (investigation of 2026-11-24):
/// a ventricular Pace that is pending is no earlier than the last
/// ventricular event plus the URL interval.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn p1_inv(p: Params, st: State) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! { st.chamber@[1].seen && st.chamber@[1].pending.armed ==> st.chamber@[1].pending.at.0@ >= crate::time::sum(st.chamber@[1].last, p.url) }
}

/// Property P2's "no later", as a statement about a state before and after
/// something that happened at `t`: in a Mode that paces the ventricle, a
/// ventricular Pace that was pending is pending, no later than it was or
/// no later than `t` plus the escape interval.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn p2_rel(p: Params, old: State, new: State, t: Micros) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! {
        (p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && old.chamber@[1].pending.armed
            ==> new.chamber@[1].pending.armed
                && (new.chamber@[1].pending.at.0@ <= old.chamber@[1].pending.at.0@ || new.chamber@[1].pending.at.0@ <= crate::time::sum(t, p.lrl))
    }
}

/// Property P2's "no later" for a Mode that paces the atrium alone: as
/// [`p2_rel`], of the atrial Pace.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn p2_rel_a(p: Params, old: State, new: State, t: Micros) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! {
        p.mode.paces == Chambers::A && old.chamber@[0].pending.armed
            ==> new.chamber@[0].pending.armed
                && (new.chamber@[0].pending.at.0@ <= old.chamber@[0].pending.at.0@ || new.chamber@[0].pending.at.0@ <= crate::time::sum(t, p.lrl))
    }
}

/// What Property P1 supposes of the parameters: the URL interval is not
/// longer than the LRL interval (Table 19-5, for the Modes the URL limits),
/// the Mode paces the ventricle, its Paces are not triggered ones (P1's
/// exception), and a Mode with no response senses nothing (true of every
/// Mode a parameter set can name).
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn p1_params(p: Params) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! {
        p.url@ <= p.lrl@
            && (p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both)
            && p.mode.response != Response::Triggered
            && (p.mode.response == Response::None ==> p.mode.senses == Chambers::None)
    }
}

/// The instant of an event (a command has none: 0).
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn instant_of(e: Event) -> Micros {
    match e {
        Event::Sense { at, .. } => at,
        Event::Paced { at, .. } => at,
        Event::Tick { now, .. } => now,
        Event::Command(_) => Micros(0u64),
    }
}


/// For the App_Prover: a parameter set with none of the rate behaviour of
/// stage E4 (no rate adaptation, no hysteresis, no Rate Smoothing, no
/// ATR). For such a set the escape interval is the LRL interval and no
/// Mode switch comes from the step itself; the contracts of Properties P1
/// and P2 are stated for these sets until task 19.5 restates them.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn plain(p: Params) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! { !p.mode.adaptive && p.hrl@ == 0 && p.rate_smoothing@ == 0 && !p.atr }
}

/// The escape interval in force at `at` (PM9.1; stage E4: PM15, PM16,
/// PM14.6): the LRL interval, or the HRL interval after a Sense where
/// hysteresis is on; the sensor's interval if that is shorter, in a
/// rate-adaptive Mode; the Fallback interval if that is shorter, in ATR
/// Fallback; kept within the Rate Smoothing percentage of the cycle
/// before; and in the end not shorter than the URL interval (the MSR
/// interval in a rate-adaptive Mode) and not longer than the LRL or HRL
/// interval it started from.
#[cfg_attr(creusot, creusot_std::macros::ensures(plain(*p) ==> result == p.lrl))]
#[inline(always)]
fn escape(p: &Params, st: &State, at: Micros) -> u32 {
    let longest = if st.hysteresis_active && p.hrl > p.lrl { p.hrl } else { p.lrl };
    let mut e = longest;
    if p.mode.adaptive && st.sensor_known && st.sensor_interval < e {
        e = st.sensor_interval;
    }
    if p.atr && matches!(st.atr.phase, AtrPhase::Fallback) {
        let f = fallback_interval(p, st.atr.fallback_from, at.elapsed_since(st.atr.fallback_start));
        if f < e {
            e = f;
        }
    }
    if p.rate_smoothing != 0 && st.cycle_known {
        e = smoothed(e, st.last_cycle, p.rate_smoothing);
    }
    let shortest = if p.mode.adaptive { p.msr } else { p.url };
    if e < shortest {
        e = shortest;
    }
    if e > longest {
        e = longest;
    }
    e
}

/// The AV delay after an atrial Pace (PM11.2 to PM11.4): the Fixed AV
/// Delay; or, where the delay is dynamic and a cardiac cycle has been
/// measured, that cycle's length times the Fixed AV Delay over the LRL
/// interval, no shorter than the Minimum Dynamic AV Delay and no longer
/// than the Fixed AV Delay.
#[inline(always)]
const fn paced_av(p: &Params, st: &State) -> u32 {
    // Table 19-3: the delay is dynamic in the Modes that track only.
    if !p.dynamic_av || !p.mode.tracks() || !st.cycle_known || p.lrl == 0 {
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
    // Table 19-3: the offset is DDD's (and DDDR's).
    let offset = if p.mode.uses_sensed_av_offset() { p.sensed_av_offset } else { 0 };
    paced_av(p, st).saturating_sub(offset)
}

/// The escape interval of the atrium in the ventricular-based cycle: from a
/// ventricular event to the atrial Pace, so that the paced AV delay ends
/// one escape interval after the ventricular event.
#[inline(always)]
fn atrial_escape(p: &Params, st: &State, at: Micros) -> u32 {
    escape(p, st, at).saturating_sub(paced_av(p, st))
}

/// Whether the cycle is the ventricular-based one: the Mode paces the
/// ventricle (PM9.2). Otherwise each paced Chamber is timed from its own
/// events (PM9.3).
#[cfg_attr(creusot, creusot_std::macros::ensures(result == (p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both)))]
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
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*st).pvarp_extended && (*st).pvarp_extended_last_cycle) ==> !((^st).pvarp_extended && (^st).pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) ==> (^st).chamber@[1].pending.armed && (^st).chamber@[1].pending.at.0@ == crate::time::sum(v, p.lrl) && (^st).chamber@[1].last == v))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!(p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) ==> (^st).chamber@[0] == (*st).chamber@[0]))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.url@ <= p.lrl@ ==> p1_inv(*p, ^st)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel(*p, *st, ^st, v)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(forall<t: Micros> p2_rel_a(*p, *st, ^st, t)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).chamber@[1].seen && (^st).chamber@[1].last == v))]
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
    // A new cycle: noisy until a service after the refractory period is
    // not (PM13.1).
    ch.noisy = true;
    let paces_v = p.mode.paces(Chamber::V);
    if paces_v {
        // PM16.1, PM16.2: a Sense in the Chamber that starts the LRL
        // interval begins hysteresis, a Pace ends it.
        st.hysteresis_active = !paced && p.hrl != 0 && matches!(p.mode.response, Response::Inhibited | Response::Tracked);
    }
    atr_ventricular(p, st, v);
    let e = escape(p, st, v);
    st.chamber[Chamber::V.index()].pending = if paces_v { Pending::at(v.plus(e)) } else { Pending::NONE };
    st.atrial_since_ventricular = false;
    if ventricular_based(p) {
        // Property P8: not in two successive cycles.
        // Table 19-3: PVARP Extension is the tracking Modes'.
        let extend = pvc && p.mode.tracks() && p.pvarp_extension != 0 && !st.pvarp_extended;
        st.pvarp_extended_last_cycle = st.pvarp_extended;
        st.pvarp_extended = extend;
        let pvarp = if extend { p.pvarp.saturating_add(p.pvarp_extension) } else { p.pvarp };
        st.chamber[Chamber::A.index()].refractory_until = v.plus(pvarp);
        if p.mode.paces(Chamber::A) {
            let escape = atrial_escape(p, st, v);
            st.chamber[Chamber::A.index()].pending = Pending::at(v.plus(escape));
        }
    }
}

/// ATR at a ventricular event (PM14.4, PM14.6 to PM14.8), where the
/// programmed set has ATR on and a Mode that tracks. ATR Duration counts
/// the event; when the count is out with the tachycardia still there,
/// Fallback begins. Fallback ends when the tachycardia has. A change of
/// phase that changes the Mode in force (to VVIR and back) is marked as a
/// change waiting, and the Mode changes at the next ventricular event, as
/// every change of the State machine does.
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).chamber == (*st).chamber))]
#[cfg_attr(creusot, creusot_std::macros::ensures(plain(*p) ==> ^st == *st))]
#[inline(always)]
fn atr_ventricular(p: &Params, st: &mut State, v: Micros) {
    if !p.atr {
        return;
    }
    let tracks = st.own().mode.tracks();
    if !tracks {
        st.atr.phase = AtrPhase::Idle;
        st.atr.counter = 0;
        return;
    }
    match st.atr.phase {
        AtrPhase::Duration => {
            st.atr.cycles_left = st.atr.cycles_left.saturating_sub(1);
            if st.atr.cycles_left == 0 {
                st.atr.phase = AtrPhase::Fallback;
                st.atr.fallback_start = v;
                st.atr.fallback_from = if st.cycle_known { st.last_cycle } else { p.url };
                st.change_waiting = true;
            }
        }
        AtrPhase::Fallback => {
            if st.atr.counter == 0 {
                st.atr.phase = AtrPhase::Idle;
                st.change_waiting = true;
            }
        }
        AtrPhase::Idle => {}
    }
}

/// ATR at an atrial event at `a`, a refractory Sense included (PM14.1 to
/// PM14.5): the detection counter rises at an atrial interval shorter than
/// the URL interval and falls at a longer one; at PM-PAR-13 the onset is
/// detected and ATR Duration begins; at zero the tachycardia has ended,
/// which ends ATR Duration at once. Returns whether this event is the onset.
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy && (^st).chamber == (*st).chamber && (^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle))]
#[inline(always)]
fn atr_atrial(st: &mut State, a: Micros) -> bool {
    let own = *st.own();
    if !own.atr || !own.mode.tracks() {
        return false;
    }
    if st.atr.seen {
        if a.elapsed_since(st.atr.last_atrial) < own.url {
            if st.atr.counter < ATR_COUNT {
                st.atr.counter += 1;
            }
        } else if st.atr.counter > 0 {
            st.atr.counter -= 1;
        }
    }
    st.atr.seen = true;
    st.atr.last_atrial = a;
    match st.atr.phase {
        AtrPhase::Idle if st.atr.counter >= ATR_COUNT => {
            st.atr.phase = AtrPhase::Duration;
            st.atr.cycles_left = own.atr_duration;
            true
        }
        AtrPhase::Duration if st.atr.counter == 0 => {
            st.atr.phase = AtrPhase::Idle;
            false
        }
        _ => false,
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
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*st).chamber@[1].pending.armed ==> (^st).chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) ==> (^st).chamber@[0].pending.armed && (^st).chamber@[0].pending.at.0@ == crate::time::sum(a, p.lrl) && (^st).chamber@[0].last == a))]
// Property P1 on the path it concerns: the ventricular Pace an atrial event
// leaves pending is the one that was pending, or is not before the last
// ventricular event plus the URL interval.
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*st).chamber@[1].seen ==> (^st).chamber@[1].pending == (*st).chamber@[1].pending || (^st).chamber@[1].pending.at.0@ >= crate::time::sum((*st).chamber@[1].last, p.url)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p1_inv(*p, *st) ==> p1_inv(*p, ^st)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(forall<t: Micros> p2_rel(*p, *st, ^st, t)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel_a(*p, *st, ^st, a)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).chamber@[1].seen == (*st).chamber@[1].seen && (^st).chamber@[1].last == (*st).chamber@[1].last))]
#[inline(always)]
fn atrial_event(p: &Params, st: &mut State, a: Micros, paced: bool, starts_av: bool) {
    let based = ventricular_based(p);
    let delay = if paced { paced_av(p, st) } else { sensed_av(p, st) };
    let ch = &mut st.chamber[Chamber::A.index()];
    // The atrial cycle, for an atrial Mode's Rate Smoothing.
    let cycle = if ch.seen { Some(a.elapsed_since(ch.last)) } else { None };
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
        ch.noisy = true;
        let paces_a = p.mode.paces(Chamber::A);
        if paces_a {
            st.hysteresis_active = !paced && p.hrl != 0 && matches!(p.mode.response, Response::Inhibited | Response::Tracked);
        }
        if let Some(cycle) = cycle {
            st.last_cycle = cycle;
            st.cycle_known = true;
        }
        let e = escape(p, st, a);
        st.chamber[Chamber::A.index()].pending = if paces_a { Pending::at(a.plus(e)) } else { Pending::NONE };
    }
}

/// A Pace of `c` at `at`.
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*st).pvarp_extended && (*st).pvarp_extended_last_cycle) ==> !((^st).pvarp_extended && (^st).pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*st).chamber@[1].pending.armed ==> (^st).chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) && (*st).chamber@[0].pending.armed ==> (^st).chamber@[0].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.url@ <= p.lrl@ && p1_inv(*p, *st) ==> p1_inv(*p, ^st)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel(*p, *st, ^st, at)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel_a(*p, *st, ^st, at)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(((^st).chamber@[1].seen == (*st).chamber@[1].seen && (^st).chamber@[1].last == (*st).chamber@[1].last) || ((^st).chamber@[1].seen && (^st).chamber@[1].last == at)))]
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
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*st).pvarp_extended && (*st).pvarp_extended_last_cycle) ==> !((^st).pvarp_extended && (^st).pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*st).chamber@[1].pending.armed ==> (^st).chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) && (*st).chamber@[0].pending.armed ==> (^st).chamber@[0].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.url@ <= p.lrl@ && p1_inv(*p, *st) ==> p1_inv(*p, ^st)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel(*p, *st, ^st, instant)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel_a(*p, *st, ^st, instant)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(((^st).chamber@[1].seen == (*st).chamber@[1].seen && (^st).chamber@[1].last == (*st).chamber@[1].last) || ((^st).chamber@[1].seen && (^st).chamber@[1].last.0@ <= instant.0@)))]
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
#[cfg_attr(creusot, creusot_std::macros::ensures(result.therapy == s.therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*s).pvarp_extended && (*s).pvarp_extended_last_cycle) ==> !(result.pvarp_extended && result.pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*s).chamber@[1].pending.armed ==> result.chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) && (*s).chamber@[0].pending.armed ==> result.chamber@[0].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result.change_waiting == s.change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.url@ <= p.lrl@ && p1_inv(*p, *s) ==> p1_inv(*p, result)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel(*p, *s, result, instant)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p2_rel_a(*p, *s, result, instant)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(((result).chamber@[1].seen == (*s).chamber@[1].seen && (result).chamber@[1].last == (*s).chamber@[1].last) || ((result).chamber@[1].seen && (result).chamber@[1].last.0@ <= instant.0@)))]
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

#[cfg_attr(creusot, creusot_std::macros::ensures((^a).atrial == (*a).atrial && (^a).ventricular == (*a).ventricular && (^a).markers.augmentation == (*a).markers.augmentation))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match c { Chamber::A => (^a).markers.atrial == Some((marker, at)) && (^a).markers.ventricular == (*a).markers.ventricular, Chamber::V => (^a).markers.ventricular == Some((marker, at)) && (^a).markers.atrial == (*a).markers.atrial }))]
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
#[cfg_attr(creusot, creusot_std::macros::ensures((^out).markers == (*out).markers))]
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
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) ==> (^st).chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) ==> (^st).chamber@[0].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.url@ <= p.lrl@ && ((*st).chamber@[1].seen ==> now.0@ >= (*st).chamber@[1].last.0@) && p1_inv(*p, *st) ==> p1_inv(*p, ^st)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(forall<t: Micros> p2_rel(*p, *st, ^st, t)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(forall<t: Micros> p2_rel_a(*p, *st, ^st, t)))]
#[inline(always)]
fn tick(p: &Params, st: &mut State, now: Micros) {
    let paces_a = p.mode.paces(Chamber::A);
    if ventricular_based(p) {
        if !st.chamber[Chamber::V.index()].pending.armed {
            let e = escape(p, st, now);
            st.chamber[Chamber::V.index()].pending = Pending::at(now.plus(e));
            st.atrial_since_ventricular = false;
            if paces_a {
                let escape = atrial_escape(p, st, now);
                st.chamber[Chamber::A.index()].pending = Pending::at(now.plus(escape));
            }
        }
        if !paces_a {
            st.chamber[Chamber::A.index()].pending = Pending::NONE;
        }
    } else {
        st.chamber[Chamber::V.index()].pending = Pending::NONE;
        if paces_a {
            if !st.chamber[Chamber::A.index()].pending.armed {
                let e = escape(p, st, now);
                st.chamber[Chamber::A.index()].pending = Pending::at(now.plus(e));
            }
        } else {
            st.chamber[Chamber::A.index()].pending = Pending::NONE;
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

/// The Mode a battery level of ERT or ERP leaves of `m` (PM17.7): no rate
/// adaptation; DOO becomes VOO; every other Mode that paces or senses both
/// Chambers (VDD among them) becomes VVI; a Mode of one Chamber stays.
#[inline(always)]
const fn depleted(m: Mode) -> Mode {
    let both_paced = matches!(m.paces, crate::mode::Chambers::Both);
    let both_sensed = matches!(m.senses, crate::mode::Chambers::Both);
    if both_paced && matches!(m.senses, crate::mode::Chambers::None) {
        Mode::VOO
    } else if both_paced || both_sensed {
        Mode::VVI
    } else {
        Mode { adaptive: false, ..m }
    }
}

/// The Mode of the Magnet State over `m` (PM8.1): the Chambers `m` paces,
/// paced asynchronously; nothing where `m` paces nothing.
#[inline(always)]
const fn asynchronous(m: Mode) -> Mode {
    match m.paces {
        crate::mode::Chambers::A => Mode::AOO,
        crate::mode::Chambers::V => Mode::VOO,
        crate::mode::Chambers::Both => Mode::DOO,
        crate::mode::Chambers::None => Mode::OFF,
    }
}

/// The parameter set in force in the state `st`: the State's own set
/// (`State::own`), with the Mode a depleted battery leaves of it, and in
/// the Magnet State paced asynchronously at the rate that shows the
/// battery level: 100 ppm at BOL, 90 at ERN, 85 below (PM8.2).
#[inline(always)]
fn in_force(st: &State) -> Params {
    let mut p = *st.own();
    // PM14.6: in ATR Fallback the Mode is VVIR.
    if p.atr && p.mode.tracks() && matches!(st.atr.phase, AtrPhase::Fallback) {
        p.mode = Mode::VVI.rate_adaptive();
    }
    let low = matches!(st.battery_in_force, Battery::Ert | Battery::Erp | Battery::Tripped);
    if low {
        p.mode = depleted(p.mode);
    }
    if matches!(st.therapy, Therapy::Magnet) {
        p.mode = asynchronous(p.mode);
        p.lrl = match st.battery_in_force {
            Battery::Bol => interval_of_ppm(100),
            Battery::Ern => interval_of_ppm(90),
            _ => interval_of_ppm(85),
        };
    }
    p
}

/// One event applied to the state: the new state, and what to do with each
/// Chamber's compares. `p` is the parameter set in force.
///
/// Total, without side effect, allocation or loop, in integer arithmetic.
/// The State the requests ask for, put in force (design, "Therapy State
/// Machine"; PM6, PM7.4, PM8.1, PM8.3). In this order: a programmed set
/// becomes the permanent one, which takes Power-On Reset and Pace-Now to
/// Permanent; temporary pacing begins from Permanent while it is wanted and
/// the link is up, and ends when either is no longer so; the magnet
/// interrupts every State but Pace-Now and remembers it. Called at a timing
/// event; Pace-Now alone is entered at once, by its command.
///
/// Contract, Property P4: the State after is the State before or one the
/// State machine allows from it; nothing of the timing is touched.
#[cfg_attr(creusot, creusot_std::macros::ensures(allowed((*st).therapy, (^st).therapy)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle && (^st).chamber == (*st).chamber))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!(^st).change_waiting))]
#[inline(always)]
fn resolve(st: &mut State) {
    // The State under a Magnet State is the one it interrupted.
    let mut base = match (st.therapy, st.before_magnet) {
        (Therapy::Magnet, Therapy::Magnet) => Therapy::PowerOnReset,
        (Therapy::Magnet, b) => b,
        (t, _) => t,
    };
    // Temporary pacing is not entered from Power-On Reset or Pace-Now,
    // also not in the event that leaves them.
    if matches!(base, Therapy::PowerOnReset | Therapy::PaceNow) {
        st.temporary_wanted = false;
    }
    if st.staged_valid {
        st.permanent = st.staged;
        st.programmed = true;
        st.staged_valid = false;
        if matches!(base, Therapy::PowerOnReset | Therapy::PaceNow) {
            base = Therapy::Permanent;
        }
    }
    // The battery level comes into force here (PM17.7). At ERT and below
    // there is no temporary pacing.
    st.battery_in_force = st.battery;
    let low = matches!(st.battery_in_force, Battery::Ert | Battery::Erp | Battery::Tripped);
    let temporary = st.temporary_wanted && st.link_up && !low;
    base = match base {
        Therapy::Permanent if temporary => Therapy::Temporary,
        Therapy::Temporary if !temporary => Therapy::Permanent,
        b => b,
    };
    if !matches!(base, Therapy::Temporary) {
        st.temporary_wanted = false;
    }
    if st.magnet && !st.magnet_off && !matches!(base, Therapy::PaceNow) {
        st.therapy = Therapy::Magnet;
        st.before_magnet = base;
    } else {
        st.therapy = base;
        st.before_magnet = Therapy::PowerOnReset;
    }
    st.change_waiting = false;
    st.in_force = in_force(st);
    st.in_force_valid = true;
}

/// Whether [`resolve`] would change the State or the parameters: a change
/// is waiting for the next timing event.
#[inline(always)]
fn waiting(st: &State) -> bool {
    let mut t = *st;
    resolve(&mut t);
    st.staged_valid
        || t.therapy as u8 != st.therapy as u8
        || t.before_magnet as u8 != st.before_magnet as u8
        || st.battery as u8 != st.battery_in_force as u8
        // ATR Fallback began or ended: the Mode in force is to change.
        || crate::wire::mode_byte(t.in_force.mode) != crate::wire::mode_byte(st.params().mode)
}

/// A timing event of `chamber` at `at` at which a change was waiting: the
/// State is resolved, and the event is applied under the parameters now in
/// force, which it returns. Nothing stays pending from the Mode before: the
/// event starts the new Mode's cycle, and what the new Mode paces and the
/// event did not arm is armed as a tick arms it.
///
/// Contract: Property P4 as on `resolve`; Property P8 kept; and, of
/// Property P2, a ventricular Pace is pending after it in a Mode that
/// paces the ventricle, an atrial one in a Mode that paces the atrium alone.
#[cfg_attr(creusot, creusot_std::macros::ensures(allowed((*st).therapy, (^st).therapy)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*st).pvarp_extended && (*st).pvarp_extended_last_cycle) ==> !((^st).pvarp_extended && (^st).pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((result.mode.paces == Chambers::V || result.mode.paces == Chambers::Both) ==> (^st).chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((result.mode.paces == Chambers::A) ==> (^st).chamber@[0].pending.armed))]
#[inline(always)]
fn switched(st: &mut State, chamber: Chamber, at: Micros, paced: bool) -> Params {
    resolve(st);
    let np = *st.params();
    st.chamber[Chamber::A.index()].pending = Pending::NONE;
    st.chamber[Chamber::V.index()].pending = Pending::NONE;
    st.atrial_since_ventricular = false;
    match chamber {
        Chamber::V => ventricular_event(&np, st, at, paced, false),
        Chamber::A => {
            let starts_av = (paced && np.mode.paces(Chamber::A)) || (!paced && np.mode.tracks());
            atrial_event(&np, st, at, paced, starts_av);
        }
    }
    tick(&np, st, at);
    np
}

/// Whether `c` is the Chamber whose events are the Mode's timing events:
/// the ventricle, or the atrium in a Mode of the atrium alone (PM9.2, PM9.3).
#[inline(always)]
fn is_timing(p: &Params, c: Chamber) -> bool {
    matches!((p.mode.timing_chamber(), c), (Chamber::A, Chamber::A) | (Chamber::V, Chamber::V))
}

/// A service of `chamber`'s sense line at `at` that reported `edges`
/// edges, for the noise rule (PM13.1; design, "Sense intake, MIT and
/// noise"). The service is "close" when it reports more than one edge or
/// comes within PM-PAR-12 of the service before. A Chamber is noisy for
/// the cycle while every service since the end of its refractory period
/// has been close. Returns whether this service finds the cycle noisy: the
/// Sense is then no Sense. A service in a refractory period is only
/// recorded (its instant is what the next service is close to or not); a
/// blanked edge and an edge of a Chamber the Mode does not sense leave
/// no record.
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).therapy == (*st).therapy && (^st).change_waiting == (*st).change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).pvarp_extended == (*st).pvarp_extended && (^st).pvarp_extended_last_cycle == (*st).pvarp_extended_last_cycle))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).chamber@[0].pending == (*st).chamber@[0].pending && (^st).chamber@[1].pending == (*st).chamber@[1].pending))]
#[cfg_attr(creusot, creusot_std::macros::ensures((^st).chamber@[1].seen == (*st).chamber@[1].seen && (^st).chamber@[1].last == (*st).chamber@[1].last))]
#[inline(always)]
fn serviced(p: &Params, st: &mut State, chamber: Chamber, at: Micros, edges: u32) -> bool {
    // A Chamber the Mode does not sense, and a blanked edge, leave no
    // record: nothing of the state changes for them.
    if !p.mode.senses(chamber) || blanked(st, chamber, at) {
        return false;
    }
    let ch = st.chamber[chamber.index()];
    let close = edges > 1 || (ch.serviced && at.elapsed_since(ch.last_service) < NOISE_INTERVAL_US);
    let counted = !refractory(p, st, chamber, at);
    let noisy = counted && ch.noisy && close;
    let c = &mut st.chamber[chamber.index()];
    c.serviced = true;
    c.last_service = at;
    if counted {
        c.noisy = noisy;
    }
    noisy
}

/// A Sense of `chamber` at `at`, applied to the accounted state `before`.
///
/// Contract (task 12.1), Property P3 by what the step returns: a Sense it
/// marks as a Refractory sense, and a Sense it gives no Marker (one the
/// Mode does not sense, or a blanked one), leave the accounted state as it
/// was and arm and withdraw nothing. The other contracts are those the
/// step keeps.
#[cfg_attr(creusot, creusot_std::macros::ensures(result.0.therapy == before.therapy))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*before).pvarp_extended && (*before).pvarp_extended_last_cycle) ==> !(result.0.pvarp_extended && result.0.pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*before).chamber@[1].pending.armed ==> result.0.chamber@[1].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures((p.mode.paces == Chambers::A) && (*before).chamber@[0].pending.armed ==> result.0.chamber@[0].pending.armed))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result.1.markers.atrial == None && result.1.markers.ventricular == None ==> result.0 == *before && result.1.atrial == Pace::Keep && result.1.ventricular == Pace::Keep))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result.1.markers.atrial == Some((Marker::Sense { refractory: true }, at)) || result.1.markers.ventricular == Some((Marker::Sense { refractory: true }, at)) ==> result.0 == *before && result.1.atrial == Pace::Keep && result.1.ventricular == Pace::Keep))]
#[cfg_attr(creusot, creusot_std::macros::ensures(result.0.change_waiting == before.change_waiting))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p1_params(*p) && p1_inv(*p, *before) ==> p1_inv(*p, result.0)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.mode.response != Response::Triggered ==> p2_rel(*p, *before, result.0, at)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(p.mode.response != Response::Triggered ==> p2_rel_a(*p, *before, result.0, at)))]
#[inline(always)]
fn sense(p: &Params, before: &State, chamber: Chamber, at: Micros) -> (State, Arming) {
    let mut out = Arming::NONE;
    let mut st = *before;
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
    arm_difference(p, before, &st, &mut out);
    set_marker(&mut out, chamber, Marker::Sense { refractory: false }, at);
    (st, out)
}

/// Contracts (tasks 12.1, 12.2, 16.2): Property P4 (the therapy State
/// returned is the one given or one the State machine allows from it);
/// Property P8 (kept); Property P2's "a Pace is pending" for the ventricle
/// in a Mode that paces it and for the atrium in a Mode that paces it alone
/// (kept by every event at which no change of State was waiting, made so by
/// a tick; at a change, `switched` makes it so for the new Mode; a command
/// is not an event of the heart and Pace-Now withdraws what is pending
/// until the next tick).
#[cfg_attr(creusot, creusot_std::macros::ensures(allowed(s.therapy, result.0.therapy)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(!((*s).pvarp_extended && (*s).pvarp_extended_last_cycle) ==> !(result.0.pvarp_extended && result.0.pvarp_extended_last_cycle)))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Command(_) => true, _ => (p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) && (*s).chamber@[1].pending.armed && !(*s).change_waiting ==> result.0.chamber@[1].pending.armed }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Command(_) => true, _ => (p.mode.paces == Chambers::A) && (*s).chamber@[0].pending.armed && !(*s).change_waiting ==> result.0.chamber@[0].pending.armed }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Tick { .. } => ((p.mode.paces == Chambers::V || p.mode.paces == Chambers::Both) ==> result.0.chamber@[1].pending.armed) && ((p.mode.paces == Chambers::A) ==> result.0.chamber@[0].pending.armed), _ => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Paced { .. } => !s.change_waiting && p1_params(*p) && p1_inv(*p, *s) ==> p1_inv(*p, result.0), _ => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Sense { .. } => !s.change_waiting && p1_params(*p) && p1_inv(*p, *s) ==> p1_inv(*p, result.0), _ => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Tick { now, .. } => !s.change_waiting && p1_params(*p) && (s.chamber@[1].seen ==> now.0@ >= s.chamber@[1].last.0@) && p1_inv(*p, *s) ==> p1_inv(*p, result.0), _ => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Command(_) => true, _ => !s.change_waiting && p.mode.response != Response::Triggered ==> p2_rel(*p, *s, result.0, instant_of(e)) }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match e { Event::Command(_) => true, _ => !s.change_waiting && p.mode.response != Response::Triggered ==> p2_rel_a(*p, *s, result.0, instant_of(e)) }))]
#[inline(always)]
#[must_use]
pub fn step(p: &Params, s: &State, e: Event) -> (State, Arming) {
    let mut out = Arming::NONE;
    match e {
        Event::Paced { chamber, at } => {
            let mut st = *s;
            // The other Chamber's Pace, if it was due by now.
            account(p, &mut st, chamber.other(), at);
            if matches!(chamber, Chamber::A) {
                let _ = atr_atrial(&mut st, at);
            }
            let before = st;
            let ch = st.chamber[chamber.index()];
            // A Pace the state no longer had pending at this instant: a
            // Sense moved or withdrew it too late for the compare (PD-04).
            let expected = ch.unconfirmed || (ch.pending.armed && at.is_at_or_after(ch.pending.at));
            if st.change_waiting && is_timing(p, chamber) {
                // The State machine: the change in force from this event.
                let np = switched(&mut st, chamber, at, true);
                st.chamber[chamber.index()].unconfirmed = false;
                arm_difference(&np, &before, &st, &mut out);
                set_pace(&mut out, chamber, arm_from(&np, &st, chamber));
                set_marker(&mut out, chamber, Marker::Pace { fused: !expected }, at);
                return (st, out);
            }
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
        Event::Sense { chamber, at, edges } => {
            // Property P3 is stated on `sense`, against this state: the
            // accounted one, with what every service of a sense line
            // records whatever comes of it (the atrial interval for ATR,
            // the service for the noise rule).
            let mut before = accounted(p, s, at);
            let onset = matches!(chamber, Chamber::A) && atr_atrial(&mut before, at);
            if serviced(p, &mut before, chamber, at, edges) {
                // PM13.1, PM13.2: a noisy cycle. The pending Pace stays
                // where it is, whatever was sensed: asynchronous pacing.
                set_marker(&mut out, chamber, Marker::Noise, at);
                return (before, out);
            }
            if onset {
                out.markers.augmentation = Some((Marker::AtrialTachy, at));
            }
            if before.change_waiting && is_timing(p, chamber) && p.mode.senses(chamber) && !blanked(&before, chamber, at) && !refractory(p, &before, chamber, at) {
                // The State machine: the change in force from this Sense.
                let mut st = before;
                let np = switched(&mut st, chamber, at, false);
                arm_difference(&np, &before, &st, &mut out);
                set_marker(&mut out, chamber, Marker::Sense { refractory: false }, at);
                return (st, out);
            }
            let (st, mut sensed) = sense(p, &before, chamber, at);
            if onset && sensed.markers.augmentation.is_none() {
                sensed.markers.augmentation = out.markers.augmentation;
            }
            (st, sensed)
        }
        Event::Tick { now, magnet, link_up, battery, activity } => {
            let before = accounted(p, s, now);
            let mut st = before;
            // PM15: the sensor rate, one step per period, in a
            // rate-adaptive Mode; it is in force from the next event that
            // starts an escape interval.
            if p.mode.adaptive {
                let from = if st.sensor_known { st.sensor_rate } else { 0 };
                st.sensor_rate = sensor_next(p, from, activity);
                st.sensor_interval = interval_of(st.sensor_rate);
                st.sensor_known = true;
            } else {
                st.sensor_known = false;
            }
            // The inputs of the State machine, and whether they ask for a
            // change: it comes into force at the next timing event.
            st.magnet = magnet;
            st.link_up = link_up;
            st.battery = battery;
            st.change_waiting = waiting(&st);
            if st.change_waiting && matches!(p.mode.paces, crate::mode::Chambers::None) {
                // A Mode that paces nothing may have no timing event to
                // come: the change is in force from this tick.
                resolve(&mut st);
                let np = *st.params();
                st.chamber[Chamber::A.index()].pending = Pending::NONE;
                st.chamber[Chamber::V.index()].pending = Pending::NONE;
                st.atrial_since_ventricular = false;
                tick(&np, &mut st, now);
                arm_difference(&np, &before, &st, &mut out);
            } else {
                tick(p, &mut st, now);
                arm_difference(p, &before, &st, &mut out);
            }
            (st, out)
        }
        Event::Command(c) => {
            let mut st = *s;
            match c {
                Command::Program(set) => {
                    // PM7.4, PM6.7: in force, and the State with it, at the
                    // next timing event.
                    st.staged = set;
                    st.staged_valid = true;
                }
                Command::StartTemporary(set) => {
                    // PM6.3: from the Permanent State (also under the
                    // magnet), with the link up; otherwise the command is
                    // without effect. The set is not in force until the
                    // State is.
                    let base = match st.therapy {
                        Therapy::Magnet => st.before_magnet,
                        t => t,
                    };
                    let low = matches!(st.battery, Battery::Ert | Battery::Erp | Battery::Tripped);
                    if matches!(base, Therapy::Permanent) && st.link_up && !low {
                        st.temporary = set;
                        st.temporary_wanted = true;
                    }
                }
                Command::EndTemporary => st.temporary_wanted = false,
                // PM8.4: in force, with the State, at the next timing event.
                Command::MagnetResponse(on) => st.magnet_off = !on,
                Command::PaceNow => {
                    // PM6.5: at once. What was pending is withdrawn; the
                    // next tick arms the first Pace-Now pulse, one Pace-Now
                    // escape interval from it (PM6.6).
                    if !matches!(st.therapy, Therapy::PaceNow) {
                        st.therapy = Therapy::PaceNow;
                        st.before_magnet = Therapy::PowerOnReset;
                        st.temporary_wanted = false;
                        st.staged_valid = false;
                        st.chamber[Chamber::A.index()].pending = Pending::NONE;
                        st.chamber[Chamber::V.index()].pending = Pending::NONE;
                        st.atrial_since_ventricular = false;
                        st.in_force = in_force(&st);
                        st.in_force_valid = true;
                        arm_difference(p, s, &st, &mut out);
                    }
                }
            }
            st.change_waiting = waiting(&st);
            (st, out)
        }
    }
}
