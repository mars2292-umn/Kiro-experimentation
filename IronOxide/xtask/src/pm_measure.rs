//! `cargo xtask pm-measure` (docs/pacemaker/tasks.md, task 9.2; PM27.4,
//! PD-O3): the PACEMAKER Application's own Kernel_Timing_Parameters and
//! WCET_Records, measured on the rig, and the Analyzer on them.
//!
//! `--set <vvi-90|aai-90>` builds the pulse generator's measurement build
//! (the declaration variant `nrf52840_measure`: the board variant with an
//! Operating_Duration of 300 s; the Kernel's record of feature `trace-ram`;
//! no trace pins, which share P1.11 with the ventricular pace pin), flashes
//! the rig's target board and lets it run its Operating_Duration against
//! the Heart_Simulator already running on the stimulus board, under an
//! attached `probe-rs run` (which keeps the cycle counter enabled). The Kernel's
//! record is read once the system is in its safe state: a debug-probe
//! access halts the core, and before the end it would have put its own
//! delay into the lags it reads. The run's records are written under
//! `docs/pacemaker/rig/`, bound to the build's hash.
//!
//! `--analyze` runs the Analyzer on the board variant's Task_Model with
//! the records of the sets measured so far: per WCET item the record with
//! the larger maximum, per Kernel_Timing_Parameter the larger bound, every
//! other parameter the Analyzer's placeholder. The Build_Manifest is the
//! verification job's with the pulse generator's Flight_Build as the
//! analysed binary and the measurement builds as its HIL builds.
//!
//! Single-chamber Modes (stage E1) exercise one sense Task each, so two
//! sets are needed for the four Tasks.

use std::path::{Path, PathBuf};

use rsk_hil::kernel_timing::{bound, Binding};
use rsk_hil::release_timing::Hold;
use rsk_model::wcet::{KernelTimingParameter, Measured, WcetRecord, WcetSet, KERNEL_TIMING_SCHEMA, WCET_SCHEMA};
use rsk_model::KernelTiming;

use crate::diag::{Report, Rule};
use crate::workspace::Workspace;

pub const BIN: &str = "pacemaker";
pub const SETS: &[&str] = &["vvi-90", "aai-90", "ddd-90", "doo-mit"];

/// The sets that are not cargo features of the Application: the text of
/// the parameter set, given to the build in `PM_SET` (feature `set-env`).
/// `ddd-90` (task 13.1): DDD at 90 ppm, for the dual-chamber paths of the
/// step. `doo-mit` (task 13.3): DOO at 175 ppm with the shortest AV delay
/// (70 ms), the longest atrial and the shortest ventricular pulse: the
/// closest two pulse ends a programmable set gives, for `MIT_PACE`.
pub fn env_text(set: &str) -> Option<&'static str> {
    match set {
        "ddd-90" => Some("0=1,1=90"),
        "doo-mit" => Some("0=4,1=175,4=70,10=1900,11=50"),
        _ => None,
    }
}

/// The feature that selects `set`, with `PM_SET` put in this process's
/// environment (the builds inherit it) for a set given as text.
fn set_feature(set: &str) -> String {
    match env_text(set) {
        Some(text) => {
            std::env::set_var("PM_SET", text);
            "set-env".to_string()
        }
        None => format!("set-{set}"),
    }
}
/// The Operating_Duration of the declaration variant `nrf52840_measure`.
pub const DURATION_SECS: u64 = 300;
/// The wait beyond it before the record is read.
// 8 s until 2026-11-24: with `comms` in the binary the flashing took over
// 10 s of the run's time, the probe was detached 2 s before the
// Operating_Duration ended, and the halt was recorded as a deadline miss.
pub const GRACE_SECS: u64 = 25;
pub const TOOL_TIMING: &str = "xtask pm-measure: the Kernel's release-timing and entry record";
pub const TOOL_WCET: &str = "xtask pm-measure: the Kernel's Budget accounting";
const INSTRUMENTED: &str = "HIL build (trace-ram): the instrumentation's cost is included (R34.7)";

/// What a run is for.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    /// The records: Kernel_Timing_Parameters and WCET_Records (task 9.2).
    Records,
    /// The fault paths (task 9.8): `control` overruns its Budget and writes
    /// outside its Partition at stated instants (feature `inject`), so
    /// that the Kernel's fault, enforcement and restart entries run; only
    /// those entries' parameters are taken from the run.
    Inject,
    /// The coverage build (task 9.9): the branch coverage of each Task
    /// entry under the run's scenario; no timing record.
    Coverage,
}

pub fn features(set: &str) -> String {
    features_of(set, Kind::Records)
}

pub fn features_of(set: &str, kind: Kind) -> String {
    // The coverage build has its own declaration variant (a larger code
    // region for the instrumented Partition), with the same duration.
    let more = match kind {
        Kind::Records => "measure",
        Kind::Inject => "measure,inject",
        Kind::Coverage => "coverage",
    };
    format!("nrf52840,debug-port,quiet,trace-ram,{more},{}", set_feature(set))
}

/// The library crates whose functions are instantiated in the Partition
/// (they have no code of their own, findings B5), instrumented with it.
pub const LIBRARY_CRATES: &[&str] = &["pm_core", "pm_hw"];
/// The parameters an injection run is taken for.
pub const FAULT_PARAMETERS: &[&str] = &["delta_hm", "delta_restart", "delta_stop", "delta_enforce"];

