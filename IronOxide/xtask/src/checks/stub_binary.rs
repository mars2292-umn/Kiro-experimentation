//! The entry stubs of a linked binary are the assembly the proofs are tied
//! to (2026-11-22).
//!
//! `checks/stub_model.rs` ties the stub model to the assembly *source*.
//! This check ties the source to a *binary*: for every modelled stub the
//! binary links, the instructions `llvm-objdump` disassembles between the
//! stub's symbol and its end are, one for one and in order, the
//! instructions of the stub's `naked_asm!` block: the same mnemonic, the
//! same registers in the same order, the same immediates, and for a call
//! or a branch out of the stub the symbol the source names.
//!
//! What it normalises: the width suffix (`.w`), a repeated first register
//! (`sub sp, sp, #16` is `sub sp, #0x10`), register ranges, an offset of
//! zero, and the `sy` option of `dsb`/`isb`. What it does not see: the
//! target of a branch to a local label (only that a branch is there), and
//! the encoding itself, which is the assembler's and the disassembler's.
//! It applies to a Flight_Build: the witness builds of the stubs
//! (`defer-probe`) have other instructions by design.

use std::path::Path;
use std::process::Command;

use super::stub_model::{stubs_of, NOT_MODELLED, STUBS, STUB_MODELS};

/// One instruction, as far as the two texts are compared.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Insn {
    pub mnemonic: String,
    /// Registers and other operand words, in order, a repeat of the one
    /// before dropped.
    pub words: Vec<String>,
    /// Immediates other than zero, in order.
    pub imms: Vec<i64>,
    /// A named target outside the stub (`bl`, `b`).
    pub target: Option<String>,
}

fn number(t: &str) -> Option<i64> {
    let t = t.trim_start_matches('#');
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r),
        None => (false, t),
    };
    let v = match t.strip_prefix("0x") {
        Some(h) => i64::from_str_radix(h, 16).ok()?,
        None => t.parse::<i64>().ok()?,
    };
    Some(if neg { -v } else { v })
}

/// `r3-r11` or `s16-s31` as the registers of the range.
fn expand(word: &str) -> Vec<String> {
    if let Some((a, b)) = word.split_once('-') {
        let prefix: String = a.chars().take_while(|c| c.is_ascii_alphabetic()).collect();
        if !prefix.is_empty() && b.starts_with(&prefix) {
            if let (Ok(lo), Ok(hi)) = (a[prefix.len()..].parse::<u32>(), b[prefix.len()..].parse::<u32>()) {
                return (lo..=hi).map(|n| format!("{prefix}{n}")).collect();
            }
        }
    }
    vec![word.to_string()]
}

fn takes_target(mnemonic: &str) -> bool {
    matches!(mnemonic, "cbz" | "cbnz") || (mnemonic.starts_with('b') && !matches!(mnemonic, "bx" | "blx" | "bic" | "bics" | "bkpt" | "bfc" | "bfi"))
}

/// An instruction text (`mnemonic operands`), from the source or from the
/// disassembly. `named` gives the symbol of a source operand `{name}`.
fn insn(text: &str, named: &dyn Fn(&str) -> Option<String>) -> Insn {
    let text = text.split('@').next().unwrap_or("").trim();
    let (m, rest) = text.split_once(|c: char| c.is_whitespace()).unwrap_or((text, ""));
    let mut mnemonic = m.to_ascii_lowercase();
    for suffix in [".w", ".n"] {
        if let Some(s) = mnemonic.strip_suffix(suffix) {
            mnemonic = s.to_string();
        }
    }
    let branch = takes_target(&mnemonic);
    let mut out = Insn { mnemonic, words: Vec::new(), imms: Vec::new(), target: None };
    // A named target: `{name}` in the source, `<symbol>` in the disassembly.
    let mut rest = rest.to_string();
    if branch {
        if let (Some(a), Some(b)) = (rest.find('<'), rest.find('>')) {
            let sym = rest[a + 1..b].to_string();
            if !sym.contains('+') {
                out.target = Some(sym);
            }
            rest.replace_range(a..=b, "");
        } else if let (Some(a), Some(b)) = (rest.rfind('{'), rest.rfind('}')) {
            out.target = named(rest[a + 1..b].trim());
            rest.replace_range(a..=b, "");
        }
    }
    let cleaned: String = rest.chars().map(|c| if matches!(c, '{' | '}' | '[' | ']' | '!' | ',') { ' ' } else { c }).collect();
    let tokens: Vec<&str> = cleaned.split_whitespace().collect();
    for (i, t) in tokens.iter().enumerate() {
        let t = t.to_ascii_lowercase();
        if branch && i + 1 == tokens.len() && (number(&t).is_some() || t.ends_with('f') || t.ends_with('b')) && !t.starts_with('r') {
            // The address or the local label of a branch.
            continue;
        }
        if let Some(n) = number(&t) {
            if n != 0 {
                out.imms.push(n);
            }
            continue;
        }
        if t == "sy" {
            continue;
        }
        for w in expand(&t) {
            if out.words.last() != Some(&w) {
                out.words.push(w);
            }
        }
    }
    out
}

