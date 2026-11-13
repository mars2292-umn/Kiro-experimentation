//! The stub model follows the assembly (Verification_Plan 5.17).
//!
//! `crates/rsk-kernel/src/arch/cm4/stub_model.rs` states each assembly
//! stub of `unsafe_module/stubs.rs` as a sequence of instruction functions
//! (`let s = i_...(s, ...);`) and control lines (`// ctl: ...`), and Verus
//! proves the stubs' theorems about those sequences. This check ties the
//! two texts: for every stub, the instructions of its `naked_asm!` block,
//! in source order, are exactly the instruction functions and control
//! lines of its model functions, in order. An assembly line of a form the
//! table below does not know, a stub with no model, and any difference are
//! errors, so the assembly cannot change without the proof being redone.
//!
//! What the check does not see: the meaning of each instruction function
//! (its definition in the model, reviewed against the Armv7-M ARM), and
//! which model function follows which on a branch (the structure of the
//! theorems, reviewed against the control lines).

use std::path::Path;

use crate::diag::{Report, Rule};

pub const STUBS: &str = "crates/rsk-kernel/src/arch/cm4/unsafe_module/stubs.rs";
pub const MODEL: &str = "crates/rsk-kernel/src/arch/cm4/stub_model.rs";

/// Each modelled stub and the model functions that state it, in the order
/// of the assembly.
pub const STUB_MODELS: [(&str, &[&str]); 7] = [
    ("rsk_irq_entry", &["irq_to_call", "irq_after_call", "irq_dispatch", "irq_restore", "irq_return", "irq_to_unwind"]),
    ("rsk_svc_entry", &["svc_to_call", "kentry_after_call", "kentry_return"]),
    ("rsk_unwind", &["unwind_entry", "unwind_to_call", "unwind_after_call", "unwind_exit", "unwind_again"]),
    ("rsk_timer_entry", &["timer_to_call", "kentry_after_call", "kentry_return"]),
    ("rsk_memmanage_entry", &["mm_to_call", "kentry_after_call", "kentry_return"]),
    ("rsk_busfault_entry", &["fault_to_call", "kentry_after_call", "kentry_return"]),
    ("rsk_usagefault_entry", &["fault_to_call", "kentry_after_call", "kentry_return"]),
];

/// Naked functions that are not modelled, with the reason.
pub const NOT_MODELLED: [(&str, &str); 1] =
    [("job_end_trampoline", "one `svc` instruction executed by a Job; it calls no Rust half and touches no stack")];

pub fn check(root: &Path, report: &mut Report) {
    let (Ok(stubs), Ok(model)) = (std::fs::read_to_string(root.join(STUBS)), std::fs::read_to_string(root.join(MODEL))) else {
        report.error(Rule::Proof, Some(MODEL.to_string()), "the stubs or their model are missing");
        return;
    };
    let diffs = compare(&stubs, &model);
    for d in &diffs {
        report.error(Rule::Proof, Some(MODEL.to_string()), d.clone());
    }
    if diffs.is_empty() {
        report.note(format!("R44.6: the stub model matches the assembly of {} stubs line by line ({STUBS})", STUB_MODELS.len()));
    }
}

/// A number as decimal text (`0x10` and `16` are the same operand).
fn number(t: &str) -> Option<String> {
    let t = t.trim();
    let v = match t.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).ok()?,
        None => t.parse::<u64>().ok()?,
    };
    Some(v.to_string())
}

/// `rN` as `N`.
fn reg(t: &str) -> Option<String> {
    let n: u32 = t.trim().strip_prefix('r')?.parse().ok()?;
    (n <= 12).then(|| n.to_string())
}

/// `#N` as decimal text.
fn imm(t: &str) -> Option<String> {
    number(t.trim().strip_prefix('#')?)
}

/// `[sp, #N]` as decimal text.
fn sp_off(t: &str) -> Option<String> {
    let inner = t.trim().strip_prefix('[')?.strip_suffix(']')?;
    let (base, off) = inner.split_once(',')?;
    (base.trim() == "sp").then(|| imm(off)).flatten()
}

