//! Tasks 15.1 and 15.2: one test per rejection rule of R28 (and R13.3,
//! R33.5) and the response-time analysis of R29 (Property 16 is proven in
//! `rta.rs`; these tests check the term derivation and the verdicts on the
//! P1 demo's Task_Model and on hand-computed sets).
#![forbid(unsafe_code)]

use rsk_analyzer::inputs::{self, AnalysedTask, Options};
use rsk_analyzer::rta::{self, Outcome, Term};
use rsk_analyzer::{analyze, load, placeholders, response_times, EXIT_FAIL, EXIT_PASS, EXIT_REJECTED};
use rsk_model::report::Inputs;
use rsk_model::task_model::TaskModel;
use rsk_model::{KernelTiming, WcetRecord, WcetSet};

const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");
const HASH: &str = "sha256:demo";

fn model() -> TaskModel {
    rsk_model::read_task_model(DEMO).unwrap()
}

fn model_with(find: &str, replace: &str) -> TaskModel {
    assert_eq!(DEMO.matches(find).count(), 1, "`{find}` must occur once in the demo model");
    rsk_model::read_task_model(&DEMO.replacen(find, replace, 1)).unwrap_or_else(|e| panic!("{e}"))
}

/// A schema-valid Build_Manifest for the demo model.
fn manifest(model: &TaskModel) -> serde_json::Value {
    serde_json::json!({
        "schema": "rsk-build-manifest/1",
        "source": {"revision": "0000000000000000000000000000000000000000", "dirty": false, "tracked": true},
        "lock_file": {"hash": "sha256:lock"},
        "toolchain": {"rustc": "rustc 1.95.0", "cargo": "cargo 1.95.0"},
        "profile": {"version": model.profile.version, "variant": model.profile.variant},
        "build": {"target": "thumbv7em-none-eabihf", "cargo_profile": "release", "panic": "abort", "icache": "off"},
        "generated_config_checksum": model.config_checksum,
        "binaries": [{"package": "p1-demo", "target": "p1-demo", "file": "p1-demo", "hash": HASH, "loadable_bytes": 1, "config_checksum": model.config_checksum}]
    })
}

fn names() -> Inputs {
    Inputs { task_model: "m".into(), wcet_records: "w".into(), kernel_timing: "k".into(), build_manifest: "b".into() }
}

fn dev() -> Options {
    Options { allow_placeholders: true, prefer_method: None }
}

/// Runs `prepare` on the model with placeholder timing and returns the rejections.
fn rejections_of(model: &TaskModel, wcet: &WcetSet, timing: &KernelTiming, manifest: &serde_json::Value, options: &Options) -> Vec<String> {
    match inputs::prepare(model, wcet, timing, manifest, options) {
        Ok(_) => vec![],
        Err(r) => r.iter().map(ToString::to_string).collect(),
    }
}

fn assert_rejects(rejections: &[String], rule: &str, fragment: &str) {
    assert!(rejections.iter().any(|r| r.starts_with(&format!("[{rule}]")) && r.contains(fragment)), "expected [{rule}] ... {fragment} in {rejections:#?}");
}

// ------------------------------------------------------------- R28.1

#[test]
fn r28_1_schema_violations_are_reported_with_their_json_path() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let good = load(DEMO, &rsk_model::print(&w), &rsk_model::print(&t), &manifest(&m).to_string());
    assert!(good.is_ok());
    let bad_model = DEMO.replacen("\"priority\": 7", "\"priority\": 9", 1);
    let err = load(&bad_model, &rsk_model::print(&w), &rsk_model::print(&t), &manifest(&m).to_string()).err().unwrap();
    assert!(err.iter().any(|e| e.starts_with("Task_Model:") && e.contains("/tasks/0/priority")), "{err:?}");
    let bad_manifest = "{\"schema\": \"rsk-build-manifest/0\"}";
    let err = load(DEMO, &rsk_model::print(&w), &rsk_model::print(&t), bad_manifest).err().unwrap();
    assert!(err.iter().any(|e| e.starts_with("Build_Manifest:") && e.contains("not supported")), "{err:?}");
    let bad_wcet = rsk_model::print(&w).replacen("\"wcet_cycles\"", "\"wcet_cycle\"", 1);
    let err = load(DEMO, &bad_wcet, &rsk_model::print(&t), &manifest(&m).to_string()).err().unwrap();
    assert!(err.iter().any(|e| e.starts_with("WCET_Records:") && e.contains("/records/0")), "{err:?}");
}

// ------------------------------------------------------------- R28.2, R13.3

