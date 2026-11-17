//! P1 demo Partition 2 (`part2`, level B; task 14, R3.7): `filter`
//! (Priority 3, every 25 ms) keeps a floating-point state across Jobs, so
//! its Jobs carry an active FP context that the Kernel stacks and restores
//! (R11, PR-31; the `fpu` flag of the declaration), and `sampler`
//! (Priority 1, every 100 ms) requests an EasyDMA transfer of a buffer in
//! its own RAM through `rsk::DmaBuf::lend` (DD-08, R17): the Kernel checks
//! the Partition's ownership of the channel and the buffer's permission and
//! programs the channel's registers (on QEMU, the GPIO0 stand-in). Every
//! value that outlives a Job is declared per-Task state (PR-25).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part2!();

/// The fault-injection suite (task 7.5): whether the time base is inside
/// `[from, from + len)` milliseconds, and a busy wait of `ms` milliseconds
/// by the time base (bounded in iterations too, PR-16). Injections are
/// placed by instant, not by Job count, so that a restarted Partition
/// (its counts back to 0) does not repeat them.
#[cfg(any(feature = "response", feature = "stress", feature = "isolation"))]
mod inject {
    #[cfg(any(feature = "response", feature = "isolation"))]
    pub fn window(from_ms: u64, len_ms: u64) -> bool {
        let now = rsk::now();
        // Saturating: an obligation the App_Prover cannot discharge otherwise (task 17.1).
        now >= super::rsk_target::ms(from_ms) && now < super::rsk_target::ms(from_ms.saturating_add(len_ms))
    }

    /// Returns the iterations the loop ran (the loop report, task 19.2, R35.4);
    /// the label names the loop there, nothing breaks to it.
    #[allow(unused_labels)]
    pub fn spin_ms(ms: u64) -> u32 {
        let start = rsk::now();
        let mut i = 0u32;
        // R35.2: the App_Prover's bound equals the derived Loop_Bound_Annotation.
        #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(i@ <= 100_000_000))]
        #[cfg_attr(creusot, rsk::creusot_std::macros::variant(100_000_000 - i@))]
        'spin: while rsk::now().wrapping_sub(start) < super::rsk_target::ms(ms) && i < 100_000_000 {
            i += 1;
        }
        i
    }

    /// A write into the Kernel's RAM, outside every Partition's view: a
    /// MemManage fault attributed to the Partition (R14.3).
    #[cfg(feature = "response")]
    pub fn fault() {
        rsk::mmio::write(0x2000_0000, 1);
    }
}

/// The channel index of CMSDK_GPIO0's stand-in channel in the system's DMA
/// table (the only EasyDMA channel of the declaration).
// Unused under the measurement configurations alone (the traffic scenario
// names its channels, `traffic::CHANNEL_*`).
#[allow(dead_code)]
const DMA_CHANNEL: u8 = 0;

/// The plain demo's request (R17.1, R17.5): the first Job of the
/// Partition's incarnation writes its buffer and lends it to the channel;
/// the transfer is held in the Task's state from then on, and while it is
/// held the bytes are the peripheral's (`rsk::DmaBuf::bytes` gives none).
/// Returns whether the Task holds an accepted transfer.
// App_Prover (task 17.1): as `traffic_step`, the step leaves the count and
// the Resource as they were.
#[cfg(all(not(feature = "isolation"), not(feature = "measure")))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).count@ == (*cx).count@))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).stats@ == (*cx).stats@))]
fn plain_request(cx: &mut rsk_ctx::Sampler, n: u8) -> bool {
    if cx.held.with(|h| h.is_some()) {
        return true;
    }
    let transfer = cx.buf.with(|b| {
        if let Some(bytes) = b.bytes() {
            bytes[0] = n;
        }
        b.lend(rsk_target::DMA[0])
    });
    let accepted = transfer.is_some();
    cx.held.with(|h| h.put(transfer));
    accepted
}

