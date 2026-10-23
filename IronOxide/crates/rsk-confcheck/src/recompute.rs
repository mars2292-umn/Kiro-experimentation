//! The recomputation of every R26.2 value class from the declared values,
//! and the comparison with the Task_Model's derived values (`check_model`)
//! and with the tables extracted from the binary (`check_binary`).

use crate::image::{self, Image, LayoutImg, Tables};
use crate::model::{Model, Region};
use crate::Outcome;

/// The Kernel-owned peripherals of each Target (R15.6, PR-29): the time
/// base and the core peripherals a Partition may never own. The checker's
/// own list, kept in step with the Generator's Target table by the
/// comparison on the conformance Application.
pub fn kernel_owned(target: &str) -> &'static [&'static str] {
    match target {
        "nrf52840" => &["CLOCK", "TIMER0", "WDT", "PPI"],
        "qemu-mps2-an386" => &["CMSDK_TIMER0", "CMSDK_TIMER1"],
        _ => &[],
    }
}

/// The EasyDMA-capable peripherals of each Target (DD-08): their windows
/// are read-only to the owning Partition.
pub fn easy_dma(target: &str) -> &'static [&'static str] {
    match target {
        "nrf52840" => &["RADIO", "UARTE0", "SPIM0", "SPIM1", "SAADC", "PWM0"],
        _ => &[],
    }
}

/// DD-02: the NVIC priority byte of a Task level (3 implemented bits).
pub fn encode_level(level: u64) -> u64 {
    if level == 0 { 0 } else { (8 - level) << 5 }
}

fn size_of(r: &Region) -> u64 {
    1u64 << r.size_log2
}

fn disjoint(a: &Region, b: &Region) -> bool {
    a.base + size_of(a) <= b.base || b.base + size_of(b) <= a.base
}

fn aligned(r: &Region) -> bool {
    r.size_log2 >= 5 && r.size_log2 <= 32 && r.base % size_of(r) == 0
}

