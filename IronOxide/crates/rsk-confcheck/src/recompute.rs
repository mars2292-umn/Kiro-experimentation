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
        "nrf52840" | "nrf52840_coverage" => &["CLOCK", "TIMER0", "WDT", "PPI"],
        "qemu-mps2-an386" => &["CMSDK_TIMER0", "CMSDK_TIMER1"],
        _ => &[],
    }
}

/// The Kernel peripheral intervals of each Target's layout (R16.2): the
/// Private Peripheral Bus (Armv7-M, 0xE0000000, 1 MiB: the core
/// peripherals PR-22 reserves to the Kernel) first, then the register
/// windows of the Kernel-owned peripherals in the order of
/// `kernel_owned`, as `(name, base, size_log2)`. The checker's own table.
pub fn kernel_windows(target: &str) -> Vec<(&'static str, u64, u8)> {
    let mut out = vec![("PPB", 0xE000_0000, 20)];
    let windows: &[(&str, u64)] = match target {
        "nrf52840" | "nrf52840_coverage" => &[("CLOCK", 0x4000_0000), ("TIMER0", 0x4000_8000), ("WDT", 0x4001_0000), ("PPI", 0x4001_F000)],
        "qemu-mps2-an386" => &[("CMSDK_TIMER0", 0x4000_0000), ("CMSDK_TIMER1", 0x4000_1000)],
        _ => &[],
    };
    out.extend(windows.iter().map(|(n, b)| (*n, *b, 12u8)));
    out
}

/// The programmable PPI channels of each Target (docs/pacemaker PM33.1).
/// The checker's own table.
pub fn ppi_channels(target: &str) -> usize {
    match target {
        "nrf52840" | "nrf52840_coverage" => 20,
        _ => 0,
    }
}

/// docs/pacemaker PM33.4: the channel table recomputed from the Task_Model,
/// per channel the addresses of its event, task and fork registers (0: no
/// fork), with every rule of PM33.1 and PM33.2 checked on the way: the
/// channel number is the position, there are no more channels than the
/// Target has, each register is a word at an event or a task offset of a
/// peripheral, its address is that offset from the base of the window
/// through which a Partition owns the peripheral (the layout's, which this
/// checker recomputes), and one Partition owns every peripheral of the
/// channel. A register that fails a rule is reported and recomputed as 0.
pub fn expected_ppi(m: &Model, out: &mut Outcome) -> Vec<[u32; 3]> {
    let limit = ppi_channels(&m.target);
    out.require("ppi (PM33.1): channels", m.ppi.len() <= limit, format!("at most {limit} on Target `{}`", m.target), m.ppi.len());
    // (owning Partition, window base) of a peripheral.
    let owner = |per: &str| {
        m.partitions.iter().find_map(|p| p.windows.iter().find(|w| w.peripherals.iter().any(|x| x == per)).filter(|_| p.peripherals.iter().any(|x| x == per)).map(|w| (p.id.as_str(), w.region.base)))
    };
    let mut table = Vec::new();
    for (i, c) in m.ppi.iter().enumerate() {
        out.mismatch(format!("ppi {}.channel", c.id), i, c.channel);
        let mut owners: Vec<&str> = Vec::new();
        let mut register = |what: &str, r: &crate::model::PpiRegister, lo: u64, hi: u64| -> u32 {
            let item = format!("ppi {}.{what}", c.id);
            let placed = r.offset % 4 == 0 && r.offset >= lo && r.offset < hi;
            out.require(format!("{item}.offset"), placed, format!("a multiple of 4 from 0x{lo:03x} to 0x{:03x}", hi - 4), format!("0x{:03x}", r.offset));
            match owner(&r.peripheral) {
                Some((partition, base)) => {
                    owners.push(partition);
                    let expected = base + r.offset;
                    out.mismatch(format!("{item}.address"), format!("0x{expected:08x}"), format!("0x{:08x}", r.address));
                    if placed {
                        u32::try_from(expected).unwrap_or(0)
                    } else {
                        0
                    }
                }
                None => {
                    out.require(format!("{item}.peripheral ({})", r.peripheral), false, "a peripheral owned by a Partition (PM33.2)", "owned by none");
                    0
                }
            }
        };
        let event = register("event", &c.event, 0x100, 0x200);
        let task = register("task", &c.task, 0, 0x100);
        let fork = match &c.fork {
            Some(f) => register("fork", f, 0, 0x100),
            None => 0,
        };
        out.require(format!("ppi {} (PM33.2): owner", c.id), owners.windows(2).all(|w| w[0] == w[1]), "one Partition owns every peripheral of the channel", format!("{owners:?}"));
        table.push([event, task, fork]);
    }
    table
}

/// The channel table's image: the tail of the configuration image
/// (`rsk_gen::derive::ppi_bytes`, `rsk_kernel::arch::cm4::ppi`). Empty
/// when there is no channel.
pub fn ppi_image(table: &[[u32; 3]]) -> Vec<u8> {
    let mut out = Vec::new();
    if table.is_empty() {
        return out;
    }
    out.push(table.len() as u8);
    for (i, row) in table.iter().enumerate() {
        out.push(i as u8);
        for word in row {
            out.extend_from_slice(&word.to_le_bytes());
        }
    }
    out
}

/// docs/pacemaker PM33.4: the binary's channels against the recomputed
/// table. `image` is the whole configuration image, whose tail must be the
/// table's image; `symbol` is the content of `RSK_PPI`, the table the
/// Kernel writes from, absent exactly when there is no channel. Returns the
/// length of the tables' part of the image, when the image is long enough.
pub fn check_ppi_binary(table: &[[u32; 3]], image: &[u8], symbol: Option<&[u8]>, out: &mut Outcome) -> Option<usize> {
    let tail = ppi_image(table);
    let words: Vec<u8> = table.iter().flatten().flat_map(|w| w.to_le_bytes()).collect();
    match symbol {
        None => out.require("RSK_PPI", table.is_empty(), "present when the Task_Model declares PPI channels", "absent from the binary"),
        Some(found) => {
            out.require("RSK_PPI", !table.is_empty(), "absent when the Task_Model declares no PPI channel", "present in the binary");
            out.require("RSK_PPI (PM33.4): the table the Kernel writes", found == words.as_slice(), format!("{} channels: {table:08x?}", table.len()), format!("{} bytes: {found:02x?}", found.len()));
        }
    }
    let Some(head) = image.len().checked_sub(tail.len()) else {
        out.require("CONFIG image: PPI channels (PM33.4)", false, format!("{} bytes of channels at its end", tail.len()), format!("an image of {} bytes", image.len()));
        return None;
    };
    if !tail.is_empty() {
        out.require("CONFIG image: PPI channels (PM33.4)", image[head..] == tail[..], format!("{tail:02x?}"), format!("{:02x?}", &image[head..]));
    }
    Some(head)
}

/// The EasyDMA-capable peripherals of each Target (DD-08): their windows
/// are read-only to the owning Partition.
pub fn easy_dma(target: &str) -> &'static [&'static str] {
    match target {
        "nrf52840" | "nrf52840_coverage" => &["RADIO", "UARTE0", "SPIM0", "SPIM1", "SAADC", "PWM0"],
        // The QEMU spike's EasyDMA stand-in (rsk-gen target.rs, task 14).
        "qemu-mps2-an386" => &["CMSDK_GPIO0"],
        _ => &[],
    }
}

use crate::derive_v;

/// DD-02: the NVIC priority byte of a Task level (3 implemented bits).
/// The computation is `derive_v::encode_level` (verified; until 2026-11-13
/// a level above 8 underflowed here before its range was reported).
pub fn encode_level(level: u64) -> u64 {
    derive_v::encode_level(level)
}

