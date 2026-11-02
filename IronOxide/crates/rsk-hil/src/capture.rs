//! The logic analyzer capture (sigrok-cli, binary output: one byte per
//! sample, bit `n` = channel D`n`) and the edge analysis of a loopback:
//! each response edge is matched to the latest stimulus rising edge at or
//! before it, and every stimulus rise inside the run's window must be
//! answered before the next one.

use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};

use crate::config::Analyzer;

/// `sigrok-cli --version`'s first line.
pub fn version() -> Result<String, String> {
    let out = Command::new("sigrok-cli").arg("--version").output().map_err(|e| format!("cannot run sigrok-cli: {e}"))?;
    if !out.status.success() {
        return Err("sigrok-cli --version failed".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout).lines().next().unwrap_or("").trim().to_string())
}

/// `sigrok-cli --scan`'s line for the driver, if the device is attached.
pub fn scan(driver: &str) -> Result<Option<String>, String> {
    let out = Command::new("sigrok-cli").args(["-d", driver, "--scan"]).output().map_err(|e| format!("cannot run sigrok-cli: {e}"))?;
    if !out.status.success() {
        return Err(format!("sigrok-cli --scan failed: {}", String::from_utf8_lossy(&out.stderr).trim()));
    }
    let text = String::from_utf8_lossy(&out.stdout);
    Ok(text.lines().map(str::trim).find(|l| l.starts_with(driver)).map(str::to_string))
}

/// A capture in progress.
pub struct Capture {
    child: Child,
    pub path: PathBuf,
    pub command: String,
    pub samplerate_hz: u32,
}

/// Starts a capture of `duration_ms` at the analyzer's sample rate into
/// `path` (binary samples).
pub fn start(analyzer: &Analyzer, duration_ms: u64, path: &Path) -> Result<Capture, String> {
    let rate = format!("samplerate={}", analyzer.samplerate_hz);
    let ms = duration_ms.to_string();
    let args = ["-d", &analyzer.driver, "-c", &rate, "--time", &ms, "-O", "binary", "-o"];
    let command = format!("sigrok-cli {} {}", args.join(" "), path.display());
    let child = Command::new("sigrok-cli").args(args).arg(path).stdout(Stdio::null()).stderr(Stdio::piped()).spawn().map_err(|e| format!("cannot run sigrok-cli: {e}"))?;
    Ok(Capture { child, path: path.to_path_buf(), command, samplerate_hz: analyzer.samplerate_hz })
}

impl Capture {
    /// Stops the capture now (sigrok-cli ends the acquisition on SIGTERM or
    /// SIGINT and writes what it has; a job run in the background inherits
    /// SIGINT ignored, so SIGTERM goes first) and returns the samples. The
    /// capture is bounded by its `--time` in any case, and the analysis
    /// finds the run inside whatever was captured.
    pub fn stop(mut self) -> Result<Vec<u8>, String> {
        let pid = self.child.id().to_string();
        for signal in ["-TERM", "-INT"] {
            if self.child.try_wait().map_err(|e| format!("sigrok-cli: {e}"))?.is_some() {
                break;
            }
            let _ = Command::new("kill").args([signal, &pid]).status();
            std::thread::sleep(std::time::Duration::from_millis(300));
        }
        self.finish()
    }

    /// Waits for the capture to end and returns the samples.
    pub fn finish(mut self) -> Result<Vec<u8>, String> {
        let status = self.child.wait().map_err(|e| format!("sigrok-cli: {e}"))?;
        // SIGINT ends sigrok-cli with a signal status after it has written the file.
        if !status.success() && std::fs::metadata(&self.path).map(|m| m.len() == 0).unwrap_or(true) {
            let mut err = String::new();
            if let Some(mut e) = self.child.stderr.take() {
                use std::io::Read;
                let _ = e.read_to_string(&mut err);
            }
            return Err(format!("{} failed: {}", self.command, err.trim()));
        }
        std::fs::read(&self.path).map_err(|e| format!("cannot read {}: {e}", self.path.display()))
    }
}

/// The target's dispatch timeline from the trace pins (rsk-kernel feature
/// `trace-pins`): per Task index 0 to 7 (7 = none), the samples it was
/// executing and the number of its runs (entries), and the Kernel-active
/// samples and entries.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Timeline {
    pub samples: [usize; 8],
    pub runs: [usize; 8],
    pub kernel_samples: usize,
    pub kernel_entries: usize,
    /// The longest run of each Task, in samples.
    pub longest_run: [usize; 8],
}

