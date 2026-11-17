//! The emitted tables read back (R24.1; 2026-11-16): after the system
//! source is written, the numbers of its `CONFIG` table are read out of the
//! text again, by a reader that shares nothing with the emitter but the
//! field names, and compared with the values the derivation produced. A
//! difference fails the generation.
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
}
