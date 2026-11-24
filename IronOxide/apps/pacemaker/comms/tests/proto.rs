//! `proto::answer`, the link's answers (task 15.3), case by case. The
//! DCM_Stub's tests (`tools/dcm-stub/tests/session.rs`) run the same
//! function through a session.
#![forbid(unsafe_code)]

use pacemaker_comms::proto::{answer, code, peer_changed, status_in, take_forward, Link, IDENTITY_LEN, MODEL, SERIAL, STATUS_LEN, UNKNOWN};
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
        Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; 28],
               forward_ready: 0, unforwarded: 0, forward: [0; 29], have_status: 0, status: [0; 4], told: [0; 3] }
    );
    // Task 16.1 added the words of the two Endpoints (2 + 29 + 1 + 4 + 3).
    assert_eq!(core::mem::size_of::<Link>(), 9 * 4 + 4 + 28 * 4 + 28 * 4 + 39 * 4, "no padding");
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
    for k in [kind::SET_BATTERY, kind::SET_ACTIVITY, kind::SET_CLOCK, kind::RESET_HISTOGRAMS, kind::EGRAM_START, kind::EGRAM_STOP, kind::THRESHOLD_START, kind::THRESHOLD_STOP, kind::MEASURE] {
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
    for k in [kind::POLL, kind::INTERROGATE, kind::READ_PARAMETERS, kind::SET_BATTERY] {
        let _ = answer(&mut l, Some(&Frame::empty(0x21, k)));
        assert_eq!(take_forward(&mut l), Some(wire::message(wire::code::HEARD, 0x21)), "kind {k:#x}");
        assert_eq!(take_forward(&mut l), None, "once");
    }
    for k in [kind::END_TEMPORARY, kind::PACE_NOW] {
        let a = answer(&mut l, Some(&Frame::empty(0x22, k)));
        assert_eq!(status_of(&a), (0x22, code::FORWARDED));
        assert_eq!(take_forward(&mut l), Some(wire::message(k, 0x22)));
    }
    // The parts of a set are heard; the part that completes it is the command.
    let mut words = NOMINAL_WORDS;
    words[1] = 31; // `link` does not judge the set
    for (k, code) in [(kind::PROGRAM_PART, wire::code::PROGRAM), (kind::TEMPORARY_PART, wire::code::TEMPORARY)] {
        for p in 0..5u8 {
            let _ = answer(&mut l, set_part(0x30 + p, k, &words, p).as_ref());
            let m = take_forward(&mut l).expect("a message");
            if p < 4 {
                assert_eq!(m, wire::message(wire::code::HEARD, 0x30 + p));
            } else {
                assert_eq!(m, wire::message_with_set(code, 0x34, &words));
                assert!(!wire::admitted(&m));
            }
        }
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
