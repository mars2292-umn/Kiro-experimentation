//! Task 1.3 (docs/pacemaker/tasks.md): the compare write order of "Arming
//! and the lead" on the board, with an interrupt of the Kernel's measured
//! hold taken between its steps.
//!
//! The sequence is the Application's own: `pm_hw::Driver` over the
//! nRF52840's registers. For each configuration (operation, lead, guard,
//! hold) the instant asked for is swept toward and past the present in
//! steps of the timer's tick, and every outcome is checked on the chip: a
//! Pace reported armed must fire at its instant, a Pace reported withdrawn
//! must not. TIMER4's interrupt, at pseudo-random spacings of 150 to 900 µs
//! beyond the hold, spins for the hold and so lands between any two
//! register accesses of the sequence.
//!
//! The counts are written to RAM at `RESULTS` and read through the debug
//! port; the pulse widths are the logic analyzer's to check (`lead_check.py`).

use core::ptr::{read_volatile, write_volatile};
use pm_core::{Chamber, Micros};
use pm_hw::{Armed, Board, Disarmed, Driver, Hw, HwFault, Reg};

const GPIOTE: u32 = 0x4000_6000;
const TIMER3: u32 = 0x4001_A000;
const TIMER4: u32 = 0x4001_B000;
const PPI: u32 = 0x4001_F000;
const NVIC_ISER0: u32 = 0xE000_E100;

/// Where the counts are kept: the spike has no statics.
pub const RESULTS: u32 = 0x2000_1000;
/// The interrupt's own words: its generator's state, its hold, its count.
const IRQ_SEED: u32 = 0x2000_0F00;
const IRQ_HOLD: u32 = 0x2000_0F04;
const IRQ_COUNT: u32 = 0x2000_0F08;

const MAGIC_RUNNING: u32 = 0x1D3A_0001;
const MAGIC_DONE: u32 = 0x1D3A_D09E;
/// Words per configuration.
const ROW: u32 = 16;

const WIDTH: u32 = 400;

fn w(addr: u32, value: u32) {
    // SAFETY: a register of the nRF52840's peripherals, or a word of the
    // RAM this program alone uses.
    unsafe { write_volatile(addr as *mut u32, value) }
}

fn r(addr: u32) -> u32 {
    // SAFETY: as `w`.
    unsafe { read_volatile(addr as *const u32) }
}

/// The board's registers by role. The present is TIMER3's count through
/// capture register 2 (in the Application that register holds the atrial
/// sense instant, and the present comes from the Kernel's time base).
struct Nrf;

fn index(c: Chamber) -> u32 {
    c.index() as u32
}

impl Board for Nrf {
    fn configure(&mut self) -> Result<(), HwFault> {
        Ok(())
    }

    fn read(&mut self, reg: Reg) -> u32 {
        match reg {
            Reg::StartEvent(c) => r(TIMER3 + 0x140 + 4 * index(c)),
            Reg::EndEvent(c) => r(TIMER3 + 0x140 + 4 * (4 + index(c))),
            Reg::StartCompare(c) => r(TIMER3 + 0x540 + 4 * index(c)),
            Reg::EndCompare(c) => r(TIMER3 + 0x540 + 4 * (4 + index(c))),
            _ => 0,
        }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        match reg {
            Reg::StartCompare(c) => w(TIMER3 + 0x540 + 4 * index(c), value),
            Reg::EndCompare(c) => w(TIMER3 + 0x540 + 4 * (4 + index(c)), value),
            Reg::StartEvent(c) => w(TIMER3 + 0x140 + 4 * index(c), value),
            Reg::EndEvent(c) => w(TIMER3 + 0x140 + 4 * (4 + index(c)), value),
            Reg::PaceSet(c) => w(GPIOTE + 0x030 + 4 * index(c), value),
            Reg::PaceClear(c) => w(GPIOTE + 0x060 + 4 * index(c), value),
            _ => {}
        }
    }

    fn now(&mut self) -> Micros {
        w(TIMER3 + 0x048, 1);
        Micros(r(TIMER3 + 0x548))
    }
}

/// TIMER4's interrupt: the stand-in for a Kernel entry. Spins for the hold
/// (by its own counter, which its compare has just cleared), then sets the
/// next one 150 to 900 µs after the end of this.
pub extern "C" fn timer4() {
    w(TIMER4 + 0x140, 0);
    let hold = r(IRQ_HOLD);
    loop {
        w(TIMER4 + 0x044, 1);
        if r(TIMER4 + 0x544) >= hold {
            break;
        }
    }
    let mut x = r(IRQ_SEED);
    x ^= x << 13;
    x ^= x >> 17;
    x ^= x << 5;
    w(IRQ_SEED, x);
    w(TIMER4 + 0x540, hold + 150 + x % 751);
    w(IRQ_COUNT, r(IRQ_COUNT).wrapping_add(1));
}