/// Two of `rsk::DmaGuarded::footprint`'s verdicts (`WRITTEN_INSIDE`,
/// `NOT_STOPPED`), restated: the App_Prover's compiler cannot read an
/// associated constant of a generic type of another crate.
#[cfg(all(feature = "measure", feature = "gating"))]
const FOOTPRINT_WRITTEN_INSIDE: u32 = 1;
#[cfg(all(feature = "measure", feature = "gating"))]
const FOOTPRINT_NOT_STOPPED: u32 = 3;

/// The EasyDMA traffic scenario of the measurement configurations (task
/// 19.2; R17.4, R34.5): SPIM0, the Partition's EasyDMA peripheral,
/// transfers 64 bytes out of `tx` and 64 bytes into `rx` at its highest
/// rate (8 Mbps) for the whole run, the END_START shortcut starting the
/// next transfer as each ends, so that every Task of every Partition is
/// measured under the worst-case concurrent EasyDMA traffic this
/// Application can produce (part2 owns the only EasyDMA-capable
/// peripheral). The clock and data pins go to the DK's LEDs (part1, the
/// owner of `P0`, sets their direction in the measurement build).
/// The registers are reached through `rsk::dma_write` (DD-08: the window
/// is read-only to the Partition); the pointer and count registers
/// through `rsk::DmaBuf::lend`, which checks the buffers (R17.1).
#[cfg(feature = "measure")]
mod traffic {
    /// The window of SPIM0 in the Target table (the report reads the same).
    pub const SPIM0: u32 = 0x4000_3000;
    pub const TASKS_START: u32 = 0x010;
    pub const EVENTS_END: u32 = 0x118;
    pub const EVENTS_STARTED: u32 = 0x14C;
    pub const SHORTS: u32 = 0x200;
    pub const ENABLE: u32 = 0x500;
    pub const PSEL_SCK: u32 = 0x508;
    pub const PSEL_MOSI: u32 = 0x50C;
    pub const PSEL_MISO: u32 = 0x510;
    pub const FREQUENCY: u32 = 0x524;
    /// The pins (port 0): LED2 and LED3 of the DK carry SCK and MOSI (they
    /// glow at the clock's duty), LED4's pin reads MISO; part1 configures
    /// their direction in its `P0` window (the SPIM does not), and the
    /// transfer needs the clock pin connected to run.
    pub const SCK_PIN: u32 = 14;
    pub const MOSI_PIN: u32 = 15;
    pub const MISO_PIN: u32 = 16;
    pub const SHORTS_END_START: u32 = 1 << 17;
    pub const FREQUENCY_M8: u32 = 0x8000_0000;
    #[allow(dead_code)]
    pub const FREQUENCY_M1: u32 = 0x1000_0000;
    #[allow(dead_code)]
    pub const FREQUENCY_K125: u32 = 0x0200_0000;
    pub const ENABLE_SPIM: u32 = 7;
    /// The channels of the system's DMA table: RXD, then TXD.
    pub const CHANNEL_RX: u8 = 0;
    pub const CHANNEL_TX: u8 = 1;
}

