//! The stimulus Partition (`stim`): the `pulse` Task drives the rig's
//! stimulus line. Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_stim!`; the counter is per-Task
//! state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

hil_stimulus_decl::rsk_partition_stim!();

/// nRF52840 GPIO (PS 6.8; the Partition's peripheral window `P0`,
/// 0x5000_0000, holds port 1 too): the stimulus line, its mirror for the
/// logic analyzer, and LED1 of the DK (active low). The rig's wiring is
/// `hil-rig.toml`: P1.01 (Arduino D0) runs to the target board's P1.01,
/// P1.05 (D4) to the analyzer's D6; one register write drives both, so
/// the mirror carries the stimulus edge without skew.
mod gpio {
    const P0: u32 = 0x5000_0000;
    const P1: u32 = 0x5000_0300;
    pub const P1_OUTSET: u32 = P1 + 0x508;
    pub const P1_OUTCLR: u32 = P1 + 0x50C;
    const P1_PIN_CNF: u32 = P1 + 0x700;
    pub const P0_OUT: u32 = P0 + 0x504;
    pub const P0_OUTSET: u32 = P0 + 0x508;
    pub const P0_OUTCLR: u32 = P0 + 0x50C;
    const P0_PIN_CNF: u32 = P0 + 0x700;
    /// The stimulus line (P1.01) and its analyzer mirror (P1.05), as one mask.
    pub const STIMULUS: u32 = (1 << 1) | (1 << 5);
    /// The stimulus line alone (the storm).
    #[cfg(feature = "storm")]
    pub const LINE: u32 = 1 << 1;
    /// LED1 of the DK, P0.13, lit when low.
    pub const LED1: u32 = 13;

    /// The stimulus pins and the LED as outputs (PIN_CNF.DIR = 1, input
    /// buffer disconnected), the lines low and the LED off.
    pub fn configure() {
        rsk::mmio::write(P1_OUTCLR, STIMULUS);
        rsk::mmio::write(P0_OUTSET, 1 << LED1);
        rsk::mmio::write(P1_PIN_CNF + 4 * 1, 0b11);
        rsk::mmio::write(P1_PIN_CNF + 4 * 5, 0b11);
        rsk::mmio::write(P0_PIN_CNF + 4 * LED1, 0b11);
    }
}

/// Ticks of the 100 µs pulse: a tenth of a millisecond of the time base.
#[cfg(not(feature = "storm"))]
const PULSE_TICKS: u64 = rsk_target::TICK_HZ / 10_000;
/// Ticks of the storm's 16 ms of toggling.
#[cfg(feature = "storm")]
const STORM_TICKS: u64 = rsk_target::TICK_HZ / 1000 * 16;

/// Task `pulse` (periodic, 18 ms): the first Job configures the pins; every
/// Job raises the stimulus line for 100 µs (by the time base, bounded in
/// iterations too, PR-16) and the LED toggles every 28 pulses (about twice
/// a second) as the visible sign that the stimulus board runs.
pub fn pulse(mut cx: rsk_ctx::Pulse) {
    if cx.count.get() == 0 {
        gpio::configure();
    }
    #[cfg(not(feature = "storm"))]
    {
        rsk::mmio::write(gpio::P1_OUTSET, gpio::STIMULUS);
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now().wrapping_sub(start) < PULSE_TICKS && i < 100_000 {
            i += 1;
        }
        rsk::mmio::write(gpio::P1_OUTCLR, gpio::STIMULUS);
    }
    // The interrupt storm (task 7.5, R19.3): the stimulus line alone (the
    // analyzer's mirror stays quiet: millions of edges would drown the
    // capture) toggled as fast as the core can write the port, 64 edges
    // between time-base reads, for 16 of the 18 ms (within the 17 ms
    // Budget); the target's GPIOTE sees an event every few hundred
    // nanoseconds while its MIT masks the source (DD-06).
    #[cfg(feature = "storm")]
    {
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now().wrapping_sub(start) < STORM_TICKS && i < 2_000_000 {
            let mut k = 0u32;
            while k < 32 {
                rsk::mmio::write(gpio::P1_OUTSET, gpio::LINE);
                rsk::mmio::write(gpio::P1_OUTCLR, gpio::LINE);
                k += 1;
            }
            i += 1;
        }
    }
    cx.count.with(|c| *c = c.wrapping_add(1));
    if cx.count.get() % 28 == 0 {
        if rsk::mmio::read(gpio::P0_OUT) & (1 << gpio::LED1) != 0 {
            rsk::mmio::write(gpio::P0_OUTCLR, 1 << gpio::LED1);
        } else {
            rsk::mmio::write(gpio::P0_OUTSET, 1 << gpio::LED1);
        }
    }
}
