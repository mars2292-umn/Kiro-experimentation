//! Host tests of `pm-core`'s `time` module. An integration test, so that the
//! crate itself names no `std` (rsk's check R6.1).
//!
//! Until 2026-11-24 an instant had 32 bits and these tests were of the
//! comparison by signed difference across the counter's wrap. An instant
//! now has 64 bits and does not wrap: it is ordered as a number, and a sum
//! saturates at the top of the range. The counter's wrap is the hardware
//! layer's (`pm-hw`: `Count`, and the driver's `extend`), and is tested
//! there.
#![forbid(unsafe_code)]

use pm_core::Micros;

const BASES: [u64; 9] = [0, 1, 1_000_000, (1 << 31) - 1, 1 << 31, u32::MAX as u64 - 2_100_000, u32::MAX as u64, 1 << 32, 1 << 62];
const SPANS: [u32; 8] = [1, 50, 1_900, 30_000, 342_857, 2_000_000, 2_100_000, u32::MAX];

#[test]
fn an_instant_is_ordered_as_a_number() {
    for b in BASES {
        for d in SPANS {
            let a = Micros(b);
            let later = a.plus(d);
            assert_eq!(later.0, b + u64::from(d), "base {b} span {d}");
            assert!(a.is_before(later) && !later.is_before(a) && !a.is_before(a));
            assert!(later.is_at_or_after(a) && !a.is_at_or_after(later) && a.is_at_or_after(a));
            assert_eq!((a.later(later), later.later(a), a.earlier(later), later.earlier(a)), (later, later, a, a));
            assert_eq!((later.elapsed_since(a), a.elapsed_since(later), a.elapsed_since(a)), (d, 0, 0));
            assert_eq!(later.minus(d), a);
        }
    }
}

/// Nothing wraps where the 32-bit counter does: an instant just below
/// 2^32 µs is before one just above it, by their distance.
#[test]
fn nothing_happens_at_the_counters_wrap() {
    let a = Micros(u64::from(u32::MAX) - 10);
    let b = a.plus(100);
    assert_eq!(b.0, (1 << 32) + 89);
    assert!(a.is_before(b));
    assert_eq!(b.elapsed_since(a), 100);
    // Instants further apart than the counter's whole range are still ordered.
    let far = Micros(5 << 32);
    assert!(a.is_before(far) && !far.is_before(a));
}

#[test]
fn the_ends_of_the_range_saturate() {
    let top = Micros(u64::MAX - 5);
    assert_eq!(top.plus(5).0, u64::MAX);
    assert_eq!(top.plus(6).0, u64::MAX);
    assert_eq!(top.plus(u32::MAX).0, u64::MAX);
    assert!(!top.plus(100).is_before(top), "a sum is never before what it was added to");
    assert_eq!(Micros(7).minus(7).0, 0);
    assert_eq!(Micros(7).minus(8).0, 0);
    assert_eq!(Micros(0).minus(u32::MAX).0, 0);
    // A distance beyond what 32 bits hold is the most they hold.
    assert_eq!(Micros(1 << 40).elapsed_since(Micros(0)), u32::MAX);
    assert_eq!(Micros(u64::from(u32::MAX)).elapsed_since(Micros(0)), u32::MAX);
    assert_eq!(Micros(u64::from(u32::MAX) - 1).elapsed_since(Micros(0)), u32::MAX - 1);
}

#[test]
fn the_zero_instant_is_the_default() {
    assert_eq!(Micros::default(), Micros(0));
    assert_eq!(core::mem::size_of::<Micros>(), 8);
}
