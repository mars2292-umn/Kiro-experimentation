//! The PPI probe of the PACEMAKER experiment (docs/pacemaker, requirement
//! PM33.5, task 3.6): an rsk Application for the rig's target board (board
//! A) that declares fixed PPI channels, so that the Kernel's write of them
//! runs on the nRF52840 and can be read back, and that makes the isolation
//! attempts PM33.5 asks for. Not a Flight_Build.
//!
//! Two Partitions. `timer` owns TIMER3 and GPIOTE and declares two channels
//! with forks: compare 0 sets the atrial pace pin (P1.10, analyzer D0) and,
//! by the fork, its copy (P1.03); compare 4 clears both. Its Task `pulse`
//! arms a 400 µs pulse every period, so the pulses on the analyzer show the
//! channels the Kernel wrote at work, and it attempts the PPI's window
//! itself. `outsider` owns the GPIO port alone, to show its Jobs on two pins
//! (P1.12 toggles when a Job begins, P1.13 when it ends: analyzer D2 and
//! D3); its Task `poke` attempts TIMER3, GPIOTE and the PPI. Every attempt has to end its Job with a fault (the fault
//! response is END_JOB), which the per-Task state records: `attempt` counts
//! the attempts begun and `survived` gets a bit for each that returned.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = nrf52840, profile = Ravenscar, operating_duration = 2.h(),
    safe_state = crate::halt, log_capacity = 32;

    partition timer (level = B, crate = ppi_probe_timer, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = END_JOB, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [TIMER3, GPIOTE];
        state pulse.count: u32 = 0;
        state pulse.attempt: u32 = 0;
        state pulse.survived: u32 = 0;
        task pulse (periodic, period = 10.ms(), deadline = 10.ms(), budget = 2.ms(), priority = 2);
    }

    partition outsider (level = C, crate = ppi_probe_outsider, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                        fault = END_JOB, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                        mit_threshold = 10) {
        // The GPIO port only, for the two pins that show its Jobs to the analyzer.
        peripherals = [P0];
        state poke.count: u32 = 0;
        state poke.attempt: u32 = 0;
        state poke.survived: u32 = 0;
        task poke (periodic, period = 10.ms(), offset = 5.ms(), deadline = 10.ms(), budget = 2.ms(), priority = 1);
    }

    // TIMER3 COMPARE[0] -> GPIOTE SET[0] (P1.10), fork SET[2] (P1.03).
    ppi pulse_start (event = TIMER3[0x140], task = GPIOTE[0x030], fork = GPIOTE[0x038]);
    // TIMER3 COMPARE[4] -> GPIOTE CLR[0], fork CLR[2].
    ppi pulse_end (event = TIMER3[0x150], task = GPIOTE[0x060], fork = GPIOTE[0x068]);
}