/// The recomputed values against the Task_Model's derived values.
pub fn check_model(m: &Model, out: &mut Outcome) {
    // Ceilings (R8.7, R23): the maximum Priority of the accessors.
    for res in &m.resources {
        let mut ceiling = 0;
        for a in &res.accessors {
            match m.task_index(a) {
                Some(t) => ceiling = ceiling.max(m.tasks[t].priority),
                None => out.require(format!("{}.accessors", res.id), false, "a declared Task", format!("`{a}` (no such Task)")),
            }
        }
        out.mismatch(format!("{}.ceiling", res.id), ceiling, res.ceiling);
        // Every accessor lists the Resource, and the Resource's Partition is theirs.
        for a in &res.accessors {
            if let Some(t) = m.task_index(a) {
                out.require(format!("{a}.resources"), m.tasks[t].resources.contains(&res.id), format!("contains {}", res.id), "absent");
                out.mismatch(format!("{}.partition (accessor {a})", res.id), &m.tasks[t].partition, &res.partition);
            }
        }
    }
    // Priority-to-hardware mapping (DD-02): every Task level once, ascending,
    // with its encoding; the Kernel level is 0.
    let mut levels: Vec<u64> = m.tasks.iter().map(|t| t.priority).collect();
    levels.sort_unstable();
    levels.dedup();
    let expected_map: Vec<(u64, u64)> = levels.iter().map(|&l| (l, encode_level(l))).collect();
    out.mismatch("priority_map", format!("{expected_map:?}"), format!("{:?}", m.priority_map));
    out.mismatch("kernel_levels", "[0]", format!("{:?}", m.kernel_levels));
    for t in &m.tasks {
        out.require(format!("{}.priority", t.id), (1..=7).contains(&t.priority), "1 to 7", t.priority);
    }
    // Nesting depths (PR-07): the number of Resources the Task accesses.
    for t in &m.tasks {
        out.mismatch(format!("{}.nesting_depth", t.id), t.resources.len(), t.nesting_depth);
        for r in &t.resources {
            match m.resources.iter().find(|x| &x.id == r) {
                Some(res) => out.require(format!("{}.accessors", res.id), res.accessors.contains(&t.id), format!("contains {}", t.id), "absent"),
                None => out.require(format!("{}.resources", t.id), false, "a declared Resource", format!("`{r}` (no such Resource)")),
            }
        }
    }
    // Capacities (R23.4): the lock stack holds every Task's nested sections at
    // once; the timer queue holds three slots per Task plus the end-of-operation
    // and housekeeping slots (R9.4); the log capacity is declared.
    let depth_sum: u64 = m.tasks.iter().map(|t| t.resources.len() as u64).sum();
    out.mismatch("capacities.lock_stack", depth_sum, m.lock_stack);
    out.mismatch("capacities.timer_slots", 3 * m.tasks.len() as u64 + 2, m.timer_slots);
    out.require("capacities.event_log", m.event_log > 0, "> 0", m.event_log);
    // Slots: release 3t, deadline 3t+1, budget 3t+2 (periodic and deferring
    // sporadic Tasks have a release slot; discarding ones have none).
    for (i, t) in m.tasks.iter().enumerate() {
        let i = i as u64;
        let has_release = !t.sporadic || t.mit_policy.as_deref() == Some("defer");
        out.mismatch(format!("{}.slots.release", t.id), if has_release { 3 * i } else { image::NO_SLOT as u64 }, t.slot_release);
        out.mismatch(format!("{}.slots.deadline", t.id), 3 * i + 1, t.slot_deadline);
        out.mismatch(format!("{}.slots.budget", t.id), 3 * i + 2, t.slot_budget);
    }
    // Vectors: dispatch vectors ascend with the Task index (DD-03).
    for w in m.tasks.windows(2) {
        out.require(format!("{}.vector", w[1].id), w[1].vector > w[0].vector, format!("> {}", w[0].vector), w[1].vector);
    }
    // Declared timing: deadline within period or MIT, positive budget.
    for t in &m.tasks {
        out.require(format!("{}.deadline_ticks", t.id), t.deadline_ticks > 0 && t.deadline_ticks <= t.period_ticks, format!("1 to {}", t.period_ticks), t.deadline_ticks);
        out.require(format!("{}.budget_cycles", t.id), t.budget_cycles > 0, "> 0", t.budget_cycles);
    }
    // Kernel-owned peripherals (R15.6) and single ownership (PR-28).
    let owned_by_kernel = kernel_owned(&m.target);
    let dma = easy_dma(&m.target);
    let mut seen: Vec<&str> = Vec::new();
    for p in &m.partitions {
        for per in &p.peripherals {
            out.require(format!("{}.peripherals ({per})", p.id), !owned_by_kernel.contains(&per.as_str()), "a Partition-assignable peripheral", "Kernel-owned (R15.6)");
            out.require(format!("{}.peripherals ({per})", p.id), !seen.contains(&per.as_str()), "owned by one Partition", "owned twice");
            seen.push(per);
        }
        // MPU permissions (DD-07, DD-08, R16.5).
        out.require(format!("{}.mpu.code", p.id), aligned(&p.code), "an aligned power-of-two region", format!("{:?}", p.code));
        out.require(format!("{}.mpu.ram", p.id), aligned(&p.ram), "an aligned power-of-two region", format!("{:?}", p.ram));
        out.require(format!("{}.regions.stack", p.id), p.stack > 0 && p.stack <= size_of(&p.ram), format!("1 to {}", size_of(&p.ram)), p.stack);
        out.require(format!("{}.mpu.code/ram", p.id), disjoint(&p.code, &p.ram), "disjoint", "overlapping");
        for k in [&m.kernel_flash, &m.shared_code, &m.kernel_ram] {
            out.require(format!("{}.mpu vs Kernel regions", p.id), disjoint(&p.code, k) && disjoint(&p.ram, k), "disjoint", "overlapping");
        }
        let mut windowed: Vec<&str> = Vec::new();
        for w in &p.windows {
            out.require(format!("{}.mpu.windows (0x{:08x})", p.id, w.region.base), aligned(&w.region) && w.region.size_log2 == 12, "a 4 KiB aligned window", format!("{:?}", w.region));
            let any_dma = w.peripherals.iter().any(|x| dma.contains(&x.as_str()));
            out.mismatch(format!("{}.mpu.windows (0x{:08x}).read_only", p.id, w.region.base), any_dma, w.read_only);
            for x in &w.peripherals {
                out.require(format!("{}.mpu.windows ({x})", p.id), p.peripherals.contains(x), "an owned peripheral", "not owned");
                windowed.push(x);
            }
        }
        for per in &p.peripherals {
            out.require(format!("{}.peripherals ({per})", p.id), windowed.contains(&per.as_str()), "a window", "no window");
        }
    }
    for pair in [(&m.kernel_flash, &m.shared_code), (&m.kernel_flash, &m.kernel_ram), (&m.shared_code, &m.kernel_ram)] {
        out.require("layout (Kernel regions)", disjoint(pair.0, pair.1), "disjoint", "overlapping");
    }
    for (i, a) in m.partitions.iter().enumerate() {
        for b in &m.partitions[i + 1..] {
            out.require(format!("{}/{} regions", a.id, b.id), disjoint(&a.code, &b.code) && disjoint(&a.ram, &b.ram), "disjoint", "overlapping");
        }
    }
}

