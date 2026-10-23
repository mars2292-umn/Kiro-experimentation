//! Per-region worst-case stack bounds (R37.2) from the per-function usage,
//! the Task_Model's preemption relation, and the Design_Document's
//! assignment of Tasks and Kernel entry points to stack regions (DD-01,
//! DD-07):
//!
//! - A Partition's stack region holds its Jobs nested last-in first-out,
//!   one per Priority, so every Task of the Partition can be in progress at
//!   once (only strictly higher Priorities preempt, R7.1). Each Job costs
//!   its initial exception frame (32 bytes, HW-06), the worst-case usage of
//!   its Job wrapper, and the frame the hardware stacks on its PSP when it
//!   is preempted or enters the Kernel: 32 bytes, or 104 with an active
//!   floating-point context (R11.4, eager stacking).
//! - The Kernel runs on the MSP in Kernel RAM: the worst Kernel entry point
//!   (the vector-table handlers, naked stubs included) plus one level record
//!   per Job level in progress (the stub's 40-byte push, 104 with S16-S31)
//!   plus the hardware frame of an exception taken from the idle loop (32),
//!   against the Kernel RAM left above the Kernel's data and bss.

use std::collections::BTreeMap;

use serde_json::Value;

use crate::demangle::demangle;
use crate::graph::FunctionReport;
use crate::Violation;

/// Basic exception frame (HW-06).
pub const BASIC_FRAME: u32 = 32;
/// Extended frame with S0-S15 and FPSCR (HW-10, eager stacking).
pub const FP_FRAME: u32 = 104;
/// The dispatch stub's level record: R3-R11 and LR (10 words).
pub const LEVEL_RECORD: u32 = 40;
/// The level record with S16-S31.
pub const LEVEL_RECORD_FP: u32 = LEVEL_RECORD + 64;

#[derive(Debug, Clone)]
pub struct TaskInput {
    pub id: String,
    pub partition: String,
    pub priority: u64,
    pub fpu: bool,
    /// `::crate::rsk_jobs::name`
    pub entry: String,
}

#[derive(Debug, Clone)]
pub struct PartitionInput {
    pub id: String,
    pub stack: u64,
}

#[derive(Debug, Clone)]
pub struct ModelInputs {
    pub tasks: Vec<TaskInput>,
    pub partitions: Vec<PartitionInput>,
    pub kernel_ram_base: u64,
    pub kernel_ram_size: u64,
}

impl ModelInputs {
    pub fn parse(text: &str) -> Result<ModelInputs, String> {
        let json: Value = serde_json::from_str(text).map_err(|e| format!("Task_Model: {e}"))?;
        let get = |v: &Value, k: &str| v.get(k).cloned().ok_or_else(|| format!("Task_Model: missing `{k}`"));
        let mut tasks = Vec::new();
        for t in get(&json, "tasks")?.as_array().ok_or("Task_Model: tasks is not a list")? {
            tasks.push(TaskInput {
                id: get(t, "id")?.as_str().unwrap_or("").to_string(),
                partition: get(t, "partition")?.as_str().unwrap_or("").to_string(),
                priority: get(t, "priority")?.as_u64().unwrap_or(0),
                fpu: get(t, "fpu")?.as_bool().unwrap_or(false),
                entry: get(t, "entry")?.as_str().unwrap_or("").to_string(),
            });
        }
        let mut partitions = Vec::new();
        for p in get(&json, "partitions")?.as_array().ok_or("Task_Model: partitions is not a list")? {
            let regions = get(p, "regions")?;
            partitions.push(PartitionInput { id: get(p, "id")?.as_str().unwrap_or("").to_string(), stack: get(&regions, "stack")?.as_u64().unwrap_or(0) });
        }
        let layout = get(&json, "layout")?;
        let kram = get(&layout, "kernel_ram")?;
        let base = get(&kram, "base")?.as_u64().unwrap_or(0);
        let size_log2 = get(&kram, "size_log2")?.as_u64().unwrap_or(0);
        Ok(ModelInputs { tasks, partitions, kernel_ram_base: base, kernel_ram_size: 1u64 << size_log2 })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskUsage {
    pub id: String,
    pub entry: String,
    /// Worst-case usage of the Job wrapper and everything it calls.
    pub usage: u32,
    /// Initial frame plus the frame stacked on preemption.
    pub frame_bytes: u32,
    pub bounded: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RegionBound {
    pub name: String,
    /// `partition` or `msp`.
    pub kind: &'static str,
    pub bound: u64,
    pub available: u64,
    pub ok: bool,
    pub terms: Vec<String>,
}

/// The function whose demangled name is `path` (with or without a leading `::`).
fn find_entry<'a>(functions: &'a [FunctionReport], path: &str) -> Option<&'a FunctionReport> {
    let want = path.trim_start_matches("::");
    functions.iter().find(|f| demangle(&f.name) == want)
}

/// The Kernel entry points: the handlers of the vector table (words 1.. of
/// `.vector_table`), mapped to functions.
fn kernel_entries<'a>(elf: &rsk_elf::Elf, functions: &'a [FunctionReport]) -> Result<Vec<&'a FunctionReport>, String> {
    let section = elf.section_by_name(".vector_table").ok_or("binary: no .vector_table section")?;
    let data = elf.section_data(section).map_err(|e| format!("binary: .vector_table: {e}"))?;
    let by_address: BTreeMap<u32, &FunctionReport> = functions.iter().map(|f| (f.address, f)).collect();
    let mut out: Vec<&FunctionReport> = Vec::new();
    for word in data.chunks_exact(4).skip(1) {
        let addr = u32::from_le_bytes([word[0], word[1], word[2], word[3]]) & !1;
        if let Some(f) = by_address.get(&addr) {
            if !out.iter().any(|g| g.address == f.address) {
                out.push(f);
            }
        }
    }
    if out.is_empty() {
        return Err("binary: no vector-table handler matches a function symbol".to_string());
    }
    Ok(out)
}

