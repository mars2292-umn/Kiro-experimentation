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

pub mod bound_v;
pub mod cert;
pub mod demangle;
pub mod graph;
pub mod scan;
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

/// The Profile inputs of the scans (task 12.3): the disallowed crates of
/// `profile/profile.toml` and the Core_Subset of `profile/core_subset.toml`.
#[derive(Default)]
pub struct ProfileInputs {
    pub disallowed_crates: Vec<String>,
    pub core_subset_toml: String,
    /// `--verified` module prefixes (see `scan::ScanInputs::verified`).
    pub verified: Vec<String>,
    /// The system crate's name, for the DD-11 template check (`--system-crate`).
    pub system_crate: Option<String>,
}

impl ProfileInputs {
    pub fn parse(profile_toml: &str, core_subset_toml: &str) -> ProfileInputs {
        let mut disallowed = Vec::new();
        let mut in_table = false;
        for line in profile_toml.lines() {
            let t = line.trim();
            if t.starts_with('[') {
                in_table = t == "[disallowed_crates]";
                continue;
            }
            if in_table {
                if let Some((k, _)) = t.split_once('=') {
                    let k = k.trim();
                    if !k.is_empty() && !k.starts_with('#') {
                        disallowed.push(k.to_string());
                    }
                }
            }
        }
        ProfileInputs { disallowed_crates: disallowed, core_subset_toml: core_subset_toml.to_string(), verified: Vec::new(), system_crate: None }
    }
}

/// Runs the Link_Checker over the binary and the Task_Model text, with the
/// scans when the Profile inputs are given.
pub fn check_with_profile(elf_bytes: &[u8], model_text: &str, profile: Option<&ProfileInputs>) -> Result<Result_, String> {
    check_with_profile_probed(elf_bytes, model_text, profile, &[])
}

/// `check_with_profile` with declared fetch probes (`check_probed`).
pub fn check_with_profile_probed(elf_bytes: &[u8], model_text: &str, profile: Option<&ProfileInputs>, fetch_probes: &[String]) -> Result<Result_, String> {
    let mut out = check_probed(elf_bytes, model_text, fetch_probes)?;
    if let Some(p) = profile {
        let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
        let model = stack::ModelInputs::parse(model_text)?;
        let mut g = graph::CallGraph::build(&elf)?;
        for path in fetch_probes {
            g.declare_fetch_probe(path)?;
        }
        let inputs = scan::ScanInputs { disallowed_crates: p.disallowed_crates.clone(), system_crate: p.system_crate.clone(), core_items: scan::core_subset(&p.core_subset_toml), model: &model, verified: p.verified.clone() };
        scan::run(&elf, &g, &inputs, &mut out.violations, &mut out.notes)?;
    } else {
        out.notes.push("no Profile given (--profile, --core-subset): the instruction-class, symbol, and Core_Subset scans of task 12.3 were not run".to_string());
    }
    Ok(out)
}

/// Runs the Link_Checker over the binary and the Task_Model text.
pub fn check(elf_bytes: &[u8], model_text: &str) -> Result<Result_, String> {
    check_probed(elf_bytes, model_text, &[])
}

/// The demangled path of the isolation suite's fetch probe (HW-09, R16.3):
/// the one function whose indirect call is an instruction fetch the MPU
/// refuses. Declared only for a HIL build with the `isolation` feature.
pub const ISOLATION_FETCH_PROBE: &str = "rsk::matrix::perform";

/// `check` with declared fetch probes (`--fetch-probe <path>`): each names
/// one function, by its demangled path, whose single indirect call is an
/// instruction fetch at an address outside the Partition's view (the
/// isolation suite's matrix walker). The call has no callee, so the
/// function is bounded by its frame and its direct calls. A declaration
/// that does not match the binary, or names Kernel code, is an error. No
/// probe is declared for a Flight_Build.
pub fn check_probed(elf_bytes: &[u8], model_text: &str, fetch_probes: &[String]) -> Result<Result_, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    if !elf.is_arm() {
        return Err("binary: not an Arm ELF".to_string());
    }
    let model = stack::ModelInputs::parse(model_text)?;
    let mut out = Result_::default();
    let mut g = graph::CallGraph::build(&elf)?;
    for path in fetch_probes {
        let i = g.declare_fetch_probe(path)?;
        let f = &g.functions[i];
        if scan::in_kernel_text(&elf, f.address) {
            return Err(format!("fetch probe `{path}` (0x{:08x}) is Kernel code: only Partition code probes a fetch", f.address));
        }
        out.notes.push(format!("fetch probe `{path}` (0x{:08x}): its one indirect call is an instruction fetch the MPU refuses (HW-09), taken as having no callee", f.address));
    }
    out.notes.push(format!("{} functions, {} call edges, {} indirect call sites", g.functions.len(), g.edge_count(), g.indirect_sites));
    // PR-18: the whole target set of an indirect call that materialises no
    // address, word by word, so that no candidate goes unseen.
    if g.embedded.is_empty() {
        out.notes.push("PR-18: no function address in the read-only data inside the Partitions' and the shared code sections".to_string());
    } else {
        let words: Vec<String> = g.embedded.iter().map(|(s, a, i)| format!("{s} 0x{a:08x} -> {}", demangle::demangle(&g.functions[*i].name))).collect();
        out.notes.push(format!("PR-18: {} function address(es) in the read-only data inside the code sections, the target set of the callers that can read each section: {}", words.len(), words.join("; ")));
    }
    if g.table_words.is_empty() {
        out.notes.push(format!("PR-18: no function address in the table sections ({}): an indirect call that materialises no function address is unresolved", graph::TABLE_SECTIONS.join(", ")));
    } else {
        let words: Vec<String> = g.table_words.iter().map(|(s, a, i)| format!("{s} 0x{a:08x} -> {}", demangle::demangle(&g.functions[*i].name))).collect();
        out.notes.push(format!("PR-18: {} function address(es) in the table sections ({}), the target set of an indirect call that materialises none: {}", words.len(), graph::TABLE_SECTIONS.join(", "), words.join("; ")));
    }
    // Fail closed (2026-11-27): an instruction of a class that can write SP,
    // in a form the decoder reads no stack-pointer change from, is a
    // violation wherever it stands, reached or not, with no exemption.
    for f in &g.functions {
        for at in &f.sp_unread {
            out.violations.push(Violation { rule: "R37.3", message: format!("`{}`: the instruction at 0x{at:08x} is of a class that can write SP, in a form the decoder reads no stack-pointer change from; its frame is not known", f.name) });
        }
    }
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
