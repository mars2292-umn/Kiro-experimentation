//! Host tests of the rate behaviour of stage E4 (tasks 19.1 to 19.4):
//! rate-adaptive pacing, hysteresis, Rate Smoothing, the Atrial
//! Tachycardia Response and the noise rule, on the step as the Partition
//! calls it and on the arithmetic of `pm_core::rate`.
#![forbid(unsafe_code)]

use pm_core::rate::{fallback_interval, interval_of, rate_of, sensor_next, smoothed, target, threshold, ATR_COUNT, CONTROL_PERIOD_US, NOISE_INTERVAL_US};
use pm_core::*;

const A: Chamber = Chamber::A;
const V: Chamber = Chamber::V;

fn set(mode: Mode, ppm: u32) -> Params {
    Params { mode, lrl: interval_of_ppm(ppm), ..Params::NOMINAL }
}

fn tick(now: u64, activity: u8) -> Event {
    Event::Tick { now: Micros(now), activity, magnet: false, battery: Battery::Bol, link_up: false }
}

fn sense(c: Chamber, at: u64, edges: u32) -> Event {
    Event::Sense { chamber: c, at: Micros(at), edges }
}

/// A heart and a pulse generator for `seconds`: a tick every 20 ms with the
/// activity `activity(t)` gives; each pending Pace delivered at its instant
/// and reported; the Senses of `senses` (Chamber, instant), which must be
/// in order. Returns the state at the end and every Pace (Chamber,
/// instant) and every Marker of a Sense (Chamber, Marker, instant).
struct Run {
    state: State,
    paces: Vec<(Chamber, u64)>,
    markers: Vec<(Chamber, Marker, u64)>,
    /// (instant, Mode in force, ATR phase) at every tick.
    trace: Vec<(u64, Mode, AtrPhase)>,
}

fn run(p: Params, seconds: u64, activity: impl Fn(u64) -> u8, senses: &[(Chamber, u64)]) -> Run {
    let mut s = State { therapy: Therapy::Permanent, programmed: true, permanent: p, ..State::POWER_ON_RESET };
    let mut r = Run { state: s, paces: Vec::new(), markers: Vec::new(), trace: Vec::new() };
    let mut next_sense = 0;
    let mut next_tick = 0u64;
    let end = seconds * 1_000_000;
    loop {
        // The earliest of: the next tick, the next Sense, a pending Pace.
        let mut t = next_tick;
        let mut what = 0;
        if next_sense < senses.len() && senses[next_sense].1 < t {
            t = senses[next_sense].1;
            what = 1;
        }
        for (k, c) in [A, V].into_iter().enumerate() {
            let pending = s.chamber[k].pending;
            // A Pace due at an instant is delivered before a tick or a
            // Sense at that instant.
            if pending.armed && !s.chamber[k].unconfirmed && pending.at.0 <= t {
                t = pending.at.0;
                what = 2 + k;
                let _ = c;
            }
        }
        if t >= end {
            break;
        }
        let params = *s.params();
        let (after, arming) = match what {
            0 => {
                next_tick += u64::from(CONTROL_PERIOD_US);
                r.trace.push((t, params.mode, s.atr.phase));
                step(&params, &s, tick(t, activity(t)))
            }
            1 => {
                let c = senses[next_sense].0;
                next_sense += 1;
                step(&params, &s, sense(c, t, 1))
            }
            k => {
                let c = if k == 2 { A } else { V };
                r.paces.push((c, t));
                step(&params, &s, Event::Paced { chamber: c, at: Micros(t) })
            }
        };
        for (c, m) in [(A, arming.markers.atrial), (V, arming.markers.ventricular)] {
            if let Some((m @ (Marker::Sense { .. } | Marker::Noise), at)) = m {
                r.markers.push((c, m, at.0));
            }
        }
        s = after;
    }
    r.state = s;
    r
}

fn intervals(r: &Run, c: Chamber) -> Vec<u64> {
    let at: Vec<u64> = r.paces.iter().filter(|p| p.0 == c).map(|p| p.1).collect();
    at.windows(2).map(|w| w[1] - w[0]).collect()
}

// ------------------------------------------------------------ task 19.1

#[test]
fn a_rate_and_its_interval_convert_into_each_other() {
    for ppm in [30u32, 50, 60, 65, 90, 120, 175] {
        let i = interval_of_ppm(ppm);
        // The interval of a rate is rounded down to a microsecond: the rate
        // of that interval is the rate within a part in a million.
        assert!((rate_of(i) as i64 - (ppm as i64) * 1_000_000).abs() <= ppm as i64 * 2, "{ppm} ppm: {}", rate_of(i));
        assert!((interval_of(rate_of(i)) as i64 - i as i64).abs() <= 1, "{ppm} ppm");
    }
    assert_eq!((rate_of(0), interval_of(0)), (0, u32::MAX));
}

