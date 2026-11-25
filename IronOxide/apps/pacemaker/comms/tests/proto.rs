//! `proto::answer`, the link's answers (task 15.3), case by case. The
//! DCM_Stub's tests (`tools/dcm-stub/tests/session.rs`) run the same
//! function through a session.
#![forbid(unsafe_code)]

use pacemaker_comms::proto::{answer, code, not_resent, peer_changed, record_in, status_in, take_forward, take_request, take_resend, wants_record, Link, IDENTITY_LEN, MODEL, SERIAL, STATUS_LEN, UNKNOWN};
use pm_core::wire::{self, Status};
use pm_core::frame::{decode, encode, kind, set_part, Assembly, Frame};
use pm_core::NOMINAL_WORDS;

fn status_of(f: &Frame) -> (u8, u8) {
    assert_eq!((f.kind, f.len), (kind::STATUS, STATUS_LEN));
    assert_eq!(&f.payload[2..5], &[UNKNOWN; 3]);
    (f.payload[0], f.payload[1])
}

#[test]
fn the_zero_state_is_the_new_state() {
    assert_eq!(
        Link::new(),
        Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; 29],
               forward_ready: 0, unforwarded: 0, forward: [0; 30], have_status: 0, status: [0; 4], told: [0; 3], battery: 0, resend_battery: 0,
               request_ready: 0, unrequested: 0, request: [0; 3], record_ready: 0, records: 0, record: [0; 6], withheld: 0 }
    );
    // Task 16.1 added the words of the two Endpoints (2 + 29 + 1 + 4 + 3);
    // the battery level kept for a restarted `therapy` two more (2026-11-24);
    // a set has 29 words since task 19.5 (28 before: three words more here).
    // Task 22.4 added the request and the record with their marks (14 words).
    assert_eq!(core::mem::size_of::<Link>(), 9 * 4 + 4 + 29 * 4 + 29 * 4 + 42 * 4 + 14 * 4, "no padding");
}

#[test]
fn every_answer_is_a_frame_and_numbered_in_order() {
    let mut l = Link::new();
    let mut frames = vec![None, Some(Frame::empty(9, kind::POLL)), Some(Frame::empty(10, kind::INTERROGATE)), Some(Frame::empty(11, kind::PACE_NOW))];
    frames.extend((0..5).map(|p| set_part(20 + p, kind::PROGRAM_PART, &NOMINAL_WORDS, p)));
    frames.push(Some(Frame::empty(30, kind::READ_PARAMETERS)));
    frames.extend([None; 6]);
    for (n, f) in frames.iter().enumerate() {
        let a = answer(&mut l, f.as_ref());
        assert_eq!(a.seq as usize, n);
        assert_eq!(decode(&encode(&a)), Some(a), "answer {n}");
        assert!(a.kind >= 0x80);
    }
}

#[test]
fn silence_is_counted_and_a_frame_ends_it() {
    let mut l = Link::new();
    for n in 1..=4 {
        assert_eq!(answer(&mut l, None).kind, kind::IDLE);
        assert_eq!((l.silent, l.unheard, l.heard), (n, n, 0));
    }
    // A frame of the pulse generator's own kinds is not the DCM's.
    assert_eq!(answer(&mut l, Some(&Frame::empty(0, kind::IDLE))).kind, kind::IDLE);
    assert_eq!((l.silent, l.heard), (5, 0));
    assert_eq!(answer(&mut l, Some(&Frame::empty(0, kind::POLL))).kind, kind::IDLE);
    assert_eq!((l.silent, l.heard, l.unheard), (0, 1, 5));
}

#[test]
fn the_identity_names_the_frame_it_answers() {
    let mut l = Link::new();
    let a = answer(&mut l, Some(&Frame::empty(0x5C, kind::INTERROGATE)));
    assert_eq!((a.kind, a.len, a.part), (kind::IDENTITY, IDENTITY_LEN, 0));
    assert_eq!(a.payload[..4], [0x5C, 0, 0, 0]);
    assert_eq!(a.payload[4..8], MODEL.to_le_bytes());
    assert_eq!(a.payload[8..12], SERIAL.to_le_bytes());
    assert_eq!(a.payload[12..], [0; 12]);
}