/// The instructions of a stub's source lines: labels and directives are no
/// instructions.
pub fn source_insns(lines: &[String], named: &dyn Fn(&str) -> Option<String>) -> Vec<Insn> {
    lines
        .iter()
        .map(|l| l.trim())
        .filter(|l| !l.is_empty() && !l.starts_with('.') && !l.ends_with(':'))
        .flat_map(|l| l.split(';').map(str::trim).filter(|p| !p.is_empty() && !p.ends_with(':')).map(|p| insn(p, named)).collect::<Vec<_>>())
        .collect()
}

/// The instructions `llvm-objdump -d --no-show-raw-insn` printed at
/// addresses in `[start, end)`.
pub fn disassembled_insns(text: &str, start: u32, end: u32) -> Vec<Insn> {
    let mut out = Vec::new();
    for line in text.lines() {
        let Some((addr, rest)) = line.split_once(':') else { continue };
        let Ok(a) = u32::from_str_radix(addr.trim(), 16) else { continue };
        let rest = rest.trim();
        if a < start || a >= end || rest.is_empty() || rest.starts_with('<') {
            continue;
        }
        out.push(insn(rest, &|_| None));
    }
    out
}

/// `name = sym symbol` operands of the stubs file: every symbol a name is
/// given anywhere in it.
pub fn sym_operands(src: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for line in src.lines() {
        let t = line.trim().trim_end_matches(',');
        if let Some((name, rest)) = t.split_once('=') {
            if let Some(sym) = rest.trim().strip_prefix("sym ") {
                out.push((name.trim().to_string(), sym.trim().to_string()));
            }
        }
    }
    out
}

/// The differences between a stub's source instructions and its
/// disassembly; empty when they agree.
pub fn compare(stub: &str, source: &[Insn], binary: &[Insn], syms: &[(String, String)], raw_source: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    if source.len() != binary.len() {
        out.push(format!("`{stub}`: the source has {} instructions, the binary {}", source.len(), binary.len()));
    }
    // A source `{name}` may name several symbols in the file (one per stub
    // that uses the name): the binary's target has to be one of them.
    let names_of = |text: &str| -> Vec<String> {
        match (text.rfind('{'), text.rfind('}')) {
            (Some(a), Some(b)) if a < b => syms.iter().filter(|(n, _)| n == text[a + 1..b].trim()).map(|(_, s)| s.clone()).collect(),
            _ => Vec::new(),
        }
    };
    let raw: Vec<&String> = raw_source.iter().filter(|l| { let l = l.trim(); !l.is_empty() && !l.starts_with('.') && !l.ends_with(':') }).collect();
    for (i, (s, b)) in source.iter().zip(binary.iter()).enumerate() {
        let allowed = raw.get(i).map(|r| names_of(r)).unwrap_or_default();
        let target_ok = if allowed.is_empty() { true } else { b.target.as_ref().is_some_and(|t| allowed.contains(t)) };
        if s.mnemonic != b.mnemonic || s.words != b.words || s.imms != b.imms || !target_ok {
            out.push(format!("`{stub}` instruction {}: the source is {:?}, the binary {:?}{}", i + 1, s, b, if target_ok { String::new() } else { format!(" (the source names {allowed:?})") }));
        }
    }
    out
}

/// `llvm-objdump` of the toolchain in use, or of any installed one.
pub fn objdump() -> Option<std::path::PathBuf> {
    let mut roots = Vec::new();
    if let Ok(o) = Command::new("rustc").args(["--print", "sysroot"]).output() {
        roots.push(std::path::PathBuf::from(String::from_utf8_lossy(&o.stdout).trim().to_string()));
    }
    if let Some(home) = std::env::var_os("RUSTUP_HOME").map(std::path::PathBuf::from).or_else(|| std::env::var_os("HOME").map(|h| std::path::PathBuf::from(h).join(".rustup"))) {
        if let Ok(dir) = std::fs::read_dir(home.join("toolchains")) {
            let mut all: Vec<_> = dir.flatten().map(|e| e.path()).collect();
            all.sort();
            roots.extend(all);
        }
    }
    for root in roots {
        if let Ok(dir) = std::fs::read_dir(root.join("lib").join("rustlib")) {
            for host in dir.flatten() {
                let p = host.path().join("bin").join("llvm-objdump");
                if p.is_file() {
                    return Some(p);
                }
            }
        }
    }
    None
}

/// The prefix of an error that is the disassembler's absence or failure,
/// not a difference.
pub const TOOL: &str = "tool: ";

