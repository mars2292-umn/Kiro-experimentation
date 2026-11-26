//! The Therapy_Partition of the PACEMAKER experiment (docs/pacemaker;
//! tasks 7.2, 7.3, 8.1 to 8.3): the Tasks `pace`, `sense_a`, `sense_v` and
//! `control` of Table 2, and the board layers behind the hardware layer.
//!
//! The therapy itself is not here: `pm_core::step` decides, `pm_hw::Driver`
//! writes the compares. A Task reads the hardware, gives the step one
//! event, and applies the arming it returns, all inside one
//! Critical_Section on the Resource `timing` (design, "The Tasks' bodies").
//!
//! The Resource starts as all zeros, as every static of a Partition does
//! (PM30.4): `Timing::new()` is that value, it is the Power-On Reset State
//! with nothing armed, and `control`'s first Job is recognized by it alone
//! (`started` is false).
#![no_std]
#![forbid(unsafe_code)]

pacemaker_decl::rsk_partition_therapy!();

#[cfg(feature = "nrf52840")]
pub mod board_nrf;
#[cfg(feature = "nrf52840")]
pub use board_nrf as board;
#[cfg(not(feature = "nrf52840"))]
pub mod board_qemu;
#[cfg(not(feature = "nrf52840"))]
pub use board_qemu as board;
pub mod sets;
#[cfg(feature = "sweep")]
mod sweep;

use pm_core::record::{self, aug};
use pm_core::{escape_modifiers, rearmed, step, AtrPhase, Battery, Chamber, Event, Marker, Micros, Pace, State, Therapy};
use pm_hw::{Armed, Disarmed, Driver, DriverState, Hw};

/// The records the event ring holds.
pub const RING: usize = 32;

/// What a record of the ring is of: the Markers of `pm_core::record`.
pub use pm_core::record::kind;

/// One event of the therapy (design, "Event record"; `pm_core::record`
/// has it as the three words `control` sends to `diag`): twelve bytes.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Record {
    pub kind: u8,
    /// 0 the atrium, 1 the ventricle.
    pub chamber: u8,
    /// The modifiers of a Pace (`record::modifier`), and the augmentation
    /// Marker (`record::aug`).
    pub modifiers: u8,
    pub augmentation: u8,
    /// Its instant, in therapy microseconds (the low 32 bits).
    pub at: u32,
    /// The microseconds since the Chamber's last non-refractory event
    /// before it (PM18.6); `record::NO_INTERVAL` if there was none.
    pub since: u32,
}

impl Record {
    /// The empty record: every byte zero.
    pub const NONE: Record = Record { kind: kind::NONE, chamber: 0, modifiers: 0, augmentation: 0, at: 0, since: 0 };
}

/// What the driver's outcomes were, for the rig's report (PD-04, PD-02).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Counts {
    /// Armings and withdrawals that met a Pace already fired.
    pub too_late: u32,
    /// Paces armed later than the instant asked for.
    pub moved: u32,
    /// Pulses forced by software.
    pub late: u32,
    /// Records the ring overwrote.
    pub dropped: u32,
}

/// The Resource `timing`: everything the four Tasks share.
/// The link as `control` knows it (tasks 15.4 and 16.1). All words: the
/// Resource is all zero bytes at the start, which is "not up, nothing
/// answered".
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct LinkSide {
    /// 1 while the link is up.
    pub up: u32,
    /// `control`'s Jobs since the last message of `link`'s (saturating).
    pub quiet: u32,
    /// Verdicts given, and the last: its code and, above it, the number
    /// of the frame it answers (`pm_core::wire::verdict`).
    pub serial: u32,
    pub last: u32,
    /// Messages the validation function refused.
    pub refused: u32,
    /// The battery level the link last brought (`pm_core::wire::battery_of`;
    /// 0, the value at start, is BOL), and 1 while it is below the Power-On
    /// Reset trip level.
    pub battery: u32,
    pub tripped: u32,
    /// The activity value the link last brought, 0 to 255 (PM29.3); 0 at
    /// start. It is used only while the link is up: without the link
    /// there is no sensor, and the sensor rate returns to the LRL.
    pub activity: u32,
    /// The pacing threshold test (PM17.4; task 22.6): 0 while none runs,
    /// else the value under test (`pm_core::threshold::what`) and 1; the
    /// cardiac cycles at the present setting; and the instant of the
    /// timing Chamber's last record, by which a cycle is counted.
    pub test: u32,
    pub test_cycles: u32,
    pub test_mark: u32,
}

/// The Jobs of `control` without a message after which the link is lost
/// (PM20.2, PM-PAR-11). `link` sends a message for every poll the DCM
/// answers, one in 50 ms; three unanswered polls are 150 ms without one,
/// and the eighth Job of 20 ms without a message is the first that cannot
/// be earlier than that. So the loss is known between 160 and 180 ms
/// after the Job of `link` that heard the last frame.
pub const LINK_LOST_JOBS: u32 = 8;
/// The messages `control` takes from `program` in a Job (the Endpoint's
/// `ops`), and the Jobs after which an unchanged status is sent again.
pub const MESSAGES: usize = 4;
pub const STATUS_REFRESH: u32 = 50;
/// The Jobs of `control` from one batch for `diag` to the next: one period
/// of `history` (100 ms).
pub const EVENTS_JOBS: u32 = 5;

/// What a Job of `control` took from the Endpoint `program`.
#[derive(Clone, Copy)]
pub struct Inbox {
    /// A message came.
    pub heard: bool,
    /// The Endpoint reported that `comms` was stopped or restarted.
    pub peer_changed: bool,
    /// Messages the validation function refused.
    pub refused: u32,
    /// A command came (at most one is taken in a Job), and its message.
    pub has_command: bool,
    pub command: [u32; pm_core::wire::PROGRAM_WORDS],
}

