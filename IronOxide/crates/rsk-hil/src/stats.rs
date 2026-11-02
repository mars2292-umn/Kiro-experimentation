//! The Kernel's release-timing record (rsk-kernel feature `trace-ram`,
//! R9.10): `RELEASE_STATS` holds, per Task, the count, maximum, sum, and a
//! log2 histogram of the lag from each periodic nominal instant to the
//! Release becoming effective, in CPU cycles, a count of due instants
//! that were not `O + k*T`, and a ring of the last 256 Releases.

pub const MAGIC: u32 = 0x5253_4b53;
const HEADER: usize = 4;
const PER_TASK: usize = 26;
const RING: usize = 256;

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct TaskStats {
    pub count: u32,
    pub max_cycles: u32,
    pub sum_cycles: u64,
    /// Bucket b counts lags below `2^b` cycles (bucket 15: the rest).
    pub histogram: [u32; 16],
    /// Jobs completed and the largest response time (Release due to Job
    /// end) in ticks (R18.8).
    pub resp_count: u32,
    pub resp_max_ticks: u32,
    /// Runs of the intake handler for the Task's interrupt source, the
    /// largest and the summed cycles (R19.3).
    pub intake_count: u32,
    pub intake_max_cycles: u32,
    pub intake_sum_cycles: u64,
}

impl TaskStats {
    pub fn mean_cycles(&self) -> f64 {
        if self.count == 0 { 0.0 } else { self.sum_cycles as f64 / self.count as f64 }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Release {
    pub task: u8,
    pub due: u64,
    pub lag_cycles: u32,
}

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub tasks: Vec<TaskStats>,
    /// Due instants that were not `offset + k * period`.
    pub nominal_mismatches: u32,
    /// The Releases the ring holds, oldest first.
    pub recent: Vec<Release>,
    pub releases_recorded: u32,
}

impl Stats {
    pub fn decode(words: &[u32]) -> Result<Stats, String> {
        if words.len() < HEADER || words[0] != MAGIC {
            return Err("RELEASE_STATS: no record (the magic word is absent: the Kernel never ran this build)".into());
        }
        let n = words[1] as usize;
        let need = HEADER + n * PER_TASK + 1;
        if words.len() < need {
            return Err(format!("RELEASE_STATS: {} words, fewer than the {need} the header announces", words.len()));
        }
        let mut tasks = Vec::with_capacity(n);
        for t in 0..n {
            let b = HEADER + t * PER_TASK;
            let mut histogram = [0u32; 16];
            histogram.copy_from_slice(&words[b + 4..b + 20]);
            tasks.push(TaskStats {
                count: words[b],
                max_cycles: words[b + 1],
                sum_cycles: (words[b + 3] as u64) << 32 | words[b + 2] as u64,
                histogram,
                resp_count: words[b + 20],
                resp_max_ticks: words[b + 21],
                intake_count: words[b + 22],
                intake_max_cycles: words[b + 23],
                intake_sum_cycles: (words[b + 25] as u64) << 32 | words[b + 24] as u64,
            });
        }
        let ring = HEADER + n * PER_TASK;
        let written = words[ring];
        let slots = ((words.len() - ring - 1) / 4).min(RING);
        let mut recent = Vec::new();
        if slots > 0 {
            let first = (written as usize).saturating_sub(slots);
            for k in first..written as usize {
                let s = ring + 1 + 4 * (k % slots);
                recent.push(Release { task: words[s] as u8, due: (words[s + 2] as u64) << 32 | words[s + 1] as u64, lag_cycles: words[s + 3] });
            }
        }
        Ok(Stats { tasks, nominal_mismatches: words[2], recent, releases_recorded: written })
    }

    /// The response-time table (Tasks with completed Jobs): `(task, jobs, max ms)`.
    pub fn responses(&self, tick_hz: u64) -> Vec<(usize, u32, f64)> {
        self.tasks.iter().enumerate().filter(|(_, t)| t.resp_count > 0).map(|(i, t)| (i, t.resp_count, t.resp_max_ticks as f64 * 1000.0 / tick_hz as f64)).collect()
    }

    /// The largest lag over every Task.
    pub fn max_cycles(&self) -> u32 {
        self.tasks.iter().map(|t| t.max_cycles).max().unwrap_or(0)
    }

    pub fn total(&self) -> u64 {
        self.tasks.iter().map(|t| t.count as u64).sum()
    }

