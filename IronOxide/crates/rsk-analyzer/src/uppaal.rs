//! The UPPAAL_Exporter (R31): one timed automaton per Task (release
//! generator with jitter, execution bounded by the Budget, Critical_Section
//! durations through the global SRP ceiling, Kernel overheads as locations),
//! a fixed-priority SRP scheduler automaton with Ceilings and
//! exception-number tie breaking (R7.3), a deadline-miss location per Task,
//! and the query `A[] not (exists (i : task_t) Task(i).Missed)` (R31.3).
//! The model is UPPAAL's XML (`.xml`) and the query file is separate
//! (`.q`) (R31.1). Every automaton records the Task_Model identifier it
//! represents (R31.7). [`read_export`] recovers each Task's Priority,
//! period or MIT, deadline and execution-time bound from an export
//! (R31.8, Property 15 round trip).
//!
//! Cross-check (R31.4 to R31.6, R31.9): when `verifyta` is installed the
//! query is run with a time and memory limit; the verdict is compared with
//! the RTA result and classified as agreement, pessimism (RTA FAIL while
//! no miss is reachable), discrepancy (a reachable miss while RTA PASS;
//! the trace is attached and marked as requiring confirmation, because
//! stopwatch models are verified by over-approximation), or inconclusive.
//! A possible miss found only in the stopwatch over-approximation is put to
//! a second model first ([`confirmation_of`], 2026-11-21): a stopwatch-free
//! over-approximation that verifyta checks exactly; no miss there refutes
//! the possible miss.

use crate::inputs::AnalysedTask;

/// One Task as exported.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub id: String,
    pub priority: u8,
    pub period_cycles: u64,
    pub deadline_cycles: u64,
    pub exec_cycles: u64,
    pub jitter_cycles: u64,
    pub blocking_cycles: u64,
    pub sporadic: bool,
}

