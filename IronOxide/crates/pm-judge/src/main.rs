//! `pm-judge`: judges one logic-analyzer capture of the PACEMAKER rig and
//! writes the report (docs/pacemaker task 4.3; PM31.3: the report names the
//! binaries and the scenario of its run).
//!
//! ```text
//! pm-judge CAPTURE RATE_HZ --mode VVI [--set LRL=60]... [--tight]
//!          [--scenario NAME] [--binary NAME=ELF]... [--json OUT] [--edges OUT]
//! ```
//!
//! `CAPTURE` is sigrok's binary output (one byte per sample, bit n the
//! channel Dn), or with the extension `.edges` the text form `--edges`
//! writes. The parameter set is the nominal one of Table 19-1 with the
//! Mode and the `--set` words replaced; it must be one `pm-core` accepts.
#![forbid(unsafe_code)]

use pm_judge::{decode, judge, play, Edges};
use std::process::ExitCode;

const MODES: [&str; 19] =
    ["Off", "DDD", "VDD", "DDI", "DOO", "AOO", "AAI", "VOO", "VVI", "AAT", "VVT", "DDDR", "VDDR", "DDIR", "DOOR", "AOOR", "AAIR", "VOOR", "VVIR"];
const WORDS: [&str; pm_core::WORDS] = [
    "MODE", "LRL", "URL", "MSR", "FIXED_AV", "DYNAMIC_AV", "MIN_DYNAMIC_AV", "SENSED_AV_OFFSET", "A_AMPLITUDE", "V_AMPLITUDE", "A_WIDTH", "V_WIDTH",
    "A_SENSITIVITY", "V_SENSITIVITY", "VRP", "ARP", "PVARP", "PVARP_EXTENSION", "HRL", "RATE_SMOOTHING_UP", "ATR", "ATR_DURATION", "ATR_FALLBACK_TIME",
    "V_BLANKING", "ACTIVITY_THRESHOLD", "REACTION_TIME", "RESPONSE_FACTOR", "RECOVERY_TIME", "RATE_SMOOTHING_DOWN",
];

fn stats(v: &[f64]) -> serde_json::Value {
    if v.is_empty() {
        return serde_json::json!({ "count": 0 });
    }
    let min = v.iter().copied().fold(f64::INFINITY, f64::min);
    let max = v.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    serde_json::json!({ "count": v.len(), "min_us": min, "max_us": max })
}

