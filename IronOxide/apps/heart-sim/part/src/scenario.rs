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

/// The scenario this build plays.
#[cfg(feature = "scenario-silent")]
pub const SELECTED: &Scenario = &SILENT;
#[cfg(all(feature = "scenario-fast", not(feature = "scenario-silent")))]
pub const SELECTED: &Scenario = &FAST;
#[cfg(all(feature = "scenario-responsive", not(any(feature = "scenario-silent", feature = "scenario-fast"))))]
pub const SELECTED: &Scenario = &RESPONSIVE;
#[cfg(not(any(feature = "scenario-silent", feature = "scenario-fast", feature = "scenario-responsive")))]
pub const SELECTED: &Scenario = &FIXED;