/// One Job's step of the traffic scenario: the first Job of the
/// Partition's incarnation configures and starts the transfers (and, after
/// a restart, checks that the Kernel's stop held, R17.6: a transfer that
/// starts again within a transfer's length of the check, with the
/// shortcut still set, is counted in `dma_after_stop`); every later Job
/// counts in `dma_end` whether a transfer ended since the last and clears
/// the event. Returns whether the Job's traffic was confirmed: the first
/// Job's requests accepted, a later Job's transfer ended (the demo's
/// safe state counts it as `dma_ok`).
// App_Prover (task 17.1): the step leaves the Job's count and the Resource
// as they were, which the body's assertions after the call rely on.
#[cfg(feature = "measure")]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).count@ == (*cx).count@))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).stats@ == (*cx).stats@))]
fn traffic_step(cx: &mut rsk_ctx::Sampler, n: u8) -> bool {
    use traffic::*;
    // The plain measurement configuration alone judges the footprint (the
    // others restart or overrun this Partition on purpose): at the three
    // Jobs after the first, for the traffic of the 100 ms before each, at
    // 8 Mbps, then 1 Mbps, then 125 kbps; the run goes on at 8 Mbps.
    #[cfg(feature = "gating")]
    {
        let c = cx.count.get();
        if c >= 1 && c <= 3 {
            let next = if c == 1 { FREQUENCY_M1 } else if c == 2 { FREQUENCY_K125 } else { FREQUENCY_M8 };
            let verdict = footprint_step(cx, next);
            if c == 1 {
                cx.dma_footprint.set(verdict);
            } else if c == 2 {
                cx.dma_footprint_1m.set(verdict);
            } else {
                cx.dma_footprint_125k.set(verdict);
            }
            // A transfer that wrote the buffer is a transfer that ended.
            if verdict == FOOTPRINT_WRITTEN_INSIDE {
                cx.dma_end.with(|c| *c = c.saturating_add(1));
            }
            return true;
        }
    }
    if cx.count.get() != 0 {
        let ended = rsk::mmio::read(SPIM0 + EVENTS_END) != 0;
        if ended {
            let _ = rsk::dma_write(CHANNEL_RX, EVENTS_END, 0);
            cx.dma_end.with(|c| *c = c.saturating_add(1));
        }
        return ended;
    }
    if rsk::mmio::read(SPIM0 + ENABLE) == ENABLE_SPIM {
        let _ = rsk::dma_write(CHANNEL_RX, EVENTS_STARTED, 0);
        // About 150 µs, over twice a 64-byte transfer at 8 Mbps.
        rsk::spin::<1_000>();
        if rsk::mmio::read(SPIM0 + EVENTS_STARTED) != 0 {
            cx.dma_after_stop.with(|c| *c = c.saturating_add(1));
        }
    }
    let _ = rsk::dma_write(CHANNEL_RX, PSEL_SCK, SCK_PIN);
    let _ = rsk::dma_write(CHANNEL_RX, PSEL_MOSI, MOSI_PIN);
    let _ = rsk::dma_write(CHANNEL_RX, PSEL_MISO, MISO_PIN);
    let _ = rsk::dma_write(CHANNEL_RX, FREQUENCY, FREQUENCY_M8);
    let _ = rsk::dma_write(CHANNEL_RX, SHORTS, SHORTS_END_START);
    let _ = rsk::dma_write(CHANNEL_RX, ENABLE, ENABLE_SPIM);
    // The buffers are lent for the run (R17.5): the transfers are held in
    // the Task's state, and no Job reaches the bytes until it takes one
    // back (`footprint_step`).
    let tx = cx.tx.with(|b| {
        if let Some(bytes) = b.bytes() {
            bytes[0] = n;
        }
        b.lend(rsk_target::DMA[1])
    });
    let rx = cx.rx.with(|g| {
        g.arm(0xA5);
        g.buf.lend(rsk_target::DMA[0])
    });
    let accepted = tx.is_some() && rx.is_some();
    cx.tx_held.with(|h| h.put(tx));
    cx.rx_held.with(|h| h.put(rx));
    let _ = rsk::dma_write(CHANNEL_RX, TASKS_START, 1);
    accepted
}

