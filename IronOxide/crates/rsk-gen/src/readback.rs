//! The emitted tables read back (R24.1; 2026-11-16): after the system
//! source is written, the numbers of its `CONFIG` table are read out of the
//! text again, by a reader that shares nothing with the emitter but the
//! field names, and compared with the values the derivation produced. A
//! difference fails the generation.
//!
//! The same is done for the `LAYOUT` table against the derived memory plan
//! (`check_layout`, 2026-11-17), and since 2026-11-21 for the EasyDMA
//! channel table (`check_dma`), the Endpoint table (`check_endpoints`) and
//! the memory map and channel constants each Partition's source carries
//! (`check_partition_map`, the basis of the HIL suite's access matrix).
//! Not read back: the Release_Signal table and the per-Task vector arrays.
//!
//! This closes the one unverified step between the derivation (whose
//! arithmetic is proven, `derive_v`) and the source the compiler sees: the
//! printing. What the compiler then makes of the source is checked from the
//! other end, by the Config_Checker on the linked binary (R26.4), and the
//! tables' well-formedness by the Kernel's own validation at build time
//! (the assertion `emit` writes after the table).

use crate::derive::{Generated, Kind};

/// The text of the `CONFIG` table: from its declaration to its closing line.
fn table(text: &str) -> Result<&str, String> {
    let start = text.find("pub const CONFIG:").ok_or("no `CONFIG` table in the emitted source")?;
    let rest = &text[start..];
    let end = rest.find("\n};").ok_or("the `CONFIG` table is not closed")?;
    Ok(&rest[..end])
}