impl Inbox {
    #[must_use]
    pub const fn none() -> Inbox {
        Inbox { heard: false, peer_changed: false, refused: 0, has_command: false, command: [0; pm_core::wire::PROGRAM_WORDS] }
    }
}

/// The validation function of the Endpoint `program` (R42.4, PM19.3,
/// PM22.3): exactly the messages of `pm_core::wire`, a parameter set
/// exactly when `pm_core::validate` accepts it.
pub fn validate_program(m: &[u32; pm_core::wire::PROGRAM_WORDS]) -> bool {
    pm_core::wire::admitted(m)
}

/// The validation function of the channel of `status` toward `control`,
/// which carries nothing: no message is one.
pub fn validate_status(_m: &[u32; pm_core::wire::STATUS_WORDS]) -> bool {
    false
}

/// The spare bytes of [`Timing`], which make its size a multiple of its
/// alignment (8, since an instant has 64 bits): a Partition static has no
/// padding byte. The board's own words differ in size between the boards.
#[cfg(feature = "nrf52840")]
pub const SPARE: usize = 2;
#[cfg(not(feature = "nrf52840"))]
pub const SPARE: usize = 6;

#[derive(Clone, Copy)]
pub struct Timing {
    /// `control`'s first Job has run.
    pub started: bool,
    /// The board could not be configured at Init: nothing is armed.
    pub hw_fault: bool,
    /// Always zero (a Partition static has no padding byte).
    pub reserved: [u8; SPARE],
    /// The step function's state; all zero is the Power-On Reset State.
    pub state: State,
    /// The driver's state.
    pub driver: DriverState,
    /// The board layer's own words.
    pub board: board::Words,
    /// The event ring: the last `RING` records, `written` in all.
    pub ring: [Record; RING],
    pub written: u32,
    /// The records handed to the Endpoint `events` or lost: the number of
    /// the oldest record still to be sent (task 22.1). `written` less this
    /// is never above `RING`.
    pub drained: u32,
    /// Always zero: the words after the ring are an even number, so that
    /// the Resource, whose alignment is 8, ends without a padding byte.
    pub spare: u32,
    /// What the records are made from beside the step's state.
    pub marks: Marks,
    pub counts: Counts,
    pub link: LinkSide,
}

impl Timing {
    /// The value at start: every field zero.
    pub const fn new() -> Timing {
        Timing {
            started: false,
            hw_fault: false,
            reserved: [0; SPARE],
            state: State::POWER_ON_RESET,
            driver: DriverState { armed: [false; 2], last_start: [Micros(0); 2], forced: [false; 2], counted: [0; 2], late_arms: 0, untimed: 0, now: Micros(0), now_count: 0 },
            board: board::Words::new(),
            ring: [Record::NONE; RING],
            written: 0,
            drained: 0,
            spare: 0,
            marks: Marks::NONE,
            counts: Counts { too_late: 0, moved: 0, late: 0, dropped: 0 },
            link: LinkSide { up: 0, quiet: 0, serial: 0, last: 0, refused: 0, battery: 0, tripped: 0, activity: 0, test: 0, test_cycles: 0, test_mark: 0 },
        }
    }
}

impl Default for Timing {
    fn default() -> Timing {
        Timing::new()
    }
}

/// The injections of task 9.8 (feature `inject`), after the fault-injection
/// suite of rsk's demonstration: placed by instant, not by Job count, so
/// that a restarted Partition (its state back to zero) does not repeat one
/// at once.
#[cfg(feature = "inject")]
mod inject {
    /// The injections' period and the length of a window, in milliseconds:
    /// one `control` period, so that one Job falls in each window.
    const EVERY_MS: u64 = 4_000;
    const WINDOW_MS: u64 = 20;

    /// Whether the time base is within the window that begins `at_ms` into
    /// each period of `EVERY_MS`.
    pub fn window(at_ms: u64) -> bool {
        let phase = rsk::now() % super::rsk_target::ms(EVERY_MS);
        phase >= super::rsk_target::ms(at_ms) && phase < super::rsk_target::ms(at_ms.saturating_add(WINDOW_MS))
    }

    /// A busy wait of `ms` milliseconds by the time base, bounded in
    /// iterations too (PR-16).
    #[allow(unused_labels)]
    pub fn spin_ms(ms: u64) -> u32 {
        let start = rsk::now();
        let mut i = 0u32;
        'spin: while rsk::now().wrapping_sub(start) < super::rsk_target::ms(ms) && i < 100_000_000 {
            i += 1;
        }
        i
    }

    /// A write into the Kernel's RAM, outside the Partition's view: a
    /// MemManage fault attributed to the Partition (R14.3).
    pub fn fault() {
        rsk::mmio::write(0x2000_0000, 1);
    }
}

/// The one injection of task 17.1 (feature `inject-once`): a fault, an
/// Overrun or a deadline miss in the Therapy_Partition, once in a run, so
/// that the pause in pacing across the restart can be measured on the pins.
///
/// What has been done is remembered where a restart of the Partition does
/// not reach: every static of the Partition is zeroed, so the mark is a
/// bit of the port's output latch for a pin nothing uses (P1.08; the pin is
/// an input, so the bit drives nothing; a reset of the board clears it).
#[cfg(feature = "inject-once")]
mod inject_once {
    /// The first letter of `PM_INJECT`: `f`ault, `o`verrun, `d`eadline.
    pub const KIND: u8 = env!("PM_INJECT").as_bytes()[0];
    /// The run's window for the injection, in milliseconds.
    pub const FROM_MS: u64 = 10_000;
    const UNTIL_MS: u64 = 35_000;
    const P1_OUT: u32 = 0x5000_0804;
    const P1_OUTSET: u32 = 0x5000_0808;
    const MARK: u32 = 1 << 8;

