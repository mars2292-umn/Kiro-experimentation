//! Tasks 15.1 and 15.2: one test per rejection rule of R28 (and R13.3,
//! R33.5) and the response-time analysis of R29 (Property 16 is proven in
//! `rta.rs`; these tests check the term derivation and the verdicts on the
//! P1 demo's Task_Model and on hand-computed sets).
#![forbid(unsafe_code)]

use rsk_analyzer::inputs::{self, AnalysedTask, Options};
use rsk_analyzer::rta::{self, Outcome, Term};
use rsk_analyzer::{analyze, load, placeholders, report, response_times, EXIT_FAIL, EXIT_PASS, EXIT_REJECTED};
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
    // Endpoints are accepted since task 21.2 (the demo declares one): their
    // operations are inside the Jobs' bounds and L_kernel, and the explicit
    // terms of R40.2 are reported as omitted until task 21.5.
    let m = model();
    assert!(!m.endpoints.is_empty(), "the demo declares the `telemetry` Endpoint");
    let (w, t) = placeholders::make(&m, HASH, "off");
    let prepared = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap_or_else(|r| panic!("{r:?}"));
    assert!(prepared.omitted_terms.iter().any(|o| o.contains("R40.2") && o.contains("telemetry")), "{:?}", prepared.omitted_terms);
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
        intake: 0,
        stops_overruns: false,
        sporadic: false,
        longest_cs: 0,
        endpoint_cost: 0,
        source_interrupt: false,
        source_endpoint: None,
        defer: false,
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
    // its placeholder WCET, the Budget of 8.75 ms = 218,750 cycles.
    assert_eq!((by("part0.control").b, by("part0.control").b_source.as_str()), (218_750, "part0.sensor#cs:state"));
    assert_eq!(by("part0.echo").b, 218_750);
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
    // set has an FPU-using Task (600), Δ_entry (300, R13.6): 4,500; plus,
    // since task 21.5 (R40.2), its two Endpoint roles' one operation per
    // Job each at max(Δ_send, Δ_recv) (600): 5,100.
    assert_eq!(by("part0.control").o, 5_100);
    // echo (sporadic, deferral policy, no Critical_Section, no Endpoint):
    // the same without the lock and Endpoint terms plus Δ_irq and Δ_stamp:
    // 3,900.
    assert_eq!(by("part0.echo").o, 3_900);
    // log (periodic, no Critical_Section): 3,300, plus its roles' declared
    // operations (`telemetry` sink `ops = 4`, `pingpong` sink 1: 1,500):
    // 4,800.
    assert_eq!(by("part1.log").o, 4_800);
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
    // part1 END_JOB: E = Budget + Δ_detect + Δ_enforce + L_j + O (L_j = 300:
    // no Resource Critical_Section, but since task 21.5 its Endpoint
    // operations are Critical_Sections of Δ_send and Δ_recv, R40.2).
    let log = by("part1.log");
    assert!(log.stops_overruns);
    assert_eq!(log.longest_cs, 300);
    assert_eq!(log.e, log.c + 300 + 300 + 300 + log.o);
}

#[test]
fn r29_6_a_term_that_exceeds_u64_is_rejected_not_saturated() {
    // Δ_release at u64::MAX: O_i of every Task exceeds u64, so `prepare`
    // reports the overflow (R29.6) instead of a saturated term.
    let m = model();
    let (w, mut t) = placeholders::make(&m, HASH, "off");
    t.parameters.iter_mut().find(|p| p.symbol == "delta_release").unwrap().cycles = u64::MAX;
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R29.6", "overflow");
    // A lower Task's Budget + Δ_detect + Δ_enforce beyond u64 overflows B_i
    // of the Tasks it blocks across Partitions (sampler's `stats` reaches
    // log, R29.2), and nothing else.
    let (w, mut t) = placeholders::make(&m, HASH, "off");
    t.parameters.iter_mut().find(|p| p.symbol == "delta_detect").unwrap().cycles = u64::MAX - 100;
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R29.6", "overflow");
}

