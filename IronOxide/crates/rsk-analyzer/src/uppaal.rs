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
/// execution-time bound E_i of R29.1, Budget plus overheads), `JITTER`,
/// `BLOCK` (the SRP blocking term B_i), `SPORADIC`) and the Task_Model
/// identifier of each automaton as a comment (R31.7). Two templates:
///
/// - `Gen(i)`: the release generator. Periodic: exactly every `PERIOD[i]`.
///   Sporadic: at least `PERIOD[i]` (the MIT) after the previous release,
///   any later instant. Each release is followed by a jitter of up to
///   `JITTER[i]` before the Task becomes pending (R31.2).
/// - `Task(i)`: `Idle`, `Pending` (released, waiting for the SRP start
///   condition `eligible(i)`: no Job of higher or equal Priority and lower
///   identifier executing, Priority above the System_Ceiling), `Blocked`
///   (the blocking term, an interval up to `BLOCK[i]`), `Running` (a
///   stopwatch `exec[i]` that progresses only while the Task executes;
///   the Job completes at any point of the interval up to `EXEC[i]`), and
///   `Missed`, entered from any busy location when the deadline passes.
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
    decl.push_str("// rsk UPPAAL export (rsk-uppaal/2). Times in CPU cycles.\n");
    decl.push_str(&format!("const int N = {n};\n"));
    decl.push_str("typedef int[0,N-1] task_t;\n");
    let col = |name: &str, f: &dyn Fn(&AnalysedTask) -> u64| {
        format!("const int {name}[N] = {{ {} }};\n", tasks.iter().map(|t| f(t).to_string()).collect::<Vec<_>>().join(", "))
    };
    decl.push_str(&col("PRIO", &|t| t.priority as u64));
    decl.push_str(&col("PERIOD", &|t| t.period_cycles));
    decl.push_str(&col("DEADLINE", &|t| t.deadline_cycles));
    decl.push_str(&col("EXEC", &|t| t.e));
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
    decl.push_str("hybrid clock exec[N];     // stopwatches: progress only while running\n");
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
<transition><source ref="g0"/><target ref="g1"/><label kind="guard">x &gt;= PERIOD[i]</label><label kind="assignment">x = 0, jit = 0</label></transition>
<transition><source ref="g1"/><target ref="g0"/><label kind="synchronisation">release[i]!</label><label kind="assignment">pending[i] = true</label></transition>
</template>
"#,
    );
    // Task automaton: Idle -> Pending -> Blocked -> Running -> Idle; Missed on deadline.
    templates.push_str(
        r#"<template><name>Task</name><parameter>const task_t i</parameter>
<declaration>clock d; clock b;</declaration>
<location id="t0" x="0" y="0"><name>Idle</name></location>
<location id="t1" x="200" y="0"><name>Pending</name><label kind="invariant">d &lt;= DEADLINE[i]</label></location>
<location id="t2" x="400" y="0"><name>Blocked</name><label kind="invariant">b &lt;= BLOCK[i] &amp;&amp; d &lt;= DEADLINE[i]</label></location>
<location id="t3" x="600" y="0"><name>Running</name><label kind="invariant">exec[i]' == (running == i) &amp;&amp; exec[i] &lt;= EXEC[i] &amp;&amp; d &lt;= DEADLINE[i]</label></location>
<location id="t4" x="800" y="0"><name>Missed</name></location>
<init ref="t0"/>
<transition><source ref="t0"/><target ref="t1"/><label kind="synchronisation">release[i]?</label><label kind="assignment">d = 0</label></transition>
<transition><source ref="t1"/><target ref="t2"/><label kind="guard">eligible(i)</label><label kind="assignment">b = 0, pending[i] = false</label></transition>
<transition><source ref="t2"/><target ref="t3"/><label kind="guard">b &lt;= BLOCK[i] &amp;&amp; preempts(i, running)</label><label kind="assignment">running = i, started[i] = true, exec[i] = 0</label></transition>
<transition><source ref="t3"/><target ref="t0"/><label kind="guard">exec[i] &lt;= EXEC[i] &amp;&amp; running == i</label><label kind="assignment">started[i] = false, running = resume()</label></transition>
<transition><source ref="t1"/><target ref="t4"/><label kind="guard">d &gt;= DEADLINE[i]</label></transition>
<transition><source ref="t2"/><target ref="t4"/><label kind="guard">d &gt;= DEADLINE[i]</label></transition>
<transition><source ref="t3"/><target ref="t4"/><label kind="guard">d &gt;= DEADLINE[i]</label></transition>
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
/// (R31.4).
pub fn cross_check(xml_path: &std::path::Path, q_path: &std::path::Path, rta_pass: bool, timeout_secs: u64, memory_mib: u64) -> CrossCheck {
    let Some(verifyta) = ["verifyta", "verifyta.sh"].iter().find_map(|name| which(name)) else {
        return CrossCheck {
            status: "not_run",
            detail: "UPPAAL's verifyta is not installed; the export was written for an offline cross-check".into(),
        };
    };
    let version = std::process::Command::new(&verifyta).arg("-v").output().ok().map(|o| String::from_utf8_lossy(&o.stdout).lines().next().unwrap_or("").to_string()).unwrap_or_default();
    let start = std::time::Instant::now();
    // `-t0`: the shortest trace on a reachable miss (R31.5). The memory
    // limit is applied by the shell's `ulimit -v` (kibibytes) where a
    // POSIX shell exists; elsewhere only the time limit applies.
    let mut command = if cfg!(unix) {
        let mut c = std::process::Command::new("/bin/sh");
        c.arg("-c").arg(format!("ulimit -v {} 2>/dev/null; exec \"$0\" -t0 \"$1\" \"$2\"", memory_mib.saturating_mul(1024))).arg(&verifyta).arg(xml_path).arg(q_path);
        c
    } else {
        let mut c = std::process::Command::new(&verifyta);
        c.arg("-t0").arg(xml_path).arg(q_path);
        c
    };
    let mut child = match command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            return CrossCheck {
                status: "inconclusive",
                detail: format!("cannot run verifyta: {e}"),
            }
        }
    };
    loop {
        match child.try_wait() {
            Ok(Some(_)) => break,
            Ok(None) if start.elapsed().as_secs() > timeout_secs => {
                let _ = child.kill();
                return CrossCheck {
                    status: "inconclusive",
                    detail: format!("verifyta exceeded the {timeout_secs}s limit (memory limit {memory_mib} MiB; ORQ-22, R31.9)"),
                };
            }
            Ok(None) => std::thread::sleep(std::time::Duration::from_millis(100)),
            Err(e) => {
                return CrossCheck {
                    status: "inconclusive",
                    detail: format!("verifyta failed: {e}"),
                }
            }
        }
    }
    let out = child.wait_with_output().map(|o| format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr))).unwrap_or_default();
    let satisfied = out.contains("is satisfied");
    let not_satisfied = out.contains("NOT satisfied");
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
            AnalysedTask { id: "p.a".into(), partition: "p".into(), level: "A".into(), priority: 7, period_cycles: 1000, deadline_cycles: 900, c: 100, b: 20, b_source: "L_kernel".into(), o: 5, j: 3, e: 105, stops_overruns: true, sporadic: true, longest_cs: 10, wcet_provenance: vec![], wcet: 90 },
            AnalysedTask { id: "p.b".into(), partition: "p".into(), level: "A".into(), priority: 5, period_cycles: 2000, deadline_cycles: 2000, c: 300, b: 0, b_source: "L_kernel".into(), o: 5, j: 3, e: 305, stops_overruns: true, sporadic: false, longest_cs: 0, wcet_provenance: vec![], wcet: 290 },
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
    }
}
