//! Analyzer inputs (R28): the Task_Model, the WCET_Record set, the
//! Kernel_Timing_Parameters and the Build_Manifest, each validated against
//! its schema (R28.1) and cross-checked (R28.2 to R28.8); then the terms
//! of the response-time analysis (R29.2 to R29.4, R29.8) in CPU cycles,
//! derived by the verified `terms` module from the resolved values this
//! module takes out of the Task_Model (2026-11-06).

use std::collections::BTreeMap;

use rsk_model::task_model::{Task, TaskModel};
use rsk_model::{KernelTiming, WcetRecord, WcetSet};

use crate::terms;

/// Why an input was rejected (exit status 2, R30.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Rejection {
    pub rule: String,
    pub message: String,
}

impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.rule, self.message)
    }
}

fn reject(rule: &str, message: impl Into<String>) -> Rejection {
    Rejection {
        rule: rule.into(),
        message: message.into(),
    }
}

/// Analysis configuration.
#[derive(Clone, Debug, Default)]
pub struct Options {
    /// Accept WCET_Records and Kernel_Timing_Parameters of method
    /// `placeholder` (pre-hardware development); the report says so.
    pub allow_placeholders: bool,
    /// R28.7: designate a method when both exist (`measured` or `static`);
    /// `None` takes the larger value.
    pub prefer_method: Option<String>,
}

/// One analysed Task with every term of R29.1 in CPU cycles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnalysedTask {
    pub id: String,
    pub partition: String,
    pub level: String,
    pub priority: u8,
    /// Period or MIT (R29.8: rounded down).
    pub period_cycles: u64,
    pub deadline_cycles: u64,
    /// C_i: the Budget.
    pub c: u64,
    /// B_i (R29.2) and the site that determines it.
    pub b: u64,
    pub b_source: String,
    /// O_i: the Kernel overhead of the Task's own Job (R29.3).
    pub o: u64,
    /// J: the release jitter (J_rel).
    pub j: u64,
    /// E_i: the interference bound this Task imposes on lower ones (R29.4).
    pub e: u64,
    /// The intake cost of the Task's Release_Source per MIT (Δ_irq for a
    /// Sporadic_Task, 0 for a Periodic_Task): interference on every Task
    /// its Jobs cannot preempt (R19.2).
    pub intake: u64,
    pub stops_overruns: bool,
    /// A Sporadic_Task (its period is the MIT), for the UPPAAL release generator (R31.2).
    pub sporadic: bool,
    pub longest_cs: u64,
    pub wcet_provenance: Vec<String>,
    /// The WCET used for the Task entry (R28.4: at most C_i).
    pub wcet: u64,
}

/// The prepared analysis.
#[derive(Clone, Debug)]
pub struct Prepared {
    pub tasks: Vec<AnalysedTask>,
    /// Overhead terms the model omits or approximates (R31.2, reported).
    pub omitted_terms: Vec<String>,
    pub placeholders: Vec<String>,
    /// The measured Kernel_Timing_Parameters used, with their provenance
    /// (task 19.3), and the manifest entry their binary hash matched.
    pub measured_parameters: Vec<String>,
}

/// Picks the WCET_Record for an item (R28.7).
fn pick<'a>(records: &'a [WcetRecord], item: &str, options: &Options) -> Option<&'a WcetRecord> {
    let mut candidates: Vec<&WcetRecord> = records.iter().filter(|r| r.item == item).collect();
    if let Some(m) = &options.prefer_method {
        if let Some(r) = candidates.iter().find(|r| &r.method == m) {
            return Some(r);
        }
    }
    candidates.sort_by_key(|r| std::cmp::Reverse(r.wcet_cycles));
    candidates.first().copied()
}

