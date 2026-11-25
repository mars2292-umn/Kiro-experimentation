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
//! | END_TEMPORARY, PACE_NOW, SET_BATTERY, SET_ACTIVITY, SET_MAGNET_RESPONSE | STATUS with FORWARDED; the command is forwarded to `therapy` (the two with a byte: BAD_PART unless the length is 1) |
//! | READ_PARAMETERS | part 0 of the last complete set as PARAM_PART, the other parts in the periods after; STATUS with NO_SET if there is none |
//! | RESET_HISTOGRAMS, TREND_START, MARKERS_START, MARKERS_STOP, SET_DATA | STATUS with REQUESTED; the request is handed to `diag` (BAD_PART for a wrong length) |
//! | READ_RECORD, MEASURE | STATUS with REQUESTED (BAD_PART for a kind or a part no record has); the RECORD follows when `diag` has answered. At a battery level of ERT or below the measured data, a Marker and MARKERS_START are answered DECLINED, and a Marker transmission is ended (PM17.7) |
//! | THRESHOLD_START, THRESHOLD_STOP | STATUS with FORWARDED; the command is forwarded to `therapy` |
//! | any other kind of the DCM's (EGRAM_START, EGRAM_STOP: finding B46) | STATUS with NOT_SERVED |
//! | a kind of the PG's | as bytes that are no frame |
//!
//! **Toward `therapy`** (task 16.1). For every frame of the DCM's it hears,
//! `link` has one message for the Endpoint `program` ([`take_forward`]):
//! the command, or the mark that a frame was heard, by which `control`
//! knows the link is up (PM20.2). `therapy`'s validation function is the
//! judge of a command (PM19.3), and a message it refuses is this
//! Partition's fault by the Kernel's rule (R42.4). So `link` asks the same
//! function first (`pm_core::wire::admitted`) and answers a command it
//! would refuse with STATUS REFUSED, naming the frame, without forwarding
//! it: a DCM's mistake is then no fault of anyone's (decided 2026-11-24).
//! The battery level is kept and sent again when `therapy` has restarted.
//!
//! **From `therapy`.** `control`'s status ([`status_in`]) fills the State,
//! the Mode and the battery level of every STATUS frame. When it differs
//! from the one last told, the next frame that would be IDLE is a STATUS
//! instead: with `control`'s verdict and the number of the frame it
//! answers when the verdict is new, with the code NONE otherwise.
//!
//! **To and from `diag`** (task 22.4). A request is one message for the
//! Endpoint `request` ([`take_request`]). A record `diag` sent
//! ([`record_in`]; the Kernel has put it through [`pm_core::record::admitted`],
//! this Partition's validation function, before `link` sees it) is told
//! as a RECORD frame in the next period that has no part of a read to
//! send: its part in the frame's, its six words as the payload.
//!
//! READ_PARAMETERS still reads back the last set `link` put together,
//! whatever `therapy` said of it: it is not the set in force.

use pm_core::frame::{kind, set_part, words_in_part, Assembly, Frame, PARTS};
use pm_core::record::{self, DATA_WORDS, REQUEST_WORDS};
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
    /// refused by the validation function, 8 declined by the State. `link`
    /// answers 7 itself, naming the frame, for a command the validation
    /// function would refuse.
    pub use pm_core::wire::verdict::{ACCEPTED, DECLINED, REFUSED};
    /// The command was handed to `therapy`; its verdict follows.
    pub const FORWARDED: u8 = 9;
    /// The request was handed to `diag`; a record follows if it asked for one.
    pub const REQUESTED: u8 = 10;
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
    /// The battery level the DCM last set (0, the value at start, is BOL),
    /// and 1 when it is to be sent to `therapy` again.
    pub battery: u32,
    pub resend_battery: u32,
    /// 1 when `request` is a request for `diag` not yet taken; requests
    /// the Endpoint did not take.
    pub request_ready: u32,
    pub unrequested: u32,
    pub request: [u32; REQUEST_WORDS],
    /// 1 when `record` is a record of `diag`'s not yet told; records told.
    pub record_ready: u32,
    pub records: u32,
    pub record: [u32; DATA_WORDS],
    /// Records of `diag`'s not taken because the battery level disables
    /// them (PM17.7; saturating).
    pub withheld: u32,
}

impl Link {
    #[must_use]
    pub const fn new() -> Link {
        Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; WORDS], forward_ready: 0, unforwarded: 0, forward: [0; PROGRAM_WORDS], have_status: 0, status: [0; STATUS_WORDS], told: [0; 3], battery: 0, resend_battery: 0, request_ready: 0, unrequested: 0, request: [0; REQUEST_WORDS], record_ready: 0, records: 0, record: [0; DATA_WORDS], withheld: 0 }
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
    // `therapy` has the level as BOL again: a lower one is sent anew.
    if l.battery != 0 {
        l.resend_battery = 1;
    }
}

