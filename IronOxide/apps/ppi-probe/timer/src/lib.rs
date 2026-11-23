//! The PPI probe's Partition `timer` (docs/pacemaker task 3.6): it owns
//! TIMER3 and GPIOTE, and the two PPI channels of the declaration connect
//! them. The Kernel wrote those channels at boot; this Partition cannot.
#![no_std]
#![forbid(unsafe_code)]

ppi_probe_decl::rsk_partition_timer!();

const GPIOTE: u32 = 0x4000_6000;
const TIMER3: u32 = 0x4001_A000;
const PPI: u32 = 0x4001_F000;

/// The pulse the channels make: its width, and how far ahead it is armed.
const WIDTH: u32 = 400;
const AHEAD: u32 = 1_000;
/// The Job from which the attempts on the PPI's window are made, one a Job.
const FIRST_ATTEMPT: u32 = 50;

/// GPIOTE CONFIG: task mode, a pin of port 1, initial level low.
const fn task_pin_port1(pin: u32) -> u32 {
    3 | (pin << 8) | (1 << 13) | (3 << 16)
}

/// Task `pulse` (periodic, 10 ms). The first Job configures the two GPIOTE
/// task pins (P1.10 and its copy P1.03) and starts TIMER3 at 1 MHz. Every
/// Job arms one pulse: the end compare (4) and then the start compare (0),
/// a millisecond ahead; the pin edges are the PPI's. Jobs 50 to 52 first
/// attempt the PPI's window, which no Partition owns: each attempt has to
/// fault, which ends that Job before it arms, so a pulse is missing there.
pub fn pulse(mut cx: rsk_ctx::Pulse) {
    let n = cx.count.get();
    cx.count.with(|c| *c = c.wrapping_add(1));
    if n == 0 {
        rsk::mmio::write(GPIOTE + 0x510, task_pin_port1(10));
        rsk::mmio::write(GPIOTE + 0x518, task_pin_port1(3));
        rsk::mmio::write(TIMER3 + 0x504, 0);
        rsk::mmio::write(TIMER3 + 0x508, 3);
        rsk::mmio::write(TIMER3 + 0x510, 4);
        rsk::mmio::write(TIMER3, 1);
    }
    if n >= FIRST_ATTEMPT && cx.attempt.get() < 3 {
        let k = cx.attempt.get();
        cx.attempt.with(|a| *a = k + 1);
        match k {
            // CHENCLR: would disable every channel.
            0 => rsk::mmio::write(PPI + 0x508, 0xFFFF_FFFF),
            // CH[0].EEP: would disconnect the start of the pulse.
            1 => rsk::mmio::write(PPI + 0x510, 0),
            // CHEN: a read.
            _ => {
                let _ = rsk::mmio::read(PPI + 0x500);
            }
        }
        cx.survived.with(|s| *s |= 1 << k);
    }
    // The present, through capture register 2.
    rsk::mmio::write(TIMER3 + 0x048, 1);
    let start = rsk::mmio::read(TIMER3 + 0x548).wrapping_add(AHEAD);
    rsk::mmio::write(TIMER3 + 0x550, start.wrapping_add(WIDTH));
    rsk::mmio::write(TIMER3 + 0x540, start);
}
