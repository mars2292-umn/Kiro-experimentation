//! The declaration of the measurement runs (task 9.2; cargo feature
//! `nrf52840_measure`): `nrf52840.rs` with an Operating_Duration of 300 s.
//! The Kernel's release-timing record (HIL builds, feature `trace-ram`) is
//! read through the debug probe, whose access halts the core; at the end of
//! the Operating_Duration the system is in its safe state and the record is
//! final, so the access no longer disturbs what it reads. Nothing else
//! differs: the Tasks, their Budgets and the channels are those of
//! `nrf52840.rs`, and the Analyzer's verdict is given on that variant's
//! Task_Model.
rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 300.s(),
    safe_state = crate::safe_state, log_capacity = 32;

    partition therapy (level = A, crate = pacemaker_therapy, code = 32.KiB(), ram = 32.KiB(), stack = 8.KiB(),
                       fault = RESTART_PARTITION, overrun = RESTART_PARTITION, deadline_miss = RESTART_PARTITION,
                       mit_threshold = 10) {
        peripherals = [GPIOTE, P0, TIMER1, TIMER2, TIMER3];
        resource timing: crate::Timing = crate::Timing::new();
        // The record of the arming sweep (task 7.2; feature `sweep` of the
        // Partition crate), read through the debug probe in one access.
        state control.sweep: [u32; 56] = [0; 56];
        // Released by the end of a pulse: compares 4 and 5 of TIMER3 (PM27.5).
        task pace (sporadic, binds = TIMER3, mit = 20.ms(), mit_policy = defer, deadline = 2.ms(),
                   budget = 400.us(), priority = 7, resources = [timing]);
        // Released by the Chamber's edge counter reaching its compare.
        task sense_v (sporadic, binds = TIMER2, mit = 50.ms(), mit_policy = defer, deadline = 3.ms(),
                      budget = 400.us(), priority = 6, resources = [timing]);
        task sense_a (sporadic, binds = TIMER1, mit = 50.ms(), mit_policy = defer, deadline = 4.ms(),
                      budget = 400.us(), priority = 5, resources = [timing]);
        task control (periodic, period = 20.ms(), deadline = 20.ms(), budget = 5.ms(), priority = 4,
                      resources = [timing]);
    }

    // A sense edge: its instant captured by TIMER3, and counted (the fork).
    ppi sense_a_edge (event = GPIOTE[0x100], task = TIMER3[0x048], fork = TIMER1[0x008]);
    ppi sense_v_edge (event = GPIOTE[0x104], task = TIMER3[0x04c], fork = TIMER2[0x008]);
    // A pulse: its start sets the pace pin and, by the fork, its copy; its
    // end clears both. Compare 0 and 4 the atrium, 1 and 5 the ventricle.
    ppi pace_a_start (event = TIMER3[0x140], task = GPIOTE[0x038], fork = GPIOTE[0x040]);
    ppi pace_v_start (event = TIMER3[0x144], task = GPIOTE[0x03c], fork = GPIOTE[0x044]);
    ppi pace_a_end (event = TIMER3[0x150], task = GPIOTE[0x068], fork = GPIOTE[0x070]);
    ppi pace_v_end (event = TIMER3[0x154], task = GPIOTE[0x06c], fork = GPIOTE[0x074]);
}