    pub fn mark() {
        rsk::mmio::write(P1_OUTSET, MARK);
    }

    /// Whether the injection is due: inside the window and not yet made.
    pub fn due() -> bool {
        let now = rsk::now();
        now >= super::rsk_target::ms(FROM_MS) && now < super::rsk_target::ms(UNTIL_MS) && rsk::mmio::read(P1_OUT) & MARK == 0
    }

    /// A busy wait of `half_ms` half milliseconds by the time base, bounded
    /// in iterations too (PR-16).
    #[allow(unused_labels)]
    pub fn spin_half_ms(half_ms: u64) -> u32 {
        let start = rsk::now();
        let ticks = super::rsk_target::ms(half_ms) / 2;
        let mut i = 0u32;
        'spin: while rsk::now().wrapping_sub(start) < ticks && i < 100_000_000 {
            i += 1;
        }
        i
    }

    /// A write into the Kernel's RAM: a MemManage fault of the Partition.
    pub fn fault() {
        rsk::mmio::write(0x2000_0000, 1);
    }
}

/// The first word of the record of a rig build (`Timing::rig_words`).
pub const RIG_MAGIC: u32 = 0x504d_5247;

#[cfg(feature = "rig")]
impl Timing {
    /// The counts of the run for the rig's report (tasks 9.4 and 9.5), in
    /// the words of the state the sweep's record uses: the magic word,
    /// `started`, `hw_fault`, the records written, then `too_late`, `moved`,
    /// `late` (forced pulses), `dropped`, and the driver's own late arms and
    /// untimed pulses.
    fn rig_words(&self) -> [u32; 56] {
        let mut w = [0u32; 56];
        w[0] = RIG_MAGIC;
        w[1] = self.started as u32;
        w[2] = self.hw_fault as u32;
        w[3] = self.written;
        w[4] = self.counts.too_late;
        w[5] = self.counts.moved;
        w[6] = self.counts.late;
        w[7] = self.counts.dropped;
        w[8] = self.driver.late_arms;
        w[9] = self.driver.untimed;
        w
    }
}

const CHAMBERS: [Chamber; 2] = [Chamber::A, Chamber::V];

/// A record into the ring. A ring that is full of records not yet handed
/// to `events` loses its oldest, and the loss is counted (PM18.8, PM22.4:
/// the ring never waits for `diag`).
fn record(ring: &mut [Record; RING], written: &mut u32, drained: &mut u32, counts: &mut Counts, r: Record) {
    ring[(*written as usize) % RING] = r;
    if written.wrapping_sub(*drained) as usize >= RING {
        counts.dropped = counts.dropped.saturating_add(1);
        *drained = drained.wrapping_add(1);
    }
    *written = written.wrapping_add(1);
}

/// What the records are made from beside the step's state (PM18.5,
/// PM18.6). The step accounts a Pace as delivered at its instant when an
/// event after that instant comes before the Pace's report (a tick, or a
/// Sense, in the 0.4 ms of the pulse): the report then finds the Chamber's
/// last event to be the Pace itself, and the cycle after it begun. So the
/// instant of each Chamber's last non-refractory record is kept here, and
/// the modifiers of the Pace that ends the cycle now running and of the
/// one that ended the cycle before.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Marks {
    /// The instant (the low 32 bits) of the Chamber's last non-refractory
    /// record, and 1 once there is one.
    pub last: [u32; 2],
    pub seen: [u32; 2],
    /// The modifiers of a Pace at the end of the cycle now running, and of
    /// the cycle before it.
    pub modifiers: [u32; 2],
}

impl Marks {
    /// At start: every word zero.
    pub const NONE: Marks = Marks { last: [0; 2], seen: [0; 2], modifiers: [0; 2] };
}

/// What an event's records need of the state before the step: whether an
/// atrial Sense now is faster than the URL (AT: the interval from the
/// last atrial event ATR has, or from the Chamber's last event, is
/// shorter than the URL interval); whether each Chamber's Pace was
/// accounted already; the instant of the timing Chamber's last event; and
/// whether ATR is idle, in Duration or in Fallback (0, 1, 2).
struct Before {
    fast_atrial: bool,
    accounted: [bool; 2],
    cycle_from: Micros,
    cycle_seen: bool,
    phase: u8,
}

fn phase_of(state: &State) -> u8 {
    match state.atr.phase {
        AtrPhase::Idle => 0,
        AtrPhase::Duration => 1,
        AtrPhase::Fallback => 2,
    }
}

fn before(state: &State, at: Micros) -> Before {
    let p = state.params();
    let a = state.chamber[0];
    let own = state.own();
    let atrial = if own.atr && own.mode.tracks() && state.atr.seen {
        at.elapsed_since(state.atr.last_atrial)
    } else if a.seen {
        at.elapsed_since(a.last)
    } else {
        record::NO_INTERVAL
    };
    let timing = state.chamber[p.mode.timing_chamber().index()];
    Before { fast_atrial: atrial < own.url, accounted: [a.unconfirmed, state.chamber[1].unconfirmed], cycle_from: timing.last, cycle_seen: timing.seen, phase: phase_of(state) }
}

/// After the step: if the timing Chamber's cycle began anew, the
/// modifiers of the Pace that will end it are kept, and those of the cycle
/// before beside them.
fn cycle(marks: &mut Marks, was: &Before, state: &State) {
    let p = state.params();
    let timing = state.chamber[p.mode.timing_chamber().index()];
    if timing.seen && (!was.cycle_seen || timing.last.0 != was.cycle_from.0) {
        marks.modifiers[1] = marks.modifiers[0];
        marks.modifiers[0] = escape_modifiers(p, state, timing.last) as u32;
    }
}

