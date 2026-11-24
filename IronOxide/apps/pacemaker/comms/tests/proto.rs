//! `proto::answer`, the link's answers (task 15.3), case by case. The
//! DCM_Stub's tests (`tools/dcm-stub/tests/session.rs`) run the same
//! function through a session.
#![forbid(unsafe_code)]

use pacemaker_comms::proto::{answer, code, Link, IDENTITY_LEN, MODEL, SERIAL, STATUS_LEN, UNKNOWN};
use pm_core::frame::{decode, encode, kind, set_part, Assembly, Frame};
use pm_core::NOMINAL_WORDS;

fn status_of(f: &Frame) -> (u8, u8) {
    assert_eq!((f.kind, f.len), (kind::STATUS, STATUS_LEN));
    assert_eq!(&f.payload[2..5], &[UNKNOWN; 3]);
    (f.payload[0], f.payload[1])
}

#[test]
fn the_zero_state_is_the_new_state() {
    assert_eq!(Link::new(), Link { sent: 0, heard: 0, unheard: 0, silent: 0, sets: 0, have_set: 0, set_kind: 0, assembly_kind: 0, read_next: 0, assembly: Assembly::EMPTY, set: [0; 28] });
    assert_eq!(core::mem::size_of::<Link>(), 9 * 4 + 4 + 28 * 4 + 28 * 4, "no padding");
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
    for k in [kind::END_TEMPORARY, kind::PACE_NOW, kind::SET_BATTERY, kind::SET_ACTIVITY, kind::SET_CLOCK, kind::RESET_HISTOGRAMS, kind::EGRAM_START, kind::EGRAM_STOP, kind::THRESHOLD_START, kind::THRESHOLD_STOP, kind::MEASURE] {
        let mut l = Link::new();
        assert_eq!(status_of(&answer(&mut l, Some(&Frame::empty(3, k)))), (3, code::NOT_SERVED), "kind {k:#x}");
    }
}