/// The battery level for a `therapy` that has started again, once: a
/// message as the DCM's SET_BATTERY gave, numbered 0. `control` answers it
/// like any command, so a verdict naming frame 0 may follow a restart.
#[must_use]
pub fn take_resend(l: &mut Link) -> Option<[u32; PROGRAM_WORDS]> {
    if l.resend_battery == 1 {
        l.resend_battery = 0;
        Some(wire::message_with_value(wire::code::BATTERY, 0, l.battery))
    } else {
        None
    }
}

/// The Endpoint did not take the message [`take_resend`] gave: it is
/// offered again in the next period.
pub fn not_resent(l: &mut Link) {
    l.resend_battery = 1;
}

/// The request for the Endpoint `request`, once.
#[must_use]
pub fn take_request(l: &mut Link) -> Option<[u32; REQUEST_WORDS]> {
    if l.request_ready == 1 {
        l.request_ready = 0;
        Some(l.request)
    } else {
        None
    }
}

/// The Endpoint did not take the request [`take_request`] gave.
pub fn not_requested(l: &mut Link) {
    l.unrequested = l.unrequested.saturating_add(1);
}

/// Whether a record can be taken from the Endpoint `records`: the one
/// before has been told.
#[must_use]
pub fn wants_record(l: &Link) -> bool {
    l.record_ready == 0
}

/// A record of `diag`'s, taken from the Endpoint `records`. At a battery
/// level of ERT or below a Marker and the measured data are not taken
/// (PM17.7): one asked for before the level fell, or one of a Marker
/// transmission `diag` has not yet been told to end.
pub fn record_in(l: &mut Link, m: &[u32; DATA_WORDS]) {
    if level(l) >= ERT && disabled_at_ert((m[0] % 0x100) as u8) {
        l.withheld = l.withheld.saturating_add(1);
        return;
    }
    l.record = *m;
    l.record_ready = 1;
}

/// The battery level (as SET_BATTERY's byte) from which the measured data
/// and the Markers are disabled (PM17.7): ERT; 3 is ERP, 4 below the trip
/// level.
pub const ERT: u32 = 2;

/// The battery level the measured data and the Markers go by: the worse
/// of the one the DCM last set and the one in `control`'s last status. The
/// first is 0 after a restart of `comms`, the second after a restart of
/// `therapy` until the level has been sent again.
#[must_use]
pub fn level(l: &Link) -> u32 {
    let told = if l.have_status == 1 { (l.status[0] / 0x1_0000) & 0xFF } else { 0 };
    if told > l.battery { told } else { l.battery }
}

/// Whether a record of `kind` is one a battery level of ERT or below
/// disables: the measured data and a Marker (PM17.7). The histograms, the
/// counts, the trend and the implant data are neither.
#[must_use]
pub const fn disabled_at_ert(kind: u8) -> bool {
    kind == record::data::MEASURED || kind == record::data::MARKER
}

