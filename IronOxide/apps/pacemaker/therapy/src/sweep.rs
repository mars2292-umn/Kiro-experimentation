//! Task 7.2: the arming sweep of the spike (task 1.3), repeated through
//! this Partition's own hardware layer on the board, under the real Kernel
//! and with the present read as the Application reads it (the Kernel's
//! time base through the measured offset).
//!
//! One trial a Job of `control` (20 ms): the outcome of the trial before is
//! settled against what the driver reports as fired, then a new one is
//! made. Three phases, a tick (1 µs) a trial:
//!
//! - arm: a Pace asked for from 1.5 ms ahead to 0.1 ms behind the present;
//! - move, withdraw: a Pace armed 1.5 ms ahead, and, when it is from 1.2 ms
//!   ahead to 0.6 ms behind, moved 5 ms on or withdrawn.
//!
//! The record is 32 words of `control`'s state (`sweep`), read through the
//! debug probe in one access.

use crate::{board, Timing};
use pm_core::{Chamber, Micros};

/// The low 32 bits of an instant: the therapy timer's count, as the record
/// keeps it (instants have 64 bits since 2026-11-24).
fn low(m: Micros) -> u32 {
    pm_hw::count_of(m).0
}

/// The instant `d` microseconds from `m`, ahead or behind.
fn offset(m: Micros, d: i32) -> Micros {
    if d >= 0 {
        m.plus(d as u32)
    } else {
        m.minus(d.unsigned_abs())
    }
}
use pm_hw::{Armed, Board as _, Disarmed, Driver, Hw};

const WIDTH: u32 = 400;
const V: Chamber = Chamber::V;

// The record's words.
pub const PHASE: usize = 0;
pub const TRIAL: usize = 1;
const EXPECT: usize = 2;
const EXPECT_AT: usize = 3;
/// Three rows of eight from here: arm, move, withdraw.
pub const ROWS: usize = 4;
const TRIALS: usize = 0;
const OK: usize = 1;
const MOVED: usize = 2;
const TOO_LATE: usize = 3;
const LATE: usize = 4;
/// Reported armed (or delivered), and no Pace reported fired at that instant.
const MISSED: usize = 5;
/// Reported withdrawn, and a Pace fired.
const WRONG: usize = 6;
const REPORTED: usize = 7;
pub const OFFSET_SPREAD: usize = 28;
pub const OFFSET_TRIES: usize = 29;
pub const MOVED_MAX: usize = 30;
pub const DONE: usize = 31;
/// The first trial that was reported armed and not reported fired: its
/// phase and number, what was promised, and the registers as that Job found
/// them; then the trial's own figures (see `job`).
pub const MISS: usize = 32;
const LAST: usize = 40;
/// Jobs since Init. The trials begin after `WARM_UP` of them: a reset
/// through the debug probe halts the core for tens of milliseconds while
/// the timers run, and the Jobs held up meanwhile then run back to back.
const JOBS: usize = 52;
const WARM_UP: u32 = 50;

const PHASES: u32 = 3;
const DONE_MAGIC: u32 = 0x5EE9_D09E;

// What the trial before promised.
const NOTHING: u32 = 0;
const FIRED_AT: u32 = 1;
const FIRED: u32 = 2;
const NOT_FIRED: u32 = 3;

fn add(words: &mut [u32; 56], phase: u32, word: usize) {
    let i = ROWS + 8 * phase as usize + word;
    words[i] = words[i].saturating_add(1);
}

/// The instants of a phase: arm from +1,500 to -100, the others from
/// +1,200 to -600.
const fn trials(phase: u32) -> u32 {
    if phase == 0 {
        1_601
    } else {
        1_801
    }
}

