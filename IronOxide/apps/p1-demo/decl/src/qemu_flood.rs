//! The flood configuration (feature `qemu_flood`; task 21.6, the A-P7
//! limitation of Verification_Plan 5.15): the QEMU declaration of `lib.rs`
//! with two differences. `part1`'s fault response is RESTART_PARTITION
//! (STOP_PARTITION in `lib.rs`), so that a rejected message (R42.4) does
//! not stop the Partition and the next incarnation sends again: of the
//! fault responses of R14.5 (RECORD_AND_CONTINUE is an Overrun_Response
//! only) END_JOB escalates to the restart as well, since no Job of the
//! sending Task executes when the receiver rejects (`Kernel::fault_of`),
//! so the restart is declared as what happens. The operating duration is
//! 500 ms. Under the Partition
//! crates' `qemu-flood` feature `log` answers EVERY `pingpong` request with
//! the invalid reply 0xFFFF_FFFF, which `control`'s `validate_rep` refuses:
//! each rejection re-initializes the Endpoint, both roles return to S0
//! through the peer-state error, and the next round starts again, so the
//! Protocol never advances past a round while `control` keeps every
//! deadline and the re-initializations are bounded by `log`'s operation
//! rate (one reply per 40 ms Job) and its fault response. The safe state
//! reports the counts (`cargo xtask qemu --flood`). Kept in step with
//! `lib.rs` by hand; the comments there apply.
rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 500.ms(),
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
        // Task 21.2: sends the `telemetry` Endpoint refused as full (R40.5).
        state control.telemetry_full: u32 = 0;
        // Task 21.3: replies received over `pingpong`, and the last one.
        state control.pingpong_reps: u32 = 0;
        state control.pingpong_last: u32 = 0;
        // Task 21.4: peer-state errors seen on each Endpoint (R42.3) and
        // replies the validation function rejected (R42.4).
        state control.telemetry_peer: u32 = 0;
        state control.pingpong_peer: u32 = 0;
        state control.pingpong_rejected: u32 = 0;
        // Task 21.6 (R41.3, the gating of Endpoints): requests answered as
        // the `sink` of `report`.
        state control.report_reqs: u32 = 0;
        // Task 21.7 (multiparty Endpoints): rounds completed as the `ctl` of `triangle`.
        state control.triangle_rounds: u32 = 0;
        state echo.count: u32 = 0;
        state sensor.count: u32 = 0;
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 1.ms(),
                      priority = 7, resources = [state, in_cs], raises = [tick]);
        task echo (sporadic, signal = tick, mit = 20.ms(), mit_policy = defer, deadline = 20.ms(),
                   budget = 300.us(), priority = 6);
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8700.us(),
                     priority = 5, resources = [state, in_cs]);
    }

    // Task 21.4: `fault = STOP_PARTITION` since 2026-11-07 (RESTART_PARTITION
    // before): the one fault of this Partition in the run is the reply its
    // `log` sends to the level-A `control` that `control`'s validation
    // function rejects (R42.4); the stop keeps the Partition's counters for
    // the safe state's report where a restart would zero them, and shows
    // the peer-state error on `control`'s next operations (R42.3).
    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [CMSDK_DUALTIMER];
        state log.count: u32 = 0;
        state pulse.count: u32 = 0;
        // Task 21.2: messages received over `telemetry` and the last one.
        state log.telemetry_rx: u32 = 0;
        state log.telemetry_last: u32 = 0;
        // Task 21.3: requests answered over `pingpong`.
        state log.pingpong_reqs: u32 = 0;
        // Task 21.6 (R41.3): rounds completed as the `ctl` of `relay`.
        state log.relay_rounds: u32 = 0;
        // Task 21.7: requests forwarded as the `mid` of `triangle`.
        state log.triangle_fwd: u32 = 0;
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
        task pulse (sporadic, binds = CMSDK_DUALTIMER, mit = 15.ms(), mit_policy = defer, deadline = 15.ms(),
                    budget = 500.us(), priority = 4);
    }

    partition part2 (level = B, crate = p1_demo_part2, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [CMSDK_GPIO0];
        // A Resource of the Partition's own type with a relational invariant
        // (R46.4, task 17.1): accepted transfers never exceed requested ones.
        resource stats: crate::Stats = crate::Stats::new() invariant |s: crate::Stats| s.ok@ <= s.total@;
        state filter.count: u32 = 0;
        state filter.y: f32 = 0.0;
        // Task 21.6 (R41.3): requests answered as the `sink` of `relay`, and
        // rounds completed as the `ctl` of `report`.
        state filter.relay_reqs: u32 = 0;
        state filter.report_rounds: u32 = 0;
        // Task 21.7: acknowledgements sent as the `sink` of `triangle`.
        state filter.triangle_acks: u32 = 0;
        state sampler.count: u32 = 0;
        state sampler.dma_ok: u32 = 0;
        state sampler.buf: [u8; 16] = [0; 16];
        task filter (periodic, period = 25.ms(), deadline = 25.ms(), budget = 800.us(), priority = 3, fpu,
                     resources = [stats]);
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 500.us(),
                      priority = 1, resources = [stats]);
    }

    // Task 21.2 (R38.1, R40.1, R42.2): a cross-Partition Endpoint from the
    // level-A `control` to the level-C `log`: four `u32` messages each way
    // in Kernel-owned storage; `control` sends its count every Job and `log`
    // drains the channel every Job (no arrival release: `log` is periodic).
    // Task 21.5 (R40.2): `log` drains up to the capacity per Job, `ops = 4`.
    // Task 21.4 (R42.4): the channel from `log` (level C) back to `control`
    // (level A) exists although `control` never reads it, so `ctl` declares
    // its validation function like `pingpong`'s.
    endpoint telemetry: u32 { ctl => part0.control validate = crate::validate_rep, sink => part1.log ops = 4 } capacity = 4, payload = 4;

    // Task 21.1 (R38.2, R38.3, R41.1): a Protocol-governed Endpoint between
    // the same Tasks, a request-response loop with a stop branch; the
    // Protocol_Checker projects it onto `ctl` and `sink` and bounds every
    // channel at one message, which the capacity of 1 meets; the payload of
    // 8 bytes is the label word and a `u32`. The typestate API over it is
    // task 21.3: `control` plays `ctl` and `log` plays `sink`.
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
    // Task 21.4 (R42.4): `Rep` crosses from level C to level A, so `ctl`
    // declares its validation function, `control`'s `validate_rep`,
    // proven total by the App_Prover with the bodies; a rejected reply is
    // the fault of `part1` (its STOP_PARTITION), whose peer-state change
    // `control` then sees on both Endpoints (R42.3).
    endpoint pingpong: protocol PingPong { ctl => part0.control validate = { Rep = crate::validate_rep }, sink => part1.log ops = 2 } capacity = 1, payload = 8;

    // Task 21.6 (R41.3, A-P8; 2026-11-08): the gating of Endpoints with a
    // non-trivial ranking. Two more Endpoints under the same Protocol:
    // `log` relays each request it answered on `pingpong` to `filter` as the
    // `ctl` of `relay` (its operations on `relay` wait for its receives on
    // `pingpong`: `after = pingpong`), and `filter` reports each request it
    // answered on `relay` to `control` as the `ctl` of `report` (`after =
    // relay`). The gating relation is pingpong -> relay -> report, ranks 0,
    // 1, 2; `log` holds two Protocol roles (pingpong's sink, relay's ctl),
    // `filter` two (relay's sink, report's ctl), and `control` two
    // (pingpong's ctl, report's sink). R42.4: `relay`'s `Req` crosses from
    // level C to level B and `report`'s from level B to level A, so each
    // `sink` declares its validation function for `Req`; `Rep` travels
    // upward in Criticality_Level on both and needs none. Each `sink`
    // receives and replies in one Job (`ops = 2`).
    endpoint relay: protocol PingPong { ctl => part1.log after = pingpong, sink => part2.filter validate = { Req = crate::validate_req } ops = 2 } capacity = 1, payload = 8;
    endpoint report: protocol PingPong { ctl => part2.filter after = relay, sink => part0.control validate = { Req = crate::validate_rep } ops = 2 } capacity = 1, payload = 8;

    // Task 21.7 (multiparty Endpoints; R38.2, R38.3, R41.1, R41.3): a
    // three-role Protocol, one channel per ordered pair of roles. `ctl`
    // chooses; `mid` learns the choice from the label it receives; `sink`
    // is not informed by the choice and receives the merge of the two
    // branches (`Fwd` or `Done`). R42.4: `Fwd` crosses from level C to
    // level B and `Ack` from level B to level A, so `sink` and `ctl`
    // declare their validation functions; `Req` travels downward and
    // needs none. `mid` and `sink` each receive and send in one Job
    // (`ops = 2`); `ctl` performs one operation per Job. No `after`: the
    // roles are independent of the Tasks' other roles.
    protocol Triangle {
        rec L {
            choice at ctl {
                Req(u32): ctl -> mid;
                Fwd(u32): mid -> sink;
                Ack(u32): sink -> ctl;
                continue L;
            } or {
                Stop(): ctl -> mid;
                Done(): mid -> sink;
                end;
            }
        }
    }
    endpoint triangle: protocol Triangle { ctl => part0.control validate = { Ack = crate::validate_rep }, mid => part1.log ops = 2, sink => part2.filter validate = { Fwd = crate::validate_req } ops = 2 } capacity = 1, payload = 8;

    // Task 21.5 (R41.4): the end-to-end deadline of the telemetry path: a
    // word `control` sends is read by `log`, a Periodic_Task that samples
    // the Endpoint once per period, so the Analyzer's bound is R_control +
    // T_log + R_log, within 100 ms with the placeholder timing.
    chain telemetry_path { part0.control -> part1.log } deadline = 100.ms();
}