/// Checks one binary. `Ok` carries the stubs compared and the modelled
/// stubs the binary does not link; `Err` the differences.
pub fn check_binary(root: &Path, elf_path: &Path) -> Result<(Vec<String>, Vec<String>), Vec<String>> {
    let src = std::fs::read_to_string(root.join(STUBS)).map_err(|e| vec![format!("{STUBS}: {e}")])?;
    let elf = rsk_elf::Elf::read(elf_path).map_err(|e| vec![format!("{}: {e}", elf_path.display())])?;
    let symbols = elf.symbols().map_err(|e| vec![format!("{}: {e}", elf_path.display())])?;
    let tool = objdump().ok_or_else(|| vec![format!("{TOOL}llvm-objdump was not found in any installed toolchain (rustup component add llvm-tools)")])?;
    let syms = sym_operands(&src);
    let stubs = stubs_of(&src);
    let (mut compared, mut absent, mut diffs) = (Vec::new(), Vec::new(), Vec::new());
    let names: Vec<&str> = STUB_MODELS.iter().map(|(n, _)| *n).chain(NOT_MODELLED.iter().map(|(n, _)| *n)).collect();
    for name in names {
        let Some((_, lines)) = stubs.iter().find(|(n, _)| n == name) else {
            diffs.push(format!("`{name}` has no `naked_asm!` block in {STUBS}"));
            continue;
        };
        let Some(sym) = symbols.iter().find(|s| s.name == name && s.is_func()) else {
            absent.push(name.to_string());
            continue;
        };
        let (start, end) = (sym.address(), sym.address() + sym.size);
        let out = Command::new(&tool)
            .args(["-d", "--no-show-raw-insn", &format!("--start-address={start}"), &format!("--stop-address={end}")])
            .arg(elf_path)
            .output()
            .map_err(|e| vec![format!("{TOOL}{}: {e}", tool.display())])?;
        if !out.status.success() {
            return Err(vec![format!("{TOOL}{} failed on {}", tool.display(), elf_path.display())]);
        }
        let named = |n: &str| syms.iter().find(|(k, _)| k == n).map(|(_, s)| s.clone());
        let source = source_insns(lines, &named);
        let binary = disassembled_insns(&String::from_utf8_lossy(&out.stdout), start, end);
        let d = compare(name, &source, &binary, &syms, lines);
        if d.is_empty() {
            compared.push(format!("{name} ({} instructions)", source.len()));
        }
        diffs.extend(d);
    }
    // A binary that links none of the three stubs every Kernel has is not
    // a Kernel binary: nothing was checked.
    for required in ["rsk_irq_entry", "rsk_svc_entry", "rsk_unwind"] {
        if absent.iter().any(|a| a == required) {
            diffs.push(format!("`{required}` is not linked in {}", elf_path.display()));
        }
    }
    if diffs.is_empty() {
        Ok((compared, absent))
    } else {
        Err(diffs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn s(text: &str) -> Insn {
        insn(text, &|n| if n == "svc" { Some("rsk_svc_rust".to_string()) } else { None })
    }

    #[test]
    fn the_source_and_the_disassembly_of_an_instruction_compare_equal() {
        for (source, binary) in [
            ("tst lr, #4", "tst.w\tlr, #0x4"),
            ("sub sp, sp, #16", "sub\tsp, #0x10"),
            ("ldr r0, [sp, #0]", "ldr\tr0, [sp]"),
            ("ldr r12, [sp, #12]", "ldr.w\tr12, [sp, #0xc]"),
            ("pop {r3-r11, lr}", "pop.w\t{r3, r4, r5, r6, r7, r8, r9, r10, r11, lr}"),
            ("vpopeq {s16-s31}", "vpopeq\t{s16, s17, s18, s19, s20, s21, s22, s23, s24, s25, s26, s27, s28, s29, s30, s31}"),
            ("dsb", "dsb\tsy"),
            ("cmp r12, #0", "cmp.w\tr12, #0x0"),
            ("bl {svc}", "bl\t0xbc50 <rsk_svc_rust>   @ imm = #0xba82"),
            ("cbz r0, 1f", "cbz\tr0, 0x1d8 <rsk_svc_entry+0x20> @ imm = #0x2"),
            ("bne 2f", "bne\t0x220 <rsk_unwind+0x40> @ imm = #0x8"),
            ("ite eq", "ite\teq"),
            ("cpsid i", "cpsid i"),
        ] {
            assert_eq!(s(source), s(binary), "{source} / {binary}");
        }
    }

    #[test]
    fn a_different_instruction_register_immediate_or_target_is_a_difference() {
        for (source, binary) in [
            ("mov r1, lr", "mov\tlr, r1"),
            ("mov r1, lr", "mov\tr1, r2"),
            ("sub sp, sp, #16", "sub\tsp, #0x18"),
            ("cpsid i", "cpsie i"),
            ("cpsid f", "cpsid i"),
            ("bl {svc}", "bl\t0xbc50 <rsk_other>"),
            ("msr psp, r0", "msr\tmsp, r0"),
            ("pop {r3-r11, lr}", "pop.w\t{r3, r4, r5, r6, r7, r8, r9, r10, lr}"),
            ("dsb", "isb\tsy"),
        ] {
            assert_ne!(s(source), s(binary), "{source} / {binary}");
        }
        // A missing or an added instruction is a difference of the count.
        let src = vec![s("cpsid i"), s("bx lr")];
        let bin = vec![s("cpsid i")];
        assert!(!compare("x", &src, &bin, &[], &["cpsid i".to_string(), "bx lr".to_string()]).is_empty());
    }
}
