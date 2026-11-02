//! P1 demo Partition 1 (`part1`, level C): the `log` Task (Priority 2),
//! which runs for a few milliseconds and is preempted by both Tasks of
//! Partition 0. Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_part1!` (task 10.5); the counters
//! are declared per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part1!();

/// The Release_Source of `pulse` on each board, started by `log`'s first
/// Job and acknowledged by every `pulse` Job. QEMU mps2-an386: the CMSDK
/// dual timer (window 0x4000_2000), Timer1 load, control (bit 7 enable,
/// bit 6 periodic, bit 5 interrupt, bit 1 32-bit) and interrupt clear,
/// clocked at 25 MHz, every 18 ms (longer than the 15 ms MIT, so every
/// event is accepted); no response line.
#[cfg(not(feature = "nrf52840"))]
mod pulse_source {
    const DUALTIMER: u32 = 0x4000_2000;
    const TIMER1_LOAD: u32 = DUALTIMER + 0x00;
    const TIMER1_CONTROL: u32 = DUALTIMER + 0x08;
    const TIMER1_INTCLR: u32 = DUALTIMER + 0x0C;
    const PERIOD_TICKS: u32 = 450_000;

    pub fn start() {
        rsk::mmio::write(TIMER1_LOAD, PERIOD_TICKS);
        rsk::mmio::write(TIMER1_CONTROL, (1 << 7) | (1 << 6) | (1 << 5) | (1 << 1));
    }

    pub fn acknowledge() {
        rsk::mmio::write(TIMER1_INTCLR, 1);
    }

    pub fn respond(_count: u32) {}
}

/// nRF52840-DK on the HIL_Rig (task 7.1, `hil-rig.toml`): GPIOTE channel
/// 0 (window 0x4000_6000, PS 6.9) in event mode on the rising edge of
/// P1.01 (Arduino D0), the line the stimulus board pulses every 18 ms;
/// the IN[0] interrupt is the Task's Release_Source. The response line
/// P1.10 (D8, the analyzer's D0) toggles in every `pulse` Job against the
/// stimulus mirror on the analyzer's D6, and LED1 (P0.13) toggles every
/// 14 pulses as the visible sign. The Partition's `P0` window holds port 1.
#[cfg(feature = "nrf52840")]
mod pulse_source {
    const GPIOTE: u32 = 0x4000_6000;
    const EVENTS_IN0: u32 = GPIOTE + 0x100;
    const INTENSET: u32 = GPIOTE + 0x304;
    const CONFIG0: u32 = GPIOTE + 0x510;
    const P0: u32 = 0x5000_0000;
    const P1: u32 = 0x5000_0300;
    const P0_OUT: u32 = P0 + 0x504;
    const P0_OUTSET: u32 = P0 + 0x508;
    const P0_OUTCLR: u32 = P0 + 0x50C;
    const P0_PIN_CNF: u32 = P0 + 0x700;
    const P1_OUT: u32 = P1 + 0x504;
    const P1_OUTSET: u32 = P1 + 0x508;
    const P1_OUTCLR: u32 = P1 + 0x50C;
    const P1_PIN_CNF: u32 = P1 + 0x700;
    /// The stimulus input, P1.01.
    const INPUT: u32 = 1;
    /// The response output, P1.10.
    const RESPONSE: u32 = 10;
    /// LED1 of the DK, P0.13, lit when low.
    const LED1: u32 = 13;

    pub fn start() {
        // Input connected with a pull-down (an unwired line reads low and
        // raises nothing); response and LED as outputs, low and off.
        rsk::mmio::write(P1_PIN_CNF + 4 * INPUT, 0b0100);
        rsk::mmio::write(P1_OUTCLR, 1 << RESPONSE);
        rsk::mmio::write(P0_OUTSET, 1 << LED1);
        rsk::mmio::write(P1_PIN_CNF + 4 * RESPONSE, 0b11);
        rsk::mmio::write(P0_PIN_CNF + 4 * LED1, 0b11);
        // CONFIG[0]: MODE = Event (1), PSEL = INPUT, PORT = 1, POLARITY = LoToHi (1).
        rsk::mmio::write(CONFIG0, 1 | (INPUT << 8) | (1 << 13) | (1 << 16));
        rsk::mmio::write(EVENTS_IN0, 0);
        rsk::mmio::write(INTENSET, 1);
    }

    pub fn acknowledge() {
        rsk::mmio::write(EVENTS_IN0, 0);
    }

    pub fn respond(count: u32) {
        if rsk::mmio::read(P1_OUT) & (1 << RESPONSE) != 0 {
            rsk::mmio::write(P1_OUTCLR, 1 << RESPONSE);
        } else {
            rsk::mmio::write(P1_OUTSET, 1 << RESPONSE);
        }
        if count % 14 == 0 {
            if rsk::mmio::read(P0_OUT) & (1 << LED1) != 0 {
                rsk::mmio::write(P0_OUTCLR, 1 << LED1);
            } else {
                rsk::mmio::write(P0_OUTSET, 1 << LED1);
            }
        }
    }
}

/// Task `log`: busy for 3 ms by the time base (bounded in iterations too,
/// PR-16; within its 6 ms Budget on any Target), then counts. Its first Job
/// starts the Release_Source of `pulse` (task 5.4).
pub fn log(mut cx: rsk_ctx::Log) {
    if cx.count.get() == 0 {
        pulse_source::start();
    }
    let start = rsk::now();
    let mut i = 0u32;
    while rsk::now().wrapping_sub(start) < rsk_target::ms(3) && i < 10_000_000 {
        i += 1;
    }
    cx.count.with(|c| *c += 1);
    rsk::debug(b"C");
}

/// Task `pulse` (sporadic): acknowledges the source's event, counts, and
/// answers on the response line where the board has one.
pub fn pulse(mut cx: rsk_ctx::Pulse) {
    pulse_source::acknowledge();
    cx.count.with(|c| *c += 1);
    pulse_source::respond(cx.count.get());
    rsk::debug(b"D");
}
