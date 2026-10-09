//! Analyzer inputs (R28): the Task_Model, the WCET_Record set, the
//! Kernel_Timing_Parameters and the Build_Manifest, each validated against
//! its schema (R28.1) and cross-checked (R28.2 to R28.8); then the terms
//! of the response-time analysis (R29.2 to R29.4, R29.8) in CPU cycles.

use std::collections::BTreeMap;

use rsk_model::task_model::{Task, TaskModel};
use rsk_model::{KernelTiming, WcetRecord, WcetSet};

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

/// Rounding directions of R29.8.
fn ticks_to_cycles_down(ticks: u64, tick_hz: u64, cpu_hz: u64) -> Option<u64> {
    let v = (ticks as u128) * (cpu_hz as u128) / (tick_hz as u128);
    u64::try_from(v).ok()
}

fn ticks_to_cycles_up(ticks: u64, tick_hz: u64, cpu_hz: u64) -> Option<u64> {
    let num = (ticks as u128) * (cpu_hz as u128);
    let den = tick_hz as u128;
    u64::try_from(num.div_ceil(den)).ok()
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
    pub stops_overruns: bool,
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

    // R28.6: constructs and versions.
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
    let mut usable: Vec<WcetRecord> = Vec::new();
    for r in &wcet.records {
        let mut why = Vec::new();
        if !binary_hashes.contains(&r.binary_hash) {
            why.push(format!("binary hash {} is not in the Build_Manifest", r.binary_hash));
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
    // Kernel_Timing_Parameters (R13.3, R28.2).
    if !binary_hashes.contains(&timing.binary_hash) {
        rejections.push(reject("R28.3", format!("the Kernel_Timing_Parameters' binary hash {} is not in the Build_Manifest", timing.binary_hash)));
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
    let delta_release = ktp("delta_release");
    let delta_dispatch = ktp("delta_dispatch");
    let delta_complete = ktp("delta_complete");
    let delta_wake = ktp("delta_wake");
    let delta_timer = ktp("delta_timer");
    let delta_fp_save = ktp("delta_fp_save");
    let delta_fp_restore = ktp("delta_fp_restore");
    let delta_lock = ktp("delta_lock_in").saturating_add(ktp("delta_lock_out"));
    let delta_detect = ktp("delta_detect");
    let delta_enforce = ktp("delta_enforce");
    let delta_hm = ktp("delta_hm");
    let delta_irq = ktp("delta_irq");
    let delta_stamp = ktp("delta_stamp");
    let l_kernel = ktp("l_kernel");
    let j_rel = ktp("j_rel");
    let delta_restart = ktp("delta_restart");
    let delta_stop = ktp("delta_stop");

    let any_fpu = model.tasks.iter().any(|t| t.fpu);
    let partition_of: BTreeMap<&str, &rsk_model::task_model::Partition> = model.partitions.iter().map(|p| (p.id.as_str(), p)).collect();

    // Critical_Section WCETs per Task (R29.2) and Task entry WCETs (R28.2, R28.4).
    let mut tasks = Vec::new();
    for t in &model.tasks {
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
        let mut longest_cs = 0u64;
        let mut cs_wcets: Vec<(String, u64)> = Vec::new();
        for site in &t.cs_sites {
            let r = pick(&usable, site, options).ok_or_else(|| vec![reject("R28.2", format!("no WCET_Record for Critical_Section site `{site}`"))])?;
            provenance.push(r.provenance());
            longest_cs = longest_cs.max(r.wcet_cycles);
            cs_wcets.push((site.clone(), r.wcet_cycles));
        }
        let period_cycles = ticks_to_cycles_down(t.window_ticks(), tick_hz, cpu_hz).ok_or_else(|| vec![reject("R29.6", format!("period overflow for `{}`", t.id))])?;
        let deadline_cycles = ticks_to_cycles_down(t.deadline_ticks, tick_hz, cpu_hz).ok_or_else(|| vec![reject("R29.6", format!("deadline overflow for `{}`", t.id))])?;
        // O_i (R29.3): release, dispatch, completion and wake-up per Job; the
        // timed events of the Task (release, Budget, deadline); FP save and
        // restore when the Task or a preemptor uses the FPU; lock in/out
        // per Critical_Section; interrupt intake for sporadic Tasks.
        let timed_events = 1 + 1 + if t.is_periodic() || t.mit_policy.as_deref() == Some("defer") { 1 } else { 0 };
        let mut o = delta_release
            .saturating_add(delta_dispatch)
            .saturating_add(delta_complete)
            .saturating_add(delta_wake)
            .saturating_add(delta_timer.saturating_mul(timed_events))
            .saturating_add(delta_lock.saturating_mul(t.cs_sites.len() as u64))
            .saturating_add(delta_hm);
        if any_fpu {
            o = o.saturating_add(delta_fp_save).saturating_add(delta_fp_restore);
        }
        if !t.is_periodic() {
            o = o.saturating_add(delta_irq).saturating_add(delta_stamp);
        }
        // E_i (R29.4).
        let budget = t.budget_cycles as u64;
        let e = if stops_overruns {
            budget.saturating_add(delta_detect).saturating_add(delta_enforce).saturating_add(longest_cs).saturating_add(o)
        } else {
            budget.saturating_add(o)
        };
        tasks.push(AnalysedTask {
            id: t.id.clone(),
            partition: t.partition.clone(),
            level: partition.level.clone(),
            priority: t.priority,
            period_cycles,
            deadline_cycles,
            c: budget,
            b: 0,
            b_source: String::new(),
            o,
            j: ticks_to_cycles_up(0, tick_hz, cpu_hz).unwrap_or(0).max(j_rel),
            e,
            stops_overruns,
            longest_cs,
            wcet_provenance: provenance,
            wcet: entry.wcet_cycles,
        });
        let _ = cs_wcets;
    }
    // B_i (R29.2): max of L_kernel and, over the Critical_Section sites of
    // lower-Priority Tasks on Resources whose Ceiling is at least P_i, the
    // site's WCET; for a site of another Partition that stops overrunning
    // Jobs, Budget_l + Δ_detect + Δ_enforce. The Partition stop/restart
    // work (R14.10) is included as a Kernel-level blocking term too.
    let restart_work = delta_restart.max(delta_stop);
    for i in 0..tasks.len() {
        let ti = &model.tasks[i];
        let mut b = l_kernel.max(restart_work);
        let mut source = if restart_work > l_kernel { "Partition stop/restart work (R14.10)".to_string() } else { "L_kernel".to_string() };
        for (l, tl) in model.tasks.iter().enumerate() {
            if tl.priority >= ti.priority {
                continue;
            }
            for site in &tl.cs_sites {
                let resource_name = site.rsplit(':').next().unwrap_or("");
                let Some(res) = model.resources.iter().find(|r| r.partition == tl.partition && r.id.ends_with(&format!(".{resource_name}"))) else { continue };
                if res.ceiling < ti.priority {
                    continue;
                }
                let site_wcet = pick(&usable, site, options).map(|r| r.wcet_cycles).unwrap_or(0);
                let candidate = if tl.partition != ti.partition && tasks[l].stops_overruns {
                    (tl.budget_cycles as u64).saturating_add(delta_detect).saturating_add(delta_enforce)
                } else {
                    site_wcet
                };
                if candidate > b {
                    b = candidate;
                    source = site.clone();
                }
            }
        }
        tasks[i].b = b;
        tasks[i].b_source = source;
    }
    let mut omitted_terms = Vec::new();
    omitted_terms.push("Endpoint operations (Phase P4) are not modelled".to_string());
    if !any_fpu {
        omitted_terms.push("no FPU-using Task: Δ_fp_save and Δ_fp_restore are not charged".to_string());
    }
    Ok(Prepared {
        tasks,
        omitted_terms,
        placeholders,
    })
}

/// Interference terms for Task `i` (R29.1: every j in hep(i), j ≠ i).
pub fn terms_for(tasks: &[AnalysedTask], i: usize) -> Vec<crate::rta::Term> {
    tasks
        .iter()
        .enumerate()
        .filter(|(j, t)| *j != i && t.priority >= tasks[i].priority)
        .map(|(_, t)| crate::rta::Term {
            jitter: t.j,
            period: t.period_cycles.max(1),
            bound: t.e,
        })
        .collect()
}

/// R29.9: Σ E_j / T_j > 1 fails without iteration (checked in integer
/// arithmetic as Σ E_j · Π T ... avoided: compared pairwise via u128).
pub fn utilization_exceeds_one(tasks: &[AnalysedTask]) -> bool {
    // Σ E_j/T_j > 1  <=>  Σ (E_j · L / T_j) > L for L = lcm; use a common
    // scale of 1e9 with u128 to avoid floating point.
    const SCALE: u128 = 1_000_000_000;
    let mut sum: u128 = 0;
    for t in tasks {
        let tp = t.period_cycles.max(1) as u128;
        sum += (t.e as u128 * SCALE).div_ceil(tp);
    }
    sum > SCALE
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
