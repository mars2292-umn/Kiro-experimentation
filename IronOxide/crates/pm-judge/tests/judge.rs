//! The judge on synthetic captures and on a recorded one, including
//! captures that must fail (docs/pacemaker task 4.3).
#![forbid(unsafe_code)]

use heart_sim_part::scenario;
use pm_core::{validate, word, Chamber, Params, NOMINAL_WORDS};
use pm_judge::{decode, judge, play, Edges, Report, PACE_A, PACE_V, SENSE_A, SENSE_V};

const RATE: u64 = 1_000_000;

/// A capture at 1 MHz of `ms` milliseconds with the given pulses:
/// (channel, start in µs, width in µs).
fn capture(ms: u64, pulses: &[(u8, u64, u64)]) -> Edges {
    let mut data = vec![0u8; (ms * 1_000) as usize];
    for (channel, at, width) in pulses {
        for b in &mut data[*at as usize..(*at + *width) as usize] {
            *b |= 1 << channel;
        }
    }
    Edges::from_raw(&data, RATE)
}

fn params(mode: u32, set: &[(usize, u32)]) -> Params {
    let mut w = NOMINAL_WORDS;
    w[word::MODE] = mode;
    for (i, v) in set {
        w[*i] = *v;
    }
    validate(&w).expect("a set of stage E1")
}

const OFF: u32 = 0;
const AOO: u32 = 5;
const AAI: u32 = 6;
const VOO: u32 = 7;
const VVI: u32 = 8;

fn run(e: &Edges, p: &Params) -> Report {
    judge(&decode(e), p, false).unwrap()
}

fn criteria(r: &Report) -> Vec<&'static str> {
    r.violations.iter().map(|v| v.criterion).collect()
}

/// Paces of the ventricle every `interval` µs from `first`, `width` wide.
fn train(channel: u8, first: u64, interval: u64, width: u64, n: u64) -> Vec<(u8, u64, u64)> {
    (0..n).map(|k| (channel, first + k * interval, width)).collect()
}

#[test]
fn decoding_finds_every_complete_pulse_and_no_cut_one() {
    let mut data = vec![0u8; 10_000];
    data[..100].fill(1 << PACE_V); // cut by the beginning
    data[1_000..1_400].fill(1 << PACE_V);
    data[2_000..2_050].fill((1 << PACE_A) | (1 << SENSE_V));
    data[9_900..].fill(1 << PACE_A); // cut by the end
    let e = Edges::from_raw(&data, RATE);
    assert_eq!(e.pulses(PACE_V), [pm_judge::Pulse { at_us: 1_000.0, width_us: 400.0 }]);
    assert_eq!(e.pulses(PACE_A), [pm_judge::Pulse { at_us: 2_000.0, width_us: 50.0 }]);
    assert_eq!(e.pulses(SENSE_V).len(), 1);
    assert_eq!(e.pulses(SENSE_A).len(), 0);
    // The text form is the same capture.
    assert_eq!(Edges::from_text(&e.to_text()).unwrap(), e);
    assert!(Edges::from_text("1000000 10\n5 01\n").is_err());
    assert!(Edges::from_text("1000000 10\n0 00\n12 01\n").is_err());
}

#[test]
fn voo_at_the_lrl_passes_and_every_interval_and_width_is_measured() {
    let e = capture(10_200, &train(PACE_V, 300_000, 1_000_000, 400, 10));
    let r = run(&e, &params(VOO, &[]));
    assert!(r.pass(), "{:?}", r.violations);
    let v = &r.chambers[Chamber::V.index()];
    assert_eq!((v.paces, v.intervals_us.len(), v.widths_us.len()), (10, 9, 10));
    assert!(v.intervals_us.iter().all(|x| *x == 1_000_000.0));
    assert!(v.widths_us.iter().all(|x| *x == 400.0));
    // Senses do not matter in VOO, in either Chamber.
    let mut pulses = train(PACE_V, 300_000, 1_000_000, 400, 10);
    pulses.extend([(SENSE_V, 800_000, 100), (SENSE_A, 4_444_000, 100)]);
    assert!(run(&capture(10_200, &pulses), &params(VOO, &[])).pass());
}

