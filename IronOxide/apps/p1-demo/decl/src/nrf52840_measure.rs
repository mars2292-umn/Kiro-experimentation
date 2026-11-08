//! The plain measurement configuration of task 19.4 (feature
//! `nrf52840_measure`): `nrf52840.rs` over 3 minutes with no injection,
//! the Partition crates' `measure` feature alone (the loop report and the
//! EasyDMA traffic scenario, Verification_Plan 11.13). Its records are the
//! measured WCETs and Kernel_Timing_Parameters of the demo as declared,
//! which the Analyzer takes for the schedulability claim under measured
//! timing (11.14), and the rig confirms every Task's observed response
//! time within the computed WCRT ("without injected Overruns", R18.7,
//! R29.5). Kept in step with `nrf52840.rs` by hand.
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
        // Tasks 21.2 to 21.4 (the `telemetry` feature of the Partition crates):
        // the Endpoint counters the QEMU configuration declares.
        state control.telemetry_full: u32 = 0;
        state control.pingpong_reps: u32 = 0;
        state control.pingpong_last: u32 = 0;
        state control.telemetry_peer: u32 = 0;
        state control.pingpong_peer: u32 = 0;
        state control.pingpong_rejected: u32 = 0;
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
        // Task 21.5 (2026-11-07): 8.25 ms here (8.8 ms before, as the other HIL
        // configurations): with the Endpoint operations charged (R40.2) the
        // measured analysis put `log` 58 us and `sampler` 1.3 ms over their
        // deadlines; sensor's measured Job is about 6.4 ms (7.9 ms with the
        // margin), so the Budget keeps its headroom and the two lowest Tasks
        // their schedulability.
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8250.us(),
                     priority = 5, resources = [state, in_cs]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [GPIOTE, P0];
        state log.count: u32 = 0;
        state log.telemetry_rx: u32 = 0;
        state log.telemetry_last: u32 = 0;
        state log.pingpong_reqs: u32 = 0;
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
        // Budget 600 µs here (500 in the plain demo): the first Job of the
        // run starts the EasyDMA traffic scenario (task 19.2; eight Kernel
        // services and the buffers' setup, 25,786 cycles measured), which
        // with the 25 % margin passes the plain Budget by 0.7 % and would
        // have the measured analysis rejected under R28.4.
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 600.us(),
                      priority = 1, resources = [stats]);
    }

    // Tasks 21.2 to 21.5: the QEMU configuration's Endpoints, Protocol, and
    // chain on the board: the measurement configuration measures Δ_send and Δ_recv on them (task 21.5, R40.2) and the span record their masking.
    endpoint telemetry: u32 { ctl => part0.control validate = crate::validate_rep, sink => part1.log ops = 4 } capacity = 4, payload = 4;
    protocol PingPong {
        rec Loop {
            choice at ctl {
                Req(u32): ctl -> sink;
                Rep(u32): sink -> ctl;
                continue Loop;
            } or {
                Stop(): ctl -> sink;
                end;
            }
        }
    }
    endpoint pingpong: protocol PingPong { ctl => part0.control validate = { Rep = crate::validate_rep }, sink => part1.log ops = 2 } capacity = 1, payload = 8;
    chain telemetry_path { part0.control -> part1.log } deadline = 100.ms();
}
