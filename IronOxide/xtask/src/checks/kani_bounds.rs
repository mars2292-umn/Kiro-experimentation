//! R45.8: the bounds of the Kani_Harnesses cover the Generated_Config
//! sizes of the Profile_Conformance_Suite Application, the P1 demo.
//!
//! The logic harnesses (`crates/rsk-logic/src/logic/kani_harness.rs`) run
//! the Kernel at the sizes `NT`, `NR`, `NP`, `NL`, `NS`, `NE` and on the
//! configuration their `config()` builds. This check reads the demo's
//! Generated_Config (`apps/p1-demo/gen/system_manifest.rs`) and requires
//! that each size of the harness is at least the demo's and that every
//! line of the demo's `CONFIG` literal (each Task, Resource, access row,
//! Partition, the operating duration, and the slot tables) appears in the
//! harness file with the `::rsk_kernel::logic::` paths stripped, so the
//! harness configuration is the demo's and a regenerated demo that the
//! harness no longer matches fails `cargo xtask check`.

use std::path::Path;

use crate::diag::{Report, Rule};

pub const HARNESS: &str = "crates/rsk-logic/src/logic/kani_harness.rs";
pub const MANIFEST: &str = "apps/p1-demo/gen/system_manifest.rs";
const SIZES: [&str; 6] = ["NT", "NR", "NP", "NL", "NS", "NE"];

pub fn check(root: &Path, report: &mut Report) {
    let (Ok(harness), Ok(manifest)) = (
        std::fs::read_to_string(root.join(HARNESS)),
        std::fs::read_to_string(root.join(MANIFEST)),
    ) else {
        report.error(Rule::KaniBounds, Some(HARNESS.to_string()), "the harness or the demo's Generated_Config is missing");
        return;
    };
    for d in compare(&harness, &manifest) {
        report.error(Rule::KaniBounds, Some(HARNESS.to_string()), d);
    }
    report.note(format!("R45.8: the logic harness bounds and configuration match {MANIFEST}"));
}

/// The value of `const NAME: usize = n;` (with or without `pub`) in a source.
fn size(src: &str, name: &str) -> Option<usize> {
    src.lines().find_map(|l| {
        let t = l.trim().trim_start_matches("pub ");
        let rest = t.strip_prefix("const ")?.strip_prefix(name)?.strip_prefix(": usize = ")?;
        rest.trim_end_matches(';').trim().parse().ok()
    })
}

/// The lines of the manifest's `CONFIG` literal that the harness must
/// carry, with the generated paths stripped.
fn config_lines(manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut inside = false;
    for l in manifest.lines() {
        if l.starts_with("pub const CONFIG:") {
            inside = true;
            continue;
        }
        if inside && l.starts_with("};") {
            break;
        }
        if !inside {
            continue;
        }
        let t = l.trim().replace("::rsk_kernel::logic::", "");
        if t.starts_with("TaskCfg {")
            || t.starts_with("ResourceCfg {")
            || t.starts_with("PartitionCfg {")
            || t.starts_with('[')
            || t.starts_with("operating_duration:")
            || t.starts_with("slot_")
        {
            out.push(t);
        }
    }
    out
}

