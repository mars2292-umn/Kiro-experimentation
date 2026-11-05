//! The HIL builds and the HIL_Rig (tasks 3.1 and 7.1; Verification_Plan
//! section 11). A HIL build is the declaration variant `nrf52840` of an
//! Application with the `debug-port` and `quiet` features (the access
//! port stays open across resets; the Tasks' semihosting prints are
//! silent): the P1 demo for the target board and the stimulus Application
//! (`apps/hil-stimulus`, policy `hil-stimulus = true`) for the stimulus
//! board. Each is its own cargo invocation (rsk-entry binds one
//! Application per build) and never the Flight_Build binary; the
//! Build_Manifest records their hashes (`hil_binaries`), which the rig's
//! read-back check compares against (R48.4).
//!
//! The rig itself (`hil-rig.toml`, `rsk-hil`) selects every board by
//! serial. Without the file, a single attached probe is used, as before
//! the rig existed.

use std::path::{Path, PathBuf};

use rsk_hil::config::Rig;

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::manifest::Binary;
use crate::sandbox::Sandbox;
use crate::workspace::Workspace;

/// probe-rs's chip name for the DK's part (the rig file names it too).
pub const CHIP: &str = "nRF52840_xxAA";
/// The declaration variant and the features of a HIL build.
pub const VARIANT: &str = "nrf52840";
pub const FEATURES: &str = "nrf52840,debug-port,quiet,trace-ram,trace-pins";
/// The board's time base and CPU clock (the Target table), for the trace
/// and the release-timing record.
pub const TICK_HZ: u64 = 16_000_000;
pub const CPU_HZ: u64 = 64_000_000;

/// The measurement configurations of the P1 demo (tasks 7.3 and 7.5):
/// the declaration variant, the bin feature that selects it, the
/// operating duration in seconds, and whether it is the stress
/// configuration (the storm and the misbehaving Partitions). The job
/// builds every one so that the Build_Manifest lists their hashes and the
/// Kernel_Timing_Parameter records the rig measures on them are fresh for
/// the Analyzer (task 19.3, R33.4).
pub struct Measurement {
    pub variant: &'static str,
    pub feature: &'static str,
    pub duration_secs: u64,
    pub stress: bool,
}

pub const MEASUREMENTS: &[Measurement] = &[
    Measurement { variant: "nrf52840_timing_short", feature: "timing-short", duration_secs: 3 * 60, stress: false },
    Measurement { variant: "nrf52840_timing", feature: "timing", duration_secs: 61 * 60, stress: false },
    Measurement { variant: "nrf52840_stress_short", feature: "stress-short", duration_secs: 3 * 60, stress: true },
    Measurement { variant: "nrf52840_stress", feature: "stress", duration_secs: 61 * 60, stress: true },
];

/// The per-Task counters the P1 demo's safe state reports: all 0 when no
/// Task was released (the boot-check suite, task 7.2).
pub const COUNTERS: &[&str] = &["control", "sensor", "log", "pulse", "echo", "filter", "sampler"];

/// The margin `m` of R34.6 applied to a measured Kernel_Timing_Parameter,
/// in percent (Verification_Plan 11.5; ORQ-13).
pub const MARGIN_PERCENT: u64 = 25;

/// The committed Kernel_Timing_Parameter record of a measurement
/// configuration (`docs/hil/kernel_timing_<variant>.json`).
pub fn timing_record_path(root: &Path, variant: &str) -> PathBuf {
    root.join("docs").join("hil").join(format!("kernel_timing_{variant}.json"))
}

/// The committed WCET_Records of a measurement configuration
/// (`docs/hil/wcet_<variant>.json`, task 19.1).
pub fn wcet_record_path(root: &Path, variant: &str) -> PathBuf {
    root.join("docs").join("hil").join(format!("wcet_{variant}.json"))
}

/// The binary hashes a Build_Manifest lists (`binaries` and `hil_binaries`).
fn manifest_hashes(manifest: &serde_json::Value) -> Vec<String> {
    let mut hashes: Vec<String> = Vec::new();
    for key in ["binaries", "hil_binaries"] {
        for b in manifest.get(key).and_then(|b| b.as_array()).into_iter().flatten() {
            if let Some(h) = b.get("hash").and_then(|h| h.as_str()) {
                hashes.push(h.to_string());
            }
        }
    }
    hashes
}

