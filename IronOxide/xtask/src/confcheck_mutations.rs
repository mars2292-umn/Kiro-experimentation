//! The Config_Checker mutation suite (R26.7, task 13.2): the cases of
//! `docs/config_mutations.toml` applied to copies of a Task_Model and a
//! Flight_Build binary; each case passes only when `rsk-confcheck` exits
//! non-zero and its report names every expected item.

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::sandbox::Sandbox;

pub const REGISTER: &str = "docs/config_mutations.toml";

#[derive(Debug, Clone)]
pub enum Change {
    /// `find` (exactly once in the Task_Model text) replaced by `replace`.
    Model { find: String, replace: String },
    /// The byte at `offset` of `symbol` in the ELF xor-ed with `xor`.
    Binary { symbol: String, offset: u32, xor: u8 },
    /// The first hexadecimal digit of the `"<field>": "0x..."` string changed.
    ModelHexDigit { field: String },
}

/// The model text with the first hex digit after `"<field>": "0x` changed.
pub fn alter_hex_digit(model_text: &str, field: &str) -> Result<String, String> {
    let key = format!("\"{field}\": \"0x");
    let at = model_text.find(&key).ok_or_else(|| format!("no `{key}` in the Task_Model"))?;
    let pos = at + key.len();
    let c = model_text[pos..].chars().next().ok_or("checksum string ends early")?;
    let replacement = match c {
        '0'..='8' => char::from(c as u8 + 1),
        '9' => 'a',
        'a'..='e' => char::from(c as u8 + 1),
        _ => '0',
    };
    let mut out = String::with_capacity(model_text.len());
    out.push_str(&model_text[..pos]);
    out.push(replacement);
    out.push_str(&model_text[pos + c.len_utf8()..]);
    Ok(out)
}

#[derive(Debug, Clone)]
pub struct Mutation {
    pub id: String,
    pub class: String,
    pub change: Change,
    pub expect: Vec<String>,
}

pub fn load(root: &Path) -> Result<Vec<Mutation>, String> {
    let path = root.join(REGISTER);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{REGISTER}: {e}"))?;
    let doc: Table = text.parse().map_err(|e| format!("{REGISTER}: {e}"))?;
    let list = doc.get("mutation").and_then(Value::as_array).ok_or(format!("{REGISTER}: no [[mutation]] entries"))?;
    let mut out = Vec::new();
    for (i, m) in list.iter().enumerate() {
        let s = |k: &str| m.get(k).and_then(Value::as_str).map(str::to_string).ok_or(format!("{REGISTER}: mutation {i}: missing `{k}`"));
        let id = s("id")?;
        let kind = s("kind")?;
        let change = match kind.as_str() {
            "model" => Change::Model { find: s("find")?, replace: s("replace")? },
            "model-hex-digit" => Change::ModelHexDigit { field: s("field")? },
            "binary" => Change::Binary {
                symbol: s("symbol")?,
                offset: m.get("offset").and_then(Value::as_integer).ok_or(format!("{REGISTER}: `{id}`: missing `offset`"))? as u32,
                xor: m.get("xor").and_then(Value::as_integer).ok_or(format!("{REGISTER}: `{id}`: missing `xor`"))? as u8,
            },
            other => return Err(format!("{REGISTER}: `{id}`: unknown kind `{other}`")),
        };
        let expect = m
            .get("expect")
            .and_then(Value::as_array)
            .ok_or(format!("{REGISTER}: `{id}`: missing `expect`"))?
            .iter()
            .filter_map(|v| v.as_str().map(str::to_string))
            .collect::<Vec<_>>();
        if expect.is_empty() {
            return Err(format!("{REGISTER}: `{id}`: `expect` is empty"));
        }
        out.push(Mutation { id, class: s("class")?, change, expect });
    }
    Ok(out)
}

/// Checks the register against the Task_Model text: every model `find`
/// occurs exactly once (the register is current).
pub fn check_against(model_text: &str, mutations: &[Mutation]) -> Vec<String> {
    let mut problems = Vec::new();
    for m in mutations {
        match &m.change {
            Change::Model { find, .. } => {
                let n = model_text.matches(find.as_str()).count();
                if n != 1 {
                    problems.push(format!("mutation `{}`: `find` occurs {n} times in the Task_Model (must be exactly once)", m.id));
                }
            }
            Change::ModelHexDigit { field } => {
                if let Err(e) = alter_hex_digit(model_text, field) {
                    problems.push(format!("mutation `{}`: {e}", m.id));
                }
            }
            Change::Binary { .. } => {}
        }
    }
    problems
}