/// The number that follows `key: ` in `line`.
fn number(line: &str, key: &str) -> Result<u64, String> {
    let needle = format!("{key}: ");
    let at = line.find(&needle).ok_or_else(|| format!("no field `{key}` in `{}`", line.trim()))?;
    let digits: String = line[at + needle.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().map_err(|_| format!("field `{key}` of `{}` is not a number", line.trim()))
}

/// The numbers of the array that follows `key: [` in the table.
fn array(table: &str, key: &str) -> Result<Vec<u64>, String> {
    let needle = format!("{key}: [");
    let at = table.find(&needle).ok_or_else(|| format!("no array `{key}` in the table"))?;
    let rest = &table[at + needle.len()..];
    let end = rest.find(']').ok_or_else(|| format!("array `{key}` is not closed"))?;
    rest[..end]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        // "No slot" is printed by its name.
        .map(|s| if s.ends_with("NO_SLOT") { Ok(crate::derive::NO_SLOT as u64) } else { s.parse().map_err(|_| format!("array `{key}` holds `{s}`, not a number")) })
        .collect()
}

fn same(what: &str, read: u64, derived: u64) -> Result<(), String> {
    if read == derived {
        Ok(())
    } else {
        Err(format!("{what}: the emitted source says {read}, the derivation {derived}"))
    }
}

/// Compares the emitted `CONFIG` table with the derived values.
pub fn check(text: &str, g: &Generated) -> Result<(), String> {
    let table = table(text)?;
    let tasks: Vec<&str> = table.lines().filter(|l| l.contains("TaskCfg {")).collect();
    if tasks.len() != g.tasks.len() {
        return Err(format!("the emitted source has {} Tasks, the derivation {}", tasks.len(), g.tasks.len()));
    }
    for (line, t) in tasks.iter().zip(&g.tasks) {
        same(&format!("{}: priority", t.id), number(line, "priority")?, t.priority as u64)?;
        same(&format!("{}: partition", t.id), number(line, "partition")?, t.partition as u64)?;
        same(&format!("{}: offset", t.id), number(line, "offset")?, t.offset_ticks)?;
        same(&format!("{}: period", t.id), number(line, "period")?, t.period_ticks)?;
        same(&format!("{}: deadline", t.id), number(line, "deadline")?, t.deadline_ticks)?;
        same(&format!("{}: budget", t.id), number(line, "budget")?, t.budget_cycles as u64)?;
        let periodic = line.contains("KIND_PERIODIC");
        if periodic != (t.kind == Kind::Periodic) || periodic == line.contains("KIND_SPORADIC") {
            return Err(format!("{}: the emitted source and the derivation disagree on the Task's kind", t.id));
        }
    }
    let resources: Vec<&str> = table.lines().filter(|l| l.contains("ResourceCfg {")).collect();
    if resources.len() != g.resources.len() {
        return Err(format!("the emitted source has {} Resources, the derivation {}", resources.len(), g.resources.len()));
    }
    for (line, r) in resources.iter().zip(&g.resources) {
        same(&format!("{}: ceiling", r.id), number(line, "ceiling")?, r.ceiling as u64)?;
        same(&format!("{}: partition", r.id), number(line, "partition")?, r.partition as u64)?;
    }
    let duration = table.lines().find(|l| l.trim_start().starts_with("operating_duration:")).ok_or("no `operating_duration` in the table")?;
    same("operating_duration", number(duration, "operating_duration")?, g.operating_duration_ticks)?;
    for (key, of) in [
        ("slot_release", (|t: &crate::derive::GenTask| t.slot_release) as fn(&crate::derive::GenTask) -> u8),
        ("slot_deadline", |t| t.slot_deadline),
        ("slot_budget", |t| t.slot_budget),
    ] {
        let read = array(table, key)?;
        if read.len() != g.tasks.len() {
            return Err(format!("`{key}` has {} entries for {} Tasks", read.len(), g.tasks.len()));
        }
        for (v, t) in read.iter().zip(&g.tasks) {
            same(&format!("{}: {key}", t.id), *v, of(t) as u64)?;
        }
    }
    Ok(())
}

/// The text of the `LAYOUT` table: from its declaration to its closing line.
fn layout_table(text: &str) -> Result<&str, String> {
    let start = text.find("pub const LAYOUT:").ok_or("no `LAYOUT` table in the emitted source")?;
    let rest = &text[start..];
    let end = rest.find("\n};").ok_or("the `LAYOUT` table is not closed")?;
    Ok(&rest[..end])
}

/// The hexadecimal number that follows `key: 0x` in `text`.
fn hex(text: &str, key: &str) -> Result<u64, String> {
    let needle = format!("{key}: 0x");
    let at = text.find(&needle).ok_or_else(|| format!("no hexadecimal field `{key}` in `{}`", text.trim()))?;
    let digits: String = text[at + needle.len()..].chars().take_while(|c| c.is_ascii_hexdigit()).collect();
    u64::from_str_radix(&digits, 16).map_err(|_| format!("field `{key}` of `{}` is not a hexadecimal number", text.trim()))
}

/// `(base, size_log2)` of the `Interval { .. }` that follows `key: ` in `line`.
fn interval_after(line: &str, key: &str) -> Result<(u64, u64), String> {
    let needle = format!("{key}: Interval {{");
    let at = line.find(&needle).ok_or_else(|| format!("no interval `{key}` in `{}`", line.trim()))?;
    let rest = &line[at + needle.len()..];
    let end = rest.find('}').ok_or_else(|| format!("interval `{key}` is not closed"))?;
    Ok((hex(&rest[..end], "base")?, number(&rest[..end], "size_log2")?))
}

/// The entries of the array that follows `key: [` in `line`, as written.
fn entries<'a>(line: &'a str, key: &str) -> Result<Vec<&'a str>, String> {
    let needle = format!(" {key}: [");
    let at = line.find(&needle).ok_or_else(|| format!("no array `{key}` in `{}`", line.trim()))?;
    let rest = &line[at + needle.len()..];
    let end = rest.find(']').ok_or_else(|| format!("array `{key}` is not closed"))?;
    // An interval holds one comma of its own: split on the closing brace.
    let body = &rest[..end];
    let mut out = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    for (i, c) in body.char_indices() {
        match c {
            '{' => depth += 1,
            '}' => depth = depth.saturating_sub(1),
            ',' if depth == 0 => {
                out.push(body[start..i].trim());
                start = i + 1;
            }
            _ => {}
        }
    }
    if !body[start..].trim().is_empty() {
        out.push(body[start..].trim());
    }
    Ok(out)
}

/// The windows of an array: `count` intervals first, then only `NO_WINDOW`.
fn windows(line: &str, key: &str, what: &str, derived: &[(u64, u64)], capacity: usize) -> Result<(), String> {
    let read = entries(line, key)?;
    if read.len() != capacity {
        return Err(format!("{what}: `{key}` has {} entries, the Kernel's table {capacity}", read.len()));
    }
    for (i, e) in read.iter().enumerate() {
        match derived.get(i) {
            Some(&(base, size_log2)) => {
                same(&format!("{what}: window {i} base"), hex(e, "base")?, base)?;
                same(&format!("{what}: window {i} size_log2"), number(e, "size_log2")?, size_log2)?;
            }
            None if *e == "NO_WINDOW" => {}
            None => return Err(format!("{what}: `{key}` entry {i} is `{e}`, the derivation has no window there")),
        }
    }
    Ok(())
}