/// The `LAYOUT` image extracted from the binary against the Task_Model's
/// MPU values (R26.2 MPU permissions, R16.7) and its checksums.
pub fn check_layout_binary(m: &Model, img: &Image, l: &LayoutImg, out: &mut Outcome) {
    let computed = image::crc32(&img.bytes);
    out.mismatch("RSK_LAYOUT_CHECKSUM (CRC-32 of the extracted image)", format!("0x{computed:08x}"), format!("0x{:08x}", img.stored_checksum));
    out.mismatch("task_model.layout_checksum", format!("0x{computed:08x}"), format!("0x{:08x}", m.layout_checksum));
    let same = |item: &str, want: &Region, got: &image::IntervalImg, out: &mut Outcome| {
        out.mismatch(format!("{item}.base"), want.base, got.base);
        out.mismatch(format!("{item}.size_log2"), want.size_log2, got.size_log2);
        out.mismatch(format!("{item}.srd"), want.srd, got.srd);
    };
    same("LAYOUT.kernel_flash", &m.kernel_flash, &l.kernel_flash, out);
    same("LAYOUT.shared_code", &m.shared_code, &l.shared_code, out);
    same("LAYOUT.kernel_ram", &m.kernel_ram, &l.kernel_ram, out);
    out.mismatch("LAYOUT.NP", m.partitions.len(), l.partitions.len());
    for (p, part) in m.partitions.iter().enumerate() {
        let Some(c) = l.partitions.get(p) else { continue };
        let item = |f: &str| format!("LAYOUT.partitions[{p}].{f} ({})", part.id);
        same(&item("code"), &part.code, &c.code, out);
        same(&item("ram"), &part.ram, &c.ram, out);
        out.mismatch(item("stack_size"), part.stack, c.stack_size);
        out.mismatch(item("window_count"), part.windows.len(), c.window_count);
        for (w, slot) in c.windows.iter().enumerate() {
            let want = part.windows.get(w).map_or((0u64, 5u8), |win| (win.region.base, win.region.size_log2));
            out.mismatch(item(&format!("windows[{w}]")), format!("{:?}", want), format!("{:?}", (slot.0 as u64, slot.1)));
        }
    }
}

