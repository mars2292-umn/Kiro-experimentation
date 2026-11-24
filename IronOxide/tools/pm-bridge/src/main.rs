//! The bridge of the PACEMAKER experiment's DCM link (docs/pacemaker,
//! task 15.2; design PD-11, "The bridge").
//!
//! One privileged program on the rig's board C. It stands between the
//! pulse generator's SPI master (board A, SPIM1) and the DCM_Stub on the
//! host, and knows nothing of the frames it carries: 32 bytes go each way
//! in every SPI transfer, whatever they are.
//!
//! **SPI side.** SPIS1 on P0.28 (SCK, in), P0.29 (MOSI, in), P0.30 (MISO,
//! out). The master has no chip select of its own (an nRF52840 SPIM has
//! none, and the pulse generator's GPIO port belongs to another Partition
//! than its link's), so the bridge frames the transfers itself: it moves
//! its own chip-select pin (P0.31, by the pin's pull resistors). The
//! pin is low while the bridge waits for a transfer. Every rise of SCK
//! restarts TIMER0 through a PPI channel; when SCK has been still for
//! `IDLE_US` the compare fires, the bridge raises the pin, which ends the
//! SPIS's transaction, takes the bytes, gives the SPIS the bytes of the
//! next transfer, and lowers the pin again.
//!
//! **Host side.** UART0 through the board's debug probe (P0.06 out, P0.08
//! in), 115,200 baud, 8N1, no flow control. Messages in both directions
//! start with 0x7E and end with the sum, modulo 256, of the bytes between.
//!
//! Host to bridge, 35 bytes: `7E, type, 32 bytes, sum`.
//! - `C` (0x43): the 32 bytes are sent in the next transfer that has no
//!   earlier `C` waiting (a queue of `QUEUE`; a `C` beyond it is dropped
//!   and counted).
//! - `I` (0x49): the 32 bytes are sent in every transfer for which no `C`
//!   waits, until another `I` or an `X`.
//! - `X` (0x58): nothing waits and nothing is sent any more: the master
//!   reads zeros, which are no frame.
//! - `L` (0x4C): the 32 bytes come straight back in a report (the
//!   loop-back of the host side; nothing goes to the SPI side).
//!
//! Bridge to host, 38 bytes: `7E, count, flags, amount, dropped, 32 bytes, sum`.
//! `count` is the number of the report modulo 256; `flags` says what the
//! bridge sent in the transfer the report is of (`SENT_COMMAND`,
//! `SENT_IDLE`, neither: zeros), or `LOOPED` for the answer to an `L`;
//! `amount` is the number of bytes the master clocked; `dropped` the `C`
//! messages dropped and the reports not sent for want of room, modulo 256.
#![no_std]
#![no_main]

use core::ptr::{read_volatile, write_volatile};

const CLOCK: u32 = 0x4000_0000;
const UART0: u32 = 0x4000_2000;
const SPIS1: u32 = 0x4000_4000;
const GPIOTE: u32 = 0x4000_6000;
const TIMER0: u32 = 0x4000_8000;
const PPI: u32 = 0x4001_F000;
const P0: u32 = 0x5000_0000;

/// APPROTECT.DISABLE: 0x5A keeps the access port open until the next reset.
const APPROTECT_DISABLE: u32 = 0x4000_0558;

const SCK: u32 = 28;
const MOSI: u32 = 29;
const MISO: u32 = 30;
const CSN: u32 = 31;
const UART_TX: u32 = 6;
const UART_RX: u32 = 8;

/// SCK still for this long ends a transfer. The master clocks 32 bytes
/// without a gap (EasyDMA) at 500 kbps, 2 µs a bit.
const IDLE_US: u32 = 300;
const FRAME: usize = 32;
const QUEUE: usize = 8;
const TX_RING: usize = 512;

/// The chip-select pin's two configurations: an input pulled up, and
/// pulled down. The enabled SPIS makes its CSN pin an input whatever the
/// GPIO's direction says (driven high as an output it read low: measured,
/// 2026-11-24), so the program moves the pin by its pull resistor. Nothing
/// else drives the line: the pulse generator's end of the wire is an
/// input with its buffer disconnected.
const CSN_HIGH: u32 = 0b1100;
const CSN_LOW: u32 = 0b0100;

const START: u8 = 0x7E;
const SENT_COMMAND: u8 = 1;
const SENT_IDLE: u8 = 2;
const LOOPED: u8 = 0x80;

fn w(addr: u32, value: u32) {
    // SAFETY: a register of the nRF52840's peripherals named above.
    unsafe { write_volatile(addr as *mut u32, value) }
}

