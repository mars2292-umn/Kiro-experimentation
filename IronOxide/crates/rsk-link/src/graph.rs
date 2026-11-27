//! The call graph over the ELF's function symbols (PR-17, PR-18) and the
//! per-function worst-case stack usage.

use std::collections::{BTreeMap, BTreeSet};

use crate::thumb::{self, Effect, FpClass};

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub address: u32,
    pub size: u32,
    /// Sum of the SP decrements of the body (the frame model).
    pub frame: u32,
    pub dynamic_sp: bool,
    /// The addresses of the instructions the decoder answered `SpUnread`
    /// for: of a class that can write SP, in a form it reads no change
    /// from. A function with one has no stack bound (2026-11-27).
    pub sp_unread: Vec<u32>,
    /// Direct call and tail-call targets (addresses).
    pub direct: Vec<u32>,
    /// Indirect call sites (`blx Rm`).
    pub indirect_sites: u32,
    /// `bx Rm` sites: indirect tail calls, or exception returns in the stubs.
    pub indirect_branches: u32,
    /// Function addresses the body materialises (literal pool, movw/movt).
    pub materialised: Vec<u32>,
    pub svcs: Vec<u8>,
    /// Floating-point instructions by class: (address, class).
    pub fp: Vec<(u32, FpClass)>,
    /// `msr`/`mrs`/`cps`: (address, description).
    pub privileged: Vec<(u32, String)>,
    /// Addresses in the Private Peripheral Bus (0xE000_0000..0xE010_0000) the body materialises.
    pub ppb_refs: Vec<(u32, u32)>,
    /// `cbz`/`cbnz` instructions whose target lies outside the function (ORQ-32), and `nop`s.
    pub nonlocal_cbz: Vec<(u32, u32)>,
    pub nops: Vec<u32>,
    /// Addresses of `bx` instructions (returns and exception returns) not
    /// immediately preceded by a `dsb`, allowing only `cpsie`/`cpsid` between
    /// (erratum 838869's workaround, DD-01, R11.6).
    pub returns_without_dsb: Vec<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FunctionReport {
    pub name: String,
    pub address: u32,
    pub frame: u32,
    /// Worst-case stack usage from entry, callees included.
    pub usage: u32,
    pub calls: Vec<String>,
    pub unbounded: Option<String>,
}

/// The special register of an `msr`/`mrs` SYSm value (B5.2.3).
pub fn special_register(sysm: u8) -> &'static str {
    match sysm {
        0..=7 => "APSR/xPSR",
        8 => "MSP",
        9 => "PSP",
        0x10 => "PRIMASK",
        0x11 => "BASEPRI",
        0x12 => "BASEPRI_MAX",
        0x13 => "FAULTMASK",
        0x14 => "CONTROL",
        _ => "unknown special register",
    }
}

/// The sections whose words may be indirect-call targets (PR-18): the
/// generated tables and the shared read-only data. A function table is
/// read-only (the Profile allows function pointers in generated tables
/// only). The initialised data (`.kernel.data`, `.data`) is state: until
/// 2026-11-18 it was scanned too, and one word of the Kernel's state that
/// happened to equal a function address bounded every unresolved indirect
/// call of the isolation build (Verification_Plan, 2026-11-18).
pub const TABLE_SECTIONS: [&str; 2] = [".rodata", ".rsk.tables"];

/// The words of `sections` (`(name, address, bytes)`) that are a function
/// address with the Thumb bit, at a 4-aligned address, as `(section,
/// address of the word, function index)`.
pub fn table_words(sections: &[(&str, u32, &[u8])], by_address: &BTreeMap<u32, usize>) -> Vec<(String, u32, usize)> {
    let mut out = Vec::new();
    for (name, addr, data) in sections {
        // Words are read at 4-aligned addresses whatever the section's start.
        let skip = ((4 - (addr % 4)) % 4) as usize;
        for (k, w) in data.get(skip..).unwrap_or(&[]).chunks_exact(4).enumerate() {
            let v = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
            if v & 1 == 1 {
                if let Some(&i) = by_address.get(&(v & !1)) {
                    out.push((name.to_string(), addr + (skip + 4 * k) as u32, i));
                }
            }
        }
    }
    out
}

/// Whether read-only data the linker script placed inside the executable
/// section `name` is read for function addresses: a Partition's code
/// section and the shared code. The Kernel's is not: no indirect call is
/// allowed in Kernel code (PR-18, the scan), and the addresses its tables
/// hold (Task entries, handlers) are entry points, not call targets.
pub fn embedded_table_section(name: &str) -> bool {
    name == ".text" || name == ".shared.rsk" || (name.starts_with(".p") && name.ends_with(".text"))
}

