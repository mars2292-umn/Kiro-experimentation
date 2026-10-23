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

pub struct CallGraph {
    pub functions: Vec<Function>,
    /// Address -> index into `functions`.
    by_address: BTreeMap<u32, usize>,
    pub indirect_sites: u32,
    /// Functions whose address appears as a word of the read-only data
    /// (vtable slots, function tables): the finite target universe of an
    /// indirect call that materialises no function address itself (PR-18).
    pub table_targets: Vec<usize>,
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
            };
            let mut partial: BTreeMap<u8, u32> = BTreeMap::new();
            for insn in &insns {
                match &insn.effect {
                    Effect::SpDown(n) => f.frame = f.frame.saturating_add(*n),
                    Effect::SpDynamic => f.dynamic_sp = true,
                    Effect::Call(t) => f.direct.push(*t),
                    Effect::Branch(t) => {
                        if *t < *address || *t >= end {
                            f.direct.push(*t);
                        }
                    }
                    Effect::CompareBranch(t) => {
                        if *t < *address || *t >= end {
                            f.direct.push(*t);
                            f.nonlocal_cbz.push((insn.address, *t));
                        }
                    }
                    Effect::Nop => f.nops.push(insn.address),
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
            f.direct.sort_unstable();
            f.direct.dedup();
            f.materialised.sort_unstable();
            f.materialised.dedup();
            indirect_total += f.indirect_sites;
            by_address.insert(*address, i);
            functions.push(f);
        }
        // Function addresses stored in the read-only and initialised data.
        let mut table_targets = Vec::new();
        for name in [".rodata", ".kernel.data", ".data", ".rsk.tables"] {
            if let Some(section) = elf.section_by_name(name) {
                if let Ok(data) = elf.section_data(section) {
                    for w in data.chunks_exact(4) {
                        let v = u32::from_le_bytes([w[0], w[1], w[2], w[3]]);
                        if v & 1 == 1 {
                            if let Some(&i) = by_address.get(&(v & !1)) {
                                table_targets.push(i);
                            }
                        }
                    }
                }
            }
        }
        table_targets.sort_unstable();
        table_targets.dedup();
        Ok(CallGraph { functions, by_address, indirect_sites: indirect_total, table_targets })
    }

    /// A graph over given functions with no indirect-call universe (fixtures).
    pub fn from_functions(functions: Vec<Function>) -> CallGraph {
        let by_address = functions.iter().enumerate().map(|(i, f)| (f.address, i)).collect();
        CallGraph { functions, by_address, indirect_sites: 0, table_targets: vec![] }
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
        if f.indirect_sites > 0 || f.indirect_branches > 0 {
            if !f.materialised.is_empty() {
                out.extend(f.materialised.iter().filter_map(|a| self.index_of(*a)));
            } else if f.indirect_sites > 0 || self.uses_table(i) {
                // No address in the body: the call goes through a table (a
                // vtable or function table) in the data, whose function
                // addresses are the finite target set.
                if self.table_targets.is_empty() {
                    return None;
                }
                out.extend(self.table_targets.iter().copied());
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
            functions.push(Function { name: name.into(), address, size: 4, frame, dynamic_sp: dynamic, direct, indirect_sites: indirect, indirect_branches: 0, materialised, svcs: vec![], fp: vec![], privileged: vec![], ppb_refs: vec![], nonlocal_cbz: vec![], nops: vec![] });
        }
        CallGraph { functions, by_address, indirect_sites: 0, table_targets: vec![] }
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
}

