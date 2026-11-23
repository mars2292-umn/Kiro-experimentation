//! The Heart_Simulator of the PACEMAKER experiment (docs/pacemaker,
//! requirement PM26.3, task 4.2): an rsk Application for the rig's stimulus
//! nRF52840-DK (board B), not a Flight_Build. One Partition, one periodic
//! Task `beat` (every 1 ms) that plays a scenario: it drives the two sense
//! lines to the PG (P1.01 atrial, P1.02 ventricular) with a mirror of each
//! for the logic analyzer in the same register write (P1.05, P1.06), and in
//! a responsive scenario reads the PG's pace pulses on P1.03 and P1.04
//! (`hil-rig.toml`, keys `pacemaker_*`).
//!
//! Like the rig's stimulus Application it is bench firmware under rsk's
//! own rules: no unsafe code outside the Kernel. The operating duration
//! covers an hour-long scenario with margin; the safe state after it idles
//! with the watchdog fed, so the board stays quiet until it is reset.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 2.h(),
    safe_state = crate::halt, log_capacity = 16;

    partition heart (level = C, crate = heart_sim_part, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [P0];
        // The simulated heart, one word per field (PR-25; all zero at start).
        state beat.ms: u32 = 0;
        state beat.next_a: u32 = 0;
        state beat.next_v: u32 = 0;
        state beat.conducted: u32 = 0;
        state beat.beats_a: u32 = 0;
        state beat.beats_v: u32 = 0;
        // The pace pins seen high by the Job before (a pulse spans Jobs).
        state beat.pace_seen: u32 = 0;
        // A Job is a few register accesses and, on a beat, a 100 us pulse.
        task beat (periodic, period = 1.ms(), deadline = 1.ms(), budget = 500.us(), priority = 1);
    }
}
