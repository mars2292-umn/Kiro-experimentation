//! The messages of the Endpoints `program` and `status` (design,
//! "Endpoints"; task 16.1), as words: every bit pattern is a value of the
//! payload's type, and the meaning is decided here.
//!
//! **`program`**, from `comms.link` to `therapy.control`:
//! [`PROGRAM_WORDS`] words. Word 0 is a code in its low byte and, in the
//! byte above, the sequence number of the DCM's frame that caused the
//! message; words 1 to 29 are a parameter set for the two codes that carry
//! one, a value in word 1 for the two that carry a value, and zero
//! otherwise. `link` sends one message for every frame
//! of the DCM's it hears: a command's, or [`code::HEARD`] for any other,
//! so that `control` knows the link by its messages (PM20.2).
//! [`admitted`] is the validation function `therapy` declares for the
//! Endpoint (PM19.3, PM22.3): it accepts exactly these messages, with a
//! set exactly when `validate` accepts the set.
//!
//! **`status`**, from `therapy.control` to `comms.link`: [`STATUS_WORDS`]
//! words ([`Status`]).

use crate::event::Battery;
use crate::mode::Mode;
use crate::state::Therapy;
use crate::validate::{validate, WORDS};
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// The words of a `program` message and of a `status` message.
pub const PROGRAM_WORDS: usize = 30;
pub const STATUS_WORDS: usize = 4;

/// The codes of a `program` message: the kind of the DCM's frame for a
/// command (`frame::kind`), and `HEARD` for a frame that is none.
pub mod code {
    pub const HEARD: u8 = 0x01;
    pub const PROGRAM: u8 = 0x10;
    pub const TEMPORARY: u8 = 0x11;
    pub const END_TEMPORARY: u8 = 0x12;
    pub const PACE_NOW: u8 = 0x13;
    /// The battery level, in word 1: 0 BOL, 1 ERN, 2 ERT, 3 ERP, 4 below
    /// the Power-On Reset trip level (PM29.2, PM30.3).
    pub const BATTERY: u8 = 0x20;
    /// The activity value, in word 1: 0 to 255 (PM29.3).
    pub const ACTIVITY: u8 = 0x21;
    /// The magnet response, in word 1: 1 on, 0 off (PM8.4).
    pub const MAGNET_RESPONSE: u8 = 0x22;
    /// The pacing threshold test started, the value under test in word 1
    /// (`threshold::what`, 0 to 3), and stopped (PM17.4; task 22.6).
    pub const THRESHOLD_START: u8 = 0x34;
    pub const THRESHOLD_STOP: u8 = 0x35;
}

/// What `control` answers a command with (a STATUS frame's code; the
/// numbers below 6 are `link`'s own answers).
pub mod verdict {
    pub const NONE: u8 = 0;
    /// The command was taken: a set is staged, temporary pacing is wanted
    /// or ended, Pace-Now is entered.
    pub const ACCEPTED: u8 = 6;
    /// The validation function refused a message (the Kernel has recorded
    /// it against `comms`); nothing was delivered, so no frame is named.
    pub const REFUSED: u8 = 7;
    /// A valid command the State does not allow (temporary pacing from
    /// another State than Permanent, or with the link not up).
    pub const DECLINED: u8 = 8;
}

/// A `program` message of `code` for the frame numbered `seq`, without a set.
#[inline(always)]
#[must_use]
pub const fn message(code: u8, seq: u8) -> [u32; PROGRAM_WORDS] {
    let mut m = [0u32; PROGRAM_WORDS];
    m[0] = code as u32 + seq as u32 * 0x100;
    m
}

/// A `program` message of `code` with the set `words`.
#[inline(always)]
#[must_use]
pub fn message_with_set(code: u8, seq: u8, words: &[u32; WORDS]) -> [u32; PROGRAM_WORDS] {
    let mut m = message(code, seq);
    let mut k = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(WORDS@ - k@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 29 && m@.len() == 30))]
    while k < WORDS {
        m[k + 1] = words[k];
        k += 1;
    }
    m
}

/// A `program` message of `code` with one value, in word 1.
#[inline(always)]
#[must_use]
pub const fn message_with_value(code: u8, seq: u8, value: u32) -> [u32; PROGRAM_WORDS] {
    let mut m = message(code, seq);
    m[1] = value;
    m
}

/// The value of a message that carries one.
#[inline(always)]
#[must_use]
pub const fn value_of(m: &[u32; PROGRAM_WORDS]) -> u32 {
    m[1]
}

/// The code and the sequence number of a message.
#[inline(always)]
#[must_use]
pub const fn code_of(m: &[u32; PROGRAM_WORDS]) -> u8 {
    (m[0] & 0xFF) as u8
}

#[inline(always)]
#[must_use]
pub const fn seq_of(m: &[u32; PROGRAM_WORDS]) -> u8 {
    ((m[0] / 0x100) & 0xFF) as u8
}

