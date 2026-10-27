//! Generators of valid App_Declarations for property-based tests
//! (Properties 12 to 15; feature `arbitrary`, tests only). Every value
//! `declaration()` yields is one the validator accepts for the nRF52840
//! Target: up to three Partitions, up to seven Tasks (the Target's dispatch
//! vectors), Priorities 1 to 7 with ties, periods in several units,
//! deadlines at or below the period, Budgets below the deadline, offsets,
//! Resources with random accessor sets (each with at least one), and
//! Sporadic_Tasks released by a Release_Signal that the first (periodic)
//! Task of their Partition raises. `declaration_with_sizes` varies the
//! Partition memory sizes freely for the layout property.

use proptest::prelude::*;

use crate::decl::*;

/// A Partition's memory sizes within what the Target comfortably holds.
pub fn fitting_sizes() -> impl Strategy<Value = (Bytes, Bytes, Bytes)> {
    (
        prop::sample::select(vec![8u64 * 1024, 16 * 1024, 24 * 1024, 32 * 1024, 48 * 1024, 64 * 1024]),
        prop::sample::select(vec![8u64 * 1024, 16 * 1024, 24 * 1024, 32 * 1024, 48 * 1024, 64 * 1024]),
        prop::sample::select(vec![1024u64, 2048, 4096]),
    )
}

/// Memory sizes over the whole range, fitting or not (Property 12).
pub fn any_sizes() -> impl Strategy<Value = (Bytes, Bytes, Bytes)> {
    (1024u64..=2 * 1024 * 1024, 1024u64..=512 * 1024, 256u64..=8192)
}

/// A period: value and unit, in a range the time base represents and the
/// Budget fractions below keep above a microsecond.
fn period() -> impl Strategy<Value = Time> {
    prop_oneof![
        (5u64..=500).prop_map(|v| Time { value: v, unit: TimeUnit::Ms }),
        (2_000u64..=500_000).prop_map(|v| Time { value: v, unit: TimeUnit::Us }),
        (1u64..=2).prop_map(|v| Time { value: v, unit: TimeUnit::S }),
        (100_000u64..=8_000_000).prop_map(|v| Time { value: v, unit: TimeUnit::Ticks }),
    ]
}

/// One Task's parameters before placement.
#[derive(Clone, Debug)]
struct TaskSpec {
    partition: usize,
    priority: u8,
    period: Time,
    /// Deadline as `period * deadline_num / 4` (2..=4: at or below the period).
    deadline_num: u8,
    /// Budget as a fraction of the deadline, in microseconds: `deadline / budget_div`.
    budget_div: u8,
    sporadic: bool,
    discard: bool,
    fpu: bool,
    /// Offset as `period * offset_num / 8`, `None` for no offset.
    offset_num: Option<u8>,
    resource_mask: u8,
}

fn task_spec(partitions: usize) -> impl Strategy<Value = TaskSpec> {
    (0..partitions, 1u8..=7, period(), 2u8..=4, 2u8..=8, any::<bool>(), any::<bool>(), any::<bool>(), prop::option::of(0u8..=3), any::<u8>()).prop_map(
        |(partition, priority, period, deadline_num, budget_div, sporadic, discard, fpu, offset_num, resource_mask)| TaskSpec {
            partition,
            priority,
            period,
            deadline_num,
            budget_div,
            sporadic,
            discard,
            fpu,
            offset_num,
            resource_mask,
        },
    )
}

/// One Partition's parameters.
#[derive(Clone, Debug)]
struct PartitionSpec {
    sizes: (Bytes, Bytes, Bytes),
    level: Level,
    overrun: Response,
    mit_threshold: u32,
    resources: usize,
}

fn partition_spec<S: Strategy<Value = (Bytes, Bytes, Bytes)>>(sizes: S) -> impl Strategy<Value = PartitionSpec> {
    (
        sizes,
        prop::sample::select(vec![Level::A, Level::B, Level::C]),
        prop::sample::select(vec![Response::EndJob, Response::RecordAndContinue]),
        1u32..=100,
        0usize..=2,
    )
        .prop_map(|(sizes, level, overrun, mit_threshold, resources)| PartitionSpec { sizes, level, overrun, mit_threshold, resources })
}

/// Microseconds of a time value, for the Budget fractions (every unit the
/// generator uses is at least a microsecond, or ticks at 16 MHz).
fn micros(t: Time) -> u64 {
    match t.unit {
        TimeUnit::Us => t.value,
        TimeUnit::Ms => t.value * 1000,
        TimeUnit::S => t.value * 1_000_000,
        TimeUnit::Ticks => t.value / 16,
        _ => t.value,
    }
}

fn scaled(t: Time, num: u64, den: u64) -> Time {
    Time { value: (t.value * num / den).max(1), unit: t.unit }
}