/// The instant of an event (a command has none: it gives no record).
fn instant(e: &Event) -> Micros {
    match *e {
        Event::Sense { at, .. } => at,
        Event::Paced { at, .. } => at,
        Event::Tick { now, .. } => now,
        Event::Command(_) => Micros(0),
    }
}

/// The augmentation Marker of an event (PM18.5): ATR's phase as it changed
/// in the step (ATR-Dur, ATR-FB, ATR-End), or the PVARP extended after a
/// PVC (PVP→).
fn augmentation(was: u8, state: &State, pvc: bool) -> u8 {
    let now = phase_of(state);
    if now != was {
        if now == 1 {
            aug::ATR_DURATION
        } else if now == 2 {
            aug::ATR_FALLBACK
        } else {
            aug::ATR_END
        }
    } else if pvc && state.pvarp_extended {
        aug::PVARP_EXTENDED
    } else {
        aug::NONE
    }
}

/// The record's Marker of a Chamber's Marker of the step.
fn kind_of(marker: Option<(Marker, Micros)>, atrium: bool, fast_atrial: bool, pvc: bool) -> u8 {
    match marker {
        Some((Marker::Pace { fused: false }, _)) => kind::PACE,
        Some((Marker::Pace { fused: true }, _)) => kind::PACE_FUSED,
        Some((Marker::Sense { refractory: false }, _)) => {
            if atrium && fast_atrial {
                kind::AT
            } else if !atrium && pvc {
                kind::PVC
            } else {
                kind::SENSE
            }
        }
        Some((Marker::Sense { refractory: true }, _)) => kind::SENSE_REFRACTORY,
        Some((Marker::Noise, _)) => kind::NOISE,
        _ => kind::NONE,
    }
}

/// One event: the step, then the arming it returns applied to the
/// hardware, and its Markers put in the ring.
fn event<H: Hw>(state: &mut State, ring: &mut [Record; RING], written: &mut u32, drained: &mut u32, marks: &mut Marks, counts: &mut Counts, hw: &mut H, e: Event) {
    let params = *state.params();
    let was = before(state, instant(&e));
    let (next, arming) = step(&params, state, e);
    *state = next;
    let pvc = matches!(arming.markers.augmentation, Some((Marker::Pvc, _)));
    let mut augmentation = augmentation(was.phase, state, pvc);
    let mut k = 0;
    while k < 2 {
        let c = CHAMBERS[k];
        match arming.pace(c) {
            Pace::Keep => {}
            Pace::Withdraw => {
                if matches!(hw.disarm(c), Disarmed::TooLate) {
                    counts.too_late = counts.too_late.saturating_add(1);
                }
            }
            Pace::At { at, width } => match hw.arm(c, at, width) {
                Armed::Ok { at: armed, moved } => {
                    if moved {
                        // The state has the Pace pending where it is armed,
                        // so that it is accounted when it has fired.
                        counts.moved = counts.moved.saturating_add(1);
                        *state = rearmed(state, c, armed);
                    }
                }
                Armed::TooLate => counts.too_late = counts.too_late.saturating_add(1),
                Armed::Late { at: forced, .. } => {
                    counts.late = counts.late.saturating_add(1);
                    *state = rearmed(state, c, forced);
                }
            },
        }
        let marker = match c {
            Chamber::A => arming.markers.atrial,
            Chamber::V => arming.markers.ventricular,
        };
        let kind = kind_of(marker, k == 0, was.fast_atrial, pvc);
        if let (Some((_, at)), true) = (marker, kind != kind::NONE) {
            // The record keeps the instant's low 32 bits: the therapy timer's count.
            let count = pm_hw::count_of(at).0;
            let paced = kind == kind::PACE || kind == kind::PACE_FUSED;
            // A Pace accounted before its report ended the cycle before the one now running.
            let modifiers = if !paced {
                0
            } else if was.accounted[k] {
                (marks.modifiers[1] % 0x100) as u8
            } else {
                (marks.modifiers[0] % 0x100) as u8
            };
            let since = if marks.seen[k] != 0 { count.wrapping_sub(marks.last[k]) } else { record::NO_INTERVAL };
            record(ring, written, drained, counts, Record { kind, chamber: k as u8, modifiers, augmentation, at: count, since });
            if kind != kind::SENSE_REFRACTORY && kind != kind::NOISE {
                marks.last[k] = count;
                marks.seen[k] = 1;
            }
            // One augmentation Marker an event (Property P9): with its first record.
            augmentation = aug::NONE;
        }
        k += 1;
    }
    cycle(marks, &was, state);
}

/// For the App_Prover (PM7.2, PM30.3): the therapy's state is off: the
/// Power-On Reset State, nothing programmed, and no Pace pending in either
/// Chamber, so the step has nothing to arm.
#[cfg(creusot)]
#[rsk::creusot_std::macros::logic(open)]
pub fn off(st: State) -> bool {
    use rsk::creusot_std::macros::pearlite;
    pearlite! { st.therapy == Therapy::PowerOnReset && !st.programmed && !st.chamber@[0].pending.armed && !st.chamber@[1].pending.armed }
}

/// For the App_Prover: below the trip level the state is off. Every Job
/// of the Partition keeps it (the contracts of `on_pace`, `on_sense` and
/// `on_control`).
#[cfg(creusot)]
#[rsk::creusot_std::macros::logic(open)]
pub fn tripped_off(t: Timing) -> bool {
    use rsk::creusot_std::macros::pearlite;
    pearlite! { t.link.tripped@ != 0 ==> off(t.state) }
}

