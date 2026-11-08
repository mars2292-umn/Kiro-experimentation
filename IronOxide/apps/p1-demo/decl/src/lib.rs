//! The P1 demo App_Declaration (design.md "App_Declaration and generation
//! pipeline"; task 10.5 build spike). Three periodic Tasks in two
//! Partitions on the QEMU mps2-an386 board:
//!
//! | Task | Partition | Priority | Period | Resources |
//! |---|---|---|---|---|
//! | `control` | `part0` (level A) | 7 | 10 ms | `state` |
//! | `sensor` | `part0` (level A) | 5 | 20 ms, offset 5 ms | `state` (held for 8 ms) |
//! | `echo` | `part0` (level A) | 6 | sporadic, MIT 20 ms, released by the signal `tick` | none |
//! | `log` | `part1` (level C) | 2 | 40 ms | none |
//! | `pulse` | `part1` (level C) | 4 | sporadic, MIT 15 ms, bound to CMSDK_DUALTIMER (nRF52840: GPIOTE, the HIL_Rig's stimulus line) | none |
//!
//! `pulse`'s Budget is 500 µs (task 19.1, 2026-11-02): the HIL build's agent measured its Job at 0.278 ms with the Budget accounting charging the SVC handlers' time to the Job, over the 200 µs first declared, and under the storm the ended Jobs answered a third of the windows.
//!
//! `echo`'s Budget is 300 µs (200 first) and `filter`'s 800 µs (500 first) since the WCET_Records of task 19.1 (2026-11-02): the measured executions with the 25 % margin of R34.6 are 15,712 and 49,419 cycles, and R28.4 keeps every Budget at or above its WCET.
//! `sensor`'s Budget is 8.7 ms here (8.8 ms first, 8.75 ms from 2026-11-06 to 2026-11-08, and still 8.8 ms in the HIL configurations): its Critical_Section is 6 ms (8 first) since the same records, so that its measured Job (about 6.4 ms, 7.9 ms with the margin) fits the Budget and `control`'s measured blocking term (about 7.7 ms with the margin) fits its 10 ms deadline. With placeholder timing every Critical_Section is its Task's whole Budget, which put `control` at exactly its deadline (B = 8.8 ms, C = 1 ms, O and J the rest); the 50 µs of 2026-11-06 gave it the slack for the intake terms of R19.2 (`echo`'s and `pulse`'s sources), and the further 50 µs of 2026-11-08 (task 21.6) the slack for the two operations per Job of its `sink` role on `report` (R40.2: 2 × max(Δ_send, Δ_recv) in O_i, 22 µs with the placeholders), both staying above the measured Job.
//! | `filter` | `part2` (level B) | 3 | 25 ms; uses the FPU | none |
//! | `sampler` | `part2` (level B) | 1 | 100 ms; EasyDMA requests on CMSDK_GPIO0 | none |
//!
//! `part2` (task 14, R3.7) adds the constructs the accepted Application
//! must exercise: a Partition at Criticality_Level B, a Task with an
//! active floating-point context (`fpu`; R11, PR-31), and an EasyDMA
//! peripheral (the QEMU stand-in channel of CMSDK_GPIO0, DD-08) whose
//! transfers go through `rsk::dma_request`.
//!
//! `pulse` is the sporadic Task (DD-06, task 5.4): `log`'s first Job starts
//! the Partition's dual timer at an 18 ms period through the peripheral
//! window (`rsk::mmio`), each interrupt is a Release_Source event taken by
//! the Kernel's intake handler, and `pulse` clears the interrupt. The 18 ms
//! period exceeds the 15 ms MIT, so every event is accepted and the source
//! is masked and unmasked once per event (the unmask event of DD-06). The
//! 15 ms deadline covers sensor's 8.8 ms Budget (a higher Priority: it
//! interferes with pulse in full, and its 6 ms Critical_Section blocks a
//! Release that lands inside it, R8.4) plus two control Jobs, as the
//! Analyzer computes (R = 11.5 ms); with a 2 ms deadline the Kernel
//! reported a deadline miss at every Release that coincided with sensor's
//! Critical_Section, and with 10 ms the Analyzer rejected the set.
//!
//! `echo` is released by the Release_Signal `tick`, which `control` raises
//! on every third Job (every 30 ms, so every raise is accepted against the
//! 20 ms MIT): the raise path of R10.5 (`rsk::Signal::raise`, SVC 7) on
//! QEMU. Its 20 ms deadline covers sensor's Budget and Critical_Section
//! (Priority 6 is below Ceiling 7) and one control Job.
//!
//! Budgets (1 ms, 8.8 ms, 6 ms) bound the Jobs' real execution on the board
//! (QEMU's DWT does not count cycles, so they are not enforced there). The
//! sensor's 8 ms Critical_Section starting 5 ms into each period straddles
//! a control Release (blocking, R8.4) and leaves control schedulable in the
//! worst case (R = 1 + 8.8 ms + overheads < 10 ms); Partition 0 at the
//! highest level lets an overrunning Job complete (RECORD_AND_CONTINUE),
//! so its interference term is its Budget (R29.4). The Analyzer reports
//! the set schedulable (`cargo xtask analyze`).
//!
//! `rsk::app!` validates the declaration and exports `rsk_system!` (for
//! the system crate) and `rsk_partition_part0!` / `rsk_partition_part1!`
//! (for the Partition crates). `cargo xtask gen` runs the Generator CLI on
//! this file for the Task_Model, the linker script and the summary
//! (`apps/p1-demo/gen/`), and `cargo xtask verify` checks that they are up
//! to date.
#![no_std]
#![forbid(unsafe_code)]

