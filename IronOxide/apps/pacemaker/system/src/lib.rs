//! The pulse generator's system crate: the tables of Generated_Config come
//! from the App_Declaration (`pacemaker_decl`, `rsk::app!`) through the
//! `rsk_system!` manifest; this crate adds the safe state.
//!
//! On the board the safe state idles with the watchdog fed: the Kernel has
//! stopped every Task, and the pace pins are low because nothing arms a
//! compare any more and a pulse in progress ends at its own end compare
//! (PM23.5, PM27.4; shown on the rig in stage E3).
//!
//! On QEMU the safe state at the end of the operating duration is the
//! report of the run (task 7.3): the event ring against the sequence the
//! script of Senses must give, and the exit status 0 when they agree.
#![no_std]
#![forbid(unsafe_code)]

use rsk_kernel::arch::cm4::unsafe_module::hw;

pacemaker_decl::rsk_system!();

/// The trace freeze's expectation (the Kernel's event mirror, feature
/// `trace-ram`): no Health_Monitor event is the Application's own.
pub const TRACE_EXPECT: (u32, u8, u32) = (0, 0xFF, 0);

/// The system safe state on the board (R14.4): quiet, watchdog fed, forever.
#[cfg(feature = "nrf52840")]
pub fn safe_state() -> ! {
    loop {
        hw::safe_state_idle()
    }
}

/// The sequence of a QEMU run in an inhibited Mode (VVI, AAI; the script
/// of `pacemaker_therapy::board::SCRIPT` at 120 ppm), in therapy
/// microseconds, all in the paced Chamber:
///
/// ```text
/// S 300,000   S 450,000     P 800,000   P 1,300,000   S 1,700,000   P 2,200,000   P 2,700,000
///   |- refractory -|           |-- 500 ms --|              |---- 500 ms ----|-- 500 ms --|
///   |--------- 500 ms ---------|
/// ```
///
/// The Sense of the script in the other Chamber, at 1,000,000, leaves no
/// record: a single-chamber Mode does not sense it.
///
/// The Sense at 1,700,000 comes 400 ms after the Pace before it, which is
/// shorter than the URL interval (500 ms at the nominal 120 ppm). In the
/// atrium that is the Marker AT (PM18.5; `SENSE_AT_1_7`): it inhibits as
/// any non-refractory Sense does. Until 2026-11-27 the AAI run expected a
/// plain Sense there, as before the AT Marker existed, and failed on it.
#[cfg(feature = "qemu-aai")]
const SENSE_AT_1_7: u8 = pacemaker_therapy::kind::AT;
#[cfg(not(any(feature = "nrf52840", feature = "qemu-voo", feature = "qemu-aoo", feature = "qemu-aai")))]
const SENSE_AT_1_7: u8 = pacemaker_therapy::kind::SENSE;
#[cfg(not(any(feature = "nrf52840", feature = "qemu-voo", feature = "qemu-aoo")))]
const INHIBITED: [(u8, u32); 7] = {
    use pacemaker_therapy::kind::{PACE, SENSE, SENSE_REFRACTORY};
    [(SENSE, 300_000), (SENSE_REFRACTORY, 450_000), (PACE, 800_000), (PACE, 1_300_000), (SENSE_AT_1_7, 1_700_000), (PACE, 2_200_000), (PACE, 2_700_000)]
};

#[cfg(not(feature = "nrf52840"))]
fn print_number(label: &[u8], v: u32) {
    let mut line = [0u8; 48];
    let mut n = 0;
    while n < label.len() && n < 32 {
        line[n] = label[n];
        n += 1;
    }
    line[n] = b'=';
    n += 1;
    let mut digits = [0u8; 10];
    let mut d = 0;
    let mut x = v;
    loop {
        digits[d] = b'0' + (x % 10) as u8;
        d += 1;
        x /= 10;
        if x == 0 || d == 10 {
            break;
        }
    }
    while d > 0 {
        d -= 1;
        line[n] = digits[d];
        n += 1;
    }
    line[n] = b'\n';
    hw::semihosting_write(&line[..n + 1]);
}