/// PM15.3, PM15.4: nothing at or below the threshold; the LRL plus
/// (MSR − LRL) × e × f / 16 above it; the seven thresholds evenly spaced.
#[test]
fn the_target_rate_follows_the_activity_the_threshold_and_the_response_factor() {
    use ActivityThreshold::*;
    assert_eq!([VLow, Low, MedLow, Med, MedHigh, High, VHigh].map(threshold), [31, 63, 95, 127, 159, 191, 223]);
    let p = Params { msr: interval_of_ppm(120), response_factor: 16, activity_threshold: Med, ..set(Mode::VVI.rate_adaptive(), 60) };
    let (low, high) = (rate_of(p.lrl), rate_of(p.msr));
    for a in 0..=127u8 {
        assert_eq!(target(&p, a), low, "activity {a}");
    }
    assert_eq!(target(&p, 255), high);
    // Half way up the range above the threshold: half way from LRL to MSR.
    let mid = target(&p, 191);
    assert!((mid as i64 - (low as i64 + high as i64) / 2).abs() < 1_000_000 / 2, "{mid}");
    // Rising with the activity, and with the Response Factor.
    let mut last = low;
    for a in 128..=255u8 {
        let t = target(&p, a);
        assert!(t >= last && t <= high);
        last = t;
    }
    let mut last = low;
    for f in 1..=16u8 {
        let t = target(&Params { response_factor: f, ..p }, 255);
        assert!(t > last, "factor {f}");
        last = t;
    }
    assert_eq!(last, high, "the largest at 16");
    // At the smallest factor the full activity gives a sixteenth of the range.
    let t1 = target(&Params { response_factor: 1, ..p }, 255);
    assert!((t1 as i64 - low as i64 - (high as i64 - low as i64) / 16).abs() <= 1);
}

/// PM15.5, PM15.6: sustained activity takes the rate from the LRL to the
/// MSR in the Reaction Time, and its end takes it back in the Recovery
/// Time, one step in every period of `control`.
#[test]
fn the_rate_rises_in_the_reaction_time_and_falls_in_the_recovery_time() {
    for (reaction_s, recovery_min) in [(10u32, 2u32), (30, 5), (50, 16)] {
        let p = Params {
            msr: interval_of_ppm(120),
            response_factor: 16,
            reaction_time: reaction_s * 1_000_000,
            recovery_time: recovery_min * 60_000_000,
            ..set(Mode::VVI.rate_adaptive(), 60)
        };
        let (low, high) = (rate_of(p.lrl), rate_of(p.msr));
        let mut rate = low;
        let mut ticks = 0u64;
        while rate < high {
            let next = sensor_next(&p, rate, 255);
            assert!(next > rate && next <= high);
            rate = next;
            ticks += 1;
        }
        let took = ticks * u64::from(CONTROL_PERIOD_US);
        let want = u64::from(p.reaction_time);
        assert!(took >= want && took <= want + want / 1000 + u64::from(CONTROL_PERIOD_US), "reaction {reaction_s} s took {took} µs");
        let mut ticks = 0u64;
        while rate > low {
            let next = sensor_next(&p, rate, 0);
            assert!(next < rate && next >= low);
            rate = next;
            ticks += 1;
        }
        let took = ticks * u64::from(CONTROL_PERIOD_US);
        let want = u64::from(p.recovery_time);
        assert!(took >= want && took <= want + want / 1000 + u64::from(CONTROL_PERIOD_US), "recovery {recovery_min} min took {took} µs");
    }
}

/// PM15.2 as a property: whatever the set, the activity and the rate it
/// started from, the sensor rate is within the LRL and the MSR.
#[test]
fn the_sensor_rate_never_exceeds_the_msr() {
    let mut x = 0x9E37_79B9_7F4A_7C15u64;
    let mut next = move || {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        x
    };
    let thresholds = [ActivityThreshold::VLow, ActivityThreshold::Low, ActivityThreshold::MedLow, ActivityThreshold::Med, ActivityThreshold::MedHigh, ActivityThreshold::High, ActivityThreshold::VHigh];
    for _ in 0..2_000 {
        let lrl = 30 + (next() % 146) as u32;
        let msr = 50 + (next() % 126) as u32;
        let p = Params {
            msr: interval_of_ppm(msr),
            response_factor: 1 + (next() % 16) as u8,
            activity_threshold: thresholds[(next() % 7) as usize],
            reaction_time: (10 + (next() % 5) as u32 * 10) * 1_000_000,
            recovery_time: (2 + (next() % 15) as u32) * 60_000_000,
            ..set(Mode::VVI.rate_adaptive(), lrl)
        };
        let (low, high) = (rate_of(p.lrl), rate_of(p.msr));
        let mut rate = next() as u32;
        for _ in 0..200 {
            rate = sensor_next(&p, rate, (next() % 256) as u8);
            assert!(rate >= low && (rate <= high || high <= low), "lrl {lrl} msr {msr}: {rate}");
            assert!(target(&p, (next() % 256) as u8) <= high.max(low));
        }
    }
}

