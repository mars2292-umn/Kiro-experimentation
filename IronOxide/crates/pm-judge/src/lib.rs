//! The rig's judgement of the PACEMAKER Application (docs/pacemaker,
//! PM24.3, PM31.3; task 4.3).
//!
//! A logic-analyzer capture holds four signals (`hil-rig.toml`, keys
//! `pacemaker_*`): the two pace pins of the PG and the Heart_Simulator's
//! mirror of each sense line. [`decode`] turns the capture into pulses;
//! [`judge`] measures every pace interval, refractory period and pulse
//! width and judges each against its tolerance, for the parameter set the
//! PG was given; [`play`] checks that the sense lines are the scenario's.
//!
//! The judge decides from the pins alone. It does not know what the PG
//! believed, so wherever the tolerance leaves two readings open (a Sense
//! within PM-PAR-01 of the end of a refractory period, a Sense within
//! PM-PAR-15 before a Pace) it accepts either and counts the case.
//!
//! Stage E1: the single-chamber Modes (Off, AOO, VOO, AAI, VVI). A Mode
//! that paces both Chambers, tracks or triggers is refused, not passed:
//! its AV delays are measured and reported, and judged from stage E2.
#![forbid(unsafe_code)]

use heart_sim_part::heart::Scenario;
use pm_core::{Chamber, Mode, Params, Response};

/// The analyzer channel of each signal.
pub const PACE_A: u8 = 0;
pub const PACE_V: u8 = 1;
pub const SENSE_A: u8 = 6;
pub const SENSE_V: u8 = 7;

/// PM-PAR-01: the tolerance of a pacing or refractory interval, µs.
pub const INTERVAL_TOLERANCE_US: f64 = 8_000.0;
/// PM-PAR-05: the tolerance of a pulse width, µs.
pub const WIDTH_TOLERANCE_US: f64 = 200.0;
/// PM-PAR-05 for the Power-On Reset and Pace-Now pulse, µs.
pub const WIDTH_TOLERANCE_TIGHT_US: f64 = 20.0;
/// PM-PAR-15: a Sense this close before a Pace need not cancel it, µs.
pub const INHIBITION_WINDOW_US: f64 = 2_000.0;

/// One high pulse of a channel: its rising edge and its width, in µs from
/// the capture's first sample.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pulse {
    pub at_us: f64,
    pub width_us: f64,
}

/// A capture reduced to the changes of its byte: `(sample, value)`, the
/// first entry at sample 0. Bit n of a value is channel Dn.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Edges {
    pub rate_hz: u64,
    pub samples: u64,
    pub changes: Vec<(u64, u8)>,
}

impl Edges {
    /// From sigrok's binary output: one byte per sample.
    pub fn from_raw(data: &[u8], rate_hz: u64) -> Edges {
        let mut changes = Vec::new();
        let mut last = None;
        for (i, b) in data.iter().enumerate() {
            if last != Some(*b) {
                changes.push((i as u64, *b));
                last = Some(*b);
            }
        }
        Edges { rate_hz, samples: data.len() as u64, changes }
    }

    /// The text form kept as a fixture: a header line `rate samples`, then
    /// one `sample value` line per change (the value in hex).
    pub fn to_text(&self) -> String {
        let mut s = format!("{} {}\n", self.rate_hz, self.samples);
        for (i, v) in &self.changes {
            s.push_str(&format!("{i} {v:02x}\n"));
        }
        s
    }

