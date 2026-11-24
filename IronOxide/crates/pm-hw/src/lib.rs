//! pm-hw: the hardware layer of the PACEMAKER experiment
//! (`docs/pacemaker/design.md`, decisions PD-02, PD-03 and PD-10; task 7.1).
//!
//! The therapy Tasks see the board through [`Hw`]. [`Driver`] implements it
//! as sequences of register accesses over a [`Board`], which names the
//! registers by their role: on the nRF52840 a role is a register of TIMER3,
//! of a counter (TIMER1, TIMER2) or of GPIOTE, reached through `rsk::mmio`
//! (task 7.2); on the host it is the register model of this crate's tests,
//! which lets the timer run on, and a compare fire, between any two
//! accesses. The sequences are the same code in both.
//!
//! What the sequences have to get right (design, "Arming and the lead"):
//!
//! - a compare written with a value the counter has passed does not fire
//!   until the counter wraps, so a start is only called armed when a
//!   reading of the present taken after the write still has it ahead;
//! - an end compare written while its pulse is high, or just before an
//!   armed start fires, leaves the pin high until the new value, so an end
//!   compare is written only with the start disarmed, its event flag clear
//!   and no forced pulse in progress (Property P6).
#![no_std]
#![forbid(unsafe_code)]

// The App_Prover's library, when this source is compiled under it.
#[cfg(feature = "creusot")]
extern crate creusot_std;

use pm_core::{Chamber, Micros};

/// How far ahead a start is placed when the instant asked for cannot be
/// met: it has to cover what can delay the Job between its reading of the
/// present and its write of the start compare (a Kernel hold, and entries
/// that follow it). Proposal; task 1.3 measures it.
pub const LEAD: u32 = 500;

/// What a start compare must still be ahead of the present, read after the
/// write, to count as armed: the error of the present's reading (half the
/// clock offset's spread, "The two clocks") and the timer's own latency of
/// a few ticks. Proposal; task 1.3 measures it.
pub const GUARD: u32 = 64;

/// How far behind the present a disarmed start compare is put: far enough
/// that no error of the present's reading puts it ahead. It would fire
/// when the counter comes round to it, 2^32 - 2^30 µs (53.7 minutes)
/// later, so a Chamber that stays disarmed is refreshed ([`Hw::refresh`]).
pub const FAR: u32 = 1 << 30;

/// The rounds of the arming sequence (a constant bound, PR-16).
pub const ROUNDS: usize = 3;

/// The rounds of the sense service's exact re-arming (a constant bound).
pub const SENSE_ROUNDS: usize = 3;

/// How many edges ahead the sense compare is put when every exact round
/// lost its race against arriving edges: only a storm of edges faster than
/// a register access does that, and it reaches the compare at once.
pub const STORM_SKIP: u32 = 16;

/// A register by its role. The Board maps each to a register address.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Reg {
    /// The therapy timer's compare that starts the Chamber's pulse.
    StartCompare(Chamber),
    /// The compare that ends it.
    EndCompare(Chamber),
    /// The start compare's event flag: reads non-zero once the compare has
    /// fired; written 0 to clear.
    StartEvent(Chamber),
    /// The end compare's event flag, which raises the timer's interrupt.
    EndEvent(Chamber),
    /// The therapy timer's capture of the Chamber's latest sense edge.
    SenseInstant(Chamber),
    /// The Chamber's edge counter: written 1, captures the count.
    CountCapture(Chamber),
    /// The captured count.
    Count(Chamber),
    /// The count at which the counter raises its interrupt.
    CountCompare(Chamber),
    /// The counter's compare event flag; written 0 to clear.
    CountEvent(Chamber),
    /// Written 1: the Chamber's pace pin (and its copy) high, by software.
    PaceSet(Chamber),
    /// Written 1: the Chamber's pace pin (and its copy) low, by software.
    PaceClear(Chamber),
    /// Reads non-zero while the magnet is applied.
    Magnet,
}

/// The board's fault at Init.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum HwFault {
    /// The offset between the Kernel time base and the therapy timer could
    /// not be measured within its spread.
    ClockOffset,
}

/// The registers and the clock of a board.
pub trait Board {
    /// Init: pins, channels, timers started, the clock offset measured.
    fn configure(&mut self) -> Result<(), HwFault>;
    fn read(&mut self, reg: Reg) -> u32;
    fn write(&mut self, reg: Reg, value: u32);
    /// The present, in therapy microseconds.
    fn now(&mut self) -> Micros;
}

