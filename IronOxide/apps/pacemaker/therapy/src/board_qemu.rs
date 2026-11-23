//! The QEMU board's registers by role (task 7.3; design "The hardware
//! layer"). The mps2-an386 has no timer with compares and no pins, so the
//! roles are kept in memory and one device stands behind them: timer 1 of
//! the Partition's dual timer, a one-shot set to the earliest armed start,
//! whose interrupt releases `pace`. A start that the present has reached
//! has fired: its start and end flags are set together (there is no pulse
//! to wait for). Senses come from a table that `control` plays. This is
//! for the logic and the Analyzer's inputs, not for timing claims.

use pm_core::{Chamber, Micros};
use pm_hw::{HwFault, Reg};

const DUALTIMER: u32 = 0x4000_2000;
const TIMER1_LOAD: u32 = DUALTIMER;
const TIMER1_CONTROL: u32 = DUALTIMER + 0x08;
const TIMER1_INTCLR: u32 = DUALTIMER + 0x0C;
/// Enabled, interrupt enabled, 32 bits, one-shot.
const ONE_SHOT: u32 = (1 << 7) | (1 << 5) | (1 << 1) | 1;

/// Time-base ticks in a microsecond (the dual timer runs at the same rate).
const TICKS_PER_MICRO: u64 = crate::rsk_target::TICK_HZ / 1_000_000;

/// One Sense of the scripted sequence: its instant in µs of the time base,
/// and its Chamber (0 atrium, 1 ventricle).
pub struct ScriptedSense {
    pub at: u32,
    pub chamber: u8,
}

/// The Chamber the script is about: the atrium in the atrial Modes' runs
/// (features `qemu-aoo`, `qemu-aai`), the ventricle otherwise.
#[cfg(any(feature = "qemu-aoo", feature = "qemu-aai"))]
pub const PACED: u8 = 0;
#[cfg(not(any(feature = "qemu-aoo", feature = "qemu-aai")))]
pub const PACED: u8 = 1;

/// The sequence every QEMU run plays (120 ppm; VRP 320 ms, ARP 250 ms):
/// in the paced Chamber a Sense that inhibits, one inside the refractory
/// period and, later, one more that inhibits; and one in the other
/// Chamber, which no single-chamber Mode senses. The asynchronous Modes
/// sense none of them.
pub const SCRIPT: [ScriptedSense; 4] = [
    ScriptedSense { at: 300_000, chamber: PACED },
    ScriptedSense { at: 450_000, chamber: PACED },
    ScriptedSense { at: 1_000_000, chamber: 1 - PACED },
    ScriptedSense { at: 1_700_000, chamber: PACED },
];

/// The roles kept in memory. All zero before Init.
#[derive(Clone, Copy)]
pub struct Words {
    pub start: [u32; 2],
    pub end: [u32; 2],
    /// The start compare is ahead of the present and will fire.
    pub armed: [bool; 2],
    pub start_event: [bool; 2],
    pub end_event: [bool; 2],
    pub count: [u32; 2],
    pub count_captured: [u32; 2],
    pub sense_instant: [u32; 2],
    /// The next entry of the script.
    pub next_sense: u32,
    /// Pulses set by software (the driver's forced path).
    pub forced: u32,
    /// Always zero (a Partition static has no padding byte).
    pub reserved: [u8; 2],
}

impl Words {
    pub const fn new() -> Words {
        Words {
            start: [0; 2],
            end: [0; 2],
            armed: [false; 2],
            start_event: [false; 2],
            end_event: [false; 2],
            count: [0; 2],
            count_captured: [0; 2],
            sense_instant: [0; 2],
            next_sense: 0,
            forced: 0,
            reserved: [0; 2],
        }
    }
}

pub struct Board<'a> {
    pub words: &'a mut Words,
}

fn micros() -> Micros {
    Micros((rsk::now() / TICKS_PER_MICRO) as u32)
}