#[test]
fn r28_1_a_critical_section_site_whose_resource_is_unknown_is_rejected_naming_it() {
    // The Resource `part0.state` renamed in its declaration only: the sites
    // `part0.control#cs:state` and `part0.sensor#cs:state` then name no
    // Resource of part0; the first Task's site is reported.
    let m = model_with("\"id\": \"part0.state\"", "\"id\": \"part0.stat3\"");
    let (w, t) = placeholders::make(&m, HASH, "off");
    assert_rejects(&rejections_of(&m, &w, &t, &manifest(&m), &dev()), "R28.1", "part0.control#cs:state");
}

#[test]
fn r19_2_the_intake_of_a_lower_sporadic_task_interferes_with_every_task() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let p = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap();
    let by = |id: &str| p.tasks.iter().position(|x| x.id == id).unwrap();
    // echo (sporadic, P6) is below control (P7): control's recurrence
    // carries echo's intake, ⌈w / MIT⌉ · Δ_irq (300 placeholder cycles per
    // 500,000-cycle MIT), and nothing else of echo.
    let control = inputs::terms_for(&p.tasks, by("part0.control"));
    let echo_terms: Vec<_> = control.iter().filter(|x| x.task == by("part0.echo")).collect();
    assert_eq!(echo_terms.len(), 1);
    assert!(echo_terms[0].intake);
    assert_eq!((echo_terms[0].term.jitter, echo_terms[0].term.period, echo_terms[0].term.bound), (0, 500_000, 300));
    // sensor (P5) is below echo: echo is in hep(sensor) and interferes by
    // its Jobs (E = Budget + O, the intake in O), not by a separate term.
    let sensor = inputs::terms_for(&p.tasks, by("part0.sensor"));
    let echo_terms: Vec<_> = sensor.iter().filter(|x| x.task == by("part0.echo")).collect();
    assert_eq!(echo_terms.len(), 1);
    assert!(!echo_terms[0].intake);
    assert_eq!(echo_terms[0].term.bound, p.tasks[by("part0.echo")].e);
    // A Periodic_Task has no intake term anywhere.
    assert!(p.tasks.iter().filter(|x| !x.sporadic).all(|x| x.intake == 0));
    let r = response_times(&p.tasks);
    let control_r = r.iter().find(|x| x.id == "part0.control").unwrap();
    assert!(control_r.interference.iter().any(|i| i.task == "part0.echo (intake, R19.2)" && i.e_cycles == 300 && i.jobs == 1));
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

// ------------------------------------------------------------- task 19.3: measured Kernel_Timing_Parameters

/// A measured `j_rel` record as the rig writes it, bound to `hash`.
fn measured_j_rel(hash: &str, max: u64, cycles: u64) -> KernelTiming {
    KernelTiming {
        schema: rsk_model::wcet::KERNEL_TIMING_SCHEMA.into(),
        binary_hash: hash.into(),
        target: "nrf52840/r0p1".into(),
        clock_hz: 64_000_000,
        icache: "off".into(),
        parameters: vec![rsk_model::wcet::KernelTimingParameter {
            symbol: "j_rel".into(),
            cycles,
            method: "measured".into(),
            tool: "rsk-hil release-timing record".into(),
            conditions: vec!["HIL build".into()],
            measured: Some(rsk_model::wcet::Measured { runs: 40_000, max_observed: max, margin_percent: 25, coverage_branch_percent: 0, scenarios: vec!["nrf52840_stress_short (180 s)".into()] }),
        }],
    }
}