/// The region arithmetic is `derive_v`'s (verified, free of overflow for
/// every base and size; until 2026-11-13 a `size_log2` of 64 or more
/// overflowed the shift, and a base near the top of the address space the
/// sum).
fn size_text(r: &Region) -> String {
    match derive_v::size_of(r.size_log2) {
        Some(s) => s.to_string(),
        None => format!("2^{}", r.size_log2),
    }
}

fn disjoint(a: &Region, b: &Region) -> bool {
    derive_v::disjoint(a.base, a.size_log2, b.base, b.size_log2)
}

fn aligned(r: &Region) -> bool {
    derive_v::aligned(r.base, r.size_log2)
}

/// The recomputed values against the Task_Model's derived values.
pub fn check_model(m: &Model, out: &mut Outcome) {
    // docs/pacemaker PM33: the fixed PPI channels (every rule is reported
    // by the recomputation itself).
    let _ = expected_ppi(m, out);
    // Ceilings (R8.7, R23): the maximum Priority of the accessors.
    for res in &m.resources {
        let mut priorities: Vec<u64> = Vec::new();
        for a in &res.accessors {
            match m.task_index(a) {
                Some(t) => priorities.push(m.tasks[t].priority),
                None => out.require(format!("{}.accessors", res.id), false, "a declared Task", format!("`{a}` (no such Task)")),
            }
        }
        let ceiling = derive_v::ceiling_of(&priorities);
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
    let priorities: Vec<u64> = m.tasks.iter().map(|t| t.priority).collect();
    let levels = derive_v::levels_of(&priorities);
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
    let depths: Vec<u64> = m.tasks.iter().map(|t| t.resources.len() as u64).collect();
    match derive_v::sum_checked(&depths) {
        // At least one slot: the Generator's floor (`derive`: the sum, or 1
        // for an Application whose Tasks access no Resource, the Kernel's
        // lock stack being an array of the capacity). Until 2026-11-17 the
        // floor was missing here, and such an Application (the HIL
        // stimulus) was reported with "expected 0, found 1".
        Some(depth_sum) => out.mismatch("capacities.lock_stack", depth_sum.max(1), m.lock_stack),
        None => out.require("capacities.lock_stack", false, "a sum within 64 bits", "overflow"),
    }
    match derive_v::timer_slots(m.tasks.len() as u64) {
        Some(slots) => out.mismatch("capacities.timer_slots", slots, m.timer_slots),
        None => out.require("capacities.timer_slots", false, "a count within 64 bits", "overflow"),
    }
    out.require("capacities.event_log", m.event_log > 0, "> 0", m.event_log);
    // Slots: release 3t, deadline 3t+1, budget 3t+2 (periodic and deferring
    // sporadic Tasks have a release slot; discarding ones have none).
    for (i, t) in m.tasks.iter().enumerate() {
        let i = i as u64;
        let has_release = !t.sporadic || t.mit_policy.as_deref() == Some("defer");
        let (Some(release), Some(deadline), Some(budget)) = (derive_v::slot_index(i, 0), derive_v::slot_index(i, 1), derive_v::slot_index(i, 2)) else {
            out.require(format!("{}.slots", t.id), false, "indices within 64 bits", "overflow");
            continue;
        };
        out.mismatch(format!("{}.slots.release", t.id), if has_release { release } else { image::NO_SLOT as u64 }, t.slot_release);
        out.mismatch(format!("{}.slots.deadline", t.id), deadline, t.slot_deadline);
        out.mismatch(format!("{}.slots.budget", t.id), budget, t.slot_budget);
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
        out.require(format!("{}.regions.stack", p.id), derive_v::stack_fits(p.stack, p.ram.size_log2), format!("1 to {}", size_text(&p.ram)), p.stack);
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
    // The Kernel peripheral intervals (R16.2): the PPB and every Kernel-owned
    // peripheral, each with its register window, and no Partition interval
    // over any of them (what the verified layout keeps and the Kernel checks
    // at boot).
    let expected = kernel_windows(&m.target);
    out.mismatch("layout.kernel_windows (count)", expected.len(), m.kernel_windows.len());
    for (k, (name, base, size_log2)) in expected.iter().enumerate() {
        let Some(w) = m.kernel_windows.get(k) else { continue };
        out.mismatch(format!("layout.kernel_windows[{k}].peripherals"), format!("{name:?}"), format!("{:?}", w.peripherals.join(",")));
        out.mismatch(format!("layout.kernel_windows[{k}].base ({name})"), format!("0x{base:08x}"), format!("0x{:08x}", w.region.base));
        out.mismatch(format!("layout.kernel_windows[{k}].size_log2 ({name})"), *size_log2, w.region.size_log2);
        out.require(format!("layout.kernel_windows[{k}] ({name})"), aligned(&w.region), "an aligned power-of-two region", format!("{:?}", w.region));
        out.mismatch(format!("layout.kernel_windows[{k}].read_only ({name}: a Kernel interval, not a Partition's read-only window)"), false, w.read_only);
        out.require(format!("layout.kernel_windows[{k}] vs shared code"), disjoint(&w.region, &m.shared_code), "disjoint", "overlapping");
        for p in &m.partitions {
            out.require(format!("{}.mpu vs Kernel peripheral interval {name} (R16.2)", p.id), disjoint(&p.code, &w.region) && disjoint(&p.ram, &w.region), "disjoint", "overlapping");
            for win in &p.windows {
                out.require(format!("{}.mpu.windows (0x{:08x}) vs Kernel peripheral interval {name} (R16.2)", p.id, win.region.base), disjoint(&win.region, &w.region), "disjoint", "overlapping");
            }
        }
    }
    for (i, a) in m.partitions.iter().enumerate() {
        for b in &m.partitions[i + 1..] {
            out.require(format!("{}/{} regions", a.id, b.id), disjoint(&a.code, &b.code) && disjoint(&a.ram, &b.ram), "disjoint", "overlapping");
            // R16.7 (2026-11-13): no Partition's RAM, which it writes, lies
            // over another Partition's code. Until this date only code
            // against code and RAM against RAM were compared.
            out.require(format!("{}/{} regions (RAM against code)", a.id, b.id), disjoint(&a.code, &b.ram) && disjoint(&a.ram, &b.code), "disjoint", "overlapping");
        }
    }
}

/// The Link_Checker's stack bounds (`rsk-stack-bounds/1`, R37.4) against
/// the declared stack regions (R26.2): every Partition has a bound, its
/// `available` is the declared size, and the bound fits; the Kernel MSP
/// bound fits as well.
pub fn check_stack_bounds(m: &Model, bounds: &serde_json::Value, out: &mut Outcome) -> Result<(), String> {
    let schema = bounds.get("schema").and_then(|v| v.as_str()).unwrap_or("");
    if schema != "rsk-stack-bounds/1" {
        return Err(format!("stack bounds: unsupported schema `{schema}`"));
    }
    let regions = bounds.get("regions").and_then(|v| v.as_array()).ok_or("stack bounds: no regions")?;
    for p in &m.partitions {
        match regions.iter().find(|r| r.get("name").and_then(|v| v.as_str()) == Some(p.id.as_str()) && r.get("kind").and_then(|v| v.as_str()) == Some("partition")) {
            Some(r) => {
                let bound = r.get("bound").and_then(|v| v.as_u64()).unwrap_or(u64::MAX);
                let available = r.get("available").and_then(|v| v.as_u64()).unwrap_or(0);
                out.mismatch(format!("{}.regions.stack (Link_Checker's view of the declared size)", p.id), p.stack, available);
                out.require(format!("{}.regions.stack (worst case {bound})", p.id), bound <= p.stack, format!(">= {bound}"), p.stack);
            }
            None => out.require(format!("{}.regions.stack", p.id), false, "a Link_Checker bound", "none"),
        }
    }
    match regions.iter().find(|r| r.get("kind").and_then(|v| v.as_str()) == Some("msp")) {
        Some(r) => {
            let ok = r.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
            out.require("kernel MSP (Link_Checker bound within Kernel RAM)", ok, "bound <= available", format!("{:?} <= {:?}", r.get("bound"), r.get("available")));
        }
        None => out.require("kernel MSP", false, "a Link_Checker bound", "none"),
    }
    Ok(())
}

/// The `LAYOUT` image extracted from the binary against the Task_Model's
/// MPU values (R26.2 MPU permissions, R16.7) and its checksums.
/// The `LAYOUT` image the Task_Model calls for (R16.7, R26.4): every
/// interval, every window slot (an unused one is `(0, 5)`, not read-only)
/// and each Partition's stack size, from the model's MPU values.
pub fn expected_layout(m: &Model) -> Result<LayoutImg, String> {
    // The layout is `tables_v::build_layout`'s (verified, 2026-11-26);
    // this function words its refusals.
    use crate::tables_v::LayoutErr;
    let beyond = |what: &str, base: u64| format!("{what}: base 0x{base:x} beyond 32 bits");
    let slots = |what: &str, n: usize, max: usize| format!("{what}: {n} windows, the table has {max} slots");
    crate::tables_v::build_layout(m).map_err(|e| match e {
        LayoutErr::KernelWindowBase => "the Kernel: a window base beyond 32 bits".to_string(),
        LayoutErr::WindowBase(p) => format!("{}: a window base beyond 32 bits", m.partitions[p].id),
        LayoutErr::PartitionBase(p, which) => {
            let part = &m.partitions[p];
            beyond(&part.id, if which == 0 { part.code.base } else { part.ram.base })
        }
        LayoutErr::Stack(p) => format!("{}: stack size beyond 32 bits", m.partitions[p].id),
        LayoutErr::Windows(p) => slots(&m.partitions[p].id, m.partitions[p].windows.len(), image::MAX_WINDOWS),
        LayoutErr::KernelBase(which) => match which {
            0 => beyond("kernel_flash", m.kernel_flash.base),
            1 => beyond("shared_code", m.shared_code.base),
            _ => beyond("kernel_ram", m.kernel_ram.base),
        },
        LayoutErr::KernelWindows => slots("the Kernel", m.kernel_windows.len(), image::MAX_KERNEL_WINDOWS),
    })
}

pub fn check_layout_binary(m: &Model, img: &Image, l: &LayoutImg, out: &mut Outcome) {
    // The verdict (2026-11-21): the image is, byte for byte, the encoding of
    // the layout the Task_Model calls for (`image_v::layout_image_is`,
    // proven). The items after it name the fields for the reader.
    let whole = "LAYOUT image as a whole (R16.7, R26.4)";
    match expected_layout(m) {
        Ok(expected) => out.require(whole, crate::image_v::layout_image_is(&img.bytes, &expected), "the encoding of the layout the Task_Model calls for", "another image (the items that follow name the fields)"),
        Err(why) => out.require(whole, false, "a layout that the Task_Model's values fit", why),
    }
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
    out.mismatch("LAYOUT.kernel_window_count", m.kernel_windows.len(), l.kernel_window_count);
    for (k, slot) in l.kernel_windows.iter().enumerate() {
        let want = m.kernel_windows.get(k).map_or((0u64, 5u8), |win| (win.region.base, win.region.size_log2));
        out.mismatch(format!("LAYOUT.kernel_windows[{k}]"), format!("{:?}", want), format!("{:?}", (slot.0 as u64, slot.1)));
    }
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
            // DD-08 (audit finding 1): the view maps the window read-only exactly when the model says so.
            out.mismatch(item(&format!("windows[{w}].read_only")), part.windows.get(w).is_some_and(|win| win.read_only), slot.2);
        }
    }
}