    /// The record as a Markdown table (one row per Task with Releases).
    pub fn render(&self, cpu_hz: u64, tick_hz: u64) -> String {
        let mut s = format!(
            "| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |\n|---|---|---|---|---|\n"
        );
        for (t, st) in self.tasks.iter().enumerate() {
            if st.count == 0 {
                continue;
            }
            let lo = st.histogram.iter().position(|c| *c > 0).unwrap_or(0);
            let hi = st.histogram.iter().rposition(|c| *c > 0).unwrap_or(0);
            let hist: Vec<String> = (lo..=hi).map(|b| format!("2^{b}:{}", st.histogram[b])).collect();
            s.push_str(&format!(
                "| {t} | {} | {} / {:.1} | {:.0} | {} |\n",
                st.count,
                st.max_cycles,
                st.max_cycles as f64 * 1e6 / cpu_hz as f64,
                st.mean_cycles(),
                hist.join(" ")
            ));
        }
        s.push_str(&format!(
            "\nReleases recorded: {}; nominal instants not `O + k*T`: {}.\n",
            self.releases_recorded, self.nominal_mismatches
        ));
        if self.tasks.iter().any(|t| t.resp_count > 0) {
            s.push_str("\n| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |\n|---|---|---|---|---|---|\n");
            for (t, st) in self.tasks.iter().enumerate() {
                if st.resp_count == 0 && st.intake_count == 0 {
                    continue;
                }
                s.push_str(&format!(
                    "| {t} | {} | {:.3} | {} | {} | {:.0} |\n",
                    st.resp_count,
                    st.resp_max_ticks as f64 * 1000.0 / tick_hz as f64,
                    st.intake_count,
                    st.intake_max_cycles,
                    if st.intake_count == 0 { 0.0 } else { st.intake_sum_cycles as f64 / st.intake_count as f64 }
                ));
            }
        }
        if !self.recent.is_empty() {
            s.push_str("\nThe last recorded Releases (Task, nominal instant, lag):\n\n");
            for r in self.recent.iter().rev().take(12).collect::<Vec<_>>().into_iter().rev() {
                s.push_str(&format!("- Task {} at {:.3} ms, {} cycles\n", r.task, r.due as f64 * 1000.0 / tick_hz as f64, r.lag_cycles));
            }
        }
        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn decodes_the_record() {
        let n = 2;
        let mut w = vec![0u32; HEADER + n * PER_TASK + 1 + 4 * 3];
        w[0] = MAGIC;
        w[1] = n as u32;
        w[2] = 1;
        let b = HEADER + PER_TASK;
        w[b] = 3;
        w[b + 1] = 3000;
        w[b + 2] = 7500;
        w[b + 4 + 12] = 3;
        w[b + 20] = 3;
        w[b + 21] = 160_000;
        w[b + 22] = 2;
        w[b + 23] = 900;
        w[b + 24] = 1500;
        let ring = HEADER + n * PER_TASK;
        w[ring] = 4; // four written into three slots: entries 1, 2, 3 remain
        for k in 0..4u32 {
            let s = ring + 1 + 4 * (k as usize % 3);
            w[s] = 1;
            w[s + 1] = 160_000 * (k + 1);
            w[s + 3] = 2500 + k;
        }
        let st = Stats::decode(&w).unwrap();
        assert_eq!(st.tasks[1].count, 3);
        assert_eq!(st.tasks[1].max_cycles, 3000);
        assert_eq!(st.tasks[1].mean_cycles(), 2500.0);
        assert_eq!(st.nominal_mismatches, 1);
        assert_eq!(st.max_cycles(), 3000);
        assert_eq!(st.recent.len(), 3);
        assert_eq!(st.recent[0].due, 320_000);
        assert_eq!(st.recent[2].lag_cycles, 2503);
        assert!(st.render(64_000_000, 16_000_000).contains("| 1 | 3 | 3000 / 46.9 | 2500 |"));
        assert_eq!(st.tasks[1].resp_max_ticks, 160_000);
        assert_eq!(st.responses(16_000_000), vec![(1, 3, 10.0)]);
        assert!(st.render(64_000_000, 16_000_000).contains("| 1 | 3 | 10.000 | 2 | 900 | 750 |"));
        assert!(Stats::decode(&[0, 0, 0, 0]).is_err());
    }
}
