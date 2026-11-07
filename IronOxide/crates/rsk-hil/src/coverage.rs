//! Branch coverage of the measurement inputs (task 19.2; R34.4, R53.3):
//! the coverage build compiles the Partition crates with
//! `-C instrument-coverage -Z coverage-options=branch` (the Kernel, the
//! API, and the declaration as in every HIL build), its counters lie in
//! each Partition's RAM above its image (`.p<i>.cov` of the generated
//! linker script), zeroed at instant 0 by the Kernel, and after the run
//! the rig reads them over the probe and assembles an LLVM raw profile
//! from them and the ELF's profile data and names, which `llvm-profdata`
//! and `llvm-cov` of the coverage toolchain turn into the branch report.
//! Each Task entry's coverage is that of its body function and the
//! helpers of its crate (every function of the crate that is not another
//! Task's body: the spins, the injections, the Critical_Section closures),
//! so that a Job's whole execution is covered; each Critical_Section site
//! takes its Task's figure. An item below the threshold the
//! Verification_Plan records (`[coverage] branch_percent` of
//! `hil-rig.toml`) fails the run (R34.4).

use std::path::{Path, PathBuf};
use std::process::Command;

use serde::{Deserialize, Serialize};

use crate::board::Target;

pub const SCHEMA: &str = "rsk-branch-coverage/1";

/// The raw-profile format the coverage toolchain's `llvm-profdata` reads
/// (LLVM 23: version 11, nineteen 64-bit header fields; 32-bit targets
/// carry the `R` magic), as written by the host's own instrumented runs
/// and checked by `profraw_round_trips_through_llvm_profdata`.
const RAW_VERSION: u64 = 11;
const RAW_MAGIC_32: u64 = (0xffu64 << 56) | (b'l' as u64) << 48 | (b'p' as u64) << 40 | (b'r' as u64) << 32 | (b'o' as u64) << 24 | (b'f' as u64) << 16 | (b'R' as u64) << 8 | 129;
/// The profile data record of a 32-bit target under this LLVM: name
/// hash, function hash, the relative counter pointer at byte 16, and the
/// rest (56 bytes in all).
const DATA_RECORD: usize = 56;
const COUNTER_PTR_AT: usize = 16;
const NUM_COUNTERS_AT: usize = 36;
/// The virtual distance from the data to the counters in the assembled
/// file (any value above the data's size; every record is rewritten
/// against it).
const VIRTUAL_COUNTERS_DELTA: i64 = 0x1000_0000;

/// The Kernel's counter range, as `Range::partition`.
pub const KERNEL_RANGE: usize = 255;

/// A Partition's counter range in RAM: `_p<i>_scov` to `_p<i>_ecov`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Range {
    pub partition: usize,
    pub start: u32,
    pub end: u32,
}

/// The counter ranges of the binary, one per Partition with counters.
pub fn ranges(elf: &rsk_elf::Elf) -> Result<Vec<Range>, String> {
    let symbols = elf.symbols().map_err(|e| e.to_string())?;
    let mut out = Vec::new();
    for i in 0..16 {
        let start = symbols.iter().find(|s| s.name == format!("_p{i}_scov")).map(|s| s.value);
        let end = symbols.iter().find(|s| s.name == format!("_p{i}_ecov")).map(|s| s.value);
        match (start, end) {
            (Some(start), Some(end)) if end > start => out.push(Range { partition: i, start, end }),
            _ => {}
        }
    }
    // The Kernel's counters (the Kernel coverage build, task 19.5): `_k_scov` to `_k_ecov`, Partition 255.
    let start = symbols.iter().find(|s| s.name == "_k_scov").map(|s| s.value);
    let end = symbols.iter().find(|s| s.name == "_k_ecov").map(|s| s.value);
    if let (Some(start), Some(end)) = (start, end) {
        if end > start {
            out.push(Range { partition: KERNEL_RANGE, start, end });
        }
    }
    Ok(out)
}

