//! Evidence_Items from the rig's committed campaign records (R50.6; task of
//! 2026-11-17).
//!
//! The hour-long release-timing and stress campaigns and the
//! Kernel_Timing_Parameter records are operator runs: their reports and
//! records are committed under `docs/hil/`, and until now no Evidence_Item
//! stood for them, so the criteria they answer (R9.10, R18.8, R19.3, the
//! measured bounds of R8.8, R10.6 and R11.3) had none. This module reads
//! what is committed and says, for each record, what it is evidence for
//! and whether it is of a binary of this Build_Manifest:
//!
//! - `measure-kernel-timing-<variant>`: one per committed
//!   `kernel_timing_<variant>.json`. It covers a criterion only when every
//!   Kernel_Timing_Parameter the criterion names is in the record as a
//!   measured value (so not R7.8, R14.8 or R14.10, each of which names a
//!   parameter no record measures).
//! - `hil-release-timing`, `hil-stress`: the campaigns of at least
//!   `CAMPAIGN_SECS`, from the report's own verdict and operating duration.
//!   A report carries no binary hash; the Kernel_Timing_Parameter record
//!   that the same command writes after a passed run does, and the item
//!   takes its binary from it.
//!
//! A record of a binary the manifest does not list is stale (R33.4): its
//! item is `inconclusive` and says so. Nothing here runs the rig.

use crate::evidence::{Kind, Verdict};
use std::path::Path;

/// The fixed arm of PAR-05 (the longer of 1 hour and 1,000 hyperperiods).
/// The hyperperiod arm is not checked here.
pub const CAMPAIGN_SECS: u64 = 3600;

/// The criteria a Kernel_Timing_Parameter record can stand for, each with
/// the parameters it names: all of them must be measured values of the
/// record.
pub const TIMING_CRITERIA: &[(&str, &[&str])] = &[
    ("R7.8", &["delta_dispatch", "delta_wake", "delta_complete"]),
    ("R8.8", &["delta_lock_in", "delta_lock_out"]),
    ("R10.6", &["j_rel"]),
    ("R11.3", &["delta_fp_save", "delta_fp_restore"]),
    ("R14.8", &["delta_enforce", "delta_hm"]),
    ("R14.10", &["delta_restart", "delta_stop"]),
];

/// One Evidence_Item to be written.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RigItem {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub verdict: Verdict,
    pub covers: Vec<&'static str>,
    pub notes: Vec<String>,
}

/// The criteria whose parameters are all measured values of `timing`.
pub fn timing_covers(timing: &rsk_model::KernelTiming) -> Vec<&'static str> {
    let measured = |s: &str| timing.parameter(s).is_some_and(|p| p.method == "measured");
    TIMING_CRITERIA.iter().filter(|(_, symbols)| symbols.iter().all(|s| measured(s))).map(|(c, _)| *c).collect()
}

/// The verdict line and the operating duration of a campaign report
/// (`Verdict: **pass**`, `Operating duration 3660 s on ...`), the first of
/// each.
pub fn report_facts(text: &str) -> (Option<String>, Option<u64>) {
    let verdict = text.lines().find_map(|l| l.strip_prefix("Verdict: **").and_then(|r| r.split("**").next()).map(str::to_string));
    let secs = text.lines().find_map(|l| l.strip_prefix("Operating duration ").and_then(|r| r.split(' ').next()).and_then(|n| n.parse().ok()));
    (verdict, secs)
}

/// The target binary a campaign report names (`Target binary: sha256:...`,
/// printed since 2026-11-17); `None` for an earlier report.
pub fn report_binary(text: &str) -> Option<String> {
    text.lines().find_map(|l| l.strip_prefix("Target binary: ")).map(|h| h.trim().to_string())
}

fn manifest_hashes(manifest: &serde_json::Value) -> Vec<String> {
    let mut hashes = Vec::new();
    for key in ["binaries", "hil_binaries"] {
        for b in manifest.get(key).and_then(|b| b.as_array()).into_iter().flatten() {
            if let Some(h) = b.get("hash").and_then(|h| h.as_str()) {
                hashes.push(h.to_string());
            }
        }
    }
    hashes
}

/// PAR-02: the minimum measured executions per WCET item (R34.2).
pub const PAR_02_RUNS: u64 = 10_000;