fn xml_escape(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

/// The model (XML) and the query file.
///
/// The declaration carries the Task parameters as arrays indexed by Task
/// (`PRIO`, `PERIOD` (the MIT for a Sporadic_Task), `DEADLINE`, `EXEC` (the
/// execution-time bound E_i of R29.4: what a Job of the Task can take
/// from the Jobs it preempts, its Budget and overheads and, where its
/// Partition stops an overrunning Job, the overrun up to the stop), `OWN`
/// (the demand C_i + O_i of the Task's own Job in R29.1, at most `EXEC`),
/// `JITTER`, `BLOCK` (the SRP blocking term B_i), `SPORADIC`) and the Task_Model
/// identifier of each automaton as a comment (R31.7). Two templates:
///
/// - `Gen(i)`: the release generator. Periodic: exactly every `PERIOD[i]`.
///   Sporadic: at least `PERIOD[i]` (the MIT) after the previous release,
///   any later instant. Each release is followed by a jitter of up to
///   `JITTER[i]` before the Task becomes pending (R31.2).
/// - `Task(i)`: `Idle`, `Pending` (released, waiting for the SRP start
///   condition `eligible(i)`: no Job of higher or equal Priority and lower
///   identifier executing, Priority above the System_Ceiling), `Blocked`
///   (the blocking term, an interval up to `BLOCK[i]`), `Ready` (the
///   blocking interval is over and another Job took the processor in it),
///   `Running` (a stopwatch `exec[i]` that progresses only while the Task
///   executes; the Job completes at any point of the interval up to
///   `EXEC[i]`), and `Missed`, entered from a waiting location once the
///   deadline has passed, and from `Running` when at the deadline the Job
///   has executed for less than `OWN[i]`. `rsk-uppaal/3` judged a running
///   Job by `EXEC[i]`: a Job that had its whole demand C_i + O_i before
///   its deadline and went on into the overrun allowance of E_i counted as
///   a miss, which the RTA does not claim to exclude (R18.9: its bound is
///   for a Job that keeps to its Budget). Found on corpus model m49
///   (2026-11-21): a Job of 35,600 cycles of demand judged against 68,200.
///
/// Dispatch is urgent (`rsk-uppaal/3`, 2026-11-14): the steps out of
/// `Pending` and `Ready` synchronise on the urgent broadcast channel `go`,
/// so no time passes while a Task is eligible. This is the Hardware_Model's
/// rule (the dispatch exception is taken before the preempted code
/// continues). `rsk-uppaal/2` let an eligible Task wait with the processor
/// idle until its deadline, so a miss was reachable in every model. The
/// deadline clock `dl[i]` starts at the nominal release instant, before the
/// jitter, as the response time R_i = J_i + w_i is measured (R29.1), and a
/// Job that completes exactly at its deadline has not missed it. The
/// stopwatches are ordinary clocks with a rate: `rsk-uppaal/2` declared
/// them `hybrid clock`, which UPPAAL's symbolic checker does not support
/// (verifyta 5.0.0 and 5.1.0-beta5 crash on it).
///   Preemption: a higher Task entering `Running` takes `running`; on its
///   completion `running` returns to the highest-Priority preempted Task
///   (`resume()`), as fixed-priority preemptive dispatching does (R7.1 to
///   R7.3).
///
/// Approximations, recorded in the report's omitted terms (R31.2): the
/// Critical_Sections and the System_Ceiling are represented by the
/// blocking interval spent before the Job starts, not by locations that
/// raise `ceiling` (so `ceiling` stays 0 and the SRP start condition
/// reduces to the Priority test); the Kernel overheads of R29.3 are
/// folded into `EXEC[i]`, not modelled as locations.
pub fn export(tasks: &[AnalysedTask]) -> (String, String) {
    let n = tasks.len();
    let mut decl = String::new();
    decl.push_str("// rsk UPPAAL export (rsk-uppaal/4). Times in CPU cycles.\n");
    decl.push_str(&format!("const int N = {n};\n"));
    decl.push_str("typedef int[0,N-1] task_t;\n");
    let col = |name: &str, f: &dyn Fn(&AnalysedTask) -> u64| {
        format!("const int {name}[N] = {{ {} }};\n", tasks.iter().map(|t| f(t).to_string()).collect::<Vec<_>>().join(", "))
    };
    decl.push_str(&col("PRIO", &|t| t.priority as u64));
    decl.push_str(&col("PERIOD", &|t| t.period_cycles));
    decl.push_str(&col("DEADLINE", &|t| t.deadline_cycles));
    decl.push_str(&col("EXEC", &|t| t.e));
    decl.push_str(&col("OWN", &|t| t.c.saturating_add(t.o).min(t.e)));
    decl.push_str(&col("JITTER", &|t| t.j));
    decl.push_str(&col("BLOCK", &|t| t.b));
    decl.push_str(&format!("const bool SPORADIC[N] = {{ {} }};\n", tasks.iter().map(|t| t.sporadic.to_string()).collect::<Vec<_>>().join(", ")));
    decl.push_str("// Task_Model identifiers, by index (R31.7):\n");
    for (i, t) in tasks.iter().enumerate() {
        decl.push_str(&format!("// {i}: {}\n", t.id));
    }
    decl.push_str("int ceiling = 0;          // System_Ceiling (SRP); Critical_Sections are approximated by BLOCK\n");
    decl.push_str("int running = -1;         // the executing Task, -1 when idle\n");
    decl.push_str("bool pending[N];          // released, not started\n");
    decl.push_str("bool started[N];          // started and not completed (executing or preempted)\n");
    decl.push_str("chan release[N];\n");
    decl.push_str("urgent broadcast chan go;  // dispatch: no time passes while a Task is eligible\n");
    decl.push_str("clock dl[N];              // time since the nominal release instant\n");
    decl.push_str("clock exec[N];            // stopwatches: progress only while running (a rate of 0 or 1 in the invariant; `hybrid clock` is for the statistical checker and crashes verifyta's symbolic one)\n");
    decl.push_str("bool preempts(task_t i, int j) { return j < 0 || PRIO[i] > PRIO[j] || (PRIO[i] == PRIO[j] && i < j); }\n");
    decl.push_str("bool eligible(task_t i) { return pending[i] && PRIO[i] > ceiling && preempts(i, running); }\n");
    decl.push_str("// The Task that resumes when the executing one completes: the started one of highest Priority, lowest identifier among equals (R7.3); -1 when none.\n");
    decl.push_str("int resume() { int best = -1; for (k : task_t) { if (started[k] && (best < 0 || PRIO[k] > PRIO[best])) { best = k; } } return best; }\n");

    let mut templates = String::new();
    // Release generator: periodic (exactly every PERIOD) or sporadic (at
    // least the MIT after the previous release), then jitter.
    templates.push_str(
        r#"<template><name>Gen</name><parameter>const task_t i</parameter>
<declaration>clock x; clock jit;</declaration>
<location id="g0" x="0" y="0"><name>Wait</name><label kind="invariant">SPORADIC[i] || x &lt;= PERIOD[i]</label></location>
<location id="g1" x="200" y="0"><name>Jitter</name><label kind="invariant">jit &lt;= JITTER[i]</label></location>
<init ref="g0"/>
<transition><source ref="g0"/><target ref="g1"/><label kind="guard">x &gt;= PERIOD[i]</label><label kind="assignment">x = 0, jit = 0, dl[i] = 0</label></transition>
<transition><source ref="g1"/><target ref="g0"/><label kind="synchronisation">release[i]!</label><label kind="assignment">pending[i] = true</label></transition>
</template>
"#,
    );
    // Task automaton: Idle -> Pending -> Blocked -> (Ready ->) Running ->
    // Idle; Missed at the deadline, from Running only while the Job has had
    // less than its own demand OWN (a Job that has executed for OWN at its
    // deadline has not missed: what it does past OWN is an overrun, which
    // the model keeps as interference up to EXEC).
    templates.push_str(
        r#"<template><name>Task</name><parameter>const task_t i</parameter>
<declaration>clock b;</declaration>
<location id="t0" x="0" y="0"><name>Idle</name></location>
<location id="t1" x="200" y="0"><name>Pending</name><label kind="invariant">dl[i] &lt;= DEADLINE[i]</label></location>
<location id="t2" x="400" y="0"><name>Blocked</name><label kind="invariant">b &lt;= BLOCK[i] &amp;&amp; dl[i] &lt;= DEADLINE[i]</label></location>
<location id="t5" x="500" y="100"><name>Ready</name><label kind="invariant">dl[i] &lt;= DEADLINE[i]</label></location>
<location id="t3" x="600" y="0"><name>Running</name><label kind="invariant">exec[i]' == (running == i) &amp;&amp; exec[i] &lt;= EXEC[i] &amp;&amp; dl[i] &lt;= DEADLINE[i]</label></location>
<location id="t4" x="800" y="0"><name>Missed</name></location>
<init ref="t0"/>
<transition><source ref="t0"/><target ref="t1"/><label kind="synchronisation">release[i]?</label></transition>
<transition><source ref="t1"/><target ref="t2"/><label kind="guard">eligible(i)</label><label kind="synchronisation">go!</label><label kind="assignment">b = 0, pending[i] = false</label></transition>
<transition><source ref="t2"/><target ref="t3"/><label kind="guard">b &lt;= BLOCK[i] &amp;&amp; preempts(i, running)</label><label kind="assignment">running = i, started[i] = true, exec[i] = 0</label></transition>
<transition><source ref="t2"/><target ref="t5"/><label kind="guard">b &gt;= BLOCK[i]</label><label kind="assignment">pending[i] = true</label></transition>
<transition><source ref="t5"/><target ref="t3"/><label kind="guard">eligible(i)</label><label kind="synchronisation">go!</label><label kind="assignment">pending[i] = false, running = i, started[i] = true, exec[i] = 0</label></transition>
<transition><source ref="t3"/><target ref="t0"/><label kind="guard">exec[i] &lt;= EXEC[i] &amp;&amp; running == i</label><label kind="assignment">started[i] = false, running = resume()</label></transition>
<transition><source ref="t1"/><target ref="t4"/><label kind="guard">dl[i] &gt;= DEADLINE[i]</label></transition>
<transition><source ref="t2"/><target ref="t4"/><label kind="guard">dl[i] &gt;= DEADLINE[i]</label></transition>
<transition><source ref="t5"/><target ref="t4"/><label kind="guard">dl[i] &gt;= DEADLINE[i]</label></transition>
<transition><source ref="t3"/><target ref="t4"/><label kind="guard">dl[i] &gt;= DEADLINE[i] &amp;&amp; exec[i] &lt; OWN[i]</label></transition>
</template>
"#,
    );
    let mut system = String::new();
    for i in 0..n {
        system.push_str(&format!("G{i} = Gen({i}); T{i} = Task({i});\n"));
    }
    system.push_str("system ");
    system.push_str(&(0..n).flat_map(|i| [format!("G{i}"), format!("T{i}")]).collect::<Vec<_>>().join(", "));
    system.push_str(";\n");
    let xml = format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n<!DOCTYPE nta PUBLIC '-//Uppaal Team//DTD Flat System 1.1//EN' 'http://www.it.uu.se/research/group/darts/uppaal/flat-1_2.dtd'>\n<nta>\n<declaration>{}</declaration>\n{templates}<system>{}</system>\n</nta>\n",
        xml_escape(&decl),
        xml_escape(&system)
    );
    let mut query = String::new();
    query.push_str("// rsk schedulability query (R31.3): no deadline-miss location is reachable.\n");
    query.push_str(&format!(
        "A[] not ({})\n",
        (0..n).map(|i| format!("T{i}.Missed")).collect::<Vec<_>>().join(" || ")
    ));
    (xml, query)
}