#[test]
fn r28_2_a_task_entry_or_critical_section_without_a_record_fails_naming_it() {
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    w.records.retain(|r| r.item != "part0.control");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.2", "part0.control");
    let (mut w, _) = placeholders::make(&m, HASH, "off");
    w.records.retain(|r| r.item != "part0.sensor#cs:state");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.2", "part0.sensor#cs:state");
}

#[test]
fn r13_3_a_missing_kernel_timing_parameter_is_rejected_naming_it() {
    let m = model();
    let (w, mut t) = placeholders::make(&m, HASH, "off");
    t.parameters.retain(|p| p.symbol != "delta_timer");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R13.3", "delta_timer");
}

// ------------------------------------------------------------- R28.3

#[test]
fn r28_3_records_of_another_binary_clock_or_cache_setting_are_rejected() {
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    w.records[0].binary_hash = "sha256:other".into();
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.3", "binary hash sha256:other");
    let (mut w, _) = placeholders::make(&m, HASH, "off");
    w.records[0].clock_hz = 64_000_000;
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.3", "clock 64000000 Hz");
    let (mut w, _) = placeholders::make(&m, HASH, "off");
    w.records[0].icache = "on".into();
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.3", "icache `on`");
    let (w, mut t2) = placeholders::make(&m, HASH, "off");
    t2.binary_hash = "sha256:other".into();
    assert_rejects(&rejections_of(&m, &w, &t2, &manifest(&m), &dev()), "R28.3", "Kernel_Timing_Parameters' binary hash");
    // Placeholders are rejected unless the development option is given (R33.1).
    let r = rejections_of(&m, &w, &t, &manifest(&m), &Options::default());
    assert_rejects(&r, "R28.3", "placeholder");
    assert_rejects(&r, "R13.2", "placeholder");
}

// ------------------------------------------------------------- R28.4

#[test]
fn r28_4_a_wcet_above_the_budget_fails_naming_the_task() {
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    let control = w.records.iter_mut().find(|r| r.item == "part0.control").unwrap();
    control.wcet_cycles = 25_001; // the Budget is 25,000 cycles (1 ms at 25 MHz)
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.4", "part0.control");
}

// ------------------------------------------------------------- R28.5

#[test]
fn r28_5_a_task_on_another_core_is_rejected_citing_ng_01() {
    let m = model_with("\"id\": \"part0.control\",\n      \"core\": 0", "\"id\": \"part0.control\",\n      \"core\": 1");
    let (w, t) = placeholders::make(&m, HASH, "off");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "NG-01", "part0.control");
}

// ------------------------------------------------------------- R28.6

#[test]
fn r28_6_unsupported_variant_version_or_construct_is_rejected_naming_it() {
    let m = model_with("\"variant\": \"Ravenscar\"", "\"variant\": \"Jorvik\"");
    let (w, t) = placeholders::make(&m, HASH, "off");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.6", "Jorvik");
    let m = model_with(&format!("\"version\": \"{}\"", rsk_analyzer::profile_version::PROFILE_VERSION), "\"version\": \"0.1\"");
    let (w, t) = placeholders::make(&m, HASH, "off");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.6", "Profile 0.1");
    let m = model_with(
        "\"endpoints\": []",
        "\"endpoints\": [{\"id\": \"link\", \"cores\": [0, 1], \"capacity\": 4, \"k\": 1, \"ceiling\": \"kernel\", \"payload_bytes\": 8}]",
    );
    let (w, t) = placeholders::make(&m, HASH, "off");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.6", "Endpoints");
}

// ------------------------------------------------------------- R28.7

#[test]
fn r28_7_the_larger_record_is_used_unless_a_method_is_designated_and_the_record_is_reported() {
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    let base = w.records.iter().find(|r| r.item == "part0.control").unwrap().clone();
    w.records.retain(|r| r.item != "part0.control");
    w.records.push(WcetRecord { method: "measured".into(), wcet_cycles: 20_000, tool: "rsk-wcet".into(), ..base.clone() });
    w.records.push(WcetRecord { method: "static".into(), wcet_cycles: 22_000, tool: "static-tool".into(), ..base.clone() });
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    let control = p.tasks.iter().find(|x| x.id == "part0.control").unwrap();
    assert_eq!(control.wcet, 22_000);
    assert!(control.wcet_provenance[0].contains("static"), "{:?}", control.wcet_provenance);
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &Options { allow_placeholders: true, prefer_method: Some("measured".into()) }).unwrap();
    let control = p.tasks.iter().find(|x| x.id == "part0.control").unwrap();
    assert_eq!(control.wcet, 20_000);
    assert!(control.wcet_provenance[0].contains("measured"));
}

