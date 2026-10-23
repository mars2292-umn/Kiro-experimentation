//! Task 15.3: the report (R30) and the R18.9 statement. Golden files under
//! `tests/golden` hold the JSON and text reports of the P1 demo with
//! placeholder timing and fixed input names; `UPDATE_GOLDEN=1 cargo test`
//! rewrites them after an intended change. The command-line exit statuses
//! are checked on the built binary (R30.4), and identical inputs give
//! identical reports (R30.5).
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::Command;

use rsk_analyzer::inputs::Options;
use rsk_analyzer::{analyze, placeholders, report, EXIT_FAIL, EXIT_INTERNAL, EXIT_PASS, EXIT_REJECTED};
use rsk_model::report::Inputs;
use rsk_model::task_model::TaskModel;

const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");
const HASH: &str = "sha256:demo";

fn model() -> TaskModel {
    rsk_model::read_task_model(DEMO).unwrap()
}

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
    Inputs {
        task_model: "task_model.json (fnv:0001)".into(),
        wcet_records: "wcet.json (fnv:0002)".into(),
        kernel_timing: "kernel_timing.json (fnv:0003)".into(),
        build_manifest: "build-manifest.json (fnv:0004)".into(),
    }
}

fn dev() -> Options {
    Options { allow_placeholders: true, prefer_method: None }
}

fn golden(name: &str, actual: &str) {
    let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/golden").join(name);
    if std::env::var_os("UPDATE_GOLDEN").is_some() {
        std::fs::write(&path, actual).unwrap();
        return;
    }
    let expected = std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e} (run with UPDATE_GOLDEN=1 to create it)", path.display()));
    assert!(expected == actual, "{} differs from the golden file; diff the output or run with UPDATE_GOLDEN=1 after an intended change:\n{actual}", path.display());
}

#[test]
fn r30_1_the_demo_report_matches_its_golden_files() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!(a.exit, EXIT_PASS);
    golden("p1-demo.report.json", &report::render_json(&a.report));
    golden("p1-demo.report.txt", &report::render_text(&a.report));
    // The fields R30.1 names, per Task.
    for task in &a.report.tasks {
        assert!(!task.partition.is_empty() && !task.level.is_empty() && task.priority > 0);
        assert!(task.period_cycles > 0 && task.deadline_cycles > 0 && task.c_cycles > 0);
        assert!(!task.b_source.is_empty(), "B_i names the Critical_Section or the Kernel term that determines it");
        assert_eq!(task.slack_cycles, task.deadline_cycles - task.r_cycles);
        assert!(!task.wcet_provenance.is_empty());
    }
    assert!(a.report.statements.iter().any(|s| s.contains("R27.5")) && a.report.statements.iter().any(|s| s.contains("R18.9")));
}

#[test]
fn r30_1_a_not_schedulable_task_reports_the_lower_bound_and_no_slack() {
    let short = DEMO.replacen("\"deadline_ticks\": 2500000,\n      \"budget_cycles\": 12500", "\"deadline_ticks\": 25000,\n      \"budget_cycles\": 12500", 1);
    let m = rsk_model::read_task_model(&short).unwrap();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!((a.exit, a.report.result.as_str()), (EXIT_FAIL, "FAIL"));
    let sampler = a.report.tasks.iter().find(|x| x.id == "part2.sampler").unwrap();
    assert!(!sampler.schedulable && sampler.r_cycles > sampler.deadline_cycles && sampler.slack_cycles == 0);
    assert!(report::render_text(&a.report).contains("NOT schedulable (lower bound)"));
}

#[test]
fn r30_2_r30_3_hashes_versions_and_provenance_are_recorded() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    let json = report::render_json(&a.report);
    for h in ["fnv:0001", "fnv:0002", "fnv:0003", "fnv:0004"] {
        assert!(json.contains(h), "input hash {h} missing");
    }
    assert_eq!(a.report.profile.version, m.profile.version);
    assert_eq!(a.report.profile.variant, "Ravenscar");
    assert_eq!(a.report.analyzer, rsk_analyzer::VERSION);
    // Every WCET used names its record's method, tool, and conditions (R30.3).
    for task in &a.report.tasks {
        for p in &task.wcet_provenance {
            assert!(p.contains("placeholder") && p.contains(placeholders::TOOL) && p.contains("conditions:"), "{p}");
        }
    }
}

#[test]
fn r30_5_identical_inputs_give_identical_reports() {
    let m = model();
    let (w, t) = placeholders::make(&m, HASH, "off");
    let a = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    let b = analyze(&m, &w, &t, &manifest(&m), names(), &dev());
    assert_eq!(report::render_json(&a.report), report::render_json(&b.report));
    assert_eq!(report::render_text(&a.report), report::render_text(&b.report));
    assert!(!report::render_json(&a.report).contains("timestamp"));
}