/// Replaces exactly `n` occurrences of `from`; false, with `s` untouched,
/// when the count differs.
fn swap(s: &mut String, from: &str, to: &str, n: usize) -> bool {
    if s.matches(from).count() != n {
        return false;
    }
    *s = s.replace(from, to);
    true
}

/// The confirmation model of an export (R31.5, 2026-11-21): the same
/// automata without stopwatches, and its two queries. `None` when the text
/// is not an export of this version.
///
/// verifyta checks a model with stopwatches by over-approximation, so a
/// miss it finds there "may" be reachable and comes without a trace. This
/// model has ordinary clocks only, which verifyta checks exactly, and it
/// admits every behaviour of the export:
///
/// - `c[i]` is the wall clock since the Job of Task `i` started. `W[i]` is
///   `EXEC[i]` plus `EXEC[j]` for every Job that started while the Job of
///   `i` was started (`begin`). A started Job is delayed only by the Jobs
///   that start above it, each for at most its `EXEC`, so at its
///   completion in the export `c[i] <= W[i]`: the model lets the Job
///   complete at any instant it is the executing one, within `c[i] <=
///   W[i]`.
/// - A Job the export judges missed while `Running` has executed for less
///   than `OWN[i]`, so its `c[i]` is below `OWN[i]` plus the same sum,
///   `W[i] - EXEC[i] + OWN[i]`: the model's guard.
/// - `W` is capped at one above the largest value either comparison can
///   tell from a larger one (`c[i]` never exceeds `DEADLINE[i]` in
///   `Running`), which bounds the state space and changes no comparison.
///
/// What the model adds to the export's behaviours is a Job running on in
/// time a Job above it left unused. So no miss here means no miss in the
/// export; a miss here may be that addition's. The second query checks
/// that the model's invariants never stop time short of a miss, without
/// which "no miss" could be an artefact of a blocked state.
pub fn confirmation_of(xml: &str, query: &str) -> Option<(String, String)> {
    let tasks = read_export(xml).ok()?;
    let dmax = tasks.iter().map(|t| t.deadline_cycles).max()?;
    let emax = tasks.iter().map(|t| t.exec_cycles).max()?;
    let wmax = dmax.checked_add(emax)?.checked_add(1)?;
    let big = wmax.checked_mul(2)?;
    if big > i32::MAX as u64 {
        return None;
    }
    let mut x = xml.to_string();
    let line_start = x.find("clock exec[N];")?;
    let line_end = line_start + x[line_start..].find('\n')?;
    let decl = format!(
        "clock c[N];               // wall clock since the Job started\n\
         typedef int[0,{big}] big_t;\n\
         int[0,{wmax}] W[N];       // bound on c[i] at completion (EXEC of the Job and of every Job started while it was started), capped\n\
         big_t cap(task_t k, big_t v) {{ return v &gt; DEADLINE[k] + EXEC[k] - OWN[k] ? DEADLINE[k] + 1 + EXEC[k] - OWN[k] : v; }}\n\
         void begin(task_t i) {{ for (k : task_t) {{ if (started[k] &amp;&amp; k != i) {{ W[k] = cap(k, W[k] + EXEC[i]); }} }} W[i] = cap(i, EXEC[i]); running = i; started[i] = true; }}"
    );
    x.replace_range(line_start..line_end, &decl);
    let ok = swap(&mut x, "rsk UPPAAL export (rsk-uppaal/4).", "rsk UPPAAL confirmation model (rsk-uppaal/4, stopwatch-free over-approximation of the export).", 1)
        && swap(&mut x, "exec[i]' == (running == i) &amp;&amp; exec[i] &lt;= EXEC[i] &amp;&amp; dl[i] &lt;= DEADLINE[i]", "c[i] &lt;= W[i] &amp;&amp; dl[i] &lt;= DEADLINE[i]", 1)
        && swap(&mut x, "running = i, started[i] = true, exec[i] = 0", "begin(i), c[i] = 0", 2)
        && swap(&mut x, "<label kind=\"guard\">exec[i] &lt;= EXEC[i] &amp;&amp; running == i</label>", "<label kind=\"guard\">running == i</label>", 1)
        && swap(&mut x, "dl[i] &gt;= DEADLINE[i] &amp;&amp; exec[i] &lt; OWN[i]", "dl[i] &gt;= DEADLINE[i] &amp;&amp; c[i] &lt; W[i] - (EXEC[i] - OWN[i])", 1);
    if !ok || x.contains("exec[") {
        return None;
    }
    let start = query.find("A[] not (")? + "A[] not (".len();
    let end = start + query[start..].find(")\n")?;
    let missed = &query[start..end];
    let q = format!(
        "// rsk confirmation queries (R31.5): no deadline-miss location is reachable in the stopwatch-free over-approximation,\n// and its invariants stop time only after a miss.\nA[] not ({missed})\nA[] (deadlock imply ({missed}))\n"
    );
    Some((x, q))
}