/// R13.2, R33.4: a measured record bound to a HIL build the manifest lists
/// (`hil_binaries`) is accepted over the placeholders, the report names
/// the measurement binary and the provenance, and the measured symbol is
/// no longer a placeholder; the same record bound to a binary the manifest
/// does not list is rejected under R28.3. The demo's control Task has 300
/// cycles of slack under placeholder WCETs (its blocking term is sensor's
/// whole Budget), so a jitter of 800 cycles still passes and one of 6,250
/// fails the analysis with the measured provenance stated either way.
#[test]
fn task_19_3_measured_parameters_of_a_listed_hil_build_replace_placeholders() {
    let m = model();
    let mut manifest = manifest(&m);
    manifest["hil_binaries"] = serde_json::json!([{"package": "p1-demo", "target": "p1-demo (nrf52840, stress-short)", "file": "p1-demo", "hash": "sha256:hil", "loadable_bytes": 1, "config_checksum": "0x1"}]);
    for (max, cycles, exit) in [(320u64, 400u64, EXIT_PASS), (5000, 6250, EXIT_FAIL)] {
        let (w, mut t) = placeholders::make(&m, HASH, "off");
        let taken = t.overlay(&measured_j_rel("sha256:hil", max, cycles));
        assert_eq!(taken, vec!["j_rel".to_string()]);
        t.binary_hash = "sha256:hil".into();
        let a = analyze(&m, &w, &t, &manifest, names(), &dev());
        assert_eq!(a.exit, exit, "{:?}", a.report.rejections);
        assert!(a.report.statements.iter().any(|s| s.contains("MEASURED Kernel_Timing_Parameters") && s.contains(&format!("j_rel: {cycles} cycles")) && s.contains(&format!("max {max} +25%"))), "{:?}", a.report.statements);
        assert!(a.report.statements.iter().any(|s| s.contains("HIL build `p1-demo (nrf52840, stress-short)`")), "{:?}", a.report.statements);
        let placeholder_line = a.report.statements.iter().find(|s| s.starts_with("PLACEHOLDER")).unwrap();
        assert!(!placeholder_line.contains("j_rel"), "{placeholder_line}");
        assert!(a.report.tasks.iter().all(|task| task.j_cycles == cycles));
        assert_eq!(a.report.tasks[0].schedulable, exit == EXIT_PASS);
        // The same record of an unlisted binary is stale (R33.4): rejected.
        let unlisted = manifest.as_object().map(|o| { let mut o = o.clone(); o.remove("hil_binaries"); serde_json::Value::Object(o) }).unwrap();
        let r = rejections_of(&m, &w, &t, &unlisted, &dev());
        assert_rejects(&r, "R28.3", "sha256:hil");
    }
}

/// R13.4: a measured parameter whose maximum observed value exceeds its
/// bound is rejected naming the symbol.
#[test]
fn r13_4_an_observed_value_above_the_bound_is_rejected() {
    let m = model();
    let (w, mut t) = placeholders::make(&m, HASH, "off");
    t.overlay(&measured_j_rel(HASH, 11_000, 10_584));
    let r = rejections_of(&m, &w, &t, &manifest(&m), &dev());
    assert_rejects(&r, "R13.4", "j_rel");
}

/// The overlay keeps a larger existing bound (a measurement never lowers
/// a measured or static bound) but replaces a placeholder whatever its
/// size.
#[test]
fn task_19_3_overlay_never_lowers_a_bound_but_replaces_placeholders() {
    let m = model();
    let (_, mut t) = placeholders::make(&m, HASH, "off");
    assert_eq!(t.get("j_rel"), Some(500));
    assert_eq!(t.overlay(&measured_j_rel(HASH, 100, 125)), vec!["j_rel".to_string()]);
    assert_eq!(t.get("j_rel"), Some(125));
    assert!(t.overlay(&measured_j_rel(HASH, 80, 100)).is_empty());
    assert_eq!(t.get("j_rel"), Some(125));
    assert_eq!(t.overlay(&measured_j_rel(HASH, 8467, 10584)), vec!["j_rel".to_string()]);
    assert_eq!(t.get("j_rel"), Some(10584));
}

