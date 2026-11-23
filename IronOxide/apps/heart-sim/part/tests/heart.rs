//! Host tests of the simulated heart (`src/heart.rs`) and of the scenario
//! tables. They are an integration test so that the Partition crate itself
//! names no `std` (R6.1).
#![forbid(unsafe_code)]

use heart_sim_part::heart::*;
use heart_sim_part::scenario::{ALL, SELECTED};

const BASE: Scenario = Scenario {
    name: "test",
    atrial: Rhythm { first_ms: 200, interval_ms: 800 },
    ventricular: Rhythm { first_ms: 0, interval_ms: 0 },
    conduction_ms: 160,
    overrides: [Override::NONE; OVERRIDES],
    extras: [Extra::NONE; EXTRAS],
    responsive: false,
};

/// Plays `ms` milliseconds; `paces` are (instant, chamber). Returns the
/// instants of the atrial and of the ventricular beats.
fn play(sc: &Scenario, ms: u32, paces: &[(u32, u8)]) -> (Vec<u32>, Vec<u32>) {
    let mut h = Heart::default();
    let (mut a, mut v) = (Vec::new(), Vec::new());
    for t in 1..=ms {
        let b = h.tick(sc, paces.contains(&(t, 0)), paces.contains(&(t, 1)));
        assert_eq!(h.ms, t);
        if b.atrial {
            a.push(t);
        }
        if b.ventricular {
            v.push(t);
        }
    }
    (a, v)
}

#[test]
fn the_all_zero_state_is_the_state_before_the_first_job() {
    assert_eq!(Heart::default(), Heart { ms: 0, next_a: 0, next_v: 0, conducted: 0, beats_a: 0, beats_v: 0 });
}

#[test]
fn a_fixed_rhythm_with_conduction() {
    let (a, v) = play(&BASE, 3_000, &[]);
    assert_eq!(a, [200, 1_000, 1_800, 2_600]);
    assert_eq!(v, [360, 1_160, 1_960, 2_760]);
}

#[test]
fn a_silent_heart_never_beats() {
    let silent = Scenario { atrial: Rhythm { first_ms: 0, interval_ms: 0 }, conduction_ms: 0, ..BASE };
    assert_eq!(play(&silent, 5_000, &[]), (Vec::new(), Vec::new()));
    // Nor does it answer a Pace when it is not responsive.
    assert_eq!(play(&silent, 5_000, &[(100, 0), (300, 1)]), (Vec::new(), Vec::new()));
}

#[test]
fn an_override_moves_one_beat_and_the_rhythm_follows_from_it() {
    // The third atrial beat (index 2) is 300 ms premature.
    let mut sc = BASE;
    sc.overrides[0] = Override { chamber: 0, beat: 2, shift_ms: -300 };
    let (a, v) = play(&sc, 3_500, &[]);
    assert_eq!(a, [200, 1_000, 1_500, 2_300, 3_100]);
    assert_eq!(v, [360, 1_160, 1_660, 2_460, 3_260]);
}

#[test]
fn a_ventricle_with_its_own_rhythm_and_no_conduction() {
    // Complete block: the atrium at 75 a minute, the ventricle escapes at 40.
    let block = Scenario { ventricular: Rhythm { first_ms: 500, interval_ms: 1_500 }, conduction_ms: 0, ..BASE };
    let (a, v) = play(&block, 4_000, &[]);
    assert_eq!(a, [200, 1_000, 1_800, 2_600, 3_400]);
    assert_eq!(v, [500, 2_000, 3_500]);
}

#[test]
fn an_extra_ventricular_beat_restarts_nothing() {
    let mut sc = BASE;
    sc.extras[0] = Extra { chamber: 1, at_ms: 700 };
    let (a, v) = play(&sc, 2_000, &[]);
    assert_eq!(a, [200, 1_000, 1_800]);
    assert_eq!(v, [360, 700, 1_160, 1_960]);
}

/// A conducted beat restarts the ventricle's own rhythm: an escape
/// rhythm slower than the conducted one never shows.
#[test]
fn a_conducted_beat_restarts_the_escape_rhythm() {
    let sc = Scenario { ventricular: Rhythm { first_ms: 700, interval_ms: 1_200 }, ..BASE };
    let (_, v) = play(&sc, 3_000, &[]);
    assert_eq!(v, [360, 1_160, 1_960, 2_760]);
}

#[test]
fn a_non_responsive_heart_ignores_paces() {
    assert_eq!(play(&BASE, 3_000, &[(500, 0), (600, 1), (900, 1)]), play(&BASE, 3_000, &[]));
}

#[test]
fn a_responsive_heart_is_captured_by_a_pace() {
    // A slow heart: the atrium every 1,500 ms, conduction 180 ms.
    let slow = Scenario { atrial: Rhythm { first_ms: 300, interval_ms: 1_500 }, conduction_ms: 180, responsive: true, ..BASE };
    // An atrial Pace at 1,300 restarts the atrial rhythm and conducts.
    let (a, v) = play(&slow, 3_500, &[(1_300, 0)]);
    assert_eq!(a, [300, 2_800]);
    assert_eq!(v, [480, 1_480, 2_980]);
    // A ventricular Pace before the conducted beat takes its place.
    let (a, v) = play(&slow, 2_000, &[(400, 1)]);
    assert_eq!(a, [300, 1_800]);
    assert_eq!(v, [1_980]);
    // A ventricular Pace restarts the ventricle's own rhythm.
    let block = Scenario { ventricular: Rhythm { first_ms: 500, interval_ms: 1_500 }, conduction_ms: 0, responsive: true, ..BASE };
    let (_, v) = play(&block, 3_999, &[(1_000, 1)]);
    assert_eq!(v, [500, 2_500]);
}

#[test]
fn the_counter_s_wrap_keeps_the_rhythm() {
    let mut h = Heart::default();
    h.tick(&BASE, false, false);
    // Move the state to just before the wrap, keeping its phase.
    h.ms = u32::MAX - 300;
    h.next_a = u32::MAX - 100;
    let mut a = Vec::new();
    for _ in 0..2_000 {
        if h.tick(&BASE, false, false).atrial {
            a.push(h.ms);
        }
    }
    assert_eq!(a.len(), 3);
    assert_eq!(a[1].wrapping_sub(a[0]), 800);
    assert_eq!(a[2].wrapping_sub(a[1]), 800);
}

#[test]
fn the_names_are_distinct_and_the_default_is_fixed() {
    for (i, a) in ALL.iter().enumerate() {
        for b in &ALL[..i] {
            assert_ne!(a.name, b.name);
        }
    }
    #[cfg(not(any(feature = "scenario-silent", feature = "scenario-fast", feature = "scenario-responsive")))]
    assert_eq!(SELECTED.name, "fixed");
}

/// A conducted beat arrives before the next atrial beat in every
/// scenario, so the conduction field is never overwritten unplayed.
#[test]
fn conduction_is_shorter_than_every_atrial_interval() {
    for s in ALL {
        if s.atrial.interval_ms != 0 && s.conduction_ms != 0 {
            let shortest = s.overrides.iter().filter(|o| o.chamber == 0 && o.beat != u32::MAX).map(|o| s.atrial.interval_ms as i64 + o.shift_ms as i64).min().unwrap_or(s.atrial.interval_ms as i64);
            assert!((s.conduction_ms as i64) < shortest.min(s.atrial.interval_ms as i64), "{}", s.name);
        }
    }
}
