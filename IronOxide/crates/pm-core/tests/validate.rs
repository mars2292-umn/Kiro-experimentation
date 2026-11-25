//! Host tests of `pm-core`'s `validate` module. They are an integration test
//! so that the crate itself names no `std`: it is in the Target graph of
//! the Therapy_Partition (rsk's check R6.1).
#![forbid(unsafe_code)]
#![allow(unused_imports)]

use pm_core::*;


/// The code of VVI in `Mode::PROGRAMMABLE`.
const VVI: u32 = 8;

/// The nominal set in VVI.
fn base() -> [u32; WORDS] {
    let mut w = NOMINAL_WORDS;
    w[word::MODE] = VVI;
    w
}

fn range(lo: u32, hi: u32, step: u32) -> Vec<u32> {
    (lo..=hi).step_by(step as usize).collect()
}

/// Table 19-1 written out row by row, independently of the predicates
/// above: the values each word may take.
fn table() -> Vec<(usize, Vec<u32>)> {
    let lrl: Vec<u32> = [range(30, 50, 5), range(51, 90, 1), range(95, 175, 5)].concat();
    let amplitude: Vec<u32> = [std::vec![0], range(5, 32, 1), range(35, 70, 5)].concat();
    let width: Vec<u32> = [std::vec![50], range(100, 1_900, 100)].concat();
    let sensitivity: Vec<u32> = [std::vec![25, 50, 75], range(100, 1_000, 50)].concat();
    let smoothing: Vec<u32> = [std::vec![0], range(3, 21, 3), std::vec![25]].concat();
    std::vec![
        (word::LRL, lrl.clone()),
        (word::URL, range(50, 175, 5)),
        (word::MSR, range(50, 175, 5)),
        (word::FIXED_AV, range(70, 300, 10)),
        (word::DYNAMIC_AV, std::vec![0, 1]),
        (word::MIN_DYNAMIC_AV, range(30, 100, 10)),
        (word::SENSED_AV_OFFSET, [std::vec![0], range(10, 100, 10)].concat()),
        (word::A_AMPLITUDE, amplitude.clone()),
        (word::V_AMPLITUDE, amplitude),
        (word::A_WIDTH, width.clone()),
        (word::V_WIDTH, width),
        (word::A_SENSITIVITY, sensitivity.clone()),
        (word::V_SENSITIVITY, sensitivity),
        (word::VRP, range(150, 500, 10)),
        (word::ARP, range(150, 500, 10)),
        (word::PVARP, range(150, 500, 10)),
        (word::PVARP_EXTENSION, [std::vec![0], range(50, 400, 50)].concat()),
        (word::HRL, [std::vec![0], lrl.clone()].concat()),
        (word::RATE_SMOOTHING_UP, smoothing.clone()),
        (word::ATR, std::vec![0, 1]),
        (word::ATR_DURATION, [std::vec![10], range(20, 80, 20), range(100, 2_000, 100)].concat()),
        (word::ATR_FALLBACK_TIME, range(1, 5, 1)),
        (word::V_BLANKING, range(30, 60, 10)),
        (word::ACTIVITY_THRESHOLD, range(0, 6, 1)),
        (word::REACTION_TIME, range(10, 50, 10)),
        (word::RESPONSE_FACTOR, range(1, 16, 1)),
        (word::RECOVERY_TIME, range(2, 16, 1)),
        (word::RATE_SMOOTHING_DOWN, smoothing),
    ]
}

/// Property P5 row by row, exhaustively per parameter: with every other
/// word nominal, in VVI, each word is accepted exactly at the values its
/// row lists, over 0 to 2,200 (above every row's largest value) and at the
/// extremes of the word. Since task 19.5 the rows of hysteresis and Rate
/// Smoothing have their values (they had Off alone), and VVI's one limit
/// is applied: an HRL that is on is not above the LRL, which is 60.
#[test]
fn each_word_is_accepted_exactly_at_its_table_values() {
    let table = table();
    assert_eq!(table.len(), WORDS - 1, "every word but the Mode has a row");
    for (i, values) in &table {
        let mut tried = 0;
        for v in (0..=2_200).chain([u32::MAX - 1, u32::MAX, 1 << 31, 65_536 + values[0], 256 + values[0]]) {
            let mut w = base();
            w[*i] = v;
            let limit = *i != word::HRL || v == 0 || v <= w[word::LRL];
            assert_eq!(validate(&w).is_some(), values.contains(&v) && limit, "word {i} value {v}");
            tried += 1;
        }
        assert_eq!(tried, 2_206);
    }
}

