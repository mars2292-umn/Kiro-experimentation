//! Instants of the therapy, 64 bits (investigation of 2026-11-24).
//!
//! An instant is a count of microseconds from the start of the Partition
//! that does not wrap: 2^64 µs are 584,000 years. Instants are therefore
//! ordered as numbers, and the App_Prover reasons about "before" and "after"
//! as integer comparisons. The hardware's timer has 32 bits; the hardware
//! layer extends its readings (`pm-hw`), and truncates an instant when it
//! writes a compare.
//!
//! Sums saturate at the top of the range, which no run reaches; a contract
//! that speaks of a sum says so through [`sum`].

/// An instant: microseconds since the start of the Partition.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Micros(pub u64);

/// For the App_Prover: `a` is before `b`.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn before(a: Micros, b: Micros) -> bool {
    use creusot_std::macros::pearlite;
    use creusot_std::model::View;
    pearlite! { a.0@ < b.0@ }
}

/// For the App_Prover: the instant `d` microseconds after `a`, as
/// [`Micros::plus`] gives it.
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn sum(a: Micros, d: u32) -> creusot_std::prelude::Int {
    use creusot_std::macros::pearlite;
    use creusot_std::model::View;
    pearlite! { sum_i(a, d@) }
}

/// For the App_Prover: the instant `d` microseconds after `a`, for a `d`
/// that is a bound and not a value of the program (the longest or the
/// shortest escape interval a parameter set allows).
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn sum_i(a: Micros, d: creusot_std::prelude::Int) -> creusot_std::prelude::Int {
    use creusot_std::macros::pearlite;
    use creusot_std::model::View;
    pearlite! { if a.0@ + d > 18446744073709551615 { 18446744073709551615 } else { a.0@ + d } }
}

impl Micros {
    /// The instant `d` microseconds after this one.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0@ == sum(self, d)))]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0@ >= self.0@))]
    pub const fn plus(self, d: u32) -> Micros {
        let (s, over) = self.0.overflowing_add(d as u64);
        Micros(if over { u64::MAX } else { s })
    }

    /// The instant `d` microseconds before this one (0 at the earliest).
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0@ <= self.0@))]
    pub const fn minus(self, d: u32) -> Micros {
        Micros(if self.0 >= d as u64 { self.0 - d as u64 } else { 0 })
    }

    /// Whether this instant is strictly before `other`.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == before(self, other)))]
    pub const fn is_before(self, other: Micros) -> bool {
        self.0 < other.0
    }

    /// Whether this instant is at or after `other`.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == !before(self, other)))]
    pub const fn is_at_or_after(self, other: Micros) -> bool {
        self.0 >= other.0
    }

    /// The later of two instants.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0@ >= self.0@ && result.0@ >= other.0@ && (result == self || result == other)))]
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
    #[cfg_attr(creusot, creusot_std::macros::ensures(result.0@ <= self.0@ && result.0@ <= other.0@ && (result == self || result == other)))]
    pub const fn earlier(self, other: Micros) -> Micros {
        if self.is_before(other) {
            self
        } else {
            other
        }
    }

    /// The microseconds from `earlier` to this instant: 0 if this instant
    /// is not after it, and the most a `u32` holds if it is further.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(self.0@ <= earlier.0@ ==> result@ == 0))]
    #[cfg_attr(creusot, creusot_std::macros::ensures(self.0@ > earlier.0@ && self.0@ - earlier.0@ <= 4294967295 ==> result@ == self.0@ - earlier.0@))]
    pub const fn elapsed_since(self, earlier: Micros) -> u32 {
        if self.0 <= earlier.0 {
            0
        } else {
            let d = self.0 - earlier.0;
            if d > u32::MAX as u64 {
                u32::MAX
            } else {
                d as u32
            }
        }
    }
}
