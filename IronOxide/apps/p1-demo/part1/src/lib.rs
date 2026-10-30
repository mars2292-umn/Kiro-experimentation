//! P1 demo Partition 1 (`part1`, level C): the `log` Task (Priority 2),
//! which runs for a few milliseconds and is preempted by both Tasks of
//! Partition 0. Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_part1!` (task 10.5); the counters
//! are declared per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part1!();

/// The timer that releases `pulse` every 18 ms (longer than its 15 ms MIT,
/// so every event is accepted), the Partition's owned peripheral on each
/// board. QEMU mps2-an386: the CMSDK dual timer (window 0x4000_2000),
/// Timer1 load, control (bit 7 enable, bit 6 periodic, bit 5 interrupt,
/// bit 1 32-bit) and interrupt clear, clocked at 25 MHz.
#[cfg(not(feature = "nrf52840"))]
mod pulse_timer {
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
}

/// nRF52840-DK: TIMER1 (window 0x4000_9000, PS 6.30) as a 32-bit timer at
/// 1 MHz (prescaler 4 of 16 MHz), CC[0] at 18 000 µs with the shortcut
/// COMPARE[0] -> CLEAR, the COMPARE[0] interrupt enabled; `pulse` clears
/// the event.
#[cfg(feature = "nrf52840")]
mod pulse_timer {
    const TIMER1: u32 = 0x4000_9000;
    const TASKS_START: u32 = TIMER1 + 0x000;
    const TASKS_CLEAR: u32 = TIMER1 + 0x00C;
    const EVENTS_COMPARE0: u32 = TIMER1 + 0x140;
    const SHORTS: u32 = TIMER1 + 0x200;
    const INTENSET: u32 = TIMER1 + 0x304;
    const MODE: u32 = TIMER1 + 0x504;
    const BITMODE: u32 = TIMER1 + 0x508;
    const PRESCALER: u32 = TIMER1 + 0x510;
    const CC0: u32 = TIMER1 + 0x540;
    const PERIOD_US: u32 = 18_000;

    pub fn start() {
        rsk::mmio::write(MODE, 0);
        rsk::mmio::write(BITMODE, 3);
        rsk::mmio::write(PRESCALER, 4);
        rsk::mmio::write(CC0, PERIOD_US);
        rsk::mmio::write(SHORTS, 1);
        rsk::mmio::write(INTENSET, 1 << 16);
        rsk::mmio::write(TASKS_CLEAR, 1);
        rsk::mmio::write(TASKS_START, 1);
    }

    pub fn acknowledge() {
        rsk::mmio::write(EVENTS_COMPARE0, 0);
    }
}

/// Task `log`: busy for a few milliseconds (bounded iterations, PR-16),
/// then counts. Its first Job starts the timer that releases `pulse`
/// (task 5.4).
pub fn log(mut cx: rsk_ctx::Log) {
    if cx.count.get() == 0 {
        pulse_timer::start();
    }
    rsk::spin::<250_000>();
    cx.count.with(|c| *c += 1);
    rsk::debug(b"C");
}

/// Task `pulse` (sporadic): acknowledges the timer event and counts.
pub fn pulse(mut cx: rsk_ctx::Pulse) {
    pulse_timer::acknowledge();
    cx.count.with(|c| *c += 1);
    rsk::debug(b"D");
}
