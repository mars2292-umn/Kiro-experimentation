//! The loop-iteration report of a measurement run (task 19.2; R34.9,
//! R35.3, R35.4): the Profile_Lint's export `rsk-loop-bounds/1` names
//! every Application loop with its bound; the measurement build keeps, per
//! loop, the largest iteration count a run reached and the safe state
//! reports it as `loop:<partition>:<function>:<label>=<count>`. A loop
//! whose observed count exceeds its bound fails the run (R35.4); a loop
//! below its bound passes only when the rig's description records the
//! bound as unreachable with a justification (`[loops] unreachable`,
//! R34.9); a loop the configuration did not compile or run is listed as
//! not observed. The result is written as `rsk-loop-iterations/1`.

use std::path::Path;

use serde::{Deserialize, Serialize};

pub const SCHEMA: &str = "rsk-loop-iterations/1";

/// A recorded unreachable bound (`hil-rig.toml`, `[loops] unreachable`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Unreachable {
    pub key: String,
    pub reason: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    /// `loop:<partition>:<function>:<label>` (empty label: `-`).
    pub key: String,
    pub file: String,
    pub line: usize,
    pub function: String,
    pub label: Option<String>,
    /// The Loop_Bound_Annotation as exported, and its value when numeric.
    pub bound: Option<String>,
    pub bound_value: Option<u64>,
    /// The largest iteration count the run observed, when reported.
    pub observed: Option<u64>,
    /// `reached`, `exceeded`, `unreachable`, `below`, `not observed`,
    /// `not run` (reported 0: the configuration never completed the loop,
    /// as when the Overrun_Response ends every Job that runs it), or
    /// `symbolic` (a bound the rig cannot evaluate).
    pub verdict: String,
    pub justification: Option<String>,
}

impl Row {
    pub fn ok(&self) -> bool {
        !matches!(self.verdict.as_str(), "exceeded" | "below")
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema: String,
    pub configuration: String,
    pub binary_hash: String,
    pub loops: Vec<Row>,
}

impl Report {
    pub fn ok(&self) -> bool {
        self.loops.iter().all(Row::ok) && self.loops.iter().any(|r| r.observed.is_some())
    }

    pub fn observed(&self) -> usize {
        self.loops.iter().filter(|r| r.observed.is_some()).count()
    }

    pub fn summary(&self) -> String {
        let count = |v: &str| self.loops.iter().filter(|r| r.verdict == v).count();
        format!(
            "{} Application loops in the export, {} observed: {} reached the bound, {} exceeded, {} below the bound with the justification recorded, {} below without one, {} never completed in this configuration, {} not observed{}",
            self.loops.len(),
            self.observed(),
            count("reached"),
            count("exceeded"),
            count("unreachable"),
            count("below"),
            count("not run"),
            count("not observed"),
            if self.observed() == 0 { " (the build reports no loop: not a measurement build?)" } else { "" }
        )
    }

    pub fn render(&self) -> String {
        let mut s = String::from("| Loop | File:line | Bound | Observed | Verdict |\n|---|---|---|---|---|\n");
        for r in &self.loops {
            s.push_str(&format!(
                "| `{}` | {}:{} | {} | {} | {}{} |\n",
                r.key,
                r.file,
                r.line,
                r.bound.as_deref().unwrap_or("-"),
                r.observed.map(|o| o.to_string()).unwrap_or_else(|| "-".into()),
                r.verdict,
                r.justification.as_deref().map(|j| format!(": {j}")).unwrap_or_default()
            ));
        }
        s
    }

    pub fn print(&self) -> String {
        serde_json::to_string_pretty(self).unwrap_or_default() + "\n"
    }
}

/// One loop of the export.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exported {
    pub file: String,
    pub line: usize,
    pub function: String,
    pub label: Option<String>,
    pub bound: Option<String>,
}