/// Task 19.1: a measured WCET_Record of a HIL build the manifest lists
/// replaces the placeholder of its item (sensor's Critical_Section site,
/// whose placeholder is sensor's whole Budget) and the blocking term of
/// control falls to it; one of an unlisted binary is rejected.
#[test]
fn task_19_1_a_measured_critical_section_record_replaces_the_placeholder_blocking_term() {
    let m = model();
    let mut manifest = manifest(&m);
    manifest["hil_binaries"] = serde_json::json!([{"package": "p1-demo", "target": "p1-demo (nrf52840, stress-short)", "file": "p1-demo", "hash": "sha256:hil", "loadable_bytes": 1, "config_checksum": "0x1"}]);
    let (mut w, t) = placeholders::make(&m, HASH, "off");
    let site = "part0.sensor#cs:state";
    let cs = WcetRecord {
        item: site.into(),
        wcet_cycles: 20_000,
        method: "measured".into(),
        tool: "rsk-hil Budget-accounting record".into(),
        binary_hash: "sha256:hil".into(),
        target: "nrf52840/r0p1".into(),
        clock_hz: m.clock.cpu_hz,
        icache: "off".into(),
        conditions: vec!["HIL build".into()],
        measured: Some(rsk_model::wcet::Measured { runs: 9000, max_observed: 16_000, margin_percent: 25, coverage_branch_percent: 0, scenarios: vec!["nrf52840_stress_short (180 s)".into()] }),
    };
    // sensor's other site, `in_cs`, keeps its placeholder (the whole Budget)
    // unless measured too: both sites are overlaid.
    let mut in_cs = cs.clone();
    in_cs.item = "part0.sensor#cs:in_cs".into();
    in_cs.wcet_cycles = 8_000;
    let measured = WcetSet { schema: rsk_model::wcet::WCET_SCHEMA.into(), records: vec![cs.clone(), in_cs] };
    assert_eq!(placeholders::overlay_wcet(&mut w, &measured), 2);
    assert!(!w.records.iter().any(|r| r.item == site && r.method == "placeholder"));
    let a = analyze(&m, &w, &t, &manifest, names(), &dev());
    assert_eq!(a.exit, EXIT_PASS, "{:?}", a.report.rejections);
    let control = &a.report.tasks[0];
    assert_eq!(control.b_cycles, 20_000, "{}", control.b_source);
    assert!(a.report.statements.iter().any(|s| s.contains("measured WCET_Records of the HIL build `p1-demo (nrf52840, stress-short)`")), "{:?}", a.report.statements);
    let sensor = a.report.tasks.iter().find(|t| t.id == "part0.sensor").unwrap();
    assert!(sensor.wcet_provenance.iter().any(|p| p.starts_with(site) && p.contains("measured")), "{:?}", sensor.wcet_provenance);
    // Unlisted: rejected under R28.3.
    let mut w2 = w.clone();
    w2.records.iter_mut().filter(|r| r.item == site).for_each(|r| r.binary_hash = "sha256:elsewhere".into());
    let r = rejections_of(&m, &w2, &t, &manifest, &dev());
    assert_rejects(&r, "R28.3", "sha256:elsewhere");
}

// ------------------------------------------------------------- R40.2, R41.4 (task 21.5)

/// R40.2: an Endpoint operation is a Critical_Section at the Kernel level
/// in the blocking term of every higher Task, costing its WCET whatever the
/// holder's Partition does with Overruns (the operation runs to its end
/// inside the SVC, R40.6), while a Resource site of a stopping Partition
/// costs Budget + Δ_detect + Δ_enforce; the declared operations per Job are
/// charged in O_i and cost no lock entry and exit.
#[test]
fn r40_2_endpoint_operations_are_kernel_level_critical_sections() {
    use rsk_analyzer::terms::{blocking, overhead, site_stats, Params, Site, TaskIn};
    let p = Params { release: 1, dispatch: 1, complete: 1, wake: 1, timer: 1, fp_save: 0, fp_restore: 0, lock_in: 10, lock_out: 10, detect: 100, enforce: 100, hm: 1, irq: 1, stamp: 1, entry: 1, l_kernel: 1_000, j_rel: 0, restart: 0, stop: 0, send: 3_000, recv: 2_500 };
    let lo = TaskIn { priority: 2, partition: 1, budget: 50_000, stops_overruns: true, periodic: true, defer: false, endpoint_cost: 4 * 3_000 };
    let hi = TaskIn { priority: 7, partition: 0, budget: 1_000, stops_overruns: false, periodic: true, defer: false, endpoint_cost: 0 };
    let tasks = vec![lo, hi];
    // The send of `lo`'s role blocks `hi` by Δ_send, above the Kernel floor.
    let sites = vec![Site { task: 0, ceiling: u8::MAX, wcet: p.send, endpoint: true }, Site { task: 0, ceiling: u8::MAX, wcet: p.recv, endpoint: true }];
    let b = blocking(&tasks, &sites, 1, p).unwrap();
    assert_eq!((b.b, b.site), (3_000, Some(0)));
    // A Resource site of the same Task, in another Partition that stops
    // overrunning Jobs, costs the Budget rule instead.
    let sites = vec![Site { task: 0, ceiling: 7, wcet: 3_000, endpoint: false }];
    let b = blocking(&tasks, &sites, 1, p).unwrap();
    assert_eq!((b.b, b.site), (50_200, Some(0)));
    // Sites: the Endpoint ones count for L_i, not for the lock cost of O_i;
    // the operations per Job are in O_i.
    let sites = vec![Site { task: 0, ceiling: u8::MAX, wcet: 3_000, endpoint: true }, Site { task: 0, ceiling: 7, wcet: 2_000, endpoint: false }];
    let st = site_stats(&sites, 0);
    assert_eq!((st.count, st.longest), (1, 3_000));
    let with = overhead(lo, p, st.count, false).unwrap();
    let without = overhead(TaskIn { endpoint_cost: 0, ..lo }, p, st.count, false).unwrap();
    assert_eq!(with - without, 4 * 3_000);
    assert_eq!(without - overhead(TaskIn { endpoint_cost: 0, ..lo }, p, 0, false).unwrap(), 20, "one lock entry and exit per Resource site");
}

