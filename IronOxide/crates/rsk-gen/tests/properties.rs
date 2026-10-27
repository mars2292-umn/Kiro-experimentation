//! Property-based tests of the Generator (tasks 10.3, 10.4, 10.6):
//! Property 13 (order independence and determinism, R23.2, R23.6, R24.4),
//! Property 14 (conversion direction, R21.2), and Property 12 carried over
//! as layout realizability (R20.4, R24.5). The declarations come from
//! `rsk_gen::arbitrary`; proptest shrinks a failing case to a minimal one.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use proptest::prelude::*;
use rsk_gen::arbitrary::{any_sizes, declaration, declaration_with_sizes, permuted};
use rsk_gen::convert::{to_cycles, to_ticks, Rounding};
use rsk_gen::decl::{ItemRef, Time, TimeUnit};
use rsk_gen::target::TARGETS;
use rsk_gen::{generate, layout, outputs};

fn config() -> ProptestConfig {
    // No regression files: a failure prints its minimal case, which is the record.
    ProptestConfig { cases: 200, failure_persistence: None, ..ProptestConfig::default() }
}

/// The derived values by name: Resource Ceilings, Task nesting depths and
/// Resource sets, capacities, and each Partition's placed regions.
fn derived(d: &rsk_gen::Declaration) -> (BTreeMap<String, u8>, BTreeMap<String, (u8, Vec<String>)>, (usize, usize), BTreeMap<String, (u32, u8, u32, u8, u32)>) {
    let g = generate(d).unwrap_or_else(|e| panic!("rejected: {e:?}"));
    let ceilings = g.resources.iter().map(|r| (r.id.clone(), r.ceiling)).collect();
    let tasks = g
        .tasks
        .iter()
        .map(|t| {
            let mut res: Vec<String> = t.resources.iter().map(|&r| g.resources[r].id.clone()).collect();
            res.sort();
            (t.id.clone(), (t.nesting_depth, res))
        })
        .collect();
    let regions = g.plan.partitions.iter().map(|p| (p.name.clone(), (p.code.base, p.code.size_log2, p.ram.base, p.ram.size_log2, p.stack))).collect();
    (ceilings, tasks, (g.slot_count, g.lock_capacity), regions)
}

