//! The hardware layer on a register model (docs/pacemaker task 7.1;
//! Properties P6 and P7).
//!
//! The model is the therapy timer with its compares acting on the pace
//! pins, and the two edge counters. Every register access of the driver is
//! preceded by a delay the test chooses, during which the timer runs on,
//! compares fire and edges arrive: the driver's sequences are tried with
//! the hardware moving between any two of their accesses.
//!
//! The model checks Property P6 itself, at every write of an end compare.
#![forbid(unsafe_code)]

use pm_core::{step, Arming, Battery, Chamber, Event, Marker, Micros, Pace, Params, State, NOMINAL_WORDS};
use pm_hw::{Armed, Board, Disarmed, Driver, Hw, HwFault, Reg, FAR, GUARD, LEAD, STORM_SKIP};

const CHAMBERS: [Chamber; 2] = [Chamber::A, Chamber::V];

/// A small deterministic generator, so that a failure names its seed.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Rng {
        Rng(seed.wrapping_mul(0x9E37_79B9_7F4A_7C15) | 1)
    }
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: u64) -> u64 {
        self.next() % n
    }
    fn between(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.below(hi - lo + 1)
    }
}

/// What may happen before a register access.
#[derive(Clone, Copy)]
struct Interference {
    /// Every access is preceded by 0 to `jitter` µs.
    jitter: u64,
    /// One access in `hold_one_in` is preceded by a further 0 to `hold` µs
    /// (a Kernel entry taken inside the Critical_Section). 0: never.
    hold_one_in: u64,
    hold: u64,
}

const QUIET: Interference = Interference { jitter: 0, hold_one_in: 0, hold: 0 };
/// The Kernel's measured hold (138 µs) at one access in six.
const KERNEL: Interference = Interference { jitter: 2, hold_one_in: 6, hold: 140 };
/// More than `LEAD` can cover: for the paths that must still be safe.
const HEAVY: Interference = Interference { jitter: 5, hold_one_in: 3, hold: 700 };

struct Model {
    rng: Rng,
    interference: Interference,
    /// µs since the model began; the timer's counter is `base + t`.
    t: u64,
    base: u32,
    /// What `now` adds to the counter: the error of the clock offset.
    clock_error: i32,
    start_cc: [u32; 2],
    end_cc: [u32; 2],
    start_ev: [bool; 2],
    end_ev: [bool; 2],
    sense_cc: [u32; 2],
    pin: [bool; 2],
    rose: [u64; 2],
    /// Every completed pulse: (start, width).
    pulses: [Vec<(u64, u64)>; 2],
    count: [u32; 2],
    count_cap: [u32; 2],
    count_cc: [u32; 2],
    count_ev: [bool; 2],
    /// Sense edges to come: (time, chamber).
    edges: Vec<(u64, Chamber)>,
    magnet: bool,
    accesses: u64,
    end_writes: u64,
}

impl Model {
    fn new(seed: u64, interference: Interference) -> Model {
        let mut rng = Rng::new(seed);
        // Near the counter's wrap in half the runs.
        let base = if seed % 2 == 0 { u32::MAX - 2_000_000 } else { rng.next() as u32 };
        let clock_error = rng.between(0, 2 * (GUARD as u64 - 2)) as i32 - (GUARD as i32 - 2);
        Model {
            rng,
            interference,
            t: 0,
            base,
            clock_error,
            start_cc: [base.wrapping_sub(FAR); 2],
            end_cc: [base.wrapping_sub(FAR); 2],
            start_ev: [false; 2],
            end_ev: [false; 2],
            sense_cc: [0; 2],
            pin: [false; 2],
            rose: [0; 2],
            pulses: [Vec::new(), Vec::new()],
            count: [0; 2],
            count_cap: [0; 2],
            count_cc: [0; 2],
            count_ev: [false; 2],
            edges: Vec::new(),
            magnet: false,
            accesses: 0,
            end_writes: 0,
        }
    }

    fn counter(&self) -> u32 {
        self.base.wrapping_add(self.t as u32)
    }