#[test]
fn an_interval_outside_pm_par_01_fails_in_either_direction() {
    for (shift, expected) in [(7_900i64, true), (-7_900, true), (8_100, false), (-8_100, false)] {
        let mut pulses = train(PACE_V, 300_000, 1_000_000, 400, 4);
        // The fifth Pace is displaced, and the sixth follows it by the LRL interval.
        let fifth = (4_300_000 + shift) as u64;
        pulses.extend([(PACE_V, fifth, 400), (PACE_V, fifth + 1_000_000, 400)]);
        let r = run(&capture(6_000, &pulses), &params(VOO, &[]));
        assert_eq!(r.pass(), expected, "shift {shift}: {:?}", r.violations);
        if !expected {
            assert_eq!(criteria(&r), ["PM9.1"]);
        }
    }
}

#[test]
fn a_missing_pace_fails_also_at_the_capture_s_end() {
    // The fourth Pace is missing.
    let mut pulses = train(PACE_V, 300_000, 1_000_000, 400, 3);
    pulses.push((PACE_V, 4_300_000, 400));
    let r = run(&capture(5_000, &pulses), &params(VOO, &[]));
    assert_eq!(criteria(&r), ["PM9.1"]);
    // Pacing stops: only the capture's end shows it.
    let r = run(&capture(6_000, &train(PACE_V, 300_000, 1_000_000, 400, 3)), &params(VOO, &[]));
    assert_eq!(criteria(&r), ["PM9.1"]);
    // Nothing at all.
    let r = run(&capture(3_000, &[]), &params(VOO, &[]));
    assert_eq!(criteria(&r), ["PM9.1"]);
    assert!(run(&capture(3_000, &[]), &params(OFF, &[])).pass());
}

#[test]
fn a_pulse_width_outside_pm_par_05_fails() {
    let mut pulses = train(PACE_V, 300_000, 1_000_000, 400, 5);
    pulses[2].2 = 590;
    assert!(run(&capture(5_200, &pulses), &params(VOO, &[])).pass());
    pulses[2].2 = 610;
    assert_eq!(criteria(&run(&capture(5_200, &pulses), &params(VOO, &[]))), ["PM3.3"]);
    // The Power-On Reset and Pace-Now pulse: ±0.02 ms.
    let por = params(VVI, &[(word::LRL, 60), (word::V_WIDTH, 500)]);
    let mut pulses = train(PACE_V, 300_000, 1_000_000, 500, 5);
    pulses[1].2 = 519;
    assert!(judge(&decode(&capture(5_200, &pulses)), &por, true).unwrap().pass());
    pulses[1].2 = 521;
    assert_eq!(criteria(&judge(&decode(&capture(5_200, &pulses)), &por, true).unwrap()), ["PM3.3"]);
    assert!(judge(&decode(&capture(5_200, &pulses)), &por, false).unwrap().pass());
}

#[test]
fn a_pace_in_a_chamber_the_mode_does_not_pace_fails() {
    let mut pulses = train(PACE_V, 300_000, 1_000_000, 400, 5);
    pulses.push((PACE_A, 2_000_000, 400));
    assert_eq!(criteria(&run(&capture(5_200, &pulses), &params(VOO, &[]))), ["PM5.7"]);
    // The same capture is wrong for AOO in both Chambers.
    let r = run(&capture(5_200, &pulses), &params(AOO, &[]));
    assert!(criteria(&r).contains(&"PM5.7") && criteria(&r).contains(&"PM9.1"));
    assert_eq!(criteria(&run(&capture(1_000, &[(PACE_V, 500_000, 400)]), &params(OFF, &[]))), ["PM5.7"]);
}

