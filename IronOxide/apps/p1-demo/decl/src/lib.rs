//! The P1 demo App_Declaration (design.md "App_Declaration and generation
//! pipeline"; task 10.5 build spike). Three periodic Tasks in two
//! Partitions on the QEMU mps2-an386 board:
//!
//! | Task | Partition | Priority | Period | Resources |
//! |---|---|---|---|---|
//! | `control` | `part0` (level A) | 7 | 10 ms | `state` |
//! | `sensor` | `part0` (level A) | 5 | 20 ms, offset 5 ms | `state` (held for 8 ms) |
//! | `log` | `part1` (level C) | 2 | 40 ms | none |
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

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 200.ms(),
    safe_state = crate::report_and_exit, log_capacity = 32;

    partition part0 (level = A, crate = p1_demo_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [];
        resource state: u32 = 0;
        task control (periodic, period = 10.ms(), offset = 0.ms(), deadline = 10.ms(), budget = 1.ms(),
                      priority = 7, resources = [state]);
        task sensor (periodic, period = 20.ms(), offset = 5.ms(), deadline = 20.ms(), budget = 8800.us(),
                     priority = 5, resources = [state]);
    }

    partition part1 (level = C, crate = p1_demo_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);
    }
}