/// The measured entries of a record (`records` of a WCET_Record file,
/// `parameters` of a Kernel_Timing_Parameter file) with fewer than PAR-02
/// executions (R34.2), by name with their run counts, and the number of
/// measured entries in all. An entry that states no run count counts as
/// short, with 0.
pub fn below_par_02(text: &str) -> (Vec<(String, u64)>, usize) {
    let Ok(json) = serde_json::from_str::<serde_json::Value>(text) else { return (Vec::new(), 0) };
    let entries = json.get("records").or_else(|| json.get("parameters")).and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let mut short = Vec::new();
    let mut measured = 0usize;
    for e in &entries {
        if e.get("method").and_then(|m| m.as_str()) != Some("measured") {
            continue;
        }
        measured += 1;
        let runs = e.pointer("/measured/runs").and_then(|r| r.as_u64()).unwrap_or(0);
        if runs < PAR_02_RUNS {
            let name = e.get("item").or_else(|| e.get("symbol")).and_then(|n| n.as_str()).unwrap_or("?").to_string();
            short.push((name, runs));
        }
    }
    (short, measured)
}

/// The sentence of an Evidence_Item's notes on PAR-02 for one record.
fn par_02_note(what: &str, text: &str) -> String {
    let (short, measured) = below_par_02(text);
    if short.is_empty() {
        format!("PAR-02 (R34.2): each of the {measured} measured {what} has at least {PAR_02_RUNS} executions")
    } else {
        format!(
            "PAR-02 (R34.2): {} of the {measured} measured {what} have fewer than {PAR_02_RUNS} executions: {}",
            short.len(),
            short.iter().map(|(n, r)| format!("{n} {r}")).collect::<Vec<_>>().join(", ")
        )
    }
}

