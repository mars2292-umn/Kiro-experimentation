//! Instants of the therapy timer.

/// Half the range of the counter, in microseconds: 2^31 µs, about 35.8
/// minutes. Two instants are comparable when they are less than this apart.
/// Every interval the therapy uses is under 2.1 s.
pub const HALF_RANGE: u32 = 1 << 31;

/// An instant: the therapy timer's count in microseconds, 32 bits.
///
/// The counter wraps every 2^32 µs (71.6 minutes), so instants are not
/// ordered as numbers. They are compared by signed difference: `a` is before
/// `b` when `b - a`, computed modulo 2^32 and read as a signed number, is
/// positive. This is right whenever the two are less than [`HALF_RANGE`]
/// apart. `Micros` therefore has no `Ord`.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Micros(pub u32);

impl Micros {
    /// The instant `d` microseconds after this one.
    #[must_use]
    pub const fn plus(self, d: u32) -> Micros {
        Micros(self.0.wrapping_add(d))
    }

    /// The instant `d` microseconds before this one.
    #[must_use]
    pub const fn minus(self, d: u32) -> Micros {
        Micros(self.0.wrapping_sub(d))
    }

    /// `self - earlier` as a signed number of microseconds: positive when
    /// this instant is after `earlier`.
    #[must_use]
    pub const fn since(self, earlier: Micros) -> i32 {
        self.0.wrapping_sub(earlier.0) as i32
    }

    /// Whether this instant is strictly before `other`.
    #[must_use]
    pub const fn is_before(self, other: Micros) -> bool {
        other.since(self) > 0
    }

    /// Whether this instant is at or after `other`.
    #[must_use]
    pub const fn is_at_or_after(self, other: Micros) -> bool {
        self.since(other) >= 0
    }

    /// The later of two instants.
    #[must_use]
    pub const fn later(self, other: Micros) -> Micros {
        if self.is_before(other) {
            other
        } else {
            self
        }
    }

    /// The earlier of two instants.
    #[must_use]
    pub const fn earlier(self, other: Micros) -> Micros {
        if self.is_before(other) {
            self
        } else {
            other
        }
    }

    /// The microseconds from `earlier` to this instant, or 0 if this instant
    /// is not after it.
    #[must_use]
    pub const fn elapsed_since(self, earlier: Micros) -> u32 {
        let d = self.since(earlier);
        if d > 0 {
            d as u32
        } else {
            0
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