    fn micros(&self, t: u64) -> Micros {
        Micros(self.base.wrapping_add(t as u32))
    }

    /// The model time of an instant near the present.
    fn time_of(&self, m: Micros) -> u64 {
        (self.t as i64 + i64::from(m.since(Micros(self.counter())))) as u64
    }

    fn set_pin(&mut self, i: usize, high: bool) {
        if high && !self.pin[i] {
            self.rose[i] = self.t;
        }
        if !high && self.pin[i] {
            self.pulses[i].push((self.rose[i], self.t - self.rose[i]));
        }
        self.pin[i] = high;
    }

    /// One tick of the timer: the counter moves, a compare equal to it
    /// fires (its event flag, and its task on the pin through the PPI),
    /// and an edge due now is captured and counted.
    fn tick(&mut self) {
        self.t += 1;
        let c = self.counter();
        for i in 0..2 {
            if self.start_cc[i] == c {
                self.start_ev[i] = true;
                self.set_pin(i, true);
            }
            if self.end_cc[i] == c {
                self.end_ev[i] = true;
                self.set_pin(i, false);
            }
        }
        let t = self.t;
        let mut k = 0;
        while k < self.edges.len() {
            if self.edges[k].0 <= t {
                let i = self.edges.swap_remove(k).1.index();
                self.sense_cc[i] = c;
                self.count[i] += 1;
                if self.count[i] == self.count_cc[i] {
                    self.count_ev[i] = true;
                }
            } else {
                k += 1;
            }
        }
    }

    fn advance(&mut self, us: u64) {
        for _ in 0..us {
            self.tick();
        }
    }

    fn interfere(&mut self) {
        self.accesses += 1;
        let f = self.interference;
        let mut d = if f.jitter == 0 { 0 } else { self.rng.between(0, f.jitter) };
        if f.hold_one_in != 0 && self.rng.below(f.hold_one_in) == 0 {
            d += self.rng.between(0, f.hold);
        }
        self.advance(d);
    }

    /// Whether the start compare of Chamber `i` will fire in the near
    /// future (it is ahead of the counter, not on its far side).
    fn start_armed(&self, i: usize) -> bool {
        let ahead = self.start_cc[i].wrapping_sub(self.counter());
        ahead != 0 && ahead < (1 << 29)
    }
}

impl Board for Model {
    fn configure(&mut self) -> Result<(), HwFault> {
        Ok(())
    }

    fn read(&mut self, reg: Reg) -> u32 {
        self.interfere();
        match reg {
            Reg::StartCompare(c) => self.start_cc[c.index()],
            Reg::EndCompare(c) => self.end_cc[c.index()],
            Reg::StartEvent(c) => u32::from(self.start_ev[c.index()]),
            Reg::EndEvent(c) => u32::from(self.end_ev[c.index()]),
            Reg::SenseInstant(c) => self.sense_cc[c.index()],
            Reg::Count(c) => self.count_cap[c.index()],
            Reg::CountCompare(c) => self.count_cc[c.index()],
            Reg::CountEvent(c) => u32::from(self.count_ev[c.index()]),
            Reg::Magnet => u32::from(self.magnet),
            Reg::CountCapture(_) | Reg::PaceSet(_) | Reg::PaceClear(_) => 0,
        }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        self.interfere();
        match reg {
            Reg::StartCompare(c) => self.start_cc[c.index()] = value,
            Reg::EndCompare(c) => {
                let i = c.index();
                // Property P6: an end compare is written only while the
                // start is disarmed and its event flag is clear, which is
                // to say with no pulse in progress and none about to begin.
                assert!(!self.pin[i], "an end compare written under a pulse");
                assert!(!self.start_armed(i), "an end compare written with the start armed");
                assert!(!self.start_ev[i], "an end compare written with a Pace unaccounted");
                self.end_writes += 1;
                self.end_cc[i] = value;
            }
            Reg::StartEvent(c) => {
                assert_eq!(value, 0);
                // The flag stands for "a pulse of this Chamber may be high".
                assert!(!self.pin[c.index()], "a start flag cleared under its pulse");
                self.start_ev[c.index()] = false;
            }
            Reg::EndEvent(c) => {
                assert_eq!(value, 0);
                self.end_ev[c.index()] = false;
            }
            Reg::CountCapture(c) => self.count_cap[c.index()] = self.count[c.index()],
            Reg::CountCompare(c) => self.count_cc[c.index()] = value,
            Reg::CountEvent(c) => {
                assert_eq!(value, 0);
                self.count_ev[c.index()] = false;
            }
            Reg::PaceSet(c) => self.set_pin(c.index(), true),
            Reg::PaceClear(c) => self.set_pin(c.index(), false),
            Reg::SenseInstant(_) | Reg::Count(_) | Reg::Magnet => panic!("a write to a read-only register"),
        }
    }