/// The footprint of the accepted transfers of the 100 ms before (HW-15;
/// 2026-11-16): the receive transfer is taken back, which stops the
/// traffic and waits for the peripheral's STOPPED event; the buffer and
/// its guards are judged (`rsk::DmaGuarded::footprint`: the transfers wrote
/// the buffer and neither guard); the peripheral is set to `frequency`,
/// the guards and the buffer are written again, the buffer is lent again
/// and the traffic restarted. Returns the verdict.
#[cfg(all(feature = "measure", feature = "gating"))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).count@ == (*cx).count@))]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).stats@ == (*cx).stats@))]
fn footprint_step(cx: &mut rsk_ctx::Sampler, frequency: u32) -> u32 {
    use traffic::*;
    let held = cx.rx_held.with(|h| h.take());
    let verdict = match held {
        Some(transfer) => cx.rx.with(|g| match g.buf.reclaim(transfer) {
            Ok(()) => g.footprint(0xA5),
            Err(_) => FOOTPRINT_NOT_STOPPED,
        }),
        None => FOOTPRINT_NOT_STOPPED,
    };
    // The rate is changed with the peripheral disabled.
    let _ = rsk::dma_write(CHANNEL_RX, ENABLE, 0);
    let _ = rsk::dma_write(CHANNEL_RX, FREQUENCY, frequency);
    let _ = rsk::dma_write(CHANNEL_RX, ENABLE, ENABLE_SPIM);
    let rx = cx.rx.with(|g| {
        g.arm(0xA5);
        g.buf.lend(rsk_target::DMA[0])
    });
    cx.rx_held.with(|h| h.put(rx));
    let _ = rsk::dma_write(CHANNEL_RX, SHORTS, SHORTS_END_START);
    let _ = rsk::dma_write(CHANNEL_RX, TASKS_START, 1);
    verdict
}

/// The isolation suite (task 7.4): the EasyDMA cases of R17.7 (R17.1,
/// R17.2), one per Job of `sampler`, then the access matrix of this
/// Partition (R16.8). The case index advances before the request, since
/// a refused request ends the Job (END_JOB; the Kernel's `dma_rejected`
/// event) before anything after it runs.
///
/// | Case | Buffer | Expected |
/// |---|---|---|
/// | 0 | the Task's own buffer (Partition RAM above its stack), lent | accepted: the channel's pointer and count registers hold it |
/// | 1 | a local array (the Partition's stack region) | refused (R17.1 excludes stacks) |
/// | 2 | another Partition's RAM | refused |
/// | 3 | the Kernel's RAM | refused |
/// | 4 | 16 bytes starting 8 bytes before the Partition's RAM ends | refused (not wholly inside the RAM) |
/// | 5 | the own buffer, taken back, on channel 7 (no such channel) | refused |
/// | 6 | the own buffer lent again | accepted |
#[cfg(feature = "isolation")]
const DMA_CASES: u32 = 7;

/// The address of the Task's buffer, for the rig's register check. A
/// pointer-to-integer cast is outside the App_Prover's language, so this
/// one-line helper is trusted (R46.5; the value is only reported).
#[cfg(feature = "isolation")]
#[cfg_attr(creusot, rsk::creusot_std::macros::trusted)]
fn address_of(buf: &[u8; 16]) -> u32 {
    buf.as_ptr() as usize as u32
}