/// The approximations of the exported model, for the report's omitted
/// terms (R31.2).
pub fn omissions() -> Vec<String> {
    vec![
        "UPPAAL export: Critical_Sections and the System_Ceiling are approximated by the blocking interval B_i spent before each Job starts, not by locations that raise the ceiling".to_string(),
        "UPPAAL export: the Kernel overheads of R29.3 are folded into each Task's execution-time bound E_i, not modelled as locations".to_string(),
    ]
}

fn decl_array(decl: &str, name: &str) -> Option<Vec<u64>> {
    let marker = format!("const int {name}[N] = {{ ");
    let start = decl.find(&marker)? + marker.len();
    let end = decl[start..].find(" }")? + start;
    decl[start..end].split(", ").map(|x| x.trim().parse().ok()).collect()
}

/// Recovers the exported parameters (R31.8).
pub fn read_export(xml: &str) -> Result<Vec<Exported>, String> {
    let start = xml.find("<declaration>").ok_or("no <declaration>")? + "<declaration>".len();
    let end = xml[start..].find("</declaration>").ok_or("unterminated <declaration>")? + start;
    let decl = xml[start..end].replace("&lt;", "<").replace("&gt;", ">").replace("&quot;", "\"").replace("&amp;", "&");
    let prio = decl_array(&decl, "PRIO").ok_or("no PRIO table")?;
    let period = decl_array(&decl, "PERIOD").ok_or("no PERIOD table")?;
    let deadline = decl_array(&decl, "DEADLINE").ok_or("no DEADLINE table")?;
    let exec = decl_array(&decl, "EXEC").ok_or("no EXEC table")?;
    let jitter = decl_array(&decl, "JITTER").ok_or("no JITTER table")?;
    let block = decl_array(&decl, "BLOCK").ok_or("no BLOCK table")?;
    let sporadic: Vec<bool> = {
        let marker = "const bool SPORADIC[N] = { ";
        let start = decl.find(marker).ok_or("no SPORADIC table")? + marker.len();
        let end = decl[start..].find(" }").ok_or("unterminated SPORADIC table")? + start;
        decl[start..end].split(", ").map(|x| x.trim() == "true").collect()
    };
    let ids: Vec<String> = decl
        .lines()
        .filter_map(|l| l.strip_prefix("// "))
        .filter_map(|l| l.split_once(": "))
        .filter(|(i, _)| i.parse::<usize>().is_ok())
        .map(|(_, id)| id.to_string())
        .collect();
    let n = prio.len();
    if [period.len(), deadline.len(), exec.len(), jitter.len(), block.len(), sporadic.len(), ids.len()].iter().any(|&l| l != n) {
        return Err("table lengths differ".into());
    }
    Ok((0..n)
        .map(|i| Exported {
            id: ids[i].clone(),
            priority: prio[i] as u8,
            period_cycles: period[i],
            deadline_cycles: deadline[i],
            exec_cycles: exec[i],
            jitter_cycles: jitter[i],
            blocking_cycles: block[i],
            sporadic: sporadic[i],
        })
        .collect())
}

