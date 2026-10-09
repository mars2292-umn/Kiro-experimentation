//! `rsk-analyzer`: the schedulability Analyzer CLI (tasks 15.1 to 16.2).
//!
//! ```text
//! rsk-analyzer --model task_model.json --wcet wcet.json --timing kernel_timing.json \
//!              --manifest build-manifest.json --out <dir> [--allow-placeholders] \
//!              [--prefer measured|static] [--uppaal [--uppaal-timeout <s>]]
//! ```
//!
//! Writes `report.json`, `report.txt`, `model.xml` and `model.q` into the
//! output directory. Exit status (R30.4): 0 PASS, 1 FAIL, 2 rejected
//! input, 3 internal error.
#![forbid(unsafe_code)]

use std::path::PathBuf;
use std::process::ExitCode;

use rsk_analyzer::{inputs::Options, EXIT_INTERNAL, EXIT_REJECTED};
use rsk_model::report::Inputs;

fn usage() -> ExitCode {
    eprintln!("usage: rsk-analyzer --model <task_model.json> --wcet <wcet.json> --timing <kernel_timing.json> --manifest <build-manifest.json> --out <dir> [--allow-placeholders] [--prefer measured|static] [--uppaal] [--uppaal-timeout <seconds>]");
    ExitCode::from(EXIT_INTERNAL)
}

fn read(path: &PathBuf) -> Result<String, String> {
    std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut model = None;
    let mut wcet = None;
    let mut timing = None;
    let mut manifest = None;
    let mut out = None;
    let mut options = Options::default();
    let mut run_uppaal = false;
    let mut uppaal_timeout = 300u64;
    let mut i = 0;
    while i < args.len() {
        let next = |i: &mut usize| -> Option<String> {
            *i += 1;
            args.get(*i).cloned()
        };
        match args[i].as_str() {
            "--model" => model = next(&mut i).map(PathBuf::from),
            "--wcet" => wcet = next(&mut i).map(PathBuf::from),
            "--timing" => timing = next(&mut i).map(PathBuf::from),
            "--manifest" => manifest = next(&mut i).map(PathBuf::from),
            "--out" => out = next(&mut i).map(PathBuf::from),
            "--allow-placeholders" => options.allow_placeholders = true,
            "--prefer" => options.prefer_method = next(&mut i),
            "--uppaal" => run_uppaal = true,
            "--uppaal-timeout" => uppaal_timeout = next(&mut i).and_then(|s| s.parse().ok()).unwrap_or(300),
            _ => return usage(),
        }
        i += 1;
    }
    let (Some(model_p), Some(wcet_p), Some(timing_p), Some(manifest_p), Some(out)) = (model, wcet, timing, manifest, out) else {
        return usage();
    };
    let texts = match (read(&model_p), read(&wcet_p), read(&timing_p), read(&manifest_p)) {
        (Ok(m), Ok(w), Ok(t), Ok(b)) => (m, w, t, b),
        (m, w, t, b) => {
            for e in [m.err(), w.err(), t.err(), b.err()].into_iter().flatten() {
                eprintln!("rsk-analyzer: {e}");
            }
            return ExitCode::from(EXIT_INTERNAL);
        }
    };
    // R28.1: schema validation of every input; each violation with its path.
    let mut rejected = Vec::new();
    let model = rsk_model::read_task_model(&texts.0).map_err(|e| rejected.push(format!("Task_Model: {e}"))).ok();
    let wcet = rsk_model::read_wcet_set(&texts.1).map_err(|e| rejected.push(format!("WCET_Records: {e}"))).ok();
    let timing = rsk_model::read_kernel_timing(&texts.2).map_err(|e| rejected.push(format!("Kernel_Timing_Parameters: {e}"))).ok();
    let manifest = rsk_model::read_build_manifest(&texts.3).map_err(|e| rejected.push(format!("Build_Manifest: {e}"))).ok();
    if !rejected.is_empty() {
        for r in &rejected {
            eprintln!("rsk-analyzer: rejected input: {r}");
        }
        return ExitCode::from(EXIT_REJECTED);
    }
    let (model, wcet, timing, manifest) = (model.unwrap(), wcet.unwrap(), timing.unwrap(), manifest.unwrap());
    let names = Inputs {
        task_model: format!("{} ({})", model_p.display(), sha(&texts.0)),
        wcet_records: format!("{} ({})", wcet_p.display(), sha(&texts.1)),
        kernel_timing: format!("{} ({})", timing_p.display(), sha(&texts.2)),
        build_manifest: format!("{} ({})", manifest_p.display(), sha(&texts.3)),
    };
    let mut analysis = rsk_analyzer::analyze(&model, &wcet, &timing, &manifest, names, &options);
    if let Err(e) = std::fs::create_dir_all(&out) {
        eprintln!("rsk-analyzer: cannot create {}: {e}", out.display());
        return ExitCode::from(EXIT_INTERNAL);
    }
    let xml_path = out.join("model.xml");
    let q_path = out.join("model.q");
    if !analysis.uppaal_xml.is_empty() {
        let _ = std::fs::write(&xml_path, &analysis.uppaal_xml);
        let _ = std::fs::write(&q_path, &analysis.uppaal_query);
        if run_uppaal {
            let cc = rsk_analyzer::uppaal::cross_check(&xml_path, &q_path, analysis.report.result == "PASS", uppaal_timeout);
            analysis.report.cross_check = rsk_model::report::CrossCheck {
                status: cc.status.to_string(),
                detail: cc.detail,
            };
        }
    }
    let json = rsk_analyzer::report::render_json(&analysis.report);
    let text = rsk_analyzer::report::render_text(&analysis.report);
    if std::fs::write(out.join("report.json"), &json).is_err() || std::fs::write(out.join("report.txt"), &text).is_err() {
        eprintln!("rsk-analyzer: cannot write the report into {}", out.display());
        return ExitCode::from(EXIT_INTERNAL);
    }
    print!("{text}");
    ExitCode::from(analysis.exit)
}

/// A short content hash for the report's input provenance (R30.2); the
/// Build_Manifest carries the binary hashes themselves.
fn sha(text: &str) -> String {
    // FNV-1a 64 over the text: a stable fingerprint without a hashing
    // dependency in the Analyzer (the evidence store hashes files itself).
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0100_0000_01b3);
    }
    format!("fnv1a:{h:016x}")
}