/// Whether a caller in section `caller` can read a table in section
/// `table`: its own section and the shared code; shared code is handed
/// tables by any Partition, so it reads every one of them.
pub fn table_readable(caller: &str, table: &str) -> bool {
    let shared = |n: &str| n == ".text" || n == ".shared.rsk";
    table == caller || shared(table) || (shared(caller) && embedded_table_section(table))
}

/// The table words of an executable section outside every function body
/// (`bodies`: `(address, size)`): read-only data placed in the code
/// section. A function's own literal pool is inside its symbol and is
/// read as that function's materialised addresses, not here.
pub fn embedded_words(name: &str, addr: u32, data: &[u8], bodies: &[(u32, u32)], by_address: &BTreeMap<u32, usize>) -> Vec<(String, u32, usize)> {
    let inside = |a: u32| bodies.iter().any(|(f, n)| a.wrapping_add(3) >= *f && a < f + n);
    table_words(&[(name, addr, data)], by_address).into_iter().filter(|w| !inside(w.1)).collect()
}

pub struct CallGraph {
    pub functions: Vec<Function>,
    /// Address -> index into `functions`.
    by_address: BTreeMap<u32, usize>,
    pub indirect_sites: u32,
    /// Functions whose address appears as a word of the read-only data
    /// (vtable slots, function tables): the finite target universe of an
    /// indirect call that materialises no function address itself (PR-18).
    pub table_targets: Vec<usize>,
    /// Where each table target was found: `(section, address of the word,
    /// function index)`, for the report (every candidate is listed).
    pub table_words: Vec<(String, u32, usize)>,
    /// Function addresses in the read-only data inside the code sections
    /// (`embedded_table_section`), as `table_words`: candidates for the
    /// callers that can read the section (`table_readable`).
    pub embedded: Vec<(String, u32, usize)>,
    /// The section of each function (empty in fixtures).
    pub section_of: Vec<String>,
    /// Declared fetch probes (`declare_fetch_probe`): functions whose one
    /// indirect call is an instruction fetch the MPU is to refuse, so it
    /// has no callee. Empty unless the caller declares one.
    pub fetch_probes: Vec<usize>,
}

