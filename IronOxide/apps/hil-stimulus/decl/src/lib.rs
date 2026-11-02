//! The HIL_Rig stimulus Application (task 7.1, R34.8, R48.4): an rsk
//! Application for the rig's stimulus nRF52840-DK, not a Flight_Build. One
//! Partition, one periodic Task `pulse` (every 18 ms, the period the P1
//! demo's `pulse` Release_Source expects: longer than its 15 ms MIT), which
//! drives a 100 µs high pulse on P1.01 (to the target board's P1.01, its
//! GPIOTE input) and, in the same register write, on P1.05 (to the logic
//! analyzer's D6), and blinks LED1 every 28 pulses (`hil-rig.toml`).
//!
//! Using rsk itself for the stimulus keeps every line of bench firmware
//! under the same rules as the Application under test (no unsafe code
//! outside the Kernel) and exercises a second board with the same Kernel.
//! The operating duration covers PAR-05 (the longer of 1 hour and 1,000
//! hyperperiods) with margin; the safe state after it idles with the
//! watchdog fed, so the board stays quiet until the rig resets it.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 2.h(),
    safe_state = crate::halt, log_capacity = 16;

    partition stim (level = C, crate = hil_stimulus_part, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                    fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                    mit_threshold = 10) {
        peripherals = [P0];
        state pulse.count: u32 = 0;
        task pulse (periodic, period = 18.ms(), deadline = 18.ms(), budget = 1.ms(), priority = 1);
    }
}