/// One assembly line as the model's text for it: an instruction function
/// call, a control line, or nothing (an `it` prefix, whose condition the
/// conditional forms carry, and a directive). `Err` for an unknown form.
pub fn asm_to_model(line: &str) -> Result<Option<String>, String> {
    let l = line.trim();
    let unknown = || Err(format!("assembly line `{l}` has no instruction function in the model"));
    if l.starts_with('.') || l == "it eq" || l == "ite eq" || l == "ite ne" {
        return Ok(None);
    }
    let control = l.ends_with(':') || l.starts_with("bl ") || l.starts_with("bx ") || l.starts_with("cbz ") || l.starts_with("bne ") || l.starts_with("b ");
    if control {
        return Ok(Some(format!("ctl: {l}")));
    }
    let fixed = [
        ("cpsid i", "i_cpsid_i(s)"),
        ("cpsie i", "i_cpsie_i(s)"),
        ("cpsid f", "i_cpsid_f(s)"),
        ("dsb", "i_dsb(s)"),
        ("isb", "i_isb(s)"),
        ("push {r3-r11, lr}", "i_push_r3_r11_lr(s)"),
        ("pop {r3-r11, lr}", "i_pop_r3_r11_lr(s)"),
        ("push {r4, lr}", "i_push_r4_lr(s)"),
        ("pop {r4, lr}", "i_pop_r4_lr(s)"),
        ("vpusheq {s16-s31}", "i_vpush_eq(s)"),
        ("vpopeq {s16-s31}", "i_vpop_eq(s)"),
    ];
    if let Some((_, m)) = fixed.iter().find(|(a, _)| *a == l) {
        return Ok(Some(m.to_string()));
    }
    let Some((op, rest)) = l.split_once(' ') else { return unknown() };
    let args: Vec<&str> = split_args(rest);
    let out = match (op, args.as_slice()) {
        ("tst", ["lr", m]) => imm(m).map(|m| format!("i_tst_lr(s, {m})")),
        ("tst", [r, m]) => reg(r).zip(imm(m)).map(|(r, m)| format!("i_tst_r(s, {r}, {m})")),
        ("mrs", [d, "ipsr"]) => reg(d).map(|d| format!("i_mrs_ipsr(s, {d})")),
        ("mrs", [d, "psp"]) => reg(d).map(|d| format!("i_mrs_psp(s, {d})")),
        ("mrs", [d, "control"]) => reg(d).map(|d| format!("i_mrs_control(s, {d})")),
        ("mrs", [d, "primask"]) => reg(d).map(|d| format!("i_mrs_primask(s, {d})")),
        ("mrseq", [d, "msp"]) => reg(d).map(|d| format!("i_mrs_msp_eq(s, {d})")),
        ("mrsne", [d, "psp"]) => reg(d).map(|d| format!("i_mrs_psp_ne(s, {d})")),
        ("moveq", [d, v]) => reg(d).zip(imm(v)).map(|(d, v)| format!("i_mov_imm_eq(s, {d}, {v})")),
        ("addeq", [d, "sp", o]) => reg(d).zip(imm(o)).map(|(d, o)| format!("i_add_r_sp_eq(s, {d}, {o})")),
        ("mov", [d, "lr"]) => reg(d).map(|d| format!("i_mov_r_lr(s, {d})")),
        ("mov", [d, "sp"]) => reg(d).map(|d| format!("i_mov_r_sp(s, {d})")),
        ("mov", [d, r]) => reg(d).zip(reg(r)).map(|(d, r)| format!("i_mov(s, {d}, {r})")),
        ("sub", ["sp", "sp", n]) => imm(n).map(|n| format!("i_sub_sp(s, {n})")),
        ("add", ["sp", "sp", n]) => imm(n).map(|n| format!("i_add_sp(s, {n})")),
        ("add", [d, "sp", o]) => reg(d).zip(imm(o)).map(|(d, o)| format!("i_add_r_sp(s, {d}, {o})")),
        ("str", [r, m]) => reg(r).zip(sp_off(m)).map(|(r, o)| format!("i_str_sp(s, {r}, {o})")),
        ("ldr", [d, m]) => reg(d).zip(sp_off(m)).map(|(d, o)| format!("i_ldr_sp(s, {d}, {o})")),
        ("msr", ["psp", r]) => reg(r).map(|r| format!("i_msr_psp(s, {r})")),
        ("msr", ["control", r]) => reg(r).map(|r| format!("i_msr_control(s, {r})")),
        ("cmp", [r, v]) => reg(r).zip(imm(v)).map(|(r, v)| format!("i_cmp_imm(s, {r}, {v})")),
        _ => None,
    };
    match out {
        Some(m) => Ok(Some(m)),
        None => unknown(),
    }
}

