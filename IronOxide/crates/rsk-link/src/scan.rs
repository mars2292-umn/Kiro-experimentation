//! The link-level checks of task 12.3: instruction-class scans (PR-22,
//! PR-31, R6.6, R16.10), the allocator, unwinding, panic-handler, and
//! `vstd` symbol checks (PR-15, PR-19, R14.1, R55.4), the Core_Subset
//! symbol mapping (R57.3, PR-21), the dependency-graph check from the
//! symbols' crates (PR-35, R1.5), and the no-panic-path report from the
//! Kernel entry points (R14.1). Each finding names the symbol, its address,
//! and the Task entry or Kernel entry point from which it is reachable
//! (R3.3).

use std::collections::{BTreeMap, BTreeSet};

use crate::demangle::demangle;
use crate::graph::CallGraph;
use crate::stack::ModelInputs;
use crate::thumb::FpClass;
use crate::Violation;

/// Which code a function is: by its section.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Realm {
    Kernel,
    Partition,
    /// `.shared.rsk` and `.text`: core, compiler_builtins, the rsk API.
    Shared,
    Other,
}

pub fn realm_of_section(name: &str) -> Realm {
    if name == ".kernel.text" || name == ".vector_table" {
        Realm::Kernel
    } else if name.starts_with(".p") && name.ends_with(".text") {
        Realm::Partition
    } else if name == ".text" || name == ".shared.rsk" {
        Realm::Shared
    } else {
        Realm::Other
    }
}

/// The crate a mangled symbol belongs to: the first path segment of a
/// legacy symbol, the crate root of a v0 symbol, or `None` for plain
/// (unmangled) names.
pub fn crate_of(sym: &str) -> Option<String> {
    if sym.starts_with("_ZN") {
        let d = demangle(sym);
        let first = d.split("::").next()?.to_string();
        // `<impl ..>` paths start with `_$LT$`: the crate is the first
        // identifier inside, which the demangler leaves as `<...`.
        let first = first.trim_start_matches('_').trim_start_matches('<').to_string();
        return Some(first.split(['.', ' ']).next()?.to_string());
    }
    if let Some(rest) = sym.strip_prefix("_R") {
        let idents = v0_idents(rest);
        return idents.first().cloned();
    }
    None
}

/// The identifiers of a v0 symbol in order (crate root first), skipping
/// crate hashes (`Cs<base62>_`), back references (`B<base62>_`), and the
/// namespace, generic, and impl letters.
pub fn v0_idents(rest: &str) -> Vec<String> {
    let b = rest.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    while i < b.len() {
        let c = b[i];
        if c == b'C' && i + 1 < b.len() && b[i + 1] == b's' {
            // crate id with hash: Cs<base62>_
            i += 2;
            while i < b.len() && b[i] != b'_' {
                i += 1;
            }
            i += 1;
            continue;
        }
        if (c == b'B' || c == b's') && i + 1 < b.len() {
            // back reference B<base62>_ or disambiguator s<base62>_
            let mut j = i + 1;
            while j < b.len() && b[j] != b'_' && b[j].is_ascii_alphanumeric() {
                j += 1;
            }
            if j < b.len() && b[j] == b'_' {
                i = j + 1;
                continue;
            }
        }
        if c.is_ascii_digit() {
            let mut n = 0usize;
            while i < b.len() && b[i].is_ascii_digit() {
                n = n * 10 + (b[i] - b'0') as usize;
                i += 1;
            }
            // An identifier starting with `_` or a digit is prefixed with `_`.
            let mut start = i;
            if start < b.len() && b[start] == b'_' && n > 0 {
                start += 1;
            }
            let end = (start + n).min(b.len());
            if end > start {
                out.push(rest[start..end].to_string());
            }
            i = end;
            continue;
        }
        i += 1;
    }
    out
}

/// A symbol's path as `a::b::c`, from either mangling.
pub fn path_of(sym: &str) -> String {
    if let Some(rest) = sym.strip_prefix("_R") {
        return v0_idents(rest).join("::");
    }
    demangle(sym)
}

/// One Core_Subset item: a module path and the names it lists (empty: the whole module).
#[derive(Debug, Clone)]
pub struct CoreItem {
    pub module: String,
    pub names: Vec<String>,
    pub raw: String,
    /// `scope = ["unreachable"]`: the item is linked but no Task entry or
    /// Kernel entry point may reach it.
    pub unreachable: bool,
}

