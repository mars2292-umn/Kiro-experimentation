//! Evidence_Items (R50.1, R50.3): one versioned JSON document per proof run,
//! test run, analysis, measurement, check, or review, traced to the
//! acceptance criteria it covers.
//!
//! Format (`rsk-evidence/1`):
//!
//! ```json
//! {
//!   "schema": "rsk-evidence/1",
//!   "id": "check-static-checks",
//!   "kind": "check",                      // proof | test | analysis | measurement | check | review
//!   "name": "static checks",
//!   "covers": ["R6.1", "R6.3", "PR-20"],   // acceptance criteria and restrictions
//!   "verdict": "pass",                     // pass | fail | inconclusive
//!   "source_revision": "...", "source_dirty": false,
//!   "profile_version": "0.2",
//!   "tool": { "name": "cargo xtask", "version": "...", "solvers": { } },
//!   "command_lines": ["cargo build --frozen ..."],
//!   "build_manifest": "sha256:..." | null,  // identity of the Build_Manifest, for items about a binary
//!   "artifacts": ["kernel-proofs.log"],
//!   "notes": ["..."],
//!   "produced_at": "2026-10-08T12:00:00Z"   // excluded from determinism comparisons (R50.4)
//! }
//! ```
//!
//! `id` is stable across runs so that a re-run from the archived inputs
//! overwrites the item with one that differs only in `produced_at`.

use std::path::{Path, PathBuf};

use serde_json::{json, Value};

use crate::tools::{now_rfc3339, Revision};

pub const SCHEMA: &str = "rsk-evidence/1";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Proof,
    Test,
    Analysis,
    Measurement,
    Check,
    Review,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Proof => "proof",
            Kind::Test => "test",
            Kind::Analysis => "analysis",
            Kind::Measurement => "measurement",
            Kind::Check => "check",
            Kind::Review => "review",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Pass,
    Fail,
    Inconclusive,
}

impl Verdict {
    pub fn as_str(self) -> &'static str {
        match self {
            Verdict::Pass => "pass",
            Verdict::Fail => "fail",
            Verdict::Inconclusive => "inconclusive",
        }
    }

    pub fn from_bool(ok: bool) -> Verdict {
        if ok {
            Verdict::Pass
        } else {
            Verdict::Fail
        }
    }
}

#[derive(Clone, Debug)]
pub struct Tool {
    pub name: String,
    pub version: String,
    /// Solver or back-end versions (Z3 for Verus, CBMC for Kani).
    pub solvers: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
pub struct Item {
    pub id: String,
    pub kind: Kind,
    pub name: String,
    pub covers: Vec<String>,
    pub verdict: Verdict,
    pub tool: Tool,
    pub command_lines: Vec<String>,
    pub build_manifest: Option<String>,
    pub artifacts: Vec<String>,
    pub notes: Vec<String>,
}

impl Item {
    pub fn new(id: &str, kind: Kind, name: &str, tool: Tool, verdict: Verdict) -> Item {
        Item {
            id: id.to_string(),
            kind,
            name: name.to_string(),
            covers: Vec::new(),
            verdict,
            tool,
            command_lines: Vec::new(),
            build_manifest: None,
            artifacts: Vec::new(),
            notes: Vec::new(),
        }
    }

    pub fn covers(mut self, criteria: &[&str]) -> Item {
        self.covers.extend(criteria.iter().map(|c| c.to_string()));
        self
    }

    pub fn to_json(&self, revision: &Revision, profile_version: &str) -> Value {
        let mut covers = self.covers.clone();
        covers.sort();
        covers.dedup();
        json!({
            "schema": SCHEMA,
            "id": self.id,
            "kind": self.kind.as_str(),
            "name": self.name,
            "covers": covers,
            "verdict": self.verdict.as_str(),
            "source_revision": revision.commit,
            "source_dirty": revision.dirty,
            "profile_version": profile_version,
            "tool": {
                "name": self.tool.name,
                "version": self.tool.version,
                "solvers": self.tool.solvers.iter()
                    .map(|(k, v)| (k.clone(), Value::String(v.clone())))
                    .collect::<serde_json::Map<String, Value>>(),
            },
            "command_lines": self.command_lines,
            "build_manifest": self.build_manifest,
            "artifacts": self.artifacts,
            "notes": self.notes,
            "produced_at": now_rfc3339(),
        })
    }
}

/// Where the job stores Evidence_Items and the logs they refer to.
#[derive(Clone, Debug)]
pub struct Store {
    pub dir: PathBuf,
    pub revision: Revision,
    pub profile_version: String,
    pub written: Vec<PathBuf>,
}

impl Store {
    pub fn new(dir: &Path, revision: Revision, profile_version: &str) -> Store {
        Store {
            dir: dir.to_path_buf(),
            revision,
            profile_version: profile_version.to_string(),
            written: Vec::new(),
        }
    }

    pub fn item_path(&self, id: &str) -> PathBuf {
        self.dir.join(format!("{id}.json"))
    }

    /// Writes one item; returns its path.
    pub fn write(&mut self, item: &Item) -> Result<PathBuf, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("cannot create {}: {e}", self.dir.display()))?;
        let path = self.item_path(&item.id);
        let text = serde_json::to_string_pretty(&item.to_json(&self.revision, &self.profile_version))
            .map_err(|e| e.to_string())?;
        std::fs::write(&path, text + "\n").map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        self.written.push(path.clone());
        Ok(path)
    }

    /// Stores a log file next to the items and returns its file name.
    pub fn write_artifact(&self, name: &str, content: &str) -> Result<String, String> {
        std::fs::create_dir_all(&self.dir).map_err(|e| format!("cannot create {}: {e}", self.dir.display()))?;
        let path = self.dir.join(name);
        std::fs::write(&path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
        Ok(name.to_string())
    }
}

/// The acceptance criteria that every item in `dir` claims to cover, for
/// the traceability report (R50.6).
pub fn covered_criteria(dir: &Path) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let Ok(entries) = std::fs::read_dir(dir) else {
        return out;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.extension().is_some_and(|e| e == "json") {
            let Ok(text) = std::fs::read_to_string(&path) else { continue };
            let Ok(value) = serde_json::from_str::<Value>(&text) else { continue };
            if value.get("schema").and_then(Value::as_str) != Some(SCHEMA) {
                continue;
            }
            let id = value.get("id").and_then(Value::as_str).unwrap_or_default().to_string();
            for c in value.get("covers").and_then(Value::as_array).into_iter().flatten() {
                if let Some(c) = c.as_str() {
                    out.push((c.to_string(), id.clone()));
                }
            }
        }
    }
    out
}