#[test]
fn parts_complete_a_set_and_a_bad_part_drops_what_had_come() {
    let mut l = Link::new();
    let part = |seq: u8, k: u8, p: u8| set_part(seq, k, &NOMINAL_WORDS, p).unwrap();
    for p in 0..4 {
        assert_eq!(status_of(&answer(&mut l, Some(&part(p, kind::PROGRAM_PART, p)))), (p, code::RECEIVED));
    }
    // A part of another kind than the set begun.
    assert_eq!(status_of(&answer(&mut l, Some(&part(4, kind::TEMPORARY_PART, 4)))), (4, code::BAD_PART));
    assert_eq!((l.assembly.have, l.have_set), (0, 0));
    // The last part alone completes nothing.
    assert_eq!(status_of(&answer(&mut l, Some(&part(5, kind::PROGRAM_PART, 4)))), (5, code::RECEIVED));
    // A part of a wrong length.
    let mut short = part(6, kind::PROGRAM_PART, 1);
    short.len = 20;
    assert_eq!(status_of(&answer(&mut l, Some(&short))), (6, code::BAD_PART));
    assert_eq!(l.assembly.have, 0);
    // In any order after part 0, with a poll between.
    for (n, p) in [0u8, 3, 1, 4].into_iter().enumerate() {
        assert_eq!(status_of(&answer(&mut l, Some(&part(10 + n as u8, kind::TEMPORARY_PART, p)))).1, code::RECEIVED);
        let _ = answer(&mut l, Some(&Frame::empty(0, kind::POLL)));
    }
    assert_eq!(status_of(&answer(&mut l, Some(&part(20, kind::TEMPORARY_PART, 2)))), (20, code::SET_COMPLETE));
    assert_eq!((l.have_set, l.set_kind, l.sets, l.set), (1, kind::TEMPORARY_PART as u32, 1, NOMINAL_WORDS));
}

#[test]
fn a_read_sends_the_five_parts_in_order_and_a_command_ends_it() {
    let mut l = Link::new();
    assert_eq!(status_of(&answer(&mut l, Some(&Frame::empty(1, kind::READ_PARAMETERS)))), (1, code::NO_SET));
    let mut words = NOMINAL_WORDS;
    words[1] = 90;
    for p in 0..5 {
        let _ = answer(&mut l, set_part(p, kind::PROGRAM_PART, &words, p).as_ref());
    }
    let read = Frame::empty(7, kind::READ_PARAMETERS);
    let mut back = Assembly::EMPTY;
    let mut got = None;
    let first = answer(&mut l, Some(&read));
    assert_eq!((first.kind, first.part), (kind::PARAM_PART, 0));
    let _ = back.accept(&first);
    // The other parts come whether the DCM polls or is not heard.
    for p in 1..5u8 {
        let poll = Frame::empty(0, kind::POLL);
        let a = answer(&mut l, if p % 2 == 0 { None } else { Some(&poll) });
        assert_eq!((a.kind, a.part), (kind::PARAM_PART, p));
        got = back.accept(&a);
    }
    assert_eq!(got, Some(words));
    assert_eq!(answer(&mut l, None).kind, kind::IDLE);
    // A command between the parts ends the read.
    let _ = answer(&mut l, Some(&read));
    assert_eq!(answer(&mut l, Some(&Frame::empty(8, kind::INTERROGATE))).kind, kind::IDENTITY);
    assert_eq!(answer(&mut l, Some(&Frame::empty(9, kind::POLL))).kind, kind::IDLE);
}

#[test]
fn the_commands_of_later_tasks_are_not_served() {
    // END_TEMPORARY and PACE_NOW were in this list until task 16.1; `link` forwards them now (the test below).
    // (SET_BATTERY too until task 16.5, SET_ACTIVITY until task 19.5; RESET_HISTOGRAMS, MEASURE and the two of the
    // threshold test until tasks 22.4 and 22.6: the tests at the end of this file. The electrogram's two stay: finding B46.)
    for k in [kind::SET_CLOCK, kind::EGRAM_START, kind::EGRAM_STOP] {
        let mut l = Link::new();
        assert_eq!(status_of(&answer(&mut l, Some(&Frame::empty(3, k)))), (3, code::NOT_SERVED), "kind {k:#x}");
    }
}