/// Decodes the timeline over `samples[from..to]`.
pub fn timeline(samples: &[u8], id_channels: &[u8], kernel: Option<u8>, from: usize, to: usize) -> Timeline {
    let mut t = Timeline::default();
    let id_of = |b: u8| id_channels.iter().enumerate().fold(0usize, |id, (bit, ch)| id | (((b >> ch) & 1) as usize) << bit);
    let mut prev: Option<usize> = None;
    let mut run = 0;
    let mut prev_k = false;
    for &b in &samples[from.min(samples.len())..to.min(samples.len())] {
        let id = id_of(b).min(7);
        t.samples[id] += 1;
        if prev != Some(id) {
            if let Some(p) = prev {
                t.longest_run[p] = t.longest_run[p].max(run);
            }
            t.runs[id] += 1;
            run = 0;
        }
        run += 1;
        prev = Some(id);
        if let Some(k) = kernel {
            let active = (b >> k) & 1 == 1;
            if active {
                t.kernel_samples += 1;
                if !prev_k {
                    t.kernel_entries += 1;
                }
            }
            prev_k = active;
        }
    }
    if let Some(p) = prev {
        t.longest_run[p] = t.longest_run[p].max(run);
    }
    t
}

/// The edges of one channel: `(sample index, level after the edge)`.
pub fn edges(samples: &[u8], channel: u8) -> Vec<(usize, bool)> {
    let mut out = Vec::new();
    let Some(first) = samples.first() else { return out };
    let mut prev = (first >> channel) & 1;
    for (i, s) in samples.iter().enumerate().skip(1) {
        let v = (s >> channel) & 1;
        if v != prev {
            out.push((i, v == 1));
            prev = v;
        }
    }
    out
}

/// The loopback analysis of one capture.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Analysis {
    pub samples: usize,
    pub samplerate_hz: u32,
    /// Stimulus rising edges in the whole capture.
    pub stimulus_rises: usize,
    /// Response edges (either direction) in the whole capture.
    pub response_edges: usize,
    /// Stimulus periods seen, in microseconds (min, max).
    pub period_us: Option<(f64, f64)>,
    /// Stimulus pulse widths seen, in microseconds (min, max).
    pub pulse_us: Option<(f64, f64)>,
    /// Each matched response: `(stimulus rise, latency in microseconds)`.
    pub latencies_us: Vec<(usize, f64)>,
    /// Response edges outside the run's burst (the line floats while the
    /// target is flashed, is driven low when its Partition starts, and
    /// floats again when the board resets after the probe detaches).
    pub responses_before_run: usize,
    pub responses_after_run: usize,
    /// Response edges with no stimulus rise before them.
    pub unmatched_responses: usize,
    /// Stimulus rises inside the window (from the first answered rise to
    /// the last answered one) that no response followed before the next rise.
    pub unanswered_in_window: usize,
    /// The first few unanswered rises, as seconds into the capture.
    pub unanswered_at_s: Vec<f64>,
    /// Stimulus rises before the window (the stimulus board runs before
    /// the target) and after it (after the target's operating duration).
    pub stimulus_before: usize,
    pub stimulus_after: usize,
    /// The window as seconds into the capture.
    pub window_s: Option<(f64, f64)>,
}

impl Analysis {
    pub fn matched(&self) -> usize {
        self.latencies_us.len()
    }

    pub fn latency_min_us(&self) -> Option<f64> {
        self.latencies_us.iter().map(|(_, l)| *l).reduce(f64::min)
    }

    pub fn latency_max_us(&self) -> Option<f64> {
        self.latencies_us.iter().map(|(_, l)| *l).reduce(f64::max)
    }

    pub fn latency_median_us(&self) -> Option<f64> {
        let mut v: Vec<f64> = self.latencies_us.iter().map(|(_, l)| *l).collect();
        if v.is_empty() {
            return None;
        }
        v.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        Some(v[v.len() / 2])
    }

    /// Matched responses later than `deadline_us` after their stimulus.
    pub fn late(&self, deadline_us: f64) -> usize {
        self.latencies_us.iter().filter(|(_, l)| *l > deadline_us).count()
    }