    fn now(&mut self) -> Micros {
        self.interfere();
        Micros(self.counter().wrapping_add(self.clock_error as u32))
    }
}

fn driver(seed: u64, interference: Interference) -> Driver<Model> {
    Driver::new(Model::new(seed, interference))
}

/// Property P7 and the pulse itself. A Pace asked for at every distance
/// from 2 ms ahead to 0.2 ms behind the present, in steps of the timer's
/// tick near the present: `Ok` means the start is ahead of the counter when
/// `arm` returns and exactly one pulse of the width follows at that
/// instant; whatever is returned, no pulse is longer than its width.
#[test]
fn arming_swept_toward_the_present() {
    for (interference, name) in [(QUIET, "quiet"), (KERNEL, "kernel"), (HEAVY, "heavy")] {
        let mut ok = 0u64;
        let mut moved = 0u64;
        let mut late = 0u64;
        let mut untimed = 0u64;
        let mut seed = 0;
        for width in [50u32, 400, 1_900] {
            for distance in (-200i32..=2_000).filter(|d| *d < 700 || d % 13 == 0) {
                seed += 1;
                let mut d = driver(seed, interference);
                d.board.advance(1_000);
                let asked = Micros(d.board.counter()).plus(distance as u32);
                let accesses = d.board.accesses;
                let armed = d.arm(Chamber::V, asked, width);
                // The sequence is bounded: three rounds of six accesses and the forced path.
                assert!(d.board.accesses - accesses <= 32, "{name} seed {seed}: {} accesses", d.board.accesses - accesses);
                let returned = d.board.t;
                d.board.interference = QUIET;
                d.board.advance(5_000);
                let pulses = d.board.pulses[1].clone();
                assert!(!d.board.pin[1], "{name} seed {seed}: the pin is still high");
                assert!(d.board.pulses[0].is_empty());
                match armed {
                    Armed::Ok { at, moved: m } => {
                        let t = d.board.time_of(at);
                        assert!(t > returned, "{name} seed {seed}: Ok with the start not ahead (P7)");
                        assert_eq!(pulses, [(t, u64::from(width))], "{name} seed {seed}");
                        if m {
                            moved += 1;
                            assert!(at.since(asked) > 0, "{name} seed {seed}: moved to an earlier instant");
                            if name == "quiet" {
                                assert!(at.since(asked) <= (LEAD + 300) as i32, "{name} seed {seed}");
                            }
                        } else {
                            assert_eq!(at, asked);
                        }
                        ok += 1;
                    }
                    Armed::Late { timed, .. } => {
                        // The forced pulse: made by software, so its length
                        // is not the programmed width, but it is bounded.
                        late += 1;
                        untimed += u64::from(!timed);
                        assert_eq!(pulses.len(), 1, "{name} seed {seed}");
                        assert!(pulses[0].1 <= u64::from(width + LEAD) + 1_500, "{name} seed {seed}: {pulses:?}");
                    }
                    Armed::TooLate => {
                        // A round's start was delivered after all, with its own end.
                        assert_eq!(pulses.len(), 1, "{name} seed {seed}");
                        assert_eq!(pulses[0].1, u64::from(width), "{name} seed {seed}");
                    }
                }
                // The pace Job that follows reports exactly that pulse.
                let fired = d.fired();
                assert_eq!(fired.fired, [false, !pulses.is_empty() && !matches!(armed, Armed::Late { timed: false, .. })], "{name} seed {seed}");
                assert_eq!(d.fired().fired, [false, false]);
            }
        }
        assert!(ok > 1_000, "{name}: {ok}");
        if name == "quiet" {
            assert_eq!((late, untimed), (0, 0));
            assert!(moved > 100, "an instant behind the present is moved ahead: {moved}");
        }
        if name == "kernel" {
            // With the Kernel's hold the forced path is not needed.
            assert_eq!(late, 0, "{name}: {late} late arms");
        }
        let _ = (moved, untimed);
    }
}

