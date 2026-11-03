//! P1 demo Partition 0 (`part0`, level A): `control` (Priority 7) and
//! `sensor` (Priority 5) share the Resource `state` (Ceiling 7). The
//! Resource cells, the per-Task state cells, the Context types and the Job
//! wrappers come from the App_Declaration through `rsk_partition_part0!`
//! (task 10.5); Task bodies are synchronous functions that take their
//! Context and return to their Release_Point (PR-09). The counters the safe
//! state reads are declared per-Task state (PR-25), so this crate holds no
//! `static` of its own (PR-05).
#![no_std]
#![forbid(unsafe_code)]

p1_demo_decl::rsk_partition_part0!();

/// The fault-injection suite (task 7.5): whether the time base is inside
/// `[from, from + len)` milliseconds, and a busy wait of `ms` milliseconds
/// by the time base (bounded in iterations too, PR-16). Injections are
/// placed by instant, not by Job count, so that a restarted Partition
/// (its counts back to 0) does not repeat them.
#[cfg(feature = "response")]
mod inject {
    pub fn window(from_ms: u64, len_ms: u64) -> bool {
        let now = rsk::now();
        now >= super::rsk_target::ms(from_ms) && now < super::rsk_target::ms(from_ms + len_ms)
    }

    pub fn spin_ms(ms: u64) {
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now().wrapping_sub(start) < super::rsk_target::ms(ms) && i < 100_000_000 {
            i += 1;
        }
    }

    /// A write into the Kernel's RAM, outside every Partition's view: a
    /// MemManage fault attributed to the Partition (R14.3).
    pub fn fault() {
        rsk::mmio::write(0x2000_0000, 1);
    }
}

/// Task `control`: reads the state under the lock, counts, and raises
/// `tick` on every third Job (R10.5; every 30 ms, so every raise is
/// accepted against echo's 20 ms MIT). `in_cs` says whether this Release
/// fell inside sensor's Critical_Section: a control Release during that
/// window was blocked by the Ceiling, not preempting (R8.4).
// App_Prover (task 17.1): the body assumes its Resources' declared
// invariants (`in_cs` is 0 or 1) and proves, beyond panic freedom, that
// the Job advances its counter by one (saturating).
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::control_pre(&cx)))]
pub fn control(mut cx: rsk_ctx::Control) {
    // The response configuration (task 7.5, R14.11): the Job of 200 ms
    // exceeds its 1 ms Budget (RECORD_AND_CONTINUE, R18.6, so it still
    // counts), and the Job of 1,900 ms faults into the declared SAFE_STATE.
    #[cfg(feature = "response")]
    {
        if inject::window(200, 10) {
            inject::spin_ms(2);
        }
        if inject::window(1900, 10) {
            inject::fault();
        }
    }
    #[cfg(creusot)]
    let count_before = rsk::creusot_std::macros::snapshot!(cx.count@);
    // A Critical_Section's closure carries its postcondition for the
    // App_Prover (an unspecified closure's is `true`, which cannot show the
    // Resource invariant preserved, R46.4).
    #[cfg(creusot)]
    let read = #[rsk::creusot_std::macros::ensures(^v == *v && result == *v)] |v: &mut u32| *v;
    #[cfg(not(creusot))]
    let read = |v: &mut u32| *v;
    let in_cs = cx.in_cs.lock(read);
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(in_cs@ <= 1);
    cx.state.lock(|s| {
        let _ = *s;
    });
    if in_cs != 0 {
        cx.blocked.with(|b| *b = b.saturating_add(1));
    }
    // Saturating: a bound the App_Prover can see (task 17.1).
    let n = cx.count.get().saturating_add(1);
    cx.count.set(n);
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(cx.count@ == n && n@ >= (*count_before)@ && n@ <= (*count_before)@ + 1);
    if n % 3 == 0 {
        let _ = cx.tick.raise();
        // fi-PR-12: a second raise while echo's Release is pending (echo's
        // Priority 6 is below control's 7, so it has not run): the
        // Kernel counts an MIT_Violation (R9.8, PR-12).
        #[cfg(feature = "fi-pr12")]
        {
            let _ = cx.tick.raise();
        }
    }
    // Byte arrays, not byte-string literals: Creusot has no `b".."` yet.
    rsk::debug(&[b'A']);
}