pub fn inject_path(root: &Path, set: &str) -> PathBuf {
    rig_dir(root).join(format!("task_9_8_kernel_timing_inject_{}.json", set.replace('-', "_")))
}

pub fn coverage_path(root: &Path, set: &str) -> PathBuf {
    rig_dir(root).join(format!("task_9_9_coverage_{}.json", set.replace('-', "_")))
}

pub fn rig_dir(root: &Path) -> PathBuf {
    root.join("docs").join("pacemaker").join("rig")
}

pub fn timing_path(root: &Path, set: &str) -> PathBuf {
    rig_dir(root).join(format!("task_9_2_kernel_timing_{}.json", set.replace('-', "_")))
}

pub fn wcet_path(root: &Path, set: &str) -> PathBuf {
    rig_dir(root).join(format!("task_9_2_wcet_{}.json", set.replace('-', "_")))
}

fn fail(report: &mut Report, message: String) -> bool {
    report.error(Rule::Step, None, message);
    false
}

/// One measurement run of `set`. `scenario` names what the stimulus board
/// plays (it is not flashed here).
pub fn measure(ws: &Workspace, rig: &rsk_hil::config::Rig, set: &str, scenario: &str, kind: Kind, report: &mut Report) -> bool {
    let features = features_of(set, kind);
    let target_dir = ws.root.join("target").join(match kind {
        Kind::Records => format!("pacemaker-measure-{set}"),
        Kind::Inject => format!("pacemaker-inject-{set}"),
        Kind::Coverage => format!("pacemaker-coverage-{set}"),
    });
    let tools = if kind == Kind::Coverage {
        match crate::hil::coverage_tools(report) {
            Some(t) => Some(t),
            None => return false,
        }
    } else {
        None
    };
    let build = match kind {
        Kind::Coverage => crate::hil::coverage_build_with(ws, BIN, &features, &target_dir, false, LIBRARY_CRATES, report),
        _ => crate::hil::build_with(ws, BIN, &features, &target_dir, None, report),
    };
    let Some(build) = build else { return false };
    let Some(model) = crate::hil::task_model(ws, BIN, "nrf52840") else {
        return fail(report, "no Task_Model of the pulse generator's board variant: run `cargo xtask gen`".into());
    };
    let target = rsk_hil::board::Target { chip: &rig.chip, serial: &rig.target.serial };
    let scratch = target_dir.join("probe-read.bin");
    // Flashed and run under an attached `probe-rs run`, as rsk's rig runs
    // its configurations: a `probe-rs reset` halts the core some 57 ms into
    // the run (a deadline miss and a restart of the Partition) and, as it
    // detaches, clears DEMCR.TRCENA, after which the cycle counter stands
    // still and every later execution is recorded as 0 cycles (found on the
    // first run of this command, 2026-11-23). The console is silent (feature
    // `quiet`), so the run ends by the timeout, in the safe state.
    report.note(format!("`{BIN}` ({features}, {}): running {} s under the probe", build.binary.hash, DURATION_SECS + GRACE_SECS));
    let run = target.run_with(&build.elf, DURATION_SECS + GRACE_SECS, &[]);
    if !run.timed_out {
        return fail(report, format!("the run ended before its Operating_Duration ({:.1} s):\n{}", run.seconds, run.log));
    }
    let st = match target.read_stats(&build.elf, &scratch) {
        Ok(Some(st)) => st,
        Ok(None) => return fail(report, "the build has no release-timing record".into()),
        Err(e) => return fail(report, e),
    };
    let hold = match target.read_words(&build.elf, "ARCH_HOLD", &scratch) {
        Ok(Some(w)) if w.len() >= 10 && w[0] == 0x5253_4b41 => Some(Hold { max_cycles: w[1], count: w[2], kind: w[5] as u8, entries: w[6], first: w[7], last: w[8], by_kind: w[10..].to_vec() }),
        _ => None,
    };
    let events = target.read_events(&build.elf, &scratch).ok().flatten();
    let cpu_hz = crate::hil::CPU_HZ;
    let tick_hz = crate::hil::TICK_HZ;
    let margin = crate::hil::MARGIN_PERCENT;
    let scenario = format!("nrf52840_measure, set {set}, Heart_Simulator `{scenario}` ({DURATION_SECS} s)");
    let target_name = format!("{}/{}", model.target, model.core_revision);
    let binding = Binding { binary_hash: &build.binary.hash, target: &target_name, icache: "off", configuration: "nrf52840_measure" };

    if let Some(tools) = &tools {
        return coverage(ws, rig, set, &scenario, &build, &target, &target_dir, tools, &model, &st, report);
    }

    // The run's report: what the Kernel recorded.
    let us = |c: u64| c as f64 * 1e6 / cpu_hz as f64;
    let title = if kind == Kind::Inject { "Task 9.8: injection run" } else { "Task 9.2: measurement run" };
    let mut text = format!(
        "# {title}, set {set}\n\nBinary `{BIN}` ({features}): {}, Generated_Config checksum {}.\nScenario: {scenario}.\n\n",
        build.binary.hash,
        build.binary.config_checksum.as_deref().unwrap_or("?")
    );
    text.push_str("| Task | Jobs | Execution max (cycles / µs) | Budget (µs) | Above Budget | Response max (ms) | Deadline (ms) | Intakes | Intake max (cycles) | Release lag max (cycles) |\n|---|---|---|---|---|---|---|---|---|---|\n");
    let mut ok = true;
    for (i, ts) in st.tasks.iter().enumerate() {
        let Some(task) = model.tasks.get(i) else { continue };
        let resp_ms = ts.resp_max_ticks as f64 * 1000.0 / tick_hz as f64;
        text.push_str(&format!(
            "| `{}` | {} | {} / {:.1} | {:.0} | {} | {:.3} | {:.3} | {} | {} | {} |\n",
            task.id,
            ts.exec_count,
            ts.exec_max_cycles,
            us(ts.exec_max_cycles as u64),
            us(task.budget_cycles as u64),
            ts.exec_over_count,
            resp_ms,
            task.deadline_ticks as f64 * 1000.0 / tick_hz as f64,
            ts.intake_count,
            ts.intake_max_cycles,
            ts.max_cycles
        ));
        // A Job above its Budget, or Jobs recorded with no execution time
        // (the cycle counter did not run): not a record.
        if (kind == Kind::Records && ts.exec_over_count > 0) || (ts.exec_count > 0 && ts.exec_max_cycles == 0) {
            ok = false;
        }
    }
    text.push_str("\nKernel entries (Rust halves):\n\n| Entry | Runs | Max (cycles / µs) |\n|---|---|---|\n");
    for (k, e) in st.entries.iter().enumerate() {
        if e.count > 0 {
            text.push_str(&format!("| {} | {} | {} / {:.1} |\n", rsk_hil::stats::ENTRY_NAMES.get(k).copied().unwrap_or("?"), e.count, e.max_cycles, us(e.max_cycles as u64)));
        }
    }
    if let Some(h) = &hold {
        text.push_str(&format!("\nHold (L_kernel): {} cycles ({:.1} µs) over {} holds.\n", h.max_cycles, us(h.max_cycles as u64), h.count));
    }
    match &events {
        Some(ev) => {
            text.push_str(&format!("\nEvents: {}\n", ev.summary(tick_hz)));
            // Anything but the end of the Operating_Duration, the safe state and
            // the attached debugger is a finding.
            if kind == Kind::Records && ev.counts().iter().any(|((kind, _), _)| !["operating_end", "safe_state", "debugger"].contains(&rsk_hil::events::kind_name(*kind))) {
                ok = false;
            }
            // The Kernel's trace stopped at the first such event: its last
            // entries are what led to it (added 2026-11-24, task 17.2).
            if ev.frozen_in_run().is_some() {
                if let Ok(Some(trace)) = target.read_trace(&build.elf, &scratch) {
                    let from = trace.len().saturating_sub(48);
                    text.push_str(&format!("\nThe trace's last {} entries before it froze:\n\n```\n{}```\n", trace.len() - from, rsk_hil::trace::render(&trace[from..], tick_hz)));
                }
            }
        }
        None => text.push_str("\nEvents: the event mirror could not be read.\n"),
    }

    if kind == Kind::Inject {
        // The run is for the fault paths: each of the three entries must have run.
        for k in [9usize, 11, 13] {
            if st.entries.get(k).map_or(0, |e| e.count) == 0 {
                text.push_str(&format!("
No `{}` entry ran.
", rsk_hil::stats::ENTRY_NAMES[k]));
                ok = false;
            }
        }
    }

    // Kernel_Timing_Parameters: J_rel from every periodic Release, Δ_irq
    // from the intakes of the three sporadic Tasks' sources, the entry
    // table's parameters and the hold.
    let mut parameters = Vec::new();
    let measured = |runs: u64, max: u64| Some(Measured { runs, max_observed: max, margin_percent: margin, coverage_branch_percent: 0, scenarios: vec![scenario.clone()] });
    if st.total() > 0 {
        let max = st.max_cycles() as u64;
        parameters.push(KernelTimingParameter {
            symbol: "j_rel".into(),
            cycles: bound(max, margin),
            method: "measured".into(),
            tool: TOOL_TIMING.into(),
            conditions: vec!["the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)".into(), INSTRUMENTED.into()],
            measured: measured(st.total(), max),
        });
    }
    let intakes: Vec<&rsk_hil::stats::TaskStats> = st.tasks.iter().filter(|t| t.intake_count > 0).collect();
    if let Some(max) = intakes.iter().map(|t| t.intake_max_cycles as u64).max() {
        parameters.push(KernelTimingParameter {
            symbol: "delta_irq".into(),
            cycles: bound(max, margin),
            method: "measured".into(),
            tool: TOOL_TIMING.into(),
            conditions: vec!["one run of the intake handler for a sporadic Task's source (TIMER3, TIMER2 or TIMER1), the Kernel's own record; no interrupt storm (R19.3 is not measured here)".into(), INSTRUMENTED.into()],
            measured: measured(intakes.iter().map(|t| t.intake_count as u64).sum(), max),
        });
    }
    // Δ_stamp and Δ_wake: rsk's stamp probe of HIL builds latches GPIOTE's
    // IN[0] event in the time base, and in this Application that event is
    // the atrial sense edge, whose count releases `sense_a`. The lag is
    // recorded at the first intake after each latched edge.
    let cycles_per_tick = cpu_hz / tick_hz;
    // Only `sense_a`'s record is that lag: the Kernel keeps one per Task, and
    // another Task's first intake after an atrial edge is as far from the edge
    // as its own event happens to be (0.56 s for `pace` in the first run).
    let stamped: Vec<&rsk_hil::stats::TaskStats> = st.tasks.iter().enumerate().filter(|(i, t)| t.stamp_count > 0 && model.tasks.get(*i).is_some_and(|m| m.id.ends_with(".sense_a"))).map(|(_, t)| t).collect();
    if let Some(max_ticks) = stamped.iter().map(|t| t.stamp_max_ticks as u64).max() {
        let max = max_ticks * cycles_per_tick;
        parameters.push(KernelTimingParameter {
            symbol: "delta_stamp".into(),
            cycles: bound(max, margin),
            method: "measured".into(),
            tool: TOOL_TIMING.into(),
            conditions: vec![
                "from the atrial sense edge, latched by the time base through PPI (GPIOTE IN[0] to TIMER0 CAPTURE[3], the stamp probe of HIL builds), to the timestamp of the intake of `sense_a`'s source that follows it: the edge counter's compare, the masking of any Kernel entry in progress, the exception entry and the handler's read of the time base. Measured on the atrial sense source only; the other two sources (TIMER2, TIMER3) have no probe and take the same handler".into(),
                INSTRUMENTED.into(),
            ],
            measured: measured(stamped.iter().map(|t| t.stamp_count as u64).sum(), max),
        });
    }
    let woken: Vec<&rsk_hil::stats::TaskStats> = stamped.iter().copied().filter(|t| t.wake_count > 0).collect();
    if let Some(mut p) = rsk_hil::kernel_timing::wake_parameter(&woken, cycles_per_tick, margin, &scenario) {
        p.conditions = vec!["the same lag over the atrial sense edges whose intake found no Job executing (the processor in the idle activity's WFI): the wake-up from sleep with the exception entry and the handler's read after it".into(), INSTRUMENTED.into()];
        parameters.push(p);
    }
    parameters.extend(rsk_hil::kernel_timing::entry_parameters(&st, &scenario, margin, None, hold.as_ref()));
    if kind == Kind::Inject {
        // Only the fault paths' parameters: the rest of an injection run
        // (overrunning Jobs, restarts) is not the Application's timing.
        parameters.retain(|p| FAULT_PARAMETERS.contains(&p.symbol.as_str()));
    }
    for p in &mut parameters {
        p.tool = TOOL_TIMING.into();
    }
    let timing = KernelTiming { schema: KERNEL_TIMING_SCHEMA.into(), binary_hash: binding.binary_hash.to_string(), target: target_name.clone(), clock_hz: cpu_hz, icache: "off".into(), parameters };

    // WCET_Records: each Task entry from the Budget accounting. The
    // Application has no Critical_Section site in its Task_Model beyond
    // what the model names; a site the Kernel recorded is written as rsk's
    // rig writes it.
    let mut records = Vec::new();
    let make = |item: String, max: u32, runs: u64, what: String| {
        let mut conditions = vec![what, INSTRUMENTED.to_string(), format!("cycle-counter read cost {} cycles calibrated at instant 0 (R34.3)", st.calibration_cycles)];
        if runs < rsk_hil::wcet_records::PAR_02_RUNS {
            conditions.push(format!("{runs} executions, below PAR-02's {}", rsk_hil::wcet_records::PAR_02_RUNS));
        }
        conditions.push("branch coverage not measured (R34.4)".to_string());
        WcetRecord {
            item,
            wcet_cycles: bound(max as u64, margin),
            method: "measured".into(),
            tool: TOOL_WCET.into(),
            binary_hash: binding.binary_hash.to_string(),
            target: target_name.clone(),
            clock_hz: cpu_hz,
            icache: "off".into(),
            conditions,
            measured: measured(runs, max as u64),
        }
    };
    for (i, ts) in st.tasks.iter().enumerate() {
        let Some(task) = model.tasks.get(i) else { continue };
        if ts.exec_count > 0 {
            let mut r = make(task.id.clone(), ts.exec_max_cycles, ts.exec_count as u64, "the Task entry: one Job's execution from the Budget accounting (R18.1)".to_string());
            r.conditions.push(format!("Kernel entries that interrupted the longest Job, their Rust halves inside its execution: {}", ts.exec_max_interruptions()));
            records.push(r);
        }
    }
    for c in &st.cs {
        let (Some(task), Some(res)) = (model.tasks.get(c.task as usize), model.resources.get(c.resource as usize)) else { continue };
        let site = format!("{}#cs:{}", task.id, res.id.rsplit('.').next().unwrap_or(&res.id));
        if task.cs_sites.contains(&site) {
            records.push(make(site, c.max_cycles, c.count as u64, "the Critical_Section: execution from the lock's return to the unlock's entry, from the Budget accounting".to_string()));
        }
    }
    let wcet = WcetSet { schema: WCET_SCHEMA.into(), records };

    let dir = rig_dir(&ws.root);
    let _ = std::fs::create_dir_all(&dir);
    if kind == Kind::Inject {
        let run_path = dir.join(format!("task_9_8_run_inject_{}.md", set.replace('-', "_")));
        text.push_str(&format!("\nVerdict of the run: **{}** (a fault, an enforcement and a restart entry each ran, and the cycle counter ran).\n", if ok { "pass" } else { "FAIL" }));
        for (path, body) in [(run_path.clone(), text.clone()), (inject_path(&ws.root, set), rsk_model::print(&timing))] {
            if let Err(e) = std::fs::write(&path, body) {
                return fail(report, format!("cannot write {}: {e}", ws.rel(&path)));
            }
        }
        print!("{text}");
        report.note(format!("records written: {}, {}", ws.rel(&run_path), ws.rel(&inject_path(&ws.root, set))));
        return ok || fail(report, "the injection run did not exercise the fault paths (report above)".into());
    }
    let run_path = dir.join(format!("task_9_2_run_{}.md", set.replace('-', "_")));
    text.push_str(&format!("\nVerdict of the run: **{}** (no Job above its Budget, no event but the end of the Operating_Duration and the attached debugger).\n", if ok { "pass" } else { "FAIL" }));
    for (path, body) in [(run_path.clone(), text.clone()), (timing_path(&ws.root, set), rsk_model::print(&timing)), (wcet_path(&ws.root, set), rsk_model::print(&wcet))] {
        if let Err(e) = std::fs::write(&path, body) {
            return fail(report, format!("cannot write {}: {e}", ws.rel(&path)));
        }
    }
    print!("{text}");
    report.note(format!("records written: {}, {}, {}", ws.rel(&run_path), ws.rel(&timing_path(&ws.root, set)), ws.rel(&wcet_path(&ws.root, set))));
    if !ok {
        return fail(report, "the measurement run recorded an overrun or an unexpected event (report above)".into());
    }
    true
}

/// The Analyzer on the board variant's Task_Model with the records of the
/// sets measured. Returns whether the Analyzer ran and its result was read;
/// the result itself is in the note and the files.
pub fn analyze(ws: &Workspace, report: &mut Report) -> bool {
    let app_dir = ws.root.join("apps").join(BIN);
    let model_path = app_dir.join("gen-nrf52840").join("task_model.json");
    let job_manifest = ws.root.join("target").join("xtask-verify").join("evidence").join("build-manifest.json");
    let Some(mut manifest) = std::fs::read_to_string(&job_manifest).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
        return fail(report, format!("no Build_Manifest at {} (the verification job writes it; its toolchain and build settings are taken from it)", ws.rel(&job_manifest)));
    };
    // The analysed binary: the pulse generator's Flight_Build.
    let flight_dir = ws.root.join("target").join("pacemaker-flight");
    let Some(flight) = crate::hil::build_with(ws, BIN, crate::hil::FEATURES_FLIGHT, &flight_dir, None, report) else { return false };
    let entry = |b: &crate::manifest::Binary| {
        serde_json::json!({
            "package": b.package, "target": b.target_name, "file": BIN, "hash": b.hash,
            "loadable_bytes": b.loadable_bytes, "config_checksum": b.config_checksum,
        })
    };
    manifest["binaries"] = serde_json::json!([entry(&flight.binary)]);
    manifest["generated_config_checksum"] = serde_json::json!(flight.binary.config_checksum);
    let mut hil = Vec::new();
    let mut timings = Vec::new();
    let mut best: Vec<WcetRecord> = Vec::new();
    for set in SETS {
        let (tp, wp) = (timing_path(&ws.root, set), wcet_path(&ws.root, set));
        let (Some(t), Some(w)) = (
            std::fs::read_to_string(&tp).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok()),
            std::fs::read_to_string(&wp).ok().and_then(|t| rsk_model::read_wcet_set(&t).ok()),
        ) else {
            report.note(format!("set {set}: no records (run `cargo xtask pm-measure --set {set}`)"));
            continue;
        };
        // The records are of the build as it is now, or they are stale (R33.4).
        let target_dir = ws.root.join("target").join(format!("pacemaker-measure-{set}"));
        let Some(build) = crate::hil::build_with(ws, BIN, &features(set), &target_dir, None, report) else { return false };
        if build.binary.hash != t.binary_hash || w.records.iter().any(|r| r.binary_hash != build.binary.hash) {
            return fail(report, format!("set {set}: the records are of {}, the build is now {}: measure again", t.binary_hash, build.binary.hash));
        }
        hil.push(entry(&build.binary));
        timings.push(tp);
        // Task 9.8: the fault paths' parameters of the set's injection build.
        let ip = inject_path(&ws.root, set);
        if let Some(i) = std::fs::read_to_string(&ip).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok()) {
            let dir = ws.root.join("target").join(format!("pacemaker-inject-{set}"));
            let Some(b) = crate::hil::build_with(ws, BIN, &features_of(set, Kind::Inject), &dir, None, report) else { return false };
            if b.binary.hash != i.binary_hash {
                return fail(report, format!("set {set}: the injection record is of {}, the build is now {}: run it again", i.binary_hash, b.binary.hash));
            }
            hil.push(entry(&b.binary));
            timings.push(ip);
        }
        for r in w.records {
            let max = |r: &WcetRecord| r.measured.as_ref().map_or(0, |m| m.max_observed);
            match best.iter_mut().find(|b| b.item == r.item) {
                Some(b) if max(&r) > max(b) => *b = r,
                Some(_) => {}
                None => best.push(r),
            }
        }
    }
    if timings.is_empty() {
        return fail(report, "no measured records".into());
    }
    // Task 9.8: Δ_entry, the cycles of the Kernel's entry and return stubs
    // around the charge points, is the Kernel's alone: the stubs are the same
    // assembly in every binary (the build check `entry_stubs` compares the
    // pulse generator's with the source, instruction for instruction). rsk
    // measures it with its cycle test on its demonstration; that record is
    // taken while the job's manifest lists its binary.
    let cycles = crate::hil::timing_record_path(&ws.root, "nrf52840_cycles");
    if let Some(c) = std::fs::read_to_string(&cycles).ok().and_then(|t| rsk_model::read_kernel_timing(&t).ok()) {
        let listed = manifest.get("hil_binaries").and_then(|b| b.as_array()).and_then(|l| l.iter().find(|b| b.get("hash").and_then(|h| h.as_str()) == Some(c.binary_hash.as_str())).cloned());
        match (listed, c.parameters.iter().find(|p| p.symbol == "delta_entry")) {
            (Some(b), Some(p)) => {
                let only = KernelTiming { parameters: vec![p.clone()], ..c.clone() };
                let path = rig_dir(&ws.root).join("task_9_8_analysis_delta_entry.json");
                if std::fs::write(&path, rsk_model::print(&only)).is_err() {
                    return fail(report, format!("cannot write {}", ws.rel(&path)));
                }
                hil.push(b);
                // First: a later record's binary is the one the set is named after.
                timings.insert(0, path);
                report.note(format!("Δ_entry taken from rsk's cycle test ({}, binary {}): the Kernel's stubs, not this Application's binary", ws.rel(&cycles), c.binary_hash));
            }
            _ => report.note(format!("Δ_entry stays a placeholder: {} has no record of a binary the job's manifest lists", ws.rel(&cycles))),
        }
    }
    manifest["hil_binaries"] = serde_json::Value::Array(hil);
    let out = rig_dir(&ws.root).join("task_9_8_analysis");
    let _ = std::fs::create_dir_all(&out);
    let manifest_path = out.join("build-manifest.json");
    let wcet_merged = out.join("wcet.json");
    if std::fs::write(&manifest_path, serde_json::to_string_pretty(&manifest).unwrap_or_default()).is_err()
        || std::fs::write(&wcet_merged, rsk_model::print(&WcetSet { schema: WCET_SCHEMA.into(), records: best })).is_err()
    {
        return fail(report, format!("cannot write under {}", ws.rel(&out)));
    }
    let mut cmd = crate::cmd::Cmd::cargo_frozen("run")
        .args(["--quiet", "-p", "rsk-analyzer", "--manifest-path"])
        .arg(ws.root.join("Cargo.toml"))
        .arg("--target-dir")
        .arg(ws.root.join("target"))
        .arg("--")
        .arg("--model")
        .arg(&model_path)
        .arg("--manifest")
        .arg(&manifest_path)
        .arg("--placeholders");
    for t in &timings {
        cmd = cmd.arg("--timing").arg(t);
    }
    let cmd = cmd.arg("--wcet").arg(&wcet_merged).arg("--out").arg(&out);
    let (status, text) = match cmd.output(None) {
        Ok(o) => (o.status.code().unwrap_or(3), format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))),
        Err(e) => return fail(report, format!("cannot run the Analyzer: {e}")),
    };
    print!("{text}");
    let Ok(rep) = std::fs::read_to_string(out.join("report.json")).map_err(|e| e.to_string()).and_then(|t| rsk_model::read_report(&t).map_err(|e| format!("{e:?}"))) else {
        return fail(report, format!("the Analyzer (exit {status}) wrote no report"));
    };
    let cpu_hz = crate::hil::CPU_HZ;
    let ms = |c: u64| c as f64 * 1000.0 / cpu_hz as f64;
    println!("\nResult {} (utilization {} %), exit {status}", rep.result, rep.utilization_percent);
    println!("| Task | WCET used (µs) | WCRT (ms) | Deadline (ms) | Schedulable |\n|---|---|---|---|---|");
    for t in &rep.tasks {
        println!("| `{}` | {:.1} | {:.3} | {:.3} | {} |", t.id, ms(t.c_cycles) * 1000.0, ms(t.r_cycles), ms(t.deadline_cycles), t.schedulable);
    }
    for r in &rep.rejections {
        println!("rejection: {r}");
    }
    report.note(format!("the Analyzer on the measured records: {} (utilization {} %); report under {}", rep.result, rep.utilization_percent, ws.rel(&out)));
    true
}

