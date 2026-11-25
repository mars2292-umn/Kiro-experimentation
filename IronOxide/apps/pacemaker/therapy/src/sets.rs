//! The parameter set of the build (task 8.2): until the link exists, the
//! PG is "programmed" at Init with a set chosen by a cargo feature, one per
//! rig scenario. Each is the nominal set of Table 19-1 with the words named
//! here replaced, given to `pm_core::validate` like a set from the DCM: a
//! set it refuses leaves the Power-On Reset parameters in force.

use pm_core::{word, NOMINAL_WORDS, WORDS};

/// The Mode codes of `Mode::PROGRAMMABLE` used here.
pub const AOO: u32 = 5;
pub const AAI: u32 = 6;
pub const VOO: u32 = 7;
pub const VVI: u32 = 8;

/// The nominal set with the Mode and the Lower Rate Limit replaced.
pub const fn set(mode: u32, lrl_ppm: u32) -> [u32; WORDS] {
    let mut w = NOMINAL_WORDS;
    w[word::MODE] = mode;
    w[word::LRL] = lrl_ppm;
    w
}

/// VVI at the nominal values (LRL 60 ppm, VRP 320 ms, width 0.4 ms).
pub const VVI_60: [u32; WORDS] = set(VVI, 60);
pub const VVI_90: [u32; WORDS] = set(VVI, 90);
pub const VOO_60: [u32; WORDS] = set(VOO, 60);
pub const AAI_90: [u32; WORDS] = set(AAI, 90);
pub const AOO_90: [u32; WORDS] = set(AOO, 90);
/// The QEMU runs' sets (task 9.1): each stage E1 Mode at 120 ppm, so that
/// a 3 s run holds four Paces or more.
pub const VVI_120: [u32; WORDS] = set(VVI, 120);
pub const VOO_120: [u32; WORDS] = set(VOO, 120);
pub const AAI_120: [u32; WORDS] = set(AAI, 120);
pub const AOO_120: [u32; WORDS] = set(AOO, 120);

/// A set given to the build as text (feature `set-env`, rig builds only;
/// task 9.4): the nominal set with the words named replaced, as
/// `INDEX=VALUE,INDEX=VALUE` in the environment variable `PM_SET` at build
/// time (the indices are `pm_core::word`'s). Anything else in the text is
/// skipped; `validate` judges the result like any set. The text comes as
/// bytes: the App_Prover has no contract for the conversion.
#[rsk::bounds('text = 256)]
pub const fn from_text(b: &[u8]) -> [u32; WORDS] {
    let mut w = NOMINAL_WORDS;
    let mut i = 0;
    let mut index = 0usize;
    let mut value = 0u32;
    let mut in_value = false;
    // The App_Prover keeps nothing about a variable a loop changes unless an
    // invariant says it: the set keeps its length.
    #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(w@.len() == 29))]
    'text: while i < b.len() && i < 256 {
        let c = b[i];
        if c >= b'0' && c <= b'9' {
            let d = (c - b'0') as u32;
            // A number too long to be a word or an index stops growing: it
            // is already one that `validate` or the bound below refuses.
            if in_value {
                if value <= 99_999_999 {
                    value = value * 10 + d;
                }
            } else if index < 1_000 {
                index = index * 10 + d as usize;
            }
        } else if c == b'=' {
            in_value = true;
        }
        if c == b',' || i + 1 == b.len() {
            if in_value && index < WORDS {
                w[index] = value;
            }
            index = 0;
            value = 0;
            in_value = false;
        }
        i += 1;
    }
    w
}

/// The set this build is programmed with.
#[cfg(feature = "set-env")]
pub const SELECTED: [u32; WORDS] = from_text(env!("PM_SET").as_bytes());
#[cfg(not(feature = "set-env"))]
pub const SELECTED: [u32; WORDS] = BY_FEATURE;

/// The set a cargo feature names (public: the App_Prover refuses a public
/// constant built from a private one).
#[cfg(feature = "set-vvi-90")]
pub const BY_FEATURE: [u32; WORDS] = VVI_90;
#[cfg(all(feature = "set-voo-60", not(feature = "set-vvi-90")))]
pub const BY_FEATURE: [u32; WORDS] = VOO_60;
#[cfg(all(feature = "set-aai-90", not(any(feature = "set-vvi-90", feature = "set-voo-60"))))]
pub const BY_FEATURE: [u32; WORDS] = AAI_90;
#[cfg(all(feature = "set-aoo-90", not(any(feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90"))))]
pub const BY_FEATURE: [u32; WORDS] = AOO_90;
#[cfg(all(feature = "nrf52840", not(any(feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90", feature = "set-aoo-90"))))]
pub const BY_FEATURE: [u32; WORDS] = VVI_60;
#[cfg(all(feature = "qemu-voo", not(any(feature = "nrf52840", feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90", feature = "set-aoo-90"))))]
pub const BY_FEATURE: [u32; WORDS] = VOO_120;
#[cfg(all(feature = "qemu-aoo", not(any(feature = "qemu-voo", feature = "nrf52840", feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90", feature = "set-aoo-90"))))]
pub const BY_FEATURE: [u32; WORDS] = AOO_120;
#[cfg(all(feature = "qemu-aai", not(any(feature = "qemu-voo", feature = "qemu-aoo", feature = "nrf52840", feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90", feature = "set-aoo-90"))))]
pub const BY_FEATURE: [u32; WORDS] = AAI_120;
#[cfg(not(any(feature = "qemu-voo", feature = "qemu-aoo", feature = "qemu-aai", feature = "nrf52840", feature = "set-vvi-90", feature = "set-voo-60", feature = "set-aai-90", feature = "set-aoo-90")))]
pub const BY_FEATURE: [u32; WORDS] = VVI_120;

/// Whether the build is programmed with `SELECTED` at Init. A build of the
/// board without a set of its own is not: it starts in the Power-On Reset
/// State and stays there until a set comes over the link (PM7.1, PM7.4,
/// PM30.2; task 16.6). The QEMU Target has no link and always is.
pub const AT_INIT: bool = cfg!(any(
    not(feature = "nrf52840"),
    feature = "set-env",
    feature = "set-vvi-90",
    feature = "set-voo-60",
    feature = "set-aai-90",
    feature = "set-aoo-90"
));