// ------------------------------------------------------------- R28.8

#[test]
fn r28_8_a_checksum_mismatch_or_absence_is_rejected() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let mut bad = manifest(&m);
    bad["generated_config_checksum"] = serde_json::json!("0x00000001");
    assert_rejects(&rejections_of(&m, &w, &t, &bad, &dev()), "R28.8", "differs");
    let mut absent = manifest(&m);
    absent.as_object_mut().unwrap().remove("generated_config_checksum");
    assert_rejects(&rejections_of(&m, &w, &t, &absent, &dev()), "R28.8", "records no Generated_Config checksum");
}

// ------------------------------------------------------------- R33.5

#[test]
fn r33_5_an_observed_maximum_above_a_static_bound_is_rejected() {
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    let base = w.records.iter().find(|r| r.item == "part0.control").unwrap().clone();
    w.records.retain(|r| r.item != "part0.control");
    w.records.push(WcetRecord {
        method: "measured".into(),
        wcet_cycles: 20_000,
        measured: Some(rsk_model::wcet::Measured { runs: 100, max_observed: 19_000, margin_percent: 5, coverage_branch_percent: 100, scenarios: vec![] }),
        ..base.clone()
    });
    w.records.push(WcetRecord { method: "static".into(), wcet_cycles: 18_000, ..base.clone() });
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R33.5", "part0.control");
}

// ------------------------------------------------------------- R29.1, R29.5: the recurrence

fn task(id: &str, priority: u8, c: u64, period: u64, deadline: u64) -> AnalysedTask {
    AnalysedTask {
        id: id.into(),
        partition: "p".into(),
        level: "A".into(),
        priority,
        period_cycles: period,
        deadline_cycles: deadline,
        c,
        b: 0,
        b_source: String::new(),
        o: 0,
        j: 0,
        e: c,
        stops_overruns: false,
        sporadic: false,
        longest_cs: 0,
        wcet_provenance: vec![],
        wcet: c,
    }
}

/// The textbook set C = (1, 2, 3), T = (4, 6, 12): R = (1, 3, 10).
#[test]
fn r29_1_the_least_fixed_point_matches_the_hand_computation() {
    let tasks = vec![task("a", 3, 1, 4, 4), task("b", 2, 2, 6, 6), task("c", 1, 3, 12, 12)];
    let r: Vec<(u64, bool)> = response_times(&tasks).iter().map(|w| (w.r, w.schedulable)).collect();
    assert_eq!(r, vec![(1, true), (3, true), (10, true)]);
    // Blocking, overhead, and jitter add to the base and shift R (R29.1).
    let mut with_terms = tasks.clone();
    with_terms[2].b = 1;
    with_terms[2].o = 1;
    with_terms[2].j = 1;
    let w = &response_times(&with_terms)[2];
    // base 5: 5 -> 5+2+2=9 -> 5+3+4=12 -> 5+3+4=12; R = 12 + J = 13 > D = 12.
    assert_eq!((w.r, w.schedulable), (13, false));
}

/// Property 16 cross-check: the exec fixed point equals the least fixed
/// point found by a brute-force search on small sets.
#[test]
fn r32_1_fixpoint_agrees_with_brute_force_on_small_sets() {
    let mut seed = 12345u64;
    let mut next = || {
        seed = seed.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        (seed >> 33) as u64
    };
    for _ in 0..200 {
        let n = (next() % 3) as usize + 1;
        let terms: Vec<Term> = (0..n).map(|_| Term { jitter: next() % 4, period: next() % 9 + 1, bound: next() % 4 }).collect();
        let base = next() % 5 + 1;
        let deadline = 400;
        let brute = (0..=deadline).find(|&w| {
            let s: u64 = base + terms.iter().map(|t| (w + t.jitter).div_ceil(t.period) * t.bound).sum::<u64>();
            s == w
        });
        match (rta::fixpoint(base, &terms, 0, deadline), brute) {
            (Outcome::Fixpoint(w), Some(b)) => assert_eq!(w, b, "{terms:?} base {base}"),
            (Outcome::NotSchedulable(w), None) => assert!(w > deadline, "{terms:?}"),
            (Outcome::NotSchedulable(w), Some(b)) => assert!(w <= b && w > deadline, "{terms:?} w {w} brute {b}"),
            (r, b) => panic!("{terms:?} base {base}: {r:?} vs brute {b:?}"),
        }
    }
}

// ------------------------------------------------------------- R29.6, R29.7