/// Property P6 on a pending Pace. A Pace is armed, and at every distance
/// from its instant, before and after, it is moved to a later instant or
/// withdrawn: either the first Pace is delivered whole (`TooLate`) or it is
/// not delivered at all; never a pulse of another length. The model's own
/// assertions hold at every end-compare write.
#[test]
fn moving_and_withdrawing_swept_across_a_pending_pace() {
    for interference in [QUIET, KERNEL, HEAVY] {
        let mut delivered = 0u64;
        let mut stopped = 0u64;
        let mut seed = 1_000;
        for withdraw in [false, true] {
            for width in [50u32, 1_900] {
                for wait in 0u64..1_300 {
                    seed += 1;
                    let mut d = driver(seed, interference);
                    d.board.interference = QUIET;
                    d.board.advance(500);
                    let first = Micros(d.board.counter()).plus(1_000);
                    assert_eq!(d.arm(Chamber::A, first, width), Armed::Ok { at: first, moved: false });
                    let first_t = d.board.time_of(first);
                    // The Job arrives `wait` µs later: before, at or after the Pace.
                    d.board.advance(wait);
                    d.board.interference = interference;
                    let later = Micros(d.board.counter()).plus(300_000);
                    let (too_late, second) = if withdraw {
                        (d.disarm(Chamber::A) == Disarmed::TooLate, None)
                    } else {
                        match d.arm(Chamber::A, later, 400) {
                            Armed::TooLate => (true, None),
                            Armed::Ok { at, moved: false } => (false, Some(at)),
                            other => panic!("seed {seed}: {other:?}"),
                        }
                    };
                    d.board.interference = QUIET;
                    d.board.advance(4_000);
                    let pulses = d.board.pulses[0].clone();
                    if too_late {
                        delivered += 1;
                        assert_eq!(pulses, [(first_t, u64::from(width))], "seed {seed}: wait {wait}");
                        // Reported once, at its instant; then the Chamber is free again.
                        let fired = d.fired();
                        assert!(fired.fired[0] && fired.at[0] == first, "seed {seed}");
                        assert!(matches!(d.arm(Chamber::A, later, 400), Armed::Ok { .. }));
                    } else {
                        stopped += 1;
                        assert!(pulses.is_empty(), "seed {seed}: wait {wait}: {pulses:?}");
                        assert!(!d.fired().fired[0]);
                        if let Some(at) = second {
                            // The moved Pace is delivered whole at its own instant.
                            d.board.advance(300_000);
                            assert_eq!(d.board.pulses[0], [(d.board.time_of(at), 400)], "seed {seed}");
                        }
                    }
                    assert!(!d.board.pin[0]);
                }
            }
        }
        assert!(delivered > 500 && stopped > 500, "{delivered} {stopped}");
    }
}

