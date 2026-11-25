//! The simulated heart: a scenario table and the state that plays it, one
//! call per millisecond. Pure: no register and no rsk in it, so the host
//! tests below run the same code as the board.
//!
//! Times are milliseconds since the first Job, counted from 1; 0 in a
//! "next" field means none, which makes the all-zero state the state
//! before the first Job (a Partition's state starts zeroed).

/// The rhythm of one Chamber.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Rhythm {
    /// The instant of the first intrinsic beat.
    pub first_ms: u32,
    /// The intrinsic interval; 0: the Chamber has no rhythm of its own.
    pub interval_ms: u32,
}

/// A beat that comes earlier or later than the rhythm gives.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Override {
    /// 0: the atrium, 1: the ventricle.
    pub chamber: u8,
    /// The beat of that Chamber's own rhythm, counted from 0.
    pub beat: u32,
    /// Added to the interval that ends at this beat (negative: premature).
    pub shift_ms: i32,
}

impl Override {
    /// An unused row.
    pub const NONE: Override = Override { chamber: 0, beat: u32::MAX, shift_ms: 0 };
}

/// The number of per-beat overrides a scenario holds.
pub const OVERRIDES: usize = 4;

/// A beat outside every rhythm: it comes at its instant and restarts
/// nothing. A ventricular one between a conducted beat and the next atrial
/// beat is a PVC by the PG's definition.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Extra {
    /// 0: the atrium, 1: the ventricle.
    pub chamber: u8,
    /// Its instant; 0: an unused row.
    pub at_ms: u32,
}

impl Extra {
    /// An unused row.
    pub const NONE: Extra = Extra { chamber: 0, at_ms: 0 };
}

/// The number of extra beats a scenario holds.
pub const EXTRAS: usize = 4;

/// A stretch of beats outside every rhythm (docs/pacemaker tasks 20.2 and
/// 20.3): from `from_ms` until `to_ms`, one beat of each Chamber of `mask`
/// every `interval_ms`. Like an extra beat it restarts nothing. An atrial
/// one faster than the URL is an atrial tachycardia; one of 1 ms is a
/// storm of edges on the sense line.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Burst {
    /// Bit 0: the atrium; bit 1: the ventricle.
    pub mask: u8,
    pub from_ms: u32,
    pub to_ms: u32,
    /// 0: an unused row.
    pub interval_ms: u32,
}

impl Burst {
    /// An unused row.
    pub const NONE: Burst = Burst { mask: 0, from_ms: 0, to_ms: 0, interval_ms: 0 };
}

/// The number of bursts a scenario holds.
pub const BURSTS: usize = 2;

/// One scenario (PM26.3): the intrinsic rhythm of each Chamber, the
/// conduction from atrium to ventricle, the beats that deviate, and whether
/// the heart answers a Pace.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Scenario {
    pub name: &'static str,
    pub atrial: Rhythm,
    pub ventricular: Rhythm,
    /// The delay from an atrial beat to the ventricular beat it conducts;
    /// 0: no conduction.
    pub conduction_ms: u32,
    pub overrides: [Override; OVERRIDES],
    pub extras: [Extra; EXTRAS],
    pub bursts: [Burst; BURSTS],
    /// A Pace captures its Chamber: the Chamber's rhythm restarts from the
    /// Pace, an atrial Pace conducts, and a ventricular Pace takes the
    /// place of the conducted beat.
    pub responsive: bool,
}

/// The state of the simulated heart. All zero before the first Job.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Heart {
    /// Milliseconds played.
    pub ms: u32,
    /// The next intrinsic atrial beat, 0 for none.
    pub next_a: u32,
    /// The next ventricular beat of the ventricle's own rhythm, 0 for none.
    pub next_v: u32,
    /// The conducted ventricular beat that is on its way, 0 for none.
    pub conducted: u32,
    /// Beats of each Chamber's own rhythm so far (the index of the next).
    pub beats_a: u32,
    pub beats_v: u32,
}

