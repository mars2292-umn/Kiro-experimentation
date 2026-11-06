//! The Kernel's event mirror (rsk-kernel feature `trace-ram`): every
//! Health_Monitor event of the run, copied by Kernel_Arch into a ring of
//! the last 512 in `.rsk.noinit` (`EVENT_MIRROR`), with the trace
//! freeze: the sequence number of the first event outside the run's
//! expected set, at which the Kernel's trace ring stopped so that it holds
//! the entries leading to the event. The rig reads it after a run; a run
//! longer than the 32-entry log (the hour of PAR-05) is judged from it.

pub const MAGIC: u32 = 0x5253_4b45;
const HEADER: usize = 8;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRec {
    /// The event's index in the mirror (the run's events are numbered
    /// from 0 at instant 0).
    pub seq: u32,
    pub kind: u8,
    pub task: u8,
    pub partition: u8,
    /// The Kernel's observed instant, in ticks.
    pub instant: u64,
    pub detail: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Events {
    /// Events ever mirrored (the ring holds the last `entries.len()`).
    pub written: u32,
    /// The sequence number of the first unexpected event, at which the
    /// trace froze.
    pub frozen_at: Option<u32>,
    pub expect_kinds: u32,
    pub expect_task: u8,
    /// Oldest first.
    pub entries: Vec<EventRec>,
}

/// The event kinds of `rsk_kernel::logic::log::EV_*`.
pub fn kind_name(kind: u8) -> &'static str {
    match kind {
        1 => "panic",
        2 => "memmanage",
        3 => "busfault",
        4 => "usagefault",
        5 => "hardfault",
        6 => "overrun",
        7 => "deadline_miss",
        8 => "release_overlap",
        9 => "mit_violation",
        10 => "operating_end",
        11 => "resource_released",
        12 => "protocol_violation",
        13 => "dma_rejected",
        14 => "boot_check",
        15 => "watchdog_reset",
        16 => "partition_restart",
        17 => "partition_stop",
        18 => "safe_state",
        19 => "end_job",
        20 => "init_fault",
        21 => "unbound_irq",
        22 => "api_misuse",
        23 => "debugger",
        24 => "exec_over(diag)",
        _ => "?",
    }
}

impl Events {
    pub fn decode(words: &[u32]) -> Result<Events, String> {
        if words.len() < HEADER || words[0] != MAGIC {
            return Err("EVENT_MIRROR: no record (the magic word is absent: the Kernel never ran this build)".into());
        }
        let cap = (words[1] as usize).min((words.len() - HEADER) / 4);
        let written = words[2];
        let mut entries = Vec::new();
        if cap > 0 {
            let held = (written as usize).min(cap);
            let first = written as usize - held;
            for k in first..written as usize {
                let s = HEADER + 4 * (k % cap);
                entries.push(EventRec {
                    seq: k as u32,
                    kind: (words[s] >> 24) as u8,
                    task: (words[s] >> 16) as u8,
                    partition: (words[s] >> 8) as u8,
                    instant: (words[s + 2] as u64) << 32 | words[s + 1] as u64,
                    detail: words[s + 3],
                });
            }
        }
        Ok(Events {
            written,
            frozen_at: if words[4] == 0 { None } else { Some(words[4] - 1) },
            expect_kinds: words[5],
            expect_task: words[6] as u8,
            entries,
        })
    }

    /// The run's own events: up to and including the first `safe_state`.
    /// The mirror is cleared at instant 0 and survives the reboot the board
    /// takes when the probe detaches, whose events (a watchdog reset, a
    /// failed boot check, its own safe state) follow the run's and are not
    /// the run's.
    pub fn run_entries(&self) -> &[EventRec] {
        match self.entries.iter().position(|e| e.kind == 18) {
            Some(i) => &self.entries[..=i],
            None => &self.entries,
        }
    }

    /// The freeze, when it fell inside the run: before the run's
    /// `safe_state`, whether or not the frozen event is still held (a
    /// mirror that went on after the freeze let it out of the held window,
    /// and the stress run of 2026-11-05 reported "never froze" for a
    /// freeze at event 239 of 6,756; since then the mirror stops at the
    /// freeze, and this judgement no longer needs the event held).
    pub fn frozen_in_run(&self) -> Option<u32> {
        let run_end = self.entries.iter().find(|e| e.kind == 18).map(|e| e.seq).unwrap_or(self.written);
        self.frozen_at.filter(|seq| *seq < run_end)
    }

    pub fn is_expected(&self, e: &EventRec) -> bool {
        matches!(e.kind, 10 | 18 | 23)
            || (e.kind < 32 && (self.expect_kinds >> e.kind) & 1 == 1 && (self.expect_task == 0xFF || self.expect_task == e.task))
    }