/// The ELF with the mutation's byte altered.
pub fn patch_elf(elf_bytes: &[u8], symbol: &str, offset: u32, xor: u8) -> Result<Vec<u8>, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| e.to_string())?;
    let sym = elf
        .symbols()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|s| s.name == symbol && s.is_object())
        .ok_or_else(|| format!("no symbol `{symbol}` in the binary"))?;
    if offset >= sym.size {
        return Err(format!("offset {offset} is beyond `{symbol}` ({} bytes)", sym.size));
    }
    let addr = sym.address() + offset;
    let section = elf.section_at(addr).ok_or_else(|| format!("`{symbol}` is in no section"))?;
    let file_offset = (section.offset + (addr - section.addr)) as usize;
    let mut out = elf_bytes.to_vec();
    let b = out.get_mut(file_offset).ok_or("file offset beyond the ELF")?;
    *b ^= xor;
    Ok(out)
}

/// Runs every mutation of the register against `model` and `elf` (copies
/// in `work_dir`), with `rsk-confcheck` run through `confcheck` (a Cmd
/// template without the two file arguments). Returns whether every case
/// passed; notes and errors go to the report.
pub fn run(root: &Path, model: &Path, elf: &Path, work_dir: &Path, sandbox: Option<&Sandbox>, report: &mut Report) -> Result<bool, String> {
    let mutations = load(root)?;
    let model_text = std::fs::read_to_string(model).map_err(|e| format!("{}: {e}", model.display()))?;
    let elf_bytes = std::fs::read(elf).map_err(|e| format!("{}: {e}", elf.display()))?;
    let problems = check_against(&model_text, &mutations);
    if !problems.is_empty() {
        for p in problems {
            report.error(Rule::Step, Some(REGISTER.to_string()), p);
        }
        return Ok(false);
    }
    std::fs::create_dir_all(work_dir).map_err(|e| format!("{}: {e}", work_dir.display()))?;
    let mut all_ok = true;
    for m in &mutations {
        let (model_path, elf_path): (PathBuf, PathBuf) = match &m.change {
            Change::Model { find, replace } => {
                let altered = model_text.replacen(find.as_str(), replace, 1);
                let p = work_dir.join(format!("{}.json", m.id));
                std::fs::write(&p, altered).map_err(|e| e.to_string())?;
                (p, elf.to_path_buf())
            }
            Change::ModelHexDigit { field } => {
                let altered = alter_hex_digit(&model_text, field)?;
                let p = work_dir.join(format!("{}.json", m.id));
                std::fs::write(&p, altered).map_err(|e| e.to_string())?;
                (p, elf.to_path_buf())
            }
            Change::Binary { symbol, offset, xor } => {
                let altered = patch_elf(&elf_bytes, symbol, *offset, *xor)?;
                let p = work_dir.join(format!("{}.elf", m.id));
                std::fs::write(&p, altered).map_err(|e| e.to_string())?;
                (model.to_path_buf(), p)
            }
        };
        let cmd = Cmd::cargo_frozen("run")
            .args(["--quiet", "-p", "rsk-confcheck", "--manifest-path"])
            .arg(root.join("Cargo.toml"))
            .arg("--")
            .arg(&model_path)
            .arg(&elf_path);
        let (exit_ok, text) = match cmd.output(sandbox) {
            Ok(o) => (o.status.success(), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
            Err(e) => (true, format!("cannot run rsk-confcheck: {e}")),
        };
        let missing: Vec<&String> = m.expect.iter().filter(|e| !text.contains(e.as_str())).collect();
        if !exit_ok && missing.is_empty() && text.contains("error[R26.3]") {
            report.note(format!("R26.7: mutation `{}` ({}) reported: {}", m.id, m.class, m.expect.join("; ")));
        } else {
            all_ok = false;
            let why = if exit_ok { "the Config_Checker accepted the altered input".to_string() } else { format!("report lacks {:?}", missing) };
            report.error(Rule::Step, Some(REGISTER.to_string()), format!("mutation `{}` ({}): {why}:\n{text}", m.id, m.class));
        }
    }
    Ok(all_ok)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The register's model mutations apply to the committed demo
    /// Task_Model, and every R26.2 value class has a case (R26.7).
    #[test]
    fn the_register_matches_the_demo_task_model_and_covers_every_class() {
        let root = crate::repository_root();
        let mutations = load(&root).unwrap();
        let model = std::fs::read_to_string(root.join("apps/p1-demo/gen/task_model.json")).unwrap();
        let problems = check_against(&model, &mutations);
        assert!(problems.is_empty(), "{problems:?}");
        for class in ["ceilings", "priority-map", "nesting-depths", "capacities", "budget-and-mit-tables", "kernel-owned-peripherals", "mpu-permissions", "checksum"] {
            assert!(mutations.iter().any(|m| m.class == class), "no mutation for the value class `{class}`");
        }
        assert!(mutations.iter().any(|m| matches!(m.change, Change::Binary { .. })), "no altered-binary case");
    }
}
