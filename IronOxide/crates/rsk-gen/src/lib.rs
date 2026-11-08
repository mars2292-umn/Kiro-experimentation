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
pub mod roles;
pub mod profile_version;
#[cfg(feature = "syntax")]
pub mod syntax;
pub mod target;
pub mod validate;
#[cfg(feature = "arbitrary")]
pub mod arbitrary;

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
            state: vec![],
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
            invariant: None,
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
            fp_flush_to_zero: false,
            endpoints: vec![],
            protocols: vec![],
            chains: vec![],
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

    /// R21.2, R23.1, R23.4, R24.5: the demo's declared times become ticks, the
    /// Resource Ceiling is the maximum accessing Priority, the timer-slot and
    /// lock capacities follow from the declarations, and the MPU layout is placed.
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

    /// R22.2 (PR-33), R22.1, R15.6 (PR-29), R22.5: a Priority beyond the
    /// hardware levels, a deadline over the period, a Kernel-owned peripheral and
    /// an unaccessed Resource are each rejected naming the rule and the item.
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

    /// R27.2: a Task on a core other than 0 is rejected citing NG-01.
    #[test]
    /// Task 21.1 (R22.7, R38.5, R41.1, R21.3, R22.5): a Protocol-governed
    /// Endpoint is accepted with the checker's bound, and the rejections name
    /// their rule.
    #[test]
    fn protocols_are_checked_and_their_bound_recorded() {
        use crate::decl::{EndpointDecl, ProtocolDecl, RoleDecl};
        let roles = || vec![RoleDecl { role: "ctl".into(), partition: "part0".into(), task: "control".into(), validate: vec![(Some("Rep".into()), "crate::ok".into())], ops: 1, after: vec![] }, RoleDecl { role: "sink".into(), partition: "part1".into(), task: "log".into(), validate: vec![], ops: 1, after: vec![] }];
        let loop_text = "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }";
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: loop_text.into() });
        d.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(), capacity: 1, payload: 4 });
        let g = generate(&d).expect("accepted");
        let e = g.endpoints.iter().find(|e| e.name == "ping").unwrap();
        assert_eq!(e.k, 1);
        assert_eq!(e.channels.len(), 2);
        assert_eq!(e.projections[0].1, "rec L.sink!{Req(u32).sink?{Rep(u32).L}, Stop().end}");
        // R41.1: two requests before a reply need two slots.
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "Two".into(), text: "rec L { A(u8): ctl -> sink; B(u8): ctl -> sink; C(u8): sink -> ctl; continue L; }".into() });
        d.endpoints.push(EndpointDecl { name: "two".into(), ty: String::new(), protocol: Some("Two".into()), roles: roles(), capacity: 1, payload: 4 });
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.1" && e.item.to_string() == "endpoint two"), "{errs:?}");
        // R22.7: an unprojectable Protocol is rejected naming the interaction.
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "Bad".into(), text: "rec X { continue X; }".into() });
        d.endpoints.push(EndpointDecl { name: "bad".into(), ty: String::new(), protocol: Some("Bad".into()), roles: roles(), capacity: 2, payload: 4 });
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R22.7" && e.message.contains("unguarded")), "{errs:?}");
        // R21.3 and R22.5: an unknown Protocol, and one governing no Endpoint.
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "Idle".into(), text: loop_text.into() });
        d.endpoints.push(EndpointDecl { name: "x".into(), ty: String::new(), protocol: Some("Nope".into()), roles: roles(), capacity: 1, payload: 4 });
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R21.3" && e.message.contains("Nope")), "{errs:?}");
        assert!(errs.iter().any(|e| e.rule == "R22.5" && e.item.to_string() == "protocol Idle"), "{errs:?}");
    }

    /// Task 21.6 (R41.3; Verification_Plan 5.15, A-P8): a Task may play roles
    /// of several Protocol-governed Endpoints; the declared gating (`after`)
    /// must name a Protocol Endpoint of the same Task and be acyclic across
    /// the Application; the ranks rise along the edges. A rejected message
    /// re-initializes its Endpoint in the Kernel (A-P7), so the peer's fault
    /// response is unconstrained.
    #[test]
    fn r41_3_gating_is_declared_acyclic_and_ranked() {
        use crate::decl::{EndpointDecl, ProtocolDecl, Response, RoleDecl};
        let roles = |after_ctl: &[&str], after_sink: &[&str]| vec![
            RoleDecl { role: "ctl".into(), partition: "part0".into(), task: "control".into(), validate: vec![(Some("Rep".into()), "crate::ok".into())], ops: 1, after: after_ctl.iter().map(|s| s.to_string()).collect() },
            RoleDecl { role: "sink".into(), partition: "part1".into(), task: "log".into(), validate: vec![], ops: 1, after: after_sink.iter().map(|s| s.to_string()).collect() },
        ];
        let loop_text = "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }";
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: loop_text.into() });
        d.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&[], &[]), capacity: 1, payload: 4 });
        // A-P7 is a Kernel rule: the validated role's peer may declare any response.
        d.partitions[1].fault = Response::EndJob;
        generate(&d).expect("accepted with END_JOB");
        // Two independent Protocol Endpoints on the same Tasks: accepted, rank 0 each.
        let mut d2 = d.clone();
        d2.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&[], &[]), capacity: 1, payload: 4 });
        let g = generate(&d2).expect("independent Endpoints are accepted");
        assert!(g.endpoints.iter().filter(|e| e.protocol.is_some()).all(|e| e.gating_rank == 0));
        // Acyclic gating: `pong`'s ctl waits for `ping`; `pong` ranks above `ping`.
        let mut d3 = d.clone();
        d3.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&["ping"], &[]), capacity: 1, payload: 4 });
        let g = generate(&d3).expect("acyclic gating is accepted");
        let rank = |g: &Generated, n: &str| g.endpoints.iter().find(|e| e.name == n).unwrap().gating_rank;
        assert_eq!((rank(&g, "ping"), rank(&g, "pong")), (0, 1));
        // A cycle through two Tasks: control waits for `ping` on `pong`, log waits for `pong` on `ping`.
        let mut d4 = d.clone();
        let last = d4.endpoints.len() - 1;
        d4.endpoints[last].roles[1].after = vec!["pong".into()];
        d4.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&["ping"], &[]), capacity: 1, payload: 4 });
        let errs = generate(&d4).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("cyclic") && e.message.contains("`ping`") && e.message.contains("`pong`")), "{errs:?}");
        // `after` must name a Protocol Endpoint of the same Task, not itself, not a plain one.
        let mut d5 = d.clone();
        d5.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&["pong"], &["nope"]), capacity: 1, payload: 4 });
        let errs = generate(&d5).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("cannot gate itself")), "{errs:?}");
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("after = nope")), "{errs:?}");
        let mut d6 = d.clone();
        d6.endpoints.push(EndpointDecl { name: "plain".into(), ty: "u32".into(), protocol: None, roles: roles(&[], &[]).into_iter().map(|mut r| { r.validate = if r.role == "ctl" { vec![(None, "crate::ok".into())] } else { vec![] }; r }).collect(), capacity: 1, payload: 4 });
        d6.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(&["plain"], &[]), capacity: 1, payload: 4 });
        let errs = generate(&d6).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("plain Endpoint")), "{errs:?}");
    }

    /// Task 21.7 (R41.3; the independent review's finding 3): a Task released
    /// by a message arrival awaits it before every operation. Its role on the
    /// releasing Endpoint must begin with a receive, and the releasing
    /// Endpoint gates the Task's other Protocol Endpoints (an implicit `after`
    /// that counts in the ranks and in the acyclicity check).
    #[test]
    fn r41_3_a_message_release_gates_the_tasks_other_endpoints() {
        use crate::decl::{EndpointDecl, MitPolicy, ProtocolDecl, Release, RoleDecl, Source};
        let loop_text = "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }";
        // Both roles validate what they receive, so the R42.4 rule is quiet whichever Partition plays which.
        let roles = |ctl: (&str, &str), sink: (&str, &str)| vec![
            RoleDecl { role: "ctl".into(), partition: ctl.0.into(), task: ctl.1.into(), validate: vec![(Some("Rep".into()), "crate::ok".into())], ops: 1, after: vec![] },
            RoleDecl { role: "sink".into(), partition: sink.0.into(), task: sink.1.into(), validate: vec![(Some("Req".into()), "crate::ok".into())], ops: 1, after: vec![] },
        ];
        // `log` becomes sporadic, released by `ping` (it plays `sink`, which receives first: accepted).
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: loop_text.into() });
        d.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(("part0", "control"), ("part1", "log")), capacity: 1, payload: 4 });
        let log = d.partitions.iter_mut().find(|p| p.name == "part1").unwrap().tasks.iter_mut().find(|t| t.name == "log").unwrap();
        log.release = Release::Sporadic { mit: ms(40), policy: MitPolicy::Defer, source: Source::Endpoint("ping".into()) };
        generate(&d).expect("a receive-first role released by its Endpoint is accepted");
        // A second Protocol Endpoint `pong` on which `log` plays `ctl` (sends first): gated by `ping`
        // implicitly, so `pong` ranks above `ping`.
        let mut d2 = d.clone();
        d2.endpoints.push(EndpointDecl { name: "pong".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(("part1", "log"), ("part0", "control")), capacity: 1, payload: 4 });
        let g = generate(&d2).expect("accepted");
        let rank = |n: &str| g.endpoints.iter().find(|e| e.name == n).unwrap().gating_rank;
        assert_eq!((rank("ping"), rank("pong")), (0, 1));
        assert!(g.endpoints.iter().find(|e| e.name == "pong").unwrap().after[0].contains(&"ping".to_string()), "the implicit gate is recorded");
        // The implicit edge closes a cycle with an explicit one: refused.
        let mut d3 = d2.clone();
        let ping_ix = d3.endpoints.len() - 2;
        d3.endpoints[ping_ix].roles[1].after = vec!["pong".into()];
        let errs = generate(&d3).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("cyclic")), "{errs:?}");
        // A released role that sends first would never be released: refused.
        let mut d4 = demo();
        d4.protocols.push(ProtocolDecl { name: "PingPong".into(), text: loop_text.into() });
        d4.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: roles(("part1", "log"), ("part0", "control")), capacity: 1, payload: 4 });
        let log = d4.partitions.iter_mut().find(|p| p.name == "part1").unwrap().tasks.iter_mut().find(|t| t.name == "log").unwrap();
        log.release = Release::Sporadic { mit: ms(40), policy: MitPolicy::Defer, source: Source::Endpoint("ping".into()) };
        let errs = generate(&d4).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R41.3" && e.message.contains("begins with a send")), "{errs:?}");
    }

    #[test]
    fn multi_core_is_rejected_citing_ng01() {
        let mut d = demo();
        d.partitions[0].tasks[0].core = 1;
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "NG-01"), "{errs:?}");
    }

    /// R20.4, R24.5: a layout that does not fit the Target is rejected naming
    /// the Partition whose region fails.
    #[test]
    fn layouts_that_do_not_fit_are_rejected_naming_the_partition() {
        let mut d = demo();
        d.partitions[1].ram = 8 * 1024 * 1024;
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "R20.4" && e.item.to_string() == "partition part1.ram"), "{errs:?}");
    }
}
