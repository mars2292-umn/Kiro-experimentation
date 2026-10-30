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
        while rsk::now().wrapping_sub(start) < rsk_target::ms(8) && i < 10_000_000 {
            i += 1;
        }
        *s = s.saturating_add(1);
    });
    cx.in_cs.lock(leave);
    cx.count.with(|c| *c = c.saturating_add(1));
    rsk::debug(&[b'B']);
}
