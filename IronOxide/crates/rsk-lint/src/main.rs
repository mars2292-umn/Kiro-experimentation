//! `rsk-lint`: the Profile_Lint command line (R3.2, R3.4, R3.5).
//!
//! ```text
//! rsk-lint [--kernel <dir>]... [--unsafe <path>]... [--app <dir>]... [--gen <dir>]...
//!          --profile <profile.toml> --core-subset <core_subset.toml>
//!          [--root <workspace>] [--task-model <task_model.json>] [--out <loop_bounds.json>]
//! ```
//!
//! Exit status 0 when no violation was found, 1 when one was, 2 on a usage
//! or read error.
#![forbid(unsafe_code)]

use std::path::PathBuf;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut opts = rsk_lint::Options::default();
    let mut profile: Option<PathBuf> = None;
    let mut core_subset: Option<PathBuf> = None;
    let mut out: Option<PathBuf> = None;
    let mut task_model: Option<PathBuf> = None;
    while let Some(a) = args.next() {
        let value = |args: &mut std::iter::Skip<std::env::Args>| -> PathBuf {
            match args.next() {
                Some(v) => PathBuf::from(v),
                None => {
                    eprintln!("rsk-lint: `{a}` takes a path");
                    std::process::exit(2);
                }
            }
        };
        match a.as_str() {
            "--kernel" => opts.kernel_dirs.push(value(&mut args)),
            "--unsafe" => opts.unsafe_paths.push(value(&mut args)),
            "--app" => opts.app_dirs.push(value(&mut args)),
            "--gen" => opts.gen_dirs.push(value(&mut args)),
            "--file" => {
                let spec = value(&mut args).to_string_lossy().into_owned();
                let Some((scope, path)) = spec.split_once('=') else {
                    eprintln!("rsk-lint: `--file` takes `<scope>=<path>`");
                    std::process::exit(2);
                };
                let scope = match scope {
                    "kernel" => rsk_lint::Scope::Kernel,
                    "kernel-unsafe" => rsk_lint::Scope::KernelUnsafe,
                    "generated" => rsk_lint::Scope::Generated,
                    _ => rsk_lint::Scope::Application,
                };
                opts.files.push((scope, PathBuf::from(path)));
            }
            "--profile" => profile = Some(value(&mut args)),
            "--core-subset" => core_subset = Some(value(&mut args)),
            "--root" => opts.root = Some(value(&mut args)),
            "--task-model" => task_model = Some(value(&mut args)),
            "--out" => out = Some(value(&mut args)),
            other => {
                eprintln!("rsk-lint: unknown argument `{other}`");
                std::process::exit(2);
            }
        }
    }
    let (Some(profile), Some(core_subset)) = (profile, core_subset) else {
        eprintln!("usage: rsk-lint [--kernel <dir>]... [--unsafe <path>]... [--app <dir>]... [--gen <dir>]... --profile <profile.toml> --core-subset <core_subset.toml> [--file <scope>=<path>]... [--root <workspace>] [--task-model <json>] [--out <loop_bounds.json>]");
        std::process::exit(2);
    };
    let read = |p: &PathBuf| -> String {
        std::fs::read_to_string(p).unwrap_or_else(|e| {
            eprintln!("rsk-lint: cannot read {}: {e}", p.display());
            std::process::exit(2);
        })
    };
    opts.profile_toml = read(&profile);
    opts.core_subset_toml = read(&core_subset);
    if let Some(m) = &task_model {
        opts.task_model_json = Some(read(m));
    }
    let report = match rsk_lint::run(&opts) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rsk-lint: {e}");
            std::process::exit(2);
        }
    };
    for n in &report.notes {
        println!("note: {n}");
    }
    for f in &report.findings {
        println!("error[{}] {}:{}: {}", f.rule, f.file, f.line, f.message);
    }
    if let Some(out) = out {
        if let Err(e) = std::fs::write(&out, report.export()) {
            eprintln!("rsk-lint: cannot write {}: {e}", out.display());
            std::process::exit(2);
        }
    }
    println!("rsk-lint: {}", report.summary());
    std::process::exit(if report.findings.is_empty() { 0 } else { 1 });
}
