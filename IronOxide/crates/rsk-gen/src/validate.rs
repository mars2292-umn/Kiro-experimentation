//! Declaration validation (R15, R21, R22, R27.2): every rejection names
//! the item and the PR identifier or requirement it violates (R3.1,
//! R21.4). Validation runs on the declaration before any derivation, so
//! that derivation only sees declarations the Profile accepts.

use std::collections::{BTreeMap, BTreeSet};

use crate::convert::{to_cycles, to_ticks, Rounding};
use crate::decl::{Declaration, Diagnostic, ItemRef, Release, ResourcePlacement, Response, Source};
use crate::profile_version::PROFILE_VARIANTS;
use crate::target::Target;

/// Priorities a Task may declare (PR-33: 7 Task levels plus the Kernel level).
pub const MAX_PRIORITY: u8 = 7;
/// Kernel-reserved NVIC levels (DD-02).
pub const KERNEL_LEVELS: u8 = 1;
/// The largest operating duration in ticks (R9.5: below 2^62).
pub const MAX_DURATION_TICKS: u64 = 1 << 62;
/// Health_Monitor log capacity bound (R14.6; `Log<NE>` needs NE < 2^30).
pub const MAX_LOG_CAPACITY: u32 = 1 << 30;

/// The most Endpoints a configuration may declare (`rsk_kernel::arch::cm4::MAX_ENDPOINTS`).
pub const MAX_ENDPOINTS: usize = 8;
/// The most roles an Endpoint may bind (`rsk_kernel::arch::cm4::MAX_ROLES`; task 21.7).
pub const MAX_ROLES: usize = 4;
/// The Kernel storage every Endpoint channel may take together (PR-14): a
/// quarter of the smallest Kernel RAM region of the Target table.
pub const MAX_ENDPOINT_STORAGE: u64 = 8 * 1024;