/// A borrowed board is a board: the Tasks keep the board's own words in
/// the Resource and lend them to a driver for one Critical_Section.
impl<B: Board + ?Sized> Board for &mut B {
    #[inline(always)]
    fn configure(&mut self) -> Result<(), HwFault> {
        (**self).configure()
    }

    #[inline(always)]
    fn read(&mut self, reg: Reg) -> u32 {
        (**self).read(reg)
    }

    #[inline(always)]
    fn write(&mut self, reg: Reg, value: u32) {
        (**self).write(reg, value)
    }

    #[inline(always)]
    fn now(&mut self) -> Micros {
        (**self).now()
    }
}

/// The outcome of arming a Pace.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Armed {
    /// The start compare is armed at `at`, ahead of the present, with its
    /// end compare at `at` plus the width. `moved`: `at` is later than the
    /// instant asked for, by no more than [`LEAD`] past the present.
    Ok { at: Micros, moved: bool },
    /// A Pace of the Chamber has fired and is not yet reported: nothing
    /// was written. The `pace` Job arms from the state.
    TooLate,
    /// No start compare could be armed in [`ROUNDS`] rounds: the pin was
    /// set by software at about `at`. `timed`: its end compare ends the
    /// pulse; otherwise the pin was cleared by software as well. Either
    /// way the pulse's length is software's, not the programmed width.
    Late { at: Micros, timed: bool },
}

/// The outcome of withdrawing a Pace.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Disarmed {
    /// The start compare is disarmed; no Pace of the Chamber will fire.
    Withdrawn,
    /// The Pace had fired already (design PD-04).
    TooLate,
}

/// The Paces whose pulses have ended since the last report.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Fired {
    /// Per Chamber, whether a Pace is reported.
    pub fired: [bool; 2],
    /// Its instant, when it is.
    pub at: [Micros; 2],
}

/// One service of a Chamber's sense line.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Sensed {
    /// The instant of the latest edge.
    pub at: Micros,
    /// The edges since the last service.
    pub edges: u32,
}

/// What the therapy Tasks need of the board.
pub trait Hw {
    /// Init: pins, channels, timers started, the clock offset measured.
    fn configure(&mut self) -> Result<(), HwFault>;
    /// The present, in therapy microseconds.
    fn now(&mut self) -> Micros;
    /// Arm a Pace of `chamber`: high at `start`, low `width` µs later.
    fn arm(&mut self, chamber: Chamber, start: Micros, width: u32) -> Armed;
    /// Withdraw the armed Pace of `chamber`, if its start has not passed.
    fn disarm(&mut self, chamber: Chamber) -> Disarmed;
    /// Keep a Chamber that has no Pace armed disarmed: called every period
    /// of `control`, so that a start compare left behind the counter never
    /// comes round. Does nothing to a Chamber with a Pace armed or fired.
    fn refresh(&mut self, chamber: Chamber);
    /// The Paces whose pulses have ended since the last call (`pace`).
    fn fired(&mut self) -> Fired;
    /// The edges of `chamber` since the last call and the latest one's
    /// instant. The call also arms the Chamber's counter for the next edge,
    /// so Init makes one call per Chamber before any edge can be awaited.
    fn sensed(&mut self, chamber: Chamber) -> Option<Sensed>;
    /// The magnet pin.
    fn magnet(&mut self) -> bool;
}

/// What the driver keeps between calls. All zero at start (a Partition's
/// statics are): nothing armed, nothing forced, no edge counted.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct DriverState {
    /// Per Chamber: a start compare was armed and its Pace is not yet
    /// reported or withdrawn.
    pub armed: [bool; 2],
    /// The instant last written to the start compare, or of a forced pulse.
    pub last_start: [Micros; 2],
    /// A pulse set by software is high until its end compare (no start
    /// event flag stands for it).
    pub forced: [bool; 2],
    /// The count of the Chamber's edge counter at the last service.
    pub counted: [u32; 2],
    /// Pulses set by software because no start compare could be armed.
    pub late_arms: u32,
    /// Of those, pulses whose end was also made by software.
    pub untimed: u32,
}

