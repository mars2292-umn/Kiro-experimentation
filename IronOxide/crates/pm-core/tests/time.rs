//! Host tests of `pm-core`'s `time` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


/// Bases on both sides of the wrap and of the sign change.
const BASES: [u32; 9] =
    [0, 1, 1_000_000, HALF_RANGE - 1, HALF_RANGE, HALF_RANGE + 1, u32::MAX - 2_100_000, u32::MAX - 1, u32::MAX];
/// Distances the therapy uses, and the largest comparable one.
const SPANS: [u32; 8] = [1, 50, 1_900, 30_000, 342_857, 2_000_000, 2_100_000, HALF_RANGE - 1];

#[test]
fn plus_then_since_is_the_distance_at_every_base() {
    for b in BASES {
        for d in SPANS {
            let a = Micros(b);
            let later = a.plus(d);
            assert_eq!(later.since(a), d as i32, "base {b} span {d}");
            assert_eq!(a.since(later), -(d as i32), "base {b} span {d}");
            assert_eq!(later.elapsed_since(a), d);
            assert_eq!(a.elapsed_since(later), 0);
            assert_eq!(later.minus(d), a);
        }
    }
}

#[test]
fn order_holds_across_the_wrap() {
    for b in BASES {
        for d in SPANS {
            let a = Micros(b);
            let later = a.plus(d);
            assert!(a.is_before(later), "base {b} span {d}");
            assert!(!later.is_before(a));
            assert!(later.is_at_or_after(a));
            assert!(!a.is_at_or_after(later));
            assert_eq!(a.later(later), later);
            assert_eq!(later.later(a), later);
            assert_eq!(a.earlier(later), a);
            assert_eq!(later.earlier(a), a);
        }
    }
}

#[test]
fn an_instant_is_neither_before_nor_after_itself() {
    for b in BASES {
        let a = Micros(b);
        assert!(!a.is_before(a));
        assert!(a.is_at_or_after(a));
        assert_eq!(a.since(a), 0);
        assert_eq!(a.later(a), a);
        assert_eq!(a.earlier(a), a);
    }
}

#[test]
fn the_wrap_itself() {
    let before = Micros(u32::MAX - 9);
    let after = before.plus(20);
    assert_eq!(after, Micros(10));
    assert!(before.is_before(after));
    assert_eq!(after.since(before), 20);
}

/// The comparison is only meaningful under half the range: at exactly
/// half, the difference reads as negative. Recorded so that a change of
/// the representation is noticed.
#[test]
fn half_the_range_is_the_limit() {
    let a = Micros(0);
    assert!(a.is_before(a.plus(HALF_RANGE - 1)));
    assert!(!a.is_before(a.plus(HALF_RANGE)));
}

#[test]
fn sums_of_therapy_intervals_compose() {
    // v + E - AV, the atrial instant of a dual-chamber cycle, across the wrap.
    let v = Micros(u32::MAX - 500_000);
    let e = 1_000_000;
    let av = 150_000;
    let atrial = v.plus(e).minus(av);
    assert_eq!(atrial.since(v), (e - av) as i32);
    assert!(atrial.is_before(v.plus(e)));
}
