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

use pm_core::{rearmed, step, Battery, Chamber, Event, Marker, Micros, Pace, State, Therapy};
use pm_hw::{Armed, Disarmed, Driver, DriverState, Hw};

/// The records the event ring holds.
pub const RING: usize = 32;

/// What a record of the ring is of.
pub mod kind {
    pub const NONE: u8 = 0;
    pub const PACE: u8 = 1;
    pub const PACE_FUSED: u8 = 2;
    pub const SENSE: u8 = 3;
    pub const SENSE_REFRACTORY: u8 = 4;
}

/// One event of the therapy (design, "Event record"; the Marker stream of
/// stage E5 is built from these).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Record {
    pub kind: u8,
    /// 0 the atrium, 1 the ventricle.
    pub chamber: u8,
    /// Always zero (a Partition static has no padding byte).
    pub reserved: [u8; 2],
    /// Its instant, in therapy microseconds.
    pub at: u32,
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

#[derive(Clone, Copy)]
pub struct Timing {
    /// `control`'s first Job has run.
    pub started: bool,
    /// The board could not be configured at Init: nothing is armed.
    pub hw_fault: bool,
    /// Always zero (a Partition static has no padding byte).
    pub reserved: [u8; 2],
    /// The step function's state; all zero is the Power-On Reset State.
    pub state: State,
    /// The driver's state.
    pub driver: DriverState,
    /// The board layer's own words.
    pub board: board::Words,
    /// The event ring: the last `RING` records, `written` in all.
    pub ring: [Record; RING],
    pub written: u32,
    pub counts: Counts,
    pub link: LinkSide,
}