/// The witness that a Chamber's start compare has just been put on the far
/// side of the counter and its event flag then read clear: what Property P6
/// asks before an end compare is written. Only
/// [`Driver::disarm_and_check`] makes one, it cannot be copied, and
/// [`Driver::write_end`], the only writer of an end compare, consumes it:
/// the order of P6 is checked by the compiler (task 12.4).
struct Free(Chamber);

/// [`Hw`] over a [`Board`].
pub struct Driver<B: Board> {
    pub board: B,
    pub state: DriverState,
    /// The lead and the guard in force: [`LEAD`] and [`GUARD`], unless a
    /// measurement sets others (the spike of task 1.3 sweeps them).
    pub lead: u32,
    pub guard: u32,
}

impl<B: Board> Driver<B> {
    #[inline(always)]
    pub fn new(board: B) -> Driver<B> {
        Driver { board, state: DriverState { armed: [false; 2], last_start: [Micros(0); 2], forced: [false; 2], counted: [0; 2], late_arms: 0, untimed: 0 }, lead: LEAD, guard: GUARD }
    }

    /// A driver that goes on from `state`, with the lead and the guard of
    /// the design: for one Critical_Section, after which the caller keeps
    /// `self.state` again.
    #[inline(always)]
    pub fn resume(board: B, state: DriverState) -> Driver<B> {
        Driver { board, state, lead: LEAD, guard: GUARD }
    }

    /// Steps 1 and 2 of the sequence: the start compare to the far side of
    /// the counter, after which nothing of the Chamber can fire; then
    /// whether a Pace had fired before that (its event flag is set).
    /// Returns the witness that the Chamber is free (disarmed and nothing
    /// fired), or `None`.
    #[inline(always)]
    fn disarm_and_check(&mut self, c: Chamber) -> Option<Free> {
        let far = self.board.now().minus(FAR);
        self.board.write(Reg::StartCompare(c), far.0);
        if self.board.read(Reg::StartEvent(c)) == 0 {
            Some(Free(c))
        } else {
            None
        }
    }

    /// The one place an end compare is written (Property P6): it takes the
    /// witness of [`Driver::disarm_and_check`] and spends it, so that no
    /// end compare can be written without the start compare disarmed and
    /// its event flag read clear just before, and none twice on one check.
    /// The stale end flag is cleared first (no pulse is in progress, so a
    /// set flag is the end compare of a withdrawn Pace coming past).
    #[inline(always)]
    fn write_end(&mut self, free: Free, end: Micros) {
        let Free(c) = free;
        self.board.write(Reg::EndEvent(c), 0);
        self.board.write(Reg::EndCompare(c), end.0);
    }

    /// The count of the Chamber's edge counter.
    #[inline(always)]
    fn count(&mut self, c: Chamber) -> u32 {
        self.board.write(Reg::CountCapture(c), 1);
        self.board.read(Reg::Count(c))
    }
}

impl<B: Board> Hw for Driver<B> {
    #[inline(always)]
    fn configure(&mut self) -> Result<(), HwFault> {
        self.board.configure()
    }

    #[inline(always)]
    fn now(&mut self) -> Micros {
        self.board.now()
    }

    #[inline(always)]
    fn arm(&mut self, c: Chamber, start: Micros, width: u32) -> Armed {
        let i = c.index();
        if self.state.forced[i] {
            return Armed::TooLate;
        }
        let mut start = start;
        let mut moved = false;
        let mut round = 0;
        // Property P7: the sequence ends within `ROUNDS` rounds.
        #[cfg_attr(creusot, creusot_std::macros::variant(ROUNDS@ - round@))]
        #[cfg_attr(creusot, creusot_std::macros::invariant(round@ <= ROUNDS@))]
        while round < ROUNDS {
            let free = match self.disarm_and_check(c) {
                Some(free) => free,
                // Before this Job, or as it began; or, in a later round,
                // the start of the round before, delivered with its own
                // end in force.
                None => return Armed::TooLate,
            };
            // The start is disarmed and nothing has fired: the end may be
            // written, and then the start.
            self.write_end(free, start.plus(width));
            self.board.write(Reg::StartCompare(c), start.0);
            self.state.last_start[i] = start;
            self.state.armed[i] = true;
            // Read after the write: if the start is still ahead by the
            // guard, the write was in time and the compare will fire.
            let now = self.board.now();
            if start.since(now) >= self.guard as i32 {
                return Armed::Ok { at: start, moved };
            }
            start = now.plus(self.lead);
            moved = true;
            round += 1;
        }
        // No round was in time. The last start may fire all the same.
        let free = match self.disarm_and_check(c) {
            Some(free) => free,
            None => return Armed::TooLate,
        };
        let now = self.board.now();
        let end = now.plus(self.lead).plus(width);
        self.write_end(free, end);
        self.board.write(Reg::PaceSet(c), 1);
        self.state.last_start[i] = now;
        self.state.armed[i] = true;
        self.state.forced[i] = true;
        self.state.late_arms = self.state.late_arms.saturating_add(1);
        let timed = end.since(self.board.now()) >= self.guard as i32;
        if !timed {
            // The end compare may be behind the counter: end the pulse now.
            self.board.write(Reg::PaceClear(c), 1);
            self.state.forced[i] = false;
            self.state.armed[i] = false;
            self.state.untimed = self.state.untimed.saturating_add(1);
        }
        Armed::Late { at: now, timed }
    }