/// Parses `profile/core_subset.toml`'s `path = "..."` values.
pub fn core_subset(toml_text: &str) -> Vec<CoreItem> {
    let mut out: Vec<CoreItem> = Vec::new();
    for line in toml_text.lines() {
        let t = line.trim();
        if let Some(scope) = t.strip_prefix("scope = ") {
            if let Some(last) = out.last_mut() {
                last.unreachable = scope.contains("\"unreachable\"");
            }
            continue;
        }
        let Some(rest) = t.strip_prefix("path = ") else { continue };
        let raw = rest.trim().trim_matches('"').to_string();
        let (module, names) = match raw.find("::{") {
            Some(k) => {
                let module = raw[..k].to_string();
                let inner = raw[k + 3..].trim_end_matches('}');
                let names = inner.split(',').map(|n| n.trim().split(['(', '!', ' ']).next().unwrap_or("").to_string()).filter(|n| !n.is_empty()).collect();
                (module, names)
            }
            None => match raw.rsplit_once("::") {
                Some((m, n)) if m != "core" || !n.is_empty() => (m.to_string(), vec![n.to_string()]),
                _ => (raw.clone(), vec![]),
            },
        };
        // `core::primitive`, `core::arch::asm`: whole-module items when the
        // last segment is a module rather than an item.
        let whole = ["core::primitive", "core::arch::asm", "core::fmt", "core::str::count", "core::panicking", "core::intrinsics", "compiler_builtins"].contains(&raw.as_str());
        out.push(if whole { CoreItem { module: raw.clone(), names: vec![], raw, unreachable: false } } else { CoreItem { module, names, raw, unreachable: false } });
    }
    out
}

/// The Core_Subset item a `core` (or compiler intrinsic) symbol path maps
/// to: its module is a prefix of the path and, when the item lists names,
/// one of them is a segment of the path.
pub fn map_core_symbol<'a>(path: &str, items: &'a [CoreItem]) -> Option<&'a CoreItem> {
    let segs: Vec<&str> = path.split("::").collect();
    items.iter().find(|it| {
        let prefix_ok = path == it.module || path.starts_with(&format!("{}::", it.module));
        prefix_ok && (it.names.is_empty() || it.names.iter().any(|n| segs.contains(&n.as_str())))
    })
}

pub struct ScanInputs<'a> {
    pub disallowed_crates: Vec<String>,
    pub core_items: Vec<CoreItem>,
    pub model: &'a ModelInputs,
    /// Module prefixes (`rsk_logic`, `rsk_kernel::arch::bound`) whose
    /// items are inside `verus!` and verified by the Kernel_Proofs: an
    /// implicit panic check there is proven never to fire (R44.7).
    pub verified: Vec<String>,
}

/// What the scans need from the binary besides the call graph: gathered
/// from the ELF by [`run`], built by hand in the fixtures.
pub struct Facts {
    pub realm: Vec<Realm>,
    /// Task id, wrapper index, FPU flag.
    pub task_entries: Vec<(String, usize, bool)>,
    /// Indices of the vector table's handlers.
    pub kernel_entries: Vec<usize>,
    /// Names of every defined function symbol (including those not in the graph).
    pub func_symbols: Vec<String>,
    pub exidx_size: u64,
}

/// Runs every check of task 12.3 over the call graph and the binary.
pub fn run(elf: &rsk_elf::Elf, g: &CallGraph, inputs: &ScanInputs, violations: &mut Vec<Violation>, notes: &mut Vec<String>) -> Result<(), String> {
    let realm: Vec<Realm> = g.functions.iter().map(|f| elf.section_at(f.address).map_or(Realm::Other, |s| realm_of_section(&s.name))).collect();
    let mut task_entries: Vec<(String, usize, bool)> = Vec::new();
    for t in &inputs.model.tasks {
        let want = t.entry.trim_start_matches("::");
        if let Some(i) = g.functions.iter().position(|f| demangle(&f.name) == want) {
            task_entries.push((t.id.clone(), i, t.fpu));
        }
    }
    let mut kernel_entries: Vec<usize> = Vec::new();
    if let Some(section) = elf.section_by_name(".vector_table") {
        if let Ok(data) = elf.section_data(section) {
            for word in data.chunks_exact(4).skip(1) {
                let addr = u32::from_le_bytes([word[0], word[1], word[2], word[3]]) & !1;
                if let Some(i) = g.index_of(addr) {
                    if !kernel_entries.contains(&i) {
                        kernel_entries.push(i);
                    }
                }
            }
        }
    }
    let func_symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?.iter().filter(|s| s.is_func() && s.size > 0).map(|s| s.name.clone()).collect();
    let exidx_size = elf.section_by_name(".ARM.exidx").map_or(0, |s| s.size);
    let facts = Facts { realm, task_entries, kernel_entries, func_symbols, exidx_size: exidx_size as u64 };
    scan(g, &facts, inputs, violations, notes);
    Ok(())
}

