//! The Task_Model (`rsk-task-model/1`) as serde types (R25.5: the
//! Analyzer parses every document the printer produces into an equal
//! model; the Config_Checker has its own parser, R26.1).

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "rsk-task-model/1";

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TaskModel {
    pub schema: String,
    pub profile: Profile,
    pub generator: String,
    pub kernel: String,
    pub target: String,
    pub core_revision: String,
    pub clock: Clock,
    pub config_checksum: String,
    pub operating_duration_ticks: u64,
    pub safe_state: String,
    pub kernel_levels: Vec<u8>,
    pub priority_map: Vec<PriorityMapping>,
    pub capacities: Capacities,
    pub layout: Layout,
    pub partitions: Vec<Partition>,
    pub tasks: Vec<Task>,
    pub resources: Vec<Resource>,
    pub signals: Vec<Signal>,
    pub endpoints: Vec<Endpoint>,
    pub conversions: Vec<Conversion>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Profile {
    pub version: String,
    pub variant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Clock {
    pub cpu_hz: u64,
    pub tick_hz: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PriorityMapping {
    pub level: u8,
    pub nvic: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Capacities {
    pub lock_stack: u64,
    pub timer_slots: u64,
    pub event_log: u64,
    pub derived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Region {
    pub base: u32,
    pub size_log2: u8,
    pub srd: u8,
    pub usable_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Layout {
    pub kernel_flash: Region,
    pub shared_code: Region,
    pub kernel_ram: Region,
    pub derived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Window {
    pub base: u32,
    pub size_log2: u8,
    pub srd: u8,
    pub peripherals: Vec<String>,
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PartitionMpu {
    pub code: Region,
    pub ram: Region,
    pub windows: Vec<Window>,
    pub derived: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct RegionSizes {
    pub code: u64,
    pub ram: u64,
    pub stack: u64,
    pub init_arena: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Partition {
    pub id: String,
    #[serde(rename = "crate")]
    pub krate: String,
    pub level: String,
    pub fault: String,
    pub overrun: String,
    pub deadline_miss: String,
    pub mit_threshold: u32,
    pub regions: RegionSizes,
    pub peripherals: Vec<String>,
    pub mpu: PartitionMpu,
    pub tasks: Vec<String>,
    pub resources: Vec<String>,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields, untagged)]
pub enum Source {
    Interrupt { interrupt: String, irq: u16 },
    Signal { signal: String },
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Slots {
    pub release: Option<u8>,
    pub deadline: u8,
    pub budget: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub id: String,
    pub core: u8,
    pub partition: String,
    pub priority: u8,
    pub vector: u16,
    pub kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub offset_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub period_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mit_ticks: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub mit_policy: Option<String>,
    pub source: Option<Source>,
    pub deadline_ticks: u64,
    pub budget_cycles: u32,
    pub fpu: bool,
    pub resources: Vec<String>,
    pub raises: Vec<String>,
    pub nesting_depth: u8,
    pub slots: Slots,
    pub cs_sites: Vec<String>,
    pub entry: String,
    pub derived: Vec<String>,
}

impl Task {
    /// Period of a Periodic_Task or MIT of a Sporadic_Task, in ticks.
    pub fn window_ticks(&self) -> u64 {
        self.period_ticks.or(self.mit_ticks).unwrap_or(0)
    }
    pub fn is_periodic(&self) -> bool {
        self.kind == "periodic"
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Resource {
    pub id: String,
    pub core: serde_json::Value,
    pub partition: String,
    pub ceiling: u8,
    pub accessors: Vec<String>,
    #[serde(rename = "type")]
    pub ty: String,
    pub derived: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Signal {
    pub id: String,
    pub partition: String,
    pub bound_task: String,
    pub raised_by: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Endpoint {
    pub id: String,
    pub cores: Vec<u8>,
    pub capacity: u32,
    pub k: u32,
    pub ceiling: String,
    pub payload_bytes: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct DeclaredTime {
    pub value: u64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Conversion {
    pub item: String,
    pub declared: DeclaredTime,
    pub converted: u64,
    pub unit: String,
}

impl TaskModel {
    pub fn task(&self, id: &str) -> Option<&Task> {
        self.tasks.iter().find(|t| t.id == id)
    }
    pub fn resource(&self, id: &str) -> Option<&Resource> {
        self.resources.iter().find(|r| r.id == id)
    }
    pub fn partition(&self, id: &str) -> Option<&Partition> {
        self.partitions.iter().find(|p| p.id == id)
    }
}