fn r(addr: u32) -> u32 {
    // SAFETY: as `w`.
    unsafe { read_volatile(addr as *const u32) }
}

/// Waits for an event register, at most `polls` reads; whether it came.
fn wait(event: u32, polls: u32) -> bool {
    let mut i = 0;
    while i < polls {
        if r(event) != 0 {
            return true;
        }
        i += 1;
    }
    false
}

/// The bytes to the host: a ring the main loop empties into the UART one
/// byte at a time, so that no part of the loop waits for the UART.
struct Out {
    ring: [u8; TX_RING],
    head: usize,
    len: usize,
    busy: bool,
    count: u8,
    dropped: u8,
}

impl Out {
    fn report(&mut self, flags: u8, amount: u8, bytes: &[u8; FRAME]) {
        if self.len + FRAME + 6 > TX_RING {
            self.dropped = self.dropped.wrapping_add(1);
            return;
        }
        let head = [START, self.count, flags, amount, self.dropped];
        self.count = self.count.wrapping_add(1);
        let mut sum = 0u8;
        for (k, b) in head.iter().chain(bytes.iter()).enumerate() {
            if k > 0 {
                sum = sum.wrapping_add(*b);
            }
            self.push(*b);
        }
        self.push(sum);
    }

    fn push(&mut self, b: u8) {
        self.ring[(self.head + self.len) % TX_RING] = b;
        self.len += 1;
    }

    fn pump(&mut self) {
        if self.busy && r(UART0 + 0x11C) != 0 {
            w(UART0 + 0x11C, 0);
            self.busy = false;
        }
        if !self.busy && self.len > 0 {
            w(UART0 + 0x51C, self.ring[self.head] as u32);
            self.head = (self.head + 1) % TX_RING;
            self.len -= 1;
            self.busy = true;
        }
    }
}

/// Gives the SPIS the bytes of the next transfer and lowers the chip
/// select. The semaphore is the program's on entry (the END_ACQUIRE
/// shortcut, or the acquisition at start-up).
fn arm(tx: &[u8; FRAME], rx: &mut [u8; FRAME]) {
    w(SPIS1 + 0x544, tx.as_ptr() as u32);
    w(SPIS1 + 0x548, FRAME as u32);
    w(SPIS1 + 0x534, rx.as_mut_ptr() as u32);
    w(SPIS1 + 0x538, FRAME as u32);
    w(SPIS1 + 0x104, 0);
    w(SPIS1 + 0x128, 0);
    w(TIMER0 + 0x140, 0);
    w(SPIS1 + 0x028, 1);
    w(P0 + 0x700 + 4 * CSN, CSN_LOW);
}

extern "C" fn reset() -> ! {
    w(APPROTECT_DISABLE, 0x5A);
    // The crystal oscillator, for the UART's rate.
    w(CLOCK + 0x100, 0);
    w(CLOCK, 1);
    while r(CLOCK + 0x100) == 0 {}

    // Pins. The chip select is high until armed.
    w(P0 + 0x508, 1 << UART_TX);
    w(P0 + 0x700 + 4 * CSN, CSN_HIGH);
    w(P0 + 0x700 + 4 * SCK, 0);
    w(P0 + 0x700 + 4 * MOSI, 0);
    w(P0 + 0x700 + 4 * MISO, 0);
    w(P0 + 0x700 + 4 * UART_TX, 0b11);
    w(P0 + 0x700 + 4 * UART_RX, 0);

    // UART0: 115,200 baud, 8N1, no flow control.
    w(UART0 + 0x50C, UART_TX);
    w(UART0 + 0x514, UART_RX);
    w(UART0 + 0x524, 0x01D7_E000);
    w(UART0 + 0x500, 4);
    w(UART0 + 0x000, 1);
    w(UART0 + 0x008, 1);

    // TIMER0 at 1 MHz, restarted by every rise of SCK through PPI channel
    // 0 and its fork; it stops itself at the compare. The rise is the
    // port event of the pin's level detection, which leaves the pin to
    // the SPIS (a GPIOTE channel on the pin was not tried to the end).
    w(TIMER0 + 0x504, 0);
    w(TIMER0 + 0x508, 3);
    w(TIMER0 + 0x510, 4);
    w(TIMER0 + 0x540, IDLE_US);
    w(TIMER0 + 0x200, 1 << 8);
    w(P0 + 0x700 + 4 * SCK, 2 << 16);
    w(PPI + 0x510, GPIOTE + 0x17C);
    w(PPI + 0x514, TIMER0 + 0x00C);
    w(PPI + 0x910, TIMER0 + 0x000);
    w(PPI + 0x504, 1);

    // SPIS1: mode 0, most significant bit first; zeros when it has nothing.
    w(SPIS1 + 0x508, SCK);
    w(SPIS1 + 0x50C, MISO);
    w(SPIS1 + 0x510, MOSI);
    w(SPIS1 + 0x514, CSN);
    w(SPIS1 + 0x554, 0);
    w(SPIS1 + 0x55C, 0);
    w(SPIS1 + 0x5C0, 0);
    w(SPIS1 + 0x200, 1 << 2);
    w(SPIS1 + 0x500, 2);
    w(SPIS1 + 0x128, 0);
    w(SPIS1 + 0x024, 1);
    let _ = wait(SPIS1 + 0x128, 100_000);

    run()
}