/// R28.2 to R28.8 and the term derivation. The Build_Manifest is a
/// validated JSON value (its type belongs to the Build_System).
pub fn prepare(
    model: &TaskModel,
    wcet: &WcetSet,
    timing: &KernelTiming,
    manifest: &serde_json::Value,
    options: &Options,
) -> Result<Prepared, Vec<Rejection>> {
    let mut rejections = Vec::new();
    let mut placeholders = Vec::new();
    let cpu_hz = model.clock.cpu_hz;
    let tick_hz = model.clock.tick_hz;

    // R28.6: constructs and versions. The Analyzer implements one Profile
    // version (profile_version.rs, generated from profile/profile.toml).
    if model.profile.version != crate::profile_version::PROFILE_VERSION {
        rejections.push(reject(
            "R28.6",
            format!("the Task_Model records Profile {}; this Analyzer implements Profile {} (R3.5)", model.profile.version, crate::profile_version::PROFILE_VERSION),
        ));
    }
    if model.profile.variant != "Ravenscar" {
        rejections.push(reject("R28.6", format!("Profile_Variant `{}` is not supported by this Analyzer", model.profile.variant)));
    }
    if !model.endpoints.is_empty() {
        rejections.push(reject("R28.6", "Endpoints (Phase P4) are not supported by this Analyzer version"));
    }
    // R28.5 / R27.2: single core.
    for t in &model.tasks {
        if t.core != 0 {
            rejections.push(reject("NG-01", format!("Task `{}` is assigned to core {} (R27.2)", t.id, t.core)));
        }
    }
    for r in &model.resources {
        if r.core != serde_json::json!(0) {
            rejections.push(reject("NG-01", format!("Resource `{}` is not on core 0 (R27.2)", r.id)));
        }
    }
    // R28.8: the model's checksum is the binary's.
    let binaries = manifest.get("binaries").and_then(|b| b.as_array()).cloned().unwrap_or_default();
    let manifest_checksum = manifest.get("generated_config_checksum").and_then(|c| c.as_str()).map(str::to_string);
    match &manifest_checksum {
        Some(c) if c == &model.config_checksum => {}
        Some(c) => rejections.push(reject(
            "R28.8",
            format!("the Task_Model's Generated_Config checksum {} differs from the Build_Manifest's {c}", model.config_checksum),
        )),
        None => rejections.push(reject("R28.8", "the Build_Manifest records no Generated_Config checksum for the analysed binary")),
    }
    let binary_hashes: Vec<String> = binaries.iter().filter_map(|b| b.get("hash").and_then(|h| h.as_str()).map(str::to_string)).collect();
    let icache = manifest.pointer("/build/icache").and_then(|v| v.as_str()).unwrap_or("off").to_string();
    // R28.3 / R33.4: each record matches the binary and its configuration.
    // A measured record of a HIL build the manifest lists (`hil_binaries`,
    // task 19.1) is accepted as a Kernel_Timing_Parameter set of one is.
    let hil_binaries = manifest.get("hil_binaries").and_then(|b| b.as_array()).cloned().unwrap_or_default();
    let hil_hashes: Vec<String> = hil_binaries.iter().filter_map(|b| b.get("hash").and_then(|h| h.as_str()).map(str::to_string)).collect();
    let mut measured_parameters = Vec::new();
    let mut hil_wcet_named = false;
    let mut usable: Vec<WcetRecord> = Vec::new();
    for r in &wcet.records {
        let mut why = Vec::new();
        if !binary_hashes.contains(&r.binary_hash) {
            if r.method == "measured" && hil_hashes.contains(&r.binary_hash) {
                if !hil_wcet_named {
                    let name = hil_binaries.iter().find(|b| b.get("hash").and_then(|h| h.as_str()) == Some(r.binary_hash.as_str())).and_then(|b| b.get("target")).and_then(|t| t.as_str()).unwrap_or("?");
                    measured_parameters.push(format!("measured WCET_Records of the HIL build `{name}` ({}) of the Build_Manifest, not the analysed Flight_Build", r.binary_hash));
                    hil_wcet_named = true;
                }
            } else {
                why.push(format!("binary hash {} is not in the Build_Manifest", r.binary_hash));
            }
        }
        if r.clock_hz != cpu_hz {
            why.push(format!("clock {} Hz differs from the model's {cpu_hz} Hz", r.clock_hz));
        }
        if r.icache != icache {
            why.push(format!("icache `{}` differs from the build's `{icache}`", r.icache));
        }
        if r.method == "placeholder" {
            if options.allow_placeholders {
                placeholders.push(r.item.clone());
            } else {
                why.push("method `placeholder` (no measurement or analysis; pass --allow-placeholders for development runs)".to_string());
            }
        }
        if why.is_empty() {
            usable.push(r.clone());
        } else {
            rejections.push(reject("R28.3", format!("WCET_Record `{}` rejected: {}", r.item, why.join("; "))));
        }
    }
    // R33.5: measured maxima never exceed static bounds for the same item.
    for r in &usable {
        if r.method == "static" {
            if let Some(m) = usable.iter().filter(|m| m.item == r.item && m.method == "measured").find_map(|m| m.measured.as_ref()) {
                if m.max_observed > r.wcet_cycles {
                    rejections.push(reject(
                        "R33.5",
                        format!("WCET inconsistency for `{}`: observed {} cycles exceed the static bound {} cycles", r.item, m.max_observed, r.wcet_cycles),
                    ));
                }
            }
        }
    }
    // Kernel_Timing_Parameters (R13.3, R28.2). Their binary is the analysed
    // Flight_Build or, for parameters the HIL_Rig measured (task 19.3), a
    // HIL build the Build_Manifest lists (`hil_binaries`: the same source,
    // toolchain, clock and cache setting with the rig's instrumentation,
    // whose cost the measurement includes); the report names it.
    let hil_binary = hil_binaries.iter().find(|b| b.get("hash").and_then(|h| h.as_str()) == Some(timing.binary_hash.as_str()));
    if !binary_hashes.contains(&timing.binary_hash) {
        match hil_binary {
            Some(b) => {
                let name = b.get("target").and_then(|t| t.as_str()).unwrap_or("?");
                measured_parameters.push(format!("the Kernel_Timing_Parameters' binary {} is the HIL build `{name}` of the Build_Manifest, not the analysed Flight_Build", timing.binary_hash));
            }
            None => rejections.push(reject("R28.3", format!("the Kernel_Timing_Parameters' binary hash {} is not in the Build_Manifest", timing.binary_hash))),
        }
    }
    if timing.icache != icache {
        rejections.push(reject("R28.3", format!("the Kernel_Timing_Parameters' icache `{}` differs from the build's `{icache}`", timing.icache)));
    }
    // R13.4: a measured parameter's maximum observed value never exceeds its
    // bound.
    for p in &timing.parameters {
        if let Some(m) = &p.measured {
            if m.max_observed > p.cycles {
                rejections.push(reject("R13.4", format!("Kernel_Timing_Parameter `{}`: observed {} cycles exceed its bound {} cycles", p.symbol, m.max_observed, p.cycles)));
            }
        }
        if p.method == "measured" {
            measured_parameters.push(p.provenance());
        }
    }
    if timing.clock_hz != cpu_hz {
        rejections.push(reject("R28.3", format!("the Kernel_Timing_Parameters' clock {} Hz differs from the model's {cpu_hz} Hz", timing.clock_hz)));
    }
    let missing = timing.missing();
    if !missing.is_empty() {
        rejections.push(reject("R13.3", format!("Kernel_Timing_Parameters without a WCET_Record: {}", missing.join(", "))));
    }
    for p in &timing.parameters {
        if p.method == "placeholder" {
            if options.allow_placeholders {
                placeholders.push(p.symbol.clone());
            } else {
                rejections.push(reject("R13.2", format!("Kernel_Timing_Parameter `{}` is a placeholder (no measurement or analysis)", p.symbol)));
            }
        }
    }
    if !rejections.is_empty() {
        return Err(rejections);
    }
    let ktp = |s: &str| timing.get(s).unwrap_or(0);
    // The Kernel_Timing_Parameters of Table 13-1 the terms use (R29.3).
    // Δ_entry (R13.6, measured by the cycle test of 11.11): the cycles a
    // Kernel entry charges the Job it interrupts; charged once per Job
    // instance, so that every interfering instance carries it.
    let params = terms::Params {
        release: ktp("delta_release"),
        dispatch: ktp("delta_dispatch"),
        complete: ktp("delta_complete"),
        wake: ktp("delta_wake"),
        timer: ktp("delta_timer"),
        fp_save: ktp("delta_fp_save"),
        fp_restore: ktp("delta_fp_restore"),
        lock_in: ktp("delta_lock_in"),
        lock_out: ktp("delta_lock_out"),
        detect: ktp("delta_detect"),
        enforce: ktp("delta_enforce"),
        hm: ktp("delta_hm"),
        irq: ktp("delta_irq"),
        stamp: ktp("delta_stamp"),
        entry: ktp("delta_entry"),
        l_kernel: ktp("l_kernel"),
        j_rel: ktp("j_rel"),
        restart: ktp("delta_restart"),
        stop: ktp("delta_stop"),
    };
    if tick_hz == 0 {
        return Err(vec![reject("R29.8", "the Task_Model's tick_hz is 0: no clock ratio")]);
    }

    let any_fpu = model.tasks.iter().any(|t| t.fpu);
    let partition_of: BTreeMap<&str, &rsk_model::task_model::Partition> = model.partitions.iter().map(|p| (p.id.as_str(), p)).collect();
    let partition_index: BTreeMap<&str, u32> = model.partitions.iter().enumerate().map(|(i, p)| (p.id.as_str(), i as u32)).collect();

    // The resolved inputs of the term derivation: each Task (R29.1's C_i,
    // its Priority and Partition, its Partition's Overrun_Response, its
    // kind) and each Critical_Section site with its holder, the Ceiling of
    // its Resource, and its WCET (R28.2, R28.4, R29.2).
    let mut task_in = Vec::new();
    let mut sites_in = Vec::new();
    let mut site_names: Vec<String> = Vec::new();
    let mut per_task = Vec::new();
    for (index, t) in model.tasks.iter().enumerate() {
        let partition = partition_of.get(t.partition.as_str()).ok_or_else(|| vec![reject("R28.1", format!("Task `{}` names an unknown Partition", t.id))])?;
        let stops_overruns = partition.overrun != "RECORD_AND_CONTINUE";
        let mut provenance = Vec::new();
        let entry = pick(&usable, &t.id, options).ok_or_else(|| vec![reject("R28.2", format!("no WCET_Record for Task entry `{}`", t.id))])?;
        provenance.push(entry.provenance());
        if entry.wcet_cycles > t.budget_cycles as u64 {
            return Err(vec![reject(
                "R28.4",
                format!("the WCET of Task `{}` ({} cycles) exceeds its Budget ({} cycles)", t.id, entry.wcet_cycles, t.budget_cycles),
            )]);
        }
        for site in &t.cs_sites {
            let r = pick(&usable, site, options).ok_or_else(|| vec![reject("R28.2", format!("no WCET_Record for Critical_Section site `{site}`"))])?;
            provenance.push(r.provenance());
            let resource_name = site.rsplit(':').next().unwrap_or("");
            let res = model
                .resources
                .iter()
                .find(|r| r.partition == t.partition && r.id.ends_with(&format!(".{resource_name}")))
                .ok_or_else(|| vec![reject("R28.1", format!("Critical_Section site `{site}` names no Resource of Partition `{}`", t.partition))])?;
            sites_in.push(terms::Site { task: index, ceiling: res.ceiling, wcet: r.wcet_cycles });
            site_names.push(site.clone());
        }
        let period_cycles = terms::ticks_to_cycles_down(t.window_ticks(), tick_hz, cpu_hz).ok_or_else(|| vec![reject("R29.6", format!("period overflow for `{}`", t.id))])?;
        let deadline_cycles = terms::ticks_to_cycles_down(t.deadline_ticks, tick_hz, cpu_hz).ok_or_else(|| vec![reject("R29.6", format!("deadline overflow for `{}`", t.id))])?;
        task_in.push(terms::TaskIn {
            priority: t.priority,
            partition: *partition_index.get(t.partition.as_str()).unwrap_or(&u32::MAX),
            budget: t.budget_cycles as u64,
            stops_overruns,
            periodic: t.is_periodic(),
            defer: t.mit_policy.as_deref() == Some("defer"),
        });
        per_task.push((period_cycles, deadline_cycles, partition.level.clone(), provenance, entry.wcet_cycles));
    }
    // The terms (R29.2 to R29.4; `terms::derive_all`, proven): O_i, E_i,
    // L_i, B_i with the site that attains it, and J_i = J_rel.
    let derived = terms::derive_all(&task_in, &sites_in, params, any_fpu)
        .ok_or_else(|| vec![reject("R29.6", "a term of the response-time analysis exceeds u64 (overflow)")])?;
    let restart_work = params.restart.max(params.stop);
    let floor_source = if restart_work > params.l_kernel { "Partition stop/restart work (R14.10)" } else { "L_kernel" };
    let mut tasks = Vec::new();
    for (i, t) in model.tasks.iter().enumerate() {
        let d = derived[i];
        let (period_cycles, deadline_cycles, level, provenance, wcet) = per_task[i].clone();
        tasks.push(AnalysedTask {
            id: t.id.clone(),
            partition: t.partition.clone(),
            level,
            priority: t.priority,
            period_cycles,
            deadline_cycles,
            c: task_in[i].budget,
            b: d.b,
            b_source: match d.b_site {
                Some(k) => site_names[k].clone(),
                None => floor_source.to_string(),
            },
            o: d.o,
            j: d.j,
            e: d.e,
            intake: if t.is_periodic() { 0 } else { params.irq },
            stops_overruns: task_in[i].stops_overruns,
            sporadic: !t.is_periodic(),
            longest_cs: d.longest_cs,
            wcet_provenance: provenance,
            wcet,
        });
    }
    let mut omitted_terms = Vec::new();
    omitted_terms.push("Endpoint operations (Phase P4) are not modelled".to_string());
    if !any_fpu {
        omitted_terms.push("no FPU-using Task: Δ_fp_save and Δ_fp_restore are not charged".to_string());
    }
    if tasks.iter().any(|t| t.intake > 0) {
        omitted_terms.push("the intake handling of a Sporadic_Task's Release_Source (R19.2, Δ_irq per MIT) interferes with every Task in the RTA; the UPPAAL model carries it only in E_j, on the Tasks the Task's Jobs preempt".to_string());
    }
    Ok(Prepared {
        tasks,
        omitted_terms,
        placeholders,
        measured_parameters,
    })
}

