//! The Task_Model printer (R25): a JSON document conforming to
//! `rsk-task-model/1` (the schema is published by `rsk-model`), with
//! every declared value of R21.1, every derived value of R23.5 marked as
//! derived, stable identifiers for Tasks, Resources, Critical_Section
//! sites and Partitions (R25.2, R25.3), integer times with their unit in
//! the key name, and keys in canonical order (R25.4).

use crate::derive::{Generated, Kind, ReleaseSource, NO_SLOT};
use crate::json::Value;
use crate::layout::{Region, Window};
use crate::profile_version::PROFILE_VERSION;

pub const SCHEMA: &str = "rsk-task-model/1";
/// Generator and Kernel versions (the workspace version).
pub const GENERATOR_VERSION: &str = env!("CARGO_PKG_VERSION");

fn region(r: &Region) -> Value {
    Value::obj()
        .with("base", r.base)
        .with("size_log2", r.size_log2)
        .with("srd", r.srd)
        .with("usable_bytes", r.usable)
}

fn window(w: &Window) -> Value {
    Value::obj()
        .with("base", w.base)
        .with("size_log2", w.size_log2)
        .with("srd", w.srd)
        .with("peripherals", w.peripherals.clone())
        .with("read_only", w.read_only)
}

fn slot(s: u8) -> Value {
    if s == NO_SLOT {
        Value::Null
    } else {
        Value::Int(s as u64)
    }
}

/// The Critical_Section site identifiers of a Task: one per accessed
/// Resource (the `rsk` API gives a Task one proxy per Resource, R8.7).
pub fn cs_sites(g: &Generated, t: usize) -> Vec<String> {
    g.tasks[t].resources.iter().map(|&r| format!("{}#cs:{}", g.tasks[t].id, g.resources[r].name)).collect()
}

