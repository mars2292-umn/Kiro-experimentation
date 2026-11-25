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