#[test]
fn every_frame_heard_gives_control_one_message() {
    let mut l = Link::new();
    assert_eq!(take_forward(&mut l), None);
    let _ = answer(&mut l, None);
    assert_eq!(take_forward(&mut l), None, "nothing heard, nothing told");
    let _ = answer(&mut l, Some(&Frame::empty(0, kind::IDLE)));
    assert_eq!(take_forward(&mut l), None, "a frame of the pulse generator's kinds is not heard");
    for k in [kind::POLL, kind::INTERROGATE, kind::READ_PARAMETERS, kind::SET_CLOCK] {
        let _ = answer(&mut l, Some(&Frame::empty(0x21, k)));
        assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 0x21)), "kind {k:#x}");
        assert_eq!(take_forward(&mut l), None, "once");
    }
    for k in [kind::END_TEMPORARY, kind::PACE_NOW] {
        let a = answer(&mut l, Some(&Frame::empty(0x22, k)));
        assert_eq!(status_of(&a), (0x22, code::FORWARDED));
        assert_eq!(take_forward(&mut l), Some(wire::message(k, 0x22)));
    }
    // The parts of a set are heard; the part that completes it is the
    // command, if `therapy`'s validation function would admit it. (Until
    // 2026-11-24 `link` forwarded a set without judging it, and this test
    // expected an LRL of 31 ppm to go through.)
    let mut words = NOMINAL_WORDS;
    words[1] = 90;
    let mut bad = NOMINAL_WORDS;
    bad[1] = 31;
    for (k, code_) in [(kind::PROGRAM_PART, wire::code::PROGRAM), (kind::TEMPORARY_PART, wire::code::TEMPORARY)] {
        for p in 0..5u8 {
            let _ = answer(&mut l, set_part(0x30 + p, k, &words, p).as_ref());
            let m = take_forward(&mut l).expect("a message");
            if p < 4 {
                assert_eq!(m, wire::message(wire::code::HEARD, 0x30 + p));
            } else {
                assert_eq!(m, wire::message_with_set(code_, 0x34, &words));
                assert!(wire::admitted(&m));
            }
        }
        // A set the validation function refuses: refused by `link`, naming
        // the frame; `control` hears of the frame and not of the set, and
        // the set last accepted stays the one read back.
        let sets = l.sets;
        for p in 0..5u8 {
            let a = answer(&mut l, set_part(0x40 + p, k, &bad, p).as_ref());
            assert_eq!(status_of(&a), (0x40 + p, if p < 4 { code::RECEIVED } else { code::REFUSED }));
            assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 0x40 + p)));
        }
        assert_eq!((l.sets, l.set), (sets, words));
    }
    // A message not taken is replaced by the next frame's.
    let _ = answer(&mut l, Some(&Frame::empty(1, kind::PACE_NOW)));
    let _ = answer(&mut l, Some(&Frame::empty(2, kind::POLL)));
    assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 2)));
}

#[test]
fn a_new_status_is_told_once_in_place_of_idle() {
    let mut l = Link::new();
    let poll = Frame::empty(0, kind::POLL);
    let mut st = Status { therapy: 1, mode: 0x15, battery: 0, link_up: true, serial: 0, verdict: 0, of: 0, refused: 0 };
    status_in(&mut l, &st.words());
    // A change without a verdict: the code NONE.
    let a = answer(&mut l, Some(&poll));
    assert_eq!((a.kind, a.len, &a.payload[..8]), (kind::STATUS, STATUS_LEN, &[0, code::NONE, 1, 0x15, 0, 1, 0, 0][..]));
    assert_eq!(answer(&mut l, Some(&poll)).kind, kind::IDLE, "told once");
    // A verdict: its code and the frame it answers; also when the DCM is not heard.
    st.serial = 1;
    st.verdict = code::ACCEPTED;
    st.of = 0x44;
    st.therapy = 2;
    status_in(&mut l, &st.words());
    let a = answer(&mut l, None);
    assert_eq!(&a.payload[..8], &[0x44, code::ACCEPTED, 2, 0x15, 0, 1, 1, 0]);
    assert_eq!(answer(&mut l, None).kind, kind::IDLE);
    // The refused count alone is not a change to tell (it comes with its verdict).
    st.refused = 1;
    status_in(&mut l, &st.words());
    assert_eq!(answer(&mut l, Some(&poll)).kind, kind::IDLE);
    // `link`'s own answers carry `control`'s bytes.
    let a = answer(&mut l, Some(&Frame::empty(5, kind::SET_CLOCK)));
    assert_eq!(&a.payload[..8], &[5, code::NOT_SERVED, 2, 0x15, 0, 1, 1, 1]);
    // An answer to a command comes before the status; the status waits.
    st.therapy = 1;
    status_in(&mut l, &st.words());
    assert_eq!(answer(&mut l, Some(&Frame::empty(6, kind::INTERROGATE))).kind, kind::IDENTITY);
    let a = answer(&mut l, Some(&poll));
    assert_eq!(&a.payload[..3], &[0, code::NONE, 1]);
    // After `therapy` restarted its status is unknown until it sends one.
    peer_changed(&mut l);
    let a = answer(&mut l, Some(&Frame::empty(7, kind::SET_CLOCK)));
    assert_eq!(&a.payload[2..5], &[UNKNOWN; 3]);
    status_in(&mut l, &Status { serial: 0, verdict: 0, of: 0, refused: 0, ..st }.words());
    assert_eq!(answer(&mut l, Some(&poll)).kind, kind::STATUS, "told again");
}

