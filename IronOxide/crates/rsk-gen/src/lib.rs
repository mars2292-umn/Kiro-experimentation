//! rsk-gen: the Generator core shared by the `rsk::app!` proc-macro and the
//! `rsk-gen` CLI (design.md, "App_Declaration and generation pipeline";
//! tasks 10.2 to 10.6).
//!
//! Pipeline: [`decl::Declaration`] (from the `syntax` parser or built by a
//! test) -> [`validate::validate`] (R15, R21, R22) -> [`derive::derive`]
//! (R23, R21.2, the MPU layout of R24.5) -> the outputs of R24 and R25:
//! the system crate manifest, one manifest per Partition crate, the linker
//! script and memory map, the configuration summary, and the Task_Model
//! JSON. Everything is a pure function of the declaration, so repeated runs
//! give byte-identical outputs (R24.4, R23.6).
#![forbid(unsafe_code)]

pub mod convert;
pub mod decl;
pub mod derive;
pub mod emit;
pub mod json;
pub mod layout;
pub mod model;
pub mod profile_version;
#[cfg(feature = "syntax")]
pub mod syntax;
pub mod target;
pub mod validate;

pub use decl::{Declaration, Diagnostic, ItemRef};
pub use derive::Generated;

/// Everything one declaration produces (R24.1, R24.3, R25.1).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Outputs {
    pub generated: Generated,
    pub system_source: String,
    /// (Partition name, manifest source), in declaration order.
    pub partition_sources: Vec<(String, String)>,
    pub memory_x: String,
    pub link_x: String,
    pub summary: String,
    pub task_model_json: String,
}

/// Validates and derives; the first error list is returned as a whole so
/// that the front end can report every violation at once (R21.4).
pub fn generate(decl: &Declaration) -> Result<Generated, Vec<Diagnostic>> {
    let target = validate::validate(decl)?;
    derive::derive(decl, target)
}

/// Generates every output; `system_crate` is the name of the crate that
/// expands the system manifest (for the linker script's archive pattern).
pub fn outputs(decl: &Declaration, system_crate: &str) -> Result<Outputs, Vec<Diagnostic>> {
    let generated = generate(decl)?;
    let partition_sources = generated
        .partitions
        .iter()
        .map(|p| (p.name.clone(), emit::partition_source(&generated, p.index)))
        .collect();
    Ok(Outputs {
        system_source: emit::system_source(&generated),
        partition_sources,
        memory_x: emit::memory_x(&generated),
        link_x: emit::link_x(&generated, system_crate),
        summary: emit::summary(&generated),
        task_model_json: model::task_model(&generated).render(),
        generated,
    })
}

#[cfg(test)]
mod tests {
    use super::decl::*;
    use super::*;

    fn ms(v: u64) -> Time {
        Time { value: v, unit: TimeUnit::Ms }
    }
    fn cycles(v: u64) -> Time {
        Time { value: v, unit: TimeUnit::Cycles }
    }

    fn periodic(name: &str, period_ms: u64, priority: u8, resources: &[&str]) -> TaskDecl {
        TaskDecl {
            name: name.into(),
            release: Release::Periodic { period: ms(period_ms), offset: None },
            deadline: ms(period_ms),
            budget: cycles(100_000),
            priority,
            core: 0,
            fpu: false,
            resources: resources.iter().map(|s| s.to_string()).collect(),
            raises: vec![],
        }
    }

    fn partition(name: &str, level: Level, tasks: Vec<TaskDecl>, resources: Vec<ResourceDecl>) -> PartitionDecl {
        PartitionDecl {
            name: name.into(),
            krate: format!("demo_{name}"),
            level,
            code: 32 * 1024,
            ram: 32 * 1024,
            stack: 4096,
            fault: Response::RestartPartition,
            overrun: Response::EndJob,
            deadline_miss: Response::RecordOnly,
            mit_threshold: 10,
            peripherals: vec![],
            resources,
            tasks,
            signals: vec![],
            init_arena: 0,
        }
    }

    fn resource(name: &str) -> ResourceDecl {
        ResourceDecl {
            name: name.into(),
            ty: "u32".into(),
            init: "0".into(),
            placement: ResourcePlacement::Core(0),
        }
    }