fn run() -> ! {
    let zeros = [0u8; FRAME];
    let mut tx = [0u8; FRAME];
    let mut rx = [0u8; FRAME];
    let mut idle = [0u8; FRAME];
    let mut have_idle = false;
    let mut queue = [[0u8; FRAME]; QUEUE];
    let mut q_head = 0usize;
    let mut q_len = 0usize;
    let mut sent = 0u8;
    let mut out = Out { ring: [0; TX_RING], head: 0, len: 0, busy: false, count: 0, dropped: 0 };
    // The message being received: its bytes after the start byte.
    let mut msg = [0u8; FRAME + 2];
    let mut got = 0usize;
    let mut in_msg = false;

    arm(&tx, &mut rx);
    loop {
        out.pump();

        // A byte from the host.
        if r(UART0 + 0x108) != 0 {
            w(UART0 + 0x108, 0);
            let b = r(UART0 + 0x518) as u8;
            if !in_msg {
                if b == START {
                    in_msg = true;
                    got = 0;
                }
            } else {
                msg[got] = b;
                got += 1;
                if got == FRAME + 2 {
                    in_msg = false;
                    let mut sum = 0u8;
                    let mut body = [0u8; FRAME];
                    for k in 0..FRAME + 1 {
                        sum = sum.wrapping_add(msg[k]);
                        if k > 0 {
                            body[k - 1] = msg[k];
                        }
                    }
                    if sum == msg[FRAME + 1] {
                        match msg[0] {
                            b'C' => {
                                if q_len < QUEUE {
                                    queue[(q_head + q_len) % QUEUE] = body;
                                    q_len += 1;
                                } else {
                                    out.dropped = out.dropped.wrapping_add(1);
                                }
                            }
                            b'I' => {
                                idle = body;
                                have_idle = true;
                            }
                            b'X' => {
                                have_idle = false;
                                q_len = 0;
                            }
                            b'L' => out.report(LOOPED, FRAME as u8, &body),
                            _ => {}
                        }
                    }
                }
            }
        }

        // SCK has been still for IDLE_US after a transfer: end it.
        if r(TIMER0 + 0x140) != 0 {
            w(TIMER0 + 0x140, 0);
            w(P0 + 0x700 + 4 * CSN, CSN_HIGH);
            let ended = wait(SPIS1 + 0x104, 10_000);
            let acquired = wait(SPIS1 + 0x128, 10_000);
            if !acquired {
                // No transaction was granted: take the semaphore.
                w(SPIS1 + 0x024, 1);
                let _ = wait(SPIS1 + 0x128, 100_000);
            }
            let amount = if ended { r(SPIS1 + 0x53C) as u8 } else { 0 };
            // The SPIS wrote `rx` behind the compiler's back.
            let mut bytes = [0u8; FRAME];
            for (k, b) in bytes.iter_mut().enumerate() {
                // SAFETY: a byte of `rx`, which is live.
                *b = unsafe { read_volatile(rx.as_ptr().add(k)) };
            }
            out.report(sent, amount, &bytes);
            if q_len > 0 {
                tx = queue[q_head];
                q_head = (q_head + 1) % QUEUE;
                q_len -= 1;
                sent = SENT_COMMAND;
            } else if have_idle {
                tx = idle;
                sent = SENT_IDLE;
            } else {
                tx = zeros;
                sent = 0;
            }
            rx = zeros;
            arm(&tx, &mut rx);
        }
    }
}

extern "C" fn fault() -> ! {
    loop {}
}

#[repr(C)]
#[derive(Clone, Copy)]
pub union Vector {
    diverges: extern "C" fn() -> !,
    reserved: usize,
}

// SAFETY: a table of code addresses, never written.
unsafe impl Sync for Vector {}

#[link_section = ".vectors"]
#[no_mangle]
pub static VECTORS: [Vector; 15 + 48] = {
    let mut v = [Vector { diverges: fault }; 15 + 48];
    v[0] = Vector { diverges: reset };
    v
};

#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    loop {}
}