/// A Chamber's start flag is cleared, and its Pace reported, only when its
/// pulse has ended; the other Chamber's pulse in progress is left alone.
#[test]
fn a_pace_is_reported_only_after_its_pulse() {
    let mut d = driver(7, QUIET);
    d.board.advance(100);
    let a = Micros(d.board.counter()).plus(1_000);
    let v = a.plus(1_000);
    assert!(matches!(d.arm(Chamber::A, a, 400), Armed::Ok { .. }));
    assert!(matches!(d.arm(Chamber::V, v, 1_900), Armed::Ok { .. }));
    d.board.advance(1_100);
    // The atrial pulse is high: nothing to report, and arming it is refused.
    assert_eq!(d.fired().fired, [false, false]);
    assert_eq!(d.arm(Chamber::A, a.plus(50_000), 400), Armed::TooLate);
    assert_eq!(d.disarm(Chamber::A), Disarmed::TooLate);
    d.board.advance(1_000);
    // The atrial pulse has ended; the ventricular one is high.
    assert!(d.board.pin[1]);
    let f = d.fired();
    assert_eq!((f.fired, f.at[0]), ([true, false], a));
    assert!(matches!(d.arm(Chamber::A, a.plus(50_000), 400), Armed::Ok { .. }));
    assert_eq!(d.arm(Chamber::V, v.plus(50_000), 400), Armed::TooLate);
    d.board.advance(2_000);
    let f = d.fired();
    assert_eq!((f.fired, f.at[1]), ([false, true], v));
    assert_eq!(d.board.pulses[0], [(d.board.time_of(a), 400)]);
    assert_eq!(d.board.pulses[1], [(d.board.time_of(v), 1_900)]);
}

/// A withdrawn Pace's end compare still comes past: it raises the timer's
/// interrupt and nothing is reported. A disarmed start comes round after
/// 2^32 - 2^30 µs unless the Chamber is refreshed.
#[test]
fn a_withdrawn_pace_leaves_nothing_behind_if_refreshed() {
    let mut d = driver(9, QUIET);
    let at = Micros(d.board.counter()).plus(2_000);
    assert!(matches!(d.arm(Chamber::V, at, 400), Armed::Ok { .. }));
    assert_eq!(d.disarm(Chamber::V), Disarmed::Withdrawn);
    d.board.advance(3_000);
    assert!(d.board.end_ev[1] && !d.board.start_ev[1]);
    assert_eq!(d.fired().fired, [false, false]);
    assert!(!d.board.end_ev[1]);
    assert!(d.board.pulses[1].is_empty());

    // The far side is `FAR` behind the present whatever the clock's error.
    let behind = d.board.counter().wrapping_sub(d.board.start_cc[1]);
    assert!(behind > FAR - 10_000 && behind < FAR + 10_000);
    // Refreshed, it stays that far behind; a refresh does not touch an armed Chamber.
    d.board.advance(1_000_000);
    d.refresh(Chamber::V);
    let behind = d.board.counter().wrapping_sub(d.board.start_cc[1]);
    assert!(behind > FAR - 10_000 && behind < FAR + 10_000);
    let at = Micros(d.board.counter()).plus(5_000);
    assert!(matches!(d.arm(Chamber::V, at, 400), Armed::Ok { .. }));
    d.refresh(Chamber::V);
    assert_eq!(d.board.start_cc[1], at.0);
    // Not refreshed, the model's start compare is what the counter reaches
    // after 2^32 - 2^30 ticks (stated, not run: 53.7 minutes of ticks).
    assert_eq!(0u32.wrapping_sub(FAR), 0xC000_0000);
}

