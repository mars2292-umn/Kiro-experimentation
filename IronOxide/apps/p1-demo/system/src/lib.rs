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
use rsk_kernel::logic::log::{EV_DEADLINE_MISS, EV_OPERATING_END, EV_SAFE_STATE, EV_DEBUGGER};
#[cfg(any(feature = "timing", feature = "timing-short", feature = "stress", feature = "stress-short"))]
use rsk_kernel::logic::log::{EV_MIT_VIOLATION, EV_OVERRUN, EV_RELEASE_OVERLAP};
#[cfg(any(feature = "stress", feature = "stress-short"))]
use rsk_kernel::logic::log::{EV_END_JOB, EV_PARTITION_RESTART, EV_RESOURCE_RELEASED};
#[cfg(feature = "isolation")]
use rsk_kernel::logic::log::{EV_BUSFAULT, EV_DMA_REJECTED, EV_END_JOB, EV_MEMMANAGE, EV_MIT_VIOLATION, EV_OVERRUN, EV_PARTITION_RESTART, EV_RELEASE_OVERLAP, EV_RESOURCE_RELEASED};

p1_demo_decl::rsk_system!();

/// The events the HIL_Rig's trace freeze treats as the run's own (the
/// Kernel's event mirror, feature `trace-ram`): in the release-timing
/// configuration (task 7.3) the injected late Jobs of `sensor` (Task 2)
/// produce Budget overruns, deadline misses, held Releases, and
/// MIT_Violations; any other event freezes the Kernel trace at its cause.
/// The plain demo expects no event at all.
/// The third member names the Partitions (bit p) whose events freeze the
/// trace whatever their kind: a Partition restart names no Task, so a
/// Task-based expectation never saw one of part0 (task 19.1).
#[cfg(any(feature = "timing", feature = "timing-short"))]
pub const TRACE_EXPECT: (u32, u8, u32) =
    (1 << EV_OVERRUN | 1 << EV_DEADLINE_MISS | 1 << EV_MIT_VIOLATION | 1 << EV_RELEASE_OVERLAP, 2, 0);
