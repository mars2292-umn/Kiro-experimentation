//! Time conversions (R21.2, R29.8, Property 14): every declared time
//! quantity becomes Kernel time-base ticks (periods, MITs, deadlines,
//! offsets, the operating duration) or CPU cycles (Budgets), rounded in
//! the direction that cannot make the analysis optimistic: Budgets up;
//! periods, MITs, and deadlines down; offsets to the nearest tick. Every
//! conversion that changes the value is reported (R21.2).

use crate::decl::{Time, TimeUnit};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Rounding {
    Up,
    Down,
    Nearest,
}

/// A conversion that was not exact, for the configuration summary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Change {
    pub what: String,
    pub declared: Time,
    pub converted: u64,
    pub unit: &'static str,
}

/// `value * num / den` rounded as requested; `None` on overflow.
fn scale(value: u64, num: u128, den: u128, rounding: Rounding) -> Option<(u64, bool)> {
    if den == 0 {
        return None;
    }
    // All three are verified: down and up by `derive_v::scale` (the result
    // is within one unit of the exact value, on the side asked for; R29.8,
    // Property 14), and to the nearest, a half upward, by
    // `derive_v::scale_nearest` (R21.2: a Task's offset; 2026-11-21, until
    // then computed here outside `verus!`).
    match rounding {
        Rounding::Down => crate::derive_v::scale(value, num, den, false),
        Rounding::Up => crate::derive_v::scale(value, num, den, true),
        Rounding::Nearest => crate::derive_v::scale_nearest(value, num, den),
    }
}

/// Converts `t` to time-base ticks at `tick_hz`. Cycles are converted
/// through the CPU clock. Returns the value and whether it was rounded.
pub fn to_ticks(t: Time, tick_hz: u64, cpu_hz: u64, rounding: Rounding) -> Option<(u64, bool)> {
    match t.unit {
        TimeUnit::Ticks => Some((t.value, false)),
        TimeUnit::Cycles => scale(t.value, tick_hz as u128, cpu_hz as u128, rounding),
        unit => {
            let nanos = unit.nanos()? as u128;
            scale(t.value, nanos * tick_hz as u128, 1_000_000_000, rounding)
        }
    }
}

/// Converts `t` to CPU cycles at `cpu_hz`.
pub fn to_cycles(t: Time, tick_hz: u64, cpu_hz: u64, rounding: Rounding) -> Option<(u64, bool)> {
    match t.unit {
        TimeUnit::Cycles => Some((t.value, false)),
        TimeUnit::Ticks => scale(t.value, cpu_hz as u128, tick_hz as u128, rounding),
        unit => {
            let nanos = unit.nanos()? as u128;
            scale(t.value, nanos * cpu_hz as u128, 1_000_000_000, rounding)
        }
    }
}

/// Ticks to cycles for the Analyzer (R29.8): periods, MITs and deadlines
/// down, execution times and jitter up.
pub fn ticks_to_cycles(ticks: u64, tick_hz: u64, cpu_hz: u64, rounding: Rounding) -> Option<u64> {
    scale(ticks, cpu_hz as u128, tick_hz as u128, rounding).map(|(v, _)| v)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(value: u64, unit: TimeUnit) -> Time {
        Time { value, unit }
    }

    #[test]
    fn overflow_is_reported() {
        assert_eq!(to_cycles(t(u64::MAX, TimeUnit::H), 16_000_000, 64_000_000, Rounding::Up), None);
    }
}