impl Timing {
    /// `pace`: the Paces whose pulses have ended, the earlier first.
    ///
    /// Contract: below the trip level the Job changes nothing.
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*self).link.tripped@ != 0 ==> ^self == *self))]
    fn on_pace(&mut self) {
        // Below the trip level nothing is accounted: the state is the
        // Power-On Reset State with nothing pending (`on_control`).
        if self.link.tripped != 0 {
            return;
        }
        let Timing { state, driver, board, ring, written, drained, marks, counts, .. } = self;
        let mut hw = Driver::resume(board::Board { words: board }, *driver);
        let fired = hw.fired();
        let ventricle_first = fired.fired[0] && fired.fired[1] && fired.at[1].is_before(fired.at[0]);
        let order = if ventricle_first { [Chamber::V, Chamber::A] } else { CHAMBERS };
        let mut k = 0;
        while k < 2 {
            let c = order[k];
            if fired.fired[c.index()] {
                event(state, ring, written, drained, marks, counts, &mut hw, Event::Paced { chamber: c, at: fired.at[c.index()] });
            }
            k += 1;
        }
        *driver = hw.state;
    }

    /// `sense_a`, `sense_v`: one service of the Chamber's sense line.
    ///
    /// Contract: below the trip level the Job changes nothing.
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*self).link.tripped@ != 0 ==> ^self == *self))]
    fn on_sense(&mut self, c: Chamber) {
        let Timing { state, driver, board, ring, written, drained, marks, counts, started, hw_fault, .. } = self;
        if !*started || *hw_fault || self.link.tripped != 0 {
            return;
        }
        let mut hw = Driver::resume(board::Board { words: board }, *driver);
        if let Some(s) = hw.sensed(c) {
            event(state, ring, written, drained, marks, counts, &mut hw, Event::Sense { chamber: c, at: s.at, edges: s.edges });
        }
        *driver = hw.state;
    }

    /// `control`'s first Job: the board, one service of each sense line
    /// (which arms its counter for the first edge), and the build's
    /// parameter set, programmed as a set from the DCM would be.
    fn init(&mut self) {
        let Timing { state, driver, board, hw_fault, started, .. } = self;
        let mut hw = Driver::resume(board::Board { words: board }, *driver);
        if hw.configure().is_err() {
            *hw_fault = true;
        } else {
            let _ = hw.sensed(Chamber::A);
            let _ = hw.sensed(Chamber::V);
            let set = if sets::AT_INIT { pm_core::validate(&sets::SELECTED) } else { None };
            if let Some(p) = set {
                state.permanent = p;
                state.programmed = true;
                state.therapy = Therapy::Permanent;
            }
        }
        *driver = hw.state;
        *started = true;
    }

    /// `control`: what the link brought, one tick of the step, and every
    /// Chamber with no Pace armed kept disarmed. Returns the status for
    /// `link`.
    ///
    /// The link is up from a Job that had a message until `LINK_LOST_JOBS`
    /// Jobs have had none, or the Endpoint reports that `comms` was
    /// stopped or restarted (PM23.4). The battery level is the one the
    /// link last brought (BOL from the start); below the trip level
    /// nothing paces. A command is given to the step
    /// before the tick, so that what it arms is armed in this Job; the
    /// step is told of the link by the tick, so a command of the Job that
    /// first hears the link finds it down.
    ///
    /// The function is the sequence of its parts (`heard`, `taken`,
    /// `trip`, `status_of` below), each a function of its own: as one
    /// body its verification condition was at the solvers' limit after
    /// stage E4 (findings B40).
    ///
    /// Contract (2026-11-25; PM7.2, PM30.3): a Job that finds the battery
    /// below the trip level, on hardware without a fault, ends with the
    /// level marked; and once the Partition has started, the state is off
    /// whenever the level is marked ([`tripped_off`]) after the Job if it
    /// was before.
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*self).started && tripped_off(*self) ==> tripped_off(^self)))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*self).started && (*self).link.tripped@ == 0 && (^self).link.tripped@ != 0 ==> off((^self).state) && (^self).written@ == 0 && (^self).drained@ == 0))]
    fn on_control(&mut self, inbox: &Inbox) -> [u32; pm_core::wire::STATUS_WORDS] {
        use pm_core::wire::{code, code_of, seq_of, verdict};
        if !self.started {
            self.init();
        }
        let Timing { state, driver, board, ring, written, drained, marks, counts, hw_fault, link, .. } = self;
        heard(link, inbox);
        // The battery level is taken first: it decides whether anything
        // else is done.
        let (command, mut answered) = taken(link, inbox);
        let battery = pm_core::wire::battery_of(link.battery);
        let activity = if link.up != 0 && link.activity <= 255 { link.activity as u8 } else { 0 };
        let tripped = matches!(battery, Battery::Tripped);
        let mut v = verdict::ACCEPTED;
        if !*hw_fault {
            let mut hw = Driver::resume(board::Board { words: board }, *driver);
            if tripped {
                trip(state, ring, written, drained, marks, counts, link, &mut hw);
                if command.is_some() || link.test != 0 {
                    v = verdict::DECLINED;
                    answered = true;
                }
                link.test = 0;
            } else {
                link.tripped = 0;
                if let Some(command) = command {
                    event(state, ring, written, drained, marks, counts, &mut hw, Event::Command(command));
                    if code_of(&inbox.command) == code::TEMPORARY && !state.temporary_wanted {
                        v = verdict::DECLINED;
                    }
                    answered = true;
                }
                // A threshold test is begun only in the Temporary State.
                if code_of(&inbox.command) == code::THRESHOLD_START && inbox.has_command && !matches!(state.therapy, Therapy::Temporary) {
                    link.test = 0;
                    v = verdict::DECLINED;
                }
                let now = hw.now();
                let magnet = hw.magnet();
                event(state, ring, written, drained, marks, counts, &mut hw, Event::Tick { now, activity, magnet, battery, link_up: link.up != 0 });
                threshold(state, ring, written, drained, marks, counts, link, &mut hw);
            }
            hw.refresh(Chamber::A);
            hw.refresh(Chamber::V);
            *driver = hw.state;
        }
        if answered {
            link.serial = link.serial.wrapping_add(1);
            link.last = v as u32 + seq_of(&inbox.command) as u32 * 0x100;
        }
        status_of(state, link, battery)
    }
}