impl CallGraph {
    /// Decodes every function symbol of the executable sections.
    pub fn build(elf: &rsk_elf::Elf) -> Result<CallGraph, String> {
        let symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?;
        let mut funcs: Vec<(String, u32, u32)> = symbols
            .iter()
            .filter(|s| s.is_func() && s.size > 0)
            .map(|s| (s.name.clone(), s.address(), s.size))
            .collect();
        funcs.sort_by_key(|f| f.1);
        funcs.dedup_by_key(|f| f.1);
        let func_addrs: BTreeSet<u32> = funcs.iter().map(|f| f.1).collect();
        let literal = |a: u32| -> Option<u32> {
            elf.read_at(a, 4).ok().map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let mut functions = Vec::with_capacity(funcs.len());
        let mut by_address = BTreeMap::new();
        let mut indirect_total = 0;
        for (i, (name, address, size)) in funcs.iter().enumerate() {
            let code = elf.read_at(*address, *size as usize).map_err(|e| format!("binary: `{name}`: {e}"))?;
            let insns = thumb::decode_function(&code, *address, &literal);
            let end = address + size;
            let mut f = Function {
                name: name.clone(),
                address: *address,
                size: *size,
                frame: 0,
                dynamic_sp: false,
                sp_unread: Vec::new(),
                direct: Vec::new(),
                indirect_sites: 0,
                indirect_branches: 0,
                materialised: Vec::new(),
                svcs: Vec::new(),
                fp: Vec::new(),
                privileged: Vec::new(),
                ppb_refs: Vec::new(),
                nonlocal_cbz: Vec::new(),
                nops: Vec::new(),
                returns_without_dsb: Vec::new(),
            };
            let mut partial: BTreeMap<u8, u32> = BTreeMap::new();
            // Instructions since the last `dsb`, `cps` not counted; `None` before any.
            let mut since_dsb: Option<u32> = None;
            for insn in &insns {
                match &insn.effect {
                    Effect::Dsb => since_dsb = Some(0),
                    Effect::Cps => {}
                    Effect::IndirectBranch(_) | Effect::Return => {
                        if since_dsb != Some(0) {
                            f.returns_without_dsb.push(insn.address);
                        }
                        since_dsb = None;
                    }
                    _ => since_dsb = since_dsb.map(|n| n + 1),
                }
                match &insn.effect {
                    // The frame, the dynamic-SP mark and the direct edges are
                    // taken from the verified functions after the loop (2026-11-26).
                    Effect::CompareBranch(t) => {
                        if *t < *address || *t >= end {
                            f.nonlocal_cbz.push((insn.address, *t));
                        }
                    }
                    Effect::Nop => f.nops.push(insn.address),
                    Effect::SpUnread => f.sp_unread.push(insn.address),
                    Effect::IndirectCall(_) => f.indirect_sites += 1,
                    Effect::IndirectBranch(_) => f.indirect_branches += 1,
                    Effect::Constant(reg, v) if (0xE000_0000..0xE010_0000).contains(v) => {
                        f.ppb_refs.push((insn.address, *v));
                        partial.insert(*reg, *v);
                    }
                    Effect::Constant(reg, v) => {
                        // movt completes a movw in the same register; a literal or a
                        // completed pair naming a function is a candidate target.
                        let value = if *v & 0xFFFF == 0 && *v != 0 { partial.get(reg).map_or(*v, |lo| lo | v) } else { *v };
                        partial.insert(*reg, value);
                        let thumb_addr = value & !1;
                        if value & 1 == 1 && func_addrs.contains(&thumb_addr) {
                            f.materialised.push(thumb_addr);
                        }
                    }
                    Effect::Svc(n) => f.svcs.push(*n),
                    Effect::Fp(class) => f.fp.push((insn.address, *class)),
                    Effect::Msr(sysm) => f.privileged.push((insn.address, format!("msr {}", special_register(*sysm)))),
                    Effect::Mrs(sysm) => f.privileged.push((insn.address, format!("mrs {}", special_register(*sysm)))),
                    Effect::Cps => f.privileged.push((insn.address, "cpsie/cpsid".to_string())),
                    _ => {}
                }
            }
            f.frame = crate::bound_v::frame_of(&insns);
            f.dynamic_sp = crate::bound_v::dynamic_of(&insns);
            f.direct = crate::bound_v::direct_of(&insns, *address, end);
            f.direct.sort_unstable();
            f.direct.dedup();
            f.materialised.sort_unstable();
            f.materialised.dedup();
            indirect_total += f.indirect_sites;
            by_address.insert(*address, i);
            functions.push(f);
        }
        // Function addresses stored in the read-only table sections.
        let mut sections: Vec<(&str, u32, &[u8])> = Vec::new();
        for name in TABLE_SECTIONS {
            if let Some(section) = elf.section_by_name(name) {
                if let Ok(data) = elf.section_data(section) {
                    sections.push((name, section.addr, data));
                }
            }
        }
        let table_words = table_words(&sections, &by_address);
        let mut table_targets: Vec<usize> = table_words.iter().map(|w| w.2).collect();
        table_targets.sort_unstable();
        table_targets.dedup();
        let bodies: Vec<(u32, u32)> = functions.iter().map(|f| (f.address, f.size)).collect();
        let mut embedded = Vec::new();
        for section in elf.sections.iter().filter(|x| x.is_alloc() && x.is_exec() && !x.is_nobits() && embedded_table_section(&x.name)) {
            if let Ok(data) = elf.section_data(section) {
                embedded.extend(embedded_words(&section.name, section.addr, data, &bodies, &by_address));
            }
        }
        let section_of: Vec<String> = functions.iter().map(|f| elf.section_at(f.address).map(|x| x.name.clone()).unwrap_or_default()).collect();
        Ok(CallGraph { functions, by_address, indirect_sites: indirect_total, table_targets, table_words, embedded, section_of, fetch_probes: vec![] })
    }

    /// A graph over given functions with no indirect-call universe (fixtures).
    pub fn from_functions(functions: Vec<Function>) -> CallGraph {
        let by_address = functions.iter().enumerate().map(|(i, f)| (f.address, i)).collect();
        CallGraph { functions, by_address, indirect_sites: 0, table_targets: vec![], table_words: vec![], embedded: vec![], section_of: vec![], fetch_probes: vec![] }
    }

    /// Declares the function whose demangled name is `path` a fetch probe
    /// (the isolation suite's `rsk::matrix::perform`, HW-09): its indirect
    /// call is a `blx` to an address outside the Partition's view, which
    /// the MPU refuses before any instruction there executes (IACCVIOL), so
    /// the call has no callee and adds no stack beyond the exception frame
    /// every Task stack is charged. The declaration must match the binary:
    /// exactly one function of that name, outside the Kernel's text (the
    /// caller's check), with exactly one indirect call site and no indirect
    /// branch. Anything else is an error, so a stale or widened declaration
    /// fails the check instead of hiding a call.
    pub fn declare_fetch_probe(&mut self, path: &str) -> Result<usize, String> {
        let found: Vec<usize> = (0..self.functions.len()).filter(|&i| crate::demangle::demangle(&self.functions[i].name) == path).collect();
        let [i] = found.as_slice() else {
            return Err(format!("fetch probe `{path}`: {} functions of that name in the binary (exactly one expected)", found.len()));
        };
        let f = &self.functions[*i];
        if f.indirect_sites != 1 || f.indirect_branches != 0 {
            return Err(format!("fetch probe `{path}`: {} indirect call site(s) and {} indirect branch(es) in its body (exactly one call and no branch expected)", f.indirect_sites, f.indirect_branches));
        }
        if !self.fetch_probes.contains(i) {
            self.fetch_probes.push(*i);
        }
        Ok(*i)
    }

    pub fn index_of(&self, address: u32) -> Option<usize> {
        self.by_address.get(&address).copied()
    }

    pub fn by_name(&self, name: &str) -> Option<usize> {
        self.functions.iter().position(|f| f.name == name)
    }

    /// Whether `name` is a panic path: `core::panicking::*` or the panic
    /// handler entry `rust_begin_unwind`. A call into it diverges (the
    /// handler ends in the safe state), so it is charged once per entry
    /// point and never forms part of a cycle (the "at most one panic"
    /// rule; task 12.3's no-panic-path check is to remove it altogether).
    pub fn is_panic_path(name: &str) -> bool {
        let d = crate::demangle::demangle(name);
        // Legacy (`_ZN..`) and v0 (`_R..`, `4core9panicking`) manglings.
        d.starts_with("core::panicking::") || name.contains("4core9panicking") || name.contains("rust_begin_unwind")
    }

    /// `core::fmt` in either mangling.
    pub fn is_core_fmt(name: &str) -> bool {
        crate::demangle::demangle(name).starts_with("core::fmt::") || name.contains("4core3fmt")
    }

    /// Callee indices of function `i`: direct targets that are functions,
    /// plus every materialised function address when the body has an
    /// indirect call (`blx Rm`) or indirect branch (`bx Rm`) site (PR-18
    /// resolution). Returns `None` when an indirect call has no candidate;
    /// a `bx Rm` with no candidate is taken as an exception return (the
    /// naked stubs end that way), which `exception_returns` lists.
    pub fn callees(&self, i: usize) -> Option<Vec<usize>> {
        let f = &self.functions[i];
        let mut out: Vec<usize> = f.direct.iter().filter_map(|a| self.index_of(*a)).collect();
        if self.fetch_probes.contains(&i) {
            // The one indirect call is the refused fetch: no callee.
        } else if f.indirect_sites > 0 || f.indirect_branches > 0 {
            if !f.materialised.is_empty() {
                out.extend(f.materialised.iter().filter_map(|a| self.index_of(*a)));
            } else if f.indirect_sites > 0 || self.uses_table(i) {
                // No address in the body: the call goes through a table (a
                // vtable or function table) in the data, whose function
                // addresses are the finite target set.
                let own = self.section_of.get(i).map(String::as_str).unwrap_or("");
                let embedded: Vec<usize> = self.embedded.iter().filter(|w| table_readable(own, &w.0)).map(|w| w.2).collect();
                if self.table_targets.is_empty() && embedded.is_empty() {
                    return None;
                }
                out.extend(self.table_targets.iter().copied());
                out.extend(embedded);
            }
        }
        out.sort_unstable();
        out.dedup();
        // A self edge (`bl` to the function's own entry) stays: it is direct
        // recursion, which `visit` reports (PR-17; found by conf-PR-17-lk).
        Some(out)
    }

    /// Whether `i`'s indirect branches are table calls rather than exception
    /// returns: a function that materialises any data-table address, or
    /// that is not a vector-table stub (the stubs return through `bx` with an
    /// EXC_RETURN value and materialise nothing).
    fn uses_table(&self, i: usize) -> bool {
        let f = &self.functions[i];
        !f.name.starts_with("rsk_")
    }

    /// Functions whose `bx Rm` has no resolvable target: exception returns
    /// (reviewed: the dispatch stub and the unwind, UJ-020, UJ-023).
    pub fn exception_returns(&self) -> Vec<String> {
        (0..self.functions.len())
            .filter(|&i| {
                let f = &self.functions[i];
                f.indirect_branches > 0 && f.indirect_sites == 0 && f.materialised.is_empty() && !self.uses_table(i)
            })
            .map(|i| self.functions[i].name.clone())
            .collect()
    }

    pub fn edge_count(&self) -> usize {
        (0..self.functions.len()).map(|i| self.callees(i).map_or(0, |c| c.len())).sum()
    }

    /// Per-function frame, usage, and boundedness (R37.1, R37.3). Panic
    /// edges are cut from the walk; the panic path's own worst case is
    /// then added once to every function that can reach a panic edge.
    pub fn analyse(&self) -> Vec<FunctionReport> {
        let n = self.functions.len();
        let mut usage: Vec<Option<u32>> = vec![None; n];
        let mut reason: Vec<Option<String>> = vec![None; n];
        let mut state: Vec<u8> = vec![0; n]; // 0 new, 1 on stack, 2 done
        let mut panics: Vec<bool> = vec![false; n];
        for start in 0..n {
            self.visit(start, &mut usage, &mut reason, &mut state, &mut panics, &mut Vec::new());
        }
        // The panic path: the deepest panic-path function, with panic edges
        // cut inside it as well (a second panic would be recursion, PR-17).
        let panic_usage: u32 = (0..n).filter(|&i| Self::is_panic_path(&self.functions[i].name)).map(|i| usage[i].unwrap_or(0)).max().unwrap_or(0);
        // 2026-11-22: the walk above is unverified; its result is checked
        // by the verified certificate (`cert::check`), under which every
        // chain of calls from a bounded function stays within its usage
        // (`cert::theorem_walk_within_usage`,
        // `theorem_one_panic_within_total`). A refused certificate is a
        // defect of the walk: no function keeps a bound.
        let refused = !crate::cert::check(&self.certificate(&usage, &reason, &panics), panic_usage);
        if refused {
            for r in reason.iter_mut() {
                if r.is_none() {
                    *r = Some("the stack-usage certificate was refused (a defect of the Link_Checker's walk)".to_string());
                }
            }
        }
        (0..n)
            .map(|i| {
                let f = &self.functions[i];
                let calls = self.callees(i).unwrap_or_default().iter().map(|&c| self.functions[c].name.clone()).collect();
                let own = usage[i].unwrap_or(f.frame);
                let total = if panics[i] && !Self::is_panic_path(&f.name) { own.saturating_add(panic_usage) } else { own };
                FunctionReport { name: f.name.clone(), address: f.address, frame: f.frame, usage: total, calls, unbounded: reason[i].clone() }
            })
            .collect()
    }

    /// The walk's result as the certificate's nodes: for each function its
    /// frame, the usage and the bound the walk gave it, whether it was
    /// found able to panic, and its calls split as the walk splits them
    /// (followed, or a panic edge). Written from `callees` and
    /// `is_panic_path`, apart from `visit`.
    fn certificate(&self, usage: &[Option<u32>], reason: &[Option<String>], panics: &[bool]) -> Vec<crate::cert::Node> {
        (0..self.functions.len())
            .map(|i| {
                let f = &self.functions[i];
                let outside = !Self::is_panic_path(&f.name);
                let (mut walked, mut cut) = (Vec::new(), Vec::new());
                for c in self.callees(i).unwrap_or_default() {
                    if outside && Self::is_panic_path(&self.functions[c].name) {
                        cut.push(c);
                    } else {
                        walked.push(c);
                    }
                }
                crate::cert::Node { frame: f.frame, own: usage[i].unwrap_or(f.frame), bounded: reason[i].is_none(), panics: panics[i], walked, cut }
            })
            .collect()
    }

    pub fn panic_usage(&self, reports: &[FunctionReport]) -> u32 {
        reports.iter().filter(|r| Self::is_panic_path(&r.name)).map(|r| r.usage).max().unwrap_or(0)
    }

    fn visit(&self, i: usize, usage: &mut Vec<Option<u32>>, reason: &mut Vec<Option<String>>, state: &mut Vec<u8>, panics: &mut Vec<bool>, path: &mut Vec<usize>) {
        if state[i] == 2 {
            return;
        }
        if state[i] == 1 {
            // A cycle through `i` (PR-17): every function on the cycle is unbounded.
            let pos = path.iter().position(|&p| p == i).unwrap_or(0);
            let names: Vec<String> = path[pos..].iter().map(|&p| self.functions[p].name.clone()).collect();
            for &p in &path[pos..] {
                if reason[p].is_none() {
                    reason[p] = Some(format!("recursive: {}", names.join(" -> ")));
                }
            }
            return;
        }
        state[i] = 1;
        path.push(i);
        let f = &self.functions[i];
        let mut worst = 0u32;
        let mut why: Option<String> = None;
        if f.dynamic_sp {
            why = Some("SP written from a register (dynamic stack)".to_string());
        }
        if let Some(at) = f.sp_unread.first() {
            // Fail closed: before a dynamic write, so that the form is named.
            why = Some(format!("an instruction of a class that can write SP, in a form the decoder reads no stack-pointer change from, at {at:#010x} ({} such instruction(s))", f.sp_unread.len()));
        }
        match self.callees(i) {
            None => why = why.or(Some(format!("unresolved indirect call ({} site(s), no function address materialised in the body)", f.indirect_sites))),
            Some(callees) => {
                for c in callees {
                    if c == i {
                        why = why.or(Some(format!("recursive: {} -> {}", f.name, f.name)));
                        continue;
                    }
                    if Self::is_panic_path(&self.functions[c].name) && !Self::is_panic_path(&f.name) {
                        // A panic edge: charged once per entry, not walked here.
                        panics[i] = true;
                        continue;
                    }
                    self.visit(c, usage, reason, state, panics, path);
                    if let Some(r) = &reason[c] {
                        why = why.or(Some(format!("calls unbounded `{}` ({r})", self.functions[c].name)));
                    }
                    if panics[c] {
                        panics[i] = true;
                    }
                    worst = worst.max(usage[c].unwrap_or(0));
                }
            }
        }
        path.pop();
        state[i] = 2;
        usage[i] = Some(f.frame.saturating_add(worst));
        if reason[i].is_none() {
            reason[i] = why;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn graph(funcs: Vec<(&str, u32, u32, Vec<u32>, u32, Vec<u32>, bool)>) -> CallGraph {
        let mut functions = Vec::new();
        let mut by_address = BTreeMap::new();
        for (i, (name, address, frame, direct, indirect, materialised, dynamic)) in funcs.into_iter().enumerate() {
            by_address.insert(address, i);
            functions.push(Function { name: name.into(), address, size: 4, frame, dynamic_sp: dynamic, sp_unread: vec![], direct, indirect_sites: indirect, indirect_branches: 0, materialised, svcs: vec![], fp: vec![], privileged: vec![], ppb_refs: vec![], nonlocal_cbz: vec![], nops: vec![], returns_without_dsb: vec![] });
        }
        CallGraph { functions, by_address, indirect_sites: 0, table_targets: vec![], table_words: vec![], embedded: vec![], section_of: vec![], fetch_probes: vec![] }
    }

    /// Usage is the frame plus the deepest callee; a cycle is reported as
    /// recursion naming its members (PR-17); an indirect call resolves to
    /// the materialised addresses or is unresolved (PR-18).
    #[test]
    fn usage_recursion_and_indirect_resolution() {
        let g = graph(vec![
            ("main", 0x100, 16, vec![0x200, 0x300], 0, vec![], false),
            ("a", 0x200, 8, vec![0x400], 0, vec![], false),
            ("b", 0x300, 32, vec![], 1, vec![0x400], false),
            ("leaf", 0x400, 4, vec![], 0, vec![], false),
            ("r1", 0x500, 8, vec![0x600], 0, vec![], false),
            ("r2", 0x600, 8, vec![0x500], 0, vec![], false),
            ("unres", 0x700, 8, vec![], 1, vec![], false),
            ("dyn", 0x800, 0, vec![], 0, vec![], true),
        ]);
        let r = g.analyse();
        let by = |n: &str| r.iter().find(|f| f.name == n).unwrap();
        assert_eq!(by("leaf").usage, 4);
        assert_eq!(by("a").usage, 12);
        assert_eq!(by("b").usage, 36);
        assert_eq!(by("main").usage, 16 + 36);
        assert!(by("main").unbounded.is_none());
        assert!(by("r1").unbounded.as_deref().unwrap().starts_with("recursive"));
        assert!(by("r2").unbounded.as_deref().unwrap().contains("r1"));
        assert!(by("unres").unbounded.as_deref().unwrap().starts_with("unresolved indirect"));
        assert!(by("dyn").unbounded.as_deref().unwrap().contains("dynamic"));
    }

    /// 2026-11-27, fail closed: a function with an instruction the decoder
    /// answered `SpUnread` for has no bound, and neither has its caller.
    #[test]
    fn an_unread_write_of_the_stack_pointer_leaves_no_bound() {
        let mut g = graph(vec![("main", 0x100, 16, vec![0x200], 0, vec![], false), ("odd", 0x200, 8, vec![], 0, vec![], false), ("plain", 0x300, 8, vec![], 0, vec![], false)]);
        g.functions[1].sp_unread = vec![0x204, 0x20c];
        let r = g.analyse();
        let by = |n: &str| r.iter().find(|f| f.name == n).unwrap();
        let why = by("odd").unbounded.as_deref().unwrap();
        assert!(why.contains("reads no stack-pointer change from") && why.contains("0x00000204") && why.contains("2 such"), "{why}");
        assert!(by("main").unbounded.as_deref().unwrap().contains("calls unbounded `odd`"));
        assert!(by("plain").unbounded.is_none());
    }

    /// A declared fetch probe has no callee for its one indirect call; the
    /// declaration covers that function only (another unresolved indirect
    /// call is still PR-18), and one that does not match the binary (no
    /// such function, two call sites, no call site) is refused.
    /// R37.1 (2026-11-22): the walk's result passes the verified
    /// certificate, and a usage that does not cover a callee, a bound
    /// claimed over an unbounded callee, an unmarked panic edge, or a
    /// panic charge below the panic path's usage is refused.
    #[test]
    fn the_usage_certificate_accepts_the_walk_and_refuses_an_altered_result() {
        use crate::cert::{check, Node};
        let node = |frame, own, bounded, panics, walked: &[usize], cut: &[usize]| Node { frame, own, bounded, panics, walked: walked.to_vec(), cut: cut.to_vec() };
        // 0 calls 1 and 2; 2 has a panic edge to 3 (the panic path), which calls 4.
        let good = || vec![
            node(16, 16 + 40, true, true, &[1, 2], &[]),
            node(24, 24, true, false, &[], &[]),
            node(8, 40, true, true, &[1], &[3]),
            node(32, 32 + 12, true, false, &[4], &[]),
            node(12, 12, true, false, &[], &[]),
        ];
        assert!(check(&good(), 44));
        // The usage of 0 one byte short of its frame and its deepest callee.
        let mut g = good();
        g[0].own -= 1;
        assert!(!check(&g, 44));
        // A bound claimed over a callee that has none.
        let mut g = good();
        g[1].bounded = false;
        assert!(!check(&g, 44));
        // A panic edge whose caller is not marked as able to panic.
        let mut g = good();
        g[2].panics = false;
        assert!(!check(&g, 44));
        // A caller of a function that can panic, not marked itself.
        let mut g = good();
        g[0].panics = false;
        assert!(!check(&g, 44));
        // The panic path charged less than its deepest function uses.
        assert!(!check(&good(), 43));
        // A callee index outside the graph.
        let mut g = good();
        g[4].walked.push(9);
        assert!(!check(&g, 44));
        // A function without a bound is asked nothing.
        let mut g = good();
        g[4] = node(12, 0, false, false, &[4], &[]);
        g[3].bounded = false;
        assert!(check(&g, 44));
    }

    #[test]
    fn a_fetch_probe_is_one_named_function_with_one_indirect_call() {
        let funcs = || {
            vec![
                ("_ZN3rsk6matrix7perform17h0123456789abcdefE", 0x100, 16, vec![0x400], 1, vec![], false),
                ("_ZN3app5other17h0123456789abcdefE", 0x200, 8, vec![], 1, vec![], false),
                ("_ZN3app3two17h0123456789abcdefE", 0x300, 8, vec![], 2, vec![], false),
                ("leaf", 0x400, 4, vec![], 0, vec![], false),
            ]
        };
        let undeclared = graph(funcs()).analyse();
        assert!(undeclared[0].unbounded.as_deref().unwrap().starts_with("unresolved indirect"));
        let mut g = graph(funcs());
        assert_eq!(g.declare_fetch_probe("rsk::matrix::perform"), Ok(0));
        let r = g.analyse();
        assert!(r[0].unbounded.is_none());
        assert_eq!(r[0].usage, 20);
        assert!(r[1].unbounded.as_deref().unwrap().starts_with("unresolved indirect"));
        assert!(r[2].unbounded.as_deref().unwrap().starts_with("unresolved indirect"));
        assert!(g.declare_fetch_probe("rsk::matrix::absent").unwrap_err().contains("0 functions"));
        assert!(g.declare_fetch_probe("app::two").unwrap_err().contains("2 indirect call site(s)"));
        assert!(g.declare_fetch_probe("leaf").unwrap_err().contains("0 indirect call site(s)"));
    }

    /// A table target is a word of a read-only table section that equals a
    /// function address with the Thumb bit, at a 4-aligned address. A word
    /// of the Kernel's initialised data that happens to equal one is not
    /// (the coincidence of the isolation build, 2026-11-18): no section of
    /// state is a table section.
    #[test]
    fn a_table_target_is_a_word_of_a_read_only_table_section() {
        let by_address: BTreeMap<u32, usize> = [(0x100, 0), (0x200, 1)].into_iter().collect();
        let word = |v: u32| v.to_le_bytes();
        let table: Vec<u8> = [word(0x201), word(0x200), word(0x999), word(0x101)].concat();
        let found = table_words(&[(".rsk.tables", 0x1000, &table)], &by_address);
        // 0x200 has no Thumb bit; 0x999 is no function.
        assert_eq!(found, vec![(".rsk.tables".to_string(), 0x1000, 1), (".rsk.tables".to_string(), 0x100C, 0)]);
        // The same bytes at an unaligned start: no aligned word equals an address.
        let mut shifted = vec![0u8];
        shifted.extend_from_slice(&table);
        shifted.extend_from_slice(&[0, 0, 0]);
        assert!(table_words(&[(".rodata", 0x1001, &shifted)], &by_address).iter().all(|w| w.1 % 4 == 0));
        assert!(!TABLE_SECTIONS.contains(&".kernel.data"));
        assert!(!TABLE_SECTIONS.contains(&".data"));
        assert!(TABLE_SECTIONS.iter().all(|s| !s.contains("bss") && !s.contains("noinit")));
    }

    /// A function table the linker script placed inside a Partition's code
    /// section is found (outside every function body), and it is the target
    /// set of that Partition's and the shared code's unresolved indirect
    /// calls only: another Partition's caller stays unresolved (PR-18).
    #[test]
    fn a_table_inside_a_partitions_code_section_bounds_its_own_callers() {
        let by_address: BTreeMap<u32, usize> = [(0x3_0000, 0), (0x3_0010, 1), (0x3_8000, 2), (0x2_0000, 3)].into_iter().collect();
        // .p0.text: two 16-byte bodies, then a table of one entry and one word that is no function.
        let mut data = vec![0u8; 0x20];
        // A word inside the first body that equals a function address: its literal pool, not a table.
        data[8..12].copy_from_slice(&0x3_0011u32.to_le_bytes());
        data.extend_from_slice(&0x3_0011u32.to_le_bytes());
        data.extend_from_slice(&0x1234_5679u32.to_le_bytes());
        let words = embedded_words(".p0.text", 0x3_0000, &data, &[(0x3_0000, 16), (0x3_0010, 16)], &by_address);
        assert_eq!(words, vec![(".p0.text".to_string(), 0x3_0020, 1)]);

        assert!(embedded_table_section(".p0.text") && embedded_table_section(".text") && embedded_table_section(".shared.rsk"));
        assert!(!embedded_table_section(".kernel.text") && !embedded_table_section(".vector_table"));
        assert!(table_readable(".p0.text", ".p0.text") && table_readable(".text", ".p0.text") && table_readable(".p1.text", ".text"));
        assert!(!table_readable(".p1.text", ".p0.text") && !table_readable(".kernel.text", ".p0.text"));

        let mut g = graph(vec![
            ("p0_caller", 0x3_0000, 8, vec![], 1, vec![], false),
            ("p0_target", 0x3_0010, 24, vec![], 0, vec![], false),
            ("p1_caller", 0x3_8000, 8, vec![], 1, vec![], false),
            ("shared_caller", 0x2_0000, 8, vec![], 1, vec![], false),
        ]);
        g.embedded = words;
        g.section_of = vec![".p0.text".into(), ".p0.text".into(), ".p1.text".into(), ".text".into()];
        let r = g.analyse();
        assert_eq!(r[0].usage, 8 + 24);
        assert!(r[0].unbounded.is_none());
        assert!(r[2].unbounded.as_deref().unwrap().starts_with("unresolved indirect"));
        assert_eq!(r[3].usage, 8 + 24);
    }
}