/// On the step: VVIR paces at the LRL at rest, at the MSR under sustained
/// activity and never faster; VVI with the same set ignores the activity.
#[test]
fn a_rate_adaptive_mode_paces_at_the_sensor_rate() {
    let p = Params { msr: interval_of_ppm(120), response_factor: 16, reaction_time: 10_000_000, recovery_time: 120_000_000, ..set(Mode::VVI.rate_adaptive(), 60) };
    let rest = run(p, 10, |_| 0, &[]);
    assert!(intervals(&rest, V).iter().all(|i| *i == 1_000_000), "{:?}", intervals(&rest, V));
    // Activity from the start: by 12 s the rate is at the MSR, 500 ms.
    let active = run(p, 30, |_| 255, &[]);
    let i = intervals(&active, V);
    assert!(i.iter().all(|i| *i >= 500_000 && *i <= 1_000_000), "{i:?}");
    assert!(i.windows(2).all(|w| w[1] <= w[0]), "the interval only shortens: {i:?}");
    assert!(i[i.len() - 10..].iter().all(|i| *i == 500_000), "{i:?}");
    let at_12s = active.paces.iter().position(|p| p.1 > 12_000_000).unwrap();
    assert_eq!(i[at_12s], 500_000);
    // The activity ends at 30 s: back at the LRL two minutes later.
    let back = run(p, 170, |t| if t < 30_000_000 { 255 } else { 0 }, &[]);
    let i = intervals(&back, V);
    assert_eq!(*i.last().unwrap(), 1_000_000);
    let at_100s = back.paces.iter().position(|p| p.1 > 100_000_000).unwrap();
    assert!(i[at_100s] > 600_000 && i[at_100s] < 1_000_000, "{}", i[at_100s]);
    // Not rate-adaptive: the LRL whatever the activity.
    let fixed = run(Params { mode: Mode::VVI, ..p }, 10, |_| 255, &[]);
    assert!(intervals(&fixed, V).iter().all(|i| *i == 1_000_000));
    assert!(!fixed.state.sensor_known);
}

// ------------------------------------------------------------ task 19.2

/// PM16.1, PM16.2: after a Sense in the Chamber that starts the LRL
/// interval the escape interval is the HRL's; a Pace ends hysteresis.
#[test]
fn hysteresis_waits_the_hrl_interval_after_a_sense_and_ends_at_a_pace() {
    for (mode, c) in [(Mode::VVI, V), (Mode::AAI, A), (Mode::DDD, V)] {
        let p = Params { hrl: interval_of_ppm(50), ..set(mode, 60) };
        let r = run(p, 6, |_| 0, &[(c, 1_700_000)]);
        let at: Vec<u64> = r.paces.iter().filter(|x| x.0 == c).map(|x| x.1).collect();
        // Paces at 1.0 s; the Sense at 1.7 s; then 1.2 s to the next Pace,
        // and 1.0 s from that Pace on.
        assert_eq!(at[..4], [1_000_000, 2_900_000, 3_900_000, 4_900_000], "{mode:?}");
        // Off: the LRL interval after the Sense too.
        let r = run(set(mode, 60), 6, |_| 0, &[(c, 1_700_000)]);
        let at: Vec<u64> = r.paces.iter().filter(|x| x.0 == c).map(|x| x.1).collect();
        assert_eq!(at[..3], [1_000_000, 2_700_000, 3_700_000], "{mode:?} without hysteresis");
    }
    // A Mode that does not respond to a Sense has no hysteresis.
    let r = run(Params { hrl: interval_of_ppm(50), ..set(Mode::VOO, 60) }, 4, |_| 0, &[(V, 1_700_000)]);
    assert!(intervals(&r, V).iter().all(|i| *i == 1_000_000));
}

#[test]
fn smoothing_keeps_an_interval_within_its_percentage_of_the_cycle_before() {
    assert_eq!(smoothed(1_000_000, 500_000, 0, 0), 1_000_000, "off");
    assert_eq!(smoothed(1_000_000, 500_000, 6, 6), 530_000);
    assert_eq!(smoothed(400_000, 500_000, 6, 6), 470_000);
    assert_eq!(smoothed(510_000, 500_000, 6, 6), 510_000);
    assert_eq!(smoothed(1_000_000, 500_000, 25, 25), 625_000);
    assert_eq!(smoothed(u32::MAX, u32::MAX, 25, 25), u32::MAX);
    // Up and Down are two parameters (PM16.3): each holds its own side.
    assert_eq!(smoothed(1_000_000, 500_000, 6, 0), 1_000_000, "Down off: as long as asked");
    assert_eq!(smoothed(400_000, 500_000, 6, 0), 470_000);
    assert_eq!(smoothed(400_000, 500_000, 0, 6), 400_000, "Up off: as short as asked");
    assert_eq!(smoothed(1_000_000, 500_000, 0, 6), 530_000);
    assert_eq!(smoothed(1_000_000, 500_000, 3, 21), 605_000);
    assert_eq!(smoothed(400_000, 500_000, 3, 21), 485_000);
}

/// PM16.3 as a property on the step: after an intrinsic rhythm stops, the
/// paced intervals lengthen to the LRL interval by no more than the
/// percentage each, and none is outside the LRL and the URL.
#[test]
fn rate_smoothing_limits_the_change_from_one_cycle_to_the_next() {
    for percent in [3u8, 6, 9, 12, 15, 18, 21, 25] {
        for beat in [520_000u64, 600_000, 750_000] {
            let p = Params { rate_smoothing_up: percent, rate_smoothing_down: percent, url: interval_of_ppm(120), ..set(Mode::VVI, 60) };
            // Six intrinsic beats, then silence.
            let senses: Vec<(Chamber, u64)> = (1..=6).map(|k| (V, k * beat)).collect();
            let r = run(p, 30, |_| 0, &senses);
            let mut events: Vec<u64> = senses.iter().map(|s| s.1).chain(r.paces.iter().map(|p| p.1)).collect();
            events.sort_unstable();
            let cycles: Vec<u64> = events.windows(2).map(|w| w[1] - w[0]).collect();
            let from = senses.len() - 1;
            for w in cycles[from - 1..].windows(2) {
                let (before, this) = (w[0], w[1]);
                let band = before * u64::from(percent) / 100;
                assert!(this <= before + band && this + band >= before, "{percent} % after {beat}: {before} then {this}");
                assert!((500_000..=1_000_000).contains(&this), "{percent} %: {this}");
            }
            assert!(cycles[from] > beat, "the first paced cycle is longer than the rhythm's");
            assert_eq!(*cycles.last().unwrap(), 1_000_000, "{percent} % after {beat}: back at the LRL");
            // Without Rate Smoothing the first paced cycle is the LRL's.
            let r = run(Params { rate_smoothing_up: 0, rate_smoothing_down: 0, ..p }, 12, |_| 0, &senses);
            assert_eq!(r.paces[0].1, 6 * beat + 1_000_000);
        }
    }
}

