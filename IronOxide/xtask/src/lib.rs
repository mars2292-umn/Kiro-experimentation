//! rsk Build_System checks and the verification job (task 1.1).
//!
//! `cargo xtask verify` is the single entry point that CI and local runs use.
//! It runs the checks below, builds every crate for the host and the Kernel,
//! Application API, and Flight_Build roots for the Target, and runs the
//! fixture tests. Every diagnostic names the rule it enforces:
//!
//! | Rule | Check |
//! |---|---|
//! | PR-20/R6.3 | `#![forbid(unsafe_code)]` placement; no `unsafe`, unsafe attributes, or assembly outside the Kernel_Unsafe_Module (token scan that also covers macro bodies) |
//! | R6.1 | the Kernel crates are `#![no_std]`; their Target graph contains no crate that uses `std` or `alloc` |
//! | PR-36/R55.1 | no `#![feature]` (also inside `cfg_attr`) and no `RUSTC_BOOTSTRAP` |
//! | R55.2, R55.5 | the pinned upstream rustc and the single edition of `toolchain/decision.toml` |
//! | R6.2/PAR-01 | the Kernel's executable line count (task 1.2) |
//! | PR-20/R6.4 | every `unsafe` construct of the Kernel_Unsafe_Module carries a justification identifier linked to a Kani harness, a Verus proof, or a recorded review (task 1.2) |
//! | R58.1 | lock-file-only resolution, exact version requirements, vendored sources |
//! | R58.2/R58.5 | the dependency report of the Flight_Build graph and the Trust_Base_Register gate (task 1.3) |
//! | R58.4 | offline Cargo, and build scripts and proc-macros run with the network denied |
//! | R59.1/R59.3 | the clean-checkout rebuild comparison (`cargo xtask rebuild-check`, task 1.3) |
//! | R33.3, R50.3 | the Build_Manifest and the Evidence_Items the job writes (task 1.3) |
//! | R1, R2, R4, R27.3, R57 | the Profile source, its generated document and version modules (`cargo xtask profile`, task 2.1) |
//! | R44.6, R50.2, R3.10 | the Kernel_Proofs with Verus, the assumption gate, and the PAR-01 cross-check (task 4.1) |
//! | R45.5, R45.7 | the Kani_Harnesses with the pinned Kani and CBMC (task 6.1) |
//!
//! The crate is a host tool outside the Flight_Build graph. It forbids unsafe
//! code like every rsk crate.
#![forbid(unsafe_code)]

pub mod cfg;
pub mod checks;
pub mod cmd;
pub mod diag;
pub mod evidence;
pub mod gen;
pub mod manifest;
pub mod policy;
pub mod profile;
pub mod profile_gen;
pub mod proofs;
pub mod qemu;
pub mod rebuild;
pub mod sandbox;
pub mod tokens;
pub mod tools;
pub mod units;
pub mod verify;
pub mod workspace;

use std::path::{Path, PathBuf};

pub use diag::{Diagnostic, Report, Rule};

/// The rsk repository root (the parent of this crate's directory).
pub fn repository_root() -> PathBuf {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .map_or_else(|| manifest_dir.to_path_buf(), Path::to_path_buf)
}

/// Runs the static checks that apply to any Cargo workspace with an rsk build
/// policy, such as the fixtures: build policy, PR-20/R6.3, R6.1 (workspace
/// crates), PR-36/R55.1 (workspace sources, Cargo configuration, environment),
/// R58.1, and the offline setting of R58.4.
pub fn check_workspace(root: &Path) -> Report {
    let mut report = Report::default();
    let _workspace = checks::workspace(root, &mut report);
    report
}

/// Runs [`check_workspace`] plus the repository-level checks: the toolchain
/// pin and edition (R55.2, R55.5), the CI workflow files, and the `xtask`
/// alias.
pub fn check_repository(root: &Path) -> Report {
    let mut report = Report::default();
    let _workspace = checks::repository(root, &mut report);
    report
}
