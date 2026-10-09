//! The analysis report (`rsk-analysis-report/1`, R30) as serde types.

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "rsk-analysis-report/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Inputs {
    pub task_model: String,
    pub wcet_records: String,
    pub kernel_timing: String,
    pub build_manifest: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: String,
    pub variant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Interference {
    pub task: String,
    pub e_cycles: u64,
    pub jobs: u64,
    pub cycles: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskResult {
    pub id: String,
    pub partition: String,
    pub level: String,
    pub priority: u8,
    pub period_cycles: u64,
    pub deadline_cycles: u64,
    pub c_cycles: u64,
    pub b_cycles: u64,
    pub b_source: String,
    pub o_cycles: u64,
    pub j_cycles: u64,
    pub interference: Vec<Interference>,
    pub r_cycles: u64,
    pub schedulable: bool,
    /// `deadline - R` when schedulable, else 0.
    pub slack_cycles: u64,
    pub wcet_provenance: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CrossCheck {
    pub status: String,
    pub detail: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub result: String,
    pub statements: Vec<String>,
    pub inputs: Inputs,
    pub analyzer: String,
    pub profile: Profile,
    pub tasks: Vec<TaskResult>,
    pub utilization_percent: u64,
    pub cross_check: CrossCheck,
    pub omitted_terms: Vec<String>,
    pub rejections: Vec<String>,
}