/// The profile data and names of the ELF: `(data address, data bytes, names bytes)`.
fn profile_sections(elf: &rsk_elf::Elf) -> Result<(u32, Vec<u8>, Vec<u8>), String> {
    let symbols = elf.symbols().map_err(|e| e.to_string())?;
    let sym = |name: &str| symbols.iter().find(|s| s.name == name).map(|s| s.value).ok_or_else(|| format!("no symbol `{name}` in the binary: not a coverage build"));
    let (ds, de) = (sym("__start___llvm_prf_data")?, sym("__stop___llvm_prf_data")?);
    let (ns, ne) = (sym("__start___llvm_prf_names")?, sym("__stop___llvm_prf_names")?);
    if de <= ds || ne <= ns {
        return Err("the binary carries no profile data: not a coverage build".into());
    }
    let data = elf.read_at(ds, (de - ds) as usize).map_err(|e| format!("profile data: {e}"))?;
    let names = elf.read_at(ns, (ne - ns) as usize).map_err(|e| format!("profile names: {e}"))?;
    Ok((ds, data, names))
}

/// Assembles the raw profile from the ELF's profile data and names and
/// the counter ranges as read from the board (`(range, bytes)`), the
/// counters of every Partition joined into one block and each record's
/// counter pointer rewritten to its place in it.
pub fn profraw(elf: &rsk_elf::Elf, dumps: &[(Range, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let (data_addr, mut data, names) = profile_sections(elf)?;
    if data.len() % DATA_RECORD != 0 {
        return Err(format!("profile data of {} bytes is not a whole number of {DATA_RECORD}-byte records", data.len()));
    }
    let records = data.len() / DATA_RECORD;
    // The joined counters: each range at its offset in the block.
    let mut block = Vec::new();
    let mut at = Vec::new();
    for (r, bytes) in dumps {
        if bytes.len() != (r.end - r.start) as usize {
            return Err(format!("Partition {}'s counter range read {} bytes instead of {}", r.partition, bytes.len(), r.end - r.start));
        }
        at.push((r.start, r.end, block.len()));
        block.extend_from_slice(bytes);
        while block.len() % 8 != 0 {
            block.push(0);
        }
    }
    for k in 0..records {
        let rec = &mut data[k * DATA_RECORD..(k + 1) * DATA_RECORD];
        let rel = i32::from_le_bytes(rec[COUNTER_PTR_AT..COUNTER_PTR_AT + 4].try_into().unwrap()) as i64;
        let n = u32::from_le_bytes(rec[NUM_COUNTERS_AT..NUM_COUNTERS_AT + 4].try_into().unwrap()) as i64;
        let record_addr = data_addr as i64 + (k * DATA_RECORD) as i64;
        let counter_addr = record_addr + rel;
        let Some((start, _, off)) = at.iter().find(|(s, e, _)| counter_addr >= *s as i64 && counter_addr + 8 * n <= *e as i64) else {
            return Err(format!("record {k}'s counters at {counter_addr:#x} ({n} counters) lie in no Partition's counter range"));
        };
        let in_block = *off as i64 + (counter_addr - *start as i64);
        let new_rel = VIRTUAL_COUNTERS_DELTA + in_block - (k * DATA_RECORD) as i64;
        rec[COUNTER_PTR_AT..COUNTER_PTR_AT + 4].copy_from_slice(&(new_rel as i32).to_le_bytes());
    }
    let mut out = Vec::new();
    let header: [u64; 19] = [
        RAW_MAGIC_32,
        RAW_VERSION,
        0,
        records as u64,
        0,
        (block.len() / 8) as u64,
        0,
        0,
        0,
        0,
        0,
        0,
        names.len() as u64,
        VIRTUAL_COUNTERS_DELTA as u64,
        0,
        0,
        0,
        0,
        2,
    ];
    for h in header {
        out.extend_from_slice(&h.to_le_bytes());
    }
    out.extend_from_slice(&data);
    out.extend_from_slice(&block);
    out.extend_from_slice(&names);
    while out.len() % 8 != 0 {
        out.push(0);
    }
    Ok(out)
}

/// The coverage toolchain's tools.
pub struct Tools {
    pub profdata: PathBuf,
    pub cov: PathBuf,
}

/// Runs `llvm-profdata merge` and `llvm-cov export` over the raw profile
/// and the binary, restricted to `sources`; returns the export (JSON).
pub fn export(tools: &Tools, elf: &Path, profraw: &Path, work: &Path, sources: &[PathBuf]) -> Result<serde_json::Value, String> {
    let _ = std::fs::create_dir_all(work);
    let profdata = work.join("coverage.profdata");
    let out = Command::new(&tools.profdata).arg("merge").arg("-o").arg(&profdata).arg(profraw).output().map_err(|e| format!("cannot run {}: {e}", tools.profdata.display()))?;
    if !out.status.success() {
        return Err(format!("llvm-profdata merge failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let mut cmd = Command::new(&tools.cov);
    cmd.arg("export").arg("-format=text").arg("-instr-profile").arg(&profdata).arg(elf);
    for s in sources {
        cmd.arg("-sources").arg(s);
    }
    let out = cmd.output().map_err(|e| format!("cannot run {}: {e}", tools.cov.display()))?;
    if !out.status.success() {
        return Err(format!("llvm-cov export failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let _ = std::fs::write(work.join("coverage-export.json"), &out.stdout);
    serde_json::from_slice(&out.stdout).map_err(|e| format!("llvm-cov export is not JSON: {e}"))
}

/// Reads the counters of every Partition from the board and assembles
/// the raw profile into `work`.
pub fn collect(target: &Target, elf_path: &Path, work: &Path) -> Result<(PathBuf, Vec<Range>), String> {
    let elf = rsk_elf::Elf::read(elf_path).map_err(|e| format!("{}: {e}", elf_path.display()))?;
    let ranges = ranges(&elf)?;
    if ranges.is_empty() {
        return Err("the binary has no counter range: not a coverage build".into());
    }
    let mut dumps = Vec::new();
    for r in &ranges {
        let bytes = target.read(r.start, (r.end - r.start) as usize, &work.join(format!("cov-p{}.bin", r.partition)))?;
        dumps.push((r.clone(), bytes));
    }
    let raw = profraw(&elf, &dumps)?;
    let path = work.join("coverage.profraw");
    std::fs::write(&path, &raw).map_err(|e| format!("cannot write {}: {e}", path.display()))?;
    Ok((path, ranges))
}

/// One function of the export: its name, its branches (true and false
/// outcomes each counted), and the uncovered outcomes as `line:col` with
/// the outcome.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Function {
    pub name: String,
    pub file: String,
    pub branches: u64,
    pub covered: u64,
    pub uncovered: Vec<String>,
}

/// The functions of the export (every file given to `-sources`).
pub fn functions(export: &serde_json::Value) -> Vec<Function> {
    let mut out = Vec::new();
    for f in export.get("data").and_then(|d| d.as_array()).and_then(|d| d.first()).and_then(|d| d.get("functions")).and_then(|f| f.as_array()).into_iter().flatten() {
        let name = f.get("name").and_then(|n| n.as_str()).unwrap_or("").to_string();
        let file = f.get("filenames").and_then(|n| n.as_array()).and_then(|n| n.first()).and_then(|n| n.as_str()).unwrap_or("").to_string();
        let mut branches = 0;
        let mut covered = 0;
        let mut uncovered = Vec::new();
        for b in f.get("branches").and_then(|b| b.as_array()).into_iter().flatten() {
            let Some(b) = b.as_array() else { continue };
            let at = |i: usize| b.get(i).and_then(|v| v.as_u64()).unwrap_or(0);
            let (line, col, taken, not_taken) = (at(0), at(1), at(4), at(5));
            branches += 2;
            for (count, outcome) in [(taken, "true"), (not_taken, "false")] {
                if count > 0 {
                    covered += 1;
                } else {
                    uncovered.push(format!("{line}:{col} {outcome}"));
                }
            }
        }
        out.push(Function { name, file, branches, covered, uncovered });
    }
    out
}

/// Whether a mangled symbol is `<krate>::<function>` (the crate's name
/// and the function's as length-prefixed identifiers, the function the
/// last; a legacy symbol's trailing hash segment, `17h<16 hex>E`, is
/// dropped first).
pub fn is_function(symbol: &str, krate: &str, function: &str) -> bool {
    let path = legacy_path(symbol).unwrap_or(symbol);
    path.contains(&format!("{}{krate}", krate.len())) && path.ends_with(&format!("{}{function}", function.len()))
}

/// A legacy symbol (`_ZN..E`) without its hash segment and the `E`.
fn legacy_path(symbol: &str) -> Option<&str> {
    let s = symbol.strip_prefix("_ZN")?.strip_suffix('E')?;
    let hash_at = s.rfind("17h")?;
    if s.len() - hash_at == 19 { Some(&s[..hash_at]) } else { Some(s) }
}

/// Whether a symbol belongs to the crate.
pub fn in_crate(symbol: &str, krate: &str) -> bool {
    symbol.contains(&format!("{}{krate}", krate.len()))
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Item {
    /// The WCET_Record item (a Task id or a Critical_Section site).
    pub item: String,
    /// The Task's body function and the helpers counted with it.
    pub functions: Vec<String>,
    pub branches: u64,
    pub covered: u64,
    pub percent: u64,
    pub uncovered: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub configuration: String,
    /// The coverage build's hash (not the measured binary's: the
    /// instrumented bodies differ, the inputs are the same).
    pub binary_hash: String,
    pub threshold_percent: u64,
    pub items: Vec<Item>,
    pub functions: Vec<Function>,
}

impl Report {
    pub fn ok(&self) -> bool {
        !self.items.is_empty() && self.items.iter().all(|i| i.percent >= self.threshold_percent)
    }

    pub fn below(&self) -> Vec<&Item> {
        self.items.iter().filter(|i| i.percent < self.threshold_percent).collect()
    }

    pub fn summary(&self) -> String {
        let tasks: Vec<String> = self.items.iter().filter(|i| !i.item.contains("#cs:")).map(|i| format!("{} {} % ({} of {} branch outcomes)", i.item, i.percent, i.covered, i.branches)).collect();
        format!("branch coverage of the measurement inputs against the {} % threshold (R34.4): {}; {} below", self.threshold_percent, tasks.join(", "), self.below().len())
    }

    pub fn render(&self) -> String {
        let mut s = String::from("| Item | Functions | Branch outcomes | Covered | Percent | Verdict |\n|---|---|---|---|---|---|\n");
        for i in &self.items {
            s.push_str(&format!(
                "| `{}` | {} | {} | {} | {} % | {} |\n",
                i.item,
                i.functions.len(),
                i.branches,
                i.covered,
                i.percent,
                if i.percent >= self.threshold_percent { "pass" } else { "BELOW" }
            ));
        }
        s.push_str("\n| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |\n|---|---|---|---|\n");
        for f in &self.functions {
            s.push_str(&format!("| `{}` | {} | {} | {} |\n", f.name, f.branches, f.covered, f.uncovered.join(", ")));
        }
        s
    }

    pub fn print(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }
}

/// A function's path as the Link_Checker and this reader both name it:
/// no leading `::`, closures as `{closure}`.
pub fn normalized(path: &str) -> String {
    path.trim_start_matches("::").replace("{{closure}}", "{closure}")
}

/// The items' coverage: for each Task `(item id, crate, body function,
/// Critical_Section sites)`, the body, its closures, and the crate's
/// helpers (every function of the crate that is no Task's body or
/// closure and not a generated Job wrapper). The Partition bodies are
/// inlined into their wrappers, so the binary's call graph has no call to
/// attribute a helper by; a helper therefore counts for every Task of its
/// crate, which can only lower a Task's figure. Each site takes its
/// Task's figure.
pub fn report(functions: Vec<Function>, tasks: &[(String, String, String, Vec<String>)], configuration: &str, binary_hash: &str, threshold_percent: u64) -> Report {
    let bodies: Vec<String> = tasks.iter().map(|(_, k, b, _)| format!("{k}::{b}")).collect();
    let of_body = |path: &str, body: &str| path == body || path.starts_with(&format!("{body}::{{closure}}"));
    let mut items = Vec::new();
    for (item, krate, body, sites) in tasks {
        let own = format!("{krate}::{body}");
        let mut names = Vec::new();
        let (mut branches, mut covered) = (0, 0);
        let mut uncovered = Vec::new();
        for f in &functions {
            let path = normalized(&short(&f.name));
            let in_crate = path.starts_with(&format!("{krate}::"));
            let wrapper = path.starts_with(&format!("{krate}::rsk_jobs::"));
            let other = bodies.iter().any(|b| *b != own && of_body(&path, b));
            if of_body(&path, &own) || (in_crate && !wrapper && !other) {
                names.push(f.name.clone());
                branches += f.branches;
                covered += f.covered;
                uncovered.extend(f.uncovered.iter().map(|u| format!("{}: {u}", short(&f.name))));
            }
        }
        let percent = if branches == 0 { 100 } else { covered * 100 / branches };
        items.push(Item { item: item.clone(), functions: names.clone(), branches, covered, percent, uncovered: uncovered.clone() });
        for site in sites {
            items.push(Item { item: site.clone(), functions: names.clone(), branches, covered, percent, uncovered: uncovered.clone() });
        }
    }
    let functions = functions.into_iter().map(|f| Function { name: short(&f.name), ..f }).collect();
    Report { schema: SCHEMA.into(), configuration: configuration.into(), binary_hash: binary_hash.into(), threshold_percent, items, functions }
}

/// A readable form of a v0-mangled name (`_R` symbols): the identifiers
/// of its path joined by `::`, closures as `{closure}`; generic arguments
/// are dropped, and a path form this reader does not follow (impls,
/// back-references) leaves the symbol as it is.
pub fn short(symbol: &str) -> String {
    if symbol.starts_with("_ZN") {
        return short_legacy(symbol);
    }
    let Some(rest) = symbol.strip_prefix("_R") else { return symbol.to_string() };
    let b = rest.as_bytes();
    let mut pos = 0;
    match v0_path(b, &mut pos) {
        Some(parts) if !parts.is_empty() => parts.join("::"),
        _ => symbol.to_string(),
    }
}

/// A legacy symbol's path: its length-prefixed segments, closures as
/// `{closure}`, the hash segment dropped.
fn short_legacy(symbol: &str) -> String {
    let Some(path) = legacy_path(symbol) else { return symbol.to_string() };
    let b = path.as_bytes();
    let mut pos = 0;
    let mut parts = Vec::new();
    while pos < b.len() {
        let start = pos;
        while pos < b.len() && b[pos].is_ascii_digit() {
            pos += 1;
        }
        let Ok(len) = path[start..pos].parse::<usize>() else { return symbol.to_string() };
        if len == 0 || pos + len > b.len() {
            return symbol.to_string();
        }
        let seg = &path[pos..pos + len];
        pos += len;
        parts.push(if seg.contains("closure") && seg.contains('$') { "{closure}".to_string() } else { seg.replace("$u7b$", "{").replace("$u7d$", "}").replace("$LT$", "<").replace("$GT$", ">").replace("..", "::") });
    }
    parts.join("::")
}

fn v0_ident(b: &[u8], pos: &mut usize) -> Option<String> {
    // [`s` base-62 `_`] decimal-length [`_`] bytes
    if *pos < b.len() && b[*pos] == b's' {
        *pos += 1;
        while *pos < b.len() && b[*pos] != b'_' {
            *pos += 1;
        }
        *pos += 1;
    }
    let start = *pos;
    while *pos < b.len() && b[*pos].is_ascii_digit() {
        *pos += 1;
    }
    let len: usize = std::str::from_utf8(&b[start..*pos]).ok()?.parse().ok()?;
    if *pos < b.len() && b[*pos] == b'_' {
        *pos += 1;
    }
    if *pos + len > b.len() {
        return None;
    }
    let ident = std::str::from_utf8(&b[*pos..*pos + len]).ok()?.to_string();
    *pos += len;
    Some(ident)
}

fn v0_path(b: &[u8], pos: &mut usize) -> Option<Vec<String>> {
    let tag = *b.get(*pos)?;
    *pos += 1;
    match tag {
        b'C' => Some(vec![v0_ident(b, pos)?]),
        b'N' => {
            let ns = *b.get(*pos)?;
            *pos += 1;
            let mut path = v0_path(b, pos)?;
            let ident = v0_ident(b, pos)?;
            path.push(if ident.is_empty() && ns == b'C' { "{closure}".to_string() } else if ident.is_empty() { "{_}".to_string() } else { ident });
            Some(path)
        }
        b'I' => v0_path(b, pos),
        _ => None,
    }
}

/// The Kernel's coverage report (task 19.5, R53.3): llvm-cov's totals and
/// per-file figures over the Kernel sources from the Kernel coverage build.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelFile {
    pub file: String,
    pub lines_covered: u64,
    pub lines: u64,
    pub branches_covered: u64,
    pub branches: u64,
    pub functions_covered: u64,
    pub functions: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct KernelReport {
    pub schema: String,
    pub configuration: String,
    pub binary_hash: String,
    pub totals: KernelFile,
    pub files: Vec<KernelFile>,
}

pub const KERNEL_SCHEMA: &str = "rsk-kernel-coverage/1";

fn kernel_file(name: &str, s: &serde_json::Value) -> KernelFile {
    let n = |k: &str, f: &str| s[k][f].as_u64().unwrap_or(0);
    KernelFile { file: name.to_string(), lines_covered: n("lines", "covered"), lines: n("lines", "count"), branches_covered: n("branches", "covered"), branches: n("branches", "count"), functions_covered: n("functions", "covered"), functions: n("functions", "count") }
}

/// From an export restricted to the Kernel sources; `root` is stripped
/// from the file names.
pub fn kernel_report(export: &serde_json::Value, root: &str, configuration: &str, binary_hash: &str) -> KernelReport {
    let d = &export["data"][0];
    let totals = kernel_file("total", &d["totals"]);
    let mut files: Vec<KernelFile> = d["files"]
        .as_array()
        .into_iter()
        .flatten()
        .map(|f| {
            let name = f["filename"].as_str().unwrap_or("");
            kernel_file(name.strip_prefix(root).unwrap_or(name).trim_start_matches('/'), &f["summary"])
        })
        .collect();
    files.sort_by(|a, b| a.file.cmp(&b.file));
    KernelReport { schema: KERNEL_SCHEMA.into(), configuration: configuration.into(), binary_hash: binary_hash.into(), totals, files }
}

impl KernelReport {
    pub fn pct(c: u64, n: u64) -> String {
        if n == 0 {
            "-".to_string()
        } else {
            format!("{:.1} %", c as f64 * 100.0 / n as f64)
        }
    }

    pub fn summary(&self) -> String {
        let t = &self.totals;
        format!(
            "the Kernel on the Target: lines {} ({} of {}), branch outcomes {} ({} of {}), functions {} ({} of {}) of the Kernel sources",
            Self::pct(t.lines_covered, t.lines),
            t.lines_covered,
            t.lines,
            Self::pct(t.branches_covered, t.branches),
            t.branches_covered,
            t.branches,
            Self::pct(t.functions_covered, t.functions),
            t.functions_covered,
            t.functions
        )
    }

    pub fn render(&self) -> String {
        let mut s = String::from("| File | Lines | Branch outcomes | Functions |\n|---|---|---|---|\n");
        for f in self.files.iter().chain(std::iter::once(&self.totals)) {
            s.push_str(&format!(
                "| {} | {} ({} of {}) | {} ({} of {}) | {} ({} of {}) |\n",
                if f.file == "total" { "**total**" } else { &f.file },
                Self::pct(f.lines_covered, f.lines),
                f.lines_covered,
                f.lines,
                Self::pct(f.branches_covered, f.branches),
                f.branches_covered,
                f.branches,
                Self::pct(f.functions_covered, f.functions),
                f.functions_covered,
                f.functions
            ));
        }
        s
    }

    pub fn print(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mangled_names_are_matched_by_crate_and_function() {
        let s = "_RNvCs2oSbTXImU10_13p1_demo_part07control";
        assert!(is_function(s, "p1_demo_part0", "control"));
        assert!(!is_function(s, "p1_demo_part0", "sensor"));
        assert!(!is_function(s, "p1_demo_part1", "control"));
        assert!(in_crate(s, "p1_demo_part0"));
        assert_eq!(short(s), "p1_demo_part0::control");
        assert_eq!(short("_RNvNtCs2oSbTXImU10_13p1_demo_part06inject7spin_ms"), "p1_demo_part0::inject::spin_ms");
        assert_eq!(short("_RNCNvCs2oSbTXImU10_13p1_demo_part06sensor0"), "p1_demo_part0::sensor::{closure}");
        assert_eq!(short("_RNCNvCs2oSbTXImU10_13p1_demo_part06sensors_0"), "p1_demo_part0::sensor::{closure}");
        assert_eq!(short("_RNvCs2oSbTXImU10_7___rustc17rust_begin_unwind"), "__rustc::rust_begin_unwind");
        assert_eq!(short("not_v0"), "not_v0");
        let legacy = "_ZN13p1_demo_part07control17h0123456789abcdefE";
        assert!(is_function(legacy, "p1_demo_part0", "control"));
        assert!(!is_function(legacy, "p1_demo_part0", "sensor"));
        assert_eq!(short(legacy), "p1_demo_part0::control");
        assert_eq!(short("_ZN13p1_demo_part06sensor28_$u7b$$u7b$closure$u7d$$u7d$17h0123456789abcdefE"), "p1_demo_part0::sensor::{closure}");
        assert_eq!(short("_ZN13p1_demo_part06inject7spin_ms17h0123456789abcdefE"), "p1_demo_part0::inject::spin_ms");
    }

    #[test]
    fn items_take_the_body_its_closures_and_the_crate_helpers() {
        let fs = vec![
            Function { name: "_RNvCsX_13p1_demo_part07control".into(), file: "f".into(), branches: 4, covered: 4, uncovered: vec![] },
            Function { name: "_ZN13p1_demo_part06sensor28_$u7b$$u7b$closure$u7d$$u7d$17h0123456789abcdefE".into(), file: "f".into(), branches: 2, covered: 2, uncovered: vec![] },
            Function { name: "_ZN13p1_demo_part08rsk_jobs7control17h0123456789abcdefE".into(), file: "f".into(), branches: 0, covered: 0, uncovered: vec![] },
            Function { name: "_RNvCsX_13p1_demo_part06sensor".into(), file: "f".into(), branches: 2, covered: 1, uncovered: vec!["10:5 false".into()] },
            Function { name: "_RNvNtCsX_13p1_demo_part06inject7spin_ms".into(), file: "f".into(), branches: 2, covered: 2, uncovered: vec![] },
            Function { name: "_RNvCsY_13p1_demo_part26filter".into(), file: "g".into(), branches: 2, covered: 0, uncovered: vec!["3:1 true".into(), "3:1 false".into()] },
        ];
        let tasks = vec![
            ("part0.control".to_string(), "p1_demo_part0".to_string(), "control".to_string(), vec!["part0.control#cs:state".to_string()]),
            ("part0.sensor".to_string(), "p1_demo_part0".to_string(), "sensor".to_string(), vec![]),
            ("part2.filter".to_string(), "p1_demo_part2".to_string(), "filter".to_string(), vec![]),
        ];
        let r = report(fs, &tasks, "c", "h", 75);
        assert_eq!(r.functions[0].name, "p1_demo_part0::control");
        let by = |item: &str| r.items.iter().find(|i| i.item == item).unwrap().clone();
        assert_eq!((by("part0.control").branches, by("part0.control").covered, by("part0.control").percent), (6, 6, 100));
        assert_eq!(by("part0.control#cs:state").percent, 100);
        assert_eq!((by("part0.sensor").branches, by("part0.sensor").covered, by("part0.sensor").percent), (6, 5, 83), "the body, its closure, and the helper; not control's body nor the wrapper");
        assert_eq!(by("part0.control").functions.len(), 2, "control's body and the helper: sensor's closure is sensor's");
        assert_eq!(by("part2.filter").percent, 0);
        assert!(!r.ok());
        assert_eq!(r.below().len(), 1);
        let back: Report = serde_json::from_str(&r.print()).unwrap();
        assert_eq!(back, r);
    }

    #[test]
    fn export_functions_count_both_outcomes() {
        let v: serde_json::Value = serde_json::json!({"data":[{"functions":[{"name":"_RNvCsX_13p1_demo_part07control","filenames":["a.rs"],"branches":[[5,9,5,14,3,0,0,0,4],[7,1,7,2,0,2,0,0,4]]}]}]});
        let f = functions(&v);
        assert_eq!(f.len(), 1);
        assert_eq!((f[0].branches, f[0].covered), (4, 2));
        assert_eq!(f[0].uncovered, ["5:9 false", "7:1 true"]);
    }

    /// The raw profile the rig assembles is one `llvm-profdata` of the
    /// coverage toolchain reads: run against a coverage build with zeroed
    /// counters (`RSK_COV_ELF`, `RSK_LLVM_PROFDATA`), `cargo test -p rsk-hil -- --ignored`.
    #[test]
    #[ignore]
    fn profraw_round_trips_through_llvm_profdata() {
        let elf_path = std::env::var("RSK_COV_ELF").expect("RSK_COV_ELF");
        let profdata = std::env::var("RSK_LLVM_PROFDATA").expect("RSK_LLVM_PROFDATA");
        let elf = rsk_elf::Elf::read(Path::new(&elf_path)).unwrap();
        let ranges = ranges(&elf).unwrap();
        assert!(!ranges.is_empty());
        let dumps: Vec<(Range, Vec<u8>)> = ranges.iter().map(|r| (r.clone(), vec![0; (r.end - r.start) as usize])).collect();
        let raw = profraw(&elf, &dumps).unwrap();
        let path = std::env::temp_dir().join("rsk-hil-test.profraw");
        std::fs::write(&path, &raw).unwrap();
        let out = Command::new(profdata).arg("show").arg("--all-functions").arg(&path).output().unwrap();
        assert!(out.status.success(), "{}", String::from_utf8_lossy(&out.stderr));
        let text = String::from_utf8_lossy(&out.stdout);
        assert!(text.contains("Total functions:"), "{text}");
        eprintln!("{text}");
    }
}