/// The cross-check outcome (R31.4 to R31.6, R31.9).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CrossCheck {
    pub status: &'static str,
    pub detail: String,
}

/// Runs `verifyta` when installed, under a time limit and, on Unix, an
/// address-space limit of `memory_mib` (R31.9; `ulimit -v`). `rta_pass`
/// is the RTA verdict; the verifier's version is recorded in the detail
/// (R31.4). `confirm` is the confirmation model and its queries
/// ([`confirmation_of`]), run only when the export's check ends in a
/// possible miss while the RTA reports PASS.
pub fn cross_check(xml_path: &std::path::Path, q_path: &std::path::Path, confirm: Option<(&std::path::Path, &std::path::Path)>, rta_pass: bool, timeout_secs: u64, memory_mib: u64) -> CrossCheck {
    let Some(verifyta) = ["verifyta", "verifyta.sh"].iter().find_map(|name| which(name)) else {
        return CrossCheck {
            status: "not_run",
            detail: "UPPAAL's verifyta is not installed; the export was written for an offline cross-check".into(),
        };
    };
    let version = std::process::Command::new(&verifyta).arg("-v").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string()).unwrap_or_default();
    let out = match run_verifyta(&verifyta, xml_path, q_path, timeout_secs, memory_mib) {
        Ok(out) => out,
        Err(detail) => return CrossCheck { status: "inconclusive", detail },
    };
    let satisfied = out.contains("is satisfied");
    let not_satisfied = out.contains("NOT satisfied");
    // With stopwatches verifyta checks an over-approximation: a property it
    // reports satisfied holds, and a violation it finds only there is
    // reported as "MAY NOT be satisfied", with no trace.
    let may_not = out.contains("MAY NOT be satisfied");
    if may_not && !satisfied && !not_satisfied {
        return if rta_pass {
            // The possible miss is put to the stopwatch-free model, which
            // verifyta checks exactly: no miss there, with time never
            // stopped short of one, refutes it.
            let confirmation = match confirm {
                None => "no confirmation model was given".to_string(),
                Some((cx, cq)) => match run_verifyta(&verifyta, cx, cq, timeout_secs, memory_mib) {
                    Err(detail) => format!("the confirmation model gave no verdict ({detail})"),
                    Ok(out) => {
                        let holds = out.matches("Formula is satisfied").count();
                        if holds == 2 && !out.contains("NOT") {
                            return CrossCheck {
                                status: "agree",
                                detail: format!("verifyta {version}: a deadline miss may be reachable in the stopwatch over-approximation; none is reachable in the stopwatch-free over-approximation, which is checked exactly (the confirmation model, R31.5); RTA PASS"),
                            };
                        }
                        if out.contains("Formula is NOT satisfied") {
                            "the stopwatch-free over-approximation reaches a miss too (trace: `verifyta -t0` on the confirmation model), which may be an artefact of either approximation".to_string()
                        } else {
                            "the confirmation model gave no verdict".to_string()
                        }
                    }
                },
            };
            CrossCheck {
                status: "discrepancy",
                detail: format!("verifyta {version}: a deadline miss may be reachable while RTA reports PASS (found in the stopwatch over-approximation, no trace given); {confirmation}; REQUIRES CONFIRMATION (R31.5)"),
            }
        } else {
            CrossCheck { status: "agree", detail: format!("verifyta {version}: a deadline miss may be reachable (found in the stopwatch over-approximation); RTA FAIL") }
        };
    }
    match (rta_pass, satisfied, not_satisfied) {
        (true, true, _) => CrossCheck { status: "agree", detail: format!("verifyta {version}: no deadline miss reachable; RTA PASS") },
        (false, _, true) => CrossCheck { status: "agree", detail: format!("verifyta {version}: a deadline miss is reachable; RTA FAIL") },
        (false, true, _) => CrossCheck { status: "pessimism", detail: format!("verifyta {version}: no deadline miss reachable while RTA reports FAIL (analysis pessimism, R31.6)") },
        (true, _, true) => CrossCheck {
            status: "discrepancy",
            detail: format!("verifyta {version}: a deadline miss is reachable while RTA reports PASS; the trace REQUIRES CONFIRMATION (over-approximation of stopwatches, R31.5):\n{out}"),
        },
        _ => CrossCheck { status: "inconclusive", detail: format!("verifyta {version} gave no verdict (a memory limit of {memory_mib} MiB applied; R31.9):\n{out}") },
    }
}