    pub fn from_text(text: &str) -> Result<Edges, String> {
        let mut lines = text.lines();
        let head: Vec<&str> = lines.next().ok_or("empty capture")?.split_whitespace().collect();
        let [rate, samples] = head[..] else { return Err("the first line is `rate samples`".into()) };
        let rate_hz = rate.parse().map_err(|_| "rate")?;
        let samples = samples.parse().map_err(|_| "samples")?;
        let mut changes = Vec::new();
        for l in lines.filter(|l| !l.trim().is_empty()) {
            let (i, v) = l.split_once(' ').ok_or_else(|| format!("line `{l}`"))?;
            changes.push((i.parse().map_err(|_| format!("line `{l}`"))?, u8::from_str_radix(v.trim(), 16).map_err(|_| format!("line `{l}`"))?));
        }
        if changes.first().map(|c| c.0) != Some(0) || changes.windows(2).any(|w| w[0].0 >= w[1].0) || changes.last().is_some_and(|c| c.0 >= samples) || rate_hz == 0 {
            return Err("the changes start at sample 0, ascend, and lie inside the capture".into());
        }
        Ok(Edges { rate_hz, samples, changes })
    }

    /// The capture's length in µs.
    pub fn length_us(&self) -> f64 {
        self.samples as f64 * 1e6 / self.rate_hz as f64
    }

    /// Every complete high pulse of `channel`. A pulse cut by the
    /// capture's beginning or end is left out.
    pub fn pulses(&self, channel: u8) -> Vec<Pulse> {
        let us = 1e6 / self.rate_hz as f64;
        let mask = 1u8 << channel;
        let mut out = Vec::new();
        let mut rise: Option<u64> = None;
        let mut seen_low = false;
        for (i, v) in &self.changes {
            if v & mask != 0 {
                if rise.is_none() && seen_low {
                    rise = Some(*i);
                }
            } else {
                if let Some(r) = rise.take() {
                    out.push(Pulse { at_us: r as f64 * us, width_us: (*i - r) as f64 * us });
                }
                seen_low = true;
            }
        }
        out
    }
}

/// The four signals of a capture.
#[derive(Clone, Debug, PartialEq)]
pub struct Decoded {
    pub length_us: f64,
    /// Indexed by `Chamber::index`.
    pub paces: [Vec<Pulse>; 2],
    pub senses: [Vec<Pulse>; 2],
}

pub fn decode(e: &Edges) -> Decoded {
    Decoded { length_us: e.length_us(), paces: [e.pulses(PACE_A), e.pulses(PACE_V)], senses: [e.pulses(SENSE_A), e.pulses(SENSE_V)] }
}

/// Something outside its tolerance.
#[derive(Clone, Debug, PartialEq)]
pub struct Violation {
    /// The acceptance criterion it is judged against.
    pub criterion: &'static str,
    pub chamber: Chamber,
    pub at_us: f64,
    pub what: String,
}

/// What was measured in one Chamber.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct ChamberReport {
    pub paces: usize,
    pub senses: usize,
    /// Senses read as inside the refractory period, as outside it, and
    /// those within PM-PAR-01 of its end, where either reading is accepted.
    pub refractory_senses: usize,
    pub effective_senses: usize,
    pub boundary_senses: usize,
    /// Paces delivered within PM-PAR-15 after a Sense that would have
    /// inhibited them (PD-04).
    pub fusions: usize,
    /// Pace intervals judged: from the event that started the interval to
    /// the Pace, µs.
    pub intervals_us: Vec<f64>,
    pub widths_us: Vec<f64>,
}

/// The judgement of one capture.
#[derive(Clone, Debug, PartialEq)]
pub struct Report {
    pub mode: Mode,
    pub escape_us: f64,
    pub chambers: [ChamberReport; 2],
    /// From each atrial event to the next ventricular event, µs: measured,
    /// and judged from stage E2.
    pub av_delays_us: Vec<f64>,
    pub violations: Vec<Violation>,
}

impl Report {
    pub fn pass(&self) -> bool {
        self.violations.is_empty()
    }
}

/// An event that may have started the escape interval in force.
#[derive(Clone, Copy, Debug)]
struct Start {
    at: f64,
    /// A start that a later Sense replaced: still good for a Pace until
    /// this instant (the inhibition window), then dropped.
    until: Option<f64>,
    /// Whether choosing it means the Pace was not inhibited by a Sense.
    fusion: bool,
}

