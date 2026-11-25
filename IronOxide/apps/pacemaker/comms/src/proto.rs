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
//! | PROGRAM_PART, TEMPORARY_PART | STATUS with RECEIVED, SET_COMPLETE or BAD_PART; a complete set is forwarded to `therapy` |
//! | END_TEMPORARY, PACE_NOW, SET_BATTERY, SET_MAGNET_RESPONSE | STATUS with FORWARDED; the command is forwarded to `therapy` (the two with a byte: BAD_PART unless the length is 1) |
//! | READ_PARAMETERS | part 0 of the last complete set as PARAM_PART, the other parts in the periods after; STATUS with NO_SET if there is none |
//! | any other kind of the DCM's | STATUS with NOT_SERVED |
//! | a kind of the PG's | as bytes that are no frame |
//!
//! **Toward `therapy`** (task 16.1). For every frame of the DCM's it hears,
//! `link` has one message for the Endpoint `program` ([`take_forward`]):
//! the command, or the mark that a frame was heard, by which `control`
//! knows the link is up (PM20.2). `link` does not judge a parameter set:
//! `therapy`'s validation function does (PM19.3), and a set it refuses is
//! this Partition's fault by the Kernel's rule (R42.4).
//!
//! **From `therapy`.** `control`'s status ([`status_in`]) fills the State,
//! the Mode and the battery level of every STATUS frame. When it differs
//! from the one last told, the next frame that would be IDLE is a STATUS
//! instead: with `control`'s verdict and the number of the frame it
//! answers when the verdict is new, with the code NONE otherwise.
//!
//! READ_PARAMETERS still reads back the last set `link` put together,
//! whatever `therapy` said of it: it is not the set in force.

use pm_core::frame::{kind, set_part, words_in_part, Assembly, Frame, PARTS};
use pm_core::wire::{self, Status, PROGRAM_WORDS, STATUS_WORDS};
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
    /// A part of a wrong length, or of another kind than the set begun; a
    /// command with a value whose length is not the value's.
    pub const BAD_PART: u8 = 5;
    /// `control`'s verdicts, which come with its status: 6 accepted, 7
    /// refused by the validation function, 8 declined by the State.
    pub use pm_core::wire::verdict::{ACCEPTED, DECLINED, REFUSED};
    /// The command was handed to `therapy`; its verdict follows.
    pub const FORWARDED: u8 = 9;
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
    /// 1 when `forward` is a message for the Endpoint `program` not yet
    /// taken; messages `therapy`'s Endpoint did not take.
    pub forward_ready: u32,
    pub unforwarded: u32,
    pub forward: [u32; PROGRAM_WORDS],
    /// 1 when `status` is a status of `control`'s; the one last told to
    /// the DCM (its first three words).
    pub have_status: u32,
    pub status: [u32; STATUS_WORDS],
    pub told: [u32; 3],
}

impl Link {
    #[must_use]
    pub const fn new() -> Link {
        Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; WORDS], forward_ready: 0, unforwarded: 0, forward: [0; PROGRAM_WORDS], have_status: 0, status: [0; STATUS_WORDS], told: [0; 3] }
    }
}

impl Default for Link {
    fn default() -> Link {
        Link::new()
    }
}

/// A status of `control`'s, taken from the Endpoint `status`.
pub fn status_in(l: &mut Link, w: &[u32; STATUS_WORDS]) {
    l.status = *w;
    l.have_status = 1;
}

/// The Endpoints report that `therapy` was stopped or restarted: its last
/// status is of an incarnation that is gone.
pub fn peer_changed(l: &mut Link) {
    l.have_status = 0;
    l.told = [0; 3];
}

/// The message for the Endpoint `program`, once.
#[must_use]
pub fn take_forward(l: &mut Link) -> Option<[u32; PROGRAM_WORDS]> {
    if l.forward_ready == 1 {
        l.forward_ready = 0;
        Some(l.forward)
    } else {
        None
    }
}

