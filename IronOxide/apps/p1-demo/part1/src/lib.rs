//! P1 demo Partition 1 (`part1`, level C): the `log` Task (Priority 2),
//! which runs for a few milliseconds and is preempted by both Tasks of
//! Partition 0. Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_part1!` (task 10.5); the counters
//! are declared per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part1!();

/// The CMSDK dual timer of the QEMU mps2-an386 (the Partition's peripheral,
/// window 0x4000_2000): Timer1 load, control (bit 7 enable, bit 6 periodic,
/// bit 5 interrupt, bit 1 32-bit) and interrupt clear, clocked at 25 MHz.
const DUALTIMER: u32 = 0x4000_2000;
const TIMER1_LOAD: u32 = DUALTIMER + 0x00;
const TIMER1_CONTROL: u32 = DUALTIMER + 0x08;
const TIMER1_INTCLR: u32 = DUALTIMER + 0x0C;
/// 18 ms at 25 MHz: longer than `pulse`'s 15 ms MIT, so every event is accepted.
const PULSE_PERIOD_TICKS: u32 = 450_000;

/// Task `log`: busy for about 5 ms (bounded iterations, PR-16), then counts.
/// Its first Job starts the dual timer that releases `pulse` (task 5.4).
pub fn log(mut cx: rsk_ctx::Log) {
    if cx.count.get() == 0 {
        rsk::mmio::write(TIMER1_LOAD, PULSE_PERIOD_TICKS);
        rsk::mmio::write(TIMER1_CONTROL, (1 << 7) | (1 << 6) | (1 << 5) | (1 << 1));
    }
    rsk::spin::<250_000>();
    cx.count.with(|c| *c += 1);
    rsk::debug(b"C");
}

/// Task `pulse` (sporadic): clears the timer interrupt and counts.
pub fn pulse(mut cx: rsk_ctx::Pulse) {
    rsk::mmio::write(TIMER1_INTCLR, 1);
    cx.count.with(|c| *c += 1);
    rsk::debug(b"D");
}