#[test]
fn a_battery_level_and_a_magnet_response_are_forwarded_with_their_byte() {
    // An activity value takes the whole byte: none is refused (task 19.5).
    let mut l = Link::new();
    let mut f = Frame::empty(0x52, kind::SET_ACTIVITY);
    f.len = 1;
    for v in [0u8, 1, 128, 255] {
        f.payload[0] = v;
        assert_eq!(status_of(&answer(&mut l, Some(&f))), (0x52, code::FORWARDED));
        assert_eq!(take_forward(&mut l), Some(wire::message_with_value(wire::code::ACTIVITY, 0x52, v as u32)));
    }
    for (k, top) in [(kind::SET_BATTERY, 4u8), (kind::SET_MAGNET_RESPONSE, 1)] {
        let mut l = Link::new();
        let mut f = Frame::empty(0x51, k);
        f.len = 1;
        for v in 0..=top {
            f.payload[0] = v;
            assert_eq!(status_of(&answer(&mut l, Some(&f))), (0x51, code::FORWARDED));
            assert_eq!(take_forward(&mut l), Some(wire::message_with_value(k, 0x51, v as u32)));
        }
        // A value the validation function refuses is refused here; the frame was heard.
        for v in [top + 1, 0xFF] {
            f.payload[0] = v;
            assert_eq!(status_of(&answer(&mut l, Some(&f))), (0x51, code::REFUSED), "kind {k:#x} value {v}");
            assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 0x51)));
        }
        // Without its byte, or with more, it is no such command.
        f.payload[0] = 1;
        for len in [0, 2] {
            f.len = len;
            f.payload[1] = if len == 2 { 1 } else { 0 };
            assert_eq!(status_of(&answer(&mut l, Some(&f))), (0x51, code::BAD_PART));
            assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 0x51)));
        }
    }
}

/// The battery level is `therapy`'s to lose at a restart and `link`'s to
/// give again (decided 2026-11-24): once, when the Endpoints report the
/// restart, and again if the Endpoint did not take it.
#[test]
fn the_battery_level_is_sent_again_to_a_restarted_therapy() {
    let mut l = Link::new();
    peer_changed(&mut l);
    assert_eq!(take_resend(&mut l), None, "BOL is what a restarted therapy has");
    let mut f = Frame::empty(7, kind::SET_BATTERY);
    f.len = 1;
    f.payload[0] = 2;
    let _ = answer(&mut l, Some(&f));
    let _ = take_forward(&mut l);
    assert_eq!(take_resend(&mut l), None, "nothing to send again while therapy runs");
    peer_changed(&mut l);
    let m = take_resend(&mut l).expect("the level");
    assert_eq!(m, wire::message_with_value(wire::code::BATTERY, 0, 2));
    assert!(wire::admitted(&m));
    assert_eq!(take_resend(&mut l), None, "once");
    not_resent(&mut l);
    assert_eq!(take_resend(&mut l), Some(m), "again when the Endpoint did not take it");
    // A level the DCM sets after the restart is the one that counts.
    peer_changed(&mut l);
    f.payload[0] = 1;
    let _ = answer(&mut l, Some(&f));
    assert_eq!(take_resend(&mut l), None);
    // Back at BOL there is nothing to give again.
    f.payload[0] = 0;
    let _ = answer(&mut l, Some(&f));
    peer_changed(&mut l);
    assert_eq!(take_resend(&mut l), None);
}