/// Compares the emitted `LAYOUT` table with the derived memory plan
/// (2026-11-17): every interval, subregion mask, stack size, window and
/// window count. `Layout::validate` at build time accepts any well-formed
/// layout; this is what ties the printed one to the one derived.
pub fn check_layout(text: &str, g: &Generated) -> Result<(), String> {
    let table = layout_table(text)?;
    let plan = &g.plan;
    for (key, r) in [("kernel_flash", &plan.kernel_flash), ("shared_code", &plan.shared_code), ("kernel_ram", &plan.kernel_ram)] {
        let line = table.lines().find(|l| l.trim_start().starts_with(&format!("{key}:"))).ok_or_else(|| format!("no `{key}` in the layout"))?;
        let (base, size_log2) = interval_after(line, key)?;
        same(&format!("{key}: base"), base, r.base as u64)?;
        same(&format!("{key}: size_log2"), size_log2, r.size_log2 as u64)?;
    }
    let line = table.lines().find(|l| l.trim_start().starts_with("kernel_windows:")).ok_or("no `kernel_windows` in the layout")?;
    let derived: Vec<(u64, u64)> = plan.kernel_windows.iter().map(|w| (w.base as u64, w.size_log2 as u64)).collect();
    windows(line, "kernel_windows", "Kernel", &derived, crate::layout::MAX_KERNEL_WINDOWS)?;
    let line = table.lines().find(|l| l.trim_start().starts_with("kernel_window_count:")).ok_or("no `kernel_window_count` in the layout")?;
    same("kernel_window_count", number(line, "kernel_window_count")?, plan.kernel_windows.len() as u64)?;
    let parts: Vec<&str> = table.lines().filter(|l| l.contains("PartitionLayout {")).collect();
    if parts.len() != plan.partitions.len() {
        return Err(format!("the emitted layout has {} Partitions, the derivation {}", parts.len(), plan.partitions.len()));
    }
    for (line, p) in parts.iter().zip(&plan.partitions) {
        let n = &p.name;
        for (key, r) in [("code", &p.code), ("ram", &p.ram)] {
            let (base, size_log2) = interval_after(line, key)?;
            same(&format!("{n}: {key} base"), base, r.base as u64)?;
            same(&format!("{n}: {key} size_log2"), size_log2, r.size_log2 as u64)?;
            same(&format!("{n}: {key}_srd"), hex(line, &format!("{key}_srd"))?, r.srd as u64)?;
        }
        same(&format!("{n}: stack_size"), number(line, "stack_size")?, p.stack as u64)?;
        let derived: Vec<(u64, u64)> = p.windows.iter().map(|w| (w.base as u64, w.size_log2 as u64)).collect();
        windows(line, "windows", n, &derived, crate::layout::MAX_WINDOWS)?;
        same(&format!("{n}: window_count"), number(line, "window_count")?, p.windows.len() as u64)?;
        let ro = entries(line, "windows_ro")?;
        if ro.len() != crate::layout::MAX_WINDOWS {
            return Err(format!("{n}: `windows_ro` has {} entries, the Kernel's table {}", ro.len(), crate::layout::MAX_WINDOWS));
        }
        for (i, e) in ro.iter().enumerate() {
            let derived = p.windows.get(i).is_some_and(|w| w.read_only);
            if *e != if derived { "true" } else { "false" } {
                return Err(format!("{n}: window {i} read-only: the emitted source says {e}, the derivation {derived}"));
            }
        }
    }
    Ok(())
}

/// The lines of the array table that starts with `declaration` (up to its
/// closing `];`) that hold an entry of type `entry`.
fn rows<'a>(text: &'a str, declaration: &str, entry: &str) -> Result<Vec<&'a str>, String> {
    let start = text.find(declaration).ok_or_else(|| format!("no `{declaration}` table in the emitted source"))?;
    let rest = &text[start..];
    let end = rest.find("\n];").ok_or_else(|| format!("the `{declaration}` table is not closed"))?;
    let needle = format!("{entry} {{");
    Ok(rest[..end].lines().skip(1).filter(|l| l.contains(&needle)).collect())
}

/// The number that a `pub const NAME: usize = ` line gives.
fn constant(text: &str, name: &str) -> Result<u64, String> {
    let needle = format!("pub const {name}: usize = ");
    let at = text.find(&needle).ok_or_else(|| format!("no constant `{name}` in the emitted source"))?;
    let digits: String = text[at + needle.len()..].chars().take_while(|c| c.is_ascii_digit()).collect();
    digits.parse().map_err(|_| format!("constant `{name}` is not a number"))
}

/// A number written in decimal or as `0x..`.
fn value(s: &str) -> Result<u64, String> {
    let s = s.trim();
    match s.strip_prefix("0x") {
        Some(h) => u64::from_str_radix(h, 16).map_err(|_| format!("`{s}` is not a hexadecimal number")),
        None => s.parse().map_err(|_| format!("`{s}` is not a number")),
    }
}

/// The numbers of the array that follows `key: [` in `line`.
fn values(line: &str, key: &str) -> Result<Vec<u64>, String> {
    entries(line, key)?.into_iter().map(value).collect()
}