/// The coverage run's report (task 9.9): the counters of the Partition read
/// through the probe, the branch report of the coverage toolchain, and per
/// Task entry the branches of its body, of the Partition crate's other
/// functions that are no Task's body, and of the library crates whose
/// functions are instantiated in the Partition. The counters are the
/// build's, not a Task's: a branch of shared code that any Task took
/// counts for every Task, as it does for a crate's helpers in rsk's report.
#[allow(clippy::too_many_arguments)]
fn coverage(
    ws: &Workspace,
    rig: &rsk_hil::config::Rig,
    set: &str,
    scenario: &str,
    build: &crate::hil::HilBuild,
    target: &rsk_hil::board::Target<'_>,
    target_dir: &Path,
    tools: &rsk_hil::coverage::Tools,
    model: &rsk_model::task_model::TaskModel,
    st: &rsk_hil::stats::Stats,
    report: &mut Report,
) -> bool {
    let work = target_dir.join("coverage-run");
    let _ = std::fs::create_dir_all(&work);
    let sources = [ws.root.join("apps").join(BIN).join("therapy").join("src"), ws.root.join("crates").join("pm-core").join("src"), ws.root.join("crates").join("pm-hw").join("src")];
    let export = match rsk_hil::coverage::collect(target, &build.elf, &work).and_then(|(profraw, _)| rsk_hil::coverage::export(tools, &build.elf, &profraw, &work, &sources)) {
        Ok(e) => e,
        Err(e) => return fail(report, format!("coverage: {e}")),
    };
    let functions = rsk_hil::coverage::functions(&export);
    let jobs: Vec<serde_json::Value> = model.tasks.iter().enumerate().map(|(i, m)| serde_json::json!({ "task": m.id, "jobs": st.tasks.get(i).map_or(0, |t| t.exec_count) })).collect();
    let record = serde_json::json!({
        "task": "9.9", "set": set, "scenario": scenario, "binary_hash": build.binary.hash, "toolchain": crate::hil::COVERAGE_TOOLCHAIN,
        "features": features_of(set, Kind::Coverage), "jobs": jobs, "functions": functions_json(&functions),
    });
    if std::fs::write(coverage_path(&ws.root, set), serde_json::to_string_pretty(&record).unwrap_or_default()).is_err() {
        return fail(report, format!("cannot write {}", ws.rel(&coverage_path(&ws.root, set))));
    }
    report.note(format!("coverage of set {set} written: {} ({} functions)", ws.rel(&coverage_path(&ws.root, set)), functions.len()));
    coverage_report(ws, rig.coverage_branch_percent, report)
}