// ------------------------------------------------------------ tasks 22.4, 22.6

fn framed(seq: u8, k: u8, bytes: &[u8]) -> Frame {
    let mut f = Frame::empty(seq, k);
    f.len = bytes.len() as u8;
    f.payload[..bytes.len()].copy_from_slice(bytes);
    f
}

/// Task 22.4: each record command is one request for `diag`, answered
/// REQUESTED; a wrong length, a record that is none, and an index outside
/// the data are BAD_PART and no request.
#[test]
fn the_record_commands_become_requests_for_diag() {
    use pm_core::record::{data, request};
    let cases: [(Frame, [u32; 3]); 9] = [
        (framed(1, kind::RESET_HISTOGRAMS, &[]), [request::RESET_HISTOGRAMS, 0, 0]),
        (framed(2, kind::MARKERS_START, &[]), [request::MARKERS, 1, 0]),
        (framed(3, kind::MARKERS_STOP, &[]), [request::MARKERS, 0, 0]),
        (framed(4, kind::TREND_START, &[1, 2]), [request::TREND, 1, 2]),
        (framed(5, kind::MEASURE, &[]), [request::READ, data::MEASURED as u32, 0]),
        (framed(6, kind::READ_RECORD, &[data::HISTOGRAM + 3, 3]), [request::READ, 0x13, 3]),
        (framed(7, kind::READ_RECORD, &[data::TREND, 64]), [request::READ, 0x30, 64]),
        (framed(8, kind::SET_DATA, &[0, 9, 0x78, 0x56, 0x34, 0x12]), [request::SET_IMPLANT, 9, 0x1234_5678]),
        (framed(9, kind::SET_DATA, &[1, 4, 1, 0, 0, 0]), [request::SET_MEASURED, 4, 1]),
    ];
    for (f, want) in cases {
        let mut l = Link::new();
        assert_eq!(take_request(&mut l), None);
        assert_eq!(status_of(&answer(&mut l, Some(&f))), (f.seq, code::REQUESTED), "kind {:#x}", f.kind);
        assert_eq!(take_request(&mut l), Some(want), "kind {:#x}", f.kind);
        assert_eq!(take_request(&mut l), None, "once");
        // `control` is told the frame was heard, as of any frame.
        assert_eq!(take_forward(&mut l).map(|m| wire::code_of(&m)), Some(wire::code::HEARD));
    }
    for f in [
        framed(1, kind::RESET_HISTOGRAMS, &[0]),
        framed(2, kind::MARKERS_START, &[1]),
        framed(3, kind::TREND_START, &[1]),
        framed(4, kind::MEASURE, &[0]),
        framed(5, kind::READ_RECORD, &[data::HISTOGRAM, 4]),
        framed(6, kind::READ_RECORD, &[data::MARKER, 1]),
        framed(7, kind::READ_RECORD, &[0x99, 0]),
        framed(8, kind::READ_RECORD, &[data::COUNTS]),
        framed(9, kind::SET_DATA, &[2, 0, 0, 0, 0, 0]),
        framed(10, kind::SET_DATA, &[0, 10, 0, 0, 0, 0]),
        framed(11, kind::SET_DATA, &[1, 5, 0, 0, 0, 0]),
        framed(12, kind::SET_DATA, &[0, 0, 0]),
    ] {
        let mut l = Link::new();
        assert_eq!(status_of(&answer(&mut l, Some(&f))), (f.seq, code::BAD_PART), "kind {:#x}", f.kind);
        assert_eq!(take_request(&mut l), None);
    }
}