/// The system safe state on QEMU: the report, and the exit.
#[cfg(not(feature = "nrf52840"))]
pub fn safe_state() -> ! {
    let t = pacemaker_therapy::rsk_resources::TIMING.peek();
    hw::semihosting_write(b"\nPM-SAFE-STATE\n");
    print_number(b"started", t.started as u32);
    print_number(b"hw_fault", t.hw_fault as u32);
    print_number(b"mode_paces_ventricle", t.state.params().mode.paces(pm_core::Chamber::V) as u32);
    print_number(b"mode_senses", (t.state.params().mode.senses(pm_core::Chamber::A) || t.state.params().mode.senses(pm_core::Chamber::V)) as u32);
    print_number(b"escape_us", t.state.params().lrl);
    print_number(b"records", t.written);
    print_number(b"too_late", t.counts.too_late);
    print_number(b"moved", t.counts.moved);
    print_number(b"forced", t.counts.late);
    let paced = pacemaker_therapy::board::PACED;
    print_number(b"paced_chamber", paced as u32);
    let mut ok = t.started && !t.hw_fault && t.counts.late == 0 && t.counts.dropped == 0;
    let mut i = 0;
    while i < pacemaker_therapy::RING && i < t.written as usize {
        let r = t.ring[i];
        print_number(match r.kind {
            pacemaker_therapy::kind::PACE => b"pace_at",
            pacemaker_therapy::kind::PACE_FUSED => b"fused_pace_at",
            pacemaker_therapy::kind::SENSE => b"sense_at",
            pacemaker_therapy::kind::SENSE_REFRACTORY => b"refractory_sense_at",
            pacemaker_therapy::kind::AT => b"at_sense_at",
            _ => b"other_record_at",
        }, r.at);
        // Every record is of the paced Chamber.
        if r.chamber != paced {
            ok = false;
        }
        i += 1;
    }
    let pending = t.state.chamber[paced as usize].pending;
    print_number(b"pending_at", (pending.at.0 % 0x1_0000_0000) as u32);
    if t.state.chamber[1 - paced as usize].pending.armed {
        ok = false;
    }
    // An inhibited Mode: exactly the sequence above, and the next Pace
    // pending one escape interval after the last.
    #[cfg(not(any(feature = "qemu-voo", feature = "qemu-aoo")))]
    {
        if t.written as usize != INHIBITED.len() || !(pending.armed && pending.at.0 == 3_200_000) {
            ok = false;
        }
        let mut k = 0;
        while k < INHIBITED.len() && k < t.written as usize {
            if (t.ring[k].kind, t.ring[k].at) != INHIBITED[k] {
                ok = false;
            }
            k += 1;
        }
    }
    // An asynchronous Mode (VOO, AOO): no Sense leaves a record; the first
    // Pace is one escape interval after `control`'s first tick, which is
    // within the first period, and every Pace follows the one before by
    // exactly the escape interval, to the end of the 3 s.
    #[cfg(any(feature = "qemu-voo", feature = "qemu-aoo"))]
    {
        let n = t.written as usize;
        if n != 5 || !(pending.armed && n > 0 && (pending.at.0 % 0x1_0000_0000) as u32 == t.ring[n - 1].at.wrapping_add(500_000)) {
            ok = false;
        }
        let mut k = 0;
        while k < n && k < pacemaker_therapy::RING {
            let r = t.ring[k];
            if r.kind != pacemaker_therapy::kind::PACE {
                ok = false;
            }
            if k == 0 && !(r.at >= 500_000 && r.at < 520_000) {
                ok = false;
            }
            if k > 0 && r.at != t.ring[k - 1].at.wrapping_add(500_000) {
                ok = false;
            }
            k += 1;
        }
    }
    if ok {
        hw::semihosting_write(b"PM-OK\n");
        hw::semihosting_exit(0)
    } else {
        hw::semihosting_write(b"PM-FAIL\n");
        hw::semihosting_exit(1)
    }
}