// The fault-injection suite (task 7.5): the response configuration's
// events are all injected (the rig checks the exact sequence); the stress
// configuration's are the lower-criticality Partitions' own (overruns,
// ended Jobs, released Resources, restarts; held and discarded Releases
// and misses of their delayed Jobs), and any event of part0 freezes the
// trace (the rig checks part0 separately).
#[cfg(feature = "response")]
pub const TRACE_EXPECT: (u32, u8, u32) = (u32::MAX, 0xFF, 0);
#[cfg(any(feature = "stress", feature = "stress-short"))]
pub const TRACE_EXPECT: (u32, u8, u32) = (
    1 << EV_OVERRUN | 1 << EV_DEADLINE_MISS | 1 << EV_MIT_VIOLATION | 1 << EV_RELEASE_OVERLAP
        | 1 << EV_END_JOB | 1 << EV_RESOURCE_RELEASED | 1 << EV_PARTITION_RESTART,
    0xFF,
    1 << 0,
);
// The isolation suite (task 7.4): every attempt of the access matrix is
// a MemManage or BusFault with its END_JOB, every refused EasyDMA request
// a `dma_rejected` with its END_JOB, and filter's injected overrun at
// 3.3 s restarts part2 (its held Resource released, its delayed Jobs'
// misses and held Releases); anything else freezes the trace.
#[cfg(feature = "isolation")]
pub const TRACE_EXPECT: (u32, u8, u32) = (
    1 << EV_MEMMANAGE | 1 << EV_BUSFAULT | 1 << EV_END_JOB | 1 << EV_DMA_REJECTED | 1 << EV_OVERRUN
        | 1 << EV_PARTITION_RESTART | 1 << EV_RESOURCE_RELEASED | 1 << EV_DEADLINE_MISS | 1 << EV_RELEASE_OVERLAP
        | 1 << EV_MIT_VIOLATION,
    0xFF,
    0,
);
#[cfg(not(any(feature = "timing", feature = "timing-short", feature = "response", feature = "stress", feature = "stress-short", feature = "isolation")))]
pub const TRACE_EXPECT: (u32, u8, u32) = (0, 0xFF, 0);
// (The cycle-attribution configuration expects no event, as the plain demo.)

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
#[rsk::bounds('events = NE, 'part0 = NE)]
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
    // No debugger (a board left running after the probe detached): no
    // console to report to, and a semihosting breakpoint would fault; the
    // safe state is the idle loop alone (ORQ-30, ORQ-31).
    if !hw::debugger_attached() {
        hw::semihosting_exit(2);
    }
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
    // The isolation suite (task 7.4) and the differential tests (task
    // 3.3): what the probes saw, judged by the rig (Verification_Plan 11.7).
    #[cfg(feature = "isolation")]
    {
        print_kv(b"control_matrix", p1_demo_part0::rsk_state::CONTROL_MATRIX.peek());
        print_kv(b"control_survived", p1_demo_part0::rsk_state::CONTROL_SURVIVED.peek());
        print_kv(b"log_matrix", p1_demo_part1::rsk_state::LOG_MATRIX.peek());
        print_kv(b"log_survived", p1_demo_part1::rsk_state::LOG_SURVIVED.peek());
        print_kv(b"sampler_dma_case", p1_demo_part2::rsk_state::SAMPLER_DMA_CASE.peek());
        print_kv(b"sampler_dma_ptr", p1_demo_part2::rsk_state::SAMPLER_DMA_PTR.peek());
        print_kv(b"sampler_survived", p1_demo_part2::rsk_state::SAMPLER_SURVIVED.peek());
        print_kv(b"filter_pattern_runs", p1_demo_part2::rsk_state::FILTER_PATTERN_RUNS.peek());
        print_kv(b"filter_pattern_fail", p1_demo_part2::rsk_state::FILTER_PATTERN_FAIL.peek());
        print_kv(b"filter_pattern_mask", p1_demo_part2::rsk_state::FILTER_PATTERN_MASK.peek());
        print_kv(b"filter_fpscr", p1_demo_part2::rsk_state::FILTER_FPSCR.peek());
        // SPIM0's EasyDMA registers (nRF52840 PS 6.25.7), read here because
        // the reset the board takes when the probe detaches clears them:
        // channel 0 (RXD) holds the last accepted request's buffer and
        // count, channel 1 (TXD) was never programmed, the LIST registers
        // are clear (DD-08), and no transfer was ever started (R17.2: the
        // refused requests reached no register; the Kernel programs the
        // pointer and count only).
        const SPIM0: u32 = 0x4000_3000;
        print_kv(b"spim0_rxd_ptr", hw::mmio_read(SPIM0 + 0x534));
        print_kv(b"spim0_rxd_maxcnt", hw::mmio_read(SPIM0 + 0x538));
        print_kv(b"spim0_rxd_amount", hw::mmio_read(SPIM0 + 0x53C));
        print_kv(b"spim0_rxd_list", hw::mmio_read(SPIM0 + 0x540));
        print_kv(b"spim0_txd_ptr", hw::mmio_read(SPIM0 + 0x544));
        print_kv(b"spim0_txd_maxcnt", hw::mmio_read(SPIM0 + 0x548));
        print_kv(b"spim0_txd_list", hw::mmio_read(SPIM0 + 0x550));
        print_kv(b"spim0_enable", hw::mmio_read(SPIM0 + 0x500));
        print_kv(b"spim0_events_started", hw::mmio_read(SPIM0 + 0x14C));
        print_kv(b"spim0_events_end", hw::mmio_read(SPIM0 + 0x118));
        print_kv(b"spim0_events_stopped", hw::mmio_read(SPIM0 + 0x104));
    }
    // The first MemManage fault's status and address (HIL diagnosis, R14.3).
    let (cfsr, mmfar) = rsk_kernel::arch::cm4::first_memmanage();
    print_kv(b"first_memmanage_cfsr", cfsr);
    print_kv(b"first_memmanage_mmfar", mmfar);
    let len = k.log_len();
    print_kv(b"events", len as u32);
    let mut faults = 0u32;
    let mut ended = false;
    let mut misses = 0u32;
    // The release-timing configuration (task 7.3): the injected late
    // Jobs of `sensor` (Task 2) produce its Budget overruns, its deadline
    // misses, its held Releases (R9.7, EV_RELEASE_OVERLAP), and Partition
    // 0's MIT_Violations for the discarded ones (R9.8); those are expected
    // and counted apart, anything else is a fault.
    #[cfg(any(feature = "timing", feature = "timing-short"))]
    let mut expected_late = 0u32;
    let mut i = 0;
    'events: while i < len {
        let kind = k.log_kind(i);
        let task = k.log_task(i);
        #[cfg(any(feature = "timing", feature = "timing-short"))]
        if task == 2 && (kind == EV_OVERRUN || kind == EV_DEADLINE_MISS || kind == EV_MIT_VIOLATION || kind == EV_RELEASE_OVERLAP) {
            expected_late += 1;
            i += 1;
            continue;
        }
        print_kv(b"event", kind as u32);
        print_kv(b"  task", task as u32);
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
        } else if kind != 0 && kind != EV_SAFE_STATE && kind != EV_DEBUGGER {
            // EV_DEBUGGER is information (ORQ-30): a probe is attached on the
            // HIL bench, never on QEMU.
            faults += 1;
        }
        i += 1;
    }
    // 200 ms: 20 control, 10 sensor, 5 log Releases; at least one blocked
    // control Release observed while sensor held the Resource; the source
    // started by log's first Job (QEMU: the dual timer; the nRF52840-DK: the
    // HIL_Rig's stimulus board on GPIOTE) releases pulse every 18 ms (about 11
    // accepted events, none inside an MIT window: no MIT_Violation among
    // the faults, and none missed: the 15 ms deadline covers sensor's
    // interference and blocking). Every accepted event needs the source
    // unmasked again after the previous window (DD-06), so a low count
    // means the unmask event failed. control raises `tick` on Jobs 3, 6,
    // ..., 18: six raises, each a Release of echo (R10.5).
    // part2 (task 14): 8 filter Jobs (25 ms) with the FP state converging,
    // and 2 sampler Jobs (0 ms, 100 ms), each an accepted EasyDMA request.
    // The cycle-attribution configuration (task 7.5): 2 seconds without a
    // fault or a miss; the rig judges the executions from the record.
    #[cfg(feature = "cycles")]
    let ok = {
        let _ = (filter_ok, a, b, c, d, e, f, g, dma_ok, blocked);
        ended && faults == 0 && misses == 0
    };
    #[cfg(not(any(feature = "timing", feature = "timing-short", feature = "response", feature = "stress", feature = "stress-short", feature = "isolation", feature = "cycles")))]
    let ok = ended && faults == 0 && misses == 0 && a >= 19 && b >= 9 && c >= 4 && blocked >= 1 && d >= 9 && e >= 5
        && f >= 7 && filter_ok && g >= 2 && dma_ok >= 2;
    // The release-timing configuration: every count within two Jobs of
    // the operating duration over the period (the last Jobs end with the
    // duration), `sensor` short by the Releases its late Jobs discarded
    // (one per 1024 Jobs), `pulse` released by the stimulus board at 18 ms
    // (not counted here: it depends on the rig), no fault and no miss
    // other than the late Jobs' own, and the FP state converged.
    // The fault-injection suite (task 7.5): the response configuration's
    // verdict is the rig's, from the event mirror and the counts (every
    // event here is injected; the run ends in SAFE_STATE by design); the
    // stress configuration's own verdict covers part0 only (level A: its
    // counts exact and no event of its Tasks; the rig adds the WCRT and
    // the storm checks), the misbehaving Partitions' counts being theirs.
    #[cfg(feature = "response")]
    let ok = {
        let _ = (ended, faults, misses, filter_ok);
        true
    };
    // The isolation configuration: the verdict is the rig's (the event
    // mirror, the fault record, the differential-test record, the
    // registers); the target reports that the run ended.
    #[cfg(feature = "isolation")]
    let ok = {
        let _ = (faults, misses, filter_ok);
        ended
    };
    #[cfg(any(feature = "stress", feature = "stress-short"))]
    let ok = {
        let duration = CONFIG.operating_duration;
        let jobs = |t: usize| (duration / CONFIG.tasks[t].period) as u32;
        print_kv(b"expected_control", jobs(0));
        print_kv(b"expected_sensor", jobs(2));
        let mut part0_events = 0u32;
        let mut i = 0;
        'part0: while i < len {
            if k.log_task(i) <= 2 {
                part0_events += 1;
            }
            i += 1;
        }
        print_kv(b"part0_events", part0_events);
        let _ = (faults, misses, filter_ok);
        ended && part0_events == 0 && a + 2 >= jobs(0) && a <= jobs(0) && b + 2 >= jobs(2) && b <= jobs(2) && blocked >= 1
    };
    #[cfg(any(feature = "timing", feature = "timing-short"))]
    let ok = {
        let duration = CONFIG.operating_duration;
        let jobs = |t: usize| (duration / CONFIG.tasks[t].period) as u32;
        let late = jobs(2) / 1024;
        print_kv(b"expected_control", jobs(0));
        print_kv(b"expected_sensor", jobs(2) - late);
        print_kv(b"expected_late_jobs", late);
        print_kv(b"late_events_seen", expected_late);
        ended && faults == 0 && misses == 0
            && a + 2 >= jobs(0) && b + 2 >= jobs(2) - late && b <= jobs(2) - late
            && c + 2 >= jobs(3) && f + 2 >= jobs(5) && filter_ok && g + 2 >= jobs(6) && dma_ok + 2 >= jobs(6)
            && blocked >= 1 && d >= 9
    };
    hw::semihosting_write(if ok { b"RSK-OK\n" } else { b"RSK-FAIL\n" });
    hw::semihosting_exit(if ok { 0 } else { 1 })
}
