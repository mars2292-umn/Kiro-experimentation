//! The cycle-attribution configuration of task 7.5's `hil-cycles` (R18.1,
//! ASM-06; feature `nrf52840_cycles`): `nrf52840.rs` over 2 seconds with
//! the Partition crates' `cycles` feature, under which `control` (the
//! highest Priority, preempted by nothing but the Kernel's own entries)
//! and `log` (the lowest periodic Priority, preempted by every other Task
//! and by the Kernel's work for them) each execute the same fixed-
//! iteration spin in every Job. The Budget accounting charges each Job
//! its own cycles (R18.1), so the two Tasks' executions agree although
//! `log`'s Jobs take many times longer to complete; the rig compares the
//! Kernel's record of both (Verification_Plan 11.11). `control`'s Budget
//! is 2 ms for the spin, `control` does nothing but the spin (its lock and
//! raise SVCs would be its own cost and not compare), and `log` is
//! released 4 ms into its period so that `sensor`'s Critical_Section
//! preempts every one of its Jobs.
rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 2.s(),
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
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 2.ms(),
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
        // Offset 4 ms: the 1.2 ms Job is in progress when sensor's 6 ms
        // Critical_Section starts at 5 ms, so that every Job is preempted for
        // several times its own execution.
        task log (periodic, period = 40.ms(), offset = 4.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
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
        task filter (periodic, period = 25.ms(), deadline = 25.ms(), budget = 800.us(), priority = 3, fpu,
                     resources = [stats]);
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 500.us(),
                      priority = 1, resources = [stats]);
    }
}
