//! The pacing threshold test (docs/pacemaker task 22.6; PM17.4): the next
//! lower setting of the value under test.
//!
//! The test runs in the Temporary State: the DCM starts temporary pacing
//! with the back-up Mode and rate and the start value, and names the
//! value under test. About every fourth cardiac cycle the Partition's
//! `control` starts temporary pacing again with the value one setting
//! lower (Table 19-1's settings), until the lowest. A Stop, or the loss
//! of the link, ends temporary pacing and the test with it.

use crate::params::Params;
// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// What is under test: an amplitude or a pulse width, atrial or ventricular.
pub mod what {
    pub const ATRIAL_AMPLITUDE: u32 = 0;
    pub const ATRIAL_WIDTH: u32 = 1;
    pub const VENTRICULAR_AMPLITUDE: u32 = 2;
    pub const VENTRICULAR_WIDTH: u32 = 3;
}

/// The cardiac cycles at each setting.
pub const CYCLES: u32 = 4;

/// The next lower pulse width of Table 19-1 (0.05 ms, then 0.1 to 1.9 ms
/// by 0.1), in microseconds; `None` below the lowest.
///
/// Contract (2026-11-25): what is returned is lower than `us` and is a
/// setting of the table; there is none exactly when `us` is at or below
/// the lowest; and from a setting it is the next one down (nothing of the
/// table lies between).
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(w) => w@ < us@ && (w@ == 50 || (100 <= w@ && w@ <= 1_900 && w@ % 100 == 0)), None => us@ <= 50 }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(w) => us@ <= 1_900 ==> us@ - w@ <= 100, None => true }))]
#[inline(always)]
#[must_use]
pub const fn lower_width(us: u32) -> Option<u32> {
    if us > 1_900 {
        Some(1_900)
    } else if us > 100 {
        // The setting below: the next multiple of 100 under it.
        Some((us - 1) / 100 * 100)
    } else if us > 50 {
        Some(50)
    } else {
        None
    }
}

/// The next lower amplitude of Table 19-1 (0.5 to 3.2 V by 0.1, then 3.5
/// to 7.0 V by 0.5), in tenths of a volt; `None` below the lowest. Off
/// (0) is not a setting of the test.
///
/// Contract (2026-11-25): what is returned is lower than `tenths` and is a
/// setting of the table; there is none exactly when `tenths` is at or
/// below the lowest; and the step is at most one of the table's.
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(a) => a@ < tenths@ && ((5 <= a@ && a@ <= 32) || (35 <= a@ && a@ <= 70 && a@ % 5 == 0)), None => tenths@ <= 5 }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(a) => (tenths@ <= 32 ==> a@ == tenths@ - 1) && (tenths@ <= 70 ==> tenths@ - a@ <= 5), None => true }))]
#[inline(always)]
#[must_use]
pub const fn lower_amplitude(tenths: u8) -> Option<u8> {
    if tenths > 70 {
        Some(70)
    } else if tenths > 35 {
        Some((tenths - 1) / 5 * 5)
    } else if tenths > 32 {
        Some(32)
    } else if tenths > 5 {
        Some(tenths - 1)
    } else {
        None
    }
}

/// The set `p` with the value `which` one setting lower; `None` when the
/// value is at its lowest setting, or `which` names nothing.
///
/// Contract (2026-11-25): the set returned differs from `p` in the value
/// under test alone, which is lower; every other value, the Mode and the
/// rates among them, is `p`'s.
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(n) => n.mode == p.mode && n.lrl == p.lrl && n.url == p.url && n.vrp == p.vrp && n.arp == p.arp && n.pvarp == p.pvarp, None => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(match result { Some(n) =>
    (which@ == 0 ==> n.amplitude@[0]@ < p.amplitude@[0]@ && n.amplitude@[1] == p.amplitude@[1] && n.width == p.width)
    && (which@ == 2 ==> n.amplitude@[1]@ < p.amplitude@[1]@ && n.amplitude@[0] == p.amplitude@[0] && n.width == p.width)
    && (which@ == 1 ==> n.width@[0]@ < p.width@[0]@ && n.width@[1] == p.width@[1] && n.amplitude == p.amplitude)
    && (which@ == 3 ==> n.width@[1]@ < p.width@[1]@ && n.width@[0] == p.width@[0] && n.amplitude == p.amplitude),
  None => true }))]
#[cfg_attr(creusot, creusot_std::macros::ensures(which@ > 3 ==> result == None))]
#[inline(always)]
#[must_use]
pub const fn step_down(p: &Params, which: u32) -> Option<Params> {
    let mut next = *p;
    if which == what::ATRIAL_AMPLITUDE {
        match lower_amplitude(p.amplitude[0]) {
            Some(a) => next.amplitude = [a, p.amplitude[1]],
            None => return None,
        }
    } else if which == what::VENTRICULAR_AMPLITUDE {
        match lower_amplitude(p.amplitude[1]) {
            Some(a) => next.amplitude = [p.amplitude[0], a],
            None => return None,
        }
    } else if which == what::ATRIAL_WIDTH {
        match lower_width(p.width[0]) {
            Some(w) => next.width = [w, p.width[1]],
            None => return None,
        }
    } else if which == what::VENTRICULAR_WIDTH {
        match lower_width(p.width[1]) {
            Some(w) => next.width = [p.width[0], w],
            None => return None,
        }
    } else {
        return None;
    }
    Some(next)
}