/// The differences between the harness and the manifest: each a message.
pub fn compare(harness: &str, manifest: &str) -> Vec<String> {
    let mut out = Vec::new();
    for name in SIZES {
        match (size(harness, name), size(manifest, name)) {
            (Some(h), Some(m)) if h < m => out.push(format!("harness bound {name} = {h} is below the demo's {name} = {m} (R45.8)")),
            (Some(_), Some(_)) => {}
            (h, m) => out.push(format!("size {name} not found (harness: {h:?}, demo: {m:?})")),
        }
    }
    let lines = config_lines(manifest);
    if lines.len() < 10 {
        out.push(format!("the demo's CONFIG literal has {} recognizable lines; expected its Tasks, Resources, access rows, Partitions, duration, and slots", lines.len()));
    }
    let harness_lines: Vec<String> = harness.lines().map(|l| l.trim().to_string()).collect();
    for l in lines {
        if !harness_lines.iter().any(|h| h == &l) {
            out.push(format!("the harness configuration lacks the demo's line `{l}`"));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const MANIFEST: &str = "\
pub const NT: usize = 2;
pub const NR: usize = 1;
pub const NP: usize = 1;
pub const NL: usize = 2;
pub const NS: usize = 8;
pub const NE: usize = 4;
pub const CONFIG: ::rsk_kernel::logic::Config<NT, NR, NP> = ::rsk_kernel::logic::Config {
    tasks: [
        ::rsk_kernel::logic::TaskCfg { priority: 7, partition: 0, kind: ::rsk_kernel::logic::KIND_PERIODIC, offset: 0, period: 10, deadline: 10, budget: 1, mit_policy: ::rsk_kernel::logic::MIT_DEFER, fpu: false }, // a
    ],
    resources: [
        ::rsk_kernel::logic::ResourceCfg { ceiling: 7, partition: 0 }, // r
    ],
    access: [
        [true],
        [false],
    ],
    partitions: [
        ::rsk_kernel::logic::PartitionCfg { level: 0, fault_response: ::rsk_kernel::logic::RESP_RESTART_PARTITION, overrun_response: ::rsk_kernel::logic::RESP_END_JOB, deadline_response: ::rsk_kernel::logic::RESP_RECORD_ONLY, mit_threshold: 10 }, // p
    ],
    operating_duration: 100,
    slot_release: [0, 3],
    slot_deadline: [1, 4],
    slot_budget: [2, 5],
    slot_count: NS as u8,
};
pub const LAYOUT: u32 = 0;
";

    const HARNESS: &str = "\
const NT: usize = 2;
const NR: usize = 1;
const NP: usize = 1;
const NL: usize = 2;
const NS: usize = 8;
const NE: usize = 4;
fn config() -> Config<NT, NR, NP> {
    Config {
        tasks: [
            TaskCfg { priority: 7, partition: 0, kind: KIND_PERIODIC, offset: 0, period: 10, deadline: 10, budget: 1, mit_policy: MIT_DEFER, fpu: false }, // a
        ],
        resources: [
            ResourceCfg { ceiling: 7, partition: 0 }, // r
        ],
        access: [
            [true],
            [false],
        ],
        partitions: [
            PartitionCfg { level: 0, fault_response: RESP_RESTART_PARTITION, overrun_response: RESP_END_JOB, deadline_response: RESP_RECORD_ONLY, mit_threshold: 10 }, // p
        ],
        operating_duration: 100,
        slot_release: [0, 3],
        slot_deadline: [1, 4],
        slot_budget: [2, 5],
        slot_count: NS as u8,
    }
}
";

    /// R45.8: a harness at the demo's sizes carrying the demo's configuration passes; a smaller bound or a changed Task line is reported.
    #[test]
    fn matching_bounds_and_configuration_pass_and_differences_are_named() {
        assert!(compare(HARNESS, MANIFEST).is_empty());
        let smaller = HARNESS.replace("const NT: usize = 2;", "const NT: usize = 1;");
        let d = compare(&smaller, MANIFEST);
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("NT = 1 is below the demo's NT = 2"));
        let changed = HARNESS.replace("budget: 1,", "budget: 2,");
        let d = compare(&changed, MANIFEST);
        assert_eq!(d.len(), 1);
        assert!(d[0].contains("lacks the demo's line `TaskCfg { priority: 7"));
    }

    /// R45.8: the repository's harness matches the repository's demo.
    #[test]
    fn repository_harness_matches_the_demo() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
        let harness = std::fs::read_to_string(root.join(super::HARNESS)).unwrap();
        let manifest = std::fs::read_to_string(root.join(super::MANIFEST)).unwrap();
        let d = compare(&harness, &manifest);
        assert!(d.is_empty(), "{d:?}");
    }
}
