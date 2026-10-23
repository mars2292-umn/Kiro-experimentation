//! The Config_Checker's own reading of the Task_Model (R26.1): plain
//! structures filled from the JSON by name, nothing shared with the
//! Generator or the schema crate.

use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub base: u64,
    pub size_log2: u8,
    pub srd: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Window {
    pub region: Region,
    pub peripherals: Vec<String>,
    pub read_only: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Partition {
    pub id: String,
    pub level: String,
    pub fault: String,
    pub overrun: String,
    pub deadline_miss: String,
    pub mit_threshold: u64,
    pub stack: u64,
    pub peripherals: Vec<String>,
    pub code: Region,
    pub ram: Region,
    pub windows: Vec<Window>,
    pub tasks: Vec<String>,
    pub resources: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Task {
    pub id: String,
    pub partition: String,
    pub priority: u64,
    pub vector: u64,
    pub sporadic: bool,
    pub offset_ticks: u64,
    /// Period (periodic) or MIT (sporadic).
    pub period_ticks: u64,
    pub deadline_ticks: u64,
    pub budget_cycles: u64,
    pub mit_policy: Option<String>,
    pub fpu: bool,
    pub resources: Vec<String>,
    pub nesting_depth: u64,
    pub slot_release: u64,
    pub slot_deadline: u64,
    pub slot_budget: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Resource {
    pub id: String,
    pub partition: String,
    pub ceiling: u64,
    pub accessors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub target: String,
    pub config_checksum: u32,
    pub layout_checksum: u32,
    pub operating_duration_ticks: u64,
    pub kernel_levels: Vec<u64>,
    /// (level, nvic byte)
    pub priority_map: Vec<(u64, u64)>,
    pub lock_stack: u64,
    pub timer_slots: u64,
    pub event_log: u64,
    pub kernel_flash: Region,
    pub shared_code: Region,
    pub kernel_ram: Region,
    pub partitions: Vec<Partition>,
    pub tasks: Vec<Task>,
    pub resources: Vec<Resource>,
}

fn field<'a>(v: &'a Value, name: &str, at: &str) -> Result<&'a Value, String> {
    v.get(name).ok_or_else(|| format!("{at}: missing field `{name}`"))
}

fn u64_of(v: &Value, name: &str, at: &str) -> Result<u64, String> {
    field(v, name, at)?.as_u64().ok_or_else(|| format!("{at}: `{name}` is not an unsigned integer"))
}

fn str_of(v: &Value, name: &str, at: &str) -> Result<String, String> {
    field(v, name, at)?.as_str().map(str::to_string).ok_or_else(|| format!("{at}: `{name}` is not a string"))
}

fn bool_of(v: &Value, name: &str, at: &str) -> Result<bool, String> {
    field(v, name, at)?.as_bool().ok_or_else(|| format!("{at}: `{name}` is not a boolean"))
}

fn strings_of(v: &Value, name: &str, at: &str) -> Result<Vec<String>, String> {
    field(v, name, at)?
        .as_array()
        .ok_or_else(|| format!("{at}: `{name}` is not a list"))?
        .iter()
        .map(|x| x.as_str().map(str::to_string).ok_or_else(|| format!("{at}: `{name}` holds a non-string")))
        .collect()
}

fn region_of(v: &Value, at: &str) -> Result<Region, String> {
    Ok(Region {
        base: u64_of(v, "base", at)?,
        size_log2: u8::try_from(u64_of(v, "size_log2", at)?).map_err(|_| format!("{at}: size_log2 out of range"))?,
        srd: u8::try_from(u64_of(v, "srd", at)?).map_err(|_| format!("{at}: srd out of range"))?,
    })
}

impl Model {
    pub fn parse(json: &Value) -> Result<Model, String> {
        let at = "task_model";
        let schema = str_of(json, "schema", at)?;
        if schema != "rsk-task-model/1" {
            return Err(format!("unsupported Task_Model schema `{schema}` (this checker reads rsk-task-model/1)"));
        }
        let checksum_text = str_of(json, "config_checksum", at)?;
        let config_checksum = u32::from_str_radix(checksum_text.trim_start_matches("0x"), 16)
            .map_err(|_| format!("{at}: config_checksum `{checksum_text}` is not a hexadecimal word"))?;
        let layout_text = str_of(json, "layout_checksum", at)?;
        let layout_checksum = u32::from_str_radix(layout_text.trim_start_matches("0x"), 16)
            .map_err(|_| format!("{at}: layout_checksum `{layout_text}` is not a hexadecimal word"))?;
        let caps = field(json, "capacities", at)?;
        let layout = field(json, "layout", at)?;
        let mut partitions = Vec::new();
        for (i, p) in field(json, "partitions", at)?.as_array().ok_or("partitions is not a list")?.iter().enumerate() {
            let pat = format!("partitions[{i}]");
            let regions = field(p, "regions", &pat)?;
            let mpu = field(p, "mpu", &pat)?;
            let mut windows = Vec::new();
            for (w, win) in field(mpu, "windows", &pat)?.as_array().ok_or("windows is not a list")?.iter().enumerate() {
                let wat = format!("{pat}.mpu.windows[{w}]");
                windows.push(Window {
                    region: region_of(win, &wat)?,
                    peripherals: strings_of(win, "peripherals", &wat)?,
                    read_only: bool_of(win, "read_only", &wat)?,
                });
            }
            partitions.push(Partition {
                id: str_of(p, "id", &pat)?,
                level: str_of(p, "level", &pat)?,
                fault: str_of(p, "fault", &pat)?,
                overrun: str_of(p, "overrun", &pat)?,
                deadline_miss: str_of(p, "deadline_miss", &pat)?,
                mit_threshold: u64_of(p, "mit_threshold", &pat)?,
                stack: u64_of(regions, "stack", &pat)?,
                peripherals: strings_of(p, "peripherals", &pat)?,
                code: region_of(field(mpu, "code", &pat)?, &pat)?,
                ram: region_of(field(mpu, "ram", &pat)?, &pat)?,
                windows,
                tasks: strings_of(p, "tasks", &pat)?,
                resources: strings_of(p, "resources", &pat)?,
            });
        }
        let mut tasks = Vec::new();
        for (i, t) in field(json, "tasks", at)?.as_array().ok_or("tasks is not a list")?.iter().enumerate() {
            let tat = format!("tasks[{i}]");
            let kind = str_of(t, "kind", &tat)?;
            let sporadic = match kind.as_str() {
                "periodic" => false,
                "sporadic" => true,
                other => return Err(format!("{tat}: unknown kind `{other}`")),
            };
            let slots = field(t, "slots", &tat)?;
            tasks.push(Task {
                id: str_of(t, "id", &tat)?,
                partition: str_of(t, "partition", &tat)?,
                priority: u64_of(t, "priority", &tat)?,
                vector: u64_of(t, "vector", &tat)?,
                sporadic,
                offset_ticks: if sporadic { 0 } else { u64_of(t, "offset_ticks", &tat)? },
                period_ticks: if sporadic { u64_of(t, "mit_ticks", &tat)? } else { u64_of(t, "period_ticks", &tat)? },
                deadline_ticks: u64_of(t, "deadline_ticks", &tat)?,
                budget_cycles: u64_of(t, "budget_cycles", &tat)?,
                mit_policy: if sporadic { Some(str_of(t, "mit_policy", &tat)?) } else { None },
                fpu: bool_of(t, "fpu", &tat)?,
                resources: strings_of(t, "resources", &tat)?,
                nesting_depth: u64_of(t, "nesting_depth", &tat)?,
                slot_release: u64_of(slots, "release", &tat)?,
                slot_deadline: u64_of(slots, "deadline", &tat)?,
                slot_budget: u64_of(slots, "budget", &tat)?,
            });
        }
        let mut resources = Vec::new();
        for (i, r) in field(json, "resources", at)?.as_array().ok_or("resources is not a list")?.iter().enumerate() {
            let rat = format!("resources[{i}]");
            resources.push(Resource {
                id: str_of(r, "id", &rat)?,
                partition: str_of(r, "partition", &rat)?,
                ceiling: u64_of(r, "ceiling", &rat)?,
                accessors: strings_of(r, "accessors", &rat)?,
            });
        }
        let mut priority_map = Vec::new();
        for (i, e) in field(json, "priority_map", at)?.as_array().ok_or("priority_map is not a list")?.iter().enumerate() {
            let eat = format!("priority_map[{i}]");
            priority_map.push((u64_of(e, "level", &eat)?, u64_of(e, "nvic", &eat)?));
        }
        let kernel_levels = field(json, "kernel_levels", at)?
            .as_array()
            .ok_or("kernel_levels is not a list")?
            .iter()
            .map(|x| x.as_u64().ok_or_else(|| "kernel_levels holds a non-integer".to_string()))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Model {
            target: str_of(json, "target", at)?,
            config_checksum,
            layout_checksum,
            operating_duration_ticks: u64_of(json, "operating_duration_ticks", at)?,
            kernel_levels,
            priority_map,
            lock_stack: u64_of(caps, "lock_stack", "capacities")?,
            timer_slots: u64_of(caps, "timer_slots", "capacities")?,
            event_log: u64_of(caps, "event_log", "capacities")?,
            kernel_flash: region_of(field(layout, "kernel_flash", "layout")?, "layout.kernel_flash")?,
            shared_code: region_of(field(layout, "shared_code", "layout")?, "layout.shared_code")?,
            kernel_ram: region_of(field(layout, "kernel_ram", "layout")?, "layout.kernel_ram")?,
            partitions,
            tasks,
            resources,
        })
    }

    pub fn task_index(&self, id: &str) -> Option<usize> {
        self.tasks.iter().position(|t| t.id == id)
    }

    pub fn partition_index(&self, id: &str) -> Option<usize> {
        self.partitions.iter().position(|p| p.id == id)
    }
}