/// The EasyDMA channels the derivation implies, in the table's order
/// (Partition, then window, then peripheral, then the peripheral's
/// channels): (Partition, pointer register, count register, stop
/// registers, list registers, shortcut register), absolute addresses, 0
/// where the peripheral class has no such register. Written here from the
/// Target description and the memory plan, apart from the emitter.
fn derived_dma(g: &Generated) -> Vec<(u64, u64, u64, [u64; 2], [u64; 2], u64)> {
    let mut out = Vec::new();
    for (partition, regions) in g.plan.partitions.iter().enumerate() {
        for window in &regions.windows {
            for name in &window.peripherals {
                let Some(peripheral) = g.target.peripheral(name) else { continue };
                let Some(registers) = peripheral.dma else { continue };
                let base = peripheral.window_base as u64;
                let at = |offset: Option<&u32>| offset.map_or(0, |o| base + *o as u64);
                for &(pointer, count) in registers.channels {
                    out.push((
                        partition as u64,
                        base + pointer as u64,
                        if count == 0 { 0 } else { base + count as u64 },
                        [at(registers.stops.first()), at(registers.stops.get(1))],
                        [at(registers.lists.first()), at(registers.lists.get(1))],
                        if registers.shorts == 0 { 0 } else { base + registers.shorts as u64 },
                    ));
                }
            }
        }
    }
    out
}

/// Compares the emitted `DMA` table with the EasyDMA channels the Target
/// description and the memory plan imply (2026-11-21, DD-08, R17): the
/// owning Partition and every register address of every channel, and the
/// channel count. A channel printed for another Partition, or a pointer
/// register printed at another address, would otherwise reach the Kernel's
/// request check unseen by the Generator.
pub fn check_dma(text: &str, g: &Generated) -> Result<(), String> {
    let derived = derived_dma(g);
    same("ND (EasyDMA channels)", constant(text, "ND")?, derived.len() as u64)?;
    let read = rows(text, "pub const DMA:", "DmaCfg")?;
    if read.len() != derived.len() {
        return Err(format!("the emitted DMA table has {} channels, the derivation {}", read.len(), derived.len()));
    }
    for (i, (line, d)) in read.iter().zip(&derived).enumerate() {
        let what = format!("DMA channel {i}");
        same(&format!("{what}: partition"), number(line, "partition")?, d.0)?;
        same(&format!("{what}: ptr"), hex(line, "ptr")?, d.1)?;
        same(&format!("{what}: maxcnt"), hex(line, "maxcnt")?, d.2)?;
        for (key, expected) in [("stops", d.3), ("lists", d.4)] {
            let got = values(line, key)?;
            if got != expected {
                return Err(format!("{what}: {key}: the emitted source says {got:x?}, the derivation {expected:x?}"));
            }
        }
        same(&format!("{what}: shorts"), hex(line, "shorts")?, d.5)?;
    }
    Ok(())
}

/// Compares the emitted `ENDPOINTS` table with the derived Endpoints
/// (2026-11-21, R40.1, R42): capacity, payload size, role count, each
/// role's Task and that Task's Partition, the arrival-release flags, the
/// storage offset, the validation edges, the Endpoint count and the bytes
/// of the Kernel's storage.
pub fn check_endpoints(text: &str, g: &Generated) -> Result<(), String> {
    let roles_max = crate::validate::MAX_ROLES;
    same("NEP (Endpoints)", constant(text, "NEP")?, g.endpoints.len() as u64)?;
    same("ENDPOINT_STORAGE_BYTES", constant(text, "ENDPOINT_STORAGE_BYTES")?, g.endpoint_storage_bytes as u64)?;
    let read = rows(text, "pub const ENDPOINTS:", "EndpointCfg")?;
    if read.len() != g.endpoints.len() {
        return Err(format!("the emitted Endpoint table has {} Endpoints, the derivation {}", read.len(), g.endpoints.len()));
    }
    for (line, e) in read.iter().zip(&g.endpoints) {
        let n = &e.name;
        same(&format!("{n}: capacity"), number(line, "capacity")?, e.capacity as u64)?;
        same(&format!("{n}: payload"), number(line, "payload")?, e.payload as u64)?;
        same(&format!("{n}: roles"), number(line, "roles")?, e.roles.len() as u64)?;
        same(&format!("{n}: storage"), number(line, "storage")?, e.storage as u64)?;
        let tasks = values(line, "tasks")?;
        let partitions = values(line, "partitions")?;
        let release = entries(line, "release")?;
        if tasks.len() != roles_max || partitions.len() != roles_max || release.len() != roles_max {
            return Err(format!("{n}: a role array does not have the Kernel's {roles_max} slots"));
        }
        for r in 0..roles_max {
            let (task, partition, released) = match e.roles.get(r) {
                Some((_, t)) => (*t as u64, g.tasks[*t].partition as u64, e.release.get(r).copied().unwrap_or(false)),
                None => (0xFF, 0xFF, false),
            };
            same(&format!("{n}: role {r} Task"), tasks[r], task)?;
            same(&format!("{n}: role {r} Partition"), partitions[r], partition)?;
            if release[r] != if released { "true" } else { "false" } {
                return Err(format!("{n}: role {r} arrival release: the emitted source says {}, the derivation {released}", release[r]));
            }
        }
        // The validation edges: a square of flags after `validates: `.
        let at = line.find(" validates: [").ok_or_else(|| format!("{n}: no `validates` in `{}`", line.trim()))?;
        let rest = &line[at..];
        let end = rest.find("] }").ok_or_else(|| format!("{n}: `validates` is not closed"))?;
        let flags: Vec<bool> = rest[..end].split(|c: char| !c.is_ascii_alphabetic()).filter_map(|w| match w { "true" => Some(true), "false" => Some(false), _ => None }).collect();
        if flags.len() != roles_max * roles_max {
            return Err(format!("{n}: `validates` has {} flags, the Kernel's table {}", flags.len(), roles_max * roles_max));
        }
        for r in 0..roles_max {
            for f in 0..roles_max {
                let derived = e.validates.get(r).and_then(|v| v.get(f)).copied().unwrap_or(false);
                if flags[r * roles_max + f] != derived {
                    return Err(format!("{n}: validation of role {r} from role {f}: the emitted source says {}, the derivation {derived}", flags[r * roles_max + f]));
                }
            }
        }
    }
    Ok(())
}