// ------------------------------------------------------------ task 19.3

fn atr_set(duration: u16, fallback_minutes: u32) -> Params {
    Params { atr: true, atr_duration: duration, atr_fallback_time: fallback_minutes * 60_000_000, url: interval_of_ppm(120), pvarp: 250_000, ..set(Mode::DDD, 60) }
}

/// Atrial Senses every `interval` µs from `from` to `to`.
fn atrial(from: u64, to: u64, interval: u64) -> Vec<(Chamber, u64)> {
    (0..).map(|k| from + k * interval).take_while(|t| *t < to).map(|t| (A, t)).collect()
}

/// PM14.1, PM14.4, PM14.6, PM14.8: an atrial rhythm faster than the URL is
/// detected at the eighth short interval; ATR Duration counts ventricular
/// events; Fallback switches to VVIR at a ventricular event and lowers the
/// rate to the LRL over the Fallback Time.
#[test]
fn an_atrial_tachycardia_goes_through_duration_into_fallback() {
    let p = atr_set(10, 1);
    // 200 ppm in the atrium (300 ms) for 100 s.
    let r = run(p, 100, |_| 0, &atrial(1_000_000, 100_000_000, 300_000));
    let onset = r.trace.iter().find(|t| t.2 == AtrPhase::Duration).expect("onset").0;
    // The eighth short interval. The first is from the atrial Pace at
    // 0.85 s (one AV delay before the first escape interval's end) to the
    // first Sense at 1.0 s, so the eighth ends at 1.0 s + 7 × 0.3 s; the
    // trace is of the ticks, 20 ms apart.
    assert!((3_100_000..=3_120_000).contains(&onset), "{onset}");
    let fallback = r.trace.iter().find(|t| t.2 == AtrPhase::Fallback).expect("Fallback").0;
    // Ten ventricular events after the onset (tracking 2:1 at the URL or
    // close to it: some 5 to 6 s).
    let v_between = r.paces.iter().filter(|p| p.0 == V && p.1 > onset && p.1 <= fallback).count();
    assert_eq!(v_between, 10);
    // The Mode in force: DDD until the ventricular event at which Fallback
    // begins, VVIR from it (finding B41; until 2026-11-25 this test had the
    // Mode change one ventricular event later). The trace is of the ticks:
    // the first that has the phase has the Mode.
    let switched = r.trace.iter().find(|t| t.1 == Mode::VVI.rate_adaptive()).expect("VVIR").0;
    assert_eq!(switched, fallback);
    assert!(r.trace.iter().all(|t| t.1 == if t.0 < switched { Mode::DDD } else { Mode::VVI.rate_adaptive() }));
    assert!(r.paces.iter().any(|p| p.0 == V && p.1 + 20_000 >= switched && p.1 <= switched), "the switch is at a ventricular event");
    // In Fallback no atrial Pace and no tracking: the ventricular interval
    // lengthens from the tracked cycle to the LRL's over the Fallback Time.
    assert!(r.paces.iter().all(|p| p.0 == V || p.1 < switched));
    let after: Vec<u64> = r.paces.iter().filter(|p| p.0 == V && p.1 >= switched).map(|p| p.1).collect();
    let i: Vec<u64> = after.windows(2).map(|w| w[1] - w[0]).collect();
    assert!(i.windows(2).all(|w| w[1] >= w[0]), "the interval only lengthens: {i:?}");
    assert!(i[0] < 700_000, "it begins near the tracked rate: {}", i[0]);
    let late = after.iter().position(|t| *t > fallback + 61_000_000).expect("a Pace after the Fallback Time");
    assert!(i[late..].iter().all(|x| *x == 1_000_000), "at the LRL after the Fallback Time: {:?}", &i[late..]);
    let half = after.iter().position(|t| *t > fallback + 30_000_000).unwrap();
    assert!(i[half] > i[0] && i[half] < 1_000_000, "half way: {}", i[half]);
    assert_eq!(r.state.atr.counter, ATR_COUNT);
}

