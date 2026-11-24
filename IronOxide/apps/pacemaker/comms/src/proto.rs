//! What `link` answers (design, "The link"; task 15.3). No register and no
//! Kernel service is named here: the Task gives [`answer`] the frame of the
//! last transfer, or `None`, and sends the frame it returns.
//!
//! One frame goes each way in every period, so the answer to the frame of
//! period n is sent in period n + 1.
//!
//! | The DCM's frame | The answer |
//! |---|---|
//! | none, or bytes that are no frame | the next part of a read in progress, else IDLE |
//! | POLL | the same |
//! | INTERROGATE | IDENTITY: the answered `seq`, three zero bytes, the model, the serial number |
//! | PROGRAM_PART, TEMPORARY_PART | STATUS with RECEIVED, SET_COMPLETE or BAD_PART |
//! | READ_PARAMETERS | part 0 of the last complete set as PARAM_PART, the other parts in the periods after; STATUS with NO_SET if there is none |
//! | any other kind of the DCM's | STATUS with NOT_SERVED |
//! | a kind of the PG's | as bytes that are no frame |
//!
//! Until the Endpoints of task 16.1 exist, a complete set goes no further
//! than this Partition: `link` keeps it and READ_PARAMETERS reads it back.
//! It is not the set in force, and the bytes 2 to 4 of a STATUS (the State,
//! the Mode in force, the battery level) are `UNKNOWN`.

use pm_core::frame::{kind, set_part, words_in_part, Assembly, Frame, PARTS};
use pm_core::WORDS;
#[cfg(creusot)]
#[allow(unused_imports)]
use rsk::creusot_std::model::View;

/// The device model ("PMG1") and, until a device has its own, the serial
/// number of the rig's pulse generator (PM20.3).
pub const MODEL: u32 = 0x504D_4731;
pub const SERIAL: u32 = 1;

/// The answer codes of a STATUS frame (its byte 1).
pub mod code {
    pub const NONE: u8 = 0;
    /// A part was taken; the set is not complete.
    pub const RECEIVED: u8 = 1;
    /// The part completed a set.
    pub const SET_COMPLETE: u8 = 2;
    /// READ_PARAMETERS with no complete set.
    pub const NO_SET: u8 = 3;
    /// A command this stage does not serve.
    pub const NOT_SERVED: u8 = 4;
    /// A part of a wrong length, or of another kind than the set begun.
    pub const BAD_PART: u8 = 5;
}

/// A STATUS byte `link` has no value for.
pub const UNKNOWN: u8 = 0xFF;
/// The payload bytes of a STATUS and of an IDENTITY.
pub const STATUS_LEN: u8 = 8;
pub const IDENTITY_LEN: u8 = 12;

/// The link's state: per-Task state of `link`. All zero at the start of an
/// incarnation, which is "nothing heard, no set" (no padding byte: every
/// field is a word or of bytes in fours).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Link {
    /// Frames sent; its low byte is the next frame's `seq`.
    pub sent: u32,
    /// Frames of the DCM's received.
    pub heard: u32,
    /// Transfers whose bytes were no frame of the DCM's.
    pub unheard: u32,
    /// The periods since the last frame of the DCM's (saturating).
    pub silent: u32,
    /// Sets completed.
    pub sets: u32,
    /// 1 when `set` is a complete set, of kind `set_kind`.
    pub have_set: u32,
    pub set_kind: u32,
    /// The kind of the set `assembly` is of.
    pub assembly_kind: u32,
    /// The part of a read to send next; 0 when no read is in progress.
    pub read_next: u32,
    pub assembly: Assembly,
    pub set: [u32; WORDS],
}

impl Link {
    #[must_use]
    pub const fn new() -> Link {
        Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; WORDS] }
    }
}

impl Default for Link {
    fn default() -> Link {
        Link::new()
    }
}

/// A STATUS frame answering the frame `of` with `code`.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result.kind == kind::STATUS))]
fn status(seq: u8, of: u8, code: u8) -> Frame {
    let mut f = Frame::empty(seq, kind::STATUS);
    f.len = STATUS_LEN;
    f.payload[0] = of;
    f.payload[1] = code;
    f.payload[2] = UNKNOWN;
    f.payload[3] = UNKNOWN;
    f.payload[4] = UNKNOWN;
    f
}

/// The four bytes of `w`, least significant first, at `at` of `p`.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(at@ + 4 <= 24))]
fn put_word(p: &mut [u8; 24], at: usize, w: u32) {
    p[at] = (w & 0xFF) as u8;
    p[at + 1] = ((w >> 8) & 0xFF) as u8;
    p[at + 2] = ((w >> 16) & 0xFF) as u8;
    p[at + 3] = ((w >> 24) & 0xFF) as u8;
}

/// The next part of a read in progress, or IDLE.
fn idle_or_part(l: &mut Link, seq: u8) -> Frame {
    if l.read_next >= 1 && l.read_next < PARTS as u32 && l.have_set == 1 {
        let part = l.read_next as u8;
        l.read_next = if l.read_next + 1 < PARTS as u32 { l.read_next + 1 } else { 0 };
        if let Some(f) = set_part(seq, kind::PARAM_PART, &l.set, part) {
            return f;
        }
    }
    l.read_next = 0;
    Frame::empty(seq, kind::IDLE)
}

/// The frame to send after `got`, the frame of the last transfer (`None`
/// when its bytes were no frame, or no transfer ended).
#[must_use]
pub fn answer(l: &mut Link, got: Option<&Frame>) -> Frame {
    let seq = (l.sent & 0xFF) as u8;
    l.sent = l.sent.wrapping_add(1);
    let f = match got {
        Some(f) if f.kind < 0x80 => f,
        _ => {
            l.unheard = l.unheard.saturating_add(1);
            l.silent = l.silent.saturating_add(1);
            return idle_or_part(l, seq);
        }
    };
    l.heard = l.heard.wrapping_add(1);
    l.silent = 0;
    if f.kind == kind::POLL {
        return idle_or_part(l, seq);
    }
    // Any command ends a read in progress.
    l.read_next = 0;
    if f.kind == kind::INTERROGATE {
        let mut a = Frame::empty(seq, kind::IDENTITY);
        a.len = IDENTITY_LEN;
        a.payload[0] = f.seq;
        put_word(&mut a.payload, 4, MODEL);
        put_word(&mut a.payload, 8, SERIAL);
        return a;
    }
    if f.kind == kind::PROGRAM_PART || f.kind == kind::TEMPORARY_PART {
        if f.part != 0 && l.assembly_kind != f.kind as u32 {
            l.assembly.have = 0;
            return status(seq, f.seq, code::BAD_PART);
        }
        l.assembly_kind = f.kind as u32;
        if f.part as usize >= PARTS || f.len as usize != 4 * words_in_part(f.part as usize) {
            l.assembly.have = 0;
            return status(seq, f.seq, code::BAD_PART);
        }
        return match l.assembly.accept(f) {
            Some(words) => {
                l.set = words;
                l.set_kind = f.kind as u32;
                l.have_set = 1;
                l.sets = l.sets.wrapping_add(1);
                status(seq, f.seq, code::SET_COMPLETE)
            }
            None => status(seq, f.seq, code::RECEIVED),
        };
    }
    if f.kind == kind::READ_PARAMETERS {
        if l.have_set != 1 {
            return status(seq, f.seq, code::NO_SET);
        }
        l.read_next = 1;
        return match set_part(seq, kind::PARAM_PART, &l.set, 0) {
            Some(a) => a,
            None => status(seq, f.seq, code::NO_SET),
        };
    }
    status(seq, f.seq, code::NOT_SERVED)
}
