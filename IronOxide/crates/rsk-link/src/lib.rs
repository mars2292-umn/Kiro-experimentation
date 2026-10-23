//! The Link_Checker (tasks 12.1 and 12.2): worst-case stack usage from the
//! linked binary (R37) and the call-graph rules it rests on (PR-17, PR-18).
//!
//! - `thumb`: the Thumb-2 instructions that move SP or transfer control,
//!   decoded from the function bodies; every other instruction is skipped.
//! - `graph`: the call graph over the ELF's function symbols, recursion
//!   detection (PR-17), indirect-call resolution through the addresses a
//!   function loads (PR-18), and per-function worst-case usage.
//! - `stack`: the per-region bounds of R37.2 from the Task_Model's
//!   preemption relation and the Design_Document's assignment of Tasks and
//!   Kernel entry points to stack regions, and their export (R37.4).
//!
//! Frame model: a function's frame is the sum of every SP decrement in its
//! body (pushes, `vpush`, `sub sp`, pre-indexed stores), which bounds the
//! compiler's prologue-and-epilogue discipline from above; a function that
//! writes SP from a register, is recursive, or makes an indirect call with
//! no resolvable target is unbounded and fails the build (R37.3).
#![forbid(unsafe_code)]

pub mod demangle;
pub mod graph;
pub mod stack;
pub mod thumb;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Violation {
    pub rule: &'static str,
    pub message: String,
}

#[derive(Debug, Default)]
pub struct Result_ {
    pub functions: Vec<graph::FunctionReport>,
    pub tasks: Vec<stack::TaskUsage>,
    pub regions: Vec<stack::RegionBound>,
    pub violations: Vec<Violation>,
    pub notes: Vec<String>,
}

impl Result_ {
    /// The export for the Config_Checker and the Evidence_Item (R37.4):
    /// `rsk-stack-bounds/1`.
    pub fn export(&self) -> String {
        let functions: Vec<serde_json::Value> = self
            .functions
            .iter()
            .map(|f| {
                serde_json::json!({
                    "name": f.name, "address": format!("0x{:08x}", f.address), "frame": f.frame,
                    "usage": f.usage, "calls": f.calls, "bounded": f.unbounded.is_none(), "reason": f.unbounded,
                })
            })
            .collect();
        let tasks: Vec<serde_json::Value> = self
            .tasks
            .iter()
            .map(|t| serde_json::json!({ "id": t.id, "entry": t.entry, "usage": t.usage, "frame_bytes": t.frame_bytes, "bounded": t.bounded }))
            .collect();
        let regions: Vec<serde_json::Value> = self
            .regions
            .iter()
            .map(|r| serde_json::json!({ "name": r.name, "kind": r.kind, "bound": r.bound, "available": r.available, "ok": r.ok, "terms": r.terms }))
            .collect();
        let v = serde_json::json!({
            "schema": "rsk-stack-bounds/1",
            "functions": functions,
            "tasks": tasks,
            "regions": regions,
            "violations": self.violations.iter().map(|v| serde_json::json!({ "rule": v.rule, "message": v.message })).collect::<Vec<_>>(),
        });
        serde_json::to_string_pretty(&v).unwrap_or_default() + "\n"
    }
}

/// Runs the Link_Checker over the binary and the Task_Model text.
pub fn check(elf_bytes: &[u8], model_text: &str) -> Result<Result_, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    if !elf.is_arm() {
        return Err("binary: not an Arm ELF".to_string());
    }
    let model = stack::ModelInputs::parse(model_text)?;
    let mut out = Result_::default();
    let g = graph::CallGraph::build(&elf)?;
    out.notes.push(format!("{} functions, {} call edges, {} indirect call sites", g.functions.len(), g.edge_count(), g.indirect_sites));
    let reports = g.analyse();
    let exception_returns = g.exception_returns();
    if !exception_returns.is_empty() {
        out.notes.push(format!("`bx Rm` with no resolvable target taken as an exception return in: {} (the naked stubs, UJ-020, UJ-023)", exception_returns.join(", ")));
    }
    let panic_usage = g.panic_usage(&reports);
    if panic_usage > 0 {
        out.notes.push(format!("panic path (core::panicking, rust_begin_unwind, the Kernel panic handler) charged once per entry point: {panic_usage} bytes; its removal from Kernel entry points is task 12.3"));
    }
    let mut dead_formatting = Vec::new();
    for r in &reports {
        if let Some(reason) = &r.unbounded {
            // `core::fmt` is linked for the panic message, which the Kernel
            // panic handler never formats (`_info` unused, R14.1): its
            // trait-object calls have no vtable in the image and are never
            // reached. Reported as a note; task 12.3 checks the handler.
            if graph::CallGraph::is_core_fmt(&r.name) {
                dead_formatting.push(r.name.clone());
                continue;
            }
            let rule = match reason.as_str() {
                s if s.starts_with("recursive") => "PR-17",
                s if s.starts_with("unresolved indirect") => "PR-18",
                _ => "R37.3",
            };
            out.violations.push(Violation { rule, message: format!("`{}` (0x{:08x}) has no stack bound: {reason}", r.name, r.address) });
        }
    }
    if !dead_formatting.is_empty() {
        out.notes.push(format!(
            "{} core::fmt functions with unresolved trait-object calls are linked for the panic message, which the panic handler does not format (never reached): {}",
            dead_formatting.len(),
            dead_formatting.iter().map(|n| demangle::demangle(n)).collect::<Vec<_>>().join(", ")
        ));
    }
    out.functions = reports;
    let (tasks, regions, mut vs, mut notes) = stack::bounds(&elf, &model, &out.functions)?;
    out.tasks = tasks;
    out.regions = regions;
    out.violations.append(&mut vs);
    out.notes.append(&mut notes);
    Ok(out)
}