fn functions_json(functions: &[rsk_hil::coverage::Function]) -> Vec<serde_json::Value> {
    functions
        .iter()
        .map(|f| serde_json::json!({ "function": rsk_hil::coverage::short(&f.name), "symbol": f.name, "file": rsk_hil::coverage::normalized(&f.file), "branch_outcomes": f.branches, "taken": f.covered, "not_taken": f.uncovered }))
        .collect()
}

/// The functions that belong to one Task alone, by the function's name as
/// the report prints it: the Task's body and what only it calls. Every
/// other function of the three crates is shared by the four Tasks (the
/// step, the driver, the board's registers, the event ring).
pub const OWN: &[(&str, &[&str])] = &[
    ("pace", &["pacemaker_therapy::pace", "pacemaker_therapy::Timing::on_pace"]),
    ("sense_v", &["pacemaker_therapy::sense_v", "pacemaker_therapy::Timing::on_sense"]),
    ("sense_a", &["pacemaker_therapy::sense_a", "pacemaker_therapy::Timing::on_sense"]),
    ("control", &["pacemaker_therapy::control", "pacemaker_therapy::Timing::on_control", "pacemaker_therapy::Timing::init", "pm_core::validate::", "pm_hw::Board>::configure"]),
];
/// Functions the compiler evaluates and the binary never runs: the text of
/// a rig build's parameter set (`sets::from_text`, a constant's initializer).
pub const COMPILE_TIME: &[&str] = &["pacemaker_therapy::sets::from_text"];

