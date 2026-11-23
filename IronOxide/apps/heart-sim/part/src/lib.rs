//! The Heart_Simulator's Partition (`heart`): the `beat` Task plays the
//! scenario of the build, one millisecond per Job (docs/pacemaker PM26.3,
//! task 4.2). Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_heart!`; the heart's state is
//! per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

pub mod heart;
pub mod scenario;

heart_sim_decl::rsk_partition_heart!();

/// nRF52840 GPIO (PS 6.8; the Partition's peripheral window `P0`,
/// 0x5000_0000, holds port 1 too). The wiring is `hil-rig.toml`, keys
/// `pacemaker_*`:
///
/// | Signal | Pin | To |
/// |---|---|---|
/// | atrial sense | P1.01 | the PG's P1.01 |
/// | ventricular sense | P1.02 | the PG's P1.02 |
/// | atrial sense, mirror | P1.05 | the analyzer's D6 |
/// | ventricular sense, mirror | P1.06 | the analyzer's D7 |
/// | atrial pace | P1.03 | from the PG's P1.03 |
/// | ventricular pace | P1.04 | from the PG's P1.04 |
///
/// A sense pulse and its mirror are driven by one register write, so the
/// analyzer sees the edge the PG sees without skew.
mod gpio {
    const P0: u32 = 0x5000_0000;
    const P1: u32 = 0x5000_0300;
    pub const P1_OUTSET: u32 = P1 + 0x508;
    pub const P1_OUTCLR: u32 = P1 + 0x50C;
    const P1_LATCH: u32 = P1 + 0x520;
    const P1_PIN_CNF: u32 = P1 + 0x700;
    pub const P0_OUT: u32 = P0 + 0x504;
    pub const P0_OUTSET: u32 = P0 + 0x508;
    pub const P0_OUTCLR: u32 = P0 + 0x50C;
    const P0_PIN_CNF: u32 = P0 + 0x700;
    /// The atrial sense line and its mirror, as one mask.
    pub const SENSE_A: u32 = (1 << 1) | (1 << 5);
    /// The ventricular sense line and its mirror.
    pub const SENSE_V: u32 = (1 << 2) | (1 << 6);
    /// The pace inputs.
    pub const PACE_A: u32 = 1 << 3;
    pub const PACE_V: u32 = 1 << 4;
    /// LED1 of the DK, P0.13, lit when low.
    pub const LED1: u32 = 13;

    /// The sense pins and the LED as outputs (PIN_CNF.DIR = 1, input
    /// buffer disconnected), low and off; the pace pins as inputs with a
    /// pull-down and the latch armed for a high level (SENSE = High), so
    /// that a pulse far shorter than a Job's period is not missed.
    pub fn configure() {
        rsk::mmio::write(P1_OUTCLR, SENSE_A | SENSE_V);
        rsk::mmio::write(P0_OUTSET, 1 << LED1);
        rsk::mmio::write(P1_PIN_CNF + 4 * 1, 0b11);
        rsk::mmio::write(P1_PIN_CNF + 4 * 2, 0b11);
        rsk::mmio::write(P1_PIN_CNF + 4 * 5, 0b11);
        rsk::mmio::write(P1_PIN_CNF + 4 * 6, 0b11);
        rsk::mmio::write(P0_PIN_CNF + 4 * LED1, 0b11);
        let input = (1 << 2) | (2 << 16);
        rsk::mmio::write(P1_PIN_CNF + 4 * 3, input);
        rsk::mmio::write(P1_PIN_CNF + 4 * 4, input);
        rsk::mmio::write(P1_LATCH, PACE_A | PACE_V);
    }

    /// The pace pins that went high since the last call. A latch bit that
    /// is cleared while its pin is still high is set again at once, so a
    /// pulse longer than a period is seen in successive Jobs; the caller
    /// counts a Pace at the first of them.
    pub fn take_paces() -> u32 {
        let latched = rsk::mmio::read(P1_LATCH) & (PACE_A | PACE_V);
        rsk::mmio::write(P1_LATCH, latched);
        latched
    }
}

/// Ticks of a sense pulse: 100 µs of the time base.
const PULSE_TICKS: u64 = rsk_target::TICK_HZ / 10_000;

/// Task `beat` (periodic, 1 ms): the first Job configures the pins; every
/// Job gives the heart the Paces that began since the last one and plays
/// one millisecond; for each beat the Chamber's sense line and its mirror
/// go high for 100 µs (by the time base, bounded in iterations too,
/// PR-16). The LED toggles every 512 ms as the sign that the board runs.
pub fn beat(mut cx: rsk_ctx::Beat) {
    if cx.ms.get() == 0 {
        gpio::configure();
    }
    // A Pace is counted in the first Job that sees its latch bit; a pulse
    // of up to 1.9 ms shows in up to three successive Jobs.
    let latched = gpio::take_paces();
    let began = latched & !cx.pace_seen.get();
    cx.pace_seen.with(|p| *p = latched);

    let mut h = heart::Heart {
        ms: cx.ms.get(),
        next_a: cx.next_a.get(),
        next_v: cx.next_v.get(),
        conducted: cx.conducted.get(),
        beats_a: cx.beats_a.get(),
        beats_v: cx.beats_v.get(),
    };
    let beats = h.tick(scenario::SELECTED, began & gpio::PACE_A != 0, began & gpio::PACE_V != 0);
    cx.ms.with(|v| *v = h.ms);
    cx.next_a.with(|v| *v = h.next_a);
    cx.next_v.with(|v| *v = h.next_v);
    cx.conducted.with(|v| *v = h.conducted);
    cx.beats_a.with(|v| *v = h.beats_a);
    cx.beats_v.with(|v| *v = h.beats_v);

    let mut mask = 0;
    if beats.atrial {
        mask |= gpio::SENSE_A;
    }
    if beats.ventricular {
        mask |= gpio::SENSE_V;
    }
    if mask != 0 {
        rsk::mmio::write(gpio::P1_OUTSET, mask);
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now().wrapping_sub(start) < PULSE_TICKS && i < 100_000 {
            i += 1;
        }
        rsk::mmio::write(gpio::P1_OUTCLR, mask);
    }
    if h.ms % 512 == 0 {
        if rsk::mmio::read(gpio::P0_OUT) & (1 << gpio::LED1) != 0 {
            rsk::mmio::write(gpio::P0_OUTCLR, 1 << gpio::LED1);
        } else {
            rsk::mmio::write(gpio::P0_OUTSET, 1 << gpio::LED1);
        }
    }
}
