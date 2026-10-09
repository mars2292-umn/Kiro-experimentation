//! Compilation units of a Cargo build, from its JSON messages.
//!
//! The Target build runs with `--message-format=json-render-diagnostics`.
//! Each `compiler-artifact` message names one compiled unit with its exact
//! feature set and profile; each `build-script-executed` message carries the
//! cfgs and environment variables a build script emitted. Together with
//! `rustc --print cfg` they give the configuration every crate was compiled
//! with, which the post-build checks use.

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::cfg::CfgSet;
use crate::cmd::{stdout_of, Cmd};

#[derive(Clone, Debug)]
pub struct Unit {
    pub package_id: String,
    pub kinds: Vec<String>,
    pub src_path: PathBuf,
    pub features: Vec<String>,
    pub debug_assertions: bool,
    /// Compiled for the Target (as opposed to the host).
    pub for_target: bool,
}

impl Unit {
    pub fn is_build_script(&self) -> bool {
        self.kinds.iter().any(|k| k == "custom-build")
    }
}

#[derive(Clone, Debug)]
pub struct ScriptRun {
    pub package_id: String,
    pub cfgs: Vec<String>,
    pub env: Vec<(String, String)>,
}

#[derive(Clone, Debug, Default)]
pub struct Outcome {
    pub units: Vec<Unit>,
    pub scripts: Vec<ScriptRun>,
}

/// Parses Cargo's JSON messages. Artifacts under `<target_dir>/<triple>/`
/// were compiled for the Target; all others for the host.
pub fn parse_messages(stdout: &str, target_dir: &Path, triple: &str) -> Outcome {
    let target_prefix = target_dir.join(triple);
    let mut outcome = Outcome::default();
    for line in stdout.lines() {
        let Ok(msg) = serde_json::from_str::<Value>(line) else {
            continue;
        };
        let package_id = msg
            .get("package_id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        match msg.get("reason").and_then(Value::as_str) {
            Some("compiler-artifact") => {
                let target = msg.get("target");
                let kinds = target
                    .and_then(|t| t.get("kind"))
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                let src_path = target
                    .and_then(|t| t.get("src_path"))
                    .and_then(Value::as_str)
                    .map(PathBuf::from)
                    .unwrap_or_default();
                let features = msg
                    .get("features")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                let debug_assertions = msg
                    .get("profile")
                    .and_then(|p| p.get("debug_assertions"))
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
                let for_target = msg
                    .get("filenames")
                    .and_then(Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(Value::as_str)
                    .any(|f| Path::new(f).starts_with(&target_prefix));
                outcome.units.push(Unit {
                    package_id,
                    kinds,
                    src_path,
                    features,
                    debug_assertions,
                    for_target,
                });
            }
            Some("build-script-executed") => {
                let list = |key: &str| -> Vec<Value> {
                    msg.get(key)
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default()
                };
                let cfgs = list("cfgs")
                    .iter()
                    .filter_map(Value::as_str)
                    .map(str::to_string)
                    .collect();
                let env = list("env")
                    .iter()
                    .filter_map(|pair| {
                        let pair = pair.as_array()?;
                        Some((
                            pair.first()?.as_str()?.to_string(),
                            pair.get(1)?.as_str()?.to_string(),
                        ))
                    })
                    .collect();
                outcome.scripts.push(ScriptRun {
                    package_id,
                    cfgs,
                    env,
                });
            }
            _ => {}
        }
    }
    outcome
}

/// `rustc --print cfg` for the host and the Target.
#[derive(Clone, Debug)]
pub struct CfgContext {
    pub host: CfgSet,
    pub target: CfgSet,
}

impl CfgContext {
    pub fn load(root: &Path, triple: &str) -> Result<CfgContext, String> {
        let host = stdout_of(&Cmd::rustc().args(["--print", "cfg"]).cwd(root))?;
        let target = stdout_of(
            &Cmd::rustc()
                .args(["--print", "cfg", "--target", triple])
                .cwd(root),
        )?;
        Ok(CfgContext {
            host: CfgSet::from_rustc_print(&host),
            target: CfgSet::from_rustc_print(&target),
        })
    }

    /// The configuration `unit` was compiled with.
    pub fn for_unit(&self, unit: &Unit, scripts: &[ScriptRun]) -> CfgSet {
        let mut set = if unit.for_target {
            self.target.clone()
        } else {
            self.host.clone()
        };
        set.set_name("debug_assertions", unit.debug_assertions);
        for feature in &unit.features {
            set.insert_feature(feature);
        }
        if !unit.is_build_script() {
            for script in scripts.iter().filter(|s| s.package_id == unit.package_id) {
                for spec in &script.cfgs {
                    set.insert_spec(spec);
                }
            }
        }
        set
    }
}