// App_Prover (task 17.1): the step leaves `stats` as it was, so that the
// body's Critical_Section on it still finds the Resource invariant
// (`sampler_pre`) after the call; the step's own obligations are its
// counters' arithmetic and the access matrix's preconditions.
#[cfg(feature = "isolation")]
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^cx).stats@ == (*cx).stats@))]
fn isolation_step(cx: &mut rsk_ctx::Sampler) {
    use rsk_target::layout as l;
    let case = cx.dma_case.get();
    cx.dma_case.set(case.saturating_add(1));
    let other = if l::SELF == 0 { 1 } else { 0 };
    match case {
        0 | 6 => {
            // The transfer is held from here on (R17.5): case 5 takes it
            // back before its own request, so case 6 lends again.
            let transfer = cx.buf.with(|b| b.lend(rsk_target::DMA[0]));
            if transfer.is_some() {
                cx.dma_ok.with(|c| *c = c.saturating_add(1));
            }
            cx.held.with(|h| h.put(transfer));
        }
        1 => {
            let local = [0u8; 16];
            let _ = rsk::hw_dt::dma_request_raw(DMA_CHANNEL, address_of(&local), 16);
        }
        2 => {
            let _ = rsk::hw_dt::dma_request_raw(DMA_CHANNEL, l::RAM[other].0 + 0x1000, 16);
        }
        3 => {
            let _ = rsk::hw_dt::dma_request_raw(DMA_CHANNEL, l::KERNEL_RAM.0 + 0x100, 16);
        }
        4 => {
            let _ = rsk::hw_dt::dma_request_raw(DMA_CHANNEL, l::RAM[l::SELF].0 + l::RAM[l::SELF].1 - 8, 16);
        }
        5 => {
            // The held transfer is given back first (nothing was started:
            // the peripheral is not enabled, so the bytes are at rest), and
            // the buffer is then offered on a channel that does not exist.
            let held = cx.held.with(|h| h.take());
            if let Some(transfer) = held {
                let _ = cx.buf.with(|b| b.reclaim(transfer));
            }
            let _ = rsk::hw_dt::dma_request_raw(7, cx.buf.with(|b| b.address()), 16);
        }
        _ => {
            let layout = rsk::matrix::Layout {
                this: l::SELF, code: &l::CODE, ram: &l::RAM, stack: &l::STACK, windows: &l::WINDOWS, windows_ro: &l::WINDOWS_RO,
                kernel_flash: l::KERNEL_FLASH, kernel_ram: l::KERNEL_RAM, kernel_windows: &l::KERNEL_WINDOWS,
            };
            let i = (case - DMA_CASES) as usize;
            if i < 32 && i < rsk::matrix::count(&layout) && matches!(rsk::matrix::perform(&layout, i), rsk::matrix::Outcome::Survived) {
                cx.survived.with(|m| *m |= 1 << i);
            }
        }
    }
}

/// The Resource `stats` (declared `resource stats: crate::Stats`): DMA
/// transfers requested and accepted. Its declared invariant is
/// `ok <= total` (R46.4); `sampler` writes it and `filter` reads it.
#[derive(Clone, Copy, Debug)]
pub struct Stats {
    pub total: u32,
    pub ok: u32,
}

impl Stats {
    /// The initial value; the App_Prover checks it satisfies the invariant
    /// from this postcondition.
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result.total@ == 0 && result.ok@ == 0))]
    pub const fn new() -> Stats {
        Stats { total: 0, ok: 0 }
    }

    /// Records one request and whether it was accepted (saturating).
    #[cfg_attr(creusot, rsk::creusot_std::macros::requires(self.ok@ <= self.total@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^self).ok@ <= (^self).total@))]
    #[cfg_attr(creusot, rsk::creusot_std::macros::ensures((^self).total@ >= self.total@))]
    pub fn record(&mut self, accepted: bool) {
        self.total = self.total.saturating_add(1);
        if accepted {
            self.ok = self.ok.saturating_add(1);
        }
    }
}

/// The receiver-side validation function of `Req` on `relay` (task 21.6;
/// R42.4: `log`'s Partition is at level C, below this one): `log` relays
/// the count of requests it answered, a 31-bit value, so a word with bit
/// 31 set is no request this Partition expects. Total and panic-free (the
/// App_Prover proves it with the bodies, with its postcondition).
#[cfg_attr(creusot, rsk::creusot_std::macros::ensures(result == ((*v)@ < 0x8000_0000)))]
pub fn validate_req(v: &u32) -> bool {
    *v < 0x8000_0000
}

/// y = y + (x - y) / 8 with x = 100.0, in `f32`: a first-order low-pass of
/// a constant input converges to it, which the safe state checks. Creusot's
/// floating-point support is partial (RN-10), so this function alone is
/// trusted under the App_Prover (task 17.1); the Task body is proven.
#[cfg_attr(creusot, rsk::creusot_std::macros::trusted)]
fn lowpass(y: &mut f32) {
    let x: f32 = 100.0;
    *y += (x - *y) * 0.125;
}

