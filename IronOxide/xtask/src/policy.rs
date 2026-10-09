//! The build policy table `[workspace.metadata.rsk]` of the root `Cargo.toml`.
//!
//! It is the single place that states, per crate, the targets the
//! verification job builds it for, whether it is a Kernel crate, where its
//! Kernel_Unsafe_Module is, and whether it is a Flight_Build root. Unknown
//! keys are errors, so a misspelt key cannot silently weaken a rule.

use std::collections::BTreeMap;
use std::path::{Component, Path};

use serde_json::Value;

/// The `targets` entry that stands for the host.
pub const HOST: &str = "host";

/// PAR-01: the default maximum number of executable Kernel lines.
pub const DEFAULT_MAX_EXEC_LINES: u64 = 5_000;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CratePolicy {
    /// `host` and/or the Target triple.
    pub targets: Vec<String>,
    /// Kernel crate (R6.1, R6.3).
    pub kernel: bool,
    /// Flight_Build root (PR-36 applies to the closure of these crates).
    pub flight: bool,
    /// Kernel_Unsafe_Module of a Kernel crate: a module path relative to the
    /// package directory, without extension (for example
    /// `src/arch/cm4/unsafe_module`).
    pub unsafe_module: Option<String>,
    /// The whole library belongs to the Kernel_Unsafe_Module.
    pub unsafe_crate: bool,
    /// A Flight_Build root whose binaries the verification job runs on
    /// QEMU (the dispatch spike of task 3.1).
    pub qemu_spike: bool,
    /// An App_Declaration crate (`rsk::app!` at its root): `cargo xtask gen`
    /// runs the Generator on it (R24.3) and the verification job checks
    /// that the generated files next to it are current (R24.4).
    pub declaration: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Policy {
    /// The Target triple.
    pub target: String,
    /// R6.2: the maximum number of executable Kernel lines (PAR-01 unless
    /// `kernel-max-exec-lines` overrides it, as the fixtures do).
    pub max_exec_lines: u64,
    pub crates: BTreeMap<String, CratePolicy>,
}

impl Policy {
    /// Reads the policy from `cargo metadata`'s top-level `metadata` value
    /// (the workspace metadata table).
    pub fn from_workspace_metadata(metadata: &Value) -> Result<Policy, String> {
        let rsk = metadata
            .get("rsk")
            .and_then(Value::as_object)
            .ok_or("the root Cargo.toml has no [workspace.metadata.rsk] table")?;
        for key in rsk.keys() {
            if key != "target" && key != "crates" && key != "kernel-max-exec-lines" {
                return Err(format!("unknown key `{key}` in [workspace.metadata.rsk]"));
            }
        }
        let max_exec_lines = match rsk.get("kernel-max-exec-lines") {
            None => DEFAULT_MAX_EXEC_LINES,
            Some(v) => v
                .as_u64()
                .ok_or("[workspace.metadata.rsk] `kernel-max-exec-lines` must be a positive integer")?,
        };
        let target = rsk
            .get("target")
            .and_then(Value::as_str)
            .ok_or("[workspace.metadata.rsk] lacks `target`")?
            .to_string();
        let table = rsk
            .get("crates")
            .and_then(Value::as_object)
            .ok_or("[workspace.metadata.rsk] lacks a `crates` table")?;
        let mut crates = BTreeMap::new();
        for (name, entry) in table {
            crates.insert(name.clone(), parse_crate(name, entry, &target)?);
        }
        let unsafe_modules = crates
            .values()
            .filter(|c| c.unsafe_module.is_some())
            .count();
        let unsafe_crates = crates.values().filter(|c| c.unsafe_crate).count();
        if unsafe_modules > 1 || unsafe_crates > 1 {
            return Err(
                "the Kernel_Unsafe_Module is one module of the Kernel crate plus the entry crate (R6.3): \
                 at most one `unsafe-module` and one `unsafe-crate` are allowed"
                    .to_string(),
            );
        }
        Ok(Policy {
            target,
            max_exec_lines,
            crates,
        })
    }

    pub fn builds_for(&self, name: &str, target: &str) -> bool {
        self.crates
            .get(name)
            .is_some_and(|c| c.targets.iter().any(|t| t == target))
    }

    pub fn kernel_crates(&self) -> impl Iterator<Item = &str> {
        self.crates
            .iter()
            .filter(|(_, c)| c.kernel)
            .map(|(n, _)| n.as_str())
    }

    pub fn flight_roots(&self) -> impl Iterator<Item = &str> {
        self.crates
            .iter()
            .filter(|(_, c)| c.flight)
            .map(|(n, _)| n.as_str())
    }

    /// Flight_Build roots marked `qemu-spike = true`.
    pub fn qemu_spikes(&self) -> impl Iterator<Item = &str> {
        self.crates
            .iter()
            .filter(|(_, c)| c.flight && c.qemu_spike)
            .map(|(n, _)| n.as_str())
    }
}

fn parse_crate(name: &str, entry: &Value, target: &str) -> Result<CratePolicy, String> {
    let table = entry
        .as_object()
        .ok_or_else(|| format!("policy entry `{name}` is not a table"))?;
    let mut policy = CratePolicy::default();
    for (key, value) in table {
        let bad = || format!("policy entry `{name}`: `{key}` has the wrong type");
        match key.as_str() {
            "targets" => {
                let list = value.as_array().ok_or_else(bad)?;
                for t in list {
                    let t = t.as_str().ok_or_else(bad)?;
                    if t != HOST && t != target {
                        return Err(format!(
                            "policy entry `{name}`: target `{t}` is neither `{HOST}` nor `{target}`"
                        ));
                    }
                    policy.targets.push(t.to_string());
                }
            }
            "kernel" => policy.kernel = value.as_bool().ok_or_else(bad)?,
            "flight" => policy.flight = value.as_bool().ok_or_else(bad)?,
            "qemu-spike" => policy.qemu_spike = value.as_bool().ok_or_else(bad)?,
            "declaration" => policy.declaration = value.as_bool().ok_or_else(bad)?,
            "unsafe-crate" => policy.unsafe_crate = value.as_bool().ok_or_else(bad)?,
            "unsafe-module" => {
                let path = value.as_str().ok_or_else(bad)?;
                let plain = Path::new(path)
                    .components()
                    .all(|c| matches!(c, Component::Normal(_)));
                if !plain || Path::new(path).extension().is_some() {
                    return Err(format!(
                        "policy entry `{name}`: `unsafe-module` must be a relative module path without \
                         extension, `..`, or root, such as `src/arch/cm4/unsafe_module`"
                    ));
                }
                policy.unsafe_module = Some(path.to_string());
            }
            other => return Err(format!("policy entry `{name}`: unknown key `{other}`")),
        }
    }
    if policy.targets.is_empty() {
        return Err(format!("policy entry `{name}` lists no targets"));
    }
    if (policy.unsafe_module.is_some() || policy.unsafe_crate) && !policy.kernel {
        return Err(format!(
            "policy entry `{name}`: only Kernel crates contain the Kernel_Unsafe_Module (R6.3)"
        ));
    }
    if policy.qemu_spike && !policy.flight {
        return Err(format!("policy entry `{name}`: only Flight_Build roots can be QEMU spikes"));
    }
    if policy.unsafe_module.is_some() && policy.unsafe_crate {
        return Err(format!(
            "policy entry `{name}`: `unsafe-module` and `unsafe-crate` are mutually exclusive"
        ));
    }
    Ok(policy)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn parse(v: Value) -> Result<Policy, String> {
        Policy::from_workspace_metadata(&json!({ "rsk": v }))
    }

    #[test]
    fn reads_a_valid_policy() {
        let policy = parse(json!({
            "target": "thumbv7em-none-eabihf",
            "crates": {
                "k": { "targets": ["host", "thumbv7em-none-eabihf"], "kernel": true, "flight": true,
                       "unsafe-module": "src/arch/unsafe_module" },
                "tool": { "targets": ["host"] }
            }
        }))
        .expect("valid policy");
        assert!(policy.builds_for("k", "thumbv7em-none-eabihf"));
        assert!(!policy.builds_for("tool", "thumbv7em-none-eabihf"));
        assert_eq!(policy.kernel_crates().collect::<Vec<_>>(), vec!["k"]);
        assert_eq!(policy.flight_roots().collect::<Vec<_>>(), vec!["k"]);
    }

    #[test]
    fn rejects_unknown_keys_and_misplaced_unsafe_modules() {
        let base = |entry: Value| json!({ "target": "t", "crates": { "c": entry } });
        assert!(parse(base(json!({ "targets": ["host"], "kernal": true }))).is_err());
        assert!(parse(base(
            json!({ "targets": ["host"], "unsafe-module": "src/u" })
        ))
        .is_err());
        assert!(parse(base(
            json!({ "targets": ["host"], "kernel": true, "unsafe-module": "../u" })
        ))
        .is_err());
        assert!(parse(base(json!({ "targets": ["x86_64-unknown-linux-gnu"] }))).is_err());
        assert!(parse(base(json!({ "targets": [] }))).is_err());
    }
}
