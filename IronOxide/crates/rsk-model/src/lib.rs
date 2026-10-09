//! rsk-model: the published, versioned JSON Schemas of the Task_Model,
//! the WCET_Record set, the Kernel_Timing_Parameters, the Build_Manifest
//! and the analysis report (R25.1, R33.1, R30.1), the serde types that
//! read and write them, and the schema validator that every reading tool
//! applies before trusting a document (R25.6, R28.1).
//!
//! The Config_Checker (R26.1) uses only the schema files of this crate,
//! never its types.
#![forbid(unsafe_code)]

pub mod report;
pub mod schema;
pub mod task_model;
pub mod wcet;

pub use schema::{validate, Violation};
pub use task_model::TaskModel;
pub use wcet::{KernelTiming, WcetRecord, WcetSet};

/// The schema documents, as published.
pub const TASK_MODEL_SCHEMA: &str = include_str!("../schemas/task_model.json");
pub const WCET_RECORD_SCHEMA: &str = include_str!("../schemas/wcet_record.json");
pub const KERNEL_TIMING_SCHEMA: &str = include_str!("../schemas/kernel_timing.json");
pub const BUILD_MANIFEST_SCHEMA: &str = include_str!("../schemas/build_manifest.json");
pub const ANALYSIS_REPORT_SCHEMA: &str = include_str!("../schemas/analysis_report.json");

/// A document rejected before parsing: schema violations with their JSON
/// paths, or an unsupported schema version (R25.6).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Rejection {
    NotJson(String),
    UnsupportedVersion { found: String, supported: &'static str },
    Schema(Vec<Violation>),
    Parse(String),
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Rejection::NotJson(e) => write!(f, "not a JSON document: {e}"),
            Rejection::UnsupportedVersion { found, supported } => {
                write!(f, "schema version `{found}` is not supported (this tool reads `{supported}`)")
            }
            Rejection::Schema(v) => {
                write!(f, "{} schema violation(s):", v.len())?;
                for x in v {
                    write!(f, "\n  {x}")?;
                }
                Ok(())
            }
            Rejection::Parse(e) => write!(f, "cannot parse: {e}"),
        }
    }
}

fn schema_value(schema: &str) -> serde_json::Value {
    serde_json::from_str(schema).expect("the published schemas are valid JSON")
}

/// Checks the schema version, validates against the schema, then parses.
pub fn read_checked<T: serde::de::DeserializeOwned>(schema: &str, expected: &'static str, text: &str) -> Result<T, Rejection> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| Rejection::NotJson(e.to_string()))?;
    match value.get("schema").and_then(serde_json::Value::as_str) {
        Some(v) if v == expected => {}
        Some(v) => {
            return Err(Rejection::UnsupportedVersion {
                found: v.to_string(),
                supported: expected,
            })
        }
        None => {
            return Err(Rejection::Schema(vec![Violation {
                path: String::new(),
                rule: "required: missing property `schema`".into(),
            }]))
        }
    }
    let violations = validate(&schema_value(schema), &value);
    if !violations.is_empty() {
        return Err(Rejection::Schema(violations));
    }
    serde_json::from_value(value).map_err(|e| Rejection::Parse(e.to_string()))
}

pub fn read_task_model(text: &str) -> Result<TaskModel, Rejection> {
    read_checked(TASK_MODEL_SCHEMA, task_model::SCHEMA, text)
}

pub fn read_wcet_set(text: &str) -> Result<WcetSet, Rejection> {
    read_checked(WCET_RECORD_SCHEMA, wcet::WCET_SCHEMA, text)
}

pub fn read_kernel_timing(text: &str) -> Result<KernelTiming, Rejection> {
    read_checked(KERNEL_TIMING_SCHEMA, wcet::KERNEL_TIMING_SCHEMA, text)
}

pub fn read_report(text: &str) -> Result<report::Report, Rejection> {
    read_checked(ANALYSIS_REPORT_SCHEMA, report::SCHEMA, text)
}

/// Validates a Build_Manifest document against its schema (the manifest's
/// own type lives in the Build_System; readers use the JSON value).
pub fn read_build_manifest(text: &str) -> Result<serde_json::Value, Rejection> {
    let value: serde_json::Value = serde_json::from_str(text).map_err(|e| Rejection::NotJson(e.to_string()))?;
    match value.get("schema").and_then(serde_json::Value::as_str) {
        Some("rsk-build-manifest/1") => {}
        Some(v) => {
            return Err(Rejection::UnsupportedVersion {
                found: v.to_string(),
                supported: "rsk-build-manifest/1",
            })
        }
        None => return Err(Rejection::Schema(vec![Violation { path: String::new(), rule: "required: missing property `schema`".into() }])),
    }
    let violations = validate(&schema_value(BUILD_MANIFEST_SCHEMA), &value);
    if !violations.is_empty() {
        return Err(Rejection::Schema(violations));
    }
    Ok(value)
}

