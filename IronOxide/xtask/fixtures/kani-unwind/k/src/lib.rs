//! A harness with a loop bound above its `#[kani::unwind]`: every assertion
//! holds, but the unwinding assertion fails, which the verification job
//! must report as a failed run (R45.5).
#![no_std]
#![forbid(unsafe_code)]

/// Sums the first `n` naturals with a loop.
pub fn sum_to(n: u32) -> u32 {
    let mut i = 0;
    let mut acc = 0u32;
    while i < n {
        i += 1;
        acc = acc.wrapping_add(i);
    }
    acc
}

#[cfg(kani)]
mod harness {
    #[kani::proof]
    #[kani::unwind(3)]
    fn sum_to_is_bounded_but_unwinds_too_little() {
        let n: u32 = kani::any();
        kani::assume(n <= 8);
        let _ = super::sum_to(n);
    }
}