/// The `CONFIG` tables the Task_Model calls for (R26.4): every field of
/// every entry from the declared values, the Ceilings recomputed from the
/// accessors' Priorities, the slot count from the Task count. `Err` names
/// the first value that has no place in the Kernel's table.
pub fn expected_tables(m: &Model) -> Result<Tables, String> {
    // The tables are `tables_v::build_tables`'s (verified, 2026-11-26:
    // field by field the model's values, no value truncated); this
    // function words its refusals.
    use crate::tables_v::{BuildErr, Field};
    crate::tables_v::build_tables(m).map_err(|e| match e {
        BuildErr::Narrow(field, v) => {
            let what = match field {
                Field::Priority => "priority",
                Field::Budget => "budget",
                Field::SlotRelease => "slot_release",
                Field::SlotDeadline => "slot_deadline",
                Field::SlotBudget => "slot_budget",
                Field::Ceiling => "ceiling",
                Field::MitThreshold => "mit_threshold",
                Field::SlotCount => "slot_count",
            };
            format!("{what} = {v} does not fit the table's field")
        }
        BuildErr::TaskPartition(i) => format!("{}: no Partition `{}`", m.tasks[i].id, m.tasks[i].partition),
        BuildErr::ResourcePartition(i) => format!("{}: no Partition `{}`", m.resources[i].id, m.resources[i].partition),
        BuildErr::Level(p) => format!("Partition `{}`: no level `{}`", m.partitions[p].id, m.partitions[p].level),
        BuildErr::Response(p, which) => {
            let part = &m.partitions[p];
            let name = match which {
                0 => &part.fault,
                1 => &part.overrun,
                _ => &part.deadline_miss,
            };
            format!("Partition `{}`: no response `{name}`", part.id)
        }
    })
}