/// Task 22.4: a record of `diag`'s is told as a RECORD frame, once, in a
/// period that has nothing of a read to send, and before a status.
#[test]
fn a_record_is_told_as_a_frame() {
    use pm_core::record::{data, head};
    let poll = Frame::empty(0, kind::POLL);
    let mut l = Link::new();
    assert!(wants_record(&l));
    let m = [head(data::HISTOGRAM + 2, 3), 1, 2, 0x0403_0201, 0, 0xFFFF_FFFF];
    record_in(&mut l, &m);
    assert!(!wants_record(&l), "one at a time");
    let a = answer(&mut l, Some(&poll));
    assert_eq!((a.kind, a.part, a.len), (kind::RECORD, 3, 24));
    assert_eq!(&a.payload[..4], &[0x12, 3, 0, 0]);
    assert_eq!(&a.payload[12..16], &[1, 2, 3, 4], "least significant byte first");
    assert_eq!(&a.payload[20..24], &[0xFF; 4]);
    assert!(wants_record(&l));
    assert_eq!(l.records, 1);
    assert_eq!(answer(&mut l, Some(&poll)).kind, kind::IDLE, "told once");
    // With no frame heard as well (the DCM's poll was lost): the record is not kept back.
    record_in(&mut l, &m);
    assert_eq!(answer(&mut l, None).kind, kind::RECORD);
}

/// The validation function of the Endpoint `records` is `pm_core`'s.
#[test]
fn the_validation_of_a_record() {
    use pm_core::record::{data, head};
    assert!(pacemaker_comms::validate_records(&[head(data::COUNTS, 1), 9, 9, 9, 9, 9]));
    assert!(!pacemaker_comms::validate_records(&[0x99, 0, 0, 0, 0, 0]));
    assert!(!pacemaker_comms::validate_records(&[head(data::COUNTS, 2), 0, 0, 0, 0, 0]));
}

/// Task 22.6: the threshold test's two commands go to `therapy`.
#[test]
fn the_threshold_test_s_commands_are_forwarded() {
    let mut l = Link::new();
    assert_eq!(status_of(&answer(&mut l, Some(&framed(4, kind::THRESHOLD_START, &[3])))), (4, code::FORWARDED));
    assert_eq!(take_forward(&mut l), Some(wire::message_with_value(wire::code::THRESHOLD_START, 4, 3)));
    assert_eq!(status_of(&answer(&mut l, Some(&framed(5, kind::THRESHOLD_STOP, &[])))), (5, code::FORWARDED));
    assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::THRESHOLD_STOP, 5)));
    // A value that names nothing is refused here; a wrong length is a bad part.
    assert_eq!(status_of(&answer(&mut l, Some(&framed(6, kind::THRESHOLD_START, &[4])))), (6, code::REFUSED));
    assert_eq!(status_of(&answer(&mut l, Some(&framed(7, kind::THRESHOLD_START, &[])))), (7, code::BAD_PART));
    assert_eq!(status_of(&answer(&mut l, Some(&framed(8, kind::THRESHOLD_STOP, &[1])))), (8, code::BAD_PART));
}