    /// One line per edge, for the evidence artifact.
    pub fn edges_text(&self, samples: &[u8], stimulus: u8, response: u8) -> String {
        let mut out = format!("# samples={} samplerate_hz={} stimulus=D{stimulus} response=D{response}\n# t_s channel level\n", self.samples, self.samplerate_hz);
        let mut all: Vec<(usize, u8, bool)> = edges(samples, stimulus).into_iter().map(|(i, l)| (i, stimulus, l)).collect();
        all.extend(edges(samples, response).into_iter().map(|(i, l)| (i, response, l)));
        all.sort_unstable();
        for (i, ch, level) in all {
            out.push_str(&format!("{:.6} D{ch} {}\n", i as f64 / self.samplerate_hz as f64, u8::from(level)));
        }
        out
    }
}

/// Splits edge instants into bursts: consecutive edges closer than `gap`
/// samples belong to one burst.
pub fn bursts(edges: &[usize], gap: usize) -> Vec<&[usize]> {
    let mut out = Vec::new();
    let mut start = 0;
    for i in 1..=edges.len() {
        if i == edges.len() || edges[i] - edges[i - 1] > gap {
            out.push(&edges[start..i]);
            start = i;
        }
    }
    out
}

/// Analyses a capture: stimulus rises on `stimulus`, response edges on
/// `response`. With `run_responses` (the Releases the target counted),
/// the run is the last that many edges of the burst of response edges
/// (edges less than `burst_gap` samples apart) whose size is closest to
/// that count; edges outside it are the line floating during flashing,
/// its first drive, and the board's reset after the probe detaches,
/// counted apart.
pub fn analyze(samples: &[u8], samplerate_hz: u32, stimulus: u8, response: u8, run_responses: Option<usize>, burst_gap: usize) -> Analysis {
    let us = |n: usize| n as f64 * 1e6 / samplerate_hz as f64;
    let stim = edges(samples, stimulus);
    let rises: Vec<usize> = stim.iter().filter(|(_, l)| *l).map(|(i, _)| *i).collect();
    let falls: Vec<usize> = stim.iter().filter(|(_, l)| !*l).map(|(i, _)| *i).collect();
    let all_resp: Vec<usize> = edges(samples, response).into_iter().map(|(i, _)| i).collect();
    let mut a = Analysis { samples: samples.len(), samplerate_hz, stimulus_rises: rises.len(), response_edges: all_resp.len(), ..Analysis::default() };
    let resp: Vec<usize> = match run_responses {
        Some(n) if !all_resp.is_empty() => {
            let groups = bursts(&all_resp, burst_gap);
            let run = groups.iter().min_by_key(|g| (g.len().abs_diff(n), usize::MAX - g[0])).copied().unwrap_or(&[]);
            // The Partition's first Job drives the floating line low just
            // before the first pulse: that edge joins the burst and is the
            // one before the last `n`.
            let run = if run.len() > n { &run[run.len() - n..] } else { run };
            let first = run.first().copied().unwrap_or(0);
            let last = run.last().copied().unwrap_or(0);
            a.responses_before_run = all_resp.iter().filter(|r| **r < first).count();
            a.responses_after_run = all_resp.iter().filter(|r| **r > last).count();
            run.to_vec()
        }
        _ => all_resp,
    };
    if rises.len() > 1 {
        let periods: Vec<f64> = rises.windows(2).map(|w| us(w[1] - w[0])).collect();
        a.period_us = Some((periods.iter().cloned().fold(f64::INFINITY, f64::min), periods.iter().cloned().fold(0.0, f64::max)));
    }
    let widths: Vec<f64> = rises.iter().filter_map(|r| falls.iter().find(|f| *f > r).map(|f| us(f - r))).collect();
    if !widths.is_empty() {
        a.pulse_us = Some((widths.iter().cloned().fold(f64::INFINITY, f64::min), widths.iter().cloned().fold(0.0, f64::max)));
    }
    // Each response edge answers the latest rise at or before it; a rise
    // answered twice (a toggle glitch) counts the second as unmatched.
    let mut answered = vec![false; rises.len()];
    for r in &resp {
        let k = rises.partition_point(|s| *s <= *r);
        if k == 0 || answered[k - 1] {
            a.unmatched_responses += 1;
            continue;
        }
        answered[k - 1] = true;
        a.latencies_us.push((rises[k - 1], us(r - rises[k - 1])));
    }
    if let (Some(first), Some(last)) = (answered.iter().position(|x| *x), answered.iter().rposition(|x| *x)) {
        a.stimulus_before = first;
        a.stimulus_after = rises.len() - 1 - last;
        a.unanswered_in_window = answered[first..=last].iter().filter(|x| !**x).count();
        a.unanswered_at_s = (first..=last).filter(|k| !answered[*k]).take(8).map(|k| rises[k] as f64 / samplerate_hz as f64).collect();
        a.window_s = Some((rises[first] as f64 / samplerate_hz as f64, (resp.last().copied().unwrap_or(rises[last])) as f64 / samplerate_hz as f64));
    } else {
        a.stimulus_before = rises.len();
    }
    a
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A synthetic capture at 1 MHz: the stimulus (bit 6) rises every
    /// 1,000 samples for 100; the response (bit 0) toggles 50 samples
    /// after the rises 2 to 5 of the 8.
    fn capture() -> Vec<u8> {
        let mut s = vec![0u8; 9_000];
        for k in 0..8 {
            for i in 0..100 {
                s[500 + k * 1000 + i] |= 1 << 6;
            }
        }
        let mut level = 0u8;
        for k in 2..6 {
            level ^= 1;
            for i in 550 + k * 1000..s.len() {
                s[i] = (s[i] & !1) | level;
            }
        }
        s
    }

    #[test]
    fn decodes_the_timeline() {
        // Task ids on bits 1 to 3, the Kernel flag on bit 4: idle, Task 2
        // for 3 samples, a Kernel entry, Task 2 again, idle.
        let s = [0b1110u8, 0b0100, 0b0100, 0b0100, 0b10100, 0b0100, 0b1110];
        let t = timeline(&s, &[1, 2, 3], Some(4), 0, s.len());
        assert_eq!(t.samples[2], 5);
        assert_eq!(t.runs[2], 1);
        assert_eq!(t.longest_run[2], 5);
        assert_eq!(t.samples[7], 2);
        assert_eq!(t.runs[7], 2);
        assert_eq!(t.kernel_samples, 1);
        assert_eq!(t.kernel_entries, 1);
    }

    #[test]
    fn finds_edges() {
        let e = edges(&capture(), 6);
        assert_eq!(e.len(), 16);
        assert_eq!(e[0], (500, true));
        assert_eq!(e[1], (600, false));
        assert!(edges(&[], 0).is_empty());
    }

    #[test]
    fn matches_responses_to_stimuli() {
        let a = analyze(&capture(), 1_000_000, 6, 0, None, 3000);
        assert_eq!(a.stimulus_rises, 8);
        assert_eq!(a.response_edges, 4);
        assert_eq!(a.matched(), 4);
        assert_eq!(a.period_us, Some((1000.0, 1000.0)));
        assert_eq!(a.pulse_us, Some((100.0, 100.0)));
        assert_eq!(a.latency_min_us(), Some(50.0));
        assert_eq!(a.latency_max_us(), Some(50.0));
        assert_eq!(a.stimulus_before, 2);
        assert_eq!(a.stimulus_after, 2);
        assert_eq!(a.unanswered_in_window, 0);
        assert_eq!(a.unmatched_responses, 0);
        assert_eq!(a.late(40.0), 4);
        assert_eq!(a.late(60.0), 0);
        let text = a.edges_text(&capture(), 6, 0);
        assert!(text.lines().nth(2).unwrap().starts_with("0.000500 D6 1"));
    }

    #[test]
    fn counts_unanswered_and_unmatched() {
        let mut s = capture();
        // Invert the response between the toggles at 3550 and 4550: both
        // edges vanish, so rises 3 and 4 go unanswered inside the window.
        for i in 3550..4550 {
            s[i] ^= 1;
        }
        let a = analyze(&s, 1_000_000, 6, 0, None, 3000);
        assert_eq!(a.matched(), 2);
        assert_eq!(a.unanswered_in_window, 2);
        assert_eq!(a.stimulus_before, 2);
        assert_eq!(a.stimulus_after, 2);
        // A response before any stimulus is unmatched; told the run had 4
        // responses, the analysis sets the two early edges apart instead.
        let mut s = capture();
        for i in 10..20 {
            s[i] |= 1;
        }
        let a = analyze(&s, 1_000_000, 6, 0, None, 3000);
        assert_eq!(a.unmatched_responses, 2);
        // A stray edge long after the run (the board's reset) is set apart too.
        for i in 8_900..9_000 {
            s[i] |= 1;
        }
        let a = analyze(&s, 1_000_000, 6, 0, Some(4), 3000);
        assert_eq!(a.responses_before_run, 2);
        assert_eq!(a.responses_after_run, 1);
        assert_eq!(a.unmatched_responses, 0);
        assert_eq!(a.matched(), 4);
        assert_eq!(bursts(&[1, 2, 10, 11, 30], 3).len(), 3);
    }
}
