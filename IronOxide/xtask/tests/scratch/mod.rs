//! Scratch Cargo workspaces for the tests that compile generated or
//! documented source as a user would write it (`skeletons.rs`,
//! `tutorial.rs`).
//!
//! A scratch workspace lies under `target/scratch-check/<name>/`, inside
//! the repository, so that Cargo finds `.cargo/config.toml` (vendored
//! sources, offline) as the fixture workspaces do. All of them share one
//! target directory, so `rsk` and its macros are compiled once. A file is
//! written only when its content changed, so a second run rebuilds nothing.
#![allow(dead_code)]

use std::fs;
use std::path::{Path, PathBuf};

/// The repository root.
pub fn root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).parent().expect("xtask lies in the workspace root").to_path_buf()
}

/// A dependency line on a crate of the repository, by its path from the root.
pub fn repo_dep(name: &str, path: &str, rest: &str) -> String {
    format!("{name} = {{ path = {:?}{rest} }}\n", root().join(path))
}

/// A dependency line on another member of the same scratch workspace.
pub fn member_dep(name: &str, member: &str) -> String {
    format!("{name} = {{ package = \"{member}\", path = \"../{member}\" }}\n")
}

/// A member's manifest: the package is named as its directory, the library
/// `lib` (the name the sources of the other members use).
pub fn manifest(member: &str, lib: &str, dependencies: &str) -> String {
    format!(
        "[package]\nname = \"{member}\"\nversion.workspace = true\nedition.workspace = true\nrust-version.workspace = true\npublish.workspace = true\n\n[lib]\nname = \"{lib}\"\n\n[dependencies]\n{dependencies}\n[lints.rust.unexpected_cfgs]\nlevel = \"warn\"\ncheck-cfg = [\"cfg(creusot)\"]\n"
    )
}

pub struct Scratch {
    dir: PathBuf,
    members: Vec<String>,
}

/// The outcome of `cargo check` on a scratch workspace.
pub struct Checked {
    pub ok: bool,
    pub stderr: String,
}

impl Checked {
    /// The check passed and no diagnostic, warning or error, lies in a
    /// member's own source (Cargo prints a member's paths relative to the
    /// scratch workspace).
    pub fn assert_clean(&self, what: &str) {
        assert!(self.ok, "{what} must compile:\n{}", self.stderr);
        assert!(!self.stderr.contains("--> crates/"), "{what} must compile without a diagnostic:\n{}", self.stderr);
    }
}

fn write_if_changed(path: &Path, content: &str) {
    if fs::read_to_string(path).map(|old| old == content).unwrap_or(false) {
        return;
    }
    fs::create_dir_all(path.parent().expect("a file in a directory")).expect("the scratch directory");
    fs::write(path, content).unwrap_or_else(|e| panic!("{}: {e}", path.display()));
}

impl Scratch {
    pub fn new(name: &str) -> Scratch {
        Scratch { dir: root().join("target").join("scratch-check").join(name), members: Vec::new() }
    }

    /// Adds the member `crates/<member>` with its manifest and its `src/lib.rs`.
    pub fn member(&mut self, member: &str, manifest: &str, lib: &str) {
        assert!(!self.members.iter().any(|m| m == member), "member `{member}` twice");
        let dir = self.dir.join("crates").join(member);
        write_if_changed(&dir.join("Cargo.toml"), manifest);
        write_if_changed(&dir.join("src").join("lib.rs"), lib);
        self.members.push(member.to_string());
    }

    /// Runs `cargo check` on every member, for the host.
    pub fn check(&self) -> Checked {
        // A member of an earlier run that this one does not have.
        if let Ok(entries) = fs::read_dir(self.dir.join("crates")) {
            for e in entries.filter_map(|e| e.ok()) {
                if !self.members.iter().any(|m| *m == e.file_name().to_string_lossy()) {
                    let _ = fs::remove_dir_all(e.path());
                }
            }
        }
        let members: Vec<String> = self.members.iter().map(|m| format!("\"crates/{m}\"")).collect();
        write_if_changed(
            &self.dir.join("Cargo.toml"),
            &format!("[workspace]\nresolver = \"2\"\nmembers = [{}]\n\n[workspace.package]\nversion = \"0.0.0\"\nedition = \"2021\"\nrust-version = \"1.95.0\"\npublish = false\n", members.join(", ")),
        );
        let out = std::process::Command::new("cargo")
            .args(["check", "--offline", "--workspace", "--target-dir"])
            .arg(root().join("target").join("scratch-check").join("target"))
            .current_dir(&self.dir)
            .output()
            .expect("cargo check runs");
        Checked { ok: out.status.success(), stderr: String::from_utf8_lossy(&out.stderr).into_owned() }
    }
}