/// PM14.2, PM14.5: the tachycardia ends during ATR Duration: Duration ends
/// at once, there is no Fallback and the Mode stays.
#[test]
fn a_tachycardia_that_ends_in_duration_gives_no_fallback() {
    let p = atr_set(100, 1);
    let mut senses = atrial(1_000_000, 8_000_000, 300_000);
    senses.extend(atrial(8_800_000, 40_000_000, 800_000));
    let r = run(p, 40, |_| 0, &senses);
    let onset = r.trace.iter().find(|t| t.2 == AtrPhase::Duration).expect("onset").0;
    let ended = r.trace.iter().find(|t| t.0 > onset && t.2 == AtrPhase::Idle).expect("the end").0;
    // Eight slow intervals bring the counter from 8 to 0: the first is from
    // the last fast beat to 8.8 s, the eighth ends at 8.8 + 7 × 0.8 s.
    assert!((14_400_000..=14_420_000).contains(&ended), "{ended}");
    assert!(r.trace.iter().all(|t| t.2 != AtrPhase::Fallback && t.1 == Mode::DDD));
    assert_eq!(r.state.atr.counter, 0);
}

/// PM14.7, PM14.8: the tachycardia ends during Fallback: the programmed
/// Mode is back at a ventricular event.
#[test]
fn a_tachycardia_that_ends_in_fallback_returns_to_the_programmed_mode() {
    let p = atr_set(10, 5);
    let mut senses = atrial(1_000_000, 30_000_000, 300_000);
    senses.extend(atrial(30_800_000, 60_000_000, 800_000));
    let r = run(p, 60, |_| 0, &senses);
    let vvir = Mode::VVI.rate_adaptive();
    let switched = r.trace.iter().find(|t| t.1 == vvir).expect("VVIR").0;
    assert!(switched < 20_000_000);
    let back = r.trace.iter().find(|t| t.0 > switched && t.1 == Mode::DDD).expect("DDD again").0;
    // The counter is at zero 8 slow intervals after 30 s, at 36.4 s; the
    // phase ends at the next ventricular event and the Mode with it
    // (finding B41; the bound was two intervals until 2026-11-25).
    assert!(back > 36_400_000 && back < 36_400_000 + 1_020_000, "{back}");
    let idle = r.trace.iter().find(|t| t.0 > switched && t.2 == AtrPhase::Idle).expect("Idle").0;
    assert_eq!(back, idle, "the Mode is back in the tick that has the phase ended");
    assert!(r.trace.iter().filter(|t| t.0 > back).all(|t| t.1 == Mode::DDD && t.2 == AtrPhase::Idle));
    // Tracking again: a ventricular Pace one sensed AV delay after an atrial Sense.
    let last_a = senses.iter().rev().find(|s| s.1 < 59_000_000).unwrap().1;
    assert!(r.paces.iter().any(|p| p.0 == V && p.1 > last_a && p.1 <= last_a + 150_000));
}

/// PM14.3: occasional premature atrial beats do not start ATR, and ATR Off
/// or a Mode that does not track never does.
#[test]
fn premature_beats_do_not_start_atr() {
    // One premature beat in four: three long intervals for each short one.
    let mut senses = Vec::new();
    let mut t = 1_000_000u64;
    for k in 0..120 {
        senses.push((A, t));
        t += if k % 4 == 3 { 350_000 } else { 800_000 };
    }
    let r = run(atr_set(10, 1), 80, |_| 0, &senses);
    assert!(r.trace.iter().all(|x| x.2 == AtrPhase::Idle && x.1 == Mode::DDD));
    assert!(r.state.atr.counter <= 1);
    // A sustained tachycardia with ATR Off, and in DDI: no response.
    let fast = atrial(1_000_000, 30_000_000, 300_000);
    for p in [Params { atr: false, ..atr_set(10, 1) }, Params { mode: Mode::DDI, ..atr_set(10, 1) }] {
        let r = run(p, 30, |_| 0, &fast);
        assert!(r.trace.iter().all(|x| x.2 == AtrPhase::Idle && x.1 == p.mode), "{:?}", p.mode);
    }
}

#[test]
fn the_fallback_rate_falls_in_a_straight_line_to_the_lrl() {
    let p = atr_set(10, 1);
    assert_eq!(fallback_interval(&p, 500_000, 0), 500_000);
    assert_eq!(fallback_interval(&p, 500_000, 60_000_000), 1_000_000);
    assert_eq!(fallback_interval(&p, 500_000, 600_000_000), 1_000_000);
    // Half the time: half way in rate, 90 ppm.
    assert!((fallback_interval(&p, 500_000, 30_000_000) as i64 - 666_666).abs() <= 1);
    // A cycle already slower than the LRL: the LRL.
    assert_eq!(fallback_interval(&p, 1_200_000, 0), 1_000_000);
}

// ------------------------------------------------------------ task 19.4

fn programmed(p: Params) -> State {
    State { therapy: Therapy::Permanent, programmed: true, permanent: p, ..State::POWER_ON_RESET }
}