// The Target variant: the default declaration targets the QEMU board of
// the pre-hardware spike; with the cargo feature `nrf52840` the declaration
// of `nrf52840.rs` (the same Application on the nRF52840-DK, with that
// board's peripherals) is compiled instead. `cargo xtask gen` generates
// both (`gen/` and `gen-nrf52840/`; policy `variants`).
#[cfg(all(
    feature = "nrf52840",
    not(any(feature = "nrf52840_timing", feature = "nrf52840_timing_short", feature = "nrf52840_response", feature = "nrf52840_stress", feature = "nrf52840_stress_short", feature = "nrf52840_isolation", feature = "nrf52840_cycles", feature = "nrf52840_cycles_fpu", feature = "nrf52840_measure", feature = "nrf52840_coverage"))
))]
mod nrf52840;
#[cfg(feature = "nrf52840_timing")]
mod nrf52840_timing;
#[cfg(feature = "nrf52840_timing_short")]
mod nrf52840_timing_short;
#[cfg(feature = "nrf52840_response")]
mod nrf52840_response;
#[cfg(feature = "nrf52840_stress")]
mod nrf52840_stress;
#[cfg(feature = "nrf52840_stress_short")]
mod nrf52840_stress_short;
#[cfg(feature = "nrf52840_isolation")]
mod nrf52840_isolation;
#[cfg(feature = "nrf52840_cycles")]
mod nrf52840_cycles;
#[cfg(feature = "nrf52840_cycles_fpu")]
mod nrf52840_cycles_fpu;
#[cfg(feature = "nrf52840_measure")]
mod nrf52840_measure;
#[cfg(feature = "nrf52840_coverage")]
mod nrf52840_coverage;
// The flood configuration (task 21.6, A-P7): the QEMU declaration with
// `part1` at `fault = RESTART_PARTITION` over 500 ms (`cargo xtask qemu --flood`).
#[cfg(feature = "qemu_flood")]
mod qemu_flood;

#[cfg(not(any(feature = "nrf52840", feature = "qemu_flood")))]
rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 200.ms(),
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
                     fault = STOP_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
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

    // Task 21.5 (R41.4): the end-to-end deadline of the telemetry path: a
    // word `control` sends is read by `log`, a Periodic_Task that samples
    // the Endpoint once per period, so the Analyzer's bound is R_control +
    // T_log + R_log, within 100 ms with the placeholder timing.
    chain telemetry_path { part0.control -> part1.log } deadline = 100.ms();
}