/// `cargo xtask pm-measure --coverage-report` (task 9.9), also run after
/// each coverage run: the sets' coverage records combined (a branch outcome
/// counts as taken when a run of either set took it; the two builds differ
/// in the parameter set alone), attributed to the Task entries by `OWN`,
/// judged against the rig's threshold, and entered into the sets'
/// WCET_Records as their `coverage_branch_percent`. A set's record is first
/// brought up to the full function list from the export its run left under
/// `target/`, where there is one.
pub fn coverage_report(ws: &Workspace, threshold: u64, report: &mut Report) -> bool {
    use std::collections::BTreeMap;
    // (function, file) -> (branch outcomes, the outcomes no run took)
    let mut all: BTreeMap<(String, String), (u64, Vec<String>)> = BTreeMap::new();
    let mut sets = Vec::new();
    for set in SETS {
        let path = coverage_path(&ws.root, set);
        let Some(mut rec) = std::fs::read_to_string(&path).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else { continue };
        let export = ws.root.join("target").join(format!("pacemaker-coverage-{set}")).join("coverage-run").join("coverage-export.json");
        if rec.get("functions").map_or(true, |f| !f.is_array()) || rec["functions"].as_array().is_some_and(|a| a.first().is_some_and(|f| f.get("symbol").is_none())) {
            let Some(e) = std::fs::read_to_string(&export).ok().and_then(|t| serde_json::from_str::<serde_json::Value>(&t).ok()) else {
                report.note(format!("set {set}: its record has no function list and {} is gone: run the coverage again", ws.rel(&export)));
                continue;
            };
            rec["functions"] = serde_json::Value::Array(functions_json(&rsk_hil::coverage::functions(&e)));
            for k in ["items", "functions_with_outcomes_not_taken", "threshold_percent", "verdict"] {
                if let Some(o) = rec.as_object_mut() {
                    o.remove(k);
                }
            }
            let _ = std::fs::write(&path, serde_json::to_string_pretty(&rec).unwrap_or_default());
        }
        for f in rec["functions"].as_array().cloned().unwrap_or_default() {
            let key = (f["function"].as_str().unwrap_or("?").to_string(), f["file"].as_str().unwrap_or("?").to_string());
            let not: Vec<String> = f["not_taken"].as_array().map(|a| a.iter().filter_map(|x| x.as_str().map(str::to_string)).collect()).unwrap_or_default();
            let n = f["branch_outcomes"].as_u64().unwrap_or(0);
            match all.get_mut(&key) {
                Some((_, kept)) => kept.retain(|o| not.contains(o)),
                None => {
                    all.insert(key, (n, not));
                }
            }
        }
        sets.push((set.to_string(), rec["binary_hash"].as_str().unwrap_or("?").to_string(), rec["scenario"].as_str().unwrap_or("?").to_string()));
    }
    if sets.is_empty() {
        return fail(report, "no coverage record".into());
    }
    let own_of = |name: &str| -> Vec<&str> { OWN.iter().filter(|(_, fs)| fs.iter().any(|f| if f.ends_with("::") || f.contains('>') { name.contains(f) } else { name == *f })).map(|(t, _)| *t).collect() };
    let mut text = format!("# Task 9.9: branch coverage of the measured items\n\nCoverage builds ({}), each run {DURATION_SECS} s under its records' scenario:\n\n", crate::hil::COVERAGE_TOOLCHAIN);
    for (set, hash, scenario) in &sets {
        text.push_str(&format!("- set {set}: {hash}; {scenario}\n"));
    }
    text.push_str(&format!("\nA branch outcome is taken when a run of either set took it. Threshold: {threshold} % (`hil-rig.toml`, R34.4).\n\n| Task entry | Branch outcomes | Taken | Percent | Of its own functions | Of the shared functions |\n|---|---|---|---|---|---|\n"));
    let shared: Vec<(&(String, String), &(u64, Vec<String>))> = all.iter().filter(|((n, _), _)| own_of(n).is_empty() && !COMPILE_TIME.contains(&n.as_str())).collect();
    let sum = |fs: &[(&(String, String), &(u64, Vec<String>))]| fs.iter().fold((0u64, 0u64), |a, (_, (n, not))| (a.0 + n, a.1 + n.saturating_sub(not.len() as u64)));
    let (sn, st) = sum(&shared);
    let mut items = Vec::new();
    let mut ok = true;
    for (task, _) in OWN {
        let own: Vec<(&(String, String), &(u64, Vec<String>))> = all.iter().filter(|((n, _), _)| own_of(n).contains(task)).collect();
        let (on, ot) = sum(&own);
        let (n, taken) = (on + sn, ot + st);
        let percent = if n == 0 { 100 } else { taken * 100 / n };
        ok = ok && percent >= threshold;
        text.push_str(&format!("| `therapy.{task}` | {n} | {taken} | {percent} | {ot} of {on} | {st} of {sn} |\n"));
        items.push(serde_json::json!({ "task": format!("therapy.{task}"), "branch_outcomes": n, "taken": taken, "percent": percent, "own": { "branch_outcomes": on, "taken": ot }, "shared": { "branch_outcomes": sn, "taken": st } }));
    }
    text.push_str("\nBranch outcomes no run took (`line:column outcome`):\n\n");
    let mut not_taken = Vec::new();
    for ((name, file), (n, not)) in &all {
        if not.is_empty() {
            continue;
        }
        let whose = if COMPILE_TIME.contains(&name.as_str()) { "not counted: evaluated by the compiler only".to_string() } else if own_of(name).is_empty() { "shared".to_string() } else { own_of(name).join(", ") };
        let short_file = file.rsplit("IronOxide/").next().unwrap_or(file);
        text.push_str(&format!("- `{name}` ({short_file}; {whose}): {} of {n}: {}\n", not.len(), not.join(", ")));
        not_taken.push(serde_json::json!({ "function": name, "file": short_file, "of": whose, "branch_outcomes": n, "not_taken": not }));
    }
    text.push_str(&format!("\nVerdict: **{}**\n", if ok { "pass" } else { "FAIL" }));
    let dir = rig_dir(&ws.root);
    let record = serde_json::json!({
        "task": "9.9", "threshold_percent": threshold, "sets": sets.iter().map(|(s, h, c)| serde_json::json!({ "set": s, "binary_hash": h, "scenario": c })).collect::<Vec<_>>(),
        "items": items, "not_taken": not_taken, "verdict": if ok { "pass" } else { "FAIL" },
    });
    let (md, js) = (dir.join("task_9_9_coverage.md"), dir.join("task_9_9_coverage.json"));
    if std::fs::write(&md, &text).is_err() || std::fs::write(&js, serde_json::to_string_pretty(&record).unwrap_or_default()).is_err() {
        return fail(report, format!("cannot write under {}", ws.rel(&dir)));
    }
    // Into the WCET_Records, as rsk's coverage run enters its figures.
    for set in SETS {
        let path = wcet_path(&ws.root, set);
        let Some(mut w) = std::fs::read_to_string(&path).ok().and_then(|t| rsk_model::read_wcet_set(&t).ok()) else { continue };
        for r in &mut w.records {
            let task = r.item.split('#').next().unwrap_or(&r.item).to_string();
            let Some(p) = items.iter().find(|i| i["task"].as_str() == Some(task.as_str())).and_then(|i| i["percent"].as_u64()) else { continue };
            if let Some(m) = &mut r.measured {
                m.coverage_branch_percent = p;
            }
            r.conditions.retain(|c| !c.starts_with("branch coverage"));
            r.conditions.push(format!("branch coverage {p} % of the Task entry's functions, from the coverage builds' runs under the same scenarios (docs/pacemaker/rig/task_9_9_coverage.json; R34.4)"));
        }
        let _ = std::fs::write(&path, rsk_model::print(&w));
    }
    print!("{text}");
    report.note(format!("coverage report written: {}, {}", ws.rel(&md), ws.rel(&js)));
    ok || fail(report, "an item is below the coverage threshold (report above)".into())
}