/// The table has the number of values the source's rows give.
#[test]
fn the_rows_have_the_source_s_counts() {
    let count = |i: usize| table().into_iter().find(|(j, _)| *j == i).unwrap().1.len();
    assert_eq!(count(word::LRL), 5 + 40 + 17);
    assert_eq!(count(word::URL), 26);
    assert_eq!(count(word::A_AMPLITUDE), 1 + 28 + 8);
    assert_eq!(count(word::A_WIDTH), 20);
    assert_eq!(count(word::A_SENSITIVITY), 3 + 19);
    assert_eq!(count(word::VRP), 36);
    assert_eq!(count(word::ATR_DURATION), 1 + 4 + 20);
}

/// Stage E4 (task 19.5): the nineteen Modes of Table 19-1 and no other.
/// Until then this test named the eleven fixed-rate Modes of stage E2.
#[test]
fn the_modes_of_table_19_1_and_no_other() {
    assert_eq!(Mode::PROGRAMMABLE.len(), 19);
    for code in 0..40 {
        let mut w = base();
        w[word::MODE] = code;
        let expected = Mode::PROGRAMMABLE.get(code as usize).is_some();
        assert_eq!(validate(&w).is_some(), expected, "mode code {code}");
        if let Some(p) = validate(&w) {
            assert_eq!(p.mode, Mode::PROGRAMMABLE[code as usize]);
        }
    }
    let mut w = base();
    w[word::MODE] = u32::MAX;
    assert_eq!(validate(&w), None);
}

/// Since task 19.5 hysteresis and Rate Smoothing are accepted (until then
/// this test had them refused): the HRL as its interval, Up and Down each
/// in its own field.
#[test]
fn hysteresis_and_rate_smoothing_reach_their_fields() {
    let mut w = base();
    w[word::HRL] = 50;
    w[word::RATE_SMOOTHING_UP] = 3;
    w[word::RATE_SMOOTHING_DOWN] = 25;
    let p = validate(&w).unwrap();
    assert_eq!((p.hrl, p.rate_smoothing_up, p.rate_smoothing_down), (1_200_000, 3, 25));
    w[word::HRL] = 0;
    assert_eq!(validate(&w).unwrap().hrl, 0, "Off stays Off");
}

#[test]
fn only_the_exact_length_is_accepted() {
    let w = base();
    assert!(validate(&w).is_some());
    assert_eq!(validate(&w[..WORDS - 1]), None);
    assert_eq!(validate(&[]), None);
    let mut longer = Vec::from(w);
    longer.push(0);
    assert_eq!(validate(&longer), None);
}

/// The nominal words give the nominal values (PM19.1), field by field,
/// apart from the Mode this stage needs.
#[test]
fn the_nominal_words_are_the_nominal_set() {
    let p = validate(&base()).unwrap();
    assert_eq!(p, Params { mode: Mode::VVI, ..Params::NOMINAL });
    assert_eq!(Mode::PROGRAMMABLE[NOMINAL_WORDS[word::MODE] as usize], Params::NOMINAL.mode);
}

/// Every rate becomes its interval rounded down: the rate delivered is
/// not below the programmed one, and under 1 µs from it (PM26.5).
#[test]
fn each_rate_becomes_its_interval_rounded_down() {
    for (i, values) in table() {
        if ![word::LRL, word::URL, word::MSR].contains(&i) {
            continue;
        }
        for ppm in values {
            let mut w = base();
            w[i] = ppm;
            let p = validate(&w).unwrap();
            let interval = [p.lrl, p.url, p.msr][i - word::LRL];
            assert!(u64::from(interval) * u64::from(ppm) <= 60_000_000, "{ppm} ppm");
            assert!(u64::from(interval + 1) * u64::from(ppm) > 60_000_000, "{ppm} ppm");
        }
    }
}

