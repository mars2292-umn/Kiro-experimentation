//! The Build_System checks.

pub mod ci;
pub mod config;
pub mod deps;
pub mod features;
pub mod justifications;
pub mod kernel_size;
pub mod kernel_std;
pub mod lockfile;
pub mod toolchain;
pub mod unsafe_code;

use std::path::Path;

use crate::diag::{Report, Rule};
use crate::workspace::Workspace;

/// Checks that apply to any workspace with an rsk build policy. Returns the
/// loaded workspace when `cargo metadata --frozen` succeeded.
pub fn workspace(root: &Path, report: &mut Report) -> Option<Workspace> {
    config::check(root, report);
    config::check_environment(report);
    let ws = Workspace::load(root, report)?;
    policy_matches_members(&ws, report);
    lockfile::check(&ws, report);
    unsafe_code::check(&ws, report);
    justifications::check(&ws, report);
    kernel_size::check(&ws, report);
    kernel_std::check(&ws, report);
    features::check_sources(&ws, report);
    Some(ws)
}

/// [`workspace`] plus the repository-level checks.
pub fn repository(root: &Path, report: &mut Report) -> Option<Workspace> {
    let ws = workspace(root, report);
    toolchain::check(root, ws.as_ref(), report);
    ci::check(root, report);
    config::check_alias(root, report);
    ws
}

/// Every workspace member has exactly one policy entry, and every Kernel
/// crate is built for the Target.
fn policy_matches_members(ws: &Workspace, report: &mut Report) {
    let location = Some("Cargo.toml".to_string());
    for pkg in ws.member_packages() {
        if !ws.policy.crates.contains_key(&pkg.name) {
            report.error(
                Rule::Policy,
                location.clone(),
                format!(
                    "workspace member `{}` has no entry in [workspace.metadata.rsk.crates]",
                    pkg.name
                ),
            );
        }
    }
    for (name, policy) in &ws.policy.crates {
        if ws.package_named(name).is_none() {
            report.error(
                Rule::Policy,
                location.clone(),
                format!("[workspace.metadata.rsk.crates] names `{name}`, which is not a workspace member"),
            );
        }
        if policy.kernel && !policy.targets.contains(&ws.policy.target) {
            report.error(
                Rule::Policy,
                location.clone(),
                format!(
                    "Kernel crate `{name}` must be built for {}",
                    ws.policy.target
                ),
            );
        }
    }
}