#[test]
fn r18_9_two_highest_level_partitions_with_a_continuing_overrun_response_are_stated() {
    let m = model();
    assert!(rsk_analyzer::unprotected_partitions(&m).is_empty(), "the demo has one level-A Partition");
    let two = DEMO.replacen("\"level\": \"B\"", "\"level\": \"A\"", 1);
    let m2 = rsk_model::read_task_model(&two).unwrap();
    let u = rsk_analyzer::unprotected_partitions(&m2);
    assert_eq!(u, vec!["part0 (continues)", "part2 (stops)"]);
    let (w, t) = placeholders::make(&m2, HASH, "off");
    let a = analyze(&m2, &w, &t, &manifest(&m2), names(), &dev());
    assert!(a.report.statements.iter().any(|s| s.contains("R18.7 does not protect the Partitions part0 (continues), part2 (stops)")), "{:?}", a.report.statements);
    // Two stopping Partitions at the top level need no statement.
    let both_stop = two.replacen("\"overrun\": \"RECORD_AND_CONTINUE\"", "\"overrun\": \"END_JOB\"", 1);
    assert!(rsk_analyzer::unprotected_partitions(&rsk_model::read_task_model(&both_stop).unwrap()).is_empty());
}

/// R30.4 on the built command: 0 PASS, 1 FAIL, 2 rejected input, 3 internal error.
#[test]
fn r30_4_exit_statuses_are_distinct() {
    let bin = env!("CARGO_BIN_EXE_rsk-analyzer");
    let dir = std::env::temp_dir().join(format!("rsk-analyzer-exit-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let m = model();
    let model_path = dir.join("task_model.json");
    std::fs::write(&model_path, DEMO).unwrap();
    let manifest_path = dir.join("build-manifest.json");
    std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest(&m)).unwrap()).unwrap();
    let run = |args: &[&str]| -> (i32, String) {
        let out = Command::new(bin).args(args).output().expect("rsk-analyzer runs");
        (out.status.code().unwrap_or(-1), format!("{}{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr)))
    };
    let out1: PathBuf = dir.join("out1");
    let (code, text) = run(&["--model", model_path.to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--placeholders", "--out", out1.to_str().unwrap()]);
    assert_eq!(code, EXIT_PASS as i32, "{text}");
    assert!(out1.join("report.json").is_file() && out1.join("report.txt").is_file() && out1.join("model.xml").is_file());
    // Determinism across runs of the command (R30.5).
    let out2 = dir.join("out2");
    let (code2, _) = run(&["--model", model_path.to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--placeholders", "--out", out2.to_str().unwrap()]);
    assert_eq!(code2, EXIT_PASS as i32);
    assert_eq!(std::fs::read_to_string(out1.join("report.json")).unwrap(), std::fs::read_to_string(out2.join("report.json")).unwrap());
    assert_eq!(std::fs::read_to_string(out1.join("report.txt")).unwrap(), std::fs::read_to_string(out2.join("report.txt")).unwrap());
    // FAIL: a deadline the response time cannot meet.
    let short_path = dir.join("short.json");
    std::fs::write(&short_path, DEMO.replacen("\"deadline_ticks\": 2500000,\n      \"budget_cycles\": 12500", "\"deadline_ticks\": 25000,\n      \"budget_cycles\": 12500", 1)).unwrap();
    let (code, text) = run(&["--model", short_path.to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--placeholders", "--out", dir.join("out3").to_str().unwrap()]);
    assert_eq!(code, EXIT_FAIL as i32, "{text}");
    // Rejected: a schema violation in the model.
    let bad_path = dir.join("bad.json");
    std::fs::write(&bad_path, DEMO.replacen("\"priority\": 7", "\"priority\": 9", 1)).unwrap();
    let (code, text) = run(&["--model", bad_path.to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--placeholders", "--out", dir.join("out4").to_str().unwrap()]);
    assert_eq!(code, EXIT_REJECTED as i32, "{text}");
    assert!(text.contains("/tasks/0/priority"), "{text}");
    // Rejected: placeholders without the development option.
    let (w, t) = placeholders::make(&m, HASH, "off");
    let w_path = dir.join("wcet.json");
    let t_path = dir.join("timing.json");
    std::fs::write(&w_path, rsk_model::print(&w)).unwrap();
    std::fs::write(&t_path, rsk_model::print(&t)).unwrap();
    let (code, _) = run(&["--model", model_path.to_str().unwrap(), "--wcet", w_path.to_str().unwrap(), "--timing", t_path.to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--out", dir.join("out5").to_str().unwrap()]);
    assert_eq!(code, EXIT_REJECTED as i32);
    // Internal error: an unreadable input or a usage error.
    let (code, _) = run(&["--model", dir.join("missing.json").to_str().unwrap(), "--manifest", manifest_path.to_str().unwrap(), "--placeholders", "--out", dir.join("out6").to_str().unwrap()]);
    assert_eq!(code, EXIT_INTERNAL as i32);
    let (code, _) = run(&["--bogus"]);
    assert_eq!(code, EXIT_INTERNAL as i32);
    let _ = std::fs::remove_dir_all(&dir);
}
