//! UI tests (task 11.1): each fixture under `fixtures/ui` declares its scope
//! and the lint runs over it; the expected findings are listed here as
//! (rule, line) pairs and must match exactly, so every lint has at least
//! one rejected program and the positive fixture has none.
#![forbid(unsafe_code)]

use std::path::Path;

fn core_items() -> Vec<rsk_lint::core_subset::Item> {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profile/core_subset.toml");
    rsk_lint::core_subset::parse(&std::fs::read_to_string(p).unwrap()).unwrap()
}

fn run(name: &str) -> (Vec<(String, usize)>, rsk_lint::Report) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR")).join("fixtures/ui").join(name);
    let text = std::fs::read_to_string(&p).unwrap();
    let scope = match text.lines().next().unwrap().trim_start_matches("//@ scope: ").trim() {
        "kernel" => rsk_lint::Scope::Kernel,
        "kernel-unsafe" => rsk_lint::Scope::KernelUnsafe,
        "generated" => rsk_lint::Scope::Generated,
        _ => rsk_lint::Scope::Application,
    };
    let mut report = rsk_lint::Report::default();
    rsk_lint::lint_source(name, &text, scope, &core_items(), &mut report).unwrap();
    let mut found: Vec<(String, usize)> = report.findings.iter().map(|f| (f.rule.to_string(), f.line)).collect();
    found.sort();
    (found, report)
}

fn expect(name: &str, expected: &[(&str, usize)]) -> rsk_lint::Report {
    let (found, report) = run(name);
    let mut expected: Vec<(String, usize)> = expected.iter().map(|(r, l)| (r.to_string(), *l)).collect();
    expected.sort();
    assert_eq!(found, expected, "{name}: {:#?}", report.findings);
    report
}

#[test]
fn pr09_async() {
    expect("async_task.rs", &[("PR-09", 3), ("PR-09", 6), ("PR-09", 8), ("PR-09", 9)]);
}

/// R35.1: each loop with neither a Loop_Bound_Annotation nor an iteration
/// count derivable from compile-time constants is reported naming PR-16;
/// annotated and derived loops are accepted.
#[test]
fn pr16_loops() {
    let r = expect("loops.rs", &[("PR-16", 7), ("PR-16", 10), ("PR-16", 11), ("PR-16", 55), ("PR-16", 59), ("PR-16", 69)]);
    let kinds: Vec<(&str, Option<&str>)> = r.loops.iter().map(|l| (l.kind, l.bound.as_deref())).collect();
    assert!(kinds.contains(&("annotated", Some("N * 2"))) && kinds.contains(&("annotated", Some("3"))));
    assert!(kinds.contains(&("derived", Some("N"))) && kinds.contains(&("derived", Some("10_000"))) && kinds.contains(&("derived", Some("3"))));
    assert_eq!(kinds.iter().filter(|(k, _)| *k == "exempt").count(), 3);
}

/// R35.1: a loop inside `verus!` in a Kernel function without a `decreases`
/// clause is reported naming PR-16; `decreases` and a range `for` are bounds.
#[test]
fn pr16_verus_decreases() {
    let r = expect("verus_loops.rs", &[("PR-16", 22)]);
    assert_eq!(r.loops.iter().filter(|l| l.kind == "verus").count(), 2);
    assert!(r.loops.iter().all(|l| l.function == "ok"));
}

/// R57.2: each use of a core item outside the Core_Subset, in Application
/// and Kernel_Unsafe_Module scope, is reported naming PR-21.
#[test]
fn pr21_core_subset_scopes() {
    expect("core_subset.rs", &[("PR-21", 3), ("PR-21", 5), ("PR-21", 8)]);
    expect("kernel_unsafe_core.rs", &[("PR-21", 4)]);
}

#[test]
fn pr22_assembly_in_application_code() {
    expect("asm_in_task.rs", &[("PR-21", 5), ("PR-22", 5)]);
}

#[test]
fn r3_4_lint_lowering_attributes_are_rejected() {
    expect("lint_lowering.rs", &[("R3.4", 3), ("R3.4", 6), ("R3.4", 9), ("R3.4", 12), ("R3.4", 18)]);
}

#[test]
fn pr05_pr17_pr18() {
    expect("statics_recursion_dyn.rs", &[("PR-21", 3), ("PR-05", 5), ("PR-05", 6), ("PR-05", 8), ("PR-17", 11), ("PR-18", 18), ("PR-18", 22), ("PR-17", 28)]);
}

/// PR-16: loops under `cfg(test)` and `cfg(kani)` are outside the
/// Flight_Build and not reported; loops in feature-gated code are.
#[test]
fn cfg_gated_code() {
    expect("cfg_gated.rs", &[("PR-16", 19), ("PR-16", 30)]);
}

#[test]
fn positive_task_body_has_no_finding() {
    let r = expect("positive.rs", &[]);
    assert_eq!(r.loops.len(), 2);
}

/// Bypass fixtures of R3.4: a `.cargo/config.toml` that caps lints and a
/// `[lints]` table that allows a deny-list lint are both rejected.
#[test]
fn configuration_bypasses_are_rejected() {
    let dir = std::env::temp_dir().join(format!("rsk-lint-ui-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(dir.join(".cargo")).unwrap();
    std::fs::create_dir_all(dir.join("app/src")).unwrap();
    std::fs::write(dir.join(".cargo/config.toml"), "[build]\nrustflags = [\"--cap-lints\", \"warn\"]\n").unwrap();
    std::fs::write(dir.join("Cargo.toml"), "[workspace]\nmembers = [\"app\"]\n[workspace.lints.clippy]\ndisallowed_methods = \"allow\"\n").unwrap();
    std::fs::write(dir.join("app/Cargo.toml"), "[package]\nname = \"app\"\n[lints.clippy]\ndisallowed_types = \"warn\"\n").unwrap();
    std::fs::write(dir.join("app/src/lib.rs"), "pub fn f() {}\n").unwrap();
    let core = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../profile/core_subset.toml");
    let opts = rsk_lint::Options { app_dirs: vec![dir.join("app")], root: Some(dir.clone()), core_subset_toml: std::fs::read_to_string(core).unwrap(), ..Default::default() };
    let report = rsk_lint::run(&opts).unwrap();
    let mut rules: Vec<(&str, String)> = report.findings.iter().map(|f| (f.rule, f.file.clone())).collect();
    rules.sort();
    assert_eq!(rules, vec![("R3.4", ".cargo/config.toml".to_string()), ("R3.4", "Cargo.toml".to_string()), ("R3.4", "app/Cargo.toml".to_string())], "{:#?}", report.findings);
    let _ = std::fs::remove_dir_all(&dir);
}