/// VVI, LRL 60 ppm, VRP 320 ms: Paces at 0.3 s and 1.3 s, a Sense at
/// 1.9 s (outside the VRP: the next Pace is due at 2.9 s), a Sense at
/// 2.0 s (inside the VRP of the first: ignored), the Pace at 2.9 s.
fn vvi_pulses() -> Vec<(u8, u64, u64)> {
    vec![(PACE_V, 300_000, 400), (PACE_V, 1_300_000, 400), (SENSE_V, 1_900_000, 100), (SENSE_V, 2_000_000, 100), (PACE_V, 2_900_000, 400), (PACE_V, 3_900_000, 400)]
}

#[test]
fn vvi_inhibition_and_the_refractory_period_pass() {
    let r = run(&capture(4_500, &vvi_pulses()), &params(VVI, &[]));
    assert!(r.pass(), "{:?}", r.violations);
    let v = &r.chambers[Chamber::V.index()];
    assert_eq!((v.paces, v.senses, v.effective_senses, v.refractory_senses, v.boundary_senses, v.fusions), (4, 2, 1, 1, 0, 0));
    assert_eq!(v.intervals_us, [1_000_000.0, 1_000_000.0, 1_000_000.0]);
}

#[test]
fn a_pace_that_ignores_an_effective_sense_fails() {
    // The PG paces at 2.3 s as if the Sense at 1.9 s had not happened.
    let mut pulses = vvi_pulses();
    pulses[4].1 = 2_300_000;
    pulses[5].1 = 3_300_000;
    let r = run(&capture(4_000, &pulses), &params(VVI, &[]));
    assert_eq!(criteria(&r), ["PM5.5"], "{:?}", r.violations);
}

#[test]
fn a_pace_restarted_by_a_refractory_sense_fails() {
    // Pace at 0.3 s; a Sense at 0.5 s (inside the VRP) wrongly restarts the
    // interval: the next Pace comes at 1.5 s, not 1.3 s.
    let pulses = [(PACE_V, 300_000, 400), (SENSE_V, 500_000, 100), (PACE_V, 1_500_000, 400), (PACE_V, 2_500_000, 400)];
    let r = run(&capture(3_000, &pulses), &params(VVI, &[]));
    assert_eq!(criteria(&r), ["PM9.1"], "{:?}", r.violations);
}

#[test]
fn a_sense_at_the_end_of_the_refractory_period_is_accepted_either_way() {
    // 3 ms before the VRP's end (320 ms): within PM-PAR-01 of it.
    for next in [1_300_000u64, 1_617_000] {
        let pulses = [(PACE_V, 300_000, 400), (SENSE_V, 617_000, 100), (PACE_V, next, 400), (PACE_V, next + 1_000_000, 400)];
        let r = run(&capture(3_000, &pulses), &params(VVI, &[]));
        assert!(r.pass(), "next {next}: {:?}", r.violations);
        assert_eq!(r.chambers[1].boundary_senses, 1);
    }
    // 10 ms before its end the Sense must be ignored, 10 ms after it must count.
    let early = [(PACE_V, 300_000, 400), (SENSE_V, 610_000, 100), (PACE_V, 1_610_000, 400)];
    assert!(!run(&capture(2_000, &early), &params(VVI, &[])).pass());
    let late = [(PACE_V, 300_000, 400), (SENSE_V, 630_000, 100), (PACE_V, 1_300_000, 400)];
    assert_eq!(criteria(&run(&capture(2_000, &late), &params(VVI, &[]))), ["PM5.5"]);
}