/// The terms of Task `i`'s recurrence, each with its origin
/// (`terms::terms_for`, proven): every j in hep(i), j ≠ i, by its Jobs
/// (R29.1), and every lower-Priority Sporadic_Task by its intake (R19.2).
pub fn terms_for(tasks: &[AnalysedTask], i: usize) -> Vec<terms::TermOf> {
    let interferers: Vec<terms::Interferer> = tasks
        .iter()
        .map(|t| terms::Interferer { priority: t.priority, jitter: t.j, period: t.period_cycles.max(1), bound: t.e, intake: t.intake })
        .collect();
    terms::terms_for(&interferers, i)
}

/// R29.9: Σ E_j / T_j > 1 fails without iteration (`terms::utilization_exceeds_one`,
/// proven: the sum on the scale `terms::SCALE`, each term rounded up).
pub fn utilization_exceeds_one(tasks: &[AnalysedTask]) -> bool {
    let loads: Vec<terms::Load> = tasks.iter().map(|t| terms::Load { e: t.e, period: t.period_cycles.max(1) }).collect();
    assert!(loads.len() < 1 << 32);
    terms::utilization_exceeds_one(&loads)
}

/// Utilization in percent (rounded up), for the report.
pub fn utilization_percent(tasks: &[AnalysedTask]) -> u64 {
    let mut sum: u128 = 0;
    for t in tasks {
        let tp = t.period_cycles.max(1) as u128;
        sum += (t.e as u128 * 100_000).div_ceil(tp);
    }
    (sum.div_ceil(1000)) as u64
}

/// The Task_Model's Tasks in the analysed order (for the UPPAAL export).
pub fn model_tasks(model: &TaskModel) -> &[Task] {
    &model.tasks
}