/// The Application loops of an `rsk-loop-bounds/1` export: those of files
/// under `apps/<app>/<partition>/`, with their Partition.
pub fn read_export(text: &str) -> Result<Vec<(String, Exported)>, String> {
    let v: serde_json::Value = serde_json::from_str(text).map_err(|e| format!("not JSON: {e}"))?;
    if v.get("schema").and_then(|s| s.as_str()) != Some("rsk-loop-bounds/1") {
        return Err("not an rsk-loop-bounds/1 export".into());
    }
    let mut out = Vec::new();
    for l in v.get("loops").and_then(|l| l.as_array()).into_iter().flatten() {
        let file = l.get("file").and_then(|f| f.as_str()).unwrap_or("").to_string();
        let Some(partition) = partition_of(&file) else { continue };
        out.push((
            partition,
            Exported {
                file,
                line: l.get("line").and_then(|x| x.as_u64()).unwrap_or(0) as usize,
                function: l.get("function").and_then(|f| f.as_str()).unwrap_or("").to_string(),
                label: l.get("label").and_then(|f| f.as_str()).map(|f| f.trim_start_matches('\'').to_string()),
                bound: l.get("bound").and_then(|f| f.as_str()).map(str::to_string),
            },
        ));
    }
    Ok(out)
}

/// The Partition crate directory of an Application source file
/// (`apps/<app>/<partition>/src/..`), else `None` (Kernel and tool code).
fn partition_of(file: &str) -> Option<String> {
    let parts: Vec<&str> = file.split(['/', '\\']).collect();
    let i = parts.iter().position(|p| *p == "apps")?;
    let partition = parts.get(i + 2)?;
    let dir = parts.get(i + 3)?;
    (*dir == "src" && !matches!(*partition, "system" | "decl" | "bin")).then(|| partition.to_string())
}

/// A numeric bound (`100_000_000`), else `None`.
pub fn bound_value(text: &str) -> Option<u64> {
    let clean: String = text.chars().filter(|c| *c != '_' && !c.is_whitespace()).collect();
    clean.parse().ok()
}

/// Judges the export against the run's report (`value`: the count the
/// safe state printed for a key, when any) and the recorded unreachable
/// bounds.
pub fn judge(export: &[(String, Exported)], value: &dyn Fn(&str) -> Option<u64>, unreachable: &[Unreachable], configuration: &str, binary_hash: &str) -> Report {
    let mut loops = Vec::new();
    for (partition, l) in export {
        let key = format!("loop:{partition}:{}:{}", l.function, l.label.as_deref().unwrap_or("-"));
        let bound_value = l.bound.as_deref().and_then(bound_value);
        let observed = value(&key);
        let recorded = unreachable.iter().find(|u| u.key == key);
        let (verdict, justification) = match (observed, bound_value) {
            (None, _) => ("not observed", None),
            (Some(0), _) => ("not run", None),
            (Some(_), None) => ("symbolic", None),
            (Some(o), Some(b)) if o > b => ("exceeded", None),
            (Some(o), Some(b)) if o == b => ("reached", None),
            (Some(_), Some(_)) => match recorded {
                Some(u) => ("unreachable", Some(u.reason.clone())),
                None => ("below", None),
            },
        };
        loops.push(Row {
            key,
            file: l.file.clone(),
            line: l.line,
            function: l.function.clone(),
            label: l.label.clone(),
            bound: l.bound.clone(),
            bound_value,
            observed,
            verdict: verdict.to_string(),
            justification,
        });
    }
    Report { schema: SCHEMA.into(), configuration: configuration.into(), binary_hash: binary_hash.into(), loops }
}

/// Reads an export file and judges it.
pub fn judge_file(path: &Path, value: &dyn Fn(&str) -> Option<u64>, unreachable: &[Unreachable], configuration: &str, binary_hash: &str) -> Result<Report, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    let export = read_export(&text).map_err(|e| format!("{}: {e}", path.display()))?;
    Ok(judge(&export, value, unreachable, configuration, binary_hash))
}