/// One run of `verifyta -t0` under the limits: its output, or why there is
/// none.
fn run_verifyta(verifyta: &std::path::Path, xml_path: &std::path::Path, q_path: &std::path::Path, timeout_secs: u64, memory_mib: u64) -> Result<String, String> {
    let start = std::time::Instant::now();
    // `-t0`: the shortest trace on a reachable miss (R31.5). The memory
    // limit is applied by the shell's `ulimit -v` (kibibytes) where a
    // POSIX shell exists; elsewhere only the time limit applies.
    let mut command = if cfg!(unix) {
        let mut c = std::process::Command::new("/bin/sh");
        c.arg("-c").arg(format!("ulimit -v {} 2>/dev/null; exec \"$0\" -t0 \"$1\" \"$2\"", memory_mib.saturating_mul(1024))).arg(verifyta).arg(xml_path).arg(q_path);
        c
    } else {
        let mut c = std::process::Command::new(verifyta);
        c.arg("-t0").arg(xml_path).arg(q_path);
        c
    };
    let mut child = command.stdout(std::process::Stdio::piped()).stderr(std::process::Stdio::piped()).spawn().map_err(|e| format!("cannot run verifyta: {e}"))?;
    // The pipes are drained while the verifier runs: a trace can exceed a
    // pipe's buffer, and a full pipe would stall the verifier until the
    // time limit.
    let mut stdout = child.stdout.take();
    let mut stderr = child.stderr.take();
    let read = |pipe: Option<&mut dyn std::io::Read>| -> String {
        let mut text = Vec::new();
        if let Some(p) = pipe {
            let _ = p.read_to_end(&mut text);
        }
        String::from_utf8_lossy(&text).into_owned()
    };
    let out_thread = std::thread::spawn(move || read(stdout.as_mut().map(|p| p as &mut dyn std::io::Read)));
    let err_thread = std::thread::spawn(move || read(stderr.as_mut().map(|p| p as &mut dyn std::io::Read)));
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed().as_secs() > timeout_secs => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(format!("verifyta exceeded the {timeout_secs}s limit (memory limit {memory_mib} MiB; ORQ-22, R31.9)"));
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => return Err(format!("verifyta failed: {e}")),
        }
    }
    Ok(format!("{}{}", out_thread.join().unwrap_or_default(), err_thread.join().unwrap_or_default()))
}