/// Judges a capture against the parameter set the PG was given.
/// `tight_width`: the pulse is the Power-On Reset or Pace-Now one, whose
/// width is held to ±0.02 ms.
pub fn judge(d: &Decoded, p: &Params, tight_width: bool) -> Result<Report, String> {
    if !pm_core::delivered_in_e1(p.mode) {
        return Err(format!("the judge of stage E1 does not judge {:?}: a Mode that paces both Chambers, tracks, triggers or adapts its rate", p.mode));
    }
    let escape = p.lrl as f64;
    let width_tolerance = if tight_width { WIDTH_TOLERANCE_TIGHT_US } else { WIDTH_TOLERANCE_US };
    let mut report = Report { mode: p.mode, escape_us: escape, chambers: [ChamberReport::default(), ChamberReport::default()], av_delays_us: Vec::new(), violations: Vec::new() };

    for c in [Chamber::A, Chamber::V] {
        let i = c.index();
        let paces = &d.paces[i];
        let senses = &d.senses[i];
        let r = &mut report.chambers[i];
        r.paces = paces.len();
        r.senses = senses.len();
        let mut violation = |criterion, at_us, what: String| report.violations.push(Violation { criterion, chamber: c, at_us, what });

        if !p.mode.paces(c) {
            // PM5.7: only the Chambers the Mode names are paced.
            for pulse in paces {
                violation("PM5.7", pulse.at_us, format!("a Pace of {:.1} µs in a Chamber {:?} does not pace", pulse.width_us, p.mode));
            }
            continue;
        }
        // PM3.3: every pulse width.
        let width = p.width[i] as f64;
        for pulse in paces {
            r.widths_us.push(pulse.width_us);
            if (pulse.width_us - width).abs() > width_tolerance {
                violation("PM3.3", pulse.at_us, format!("pulse width {:.2} µs, programmed {width} µs, tolerance {width_tolerance} µs", pulse.width_us));
            }
        }

        // The Chamber's events in order. A Sense counts only where the
        // Mode senses the Chamber and its response is I.
        let inhibited = p.mode.senses(c) && matches!(p.mode.response, Response::Inhibited);
        let refractory = match c {
            Chamber::A => p.arp,
            Chamber::V => p.vrp,
        } as f64;
        let mut events: Vec<(f64, bool)> = paces.iter().map(|x| (x.at_us, true)).collect();
        if inhibited {
            events.extend(senses.iter().map(|x| (x.at_us, false)));
        }
        events.sort_by(|a, b| a.0.total_cmp(&b.0));

        // The candidates for the start of the interval in force. Empty
        // until the first event. What came before the capture is unknown:
        // a first Sense may be a Refractory sense of an event before it, so
        // the start is `uncertain` until a Pace, or a Sense that is outside
        // the refractory period under either reading; meanwhile a Pace is
        // not judged and the escape interval is allowed one refractory
        // period more.
        let mut starts: Vec<Start> = Vec::new();
        let mut known = false;
        let mut uncertain = false;
        for (t, is_pace) in events {
            starts.retain(|s| s.until.is_none_or(|u| t <= u));
            // PM9.1: a Pace no later than one escape interval after the
            // start. Checked at a Sense; a Pace is judged by its interval.
            if known && !is_pace {
                if let Some(latest) = starts.iter().filter(|s| s.until.is_none()).map(|s| s.at).reduce(f64::max) {
                    let allowed = escape + INTERVAL_TOLERANCE_US + if uncertain { refractory } else { 0.0 };
                    if t - latest > allowed {
                        violation("PM9.1", t, format!("no Pace for {:.1} ms after the event at {:.1} ms; the escape interval is {:.1} ms", (t - latest) / 1e3, latest / 1e3, escape / 1e3));
                    }
                }
            }
            if is_pace {
                if known && !uncertain {
                    let best = starts.iter().copied().min_by(|a, b| (t - a.at - escape).abs().total_cmp(&(t - b.at - escape).abs()));
                    match best {
                        Some(s) if (t - s.at - escape).abs() <= INTERVAL_TOLERANCE_US => {
                            r.intervals_us.push(t - s.at);
                            r.fusions += usize::from(s.fusion);
                        }
                        Some(s) => {
                            r.intervals_us.push(t - s.at);
                            let criterion = if inhibited && t - s.at < escape { "PM5.5" } else { "PM9.1" };
                            violation(criterion, t, format!("a Pace {:.2} ms after the event at {:.1} ms; the escape interval is {:.2} ms, tolerance {} ms", (t - s.at) / 1e3, s.at / 1e3, escape / 1e3, INTERVAL_TOLERANCE_US / 1e3));
                        }
                        None => {}
                    }
                }
                starts = vec![Start { at: t, until: None, fusion: false }];
                known = true;
                uncertain = false;
            } else if !known {
                // The first event is a Sense: it may be a Refractory sense
                // of an event before the capture.
                starts = vec![Start { at: t, until: None, fusion: false }];
                known = true;
                uncertain = true;
                r.boundary_senses += 1;
            } else {
                // A Sense, read against every candidate start.
                let mut next: Vec<Start> = Vec::new();
                let mut reading = (false, false);
                for s in &starts {
                    if s.until.is_some() {
                        next.push(*s);
                        continue;
                    }
                    let since = t - s.at;
                    if since < refractory - INTERVAL_TOLERANCE_US {
                        reading.0 = true;
                        next.push(*s);
                    } else if since > refractory + INTERVAL_TOLERANCE_US {
                        // Outside the refractory period: the interval
                        // starts again, and the old start stays good for
                        // the inhibition window only.
                        reading.1 = true;
                        next.push(Start { at: s.at, until: Some(t + INHIBITION_WINDOW_US), fusion: true });
                    } else {
                        // Within the tolerance of the period's end.
                        reading = (true, true);
                        next.push(*s);
                    }
                }
                if reading.1 {
                    next.push(Start { at: t, until: None, fusion: false });
                }
                match reading {
                    (true, true) => r.boundary_senses += 1,
                    (true, false) => r.refractory_senses += 1,
                    _ => {
                        r.effective_senses += 1;
                        uncertain = false;
                    }
                }
                starts = next;
            }
        }
        // The same at the capture's end.
        match starts.iter().filter(|s| s.until.is_none()).map(|s| s.at).reduce(f64::max) {
            Some(latest) => {
                let allowed = escape + INTERVAL_TOLERANCE_US + if uncertain { refractory } else { 0.0 };
                if d.length_us - latest > allowed {
                    violation("PM9.1", d.length_us, format!("no Pace for {:.1} ms after the event at {:.1} ms, to the capture's end; the escape interval is {:.1} ms", (d.length_us - latest) / 1e3, latest / 1e3, escape / 1e3));
                }
            }
            None if d.length_us > escape + INTERVAL_TOLERANCE_US => {
                violation("PM9.1", d.length_us, format!("no event in a capture of {:.1} ms; the escape interval is {:.1} ms", d.length_us / 1e3, escape / 1e3));
            }
            None => {}
        }
    }

    // AV delays, measured: from each atrial event to the next ventricular one.
    let mut atrial: Vec<f64> = d.paces[0].iter().chain(&d.senses[0]).map(|x| x.at_us).collect();
    let mut ventricular: Vec<f64> = d.paces[1].iter().chain(&d.senses[1]).map(|x| x.at_us).collect();
    atrial.sort_by(f64::total_cmp);
    ventricular.sort_by(f64::total_cmp);
    for (k, a) in atrial.iter().enumerate() {
        let next_a = atrial.get(k + 1).copied().unwrap_or(f64::INFINITY);
        if let Some(v) = ventricular.iter().find(|v| **v >= *a && **v < next_a) {
            report.av_delays_us.push(v - a);
        }
    }
    report.violations.sort_by(|a, b| a.at_us.total_cmp(&b.at_us));
    Ok(report)
}