pub fn task_model(g: &Generated) -> Value {
    let tasks: Vec<Value> = g
        .tasks
        .iter()
        .map(|t| {
            let mut v = Value::obj()
                .with("id", t.id.as_str())
                .with("core", 0u8)
                .with("partition", g.partitions[t.partition].name.as_str())
                .with("priority", t.priority)
                .with("vector", t.vector())
                .with("kind", t.kind.name());
            v = match t.kind {
                Kind::Periodic => v.with("offset_ticks", t.offset_ticks).with("period_ticks", t.period_ticks),
                Kind::Sporadic => v.with("mit_ticks", t.period_ticks).with("mit_policy", t.mit_policy.name()),
            };
            let source = match &t.source {
                None => Value::Null,
                Some(ReleaseSource::Interrupt { peripheral, irq }) => {
                    Value::obj().with("interrupt", peripheral.as_str()).with("irq", *irq as u64)
                }
                Some(ReleaseSource::Signal(s)) => Value::obj().with("signal", s.as_str()),
            };
            v.with("source", source)
                .with("deadline_ticks", t.deadline_ticks)
                .with("budget_cycles", t.budget_cycles)
                .with("fpu", t.fpu)
                .with("resources", t.resources.iter().map(|&r| g.resources[r].id.clone()).collect::<Vec<_>>())
                .with("raises", t.raises.clone())
                .with("nesting_depth", t.nesting_depth)
                .with(
                    "slots",
                    Value::obj()
                        .with("release", slot(t.slot_release))
                        .with("deadline", slot(t.slot_deadline))
                        .with("budget", slot(t.slot_budget)),
                )
                .with("cs_sites", cs_sites(g, t.index))
                .with("entry", t.entry_path.as_str())
                .with("derived", vec!["vector".to_string(), "nesting_depth".to_string(), "slots".to_string(), "cs_sites".to_string()])
        })
        .collect();
    let resources: Vec<Value> = g
        .resources
        .iter()
        .map(|r| {
            Value::obj()
                .with("id", r.id.as_str())
                .with("core", 0u8)
                .with("partition", g.partitions[r.partition].name.as_str())
                .with("ceiling", r.ceiling)
                .with("accessors", r.accessors.iter().map(|&t| g.tasks[t].id.clone()).collect::<Vec<_>>())
                .with("type", r.ty.as_str())
                .with("derived", vec!["ceiling".to_string()])
        })
        .collect();
    let partitions: Vec<Value> = g
        .partitions
        .iter()
        .map(|p| {
            let regions = &g.plan.partitions[p.index];
            Value::obj()
                .with("id", p.name.as_str())
                .with("crate", p.krate.as_str())
                .with("level", p.level.name())
                .with("fault", p.fault.name())
                .with("overrun", p.overrun.name())
                .with("deadline_miss", p.deadline_miss.name())
                .with("mit_threshold", p.mit_threshold)
                .with(
                    "regions",
                    Value::obj()
                        .with("code", p.code_bytes)
                        .with("ram", p.ram_bytes)
                        .with("stack", p.stack_bytes)
                        .with("init_arena", p.init_arena),
                )
                .with("peripherals", p.peripherals.clone())
                .with(
                    "mpu",
                    Value::obj()
                        .with("code", region(&regions.code))
                        .with("ram", region(&regions.ram))
                        .with("windows", regions.windows.iter().map(window).collect::<Vec<_>>())
                        .with("derived", true),
                )
                .with("tasks", p.tasks.iter().map(|&t| g.tasks[t].id.clone()).collect::<Vec<_>>())
                .with("resources", p.resources.iter().map(|&r| g.resources[r].id.clone()).collect::<Vec<_>>())
                .with("signals", p.signals.clone())
        })
        .collect();
    let signals: Vec<Value> = g
        .partitions
        .iter()
        .flat_map(|p| {
            p.signals.iter().map(move |s| {
                let bound = g
                    .tasks
                    .iter()
                    .find(|t| matches!(&t.source, Some(ReleaseSource::Signal(x)) if x == s))
                    .map(|t| t.id.clone())
                    .unwrap_or_default();
                let raised_by: Vec<String> = g.tasks.iter().filter(|t| t.raises.contains(s)).map(|t| t.id.clone()).collect();
                Value::obj()
                    .with("id", format!("{}.{}", p.name, s))
                    .with("partition", p.name.as_str())
                    .with("bound_task", bound)
                    .with("raised_by", raised_by)
            })
        })
        .collect();
    let conversions: Vec<Value> = g
        .changes
        .iter()
        .map(|c| {
            Value::obj()
                .with("item", c.what.as_str())
                .with("declared", Value::obj().with("value", c.declared.value).with("unit", c.declared.unit.name()))
                .with("converted", c.converted)
                .with("unit", c.unit)
        })
        .collect();
    Value::obj()
        .with("schema", SCHEMA)
        .with("profile", Value::obj().with("version", PROFILE_VERSION).with("variant", g.variant.as_str()))
        .with("generator", GENERATOR_VERSION)
        .with("kernel", GENERATOR_VERSION)
        .with("target", g.target.model_name)
        .with("core_revision", g.target.core_revision)
        .with("clock", Value::obj().with("cpu_hz", g.target.cpu_hz).with("tick_hz", g.target.tick_hz))
        .with("config_checksum", format!("0x{:08x}", g.checksum))
        .with("operating_duration_ticks", g.operating_duration_ticks)
        .with("safe_state", g.safe_state.as_str())
        .with("kernel_levels", g.kernel_levels.iter().map(|&l| l as u64).collect::<Vec<u64>>())
        .with(
            "priority_map",
            g.priority_map
                .iter()
                .map(|&(level, nvic)| Value::obj().with("level", level).with("nvic", nvic))
                .collect::<Vec<_>>(),
        )
        .with(
            "capacities",
            Value::obj()
                .with("lock_stack", g.lock_capacity)
                .with("timer_slots", g.slot_count)
                .with("event_log", g.log_capacity)
                .with("derived", true),
        )
        .with(
            "layout",
            Value::obj()
                .with("kernel_flash", region(&g.plan.kernel_flash))
                .with("shared_code", region(&g.plan.shared_code))
                .with("kernel_ram", region(&g.plan.kernel_ram))
                .with("derived", true),
        )
        .with("partitions", partitions)
        .with("tasks", tasks)
        .with("resources", resources)
        .with("signals", signals)
        .with("endpoints", Vec::<Value>::new())
        .with("conversions", conversions)
}
