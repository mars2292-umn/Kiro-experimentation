//! Host tests of `pm-core`'s `params` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


#[test]
fn a_rate_s_interval_is_rounded_down() {
    assert_eq!(interval_of_ppm(60), 1_000_000);
    assert_eq!(interval_of_ppm(30), 2_000_000);
    assert_eq!(interval_of_ppm(65), 923_076);
    assert_eq!(interval_of_ppm(175), 342_857);
    // Rounded down: the interval times the rate never exceeds a minute,
    // and is less than one interval-rounding short of it.
    let mut ppm = 30;
    while ppm <= 175 {
        let i = interval_of_ppm(ppm);
        assert!(i * ppm <= 60_000_000);
        assert!((i + 1) * ppm > 60_000_000);
        ppm += 1;
    }
}

#[test]
fn the_nominal_set_is_table_19_1() {
    let p = Params::NOMINAL;
    assert_eq!(p.mode, Mode::DDD);
    assert_eq!(p.lrl, 1_000_000);
    assert_eq!(p.url, 500_000);
    assert_eq!(p.fixed_av, 150_000);
    assert_eq!(p.width, [400, 400]);
    assert_eq!(p.vrp, 320_000);
    assert_eq!(p.arp, 250_000);
    assert_eq!(p.pvarp, 250_000);
    assert_eq!(p.v_blanking, 40_000);
    assert_eq!(p.activity_threshold, ActivityThreshold::default());
}

#[test]
fn power_on_reset_and_pace_now_are_the_source_s() {
    let p = Params::POWER_ON_RESET;
    assert_eq!(p.mode, Mode::VVI);
    assert_eq!(p.lrl, 923_076);
    assert_eq!(p.width[1], 500);
    assert_eq!(p.amplitude[1], 50);
    assert_eq!(p.sensitivity[1], 150);
    assert_eq!(p.vrp, 320_000);
    let n = Params::PACE_NOW;
    assert_eq!(n.mode, Mode::VVI);
    assert_eq!(n.lrl, 923_076);
    assert_eq!(n.width[1], 1_000);
    assert_eq!(n.vrp, 320_000);
}