/// The tables extracted from the binary against the declared values
/// (R26.4), and the checksums (R25.2).
pub fn check_binary(m: &Model, img: &Image, t: &Tables, out: &mut Outcome) {
    let computed = image::crc32(&img.bytes);
    out.mismatch("RSK_CONFIG_CHECKSUM (CRC-32 of the extracted image)", format!("0x{computed:08x}"), format!("0x{:08x}", img.stored_checksum));
    out.mismatch("task_model.config_checksum", format!("0x{computed:08x}"), format!("0x{:08x}", m.config_checksum));
    out.mismatch("CONFIG.NT", m.tasks.len(), t.tasks.len());
    out.mismatch("CONFIG.NR", m.resources.len(), t.resources.len());
    out.mismatch("CONFIG.NP", m.partitions.len(), t.partitions.len());
    out.mismatch("CONFIG.operating_duration", m.operating_duration_ticks, t.operating_duration);
    out.mismatch("CONFIG.slot_count", 3 * m.tasks.len() + 2, t.slot_count);
    for (i, task) in m.tasks.iter().enumerate() {
        let Some(c) = t.tasks.get(i) else { continue };
        let item = |f: &str| format!("CONFIG.tasks[{i}].{f} ({})", task.id);
        out.mismatch(item("priority"), task.priority, c.priority);
        out.mismatch(item("partition"), m.partition_index(&task.partition).map_or(255, |p| p as u8), c.partition);
        out.mismatch(item("kind"), if task.sporadic { image::KIND_SPORADIC } else { image::KIND_PERIODIC }, c.kind);
        out.mismatch(item("offset"), task.offset_ticks, c.offset);
        out.mismatch(item("period/mit"), task.period_ticks, c.period);
        out.mismatch(item("deadline"), task.deadline_ticks, c.deadline);
        out.mismatch(item("budget"), task.budget_cycles, c.budget);
        let policy = match task.mit_policy.as_deref() {
            Some("discard") => image::MIT_DISCARD,
            _ => image::MIT_DEFER,
        };
        out.mismatch(item("mit_policy"), policy, c.mit_policy);
        out.mismatch(item("fpu"), task.fpu, c.fpu);
        if let (Some(a), Some(b), Some(d)) = (t.slot_release.get(i), t.slot_deadline.get(i), t.slot_budget.get(i)) {
            out.mismatch(item("slot_release"), task.slot_release, *a);
            out.mismatch(item("slot_deadline"), task.slot_deadline, *b);
            out.mismatch(item("slot_budget"), task.slot_budget, *d);
        }
        for (r, res) in m.resources.iter().enumerate() {
            if let Some(row) = t.access.get(i) {
                if let Some(bit) = row.get(r) {
                    out.mismatch(format!("CONFIG.access[{i}][{r}] ({} / {})", task.id, res.id), res.accessors.contains(&task.id), *bit);
                }
            }
        }
    }
    for (r, res) in m.resources.iter().enumerate() {
        let Some(c) = t.resources.get(r) else { continue };
        let ceiling = res.accessors.iter().filter_map(|a| m.task_index(a)).map(|i| m.tasks[i].priority).max().unwrap_or(0);
        out.mismatch(format!("CONFIG.resources[{r}].ceiling ({})", res.id), ceiling, c.ceiling);
        out.mismatch(format!("CONFIG.resources[{r}].partition ({})", res.id), m.partition_index(&res.partition).map_or(255, |p| p as u8), c.partition);
    }
    for (p, part) in m.partitions.iter().enumerate() {
        let Some(c) = t.partitions.get(p) else { continue };
        let item = |f: &str| format!("CONFIG.partitions[{p}].{f} ({})", part.id);
        out.mismatch(item("level"), image::level_code(&part.level).map_or("?".to_string(), |v| v.to_string()), c.level);
        out.mismatch(item("fault_response"), image::response_code(&part.fault).map_or("?".to_string(), |v| v.to_string()), c.fault_response);
        out.mismatch(item("overrun_response"), image::response_code(&part.overrun).map_or("?".to_string(), |v| v.to_string()), c.overrun_response);
        out.mismatch(item("deadline_response"), image::response_code(&part.deadline_miss).map_or("?".to_string(), |v| v.to_string()), c.deadline_response);
        out.mismatch(item("mit_threshold"), part.mit_threshold, c.mit_threshold);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");

    fn demo() -> Model {
        Model::parse(&serde_json::from_str(DEMO).unwrap()).unwrap()
    }

    /// The generated demo Task_Model's derived values are what this
    /// independent recomputation gives (R26.2 against the Generator).
    #[test]
    fn the_demo_task_model_recomputes_without_discrepancy() {
        let mut out = Outcome::default();
        check_model(&demo(), &mut out);
        assert!(out.discrepancies.is_empty(), "{:?}", out.discrepancies);
        assert!(out.checked > 40);
    }

    /// Altered derived values are reported with the item, the expected and
    /// the found value (R26.3): one case per value class.
    #[test]
    fn altered_derived_values_are_reported_per_class() {
        let cases: &[(&str, &str, &str)] = &[
            ("\"ceiling\": 7", "\"ceiling\": 5", "part0.state.ceiling"),
            ("\"nvic\": 32", "\"nvic\": 64", "priority_map"),
            ("\"nesting_depth\": 1,\n      \"slots\": {\n        \"release\": 0", "\"nesting_depth\": 2,\n      \"slots\": {\n        \"release\": 0", "part0.control.nesting_depth"),
            ("\"lock_stack\": 2", "\"lock_stack\": 3", "capacities.lock_stack"),
            ("\"timer_slots\": 17", "\"timer_slots\": 16", "capacities.timer_slots"),
            ("\"peripherals\": [\n        \"CMSDK_DUALTIMER\"\n      ],\n      \"mpu\"", "\"peripherals\": [\n        \"CMSDK_DUALTIMER\",\n        \"CMSDK_TIMER0\"\n      ],\n      \"mpu\"", "part1.peripherals (CMSDK_TIMER0)"),
            ("\"read_only\": false", "\"read_only\": true", "read_only"),
        ];
        for (from, to, item) in cases {
            assert!(DEMO.contains(from), "test fixture drifted: `{from}` not in the demo model");
            let altered = DEMO.replacen(from, to, 1);
            let model = Model::parse(&serde_json::from_str(&altered).unwrap()).unwrap();
            let mut out = Outcome::default();
            check_model(&model, &mut out);
            assert!(out.discrepancies.iter().any(|d| d.item.contains(item)), "{item}: {:?}", out.discrepancies);
        }
    }
}
