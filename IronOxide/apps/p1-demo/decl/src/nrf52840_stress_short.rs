//! The stress configuration of the fault-injection suite (task 7.5,
//! R18.8, R19.3; feature `nrf52840_stress_short`): `nrf52840.rs` over 3 minutes, the pipeline check
//! with the lower-criticality Partitions misbehaving throughout under
//! the Partition crates' `stress` feature (part1's `log` exceeding its
//! Budget outside a Critical_Section every fourth Job; part2's `filter`
//! holding its Resource for its maximal Critical_Section every Job and
//! exceeding its Budget inside it every eighth, `sampler` exceeding its
//! Budget every second Job) and the stimulus board driving `pulse`'s
//! source at the highest rate it can generate (`hil-stimulus` feature
//! `storm`). The rig checks that every Job of part0 (level A) completes
//! within the WCRT the Analyzer computes and that the source costs one
//! intake per MIT (Verification_Plan 11.4).
//! Outside the schedulability claim (2026-11-05, task 19.2): its Tasks
//! exceed their Budgets and deadlines by design, so its measured analysis
//! supplies the Kernel_Timing_Parameters and interference figures and
//! makes no claim about the set (Verification_Plan 11.11).
rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 3.min(),
    safe_state = crate::report_and_exit, log_capacity = 32;

    partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
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
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8800.us(),
                     priority = 5, resources = [state, in_cs]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [GPIOTE, P0];
        state log.count: u32 = 0;
        state pulse.count: u32 = 0;
        state log.loop_spin: u32 = 0;
        state log.loop_busy: u32 = 0;
        // Budget 4 ms here (6 in the plain demo; the Job's 3 ms wait measures
        // 3.2 ms, and the injected 20 ms spins overrun any Budget): with the
        // measured Kernel_Timing_Parameters the set's utilization passed
        // 100 % at 6 ms and the Analyzer computed no WCRT at all; at 4 ms the
        // fixed point exists and part0's WCRTs are the claim of task 19.4.
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 4.ms(), priority = 2);
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
        // Budget 800 µs here (500 in the plain demo): the first Job of each
        // incarnation starts the EasyDMA traffic scenario (task 19.2), which
        // the restarts of this configuration repeat every 200 ms, and its
        // measured record with the margin must stay within the Budget for
        // the measured analysis to have a verdict (R28.4; task 19.4).
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 800.us(),
                      priority = 1, resources = [stats]);
    }
}