fn which(name: &str) -> Option<std::path::PathBuf> {
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path).map(|p| p.join(name)).find(|p| p.is_file())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R31.8, R31.3: the reader recovers each Task's Priority, period, deadline
    /// and execution-time bound from the export, and the query is the
    /// no-deadline-miss query.
    #[test]
    fn export_round_trips_the_parameters() {
        let tasks = vec![
            AnalysedTask { id: "p.a".into(), partition: "p".into(), level: "A".into(), priority: 7, period_cycles: 1000, deadline_cycles: 900, c: 100, b: 20, b_source: "L_kernel".into(), o: 5, j: 3, e: 105, intake: 0, events: 0, hk_period: 1, hk_cost: 0, stops_overruns: true, sporadic: true, longest_cs: 10, endpoint_cost: 0, source_interrupt: false, source_endpoint: None, defer: false, wcet_provenance: vec![], wcet: 90 },
            AnalysedTask { id: "p.b".into(), partition: "p".into(), level: "A".into(), priority: 5, period_cycles: 2000, deadline_cycles: 2000, c: 300, b: 0, b_source: "L_kernel".into(), o: 5, j: 3, e: 305, intake: 0, events: 0, hk_period: 1, hk_cost: 0, stops_overruns: true, sporadic: false, longest_cs: 0, endpoint_cost: 0, source_interrupt: false, source_endpoint: None, defer: false, wcet_provenance: vec![], wcet: 290 },
        ];
        let (xml, q) = export(&tasks);
        assert!(q.contains("A[] not (T0.Missed || T1.Missed)"));
        let back = read_export(&xml).unwrap();
        assert_eq!(back.len(), 2);
        assert_eq!((back[0].id.as_str(), back[0].priority, back[0].period_cycles, back[0].deadline_cycles, back[0].exec_cycles), ("p.a", 7, 1000, 900, 105));
        assert_eq!((back[1].jitter_cycles, back[1].blocking_cycles), (3, 0));
        assert_eq!((back[0].sporadic, back[1].sporadic), (true, false));
        // R31.2: the model's structure. A sporadic generator releases at or
        // after the MIT and a periodic one exactly at the period; the
        // executing Task is resumed after a preemption; execution and
        // blocking are intervals; every automaton names its Task (R31.7).
        assert!(xml.contains("SPORADIC[i] || x &lt;= PERIOD[i]") && xml.contains("x &gt;= PERIOD[i]"));
        assert!(xml.contains("running = resume()") && xml.contains("int resume()"));
        assert!(xml.contains("exec[i] &lt;= EXEC[i] &amp;&amp; running == i") && xml.contains("b &lt;= BLOCK[i] &amp;&amp; preempts(i, running)"));
        assert!(xml.contains("// 0: p.a") && xml.contains("// 1: p.b"));
        assert_eq!(omissions().len(), 2);
        // 2026-11-21: a running Job is judged at its deadline by its own
        // demand C + O (here 105 and 305, equal to E), not by E.
        assert!(xml.contains("const int OWN[N] = { 105, 305 };") && xml.contains("exec[i] &lt; OWN[i]"));
    }

    /// The Task's own demand is C + O even where E carries an overrun
    /// allowance, and never above E.
    #[test]
    fn own_demand_is_the_budget_and_overhead_below_e() {
        let task = |c: u64, o: u64, e: u64| AnalysedTask { id: "p.a".into(), partition: "p".into(), level: "A".into(), priority: 7, period_cycles: 1000, deadline_cycles: 900, c, b: 0, b_source: "L_kernel".into(), o, j: 0, e, intake: 0, events: 0, hk_period: 1, hk_cost: 0, stops_overruns: true, sporadic: false, longest_cs: 0, endpoint_cost: 0, source_interrupt: false, source_endpoint: None, defer: false, wcet_provenance: vec![], wcet: 0 };
        let (xml, _) = export(&[task(320, 36, 682), task(100, 5, 90)]);
        assert!(xml.contains("const int EXEC[N] = { 682, 90 };") && xml.contains("const int OWN[N] = { 356, 90 };"), "{xml}");
    }

    /// R31.5: the confirmation model has no stopwatch, keeps the export's
    /// parameters and asks both queries; a text that is not an export has
    /// none.
    #[test]
    fn the_confirmation_model_is_stopwatch_free() {
        let task = |id: &str, priority: u8, e: u64| AnalysedTask { id: id.into(), partition: "p".into(), level: "A".into(), priority, period_cycles: 1000, deadline_cycles: 900, c: 100, b: 20, b_source: "L_kernel".into(), o: 5, j: 3, e, intake: 0, events: 0, hk_period: 1, hk_cost: 0, stops_overruns: true, sporadic: false, longest_cs: 10, endpoint_cost: 0, source_interrupt: false, source_endpoint: None, defer: false, wcet_provenance: vec![], wcet: 90 };
        let (xml, q) = export(&[task("p.a", 7, 150), task("p.b", 5, 105)]);
        let (cx, cq) = confirmation_of(&xml, &q).expect("an export has a confirmation model");
        assert!(!cx.contains("exec[") && !cx.contains("' =="), "{cx}");
        assert!(cx.contains("int[0,1051] W[N];") && cx.contains("typedef int[0,2102] big_t;"));
        assert!(cx.contains("begin(i), c[i] = 0") && cx.contains("c[i] &lt; W[i] - (EXEC[i] - OWN[i])") && cx.contains("c[i] &lt;= W[i] &amp;&amp; dl[i] &lt;= DEADLINE[i]"));
        assert_eq!(read_export(&cx).unwrap(), read_export(&xml).unwrap());
        assert!(cq.contains("A[] not (T0.Missed || T1.Missed)\n") && cq.contains("A[] (deadlock imply (T0.Missed || T1.Missed))\n"));
        assert!(confirmation_of("<nta></nta>", &q).is_none());
        assert!(confirmation_of(&cx, &q).is_none(), "a confirmation model is not an export");
    }
}