/// The link's side after what a Job of `control` took from the Endpoint:
/// up from a Job that had a message until `LINK_LOST_JOBS` Jobs have had
/// none or the Endpoint reports its peer changed; a refused message is
/// counted and answered.
// For `on_control`'s contract: the mark of the trip level is `trip`'s and `on_control`'s alone.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^link).tripped == (*link).tripped))]
fn heard(link: &mut LinkSide, inbox: &Inbox) {
    if inbox.peer_changed {
        link.up = 0;
        link.quiet = LINK_LOST_JOBS;
    } else if inbox.heard {
        link.up = 1;
        link.quiet = 0;
    } else {
        link.quiet = link.quiet.saturating_add(1);
        if link.quiet >= LINK_LOST_JOBS {
            link.up = 0;
        }
    }
    if inbox.refused != 0 {
        link.refused = link.refused.saturating_add(inbox.refused);
        link.serial = link.serial.wrapping_add(1);
        link.last = pm_core::wire::verdict::REFUSED as u32;
    }
}

/// The command of the Job's message, if it has one for the step, and
/// whether the message is answered already: the battery level and the
/// activity value are kept by the link's side and answered here.
// For `on_control`'s contract: the mark of the trip level is `trip`'s and `on_control`'s alone.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^link).tripped == (*link).tripped))]
fn taken(link: &mut LinkSide, inbox: &Inbox) -> (Option<pm_core::Command>, bool) {
    use pm_core::wire::{code, code_of, set_of};
    let mut command = None;
    let mut answered = false;
    if inbox.has_command {
        let c = code_of(&inbox.command);
        let set = set_of(&inbox.command);
        if c == code::BATTERY {
            link.battery = pm_core::wire::value_of(&inbox.command);
            answered = true;
        } else if c == code::ACTIVITY {
            link.activity = pm_core::wire::value_of(&inbox.command);
            answered = true;
        } else if c == code::PROGRAM {
            command = pm_core::validate(&set).map(pm_core::Command::Program);
        } else if c == code::TEMPORARY {
            command = pm_core::validate(&set).map(pm_core::Command::StartTemporary);
        } else if c == code::END_TEMPORARY {
            command = Some(pm_core::Command::EndTemporary);
        } else if c == code::PACE_NOW {
            command = Some(pm_core::Command::PaceNow);
        } else if c == code::MAGNET_RESPONSE {
            command = Some(pm_core::Command::MagnetResponse(pm_core::wire::value_of(&inbox.command) == 1));
        } else if c == code::THRESHOLD_START {
            // Whether a test can run is decided where the State is known
            // (`on_control`): here it is only asked for.
            link.test = pm_core::wire::value_of(&inbox.command).saturating_add(1);
            link.test_cycles = 0;
            answered = true;
        } else if c == code::THRESHOLD_STOP {
            // PM17.4: a Stop ends the test, and temporary pacing with it.
            link.test = 0;
            command = Some(pm_core::Command::EndTemporary);
        }
    }
    (command, answered)
}

/// PM7.2, PM30.3: below the trip level every function is off, as a
/// device without power is. What was armed is withdrawn (a pulse already
/// begun ends at its own end compare), and the state is the Power-On
/// Reset State with empty histories, which is the State entered when the
/// level is back. Done once, in the first Job below the level.
///
/// Contract (2026-11-25): the first Job below the level leaves the
/// Power-On Reset State with no Pace pending, an empty ring and the level
/// marked; a later one changes nothing.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*link).tripped@ == 0 ==> off(^state) && (^written)@ == 0 && (^drained)@ == 0 && (^link).tripped@ == 1))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((*link).tripped@ != 0 ==> ^state == *state && ^link == *link && ^written == *written && ^drained == *drained && ^ring == *ring && ^marks == *marks && ^counts == *counts))]
fn trip<H: Hw>(state: &mut State, ring: &mut [Record; RING], written: &mut u32, drained: &mut u32, marks: &mut Marks, counts: &mut Counts, link: &mut LinkSide, hw: &mut H) {
    if link.tripped == 0 {
        let mut k = 0;
        while k < 2 {
            if matches!(hw.disarm(CHAMBERS[k]), Disarmed::TooLate) {
                counts.too_late = counts.too_late.saturating_add(1);
            }
            k += 1;
        }
        *state = State::POWER_ON_RESET;
        *ring = [Record::NONE; RING];
        *written = 0;
        *drained = 0;
        *marks = Marks::NONE;
        link.tripped = 1;
    }
}

