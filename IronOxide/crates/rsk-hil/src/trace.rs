//! The Kernel's RAM trace ring (rsk-kernel feature `trace-ram`): the
//! `TRACE_RING` symbol holds a count followed by triples (tag << 24 |
//! value, time-base low word, high word) of every Kernel entry point
//! (`now()` calls excepted). The rig reads it over the probe after a run,
//! decodes it, and looks for an interrupt storm: one vector re-entering
//! the Kernel many times with no dispatch between, which starves every
//! Task below it (the GPIOTE storm of 2026-11-01).

/// One decoded entry.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// The time base in ticks.
    pub ticks: u64,
    pub tag: &'static str,
    /// The tag's number, for the ones `tag_name` does not know.
    pub raw_tag: u8,
    pub value: u32,
}

/// The trace tags of `rsk_kernel::arch::cm4::entry::TR_*`.
pub fn tag_name(tag: u8) -> &'static str {
    match tag {
        1 => "boot",
        2 => "invalid",
        3 => "bootfail",
        4 => "booted",
        5 => "start",
        6 => "irq",
        7 => "dispatch",
        8 => "tick",
        9 => "svc",
        10 => "epilogue",
        11 => "fault",
        12 => "arm",
        13 => "cfsr",
        14 => "pc",
        15 => "dmastop",
        16 => "reinit",
        17 => "window",
        _ => "?",
    }
}

/// Decodes the ring's words (the count first) into the entries it holds,
/// oldest first.
pub fn decode(words: &[u32]) -> Vec<Entry> {
    let Some(&count) = words.first() else { return Vec::new() };
    let len = (words.len() - 1) / 3;
    if len == 0 {
        return Vec::new();
    }
    let count = count as usize;
    let first = count.saturating_sub(len);
    (first..count)
        .map(|k| {
            let s = 1 + 3 * (k % len);
            Entry { ticks: (words[s + 2] as u64) << 32 | words[s + 1] as u64, tag: tag_name((words[s] >> 24) as u8), raw_tag: (words[s] >> 24) as u8, value: words[s] & 0x00FF_FFFF }
        })
        .collect()
}

/// The longest run of consecutive `irq` entries of one vector with no
/// dispatch, tick, or SVC between them: `(vector, entries, ticks spanned)`.
pub fn longest_storm(entries: &[Entry]) -> Option<(u32, usize, u64)> {
    let mut best: Option<(u32, usize, u64)> = None;
    let mut run: Option<(u32, usize, u64)> = None;
    for e in entries {
        match (e.tag, &mut run) {
            ("irq", Some((v, n, start))) if *v == e.value => {
                *n += 1;
                let span = e.ticks.saturating_sub(*start);
                if best.is_none_or(|b| *n > b.1) {
                    best = Some((*v, *n, span));
                }
            }
            ("irq", _) => run = Some((e.value, 1, e.ticks)),
            _ => run = None,
        }
    }
    best
}

/// The entries as text: one line per entry, the time in milliseconds of
/// the time base at `tick_hz`.
pub fn render(entries: &[Entry], tick_hz: u64) -> String {
    let mut out = format!("# {} entries; t_ms tag value\n", entries.len());
    for e in entries {
        let ms = e.ticks as f64 * 1000.0 / tick_hz as f64;
        let extra = match e.tag {
            "irq" => format!(" vector={}", e.value),
            "dispatch" => format!(" task={} frame={:#x}", e.value & 7, e.value & !7),
            "svc" => format!(" number={}", e.value),
            "?" => format!(" tag={}", e.raw_tag),
            _ => String::new(),
        };
        out.push_str(&format!("{ms:10.3} {:<9} {:#08x}{extra}\n", e.tag, e.value));
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn word(tag: u8, v: u32) -> u32 {
        (tag as u32) << 24 | v
    }

    #[test]
    fn decodes_a_wrapped_ring() {
        // A ring of 2 entries that has seen 3 writes: entries 1 and 2 remain.
        let words = vec![3, word(6, 22), 300, 0, word(8, 7), 200, 0];
        let e = decode(&words);
        assert_eq!(e.len(), 2);
        assert_eq!(e[0], Entry { ticks: 200, tag: "tick", raw_tag: 8, value: 7 });
        assert_eq!(e[1], Entry { ticks: 300, tag: "irq", raw_tag: 6, value: 22 });
        assert!(decode(&[0]).is_empty());
        assert!(decode(&[]).is_empty());
    }

    #[test]
    fn finds_the_storm() {
        let mut entries = vec![Entry { ticks: 0, tag: "tick", raw_tag: 8, value: 0 }];
        for k in 0..5 {
            entries.push(Entry { ticks: 100 + k * 10, tag: "irq", raw_tag: 6, value: 22 });
        }
        entries.push(Entry { ticks: 200, tag: "dispatch", raw_tag: 7, value: 4 });
        entries.push(Entry { ticks: 300, tag: "irq", raw_tag: 6, value: 22 });
        entries.push(Entry { ticks: 310, tag: "irq", raw_tag: 6, value: 24 });
        assert_eq!(longest_storm(&entries), Some((22, 5, 40)));
        assert_eq!(longest_storm(&entries[..1]), None);
        let text = render(&entries, 16_000_000);
        assert!(text.contains("irq       0x000016 vector=22"));
        assert!(text.contains("dispatch  0x000004 task=4"));
    }
}