/// R40.2 on the demo: `log` (role `sink` of `telemetry`, `ops = 4`, and of
/// `pingpong`) carries 4 + 1 operations × max(Δ_send, Δ_recv) in O_i, and
/// the sites appear in the blocking provenance when they determine B_i.
#[test]
fn r40_2_the_demo_charges_the_declared_operations_per_job() {
    let m = model();
    let (w, mut t) = placeholders::make(&m, HASH, "off");
    let op = |t: &KernelTiming, s: &str| t.parameters.iter().find(|p| p.symbol == s).unwrap().cycles;
    let base = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap().tasks;
    let log = base.iter().find(|x| x.id == "part1.log").unwrap();
    assert_eq!(log.endpoint_cost, 5 * op(&t, "delta_send").max(op(&t, "delta_recv")));
    let control = base.iter().find(|x| x.id == "part0.control").unwrap();
    assert_eq!(control.endpoint_cost, 2 * op(&t, "delta_send").max(op(&t, "delta_recv")));
    assert!(base.iter().filter(|x| x.id == "part0.echo" || x.id == "part1.pulse").all(|x| x.endpoint_cost == 0));
    // A Δ_send above the Kernel floor becomes the blocking term of every
    // Task above `log`, named as the site.
    for p in &mut t.parameters {
        if p.symbol == "delta_send" {
            p.cycles = 1_000_000;
        }
    }
    let heavy = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap().tasks;
    let sensor = heavy.iter().find(|x| x.id == "part0.sensor").unwrap();
    assert_eq!(sensor.b, 1_000_000);
    assert!(sensor.b_source.contains("part1.log:endpoint:") && sensor.b_source.contains(":send"), "{}", sensor.b_source);
}

