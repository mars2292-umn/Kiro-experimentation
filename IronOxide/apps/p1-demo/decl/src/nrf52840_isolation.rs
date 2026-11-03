//! The isolation configuration of the HIL suites of task 7.4 (R16.8,
//! R17.7) and the Hardware_Model differential tests of task 3.3 (R49.3;
//! feature `nrf52840_isolation`): `nrf52840.rs` over 3.6 seconds with
//! every Partition's fault response END_JOB, so that one Task per
//! Partition can walk the access matrix one attempt per Job (`control`
//! for part0, `log` for part1, `sampler` for part2 after its EasyDMA
//! cases), each refused access ending that Job with a MemManage or
//! BusFault event the rig matches against the attempt (Verification_Plan
//! 11.7); part2's Overrun_Response RESTART_PARTITION, triggered by
//! `filter` at 50 ms, so that the rig sees the Partition's EasyDMA stop
//! precede its re-initialization (R17.6); and `filter`'s Budget raised to
//! 15 ms for the register-pattern probe it runs across preemptions
//! (HW-06, HW-10). The per-Task state records what each probe saw, for
//! the safe state's report.
rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 3600.ms(),
    safe_state = crate::report_and_exit, log_capacity = 32;

    partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = END_JOB, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
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
        // The access matrix: the next attempt, and a bit per attempt that returned.
        state control.matrix: u32 = 0;
        state control.survived: u32 = 0;
        state echo.count: u32 = 0;
        state sensor.count: u32 = 0;
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 1.ms(),
                      priority = 7, resources = [state, in_cs], raises = [tick]);
        task echo (sporadic, signal = tick, mit = 20.ms(), mit_policy = defer, deadline = 20.ms(),
                   budget = 300.us(), priority = 6);
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8800.us(),
                     priority = 5, resources = [state, in_cs]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = END_JOB, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [GPIOTE, P0];
        state log.count: u32 = 0;
        state log.matrix: u32 = 0;
        state log.survived: u32 = 0;
        state pulse.count: u32 = 0;
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
        task pulse (sporadic, binds = GPIOTE, mit = 15.ms(), mit_policy = defer, deadline = 15.ms(),
                    budget = 500.us(), priority = 4);
    }

    partition part2 (level = B, crate = p1_demo_part2, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = END_JOB, overrun = RESTART_PARTITION, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [SPIM0];
        // A Resource of the Partition's own type with a relational invariant
        // (R46.4, task 17.1): accepted transfers never exceed requested ones.
        resource stats: crate::Stats = crate::Stats::new() invariant |s: crate::Stats| s.ok@ <= s.total@;
        state filter.count: u32 = 0;
        state filter.y: f32 = 0.0;
        // The register-pattern probe (HW-06, HW-10): runs, failures, the
        // union of the mismatch masks, and FPSCR as the first Job found it.
        state filter.pattern_runs: u32 = 0;
        state filter.pattern_fail: u32 = 0;
        state filter.pattern_mask: u32 = 0;
        state filter.fpscr: u32 = 0xFFFF_FFFF;
        state sampler.count: u32 = 0;
        state sampler.dma_ok: u32 = 0;
        state sampler.buf: [u8; 16] = [0; 16];
        // The EasyDMA cases then the access matrix: the next case, the
        // buffer's address, and a bit per matrix attempt that returned.
        state sampler.dma_case: u32 = 0;
        state sampler.dma_ptr: u32 = 0;
        state sampler.survived: u32 = 0;
        task filter (periodic, period = 25.ms(), deadline = 25.ms(), budget = 15.ms(), priority = 3, fpu,
                     resources = [stats]);
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 500.us(),
                      priority = 1, resources = [stats]);
    }
}
