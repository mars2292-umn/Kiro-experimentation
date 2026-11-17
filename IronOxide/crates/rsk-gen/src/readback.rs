//! The emitted tables read back (R24.1; 2026-11-16): after the system
//! source is written, the numbers of its `CONFIG` table are read out of the
//! text again, by a reader that shares nothing with the emitter but the
//! field names, and compared with the values the derivation produced. A
//! difference fails the generation.
//!
//! The same is done for the `LAYOUT` table against the derived memory plan
//! (`check_layout`, 2026-11-17).
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
}