fn interference(hold: u32) {
    w(TIMER4 + 0x308, 1 << 16);
    w(TIMER4 + 0x004, 1);
    w(TIMER4 + 0x140, 0);
    if hold != 0 {
        w(IRQ_HOLD, hold);
        w(TIMER4 + 0x00C, 1);
        w(TIMER4 + 0x540, hold + 300);
        w(TIMER4 + 0x304, 1 << 16);
        w(TIMER4 + 0x000, 1);
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Op {
    /// A Pace asked for `d` µs from the present, nothing pending.
    Arm = 0,
    /// A Pace pending `d` µs from the present is moved 5 ms on.
    Move = 1,
    /// A Pace pending `d` µs from the present is withdrawn.
    Withdraw = 2,
}

#[derive(Clone, Copy)]
struct Config {
    op: Op,
    lead: u32,
    guard: u32,
    hold: u32,
}

const fn cfg(op: Op, lead: u32, guard: u32, hold: u32) -> Config {
    Config { op, lead, guard, hold }
}

/// The Kernel's hold in the hour-long stress record is 138 µs; its longest
/// run of entries back to back, 406 µs.
const CONFIGS: [Config; 22] = [
    // The lead, under the hold.
    cfg(Op::Arm, 50, 64, 140),
    cfg(Op::Arm, 100, 64, 140),
    cfg(Op::Arm, 150, 64, 140),
    cfg(Op::Arm, 200, 64, 140),
    cfg(Op::Arm, 300, 64, 140),
    cfg(Op::Arm, 500, 64, 140),
    // The guard, with nothing interrupting: what the timer itself needs.
    cfg(Op::Arm, 500, 0, 0),
    cfg(Op::Arm, 500, 1, 0),
    cfg(Op::Arm, 500, 2, 0),
    cfg(Op::Arm, 500, 4, 0),
    cfg(Op::Arm, 500, 8, 0),
    // The guard, under the hold.
    cfg(Op::Arm, 500, 0, 140),
    cfg(Op::Arm, 500, 2, 140),
    cfg(Op::Arm, 500, 8, 140),
    // Moving and withdrawing a pending Pace.
    cfg(Op::Move, 500, 64, 140),
    cfg(Op::Withdraw, 500, 64, 140),
    // The same under the longest run of entries.
    cfg(Op::Arm, 300, 64, 410),
    cfg(Op::Arm, 500, 64, 410),
    cfg(Op::Arm, 800, 64, 410),
    cfg(Op::Move, 500, 64, 410),
    cfg(Op::Withdraw, 500, 64, 410),
    // And with nothing interrupting, as the reference.
    cfg(Op::Move, 500, 64, 0),
];

// The words of a configuration's row.
const TRIALS: u32 = 4;
const OK: u32 = 5;
const MOVED: u32 = 6;
const TOO_LATE: u32 = 7;
const LATE_TIMED: u32 = 8;
const LATE_UNTIMED: u32 = 9;
/// Reported armed, and no Pace fired at the instant reported.
const MISSED: u32 = 10;
/// Any other outcome that the chip's own check contradicts.
const WRONG: u32 = 11;
/// Pulses the driver reported as fired.
const REPORTED: u32 = 12;
const INTERRUPTS: u32 = 13;
/// The largest distance by which a Pace was moved past the instant asked for.
const MOVED_MAX: u32 = 14;

struct Row(u32);

impl Row {
    fn add(&self, word: u32) {
        let a = self.0 + 4 * word;
        w(a, r(a).wrapping_add(1));
    }
    fn max(&self, word: u32, v: u32) {
        let a = self.0 + 4 * word;
        if v > r(a) {
            w(a, v);
        }
    }
}

fn wait_until(d: &mut Driver<Nrf>, instant: Micros) {
    while d.now().since(instant) < 0 {}
}

const V: Chamber = Chamber::V;

/// After an outcome, when the pulse (if any) has ended: what the driver
/// reports, against what the outcome promised.
fn settle(d: &mut Driver<Nrf>, row: &Row, expect: Option<Micros>, until: Micros, missed_word: u32) {
    wait_until(d, until);
    let f = d.fired();
    if f.fired[1] {
        row.add(REPORTED);
    }
    match expect {
        Some(at) if f.fired[1] && f.at[1] == at => {}
        Some(_) => row.add(missed_word),
        None if f.fired[1] => row.add(WRONG),
        None => {}
    }
}

fn outcome(d: &mut Driver<Nrf>, row: &Row, asked: Micros, armed: Armed, pending: Option<Micros>) {
    match armed {
        Armed::Ok { at, moved } => {
            row.add(OK);
            if moved {
                row.add(MOVED);
                row.max(MOVED_MAX, at.since(asked).max(0) as u32);
            }
            settle(d, row, Some(at), at.plus(WIDTH + 600), MISSED);
        }
        Armed::TooLate => {
            row.add(TOO_LATE);
            // A Pace was delivered: the pending one, or a round's own.
            let now = d.now();
            match pending {
                Some(first) => settle(d, row, Some(first), first.plus(WIDTH + 600), WRONG),
                None => {
                    wait_until(d, now.plus(WIDTH + 1_500));
                    let f = d.fired();
                    if f.fired[1] {
                        row.add(REPORTED);
                    } else {
                        row.add(WRONG);
                    }
                }
            }
        }
        Armed::Late { at, timed } => {
            row.add(if timed { LATE_TIMED } else { LATE_UNTIMED });
            wait_until(d, at.plus(d.lead + WIDTH + 1_500));
            let f = d.fired();
            if f.fired[1] {
                row.add(REPORTED);
            }
            if f.fired[1] != timed {
                row.add(WRONG);
            }
        }
    }
}

pub fn run() -> ! {
    // The pace pins as GPIOTE task pins, low (P1.10 atrial, P1.11 ventricular).
    w(GPIOTE + 0x510, 3 | (10 << 8) | (1 << 13) | (3 << 16));
    w(GPIOTE + 0x514, 3 | (11 << 8) | (1 << 13) | (3 << 16));
    // TIMER3 and TIMER4: timers of 32 bits at 1 MHz. TIMER4's compare 0
    // clears it.
    for t in [TIMER3, TIMER4] {
        w(t + 0x504, 0);
        w(t + 0x508, 3);
        w(t + 0x510, 4);
    }
    w(TIMER4 + 0x200, 1);
    // Compare 0 sets the atrial pin and compare 4 clears it; compares 1
    // and 5 do the same for the ventricular pin.
    let channels = [(0u32, GPIOTE + 0x030), (4, GPIOTE + 0x060), (1, GPIOTE + 0x034), (5, GPIOTE + 0x064)];
    let mut n = 0;
    while n < 4 {
        let (compare, task) = channels[n as usize];
        w(PPI + 0x510 + 8 * n, TIMER3 + 0x140 + 4 * compare);
        w(PPI + 0x514 + 8 * n, task);
        n += 1;
    }
    w(PPI + 0x504, 0xF);
    w(TIMER3, 1);
    w(IRQ_SEED, 0x2545_F491);
    w(IRQ_COUNT, 0);
    w(NVIC_ISER0, 1 << 27);

    let mut i = 0;
    while i < ROW * (CONFIGS.len() as u32 + 1) {
        w(RESULTS + 4 * i, 0);
        i += 1;
    }
    w(RESULTS, MAGIC_RUNNING);
    w(RESULTS + 4, CONFIGS.len() as u32);
    w(RESULTS + 8, ROW);

    let mut d = Driver::new(Nrf);
    let mut k = 0;
    while k < CONFIGS.len() {
        let c = CONFIGS[k];
        let row = Row(RESULTS + 4 * ROW * (k as u32 + 1));
        w(row.0, c.op as u32);
        w(row.0 + 4, c.lead);
        w(row.0 + 8, c.guard);
        w(row.0 + 12, c.hold);
        d.lead = c.lead;
        d.guard = c.guard;
        let interrupts = r(IRQ_COUNT);
        interference(c.hold);
        match c.op {
            Op::Arm => {
                // From 1.5 ms ahead to 0.1 ms behind, a tick at a time.
                let mut dist: i32 = 1_500;
                while dist >= -100 {
                    row.add(TRIALS);
                    let asked = d.now().plus(dist as u32);
                    let armed = d.arm(V, asked, WIDTH);
                    outcome(&mut d, &row, asked, armed, None);
                    dist -= 1;
                }
            }
            Op::Move | Op::Withdraw => {
                // The pending Pace from 1.2 ms ahead to 0.6 ms behind
                // (its pulse over), a tick at a time.
                let mut dist: i32 = 1_200;
                while dist >= -600 {
                    let first = d.now().plus(1_500);
                    if let Armed::Ok { at, moved: false } = d.arm(V, first, WIDTH) {
                        row.add(TRIALS);
                        wait_until(&mut d, at.minus(dist as u32));
                        if c.op == Op::Move {
                            let asked = d.now().plus(5_000);
                            let armed = d.arm(V, asked, WIDTH);
                            outcome(&mut d, &row, asked, armed, Some(at));
                        } else {
                            match d.disarm(V) {
                                Disarmed::Withdrawn => {
                                    row.add(OK);
                                    settle(&mut d, &row, None, at.plus(WIDTH + 600), WRONG);
                                }
                                Disarmed::TooLate => {
                                    row.add(TOO_LATE);
                                    settle(&mut d, &row, Some(at), at.plus(WIDTH + 600), WRONG);
                                }
                            }
                        }
                        dist -= 1;
                    } else {
                        // The set-up itself was interrupted too long: let
                        // whatever it armed pass, and try the distance again.
                        let now = d.now();
                        wait_until(&mut d, now.plus(3_000));
                        if d.fired().fired[1] {
                            row.add(REPORTED);
                        }
                    }
                }
            }
        }
        interference(0);
        w(row.0 + 4 * INTERRUPTS, r(IRQ_COUNT).wrapping_sub(interrupts));
        w(row.0 + 4 * 15, MAGIC_DONE);
        // A gap between configurations, for the reader of the capture.
        let now = d.now();
        wait_until(&mut d, now.plus(20_000));
        k += 1;
    }
    w(RESULTS, MAGIC_DONE);
    loop {
        core::hint::spin_loop();
    }
}