    /// The P1 demo as a declaration.
    pub fn demo() -> Declaration {
        Declaration {
            target: "qemu_mps2_an386".into(),
            variant: "Ravenscar".into(),
            operating_duration: ms(200),
            safe_state: "crate::report_and_exit".into(),
            log_capacity: 32,
            partitions: vec![
                partition(
                    "part0",
                    Level::A,
                    vec![periodic("control", 10, 7, &["state"]), periodic("sensor", 20, 5, &["state"])],
                    vec![resource("state")],
                ),
                partition("part1", Level::C, vec![periodic("log", 40, 2, &[])], vec![]),
            ],
        }
    }

    #[test]
    fn the_demo_generates_the_hand_written_tables() {
        let g = generate(&demo()).expect("valid");
        assert_eq!(g.tasks.len(), 3);
        assert_eq!(g.resources[0].ceiling, 7);
        assert_eq!(g.tasks[0].period_ticks, 250_000);
        assert_eq!(g.operating_duration_ticks, 5_000_000);
        assert_eq!(g.slot_count, 11);
        assert_eq!(g.lock_capacity, 2);
        assert_eq!((g.tasks[0].slot_release, g.tasks[0].slot_deadline, g.tasks[0].slot_budget), (0, 1, 2));
        assert_eq!(g.tasks.iter().map(|t| t.vector()).collect::<Vec<_>>(), vec![36, 37, 38]);
        assert_eq!(g.plan.partitions[0].code.base, 0x2_0000);
        assert_eq!(g.plan.partitions[1].ram.base, 0x2001_0000);
        let out = outputs(&demo(), "rsk-system").unwrap();
        assert!(out.system_source.contains("pub const NS: usize = 11;"));
        assert!(out.link_x.contains("*libdemo_part0-*:*(.text .text.* .rodata .rodata.*)"));
        assert!(out.task_model_json.contains("\"schema\": \"rsk-task-model/1\""));
    }

    #[test]
    fn outputs_are_deterministic_and_ceilings_order_independent() {
        let a = outputs(&demo(), "rsk-system").unwrap();
        let b = outputs(&demo(), "rsk-system").unwrap();
        assert_eq!(a, b);
        // Reverse the declaration order of the Tasks: same Ceiling, same
        // capacities, same layout (Property 13).
        let mut d = demo();
        d.partitions[0].tasks.reverse();
        let g = generate(&d).unwrap();
        let g0 = generate(&demo()).unwrap();
        assert_eq!(g.resources[0].ceiling, g0.resources[0].ceiling);
        assert_eq!((g.slot_count, g.lock_capacity), (g0.slot_count, g0.lock_capacity));
        assert_eq!(g.plan, g0.plan);
    }

    #[test]
    fn violations_name_the_rule_and_the_item() {
        let mut d = demo();
        d.partitions[0].tasks[0].priority = 9;
        d.partitions[0].tasks[1].deadline = ms(30);
        d.partitions[1].peripherals.push("CMSDK_TIMER1".into());
        d.partitions[1].resources.push(resource("unused"));
        let errs = generate(&d).unwrap_err();
        let rules: Vec<(String, String)> = errs.iter().map(|e| (e.rule.clone(), e.item.to_string())).collect();
        assert!(rules.contains(&("PR-33".into(), "task part0.control.priority".into())), "{rules:?}");
        assert!(rules.contains(&("R22.1".into(), "task part0.sensor.deadline".into())), "{rules:?}");
        assert!(rules.contains(&("PR-29".into(), "peripheral part1.CMSDK_TIMER1".into())), "{rules:?}");
        assert!(rules.contains(&("R22.5".into(), "resource part1.unused".into())), "{rules:?}");
    }

    #[test]
    fn cross_partition_resource_access_is_pr30() {
        let mut d = demo();
        d.partitions[1].tasks[0].resources.push("state".into());
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "PR-30"), "{errs:?}");
    }

    #[test]
    fn multi_core_is_rejected_citing_ng01() {
        let mut d = demo();
        d.partitions[0].tasks[0].core = 1;
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "NG-01"), "{errs:?}");
    }

    #[test]
    fn layouts_that_do_not_fit_are_rejected_naming_the_partition() {
        let mut d = demo();
        d.partitions[1].ram = 8 * 1024 * 1024;
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R20.4" && e.item.to_string() == "partition part1.ram"), "{errs:?}");
    }
}
