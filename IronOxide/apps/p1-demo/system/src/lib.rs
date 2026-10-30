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

#[rsk::bounds('digits = 10, 'copy = 10)]
fn write_num(buf: &mut [u8; 32], mut v: u32) -> usize {
    let mut tmp = [0u8; 10];
    let mut n = 0;
    if v == 0 {
        tmp[0] = b'0';
        n = 1;
    }
    'digits: while v > 0 {
        tmp[n] = b'0' + (v % 10) as u8;
        v /= 10;
        n += 1;
    }
    let mut i = 0;
    'copy: while i < n {
        buf[i] = tmp[n - 1 - i];
        i += 1;
    }
    n
}

#[rsk::bounds('key = 24, 'num = 10)]
fn print_kv(key: &[u8], v: u32) {
    let mut line = [0u8; 48];
    let mut n = 0;
    'key: for b in key {
        line[n] = *b;
        n += 1;
    }
    line[n] = b'=';
    n += 1;
    let mut num = [0u8; 32];
    let d = write_num(&mut num, v);
    let mut i = 0;
    'num: while i < d {
        line[n] = num[i];
        n += 1;
        i += 1;
    }
    line[n] = b'\n';
    n += 1;
    hw::semihosting_write(&line[..n]);
}

/// The system safe state of the demo (R14.4, R9.9): report and exit.
#[rsk::bounds('events = NE)]
pub fn report_and_exit() -> ! {
    let k = Sys::kernel().get();
    // The per-Task state cells the Generator emits (PR-25), read at the
    // Kernel level once every Job has ended.
    let a = p1_demo_part0::rsk_state::CONTROL_COUNT.peek();
    let b = p1_demo_part0::rsk_state::SENSOR_COUNT.peek();
    let c = p1_demo_part1::rsk_state::LOG_COUNT.peek();
    let d = p1_demo_part1::rsk_state::PULSE_COUNT.peek();
    let e = p1_demo_part0::rsk_state::ECHO_COUNT.peek();
    let blocked = p1_demo_part0::rsk_state::CONTROL_BLOCKED.peek();
    let f = p1_demo_part2::rsk_state::FILTER_COUNT.peek();
    let g = p1_demo_part2::rsk_state::SAMPLER_COUNT.peek();
    let dma_ok = p1_demo_part2::rsk_state::SAMPLER_DMA_OK.peek();
    // After 8 filter Jobs the state is 100 * (1 - 0.875^8) = 65.7; after 7,
    // 60.8: a state above 60 shows the FP context survived every Job
    // (R11.2; a lost context would restart from 0 or hold garbage).
    // Compared as IEEE-754 bit patterns (positive floats order as their
    // bits): the safe state is Kernel-reachable code, which executes no
    // floating-point comparison (R6.6, PR-31). 60.0 = 0x4270_0000,
    // 100.0 = 0x42C8_0000.
    let filter_bits = p1_demo_part2::rsk_state::FILTER_Y.peek().to_bits();
    let filter_ok = filter_bits > 0x4270_0000 && filter_bits < 0x42C8_0000;
    hw::semihosting_write(b"\nRSK-SAFE-STATE\n");
    print_kv(b"control", a);
    print_kv(b"sensor", b);
    print_kv(b"log", c);
    print_kv(b"pulse", d);
    print_kv(b"echo", e);
    print_kv(b"blocked_releases", blocked);
    print_kv(b"filter", f);
    print_kv(b"filter_state_bits", filter_bits);
    print_kv(b"sampler", g);
    print_kv(b"dma_ok", dma_ok);
    let len = k.log_len();
    print_kv(b"events", len as u32);
    let mut faults = 0u32;
    let mut ended = false;
    let mut misses = 0u32;
    let mut i = 0;
    'events: while i < len {
        let kind = k.log_kind(i);
        print_kv(b"event", kind as u32);
        print_kv(b"  task", k.log_task(i) as u32);
        print_kv(b"  at_ms", (k.log_instant(i) / MS) as u32);
        // The event's detail word: for a boot check, the failed checks (R12.2).
        print_kv(b"  detail", k.log_detail(i));
        // R3.9: the restriction identifiers of the event (bit n = PR-n).
        let prs = k.log_prs(i);
        print_kv(b"  prs_lo", prs as u32);
        print_kv(b"  prs_hi", (prs >> 32) as u32);
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
    // control Release observed while sensor held the Resource; the dual
    // timer started by log's first Job releases pulse every 18 ms (about 11
    // accepted events, none inside an MIT window: no MIT_Violation among
    // the faults, and none missed: the 15 ms deadline covers sensor's
    // interference and blocking). Every accepted event needs the source
    // unmasked again after the previous window (DD-06), so a low count
    // means the unmask event failed. control raises `tick` on Jobs 3, 6,
    // ..., 18: six raises, each a Release of echo (R10.5).
    // part2 (task 14): 8 filter Jobs (25 ms) with the FP state converging,
    // and 2 sampler Jobs (0 ms, 100 ms), each an accepted EasyDMA request.
    let ok = ended && faults == 0 && misses == 0 && a >= 19 && b >= 9 && c >= 4 && blocked >= 1 && d >= 9 && e >= 5
        && f >= 7 && filter_ok && g >= 2 && dma_ok >= 2;
    hw::semihosting_write(if ok { b"RSK-OK\n" } else { b"RSK-FAIL\n" });
    hw::semihosting_exit(if ok { 0 } else { 1 })
}