/// Every `(a, b)` pair of numbers written in `text`, in order.
fn pairs(text: &str) -> Result<Vec<(u64, u64)>, String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(open) = rest.find('(') {
        let close = rest[open..].find(')').ok_or("a pair is not closed")? + open;
        let (a, b) = rest[open + 1..close].split_once(',').ok_or_else(|| format!("`{}` is not a pair", &rest[open..=close]))?;
        out.push((value(a)?, value(b)?));
        rest = &rest[close + 1..];
    }
    Ok(out)
}

/// The text after `= ` of the constant `name` of the Partition source's
/// memory map, up to the `;` that ends it.
fn map_constant<'a>(map: &'a str, name: &str) -> Result<&'a str, String> {
    let needle = format!("pub const {name}: ");
    let at = map.find(&needle).ok_or_else(|| format!("no `{name}` in the Partition's memory map"))?;
    let rest = &map[at..];
    let eq = rest.find("= ").ok_or_else(|| format!("`{name}` has no value"))?;
    let end = rest.find(';').ok_or_else(|| format!("`{name}` is not closed"))?;
    // The type of an array of pairs holds a `;` of its own: the value's end
    // is the first `;` after the `=`.
    let end = if end < eq { eq + rest[eq..].find(';').ok_or_else(|| format!("`{name}` is not closed"))? } else { end };
    Ok(&rest[eq + 2..end])
}