/// The sense service: every edge is counted once, whatever arrives between
/// its accesses; after a service the next edge raises the interrupt, or an
/// edge is already counted that a second service reports.
#[test]
fn the_sense_service_loses_no_edge() {
    for (interference, name) in [(QUIET, "quiet"), (KERNEL, "kernel"), (HEAVY, "heavy")] {
        for seed in 1..=300 {
            let mut d = driver(seed + 5_000, interference);
            // Init: the first service arms the counter for the first edge.
            d.board.interference = QUIET;
            assert_eq!(d.sensed(Chamber::V), None);
            d.board.interference = interference;
            let mut r = Rng::new(seed);
            // Edges at heart-like spacings, with bursts a few µs apart.
            let mut t = 100;
            let mut made = 0u32;
            for _ in 0..40 {
                t += if r.below(4) == 0 { r.between(1, 30) } else { r.between(5_000, 400_000) };
                d.board.edges.push((t, Chamber::V));
                made += 1;
            }
            let end = t + 10_000;
            let mut reported = 0u32;
            let mut services = 0;
            let mut last_instant = None;
            while d.board.t < end {
                d.board.advance(1);
                // The Job runs some time after the interrupt.
                if d.board.count_ev[1] {
                    d.board.advance(r.between(5, 1_500));
                    if let Some(s) = d.sensed(Chamber::V) {
                        reported += s.edges;
                        services += 1;
                        // The instant is that of an edge at or after the last reported one.
                        if let Some(prev) = last_instant {
                            assert!(s.at.since(prev) >= 0, "{name} seed {seed}");
                        }
                        last_instant = Some(s.at);
                    }
                    // Armed for an edge to come (the next one unless a storm
                    // was met), or that edge has come and raised the interrupt.
                    let ahead = d.board.count_cc[1].wrapping_sub(d.board.count[1]);
                    assert!((1..=STORM_SKIP).contains(&ahead) || d.board.count_ev[1], "{name} seed {seed}: compare {ahead} ahead");
                    // The flag left set stands for an edge that reached the
                    // compare after the service cleared it, never a stale one.
                    assert!(!d.board.count_ev[1] || (ahead as i32) <= 0, "{name} seed {seed}: the event flag left set with the compare {ahead} ahead");
                }
            }
            // One last service collects what the final compare had not yet raised.
            if let Some(s) = d.sensed(Chamber::V) {
                reported += s.edges;
            }
            assert_eq!(reported, made, "{name} seed {seed}");
            assert_eq!(d.state.counted[1], made);
            assert!(services >= 10, "{name} seed {seed}: {services}");
            assert_eq!(d.sensed(Chamber::A), None);
        }
    }
}

/// A storm faster than the service: the service still ends, leaves the
/// compare a bounded number of edges ahead, and counts every edge.
#[test]
fn the_sense_service_ends_under_a_storm() {
    let mut d = driver(11, Interference { jitter: 3, hold_one_in: 0, hold: 0 });
    for t in 1..=20_000u64 {
        d.board.edges.push((t, Chamber::A));
    }
    d.board.advance(50);
    let mut reported = d.sensed(Chamber::A).map_or(0, |s| s.edges);
    let mut services = 0;
    while d.board.t < 20_100 {
        let before = d.board.accesses;
        if let Some(s) = d.sensed(Chamber::A) {
            reported += s.edges;
        }
        services += 1;
        assert!(d.board.accesses - before <= 18);
        d.board.advance(200);
    }
    d.board.advance(100);
    if let Some(s) = d.sensed(Chamber::A) {
        reported += s.edges;
    }
    assert_eq!(reported, 20_000);
    assert!(services > 50);
}

#[test]
fn the_magnet_and_the_present_are_the_boards() {
    let mut d = driver(3, QUIET);
    assert_eq!(d.configure(), Ok(()));
    assert!(!d.magnet());
    d.board.magnet = true;
    assert!(d.magnet());
    d.board.advance(1_234);
    let error = d.board.clock_error;
    assert_eq!(d.now(), Micros(d.board.counter().wrapping_add(error as u32)));
}

// ---------------------------------------------------------------- the loop

/// The four Tasks of the Therapy_Partition as the design gives them, on the
/// model: `sense_a`, `sense_v`, `pace` and `control`, each a Job that runs
/// some time after its interrupt.
struct Loop {
    d: Driver<Model>,
    p: Params,
    s: State,
    /// What the step reported: Paces (instant, fused) and effective Senses.
    paced: [Vec<(u64, bool)>; 2],
    sensed: [Vec<u64>; 2],
    too_late: u64,
    moved: u64,
}