#[test]
fn r29_6_overflow_is_an_error_not_a_result() {
    let terms = vec![Term { jitter: 0, period: 1, bound: u64::MAX / 2 }, Term { jitter: 0, period: 1, bound: u64::MAX / 2 }];
    assert_eq!(rta::fixpoint(10, &terms, 0, u64::MAX), Outcome::Overflow);
    let tasks = vec![task("a", 2, u64::MAX, 10, u64::MAX), task("b", 1, 1, 10, u64::MAX)];
    let w = &response_times(&tasks)[1];
    assert!(w.overflow && !w.schedulable);
    let m = model();
    let (wc, t) = placeholders::make(&m, HASH, "off");
    let mut tasks = inputs::prepare(&m, &wc, &t, &manifest(&m), &dev()).unwrap().tasks;
    tasks[0].e = u64::MAX;
    let a = response_times(&tasks);
    assert!(a.iter().skip(1).all(|w| w.overflow), "{a:?}");
}

#[test]
fn r29_7_iteration_stops_at_the_deadline_with_a_lower_bound() {
    let terms = vec![Term { jitter: 0, period: 4, bound: 2 }];
    // w = 3 + 2*ceil(w/4): 3 -> 5 -> 7 -> 7; with D = 6 the iteration stops at 7 > 6.
    assert_eq!(rta::fixpoint(3, &terms, 0, 100), Outcome::Fixpoint(7));
    assert_eq!(rta::fixpoint(3, &terms, 0, 6), Outcome::NotSchedulable(7));
    // The Task's own jitter counts against the deadline (R = J + w).
    assert_eq!(rta::fixpoint(3, &terms, 1, 7), Outcome::NotSchedulable(7));
    assert_eq!(rta::fixpoint(3, &terms, 1, 8), Outcome::Fixpoint(7));
}

// ------------------------------------------------------------- R29.2, R29.3, R29.4: the terms on the demo

#[test]
fn r29_2_blocking_is_the_longest_lower_priority_critical_section_with_a_ceiling_at_least_the_priority() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    let by = |id: &str| p.tasks.iter().find(|x| x.id == id).unwrap();
    // control (P7) and echo (P6): sensor (P5) holds `state` (Ceiling 7) for
    // its placeholder WCET, the Budget of 8.8 ms = 220,000 cycles.
    assert_eq!((by("part0.control").b, by("part0.control").b_source.as_str()), (220_000, "part0.sensor#cs:state"));
    assert_eq!(by("part0.echo").b, 220_000);
    // sensor itself and the Tasks below the Ceiling's reach with no lower
    // locker: the Kernel-level terms (L_kernel 2,000 or the stop/restart
    // work 5,000 of R14.10, whichever is larger).
    assert_eq!((by("part0.sensor").b, by("part0.sensor").b_source.as_str()), (5_000, "Partition stop/restart work (R14.10)"));
    // The cross-Partition clause of R29.2: a site of a lower Task of another
    // Partition whose Partition stops overrunning Jobs (part2: `overrun =
    // END_JOB`) blocks for Budget_l + Δ_detect + Δ_enforce. sampler (part2,
    // P1) holds `stats` (Ceiling 3), which reaches log (part1, P2): 12,500 +
    // 300 + 300.
    assert_eq!((by("part1.log").b, by("part1.log").b_source.as_str()), (13_100, "part2.sampler#cs:stats"));
    // Within part2 the same site blocks filter (P3) for its placeholder
    // WCET, the Budget of 500 us = 12,500 cycles.
    assert_eq!((by("part2.filter").b, by("part2.filter").b_source.as_str()), (12_500, "part2.sampler#cs:stats"));
    // pulse (part1, P4) is above every Ceiling that a lower Task of another
    // Partition can raise (part2's is 3); sampler is the lowest Task.
    assert_eq!(by("part1.pulse").b, 5_000);
    assert_eq!(by("part2.sampler").b, 5_000);
    // Every accessor of a Resource is a Task of the declaring Partition (PR-03).
    assert!(m.resources.iter().all(|r| r.accessors.iter().all(|a| a.starts_with(&format!("{}.", r.partition)))));
}

#[test]
fn r29_3_the_kernel_overhead_of_a_job_sums_the_table_13_1_terms() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    let by = |id: &str| p.tasks.iter().find(|x| x.id == id).unwrap();
    // Placeholders: every Δ is 300 cycles. control: release + dispatch +
    // complete + wake (1,200), three timed events (900), two Critical_Section
    // sites' lock in + out (1,200), Δ_hm (300), FP save + restore because the
    // set has an FPU-using Task (600): 4,200.
    assert_eq!(by("part0.control").o, 4_200);
    // echo (sporadic, deferral policy, no Critical_Section): the same
    // without the lock terms plus Δ_irq and Δ_stamp: 3,600.
    assert_eq!(by("part0.echo").o, 3_600);
    // log (periodic, no Critical_Section): 3,000.
    assert_eq!(by("part1.log").o, 3_000);
    // J is J_rel for every Task (Table 13-1).
    assert!(p.tasks.iter().all(|x| x.j == 500));
}