/// Task `filter`: steps the low-pass state and reads the transfer
/// statistics inside a Critical_Section it leaves unchanged.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::filter_pre(&cx)))]
// The loop's label names it in the loop report (task 19.2); nothing breaks to it.
#[allow(unused_labels)]
pub fn filter(mut cx: rsk_ctx::Filter) {
    // The isolation suite (task 7.4) with the Hardware_Model differential
    // tests (task 3.3, HW-06, HW-10): FPSCR as the first Job finds it,
    // then the register-pattern probe across about 3 ms, during which
    // control, echo, sensor, and pulse preempt the Job; and the Job of
    // 50 ms exceeds its 15 ms Budget so that the Partition restarts
    // (RESTART_PARTITION: the Kernel's EasyDMA stop before the
    // re-initialization, R17.6; early, so that the probes' records and
    // the EasyDMA cases that follow are those of one uninterrupted run).
    #[cfg(feature = "isolation")]
    {
        if cx.pattern_runs.get() == 0 {
            cx.fpscr.set(rsk::hw_dt::fpscr());
            cx.fpscr_seen.set(1);
        }
        let mask = rsk::hw_dt::register_patterns_fp(50_000);
        cx.pattern_runs.with(|c| *c = c.saturating_add(1));
        if mask != 0 {
            cx.pattern_fail.with(|c| *c = c.saturating_add(1));
            cx.pattern_mask.with(|m| *m |= mask);
        }
        if inject::window(50, 25) {
            inject::spin_ms(40);
        }
    }
    // The response configuration (task 7.5, R14.11): the Job of 250 ms
    // faults (END_JOB as a fault response), and the Job of 500 ms exceeds
    // its 500 µs Budget inside its Critical_Section on `stats` (END_JOB
    // escalates to RESTART_PARTITION, R18.4).
    #[cfg(feature = "response")]
    {
        if inject::window(250, 25) {
            inject::fault();
        }
        if inject::window(500, 25) {
            cx.stats.lock(|_s| inject::spin_ms(1));
        }
    }
    // The stress configuration (task 7.5, R18.8): every Job holds `stats`
    // for 250 µs, its maximal Critical_Section (half the 500 µs Budget:
    // a 400 µs hold plus the body overran on every Job, which restarted
    // the Partition every period), and every eighth exceeds the Budget
    // inside it (the escalation above, once per 200 ms).
    #[cfg(feature = "stress")]
    {
        let long = cx.count.get() % 8 == 7;
        cx.stats.lock(|_s| {
            let start = rsk::now();
            let mut i = 0u32;
            let hold = if long { rsk_target::ms(1) } else { rsk_target::TICK_HZ / 4000 };
            #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(i@ <= 10_000_000))]
            #[cfg_attr(creusot, rsk::creusot_std::macros::variant(10_000_000 - i@))]
            'hold: while rsk::now().wrapping_sub(start) < hold && i < 10_000_000 {
                i += 1;
            }
            // The measurement build (task 19.2, R35.4): the largest iteration
            // count the loop reached, for the safe state's loop report.
            #[cfg(feature = "measure")]
            cx.loop_hold.with(|m| if i > *m { *m = i });
        });
    }
    // Task 21.6 (R41.3, A-P8): the `sink` role of `PingPong` on `relay`
    // (rank 1): a request `log` relays is answered with its value in the
    // same Job; then the `ctl` role on `report` (`after = relay`, rank 2),
    // stepped only in a Job whose `relay` receive delivered a request: in
    // S0 sending this Task's Job count on to `control`, in S1 taking
    // `control`'s reply (a completed round). Both states go back to their
    // slots either way (R41.2).
    #[cfg(feature = "gating")]
    {
        let mut relay_received = false;
        {
            use rsk_roles::relay_sink::{NotReceived, NotSent, RecvS0, State};
            let state = cx.relay.take(State::Empty);
            let next = match state {
                State::S0(s) => match s.try_recv() {
                    Ok(received) => {
                        // Task 21.6 (A-P8): the completed receive opens the
                        // gate of this Job's operations on `report`.
                        cx.relay_gate.open(&received);
                        relay_received = true;
                        match received {
                            RecvS0::Req(v, s1) => match s1.send_rep(v) {
                                Ok(s0) => {
                                    cx.relay_reqs.with(|n| *n = n.saturating_add(1));
                                    State::S0(s0)
                                }
                                Err(NotSent::Full(s1)) | Err(NotSent::Refused(s1)) => State::S1(s1),
                                Err(NotSent::PeerChanged(s0)) => State::S0(s0),
                            },
                            RecvS0::Stop(e) => e.finish(),
                        }
                    }
                    Err(NotReceived::Empty(s0)) | Err(NotReceived::Unexpected(s0)) | Err(NotReceived::Rejected(s0)) | Err(NotReceived::PeerChanged(s0)) => State::S0(s0),
                },
                State::S1(s) => match s.send_rep(0) {
                    Ok(s0) => State::S0(s0),
                    Err(NotSent::Full(s1)) | Err(NotSent::Refused(s1)) => State::S1(s1),
                    Err(NotSent::PeerChanged(s0)) => State::S0(s0),
                },
                other => other,
            };
            cx.relay.store(next);
        }
        {
            use rsk_roles::report_ctl::{NotReceived, NotSent, RecvS1, State};
            let state = cx.report.take(State::Empty);
            let next = if !relay_received {
                state
            } else {
                match state {
                    State::S0(c) => match c.send_req(cx.count.get(), &cx.relay_gate) {
                        Ok(c1) => State::S1(c1),
                        Err(NotSent::Full(c0)) | Err(NotSent::Refused(c0)) | Err(NotSent::PeerChanged(c0)) => State::S0(c0),
                    },
                    State::S1(c) => match c.try_recv(&cx.relay_gate) {
                        Ok(RecvS1::Rep(_, c0)) => {
                            cx.report_rounds.with(|n| *n = n.saturating_add(1));
                            State::S0(c0)
                        }
                        Err(NotReceived::Empty(c1)) | Err(NotReceived::Unexpected(c1)) | Err(NotReceived::Rejected(c1)) => State::S1(c1),
                        Err(NotReceived::PeerChanged(c0)) => State::S0(c0),
                    },
                    other => other,
                }
            };
            cx.report.store(next);
        }
    }
    // Task 21.7 (multiparty Endpoints): the `sink` role of `Triangle` on
    // `triangle`, the role the choice does not inform (its state is the
    // merge of the two branches): a forwarded request from `mid` (`log`)
    // is acknowledged to `ctl` (`control`) with its value in the same
    // Job; a `Done` finishes the role. The validation function of `Fwd` is
    // `validate_req` (R42.4: `log`'s Partition is at level C, below this
    // one); a peer-state change resets the role.
    #[cfg(feature = "gating")]
    {
        use rsk_roles::triangle_sink::{NotReceived, NotSent, RecvS0, State};
        let state = cx.triangle.take(State::Empty);
        let next = match state {
            State::S0(s) => match s.try_recv() {
                Ok(RecvS0::Fwd(v, s1)) => match s1.send_ack(v) {
                    Ok(s0) => {
                        cx.triangle_acks.with(|n| *n = n.saturating_add(1));
                        State::S0(s0)
                    }
                    Err(NotSent::Full(s1)) | Err(NotSent::Refused(s1)) => State::S1(s1),
                    Err(NotSent::PeerChanged(s0)) => State::S0(s0),
                },
                Ok(RecvS0::Done(s2)) => s2.finish(),
                Err(NotReceived::Empty(s0)) | Err(NotReceived::Unexpected(s0)) | Err(NotReceived::Rejected(s0)) | Err(NotReceived::PeerChanged(s0)) => State::S0(s0),
            },
            State::S1(s) => match s.send_ack(0) {
                Ok(s0) => State::S0(s0),
                Err(NotSent::Full(s1)) | Err(NotSent::Refused(s1)) => State::S1(s1),
                Err(NotSent::PeerChanged(s0)) => State::S0(s0),
            },
            other => other,
        };
        cx.triangle.store(next);
    }
    cx.y.with(|y| lowpass(y));
    // The floating-point variant of the cycle-attribution test (task 19.3,
    // Δ_fp_save and Δ_fp_restore): with the context active from the step
    // above, the same fixed-iteration spin as `control`'s, preempted by
    // every other Task; the rest of the body is skipped as `log`'s is.
    #[cfg(feature = "cycles")]
    {
        rsk::spin::<8_000>();
        cx.count.with(|c| *c = c.saturating_add(1));
        rsk::debug(&[b'F']);
        return;
    }
    #[cfg(creusot)]
    let read = #[rsk::creusot_std::macros::ensures(^s == *s && result == (*s).ok)] |s: &mut Stats| s.ok;
    #[cfg(not(creusot))]
    let read = |s: &mut Stats| s.ok;
    let ok = cx.stats.lock(read);
    // Known from the Resource invariant alone: no Job of this Task has seen
    // more accepted than requested transfers.
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(exists<t: Stats> rsk_spec::stats_inv(t) && t.ok == ok);
    let _ = ok;
    cx.count.with(|c| *c = c.saturating_add(1));
    // A byte array, not a byte-string literal: Creusot has no `b".."` yet.
    rsk::debug(&[b'F']);
}

