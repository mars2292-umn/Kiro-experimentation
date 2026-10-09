//! Fixture tests (testing evidence for task 1.1): the static checks accept
//! the positive fixture and reject each negative fixture with a diagnostic
//! that names the violated rule.
//!
//! Each fixture under `xtask/fixtures/` is a separate Cargo workspace with
//! its own `[workspace]` table, build policy, and `Cargo.lock`. The main
//! workspace excludes them and the source scans skip them. They inherit the
//! repository's `.cargo/config.toml` (vendored sources, offline), as Cargo
//! does.
//!
//! | Fixture | Content | Rule |
//! |---|---|---|
//! | `positive` | a Kernel crate with `unsafe` code and `asm!` inside its Kernel_Unsafe_Module (each construct justified by a Kani harness, a `verus!` lemma, or a review), an entry crate with `#[no_mangle]`, an Application crate, a host tool with an integration test | accepted |
//! | `missing-forbid` | a crate root without `#![forbid(unsafe_code)]` | PR-20/R6.3 |
//! | `feature-attr` | `#![feature]` and `cfg_attr(docsrs, feature(...))` | PR-36/R55.1 |
//! | `unlocked-dependency` | a path dependency absent from the lock file | R58.1 |
//! | `inexact-requirement` | a crates.io requirement `1.0` instead of `=x.y.z` | R58.1 |
//! | `kernel-violations` | `extern crate alloc`; `unsafe` in a macro body and a naked function outside the Kernel_Unsafe_Module; a Kernel module without its own `forbid` | R6.1, PR-20/R6.3 |
//! | `bootstrap-in-config` | `RUSTC_BOOTSTRAP` in `.cargo/config.toml` `[env]` | PR-36/R55.1 |
//! | `third-party-kernel` | vendored crates in the Kernel's Target graph (one declares `extern crate alloc`) and in the Flight_Build graph; built for the Target, then checked | R6.1 (post-build) |
//! | `unsafe-unjustified` | an `unsafe` block without an identifier, an identifier absent from the register, a link to a missing Kani harness, a review without its statement | PR-20/R6.4 |
//! | `kernel-too-large` | a Kernel with four executable lines against `kernel-max-exec-lines = 3` | R6.2/PAR-01 |
//! | `kani-unwind` | a Kani harness whose unwinding assertion fails; its captured output (`expected.log`) is checked by a unit test of `proofs::kani_verdict`, and the fixture is run live when `cargo kani` is installed | R45.5 |
//!
//! The `network-probe` fixture is not tested here: `cargo xtask verify`
//! builds it inside the network sandbox, where its build script and
//! proc-macro fail the build unless their connection attempt is denied
//! (R58.4).
#![forbid(unsafe_code)]

use std::path::PathBuf;

use xtask::{check_workspace, Diagnostic, Report, Rule};

fn fixture_root(fixture: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("fixtures")
        .join(fixture)
}

fn diagnostics(fixture: &str) -> Vec<Diagnostic> {
    check_workspace(&fixture_root(fixture)).diagnostics
}

fn render(diagnostics: &[Diagnostic]) -> String {
    diagnostics
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n")
}

/// Asserts that every diagnostic belongs to one of `rules` and that, for
/// each expected line, some diagnostic contains every one of its fragments.
fn assert_diagnostics(diagnostics: &[Diagnostic], rules: &[Rule], expected: &[&[&str]]) {
    let text = render(diagnostics);
    assert!(
        diagnostics.iter().all(|d| rules.contains(&d.rule)),
        "unexpected rule in:\n{text}"
    );
    for fragments in expected {
        assert!(
            diagnostics.iter().any(|d| {
                let line = d.to_string();
                fragments.iter().all(|f| line.contains(f))
            }),
            "no diagnostic contains {fragments:?} in:\n{text}"
        );
    }
}

#[test]
fn positive_fixture_passes_every_workspace_check() {
    let found = diagnostics("positive");
    assert!(found.is_empty(), "{}", render(&found));
}

#[test]
fn crate_without_forbid_is_rejected_naming_pr20_r6_3() {
    let found = diagnostics("missing-forbid");
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::UnsafeConfinement],
        &[&[
            "error[PR-20/R6.3] noforbid/src/lib.rs",
            "lacks an unconditional `#![forbid(unsafe_code)]`",
        ]],
    );
}

#[test]
fn feature_attributes_are_rejected_naming_pr36_r55_1() {
    let found = diagnostics("feature-attr");
    assert_eq!(found.len(), 2, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::NoUnstable],
        &[
            &[
                "error[PR-36/R55.1] unstable/src/lib.rs:4",
                "`#![feature(never_type)]`",
            ],
            &[
                "error[PR-36/R55.1] unstable/src/lib.rs:5",
                "`feature(doc_cfg)` inside `cfg_attr(docsrs)`",
            ],
        ],
    );
}

#[test]
fn dependency_missing_from_the_lock_file_is_rejected_naming_r58_1() {
    let found = diagnostics("unlocked-dependency");
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::Locked],
        &[&["error[R58.1] Cargo.lock", "does not list", "--frozen"]],
    );
}

#[test]
fn inexact_requirement_is_rejected_naming_r58_1() {
    let found = diagnostics("inexact-requirement");
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::Locked],
        &[&[
            "error[R58.1] tool/Cargo.toml",
            "`proc-macro2`",
            "`^1.0`",
            "exact",
        ]],
    );
}

