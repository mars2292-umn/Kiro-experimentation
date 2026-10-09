//! `rsk-gen` CLI (task 10.6): runs the Generator on a declaration crate's
//! source and writes the Task_Model JSON (R25), the linker script and
//! memory map, the MPU layout report, the configuration summary (R24.3),
//! and, for review, the manifests the `rsk::app!` macro expands to. Output
//! is byte-identical for identical inputs (R24.4); `--check` compares the
//! files on disk with what the Generator produces and fails on any
//! difference (the Build_System's drift check).
//!
//! ```text
//! rsk-gen --decl apps/p1-demo/decl/src/lib.rs --system-crate rsk-system --out apps/p1-demo/gen [--check]
//! ```
#![forbid(unsafe_code)]

use std::path::{Path, PathBuf};
use std::process::ExitCode;

fn usage() -> ExitCode {
    eprintln!("usage: rsk-gen --decl <declaration.rs> --system-crate <name> --out <dir> [--check]");
    ExitCode::from(2)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut decl: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut system_crate = "rsk-system".to_string();
    let mut check = false;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--decl" => {
                i += 1;
                decl = args.get(i).map(PathBuf::from);
            }
            "--out" => {
                i += 1;
                out = args.get(i).map(PathBuf::from);
            }
            "--system-crate" => {
                i += 1;
                system_crate = args.get(i).cloned().unwrap_or_default();
            }
            "--check" => check = true,
            _ => return usage(),
        }
        i += 1;
    }
    let (Some(decl), Some(out)) = (decl, out) else { return usage() };
    let source = match std::fs::read_to_string(&decl) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("rsk-gen: cannot read {}: {e}", decl.display());
            return ExitCode::from(2);
        }
    };
    let parsed = match rsk_gen::syntax::parse_source(&source) {
        Ok(p) => p,
        Err(e) => {
            eprintln!("rsk-gen: {e}");
            return ExitCode::from(1);
        }
    };
    let outputs = match rsk_gen::outputs(&parsed.decl, &system_crate) {
        Ok(o) => o,
        Err(errors) => {
            for e in &errors {
                eprintln!("rsk-gen: {e}");
            }
            eprintln!("rsk-gen: {} violation(s) in {}", errors.len(), decl.display());
            return ExitCode::from(1);
        }
    };
    let mut files: Vec<(String, String)> = vec![
        ("task_model.json".to_string(), outputs.task_model_json.clone()),
        ("memory.x".to_string(), outputs.memory_x.clone()),
        ("link.x".to_string(), outputs.link_x.clone()),
        ("summary.txt".to_string(), outputs.summary.clone()),
        ("system_manifest.rs".to_string(), outputs.system_source.clone()),
    ];
    for (name, src) in &outputs.partition_sources {
        files.push((format!("partition_{name}.rs"), src.clone()));
    }
    if check {
        let mut drift = 0;
        for (name, content) in &files {
            let path = out.join(name);
            match std::fs::read_to_string(&path) {
                Ok(existing) if existing == *content => {}
                Ok(_) => {
                    eprintln!("rsk-gen: {} differs from the Generator's output (run rsk-gen without --check)", path.display());
                    drift += 1;
                }
                Err(_) => {
                    eprintln!("rsk-gen: {} is missing", path.display());
                    drift += 1;
                }
            }
        }
        if drift > 0 {
            return ExitCode::from(1);
        }
        println!("rsk-gen: {} generated files are up to date in {}", files.len(), out.display());
        return ExitCode::SUCCESS;
    }
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("rsk-gen: cannot create {}: {e}", out.display());
        return ExitCode::from(2);
    }
    for (name, content) in &files {
        if let Err(e) = write_if_changed(&out.join(name), content) {
            eprintln!("rsk-gen: {e}");
            return ExitCode::from(2);
        }
    }
    println!("rsk-gen: wrote {} files to {}", files.len(), out.display());
    ExitCode::SUCCESS
}

fn write_if_changed(path: &Path, content: &str) -> Result<(), String> {
    if std::fs::read_to_string(path).map(|s| s == content).unwrap_or(false) {
        return Ok(());
    }
    std::fs::write(path, content).map_err(|e| format!("cannot write {}: {e}", path.display()))
}
