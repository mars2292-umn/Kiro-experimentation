//! The nRF52840-DK's registers by role (task 7.2; design "Hardware signal
//! flow"), through `rsk::mmio`: every address lies in a peripheral window
//! the Therapy_Partition owns (GPIOTE, P0, TIMER1, TIMER2, TIMER3).
//!
//! | Role | Register |
//! |---|---|
//! | start compare, atrium / ventricle | TIMER3 CC[0] / CC[1] |
//! | end compare | TIMER3 CC[4] / CC[5] |
//! | sense instant | TIMER3 CC[2] / CC[3], captured through the PPI |
//! | edge counter | TIMER1 / TIMER2 in counter mode: CC[0] the compare, CC[1] the capture |
//! | pace pin and its copy | GPIOTE channels 2 and 4 / 3 and 5 |
//! | sense input | GPIOTE channels 0 / 1 (P1.01 / P1.02, rising) |
//! | magnet | P1.07 |

use pm_core::{Chamber, Micros};
use pm_hw::{HwFault, Reg};

// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use rsk::creusot_std::model::View;

const GPIOTE: u32 = 0x4000_6000;
const TIMER1: u32 = 0x4000_9000;
const TIMER2: u32 = 0x4000_A000;
const TIMER3: u32 = 0x4001_A000;
const P1: u32 = 0x5000_0300;

const T_START: u32 = 0x000;
const T_CAPTURE: u32 = 0x040;
const T_EVENT: u32 = 0x140;
const T_INTENSET: u32 = 0x304;
const T_MODE: u32 = 0x504;
const T_BITMODE: u32 = 0x508;
const T_PRESCALER: u32 = 0x510;
const T_CC: u32 = 0x540;
const G_SET: u32 = 0x030;
const G_CLR: u32 = 0x060;
const G_CONFIG: u32 = 0x510;
const P_IN: u32 = 0x510;
const P_PIN_CNF: u32 = 0x700;

/// The pins (port 1; `hil-rig.toml`, keys `pacemaker_*`).
const SENSE_A: u32 = 1;
const SENSE_V: u32 = 2;
const COPY_A: u32 = 3;
const COPY_V: u32 = 4;
const MAGNET: u32 = 7;
const PACE_A: u32 = 10;
const PACE_V: u32 = 11;

/// The largest time between the two readings of the Kernel's time base
/// around the capture for the offset to be taken, in µs (proposal, "The
/// two clocks"); the offset's error is half of it.
pub const OFFSET_SPREAD: u32 = 64;
/// The attempts at it (a constant bound, PR-16).
const OFFSET_TRIES: u32 = 8;

/// Kernel time-base ticks in a microsecond (16 MHz).
const TICKS_PER_MICRO: u64 = crate::rsk_target::TICK_HZ / 1_000_000;

/// What the board layer keeps: the offset from the Kernel's time base to
/// the therapy timer, and how it was measured. All zero before Init.
#[derive(Clone, Copy)]
pub struct Words {
    /// Therapy microseconds less the Kernel time base's.
    pub offset: u32,
    /// The spread of the accepted measurement, µs.
    pub spread: u32,
    /// The attempts it took.
    pub tries: u32,
}

impl Words {
    pub const fn new() -> Words {
        Words { offset: 0, spread: 0, tries: 0 }
    }
}

pub struct Board<'a> {
    pub words: &'a mut Words,
}

#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result@ <= 5))]
const fn timer3_compare(c: Chamber, end: bool) -> u32 {
    let n = match c {
        Chamber::A => 0,
        Chamber::V => 1,
    };
    if end {
        n + 4
    } else {
        n
    }
}

#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result@ == 0x4000_9000 || result@ == 0x4000_A000))]
const fn counter(c: Chamber) -> u32 {
    match c {
        Chamber::A => TIMER1,
        Chamber::V => TIMER2,
    }
}

/// The Chamber's two GPIOTE channels: the pace pin's and its copy's.
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result.0@ <= 3 && result.1@ <= 5))]
const fn pace_channels(c: Chamber) -> (u32, u32) {
    match c {
        Chamber::A => (2, 4),
        Chamber::V => (3, 5),
    }
}

/// The Kernel's time base in microseconds, as 32 bits.
fn kernel_micros() -> u32 {
    (rsk::now() / TICKS_PER_MICRO) as u32
}

/// GPIOTE CONFIG: a task pin of port 1, initial level low.
const fn task_pin(pin: u32) -> u32 {
    3 | (pin << 8) | (1 << 13) | (3 << 16)
}

/// GPIOTE CONFIG: an event on the rising edge of a pin of port 1.
const fn event_pin(pin: u32) -> u32 {
    1 | (pin << 8) | (1 << 13) | (1 << 16)
}