/// Each word reaches its own field, in the field's unit.
#[test]
fn each_word_reaches_its_field() {
    let mut w = base();
    w[word::MODE] = 6; // AAI
    w[word::LRL] = 75;
    w[word::URL] = 150;
    w[word::MSR] = 175;
    w[word::FIXED_AV] = 300;
    w[word::DYNAMIC_AV] = 1;
    w[word::MIN_DYNAMIC_AV] = 30;
    w[word::SENSED_AV_OFFSET] = 100;
    w[word::A_AMPLITUDE] = 5;
    w[word::V_AMPLITUDE] = 70;
    w[word::A_WIDTH] = 50;
    w[word::V_WIDTH] = 1_900;
    w[word::A_SENSITIVITY] = 25;
    w[word::V_SENSITIVITY] = 1_000;
    w[word::VRP] = 500;
    w[word::ARP] = 150;
    w[word::PVARP] = 160;
    w[word::PVARP_EXTENSION] = 400;
    w[word::HRL] = 40;
    w[word::RATE_SMOOTHING_UP] = 9;
    w[word::RATE_SMOOTHING_DOWN] = 21;
    w[word::ATR] = 1;
    w[word::ATR_DURATION] = 2_000;
    w[word::ATR_FALLBACK_TIME] = 5;
    w[word::V_BLANKING] = 60;
    w[word::ACTIVITY_THRESHOLD] = 6;
    w[word::REACTION_TIME] = 50;
    w[word::RESPONSE_FACTOR] = 16;
    w[word::RECOVERY_TIME] = 16;
    let p = validate(&w).unwrap();
    let expected = Params {
        mode: Mode::AAI,
        lrl: 800_000,
        url: 400_000,
        msr: 342_857,
        fixed_av: 300_000,
        dynamic_av: true,
        min_dynamic_av: 30_000,
        sensed_av_offset: 100_000,
        amplitude: [5, 70],
        width: [50, 1_900],
        sensitivity: [25, 1_000],
        vrp: 500_000,
        arp: 150_000,
        pvarp: 160_000,
        pvarp_extension: 400_000,
        hrl: 1_500_000,
        rate_smoothing_up: 9,
        rate_smoothing_down: 21,
        atr: true,
        atr_duration: 2_000,
        atr_fallback_time: 300_000_000,
        v_blanking: 60_000,
        activity_threshold: ActivityThreshold::VHigh,
        reaction_time: 50_000_000,
        response_factor: 16,
        recovery_time: 960_000_000,
        reserved: [0; 2],
    };
    assert_eq!(p, expected);
}

// The interactive limits of Table 19-5 (stage E2, task 11.6; over every
// Mode and with the limits of the sensor and of hysteresis since task
// 19.5), each exhaustively over the values of the parameters it names and
// over every Mode, against the limit written out here from the table and
// from Table 19-3 as amended by Table 19-4, independently of `validate`.

/// The Modes by code, with the two properties the limits of stage E2 turn
/// on: whether the Mode tracks (VDD, DDD and their rate-adaptive forms)
/// and whether it is DDD or DDDR. Until task 19.5 the eleven fixed-rate
/// Modes; the loose set keeps the sensor's limit (its LRL is the lowest).
fn fixed_rate_modes() -> Vec<(u32, bool, bool)> {
    (0..19u32)
        .map(|code| {
            let m = Mode { adaptive: false, ..Mode::PROGRAMMABLE[code as usize] };
            (code, m == Mode::VDD || m == Mode::DDD, m == Mode::DDD)
        })
        .collect()
}

/// Whether the Mode of `code` is rate-adaptive, and whether it uses
/// hysteresis: from Table 19-3, written out here.
fn adaptive(code: u32) -> bool {
    code >= 11
}

fn has_hysteresis(code: u32) -> bool {
    let m = Mode { adaptive: false, ..Mode::PROGRAMMABLE[code as usize] };
    m == Mode::AAI || m == Mode::VVI || m == Mode::DDD
}

fn values(i: usize) -> Vec<u32> {
    table().into_iter().find(|(j, _)| *j == i).unwrap().1
}