proptest! {
    #![proptest_config(config())]

    /// R23.2, R23.6, Property 13: every permutation of the declaration
    /// order of Partitions, Tasks, Resources, and signals gives the same
    /// Ceilings, nesting depths, Resource sets, capacities, and layout.
    #[test]
    fn r23_2_derived_values_are_independent_of_declaration_order(d in declaration(), rotation in 0usize..3) {
        let p = permuted(&d, rotation);
        prop_assert_eq!(derived(&d), derived(&p));
    }

    /// R23.6, R24.4, Property 13: two runs on the same declaration give
    /// byte-identical Generated_Config, Task_Model, and every other output.
    #[test]
    fn r24_4_outputs_are_deterministic(d in declaration()) {
        let a = outputs(&d, "rsk-system").unwrap_or_else(|e| panic!("rejected: {e:?}"));
        let b = outputs(&d, "rsk-system").unwrap();
        prop_assert_eq!(&a.task_model_json, &b.task_model_json);
        prop_assert_eq!(&a.system_source, &b.system_source);
        prop_assert_eq!(&a.partition_sources, &b.partition_sources);
        prop_assert_eq!(&a.memory_x, &b.memory_x);
        prop_assert_eq!(&a.link_x, &b.link_x);
        prop_assert_eq!(&a.summary, &b.summary);
        prop_assert_eq!(a.generated.checksum, b.generated.checksum);
    }

    /// R21.2, Property 14: a conversion to ticks or cycles rounds in the
    /// requested direction (Budgets up; periods, MITs, deadlines down;
    /// offsets to the nearest), is exact when the value divides, and
    /// reports a change exactly when it was inexact.
    #[test]
    fn r21_2_conversions_round_in_the_declared_direction(
        value in 0u64..=1_000_000_000,
        unit in prop::sample::select(vec![TimeUnit::Ns, TimeUnit::Us, TimeUnit::Ms, TimeUnit::S, TimeUnit::Min, TimeUnit::H, TimeUnit::Ticks, TimeUnit::Cycles]),
        tick_hz in prop::sample::select(vec![16_000_000u64, 25_000_000, 32_768, 1_000_000]),
        cpu_hz in prop::sample::select(vec![64_000_000u64, 25_000_000, 48_000_000]),
        rounding in prop::sample::select(vec![Rounding::Up, Rounding::Down, Rounding::Nearest]),
    ) {
        let t = Time { value, unit };
        // The exact value as a fraction num/den, for each conversion.
        let (num_t, den_t): (u128, u128) = match unit {
            TimeUnit::Ticks => (value as u128, 1),
            TimeUnit::Cycles => (value as u128 * tick_hz as u128, cpu_hz as u128),
            u => (value as u128 * u.nanos().unwrap() as u128 * tick_hz as u128, 1_000_000_000),
        };
        let (num_c, den_c): (u128, u128) = match unit {
            TimeUnit::Cycles => (value as u128, 1),
            TimeUnit::Ticks => (value as u128 * cpu_hz as u128, tick_hz as u128),
            u => (value as u128 * u.nanos().unwrap() as u128 * cpu_hz as u128, 1_000_000_000),
        };
        for ((num, den), result) in [((num_t, den_t), to_ticks(t, tick_hz, cpu_hz, rounding)), ((num_c, den_c), to_cycles(t, tick_hz, cpu_hz, rounding))] {
            // `None` only when the rounded value does not fit a u64.
            let Some((r, changed)) = result else {
                prop_assert!(num / den + 1 > u64::MAX as u128, "overflow reported for a representable value");
                continue;
            };
            let r = r as u128;
            let exact = num % den == 0;
            prop_assert_eq!(changed, !exact, "change reported iff inexact");
            match rounding {
                Rounding::Down => prop_assert!(r * den <= num && num < (r + 1) * den),
                Rounding::Up => prop_assert!(num <= r * den && (r == 0 || (r - 1) * den < num)),
                Rounding::Nearest => {
                    let diff = if r * den >= num { r * den - num } else { num - r * den };
                    prop_assert!(diff * 2 <= den);
                }
            }
        }
    }

    /// R20.4, R24.5, Property 12 (layout realizability): for any Partition
    /// sizes, the planner either places every Partition in power-of-two
    /// sized, aligned, disjoint regions inside the Target's memories, above
    /// the Kernel's, each holding at least the declared bytes and the
    /// stack within the RAM region, or rejects the declaration with R20.4
    /// naming a declared Partition.
    #[test]
    fn r20_4_layouts_are_realizable_or_rejected_naming_the_partition(d in declaration_with_sizes(any_sizes)) {
        let target = TARGETS.iter().find(|t| t.name == d.target).unwrap();
        match layout::plan(&d, target) {
            Ok(plan) => {
                let flash_lo = (target.flash.base + target.kernel_flash + target.shared_code) as u64;
                let flash_hi = target.flash.base as u64 + target.flash.size as u64;
                let ram_lo = (target.ram.base + target.kernel_ram) as u64;
                let ram_hi = target.ram.base as u64 + target.ram.size as u64;
                let mut codes: Vec<(u64, u64)> = Vec::new();
                let mut rams: Vec<(u64, u64)> = Vec::new();
                for p in &plan.partitions {
                    let decl = d.partitions.iter().find(|q| q.name == p.name).expect("a declared Partition");
                    for (region, lo, hi, want, list) in [(&p.code, flash_lo, flash_hi, decl.code, &mut codes), (&p.ram, ram_lo, ram_hi, decl.ram, &mut rams)] {
                        let size = 1u64 << region.size_log2;
                        prop_assert!(region.size_log2 >= 5, "Armv7-M regions are at least 32 bytes");
                        prop_assert_eq!(region.base as u64 % size, 0, "aligned to its size");
                        prop_assert!(region.usable as u64 >= want && region.usable as u64 <= size, "holds the declared bytes");
                        prop_assert!(region.base as u64 >= lo && region.base as u64 + size <= hi, "inside the Target memory above the Kernel");
                        list.push((region.base as u64, region.base as u64 + size));
                    }
                    prop_assert!(p.stack as u64 >= decl.stack && p.stack as u64 <= p.ram.usable as u64, "stack within the RAM region");
                }
                for list in [&codes, &rams] {
                    for i in 0..list.len() {
                        for j in i + 1..list.len() {
                            prop_assert!(list[i].1 <= list[j].0 || list[j].1 <= list[i].0, "regions are disjoint");
                        }
                    }
                }
            }
            Err(errors) => {
                prop_assert!(!errors.is_empty());
                for e in &errors {
                    prop_assert_eq!(&e.rule, "R20.4");
                    match &e.item {
                        ItemRef::PartitionField(p, _) | ItemRef::Partition(p) => prop_assert!(d.partitions.iter().any(|q| &q.name == p), "names a declared Partition"),
                        other => prop_assert!(false, "unexpected item {other}"),
                    }
                }
            }
        }
    }
}