/// The items of the records committed under `docs/hil/`, for the
/// measurement configurations the rig knows.
pub fn items(root: &Path, manifest: &serde_json::Value) -> Vec<RigItem> {
    let hashes = manifest_hashes(manifest);
    let mut out = Vec::new();
    let variants = crate::hil::MEASUREMENTS.iter().map(|m| (m.variant, m.kind)).chain(crate::hil::RECORD_VARIANTS.iter().map(|(v, _)| (*v, "cycles")));
    for (variant, kind) in variants {
        let path = crate::hil::timing_record_path(root, variant);
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        let rel = path.strip_prefix(root).unwrap_or(&path).display().to_string();
        let timing = match rsk_model::read_kernel_timing(&text) {
            Ok(t) => t,
            Err(e) => {
                out.push(RigItem {
                    id: format!("measure-kernel-timing-{variant}"),
                    kind: Kind::Measurement,
                    name: format!("Kernel_Timing_Parameter record of `{variant}` ({rel})"),
                    verdict: Verdict::Fail,
                    covers: Vec::new(),
                    notes: vec![format!("not a Kernel_Timing_Parameter record: {e}")],
                });
                continue;
            }
        };
        let fresh = hashes.contains(&timing.binary_hash);
        let stale_note = format!("stale (R33.4): the record is of {}, which is not a binary of this Build_Manifest; measure again on this revision's HIL build", timing.binary_hash);
        let measured: Vec<String> = timing.parameters.iter().filter(|p| p.method == "measured").map(|p| format!("{} {}", p.symbol, p.cycles)).collect();
        let mut notes = vec![
            format!("measured maxima with the rig's margin of {} % (R34.6), in cycles: {}", crate::hil::MARGIN_PERCENT, measured.join(", ")),
            "a measured maximum is not a static bound (ORQ-13); the criteria listed are those whose parameters are all measured values of this record".to_string(),
        ];
        // PAR-02 (2026-11-21): which entries of this record and of the
        // configuration's WCET_Records fall short of the stated executions.
        notes.push(par_02_note("Kernel_Timing_Parameters", &text));
        if let Ok(wcet) = std::fs::read_to_string(root.join("docs").join("hil").join(format!("wcet_{variant}.json"))) {
            notes.push(par_02_note(&format!("WCET_Records (docs/hil/wcet_{variant}.json)"), &wcet));
        }
        if !fresh {
            notes.insert(0, stale_note.clone());
        }
        out.push(RigItem {
            id: format!("measure-kernel-timing-{variant}"),
            kind: Kind::Measurement,
            name: format!("Kernel_Timing_Parameter record of `{variant}` ({rel}; binary {})", timing.binary_hash),
            verdict: if fresh { Verdict::Pass } else { Verdict::Inconclusive },
            covers: timing_covers(&timing),
            notes,
        });
        // The campaign of this configuration, when its report is committed.
        let (id, report_name, covers): (&str, String, Vec<&'static str>) = match kind {
            "release-timing" => ("hil-release-timing", format!("release_timing_{variant}.md"), vec!["R9.10"]),
            "stress" => ("hil-stress", format!("stress_{variant}.md"), vec!["R18.8", "R19.3"]),
            _ => continue,
        };
        let report_path = root.join("docs").join("hil").join(&report_name);
        let Ok(report) = std::fs::read_to_string(&report_path) else { continue };
        let (verdict, secs) = report_facts(&report);
        // Only a campaign of PAR-05's length answers the criteria; the short
        // configurations are the job's quick runs.
        if secs.is_none_or(|s| s < CAMPAIGN_SECS) {
            continue;
        }
        let passed = verdict.as_deref() == Some("pass");
        // The report's own binary when it names one; an earlier report
        // names none, and the timing record the same command writes after
        // a passed run stands in.
        let named = report_binary(&report);
        let (binary, fresh, stale_note) = match &named {
            Some(h) => (h.clone(), hashes.contains(h), format!("stale (R33.4): the report is of {h}, which is not a binary of this Build_Manifest; run the campaign again on this revision's HIL build")),
            None => (timing.binary_hash.clone(), fresh, stale_note),
        };
        let mut notes = vec![
            format!("docs/hil/{report_name}: verdict `{}`, operating duration {} s (PAR-05's fixed arm is {CAMPAIGN_SECS} s; its 1,000-hyperperiod arm is not checked here)", verdict.as_deref().unwrap_or("none found"), secs.unwrap_or(0)),
            match &named {
                Some(_) => format!("an operator campaign, not run by this job; the report names its binary: {binary}"),
                None => format!("an operator campaign, not run by this job; the report names no binary, and the one of {rel}, which the same command writes after a passed run, is taken: {binary}"),
            },
        ];
        if !fresh {
            notes.insert(0, stale_note);
        }
        out.push(RigItem {
            id: id.to_string(),
            kind: Kind::Test,
            name: format!("HIL {} campaign on `{variant}` over PAR-05 (docs/hil/{report_name}; binary {binary})", if kind == "stress" { "fault-injection and stress" } else { "release-timing" }),
            verdict: match (fresh, passed, verdict.is_some()) {
                (true, true, _) => Verdict::Pass,
                (true, false, true) => Verdict::Fail,
                _ => Verdict::Inconclusive,
            },
            covers,
            notes,
        });
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn record(hash: &str, symbols: &[(&str, &str)]) -> String {
        let params: Vec<String> = symbols
            .iter()
            .map(|(s, m)| format!(r#"{{"symbol": "{s}", "cycles": 100, "method": "{m}", "tool": "t", "conditions": []}}"#))
            .collect();
        format!(r#"{{"schema": "rsk-kernel-timing/1", "binary_hash": "{hash}", "target": "nrf52840/r0p1", "clock_hz": 64000000, "icache": "off", "parameters": [{}]}}"#, params.join(", "))
    }

    fn root_with(files: &[(&str, String)]) -> std::path::PathBuf {
        static NEXT: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        let dir = std::env::temp_dir().join(format!("rsk-rig-records-{}-{}", std::process::id(), NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(dir.join("docs/hil")).unwrap();
        for (name, text) in files {
            std::fs::write(dir.join("docs/hil").join(name), text).unwrap();
        }
        dir
    }

    /// R33.4, R50.6: a record covers a criterion only when every parameter
    /// the criterion names is a measured value of it.
    #[test]
    fn r50_6_a_timing_record_covers_only_the_criteria_whose_parameters_it_measures() {
        let t = rsk_model::read_kernel_timing(&record(
            "sha256:a",
            &[("j_rel", "measured"), ("delta_lock_in", "measured"), ("delta_lock_out", "measured"), ("delta_dispatch", "measured"), ("delta_complete", "measured"), ("delta_enforce", "measured"), ("delta_restart", "measured")],
        ))
        .unwrap();
        // R7.8 lacks delta_wake, R14.8 lacks delta_hm, R14.10 lacks delta_stop.
        assert_eq!(timing_covers(&t), vec!["R8.8", "R10.6"]);
    }

    /// R9.10, R18.8, R19.3: an hour-long campaign whose record is of a binary
    /// of the manifest passes with its report; the same on another binary is
    /// inconclusive and says the record is stale; a short run gives no
    /// campaign item.
    #[test]
    fn r9_10_r18_8_a_campaign_counts_only_at_par_05_length_and_on_this_manifests_binary() {
        let report = |secs: u64, verdict: &str| format!("# HIL run\n\nVerdict: **{verdict}**\n\nOperating duration {secs} s on the target x\n");
        let root = root_with(&[
            ("kernel_timing_nrf52840_timing.json", record("sha256:timing", &[("j_rel", "measured")])),
            ("release_timing_nrf52840_timing.md", report(3660, "pass")),
            ("kernel_timing_nrf52840_stress.json", record("sha256:stress", &[("j_rel", "measured")])),
            ("stress_nrf52840_stress.md", report(3660, "pass")),
            ("kernel_timing_nrf52840_timing_short.json", record("sha256:short", &[("j_rel", "measured")])),
            ("release_timing_nrf52840_timing_short.md", report(180, "pass")),
        ]);
        let manifest: serde_json::Value = serde_json::from_str(r#"{"hil_binaries": [{"hash": "sha256:timing"}, {"hash": "sha256:short"}]}"#).unwrap();
        let got = items(&root, &manifest);
        let _ = std::fs::remove_dir_all(&root);
        let by = |id: &str| got.iter().find(|i| i.id == id).unwrap_or_else(|| panic!("{id}: {got:?}"));
        let timing = by("hil-release-timing");
        assert_eq!((timing.verdict, timing.covers.clone()), (Verdict::Pass, vec!["R9.10"]));
        let stress = by("hil-stress");
        assert_eq!((stress.verdict, stress.covers.clone()), (Verdict::Inconclusive, vec!["R18.8", "R19.3"]));
        assert!(stress.notes[0].starts_with("stale (R33.4)"), "{:?}", stress.notes);
        assert_eq!(by("measure-kernel-timing-nrf52840_stress").verdict, Verdict::Inconclusive);
        assert_eq!((by("measure-kernel-timing-nrf52840_timing_short").verdict, by("measure-kernel-timing-nrf52840_timing_short").covers.clone()), (Verdict::Pass, vec!["R10.6"]));
        // One campaign item per kind: the short run gives none.
        assert_eq!(got.iter().filter(|i| i.id == "hil-release-timing").count(), 1);
    }

    /// A report that names its binary is judged by that binary, not by the
    /// timing record's: a record of this manifest beside a report of
    /// another binary is still a stale campaign.
    #[test]
    fn r33_4_a_campaign_report_that_names_its_binary_is_bound_to_it() {
        let named = |hash: &str| format!("Verdict: **pass**\n\nOperating duration 3660 s on\n\nTarget binary: {hash}\n");
        assert_eq!(report_binary(&named("sha256:r")), Some("sha256:r".to_string()));
        assert_eq!(report_binary("Verdict: **pass**\n"), None);
        let manifest: serde_json::Value = serde_json::from_str(r#"{"hil_binaries": [{"hash": "sha256:s"}]}"#).unwrap();
        for (report_hash, want) in [("sha256:s", Verdict::Pass), ("sha256:other", Verdict::Inconclusive)] {
            let root = root_with(&[("kernel_timing_nrf52840_stress.json", record("sha256:s", &[("j_rel", "measured")])), ("stress_nrf52840_stress.md", named(report_hash))]);
            let got = items(&root, &manifest);
            let _ = std::fs::remove_dir_all(&root);
            let item = got.iter().find(|i| i.id == "hil-stress").unwrap();
            assert_eq!(item.verdict, want, "{report_hash}: {:?}", item.notes);
            assert!(item.name.contains(report_hash));
        }
    }

    /// A campaign report that says `fail` fails its item.
    #[test]
    fn r18_8_a_failed_campaign_report_fails_the_item() {
        let root = root_with(&[
            ("kernel_timing_nrf52840_stress.json", record("sha256:s", &[("j_rel", "measured")])),
            ("stress_nrf52840_stress.md", "Verdict: **fail**\n\nOperating duration 3660 s on\n".to_string()),
        ]);
        let manifest: serde_json::Value = serde_json::from_str(r#"{"hil_binaries": [{"hash": "sha256:s"}]}"#).unwrap();
        let got = items(&root, &manifest);
        let _ = std::fs::remove_dir_all(&root);
        assert_eq!(got.iter().find(|i| i.id == "hil-stress").map(|i| i.verdict), Some(Verdict::Fail));
    }
    /// R34.2, PAR-02: the entries of a record with fewer than 10,000
    /// measured executions are named with their counts; placeholders are
    /// not counted.
    #[test]
    fn par_02_names_the_entries_measured_fewer_than_ten_thousand_times() {
        let text = r#"{"records": [
            {"item": "a", "method": "measured", "measured": {"runs": 366000}},
            {"item": "b", "method": "measured", "measured": {"runs": 900}},
            {"item": "c", "method": "measured"},
            {"item": "d", "method": "placeholder"}]}"#;
        assert_eq!(below_par_02(text), (vec![("b".to_string(), 900), ("c".to_string(), 0)], 3));
        let params = r#"{"parameters": [{"symbol": "delta_irq", "method": "measured", "measured": {"runs": 10000}}]}"#;
        assert_eq!(below_par_02(params), (vec![], 1));
        assert!(par_02_note("x", text).contains("2 of the 3 measured x have fewer than 10000 executions: b 900, c 0"));
        assert!(par_02_note("x", params).contains("each of the 1 measured x has at least 10000 executions"));
    }

}