/// A set in which no limit binds but the one under test: the lowest LRL
/// with the lowest URL (an interval of 1,200 ms), the shortest refractory
/// periods and AV delay.
fn loose(code: u32) -> [u32; WORDS] {
    let mut w = NOMINAL_WORDS;
    w[word::MODE] = code;
    w[word::LRL] = 30;
    w[word::URL] = 50;
    w[word::VRP] = 150;
    w[word::PVARP] = 150;
    w[word::FIXED_AV] = 70;
    w[word::MIN_DYNAMIC_AV] = 30;
    w
}

#[test]
fn every_loose_set_is_accepted_in_every_mode() {
    for (code, _, _) in fixed_rate_modes() {
        assert!(validate(&loose(code)).is_some(), "mode code {code}");
    }
}

/// LRL ≤ URL, in the Modes that track.
#[test]
fn limit_lrl_not_above_url() {
    let mut cases = 0;
    for (code, tracks, _) in fixed_rate_modes() {
        for lrl in values(word::LRL) {
            for url in values(word::URL) {
                let mut w = loose(code);
                w[word::LRL] = lrl;
                w[word::URL] = url;
                // The MSR of the loose set is 120: the sensor's limit.
                let sensor = !adaptive(code) || lrl <= 120;
                assert_eq!(validate(&w).is_some(), (!tracks || lrl <= url) && sensor, "mode {code} LRL {lrl} URL {url}");
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 19 * 62 * 26);
}

/// VRP < URL interval, in the Modes that track.
#[test]
fn limit_vrp_below_the_url_interval() {
    for (code, tracks, _) in fixed_rate_modes() {
        for vrp in values(word::VRP) {
            for url in values(word::URL) {
                let mut w = loose(code);
                w[word::VRP] = vrp;
                w[word::URL] = url;
                // The URL interval in µs, rounded down as the PG keeps it.
                let holds = vrp * 1_000 < 60_000_000 / url;
                assert_eq!(validate(&w).is_some(), !tracks || holds, "mode {code} VRP {vrp} URL {url}");
            }
        }
    }
}

/// PVARP + Fixed AV Delay ≤ URL interval, in the Modes that track.
#[test]
fn limit_pvarp_and_av_delay_within_the_url_interval() {
    let (mut refused, mut accepted) = (0, 0);
    for (code, tracks, _) in fixed_rate_modes() {
        for pvarp in values(word::PVARP) {
            for av in values(word::FIXED_AV) {
                for url in values(word::URL) {
                    let mut w = loose(code);
                    w[word::PVARP] = pvarp;
                    w[word::FIXED_AV] = av;
                    w[word::URL] = url;
                    let holds = (pvarp + av) * 1_000 <= 60_000_000 / url;
                    let expected = !tracks || holds;
                    assert_eq!(validate(&w).is_some(), expected, "mode {code} PVARP {pvarp} AV {av} URL {url}");
                    if expected { accepted += 1 } else { refused += 1 }
                }
            }
        }
    }
    assert!(refused > 1_000 && accepted > 1_000);
}

/// Minimum Dynamic AV Delay ≤ Fixed AV Delay, in the Modes that track,
/// where the Dynamic AV Delay is On.
#[test]
fn limit_minimum_dynamic_av_delay_not_above_the_fixed() {
    for (code, tracks, _) in fixed_rate_modes() {
        for min in values(word::MIN_DYNAMIC_AV) {
            for av in values(word::FIXED_AV) {
                for dynamic in [0, 1] {
                    let mut w = loose(code);
                    w[word::MIN_DYNAMIC_AV] = min;
                    w[word::FIXED_AV] = av;
                    w[word::DYNAMIC_AV] = dynamic;
                    let binds = tracks && dynamic == 1;
                    assert_eq!(validate(&w).is_some(), !binds || min <= av, "mode {code} minimum {min} AV {av} dynamic {dynamic}");
                }
            }
        }
    }
}

/// Fixed AV Delay less the Sensed AV Delay Offset ≥ 30 ms, in DDD.
#[test]
fn limit_shortened_av_delay_not_below_30_ms() {
    for (code, _, ddd) in fixed_rate_modes() {
        for av in values(word::FIXED_AV) {
            for offset in values(word::SENSED_AV_OFFSET) {
                let mut w = loose(code);
                w[word::FIXED_AV] = av;
                w[word::SENSED_AV_OFFSET] = offset;
                assert_eq!(validate(&w).is_some(), !ddd || av >= offset + 30, "mode {code} AV {av} offset {offset}");
            }
        }
    }
}

/// Since task 19.5 the Atrial Tachycardia Response, hysteresis and Rate
/// Smoothing are accepted in every Mode (until then this test had the
/// first refused in the tracking Modes and the other two everywhere).
#[test]
fn atr_hysteresis_and_smoothing_are_accepted_in_every_mode() {
    for (code, _, _) in fixed_rate_modes() {
        for (i, v) in [(word::ATR, 1), (word::HRL, 30), (word::RATE_SMOOTHING_UP, 12), (word::RATE_SMOOTHING_DOWN, 25)] {
            let mut w = loose(code);
            w[i] = v;
            assert!(validate(&w).is_some(), "mode {code} word {i}");
        }
    }
}

/// LRL ≤ MSR, in the rate-adaptive Modes.
#[test]
fn limit_lrl_not_above_msr() {
    let mut cases = 0;
    for (code, _, _) in fixed_rate_modes() {
        for lrl in values(word::LRL) {
            for msr in values(word::MSR) {
                let mut w = loose(code);
                w[word::LRL] = lrl;
                w[word::MSR] = msr;
                // No other limit: the URL at its highest keeps LRL ≤ URL,
                // and the loose refractory periods fit its interval.
                w[word::URL] = 175;
                assert_eq!(validate(&w).is_some(), !adaptive(code) || lrl <= msr, "mode {code} LRL {lrl} MSR {msr}");
                cases += 1;
            }
        }
    }
    assert_eq!(cases, 19 * 62 * 26);
}

/// HRL ≤ LRL where hysteresis is on, in the Modes that use hysteresis.
#[test]
fn limit_hrl_not_above_lrl() {
    let (mut refused, mut accepted) = (0, 0);
    for (code, _, _) in fixed_rate_modes() {
        for lrl in values(word::LRL) {
            for hrl in values(word::HRL) {
                let mut w = loose(code);
                w[word::LRL] = lrl;
                w[word::HRL] = hrl;
                w[word::URL] = 175;
                w[word::MSR] = 175;
                let expected = !has_hysteresis(code) || hrl == 0 || hrl <= lrl;
                assert_eq!(validate(&w).is_some(), expected, "mode {code} LRL {lrl} HRL {hrl}");
                if expected { accepted += 1 } else { refused += 1 }
            }
        }
    }
    assert_eq!(accepted + refused, 19 * 62 * 63);
    assert!(refused > 1_000);
}

/// Property P5 per parameter in DDD, where every limit binds: with every
/// other word at the loose set's value, a word is accepted exactly at the
/// values of its row that keep the limits.
#[test]
fn each_word_in_ddd_is_accepted_exactly_at_its_table_values_within_the_limits() {
    for (i, row) in table() {
        for v in (0..=2_200).chain([u32::MAX]) {
            let mut w = loose(1);
            assert_eq!(Mode::PROGRAMMABLE[1], Mode::DDD);
            w[i] = v;
            let in_row = row.contains(&v);
            let limits = w[word::LRL] <= w[word::URL]
                && w[word::VRP].saturating_mul(1_000) < 60_000_000 / w[word::URL].max(1)
                && (w[word::PVARP].saturating_add(w[word::FIXED_AV])).saturating_mul(1_000) <= 60_000_000 / w[word::URL].max(1)
                && (w[word::DYNAMIC_AV] != 1 || w[word::MIN_DYNAMIC_AV] <= w[word::FIXED_AV])
                && w[word::FIXED_AV] >= w[word::SENSED_AV_OFFSET].saturating_add(30);
            // Since task 19.5 no stage rule (it had hysteresis, Rate
            // Smoothing and ATR refused here); hysteresis has its limit.
            let hysteresis = w[word::HRL] == 0 || w[word::HRL] <= w[word::LRL];
            assert_eq!(validate(&w).is_some(), in_row && limits && hysteresis, "word {i} value {v}");
        }
    }
}