/// Task `sampler`: fills its buffer (in the Partition's RAM, so
/// DMA-permitted), requests the transfer, and records the outcome in the
/// Resource `stats`, whose invariant the Critical_Section preserves.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::sampler_pre(&cx)))]
pub fn sampler(mut cx: rsk_ctx::Sampler) {
    // The stress configuration (task 7.5, R18.8): every second Job exceeds
    // its 500 µs Budget outside a Critical_Section (END_JOB).
    #[cfg(feature = "stress")]
    if cx.count.get() % 2 == 1 {
        let n = inject::spin_ms(2);
        #[cfg(feature = "measure")]
        cx.loop_spin.with(|m| if n > *m { *m = n });
        #[cfg(not(feature = "measure"))]
        let _ = n;
    }
    // The isolation suite (task 7.4): the EasyDMA cases, then the access
    // matrix; the buffer's address for the rig's register check.
    #[cfg(feature = "isolation")]
    {
        cx.dma_ptr.set(cx.buf.with(|b| b.address()));
        isolation_step(&mut cx);
    }
    #[cfg(creusot)]
    let before = rsk::creusot_std::macros::snapshot!(cx.count@);
    // The low byte of the counter: a reduction before the cast, so that the
    // cast is lossless (the App_Prover checks every `as`).
    let n = (cx.count.get() % 256) as u8;
    // The isolation suite's Jobs made their request in `isolation_step`
    // (every Job of the plain demo requests; the suite's cases decide).
    #[cfg(feature = "isolation")]
    let accepted = {
        let _ = n;
        false
    };
    // The measurement configurations (task 19.2): the EasyDMA traffic
    // scenario of R17.4 and R34.5 in place of the per-Job request.
    #[cfg(all(not(feature = "isolation"), feature = "measure"))]
    let accepted = traffic_step(&mut cx, n);
    #[cfg(all(not(feature = "isolation"), not(feature = "measure")))]
    let accepted = plain_request(&mut cx, n);
    if accepted {
        cx.dma_ok.with(|c| *c = c.saturating_add(1));
    }
    #[cfg(creusot)]
    let record = #[rsk::creusot_std::macros::requires(s.ok@ <= s.total@)]
    #[rsk::creusot_std::macros::ensures((^s).ok@ <= (^s).total@)]
    |s: &mut Stats| s.record(accepted);
    #[cfg(not(creusot))]
    let record = |s: &mut Stats| s.record(accepted);
    cx.stats.lock(record);
    cx.count.with(|c| *c = c.saturating_add(1));
    // Functional property (task 17.1): the Job advances the sampler count by one.
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(cx.count@@ >= (*before)@ && cx.count@@ <= (*before)@ + 1);
    rsk::debug(&[b'G']);
}