/// PM17.7 (finding A9): at a battery level of ERT or below the measured
/// data and the Markers are disabled. The requests for them are declined
/// and not handed to `diag`, a Marker transmission is ended, and a record
/// of either kind is not told; everything else is as at BOL.
#[test]
fn at_ert_the_measured_data_and_the_markers_are_disabled() {
    use pm_core::record::{data, head, request};
    let poll = Frame::empty(0, kind::POLL);
    let disabled = [
        framed(1, kind::MEASURE, &[]),
        framed(2, kind::READ_RECORD, &[data::MEASURED, 1]),
        framed(3, kind::READ_RECORD, &[data::MARKER, 0]),
        framed(4, kind::MARKERS_START, &[]),
    ];
    for level in 0u8..=4 {
        for f in &disabled {
            let mut l = Link::new();
            assert_eq!(status_of(&answer(&mut l, Some(&framed(9, kind::SET_BATTERY, &[level])))), (9, code::FORWARDED));
            // Setting the level ends a Marker transmission from ERT on.
            assert_eq!(take_request(&mut l), if level >= 2 { Some([request::MARKERS, 0, 0]) } else { None }, "level {level}");
            let a = status_of(&answer(&mut l, Some(f)));
            if level >= 2 {
                assert_eq!(a, (f.seq, code::DECLINED), "level {level}, kind {:#x}", f.kind);
                assert_eq!(take_request(&mut l), None, "level {level}, kind {:#x}", f.kind);
            } else {
                assert_eq!(a, (f.seq, code::REQUESTED), "level {level}, kind {:#x}", f.kind);
                assert!(take_request(&mut l).is_some());
            }
        }
        // What the level does not disable is requested at every level.
        for (f, want) in [
            (framed(1, kind::READ_RECORD, &[data::HISTOGRAM, 0]), [request::READ, data::HISTOGRAM as u32, 0]),
            (framed(2, kind::READ_RECORD, &[data::COUNTS, 1]), [request::READ, data::COUNTS as u32, 1]),
            (framed(3, kind::READ_RECORD, &[data::TREND, 5]), [request::READ, data::TREND as u32, 5]),
            (framed(4, kind::READ_RECORD, &[data::IMPLANT, 0]), [request::READ, data::IMPLANT as u32, 0]),
            (framed(5, kind::MARKERS_STOP, &[]), [request::MARKERS, 0, 0]),
            (framed(6, kind::RESET_HISTOGRAMS, &[]), [request::RESET_HISTOGRAMS, 0, 0]),
        ] {
            let mut l = Link::new();
            let _ = answer(&mut l, Some(&framed(9, kind::SET_BATTERY, &[level])));
            let _ = take_request(&mut l);
            assert_eq!(status_of(&answer(&mut l, Some(&f))), (f.seq, code::REQUESTED), "level {level}, kind {:#x}", f.kind);
            assert_eq!(take_request(&mut l), Some(want));
        }
        // The records: a Marker and the measured data are not told from ERT on.
        for k in [data::MARKER, data::MEASURED, data::HISTOGRAM, data::COUNTS, data::TREND, data::IMPLANT] {
            let off = level >= 2 && (k == data::MARKER || k == data::MEASURED);
            let m = [head(k, 0), 1, 2, 3, 4, 5];
            // One that comes at the level.
            let mut l = Link::new();
            let _ = answer(&mut l, Some(&framed(9, kind::SET_BATTERY, &[level])));
            record_in(&mut l, &m);
            assert_eq!(wants_record(&l), off, "level {level}, kind {k:#x}");
            assert_eq!(l.withheld, u32::from(off));
            assert_eq!(answer(&mut l, Some(&poll)).kind, if off { kind::IDLE } else { kind::RECORD }, "level {level}, kind {k:#x}");
            // One taken before the level was set and not yet told.
            let mut l = Link::new();
            record_in(&mut l, &m);
            let _ = answer(&mut l, Some(&framed(9, kind::SET_BATTERY, &[level])));
            assert_eq!(l.withheld, u32::from(off));
            assert_eq!(answer(&mut l, Some(&poll)).kind, if off { kind::IDLE } else { kind::RECORD }, "level {level}, kind {k:#x}");
        }
    }
    // The level of `control`'s status counts as well: a `link` that has
    // started again has no level of the DCM's.
    for told in 0u8..=4 {
        let mut l = Link::new();
        let st = wire::Status { battery: told, ..wire::Status::of_words(&[0; 4]) };
        status_in(&mut l, &st.words());
        assert_eq!(pacemaker_comms::proto::level(&l), u32::from(told));
        let want = if told >= 2 { code::DECLINED } else { code::REQUESTED };
        let a = answer(&mut l, Some(&framed(3, kind::MEASURE, &[])));
        assert_eq!((a.kind, a.payload[0], a.payload[1]), (kind::STATUS, 3, want), "status level {told}");
        record_in(&mut l, &[head(data::MARKER, 0), 0, 0, 0, 0, 0]);
        assert_eq!(l.withheld, u32::from(told >= 2));
        // The worse of the two.
        let _ = answer(&mut l, Some(&framed(4, kind::SET_BATTERY, &[3])));
        assert_eq!(pacemaker_comms::proto::level(&l), u32::from(told.max(3)));
    }
    // Back at BOL they are supplied again.
    let mut l = Link::new();
    let _ = answer(&mut l, Some(&framed(1, kind::SET_BATTERY, &[2])));
    let _ = answer(&mut l, Some(&framed(2, kind::SET_BATTERY, &[0])));
    let _ = take_request(&mut l);
    assert_eq!(status_of(&answer(&mut l, Some(&framed(3, kind::MEASURE, &[])))), (3, code::REQUESTED));
}