pub fn job(t: &mut Timing, words: &mut [u32; 56]) {
    let Timing { driver, board, started, hw_fault, .. } = t;
    let mut hw = Driver::resume(board::Board { words: board }, *driver);
    if !*started {
        *started = true;
        *hw_fault = hw.configure().is_err();
        *driver = hw.state;
        words[OFFSET_SPREAD] = hw.board.words.spread;
        words[OFFSET_TRIES] = hw.board.words.tries;
        return;
    }
    if *hw_fault {
        return;
    }
    if words[DONE] == DONE_MAGIC {
        // The last pulse's flags, so that `pace` is not released for ever.
        let _ = hw.fired();
        *driver = hw.state;
        return;
    }
    words[JOBS] = words[JOBS].saturating_add(1);
    if words[JOBS] <= WARM_UP {
        *driver = hw.state;
        return;
    }
    let phase = words[PHASE];
    // The trial before is settled only when its pulse, if any, is over: a
    // Job that comes early (two Jobs back to back after a hold) waits.
    if words[EXPECT] == FIRED_AT && pm_hw::count_of(hw.now()).since(pm_hw::Count(words[EXPECT_AT]).plus(WIDTH + 600)) < 0 {
        *driver = hw.state;
        return;
    }
    let fired = hw.fired();
    if fired.fired[1] {
        add(words, phase, REPORTED);
    }
    let missed = match words[EXPECT] {
        FIRED_AT => !(fired.fired[1] && low(fired.at[1]) == words[EXPECT_AT]),
        FIRED => !fired.fired[1],
        _ => false,
    };
    if missed {
        add(words, phase, MISSED);
        if words[MISS] == 0 {
            // The record of the first miss: (phase + 1, trial), the promise,
            // what was reported, and the timer as it stands now.
            words[MISS] = phase + 1;
            words[MISS + 1] = words[TRIAL];
            words[MISS + 2] = words[EXPECT];
            words[MISS + 3] = words[EXPECT_AT];
            words[MISS + 4] = (fired.fired[1] as u32) | (hw.board.read(pm_hw::Reg::StartEvent(V)) << 1) | (hw.board.read(pm_hw::Reg::EndEvent(V)) << 2);
            words[MISS + 5] = hw.board.read(pm_hw::Reg::StartCompare(V));
            words[MISS + 6] = hw.board.read(pm_hw::Reg::EndCompare(V));
            words[MISS + 7] = low(hw.now());
            // And the trial's own figures, kept when it was made.
            let mut k = 0;
            while k < 6 {
                words[LAST + 6 + k] = words[LAST + k];
                k += 1;
            }
        }
    }
    match words[EXPECT] {
        NOT_FIRED => {
            if fired.fired[1] {
                add(words, phase, WRONG);
            }
        }
        _ => {}
    }
    words[EXPECT] = NOTHING;
    if words[TRIAL] >= trials(phase) {
        words[TRIAL] = 0;
        words[PHASE] = phase + 1;
        if phase + 1 >= PHASES {
            words[DONE] = DONE_MAGIC;
        }
        *driver = hw.state;
        return;
    }
    let trial = words[TRIAL];
    words[TRIAL] = trial + 1;
    add(words, phase, TRIALS);
    let mut outcome = |words: &mut [u32; 56], asked: Micros, armed: Armed, pending: Option<Micros>| match armed {
        Armed::Ok { at, moved } => {
            add(words, phase, OK);
            if moved {
                add(words, phase, MOVED);
                let by = at.elapsed_since(asked);
                if by > words[MOVED_MAX] {
                    words[MOVED_MAX] = by;
                }
            }
            words[EXPECT] = FIRED_AT;
            words[EXPECT_AT] = low(at);
        }
        Armed::TooLate => {
            add(words, phase, TOO_LATE);
            match pending {
                Some(first) => {
                    words[EXPECT] = FIRED_AT;
                    words[EXPECT_AT] = low(first);
                }
                None => words[EXPECT] = FIRED,
            }
        }
        Armed::Late { timed, .. } => {
            add(words, phase, LATE);
            words[EXPECT] = if timed { FIRED } else { NOTHING };
        }
    };
    if phase == 0 {
        let before = hw.now();
        let asked = offset(before, 1_500 - trial as i32);
        let armed = hw.arm(V, asked, WIDTH);
        // What this trial did, should it turn out a miss: the present
        // before and after, the instant asked for, the outcome, and the
        // compares as the sequence left them.
        words[LAST] = low(before);
        words[LAST + 1] = low(asked);
        words[LAST + 2] = match armed {
            Armed::Ok { moved: false, .. } => 1,
            Armed::Ok { moved: true, .. } => 2,
            Armed::TooLate => 3,
            Armed::Late { .. } => 4,
        };
        words[LAST + 3] = low(hw.now());
        words[LAST + 4] = hw.board.read(pm_hw::Reg::StartCompare(V));
        words[LAST + 5] = hw.board.read(pm_hw::Reg::EndCompare(V));
        outcome(words, asked, armed, None);
    } else {
        let first = hw.now().plus(1_500);
        if let Armed::Ok { at, moved: false } = hw.arm(V, first, WIDTH) {
            // Until the pending Pace is the trial's distance away: from
            // 1.2 ms ahead to 0.6 ms behind. The wait is by the present and
            // bounded in iterations too (PR-16).
            let when = offset(at, trial as i32 - 1_200);
            let mut i = 0u32;
            while hw.now().is_before(when) && i < 100_000 {
                i += 1;
            }
            if phase == 1 {
                let asked = hw.now().plus(5_000);
                let armed = hw.arm(V, asked, WIDTH);
                outcome(words, asked, armed, Some(at));
            } else {
                match hw.disarm(V) {
                    Disarmed::Withdrawn => {
                        add(words, phase, OK);
                        words[EXPECT] = NOT_FIRED;
                    }
                    Disarmed::TooLate => {
                        add(words, phase, TOO_LATE);
                        words[EXPECT] = FIRED_AT;
                        words[EXPECT_AT] = low(at);
                    }
                }
            }
        } else {
            // The set-up was not armed as asked: whatever it armed passes
            // before the next Job, and the instant is tried again.
            words[TRIAL] = trial;
            words[ROWS + 8 * phase as usize + TRIALS] -= 1;
        }
    }
    *driver = hw.state;
}
