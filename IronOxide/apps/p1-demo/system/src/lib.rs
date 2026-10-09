//! The P1 demo system crate: the tables of Generated_Config (R24.1) come
//! from the App_Declaration (`p1_demo_decl`, `rsk::app!`) through the
//! `rsk_system!` manifest; this crate adds only the demo's safe state,
//! which reports the counters and the event log and exits QEMU with 0
//! when every expectation holds (task 3.1 spike, task 10.5 build spike).
//!
//! Expected behaviour over the 200 ms operating duration: `control`
//! preempts `sensor` and `log`; while `sensor` holds `state` (Ceiling 7)
//! a Release of `control` is blocked until the unlock (SRP); the Kernel
//! then enters the safe state.
#![no_std]
#![forbid(unsafe_code)]

use rsk_kernel::arch::cm4::unsafe_module::hw;
use rsk_kernel::arch::{KernelOps, System};
use rsk_kernel::logic::log::{EV_DEADLINE_MISS, EV_OPERATING_END, EV_SAFE_STATE};

p1_demo_decl::rsk_system!();

/// Time-base ticks per millisecond on the board.
const MS: u64 = TICK_HZ / 1000;

fn write_num(buf: &mut [u8; 32], mut v: u32) -> usize {
    let mut tmp = [0u8; 10];
    let mut n = 0;
    if v == 0 {
        tmp[0] = b'0';
        n = 1;
    }
    while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    let mut i = 0;
    while i < n {
        buf[i] = tmp[n - 1 - i];
        i += 1;
    }
    n
}

fn print_kv(key: &[u8], v: u32) {
    let mut line = [0u8; 48];
    let mut n = 0;
    for b in key {
        line[n] = *b;
        n += 1;
    }
    line[n] = b'=';
    n += 1;
    let mut num = [0u8; 32];
    let d = write_num(&mut num, v);
    let mut i = 0;
    while i < d {
        line[n] = num[i];
        n += 1;
        i += 1;
    }
    line[n] = b'\n';
    n += 1;
    hw::semihosting_write(&line[..n]);
}

/// The system safe state of the demo (R14.4, R9.9): report and exit.
pub fn report_and_exit() -> ! {
    let k = Sys::kernel().get();
    let a = p1_demo_part0::COUNT_CONTROL.peek();
    let b = p1_demo_part0::COUNT_SENSOR.peek();
    let c = p1_demo_part1::COUNT_LOG.peek();
    let blocked = p1_demo_part0::BLOCKED_SEEN.peek();
    hw::semihosting_write(b"\nRSK-SAFE-STATE\n");
    print_kv(b"control", a);
    print_kv(b"sensor", b);
    print_kv(b"log", c);
    print_kv(b"blocked_releases", blocked);
    let len = k.log_len();
    print_kv(b"events", len as u32);
    let mut faults = 0u32;
    let mut ended = false;
    let mut misses = 0u32;
    let mut i = 0;
    while i < len {
        let kind = k.log_kind(i);
        print_kv(b"event", kind as u32);
        print_kv(b"  task", k.log_task(i) as u32);
        print_kv(b"  at_ms", (k.log_instant(i) / MS) as u32);
        if kind == EV_OPERATING_END {
            ended = true;
        } else if kind == EV_DEADLINE_MISS {
            misses += 1;
        } else if kind != 0 && kind != EV_SAFE_STATE {
            faults += 1;
        }
        i += 1;
    }
    // 200 ms: 20 control, 10 sensor, 5 log Releases; at least one blocked
    // control Release observed while sensor held the Resource.
    let ok = ended && faults == 0 && misses == 0 && a >= 19 && b >= 9 && c >= 4 && blocked >= 1;
    hw::semihosting_write(if ok { b"RSK-OK\n" } else { b"RSK-FAIL\n" });
    hw::semihosting_exit(if ok { 0 } else { 1 })
}