    #[inline(always)]
    fn disarm(&mut self, c: Chamber) -> Disarmed {
        let i = c.index();
        if self.state.forced[i] {
            return Disarmed::TooLate;
        }
        match self.disarm_and_check(c) {
            Some(_) => {
                self.state.armed[i] = false;
                Disarmed::Withdrawn
            }
            None => Disarmed::TooLate,
        }
    }

    #[inline(always)]
    fn refresh(&mut self, c: Chamber) {
        let i = c.index();
        if !self.state.armed[i] && !self.state.forced[i] {
            let far = self.board.now().minus(FAR);
            self.board.write(Reg::StartCompare(c), far.0);
        }
    }

    #[inline(always)]
    fn fired(&mut self) -> Fired {
        let mut out = Fired { fired: [false; 2], at: [Micros(0); 2] };
        let mut k = 0;
        #[cfg_attr(creusot, creusot_std::macros::variant(2 - k@))]
        #[cfg_attr(creusot, creusot_std::macros::invariant(k@ <= 2))]
        while k < 2 {
            let c = if k == 0 { Chamber::A } else { Chamber::V };
            // A Pace is reported, and its start flag cleared, only when its
            // pulse has ended: until then the Chamber's compares must not
            // be written, and the set start flag is what says so.
            if self.board.read(Reg::EndEvent(c)) != 0 {
                self.board.write(Reg::EndEvent(c), 0);
                let reported = if self.state.forced[k] {
                    self.state.forced[k] = false;
                    true
                } else if self.board.read(Reg::StartEvent(c)) != 0 {
                    self.board.write(Reg::StartEvent(c), 0);
                    true
                } else {
                    // The end compare of a withdrawn Pace came past.
                    false
                };
                if reported {
                    self.state.armed[k] = false;
                    out.fired[k] = true;
                    out.at[k] = self.state.last_start[k];
                }
            }
            k += 1;
        }
        out
    }

    #[inline(always)]
    fn sensed(&mut self, c: Chamber) -> Option<Sensed> {
        let i = c.index();
        let mut n = self.count(c);
        let mut round = 0;
        let mut exact = false;
        #[cfg_attr(creusot, creusot_std::macros::variant(SENSE_ROUNDS@ - round@))]
        #[cfg_attr(creusot, creusot_std::macros::invariant(round@ <= SENSE_ROUNDS@))]
        while round < SENSE_ROUNDS && !exact {
            // The flag first, then the compare: the compare still in force
            // is at or behind the count and cannot fire, so the flag stays
            // clear until an edge reaches the new compare, and no edge is
            // lost between a clear and a write.
            self.board.write(Reg::CountEvent(c), 0);
            // The next edge raises the interrupt, unless it came between
            // the capture and this write: the second capture tells.
            self.board.write(Reg::CountCompare(c), n.wrapping_add(1));
            let again = self.count(c);
            exact = again == n;
            n = again;
            round += 1;
        }
        if !exact {
            self.board.write(Reg::CountEvent(c), 0);
            self.board.write(Reg::CountCompare(c), n.wrapping_add(STORM_SKIP));
        }
        let at = Micros(self.board.read(Reg::SenseInstant(c)));
        let edges = n.wrapping_sub(self.state.counted[i]);
        self.state.counted[i] = n;
        if edges == 0 {
            None
        } else {
            Some(Sensed { at, edges })
        }
    }

    #[inline(always)]
    fn magnet(&mut self) -> bool {
        self.board.read(Reg::Magnet) != 0
    }
}
