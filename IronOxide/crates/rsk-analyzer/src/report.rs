//! The analysis report (R30): the schema-versioned JSON document
//! (`rsk-model::report`) and the human-readable rendering, with the
//! statements of R18.9 (Overrun handling assumed) and R27.5 (single core),
//! the input hashes (R30.2), and the provenance of every WCET used (R30.3).
//! No timestamp is written, so identical inputs give identical reports
//! (R30.5).

use rsk_model::report::{CrossCheck, Inputs, Interference, Profile, Report, TaskResult};

pub use rsk_model::report::SCHEMA;

/// The per-Task outcome of the fixed point, with the interference split.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskOutcome {
    pub r: u64,
    pub schedulable: bool,
    pub interference: Vec<Interference>,
}

pub fn build(
    result: &str,
    inputs: Inputs,
    profile: Profile,
    analyzer_version: &str,
    tasks: Vec<TaskResult>,
    utilization_percent: u64,
    cross_check: CrossCheck,
    omitted_terms: Vec<String>,
    rejections: Vec<String>,
    placeholders: &[String],
) -> Report {
    let mut statements = vec![
        "This analysis assumes a single core (R27.5): it is no evidence for a multi-core configuration.".to_string(),
        "Interference bounds assume the Overrun handling of R18 (R18.9): a Task of a Partition whose Overrun_Response stops overrunning Jobs is charged Budget + Δ_detect + Δ_enforce + its longest Critical_Section; other Tasks are charged their Budget.".to_string(),
        "Every offset is treated as zero (synchronous release, R29.1); release jitter J_rel applies to every Task.".to_string(),
    ];
    if !placeholders.is_empty() {
        statements.push(format!(
            "PLACEHOLDER TIMING DATA: {} input value(s) carry the method `placeholder` (no measurement or analysis): {}. This report is not timing evidence.",
            placeholders.len(),
            placeholders.join(", ")
        ));
    }
    Report {
        schema: SCHEMA.to_string(),
        result: result.to_string(),
        statements,
        inputs,
        analyzer: analyzer_version.to_string(),
        profile,
        tasks,
        utilization_percent,
        cross_check,
        omitted_terms,
        rejections,
    }
}

/// The human-readable report (R30.1).
pub fn render_text(r: &Report) -> String {
    let mut s = String::new();
    s.push_str(&format!("rsk analysis report ({}) — result: {}\n", r.schema, r.result));
    s.push_str(&format!("Analyzer {}; Profile {} {}\n", r.analyzer, r.profile.version, r.profile.variant));
    s.push_str("Inputs:\n");
    s.push_str(&format!("  Task_Model              {}\n", r.inputs.task_model));
    s.push_str(&format!("  WCET_Records            {}\n", r.inputs.wcet_records));
    s.push_str(&format!("  Kernel_Timing_Parameters {}\n", r.inputs.kernel_timing));
    s.push_str(&format!("  Build_Manifest          {}\n", r.inputs.build_manifest));
    for st in &r.statements {
        s.push_str(&format!("Statement: {st}\n"));
    }
    if !r.rejections.is_empty() {
        s.push_str("Rejected inputs:\n");
        for x in &r.rejections {
            s.push_str(&format!("  {x}\n"));
        }
    }
    s.push_str(&format!("Utilization (Σ E_j/T_j): {}%\n", r.utilization_percent));
    s.push_str("Tasks:\n");
    for t in &r.tasks {
        s.push_str(&format!(
            "  {} (Partition {}, level {}, Priority {}): T={} D={} C={} B={} (from {}) O={} J={} R={} {} slack={}\n",
            t.id,
            t.partition,
            t.level,
            t.priority,
            t.period_cycles,
            t.deadline_cycles,
            t.c_cycles,
            t.b_cycles,
            t.b_source,
            t.o_cycles,
            t.j_cycles,
            t.r_cycles,
            if t.schedulable { "schedulable" } else { "NOT schedulable (lower bound)" },
            t.slack_cycles
        ));
        for i in &t.interference {
            s.push_str(&format!("      interference from {}: {} jobs × E={} = {} cycles\n", i.task, i.jobs, i.e_cycles, i.cycles));
        }
        for p in &t.wcet_provenance {
            s.push_str(&format!("      WCET {p}\n"));
        }
    }
    s.push_str(&format!("UPPAAL cross-check: {} ({})\n", r.cross_check.status, r.cross_check.detail));
    if !r.omitted_terms.is_empty() {
        s.push_str("Omitted or approximated terms:\n");
        for o in &r.omitted_terms {
            s.push_str(&format!("  {o}\n"));
        }
    }
    s
}

pub fn render_json(r: &Report) -> String {
    rsk_model::print(r)
}
