//! The scenario tables (PM26.3). A build plays one, chosen by a cargo
//! feature; the rig's judge is given the same name (docs/pacemaker,
//! task 4.3) and reads the numbers from this table.

use crate::heart::{Extra, Override, Rhythm, Scenario, EXTRAS, OVERRIDES};

const NO_OVERRIDES: [Override; OVERRIDES] = [Override::NONE; OVERRIDES];
const NO_EXTRAS: [Extra; EXTRAS] = [Extra::NONE; EXTRAS];
const NO_RHYTHM: Rhythm = Rhythm { first_ms: 0, interval_ms: 0 };

/// A regular sinus rhythm of 75 a minute, conducted in 160 ms. Above the
/// nominal LRL, so an inhibited Mode does not pace.
pub const FIXED: Scenario = Scenario {
    name: "fixed",
    atrial: Rhythm { first_ms: 200, interval_ms: 800 },
    ventricular: NO_RHYTHM,
    conduction_ms: 160,
    overrides: NO_OVERRIDES,
    extras: NO_EXTRAS,
    responsive: false,
};

/// No intrinsic activity: every beat has to be a Pace.
pub const SILENT: Scenario = Scenario { name: "silent", atrial: NO_RHYTHM, ventricular: NO_RHYTHM, conduction_ms: 0, overrides: NO_OVERRIDES, extras: NO_EXTRAS, responsive: false };

/// A sinus rhythm of 120 a minute, conducted in 150 ms, with one atrial
/// beat 200 ms premature and one extra ventricular beat 250 ms after a
/// conducted one (a PVC: no atrial event since the last ventricular event).
pub const FAST: Scenario = Scenario {
    name: "fast",
    atrial: Rhythm { first_ms: 200, interval_ms: 500 },
    ventricular: NO_RHYTHM,
    conduction_ms: 150,
    overrides: [Override { chamber: 0, beat: 10, shift_ms: -200 }, Override::NONE, Override::NONE, Override::NONE],
    // The atrial beat at 2,200 conducts at 2,350; the next atrial beat is at 2,700.
    extras: [Extra { chamber: 1, at_ms: 2_600 }, Extra::NONE, Extra::NONE, Extra::NONE],
    responsive: false,
};

/// A sinus rhythm of 40 a minute, conducted in 180 ms, that a Pace
/// captures: below every LRL from 45 ppm, so the PG paces and the heart
/// answers.
pub const RESPONSIVE: Scenario = Scenario {
    name: "responsive",
    atrial: Rhythm { first_ms: 300, interval_ms: 1_500 },
    ventricular: NO_RHYTHM,
    conduction_ms: 180,
    overrides: NO_OVERRIDES,
    extras: NO_EXTRAS,
    responsive: true,
};

/// Every scenario, for the tests and the judge's cross-check.
pub const ALL: [&Scenario; 4] = [&FIXED, &SILENT, &FAST, &RESPONSIVE];

/// The numbers of a scenario given to the build as text (feature
/// `scenario-env`, docs/pacemaker tasks 9.4 and 9.5): up to five decimal
/// numbers separated by commas in the environment variable `PM_HEART` at
/// build time: the atrial rhythm's first beat and interval, the
/// ventricular rhythm's first beat and interval, and the conduction, all in
/// milliseconds; a number left out is 0.
#[rsk::bounds('text = 128)]
pub const fn numbers(text: &str) -> [u32; 5] {
    let b = text.as_bytes();
    let mut out = [0u32; 5];
    let mut k = 0;
    let mut i = 0;
    'text: while i < b.len() && i < 128 {
        let c = b[i];
        if c >= b'0' && c <= b'9' {
            if k < 5 {
                out[k] = out[k].saturating_mul(10).saturating_add((c - b'0') as u32);
            }
        } else if c == b',' {
            k += 1;
        }
        i += 1;
    }
    out
}

/// The scenario of `PM_HEART`: two free-running rhythms and a conduction,
/// no deviation, and no answer to a Pace.
#[cfg(feature = "scenario-env")]
pub const ENV: Scenario = {
    let n = numbers(env!("PM_HEART"));
    Scenario {
        name: "env",
        atrial: Rhythm { first_ms: n[0], interval_ms: n[1] },
        ventricular: Rhythm { first_ms: n[2], interval_ms: n[3] },
        conduction_ms: n[4],
        overrides: NO_OVERRIDES,
        extras: NO_EXTRAS,
        responsive: false,
    }
};

/// The scenario this build plays.
#[cfg(feature = "scenario-env")]
pub const SELECTED: &Scenario = &ENV;
#[cfg(all(feature = "scenario-silent", not(feature = "scenario-env")))]
pub const SELECTED: &Scenario = &SILENT;
#[cfg(all(feature = "scenario-fast", not(any(feature = "scenario-silent", feature = "scenario-env"))))]
pub const SELECTED: &Scenario = &FAST;
#[cfg(all(feature = "scenario-responsive", not(any(feature = "scenario-silent", feature = "scenario-fast", feature = "scenario-env"))))]
pub const SELECTED: &Scenario = &RESPONSIVE;
#[cfg(not(any(feature = "scenario-silent", feature = "scenario-fast", feature = "scenario-responsive", feature = "scenario-env")))]
pub const SELECTED: &Scenario = &FIXED;
