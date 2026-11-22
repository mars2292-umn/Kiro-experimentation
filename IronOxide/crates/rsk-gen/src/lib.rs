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
pub mod derive_v;
pub mod emit;
pub mod json;
pub mod layout;
pub mod model;
pub mod roles;
pub mod profile_version;
pub mod readback;
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
    /// A starter Task body per role of every Protocol-governed Endpoint
    /// (`skeletons/<endpoint>_<role>.rs`): for the user to copy, read by no build.
    pub skeletons: Vec<roles::Skeleton>,
}

/// The directory of the skeletons under an output directory. The Generator
/// owns it whole: every file in it is one of its outputs or is stale.
pub const SKELETON_DIR: &str = "skeletons";

/// The files under `out_dir/skeletons` that are not among `files` (the
/// names of the Generator's outputs, relative to `out_dir`): the skeleton
/// of a role or an Endpoint the declaration no longer has. Sorted; empty
/// when the directory does not exist.
pub fn stale_skeletons(out_dir: &std::path::Path, files: &[String]) -> Vec<std::path::PathBuf> {
    let Ok(entries) = std::fs::read_dir(out_dir.join(SKELETON_DIR)) else { return Vec::new() };
    let mut stale: Vec<std::path::PathBuf> = entries
        .filter_map(|e| e.ok())
        .filter(|e| !files.iter().any(|f| *f == format!("{SKELETON_DIR}/{}", e.file_name().to_string_lossy())))
        .map(|e| e.path())
        .collect();
    stale.sort();
    stale
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
    let system_source = emit::system_source(&generated);
    // R24.1 (2026-11-16): the table as printed is read back and compared
    // with the derived values before anything is handed out.
    if let Err(difference) = readback::check(&system_source, &generated) {
        return Err(vec![Diagnostic::new("R24.1", decl::ItemRef::Declaration, format!("the emitted configuration does not match the derived values: {difference}"))]);
    }
    // The same for the memory layout (2026-11-17): the build-time
    // validation accepts any well-formed layout, this the derived one only.
    if let Err(difference) = readback::check_layout(&system_source, &generated) {
        return Err(vec![Diagnostic::new("R24.1", decl::ItemRef::Declaration, format!("the emitted layout does not match the derived memory plan: {difference}"))]);
    }
    // And for the tables the read-back did not reach until 2026-11-21: the
    // EasyDMA channels, the Endpoints, and each Partition's memory map.
    if let Err(difference) = readback::check_dma(&system_source, &generated).and_then(|()| readback::check_endpoints(&system_source, &generated)) {
        return Err(vec![Diagnostic::new("R24.1", decl::ItemRef::Declaration, format!("the emitted tables do not match the derived values: {difference}"))]);
    }
    let partition_sources: Vec<(String, String)> = partition_sources;
    for (index, (_, source)) in partition_sources.iter().enumerate() {
        if let Err(difference) = readback::check_partition_map(source, &generated, generated.partitions[index].index) {
            return Err(vec![Diagnostic::new("R24.1", decl::ItemRef::Declaration, format!("a Partition's emitted memory map does not match the derived memory plan: {difference}"))]);
        }
    }
    Ok(Outputs {
        system_source,
        partition_sources,
        memory_x: emit::memory_x(&generated),
        link_x: emit::link_x(&generated, system_crate),
        summary: emit::summary(&generated),
        task_model_json: model::task_model(&generated).render(),
        skeletons: roles::skeletons(&generated),
        generated,
    })
}

#[cfg(test)]
pub(crate) mod tests {
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

    fn rules_of(d: &Declaration) -> Vec<(String, String)> {
        generate(d).unwrap_err().iter().map(|e| (e.rule.clone(), e.item.to_string())).collect()
    }