#[test]
fn r29_4_interference_bounds_depend_on_the_partition_overrun_response() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    let by = |id: &str| p.tasks.iter().find(|x| x.id == id).unwrap();
    // part0 RECORD_AND_CONTINUE: E = Budget + O.
    let control = by("part0.control");
    assert!(!control.stops_overruns);
    assert_eq!(control.e, control.c + control.o);
    // part1 END_JOB: E = Budget + Δ_detect + Δ_enforce + L_j + O (L_j = 0: no Critical_Section).
    let log = by("part1.log");
    assert!(log.stops_overruns);
    assert_eq!(log.e, log.c + 300 + 300 + 0 + log.o);
}

// ------------------------------------------------------------- R29.8

#[test]
fn r29_8_ticks_convert_with_periods_rounded_down_and_execution_times_up() {
    // 1,001 ticks at 32,768 Hz on a 64 MHz core = 1,955,078.125 cycles.
    let m = model_with("\"tick_hz\": 25000000", "\"tick_hz\": 32768");
    let m = TaskModel { clock: rsk_model::task_model::Clock { cpu_hz: 64_000_000, tick_hz: 32768 }, ..m };
    let mut m = m;
    m.tasks[0].period_ticks = Some(1001);
    m.tasks[0].deadline_ticks = 1001;
    let (mut w, mut t) = placeholders::make(&m, HASH, "off");
    for r in &mut w.records {
        r.clock_hz = 64_000_000;
    }
    t.clock_hz = 64_000_000;
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    assert_eq!(p.tasks[0].period_cycles, 1_955_078);
    assert_eq!(p.tasks[0].deadline_cycles, 1_955_078);
}

// ------------------------------------------------------------- R29.9

#[test]
fn r29_9_over_utilization_fails_without_iteration() {
    let tasks = vec![task("a", 2, 6, 10, 10), task("b", 1, 6, 10, 10)];
    assert!(inputs::utilization_exceeds_one(&tasks));
    assert_eq!(inputs::utilization_percent(&tasks), 120);
    let m = model();
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    // Budgets are the placeholders' WCETs; a huge Budget cannot be declared
    // here, so the set is driven over 100 % through the records: no. The
    // Budget is the model's: raise control's to its whole period.
    let m = model_with("\"budget_cycles\": 25000,\n      \"fpu\": false,\n      \"resources\": [\n        \"part0.state\",\n        \"part0.in_cs\"\n      ],\n      \"raises\"", "\"budget_cycles\": 250000,\n      \"fpu\": false,\n      \"resources\": [\n        \"part0.state\",\n        \"part0.in_cs\"\n      ],\n      \"raises\"");
    w.records.iter_mut().filter(|r| r.item.starts_with("part0.control")).for_each(|r| r.wcet_cycles = 250_000);
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!(a.exit, EXIT_FAIL);
    assert!(a.report.rejections.iter().any(|r| r.contains("R29.9")), "{:?}", a.report.rejections);
    assert!(a.report.tasks.iter().all(|x| x.r_cycles == 0 && !x.schedulable));
}

// ------------------------------------------------------------- R29.5: the verdict

#[test]
fn r29_5_pass_only_when_every_task_meets_its_deadline_and_nothing_was_rejected() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!(a.exit, EXIT_PASS);
    assert!(a.report.tasks.iter().all(|x| x.schedulable && x.r_cycles <= x.deadline_cycles));
    // A deadline below the response time fails the set and names the Task.
    let short = model_with("\"deadline_ticks\": 2500000,\n      \"budget_cycles\": 12500,\n      \"fpu\": false", "\"deadline_ticks\": 25000,\n      \"budget_cycles\": 12500,\n      \"fpu\": false");
    let (w, t) = placeholders::make(&short, HASH, "off");
    let a = analyze(&short, &w, &t, &manifest(&short), names(), &dev());
    assert_eq!(a.exit, EXIT_FAIL);
    let sampler = a.report.tasks.iter().find(|x| x.id == "part2.sampler").unwrap();
    assert!(!sampler.schedulable && sampler.r_cycles > sampler.deadline_cycles);
    // A rejected input never passes.
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    w.records.pop();
    assert_eq!(analyze(&m, &w, &t, &manifest(&m), names(), &dev()).exit, EXIT_REJECTED);
}
