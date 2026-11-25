//! Tasks 22.4 and 22.6: the records of `diag` as `comms` admits them, and
//! the settings of the threshold test.
#![forbid(unsafe_code)]

use pm_core::record::{admitted, head, parts, DATA_WORDS};
use pm_core::threshold::{lower_amplitude, lower_width, step_down, what};
use pm_core::{validate, Params, NOMINAL_WORDS};

/// `comms`'s validation function: a kind of record and a part the kind
/// has, nothing above them, whatever the data.
#[test]
fn a_record_is_admitted_exactly_for_a_kind_and_a_part_it_has() {
    let mut admitted_heads = 0;
    for kind in 0..=255u8 {
        for part in 0..=255u8 {
            let mut m = [0xFFFF_FFFFu32; DATA_WORDS];
            m[0] = head(kind, part);
            let want = match kind {
                0x10..=0x13 => part < 4,
                0x20 | 0x50 | 0x60 => part < 2,
                0x30 => part < 65,
                0x40 => part == 0,
                _ => false,
            };
            assert_eq!(admitted(&m), want, "kind {kind:#x} part {part}");
            assert_eq!((part as u32) < parts(kind), want);
            admitted_heads += want as u32;
            m[0] |= 0x1_0000;
            assert!(!admitted(&m), "a bit above the part");
        }
    }
    assert_eq!(admitted_heads, 4 * 4 + 3 * 2 + 65 + 1);
    assert!(!admitted(&[0; DATA_WORDS]), "all zero is no record");
}

/// PM17.4: one setting lower, through every setting of Table 19-1.
#[test]
fn the_threshold_test_steps_through_every_setting() {
    // Pulse width: 1.9 ms down to 0.1 ms by 0.1, then 0.05 ms.
    let mut w = 1_900;
    let mut seen = vec![w];
    while let Some(next) = lower_width(w) {
        assert!(next < w);
        w = next;
        seen.push(w);
    }
    let mut want: Vec<u32> = (1..=19).rev().map(|k| k * 100).collect();
    want.push(50);
    assert_eq!(seen, want);
    // Amplitude: 7.0 V down to 3.5 V by 0.5, then 3.2 V down to 0.5 V by 0.1.
    let mut a = 70u8;
    let mut seen = vec![a];
    while let Some(next) = lower_amplitude(a) {
        assert!(next < a);
        a = next;
        seen.push(a);
    }
    let mut want: Vec<u8> = (7..=14).rev().map(|k| k * 5).collect();
    want.extend((5..=32).rev());
    assert_eq!(seen, want);
    // Every setting the test reaches is one `validate` accepts.
    for (index, values) in [(10usize, (1..=19).map(|k| k * 100).chain([50]).collect::<Vec<u32>>()), (8, want.iter().map(|x| *x as u32).collect())] {
        for v in values {
            let mut words = NOMINAL_WORDS;
            words[index] = v;
            assert!(validate(&words).is_some(), "word {index} = {v}");
        }
    }
    // A value between two settings goes to the setting below it.
    assert_eq!((lower_width(450), lower_width(75), lower_width(50), lower_width(5_000)), (Some(400), Some(50), None, Some(1_900)));
    assert_eq!((lower_amplitude(37), lower_amplitude(33), lower_amplitude(5), lower_amplitude(0)), (Some(35), Some(32), None, None));
}

/// The set with one value lower and nothing else changed.
#[test]
fn a_step_changes_the_value_under_test_alone() {
    let p = Params::NOMINAL;
    for which in 0..4 {
        let n = step_down(&p, which).expect("the nominal values are not the lowest");
        let mut back = n;
        match which {
            what::ATRIAL_AMPLITUDE => {
                assert_eq!(n.amplitude, [32, 35]);
                back.amplitude = p.amplitude;
            }
            what::VENTRICULAR_AMPLITUDE => {
                assert_eq!(n.amplitude, [35, 32]);
                back.amplitude = p.amplitude;
            }
            what::ATRIAL_WIDTH => {
                assert_eq!(n.width, [300, 400]);
                back.width = p.width;
            }
            _ => {
                assert_eq!(n.width, [400, 300]);
                back.width = p.width;
            }
        }
        assert_eq!(back, p);
    }
    assert_eq!(step_down(&p, 4), None);
    let lowest = Params { width: [50, 50], amplitude: [5, 5], ..p };
    for which in 0..4 {
        assert_eq!(step_down(&lowest, which), None);
    }
}

/// Task 22.6: the temporary set is replaced only while temporary pacing is
/// in force, and the State does not change.
#[test]
fn a_temporary_set_is_stepped_only_in_the_temporary_state() {
    use pm_core::{step, Battery, Command, Event, Micros, State, Therapy};
    let p = Params::NOMINAL;
    let lower = step_down(&p, what::VENTRICULAR_WIDTH).unwrap();
    // In the Permanent State: without effect.
    let s = State { therapy: Therapy::Permanent, programmed: true, permanent: p, ..State::POWER_ON_RESET };
    let (t, _) = step(&p, &s, Event::Command(Command::StepTemporary(lower)));
    assert_eq!((t.therapy, t.params().width), (Therapy::Permanent, [400, 400]));
    assert_eq!(t.temporary, s.temporary);
    // Into the Temporary State by the State machine: the link up, the command, a ventricular event.
    let tick = |now: u64| Event::Tick { now: Micros(now), activity: 0, magnet: false, battery: Battery::Bol, link_up: true };
    let (s, _) = step(&p, &s, tick(0));
    let (s, _) = step(&p, &s, Event::Command(Command::StartTemporary(p)));
    let (s, _) = step(&p, &s, tick(20_000));
    let (s, _) = step(&p, &s, Event::Paced { chamber: pm_core::Chamber::V, at: Micros(1_000_000) });
    assert_eq!(s.therapy, Therapy::Temporary);
    let (t, a) = step(s.params(), &s, Event::Command(Command::StepTemporary(lower)));
    assert_eq!((t.therapy, t.params().width, t.temporary.width), (Therapy::Temporary, [400, 300], [400, 300]));
    assert_eq!(t.chamber[1].pending, s.chamber[1].pending, "nothing of the timing changes");
    assert_eq!((a.atrial, a.ventricular), (pm_core::Pace::Keep, pm_core::Pace::Keep));
    // The next ventricular Pace is armed with the new width.
    let (_, a) = step(t.params(), &t, Event::Paced { chamber: pm_core::Chamber::V, at: Micros(2_000_000) });
    assert!(matches!(a.ventricular, pm_core::Pace::At { width: 300, .. }), "{:?}", a.ventricular);
}