/// Compares the memory map a Partition's generated source carries (the
/// `rsk_target::layout` module, from which the HIL isolation suite builds
/// its access matrix, R16.8) and the Partition's EasyDMA channel constants
/// with the derived memory plan (2026-11-21). It is a second printing of
/// the plan beside the Kernel's `LAYOUT` table: a difference would make the
/// suite test another layout than the one the MPU views are computed from,
/// or hand a Task a channel index of another Partition.
pub fn check_partition_map(text: &str, g: &Generated, partition: usize) -> Result<(), String> {
    let start = text.find("pub mod layout {").ok_or("no memory map in the Partition's source")?;
    let map = &text[start..];
    let plan = &g.plan;
    let what = |name: &str| format!("Partition {partition}'s map: {name}");
    let region = |r: &crate::layout::Region| (r.base as u64, 1u64 << r.size_log2);
    same(&what("SELF"), value(map_constant(map, "SELF")?)?, partition as u64)?;
    same(&what("PARTITIONS"), value(map_constant(map, "PARTITIONS")?)?, plan.partitions.len() as u64)?;
    let list = |name: &str, derived: Vec<(u64, u64)>| -> Result<(), String> {
        let read = pairs(map_constant(map, name)?)?;
        if read != derived {
            return Err(format!("{}: the emitted source says {read:x?}, the derivation {derived:x?}", what(name)));
        }
        Ok(())
    };
    list("KERNEL_FLASH", vec![region(&plan.kernel_flash)])?;
    list("SHARED_CODE", vec![region(&plan.shared_code)])?;
    list("KERNEL_RAM", vec![region(&plan.kernel_ram)])?;
    list("KERNEL_WINDOWS", plan.kernel_windows.iter().map(|w| (w.base as u64, 1u64 << w.size_log2)).collect())?;
    list("CODE", plan.partitions.iter().map(|q| region(&q.code)).collect())?;
    list("RAM", plan.partitions.iter().map(|q| region(&q.ram)).collect())?;
    let numbers = |name: &str, derived: Vec<u64>| -> Result<(), String> {
        let body = map_constant(map, name)?;
        let read: Vec<u64> = body.trim().trim_start_matches('[').trim_end_matches(']').split(',').filter(|s| !s.trim().is_empty()).map(value).collect::<Result<_, _>>()?;
        if read != derived {
            return Err(format!("{}: the emitted source says {read:x?}, the derivation {derived:x?}", what(name)));
        }
        Ok(())
    };
    numbers("STACK", plan.partitions.iter().map(|q| q.stack as u64).collect())?;
    numbers("RAM_SRD", plan.partitions.iter().map(|q| q.ram.srd as u64).collect())?;
    let capacity = crate::layout::MAX_WINDOWS;
    let mut windows = Vec::new();
    let mut read_only = Vec::new();
    for q in &plan.partitions {
        for i in 0..capacity {
            windows.push(q.windows.get(i).map_or((0, 0), |w| (w.base as u64, 1u64 << w.size_log2)));
            read_only.push(q.windows.get(i).is_some_and(|w| w.read_only));
        }
    }
    list("WINDOWS", windows)?;
    let flags: Vec<bool> = map_constant(map, "WINDOWS_RO")?.split(|c: char| !c.is_ascii_alphabetic()).filter_map(|w| match w { "true" => Some(true), "false" => Some(false), _ => None }).collect();
    if flags != read_only {
        return Err(format!("{}: the emitted source says {flags:?}, the derivation {read_only:?}", what("WINDOWS_RO")));
    }
    // The Partition's own EasyDMA channels: their indices in the system's
    // table and the base addresses of their peripherals.
    let own: Vec<(u64, u64)> = {
        let mut index = 0u64;
        let mut out = Vec::new();
        for (pi, q) in plan.partitions.iter().enumerate() {
            for w in &q.windows {
                for name in &w.peripherals {
                    let Some(peripheral) = g.target.peripheral(name) else { continue };
                    let Some(registers) = peripheral.dma else { continue };
                    for _ in registers.channels {
                        if pi == partition {
                            out.push((index, peripheral.window_base as u64));
                        }
                        index += 1;
                    }
                }
            }
        }
        out
    };
    let needle = "pub const DMA: [::rsk::DmaChannel; ";
    let at = text.find(needle).ok_or("no EasyDMA channel constants in the Partition's source")?;
    let line = text[at..].lines().next().unwrap_or("");
    let read = pairs(line.split_once("] = ").map_or("", |(_, v)| v))?;
    if read != own {
        return Err(format!("{}: the emitted source says {read:x?}, the derivation {own:x?}", what("DMA")));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R24.1: the demo's emitted table reads back as derived, and a table in which one printed value differs from the derivation is refused naming the item.
    #[test]
    fn r24_1_the_emitted_table_reads_back_and_an_altered_one_is_refused() {
        let g = crate::generate(&crate::tests::demo()).expect("the demo is valid");
        let text = crate::emit::system_source(&g);
        check(&text, &g).expect("the emitted table matches the derivation");
        // One Task's period printed one tick short.
        let period = format!("period: {}", g.tasks[0].period_ticks);
        assert!(text.contains(&period));
        let altered = text.replacen(&period, &format!("period: {}", g.tasks[0].period_ticks - 1), 1);
        let e = check(&altered, &g).unwrap_err();
        assert!(e.contains(&g.tasks[0].id) && e.contains("period"), "{e}");
        // A Resource's Ceiling printed one level low.
        let ceiling = format!("ceiling: {}", g.resources[0].ceiling);
        let altered = text.replacen(&ceiling, &format!("ceiling: {}", g.resources[0].ceiling - 1), 1);
        let e = check(&altered, &g).unwrap_err();
        assert!(e.contains("ceiling"), "{e}");
        // A slot dropped from an array.
        let altered = text.replacen("slot_budget: [", "slot_budget: [0, ", 1);
        assert!(check(&altered, &g).is_err());
    }

    /// R24.1, R16.6: the demo's emitted layout reads back as derived, and a layout that is still well-formed but not the one derived is refused naming the item.
    #[test]
    fn r24_1_the_emitted_layout_reads_back_and_an_altered_one_is_refused() {
        let g = crate::generate(&crate::tests::demo()).expect("the demo is valid");
        let text = crate::emit::system_source(&g);
        check_layout(&text, &g).expect("the emitted layout matches the derivation");
        let p = &g.plan.partitions[1];
        // A Partition's RAM region printed at another Partition's base.
        let ram = format!("ram: Interval {{ base: 0x{:08x}", p.ram.base);
        assert!(text.contains(&ram));
        let altered = text.replacen(&ram, &format!("ram: Interval {{ base: 0x{:08x}", p.ram.base + (1 << p.ram.size_log2)), 1);
        let e = check_layout(&altered, &g).unwrap_err();
        assert!(e.contains(&p.name) && e.contains("ram base"), "{e}");
        // A stack printed smaller: well-formed, not the derived one.
        let stack = format!("stack_size: {}", p.stack);
        let altered = text.replacen(&stack, &format!("stack_size: {}", p.stack / 2), 1);
        assert!(check_layout(&altered, &g).unwrap_err().contains("stack_size"));
        // A peripheral window granted that the derivation does not have.
        let none = "windows: [NO_WINDOW";
        assert!(text.contains(none) && g.plan.partitions.iter().any(|p| p.windows.is_empty()));
        let altered = text.replacen(none, "windows: [Interval { base: 0x40002000, size_log2: 12 }", 1);
        assert!(check_layout(&altered, &g).unwrap_err().contains("the derivation has no window there"));
        // A Kernel window's size printed one step larger.
        let k = &g.plan.kernel_windows[0];
        let kw = format!("kernel_windows: [Interval {{ base: 0x{:08x}, size_log2: {} }}", k.base, k.size_log2);
        assert!(text.contains(&kw));
        let altered = text.replacen(&kw, &format!("kernel_windows: [Interval {{ base: 0x{:08x}, size_log2: {} }}", k.base, k.size_log2 + 1), 1);
        assert!(check_layout(&altered, &g).unwrap_err().contains("Kernel: window 0 size_log2"));
    }
    /// The demo on the nRF52840 with an EasyDMA peripheral in the second
    /// Partition and one Protocol-governed Endpoint between the Partitions.
    fn demo_with_dma_and_an_endpoint() -> Generated {
        use crate::decl::{EndpointDecl, ProtocolDecl, RoleDecl};
        let mut d = crate::tests::demo();
        d.target = "nrf52840".into();
        d.partitions[1].peripherals = vec!["SPIM0".into()];
        d.protocols.push(ProtocolDecl { name: "PingPong".into(), text: "rec L { choice at ctl { Req(u32): ctl -> sink; Rep(u32): sink -> ctl; continue L; } or { Stop(): ctl -> sink; end; } }".into() });
        d.endpoints.push(EndpointDecl {
            name: "ping".into(),
            ty: String::new(),
            protocol: Some("PingPong".into()),
            roles: vec![
                RoleDecl { role: "ctl".into(), partition: "part0".into(), task: "control".into(), validate: vec![(Some("Rep".into()), "crate::ok".into())], ops: 1, after: vec![] },
                RoleDecl { role: "sink".into(), partition: "part1".into(), task: "log".into(), validate: vec![], ops: 1, after: vec![] },
            ],
            capacity: 1,
            payload: 4,
        });
        crate::generate(&d).unwrap_or_else(|e| panic!("the declaration is valid: {e:?}"))
    }

    /// R24.1, R17 (2026-11-21): the emitted EasyDMA channel table reads back
    /// as the Target description and the memory plan imply, and a channel
    /// printed for another Partition or at another register is refused.
    #[test]
    fn r24_1_the_emitted_dma_table_reads_back_and_an_altered_one_is_refused() {
        let g = demo_with_dma_and_an_endpoint();
        let text = crate::emit::system_source(&g);
        check_dma(&text, &g).expect("the emitted DMA table matches the derivation");
        assert_eq!(derived_dma(&g).len(), 2, "SPIM0 has a receive and a transmit channel");
        // The receive channel given to the other Partition.
        let owner = "DmaCfg { partition: 1, ptr: 0x40003534";
        assert!(text.contains(owner), "{text}");
        let e = check_dma(&text.replacen(owner, "DmaCfg { partition: 0, ptr: 0x40003534", 1), &g).unwrap_err();
        assert!(e.contains("DMA channel 0: partition"), "{e}");
        // Its pointer register printed as the read-only AMOUNT register.
        let e = check_dma(&text.replacen("ptr: 0x40003534", "ptr: 0x4000353c", 1), &g).unwrap_err();
        assert!(e.contains("DMA channel 0: ptr"), "{e}");
        // A list register dropped.
        assert!(check_dma(&text.replacen("lists: [0x40003540, 0x40003550]", "lists: [0x40003540, 0x00000000]", 1), &g).unwrap_err().contains("lists"));
        // A channel left out of the table.
        let row = text.lines().find(|l| l.contains(owner)).unwrap();
        assert!(check_dma(&text.replacen(row, "", 1), &g).unwrap_err().contains("channels"));
        // The demo on QEMU has none, and the table says so.
        let plain = crate::generate(&crate::tests::demo()).unwrap();
        check_dma(&crate::emit::system_source(&plain), &plain).expect("an empty table");
    }

    /// R24.1, R40.1, R42 (2026-11-21): the emitted Endpoint table reads back
    /// as derived, and an altered capacity, role binding, Partition, release
    /// flag, storage offset or validation edge is refused naming the Endpoint.
    #[test]
    fn r24_1_the_emitted_endpoint_table_reads_back_and_an_altered_one_is_refused() {
        let g = demo_with_dma_and_an_endpoint();
        let text = crate::emit::system_source(&g);
        check_endpoints(&text, &g).expect("the emitted Endpoint table matches the derivation");
        let e = &g.endpoints[0];
        let row = text.lines().find(|l| l.contains("EndpointCfg {") && l.contains("// ping")).expect("the Endpoint's row");
        let altered = |from: &str, to: &str| {
            assert!(row.contains(from), "`{from}` in `{row}`");
            check_endpoints(&text.replacen(row, &row.replacen(from, to, 1), 1), &g).unwrap_err()
        };
        assert!(altered(&format!("capacity: {}", e.capacity), &format!("capacity: {}", e.capacity + 1)).contains("ping: capacity"));
        let (control, log) = (e.roles[0].1, e.roles[1].1);
        // The two roles' Tasks exchanged.
        assert!(altered(&format!("tasks: [{control}, {log},"), &format!("tasks: [{log}, {control},")).contains("ping: role 0 Task"));
        // The second role placed in the first role's Partition.
        assert!(altered("partitions: [0, 1,", "partitions: [0, 0,").contains("ping: role 1 Partition"));
        // A validation edge the declaration does not have.
        let none = "[false, false, false, false]]";
        assert!(altered(none, "[false, false, false, true]]").contains("validation of role 3 from role 3"));
        // The storage offset moved.
        assert!(altered(&format!("storage: {}", e.storage), &format!("storage: {}", e.storage + 4)).contains("ping: storage"));
        // The Kernel's storage printed smaller than the channels need.
        let bytes = format!("pub const ENDPOINT_STORAGE_BYTES: usize = {};", g.endpoint_storage_bytes);
        assert!(check_endpoints(&text.replacen(&bytes, "pub const ENDPOINT_STORAGE_BYTES: usize = 0;", 1), &g).unwrap_err().contains("ENDPOINT_STORAGE_BYTES"));
    }

    /// R24.1, R16.8 (2026-11-21): the memory map and the EasyDMA channel
    /// constants each Partition's source carries read back as the derived
    /// plan, and an altered region, window, stack or channel index is refused.
    #[test]
    fn r24_1_a_partitions_emitted_memory_map_reads_back_and_an_altered_one_is_refused() {
        let g = demo_with_dma_and_an_endpoint();
        for p in 0..g.partitions.len() {
            check_partition_map(&crate::emit::partition_source(&g, p), &g, p).unwrap_or_else(|e| panic!("Partition {p}: {e}"));
        }
        let text = crate::emit::partition_source(&g, 1);
        let q = &g.plan.partitions[1];
        // This Partition's source printed as another Partition's.
        assert!(check_partition_map(&text, &g, 0).unwrap_err().contains("SELF"));
        // The other Partition's RAM printed at this one's base.
        let other = format!("(0x{:08x}, 0x{:x})", g.plan.partitions[0].ram.base, 1u64 << g.plan.partitions[0].ram.size_log2);
        let own = format!("(0x{:08x}, 0x{:x})", q.ram.base, 1u64 << q.ram.size_log2);
        let rams = format!("pub const RAM: [(u32, u32); PARTITIONS] = [{other}, {own}];");
        assert!(text.contains(&rams), "{rams}");
        assert!(check_partition_map(&text.replacen(&rams, &format!("pub const RAM: [(u32, u32); PARTITIONS] = [{own}, {own}];"), 1), &g, 1).unwrap_err().contains("RAM"));
        // The window of the EasyDMA peripheral printed writable.
        assert!(q.windows.iter().any(|w| w.read_only));
        let ro = text.lines().skip_while(|l| !l.contains("pub const WINDOWS_RO")).nth(2).expect("the second Partition's flags");
        assert!(ro.contains("true"), "{ro}");
        assert!(check_partition_map(&text.replacen(ro, &ro.replacen("true", "false", 1), 1), &g, 1).unwrap_err().contains("WINDOWS_RO"));
        // A stack printed larger.
        let stacks = format!("pub const STACK: [u32; PARTITIONS] = [{}, {}];", g.plan.partitions[0].stack, q.stack);
        assert!(check_partition_map(&text.replacen(&stacks, &format!("pub const STACK: [u32; PARTITIONS] = [{}, {}];", g.plan.partitions[0].stack, q.stack * 2), 1), &g, 1).unwrap_err().contains("STACK"));
        // A channel index of the system's table that is not this Partition's.
        assert!(text.contains("::rsk::DmaChannel::generated(0, 0x40003000)"));
        assert!(check_partition_map(&text.replacen("generated(0, 0x40003000)", "generated(3, 0x40003000)", 1), &g, 1).unwrap_err().contains("DMA"));
    }

}
