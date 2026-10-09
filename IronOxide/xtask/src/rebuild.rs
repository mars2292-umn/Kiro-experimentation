//! R59.1 and R59.3: the clean-checkout rebuild comparison.
//!
//! `cargo xtask rebuild-check` rebuilds the Flight_Build binaries from a
//! fresh copy of the source tree in another directory and compares their
//! binary hashes (over loadable sections, `rsk_elf::Elf::hash_input`) with
//! the hashes of the build in `target/xtask-verify`. The copy is taken with
//! `git archive` when the tree is tracked, so that only committed files take
//! part; otherwise the working tree is copied (without `target/`) and the
//! run says so, because it then shows reproducibility of the working tree,
//! not of a revision.
//!
//! Both builds compile with `--remap-path-prefix=<root>=/rsk` (see
//! `verify::target_build`), so that the checkout path does not reach the
//! binary through panic locations or debug information.

use std::path::{Path, PathBuf};

use crate::cmd::{stdout_of, Cmd};
use crate::diag::{Report, Rule};
use crate::manifest::{flight_binaries, Binary};
use crate::tools::Revision;
use crate::verify::target_build;
use crate::workspace::Workspace;

/// Copies the source tree into `dest`, skipping `target` directories and
/// `.git`.
fn copy_tree(src: &Path, dest: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dest).map_err(|e| format!("cannot create {}: {e}", dest.display()))?;
    for entry in std::fs::read_dir(src).map_err(|e| format!("cannot read {}: {e}", src.display()))? {
        let entry = entry.map_err(|e| e.to_string())?;
        let name = entry.file_name();
        let name_str = name.to_string_lossy();
        if name_str == "target" || name_str == ".git" {
            continue;
        }
        let from = entry.path();
        let to = dest.join(&name);
        let kind = entry.file_type().map_err(|e| e.to_string())?;
        if kind.is_dir() {
            copy_tree(&from, &to)?;
        } else if kind.is_file() {
            std::fs::copy(&from, &to).map_err(|e| format!("cannot copy {}: {e}", from.display()))?;
        }
    }
    Ok(())
}

/// Materializes a clean copy of the tree at `dest`.
pub fn clean_copy(root: &Path, dest: &Path, report: &mut Report) -> Result<(), String> {
    if dest.exists() {
        std::fs::remove_dir_all(dest).map_err(|e| format!("cannot remove {}: {e}", dest.display()))?;
    }
    let revision = Revision::of(root);
    if revision.tracked {
        std::fs::create_dir_all(dest).map_err(|e| e.to_string())?;
        let prefix = stdout_of(&Cmd::new("git").args(["rev-parse", "--show-prefix"]).cwd(root))?
            .trim()
            .to_string();
        let tree = if prefix.is_empty() {
            "HEAD".to_string()
        } else {
            format!("HEAD:{}", prefix.trim_end_matches('/'))
        };
        let archive = dest.with_extension("tar");
        let ok = Cmd::new("git")
            .args(["archive", "--format=tar", "-o"])
            .arg(&archive)
            .arg(&tree)
            .cwd(root)
            .status(None)
            .map_err(|e| e.to_string())?;
        if !ok {
            return Err("git archive failed".to_string());
        }
        let ok = Cmd::new("tar")
            .args(["-xf"])
            .arg(&archive)
            .args(["-C"])
            .arg(dest)
            .status(None)
            .map_err(|e| e.to_string())?;
        if !ok {
            return Err("extracting the archive failed".to_string());
        }
        let _ = std::fs::remove_file(&archive);
        if revision.dirty {
            report.warn(
                "the working tree has uncommitted changes; the rebuild uses the committed revision, so the \
                 comparison is against HEAD, not against the working tree",
            );
        }
        report.note(format!("clean checkout of {} at {}", revision.commit, dest.display()));
    } else {
        copy_tree(root, dest)?;
        report.warn(
            "the source tree is not tracked by git; the rebuild uses a copy of the working tree, which \
             demonstrates reproducibility of the working tree but is not R59.3 evidence for a revision",
        );
    }
    Ok(())
}

/// Runs the comparison. Returns whether every binary hash matched.
pub fn run(root: &Path, original_target_dir: &Path, report: &mut Report) -> bool {
    let ws = match Workspace::load(root, report) {
        Some(ws) => ws,
        None => return false,
    };
    let originals: Vec<Binary> = flight_binaries(&ws, original_target_dir)
        .into_iter()
        .filter_map(|b| match b {
            Ok(b) => Some(b),
            Err(e) => {
                report.error(Rule::Reproducible, None, e);
                None
            }
        })
        .collect();
    if originals.is_empty() {
        report.warn(format!(
            "no Flight_Build binary in {}: nothing to compare (run `cargo xtask verify` first; a binary \
             crate among the Flight_Build roots is needed)",
            original_target_dir.display()
        ));
        return !report.has_rule(Rule::Reproducible);
    }
    let copy: PathBuf = root.join("target").join("xtask-rebuild").join("checkout");
    if let Err(e) = clean_copy(root, &copy, report) {
        report.error(Rule::Reproducible, None, e);
        return false;
    }
    let mut copy_report = Report::default();
    let Some(copy_ws) = Workspace::load(&copy, &mut copy_report) else {
        report.absorb(copy_report);
        report.error(Rule::Reproducible, None, "the clean copy does not load as a workspace");
        return false;
    };
    let copy_target_dir = copy.join("target").join("xtask-verify");
    if let Err(e) = target_build(&copy_ws, &copy_target_dir, None) {
        report.error(Rule::Reproducible, None, format!("the rebuild failed: {e}"));
        return false;
    }
    let mut ok = true;
    for rebuilt in flight_binaries(&copy_ws, &copy_target_dir) {
        let rebuilt = match rebuilt {
            Ok(b) => b,
            Err(e) => {
                report.error(Rule::Reproducible, None, e);
                ok = false;
                continue;
            }
        };
        match originals
            .iter()
            .find(|o| o.package == rebuilt.package && o.target_name == rebuilt.target_name)
        {
            Some(original) if original.hash == rebuilt.hash => report.note(format!(
                "R59.1: `{}` rebuilt with the identical binary hash {}",
                rebuilt.target_name, rebuilt.hash
            )),
            Some(original) => {
                ok = false;
                report.error(
                    Rule::Reproducible,
                    None,
                    format!(
                        "`{}` is not reproducible: original {} ({} loadable bytes), rebuilt {} ({} loadable bytes)",
                        rebuilt.target_name,
                        original.hash,
                        original.loadable_bytes,
                        rebuilt.hash,
                        rebuilt.loadable_bytes
                    ),
                );
            }
            None => {
                ok = false;
                report.error(
                    Rule::Reproducible,
                    None,
                    format!("`{}` exists in the rebuild but not in the original build", rebuilt.target_name),
                );
            }
        }
    }
    ok
}