/// The committed Kernel_Timing_Parameter records whose binary the
/// Build_Manifest lists (`binaries` or `hil_binaries`): fresh for the
/// Analyzer (R33.4). The second list names the stale ones with their
/// hashes.
pub fn fresh_timing_records(root: &Path, manifest: &serde_json::Value) -> (Vec<PathBuf>, Vec<String>) {
    let hashes = manifest_hashes(manifest);
    let mut fresh = Vec::new();
    let mut stale = Vec::new();
    for m in MEASUREMENTS {
        let path = timing_record_path(root, m.variant);
        let Ok(text) = std::fs::read_to_string(&path) else { continue };
        match rsk_model::read_kernel_timing(&text) {
            Ok(t) if hashes.contains(&t.binary_hash) => fresh.push(path),
            Ok(t) => stale.push(format!("{} ({}: not a binary of this manifest)", path.display(), t.binary_hash)),
            Err(e) => stale.push(format!("{} (not a Kernel_Timing_Parameter record: {e})", path.display())),
        }
    }
    (fresh, stale)
}

/// The committed WCET_Record set of `variant` when every record's binary
/// the Build_Manifest lists (task 19.1, R33.4), else why not.
pub fn fresh_wcet_records(root: &Path, manifest: &serde_json::Value, variant: &str) -> Result<Option<PathBuf>, String> {
    let hashes = manifest_hashes(manifest);
    let path = wcet_record_path(root, variant);
    let Ok(text) = std::fs::read_to_string(&path) else { return Ok(None) };
    match rsk_model::read_wcet_set(&text) {
        Ok(w) => match w.records.iter().find(|r| !hashes.contains(&r.binary_hash)) {
            None => Ok(Some(path)),
            Some(r) => Err(format!("{} ({}: not a binary of this manifest)", path.display(), r.binary_hash)),
        },
        Err(e) => Err(format!("{} (not a WCET_Record set: {e})", path.display())),
    }
}

/// The rig description at the workspace root, if there is one.
pub fn rig(root: &Path) -> Option<Result<Rig, String>> {
    let path = root.join(rsk_hil::config::FILE);
    path.is_file().then(|| Rig::load(&path))
}

/// The probe-rs installation and the probe it sees (the rig-less case).
pub struct Probe {
    pub version: String,
    pub probe: String,
}

/// Finds `probe-rs` and one attached debug probe: the rig's target by
/// serial when a rig is described, otherwise the only probe attached.
pub fn probe(rig: Option<&Rig>) -> Option<Probe> {
    let version = rsk_hil::probes::version().ok()?;
    let list = rsk_hil::probes::list().ok()?;
    let probe = match rig {
        Some(rig) => list.iter().find(|p| p.id == rig.target.serial)?,
        None if list.len() == 1 => &list[0],
        None => return None,
    };
    Some(Probe { version, probe: format!("{} ({})", probe.id, probe.name) })
}

/// Where `cargo xtask hil` builds (its own target directory: a different
/// feature set from the job's Flight_Build).
pub fn target_dir(root: &Path) -> PathBuf {
    root.join("target").join("xtask-hil")
}

/// One HIL build.
pub struct HilBuild {
    pub bin: String,
    pub elf: PathBuf,
    pub command: String,
    pub binary: Binary,
}

/// Builds the HIL variant of `bin` for the Target under `target_dir`.
pub fn build(ws: &Workspace, bin: &str, target_dir: &Path, sandbox: Option<&Sandbox>, report: &mut Report) -> Option<HilBuild> {
    build_with(ws, bin, FEATURES, target_dir, sandbox, report)
}

