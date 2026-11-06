//! The release-timing configuration of the P1 demo on the HIL_Rig (task
//! 7.3, R9.10; feature `nrf52840_timing`): the `nrf52840` variant with the
//! operating duration raised to 61 minutes, every periodic offset 0 (all nominal
//! instants coincide every 200 ms), `sensor` holding `state` (Ceiling 7)
//! for 12 ms across `control`'s nominal instants (a Critical_Section in
//! progress at a nominal instant), and Partition 0's MIT threshold raised
//! so that the late Jobs the test injects (`sensor` runs 38 ms more every
//! 1024th Job under the Partition crate's `timing` feature, past two of
//! its nominal instants: R9.7 holds one Release, R9.8 discards the next
//! with an MIT_Violation) do not restart the Partition. `sensor` runs at
//! Priority 1 here (the lowest, tied with `sampler`), so that its late
//! Jobs starve no other Task: a late Job at Priority 5 cost `filter` and
//! `log` their deadlines and their Partitions a restart (the first short
//! run, 2026-11-02). Its Critical_Section still raises the ceiling to 7.
//! Kept in step with `nrf52840.rs` by hand.
//!
//! The base variant:
//! Partition 1's `pulse` is released by GPIOTE (an edge on P1.01, driven by
//! the HIL_Rig's stimulus board every 18 ms, task 7.1; before the rig,
//! TIMER1 stood in) and answers on P1.10 for the logic analyzer; it owns
//! GPIOTE and P0. Partition 2 owns SPIM0 for the sampler's EasyDMA
//! request. Kept in step with `lib.rs` by hand; the conformance suite's
//! anchors live in `lib.rs`.
rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 61.min(),
    safe_state = crate::report_and_exit, log_capacity = 32;

    partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 1000) {
        peripherals = [];
        resource state: u32 = 0;
        // Set by sensor around its Critical_Section on `state`, read by control:
        // shared between two Tasks, so a Resource (PR-05), not per-Task state.
        resource in_cs: u32 = 0 invariant |v: u32| v@ <= 1;
        signal tick;
        // Per-Task state (PR-25): the demo's counters, read by the safe state.
        state control.count: u32 = 0;
        state control.blocked: u32 = 0;
        state echo.count: u32 = 0;
        state sensor.count: u32 = 0;
        // The measurement build (task 19.2, R34.9, R35.4; the Partition crates'
        // `measure` feature): the largest iteration count each bounded loop
        // reached, by loop, reported by the safe state as `loop:part0:<fn>:<label>`.
        state sensor.loop_spin: u32 = 0;
        state sensor.loop_hold: u32 = 0;
        state sensor.loop_late: u32 = 0;
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 1.ms(),
                      priority = 7, resources = [state, in_cs], raises = [tick]);
        task echo (sporadic, signal = tick, mit = 20.ms(), mit_policy = defer, deadline = 20.ms(),
                   budget = 300.us(), priority = 6);
        task sensor (periodic, period = 20.ms(), offset = 0.ms(), deadline = 20.ms(), budget = 13000.us(),
                     priority = 1, resources = [state, in_cs]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [GPIOTE, P0];
        state log.count: u32 = 0;
        state pulse.count: u32 = 0;
        state log.loop_spin: u32 = 0;
        state log.loop_busy: u32 = 0;
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
        task pulse (sporadic, binds = GPIOTE, mit = 15.ms(), mit_policy = defer, deadline = 15.ms(),
                    budget = 500.us(), priority = 4);
    }

    partition part2 (level = B, crate = p1_demo_part2, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [SPIM0];
        // A Resource of the Partition's own type with a relational invariant
        // (R46.4, task 17.1): accepted transfers never exceed requested ones.
        resource stats: crate::Stats = crate::Stats::new() invariant |s: crate::Stats| s.ok@ <= s.total@;
        state filter.count: u32 = 0;
        state filter.y: f32 = 0.0;
        state sampler.count: u32 = 0;
        state sampler.dma_ok: u32 = 0;
        state sampler.buf: [u8; 16] = [0; 16];
        state filter.loop_hold: u32 = 0;
        state sampler.loop_spin: u32 = 0;
        // The EasyDMA traffic scenario of R17.4 and R34.5 (task 19.2): SPIM0
        // transfers over these buffers continuously (END_START shortcut) for
        // the whole run; `dma_end` counts the Jobs that found a transfer
        // ended since the last, `dma_after_stop` the restarts after which a
        // transfer still ran once the Kernel had stopped the peripheral (R17.6).
        state sampler.tx: [u8; 64] = [0; 64];
        state sampler.rx: [u8; 64] = [0; 64];
        state sampler.dma_end: u32 = 0;
        state sampler.dma_after_stop: u32 = 0;
        task filter (periodic, period = 25.ms(), deadline = 25.ms(), budget = 800.us(), priority = 3, fpu,
                     resources = [stats]);
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 500.us(),
                      priority = 1, resources = [stats]);
    }
}