#[cfg(test)]
mod tests {
    use super::*;

    const EXPORT: &str = r#"{"schema":"rsk-loop-bounds/1","profile_version":"1","loops":[
        {"file":"apps/p1-demo/part0/src/lib.rs","line":34,"function":"spin_ms","label":"'spin","kind":"derived","bound":"100_000_000","proof":null},
        {"file":"apps/p1-demo/part0/src/lib.rs","line":232,"function":"sensor","label":"hold","kind":"derived","bound":"10_000_000","proof":null},
        {"file":"apps/p1-demo/part1/src/lib.rs","line":211,"function":"log","label":"busy","kind":"derived","bound":"10_000_000","proof":null},
        {"file":"apps/p1-demo/part2/src/lib.rs","line":40,"function":"filter","label":"sym","kind":"annotated","bound":"N * 2","proof":null},
        {"file":"apps/p1-demo/system/src/lib.rs","line":98,"function":"print_kv","label":"key","kind":"annotated","bound":"24","proof":null},
        {"file":"crates/rsk-kernel/src/logic/kernel.rs","line":10,"function":"k","label":null,"kind":"verus","bound":"NT","proof":null}
    ],"violations":[]}"#;

    #[test]
    fn judges_each_loop_against_its_bound_and_the_recorded_justifications() {
        let export = read_export(EXPORT).unwrap();
        assert_eq!(export.len(), 4, "the system crate's and the Kernel's loops are not Application loops");
        let value = |key: &str| match key {
            "loop:part0:spin_ms:spin" => Some(123_456),
            "loop:part0:sensor:hold" => Some(10_000_001),
            "loop:part1:log:busy" => Some(10_000_000),
            "loop:part2:filter:sym" => Some(7),
            _ => None,
        };
        assert_eq!(export[0].1.label.as_deref(), Some("spin"), "the lint exports the label with its quote");
        let unreachable = [Unreachable { key: "loop:part0:spin_ms:spin".into(), reason: "the time guard ends the loop first".into() }];
        let r = judge(&export, &value, &unreachable, "nrf52840_stress_short", "sha256:x");
        let verdicts: Vec<&str> = r.loops.iter().map(|l| l.verdict.as_str()).collect();
        assert_eq!(verdicts, ["unreachable", "exceeded", "reached", "symbolic"]);
        assert!(!r.ok());
        assert_eq!(r.loops[0].justification.as_deref(), Some("the time guard ends the loop first"));
        let r2 = judge(&export, &|k| if k == "loop:part0:sensor:hold" { Some(5) } else { value(k) }, &unreachable, "c", "h");
        assert_eq!(r2.loops[1].verdict, "below");
        assert!(!r2.ok());
        let r0 = judge(&export, &|k| if k == "loop:part0:sensor:hold" { Some(0) } else { value(k) }, &unreachable, "c", "h");
        assert_eq!(r0.loops[1].verdict, "not run");
        let r3 = judge(&export, &|k| if k == "loop:part0:sensor:hold" { Some(5) } else { value(k) }, &[unreachable[0].clone(), Unreachable { key: "loop:part0:sensor:hold".into(), reason: "same".into() }], "c", "h");
        assert!(r3.ok());
        let text = r3.print();
        let back: Report = serde_json::from_str(&text).unwrap();
        assert_eq!(back, r3);
        assert!(r3.render().contains("| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:211 | 10_000_000 | 10000000 | reached |"));
    }

    #[test]
    fn a_build_reporting_no_loop_fails() {
        let export = read_export(EXPORT).unwrap();
        let r = judge(&export, &|_| None, &[], "c", "h");
        assert!(r.loops.iter().all(|l| l.verdict == "not observed"));
        assert!(!r.ok());
        assert!(r.summary().contains("not a measurement build"));
    }
}
