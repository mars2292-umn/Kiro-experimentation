//! Host tests of `pm-core`'s `event` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


#[test]
fn the_best_battery_level_is_zero() {
    assert_eq!(Battery::default() as u8, 0);
}

#[test]
fn arming_selects_by_chamber() {
    let a = Arming { atrial: Pace::Withdraw, ventricular: Pace::At { at: Micros(5), width: 400 }, markers: Markers::NONE };
    assert_eq!(a.pace(Chamber::A), Pace::Withdraw);
    assert_eq!(a.pace(Chamber::V), Pace::At { at: Micros(5), width: 400 });
    assert_eq!(Arming::NONE.pace(Chamber::A), Pace::Keep);
}
