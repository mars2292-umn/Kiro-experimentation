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
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Micros(pub u32);

/// For the App_Prover: `a` is before `b`, as [`Micros::is_before`] decides
/// it: the difference `b - a`, taken modulo 2^32, is from 1 to 2^31 - 1.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn before(a: Micros, b: Micros) -> bool {
    use creusot_std::macros::pearlite;
    use creusot_std::model::View;
    pearlite! { 0 < (b.0 - a.0)@ && (b.0 - a.0)@ < 2147483648 }
}

impl Micros {
    /// The instant `d` microseconds after this one.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0 == self.0 + d))]
    pub const fn plus(self, d: u32) -> Micros {
        Micros(self.0.wrapping_add(d))
    }

    /// The instant `d` microseconds before this one.
    #[inline(always)]
    #[must_use]
    pub const fn minus(self, d: u32) -> Micros {
        Micros(self.0.wrapping_sub(d))
    }

    /// `self - earlier` as a signed number of microseconds: positive when
    /// this instant is after `earlier`.
    #[inline(always)]
    #[must_use]
    // One direction only: the App_Prover gives the cast to a signed number
    // no meaning for a difference of 2^31 or more, so it cannot be told
    // that the result is then not positive.
    #[cfg_attr(creusot, creusot_std::macros::ensures(0 < (self.0 - earlier.0)@ && (self.0 - earlier.0)@ < 2147483648 ==> result@ == (self.0 - earlier.0)@))]
    pub const fn since(self, earlier: Micros) -> i32 {
        self.0.wrapping_sub(earlier.0) as i32
    }

    /// Whether this instant is strictly before `other`.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(before(self, other) ==> result))]
    pub const fn is_before(self, other: Micros) -> bool {
        other.since(self) > 0
    }

    /// Whether this instant is at or after `other`.
    #[inline(always)]
    #[must_use]
    pub const fn is_at_or_after(self, other: Micros) -> bool {
        self.since(other) >= 0
    }

    /// The later of two instants.
    #[inline(always)]
    #[must_use]
    pub const fn later(self, other: Micros) -> Micros {
        if self.is_before(other) {
            other
        } else {
            self
        }
    }

    /// The earlier of two instants.
    #[inline(always)]
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
    #[inline(always)]
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