/// Canonical JSON (two-space indentation, keys in struct order) for a
/// printer whose documents must be byte-identical for equal values (R25.4).
pub fn print<T: serde::Serialize>(value: &T) -> String {
    let mut s = serde_json::to_string_pretty(value).expect("serializable");
    s.push('\n');
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");

    #[test]
    fn the_schemas_are_valid_json_using_only_supported_keywords() {
        for (name, schema) in [
            ("task_model", TASK_MODEL_SCHEMA),
            ("wcet_record", WCET_RECORD_SCHEMA),
            ("kernel_timing", KERNEL_TIMING_SCHEMA),
            ("build_manifest", BUILD_MANIFEST_SCHEMA),
            ("analysis_report", ANALYSIS_REPORT_SCHEMA),
        ] {
            let v = schema_value(schema);
            // Validating an empty object exercises every keyword of the root.
            let violations = validate(&v, &serde_json::json!({}));
            assert!(!violations.iter().any(|x| x.rule.contains("unsupported")), "{name}: {violations:?}");
        }
    }

    #[test]
    fn the_generated_demo_model_conforms_and_round_trips() {
        let model = read_task_model(DEMO).unwrap_or_else(|e| panic!("{e}"));
        assert_eq!(model.tasks.len(), 3);
        assert_eq!(model.resources[0].ceiling, 7);
        let printed = print(&model);
        let again = read_task_model(&printed).unwrap();
        assert_eq!(model, again);
        // The Generator's own printer and this crate's printer agree byte for byte.
        assert_eq!(printed, DEMO);
    }

    #[test]
    fn schema_violations_name_their_path_and_versions_are_checked() {
        let bad = DEMO.replace("\"priority\": 7", "\"priority\": 9");
        match read_task_model(&bad) {
            Err(Rejection::Schema(v)) => assert_eq!(v[0].path, "/tasks/0/priority"),
            other => panic!("{other:?}"),
        }
        let old = DEMO.replace("rsk-task-model/1", "rsk-task-model/0");
        assert!(matches!(read_task_model(&old), Err(Rejection::UnsupportedVersion { .. })));
        let extra = DEMO.replacen("\"generator\":", "\"bogus\": 1,\n  \"generator\":", 1);
        assert!(matches!(read_task_model(&extra), Err(Rejection::Schema(_))));
    }

    #[test]
    fn wcet_and_timing_documents_round_trip() {
        let set = WcetSet {
            schema: wcet::WCET_SCHEMA.into(),
            records: vec![WcetRecord {
                item: "part0.control".into(),
                wcet_cycles: 912,
                method: "measured".into(),
                tool: "rsk-wcet 0.0.0".into(),
                binary_hash: "sha256:00".into(),
                target: "nrf52840/r0p1".into(),
                clock_hz: 64_000_000,
                icache: "off".into(),
                conditions: vec!["dma-max".into()],
                measured: Some(wcet::Measured {
                    runs: 10_000,
                    max_observed: 760,
                    margin_percent: 20,
                    coverage_branch_percent: 97,
                    scenarios: vec!["fp-active".into()],
                }),
            }],
        };
        let text = print(&set);
        assert_eq!(read_wcet_set(&text).unwrap(), set);
        let timing = KernelTiming {
            schema: wcet::KERNEL_TIMING_SCHEMA.into(),
            binary_hash: "sha256:00".into(),
            target: "nrf52840/r0p1".into(),
            clock_hz: 64_000_000,
            icache: "off".into(),
            parameters: wcet::KERNEL_TIMING_SYMBOLS
                .iter()
                .map(|s| wcet::KernelTimingParameter {
                    symbol: s.to_string(),
                    cycles: 100,
                    method: "placeholder".into(),
                    tool: "test".into(),
                    conditions: vec![],
                })
                .collect(),
        };
        let text = print(&timing);
        let again = read_kernel_timing(&text).unwrap();
        assert_eq!(again, timing);
        assert!(again.missing().is_empty());
    }
}