pub fn validate(decl: &Declaration) -> Result<&'static Target, Vec<Diagnostic>> {
    let mut d = Vec::new();
    let Some(target) = Target::by_name(&decl.target) else {
        d.push(Diagnostic::new(
            "R21.1",
            ItemRef::Field("target".into()),
            format!(
                "unknown Target `{}`; known Targets: {}",
                decl.target,
                crate::target::TARGETS.iter().map(|t| t.name).collect::<Vec<_>>().join(", ")
            ),
        ));
        return Err(d);
    };
    if !PROFILE_VARIANTS.contains(&decl.variant.as_str()) {
        d.push(Diagnostic::new(
            "R3.5",
            ItemRef::Field("profile".into()),
            format!("unknown Profile_Variant `{}`; the Profile defines {}", decl.variant, PROFILE_VARIANTS.join(", ")),
        ));
    } else if decl.variant != "Ravenscar" {
        d.push(Diagnostic::new(
            "R5.1",
            ItemRef::Field("profile".into()),
            format!("the Profile_Variant `{}` is not implemented by this Generator (Phase P5)", decl.variant),
        ));
    }
    if decl.log_capacity == 0 || decl.log_capacity >= MAX_LOG_CAPACITY {
        d.push(Diagnostic::new(
            "R14.6",
            ItemRef::Field("log_capacity".into()),
            format!("the event log capacity must be between 1 and {}", MAX_LOG_CAPACITY - 1),
        ));
    }
    if decl.safe_state.is_empty() {
        d.push(Diagnostic::new("R21.3", ItemRef::Field("safe_state".into()), "the system safe state is required (R21.1)"));
    }
    let (tick_hz, cpu_hz) = (target.tick_hz, target.cpu_hz);
    let duration = match to_ticks(decl.operating_duration, tick_hz, cpu_hz, Rounding::Down) {
        Some((0, _)) | None => {
            d.push(Diagnostic::new(
                "R22.6",
                ItemRef::Field("operating_duration".into()),
                "the maximum continuous operating duration must be at least one tick",
            ));
            0
        }
        Some((ticks, _)) if ticks >= MAX_DURATION_TICKS => {
            d.push(Diagnostic::new(
                "R22.6",
                ItemRef::Field("operating_duration".into()),
                format!("the Kernel time base cannot operate for {ticks} ticks without wrap-around ambiguity (R9.5: below 2^62 ticks)"),
            ));
            0
        }
        Some((ticks, _)) => ticks,
    };

    if decl.partitions.is_empty() {
        d.push(Diagnostic::new("R21.3", ItemRef::Declaration, "at least one Partition is required (PR-28)"));
    }
    let highest_level = decl.partitions.iter().map(|p| p.level.code()).min().unwrap_or(0);
    let mut partition_names = BTreeSet::new();
    let mut crate_names = BTreeSet::new();
    let mut peripheral_owner: BTreeMap<&str, &str> = BTreeMap::new();
    let mut irq_users: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut signal_owner: BTreeMap<&str, &str> = BTreeMap::new();
    let mut signal_bound: BTreeMap<&str, Vec<String>> = BTreeMap::new();
    let mut priorities = BTreeSet::new();
    let mut task_count = 0usize;

    for p in &decl.partitions {
        let pref = || ItemRef::Partition(p.name.clone());
        if !partition_names.insert(p.name.as_str()) {
            d.push(Diagnostic::new("PR-28", pref(), format!("Partition `{}` is declared twice", p.name)));
        }
        if !crate_names.insert(p.krate.as_str()) {
            d.push(Diagnostic::new(
                "DD-11",
                ItemRef::PartitionField(p.name.clone(), "crate".into()),
                format!("crate `{}` is used by more than one Partition (one crate per Partition)", p.krate),
            ));
        }
        if p.code == 0 {
            d.push(Diagnostic::new("R15.7", ItemRef::PartitionField(p.name.clone(), "code".into()), "the code region size is required and must be positive"));
        }
        if p.ram == 0 {
            d.push(Diagnostic::new("R15.7", ItemRef::PartitionField(p.name.clone(), "ram".into()), "the RAM region size is required and must be positive"));
        }
        if p.stack == 0 || p.stack > p.ram {
            d.push(Diagnostic::new(
                "R15.7",
                ItemRef::PartitionField(p.name.clone(), "stack".into()),
                "the stack size must be positive and fit the RAM region (R16.5: the stack is the bottom of the RAM region)",
            ));
        }
        if p.stack % 8 != 0 {
            d.push(Diagnostic::new("R15.7", ItemRef::PartitionField(p.name.clone(), "stack".into()), "the stack size must be a multiple of 8 bytes (HW-06: 8-byte stack alignment)"));
        }
        if !p.fault.usable_as_fault() {
            d.push(Diagnostic::new(
                "R15.7",
                ItemRef::PartitionField(p.name.clone(), "fault".into()),
                format!("`{}` is not a fault response of R14.5 (END_JOB, RESTART_PARTITION, STOP_PARTITION, SAFE_STATE)", p.fault.name()),
            ));
        }
        if !p.overrun.usable_as_overrun() {
            d.push(Diagnostic::new(
                "R15.7",
                ItemRef::PartitionField(p.name.clone(), "overrun".into()),
                format!("`{}` is not an Overrun_Response of R14.5", p.overrun.name()),
            ));
        }
        if p.overrun == Response::RecordAndContinue && p.level.code() != highest_level {
            d.push(Diagnostic::new(
                "R18.6",
                ItemRef::PartitionField(p.name.clone(), "overrun".into()),
                "RECORD_AND_CONTINUE is permitted only for Partitions at the highest Criticality_Level of the Application (R18.5)",
            ));
        }
        if !p.deadline_miss.usable_as_deadline_miss() {
            d.push(Diagnostic::new(
                "R15.7",
                ItemRef::PartitionField(p.name.clone(), "deadline_miss".into()),
                format!("`{}` is not a deadline-miss response of R14.5 (RESTART_PARTITION, STOP_PARTITION, SAFE_STATE, RECORD_ONLY)", p.deadline_miss.name()),
            ));
        }
        let mut seen = BTreeSet::new();
        for per in &p.peripherals {
            let iref = ItemRef::Peripheral(p.name.clone(), per.clone());
            if !seen.insert(per.as_str()) {
                d.push(Diagnostic::new("PR-28", iref, format!("peripheral `{per}` is listed twice")));
                continue;
            }
            match target.peripheral(per) {
                None => d.push(Diagnostic::new("R21.5", iref, format!("the Target `{}` has no peripheral `{per}`", target.name))),
                Some(t) if t.kernel_owned => d.push(Diagnostic::new(
                    "PR-29",
                    iref,
                    format!("peripheral `{per}` is owned by the Kernel (R15.6: the time base and the core peripherals cannot be assigned to a Partition)"),
                )),
                Some(t) if t.easy_dma && t.dma.is_none() => d.push(Diagnostic::new(
                    "DD-08",
                    iref,
                    format!("peripheral `{per}` is EasyDMA-capable but its DMA registers are not described in the Target table, so the Kernel cannot mediate it"),
                )),
                Some(_) => match peripheral_owner.insert(per.as_str(), p.name.as_str()) {
                    Some(other) => d.push(Diagnostic::new(
                        "PR-28",
                        iref,
                        format!("peripheral `{per}` is assigned to Partitions `{other}` and `{}`; every peripheral belongs to exactly one Partition", p.name),
                    )),
                    None => {}
                },
            }
        }
        let mut resource_names = BTreeSet::new();
        for r in &p.resources {
            let iref = ItemRef::Resource(p.name.clone(), r.name.clone());
            if !resource_names.insert(r.name.as_str()) {
                d.push(Diagnostic::new("PR-28", iref.clone(), format!("Resource `{}` is declared twice", r.name)));
            }
            if r.ty.trim().is_empty() || r.init.trim().is_empty() {
                d.push(Diagnostic::new("R21.3", iref.clone(), "a Resource declares its type and initial value (R21.1)"));
            }
            match r.placement {
                ResourcePlacement::Core(0) => {}
                ResourcePlacement::Core(c) => d.push(Diagnostic::new(
                    "NG-01",
                    iref.clone(),
                    format!("Resource `{}` is assigned to core {c}; v1 supports core 0 only (R27.2)", r.name),
                )),
                ResourcePlacement::Global => d.push(Diagnostic::new(
                    "NG-01",
                    iref.clone(),
                    format!("Resource `{}` is marked global; v1 supports core-local Resources only (R27.2)", r.name),
                )),
            }
            if !p.tasks.iter().any(|t| t.resources.iter().any(|x| x == &r.name)) {
                d.push(Diagnostic::new("R22.5", iref, format!("Resource `{}` has no accessing Task", r.name)));
            }
        }
        let mut signal_names = BTreeSet::new();
        for s in &p.signals {
            let iref = ItemRef::Signal(p.name.clone(), s.name.clone());
            if !signal_names.insert(s.name.as_str()) {
                d.push(Diagnostic::new("PR-28", iref.clone(), format!("Release_Signal `{}` is declared twice", s.name)));
            }
            if signal_owner.insert(s.name.as_str(), p.name.as_str()).is_some() {
                d.push(Diagnostic::new("PR-28", iref, format!("Release_Signal `{}` is declared in more than one Partition", s.name)));
            }
        }
        let mut task_names = BTreeSet::new();
        for t in &p.tasks {
            task_count += 1;
            let tref = || ItemRef::Task(p.name.clone(), t.name.clone());
            let field = |f: &str| ItemRef::TaskField(p.name.clone(), t.name.clone(), f.to_string());
            if !task_names.insert(t.name.as_str()) {
                d.push(Diagnostic::new("PR-28", tref(), format!("Task `{}` is declared twice", t.name)));
            }
            if t.priority == 0 || t.priority > MAX_PRIORITY {
                d.push(Diagnostic::new(
                    "PR-33",
                    field("priority"),
                    format!("Priority {} is outside 1..={MAX_PRIORITY} (the Kernel reserves the most urgent level, DD-02)", t.priority),
                ));
            } else {
                priorities.insert(t.priority);
            }
            if t.core != 0 {
                d.push(Diagnostic::new("NG-01", field("core"), format!("Task `{}` is assigned to core {}; v1 supports core 0 only (R27.2)", t.name, t.core)));
            }
            let mut res_seen = BTreeSet::new();
            for r in &t.resources {
                if !res_seen.insert(r.as_str()) {
                    d.push(Diagnostic::new("R8.10", field("resources"), format!("Resource `{r}` is listed twice")));
                }
                if !p.resources.iter().any(|x| &x.name == r) {
                    let elsewhere = decl.partitions.iter().any(|q| q.name != p.name && q.resources.iter().any(|x| &x.name == r));
                    if elsewhere {
                        d.push(Diagnostic::new(
                            "PR-30",
                            field("resources"),
                            format!("Resource `{r}` belongs to another Partition; Tasks of more than one Partition cannot access a Resource (R15.3)"),
                        ));
                    } else {
                        d.push(Diagnostic::new("R21.3", field("resources"), format!("Partition `{}` declares no Resource `{r}`", p.name)));
                    }
                }
            }
            for s in &t.raises {
                if !decl.partitions.iter().any(|q| q.signals.iter().any(|x| &x.name == s)) {
                    d.push(Diagnostic::new("R21.3", field("raises"), format!("no Partition declares a Release_Signal `{s}`")));
                }
            }
            // Time values (R22.1).
            let budget = match to_cycles(t.budget, tick_hz, cpu_hz, Rounding::Up) {
                Some((0, _)) | None => {
                    d.push(Diagnostic::new("R22.1", field("budget"), "the Budget must be at least one CPU cycle"));
                    None
                }
                Some((c, _)) if c > u32::MAX as u64 => {
                    d.push(Diagnostic::new("R22.1", field("budget"), "the Budget exceeds 2^32 - 1 CPU cycles"));
                    None
                }
                Some((c, _)) => Some(c),
            };
            let deadline = match to_ticks(t.deadline, tick_hz, cpu_hz, Rounding::Down) {
                Some((0, _)) | None => {
                    d.push(Diagnostic::new("R22.1", field("deadline"), "the relative deadline must be at least one tick after conversion"));
                    None
                }
                Some((v, _)) => Some(v),
            };
            let window = match &t.release {
                Release::Periodic { period, offset } => {
                    let period = match to_ticks(*period, tick_hz, cpu_hz, Rounding::Down) {
                        Some((0, _)) | None => {
                            d.push(Diagnostic::new("R22.1", field("period"), "the period must be at least one tick after conversion"));
                            None
                        }
                        Some((v, _)) => Some(v),
                    };
                    if let Some(o) = offset {
                        match to_ticks(*o, tick_hz, cpu_hz, Rounding::Nearest) {
                            None => d.push(Diagnostic::new("R22.6", field("offset"), "the offset is not representable in the Kernel time base")),
                            Some((v, _)) if duration > 0 && v > duration => d.push(Diagnostic::new(
                                "R22.6",
                                field("offset"),
                                "the offset lies beyond the maximum continuous operating duration",
                            )),
                            Some(_) => {}
                        }
                    }
                    period
                }
                Release::Sporadic { mit, policy: _, source } => {
                    match source {
                        Source::Interrupt(name) => match target.peripheral(name) {
                            None => d.push(Diagnostic::new(
                                "R21.5",
                                field("binds"),
                                format!("the Target `{}` has no peripheral `{name}` (the interrupt must exist in the Target's device description)", target.name),
                            )),
                            Some(per) if per.irq.is_none() => d.push(Diagnostic::new("R21.5", field("binds"), format!("peripheral `{name}` has no interrupt line"))),
                            Some(per) if per.kernel_owned => d.push(Diagnostic::new("PR-29", field("binds"), format!("peripheral `{name}` is owned by the Kernel"))),
                            Some(_) => {
                                if !p.peripherals.iter().any(|x| x == name) {
                                    d.push(Diagnostic::new(
                                        "PR-29",
                                        field("binds"),
                                        format!("Task `{}` binds the interrupt of `{name}`, which Partition `{}` does not own (R21.6)", t.name, p.name),
                                    ));
                                }
                                irq_users.entry(name.as_str()).or_default().push(format!("{}.{}", p.name, t.name));
                            }
                        },
                        Source::Signal(name) => {
                            signal_bound.entry(name.as_str()).or_default().push(format!("{}.{}", p.name, t.name));
                        }
                        Source::Endpoint(name) => {
                            // Task 21.2 (R40.3): the Task is bound to a role of the named Endpoint.
                            match decl.endpoints.iter().find(|e| &e.name == name) {
                                None => d.push(Diagnostic::new("R21.3", field("endpoint"), format!("no Endpoint `{name}` is declared"))),
                                Some(e) => {
                                    match e.roles.iter().find(|r| r.partition == p.name && r.task == t.name) {
                                        None => d.push(Diagnostic::new("R22.5", field("endpoint"), format!("Task `{}` is bound to no role of Endpoint `{name}`, so no message arrives for it", t.name))),
                                        Some(role) => {
                                            // Task 21.7 (R41.3, the independent review's finding 3): a
                                            // Task released by an arrival on a Protocol Endpoint awaits
                                            // that arrival before every operation, so its role's first
                                            // action must be a receive (a first send would never be
                                            // released), and the Endpoint gates the Task's other
                                            // Protocol Endpoints (implicit `after`, derive.rs).
                                            if let Some(pname) = &e.protocol {
                                                let roles: Vec<&str> = e.roles.iter().map(|r| r.role.as_str()).collect();
                                                if let Some(checked) = decl.protocols.iter().find(|q| &q.name == pname).and_then(|q| rsk_protocol::check_protocol(&q.text, &roles, e.capacity).ok()) {
                                                    if let Some((_, local)) = checked.locals.iter().find(|(n, _)| n == &role.role) {
                                                        let sts = crate::roles::states(local);
                                                        if matches!(sts.first().map(|st| &st.kind), Some(crate::roles::Kind::Send { .. })) {
                                                            d.push(Diagnostic::new("R41.3", field("endpoint"), format!("Task `{}` is released by an arrival on Endpoint `{name}`, but its role `{}` begins with a send (`{}`): it would await its own first message and never be released (Verification_Plan 5.15)", t.name, role.role, local.render())));
                                                        }
                                                    }
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                    match to_ticks(*mit, tick_hz, cpu_hz, Rounding::Down) {
                        Some((0, _)) | None => {
                            d.push(Diagnostic::new("R22.1", field("mit"), "the MIT must be at least one tick after conversion"));
                            None
                        }
                        Some((v, _)) => Some(v),
                    }
                }
            };
            if let (Some(dl), Some(w)) = (deadline, window) {
                if dl > w {
                    d.push(Diagnostic::new(
                        "R22.1",
                        field("deadline"),
                        format!("the relative deadline ({dl} ticks) exceeds the period or MIT ({w} ticks)"),
                    ));
                }
                if duration > 0 && w > duration {
                    d.push(Diagnostic::new(
                        "R22.6",
                        field("period"),
                        format!("the period or MIT ({w} ticks) exceeds the maximum continuous operating duration ({duration} ticks)"),
                    ));
                }
            }
            if let (Some(b), Some(dl)) = (budget, deadline) {
                let dl_cycles = crate::convert::ticks_to_cycles(dl, tick_hz, cpu_hz, Rounding::Down).unwrap_or(u64::MAX);
                if b > dl_cycles {
                    d.push(Diagnostic::new(
                        "R22.1",
                        field("budget"),
                        format!("the Budget ({b} cycles) exceeds the relative deadline ({dl_cycles} cycles)"),
                    ));
                }
            }
        }
    }
    // PR-25, PR-28: per-Task state names are unique per Task and distinct
    // from the Task's Resource and Release_Signal fields of its Context.
    for p in &decl.partitions {
        for t in &p.tasks {
            let mut seen = BTreeSet::new();
            for s in &t.state {
                let field = || ItemRef::TaskField(p.name.clone(), t.name.clone(), s.name.clone());
                if !seen.insert(s.name.as_str()) {
                    d.push(Diagnostic::new("PR-28", field(), format!("state `{}` of Task `{}` is declared twice", s.name, t.name)));
                }
                if t.resources.iter().any(|r| r == &s.name) || t.raises.iter().any(|r| r == &s.name) {
                    d.push(Diagnostic::new("R21.3", field(), format!("state `{}` of Task `{}` has the name of one of its Resources or Release_Signals; the Context has one field per item", s.name, t.name)));
                }
            }
        }
    }
    for (irq, users) in &irq_users {
        if users.len() > 1 {
            d.push(Diagnostic::new(
                "PR-12",
                ItemRef::Declaration,
                format!("the interrupt of `{irq}` releases more than one Task ({}); an interrupt releases exactly one Task (R22.3)", users.join(", ")),
            ));
        }
    }
    for (signal, owner) in &signal_owner {
        match signal_bound.get(signal).map(Vec::len).unwrap_or(0) {
            0 => d.push(Diagnostic::new(
                "R22.5",
                ItemRef::Signal(owner.to_string(), signal.to_string()),
                format!("Release_Signal `{signal}` is bound to no Task"),
            )),
            1 => {}
            n => d.push(Diagnostic::new(
                "PR-12",
                ItemRef::Signal(owner.to_string(), signal.to_string()),
                format!("Release_Signal `{signal}` releases {n} Tasks; it releases exactly one Task in the Ravenscar Profile_Variant (R22.3)"),
            )),
        }
    }
    for (signal, tasks) in &signal_bound {
        if !signal_owner.contains_key(signal) {
            d.push(Diagnostic::new(
                "R21.3",
                ItemRef::Declaration,
                format!("Task(s) {} bind the undeclared Release_Signal `{signal}`", tasks.join(", ")),
            ));
        }
    }
    if priorities.len() as u8 + KERNEL_LEVELS > (1u8 << target.priority_bits.min(3)) {
        d.push(Diagnostic::new(
            "PR-33",
            ItemRef::Declaration,
            format!(
                "{} distinct Task Priorities plus {KERNEL_LEVELS} Kernel level exceed the {} NVIC levels of the Target (PAR-06, R22.2)",
                priorities.len(),
                1u8 << target.priority_bits.min(3)
            ),
        ));
    }
    // Endpoints (task 21.2; R38.1, R40.1, PR-14, PR-28, R42.1 by the type
    // bound the Partition compiles): two roles each bound to a declared
    // Task, distinct, with a capacity and a payload the Kernel can hold.
    let mut endpoint_names = BTreeSet::new();
    let mut storage_bytes: u64 = 0;
    for e in &decl.endpoints {
        let eref = || ItemRef::Endpoint(e.name.clone());
        if !endpoint_names.insert(e.name.as_str()) {
            d.push(Diagnostic::new("PR-28", eref(), format!("Endpoint `{}` is declared twice", e.name)));
        }
        // Task 21.7: a Protocol-governed Endpoint binds two to `MAX_ROLES`
        // roles (one channel per ordered pair); the plain form two.
        if e.protocol.is_none() && e.roles.len() != 2 {
            d.push(Diagnostic::new("R38.1", eref(), format!("plain Endpoint `{}` declares {} roles; the plain form binds exactly two (a Protocol names its roles for more)", e.name, e.roles.len())));
        }
        if e.roles.len() < 2 || e.roles.len() > MAX_ROLES {
            d.push(Diagnostic::new("R38.1", eref(), format!("Endpoint `{}` declares {} roles; two to {MAX_ROLES} roles, each bound to a Task (task 21.7)", e.name, e.roles.len())));
        }
        let mut role_names = BTreeSet::new();
        let mut bound_tasks = BTreeSet::new();
        for r in &e.roles {
            if !role_names.insert(r.role.as_str()) {
                d.push(Diagnostic::new("R38.1", eref(), format!("Endpoint `{}` declares role `{}` twice", e.name, r.role)));
            }
            match decl.partitions.iter().find(|p| p.name == r.partition).and_then(|p| p.tasks.iter().find(|t| t.name == r.task)) {
                None => d.push(Diagnostic::new("R22.5", eref(), format!("role `{}` of Endpoint `{}` is bound to `{}.{}`, which is no declared Task", r.role, e.name, r.partition, r.task))),
                Some(t) => {
                    if !bound_tasks.insert(format!("{}.{}", r.partition, r.task)) {
                        d.push(Diagnostic::new("R38.1", eref(), format!("two roles of Endpoint `{}` are bound to `{}.{}`", e.name, r.partition, r.task)));
                    }
                    if t.resources.iter().any(|x| x == &e.name) || t.raises.iter().any(|x| x == &e.name) || t.state.iter().any(|s| s.name == e.name) {
                        d.push(Diagnostic::new("R21.3", eref(), format!("Endpoint `{}` has the name of a Resource, Release_Signal, or state of Task `{}`; the Context has one field per item", e.name, t.name)));
                    }
                }
            }
        }
        if e.capacity == 0 || e.capacity > 255 {
            d.push(Diagnostic::new("PR-14", eref(), format!("Endpoint `{}` has capacity {}; 1 to 255 messages per directed channel", e.name, e.capacity)));
        }
        if e.payload < 4 || e.payload > 65532 || e.payload % 4 != 0 {
            d.push(Diagnostic::new("PR-14", eref(), format!("Endpoint `{}` has payload {} bytes; 4 to 65,532, a multiple of 4 (the Kernel copies words)", e.name, e.payload)));
        }
        match &e.protocol {
            None => {
                if e.ty.is_empty() {
                    d.push(Diagnostic::new("R38.1", eref(), format!("Endpoint `{}` names no payload type", e.name)));
                }
            }
            Some(pname) => {
                // Task 21.1 (R22.7, R38.5, R41.1): the Protocol exists, its
                // roles are the Endpoint's, and the Protocol_Checker accepts
                // it with a bound within the declared capacity.
                match decl.protocols.iter().find(|p| &p.name == pname) {
                    None => d.push(Diagnostic::new("R21.3", eref(), format!("Endpoint `{}` names no declared Protocol `{pname}`", e.name))),
                    Some(p) => {
                        let roles: Vec<&str> = e.roles.iter().map(|r| r.role.as_str()).collect();
                        match rsk_protocol::check_protocol(&p.text, &roles, e.capacity) {
                            Ok(_) => {}
                            Err(err) => d.push(Diagnostic::new(
                                if err.rule == "R41.1" { "R41.1" } else { "R22.7" },
                                eref(),
                                format!("Endpoint `{}`: the Protocol_Checker rejects Protocol `{pname}` {err}", e.name),
                            )),
                        }
                    }
                }
            }
        }
        // Task 21.4 (R42.4): a role that receives messages from a Partition
        // of lower Criticality_Level declares a validation function for
        // each message type it receives (the plain payload, or each Protocol
        // label it receives that carries a payload); a declared function
        // names a message the role receives. Task 21.5 (R40.2): `ops`.
        if e.roles.len() >= 2 && e.roles.len() <= MAX_ROLES {
            let level_of = |r: &crate::decl::RoleDecl| decl.partitions.iter().find(|p| p.name == r.partition).map(|p| p.level.code());
            // Per role, the labels it receives with a payload and the role
            // each comes from (task 21.7: a role may receive from several).
            let received: Vec<Vec<(Option<String>, String)>> = match &e.protocol {
                None => e.roles.iter().enumerate().map(|(ri, _)| vec![(None, e.roles[1 - ri.min(1)].role.clone())]).collect(),
                Some(pname) => {
                    let roles: Vec<&str> = e.roles.iter().map(|r| r.role.as_str()).collect();
                    match decl.protocols.iter().find(|p| &p.name == pname).and_then(|p| rsk_protocol::check_protocol(&p.text, &roles, e.capacity).ok()) {
                        Some(checked) => e
                            .roles
                            .iter()
                            .map(|r| {
                                let mut out = Vec::new();
                                if let Some((_, local)) = checked.locals.iter().find(|(n, _)| n == &r.role) {
                                    received_messages(local, &mut out);
                                }
                                out.sort();
                                out.dedup();
                                out.into_iter().map(|(l, f)| (Some(l), f)).collect()
                            })
                            .collect(),
                        None => e.roles.iter().map(|_| Vec::new()).collect(),
                    }
                }
            };
            let received_labels_of: Vec<Vec<Option<String>>> = received.iter().map(|v| v.iter().map(|(l, _)| l.clone()).collect()).collect();
            for (ri, r) in e.roles.iter().enumerate() {
                for (label, from_name) in &received[ri] {
                    let Some(other) = e.roles.iter().find(|q| &q.role == from_name) else { continue };
                    let upward = r.partition != other.partition && matches!((level_of(r), level_of(other)), (Some(mine), Some(theirs)) if mine < theirs);
                    if upward {
                        if !r.validate.iter().any(|(l, _)| l == label) {
                            let what = match label {
                                None => format!("the payload `{}`", e.ty),
                                Some(l) => format!("label `{l}`"),
                            };
                            d.push(Diagnostic::new("R42.4", eref(), format!("role `{}` of Endpoint `{}` (`{}.{}`, level {}) receives {what} from `{}.{}` of lower Criticality_Level {} and declares no validation function for it (`validate = ..`)", r.role, e.name, r.partition, r.task, decl.partitions.iter().find(|p| p.name == r.partition).map(|p| p.level.name()).unwrap_or("?"), other.partition, other.task, decl.partitions.iter().find(|p| p.name == other.partition).map(|p| p.level.name()).unwrap_or("?"))));
                        }
                    }
                }
                let mut seen = BTreeSet::new();
                for (label, path) in &r.validate {
                    if !seen.insert(label.clone()) {
                        d.push(Diagnostic::new("R42.4", eref(), format!("role `{}` of Endpoint `{}` declares two validation functions for {}", r.role, e.name, label.as_deref().unwrap_or("the payload"))));
                    }
                    match (label, &e.protocol) {
                        (Some(l), None) => d.push(Diagnostic::new("R42.4", eref(), format!("role `{}` of Endpoint `{}` validates label `{l}`, but the Endpoint has no Protocol: `validate = {path}` validates the payload", r.role, e.name))),
                        (None, Some(_)) => d.push(Diagnostic::new("R42.4", eref(), format!("role `{}` of Endpoint `{}` validates the payload, but the Endpoint has a Protocol: `validate = {{ LABEL = {path} }}` names the label", r.role, e.name))),
                        (Some(l), Some(_)) if !received_labels_of[ri].contains(&Some(l.clone())) => d.push(Diagnostic::new("R42.4", eref(), format!("role `{}` of Endpoint `{}` validates label `{l}`, which it does not receive with a payload", r.role, e.name))),
                        _ => {}
                    }
                }
                if r.ops == 0 || r.ops > 255 {
                    d.push(Diagnostic::new("PR-14", eref(), format!("role `{}` of Endpoint `{}` declares ops = {}; 1 to 255 operations per Job", r.role, e.name, r.ops)));
                }
                // Task 21.6 (R41.3, A-P8; 2026-11-08): `after = NAME` names a
                // Protocol-governed Endpoint, other than this one, of which
                // the same Task plays a role; the gating relation is checked
                // acyclic after the loop.
                for gate in &r.after {
                    match decl.endpoints.iter().find(|g| &g.name == gate) {
                        None => d.push(Diagnostic::new("R41.3", eref(), format!("role `{}` of Endpoint `{}` declares `after = {gate}`, which is not an Endpoint", r.role, e.name))),
                        Some(g) if g.name == e.name => d.push(Diagnostic::new("R41.3", eref(), format!("role `{}` of Endpoint `{}` declares `after = {gate}`: an Endpoint cannot gate itself", r.role, e.name))),
                        Some(g) if g.protocol.is_none() => d.push(Diagnostic::new("R41.3", eref(), format!("role `{}` of Endpoint `{}` declares `after = {gate}`, a plain Endpoint: gating is between Protocol-governed Endpoints (a plain Endpoint carries no Protocol state and no progress guarantee)", r.role, e.name))),
                        Some(g) if !g.roles.iter().any(|q| q.partition == r.partition && q.task == r.task) => d.push(Diagnostic::new("R41.3", eref(), format!("role `{}` of Endpoint `{}` declares `after = {gate}`, but `{}.{}` plays no role of `{gate}`: a Task's operations can only wait for its own receives", r.role, e.name, r.partition, r.task))),
                        Some(_) if e.protocol.is_none() => d.push(Diagnostic::new("R41.3", eref(), format!("role `{}` of plain Endpoint `{}` declares `after = {gate}`; gating is declared on Protocol-governed roles", r.role, e.name))),
                        Some(_) => {}
                    }
                }
            }
        }
        storage_bytes += (e.roles.len() * e.roles.len()) as u64 * e.capacity as u64 * e.payload as u64; // task 21.7
    }
    // Task 21.6 (R41.3, A-P8): the gating relation over the Protocol-governed
    // Endpoints (`after` edges, from the gating Endpoint to the gated one) is
    // acyclic, so that the composition theorem applies
    // (`rsk_protocol::compose::theorem_system_r41_3`; Verification_Plan 5.15).
    // Kahn's algorithm; a remainder is a cycle, reported at its first member.
    {
        let protocol_endpoints: Vec<&crate::decl::EndpointDecl> = decl.endpoints.iter().filter(|e| e.protocol.is_some()).collect();
        let mut edges: Vec<(String, String)> = protocol_endpoints.iter().flat_map(|e| e.roles.iter().flat_map(move |r| r.after.iter().filter(|g| decl.endpoints.iter().any(|x| &&x.name == g && x.protocol.is_some() && x.name != e.name)).map(move |g| (g.clone(), e.name.clone())))).collect();
        // Task 21.7 (R41.3, finding 3 of the independent review): a Task
        // released by an arrival on Protocol Endpoint E awaits it before its
        // operations on its other Protocol Endpoints: implicit edges E -> E'.
        for e in &protocol_endpoints {
            for r in &e.roles {
                if let Some(rel) = release_endpoint_of(decl, &r.partition, &r.task) {
                    if rel != e.name && decl.endpoints.iter().any(|x| x.name == rel && x.protocol.is_some()) {
                        edges.push((rel, e.name.clone()));
                    }
                }
            }
        }
        let mut remaining: Vec<String> = protocol_endpoints.iter().map(|e| e.name.clone()).collect();
        loop {
            let Some(pos) = remaining.iter().position(|n| !edges.iter().any(|(_, to)| to == n && remaining.contains(&edges.iter().find(|(f, t)| t == n && remaining.contains(f)).map(|(f, _)| f.clone()).unwrap_or_default()))) else { break };
            remaining.remove(pos);
        }
        if let Some(first) = remaining.first() {
            d.push(Diagnostic::new("R41.3", ItemRef::Endpoint(first.clone()), format!("the gating of Endpoints is cyclic among {}: a Task's operations on each wait for receives on another, which the deadlock-freedom proof cannot order (Verification_Plan 5.15, A-P8); declare `after` so that the relation is acyclic", remaining.iter().map(|n| format!("`{n}`")).collect::<Vec<_>>().join(", "))));
        }
    }
    // Chains (task 21.5; R41.4): unique names, at least two Tasks, each a
    // declared Task, each consecutive pair the two roles of one Endpoint;
    // a Sporadic_Task after the first is released by that Endpoint's role
    // (any other Release_Source would leave its latency unbounded by the
    // chain), and no hop into a `discard` policy (a discarded arrival
    // leaves the message waiting for a Release the chain does not bound).
    let mut chain_names = BTreeSet::new();
    for c in &decl.chains {
        let cref = || ItemRef::Chain(c.name.clone());
        if !chain_names.insert(c.name.as_str()) {
            d.push(Diagnostic::new("PR-28", cref(), format!("chain `{}` is declared twice", c.name)));
        }
        if c.tasks.len() < 2 {
            d.push(Diagnostic::new("R41.4", cref(), format!("chain `{}` names {} Task(s); a chain is at least two Tasks connected by Endpoints", c.name, c.tasks.len())));
        }
        let task_of = |pt: &(String, String)| decl.partitions.iter().find(|p| p.name == pt.0).and_then(|p| p.tasks.iter().find(|t| t.name == pt.1));
        for (i, pt) in c.tasks.iter().enumerate() {
            let Some(t) = task_of(pt) else {
                d.push(Diagnostic::new("R22.5", cref(), format!("chain `{}` names `{}.{}`, which is no declared Task", c.name, pt.0, pt.1)));
                continue;
            };
            if i == 0 {
                continue;
            }
            let prev = &c.tasks[i - 1];
            let link = decl.endpoints.iter().find(|e| {
                e.roles.iter().any(|r| r.partition == prev.0 && r.task == prev.1)
                    && e.roles.iter().any(|r| r.partition == pt.0 && r.task == pt.1)
            });
            match link {
                None => d.push(Diagnostic::new("R41.4", cref(), format!("chain `{}`: no Endpoint connects `{}.{}` to `{}.{}`", c.name, prev.0, prev.1, pt.0, pt.1))),
                Some(e) => {
                    if let Release::Sporadic { source, policy, .. } = &t.release {
                        let role = e.roles.iter().find(|r| r.partition == pt.0 && r.task == pt.1).map(|r| r.role.clone()).unwrap_or_default();
                        match source {
                            Source::Endpoint(name) if name == &e.name => {
                                if *policy == crate::decl::MitPolicy::Discard {
                                    d.push(Diagnostic::new("R41.4", cref(), format!("chain `{}`: `{}.{}` discards arrivals inside its MIT (`mit_policy = discard`), so a message may wait for a Release the chain cannot bound; declare `defer`", c.name, pt.0, pt.1)));
                                }
                            }
                            _ => d.push(Diagnostic::new("R41.4", cref(), format!("chain `{}`: the Sporadic_Task `{}.{}` is not released by its role `{role}` of Endpoint `{}` (`endpoint = {}`), so the chain does not bound when it reads the message", c.name, pt.0, pt.1, e.name, e.name))),
                        }
                    }
                }
            }
        }
        if c.deadline.value == 0 {
            d.push(Diagnostic::new("R41.4", cref(), format!("chain `{}` declares a zero deadline", c.name)));
        }
    }
    // Protocols (task 21.1): unique names; one bound to no Endpoint is a
    // declaration error, like a Resource with no accessor (R22.5).
    let mut protocol_names = BTreeSet::new();
    for p in &decl.protocols {
        if !protocol_names.insert(p.name.as_str()) {
            d.push(Diagnostic::new("PR-28", ItemRef::Protocol(p.name.clone()), format!("Protocol `{}` is declared twice", p.name)));
        }
        if !decl.endpoints.iter().any(|e| e.protocol.as_deref() == Some(p.name.as_str())) {
            d.push(Diagnostic::new("R22.5", ItemRef::Protocol(p.name.clone()), format!("Protocol `{}` governs no Endpoint", p.name)));
        }
    }
    if decl.endpoints.len() > MAX_ENDPOINTS {
        d.push(Diagnostic::new("PR-14", ItemRef::Declaration, format!("{} Endpoints declared; the Kernel holds at most {MAX_ENDPOINTS}", decl.endpoints.len())));
    }
    if storage_bytes > MAX_ENDPOINT_STORAGE {
        d.push(Diagnostic::new("PR-14", ItemRef::Declaration, format!("the Endpoints need {storage_bytes} bytes of Kernel storage; at most {MAX_ENDPOINT_STORAGE} (the Kernel's RAM region)")));
    }
    if task_count == 0 {
        d.push(Diagnostic::new("R21.3", ItemRef::Declaration, "at least one Task is required"));
    } else if task_count > target.dispatch_irqs.len() {
        d.push(Diagnostic::new(
            "DD-01",
            ItemRef::Declaration,
            format!(
                "{task_count} Tasks need {task_count} dispatch vectors; the Target `{}` has {} free interrupt lines",
                target.name,
                target.dispatch_irqs.len()
            ),
        ));
    }
    if task_count > 255 {
        d.push(Diagnostic::new("PR-14", ItemRef::Declaration, "at most 255 Tasks are supported"));
    }
    if d.is_empty() {
        Ok(target)
    } else {
        Err(d)
    }
}

/// The labels a role's local type receives with a payload (task 21.4).
/// The Endpoint whose arrival releases the Task, when it is a Sporadic_Task
/// with `endpoint = NAME` (R40.3).
pub fn release_endpoint_of(decl: &Declaration, partition: &str, task: &str) -> Option<String> {
    let t = decl.partitions.iter().find(|p| p.name == partition)?.tasks.iter().find(|t| t.name == task)?;
    match &t.release {
        Release::Sporadic { source: Source::Endpoint(name), .. } => Some(name.clone()),
        _ => None,
    }
}

/// The labels a role receives with a payload, each with the role it comes
/// from (R42.4; task 21.7: a role of a multiparty Protocol may receive from
/// several roles).
fn received_messages(l: &rsk_protocol::Local, out: &mut Vec<(String, String)>) {
    use rsk_protocol::Local;
    match l {
        Local::End | Local::Continue(_) => {}
        Local::Rec { body, .. } => received_messages(body, out),
        Local::Send { branches, .. } => {
            for b in branches {
                received_messages(&b.cont, out);
            }
        }
        Local::Recv { from, branches } => {
            for b in branches {
                if b.payload.is_some() {
                    out.push((b.label.clone(), from.clone()));
                }
                received_messages(&b.cont, out);
            }
        }
    }
}