/// The tolerance of a simulated interval: the Heart_Simulator's Job
/// period (1 ms), half a sense pulse, and 0.1 % for the two clocks.
fn played_tolerance(us: f64) -> f64 {
    1_100.0 + us * 0.001
}

/// What the sense lines show of a scenario.
#[derive(Clone, Debug, PartialEq, Default)]
pub struct Played {
    pub atrial_beats: usize,
    pub ventricular_beats: usize,
    pub atrial_intervals_us: Vec<f64>,
    pub conduction_us: Vec<f64>,
    pub mismatches: Vec<String>,
}

/// Checks the sense lines against a scenario that does not answer Paces:
/// every atrial interval is the scenario's (or one its overrides give),
/// every atrial beat is followed by its conducted beat, and no ventricular
/// beat is unaccounted beyond the scenario's own rhythm and extra beats.
pub fn play(d: &Decoded, sc: &Scenario) -> Result<Played, String> {
    if sc.responsive {
        return Err(format!("scenario `{}` answers Paces: its sense lines depend on the PG and are not checked against the table alone", sc.name));
    }
    let a: Vec<f64> = d.senses[0].iter().map(|x| x.at_us).collect();
    let v: Vec<f64> = d.senses[1].iter().map(|x| x.at_us).collect();
    let mut out = Played { atrial_beats: a.len(), ventricular_beats: v.len(), ..Played::default() };

    let base = sc.atrial.interval_ms as f64 * 1e3;
    let mut allowed = vec![base];
    allowed.extend(sc.overrides.iter().filter(|o| o.chamber == 0 && o.beat != u32::MAX).map(|o| base + o.shift_ms as f64 * 1e3));
    let extra = |chamber: u8| sc.extras.iter().filter(|x| x.at_ms != 0 && x.chamber == chamber).count();
    if sc.atrial.interval_ms == 0 && a.len() > extra(0) {
        out.mismatches.push(format!("{} atrial beats; the scenario's atrium is silent", a.len()));
    }
    let mut odd = 0;
    for w in a.windows(2) {
        let interval = w[1] - w[0];
        out.atrial_intervals_us.push(interval);
        if sc.atrial.interval_ms != 0 && !allowed.iter().any(|x| (interval - x).abs() <= played_tolerance(*x)) {
            odd += 1;
            if odd > 2 * extra(0) {
                out.mismatches.push(format!("atrial interval {:.2} ms at {:.1} ms; the scenario gives {:?} ms", interval / 1e3, w[1] / 1e3, allowed.iter().map(|x| x / 1e3).collect::<Vec<_>>()));
            }
        }
    }
    let conduction = sc.conduction_ms as f64 * 1e3;
    let mut conducted = vec![false; v.len()];
    if sc.conduction_ms != 0 {
        for at in &a {
            let due = at + conduction;
            if due + played_tolerance(conduction) > d.length_us {
                continue;
            }
            match v.iter().position(|x| (x - due).abs() <= played_tolerance(conduction)) {
                Some(k) => {
                    conducted[k] = true;
                    out.conduction_us.push(v[k] - at);
                }
                None => out.mismatches.push(format!("no ventricular beat {} ms after the atrial beat at {:.1} ms", sc.conduction_ms, at / 1e3)),
            }
        }
    }
    // Ventricular beats that no atrial beat conducted: the ventricle's own
    // rhythm and the extra beats. The first may be conducted from an atrial
    // beat before the capture.
    let own: Vec<f64> = v.iter().zip(&conducted).filter(|(_, c)| !**c).map(|(x, _)| *x).collect();
    let before_capture = usize::from(sc.conduction_ms != 0 && own.first().is_some_and(|x| *x < conduction + played_tolerance(conduction)));
    if sc.ventricular.interval_ms == 0 && own.len() > extra(1) + before_capture {
        out.mismatches.push(format!("{} ventricular beats that nothing conducted, at {:?} ms; the scenario has {} extra", own.len(), own.iter().map(|x| (x / 1e3).round()).collect::<Vec<_>>(), extra(1)));
    }
    Ok(out)
}