/// The beats of one millisecond: a sense pulse goes out for each.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Beats {
    pub atrial: bool,
    pub ventricular: bool,
}

/// A time that is not the "none" value.
const fn some(t: u32) -> u32 {
    if t == 0 {
        1
    } else {
        t
    }
}

/// Whether `t` (not none) has been reached at `now`.
const fn reached(now: u32, t: u32) -> bool {
    t != 0 && now.wrapping_sub(t) < (1 << 31)
}

impl Scenario {
    /// The interval that ends at beat `beat` of `chamber`'s own rhythm.
    fn interval(&self, chamber: u8, beat: u32) -> u32 {
        let base = if chamber == 0 { self.atrial.interval_ms } else { self.ventricular.interval_ms };
        let mut shift = 0i32;
        let mut i = 0;
        while i < OVERRIDES {
            let o = self.overrides[i];
            if o.chamber == chamber && o.beat == beat {
                shift = o.shift_ms;
            }
            i += 1;
        }
        // Never shorter than 1 ms.
        let shifted = base as i64 + shift as i64;
        if shifted < 1 {
            1
        } else {
            shifted as u32
        }
    }
}

impl Heart {
    /// One millisecond. `paced_a` and `paced_v`: a Pace of that Chamber
    /// began since the last call.
    pub fn tick(&mut self, sc: &Scenario, paced_a: bool, paced_v: bool) -> Beats {
        if self.ms == 0 {
            if sc.atrial.interval_ms != 0 {
                self.next_a = some(sc.atrial.first_ms);
            }
            if sc.ventricular.interval_ms != 0 {
                self.next_v = some(sc.ventricular.first_ms);
            }
        }
        self.ms = some(self.ms.wrapping_add(1));
        let now = self.ms;
        let mut beats = Beats::default();

        if sc.responsive {
            if paced_a {
                if sc.atrial.interval_ms != 0 {
                    self.next_a = some(now.wrapping_add(sc.interval(0, self.beats_a)));
                }
                if sc.conduction_ms != 0 {
                    self.conducted = some(now.wrapping_add(sc.conduction_ms));
                }
            }
            if paced_v {
                self.conducted = 0;
                if sc.ventricular.interval_ms != 0 {
                    self.next_v = some(now.wrapping_add(sc.interval(1, self.beats_v)));
                }
            }
        }

        if reached(now, self.next_a) {
            beats.atrial = true;
            self.beats_a = self.beats_a.wrapping_add(1);
            self.next_a = some(now.wrapping_add(sc.interval(0, self.beats_a)));
            if sc.conduction_ms != 0 {
                self.conducted = some(now.wrapping_add(sc.conduction_ms));
            }
        }
        if reached(now, self.conducted) {
            // The conducted beat; the ventricle's own rhythm restarts from it.
            beats.ventricular = true;
            self.conducted = 0;
            if sc.ventricular.interval_ms != 0 {
                self.next_v = some(now.wrapping_add(sc.interval(1, self.beats_v)));
            }
        } else if reached(now, self.next_v) {
            // A beat of the ventricle's own rhythm.
            beats.ventricular = true;
            self.beats_v = self.beats_v.wrapping_add(1);
            self.next_v = some(now.wrapping_add(sc.interval(1, self.beats_v)));
        }
        let mut i = 0;
        while i < EXTRAS {
            let x = sc.extras[i];
            if x.at_ms != 0 && x.at_ms == now {
                if x.chamber == 0 {
                    beats.atrial = true;
                } else {
                    beats.ventricular = true;
                }
            }
            i += 1;
        }
        let mut i = 0;
        while i < BURSTS {
            let b = sc.bursts[i];
            if b.interval_ms != 0 && now >= b.from_ms && now < b.to_ms && (now - b.from_ms) % b.interval_ms == 0 {
                if b.mask & 1 != 0 {
                    beats.atrial = true;
                }
                if b.mask & 2 != 0 {
                    beats.ventricular = true;
                }
            }
            i += 1;
        }
        beats
    }
}