/// The pacing threshold test (PM17.4; task 22.6; `pm_core::threshold`),
/// once in a Job of `control`. It runs in the Temporary State and ends
/// with it: by a Stop, by the loss of the link, or by whatever else ends
/// temporary pacing. Every fourth cardiac cycle (a cycle is a new record
/// of the timing Chamber) the temporary set is replaced by the set with
/// the value under test one setting lower; at the lowest setting the test
/// stays there.
// For `on_control`'s contract: the mark of the trip level is `trip`'s and `on_control`'s alone.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^link).tripped == (*link).tripped))]
fn threshold<H: Hw>(state: &mut State, ring: &mut [Record; RING], written: &mut u32, drained: &mut u32, marks: &mut Marks, counts: &mut Counts, link: &mut LinkSide, hw: &mut H) {
    if link.test == 0 {
        return;
    }
    if !matches!(state.therapy, Therapy::Temporary) {
        link.test = 0;
        return;
    }
    let k = state.params().mode.timing_chamber().index();
    if marks.seen[k] != 0 && marks.last[k] != link.test_mark {
        link.test_mark = marks.last[k];
        link.test_cycles = link.test_cycles.saturating_add(1);
    }
    if link.test_cycles > pm_core::threshold::CYCLES {
        link.test_cycles = 1;
        if let Some(next) = pm_core::threshold::step_down(&state.temporary, link.test - 1) {
            event(state, ring, written, drained, marks, counts, hw, Event::Command(pm_core::Command::StepTemporary(next)));
        }
    }
}

/// The status for `link`: the State, the Mode in force, the battery
/// level, the link, and the last verdict.
fn status_of(state: &State, link: &LinkSide, battery: Battery) -> [u32; pm_core::wire::STATUS_WORDS] {
    pm_core::wire::Status {
        therapy: pm_core::wire::therapy_byte(state.therapy),
        mode: pm_core::wire::mode_byte(state.params().mode),
        battery: pm_core::wire::battery_byte(battery),
        link_up: link.up != 0,
        serial: link.serial,
        verdict: (link.last & 0xFF) as u8,
        of: ((link.last / 0x100) & 0xFF) as u8,
        refused: link.refused,
    }
    .words()
}

impl Timing {
    /// The batch for the Endpoint `events` (task 22.1): the oldest records
    /// not yet handed over, up to `record::BATCH` of them, with the number
    /// of the first, the count of records the ring has lost and the
    /// sensor's interval. The records stay in the ring until `delivered`
    /// says the Endpoint took them: a full Endpoint keeps them here.
    #[must_use]
    pub fn batch(&self) -> [u32; record::EVENTS_WORDS] {
        let mut b = [0u32; record::EVENTS_WORDS];
        let waiting = self.written.wrapping_sub(self.drained) as usize;
        let n = if waiting > record::BATCH { record::BATCH } else { waiting };
        b[record::batch::COUNT] = n as u32;
        b[record::batch::FIRST] = self.drained;
        b[record::batch::DROPPED] = self.counts.dropped;
        b[record::batch::SENSOR] = if self.state.sensor_known { self.state.sensor_interval } else { 0 };
        let mut k = 0;
        #[cfg_attr(creusot, rsk::creusot_std::macros::variant(record::BATCH@ - k@))]
        #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(k@ <= record::BATCH@))]
        #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(b@.len() == 28))]
        while k < record::BATCH {
            if k < n {
                let r = self.ring[(self.drained.wrapping_add(k as u32) as usize) % RING];
                let at = record::batch::RECORDS + record::RECORD_WORDS * k;
                b[at] = r.at;
                b[at + 1] = record::word1(r.kind, r.chamber, r.modifiers, r.augmentation);
                b[at + 2] = r.since;
            }
            k += 1;
        }
        b
    }

    /// A record into the ring, as an event puts it (for the host tests of
    /// the ring; the Tasks put their records through `event`).
    pub fn push(&mut self, r: Record) {
        record(&mut self.ring, &mut self.written, &mut self.drained, &mut self.counts, r);
    }

    /// The Endpoint took the batch of `count` records from number `first`.
    /// The ring may have lost some of them since the batch was made (the
    /// number of its oldest record moved on): it never goes back.
    pub fn delivered(&mut self, first: u32, count: u32) {
        let end = first.wrapping_add(count);
        if end.wrapping_sub(self.drained) <= count {
            self.drained = end;
        }
    }

    /// No Endpoint takes the records (the QEMU declaration has no `diag`):
    /// they are done with when `control` has run.
    pub fn discard(&mut self) {
        self.drained = self.written;
    }
}

/// The validation function of the channel of `events` toward `control`,
/// which carries nothing: no message is one.
pub fn validate_events(_m: &[u32; record::EVENTS_WORDS]) -> bool {
    false
}

/// Task `pace` (sporadic; released by the end of a pulse).
#[cfg(not(feature = "sweep"))]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::pace_pre(&cx)))]
pub fn pace(mut cx: rsk_ctx::Pace) {
    cx.timing.lock(|t| t.on_pace());
}

/// Task `sense_v` (sporadic; released by the ventricular edge counter).
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::sense_v_pre(&cx)))]
pub fn sense_v(mut cx: rsk_ctx::SenseV) {
    cx.timing.lock(|t| t.on_sense(Chamber::V));
}

/// Task `sense_a` (sporadic; released by the atrial edge counter).
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::sense_a_pre(&cx)))]
pub fn sense_a(mut cx: rsk_ctx::SenseA) {
    cx.timing.lock(|t| t.on_sense(Chamber::A));
}