/// PM13.1, PM13.2: edges never further apart than PM-PAR-12 from the end of
/// the refractory period are noise: each is marked TN, none inhibits, and
/// the pending Pace stays where it is.
#[test]
fn continuous_noise_does_not_inhibit() {
    for (mode, c) in [(Mode::VVI, V), (Mode::AAI, A)] {
        let p = set(mode, 60);
        let (s, _) = step(&p, &programmed(p), tick(0, 0));
        let k = c.index();
        let pending = s.chamber[k].pending;
        assert_eq!(pending.at, Micros(1_000_000));
        let (mut s, _) = step(&p, &s, Event::Paced { chamber: c, at: Micros(1_000_000) });
        let pending = s.chamber[k].pending;
        assert_eq!(pending.at, Micros(2_000_000));
        // An edge every 50 ms from the Pace to the next one.
        let mut noise = 0;
        let mut refractory = 0;
        for n in 1..20u64 {
            let (t, a) = step(&p, &s, sense(c, 1_000_000 + n * 50_000, 1));
            assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep), "{mode:?} edge {n}");
            assert_eq!(t.chamber[k].pending, pending);
            match if c == A { a.markers.atrial } else { a.markers.ventricular } {
                Some((Marker::Noise, _)) => noise += 1,
                Some((Marker::Sense { refractory: true }, _)) => refractory += 1,
                other => panic!("{mode:?} edge {n}: {other:?}"),
            }
            s = t;
        }
        // The refractory period is 320 ms (VRP) or 250 ms (ARP) of the 950.
        assert_eq!((refractory, noise), if c == V { (6, 13) } else { (4, 15) }, "{mode:?}");
        // The Pace is delivered: the next cycle is noisy again until an
        // edge comes alone.
        let (s, a) = step(&p, &s, Event::Paced { chamber: c, at: Micros(2_000_000) });
        assert!(matches!(if c == A { a.atrial } else { a.ventricular }, Pace::At { at: Micros(3_000_000), .. }));
        let (s, a) = step(&p, &s, sense(c, 2_600_000, 1));
        assert!(matches!(if c == A { a.atrial } else { a.ventricular }, Pace::At { at: Micros(3_600_000), .. }), "{mode:?}: an edge 600 ms after the last is a Sense");
        assert_eq!(s.chamber[k].last, Micros(2_600_000));
    }
}

/// PM13.1 in the ventricular-based cycle (finding B42, from the rig's storm
/// on the atrial line): in DDD and VDD the atrium's cycle begins at the
/// ventricular event, and atrial edges never further apart than PM-PAR-12
/// from the end of the PVARP are noise: none is tracked, the pending Paces
/// stay where they are.
#[test]
fn continuous_atrial_noise_is_not_tracked() {
    for mode in [Mode::DDD, Mode::VDD, Mode::DDI] {
        let p = set(mode, 60);
        let (s, _) = step(&p, &programmed(p), tick(0, 0));
        let (mut s, _) = step(&p, &s, Event::Paced { chamber: V, at: Micros(1_000_000) });
        let pending = (s.chamber[0].pending, s.chamber[1].pending);
        assert_eq!(pending.1.at, Micros(2_000_000), "{mode:?}");
        // An atrial edge every 50 ms from the ventricular Pace until the
        // atrial Pace would be due (850 ms after it).
        let (mut noise, mut refractory) = (0, 0);
        for n in 1..17u64 {
            let (t, a) = step(&p, &s, sense(A, 1_000_000 + n * 50_000, 1));
            assert_eq!((a.atrial, a.ventricular), (Pace::Keep, Pace::Keep), "{mode:?} edge {n}");
            assert_eq!((t.chamber[0].pending, t.chamber[1].pending), pending, "{mode:?} edge {n}");
            match a.markers.atrial {
                Some((Marker::Noise, _)) => noise += 1,
                Some((Marker::Sense { refractory: true }, _)) => refractory += 1,
                other => panic!("{mode:?} edge {n}: {other:?}"),
            }
            s = t;
        }
        // The PVARP is 250 ms of the 800.
        assert_eq!((refractory, noise), (4, 12), "{mode:?}");
        // The next cycle: an atrial edge that comes alone, 600 ms after the
        // ventricular Pace, is a Sense, and DDD and VDD track it.
        let (s, _) = step(&p, &s, Event::Paced { chamber: V, at: Micros(2_000_000) });
        let (_, a) = step(&p, &s, sense(A, 2_600_000, 1));
        assert_eq!(a.markers.atrial, Some((Marker::Sense { refractory: false }, Micros(2_600_000))), "{mode:?}");
        if mode != Mode::DDI {
            assert!(matches!(a.ventricular, Pace::At { at: Micros(2_750_000), .. }), "{mode:?}: {:?}", a.ventricular);
        }
    }
}

/// The two sides of PM-PAR-12, by the spacing of the services and by the
/// count of edges in one service.
#[test]
fn the_noise_rule_on_both_sides_of_the_noise_interval() {
    let p = set(Mode::VVI, 60);
    let (s, _) = step(&p, &programmed(p), tick(0, 0));
    let (s, _) = step(&p, &s, Event::Paced { chamber: V, at: Micros(1_000_000) });
    // A refractory edge at 1.30 s (the VRP ends at 1.32 s).
    let (s, a) = step(&p, &s, sense(V, 1_300_000, 1));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: true }, Micros(1_300_000))));
    let gap = u64::from(NOISE_INTERVAL_US);
    // The next edge less than the interval after it: noise.
    let (_, a) = step(&p, &s, sense(V, 1_300_000 + gap - 1, 1));
    assert_eq!(a.markers.ventricular, Some((Marker::Noise, Micros(1_300_000 + gap - 1))));
    assert_eq!(a.ventricular, Pace::Keep);
    // Exactly the interval after it: a Sense, which inhibits.
    let (t, a) = step(&p, &s, sense(V, 1_300_000 + gap, 1));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: false }, Micros(1_300_000 + gap))));
    assert!(matches!(a.ventricular, Pace::At { .. }));
    assert!(t.chamber[1].noisy, "a new cycle begins noisy until an edge comes alone");
    // Far from any other edge, but two edges in the one service: noise.
    let (q, _) = step(&p, &programmed(p), tick(0, 0));
    let (q, _) = step(&p, &q, Event::Paced { chamber: V, at: Micros(1_000_000) });
    let (_, a) = step(&p, &q, sense(V, 1_700_000, 2));
    assert_eq!(a.markers.ventricular, Some((Marker::Noise, Micros(1_700_000))));
    let (_, a) = step(&p, &q, sense(V, 1_700_000, 1));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: false }, Micros(1_700_000))));
    // Once an edge has come alone the cycle is not noisy: a close edge
    // after it is in the new refractory period, a Refractory sense.
    let (u, _) = step(&p, &q, sense(V, 1_700_000, 1));
    let (_, a) = step(&p, &u, sense(V, 1_750_000, 3));
    assert_eq!(a.markers.ventricular, Some((Marker::Sense { refractory: true }, Micros(1_750_000))));
}