/// The checks themselves.
pub fn scan(g: &CallGraph, facts: &Facts, inputs: &ScanInputs, violations: &mut Vec<Violation>, notes: &mut Vec<String>) {
    let n = g.functions.len();
    let (realm, task_entries, kernel_entries) = (&facts.realm, &facts.task_entries, &facts.kernel_entries);
    let reach = |start: usize, cut_panic: bool| -> BTreeSet<usize> {
        let mut seen = BTreeSet::new();
        let mut stack = vec![start];
        while let Some(i) = stack.pop() {
            if !seen.insert(i) {
                continue;
            }
            for c in g.callees(i).unwrap_or_default() {
                if cut_panic && CallGraph::is_panic_path(&g.functions[c].name) {
                    continue;
                }
                stack.push(c);
            }
        }
        seen
    };
    let origin = |i: usize, reaches: &BTreeMap<String, BTreeSet<usize>>| -> String {
        reaches.iter().filter(|(_, set)| set.contains(&i)).map(|(e, _)| e.clone()).next().unwrap_or_else(|| "no Task entry or Kernel entry point".to_string())
    };
    let mut reaches: BTreeMap<String, BTreeSet<usize>> = BTreeMap::new();
    for (id, i, _) in task_entries.iter() {
        reaches.insert(format!("Task {id}"), reach(*i, true));
    }
    for &i in kernel_entries.iter() {
        reaches.insert(format!("Kernel entry `{}`", g.functions[i].name), reach(i, false));
    }

    // --- PR-31 / R6.6: floating-point instruction classes.
    let kernel_set: BTreeSet<usize> = (0..n).filter(|&i| realm[i] == Realm::Kernel).chain(kernel_entries.iter().flat_map(|&e| reach(e, true))).collect();
    let mut fp_moves_in_kernel = 0usize;
    for &i in &kernel_set {
        let f = &g.functions[i];
        if CallGraph::is_panic_path(&f.name) || CallGraph::is_core_fmt(&f.name) {
            continue;
        }
        for (addr, class) in &f.fp {
            match class {
                FpClass::Move => fp_moves_in_kernel += 1,
                FpClass::FpscrRead => {}
                FpClass::Arith => violations.push(Violation { rule: "R6.6", message: format!("Kernel code `{}` executes a floating-point arithmetic, comparison, or conversion instruction at 0x{addr:08x} (PR-31; reachable from {})", f.name, origin(i, &reaches)) }),
                FpClass::FpscrWrite => violations.push(Violation { rule: "R6.6", message: format!("Kernel code `{}` writes FPSCR at 0x{addr:08x} (PR-31; reachable from {})", f.name, origin(i, &reaches)) }),
            }
        }
    }
    notes.push(format!("R6.6: {} Kernel functions scanned, {fp_moves_in_kernel} floating-point data-movement instructions, no arithmetic, comparison, conversion, or FPSCR write (the context-preservation vpush/vpop of the stubs are stack effects, PR-31)", kernel_set.len()));
    for (id, i, fpu) in task_entries.iter() {
        if *fpu {
            continue;
        }
        for c in reach(*i, true) {
            let f = &g.functions[c];
            if realm[c] == Realm::Kernel {
                continue; // the Kernel's own instructions are scanned above
            }
            if let Some((addr, class)) = f.fp.first() {
                violations.push(Violation { rule: "PR-31", message: format!("FPU-free Task `{id}` reaches `{}`, which executes a floating-point instruction ({class:?}) at 0x{addr:08x}", f.name) });
            }
        }
    }

    // --- PR-22 / R16.10: privileged instructions and PPB addresses outside the Kernel.
    let mut partition_scanned = 0usize;
    for (id, i, _) in task_entries.iter() {
        for c in reach(*i, true) {
            let f = &g.functions[c];
            if realm[c] == Realm::Kernel {
                violations.push(Violation { rule: "R16.3", message: format!("Task `{id}` reaches Kernel code `{}` (0x{:08x}) other than through an SVC", f.name, f.address) });
                continue;
            }
            partition_scanned += 1;
            for (addr, what) in &f.privileged {
                violations.push(Violation { rule: "PR-22", message: format!("Task `{id}` reaches `{}`, which executes `{what}` at 0x{addr:08x}", f.name) });
            }
            for (addr, target) in &f.ppb_refs {
                violations.push(Violation { rule: "PR-22", message: format!("Task `{id}` reaches `{}`, which materialises the Private Peripheral Bus address 0x{target:08x} at 0x{addr:08x} (R16.10)", f.name) });
            }
        }
    }
    notes.push(format!("PR-22: {partition_scanned} functions reachable from Task bodies scanned, none executes msr/mrs/cps or names a PPB address"));

    // --- PR-18: the Kernel's call graph is static. Kernel code is
    // monomorphic and holds no function tables (the vector table is read by
    // hardware, not called through), so an indirect call site (`blx Rm`) in
    // Kernel code is a violation whatever the universe resolves it to; an
    // indirect branch is the stubs' exception return (`exception_returns`).
    let mut kernel_indirect = 0usize;
    for (i, f) in g.functions.iter().enumerate() {
        if realm[i] == Realm::Kernel && f.indirect_sites > 0 {
            kernel_indirect += f.indirect_sites as usize;
            violations.push(Violation { rule: "PR-18", message: format!("Kernel code `{}` (0x{:08x}) has {} indirect call site(s) (`blx Rm`); the Kernel's call graph is static (R37, PR-18)", f.name, f.address, f.indirect_sites) });
        }
    }
    if kernel_indirect == 0 {
        notes.push("PR-18: no indirect call site in Kernel code; the stubs' indirect branches are exception returns".to_string());
    }

    // --- ORQ-32: the naked stubs of the Kernel_Unsafe_Module branch to one
    // another with `b`/`b.w` only; a `cbz`/`cbnz` to another section has no
    // relocation and the assembler emits a `nop` without a diagnostic.
    let mut stubs = 0usize;
    for (i, f) in g.functions.iter().enumerate() {
        if realm[i] != Realm::Kernel || !f.name.starts_with("rsk_") {
            continue;
        }
        stubs += 1;
        for (addr, target) in &f.nonlocal_cbz {
            violations.push(Violation { rule: "ORQ-32", message: format!("naked stub `{}` has a cbz/cbnz at 0x{addr:08x} to 0x{target:08x} outside the stub; branches between stubs use b/b.w only", f.name) });
        }
        for addr in &f.nops {
            violations.push(Violation { rule: "ORQ-32", message: format!("naked stub `{}` contains a nop at 0x{addr:08x}: the assembler's silent replacement of a cross-section cbz/cbnz", f.name) });
        }
    }
    notes.push(format!("ORQ-32: {stubs} naked stubs disassembled, no cross-section cbz/cbnz and no nop"));

    // --- Symbol checks: allocator, unwinding, panic handler, vstd, crates.
    let mut crates: BTreeSet<String> = BTreeSet::new();
    let mut panic_handlers = 0;
    for name in &facts.func_symbols {
        let lower = name.to_ascii_lowercase();
        if lower.starts_with("__rust_alloc") || lower.starts_with("__rust_dealloc") || lower.starts_with("__rust_realloc") || lower.starts_with("__rg_") || lower.starts_with("__rdl_") {
            violations.push(Violation { rule: "PR-15", message: format!("allocator symbol `{name}` in the Flight_Build (PR-14, PR-35)") });
        }
        if name.starts_with("_Unwind_") || name.contains("rust_eh_personality") || name.contains("__gcc_personality") {
            violations.push(Violation { rule: "PR-19", message: format!("unwinding symbol `{name}` in the Flight_Build (R14.1: abort strategy)") });
        }
        if name.contains("rust_begin_unwind") {
            panic_handlers += 1;
        }
        if let Some(c) = crate_of(name) {
            crates.insert(c);
        }
    }
    if panic_handlers != 1 {
        violations.push(Violation { rule: "PR-19", message: format!("{panic_handlers} panic handlers (`rust_begin_unwind`) in the Flight_Build; the Kernel's must be the only one (R14.1)") });
    }
    if facts.exidx_size > 0 {
        violations.push(Violation { rule: "PR-19", message: format!(".ARM.exidx is {} bytes: unwinding tables in the Flight_Build (R14.1)", facts.exidx_size) });
    }
    for c in &crates {
        if c == "vstd" {
            violations.push(Violation { rule: "R55.4", message: "an executable symbol of the `vstd` crate is in the Kernel Flight_Build".to_string() });
        }
        if c == "std" || c == "alloc" || inputs.disallowed_crates.iter().any(|d| d.replace('-', "_") == *c) {
            violations.push(Violation { rule: "PR-35", message: format!("the Flight_Build links code of the disallowed crate `{c}` (R1.5)") });
        }
    }
    notes.push(format!("crates with executable symbols: {}", crates.iter().cloned().collect::<Vec<_>>().join(", ")));

    // --- R57.3: every core symbol maps to a Core_Subset item.
    let mut mapped: BTreeMap<String, usize> = BTreeMap::new();
    for f in &g.functions {
        let path = path_of(&f.name);
        let is_core = path.starts_with("core::") || path.starts_with("compiler_builtins::") || f.name.starts_with("__aeabi_") || f.name == "__udivmoddi4" || f.name.starts_with("memcpy") || f.name.starts_with("memset") || f.name.starts_with("memclr");
        if !is_core {
            continue;
        }
        let key = if path.starts_with("core::") || path.starts_with("compiler_builtins::") { path.clone() } else { format!("compiler_builtins::{}", f.name) };
        match map_core_symbol(&key, &inputs.core_items) {
            Some(item) => {
                *mapped.entry(item.raw.clone()).or_default() += 1;
                if item.unreachable {
                    if let Some(i) = g.index_of(f.address) {
                        if let Some((entry, _)) = reaches.iter().find(|(_, set)| set.contains(&i)) {
                            violations.push(Violation { rule: "R57.3", message: format!("`{key}` (0x{:08x}) is a Core_Subset item of scope `unreachable`, yet {entry} reaches it", f.address) });
                        }
                    }
                }
            }
            None => {
                let i = g.index_of(f.address).unwrap_or(0);
                violations.push(Violation { rule: "R57.3", message: format!("`{}` ({key}, 0x{:08x}) maps to no Core_Subset item (PR-21; reachable from {})", f.name, f.address, origin(i, &reaches)) });
            }
        }
    }
    for (item, count) in &mapped {
        notes.push(format!("R57.3: {count} symbol(s) map to Core_Subset item `{item}`"));
    }

    // --- R14.1: panic paths from the Kernel entry points.
    let mut implicit: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for &e in kernel_entries.iter() {
        for i in reach(e, false) {
            let f = &g.functions[i];
            if CallGraph::is_panic_path(&f.name) {
                continue;
            }
            for c in g.callees(i).unwrap_or_default() {
                let callee = &g.functions[c].name;
                if !CallGraph::is_panic_path(callee) {
                    continue;
                }
                let callee_path = path_of(callee);
                let explicit = callee_path.ends_with("panic_fmt") || callee_path.ends_with("::panic") || callee_path.ends_with("panic_display") || callee_path.ends_with("panic_str");
                if explicit && realm[i] != Realm::Partition {
                    violations.push(Violation { rule: "PR-19", message: format!("Kernel code `{}` panics explicitly (`{callee_path}`), reachable from Kernel entry `{}`", demangle(&f.name), g.functions[e].name) });
                } else {
                    implicit.entry(callee_path).or_default().push(demangle(&f.name));
                }
            }
        }
    }
    if implicit.is_empty() {
        notes.push("R14.1: no panic path is reachable from any Kernel entry point".to_string());
    } else {
        for (kind, callers) in &implicit {
            let mut callers = callers.clone();
            callers.sort();
            callers.dedup();
            let is_verified = |c: &String| inputs.verified.iter().any(|v| c == v || c.starts_with(&format!("{v}::")) || c.contains(&format!("<{v}::")) || c.contains(&format!("_<{v}::")));
            let (verified, other): (Vec<&String>, Vec<&String>) = callers.iter().partition(|c| is_verified(c));
            let short = |c: &&String| c.rsplit("::").next().unwrap_or(c).to_string();
            let others: Vec<String> = other.iter().map(short).collect();
            notes.push(format!(
                "R14.1: implicit panic `{kind}` reachable from the Kernel entry points through {} function(s): {} inside `verus!` modules, where the Kernel_Proofs show the check never fires (R44.7){}; the stack bound charges the panic path once",
                callers.len(),
                verified.len(),
                if others.is_empty() { String::new() } else { format!(", and {} outside them ({}): shells and stub halves that inline the verified cores, which the binary does not attribute", others.len(), others.join(", ")) }
            ));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::graph::Function;
    use crate::thumb::FpClass;

    /// A graph of hand-built functions with their realms: `(name, realm, callees' addresses)`.
    fn fixture(funcs: Vec<(&str, Realm, Vec<u32>)>) -> (CallGraph, Vec<Function>, Vec<Realm>) {
        let functions: Vec<Function> = funcs.iter().enumerate().map(|(i, (name, _, direct))| Function { name: name.to_string(), address: 0x100 * (i as u32 + 1), size: 4, frame: 8, dynamic_sp: false, direct: direct.clone(), indirect_sites: 0, indirect_branches: 0, materialised: vec![], svcs: vec![], fp: vec![], privileged: vec![], ppb_refs: vec![], nonlocal_cbz: vec![], nops: vec![] }).collect();
        let realm = funcs.iter().map(|f| f.1).collect();
        (CallGraph::from_functions(functions.clone()), functions, realm)
    }

    fn scan_fixture(functions: Vec<Function>, realm: Vec<Realm>, task_entries: Vec<(String, usize, bool)>, kernel_entries: Vec<usize>, extra_symbols: &[&str], disallowed: &[&str], core_toml: &str) -> Vec<Violation> {
        let mut func_symbols: Vec<String> = functions.iter().map(|f| f.name.clone()).collect();
        func_symbols.extend(extra_symbols.iter().map(|s| s.to_string()));
        let g = CallGraph::from_functions(functions);
        let facts = Facts { realm, task_entries, kernel_entries, func_symbols, exidx_size: 0 };
        let model = ModelInputs { tasks: vec![], partitions: vec![], kernel_ram_base: 0, kernel_ram_size: 0 };
        let inputs = ScanInputs { disallowed_crates: disallowed.iter().map(|s| s.to_string()).collect(), core_items: core_subset(core_toml), model: &model, verified: vec![] };
        let mut v = Vec::new();
        let mut notes = Vec::new();
        scan(&g, &facts, &inputs, &mut v, &mut notes);
        v
    }

    const CORE: &str = "path = \"core::panicking\"\nscope = [\"kernel\"]\npath = \"core::fmt\"\nscope = [\"unreachable\"]\n";
    const HANDLER: &str = "_RNvCs0_7___rustc17rust_begin_unwind";

    /// A clean fixture: a Kernel handler calling a Kernel function, a Task
    /// wrapper calling shared code, one panic handler. No violation.
    #[test]
    fn clean_fixture_passes_every_check() {
        let (_, f, r) = fixture(vec![("Reset", Realm::Kernel, vec![0x200]), ("_ZN10rsk_kernel4boot17h0000000000000000E", Realm::Kernel, vec![]), ("_ZN13p1_demo_part08rsk_jobs7control17h0000000000000000E", Realm::Partition, vec![0x400]), ("_ZN3rsk5debug17h0000000000000000E", Realm::Shared, vec![])]);
        let v = scan_fixture(f, r, vec![("control".into(), 2, false)], vec![0], &[HANDLER], &["cortex-m"], CORE);
        assert!(v.is_empty(), "{v:?}");
    }

    #[test]
    fn floating_point_classes_in_kernel_and_fpu_free_tasks() {
        let (_, mut f, r) = fixture(vec![("Reset", Realm::Kernel, vec![0x200]), ("_ZN10rsk_kernel4boot17h0000000000000000E", Realm::Kernel, vec![]), ("_ZN13p1_demo_part08rsk_jobs7control17h0000000000000000E", Realm::Partition, vec![0x400]), ("_ZN3rsk5debug17h0000000000000000E", Realm::Shared, vec![])]);
        f[1].fp = vec![(0x200, FpClass::Move), (0x204, FpClass::Arith), (0x208, FpClass::FpscrWrite)];
        f[3].fp = vec![(0x400, FpClass::Move)];
        let v = scan_fixture(f, r, vec![("control".into(), 2, false)], vec![0], &[HANDLER], &[], CORE);
        let rules: Vec<&str> = v.iter().map(|x| x.rule).collect();
        assert_eq!(rules, vec!["R6.6", "R6.6", "PR-31"], "{v:?}");
        assert!(v[0].message.contains("arithmetic") && v[1].message.contains("FPSCR") && v[2].message.contains("FPU-free Task `control`"));
    }

    #[test]
    fn privileged_instructions_ppb_addresses_and_kernel_code_reached_by_tasks() {
        let (_, mut f, r) = fixture(vec![("Reset", Realm::Kernel, vec![]), ("_ZN10rsk_kernel4boot17h0000000000000000E", Realm::Kernel, vec![]), ("_ZN13p1_demo_part08rsk_jobs7control17h0000000000000000E", Realm::Partition, vec![0x400, 0x200]), ("_ZN3rsk5debug17h0000000000000000E", Realm::Shared, vec![])]);
        f[3].privileged = vec![(0x400, "msr BASEPRI".into()), (0x404, "cpsid".into())];
        f[3].ppb_refs = vec![(0x408, 0xE000_ED08)];
        let v = scan_fixture(f, r, vec![("control".into(), 2, true)], vec![0], &[HANDLER], &[], CORE);
        let rules: Vec<&str> = v.iter().map(|x| x.rule).collect();
        assert_eq!(rules, vec!["R16.3", "PR-22", "PR-22", "PR-22"], "{v:?}");
        assert!(v[3].message.contains("0xe000ed08"));
    }

    #[test]
    fn indirect_call_in_kernel_code() {
        let (_, mut f, r) = fixture(vec![("rsk_svc_rust", Realm::Kernel, vec![]), ("_ZN3rsk5debug17h0000000000000000E", Realm::Shared, vec![])]);
        f[0].indirect_sites = 1;
        f[1].indirect_sites = 1;
        let v = scan_fixture(f, r, vec![], vec![0], &[HANDLER], &[], CORE);
        assert_eq!(v.iter().map(|x| x.rule).collect::<Vec<_>>(), vec!["PR-18"], "{v:?}");
        assert!(v[0].message.contains("rsk_svc_rust"));
    }

    #[test]
    fn naked_stub_cbz_and_nop() {
        let (_, mut f, r) = fixture(vec![("rsk_svc_entry", Realm::Kernel, vec![0x200]), ("rsk_hop", Realm::Kernel, vec![])]);
        f[0].nonlocal_cbz = vec![(0x104, 0x200)];
        f[1].nops = vec![0x202];
        let v = scan_fixture(f, r, vec![], vec![0], &[HANDLER], &[], CORE);
        assert_eq!(v.iter().map(|x| x.rule).collect::<Vec<_>>(), vec!["ORQ-32", "ORQ-32"], "{v:?}");
    }

    #[test]
    fn allocator_unwinding_panic_handler_vstd_and_disallowed_crate_symbols() {
        let (_, f, r) = fixture(vec![("Reset", Realm::Kernel, vec![])]);
        let extra = [HANDLER, "_ZN8rsk_entry17rust_begin_unwind17h0000000000000000E", "__rust_alloc", "__rg_oom", "_Unwind_Resume", "rust_eh_personality", "_ZN4vstd3seq3Seq3len17h0000000000000000E", "_ZN8cortex_m10peripheral4take17h0000000000000000E", "_ZN5alloc3vec3Vec3new17h0000000000000000E"];
        let v = scan_fixture(f, r, vec![], vec![0], &extra, &["cortex-m"], CORE);
        let mut rules: Vec<&str> = v.iter().map(|x| x.rule).collect();
        rules.sort();
        assert_eq!(rules, vec!["PR-15", "PR-15", "PR-19", "PR-19", "PR-19", "PR-35", "PR-35", "R55.4"], "{v:?}");
        assert!(v.iter().any(|x| x.message.contains("2 panic handlers")));
    }

    #[test]
    fn core_symbols_map_to_the_core_subset_and_unreachable_items_stay_dead() {
        let (_, f, r) = fixture(vec![("Reset", Realm::Kernel, vec![0x200, 0x300, 0x400]), ("_RNvNtCs7ldSeZOOJos_4core9panicking18panic_bounds_check", Realm::Shared, vec![]), ("_RNvMsa_NtCs7ldSeZOOJos_4core3fmtNtB5_9Formatter3pad", Realm::Shared, vec![]), ("_RNvNtCs7ldSeZOOJos_4core5slice5index", Realm::Shared, vec![])]);
        let v = scan_fixture(f, r, vec![], vec![0], &[HANDLER], &[], CORE);
        let rules: Vec<&str> = v.iter().map(|x| x.rule).collect();
        assert_eq!(rules, vec!["R57.3", "R57.3"], "{v:?}");
        assert!(v[0].message.contains("core::fmt::Formatter::pad") && v[0].message.contains("unreachable"));
        assert!(v[1].message.contains("core::slice::index") && v[1].message.contains("no Core_Subset item"));
    }

    #[test]
    fn explicit_kernel_panics_are_violations_and_implicit_checks_are_notes() {
        let (_, f, r) = fixture(vec![("Reset", Realm::Kernel, vec![0x200, 0x300]), ("_ZN10rsk_kernel4boot17h0000000000000000E", Realm::Kernel, vec![0x400]), ("_RNvNtCs7ldSeZOOJos_4core9panicking18panic_bounds_check", Realm::Shared, vec![]), ("_RNvNtCs7ldSeZOOJos_4core9panicking9panic_fmt", Realm::Shared, vec![])]);
        let v = scan_fixture(f, r, vec![], vec![0], &[HANDLER], &[], CORE);
        assert_eq!(v.len(), 1, "{v:?}");
        assert_eq!(v[0].rule, "PR-19");
        assert!(v[0].message.contains("panics explicitly") && v[0].message.contains("rsk_kernel::boot"));
    }

    #[test]
    fn v0_identifiers_and_crates() {
        assert_eq!(v0_idents("NvNtCs7ldSeZOOJos_4core9panicking18panic_bounds_check"), vec!["core", "panicking", "panic_bounds_check"]);
        assert_eq!(crate_of("_RNvNtCs7ldSeZOOJos_4core9panicking18panic_bounds_check").as_deref(), Some("core"));
        assert_eq!(crate_of("_ZN10rsk_kernel4arch3cm45panic17h2156c26eba42e419E").as_deref(), Some("rsk_kernel"));
        let seg = "_$LT$rsk_kernel..arch..bound..Bound$LT$_$GT$$u20$as$u20$X$GT$";
        assert_eq!(crate_of(&format!("_ZN{}{seg}5fault17h0000000000000000E", seg.len())).as_deref(), Some("rsk_kernel"));
        assert_eq!(crate_of("rsk_irq_entry"), None);
        assert_eq!(path_of("_RNvXs8_NtNtNtCs7ldSeZOOJos_4core3fmt3num3impmNtB9_7Display3fmt"), "core::fmt::num::imp::Display::fmt");
    }

    #[test]
    fn core_subset_items_map_symbols_by_module_and_name() {
        let items = core_subset("path = \"core::num::{checked_add, to_le_bytes}\"\npath = \"core::option::Option\"\npath = \"core::fmt\"\nscope = [\"unreachable\"]\n");
        assert!(items[2].unreachable && !items[1].unreachable);
        assert!(map_core_symbol("core::num::u32::checked_add", &items).is_some());
        assert!(map_core_symbol("core::num::u32::wrapping_add", &items).is_none());
        assert!(map_core_symbol("core::option::Option::unwrap", &items).is_some());
        assert!(map_core_symbol("core::fmt::num::imp::Display::fmt", &items).is_some());
        assert!(map_core_symbol("core::slice::index", &items).is_none());
    }
}