impl Timing {
    /// The value at start: every field zero.
    pub const fn new() -> Timing {
        Timing {
            started: false,
            hw_fault: false,
            reserved: [0; 2],
            state: State::POWER_ON_RESET,
            driver: DriverState { armed: [false; 2], last_start: [Micros(0); 2], forced: [false; 2], counted: [0; 2], late_arms: 0, untimed: 0 },
            board: board::Words::new(),
            ring: [Record { kind: kind::NONE, chamber: 0, reserved: [0; 2], at: 0 }; RING],
            written: 0,
            counts: Counts { too_late: 0, moved: 0, late: 0, dropped: 0 },
            link: LinkSide { up: 0, quiet: 0, serial: 0, last: 0, refused: 0, battery: 0, tripped: 0 },
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

fn record(ring: &mut [Record; RING], written: &mut u32, counts: &mut Counts, r: Record) {
    ring[(*written as usize) % RING] = r;
    if *written as usize >= RING {
        counts.dropped = counts.dropped.saturating_add(1);
    }
    *written = written.wrapping_add(1);
}

/// One event: the step, then the arming it returns applied to the
/// hardware, and its Markers put in the ring.
fn event<H: Hw>(state: &mut State, ring: &mut [Record; RING], written: &mut u32, counts: &mut Counts, hw: &mut H, e: Event) {
    let params = *state.params();
    let (next, arming) = step(&params, state, e);
    *state = next;
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
        let kind = match marker {
            Some((Marker::Pace { fused: false }, _)) => kind::PACE,
            Some((Marker::Pace { fused: true }, _)) => kind::PACE_FUSED,
            Some((Marker::Sense { refractory: false }, _)) => kind::SENSE,
            Some((Marker::Sense { refractory: true }, _)) => kind::SENSE_REFRACTORY,
            _ => kind::NONE,
        };
        if let (Some((_, at)), true) = (marker, kind != kind::NONE) {
            record(ring, written, counts, Record { kind, chamber: k as u8, reserved: [0; 2], at: at.0 });
        }
        k += 1;
    }
}

impl Timing {
    /// `pace`: the Paces whose pulses have ended, the earlier first.
    fn on_pace(&mut self) {
        // Below the trip level nothing is accounted: the state is the
        // Power-On Reset State with nothing pending (`on_control`).
        if self.link.tripped != 0 {
            return;
        }
        let Timing { state, driver, board, ring, written, counts, .. } = self;
        let mut hw = Driver::resume(board::Board { words: board }, *driver);
        let fired = hw.fired();
        let ventricle_first = fired.fired[0] && fired.fired[1] && fired.at[1].is_before(fired.at[0]);
        let order = if ventricle_first { [Chamber::V, Chamber::A] } else { CHAMBERS };
        let mut k = 0;
        while k < 2 {
            let c = order[k];
            if fired.fired[c.index()] {
                event(state, ring, written, counts, &mut hw, Event::Paced { chamber: c, at: fired.at[c.index()] });
            }
            k += 1;
        }
        *driver = hw.state;
    }

    /// `sense_a`, `sense_v`: one service of the Chamber's sense line.
    fn on_sense(&mut self, c: Chamber) {
        let Timing { state, driver, board, ring, written, counts, started, hw_fault, .. } = self;
        if !*started || *hw_fault || self.link.tripped != 0 {
            return;
        }
        let mut hw = Driver::resume(board::Board { words: board }, *driver);
        if let Some(s) = hw.sensed(c) {
            event(state, ring, written, counts, &mut hw, Event::Sense { chamber: c, at: s.at, edges: s.edges });
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
    fn on_control(&mut self, inbox: &Inbox) -> [u32; pm_core::wire::STATUS_WORDS] {
        use pm_core::wire::{code, code_of, seq_of, set_of, verdict};
        if !self.started {
            self.init();
        }
        let Timing { state, driver, board, ring, written, counts, hw_fault, link, .. } = self;
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
            link.last = verdict::REFUSED as u32;
        }
        // The battery level is taken first: it decides whether anything
        // else is done.
        let mut command = None;
        let mut answered = false;
        if inbox.has_command {
            let c = code_of(&inbox.command);
            let set = set_of(&inbox.command);
            if c == code::BATTERY {
                link.battery = pm_core::wire::value_of(&inbox.command);
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
            }
        }
        let battery = pm_core::wire::battery_of(link.battery);
        let tripped = matches!(battery, Battery::Tripped);
        let mut v = verdict::ACCEPTED;
        if !*hw_fault {
            let mut hw = Driver::resume(board::Board { words: board }, *driver);
            if tripped {
                // PM7.2, PM30.3: below the trip level every function is
                // off, as a device without power is. What was armed is
                // withdrawn (a pulse already begun ends at its own end
                // compare), and the state is the Power-On Reset State with
                // empty histories, which is the State entered when the
                // level is back. Commands are declined.
                if link.tripped == 0 {
                    let mut k = 0;
                    while k < 2 {
                        if matches!(hw.disarm(CHAMBERS[k]), Disarmed::TooLate) {
                            counts.too_late = counts.too_late.saturating_add(1);
                        }
                        k += 1;
                    }
                    *state = State::POWER_ON_RESET;
                    *ring = [Record { kind: kind::NONE, chamber: 0, reserved: [0; 2], at: 0 }; RING];
                    *written = 0;
                    link.tripped = 1;
                }
                if command.is_some() {
                    v = verdict::DECLINED;
                    answered = true;
                }
            } else {
                link.tripped = 0;
                if let Some(command) = command {
                    event(state, ring, written, counts, &mut hw, Event::Command(command));
                    if code_of(&inbox.command) == code::TEMPORARY && !state.temporary_wanted {
                        v = verdict::DECLINED;
                    }
                    answered = true;
                }
                let now = hw.now();
                let magnet = hw.magnet();
                event(state, ring, written, counts, &mut hw, Event::Tick { now, activity: 0, magnet, battery, link_up: link.up != 0 });
            }
            hw.refresh(Chamber::A);
            hw.refresh(Chamber::V);
            *driver = hw.state;
        }
        if answered {
            link.serial = link.serial.wrapping_add(1);
            link.last = v as u32 + seq_of(&inbox.command) as u32 * 0x100;
        }
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
}

/// Task `control` on QEMU: as on the board, and then the script of Senses
/// is played: each Sense due is put in the board's words and its Chamber's
/// Release_Signal raised, outside the Critical_Section.
#[cfg(not(feature = "nrf52840"))]
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::control_pre(&cx)))]
pub fn control(mut cx: rsk_ctx::Control) {
    let sensed = cx.timing.lock(|t| {
        let _ = t.on_control(&Inbox::none());
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
