//! Host tests of `pm-core`'s `mode` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


#[test]
fn zero_is_off() {
    assert_eq!(Mode::default(), Mode::OFF);
    assert_eq!(Chambers::default() as u8, 0);
    assert_eq!(Response::default() as u8, 0);
}

#[test]
fn the_programmable_modes_are_distinct_and_nineteen() {
    let m = Mode::PROGRAMMABLE;
    for i in 0..m.len() {
        for j in 0..i {
            assert_ne!(m[i], m[j], "{i} and {j}");
        }
    }
    // Eight rate-adaptive forms, as Table 19-1 lists them.
    assert_eq!(m.iter().filter(|x| x.adaptive).count(), 8);
    // Off, DDD (the nominal) at the table's first two places.
    assert_eq!(m[0], Mode::OFF);
    assert_eq!(m[1], Mode::DDD);
}

#[test]
fn letters_decide_chambers() {
    assert!(Mode::VVI.paces(Chamber::V) && !Mode::VVI.paces(Chamber::A));
    assert!(Mode::VDD.paces(Chamber::V) && !Mode::VDD.paces(Chamber::A));
    assert!(Mode::VDD.senses(Chamber::A) && Mode::VDD.senses(Chamber::V));
    assert!(Mode::DOO.paces(Chamber::A) && Mode::DOO.paces(Chamber::V));
    assert!(!Mode::DOO.senses(Chamber::A) && !Mode::DOO.senses(Chamber::V));
    assert!(!Mode::OFF.paces(Chamber::A) && !Mode::OFF.paces(Chamber::V));
}

#[test]
fn the_timing_chamber_is_the_atrium_only_in_atrial_modes() {
    for m in Mode::PROGRAMMABLE {
        let atrial_only = !m.paces(Chamber::V) && !m.senses(Chamber::V);
        assert_eq!(m.timing_chamber() == Chamber::A, atrial_only, "{m:?}");
    }
    assert_eq!(Mode::AAI.timing_chamber(), Chamber::A);
    assert_eq!(Mode::DDD.timing_chamber(), Chamber::V);
    assert_eq!(Mode::OVO.timing_chamber(), Chamber::V);
}

#[test]
fn chamber_indices() {
    assert_eq!(Chamber::A.index(), 0);
    assert_eq!(Chamber::V.index(), 1);
    assert_eq!(Chamber::A.other(), Chamber::V);
    assert_eq!(Chamber::V.other(), Chamber::A);
}