/// R41.4: the end-to-end bound of the demo's chain is R_control + T_log +
/// R_log (a Periodic_Task samples the Endpoint once per period), PASS
/// within its deadline; a chain into a Sporadic_Task released by the hop's
/// Endpoint adds its MIT under `defer` and has no bound under `discard` or
/// under another Release_Source; a chain that starts at an interrupt event
/// adds Δ_stamp; a hand-edited chain the model's Endpoints do not connect
/// is rejected naming R41.4.
#[test]
fn r41_4_chain_bounds_follow_the_hops() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!(a.exit, EXIT_PASS);
    let c = a.report.chains.iter().find(|c| c.id == "telemetry_path").expect("the demo declares the chain");
    let r = |id: &str| a.report.tasks.iter().find(|x| x.id == id).unwrap();
    let t_log = r("part1.log").period_cycles;
    assert_eq!(c.bound_cycles, r("part0.control").r_cycles + t_log + r("part1.log").r_cycles);
    assert_eq!((c.result.as_str(), c.tasks.as_slice()), ("PASS", &["part0.control".to_string(), "part1.log".to_string()][..]));
    assert!(c.bound_cycles <= c.deadline_cycles);
    assert!(report::render_text(&a.report).contains("End-to-end chains (R41.4)"));

    // A hand-edited model: `log` becomes a Sporadic_Task released by its
    // `telemetry` role (MIT 40 ms, `defer`): the bound is R_control + MIT +
    // R_log; under `discard` no bound.
    let sporadic_log = |policy: &str, source: serde_json::Value| -> TaskModel {
        let mut v = serde_json::to_value(&m).unwrap();
        let task = v["tasks"].as_array_mut().unwrap().iter_mut().find(|x| x["id"] == "part1.log").unwrap();
        let obj = task.as_object_mut().unwrap();
        obj.insert("kind".into(), "sporadic".into());
        obj.remove("offset_ticks");
        let period = obj.remove("period_ticks").unwrap();
        obj.insert("mit_ticks".into(), period);
        obj.insert("mit_policy".into(), policy.into());
        obj.insert("source".into(), source);
        serde_json::from_value(v).unwrap()
    };
    let m2 = sporadic_log("defer", serde_json::json!({"endpoint": "telemetry", "role": "sink"}));
    let a2 = analyze(&m2, &w, &t, &manifest(&m2), names(), &dev());
    let c2 = a2.report.chains.iter().find(|c| c.id == "telemetry_path").unwrap();
    let r2 = |id: &str| a2.report.tasks.iter().find(|x| x.id == id).unwrap();
    assert_eq!(c2.bound_cycles, r2("part0.control").r_cycles + t_log + r2("part1.log").r_cycles);
    assert!(c2.terms.iter().any(|x| x.contains("MIT of `part1.log`")), "{:?}", c2.terms);
    let m3 = sporadic_log("discard", serde_json::json!({"endpoint": "telemetry", "role": "sink"}));
    let a3 = analyze(&m3, &w, &t, &manifest(&m3), names(), &dev());
    let c3 = a3.report.chains.iter().find(|c| c.id == "telemetry_path").unwrap();
    assert_eq!((c3.result.as_str(), c3.bound_cycles, a3.exit), ("UNBOUNDED", u64::MAX, EXIT_FAIL));
    let m4 = sporadic_log("defer", serde_json::json!({"signal": "tick"}));
    let a4 = analyze(&m4, &w, &t, &manifest(&m4), names(), &dev());
    let c4 = a4.report.chains.iter().find(|c| c.id == "telemetry_path").unwrap();
    assert_eq!(c4.result, "UNBOUNDED");
    assert!(c4.terms.iter().any(|x| x.contains("not released by its role")), "{:?}", c4.terms);

    // A chain that starts at an interrupt event: `pulse` (released by the
    // dual timer) in `control`'s place on the `telemetry` Endpoint.
    let mut v = serde_json::to_value(&m).unwrap();
    v["endpoints"][0]["roles"][0]["task"] = "part1.pulse".into();
    v["chains"][0]["hops"][0]["from"] = "part1.pulse".into();
    let m5: TaskModel = serde_json::from_value(v).unwrap();
    let a5 = analyze(&m5, &w, &t, &manifest(&m5), names(), &dev());
    let c5 = a5.report.chains.iter().find(|c| c.id == "telemetry_path").unwrap();
    let stamp = t.parameters.iter().find(|p| p.symbol == "delta_stamp").unwrap().cycles;
    let r5 = |id: &str| a5.report.tasks.iter().find(|x| x.id == id).unwrap();
    assert_eq!(c5.bound_cycles, stamp + r5("part1.pulse").r_cycles + t_log + r5("part1.log").r_cycles);
    assert!(c5.terms[0].starts_with("Δ_stamp"), "{:?}", c5.terms);

    // A hop no Endpoint connects is rejected.
    let mut v = serde_json::to_value(&m).unwrap();
    v["chains"][0]["hops"][0]["endpoint"] = "pingpong".into();
    v["chains"][0]["hops"][0]["from"] = "part0.sensor".into();
    let m6: TaskModel = serde_json::from_value(v).unwrap();
    let a6 = analyze(&m6, &w, &t, &manifest(&m6), names(), &dev());
    assert_eq!(a6.exit, EXIT_REJECTED);
    assert!(a6.report.rejections.iter().any(|r| r.contains("R41.4") && r.contains("does not connect")), "{:?}", a6.report.rejections);
}
