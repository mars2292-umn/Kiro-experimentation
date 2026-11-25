//! The messages of the Endpoints `program` and `status` (task 16.1).
#![forbid(unsafe_code)]

use pm_core::wire::{admitted, battery_of, code, code_of, message, message_with_set, message_with_value, mode_byte, seq_of, set_of, value_of, Status, PROGRAM_WORDS};
use pm_core::{validate, word, Mode, NOMINAL_WORDS, WORDS};

#[test]
fn a_message_carries_its_code_its_number_and_its_set() {
    let m = message(code::PACE_NOW, 0xA7);
    assert_eq!((code_of(&m), seq_of(&m)), (code::PACE_NOW, 0xA7));
    assert_eq!(set_of(&m), [0; WORDS]);
    let m = message_with_set(code::PROGRAM, 3, &NOMINAL_WORDS);
    assert_eq!((code_of(&m), seq_of(&m), set_of(&m)), (code::PROGRAM, 3, NOMINAL_WORDS));
    assert_eq!(m.len(), PROGRAM_WORDS);
}

#[test]
fn the_validation_admits_exactly_the_seven_codes() {
    for c in 0..=255u8 {
        let plain = admitted(&message(c, 9));
        let with_set = admitted(&message_with_set(c, 9, &NOMINAL_WORDS));
        match c {
            // (Five codes until tasks 16.4 and 16.5 added the two that carry a value; 0 is a value of both.)
            code::HEARD | code::END_TEMPORARY | code::PACE_NOW | code::BATTERY | code::MAGNET_RESPONSE => assert!(plain && !with_set, "code {c:#x}"),
            // A set of zeros is no set (the Mode's word alone would pass; the LRL's does not).
            code::PROGRAM | code::TEMPORARY => assert!(!plain && with_set, "code {c:#x}"),
            _ => assert!(!plain && !with_set, "code {c:#x}"),
        }
    }
}

#[test]
fn a_set_is_admitted_exactly_when_validate_accepts_it() {
    // Every word in turn through values around and far from its row.
    let mut both = [0u32; 2];
    for i in 0..WORDS {
        for v in (0..=2000u32).chain([0x7FFF_FFFF, 0xFFFF_FFFF]) {
            let mut w = NOMINAL_WORDS;
            w[i] = v;
            let want = validate(&w).is_some();
            both[want as usize] += 1;
            assert_eq!(admitted(&message_with_set(code::PROGRAM, 0, &w)), want, "word {i} = {v}");
            assert_eq!(admitted(&message_with_set(code::TEMPORARY, 0xFF, &w)), want, "word {i} = {v}");
        }
    }
    assert!(both[0] > 1000 && both[1] > 100, "{both:?}");
}

#[test]
fn nothing_above_the_number_and_no_stray_word() {
    let mut m = message(code::HEARD, 1);
    m[0] |= 0x1_0000;
    assert!(!admitted(&m));
    let mut m = message_with_set(code::PROGRAM, 1, &NOMINAL_WORDS);
    m[0] |= 0x8000_0000;
    assert!(!admitted(&m));
    for k in 1..PROGRAM_WORDS {
        let mut m = message(code::END_TEMPORARY, 1);
        m[k] = 1;
        assert!(!admitted(&m), "word {k}");
    }
}

#[test]
fn a_status_goes_through_its_words() {
    let s = Status { therapy: 2, mode: mode_byte(Mode::DDD), battery: 1, link_up: true, serial: 0x1234_5678, verdict: 6, of: 0xEE, refused: 3 };
    assert_eq!(Status::of_words(&s.words()), s);
    let z = Status::of_words(&[0; 4]);
    assert_eq!((z.therapy, z.link_up, z.serial, z.verdict), (0, false, 0, 0));
}

#[test]
fn the_mode_byte_tells_the_nineteen_modes_apart() {
    let bytes: Vec<u8> = Mode::PROGRAMMABLE.iter().map(|m| mode_byte(*m)).collect();
    for (i, a) in bytes.iter().enumerate() {
        for b in &bytes[i + 1..] {
            assert_ne!(a, b);
        }
    }
    let _ = word::MODE;
}

#[test]
fn a_value_is_admitted_exactly_in_its_range() {
    for v in (0..=300u32).chain([0xFFFF_FFFF]) {
        assert_eq!(admitted(&message_with_value(code::BATTERY, 2, v)), v <= 4, "battery {v}");
        assert_eq!(admitted(&message_with_value(code::MAGNET_RESPONSE, 2, v)), v <= 1, "magnet response {v}");
        // The codes without a value take none.
        assert_eq!(admitted(&message_with_value(code::PACE_NOW, 2, v)), v == 0);
    }
    for k in 2..PROGRAM_WORDS {
        for c in [code::BATTERY, code::MAGNET_RESPONSE] {
            let mut m = message_with_value(c, 1, 1);
            m[k] = 1;
            assert!(!admitted(&m), "code {c:#x}, word {k}");
        }
    }
    let m = message_with_value(code::BATTERY, 7, 3);
    assert_eq!((code_of(&m), seq_of(&m), value_of(&m)), (code::BATTERY, 7, 3));
    assert_eq!([0, 1, 2, 3, 4, 5].map(|v| battery_of(v) as u8), [0, 1, 2, 3, 4, 4]);
}