impl Loop {
    fn apply(&mut self, arming: Arming) {
        for c in CHAMBERS {
            match arming.pace(c) {
                Pace::Keep => {}
                Pace::Withdraw => {
                    self.too_late += u64::from(self.d.disarm(c) == Disarmed::TooLate);
                }
                Pace::At { at, width } => match self.d.arm(c, at, width) {
                    Armed::Ok { moved, .. } => self.moved += u64::from(moved),
                    Armed::TooLate => self.too_late += 1,
                    Armed::Late { .. } => {}
                },
            }
        }
        for (c, m) in [(Chamber::A, arming.markers.atrial), (Chamber::V, arming.markers.ventricular)] {
            match m {
                Some((Marker::Pace { fused }, at)) => self.paced[c.index()].push((self.d.board.time_of(at), fused)),
                Some((Marker::Sense { refractory: false }, at)) => self.sensed[c.index()].push(self.d.board.time_of(at)),
                _ => {}
            }
        }
    }

    fn event(&mut self, e: Event) {
        let (s, arming) = step(&self.p, &self.s, e);
        self.s = s;
        self.apply(arming);
    }

    fn sense_job(&mut self, c: Chamber) {
        if let Some(s) = self.d.sensed(c) {
            self.event(Event::Sense { chamber: c, at: s.at, edges: s.edges });
        }
    }

    fn pace_job(&mut self) {
        let f = self.d.fired();
        // The earlier Pace first.
        let mut order = [Chamber::A, Chamber::V];
        if f.fired[0] && f.fired[1] && f.at[1].is_before(f.at[0]) {
            order = [Chamber::V, Chamber::A];
        }
        for c in order {
            if f.fired[c.index()] {
                self.event(Event::Paced { chamber: c, at: f.at[c.index()] });
            }
        }
    }

    fn control_job(&mut self) {
        let now = self.d.now();
        self.event(Event::Tick { now, activity: 0, magnet: false, battery: Battery::Bol, link_up: false });
        for c in CHAMBERS {
            self.d.refresh(c);
        }
    }
}

/// One run of the Tasks on the model for `seconds`: a heart with edges
/// placed around the pending Pace, Jobs that start up to `latency` µs after
/// their interrupt, and interference inside every Job.
fn run_loop(seed: u64, mode: u32, lrl: u32, interference: Interference, latency: u64, seconds: u64) -> Loop {
    let mut words = NOMINAL_WORDS;
    words[pm_core::word::MODE] = mode;
    words[pm_core::word::LRL] = lrl;
    let mut r = Rng::new(seed ^ 0xABCD);
    words[pm_core::word::A_WIDTH] = [50, 400, 1_900][r.below(3) as usize];
    words[pm_core::word::V_WIDTH] = [50, 400, 1_900][r.below(3) as usize];
    let p = pm_core::validate(&words).expect("a set of stage E1");
    let mut l = Loop { d: driver(seed, interference), p, s: State::POWER_ON_RESET, paced: [Vec::new(), Vec::new()], sensed: [Vec::new(), Vec::new()], too_late: 0, moved: 0 };
    assert_eq!(l.d.configure(), Ok(()));
    // Init (the first Job of `control`): the first service of each sense
    // line arms its counter for the first edge.
    for c in CHAMBERS {
        assert_eq!(l.d.sensed(c), None);
    }
    let end = seconds * 1_000_000;
    let mut next_tick = 500;
    let mut next_edge = [r.between(10_000, 900_000), r.between(10_000, 900_000)];
    // When each pending Job starts: (pace, sense_a, sense_v).
    let mut due: [Option<u64>; 3] = [None; 3];
    while l.d.board.t < end {
        l.d.board.advance(1);
        let t = l.d.board.t;
        for c in CHAMBERS {
            if t >= next_edge[c.index()] {
                l.d.board.edges.push((t + 1, c));
                // The next edge: anywhere, or within 3 ms of the pending Pace.
                let pending = l.s.chamber[c.index()].pending;
                let near = l.d.board.time_of(pending.at);
                next_edge[c.index()] = if pending.armed && r.below(3) == 0 && near > t + 5_000 { near - 3_000 + r.below(6_000) } else { t + r.between(160_000, 1_300_000) };
            }
        }
        let pending = [l.d.board.end_ev[0] || l.d.board.end_ev[1], l.d.board.count_ev[0], l.d.board.count_ev[1]];
        for k in 0..3 {
            if pending[k] && due[k].is_none() {
                due[k] = Some(t + r.between(5, latency));
            }
        }
        // The Jobs by Priority: pace, then the sense Tasks, then control.
        if due[0].is_some_and(|x| t >= x) {
            due[0] = None;
            l.pace_job();
        } else if due[1].is_some_and(|x| t >= x) {
            due[1] = None;
            l.sense_job(Chamber::A);
        } else if due[2].is_some_and(|x| t >= x) {
            due[2] = None;
            l.sense_job(Chamber::V);
        } else if t >= next_tick {
            next_tick += 20_000;
            l.control_job();
        }
    }
    // Let a last pulse and its Job finish.
    l.d.board.interference = QUIET;
    l.d.board.advance(3_000);
    l.pace_job();
    l
}