/// The per-Task usages and per-region bounds, with the violations of
/// R37.2 and R37.3 and explanatory notes.
pub fn bounds(elf: &rsk_elf::Elf, model: &ModelInputs, functions: &[FunctionReport]) -> Result<(Vec<TaskUsage>, Vec<RegionBound>, Vec<Violation>, Vec<String>), String> {
    let mut tasks = Vec::new();
    let mut violations = Vec::new();
    let mut notes = Vec::new();
    for t in &model.tasks {
        match find_entry(functions, &t.entry) {
            Some(f) => {
                let frame_bytes = BASIC_FRAME + if t.fpu { FP_FRAME } else { BASIC_FRAME };
                if let Some(r) = &f.unbounded {
                    violations.push(Violation { rule: "R37.3", message: format!("Task `{}`: its Job wrapper `{}` has no stack bound: {r}", t.id, f.name) });
                }
                tasks.push(TaskUsage { id: t.id.clone(), entry: t.entry.clone(), usage: f.usage, frame_bytes, bounded: f.unbounded.is_none() });
            }
            None => violations.push(Violation { rule: "R37.1", message: format!("Task `{}`: no function named `{}` in the binary", t.id, t.entry) }),
        }
    }
    let mut regions = Vec::new();
    for p in &model.partitions {
        let mut bound = 0u64;
        let mut terms = Vec::new();
        let mut ps: Vec<&TaskUsage> = tasks.iter().filter(|u| model.tasks.iter().any(|t| t.id == u.id && t.partition == p.id)).collect();
        ps.sort_by_key(|u| model.tasks.iter().find(|t| t.id == u.id).map_or(0, |t| t.priority));
        for u in ps {
            let per_job = u.usage as u64 + u.frame_bytes as u64;
            terms.push(format!("{}: {} (wrapper) + {} (initial and stacked frames)", u.id, u.usage, u.frame_bytes));
            bound += per_job;
        }
        let ok = bound <= p.stack;
        if !ok {
            violations.push(Violation { rule: "R37.2", message: format!("Partition `{}`: stack region {} bytes, worst case {bound} bytes ({})", p.id, p.stack, terms.join("; ")) });
        }
        regions.push(RegionBound { name: p.id.clone(), kind: "partition", bound, available: p.stack, ok, terms });
    }
    // The Kernel's MSP.
    let entries = kernel_entries(elf, functions)?;
    let worst = entries.iter().max_by_key(|f| f.usage).map(|f| (f.name.clone(), f.usage)).unwrap_or_default();
    for f in &entries {
        if let Some(r) = &f.unbounded {
            violations.push(Violation { rule: "R37.3", message: format!("Kernel entry point `{}` has no stack bound: {r}", f.name) });
        }
    }
    let mut levels: Vec<u64> = model.tasks.iter().map(|t| t.priority).collect();
    levels.sort_unstable();
    levels.dedup();
    let any_fp = model.tasks.iter().any(|t| t.fpu);
    let record = if any_fp { LEVEL_RECORD_FP } else { LEVEL_RECORD } as u64;
    let msp_bound = worst.1 as u64 + levels.len() as u64 * record + BASIC_FRAME as u64;
    let mut occupied = 0u64;
    for name in [".kernel.data", ".kernel.bss"] {
        if let Some(s) = elf.section_by_name(name) {
            if (s.addr as u64) >= model.kernel_ram_base && (s.addr as u64) < model.kernel_ram_base + model.kernel_ram_size {
                occupied += s.size as u64;
            }
        }
    }
    let available = model.kernel_ram_size.saturating_sub(occupied);
    let ok = msp_bound <= available;
    let terms = vec![
        format!("worst Kernel entry point `{}`: {}", worst.0, worst.1),
        format!("{} Job levels x {record} (level records)", levels.len()),
        format!("{BASIC_FRAME} (exception frame from the idle loop)"),
        format!("Kernel RAM {} minus {occupied} of data and bss", model.kernel_ram_size),
    ];
    if !ok {
        violations.push(Violation { rule: "R37.2", message: format!("Kernel MSP: {available} bytes available, worst case {msp_bound} ({})", terms.join("; ")) });
    }
    regions.push(RegionBound { name: "kernel".to_string(), kind: "msp", bound: msp_bound, available, ok, terms });
    notes.push(format!("{} Kernel entry points from the vector table; worst `{}` at {} bytes", entries.len(), worst.0, worst.1));
    Ok((tasks, regions, violations, notes))
}
