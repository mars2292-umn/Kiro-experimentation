//! P1 demo Partition 2 (`part2`, level B; task 14, R3.7): `filter`
//! (Priority 3, every 25 ms) keeps a floating-point state across Jobs, so
//! its Jobs carry an active FP context that the Kernel stacks and restores
//! (R11, PR-31; the `fpu` flag of the declaration), and `sampler`
//! (Priority 1, every 100 ms) requests an EasyDMA transfer of a buffer in
//! its own RAM through `rsk::dma_request` (DD-08, R17): the Kernel checks
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
        now >= super::rsk_target::ms(from_ms) && now < super::rsk_target::ms(from_ms + len_ms)
    }

    pub fn spin_ms(ms: u64) {
        let start = rsk::now();
        let mut i = 0u32;
        // R35.2: the App_Prover's bound equals the derived Loop_Bound_Annotation.
        #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(i@ <= 100_000_000))]
        #[cfg_attr(creusot, rsk::creusot_std::macros::variant(100_000_000 - i@))]
        while rsk::now().wrapping_sub(start) < super::rsk_target::ms(ms) && i < 100_000_000 {
            i += 1;
        }
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
const DMA_CHANNEL: u8 = 0;

/// The isolation suite (task 7.4): the EasyDMA cases of R17.7 (R17.1,
/// R17.2), one per Job of `sampler`, then the access matrix of this
/// Partition (R16.8). The case index advances before the request, since
/// a refused request ends the Job (END_JOB; the Kernel's `dma_rejected`
/// event) before anything after it runs.
///
/// | Case | Buffer | Expected |
/// |---|---|---|
/// | 0 | the Task's own buffer (Partition RAM above its stack) | accepted: the channel's pointer and count registers hold it |
/// | 1 | a local array (the Partition's stack region) | refused (R17.1 excludes stacks) |
/// | 2 | another Partition's RAM | refused |
/// | 3 | the Kernel's RAM | refused |
/// | 4 | 16 bytes starting 8 bytes before the Partition's RAM ends | refused (not wholly inside the RAM) |
/// | 5 | the own buffer on channel 7 (no such channel) | refused |
/// | 6 | the own buffer again | accepted |
#[cfg(feature = "isolation")]
const DMA_CASES: u32 = 7;

#[cfg(feature = "isolation")]
fn isolation_step(cx: &mut rsk_ctx::Sampler) {
    use rsk_target::layout as l;
    let case = cx.dma_case.get();
    cx.dma_case.set(case + 1);
    let other = if l::SELF == 0 { 1 } else { 0 };
    match case {
        0 | 6 => {
            let accepted = cx.buf.with(|buf| {
                let bytes: &[u8] = &*buf;
                rsk::dma_request(DMA_CHANNEL, bytes)
            });
            if accepted {
                cx.dma_ok.with(|c| *c = c.saturating_add(1));
            }
        }
        1 => {
            let local = [0u8; 16];
            let _ = rsk::dma_request(DMA_CHANNEL, &local);
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
            let _ = cx.buf.with(|buf| {
                let bytes: &[u8] = &*buf;
                rsk::dma_request(7, bytes)
            });
        }
        _ => {
            let layout = rsk::matrix::Layout {
                this: l::SELF, code: &l::CODE, ram: &l::RAM, stack: &l::STACK, windows: &l::WINDOWS,
                kernel_flash: l::KERNEL_FLASH, kernel_ram: l::KERNEL_RAM, kernel_windows: &l::KERNEL_WINDOWS,
            };
            let i = (case - DMA_CASES) as usize;
            if i < rsk::matrix::count(&layout) && rsk::matrix::perform(&layout, i) == rsk::matrix::Outcome::Survived {
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
            while rsk::now().wrapping_sub(start) < hold && i < 10_000_000 {
                i += 1;
            }
        });
    }
    cx.y.with(|y| lowpass(y));
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
        inject::spin_ms(2);
    }
    // The isolation suite (task 7.4): the EasyDMA cases, then the access
    // matrix; the buffer's address for the rig's register check.
    #[cfg(feature = "isolation")]
    {
        cx.dma_ptr.set(cx.buf.with(|buf| buf.as_ptr() as usize as u32));
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
    #[cfg(not(feature = "isolation"))]
    let accepted = cx.buf.with(|buf| {
        buf[0] = n;
        // An unsizing coercion, not `Index<RangeFull>`, which Creusot has no
        // contract for on arrays.
        let bytes: &[u8] = &*buf;
        rsk::dma_request(DMA_CHANNEL, bytes)
    });
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