impl pm_hw::Board for Board<'_> {
    fn configure(&mut self) -> Result<(), HwFault> {
        // The sense inputs and the magnet: inputs with a pull-down, so an
        // unwired line reads low and raises nothing.
        rsk::mmio::write(P1 + P_PIN_CNF + 4 * SENSE_A, 0b0100);
        rsk::mmio::write(P1 + P_PIN_CNF + 4 * SENSE_V, 0b0100);
        rsk::mmio::write(P1 + P_PIN_CNF + 4 * MAGNET, 0b0100);
        // GPIOTE: the two sense events, the two pace pins and their copies.
        rsk::mmio::write(GPIOTE + G_CONFIG, event_pin(SENSE_A));
        rsk::mmio::write(GPIOTE + G_CONFIG + 4, event_pin(SENSE_V));
        rsk::mmio::write(GPIOTE + G_CONFIG + 8, task_pin(PACE_A));
        rsk::mmio::write(GPIOTE + G_CONFIG + 12, task_pin(PACE_V));
        rsk::mmio::write(GPIOTE + G_CONFIG + 16, task_pin(COPY_A));
        rsk::mmio::write(GPIOTE + G_CONFIG + 20, task_pin(COPY_V));
        // The therapy timer: 32 bits at 1 MHz. Its end compares (4 and 5)
        // raise its interrupt; the start compares and the captures do not.
        rsk::mmio::write(TIMER3 + T_MODE, 0);
        rsk::mmio::write(TIMER3 + T_BITMODE, 3);
        rsk::mmio::write(TIMER3 + T_PRESCALER, 4);
        rsk::mmio::write(TIMER3 + T_INTENSET, (1 << 20) | (1 << 21));
        rsk::mmio::write(TIMER3 + T_START, 1);
        // The edge counters: counter mode, 32 bits, compare 0 raising the
        // interrupt. The first service of each line arms the compare.
        let mut k = 0;
        while k < 2 {
            let t = if k == 0 { TIMER1 } else { TIMER2 };
            rsk::mmio::write(t + T_MODE, 1);
            rsk::mmio::write(t + T_BITMODE, 3);
            rsk::mmio::write(t + T_INTENSET, 1 << 16);
            rsk::mmio::write(t + T_START, 1);
            k += 1;
        }
        // The offset of the two clocks: a capture of the therapy timer
        // between two readings of the Kernel's time base, taken when the
        // readings are close. Capture register 2 is free until the first
        // atrial edge.
        let mut tries = 0;
        while tries < OFFSET_TRIES {
            tries += 1;
            let before = kernel_micros();
            rsk::mmio::write(TIMER3 + T_CAPTURE + 8, 1);
            let after = kernel_micros();
            let captured = rsk::mmio::read(TIMER3 + T_CC + 8);
            let spread = after.wrapping_sub(before);
            if spread <= OFFSET_SPREAD {
                let middle = before.wrapping_add(spread / 2);
                self.words.offset = captured.wrapping_sub(middle);
                self.words.spread = spread;
                self.words.tries = tries;
                return Ok(());
            }
        }
        self.words.tries = tries;
        Err(HwFault::ClockOffset)
    }

    fn read(&mut self, reg: Reg) -> u32 {
        match reg {
            Reg::StartCompare(c) => rsk::mmio::read(TIMER3 + T_CC + 4 * timer3_compare(c, false)),
            Reg::EndCompare(c) => rsk::mmio::read(TIMER3 + T_CC + 4 * timer3_compare(c, true)),
            Reg::StartEvent(c) => rsk::mmio::read(TIMER3 + T_EVENT + 4 * timer3_compare(c, false)),
            Reg::EndEvent(c) => rsk::mmio::read(TIMER3 + T_EVENT + 4 * timer3_compare(c, true)),
            Reg::SenseInstant(c) => rsk::mmio::read(TIMER3 + T_CC + 4 * (2 + timer3_compare(c, false))),
            Reg::Count(c) => rsk::mmio::read(counter(c) + T_CC + 4),
            Reg::CountCompare(c) => rsk::mmio::read(counter(c) + T_CC),
            Reg::CountEvent(c) => rsk::mmio::read(counter(c) + T_EVENT),
            Reg::Magnet => (rsk::mmio::read(P1 + P_IN) >> MAGNET) & 1,
            Reg::CountCapture(_) | Reg::PaceSet(_) | Reg::PaceClear(_) => 0,
        }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        match reg {
            Reg::StartCompare(c) => rsk::mmio::write(TIMER3 + T_CC + 4 * timer3_compare(c, false), value),
            Reg::EndCompare(c) => rsk::mmio::write(TIMER3 + T_CC + 4 * timer3_compare(c, true), value),
            Reg::StartEvent(c) => rsk::mmio::write(TIMER3 + T_EVENT + 4 * timer3_compare(c, false), value),
            Reg::EndEvent(c) => rsk::mmio::write(TIMER3 + T_EVENT + 4 * timer3_compare(c, true), value),
            Reg::CountCapture(c) => rsk::mmio::write(counter(c) + T_CAPTURE + 4, value),
            Reg::CountCompare(c) => rsk::mmio::write(counter(c) + T_CC, value),
            Reg::CountEvent(c) => rsk::mmio::write(counter(c) + T_EVENT, value),
            Reg::PaceSet(c) => {
                let (pin, copy) = pace_channels(c);
                rsk::mmio::write(GPIOTE + G_SET + 4 * pin, value);
                rsk::mmio::write(GPIOTE + G_SET + 4 * copy, value);
            }
            Reg::PaceClear(c) => {
                let (pin, copy) = pace_channels(c);
                rsk::mmio::write(GPIOTE + G_CLR + 4 * pin, value);
                rsk::mmio::write(GPIOTE + G_CLR + 4 * copy, value);
            }
            // Read-only roles.
            Reg::SenseInstant(_) | Reg::Count(_) | Reg::Magnet => {}
        }
    }

    fn now(&mut self) -> Micros {
        Micros(kernel_micros().wrapping_add(self.words.offset))
    }
}
