//! Hardware spike of the PACEMAKER experiment, task 1.1
//! (docs/pacemaker/tasks.md; design decisions PD-01 and PD-02).
//!
//! One privileged program on board A. TIMER3 runs at 1 MHz; its compares
//! 0 and 1 set the two pace pins and its compares 4 and 5 clear them,
//! through four PPI channels the program writes itself. The main loop only
//! writes compare registers ahead of time: no edge is made by software.
//!
//! Pace pins (hil-rig.toml: the target's pins that reach the analyzer):
//! atrial P1.10 (analyzer D0), ventricular P1.11 (analyzer D1).
//!
//! The program delivers groups of pulses, one group per width of `WIDTHS`,
//! `PER_GROUP` pulses on each pin, and repeats for ever. The host script
//! `measure.py` reads an analyzer capture and reports every width.
#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

const CLOCK: u32 = 0x4000_0000;
const GPIOTE: u32 = 0x4000_6000;
const TIMER3: u32 = 0x4001_A000;
const PPI: u32 = 0x4001_F000;

/// APPROTECT.DISABLE: 0x5A keeps the access port open until the next reset.
const APPROTECT_DISABLE: u32 = 0x4000_0558;
// CLOCK
const TASKS_HFCLKSTART: u32 = CLOCK;
const EVENTS_HFCLKSTARTED: u32 = CLOCK + 0x100;
// GPIOTE
const fn gpiote_set(n: u32) -> u32 {
    GPIOTE + 0x030 + 4 * n
}
const fn gpiote_clr(n: u32) -> u32 {
    GPIOTE + 0x060 + 4 * n
}
const fn gpiote_config(n: u32) -> u32 {
    GPIOTE + 0x510 + 4 * n
}
// TIMER3
const T_START: u32 = TIMER3;
const fn t_capture(n: u32) -> u32 {
    TIMER3 + 0x040 + 4 * n
}
const fn t_compare_event(n: u32) -> u32 {
    TIMER3 + 0x140 + 4 * n
}
const T_MODE: u32 = TIMER3 + 0x504;
const T_BITMODE: u32 = TIMER3 + 0x508;
const T_PRESCALER: u32 = TIMER3 + 0x510;
const fn t_cc(n: u32) -> u32 {
    TIMER3 + 0x540 + 4 * n
}
// PPI
const PPI_CHENSET: u32 = PPI + 0x504;
const fn ppi_eep(n: u32) -> u32 {
    PPI + 0x510 + 8 * n
}
const fn ppi_tep(n: u32) -> u32 {
    PPI + 0x514 + 8 * n
}

/// GPIOTE CONFIG: task mode, pin of port 1, toggle polarity (unused: only
/// the SET and CLR tasks are triggered), initial level low.
const fn task_pin_port1(pin: u32) -> u32 {
    3 | (pin << 8) | (1 << 13) | (3 << 16)
}

/// Pulse widths in microseconds: the smallest programmable, the nominal,
/// the Power-On Reset and Pace-Now widths, and the largest (Table 19-1).
const WIDTHS: [u32; 5] = [50, 400, 500, 1_000, 1_900];
const PER_GROUP: u32 = 200;
/// The arming lead of the spike: a start is written this far ahead.
const LEAD: u32 = 1_000;
/// The ventricular pulse starts this long after the atrial one.
const A_TO_V: u32 = 3_000;
/// Pause between two groups, so that a capture shows where a group begins.
const GROUP_GAP: u32 = 50_000;

fn w(addr: u32, value: u32) {
    // SAFETY: a register of the nRF52840's peripherals listed above.
    unsafe { write_volatile(addr as *mut u32, value) }
}

fn r(addr: u32) -> u32 {
    // SAFETY: as `w`.
    unsafe { read_volatile(addr as *const u32) }
}

/// The counter's present value, through capture register 2.
fn now() -> u32 {
    w(t_capture(2), 1);
    r(t_cc(2))
}

fn wait_until(instant: u32) {
    while (now().wrapping_sub(instant) as i32) < 0 {}
}

extern "C" fn reset() -> ! {
    // This silicon locks the debug port at every reset; a bench program
    // reopens it first thing, as rsk's HIL builds do (Verification_Plan,
    // "Access port protection"). Without this write the board has to be
    // unlocked by a full erase after every reset.
    w(APPROTECT_DISABLE, 0x5A);

    // The crystal oscillator, so that the timer's rate is the crystal's.
    w(EVENTS_HFCLKSTARTED, 0);
    w(TASKS_HFCLKSTART, 1);
    while r(EVENTS_HFCLKSTARTED) == 0 {}

    // The pace pins as GPIOTE task pins, low.
    w(gpiote_config(0), task_pin_port1(10));
    w(gpiote_config(1), task_pin_port1(11));

    // TIMER3: timer mode, 32 bits, 16 MHz / 2^4 = 1 MHz.
    w(T_MODE, 0);
    w(T_BITMODE, 3);
    w(T_PRESCALER, 4);

    // Compare 0 sets the atrial pin and compare 4 clears it; compare 1 sets
    // the ventricular pin and compare 5 clears it.
    let channels = [(0, gpiote_set(0)), (4, gpiote_clr(0)), (1, gpiote_set(1)), (5, gpiote_clr(1))];
    let mut n = 0;
    while n < 4 {
        let (compare, task) = channels[n as usize];
        w(ppi_eep(n), t_compare_event(compare));
        w(ppi_tep(n), task);
        n += 1;
    }
    w(PPI_CHENSET, 0xF);
    w(T_START, 1);

    loop {
        let mut g = 0;
        while g < WIDTHS.len() {
            let width = WIDTHS[g];
            let mut k = 0;
            while k < PER_GROUP {
                let a = now().wrapping_add(LEAD);
                let v = a.wrapping_add(A_TO_V);
                // The end before the start, each pair written ahead of time.
                w(t_cc(4), a.wrapping_add(width));
                w(t_cc(0), a);
                w(t_cc(5), v.wrapping_add(width));
                w(t_cc(1), v);
                wait_until(v.wrapping_add(width).wrapping_add(500));
                k += 1;
            }
            wait_until(now().wrapping_add(GROUP_GAP));
            g += 1;
        }
    }
}

extern "C" fn fault() -> ! {
    loop {}
}

/// The exception vectors after the initial stack pointer (which link.x
/// places): reset, then every core exception to `fault`.
#[link_section = ".vectors"]
#[no_mangle]
pub static VECTORS: [extern "C" fn() -> !; 15] =
    [reset, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault, fault];

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