    /// R18.5: a Partition below the highest Criticality_Level of the
    /// Application that lets an overrunning Job continue is rejected, naming
    /// the Partition's Overrun_Response.
    #[test]
    fn r18_5_a_lower_level_partition_may_not_let_an_overrunning_job_continue() {
        let mut d = demo();
        d.partitions[1].overrun = Response::RecordAndContinue;
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.item.to_string() == "partition part1.overrun" && e.message.contains("R18.5")), "{errs:?}");
        // With two Partitions of the highest level, both may.
        let mut d = demo();
        d.partitions[1].level = Level::A;
        d.partitions[0].overrun = Response::RecordAndContinue;
        d.partitions[1].overrun = Response::RecordAndContinue;
        generate(&d).expect("accepted at the highest level");
    }

    /// R18.6: the Partition of the highest Criticality_Level may record an
    /// Overrun and let the Job complete, and the generated table carries the
    /// response.
    #[test]
    fn r18_6_the_highest_level_partition_may_record_an_overrun_and_continue() {
        let mut d = demo();
        d.partitions[0].overrun = Response::RecordAndContinue;
        let g = generate(&d).expect("accepted");
        let out = outputs(&d, "rsk-system").unwrap();
        assert_eq!(g.plan.partitions.len(), 2);
        assert!(out.system_source.contains("RESP_RECORD_AND_CONTINUE"), "the response is not in the generated table");
    }

    /// R21.5: an owned peripheral, and the interrupt a Task binds, must exist
    /// in the Target's device description; each is rejected naming the item.
    #[test]
    fn r21_5_a_peripheral_or_interrupt_the_target_lacks_is_rejected() {
        let mut d = demo();
        d.partitions[1].peripherals.push("NO_SUCH_PERIPHERAL".into());
        assert!(rules_of(&d).contains(&("R21.5".into(), "peripheral part1.NO_SUCH_PERIPHERAL".into())), "{:?}", rules_of(&d));
        let mut d = demo();
        d.partitions[1].tasks[0].release =
            Release::Sporadic { mit: ms(40), policy: MitPolicy::Defer, source: Source::Interrupt("NO_SUCH_PERIPHERAL".into()) };
        assert!(rules_of(&d).contains(&("R21.5".into(), "task part1.log.binds".into())), "{:?}", rules_of(&d));
    }

    /// R22.6: an operating duration the time base cannot represent, a period
    /// longer than the operating duration and an offset beyond it are each
    /// rejected naming the field.
    #[test]
    fn r22_6_times_the_kernel_time_base_cannot_hold_are_rejected() {
        let mut d = demo();
        d.operating_duration = ms(0);
        assert!(rules_of(&d).contains(&("R22.6".into(), "app.operating_duration".into())), "{:?}", rules_of(&d));
        let mut d = demo();
        d.operating_duration = Time { value: u64::MAX, unit: TimeUnit::Ms };
        assert!(rules_of(&d).contains(&("R22.6".into(), "app.operating_duration".into())), "{:?}", rules_of(&d));
        let mut d = demo();
        d.partitions[1].tasks[0].release = Release::Periodic { period: ms(400), offset: None };
        d.partitions[1].tasks[0].deadline = ms(400);
        assert!(rules_of(&d).contains(&("R22.6".into(), "task part1.log.period".into())), "{:?}", rules_of(&d));
        let mut d = demo();
        d.partitions[1].tasks[0].release = Release::Periodic { period: ms(40), offset: Some(ms(300)) };
        assert!(rules_of(&d).contains(&("R22.6".into(), "task part1.log.offset".into())), "{:?}", rules_of(&d));
    }

    /// R15.7: a Partition that omits a region size, or declares a response
    /// outside the closed set of its kind (R14.5), is rejected naming the
    /// Partition and the field, one diagnostic per field.
    #[test]
    fn r15_7_each_missing_field_and_each_response_outside_its_set_is_named() {
        let cases: [(&str, fn(&mut PartitionDecl)); 7] = [
            ("code", |p| p.code = 0),
            ("ram", |p| p.ram = 0),
            ("stack", |p| p.stack = 0),
            ("stack", |p| p.stack = 4092),
            ("fault", |p| p.fault = Response::RecordOnly),
            ("overrun", |p| p.overrun = Response::RecordOnly),
            ("deadline_miss", |p| p.deadline_miss = Response::EndJob),
        ];
        for (field, change) in cases {
            let mut d = demo();
            change(&mut d.partitions[1]);
            let rules = rules_of(&d);
            assert!(rules.contains(&("R15.7".into(), format!("partition part1.{field}"))), "{field}: {rules:?}");
            assert!(!rules.iter().any(|(_, item)| item.starts_with("partition part0")), "{field}: the other Partition is named: {rules:?}");
        }
    }

    /// R15.5: an Application of a single Partition generates, with that
    /// Partition's regions placed apart from the Kernel's.
    #[test]
    fn r15_5_an_application_of_one_partition_generates_with_its_own_regions() {
        let mut d = demo();
        d.partitions.truncate(1);
        let g = generate(&d).expect("valid");
        assert_eq!(g.plan.partitions.len(), 1);
        let p = &g.plan.partitions[0];
        assert!(p.code.size() > 0 && p.ram.size() > 0);
        let out = outputs(&d, "rsk-system").unwrap();
        assert!(out.system_source.contains("pub const NP: usize = 1;"), "the generated tables are not for one Partition");
    }

    /// R21.6 (PR-29): a Task bound to the interrupt of a peripheral that its
    /// Partition does not own is rejected naming the binding, while the
    /// owning Partition's Task may bind it.
    #[test]
    fn r21_6_a_task_may_bind_only_an_interrupt_of_a_peripheral_its_partition_owns() {
        let bound = |d: &mut Declaration, p: usize| {
            d.partitions[p].tasks.last_mut().unwrap().release =
                Release::Sporadic { mit: ms(40), policy: MitPolicy::Defer, source: Source::Interrupt("CMSDK_DUALTIMER".into()) };
        };
        let mut d = demo();
        d.partitions[0].peripherals.push("CMSDK_DUALTIMER".into());
        bound(&mut d, 1);
        let errs = generate(&d).unwrap_err();
        assert!(errs.iter().any(|e| e.rule == "PR-29" && e.item.to_string() == "task part1.log.binds" && e.message.contains("R21.6")), "{errs:?}");
        let mut d = demo();
        d.partitions[1].peripherals.push("CMSDK_DUALTIMER".into());
        bound(&mut d, 1);
        generate(&d).expect("the owning Partition's Task binds it");
    }

    /// R19.4: an interrupt-bound Task must accept the behaviour the design
    /// provides for interrupt sources (events of an MIT window coalesced and
    /// deferred, DD-06): `discard` is rejected naming the Task's policy,
    /// `defer` is accepted, and a Task released by a Release_Signal may
    /// declare either.
    #[test]
    fn r19_4_an_interrupt_bound_task_may_not_declare_the_discard_policy() {
        let with = |policy: MitPolicy, source: Source| {
            let mut d = demo();
            if matches!(source, Source::Interrupt(_)) {
                d.partitions[1].peripherals.push("CMSDK_DUALTIMER".into());
            } else {
                d.partitions[1].signals.push(SignalDecl { name: "tick".into() });
                d.partitions[0].tasks[0].raises.push("tick".into());
            }
            d.partitions[1].tasks[0].release = Release::Sporadic { mit: ms(40), policy, source };
            d
        };
        let irq = || Source::Interrupt("CMSDK_DUALTIMER".into());
        let errs = generate(&with(MitPolicy::Discard, irq())).unwrap_err();
        assert_eq!(errs.len(), 1, "{errs:?}");
        assert!(errs[0].rule == "R19.4" && errs[0].item.to_string() == "task part1.log.mit_policy" && errs[0].message.contains("DD-06"), "{errs:?}");
        generate(&with(MitPolicy::Defer, irq())).expect("an interrupt source defers");
        for policy in [MitPolicy::Defer, MitPolicy::Discard] {
            if let Err(errs) = generate(&with(policy, Source::Signal("tick".into()))) {
                panic!("a Release_Signal offers both policies: {errs:?}");
            }
        }
    }

    /// The skeletons: one per role of every Protocol-governed Endpoint,
    /// with an arm for every state of the role, a variant arm for every
    /// message a receive may deliver, and an arm handing the state back for
    /// every way a send or a receive fails.
    #[test]
    fn a_skeleton_writes_out_every_state_and_outcome() {
        use crate::decl::{EndpointDecl, ProtocolDecl, RoleDecl};
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }".into() });
        let role = |role: &str, partition: &str, task: &str| RoleDecl { role: role.into(), partition: partition.into(), task: task.into(), validate: vec![], ops: 1, after: vec![] };
        d.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: vec![RoleDecl { validate: vec![(Some("Rep".into()), "crate::ok".into())], ..role("ctl", "part0", "control") }, role("sink", "part1", "log")], capacity: 1, payload: 8 });
        let g = generate(&d).expect("accepted");
        let skeletons = roles::skeletons(&g);
        assert_eq!(skeletons.iter().map(|s| (s.file_name(), s.task.clone(), s.partition.clone())).collect::<Vec<_>>(), vec![("skeletons/ping_ctl.rs".to_string(), "control".to_string(), "part0".to_string()), ("skeletons/ping_sink.rs".to_string(), "log".to_string(), "part1".to_string())]);
        for sk in &skeletons {
            let e = g.endpoints.iter().find(|e| e.name == sk.endpoint).expect("the Endpoint");
            let states = &e.verified_states.iter().find(|(r, _)| *r == sk.role).expect("the role").1;
            let s = &sk.source;
            assert!(s.contains(&format!("cx.{}.take(State::Empty)", sk.endpoint)) && s.contains(&format!("cx.{}.store(next);", sk.endpoint)), "{s}");
            assert!(s.contains("State::Empty => State::Empty,"), "{s}");
            for (i, st) in states.iter().enumerate() {
                assert!(s.contains(&format!("State::S{i}(role) =>")), "no arm for S{i}:\n{s}");
                match &st.kind {
                    roles::Kind::End => assert!(s.contains("role.finish()"), "{s}"),
                    roles::Kind::Send { branches, .. } => {
                        for (label, payload, _) in branches {
                            assert!(s.contains(&format!("role.send_{}(", label.to_lowercase())), "no send of {label}:\n{s}");
                            assert_eq!(payload.is_some(), s.contains(&format!("todo!(\"the {label} payload")), "{s}");
                        }
                        for reason in ["Full", "Refused"] {
                            assert!(s.contains(&format!("Err(NotSent::{reason}(s{i})) => State::S{i}(s{i}),")), "{s}");
                        }
                        assert!(s.contains("Err(NotSent::PeerChanged(s0)) => State::S0(s0),"), "{s}");
                    }
                    roles::Kind::Recv { branches, .. } => {
                        for (label, _, next) in branches {
                            assert!(s.contains(&format!("Ok(RecvS{i}::{label}(")) && s.contains(&format!("State::S{next}(s{next})")), "no arm for {label}:\n{s}");
                        }
                        // No role is declared `after` this Endpoint: no gate to open.
                        assert!(!s.contains("_gate"), "{s}");
                        for reason in ["Empty", "Unexpected", "Rejected"] {
                            assert!(s.contains(&format!("Err(NotReceived::{reason}(s{i})) => State::S{i}(s{i}),")), "{s}");
                        }
                        assert!(s.contains("Err(NotReceived::PeerChanged(s0)) => State::S0(s0),"), "{s}");
                    }
                }
            }
        }
    }

    /// A file under `skeletons/` that the Generator did not write is
    /// stale, whatever its name; the outputs themselves and the files
    /// beside the directory are not.
    #[test]
    fn a_skeleton_the_declaration_no_longer_has_is_stale() {
        let out = std::env::temp_dir().join(format!("rsk-gen-stale-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&out);
        let dir = out.join(SKELETON_DIR);
        assert_eq!(stale_skeletons(&out, &[]), Vec::<std::path::PathBuf>::new(), "no directory, nothing stale");
        std::fs::create_dir_all(&dir).expect("the directory");
        for name in ["ping_ctl.rs", "ping_sink.rs", "old_ctl.rs", "notes.txt"] {
            std::fs::write(dir.join(name), "").expect("a file");
        }
        std::fs::write(out.join("summary.txt"), "").expect("a file beside the directory");
        let files = vec!["summary.txt".to_string(), "skeletons/ping_ctl.rs".to_string(), "skeletons/ping_sink.rs".to_string()];
        assert_eq!(stale_skeletons(&out, &files), vec![dir.join("notes.txt"), dir.join("old_ctl.rs")]);
        // A name that is an output elsewhere is still stale here.
        assert_eq!(stale_skeletons(&out, &["ping_ctl.rs".to_string()]).len(), 4);
        let _ = std::fs::remove_dir_all(&out);
    }

    /// A role declared `after` an Endpoint (R41.3, A-P8): its skeleton
    /// returns before taking the state while the gate is closed and hands
    /// the gate to every operation, and the skeleton of the Task's role on
    /// the gating Endpoint opens the gate on every completed receive.
    #[test]
    fn a_gated_skeleton_tests_its_gate_and_the_gating_receive_opens_it() {
        use crate::decl::{EndpointDecl, ProtocolDecl, RoleDecl};
        let mut d = demo();
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }".into() });
        let role = |role: &str, partition: &str, task: &str, after: &[&str]| RoleDecl { role: role.into(), partition: partition.into(), task: task.into(), validate: vec![], ops: 2, after: after.iter().map(|x| x.to_string()).collect() };
        d.endpoints.push(EndpointDecl { name: "ping".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: vec![RoleDecl { validate: vec![(Some("Rep".into()), "crate::ok".into())], ..role("ctl", "part0", "control", &[]) }, role("sink", "part1", "log", &[])], capacity: 1, payload: 8 });
        d.endpoints.push(EndpointDecl { name: "relay".into(), ty: String::new(), protocol: Some("PingPong".into()), roles: vec![role("ctl", "part1", "log", &["ping"]), RoleDecl { validate: vec![(Some("Req".into()), "crate::ok".into())], ..role("sink", "part0", "control", &[]) }], capacity: 1, payload: 8 });
        let g = generate(&d).expect("accepted");
        let skeletons = roles::skeletons(&g);
        let of = |name: &str| skeletons.iter().find(|s| s.file_name() == format!("skeletons/{name}.rs")).map(|s| s.source.clone()).expect("the skeleton");

        // The gated role: the test of the gate precedes the take, and every operation takes the gate.
        let gated = of("relay_ctl");
        let (guard, take) = (gated.find("if !cx.ping_gate.is_open() {\n        return;\n    }").expect("the guard"), gated.find("cx.relay.take(").expect("the take"));
        assert!(guard < take, "{gated}");
        for op in ["role.send_req(todo!(\"the Req payload: u32\"), &cx.ping_gate)", "role.send_stop(&cx.ping_gate)", "role.try_recv(&cx.ping_gate)"] {
            assert!(gated.contains(op), "no `{op}`:\n{gated}");
        }
        assert!(!gated.contains(".open("), "{gated}");

        // The gating receive, by the same Task: opened on the outcome, before the outcome is taken apart.
        let gating = of("ping_sink");
        let (open, arms) = (gating.find("Ok(received) => {").expect("the outcome"), gating.find("match received {").expect("the arms"));
        let opened = gating.find("cx.ping_gate.open(&received);").expect("the gate opened");
        assert!(open < opened && opened < arms, "{gating}");
        assert!(gating.contains("RecvS0::Req(m, s1) => {") && gating.contains("RecvS0::Stop(s2) => {"), "{gating}");
        assert!(!gating.contains("is_open"), "{gating}");

        // The other Task's roles on the same Endpoints have no gate.
        for name in ["ping_ctl", "relay_sink"] {
            assert!(!of(name).contains("_gate"), "{}", of(name));
        }
    }

    /// R24.3: the configuration summary lists the values of R24.1 for
    /// review: the Target, the capacities, each Partition with its level and
    /// responses, each Task and each Resource with its Ceiling.
    #[test]
    fn r24_3_the_summary_lists_the_generated_values() {
        let out = outputs(&demo(), "rsk-system").unwrap();
        let s = &out.summary;
        assert!(s.starts_with("rsk Generated_Config summary (R24.3)"), "{s}");
        for needle in ["Target: qemu_mps2_an386", "Capacities: lock stack 2, timer slots 11, event log 32", "Operating duration: 5000000 ticks", "part0 (crate demo_part0, level A): fault RESTART_PARTITION, overrun END_JOB, deadline miss RECORD_ONLY, MIT threshold 10", "part1 (crate demo_part1, level C)"] {
            assert!(s.contains(needle), "the summary lacks `{needle}`:\n{s}");
        }
        for name in ["control", "sensor", "log", "state"] {
            assert!(s.contains(name), "the summary lacks `{name}`:\n{s}");
        }
    }

    /// R15.3 (PR-30): a Resource accessed by a Task of another Partition is
    /// rejected.
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
        // The role played in part0 (level A) validates what it receives from part1 (level C), so
        // the R42.4 rule is quiet whichever Partition plays which. Until 2026-11-13 both roles
        // validated; a validation function on the downward edge is rejected since (audit finding 2:
        // the Kernel accepts a rejection report only over an upward edge).
        let roles = |ctl: (&str, &str), sink: (&str, &str)| vec![
            RoleDecl { role: "ctl".into(), partition: ctl.0.into(), task: ctl.1.into(), validate: if ctl.0 == "part0" { vec![(Some("Rep".into()), "crate::ok".into())] } else { vec![] }, ops: 1, after: vec![] },
            RoleDecl { role: "sink".into(), partition: sink.0.into(), task: sink.1.into(), validate: if sink.0 == "part0" { vec![(Some("Req".into()), "crate::ok".into())] } else { vec![] }, ops: 1, after: vec![] },
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
