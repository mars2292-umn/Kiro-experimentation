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
                Some(t) if t.easy_dma => d.push(Diagnostic::new(
                    "DD-08",
                    iref,
                    format!("peripheral `{per}` is EasyDMA-capable; the Kernel's DMA mediation service (task 5.5) is not implemented yet, so it cannot be assigned to a Partition"),
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