    /// The events outside the expected set, oldest first.
    pub fn unexpected(&self) -> Vec<&EventRec> {
        self.run_entries().iter().filter(|e| !self.is_expected(e)).collect()
    }

    /// Counts per `(kind, task)`, sorted.
    pub fn counts(&self) -> Vec<((u8, u8), usize)> {
        let mut m: std::collections::BTreeMap<(u8, u8), usize> = Default::default();
        for e in self.run_entries() {
            *m.entry((e.kind, e.task)).or_default() += 1;
        }
        m.into_iter().collect()
    }

    /// One line: the counts and the freeze.
    pub fn summary(&self, tick_hz: u64) -> String {
        let counts: Vec<String> = self
            .counts()
            .into_iter()
            .map(|((k, t), n)| format!("{} of Task {}: {n}", kind_name(k), if t == 0xFF { "none".to_string() } else { t.to_string() }))
            .collect();
        let frozen = match self.frozen_in_run() {
            Some(seq) => match self.entries.iter().find(|e| e.seq == seq) {
                Some(e) => format!("; the trace froze at event {seq} ({} of Task {} at {:.3} ms)", kind_name(e.kind), e.task, e.instant as f64 * 1000.0 / tick_hz as f64),
                None => format!("; the trace froze at event {seq} (before the held window of the mirror)"),
            },
            None => "; the trace never froze during the run".to_string(),
        };
        let after = self.entries.len() - self.run_entries().len();
        format!(
            "{} events mirrored ({} held, {} of them the post-detach reboot's): {}{frozen}",
            self.written,
            self.entries.len(),
            after,
            if counts.is_empty() { "none".to_string() } else { counts.join(", ") }
        )
    }

    /// The entries as text, one per line.
    pub fn render(&self, tick_hz: u64) -> String {
        let mut out = format!("# {} events mirrored, {} held; frozen_at={:?} expect_kinds={:#x} expect_task={}\n# seq t_ms kind task partition detail\n", self.written, self.entries.len(), self.frozen_at, self.expect_kinds, self.expect_task);
        for e in &self.entries {
            out.push_str(&format!(
                "{:6} {:12.3} {:<18} {:3} {:3} {:#010x}{}\n",
                e.seq,
                e.instant as f64 * 1000.0 / tick_hz as f64,
                kind_name(e.kind),
                e.task,
                e.partition,
                e.detail,
                if self.run_entries().iter().any(|r| r.seq == e.seq) {
                    if self.is_expected(e) { "" } else { "  <- unexpected" }
                } else {
                    "  (after the run: the reboot on detach)"
                }
            ));
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_mirror_and_the_freeze() {
        let cap = 3;
        let mut w = vec![0u32; HEADER + 4 * cap];
        w[0] = MAGIC;
        w[1] = cap as u32;
        w[2] = 4; // four written into three slots: events 1, 2, 3 remain
        w[4] = 3; // frozen at event 2
        w[5] = 1 << 6;
        w[6] = 2;
        for k in 0..4u32 {
            let s = HEADER + 4 * (k as usize % cap);
            let kind = if k == 2 { 7u32 } else { 6 };
            w[s] = kind << 24 | 2 << 16 | 0 << 8;
            w[s + 1] = 16_000 * (k + 1);
            w[s + 3] = k;
        }
        let e = Events::decode(&w).unwrap();
        assert_eq!(e.entries.len(), 3);
        assert_eq!(e.run_entries().len(), 3);
        assert_eq!(e.entries[0].seq, 1);
        assert_eq!(e.frozen_at, Some(2));
        let un = e.unexpected();
        assert_eq!(un.len(), 1);
        assert_eq!(un[0].kind, 7);
        assert!(e.summary(16_000_000).contains("the trace froze at event 2 (deadline_miss of Task 2 at 3.000 ms)"));
        assert!(e.render(16_000_000).contains("<- unexpected"));
        assert!(Events::decode(&[0; 8]).is_err());
    }

    #[test]
    fn the_reboot_on_detach_is_not_the_run() {
        let cap = 4;
        let mut w = vec![0u32; HEADER + 4 * cap];
        w[0] = MAGIC;
        w[1] = cap as u32;
        w[2] = 3;
        w[4] = 3; // frozen at event 2: the reboot's watchdog reset
        for (k, kind) in [(0usize, 10u32), (1, 18), (2, 15)] {
            w[HEADER + 4 * k] = kind << 24 | 0xFF << 16 | 0xFF << 8;
        }
        let e = Events::decode(&w).unwrap();
        assert_eq!(e.run_entries().len(), 2);
        assert!(e.unexpected().is_empty());
        assert_eq!(e.frozen_in_run(), None);
        assert!(e.summary(16_000_000).contains("1 of them the post-detach reboot's"));
        assert!(e.render(16_000_000).contains("after the run"));
    }
}
