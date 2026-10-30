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
//! | `pulse` | `part1` (level C) | 4 | sporadic, MIT 15 ms, bound to CMSDK_DUALTIMER | none |
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
//! interferes with pulse in full, and its 8 ms Critical_Section blocks a
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
#[cfg(feature = "nrf52840")]
mod nrf52840;

#[cfg(not(feature = "nrf52840"))]
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
        state echo.count: u32 = 0;
        state sensor.count: u32 = 0;
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 1.ms(),
                      priority = 7, resources = [state, in_cs], raises = [tick]);
        task echo (sporadic, signal = tick, mit = 20.ms(), mit_policy = defer, deadline = 20.ms(),
                   budget = 200.us(), priority = 6);
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8800.us(),
                     priority = 5, resources = [state, in_cs]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [CMSDK_DUALTIMER];
        state log.count: u32 = 0;
        state pulse.count: u32 = 0;
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
        task pulse (sporadic, binds = CMSDK_DUALTIMER, mit = 15.ms(), mit_policy = defer, deadline = 15.ms(),
                    budget = 200.us(), priority = 4);
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
        state sampler.count: u32 = 0;
        state sampler.dma_ok: u32 = 0;
        state sampler.buf: [u8; 16] = [0; 16];
        task filter (periodic, period = 25.ms(), deadline = 25.ms(), budget = 500.us(), priority = 3, fpu,
                     resources = [stats]);
        task sampler (periodic, period = 100.ms(), deadline = 100.ms(), budget = 500.us(),
                      priority = 1, resources = [stats]);
    }
}