#[test]
fn a_fusion_inside_the_inhibition_window_is_counted_and_outside_it_fails() {
    // A Sense 1.5 ms before the Pace due at 1.3 s: both occur, and the next
    // interval starts at the Pace (PD-04).
    let pulses = [(PACE_V, 300_000, 400), (SENSE_V, 1_298_500, 100), (PACE_V, 1_300_000, 400), (PACE_V, 2_300_000, 400)];
    let r = run(&capture(3_000, &pulses), &params(VVI, &[]));
    assert!(r.pass(), "{:?}", r.violations);
    assert_eq!(r.chambers[1].fusions, 1);
    // 3 ms before: the Pace should have been cancelled.
    let pulses = [(PACE_V, 300_000, 400), (SENSE_V, 1_297_000, 100), (PACE_V, 1_300_000, 400), (PACE_V, 2_300_000, 400)];
    let r = run(&capture(3_000, &pulses), &params(VVI, &[]));
    assert_eq!(criteria(&r), ["PM5.5"], "{:?}", r.violations);
}

#[test]
fn aai_is_judged_in_the_atrium_with_the_arp() {
    // ARP 250 ms: a Sense 0.3 s after the Pace counts; in VVI's 320 ms it would not.
    let pulses = [(PACE_A, 300_000, 400), (SENSE_A, 600_000, 100), (PACE_A, 1_600_000, 400), (PACE_A, 2_600_000, 400), (SENSE_V, 700_000, 100)];
    let r = run(&capture(3_000, &pulses), &params(AAI, &[]));
    assert!(r.pass(), "{:?}", r.violations);
    assert_eq!(r.chambers[0].effective_senses, 1);
    assert_eq!(r.av_delays_us.len(), 1);
}

#[test]
fn a_dual_chamber_mode_is_refused_not_passed() {
    let mut p = params(VVI, &[]);
    p.mode = pm_core::Mode::DDD;
    assert!(judge(&decode(&capture(1_000, &[])), &p, false).is_err());
}

/// The recorded capture: the Heart_Simulator playing `fixed` on board B,
/// the PG silent (20 s at 1 MHz, 2026-11-22; `fixtures/heart_fixed.edges`).
fn recorded() -> Edges {
    Edges::from_text(include_str!("../fixtures/heart_fixed.edges")).unwrap()
}

#[test]
fn the_recorded_heart_plays_its_scenario_and_no_other() {
    let d = decode(&recorded());
    let played = play(&d, &scenario::FIXED).unwrap();
    assert!(played.mismatches.is_empty(), "{:?}", played.mismatches);
    assert!(played.atrial_beats >= 24 && played.ventricular_beats >= 24);
    assert!(played.atrial_intervals_us.iter().all(|x| (x - 800_000.0).abs() < 2_000.0));
    assert!(played.conduction_us.iter().all(|x| (x - 160_000.0).abs() < 2_000.0));
    // Against the other scenarios' tables the same capture does not fit.
    assert!(!play(&d, &scenario::FAST).unwrap().mismatches.is_empty());
    assert!(!play(&d, &scenario::SILENT).unwrap().mismatches.is_empty());
    assert!(play(&d, &scenario::RESPONSIVE).is_err());
}

#[test]
fn the_recorded_capture_is_judged_with_no_pg_pacing() {
    let e = recorded();
    // The heart beats 75 a minute and nothing paces: right for Off, and for
    // VVI and AAI at 60 ppm (every Pace inhibited); wrong at 90 ppm, where
    // a Pace is due 667 ms after each Sense, and wrong for VOO.
    assert!(run(&e, &params(OFF, &[])).pass());
    let r = run(&e, &params(VVI, &[]));
    assert!(r.pass(), "{:?}", r.violations);
    assert!(r.chambers[1].effective_senses >= 23 && r.chambers[1].paces == 0);
    assert!(run(&e, &params(AAI, &[])).pass());
    let fast = run(&e, &params(VVI, &[(word::LRL, 90)]));
    assert!(fast.violations.len() >= 20 && criteria(&fast).iter().all(|c| *c == "PM9.1"), "{:?}", fast.violations.len());
    assert!(!run(&e, &params(VOO, &[])).pass());
    // The AV delays the capture shows are the scenario's conduction.
    assert!(r.av_delays_us.iter().all(|x| (x - 160_000.0).abs() < 2_000.0));
}