fn build(parts: Vec<PartitionSpec>, tasks: Vec<TaskSpec>, duration_s: u64, log_capacity: u32) -> Declaration {
    // RECORD_AND_CONTINUE is permitted only at the Application's highest
    // Criticality_Level (R18.5, R18.6).
    let highest = parts.iter().map(|p| p.level.code()).min().unwrap_or(0);
    let mut partitions: Vec<PartitionDecl> = parts
        .iter()
        .enumerate()
        .map(|(p, spec)| PartitionDecl {
            name: format!("part{p}"),
            krate: format!("arb_part{p}"),
            level: spec.level,
            code: spec.sizes.0,
            ram: spec.sizes.1,
            stack: spec.sizes.2,
            fault: Response::RestartPartition,
            overrun: if spec.overrun == Response::RecordAndContinue && spec.level.code() != highest { Response::EndJob } else { spec.overrun },
            deadline_miss: Response::RecordOnly,
            mit_threshold: spec.mit_threshold,
            peripherals: vec![],
            resources: (0..spec.resources)
                .map(|q| ResourceDecl {
                    name: format!("r{p}{q}"),
                    invariant: None,
                    ty: "u32".into(),
                    init: "0".into(),
                    placement: ResourcePlacement::Core(0),
                })
                .collect(),
            tasks: vec![],
            signals: vec![],
            init_arena: 0,
        })
        .collect();
    // Task k < partitions.len() belongs to Partition k and is periodic, so
    // that every Partition has a periodic Task to raise its signals.
    for (k, spec) in tasks.iter().enumerate() {
        let p = if k < partitions.len() { k } else { spec.partition };
        let sporadic = spec.sporadic && k >= partitions.len();
        let deadline = scaled(spec.period, spec.deadline_num as u64, 4);
        let budget = Time { value: (micros(deadline) / spec.budget_div as u64).max(1), unit: TimeUnit::Us };
        let resources: Vec<String> = partitions[p]
            .resources
            .iter()
            .enumerate()
            .filter(|(q, _)| spec.resource_mask & (1 << q) != 0)
            .map(|(_, r)| r.name.clone())
            .collect();
        let release = if sporadic {
            partitions[p].signals.push(SignalDecl { name: format!("sig{k}") });
            Release::Sporadic {
                mit: spec.period,
                policy: if spec.discard { MitPolicy::Discard } else { MitPolicy::Defer },
                source: Source::Signal(format!("sig{k}")),
            }
        } else {
            Release::Periodic {
                period: spec.period,
                offset: spec.offset_num.map(|n| scaled(spec.period, n as u64, 8)).filter(|o| o.value > 0),
            }
        };
        partitions[p].tasks.push(TaskDecl {
            name: format!("t{k}"),
            release,
            deadline,
            budget,
            priority: spec.priority,
            core: 0,
            fpu: spec.fpu,
            resources,
            state: vec![],
            raises: vec![],
        });
    }
    for part in &mut partitions {
        // The first Task (periodic by construction) raises the Partition's signals.
        let signals: Vec<String> = part.signals.iter().map(|s| s.name.clone()).collect();
        if let Some(first) = part.tasks.first_mut() {
            first.raises = signals;
        }
        // Every Resource has an accessor (R22.5).
        for r in 0..part.resources.len() {
            let name = part.resources[r].name.clone();
            if !part.tasks.iter().any(|t| t.resources.contains(&name)) {
                part.tasks[0].resources.push(name);
            }
        }
    }
    Declaration {
        target: "nrf52840".into(),
        variant: "Ravenscar".into(),
        operating_duration: Time { value: duration_s, unit: TimeUnit::S },
        safe_state: "crate::safe_outputs".into(),
        log_capacity,
        partitions,
    }
}

/// A valid declaration (see the module documentation).
pub fn declaration() -> impl Strategy<Value = Declaration> {
    declaration_with_sizes(fitting_sizes)
}

/// A declaration whose Partition memory sizes come from `sizes`; valid
/// except where the sizes do not fit the Target (Property 12).
pub fn declaration_with_sizes<S, F>(sizes: F) -> impl Strategy<Value = Declaration>
where
    S: Strategy<Value = (Bytes, Bytes, Bytes)> + 'static,
    F: Fn() -> S + Clone + 'static,
{
    (1usize..=3).prop_flat_map(move |n_parts| {
        let sizes = sizes.clone();
        (n_parts..=7usize).prop_flat_map(move |n_tasks| {
            (
                prop::collection::vec(partition_spec(sizes()), n_parts),
                prop::collection::vec(task_spec(n_parts), n_tasks),
                // At least the longest period (2 s), R22.6.
                3u64..=3600,
                1u32..=1024,
            )
                .prop_map(|(parts, tasks, duration_s, log_capacity)| build(parts, tasks, duration_s, log_capacity))
        })
    })
}

/// The same declaration with every list in a different order: the
/// Partitions rotated by `rotation`, and the Tasks, Resources, and signals
/// of each Partition reversed (Property 13: order independence).
pub fn permuted(decl: &Declaration, rotation: usize) -> Declaration {
    let mut d = decl.clone();
    let n = d.partitions.len();
    if n > 0 {
        d.partitions.rotate_left(rotation % n);
    }
    for p in &mut d.partitions {
        p.tasks.reverse();
        p.resources.reverse();
        p.signals.reverse();
        for t in &mut p.tasks {
            t.resources.reverse();
            t.raises.reverse();
        }
    }
    d
}