/// A Chamber the Mode does not sense has no noise: nothing is recorded.
#[test]
fn a_chamber_that_is_not_sensed_is_never_noisy() {
    let p = set(Mode::VOO, 60);
    let (s, _) = step(&p, &programmed(p), tick(0, 0));
    for n in 1..10u64 {
        let (t, a) = step(&p, &s, sense(V, n * 30_000, 5));
        assert_eq!(a, Arming::NONE);
        assert_eq!(t, s);
    }
}


// ------------------------------------------------------------ task 19.5

/// A set as the link brings it: words through `validate`.
fn programmed_words(edit: impl Fn(&mut [u32; WORDS])) -> Params {
    let mut w = NOMINAL_WORDS;
    edit(&mut w);
    validate(&w).expect("an admitted set")
}

fn code_of_mode(m: Mode) -> u32 {
    Mode::PROGRAMMABLE.iter().position(|x| *x == m).unwrap() as u32
}

/// A pulse generator that paces the ventricle alone, from the state `s`,
/// for `seconds` from `from`: a tick every 20 ms with the magnet as given,
/// each pending ventricular Pace delivered at its instant. Returns the
/// state and the instants of the Paces.
fn drive(mut s: State, from: u64, seconds: u64, magnet: bool) -> (State, Vec<u64>) {
    let mut paces = Vec::new();
    let mut t = from;
    while t < from + seconds * 1_000_000 {
        let pending = s.chamber[1].pending;
        if pending.armed && pending.at.0 <= t {
            paces.push(pending.at.0);
            s = step(&{ *s.params() }, &s, Event::Paced { chamber: V, at: pending.at }).0;
            continue;
        }
        let e = Event::Tick { now: Micros(t), activity: 0, magnet, battery: Battery::Bol, link_up: true };
        s = step(&{ *s.params() }, &s, e).0;
        t += u64::from(CONTROL_PERIOD_US);
    }
    (s, paces)
}

/// A rate-adaptive Mode with hysteresis, Rate Smoothing and ATR is a set
/// `validate` admits since task 19.5, and it paces at the sensor's rate.
#[test]
fn a_rate_adaptive_set_from_its_words_paces_at_the_sensor_rate() {
    let p = programmed_words(|w| {
        w[word::MODE] = code_of_mode(Mode::VVI.rate_adaptive());
        w[word::LRL] = 60;
        w[word::MSR] = 120;
        w[word::REACTION_TIME] = 10;
        w[word::RESPONSE_FACTOR] = 16;
        w[word::ACTIVITY_THRESHOLD] = 0;
    });
    assert!(p.mode.adaptive);
    let r = run(p, 30, |_| 255, &[]);
    let i = intervals(&r, V);
    assert_eq!(*i.last().unwrap(), 500_000, "the MSR interval");
    assert!(i[0] > 500_000 && i[0] <= 1_000_000, "rising from the LRL interval: {}", i[0]);
    assert!(i.windows(2).all(|w| w[1] <= w[0]), "never slower while the activity lasts");
    let p = programmed_words(|w| {
        w[word::MODE] = code_of_mode(Mode::DDD.rate_adaptive());
        w[word::HRL] = 50;
        w[word::RATE_SMOOTHING_UP] = 9;
        w[word::RATE_SMOOTHING_DOWN] = 12;
        w[word::ATR] = 1;
    });
    assert_eq!((p.hrl, p.rate_smoothing_up, p.rate_smoothing_down, p.atr), (1_200_000, 9, 12, true));
}

/// PM5.8: the Magnet State paces asynchronously at the magnet's rate at
/// once: the hysteresis and the Rate Smoothing of the programmed set are
/// not its Mode's (Table 19-3) and change nothing there.
#[test]
fn the_magnet_state_uses_neither_hysteresis_nor_rate_smoothing() {
    let p = programmed_words(|w| {
        w[word::MODE] = code_of_mode(Mode::VVI);
        w[word::LRL] = 60;
        w[word::HRL] = 50;
        w[word::RATE_SMOOTHING_UP] = 3;
        w[word::RATE_SMOOTHING_DOWN] = 3;
    });
    let s = step(&Params::POWER_ON_RESET, &State::POWER_ON_RESET, Event::Command(Command::Program(p))).0;
    let (s, _) = drive(s, 0, 4, false);
    let q = *s.params();
    assert_eq!((q.mode, q.hrl, q.rate_smoothing_up, q.rate_smoothing_down), (Mode::VVI, 1_200_000, 3, 3), "VVI uses all three");
    let (s, paces) = drive(s, 4_000_000, 6, true);
    let q = *s.params();
    assert_eq!((q.mode, q.lrl, q.hrl, q.rate_smoothing_up, q.rate_smoothing_down), (Mode::VOO, 600_000, 0, 0, 0));
    let i: Vec<u64> = paces.windows(2).map(|w| w[1] - w[0]).collect();
    // Without smoothing the first interval after the switch is the
    // magnet's; with 3 % it would take sixteen cycles to come down.
    let first_magnet = i.iter().position(|x| *x == 600_000).expect("the magnet's rate");
    assert!(first_magnet <= 2, "{i:?}");
    assert!(i[first_magnet..].iter().all(|x| *x == 600_000), "{i:?}");
}