/// Task `control` (periodic, 20 ms).
#[cfg(all(feature = "nrf52840", not(feature = "sweep")))]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::control_pre(&cx)))]
pub fn control(mut cx: rsk_ctx::Control) {
    // The injection build (task 17.1): the first Job of an incarnation
    // that begins inside the window is the restart the injection caused.
    #[cfg(feature = "inject-once")]
    {
        let fresh = cx.timing.lock(|t| !t.started);
        if fresh && rsk::now() >= rsk_target::ms(inject_once::FROM_MS) {
            inject_once::mark();
        }
    }
    // The link's messages, taken outside the Critical_Section: at most
    // `MESSAGES`, and no further once a command has come (the next Job
    // takes the rest).
    let mut inbox = Inbox::none();
    let mut k = 0;
    #[cfg_attr(creusot, rsk::creusot_std::macros::variant(MESSAGES@ - k@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(k@ <= MESSAGES@))]
    while k < MESSAGES {
        k += 1;
        match cx.program.recv() {
            Ok(Some(m)) => {
                inbox.heard = true;
                if pm_core::wire::code_of(&m) != pm_core::wire::code::HEARD {
                    inbox.has_command = true;
                    inbox.command = m;
                    k = MESSAGES;
                }
            }
            Ok(None) => k = MESSAGES,
            Err(rsk::RecvError::Rejected) => inbox.refused = inbox.refused.saturating_add(1),
            Err(_) => {
                inbox.peer_changed = true;
                k = MESSAGES;
            }
        }
    }
    let status = cx.timing.lock(|t| t.on_control(&inbox));
    // The event records go to `diag` (task 22.1): the oldest in the ring,
    // up to eight, when there are any, and once in `STATUS_REFRESH` Jobs
    // an empty batch with the counts and the sensor's interval. One batch
    // in `EVENTS_JOBS` Jobs at most, which is one in a period of
    // `history`: the Endpoint holds two, and the Kernel records every send
    // to a full Endpoint as an event of its own (finding B44). Outside the
    // Critical_Section; the records leave the ring when the Endpoint has
    // taken them, so a full Endpoint (or `diag` restarting) keeps them
    // here and delays nothing (PM22.4).
    let quiet = cx.events_age.get();
    let mut sent = false;
    if quiet.saturating_add(1) >= EVENTS_JOBS {
        let batch = cx.timing.lock(|t| t.batch());
        if batch[record::batch::COUNT] != 0 || quiet >= STATUS_REFRESH {
            sent = true;
            if cx.events.send(&batch).is_ok() {
                cx.timing.lock(|t| t.delivered(batch[record::batch::FIRST], batch[record::batch::COUNT]));
            }
        }
    }
    cx.events_age.with(|a| *a = if sent { 0 } else { quiet.saturating_add(1) });
    // The status goes to `link` when it has changed, when `comms` has
    // restarted, and once in `STATUS_REFRESH` Jobs. A full channel is
    // left until the next refresh; an Endpoint that reports the peer's
    // change is tried again in the next Job.
    let age = cx.status_age.get();
    if status != cx.status_sent.get() || inbox.peer_changed || age >= STATUS_REFRESH {
        match cx.status.send(&status) {
            Ok(()) => {
                cx.status_sent.with(|w| *w = status);
                cx.status_age.with(|a| *a = 0);
            }
            Err(rsk::SendError::PeerChanged) => cx.status_age.with(|a| *a = STATUS_REFRESH),
            Err(_) => cx.status_age.with(|a| *a = 0),
        }
    } else {
        cx.status_age.with(|a| *a = age.saturating_add(1));
    }
    // The injection build (task 9.8): once in every four seconds this Job
    // runs past its Budget, and two seconds later it writes outside the
    // Partition, so that the Kernel's enforcement, fault and restart
    // entries run and are measured.
    #[cfg(feature = "inject")]
    {
        if inject::window(1_000) {
            let _ = inject::spin_ms(6);
        }
        if inject::window(3_000) {
            inject::fault();
        }
    }
    // Rig builds: the counts, where the probe can read them after the run.
    #[cfg(feature = "rig")]
    {
        let words = cx.timing.lock(|t| t.rig_words());
        cx.sweep.with(|w| *w = words);
    }
    // The injection build (task 17.1), once in the run:
    // - a fault: a write outside the Partition;
    // - an Overrun: this Job runs 7 ms on a Budget of 5 ms;
    // - a deadline miss: every Job of the window holds the Resource
    //   `timing` for 4 ms, inside its Budget (4.5 ms was not: the Job's
    //   own work with it passed 5 ms), until a pulse ends inside
    //   such a hold: `pace` is then blocked past its deadline of 2 ms.
    //   (3.5 ms until 2026-11-25: a pulse must end in the hold's first
    //   part, which was 1.5 ms long, and at 65 ppm the pulses fall on
    //   thirteen phases of the 20 ms period, 1.538 ms apart: a run could
    //   pass between them, and one on the stage E4 code did.)
    #[cfg(feature = "inject-once")]
    if inject_once::due() {
        if inject_once::KIND == b'f' {
            inject_once::mark();
            inject_once::fault();
        } else if inject_once::KIND == b'o' {
            inject_once::mark();
            let _ = inject_once::spin_half_ms(14);
        } else {
            cx.timing.lock(|_| {
                let _ = inject_once::spin_half_ms(8);
            });
        }
    }
}

/// Task `control` on QEMU: as on the board, and then the script of Senses
/// is played: each Sense due is put in the board's words and its Chamber's
/// Release_Signal raised, outside the Critical_Section.
#[cfg(not(feature = "nrf52840"))]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::control_pre(&cx)))]
pub fn control(mut cx: rsk_ctx::Control) {
    let sensed = cx.timing.lock(|t| {
        let _ = t.on_control(&Inbox::none());
        t.discard();
        board::Board { words: &mut t.board }.play()
    });
    if sensed & 1 != 0 {
        let _ = cx.sensed_a.raise();
    }
    if sensed & 2 != 0 {
        let _ = cx.sensed_v.raise();
    }
}

/// The arming sweep of task 7.2 in place of the therapy.
#[cfg(feature = "sweep")]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::control_pre(&cx)))]
pub fn control(mut cx: rsk_ctx::Control) {
    let mut words = cx.sweep.get();
    cx.timing.lock(|t| sweep::job(t, &mut words));
    cx.sweep.with(|w| *w = words);
}

/// In the sweep `pace` leaves everything to `control`.
#[cfg(feature = "sweep")]
pub fn pace(_cx: rsk_ctx::Pace) {}
