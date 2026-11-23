//! The PPI probe's Partition `outsider` (docs/pacemaker task 3.6): it owns
//! the GPIO port and nothing else. Its Task attempts the registers the other
//! Partition and the Kernel own; each attempt has to fault. Two pins show
//! its Jobs: P1.12 toggles when a Job begins and P1.13 when it ends, so a
//! Job a fault ended is a toggle of the first without one of the second.
#![no_std]
#![forbid(unsafe_code)]

ppi_probe_decl::rsk_partition_outsider!();

const GPIOTE: u32 = 0x4000_6000;
const TIMER3: u32 = 0x4001_A000;
const PPI: u32 = 0x4001_F000;
/// Port 1 (in the window `P0`): OUTSET, OUTCLR and PIN_CNF.
const P1_OUTSET: u32 = 0x5000_0808;
const P1_OUTCLR: u32 = 0x5000_080C;
const P1_PIN_CNF: u32 = 0x5000_0A00;
const BEGAN: u32 = 1 << 12;
const ENDED: u32 = 1 << 13;

/// The attempts, in order.
pub const ATTEMPTS: u32 = 7;
/// The Job from which they are made, one a Job.
const FIRST_ATTEMPT: u32 = 60;

/// Task `poke` (periodic, 10 ms). From Job 60 on, one attempt a Job:
/// `attempt` is counted before the access and a bit of `survived` is set
/// after it, so a fault (which ends the Job) leaves the bit clear.
pub fn poke(mut cx: rsk_ctx::Poke) {
    let n = cx.count.get();
    cx.count.with(|c| *c = c.wrapping_add(1));
    if n == 0 {
        rsk::mmio::write(P1_OUTCLR, BEGAN | ENDED);
        rsk::mmio::write(P1_PIN_CNF + 4 * 12, 0b11);
        rsk::mmio::write(P1_PIN_CNF + 4 * 13, 0b11);
    }
    let (set, clear) = if n % 2 == 0 { (P1_OUTSET, P1_OUTCLR) } else { (P1_OUTCLR, P1_OUTSET) };
    let _ = clear;
    rsk::mmio::write(set, BEGAN);
    if n >= FIRST_ATTEMPT && cx.attempt.get() < ATTEMPTS {
        attempt(&mut cx);
    }
    rsk::mmio::write(set, ENDED);
}

fn attempt(cx: &mut rsk_ctx::Poke) {
    let k = cx.attempt.get();
    cx.attempt.with(|a| *a = k + 1);
    match k {
        // TIMER3, the other Partition's: its start compare, read and
        // written, and its stop task (which would end the pulses).
        0 => {
            let _ = rsk::mmio::read(TIMER3 + 0x540);
        }
        1 => rsk::mmio::write(TIMER3 + 0x540, 0),
        2 => rsk::mmio::write(TIMER3 + 0x004, 1),
        // GPIOTE, the other Partition's: the pace pin's channel.
        3 => rsk::mmio::write(GPIOTE + 0x510, 0),
        // The PPI, the Kernel's: a channel's event end-point, the
        // disable-all register, and a read of the enable register.
        4 => rsk::mmio::write(PPI + 0x510, 0),
        5 => rsk::mmio::write(PPI + 0x508, 0xFFFF_FFFF),
        _ => {
            let _ = rsk::mmio::read(PPI + 0x500);
        }
    }
    cx.survived.with(|s| *s |= 1 << k);
}