/// The HIL build of `bin` with `features` (the release-timing
/// configurations add `timing` or `timing-short`, task 7.3).
pub fn build_with(ws: &Workspace, bin: &str, features: &str, target_dir: &Path, sandbox: Option<&Sandbox>, report: &mut Report) -> Option<HilBuild> {
    let cmd = Cmd::cargo_frozen("build")
        .args(["--quiet", "-p", bin, "--release", "--target", &ws.policy.target, "--no-default-features", "--features", features, "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(target_dir);
    let command = cmd.display();
    match cmd.output(sandbox) {
        Ok(o) if o.status.success() => {}
        Ok(o) => {
            report.error(Rule::Step, None, format!("HIL build of `{bin}` failed:\n{}", String::from_utf8_lossy(&o.stderr)));
            return None;
        }
        Err(e) => {
            report.error(Rule::Step, None, format!("cannot run cargo: {e}"));
            return None;
        }
    }
    let elf = target_dir.join(&ws.policy.target).join("release").join(bin);
    if !link_check(ws, bin, features, &elf, report) {
        return None;
    }
    match Binary::of(bin, &format!("{bin} ({VARIANT}, {features})"), &elf) {
        Ok(binary) => Some(HilBuild { bin: bin.to_string(), elf, command, binary }),
        Err(e) => {
            report.error(Rule::Step, None, format!("HIL build of `{bin}`: {e}"));
            None
        }
    }
}

/// The Link_Checker (R37) on a HIL build: its no-init records (the trace
/// ring, the event mirror, the release-timing record) occupy Kernel RAM
/// beside the Kernel's data and bss, and the MSP must keep its worst case
/// above them (a 1,024-entry mirror beside the record of task 19.1 left it
/// 776 bytes and the Kernel crashed within a second, 2026-11-02). The
/// Task_Model is the declaration variant's (`gen-<variant>/`, the variant
/// named by the last feature of a measurement build, else `gen-nrf52840/`);
/// a binary without one (the stimulus Application) is not checked.
fn link_check(ws: &Workspace, bin: &str, features: &str, elf: &Path, report: &mut Report) -> bool {
    let Some(app_dir) = ws.packages.values().find(|p| p.name == bin).and_then(|p| p.dir.parent().map(Path::to_path_buf)) else { return true };
    let variant = features
        .split(',')
        .find_map(|f| MEASUREMENTS.iter().find(|m| m.feature == f).map(|m| m.variant.to_string()))
        .or_else(|| features.split(',').any(|f| f == "response").then(|| format!("{VARIANT}_response")))
        .or_else(|| features.split(',').any(|f| f == "isolation").then(|| format!("{VARIANT}_isolation")))
        .unwrap_or_else(|| VARIANT.to_string());
    let model = app_dir.join(format!("gen-{variant}")).join("task_model.json");
    let Ok(model_text) = std::fs::read_to_string(&model) else { return true };
    let Ok(bytes) = std::fs::read(elf) else { return true };
    match rsk_link::check(&bytes, &model_text) {
        Ok(result) => {
            for v in &result.violations {
                report.error(Rule::Step, None, format!("Link_Checker on the HIL build `{bin}` ({features}): [{}] {}", v.rule, v.message));
            }
            if let Some(k) = result.regions.iter().find(|r| r.name == "kernel") {
                report.note(format!("Link_Checker on the HIL build `{bin}` ({features}): Kernel MSP worst case {} of {} bytes available", k.bound, k.available));
            }
            result.violations.is_empty()
        }
        Err(e) => {
            report.error(Rule::Step, None, format!("Link_Checker on the HIL build `{bin}`: {e}"));
            false
        }
    }
}

/// The Task_Model of a declaration variant of `bin`'s Application
/// (`gen-<variant>/task_model.json`), when generated.
pub fn task_model(ws: &Workspace, bin: &str, variant: &str) -> Option<rsk_model::task_model::TaskModel> {
    let app_dir = ws.packages.values().find(|p| p.name == bin).and_then(|p| p.dir.parent().map(Path::to_path_buf))?;
    let text = std::fs::read_to_string(app_dir.join(format!("gen-{variant}")).join("task_model.json")).ok()?;
    rsk_model::read_task_model(&text).ok()
}

/// Every HIL build of the workspace: the QEMU spikes' Applications for the
/// target board and the stimulus binaries for the stimulus board.
pub fn build_all(ws: &Workspace, target_dir: &Path, sandbox: Option<&Sandbox>, report: &mut Report) -> Vec<HilBuild> {
    let bins: Vec<String> = ws.policy.qemu_spikes().chain(ws.policy.hil_stimulus()).map(str::to_string).collect();
    let mut builds: Vec<HilBuild> = bins.iter().filter_map(|bin| build(ws, bin, target_dir, sandbox, report)).collect();
    // The measurement configurations of the spike Applications (task
    // 19.3): built in their own directories, as `cargo xtask hil` builds
    // them, so that their hashes are those of the measured binaries.
    for bin in ws.policy.qemu_spikes() {
        for m in MEASUREMENTS {
            let features = format!("{FEATURES},{}", m.feature);
            if let Some(b) = build_with(ws, bin, &features, &target_dir.join(m.variant), sandbox, report) {
                builds.push(b);
            }
        }
    }
    builds
}

/// The outcome of one run on the board.
pub struct Run {
    pub ok: bool,
    pub seconds: f64,
    pub log: String,
    pub command: String,
}

/// Flashes and runs `elf` on the board `serial`; the console is the log.
pub fn run_elf(elf: &Path, serial: &str, timeout_secs: u64) -> Run {
    let r = rsk_hil::board::Target { chip: CHIP, serial }.run(elf, timeout_secs);
    Run { ok: r.ok, seconds: r.seconds, log: r.log, command: r.command }
}

/// The spike run for one built binary on the rig's target (or the only
/// probe): run, judge.
pub fn run_build(build: &HilBuild, serial: &str, timeout_secs: u64) -> Run {
    let mut run = run_elf(&build.elf, serial, timeout_secs);
    run.command = format!("{}; {}", build.command, run.command);
    run
}

/// The serial the spike runs on: the rig's target, or the only probe.
pub fn target_serial(rig: Option<&Rig>) -> Option<String> {
    match rig {
        Some(rig) => Some(rig.target.serial.clone()),
        None => {
            let list = rsk_hil::probes::list().ok()?;
            (list.len() == 1).then(|| list[0].id.clone())
        }
    }
}