/// Task `echo` (sporadic, released by `tick`): counts.
pub fn echo(mut cx: rsk_ctx::Echo) {
    #[cfg(creusot)]
    let before = rsk::creusot_std::macros::snapshot!(cx.count@);
    cx.count.with(|c| *c = c.saturating_add(1));
    #[cfg(creusot)]
    rsk::creusot_std::macros::proof_assert!(cx.count@@ >= (*before)@ && cx.count@@ <= (*before)@ + 1);
    // fi-PR-19: a panic in a Job reaches the Kernel's panic handler as a
    // fault of the Partition (R14.2).
    #[cfg(feature = "fi-pr19")]
    {
        if cx.count.get() == 2 {
            panic!("fi-PR-19");
        }
    }
    // fi-PR-22: an unprivileged write to the NVIC (the Private Peripheral
    // Bus) is a BusFault (DD-01, R16.10).
    #[cfg(feature = "fi-pr22")]
    {
        if cx.count.get() == 2 {
            rsk::mmio::write(0xE000_E100, 1);
        }
    }
    // fi-PR-29: a write into another Partition's RAM is a MemManage fault
    // (R16.4; P1RAM of the QEMU memory map).
    #[cfg(feature = "fi-pr29")]
    {
        if cx.count.get() == 2 {
            rsk::mmio::write(0x2001_0000, 1);
        }
    }
    rsk::debug(&[b'E']);
}

/// Task `sensor`: released 5 ms into each 20 ms period, holds the Resource
/// for 8 ms (so that the control Release at 10 ms into the period falls
/// inside the Critical_Section and is blocked by the Ceiling, R8.4) and
/// updates it.
// The two writes to `in_cs` preserve its invariant (R46.4), which the
// App_Prover checks at each `lock`.
/// The Critical_Section's length: 6 ms, or 12 ms in the release-timing
/// configuration (task 7.3), so that it is in progress at `control`'s
/// nominal instants (10 ms into every 20 ms period; sensor's Jobs start
/// at 5 ms). It was 8 ms until the WCET_Records of task 19.1 (2026-11-02):
/// with the 25 % margin of R34.6 the measured section of 8.1 ms made
/// `control`'s blocking term 10.1 ms against its 10 ms deadline and
/// sensor's 8.35 ms Job exceed its 8.8 ms Budget; at 6 ms the measured
/// analysis of the demo has a verdict (Verification_Plan 11.6).
#[cfg(not(feature = "timing"))]
const CS_MS: u64 = 6;
#[cfg(feature = "timing")]
const CS_MS: u64 = 12;

#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::sensor_pre(&cx)))]
pub fn sensor(mut cx: rsk_ctx::Sensor) {
    #[cfg(creusot)]
    let enter = #[rsk::creusot_std::macros::ensures(^v == 1u32)] |v: &mut u32| *v = 1;
    #[cfg(not(creusot))]
    let enter = |v: &mut u32| *v = 1;
    #[cfg(creusot)]
    let leave = #[rsk::creusot_std::macros::ensures(^v == 0u32)] |v: &mut u32| *v = 0;
    #[cfg(not(creusot))]
    let leave = |v: &mut u32| *v = 0;
    cx.in_cs.lock(enter);
    cx.state.lock(|s| {
        // 8 ms by the Kernel time base (`rsk_target::ms`), bounded in
        // iterations too (PR-16).
        let start = rsk::now();
        let mut i = 0u32;
        // `wrapping_sub`: the time base is monotone, but the App_Prover knows
        // nothing of `now()` beyond its type (task 17.1).
        #[cfg_attr(creusot, rsk::creusot_std::macros::invariant(i@ <= 10_000_000))]
        while rsk::now().wrapping_sub(start) < rsk_target::ms(CS_MS) && i < 10_000_000 {
            i += 1;
        }
        *s = s.saturating_add(1);
    });
    cx.in_cs.lock(leave);
    cx.count.with(|c| *c = c.saturating_add(1));
    // The release-timing test (task 7.3, R9.10): every 1024th Job runs
    // on for 38 ms outside the Critical_Section (50 ms in all, past two
    // of the Task's 20 ms nominal instants and short of the third), so
    // that R9.7 holds one Release and R9.8 discards the next with an
    // MIT_Violation (RECORD_AND_CONTINUE lets the Job complete, R18.6;
    // the Budget overrun and the deadline miss are the test's expected
    // events).
    // The response configuration (task 7.5): the Job of 1,305 ms runs on
    // for 120 ms outside the Critical_Section (its own Budget overrun and
    // deadline miss recorded, RECORD_AND_CONTINUE and RECORD_ONLY) so that
    // the lower-Priority Tasks of the other Partitions miss their deadlines
    // while their Jobs are pending: the deadline-miss responses of part1
    // (RECORD_ONLY) and part2 (STOP_PARTITION).
    #[cfg(feature = "response")]
    if inject::window(1300, 20) {
        inject::spin_ms(120);
    }
    #[cfg(feature = "timing")]
    if cx.count.get() % 1024 == 0 {
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now().wrapping_sub(start) < rsk_target::ms(38) && i < 50_000_000 {
            i += 1;
        }
    }
    rsk::debug(&[b'B']);
}