impl Board<'_> {
    /// What the timer and the PPI do by themselves on the nRF52840: a start
    /// the present has reached fires, and the one-shot is set to the
    /// earliest start still armed.
    fn poll(&mut self) {
        let now = micros();
        rsk::mmio::write(TIMER1_INTCLR, 1);
        let mut next: Option<u32> = None;
        let mut k = 0;
        while k < 2 {
            if self.words.armed[k] {
                let ahead = Micros(self.words.start[k]).since(now);
                if ahead <= 0 {
                    self.words.armed[k] = false;
                    self.words.start_event[k] = true;
                    self.words.end_event[k] = true;
                } else {
                    next = Some(match next {
                        Some(n) if n <= ahead as u32 => n,
                        _ => ahead as u32,
                    });
                }
            }
            k += 1;
        }
        match next {
            Some(ahead) => {
                rsk::mmio::write(TIMER1_CONTROL, 0);
                rsk::mmio::write(TIMER1_LOAD, (ahead as u64 * TICKS_PER_MICRO).min(u32::MAX as u64) as u32);
                rsk::mmio::write(TIMER1_CONTROL, ONE_SHOT);
            }
            None => rsk::mmio::write(TIMER1_CONTROL, 0),
        }
    }

    /// Plays the script up to the present: each Sense due is counted and
    /// its instant kept, as the counter and the capture would. Returns the
    /// Chambers that sensed (bit 0 the atrium, bit 1 the ventricle), for
    /// `control` to raise their Release_Signals.
    pub fn play(&mut self) -> u32 {
        let now = micros();
        let mut sensed = 0;
        let mut k = 0;
        while k < SCRIPT.len() {
            let next = self.words.next_sense as usize;
            if next < SCRIPT.len() && now.is_at_or_after(Micros(SCRIPT[next].at)) {
                let c: usize = if SCRIPT[next].chamber == 0 { 0 } else { 1 };
                self.words.count[c] = self.words.count[c].wrapping_add(1);
                self.words.sense_instant[c] = SCRIPT[next].at;
                self.words.next_sense = self.words.next_sense.saturating_add(1);
                sensed |= if c == 0 { 1 } else { 2 };
            }
            k += 1;
        }
        sensed
    }
}

impl pm_hw::Board for Board<'_> {
    fn configure(&mut self) -> Result<(), HwFault> {
        rsk::mmio::write(TIMER1_CONTROL, 0);
        rsk::mmio::write(TIMER1_INTCLR, 1);
        Ok(())
    }

    fn read(&mut self, reg: Reg) -> u32 {
        self.poll();
        let w = &self.words;
        match reg {
            Reg::StartCompare(c) => w.start[c.index()],
            Reg::EndCompare(c) => w.end[c.index()],
            Reg::StartEvent(c) => w.start_event[c.index()] as u32,
            Reg::EndEvent(c) => w.end_event[c.index()] as u32,
            Reg::SenseInstant(c) => w.sense_instant[c.index()],
            Reg::Count(c) => w.count_captured[c.index()],
            Reg::CountCompare(_) | Reg::CountEvent(_) | Reg::Magnet => 0,
            Reg::CountCapture(_) | Reg::PaceSet(_) | Reg::PaceClear(_) => 0,
        }
    }

    fn write(&mut self, reg: Reg, value: u32) {
        // A start due before this write has fired before it.
        self.poll();
        match reg {
            Reg::StartCompare(c) => {
                let ahead = Micros(value).since(micros());
                self.words.start[c.index()] = value;
                // On the far side of the present, or behind it: not armed.
                self.words.armed[c.index()] = ahead > 0 && ahead < (1 << 29);
                self.poll();
            }
            Reg::EndCompare(c) => self.words.end[c.index()] = value,
            Reg::StartEvent(c) => self.words.start_event[c.index()] = value != 0,
            Reg::EndEvent(c) => self.words.end_event[c.index()] = value != 0,
            Reg::CountCapture(c) => self.words.count_captured[c.index()] = self.words.count[c.index()],
            Reg::PaceSet(_) => self.words.forced = self.words.forced.wrapping_add(1),
            Reg::CountCompare(_) | Reg::CountEvent(_) | Reg::PaceClear(_) => {}
            Reg::SenseInstant(_) | Reg::Count(_) | Reg::Magnet => {}
        }
    }

    fn now(&mut self) -> Micros {
        micros()
    }
}

/// The Chamber of a script entry.
pub const fn chamber_of(s: &ScriptedSense) -> Chamber {
    if s.chamber & 1 == 0 {
        Chamber::A
    } else {
        Chamber::V
    }
}
