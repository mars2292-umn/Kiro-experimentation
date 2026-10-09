//! R58.2 and R58.5: the dependency report of the Flight_Build graph and the
//! Trust_Base_Register gate.
//!
//! After the Target build, every compilation unit of the Flight_Build graph
//! is known (Cargo's `compiler-artifact` messages): the crates linked for
//! the Target, the proc-macros and their dependencies compiled for the host,
//! and the build scripts that ran. For each third-party crate among them,
//! the report states whether its sources contain `unsafe` code or assembly
//! macros, whether it has a build script, and whether it is a proc-macro.
//! Each crate with any of these properties must have an entry for its exact
//! version in `docs/trust_base_register.toml` (the machine-readable source
//! of the Trust_Base_Register), or the build fails. A new crate or a version
//! change therefore cannot enter the Flight_Build graph before its entry
//! exists (R58.5).
//!
//! The `unsafe` count is a token count of the `unsafe` keyword over every
//! `.rs` file of the crate directory, including code that `cfg` leaves out
//! and macro bodies, so it over-approximates what is compiled. It is
//! reported for review, not used as a threshold.

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use toml::{Table, Value};

use crate::diag::{Report, Rule};
use crate::tokens::{lex_file, unsafe_constructs};
use crate::units::Outcome;
use crate::workspace::{rust_files, Workspace};

pub const REGISTER: &str = "docs/trust_base_register.toml";

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrateReport {
    pub name: String,
    pub version: String,
    pub member: bool,
    pub unsafe_count: usize,
    pub asm_count: usize,
    pub build_script: bool,
    pub proc_macro: bool,
    /// `target`, `host`, or both.
    pub roles: BTreeSet<&'static str>,
}

impl CrateReport {
    /// Whether R58.2 requires a Trust_Base_Register entry.
    pub fn needs_entry(&self) -> bool {
        self.unsafe_count > 0 || self.asm_count > 0 || self.build_script || self.proc_macro
    }

    pub fn properties(&self) -> Vec<String> {
        let mut p = Vec::new();
        if self.unsafe_count > 0 {
            p.push(format!("unsafe x{}", self.unsafe_count));
        }
        if self.asm_count > 0 {
            p.push(format!("asm x{}", self.asm_count));
        }
        if self.build_script {
            p.push("build script".to_string());
        }
        if self.proc_macro {
            p.push("proc-macro".to_string());
        }
        p
    }
}

/// The crates of the Flight_Build graph, from the Target build's units.
pub fn flight_crates(ws: &Workspace, outcome: &Outcome) -> Vec<CrateReport> {
    let mut by_id: BTreeMap<String, CrateReport> = BTreeMap::new();
    for unit in &outcome.units {
        let Some(pkg) = ws.packages.get(&unit.package_id) else { continue };
        let entry = by_id.entry(unit.package_id.clone()).or_insert_with(|| {
            let (unsafe_count, asm_count) = scan(&pkg.dir);
            CrateReport {
                name: pkg.name.clone(),
                version: pkg.version.clone(),
                member: pkg.member,
                unsafe_count,
                asm_count,
                build_script: pkg.dir.join("build.rs").is_file()
                    || pkg.targets.iter().any(|t| t.kinds.iter().any(|k| k == "custom-build")),
                proc_macro: pkg.is_proc_macro(),
                roles: BTreeSet::new(),
            }
        });
        entry
            .roles
            .insert(if unit.for_target { "target" } else { "host" });
    }
    by_id.into_values().collect()
}

fn scan(dir: &Path) -> (usize, usize) {
    let mut unsafe_count = 0;
    let mut asm_count = 0;
    for file in rust_files(dir) {
        let Ok(tokens) = lex_file(&file) else { continue };
        for finding in unsafe_constructs(&tokens) {
            if finding.what.starts_with("`unsafe` keyword") {
                unsafe_count += 1;
            } else if finding.what.starts_with("assembly macro") {
                asm_count += 1;
            }
        }
    }
    (unsafe_count, asm_count)
}

/// The `[[crate]]` entries of the Trust_Base_Register source.
pub fn register_entries(root: &Path, report: &mut Report) -> BTreeSet<(String, String)> {
    let path = root.join(REGISTER);
    let mut out = BTreeSet::new();
    let text = match std::fs::read_to_string(&path) {
        Ok(text) => text,
        Err(e) => {
            report.error(
                Rule::TrustBase,
                Some(REGISTER.to_string()),
                format!("cannot read the Trust_Base_Register source: {e}"),
            );
            return out;
        }
    };
    let table: Table = match text.parse() {
        Ok(t) => t,
        Err(e) => {
            report.error(
                Rule::TrustBase,
                Some(REGISTER.to_string()),
                format!("cannot parse the Trust_Base_Register source: {e}"),
            );
            return out;
        }
    };
    for entry in table.get("crate").and_then(Value::as_array).into_iter().flatten() {
        let name = entry.get("name").and_then(Value::as_str).unwrap_or_default();
        let version = entry.get("version").and_then(Value::as_str).unwrap_or_default();
        if name.is_empty() || version.is_empty() {
            report.error(
                Rule::TrustBase,
                Some(REGISTER.to_string()),
                "a [[crate]] entry lacks `name` or `version`",
            );
            continue;
        }
        for required in ["trusted_for", "evidence", "residual_risk"] {
            if entry.get(required).and_then(Value::as_str).is_none_or(|s| s.trim().is_empty()) {
                report.error(
                    Rule::TrustBase,
                    Some(REGISTER.to_string()),
                    format!("[[crate]] `{name} {version}` lacks `{required}` (R48.2)"),
                );
            }
        }
        out.insert((name.to_string(), version.to_string()));
    }
    out
}

pub fn check(ws: &Workspace, outcome: &Outcome, report: &mut Report) {
    let crates = flight_crates(ws, outcome);
    let entries = register_entries(&ws.root, report);
    let mut needing = 0usize;
    let mut covered = 0usize;
    for c in &crates {
        let roles: Vec<&str> = c.roles.iter().copied().collect();
        let props = c.properties();
        let props_text = if props.is_empty() {
            "none of unsafe/asm/build script/proc-macro".to_string()
        } else {
            props.join(", ")
        };
        report.note(format!(
            "  {} {} [{}] {}{}",
            c.name,
            c.version,
            roles.join("+"),
            props_text,
            if c.member { " (workspace)" } else { "" }
        ));
        if c.member || !c.needs_entry() {
            continue;
        }
        needing += 1;
        if entries.contains(&(c.name.clone(), c.version.clone())) {
            covered += 1;
        } else {
            report.error(
                Rule::TrustBase,
                Some(REGISTER.to_string()),
                format!(
                    "crate `{} {}` is in the Flight_Build graph with {} but has no Trust_Base_Register entry \
                     for that version",
                    c.name,
                    c.version,
                    props.join(", ")
                ),
            );
        }
    }
    for (name, version) in &entries {
        if !crates.iter().any(|c| c.name == *name && c.version == *version) {
            report.warn(format!(
                "{REGISTER}: entry for `{name} {version}` matches no crate of the Flight_Build graph"
            ));
        }
    }
    report.note(format!(
        "R58.2/R58.5: {} crates in the Flight_Build graph; {needing} third-party crates need a \
         Trust_Base_Register entry, {covered} have one",
        crates.len()
    ));
}
