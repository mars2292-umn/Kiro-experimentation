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

/// The generated files of one application (one per declaration variant).
pub struct App {
    pub decl_crate: String,
    /// The variant (a cargo feature of the declaration crate), `None` for the default.
    pub variant: Option<String>,
    pub decl_source: PathBuf,
    pub out_dir: PathBuf,
    pub files: Vec<(String, String)>,
}

/// Runs the Generator on every declaration crate and each of its variants;
/// returns the applications (with their outputs) or records the diagnostics.
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
        let app_dir = pkg.dir.parent().map(Path::to_path_buf).unwrap_or_else(|| pkg.dir.clone());
        let mut variants: Vec<(Option<String>, PathBuf, PathBuf)> = vec![(None, pkg.dir.join("src").join("lib.rs"), app_dir.join("gen"))];
        for v in &policy.variants {
            variants.push((Some(v.clone()), pkg.dir.join("src").join(format!("{v}.rs")), app_dir.join(format!("gen-{v}"))));
        }
        for (variant, decl_source, out_dir) in variants {
            if let Some(app) = generate_one(ws, name, variant, decl_source, out_dir, report) {
                apps.push(app);
            }
        }
    }
    apps
}

fn generate_one(ws: &Workspace, name: &str, variant: Option<String>, decl_source: PathBuf, out_dir: PathBuf, report: &mut Report) -> Option<App> {
    let label = match &variant {
        Some(v) => format!("{name} (variant {v})"),
        None => name.to_string(),
    };
    let source = match std::fs::read_to_string(&decl_source) {
        Ok(s) => s,
        Err(e) => {
            report.error(Rule::Step, Some(ws.rel(&decl_source)), format!("cannot read the declaration: {e}"));
            return None;
        }
    };
    let parsed = match rsk_gen::syntax::parse_source(&source) {
        Ok(p) => p,
        Err(e) => {
            report.error(Rule::Generator, Some(ws.rel(&decl_source)), e);
            return None;
        }
    };
    let outputs = match rsk_gen::outputs(&parsed.decl, "rsk-system") {
        Ok(o) => o,
        Err(errors) => {
            for e in errors {
                report.error(Rule::Generator, Some(ws.rel(&decl_source)), e.to_string());
            }
            return None;
        }
    };
    // R25.1: the Task_Model is validated against the published schema
    // before it is written.
    if let Err(e) = rsk_model::read_task_model(&outputs.task_model_json) {
        report.error(Rule::Generator, Some(ws.rel(&decl_source)), format!("the Task_Model does not conform to its schema (R25.1): {e}"));
        return None;
    }
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
        "Generator: `{label}` accepted ({} Tasks, {} Resources, {} Partitions; Target {}; Generated_Config checksum 0x{:08x})",
        outputs.generated.tasks.len(),
        outputs.generated.resources.len(),
        outputs.generated.partitions.len(),
        outputs.generated.target.name,
        outputs.generated.checksum
    ));
    Some(App { decl_crate: name.to_string(), variant, decl_source, out_dir, files })
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