/// PM5.8: hysteresis and Rate Smoothing programmed in a Mode that does not
/// use them (Table 19-3) are kept in the permanent set and are not in
/// force.
#[test]
fn a_parameter_the_mode_does_not_use_is_not_in_force() {
    for m in [Mode::VOO, Mode::DOO, Mode::DDI, Mode::VVT, Mode::VDD] {
        let p = programmed_words(|w| {
            w[word::MODE] = code_of_mode(m);
            w[word::LRL] = 60;
            w[word::HRL] = 40;
            w[word::RATE_SMOOTHING_UP] = 6;
            w[word::RATE_SMOOTHING_DOWN] = 6;
            w[word::ATR] = 1;
        });
        let s = step(&Params::POWER_ON_RESET, &State::POWER_ON_RESET, Event::Command(Command::Program(p))).0;
        let (s, _) = drive(s, 0, 4, false);
        let q = *s.params();
        assert_eq!(q.mode, m);
        assert_eq!(q.hrl, 0, "{m:?}");
        // Rate Smoothing is VDD's too; hysteresis is not. ATR is the tracking Modes'.
        let smoothing = if m == Mode::VDD { 6 } else { 0 };
        assert_eq!((q.rate_smoothing_up, q.rate_smoothing_down), (smoothing, smoothing), "{m:?}");
        assert_eq!(q.atr, m == Mode::VDD, "{m:?}");
        assert_eq!((s.permanent.hrl, s.permanent.rate_smoothing_up, s.permanent.atr), (1_500_000, 6, true), "kept as programmed");
    }
}

// ------------------------------------------------------------ task 22.2

/// PM18.5: the modifiers of a Pace say what made its escape interval what
/// it was: the HRL's after a Sense, the sensor's, or Rate Smoothing's Up
/// or Down; none for a Pace at the LRL.
#[test]
fn the_modifiers_of_a_pace() {
    use pm_core::escape_modifiers;
    use pm_core::record::modifier;
    let at = Micros(5_000_000);
    // A plain set: none, whatever the state.
    let p = set(Mode::VVI, 60);
    let mut s = programmed(p);
    assert_eq!(escape_modifiers(&p, &s, at), 0);
    s.hysteresis_active = true;
    s.sensor_known = true;
    s.sensor_interval = 500_000;
    s.cycle_known = true;
    s.last_cycle = 600_000;
    assert_eq!(escape_modifiers(&p, &s, at), 0, "no hysteresis, no sensor, no smoothing in the set");
    // Hysteresis: only while a Sense has begun it.
    let h = Params { hrl: interval_of_ppm(40), ..p };
    assert_eq!(escape_modifiers(&h, &s, at), modifier::HYSTERESIS);
    s.hysteresis_active = false;
    assert_eq!(escape_modifiers(&h, &s, at), 0);
    // The sensor: only when its interval is the shorter one.
    let r = Params { mode: Mode::VVI.rate_adaptive(), msr: interval_of_ppm(120), ..p };
    assert_eq!(escape_modifiers(&r, &s, at), modifier::SENSOR);
    s.sensor_interval = 1_000_000;
    assert_eq!(escape_modifiers(&r, &s, at), 0, "the sensor at the LRL");
    // Rate Smoothing Down: the LRL interval after a cycle of 600 ms is cut to 672 ms.
    let d = Params { rate_smoothing_down: 12, ..p };
    assert_eq!(escape_modifiers(&d, &s, at), modifier::SMOOTHING_DOWN);
    s.last_cycle = 950_000;
    assert_eq!(escape_modifiers(&d, &s, at), 0, "within 12 % of the cycle before");
    s.cycle_known = false;
    s.last_cycle = 600_000;
    assert_eq!(escape_modifiers(&d, &s, at), 0, "no cycle before");
    // Rate Smoothing Up under the sensor: 500 ms after a cycle of 1000 ms is held at 970 ms.
    s.cycle_known = true;
    s.last_cycle = 1_000_000;
    s.sensor_interval = 500_000;
    let u = Params { rate_smoothing_up: 3, ..r };
    assert_eq!(escape_modifiers(&u, &s, at), modifier::SENSOR + modifier::SMOOTHING_UP);
    // The step's own escape interval agrees: the Pace after the Sense is one HRL interval later.
    let (t, _) = step(&h, &programmed(h), tick(0, 0));
    let (t, _) = step(&h, &t, sense(V, 600_000, 1));
    assert_eq!(t.chamber[1].pending.at, Micros(2_100_000));
    assert_eq!(escape_modifiers(&h, &t, Micros(600_000)), modifier::HYSTERESIS);
}