fn run() -> Result<bool, String> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [capture, rate, rest @ ..] = &args[..] else {
        return Err("usage: pm-judge CAPTURE RATE_HZ --mode NAME [--set WORD=VALUE]... [--tight] [--scenario NAME] [--binary NAME=ELF]... [--json OUT] [--edges OUT]".into());
    };
    let rate: u64 = rate.parse().map_err(|_| "RATE_HZ is a number")?;
    let mut words = pm_core::NOMINAL_WORDS;
    let (mut mode, mut tight, mut scenario, mut json, mut edges_out) = (None, false, None, None, None);
    let mut binaries = serde_json::Map::new();
    let mut it = rest.iter();
    while let Some(a) = it.next() {
        let mut value = || it.next().cloned().ok_or_else(|| format!("{a} takes a value"));
        match a.as_str() {
            "--mode" => {
                let name = value()?;
                let code = MODES.iter().position(|m| *m == name).ok_or_else(|| format!("no Mode `{name}`"))?;
                words[pm_core::word::MODE] = code as u32;
                mode = Some(name);
            }
            "--set" => {
                let v = value()?;
                let (k, n) = v.split_once('=').ok_or("--set WORD=VALUE")?;
                let i = WORDS.iter().position(|w| *w == k).ok_or_else(|| format!("no parameter word `{k}`"))?;
                words[i] = n.parse().map_err(|_| format!("`{n}` is not a number"))?;
            }
            "--tight" => tight = true,
            "--scenario" => scenario = Some(value()?),
            "--binary" => {
                let v = value()?;
                let (name, path) = v.split_once('=').ok_or("--binary NAME=ELF")?;
                let hash = rsk_digest::sha256_file(std::path::Path::new(path)).map_err(|e| format!("{path}: {e}"))?;
                binaries.insert(name.to_string(), serde_json::json!({ "path": path, "sha256": hash }));
            }
            "--json" => json = Some(value()?),
            "--edges" => edges_out = Some(value()?),
            other => return Err(format!("unknown argument `{other}`")),
        }
    }
    let mode = mode.ok_or("--mode is required")?;
    let params = pm_core::validate(&words).ok_or("the parameter set is not one pm-core accepts (Table 19-1, and the Modes of this stage)")?;

    let edges = if capture.ends_with(".edges") {
        Edges::from_text(&std::fs::read_to_string(capture).map_err(|e| format!("{capture}: {e}"))?)?
    } else {
        Edges::from_raw(&std::fs::read(capture).map_err(|e| format!("{capture}: {e}"))?, rate)
    };
    if let Some(path) = edges_out {
        std::fs::write(&path, edges.to_text()).map_err(|e| format!("{path}: {e}"))?;
    }
    let decoded = decode(&edges);
    let report = judge(&decoded, &params, tight)?;

    let played = match &scenario {
        None => None,
        Some(name) => {
            let sc = heart_sim_part::scenario::ALL.iter().find(|s| s.name == name).ok_or_else(|| format!("no scenario `{name}`"))?;
            if sc.responsive {
                None
            } else {
                Some(play(&decoded, sc)?)
            }
        }
    };
    let scenario_ok = played.as_ref().is_none_or(|p| p.mismatches.is_empty());
    let pass = report.pass() && scenario_ok;

    let chamber = |i: usize| {
        let c = &report.chambers[i];
        serde_json::json!({
            "paces": c.paces, "senses": c.senses,
            "senses_refractory": c.refractory_senses, "senses_effective": c.effective_senses, "senses_at_the_refractory_boundary": c.boundary_senses,
            "fusions": c.fusions, "pace_intervals": stats(&c.intervals_us), "pulse_widths": stats(&c.widths_us),
        })
    };
    let out = serde_json::json!({
        "verdict": if pass { "PASS" } else { "FAIL" },
        "capture": { "file": std::path::Path::new(capture).file_name().map(|n| n.to_string_lossy().into_owned()), "rate_hz": edges.rate_hz, "length_us": edges.length_us(), "sha256": rsk_digest::sha256_file(std::path::Path::new(capture)).ok() },
        "binaries": binaries,
        "mode": mode,
        "parameter_words": words.to_vec(),
        "escape_interval_us": report.escape_us,
        "tolerances_us": { "interval": pm_judge::INTERVAL_TOLERANCE_US, "pulse_width": if tight { pm_judge::WIDTH_TOLERANCE_TIGHT_US } else { pm_judge::WIDTH_TOLERANCE_US }, "inhibition_window": pm_judge::INHIBITION_WINDOW_US },
        "atrial": chamber(0),
        "ventricular": chamber(1),
        "av_delays_measured_not_judged": stats(&report.av_delays_us),
        "violations": report.violations.iter().map(|v| serde_json::json!({ "criterion": v.criterion, "chamber": format!("{:?}", v.chamber), "at_us": v.at_us, "what": v.what })).collect::<Vec<_>>(),
        "scenario": scenario.as_ref().map(|name| serde_json::json!({
            "name": name,
            "checked_against_its_table": played.is_some(),
            "atrial_beats": played.as_ref().map(|p| p.atrial_beats), "ventricular_beats": played.as_ref().map(|p| p.ventricular_beats),
            "atrial_intervals": played.as_ref().map(|p| stats(&p.atrial_intervals_us)), "conduction": played.as_ref().map(|p| stats(&p.conduction_us)),
            "mismatches": played.as_ref().map(|p| p.mismatches.clone()),
        })),
    });
    let text = serde_json::to_string_pretty(&out).map_err(|e| e.to_string())?;
    match json {
        Some(path) => std::fs::write(&path, text + "\n").map_err(|e| format!("{path}: {e}"))?,
        None => println!("{text}"),
    }
    eprintln!("{}: {} violation(s){}", if pass { "PASS" } else { "FAIL" }, report.violations.len(), played.map(|p| format!(", {} scenario mismatch(es)", p.mismatches.len())).unwrap_or_default());
    Ok(pass)
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::from(1),
        Err(e) => {
            eprintln!("pm-judge: {e}");
            ExitCode::from(2)
        }
    }
}
