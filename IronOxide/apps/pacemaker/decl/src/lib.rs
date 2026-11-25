//! The App_Declaration of the PACEMAKER experiment's pulse generator
//! (docs/pacemaker/design.md, "Partitions, Tasks and Resources"; task 8.1).
//!
//! Stage E1 declares the Therapy_Partition alone: the Partition `therapy`
//! at Criticality_Level A with the Tasks of Table 2 that belong to it
//! (`pace`, `sense_v`, `sense_a`, `control`) and the Resource `timing`
//! that all four share. `comms` and `diag` come with stages E3 and E5.
//!
//! Two variants, one per Target (PM26.4):
//!
//! - the nRF52840-DK (`src/nrf52840.rs`, cargo feature `nrf52840`): the
//!   five peripherals of Table 1, the three sporadic Tasks bound to
//!   TIMER3, TIMER2 and TIMER1, and the six fixed PPI channels of
//!   "Hardware signal flow", each with its fork;
//! - QEMU (below, the default): the Target has no PPI and one
//!   Partition-ownable timer, so `pace` is bound to that timer and the two
//!   sense Tasks are released by Release_Signals that `control` raises as
//!   it plays a table of Senses (design, "The hardware layer"). It is for
//!   the logic, not for timing claims.
//!
//! The deadlines and the board's Budgets are those tasks 9.2 and 9.8 settled
//! on the measured records (design.md, Table 2 and "Timing Design"): 350 µs
//! for each sporadic Task (the longest Job measured took 234 µs) and
//! deadlines of 2, 3 and 4 ms for `pace`, `sense_v` and `sense_a`. With
//! Budgets of 400 µs the Analyzer's bound for `sense_a` was 36 µs over its
//! 4 ms once the Kernel's fault entry and stubs were measured; 350 µs was
//! the user's decision of 2026-11-23. Since 2026-11-24 they are 450 µs, also
//! by the user's decision: on the stage E3 code a Job of 261 µs with two of
//! the Kernel's timer entries inside it reached 350 µs in DDD and restarted
//! the Partition (findings B36). The three responses of the Partition
//! are RESTART_PARTITION (PM23.3), so a Budget set too low restarts pacing.
//! The QEMU variant keeps Budgets of 1 ms: its execution times are the
//! emulator's and are not measured.
#![no_std]
#![forbid(unsafe_code)]

#[cfg(all(feature = "nrf52840", not(any(feature = "nrf52840_sweep", feature = "nrf52840_measure", feature = "nrf52840_rig", feature = "nrf52840_rig_stop", feature = "nrf52840_coverage"))))]
mod nrf52840;
#[cfg(feature = "nrf52840_sweep")]
mod nrf52840_sweep;
#[cfg(all(feature = "nrf52840_measure", not(feature = "nrf52840_sweep")))]
mod nrf52840_measure;
#[cfg(all(feature = "nrf52840_rig", not(any(feature = "nrf52840_sweep", feature = "nrf52840_measure", feature = "nrf52840_rig_stop"))))]
mod nrf52840_rig;
#[cfg(all(feature = "nrf52840_rig_stop", not(any(feature = "nrf52840_sweep", feature = "nrf52840_measure"))))]
mod nrf52840_rig_stop;
#[cfg(all(feature = "nrf52840_coverage", not(any(feature = "nrf52840_sweep", feature = "nrf52840_measure", feature = "nrf52840_rig"))))]
mod nrf52840_coverage;

#[cfg(not(feature = "nrf52840"))]
rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 3000.ms(),
    safe_state = crate::safe_state, log_capacity = 32;

    partition therapy (level = A, crate = pacemaker_therapy, code = 32.KiB(), ram = 32.KiB(), stack = 12.KiB(),
                       fault = RESTART_PARTITION, overrun = RESTART_PARTITION, deadline_miss = RESTART_PARTITION,
                       mit_threshold = 10) {
        peripherals = [CMSDK_DUALTIMER];
        // The step function's state, the driver's, the board's own words
        // and the event ring (design: Resource `timing`, Ceiling 7).
        resource timing: crate::Timing = crate::Timing::new();
        // QEMU only: the Release_Sources of the sense Tasks.
        signal sensed_a;
        signal sensed_v;
        task pace (sporadic, binds = CMSDK_DUALTIMER, mit = 20.ms(), mit_policy = defer, deadline = 2.ms(),
                   budget = 1.ms(), priority = 7, resources = [timing]);
        task sense_v (sporadic, signal = sensed_v, mit = 50.ms(), mit_policy = defer, deadline = 3.ms(),
                      budget = 1.ms(), priority = 6, resources = [timing]);
        task sense_a (sporadic, signal = sensed_a, mit = 50.ms(), mit_policy = defer, deadline = 4.ms(),
                      budget = 1.ms(), priority = 5, resources = [timing]);
        task control (periodic, period = 20.ms(), deadline = 20.ms(), budget = 5.ms(), priority = 4,
                      resources = [timing], raises = [sensed_a, sensed_v]);
    }
}