/// The tables extracted from the binary against the declared values
/// (R26.4), and the checksums (R25.2).
/// `tables_len`: the length of the tables' part of the image; what follows
/// it is the PPI channels' image, checked by `check_ppi_binary` (the whole
/// image when there is no channel). The checksum covers the whole image.
pub fn check_binary(m: &Model, img: &Image, tables_len: usize, t: &Tables, out: &mut Outcome) {
    // The verdict (2026-11-21): the image is, byte for byte, the encoding of
    // the tables the Task_Model calls for (`image_v::image_is`, proven:
    // every byte read in order, none left over, every field of every entry
    // compared). The items after it name the fields for the reader.
    let item = "CONFIG image as a whole (R26.4)";
    match expected_tables(m) {
        Ok(expected) => out.require(item, crate::image_v::image_is(&img.bytes[..tables_len.min(img.bytes.len())], &expected), "the encoding of the tables the Task_Model calls for", "another image (the items that follow name the fields)"),
        Err(why) => out.require(item, false, "tables that the Task_Model's values fit", why),
    }
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

/// R26.4 for the Endpoint table (task 21.2): the Kernel's table against
/// the Task_Model's Endpoints, recomputed independently: each capacity and
/// payload as declared, the role Tasks and their Partitions by the model's
/// indices, the arrival-release flags from the Tasks' sources, and the
/// storage offsets as the running sum of `2 * capacity * payload`.
/// The Endpoint table the Task_Model calls for (task 21.2, R26.4), with
/// the storage size: recomputed as `check_endpoints` describes.
pub fn expected_endpoints(m: &Model) -> Result<(Vec<image::EndpointCfg>, u32), String> {
    let mut out = Vec::new();
    let mut offset: u64 = 0;
    for e in &m.endpoints {
        if !(2..=image::MAX_ROLES).contains(&e.roles.len()) {
            return Err(format!("Endpoint `{}`: {} roles", e.id, e.roles.len()));
        }
        let part_of = |r: usize| e.roles.get(r).and_then(|(task, _)| m.task_index(task)).and_then(|t| m.partition_index(&m.tasks[t].partition));
        let (mut tasks, mut partitions, mut release, mut validates) = (Vec::new(), Vec::new(), Vec::new(), Vec::new());
        for r in 0..image::MAX_ROLES {
            let ti = e.roles.get(r).and_then(|(task, _)| m.task_index(task));
            tasks.push(ti.map_or(255, |t| t as u8));
            partitions.push(part_of(r).map_or(255, |p| p as u8));
            release.push(e.roles.get(r).is_some_and(|(_, release)| *release));
            let mut row = Vec::new();
            for f in 0..image::MAX_ROLES {
                row.push(match (part_of(r), part_of(f)) {
                    (Some(pr), Some(pf)) => e.validating.get(r).copied().unwrap_or(false) && pr != pf && m.partitions[pr].level < m.partitions[pf].level,
                    _ => false,
                });
            }
            validates.push(row);
        }
        out.push(image::EndpointCfg {
            capacity: u8::try_from(e.capacity).map_err(|_| format!("Endpoint `{}`: capacity {}", e.id, e.capacity))?,
            payload: u16::try_from(e.payload_bytes).map_err(|_| format!("Endpoint `{}`: payload {}", e.id, e.payload_bytes))?,
            roles: e.roles.len() as u8,
            tasks,
            partitions,
            release,
            storage: u32::try_from(offset).map_err(|_| format!("Endpoint `{}`: storage offset {offset}", e.id))?,
            validates,
        });
        offset += (e.roles.len() * e.roles.len()) as u64 * e.capacity * e.payload_bytes;
    }
    Ok((out, u32::try_from(offset).map_err(|_| format!("Endpoint storage of {offset} bytes"))?))
}

pub fn check_endpoints(m: &Model, img: &Image, table: &[image::EndpointCfg], storage_bytes: u32, out: &mut Outcome) {
    // The verdict (2026-11-21): the image is, byte for byte, the encoding of
    // the table the Task_Model calls for (`image_v::endpoint_image_is`,
    // proven). The items after it name the fields for the reader.
    let whole = "ENDPOINT image as a whole (R26.4)";
    match expected_endpoints(m) {
        Ok((expected, bytes)) => out.require(whole, crate::image_v::endpoint_image_is(&img.bytes, &expected, bytes), "the encoding of the Endpoint table the Task_Model calls for", "another image (the items that follow name the fields)"),
        Err(why) => out.require(whole, false, "an Endpoint table that the Task_Model's values fit", why),
    }
    let computed = image::crc32(&img.bytes);
    out.mismatch("RSK_ENDPOINT_CHECKSUM (CRC-32 of the extracted image)", format!("0x{computed:08x}"), format!("0x{:08x}", img.stored_checksum));
    out.mismatch("ENDPOINTS.count", m.endpoints.len(), table.len());
    let mut offset: u64 = 0;
    for (i, e) in m.endpoints.iter().enumerate() {
        let item = |f: &str| format!("ENDPOINTS[{i}].{f} ({})", e.id);
        let Some(c) = table.get(i) else { continue };
        out.mismatch(item("capacity"), e.capacity, c.capacity);
        out.mismatch(item("payload"), e.payload_bytes, c.payload);
        out.require(item("payload % 4"), e.payload_bytes % 4 == 0 && e.payload_bytes >= 4, "a multiple of 4, at least 4", e.payload_bytes);
        out.mismatch(item("storage"), offset, c.storage);
        // Task 21.7: two to four roles; the slots beyond them are 0xFF / false.
        out.mismatch(item("roles"), e.roles.len() as u8, c.roles);
        out.require(item("roles in range"), (2..=4).contains(&e.roles.len()), "2 to 4", e.roles.len());
        for r in 0..4 {
            match e.roles.get(r) {
                Some((task, release)) => {
                    let ti = m.task_index(task);
                    out.mismatch(item(&format!("tasks[{r}]")), ti.map_or(255, |t| t as u8), c.tasks[r]);
                    out.mismatch(item(&format!("partitions[{r}]")), ti.and_then(|t| m.partition_index(&m.tasks[t].partition)).map_or(255, |p| p as u8), c.partitions[r]);
                    out.mismatch(item(&format!("release[{r}]")), *release, c.release[r]);
                }
                None => {
                    out.mismatch(item(&format!("tasks[{r}]")), 255u8, c.tasks[r]);
                    out.mismatch(item(&format!("partitions[{r}]")), 255u8, c.partitions[r]);
                    out.mismatch(item(&format!("release[{r}]")), false, c.release[r]);
                }
            }
        }
        // R42.4 (audit finding 2, 2026-11-13): the Kernel accepts a rejection
        // report of role r against role f exactly where r declares a
        // validation function and f is of another Partition of lower
        // Criticality_Level, recomputed from the model's Partitions.
        let part_of = |r: usize| e.roles.get(r).and_then(|(task, _)| m.task_index(task)).and_then(|t| m.partition_index(&m.tasks[t].partition));
        for r in 0..4 {
            for f in 0..4 {
                let want = match (part_of(r), part_of(f)) {
                    (Some(pr), Some(pf)) => e.validating.get(r).copied().unwrap_or(false) && pr != pf && m.partitions[pr].level < m.partitions[pf].level,
                    _ => false,
                };
                out.mismatch(item(&format!("validates[{r}][{f}]")), want, c.validates[r][f]);
            }
        }
        offset += (e.roles.len() * e.roles.len()) as u64 * e.capacity * e.payload_bytes;
    }
    out.mismatch("ENDPOINT_STORAGE_BYTES", offset, storage_bytes);
}

/// R16.1 for the Endpoint storage (2026-11-21): the Kernel's statements
/// about whole messages (`send_message_ok`, `recv_message_ok`) hold under
/// `storage_placed`, a fact of the linked configuration that no Kernel
/// code tests. It is established here, of each binary: the storage static
/// has the size the Endpoint image states, starts at a word, holds every
/// Endpoint's slots at a word-aligned offset, and shares no byte with any
/// Partition's RAM interval of the `LAYOUT` image, none of which reaches
/// the last byte of the address space (`image_v::storage_placed_ok`,
/// proven). The items after the verdict name the parts for the reader.
/// A binary without Endpoints has no operation the fact is needed for.
pub fn check_storage_placement(layout: &LayoutImg, table: &[image::EndpointCfg], storage_bytes: u32, symbol: Option<(u32, u32)>, out: &mut Outcome) {
    if table.is_empty() {
        out.notes.push("binary: no Endpoints, the Endpoint storage's placement is not needed (R16.1)".to_string());
        return;
    }
    let whole = "Endpoint storage placement as a whole (R16.1)";
    let Some((base, size)) = symbol else {
        out.require(whole, false, "the symbol rsk_system::ENDPOINT_STORAGE", "absent from the binary");
        return;
    };
    out.notes.push(format!("binary: rsk_system::ENDPOINT_STORAGE {size} bytes at 0x{base:08x}"));
    out.mismatch("ENDPOINT_STORAGE symbol size against ENDPOINT_STORAGE_BYTES", storage_bytes, size);
    out.require(
        whole,
        crate::image_v::storage_placed_ok(layout, &table.to_vec(), base, storage_bytes),
        "word-aligned, every Endpoint's slots inside it, apart from every Partition's RAM",
        "not so (the items that follow name the part)",
    );
    out.require("ENDPOINT_STORAGE address % 4", base % 4 == 0, 0, base % 4);
    // 2026-11-22: inside the Kernel's RAM interval, which no Partition's
    // MPU view lets a Job write (the premise of the Kernel's statement that
    // the storage keeps its words between entries).
    let (klo, khi) = (u64::from(layout.kernel_ram.base), u64::from(layout.kernel_ram.base) + (1u64 << layout.kernel_ram.size_log2.min(63)));
    out.require(
        "ENDPOINT_STORAGE inside LAYOUT.kernel_ram",
        crate::image_v::storage_in_kernel_ram_ok(&layout.kernel_ram, base, storage_bytes),
        format!("inside [0x{klo:08x}, 0x{khi:08x})"),
        format!("[0x{base:08x}, 0x{:08x})", u64::from(base) + u64::from(storage_bytes)),
    );
    for (i, c) in table.iter().enumerate() {
        let bytes = u64::from(c.roles) * u64::from(c.roles) * u64::from(c.capacity) * u64::from(c.payload);
        out.require(format!("ENDPOINTS[{i}].storage % 4"), c.storage % 4 == 0, 0, c.storage % 4);
        out.require(format!("ENDPOINTS[{i}] slots end inside the storage"), u64::from(c.storage) + bytes <= u64::from(storage_bytes), format!("at most {storage_bytes}"), u64::from(c.storage) + bytes);
    }
    let (lo, hi) = (u64::from(base), u64::from(base) + u64::from(storage_bytes));
    for (p, pl) in layout.partitions.iter().enumerate() {
        let (rlo, rhi) = (u64::from(pl.ram.base), u64::from(pl.ram.base) + (1u64 << pl.ram.size_log2.min(63)));
        out.require(format!("LAYOUT.partitions[{p}].ram ends below 0xffffffff"), rhi <= 0xFFFF_FFFF, "at most 0xffffffff", format!("0x{rhi:x}"));
        out.require(
            format!("LAYOUT.partitions[{p}].ram apart from the Endpoint storage"),
            rhi <= lo || hi <= rlo,
            format!("no byte of [0x{lo:08x}, 0x{hi:08x})"),
            format!("[0x{rlo:08x}, 0x{rhi:08x})"),
        );
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

    /// docs/pacemaker PM33.6: the Task_Model of a declaration whose
    /// Partition owns TIMER3 and whose Sporadic_Task binds it (the fixture
    /// is the Generator's output, kept equal by rsk-gen's test
    /// `pm33_6_timer3_can_be_owned_bound_and_connected`) recomputes without
    /// discrepancy: the window, the interrupt line and the layout.
    #[test]
    fn the_timer3_task_model_recomputes_without_discrepancy() {
        const TIMER3: &str = include_str!("../fixtures/pm33_6_timer3_task_model.json");
        assert!(TIMER3.contains("\"TIMER3\""));
        let m = Model::parse(&serde_json::from_str(TIMER3).unwrap()).unwrap();
        let mut out = Outcome::default();
        check_model(&m, &mut out);
        assert!(out.discrepancies.is_empty(), "{:?}", out.discrepancies);
        assert!(out.checked > 40);
        expected_layout(&m).unwrap();
        expected_tables(&m).unwrap();
        // The window (0x4001_A000) placed on the Kernel's PPI window
        // (0x4001_F000) is a discrepancy (R16.2). The checker has no table
        // of the Partition-assignable peripherals' addresses, so a window
        // moved to another free address is not a discrepancy of this check.
        let moved = TIMER3.replace("\"base\": 1073848320", "\"base\": 1073868800");
        assert!(moved != TIMER3);
        let m = Model::parse(&serde_json::from_str(&moved).unwrap()).unwrap();
        let mut out = Outcome::default();
        check_model(&m, &mut out);
        assert!(!out.discrepancies.is_empty());
    }

    const PPI: &str = include_str!("../fixtures/pm33_4_ppi_task_model.json");

    fn ppi_model(text: &str) -> Model {
        Model::parse(&serde_json::from_str(text).unwrap()).unwrap()
    }

    fn items(out: &Outcome) -> Vec<&str> {
        out.discrepancies.iter().map(|d| d.item.as_str()).collect()
    }

    /// docs/pacemaker PM33.4: the channel table is recomputed from the
    /// Task_Model (the fixture is the Generator's output for two channels,
    /// kept equal by rsk-gen's test `pm33_4_the_kernel_continues_the_same_checksum`),
    /// and a Task_Model altered in one channel is reported with the item.
    #[test]
    fn the_ppi_channels_recompute_and_altered_ones_are_reported() {
        let m = ppi_model(PPI);
        assert_eq!(m.ppi.len(), 2);
        let mut out = Outcome::default();
        check_model(&m, &mut out);
        assert!(out.discrepancies.is_empty(), "{:?}", out.discrepancies);
        let mut out = Outcome::default();
        let table = expected_ppi(&m, &mut out);
        assert_eq!(table, [[0x4000_9140, 0x4000_6030, 0], [0x4000_6100, 0x4000_9040, 0x4000_9008]]);
        assert!(out.discrepancies.is_empty() && out.checked >= 14);

        let altered = |from: &str, to: &str| {
            let text = PPI.replace(from, to);
            assert_ne!(text, PPI, "`{from}` is not in the fixture");
            let mut out = Outcome::default();
            check_model(&ppi_model(&text), &mut out);
            out
        };
        // An address that is not the peripheral's window base plus the offset.
        assert_eq!(items(&altered("\"address\": \"0x40009140\"", "\"address\": \"0x40009144\"")), ["ppi a.event.address"]);
        assert_eq!(items(&altered("\"address\": \"0x40009008\"", "\"address\": \"0x4000a008\"")), ["ppi b.fork.address"]);
        // An offset outside the event registers, and its address with it.
        assert_eq!(items(&altered("\"offset\": 320,\n        \"address\": \"0x40009140\"", "\"offset\": 64,\n        \"address\": \"0x40009040\"")), ["ppi a.event.offset"]);
        // A task in the event range; an offset that is not a word.
        assert_eq!(items(&altered("\"offset\": 48,\n        \"address\": \"0x40006030\"", "\"offset\": 256,\n        \"address\": \"0x40006100\"")), ["ppi a.task.offset"]);
        assert_eq!(items(&altered("\"offset\": 8,\n        \"address\": \"0x40009008\"", "\"offset\": 9,\n        \"address\": \"0x40009009\"")), ["ppi b.fork.offset"]);
        // A channel number that is not the position (a number used twice).
        assert_eq!(items(&altered("\"channel\": 1", "\"channel\": 0")), ["ppi b.channel"]);
        // A peripheral no Partition owns; the Kernel's own.
        assert_eq!(items(&altered("\"peripheral\": \"GPIOTE\",\n        \"offset\": 48", "\"peripheral\": \"TIMER2\",\n        \"offset\": 48")), ["ppi a.task.peripheral (TIMER2)"]);
        assert_eq!(items(&altered("\"peripheral\": \"GPIOTE\",\n        \"offset\": 48", "\"peripheral\": \"PPI\",\n        \"offset\": 48")), ["ppi a.task.peripheral (PPI)"]);
    }

    /// PM33.2, recomputed: a channel whose peripherals two Partitions own.
    /// The fixture's second Partition is given TIMER1 with its window.
    #[test]
    fn a_channel_across_two_partitions_is_reported() {
        let mut m = ppi_model(PPI);
        let p0 = m.partitions[0].clone();
        let i = p0.peripherals.iter().position(|p| p == "TIMER1").unwrap();
        let w = p0.windows.iter().position(|w| w.peripherals.iter().any(|p| p == "TIMER1")).unwrap();
        let moved = m.partitions[0].windows.remove(w);
        m.partitions[0].peripherals.remove(i);
        m.partitions[1].peripherals.push("TIMER1".into());
        m.partitions[1].windows.push(moved);
        let mut out = Outcome::default();
        let table = expected_ppi(&m, &mut out);
        assert_eq!(items(&out), ["ppi a (PM33.2): owner", "ppi b (PM33.2): owner"]);
        // The addresses are the same: only the owners differ.
        assert_eq!(table[0], [0x4000_9140, 0x4000_6030, 0]);
    }

    /// PM33.1: no channel on a Target without a PPI, and no more than it has.
    #[test]
    fn the_channel_count_is_the_targets() {
        let mut m = ppi_model(PPI);
        m.target = "qemu-mps2-an386".into();
        let mut out = Outcome::default();
        expected_ppi(&m, &mut out);
        assert!(items(&out).contains(&"ppi (PM33.1): channels"));
        let mut m = ppi_model(PPI);
        let mut out = Outcome::default();
        while m.ppi.len() < 21 {
            let mut c = m.ppi[0].clone();
            c.channel = m.ppi.len() as u64;
            m.ppi.push(c);
        }
        expected_ppi(&m, &mut out);
        assert_eq!(items(&out), ["ppi (PM33.1): channels"]);
    }

    /// PM33.4, the binary's side, on images built here: the configuration
    /// image ends in the channels' image and `RSK_PPI` is the table. Each
    /// alteration of one channel in either is reported (the configuration
    /// mutation of docs/pacemaker task 3.4; the register's mutations run on
    /// the P1 demo's binary, which has no channel).
    #[test]
    fn the_binarys_channels_are_compared_and_one_altered_channel_is_reported() {
        let table = [[0x4000_9140u32, 0x4000_6030, 0], [0x4000_6100, 0x4000_9040, 0x4000_9008]];
        #[rustfmt::skip]
        let tail = [
            2u8,
            0, 0x40, 0x91, 0x00, 0x40, 0x30, 0x60, 0x00, 0x40, 0, 0, 0, 0,
            1, 0x00, 0x61, 0x00, 0x40, 0x40, 0x90, 0x00, 0x40, 0x08, 0x90, 0x00, 0x40,
        ];
        assert_eq!(ppi_image(&table), tail);
        assert!(ppi_image(&[]).is_empty());
        let tables_part = [0xAAu8; 40];
        let image: Vec<u8> = tables_part.iter().chain(&tail).copied().collect();
        let symbol: Vec<u8> = table.iter().flatten().flat_map(|w| w.to_le_bytes()).collect();
        let run = |image: &[u8], symbol: Option<&[u8]>, table: &[[u32; 3]]| {
            let mut out = Outcome::default();
            let head = check_ppi_binary(table, image, symbol, &mut out);
            (head, out)
        };
        let (head, out) = run(&image, Some(&symbol), &table);
        assert_eq!(head, Some(40));
        assert!(out.discrepancies.is_empty(), "{:?}", out.discrepancies);
        // No channel: the whole image is the tables, and there is no symbol.
        let (head, out) = run(&tables_part, None, &[]);
        assert_eq!(head, Some(40));
        assert!(out.discrepancies.is_empty());

        // One byte of one channel changed in the image; in the table.
        for at in [41, 42, 45, 49, 53, 54, 66] {
            let mut bad = image.clone();
            bad[at] ^= 0x04;
            assert_eq!(items(&run(&bad, Some(&symbol), &table).1), ["CONFIG image: PPI channels (PM33.4)"], "byte {at}");
        }
        for at in 0..symbol.len() {
            let mut bad = symbol.clone();
            bad[at] ^= 0x04;
            assert_eq!(items(&run(&image, Some(&bad), &table).1), ["RSK_PPI (PM33.4): the table the Kernel writes"], "byte {at}");
        }
        // The table missing, short, or present without channels; the image without its tail.
        assert_eq!(items(&run(&image, None, &table).1), ["RSK_PPI"]);
        assert_eq!(items(&run(&image, Some(&symbol[..12]), &table).1), ["RSK_PPI (PM33.4): the table the Kernel writes"]);
        assert_eq!(items(&run(&tables_part, Some(&symbol), &[]).1), ["RSK_PPI", "RSK_PPI (PM33.4): the table the Kernel writes"]);
        assert_eq!(items(&run(&tables_part, Some(&symbol), &table).1), ["CONFIG image: PPI channels (PM33.4)"]);
        let (head, out) = run(&tail[..10], Some(&symbol), &table);
        assert_eq!((head, items(&out)), (None, vec!["CONFIG image: PPI channels (PM33.4)"]));
    }

    /// R27.2: a Task on a core other than 0, a Resource marked global, or
    /// an Endpoint spanning another core is rejected by the Config_Checker
    /// as unusable input, naming the item and citing NG-01.
    #[test]
    fn a_core_other_than_zero_is_rejected_citing_ng01_r27_2() {
        assert!(crate::check(DEMO, None).is_ok());
        let task = DEMO.replacen("\"core\": 0", "\"core\": 1", 1);
        let e = crate::check(&task, None).err().expect("rejected");
        assert!(e.starts_with("NG-01") && e.contains("part0.") && e.contains("core 1"), "{e}");
        // The first Resource's `core` field, after the Tasks' ones.
        let first_resource = DEMO.find("\"resources\"").unwrap();
        let (head, tail) = DEMO.split_at(first_resource);
        let global = format!("{head}{}", tail.replacen("\"core\": 0", "\"core\": \"global\"", 1));
        let e = crate::check(&global, None).err().expect("rejected");
        assert!(e.starts_with("NG-01") && e.contains("global"), "{e}");
    }

    /// R16.1: the Endpoint storage's placement is accepted for the demo's
    /// layout with the storage in the Kernel's RAM, and each way of
    /// breaking it is reported: an address that is not a word's, a storage
    /// inside a Partition's RAM, an Endpoint whose slots run past the
    /// storage, an offset that is not a word's, a Partition RAM interval
    /// that reaches the end of the address space, a size other than the
    /// image's, and no symbol at all.
    #[test]
    fn the_endpoint_storage_placement_is_checked_r16_1() {
        let m = demo();
        let layout = expected_layout(&m).unwrap();
        let (table, bytes) = expected_endpoints(&m).unwrap();
        assert!(!table.is_empty() && bytes > 0);
        let base = layout.kernel_ram.base;
        let run = |layout: &LayoutImg, table: &[image::EndpointCfg], symbol: Option<(u32, u32)>| {
            let mut out = Outcome::default();
            check_storage_placement(layout, table, bytes, symbol, &mut out);
            out.discrepancies.iter().map(|d| d.item.clone()).collect::<Vec<_>>()
        };
        assert!(run(&layout, &table, Some((base, bytes))).is_empty());
        let whole = |items: &[String]| items.iter().any(|i| i.contains("as a whole"));
        let odd = run(&layout, &table, Some((base + 2, bytes)));
        assert!(whole(&odd) && odd.iter().any(|i| i.contains("address % 4")), "{odd:?}");
        let inside = run(&layout, &table, Some((layout.partitions[0].ram.base + 64, bytes)));
        assert!(whole(&inside) && inside.iter().any(|i| i.contains("partitions[0].ram apart")), "{inside:?}");
        let mut long = table.clone();
        let last = long.len() - 1;
        long[last].capacity += 1;
        let past = run(&layout, &long, Some((base, bytes)));
        assert!(whole(&past) && past.iter().any(|i| i.contains("slots end inside")), "{past:?}");
        let mut skew = table.clone();
        skew[0].storage += 2;
        let off = run(&layout, &skew, Some((base, bytes)));
        assert!(whole(&off) && off.iter().any(|i| i.contains("ENDPOINTS[0].storage % 4")), "{off:?}");
        let mut top = layout.clone();
        top.partitions[0].ram = image::IntervalImg { base: 0xFFFF_F000, size_log2: 12, srd: 0 };
        let end = run(&top, &table, Some((base, bytes)));
        assert!(whole(&end) && end.iter().any(|i| i.contains("ends below")), "{end:?}");
        // Below the Kernel's RAM: apart from every Partition's RAM (the
        // check as a whole passes), but not in the interval no view grants.
        let below = base - 0x1000;
        assert!(layout.partitions.iter().all(|p| p.ram.base >= base));
        let outside = run(&layout, &table, Some((below, bytes)));
        assert!(outside.iter().any(|i| i.contains("inside LAYOUT.kernel_ram")) && !whole(&outside), "{outside:?}");
        let kend = base + (1u32 << layout.kernel_ram.size_log2);
        let straddle = run(&layout, &table, Some((kend - 4, bytes)));
        assert!(straddle.iter().any(|i| i.contains("inside LAYOUT.kernel_ram")), "{straddle:?}");
        let size = run(&layout, &table, Some((base, bytes + 4)));
        assert!(size.iter().any(|i| i.contains("symbol size")), "{size:?}");
        assert!(whole(&run(&layout, &table, None)));
        // Without Endpoints nothing is asked of the binary.
        assert!(run(&layout, &[], None).is_empty());
    }

    /// Altered derived values are reported with the item, the expected and
    /// the found value (R26.3): one case per value class.
    #[test]
    fn altered_derived_values_are_reported_per_class() {
        let cases: &[(&str, &str, &str)] = &[
            ("\"ceiling\": 7", "\"ceiling\": 5", "part0.state.ceiling"),
            ("\"nvic\": 32", "\"nvic\": 64", "priority_map"),
            ("\"nesting_depth\": 2,\n      \"slots\": {\n        \"release\": 0", "\"nesting_depth\": 3,\n      \"slots\": {\n        \"release\": 0", "part0.control.nesting_depth"),
            ("\"lock_stack\": 6", "\"lock_stack\": 7", "capacities.lock_stack"),
            ("\"timer_slots\": 23", "\"timer_slots\": 22", "capacities.timer_slots"),
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

    /// The table builder as it was before `tables_v` (2026-11-26), kept as
    /// the reference the verified builder is compared with.
    fn reference_tables(m: &Model) -> Result<Tables, String> {
        fn narrow<T: TryFrom<u64>>(v: u64, what: &str) -> Result<T, String> {
            T::try_from(v).map_err(|_| format!("{what} = {v} does not fit the table's field"))
        }
        let partition_of = |id: &str, what: &str| m.partition_index(id).and_then(|p| u8::try_from(p).ok()).ok_or_else(|| format!("{what}: no Partition `{id}`"));
        let mut tasks = Vec::new();
        let mut access = Vec::new();
        let (mut slot_release, mut slot_deadline, mut slot_budget) = (Vec::new(), Vec::new(), Vec::new());
        for task in &m.tasks {
            tasks.push(image::TaskCfg {
                priority: narrow(task.priority, "priority")?,
                partition: partition_of(&task.partition, &task.id)?,
                kind: if task.sporadic { image::KIND_SPORADIC } else { image::KIND_PERIODIC },
                offset: task.offset_ticks,
                period: task.period_ticks,
                deadline: task.deadline_ticks,
                budget: narrow(task.budget_cycles, "budget")?,
                mit_policy: match task.mit_policy.as_deref() {
                    Some("discard") => image::MIT_DISCARD,
                    _ => image::MIT_DEFER,
                },
                fpu: task.fpu,
            });
            access.push(m.resources.iter().map(|res| res.accessors.contains(&task.id)).collect());
            slot_release.push(narrow(task.slot_release, "slot_release")?);
            slot_deadline.push(narrow(task.slot_deadline, "slot_deadline")?);
            slot_budget.push(narrow(task.slot_budget, "slot_budget")?);
        }
        let mut resources = Vec::new();
        for res in &m.resources {
            let ceiling = res.accessors.iter().filter_map(|a| m.task_index(a)).map(|i| m.tasks[i].priority).max().unwrap_or(0);
            resources.push(image::ResourceCfg { ceiling: narrow(ceiling, "ceiling")?, partition: partition_of(&res.partition, &res.id)? });
        }
        let mut partitions = Vec::new();
        for part in &m.partitions {
            let response = |name: &str| image::response_code(name).ok_or_else(|| format!("Partition `{}`: no response `{name}`", part.id));
            partitions.push(image::PartitionCfg {
                level: image::level_code(&part.level).ok_or_else(|| format!("Partition `{}`: no level `{}`", part.id, part.level))?,
                fault_response: response(&part.fault)?,
                overrun_response: response(&part.overrun)?,
                deadline_response: response(&part.deadline_miss)?,
                mit_threshold: narrow(part.mit_threshold, "mit_threshold")?,
            });
        }
        let slot_count = narrow(3 * m.tasks.len() as u64 + 2, "slot_count")?;
        Ok(Tables { tasks, resources, access, partitions, operating_duration: m.operating_duration_ticks, slot_release, slot_deadline, slot_budget, slot_count })
    }

    /// R26.4: the verified table builder (`tables_v::build_tables`) gives,
    /// for every committed Task_Model and for each of them with one value
    /// changed to one that has no place in its table (or names nothing),
    /// exactly what the builder before it gave: the same tables, or the
    /// same words of refusal.
    #[test]
    fn the_verified_table_builder_agrees_with_the_builder_before_it() {
        let texts = [
            include_str!("../../../apps/p1-demo/gen/task_model.json"),
            include_str!("../../../apps/heart-sim/gen/task_model.json"),
            include_str!("../../../apps/hil-stimulus/gen/task_model.json"),
            include_str!("../../../apps/pacemaker/gen/task_model.json"),
            include_str!("../../../apps/ppi-probe/gen/task_model.json"),
            include_str!("../fixtures/pm33_4_ppi_task_model.json"),
            include_str!("../fixtures/pm33_6_timer3_task_model.json"),
        ];
        let changes: Vec<(&str, fn(&mut Model))> = vec![
            ("nothing", |_| {}),
            ("priority", |m| m.tasks[0].priority = 256),
            ("priority of the last Task", |m| m.tasks.last_mut().unwrap().priority = 9),
            ("budget", |m| m.tasks[0].budget_cycles = 1 << 32),
            ("slot_release", |m| m.tasks[0].slot_release = 256),
            ("slot_deadline", |m| m.tasks.last_mut().unwrap().slot_deadline = 1000),
            ("slot_budget", |m| m.tasks[0].slot_budget = u64::MAX),
            ("a Task's Partition", |m| m.tasks[0].partition = "absent".to_string()),
            ("kind", |m| m.tasks[0].sporadic = !m.tasks[0].sporadic),
            ("policy discard", |m| m.tasks[0].mit_policy = Some("discard".to_string())),
            ("policy defer", |m| m.tasks[0].mit_policy = Some("defer".to_string())),
            ("policy other", |m| m.tasks[0].mit_policy = Some("discards".to_string())),
            ("fpu", |m| m.tasks[0].fpu = !m.tasks[0].fpu),
            ("a second Task of one id", |m| {
                let t = m.tasks[0].clone();
                m.tasks.push(t);
            }),
            ("a Task id no Resource lists", |m| m.tasks[0].id = "renamed".to_string()),
            ("ninety Tasks", |m| {
                while m.tasks.len() < 90 {
                    let t = m.tasks[0].clone();
                    m.tasks.push(t);
                }
            }),
            ("a Resource's Partition", |m| {
                if let Some(r) = m.resources.first_mut() {
                    r.partition = "absent".to_string();
                }
            }),
            ("an accessor that names no Task", |m| {
                if let Some(r) = m.resources.first_mut() {
                    r.accessors.push("nobody".to_string());
                }
            }),
            ("no accessor", |m| {
                if let Some(r) = m.resources.first_mut() {
                    r.accessors.clear();
                }
            }),
            ("every Task an accessor", |m| {
                let ids: Vec<String> = m.tasks.iter().map(|t| t.id.clone()).collect();
                if let Some(r) = m.resources.last_mut() {
                    r.accessors = ids;
                }
            }),
            ("a Ceiling beyond the field", |m| {
                if let Some(a) = m.resources.first().and_then(|r| r.accessors.first().cloned()) {
                    let t = m.task_index(&a).unwrap();
                    m.tasks[t].priority = 70_000;
                    m.tasks[0].priority = m.tasks[0].priority.min(255);
                }
            }),
            ("level", |m| m.partitions[0].level = "F".to_string()),
            ("level E", |m| m.partitions[0].level = "E".to_string()),
            ("fault", |m| m.partitions[0].fault = "END".to_string()),
            ("fault SAFE_STATE", |m| m.partitions[0].fault = "SAFE_STATE".to_string()),
            ("overrun", |m| m.partitions[0].overrun = "".to_string()),
            ("overrun STOP_PARTITION", |m| m.partitions[0].overrun = "STOP_PARTITION".to_string()),
            ("deadline_miss", |m| m.partitions.last_mut().unwrap().deadline_miss = "end_job".to_string()),
            ("deadline_miss END_JOB", |m| m.partitions[0].deadline_miss = "END_JOB".to_string()),
            ("mit_threshold", |m| m.partitions[0].mit_threshold = 1 << 32),
            ("a second Partition of one id", |m| {
                let p = m.partitions[0].clone();
                m.partitions.insert(0, p);
            }),
            ("operating duration", |m| m.operating_duration_ticks = u64::MAX),
        ];
        let mut refusals = 0;
        for text in texts {
            let model = Model::parse(&serde_json::from_str(text).unwrap()).unwrap();
            for (what, change) in &changes {
                let mut m = model.clone();
                change(&mut m);
                let (new, old) = (expected_tables(&m), reference_tables(&m));
                assert_eq!(new, old, "{what}");
                refusals += usize::from(new.is_err());
            }
        }
        assert!(refusals >= 7 * 10, "the changes refuse too few models to compare the refusals: {refusals}");
    }

    /// The layout builder as it was before `tables_v` (2026-11-26).
    fn reference_layout(m: &Model) -> Result<LayoutImg, String> {
        let interval = |r: &Region, what: &str| -> Result<image::IntervalImg, String> {
            Ok(image::IntervalImg { base: u32::try_from(r.base).map_err(|_| format!("{what}: base 0x{:x} beyond 32 bits", r.base))?, size_log2: r.size_log2, srd: r.srd })
        };
        let count = |n: usize, max: usize, what: &str| -> Result<u8, String> {
            if n > max { Err(format!("{what}: {n} windows, the table has {max} slots")) } else { Ok(n as u8) }
        };
        let base = |w: &crate::model::Window, what: &str| u32::try_from(w.region.base).map_err(|_| format!("{what}: a window base beyond 32 bits"));
        let mut kernel_windows = Vec::new();
        for k in 0..image::MAX_KERNEL_WINDOWS {
            kernel_windows.push(match m.kernel_windows.get(k) {
                Some(w) => (base(w, "the Kernel")?, w.region.size_log2),
                None => (0, 5),
            });
        }
        let mut partitions = Vec::new();
        for part in &m.partitions {
            let mut windows = Vec::new();
            for w in 0..image::MAX_WINDOWS {
                windows.push(match part.windows.get(w) {
                    Some(win) => (base(win, &part.id)?, win.region.size_log2, win.read_only),
                    None => (0, 5, false),
                });
            }
            partitions.push(image::PartitionLayoutImg {
                code: interval(&part.code, &part.id)?,
                ram: interval(&part.ram, &part.id)?,
                stack_size: u32::try_from(part.stack).map_err(|_| format!("{}: stack size beyond 32 bits", part.id))?,
                window_count: count(part.windows.len(), image::MAX_WINDOWS, &part.id)?,
                windows,
            });
        }
        Ok(LayoutImg {
            kernel_flash: interval(&m.kernel_flash, "kernel_flash")?,
            shared_code: interval(&m.shared_code, "shared_code")?,
            kernel_ram: interval(&m.kernel_ram, "kernel_ram")?,
            kernel_window_count: count(m.kernel_windows.len(), image::MAX_KERNEL_WINDOWS, "the Kernel")?,
            kernel_windows,
            partitions,
        })
    }

    /// R16.7, R26.4: the verified layout builder (`tables_v::build_layout`)
    /// gives, for every committed Task_Model and for each of them with one
    /// value changed, exactly what the builder before it gave: the same
    /// layout, or the same words of refusal.
    #[test]
    fn the_verified_layout_builder_agrees_with_the_builder_before_it() {
        let texts = [
            include_str!("../../../apps/p1-demo/gen/task_model.json"),
            include_str!("../../../apps/heart-sim/gen/task_model.json"),
            include_str!("../../../apps/hil-stimulus/gen/task_model.json"),
            include_str!("../../../apps/pacemaker/gen/task_model.json"),
            include_str!("../../../apps/ppi-probe/gen/task_model.json"),
            include_str!("../fixtures/pm33_4_ppi_task_model.json"),
            include_str!("../fixtures/pm33_6_timer3_task_model.json"),
        ];
        let window = |base: u64| crate::model::Window { region: Region { base, size_log2: 12, srd: 0 }, peripherals: Vec::new(), read_only: true };
        let changes: Vec<(&str, Box<dyn Fn(&mut Model)>)> = vec![
            ("nothing", Box::new(|_| {})),
            ("kernel_flash base", Box::new(|m| m.kernel_flash.base = 1 << 32)),
            ("shared_code base", Box::new(|m| m.shared_code.base = u64::MAX)),
            ("kernel_ram base", Box::new(|m| m.kernel_ram.base = 1 << 40)),
            ("kernel_ram size and sub-regions", Box::new(|m| {
                m.kernel_ram.size_log2 = 31;
                m.kernel_ram.srd = 0xa5;
            })),
            ("a Kernel window's base", Box::new(|m| {
                if let Some(w) = m.kernel_windows.first_mut() {
                    w.region.base = 1 << 32;
                }
            })),
            ("no Kernel window", Box::new(|m| m.kernel_windows.clear())),
            ("a Kernel window more", Box::new(move |m| m.kernel_windows.push(window(0x4000_0000)))),
            ("nine Kernel windows", Box::new(move |m| {
                while m.kernel_windows.len() < 9 {
                    m.kernel_windows.push(window(0x4000_1000));
                }
            })),
            ("a ninth Kernel window beyond 32 bits", Box::new(move |m| {
                while m.kernel_windows.len() < 8 {
                    m.kernel_windows.push(window(0x4000_1000));
                }
                m.kernel_windows.push(window(1 << 33));
            })),
            ("code base", Box::new(|m| m.partitions[0].code.base = 1 << 32)),
            ("ram base", Box::new(|m| m.partitions.last_mut().unwrap().ram.base = 1 << 32)),
            ("stack", Box::new(|m| m.partitions[0].stack = 1 << 32)),
            ("a window's base", Box::new(move |m| m.partitions[0].windows.insert(0, window(1 << 32)))),
            ("a window more", Box::new(move |m| m.partitions[0].windows.push(window(0x4000_2000)))),
            ("a window not read-only", Box::new(move |m| {
                let mut w = window(0x4000_3000);
                w.read_only = false;
                m.partitions[0].windows.insert(0, w);
            })),
            ("six windows", Box::new(move |m| {
                while m.partitions[0].windows.len() < 6 {
                    m.partitions[0].windows.push(window(0x4000_4000));
                }
            })),
            ("a sixth window beyond 32 bits", Box::new(move |m| {
                while m.partitions[0].windows.len() < 5 {
                    m.partitions[0].windows.push(window(0x4000_4000));
                }
                m.partitions[0].windows.push(window(1 << 34));
            })),
            ("no Partition", Box::new(|m| m.partitions.clear())),
        ];
        let mut refusals = 0;
        for text in texts {
            let model = Model::parse(&serde_json::from_str(text).unwrap()).unwrap();
            for (what, change) in &changes {
                let mut m = model.clone();
                change(&mut m);
                let (new, old) = (expected_layout(&m), reference_layout(&m));
                assert_eq!(new, old, "{what}");
                refusals += usize::from(new.is_err());
            }
        }
        assert!(refusals >= 7 * 9, "the changes refuse too few models to compare the refusals: {refusals}");
    }
}