/// Whether `r` asks `diag` for what a battery level of ERT or below
/// disables: a read of the measured data or of a Marker, or the start of
/// the Marker transmission. Its end is not.
fn asks_disabled(r: &[u32; REQUEST_WORDS]) -> bool {
    (r[0] == record::request::READ && (r[1] == record::data::MEASURED as u32 || r[1] == record::data::MARKER as u32))
        || (r[0] == record::request::MARKERS && r[1] != 0)
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
    // A record of `diag`'s.
    if l.record_ready == 1 {
        l.record_ready = 0;
        l.records = l.records.wrapping_add(1);
        let mut f = Frame::empty(seq, kind::RECORD);
        f.part = ((l.record[0] / 0x100) & 0xFF) as u8;
        f.len = 24;
        put_word(&mut f.payload, 0, l.record[0]);
        put_word(&mut f.payload, 4, l.record[1]);
        put_word(&mut f.payload, 8, l.record[2]);
        put_word(&mut f.payload, 12, l.record[3]);
        put_word(&mut f.payload, 16, l.record[4]);
        put_word(&mut f.payload, 20, l.record[5]);
        return f;
    }
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
                // The frame's kind is the message's code (0x10, 0x11). A
                // set `therapy` would refuse is refused here and goes no
                // further (decided 2026-11-24).
                let m = wire::message_with_set(f.kind, f.seq, &words);
                if !wire::admitted(&m) {
                    return status(l, seq, f.seq, code::REFUSED);
                }
                l.set = words;
                l.set_kind = f.kind as u32;
                l.have_set = 1;
                l.sets = l.sets.wrapping_add(1);
                l.forward = m;
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
    // The battery level, the activity value (task 19.5) and the magnet
    // response: one byte (the frame's
    // kind is the message's code). A value `therapy` would refuse is
    // refused here. The level is kept, to be sent again if `therapy`
    // starts again (decided 2026-11-24).
    if f.kind == kind::SET_BATTERY || f.kind == kind::SET_ACTIVITY || f.kind == kind::SET_MAGNET_RESPONSE {
        if f.len != 1 {
            return status(l, seq, f.seq, code::BAD_PART);
        }
        let m = wire::message_with_value(f.kind, f.seq, f.payload[0] as u32);
        if !wire::admitted(&m) {
            return status(l, seq, f.seq, code::REFUSED);
        }
        if f.kind == kind::SET_BATTERY {
            l.battery = f.payload[0] as u32;
            l.resend_battery = 0;
            // At ERT and below a Marker transmission ends (PM17.7), and a
            // record not yet told that the level disables is dropped.
            if l.battery >= ERT {
                l.request = [record::request::MARKERS, 0, 0];
                l.request_ready = 1;
                if l.record_ready == 1 && disabled_at_ert((l.record[0] % 0x100) as u8) {
                    l.record_ready = 0;
                    l.withheld = l.withheld.saturating_add(1);
                }
            }
        }
        l.forward = m;
        return status(l, seq, f.seq, code::FORWARDED);
    }
    // The pacing threshold test (task 22.6): to `therapy`, as the other
    // commands with a byte and with none.
    if f.kind == kind::THRESHOLD_START || f.kind == kind::THRESHOLD_STOP {
        let with_value = f.kind == kind::THRESHOLD_START;
        if f.len != if with_value { 1 } else { 0 } {
            return status(l, seq, f.seq, code::BAD_PART);
        }
        let m = if with_value { wire::message_with_value(f.kind, f.seq, f.payload[0] as u32) } else { wire::message(f.kind, f.seq) };
        if !wire::admitted(&m) {
            return status(l, seq, f.seq, code::REFUSED);
        }
        l.forward = m;
        return status(l, seq, f.seq, code::FORWARDED);
    }
    // The requests to `diag` (task 22.4).
    match request_of(f) {
        // The measured data and the Markers are disabled at ERT and below
        // (PM17.7): declined, as `control` declines temporary pacing there.
        Requested::Is(r) if level(l) >= ERT && asks_disabled(&r) => status(l, seq, f.seq, code::DECLINED),
        Requested::Is(r) => {
            l.request = r;
            l.request_ready = 1;
            status(l, seq, f.seq, code::REQUESTED)
        }
        Requested::Bad => status(l, seq, f.seq, code::BAD_PART),
        Requested::No => status(l, seq, f.seq, code::NOT_SERVED),
    }
}

/// What a frame asks of `diag`.
enum Requested {
    Is([u32; REQUEST_WORDS]),
    /// A request with a wrong length, or for a record that is none.
    Bad,
    /// No request.
    No,
}

/// The request a frame of the DCM's is.
fn request_of(f: &Frame) -> Requested {
    use record::request;
    if f.kind == kind::RESET_HISTOGRAMS {
        return if f.len == 0 { Requested::Is([request::RESET_HISTOGRAMS, 0, 0]) } else { Requested::Bad };
    }
    if f.kind == kind::MARKERS_START || f.kind == kind::MARKERS_STOP {
        return if f.len == 0 { Requested::Is([request::MARKERS, if f.kind == kind::MARKERS_START { 1 } else { 0 }, 0]) } else { Requested::Bad };
    }
    if f.kind == kind::TREND_START {
        return if f.len == 2 { Requested::Is([request::TREND, f.payload[0] as u32, f.payload[1] as u32]) } else { Requested::Bad };
    }
    if f.kind == kind::MEASURE {
        return if f.len == 0 { Requested::Is([request::READ, record::data::MEASURED as u32, 0]) } else { Requested::Bad };
    }
    if f.kind == kind::READ_RECORD {
        return if f.len == 2 && (f.payload[1] as u32) < record::parts(f.payload[0]) { Requested::Is([request::READ, f.payload[0] as u32, f.payload[1] as u32]) } else { Requested::Bad };
    }
    if f.kind == kind::SET_DATA {
        if f.len != 6 || f.payload[0] > 1 {
            return Requested::Bad;
        }
        let limit = if f.payload[0] == 0 { record::IMPLANT_WORDS } else { record::MEASURED_WORDS };
        if f.payload[1] as usize >= limit {
            return Requested::Bad;
        }
        let value = f.payload[2] as u32 + f.payload[3] as u32 * 0x100 + f.payload[4] as u32 * 0x1_0000 + f.payload[5] as u32 * 0x100_0000;
        return Requested::Is([if f.payload[0] == 0 { request::SET_IMPLANT } else { request::SET_MEASURED }, f.payload[1] as u32, value]);
    }
    Requested::No
}
