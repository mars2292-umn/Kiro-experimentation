//! `cargo xtask gen [--check]`: runs the Generator (`rsk-gen`) on every
//! declaration crate of the workspace (policy `declaration = true`) and
//! writes the Task_Model JSON, the linker script and memory map, the
//! configuration summary, and the manifests for review into the
//! application's `gen/` directory next to the declaration crate (R24.3,
//! R25.1). `--check` fails when any file on disk differs from the
//! Generator's output, which the verification job runs so that the
//! committed linker script and Task_Model are the ones of the declaration
//! (R24.4, R59.1).

use std::path::{Path, PathBuf};

use crate::diag::{Report, Rule};
use crate::workspace::Workspace;

/// The generated files of one application.
pub struct App {
    pub decl_crate: String,
    pub decl_source: PathBuf,
    pub out_dir: PathBuf,
    pub files: Vec<(String, String)>,
}

/// Runs the Generator on every declaration crate; returns the applications
/// (with their outputs) or records the diagnostics.
pub fn generate_all(ws: &Workspace, report: &mut Report) -> Vec<App> {
    let mut apps = Vec::new();
    for (name, policy) in &ws.policy.crates {
        if !policy.declaration {
            continue;
        }
        let Some(pkg) = ws.package_named(name) else {
            report.error(Rule::Step, None, format!("declaration crate `{name}` is not a workspace package"));
            continue;
        };
        let decl_source = pkg.dir.join("src").join("lib.rs");
        let out_dir = pkg.dir.parent().map(|p| p.join("gen")).unwrap_or_else(|| pkg.dir.join("gen"));
        let source = match std::fs::read_to_string(&decl_source) {
            Ok(s) => s,
            Err(e) => {
                report.error(Rule::Step, Some(ws.rel(&decl_source)), format!("cannot read the declaration: {e}"));
                continue;
            }
        };
        let parsed = match rsk_gen::syntax::parse_source(&source) {
            Ok(p) => p,
            Err(e) => {
                report.error(Rule::Generator, Some(ws.rel(&decl_source)), e);
                continue;
            }
        };
        let outputs = match rsk_gen::outputs(&parsed.decl, "rsk-system") {
            Ok(o) => o,
            Err(errors) => {
                for e in errors {
                    report.error(Rule::Generator, Some(ws.rel(&decl_source)), e.to_string());
                }
                continue;
            }
        };
        let mut files = vec![
            ("task_model.json".to_string(), outputs.task_model_json),
            ("memory.x".to_string(), outputs.memory_x),
            ("link.x".to_string(), outputs.link_x),
            ("summary.txt".to_string(), outputs.summary),
            ("system_manifest.rs".to_string(), outputs.system_source),
        ];
        for (pname, src) in outputs.partition_sources {
            files.push((format!("partition_{pname}.rs"), src));
        }
        report.note(format!(
            "Generator: `{name}` accepted ({} Tasks, {} Resources, {} Partitions; Generated_Config checksum 0x{:08x})",
            outputs.generated.tasks.len(),
            outputs.generated.resources.len(),
            outputs.generated.partitions.len(),
            outputs.generated.checksum
        ));
        apps.push(App {
            decl_crate: name.clone(),
            decl_source,
            out_dir,
            files,
        });
    }
    apps
}

/// Writes the outputs (or, with `check`, compares them with the files on
/// disk). Returns false on any error or drift.
pub fn run(root: &Path, check: bool, report: &mut Report) -> bool {
    let Some(ws) = Workspace::load(root, report) else { return false };
    let apps = generate_all(&ws, report);
    let mut ok = !report.has_rule(Rule::Generator) && !report.has_rule(Rule::Step);
    for app in &apps {
        for (name, content) in &app.files {
            let path = app.out_dir.join(name);
            if check {
                match std::fs::read_to_string(&path) {
                    Ok(existing) if existing == *content => {}
                    Ok(_) => {
                        report.error(
                            Rule::Generator,
                            Some(ws.rel(&path)),
                            format!("differs from the Generator's output for `{}`; run `cargo xtask gen` (R24.4)", app.decl_crate),
                        );
                        ok = false;
                    }
                    Err(_) => {
                        report.error(Rule::Generator, Some(ws.rel(&path)), "missing; run `cargo xtask gen`");
                        ok = false;
                    }
                }
            } else {
                if let Err(e) = std::fs::create_dir_all(&app.out_dir) {
                    report.error(Rule::Step, Some(ws.rel(&app.out_dir)), format!("cannot create: {e}"));
                    ok = false;
                    continue;
                }
                let unchanged = std::fs::read_to_string(&path).map(|s| s == *content).unwrap_or(false);
                if !unchanged {
                    if let Err(e) = std::fs::write(&path, content) {
                        report.error(Rule::Step, Some(ws.rel(&path)), format!("cannot write: {e}"));
                        ok = false;
                    } else {
                        report.note(format!("wrote {}", ws.rel(&path)));
                    }
                }
            }
        }
    }
    if apps.is_empty() {
        report.note("no declaration crate (policy `declaration = true`) in the workspace");
    } else if check && ok {
        report.note(format!("R24.4: the generated files of {} application(s) are up to date", apps.len()));
    }
    ok
}