#[test]
fn kernel_violations_are_rejected_naming_r6_1_and_pr20_r6_3() {
    let found = diagnostics("kernel-violations");
    assert_diagnostics(
        &found,
        &[Rule::KernelNoStd, Rule::UnsafeConfinement],
        &[
            &["error[R6.1] k/src/lib.rs:4", "`extern crate alloc`"],
            &[
                "error[PR-20/R6.3] k/src/hw.rs:",
                "lacks its own `#![forbid(unsafe_code)]`",
            ],
            &[
                "error[PR-20/R6.3] k/src/hw.rs:5",
                "unsafe attribute `naked`",
            ],
            &[
                "error[PR-20/R6.3] k/src/hw.rs:7",
                "assembly macro `naked_asm!`",
            ],
            &["error[PR-20/R6.3] k/src/logic.rs:11", "`unsafe` keyword"],
        ],
    );
    assert!(
        !render(&found).contains("unsafe_module.rs"),
        "the Kernel_Unsafe_Module must not be reported:\n{}",
        render(&found)
    );
}

#[test]
fn rustc_bootstrap_in_cargo_config_is_rejected_naming_pr36_r55_1() {
    let found = diagnostics("bootstrap-in-config");
    assert_eq!(found.len(), 1, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::NoUnstable],
        &[&["error[PR-36/R55.1] .cargo/config.toml:2", "RUSTC_BOOTSTRAP"]],
    );
}

/// The post-build checks judge third-party crates by the configuration they
/// were compiled with. The Target build of this fixture succeeds although
/// hashbrown links `alloc`, so only the explicit R6.1 check rejects it; the
/// `cfg_attr` feature gates of hashbrown and proc-macro2 evaluate as
/// inactive and are accepted (PR-36/R55.1).
#[test]
fn third_party_kernel_crate_that_links_alloc_is_rejected_after_the_target_build() {
    let root = fixture_root("third-party-kernel");
    let mut report = Report::default();
    let ws = xtask::checks::workspace(&root, &mut report).expect("the fixture workspace loads");
    assert!(
        report.diagnostics.is_empty(),
        "{}",
        render(&report.diagnostics)
    );

    let target_dir = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("third-party-kernel");
    let outcome = xtask::verify::target_build(&ws, &target_dir, None)
        .expect("the fixture builds for the Target");
    let mut post = Report::default();
    xtask::verify::post_build_checks(&ws, &outcome, &target_dir, &mut post);

    assert_eq!(post.diagnostics.len(), 1, "{}", render(&post.diagnostics));
    assert_diagnostics(
        &post.diagnostics,
        &[Rule::KernelNoStd],
        &[&[
            "error[R6.1]",
            "hashbrown-0.17.1/src/lib.rs:",
            "`hashbrown 0.17.1`",
            "`extern crate alloc`",
        ]],
    );
    for gate in [
        "inactive: hashbrown 0.17.1",
        "inactive: proc-macro2 1.0.107",
    ] {
        assert!(
            post.notes.iter().any(|n| n.contains(gate)),
            "missing note `{gate}` in {:#?}",
            post.notes
        );
    }
}

#[test]
fn unjustified_unsafe_constructs_are_rejected_naming_pr20_r6_4() {
    let found = diagnostics("unsafe-unjustified");
    assert_eq!(found.len(), 4, "{}", render(&found));
    assert_diagnostics(
        &found,
        &[Rule::UnsafeJustification],
        &[
            &[
                "error[PR-20/R6.4] k/src/arch/unsafe_module.rs:7",
                "`unsafe` block lacks a justification identifier",
            ],
            &[
                "error[PR-20/R6.4] k/src/arch/unsafe_module.rs:11",
                "`unsafe fn` `unregistered` cites `UJ-009`",
                "does not define",
            ],
            &[
                "error[PR-20/R6.4] k/src/arch/unsafe_module.rs:16",
                "`UJ-002` (`unsafe impl`) links to the Kani harness `uj_002_cell_is_sync`",
            ],
            &[
                "error[PR-20/R6.4] k/src/arch/unsafe_module.rs:19",
                "`UJ-003` (`unsafe fn` `review_without_why`) is justified by review only",
                "why_no_machine_check",
            ],
        ],
    );
}

#[test]
fn kernel_above_the_line_limit_is_rejected_naming_r6_2() {
    let report = xtask::check_workspace(&fixture_root("kernel-too-large"));
    assert_eq!(report.diagnostics.len(), 1, "{}", render(&report.diagnostics));
    assert_diagnostics(
        &report.diagnostics,
        &[Rule::KernelSize],
        &[&["error[R6.2/PAR-01]", "4 executable lines", "exceeds", "limit of 3"]],
    );
    assert!(
        report.notes.iter().any(|n| n.contains("R6.2/PAR-01: the Kernel has 4 executable lines (limit 3")),
        "{:#?}",
        report.notes
    );
}

/// R45.5: a run with a failed unwinding assertion is a failed run. Runs
/// Kani on the fixture when it is installed; otherwise only the captured
/// log is checked (by the unit test in `proofs`).
#[test]
fn kani_run_with_a_failed_unwinding_assertion_is_rejected() {
    let kani_installed = std::process::Command::new("cargo")
        .args(["kani", "--version"])
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false);
    if !kani_installed {
        eprintln!("cargo kani not installed: live fixture run skipped (captured log checked by proofs::tests)");
        return;
    }
    let root = fixture_root("kani-unwind");
    let target = std::env::temp_dir().join(format!("rsk-kani-unwind-{}", std::process::id()));
    let out = std::process::Command::new("cargo")
        .args(["kani", "--target-dir"])
        .arg(&target)
        .current_dir(&root)
        .output()
        .expect("cargo kani runs");
    let _ = std::fs::remove_dir_all(&target);
    let text = format!("{}\n{}", String::from_utf8_lossy(&out.stdout), String::from_utf8_lossy(&out.stderr));
    let verdict = xtask::proofs::kani_verdict(out.status.success(), &text);
    assert!(verdict.unwinding_failed, "no unwinding failure reported:\n{text}");
    assert!(!verdict.ok);
}