/// Operands split at the commas outside brackets.
fn split_args(rest: &str) -> Vec<&str> {
    let mut out = Vec::new();
    let (mut depth, mut start) = (0, 0);
    for (i, c) in rest.char_indices() {
        match c {
            '[' | '{' => depth += 1,
            ']' | '}' => depth -= 1,
            ',' if depth == 0 => {
                out.push(rest[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    out.push(rest[start..].trim());
    out
}

/// The condition an instruction carries, for the `it` blocks.
fn condition(l: &str) -> Option<&'static str> {
    let op = l.split(' ').next().unwrap_or("");
    if ["vpusheq", "vpopeq", "mrseq", "moveq", "addeq"].contains(&op) {
        Some("eq")
    } else if op == "mrsne" {
        Some("ne")
    } else {
        None
    }
}

/// The string literals of a `naked_asm!` block starting at `from`, with the
/// macro variables `$($name,)*` replaced by `vars`, and the offset after it.
fn asm_lines(src: &str, from: usize, vars: &[(String, Vec<String>)]) -> (Vec<String>, usize) {
    let mut out = Vec::new();
    let Some(open) = src[from..].find("naked_asm!(") else { return (out, src.len()) };
    let body_start = from + open + "naked_asm!(".len();
    let mut end = src.len();
    let mut offset = body_start;
    for line in src[body_start..].split_inclusive('\n') {
        let t = line.trim();
        if t == ")" || t.starts_with(")") {
            end = offset + line.len();
            break;
        }
        if let Some(v) = t.strip_prefix("$($").and_then(|r| r.strip_suffix(",)*")) {
            if let Some((_, lines)) = vars.iter().find(|(n, _)| n == v) {
                out.extend(lines.iter().cloned());
            }
        } else {
            // Every string literal of the line (a one-line block has several).
            let mut rest = t;
            while let Some(a) = rest.find('"') {
                let Some(b) = rest[a + 1..].find('"') else { break };
                out.push(rest[a + 1..a + 1 + b].replace("{{", "{").replace("}}", "}"));
                rest = &rest[a + 1 + b + 1..];
            }
            if t.ends_with(')') && !t.starts_with('"') == false && t.contains("naked_asm") {
                end = offset + line.len();
                break;
            }
        }
        offset += line.len();
    }
    (out, end)
}

/// The variables of the first `irq_entry_stub!(...)` instantiation (the one
/// of the Flight_Build, without the witness features).
fn stub_macro_vars(src: &str) -> Vec<(String, Vec<String>)> {
    let mut out = Vec::new();
    let Some(line) = src.lines().find(|l| l.trim_start().starts_with("irq_entry_stub!(fence:")) else { return out };
    for name in ["fence", "entry", "window"] {
        let key = format!("{name}: [");
        if let Some(a) = line.find(&key) {
            let rest = &line[a + key.len()..];
            let list = &rest[..rest.find(']').unwrap_or(0)];
            let lines = list.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, s)| s.to_string()).collect();
            out.push((name.to_string(), lines));
        }
    }
    out
}

/// Every naked function of the stubs file with its assembly lines.
pub fn stubs_of(src: &str) -> Vec<(String, Vec<String>)> {
    let vars = stub_macro_vars(src);
    let mut out = Vec::new();
    let mut at = 0;
    while let Some(i) = src[at..].find("pub unsafe extern \"C\" fn ") {
        let name_start = at + i + "pub unsafe extern \"C\" fn ".len();
        let name_end = name_start + src[name_start..].find('(').unwrap_or(0);
        let name = src[name_start..name_end].to_string();
        // A one-line block: every literal is on the `naked_asm!` line.
        let line_end = name_end + src[name_end..].find("naked_asm!(").unwrap_or(0);
        let one_line = src[line_end..].lines().next().unwrap_or("");
        if one_line.trim_end().ends_with(')') {
            let lits = one_line.split('"').enumerate().filter(|(i, _)| i % 2 == 1).map(|(_, s)| s.to_string()).collect();
            out.push((name, lits));
            at = line_end + one_line.len();
            continue;
        }
        let (lines, end) = asm_lines(src, name_end, &vars);
        out.push((name, lines));
        at = end;
    }
    out
}

/// The instruction functions and control lines of the model function
/// `name`, in order.
pub fn model_lines(model: &str, name: &str) -> Option<Vec<String>> {
    let head = format!("pub open spec fn {name}(s: M) -> M {{");
    let start = model.find(&head)? + head.len();
    let mut out = Vec::new();
    for line in model[start..].lines() {
        let t = line.trim();
        if t == "}" {
            return Some(out);
        }
        if let Some(c) = t.strip_prefix("// ctl: ") {
            out.push(format!("ctl: {c}"));
        } else if let Some(op) = t.strip_prefix("let s = ").and_then(|r| r.strip_suffix(';')) {
            out.push(normalize_call(op));
        } else if !(t.is_empty() || t == "s") {
            out.push(format!("unreadable model line `{t}`"));
        }
    }
    None
}

/// A model call with its numeric arguments as decimal text.
fn normalize_call(op: &str) -> String {
    let Some((f, rest)) = op.split_once('(') else { return op.to_string() };
    let args = rest.trim_end_matches(')');
    let args: Vec<String> = args.split(',').map(|a| number(a).unwrap_or_else(|| a.trim().to_string())).collect();
    format!("{f}({})", args.join(", "))
}

/// The differences between the assembly and the model: each a message.
pub fn compare(stubs: &str, model: &str) -> Vec<String> {
    let mut out = Vec::new();
    let found = stubs_of(stubs);
    for (name, _) in &found {
        if !STUB_MODELS.iter().any(|(n, _)| n == name) && !NOT_MODELLED.iter().any(|(n, _)| n == name) {
            out.push(format!("the naked function `{name}` has no model and is not listed as not modelled"));
        }
    }
    for (name, functions) in STUB_MODELS {
        let Some((_, asm)) = found.iter().find(|(n, _)| n == name) else {
            out.push(format!("the stub `{name}` is modelled and not in the assembly"));
            continue;
        };
        // The `it` blocks: each conditional instruction follows its prefix.
        for (i, l) in asm.iter().enumerate() {
            let expect: &[&str] = match l.trim() {
                "it eq" => &["eq"],
                "ite eq" => &["eq", "ne"],
                "ite ne" => &["ne", "eq"],
                _ => &[],
            };
            for (k, c) in expect.iter().enumerate() {
                if asm.get(i + 1 + k).and_then(|n| condition(n.trim())) != Some(c) {
                    out.push(format!("`{name}`: the instruction {} after `{}` is not conditional on `{c}`", k + 1, l.trim()));
                }
            }
            if condition(l.trim()).is_some() {
                let covered = (1..=2).any(|back| {
                    i >= back && matches!((asm[i - back].trim(), back), ("it eq", 1) | ("ite eq", 1) | ("ite ne", 1) | ("ite eq", 2) | ("ite ne", 2))
                });
                if !covered {
                    out.push(format!("`{name}`: the conditional instruction `{}` is in no `it` block", l.trim()));
                }
            }
        }
        let mut from_asm = Vec::new();
        for l in asm {
            match asm_to_model(l) {
                Ok(Some(m)) => from_asm.push((m, l.clone())),
                Ok(None) => {}
                Err(e) => out.push(format!("`{name}`: {e}")),
            }
        }
        let mut from_model = Vec::new();
        for f in functions {
            match model_lines(model, f) {
                Some(lines) => from_model.extend(lines),
                None => out.push(format!("`{name}`: the model function `{f}` is missing")),
            }
        }
        let n = from_asm.len().max(from_model.len());
        for i in 0..n {
            match (from_asm.get(i), from_model.get(i)) {
                (Some((a, l)), Some(m)) if a == m => {
                    let _ = l;
                }
                (Some((a, l)), Some(m)) => {
                    out.push(format!("`{name}`, instruction {}: the assembly has `{l}` (`{a}`), the model `{m}`", i + 1));
                    break;
                }
                (Some((_, l)), None) => {
                    out.push(format!("`{name}`: the assembly continues with `{l}` after the model's {} lines", from_model.len()));
                    break;
                }
                (None, Some(m)) => {
                    out.push(format!("`{name}`: the model continues with `{m}` after the assembly's {} instructions", from_asm.len()));
                    break;
                }
                (None, None) => {}
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tree() -> (String, String) {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap().to_path_buf();
        (std::fs::read_to_string(root.join(STUBS)).unwrap(), std::fs::read_to_string(root.join(MODEL)).unwrap())
    }

    #[test]
    fn the_model_of_the_tree_matches_its_assembly() {
        let (stubs, model) = tree();
        assert_eq!(compare(&stubs, &model), Vec::<String>::new());
        // Every stub was read with its instructions, the macro's fence among them.
        let found = stubs_of(&stubs);
        let irq = &found.iter().find(|(n, _)| n == "rsk_irq_entry").unwrap().1;
        assert_eq!(irq.iter().filter(|l| *l == "cpsid f").count(), 2);
        assert!(irq.len() > 35);
        assert_eq!(found.len(), 8);
    }

    #[test]
    fn a_changed_assembly_line_is_reported_with_its_stub() {
        let (stubs, model) = tree();
        let changed = stubs.replace("\"ldr r12, [sp, #12]\",", "\"ldr r12, [sp, #8]\",");
        let d = compare(&changed, &model);
        assert_eq!(d.len(), 1, "{d:?}");
        assert!(d[0].contains("rsk_unwind") && d[0].contains("ldr r12, [sp, #8]") && d[0].contains("i_ldr_sp(s, 12, 12)"), "{d:?}");
    }

    #[test]
    fn a_removed_or_swapped_line_and_an_unknown_instruction_are_reported() {
        let (stubs, model) = tree();
        let removed = stubs.replace("        \"cpsid f\",\n        \"cpsie i\",\n        \"bx r2\",", "        \"cpsie i\",\n        \"bx r2\",");
        assert!(compare(&removed, &model).iter().any(|d| d.contains("rsk_unwind")));
        let swapped = stubs.replace("        \"vpopeq {{s16-s31}}\",\n        \"pop {{r3-r11, lr}}\",\n        \"msr psp, r0\",", "        \"pop {{r3-r11, lr}}\",\n        \"vpopeq {{s16-s31}}\",\n        \"msr psp, r0\",");
        assert!(compare(&swapped, &model).iter().any(|d| d.contains("rsk_unwind")));
        let unknown = stubs.replace("        \"mov r1, sp\",", "        \"mov r1, sp\",\n        \"nop\",");
        assert!(compare(&unknown, &model).iter().any(|d| d.contains("`nop` has no instruction function")));
    }

    #[test]
    fn a_changed_model_line_and_an_unmodelled_stub_are_reported() {
        let (stubs, model) = tree();
        let changed = model.replace("    let s = i_mov_r_lr(s, 8);\n", "");
        assert!(compare(&stubs, &changed).iter().any(|d| d.contains("rsk_irq_entry")));
        let extra = format!("{stubs}\n#[unsafe(naked)]\npub unsafe extern \"C\" fn rsk_new_entry() {{\n    naked_asm!(\n        \"dsb\",\n    )\n}}\n");
        assert!(compare(&extra, &model).iter().any(|d| d.contains("rsk_new_entry` has no model")));
    }

    #[test]
    fn operands_are_compared_by_value_and_it_blocks_by_condition() {
        assert_eq!(asm_to_model("tst lr, #0x10").unwrap().unwrap(), "i_tst_lr(s, 16)");
        assert_eq!(normalize_call("i_tst_lr(s, 0x10)"), "i_tst_lr(s, 16)");
        assert_eq!(asm_to_model("str r4, [sp, #0]").unwrap().unwrap(), "i_str_sp(s, 4, 0)");
        assert_eq!(asm_to_model("it eq").unwrap(), None);
        assert!(asm_to_model("ldr r0, [r1, #8]").is_err());
        let (stubs, model) = tree();
        let broken = stubs.replace("                \"it eq\",\n                \"vpusheq {{s16-s31}}\",", "                \"vpusheq {{s16-s31}}\",");
        assert!(compare(&broken, &model).iter().any(|d| d.contains("in no `it` block")));
    }
}