/// The step function, the driver and the model together. Every pulse on a
/// pin is the programmed width, each is reported once at its instant, no
/// paced Chamber goes longer than the escape interval without an event, a
/// Chamber the Mode does not pace has no pulse, and Property P6 held at
/// every end-compare write (the model asserts it).
#[test]
fn the_tasks_on_the_model_pace_as_programmed() {
    let mut pulses = 0u64;
    let mut fusions = 0u64;
    let mut too_late = 0u64;
    let mut senses = 0u64;
    let mut seed = 0;
    for (mode, paced_chamber) in [(5u32, Some(Chamber::A)), (6, Some(Chamber::A)), (7, Some(Chamber::V)), (8, Some(Chamber::V)), (0, None)] {
        for lrl in [45u32, 60, 110, 175] {
            for (interference, latency) in [(QUIET, 300u64), (KERNEL, 1_500), (KERNEL, 2_000)] {
                seed += 1;
                let l = run_loop(seed, mode, lrl, interference, latency, 20);
                let escape = u64::from(l.p.lrl);
                for c in CHAMBERS {
                    let i = c.index();
                    let on_pin = &l.d.board.pulses[i];
                    assert!(!l.d.board.pin[i], "seed {seed}: pin high at the end");
                    if Some(c) != paced_chamber {
                        assert!(on_pin.is_empty(), "seed {seed}: a pulse in a Chamber the Mode does not pace");
                        continue;
                    }
                    // Every pulse is the programmed width, to the tick.
                    assert!(on_pin.iter().all(|(_, w)| *w == u64::from(l.p.width[i])), "seed {seed}: {:?}", on_pin.iter().find(|(_, w)| *w != u64::from(l.p.width[i])));
                    // The pulses on the pin are the Paces the step was told of.
                    let told: Vec<u64> = l.paced[i].iter().map(|x| x.0).collect();
                    let seen: Vec<u64> = on_pin.iter().map(|x| x.0).collect();
                    assert_eq!(told, seen, "seed {seed}: the Paces reported are not the pulses on the pin");
                    // PM9.1 on the pin and the effective Senses: no gap
                    // longer than the escape interval and one lead.
                    let mut events: Vec<u64> = seen.iter().chain(&l.sensed[i]).copied().collect();
                    events.sort_unstable();
                    let mut last = 500;
                    for t in events {
                        assert!(t - last <= escape + u64::from(LEAD) + 20_000, "seed {seed} mode {mode} lrl {lrl}: {} µs without an event at {t}", t - last);
                        last = t;
                    }
                    // To the end of the run (which lasts 3 ms past its 20 s).
                    assert!(20_003_000u64.saturating_sub(last) <= escape + 25_000, "seed {seed} mode {mode} lrl {lrl}: last event at {last}");
                    pulses += seen.len() as u64;
                    fusions += l.paced[i].iter().filter(|x| x.1).count() as u64;
                    senses += l.sensed[i].len() as u64;
                }
                assert_eq!(l.d.state.late_arms, 0, "seed {seed}: a Pace forced by software");
                too_late += l.too_late;
            }
        }
    }
    // The runs reached the cases the sequences are for.
    assert!(pulses > 1_000 && senses > 300, "{pulses} {senses}");
    assert!(too_late > 5, "arming met a fired Pace {too_late} times");
    assert!(fusions > 5, "{fusions} fusions");
}