/// The Endpoint did not take the message [`take_forward`] gave.
pub fn not_forwarded(l: &mut Link) {
    l.unforwarded = l.unforwarded.saturating_add(1);
}

/// A STATUS frame answering the frame `of` with `code`: bytes 2 to 7 are
/// `control`'s State, Mode, battery level, whether it has the link as up,
/// and the low bytes of its counts of verdicts and of refused messages;
/// the first three are `UNKNOWN` while `link` has no status of `control`'s.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result.kind == kind::STATUS))]
fn status(l: &Link, seq: u8, of: u8, code: u8) -> Frame {
    let mut f = Frame::empty(seq, kind::STATUS);
    f.len = STATUS_LEN;
    f.payload[0] = of;
    f.payload[1] = code;
    if l.have_status == 1 {
        let s = Status::of_words(&l.status);
        f.payload[2] = s.therapy;
        f.payload[3] = s.mode;
        f.payload[4] = s.battery;
        f.payload[5] = if s.link_up { 1 } else { 0 };
        f.payload[6] = (s.serial & 0xFF) as u8;
        f.payload[7] = (s.refused & 0xFF) as u8;
    } else {
        f.payload[2] = UNKNOWN;
        f.payload[3] = UNKNOWN;
        f.payload[4] = UNKNOWN;
    }
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
    // A status of `control`'s that the DCM has not been told.
    if l.have_status == 1 && (l.status[0] != l.told[0] || l.status[1] != l.told[1] || l.status[2] != l.told[2]) {
        let s = Status::of_words(&l.status);
        let new_verdict = l.status[1] != l.told[1];
        l.told = [l.status[0], l.status[1], l.status[2]];
        return if new_verdict { status(l, seq, s.of, s.verdict) } else { status(l, seq, 0, code::NONE) };
    }
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
    // One message to `control` for the frame: that it was heard, unless
    // the frame turns out below to be a command.
    l.forward = wire::message(wire::code::HEARD, f.seq);
    l.forward_ready = 1;
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
            return status(l, seq, f.seq, code::BAD_PART);
        }
        l.assembly_kind = f.kind as u32;
        if f.part as usize >= PARTS || f.len as usize != 4 * words_in_part(f.part as usize) {
            l.assembly.have = 0;
            return status(l, seq, f.seq, code::BAD_PART);
        }
        return match l.assembly.accept(f) {
            Some(words) => {
                l.set = words;
                l.set_kind = f.kind as u32;
                l.have_set = 1;
                l.sets = l.sets.wrapping_add(1);
                // The frame's kind is the message's code (0x10, 0x11).
                l.forward = wire::message_with_set(f.kind, f.seq, &words);
                status(l, seq, f.seq, code::SET_COMPLETE)
            }
            None => status(l, seq, f.seq, code::RECEIVED),
        };
    }
    if f.kind == kind::READ_PARAMETERS {
        if l.have_set != 1 {
            return status(l, seq, f.seq, code::NO_SET);
        }
        l.read_next = 1;
        return match set_part(seq, kind::PARAM_PART, &l.set, 0) {
            Some(a) => a,
            None => status(l, seq, f.seq, code::NO_SET),
        };
    }
    if f.kind == kind::END_TEMPORARY || f.kind == kind::PACE_NOW {
        l.forward = wire::message(f.kind, f.seq);
        return status(l, seq, f.seq, code::FORWARDED);
    }
    // The battery level and the magnet response: one byte, which `control`
    // judges (the frame's kind is the message's code).
    if f.kind == kind::SET_BATTERY || f.kind == kind::SET_MAGNET_RESPONSE {
        if f.len != 1 {
            return status(l, seq, f.seq, code::BAD_PART);
        }
        l.forward = wire::message_with_value(f.kind, f.seq, f.payload[0] as u32);
        return status(l, seq, f.seq, code::FORWARDED);
    }
    status(l, seq, f.seq, code::NOT_SERVED)
}