/// The set a message carries (zeros in a message without one).
#[inline(always)]
#[must_use]
pub fn set_of(m: &[u32; PROGRAM_WORDS]) -> [u32; WORDS] {
    let mut w = [0u32; WORDS];
    let mut k = 0;
    #[cfg_attr(creusot, creusot_std::macros::variant(WORDS@ - k@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 29 && w@.len() == 29))]
    while k < WORDS {
        w[k] = m[k + 1];
        k += 1;
    }
    w
}

/// Whether `m` is a message of the Endpoint `program`: word 0 a code and a
/// sequence number and nothing above them; with PROGRAM or TEMPORARY a set
/// `validate` accepts; with BATTERY a level, with ACTIVITY a value up to
/// 255, with MAGNET_RESPONSE 0 or 1 and with THRESHOLD_START 0 to 3
/// in word 1 and zeros after it; with the other codes zeros.
#[inline(always)]
#[must_use]
pub fn admitted(m: &[u32; PROGRAM_WORDS]) -> bool {
    if m[0] > 0xFFFF {
        return false;
    }
    let set = set_of(m);
    let c = code_of(m);
    if c == code::PROGRAM || c == code::TEMPORARY {
        return validate(&set).is_some();
    }
    // A value in word 1: a battery level, or on and off.
    let first = if c == code::BATTERY {
        if set[0] > 4 {
            return false;
        }
        1
    } else if c == code::ACTIVITY {
        if set[0] > 255 {
            return false;
        }
        1
    } else if c == code::MAGNET_RESPONSE {
        if set[0] > 1 {
            return false;
        }
        1
    } else if c == code::THRESHOLD_START {
        if set[0] > 3 {
            return false;
        }
        1
    } else if c == code::HEARD || c == code::END_TEMPORARY || c == code::PACE_NOW || c == code::THRESHOLD_STOP {
        0
    } else {
        return false;
    };
    let mut k = first;
    #[cfg_attr(creusot, creusot_std::macros::variant(WORDS@ - k@))]
    #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 29))]
    while k < WORDS {
        if set[k] != 0 {
            return false;
        }
        k += 1;
    }
    true
}

/// The Mode as one byte of a status: the Chambers paced in bits 0 and 1,
/// the Chambers sensed in bits 2 and 3, the response in bits 4 and 5, rate
/// adaptation in bit 6.
#[inline(always)]
#[must_use]
pub const fn mode_byte(m: Mode) -> u8 {
    m.paces as u8 % 4 + (m.senses as u8 % 4) * 4 + (m.response as u8 % 4) * 16 + if m.adaptive { 64 } else { 0 }
}

/// What `control` tells `link`.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Status {
    /// The State (`Therapy` as its number), the Mode in force
    /// ([`mode_byte`]), the battery level, whether the link is up.
    pub therapy: u8,
    pub mode: u8,
    pub battery: u8,
    pub link_up: bool,
    /// The number of verdicts given since the Partition started.
    pub serial: u32,
    /// The last verdict, and the frame it answers (none when refused).
    pub verdict: u8,
    pub of: u8,
    /// Messages the validation function refused.
    pub refused: u32,
}

impl Status {
    #[inline(always)]
    #[must_use]
    pub const fn words(&self) -> [u32; STATUS_WORDS] {
        [
            self.therapy as u32 + self.mode as u32 * 0x100 + self.battery as u32 * 0x1_0000 + if self.link_up { 0x100_0000 } else { 0 },
            self.serial,
            self.verdict as u32 + self.of as u32 * 0x100,
            self.refused,
        ]
    }

    #[inline(always)]
    #[must_use]
    pub const fn of_words(w: &[u32; STATUS_WORDS]) -> Status {
        Status {
            therapy: (w[0] & 0xFF) as u8,
            mode: ((w[0] / 0x100) & 0xFF) as u8,
            battery: ((w[0] / 0x1_0000) & 0xFF) as u8,
            link_up: (w[0] / 0x100_0000) & 1 == 1,
            serial: w[1],
            verdict: (w[2] & 0xFF) as u8,
            of: ((w[2] / 0x100) & 0xFF) as u8,
            refused: w[3],
        }
    }
}

/// The State as a status byte.
#[inline(always)]
#[must_use]
pub const fn therapy_byte(t: Therapy) -> u8 {
    t as u8
}

/// The battery level as a status byte.
#[inline(always)]
#[must_use]
pub const fn battery_byte(b: Battery) -> u8 {
    match b {
        Battery::Bol => 0,
        Battery::Ern => 1,
        Battery::Ert => 2,
        Battery::Erp => 3,
        Battery::Tripped => 4,
    }
}

/// The battery level a BATTERY message's value names; a value that names
/// none is the lowest level (the validation function admits none such).
#[inline(always)]
#[must_use]
pub const fn battery_of(value: u32) -> Battery {
    match value {
        0 => Battery::Bol,
        1 => Battery::Ern,
        2 => Battery::Ert,
        3 => Battery::Erp,
        _ => Battery::Tripped,
    }
}
