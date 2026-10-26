//! One pure control law for the Aeneas evaluation (task 17.3, R46.1): a
//! fixed-point PI step, the kind of small arithmetic computation a level A
//! Task body performs between its Critical_Sections, written as Aeneas
//! can translate it (integers, a `&mut` state, no floats, no `unsafe`).
//!
//! Scaling: gains in Q15 (1.0 is 32768). The error is clamped to
//! `ERR_MAX`, the integral to `INT_MAX` (anti-windup), the output to
//! `OUT_MAX`. The intermediate product of the integral gain is taken in
//! `i64`, since `KI * INT_MAX` does not fit an `i32`; the shifted result
//! does, which the proof establishes together with every other
//! overflow-freedom obligation (every `+`, `*`, `>>`, and `as` is a proof
//! obligation in the Lean translation: a function whose result is `fail`
//! for some input cannot be proven to return `ok`).
//!
//! Properties proven in Lean (`proofs/aeneas/lean/ControlLaw/Proofs.lean`)
//! from the Aeneas translation of this file:
//! - `pi_step` returns `ok` for every state with `|integral| <= INT_MAX` and
//!   every error (absence of panics and arithmetic overflow, R46.3 shape);
//! - the output lies in `[-OUT_MAX, OUT_MAX]` (saturation);
//! - the new integral lies in `[-INT_MAX, INT_MAX]` (the invariant is
//!   preserved from Job to Job).
#![no_std]
#![forbid(unsafe_code)]

/// Proportional gain, Q15 (0.5).
pub const KP: i32 = 16384;
/// Integral gain, Q15 (0.05).
pub const KI: i32 = 1638;
/// The error is clamped to this magnitude before use.
pub const ERR_MAX: i32 = 1 << 16;
/// Anti-windup clamp of the integral.
pub const INT_MAX: i32 = 1 << 22;
/// Output saturation (below `KP * ERR_MAX + KI * INT_MAX` in Q15, so a
/// sustained maximal error saturates the output).
pub const OUT_MAX: i32 = 1 << 17;

/// The controller's per-Task state: the accumulated error.
// `Clone, Copy` only: `Debug` would pull `core::fmt` into the translation,
// and the name `Pi` clashes with Mathlib's in the Lean namespace.
#[derive(Clone, Copy)]
pub struct PiState {
    pub integral: i32,
}

impl PiState {
    pub const fn new() -> PiState {
        PiState { integral: 0 }
    }
}

/// `x` clamped to `[lo, hi]` (`lo <= hi`).
pub fn clamp(x: i32, lo: i32, hi: i32) -> i32 {
    if x < lo {
        lo
    } else if x > hi {
        hi
    } else {
        x
    }
}

/// One PI step: clamps the error, accumulates it with anti-windup, and
/// returns the saturated output `KP * e + KI * integral` (Q15).
pub fn pi_step(state: &mut PiState, error: i32) -> i32 {
    let e = clamp(error, -ERR_MAX, ERR_MAX);
    let integral = clamp(state.integral + e, -INT_MAX, INT_MAX);
    state.integral = integral;
    let p = (KP * e) >> 15;
    let i = ((KI as i64 * integral as i64) >> 15) as i32;
    clamp(p + i, -OUT_MAX, OUT_MAX)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn output_and_integral_stay_within_their_bounds() {
        let mut s = PiState::new();
        for k in 0..100_000i32 {
            let error = (k.wrapping_mul(2_654_435_761u32 as i32)) ^ i32::MAX;
            let out = pi_step(&mut s, error);
            assert!((-OUT_MAX..=OUT_MAX).contains(&out));
            assert!((-INT_MAX..=INT_MAX).contains(&s.integral));
        }
        // A constant error saturates the integral and the output.
        let mut s = PiState::new();
        let mut last = 0;
        for _ in 0..1000 {
            last = pi_step(&mut s, ERR_MAX);
        }
        assert_eq!(s.integral, INT_MAX);
        assert_eq!(last, OUT_MAX);
    }
}
