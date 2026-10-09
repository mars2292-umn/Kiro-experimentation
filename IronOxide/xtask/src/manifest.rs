//! The Build_Manifest (R33.3, R50.3, R59.2) and its JSON form.
//!
//! One manifest describes one build of the workspace: the source revision,
//! the lock file, the toolchain versions, the Profile version, the
//! Generated_Config checksum, and the binary hash of each Flight_Build
//! binary over its loadable sections (`rsk_elf::Elf::hash_input`). The
//! Analyzer accepts a manifest as one of its four inputs (R28.1) and
//! rejects WCET_Records whose binary hash or build configuration differs
//! from it (R28.3, R33.4).
//!
//! Keys are emitted in sorted order (serde_json's default map), so equal
//! manifests are byte-identical documents.

use std::path::{Path, PathBuf};

use serde_json::{json, Map, Value};

use crate::cmd::{stdout_of, Cmd};
use crate::tools::{Kani, Revision, Verus};
use crate::workspace::Workspace;

pub const SCHEMA: &str = "rsk-build-manifest/1";

/// One Flight_Build binary.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Binary {
    pub package: String,
    pub target_name: String,
    pub path: PathBuf,
    /// `sha256:` over the loadable image (R33.3).
    pub hash: String,
    pub loadable_bytes: usize,
    /// The Generated_Config checksum stored in the binary (the
    /// `RSK_CONFIG_CHECKSUM` symbol the system manifest emits), as `0x...`
    /// (R33.3, R28.8); absent for a binary without a Generated_Config.
    pub config_checksum: Option<String>,
}

impl Binary {
    pub fn of(package: &str, target_name: &str, path: &Path) -> Result<Binary, String> {
        let elf = rsk_elf::Elf::read(path).map_err(|e| e.to_string())?;
        let input = elf.hash_input().map_err(|e| e.to_string())?;
        let config_checksum = elf
            .symbols()
            .map_err(|e| e.to_string())?
            .iter()
            .find(|s| s.name == "RSK_CONFIG_CHECKSUM" && s.is_object())
            .and_then(|s| elf.read_at(s.address(), 4).ok())
            .map(|b| format!("0x{:08x}", u32::from_le_bytes([b[0], b[1], b[2], b[3]])));
        Ok(Binary {
            package: package.to_string(),
            target_name: target_name.to_string(),
            path: path.to_path_buf(),
            hash: rsk_digest::sha256_hex(&input),
            loadable_bytes: elf
                .loadable_image()
                .map_err(|e| e.to_string())?
                .iter()
                .map(|(_, b)| b.len())
                .sum(),
            config_checksum,
        })
    }
}

#[derive(Clone, Debug)]
pub struct Manifest {
    pub revision: Revision,
    pub lock_file_hash: String,
    pub rustc: String,
    pub cargo: String,
    pub verus: Option<Verus>,
    pub kani: Option<Kani>,
    pub profile_version: String,
    pub profile_variant: String,
    pub target: String,
    pub cargo_profile: String,
    pub generated_config_checksum: Option<String>,
    pub binaries: Vec<Binary>,
}

impl Manifest {
    /// Collects everything that does not depend on a particular binary.
    pub fn collect(ws: &Workspace, verus: Option<Verus>, kani: Option<Kani>) -> Result<Manifest, String> {
        let root = &ws.root;
        let lock_file_hash = rsk_digest::sha256_file(&root.join("Cargo.lock"))
            .map_err(|e| format!("cannot hash Cargo.lock: {e}"))?;
        let rustc = stdout_of(&Cmd::rustc().arg("-vV").cwd(root))?
            .lines()
            .next()
            .unwrap_or_default()
            .to_string();
        let cargo = stdout_of(&Cmd::cargo().arg("-V").cwd(root))?.trim().to_string();
        let (profile_version, profile_variant) = crate::profile::version_of(root);
        Ok(Manifest {
            revision: Revision::of(root),
            lock_file_hash,
            rustc,
            cargo,
            verus,
            kani,
            profile_version,
            profile_variant,
            target: ws.policy.target.clone(),
            cargo_profile: "release".to_string(),
            generated_config_checksum: None,
            binaries: Vec::new(),
        })
    }

    pub fn to_json(&self) -> Value {
        let mut toolchain = Map::new();
        toolchain.insert("rustc".into(), json!(self.rustc));
        toolchain.insert("cargo".into(), json!(self.cargo));
        toolchain.insert(
            "verus".into(),
            match &self.verus {
                Some(v) => json!({ "version": v.version, "toolchain": v.toolchain, "z3": v.z3_version }),
                None => Value::Null,
            },
        );
        toolchain.insert(
            "kani".into(),
            match &self.kani {
                Some(k) => json!({ "version": k.version, "cbmc": k.cbmc_version }),
                None => Value::Null,
            },
        );
        json!({
            "schema": SCHEMA,
            "source": {
                "revision": self.revision.commit,
                "dirty": self.revision.dirty,
                "tracked": self.revision.tracked,
            },
            "lock_file": { "hash": self.lock_file_hash },
            "toolchain": toolchain,
            "profile": { "version": self.profile_version, "variant": self.profile_variant },
            "build": {
                "target": self.target,
                "cargo_profile": self.cargo_profile,
                "panic": "abort",
                "icache": "off",
            },
            "generated_config_checksum": self.generated_config_checksum,
            "binaries": self.binaries.iter().map(|b| json!({
                "package": b.package,
                "target": b.target_name,
                "file": b.path.file_name().map(|f| f.to_string_lossy().into_owned()),
                "hash": b.hash,
                "loadable_bytes": b.loadable_bytes,
                "config_checksum": b.config_checksum,
            })).collect::<Vec<_>>(),
        })
    }

    /// The manifest's own identity: the hash of its canonical JSON.
    pub fn identity(&self) -> String {
        rsk_digest::sha256_hex(self.to_json().to_string().as_bytes())
    }

    pub fn write(&self, path: &Path) -> Result<(), String> {
        if let Some(dir) = path.parent() {
            std::fs::create_dir_all(dir).map_err(|e| format!("cannot create {}: {e}", dir.display()))?;
        }
        let text = serde_json::to_string_pretty(&self.to_json()).map_err(|e| e.to_string())?;
        std::fs::write(path, text + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))
    }
}

/// The Flight_Build binaries of a Target build: every `bin` target of a
/// Flight_Build root, found under `<target_dir>/<triple>/release/`.
pub fn flight_binaries(ws: &Workspace, target_dir: &Path) -> Vec<Result<Binary, String>> {
    let mut out = Vec::new();
    let dir = target_dir.join(&ws.policy.target).join("release");
    for name in ws.policy.flight_roots() {
        let Some(pkg) = ws.package_named(name) else { continue };
        for target in &pkg.targets {
            if !target.kinds.iter().any(|k| k == "bin") {
                continue;
            }
            let path = dir.join(&target.name);
            if path.is_file() {
                out.push(Binary::of(&pkg.name, &target.name, &path));
            } else {
                out.push(Err(format!(
                    "Flight_Build binary `{}` of `{}` was not produced at {}",
                    target.name,
                    pkg.name,
                    path.display()
                )));
            }
        }
    }
    out
}
