//! Task 8.3 (PM30.4, PM7.1): the Resource `timing` at start.
//!
//! That its initial value is all zero bytes is checked where it is
//! declared: `rsk::ResourceCell::for_static`, which the Generator emits for
//! every Partition static, reads every byte of `Timing::new()` at compile
//! time and refuses a non-zero one, and a type with a padding byte cannot
//! be read at all. So a build of the Partition is that proof, and it is
//! why every type inside `Timing` carries its spare bytes as a named field.
//! What is tested here is what that all-zero value means.
#![forbid(unsafe_code)]

use pacemaker_therapy::{kind, Timing, RING};
use pm_core::{Chamber, Mode, Params, State, Therapy};

#[test]
fn the_value_at_start_is_the_power_on_reset_state_with_nothing_armed() {
    let t = Timing::new();
    // `control`'s first Job is recognized by the state alone.
    assert!(!t.started && !t.hw_fault);
    assert_eq!(t.reserved, [0; 2]);
    // The step's state is the Power-On Reset State, and the parameters in
    // force are the Power-On Reset ones (PM7.1): VVI at 65 ppm, 0.5 ms.
    assert_eq!(t.state, State::POWER_ON_RESET);
    assert_eq!(t.state.therapy, Therapy::PowerOnReset);
    assert!(!t.state.programmed);
    let p = t.state.params();
    assert_eq!(*p, Params::POWER_ON_RESET);
    assert_eq!((p.mode, p.lrl, p.width[Chamber::V.index()], p.vrp), (Mode::VVI, 923_076, 500, 320_000));
    // Nothing is pending and the driver has nothing armed or forced.
    for c in t.state.chamber {
        assert!(!c.pending.armed && !c.seen);
    }
    assert_eq!((t.driver.armed, t.driver.forced, t.driver.counted), ([false; 2], [false; 2], [0; 2]));
    assert_eq!((t.driver.last_start[0].0, t.driver.last_start[1].0, t.driver.late_arms, t.driver.untimed), (0, 0, 0, 0));
    // The ring is empty.
    assert_eq!(t.written, 0);
    assert!(t.ring.iter().all(|r| r.kind == kind::NONE && r.chamber == 0 && r.at == 0 && r.reserved == [0; 2]));
    assert_eq!(t.ring.len(), RING);
    assert_eq!((t.counts.too_late, t.counts.moved, t.counts.late, t.counts.dropped), (0, 0, 0, 0));
}

#[test]
fn the_default_is_the_value_at_start() {
    let (a, b) = (Timing::default(), Timing::new());
    assert_eq!((a.started, a.state, a.written), (b.started, b.state, b.written));
}

/// The build's parameter set is one `pm-core` accepts, so Init programs it.
#[test]
fn the_build_s_parameter_set_is_accepted() {
    for set in [
        pacemaker_therapy::sets::SELECTED,
        pacemaker_therapy::sets::VVI_60,
        pacemaker_therapy::sets::VVI_90,
        pacemaker_therapy::sets::VVI_120,
        pacemaker_therapy::sets::VOO_60,
        pacemaker_therapy::sets::AAI_90,
        pacemaker_therapy::sets::AOO_90,
    ] {
        assert!(pm_core::validate(&set).is_some());
    }
    assert_eq!(pm_core::validate(&pacemaker_therapy::sets::VVI_90).unwrap().lrl, 666_666);
}

#[test]
fn a_set_given_as_text_replaces_the_named_words_of_the_nominal_set() {
    use pacemaker_therapy::sets::from_text;
    use pm_core::{word, NOMINAL_WORDS};
    // Nothing named: the nominal set.
    assert_eq!(from_text(b""), NOMINAL_WORDS);
    // VOO at 175 ppm with 0.05 ms in the ventricle.
    let w = from_text(b"0=7,1=175,11=50");
    assert_eq!((w[word::MODE], w[word::LRL], w[word::V_WIDTH]), (7, 175, 50));
    let mut rest = w;
    rest[word::MODE] = NOMINAL_WORDS[word::MODE];
    rest[word::LRL] = NOMINAL_WORDS[word::LRL];
    rest[word::V_WIDTH] = NOMINAL_WORDS[word::V_WIDTH];
    assert_eq!(rest, NOMINAL_WORDS);
    assert!(pm_core::validate(&w).is_some());
    // An index outside the set and an entry without a value change nothing.
    assert_eq!(from_text(b"99=1,3"), NOMINAL_WORDS);
    // A value the validation refuses is passed on for it to refuse.
    assert!(pm_core::validate(&from_text(b"11=55")).is_none());
}
