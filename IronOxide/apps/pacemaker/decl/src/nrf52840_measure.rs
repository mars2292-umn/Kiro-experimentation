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
    // A Health_Monitor log of 16 events (32 until 2026-11-25): the log is in
    // the Kernel's RAM, which the builds with the Kernel's trace need for
    // the Kernel's stack (finding B44); no run has logged more than five.
    safe_state = crate::safe_state, log_capacity = 16;

    partition therapy (level = A, crate = pacemaker_therapy, code = 64.KiB(), ram = 32.KiB(), stack = 12.KiB(),
                       fault = RESTART_PARTITION, overrun = RESTART_PARTITION, deadline_miss = RESTART_PARTITION,
                       mit_threshold = 10) {
        peripherals = [GPIOTE, P0, TIMER1, TIMER2, TIMER3];
        resource timing: crate::Timing = crate::Timing::new();
        // The record of the arming sweep (task 7.2; feature `sweep` of the
        // Partition crate), read through the debug probe in one access.
        state control.sweep: [u32; 56] = [0; 56];
        // The status `control` last sent to `link`, and the Jobs since.
        state control.status_sent: [u32; 4] = [0; 4];
        state control.status_age: u32 = 0;
        // The Jobs since `control` last sent a batch to `diag` (task 22.1).
        state control.events_age: u32 = 0;
        // Released by the end of a pulse: compares 4 and 5 of TIMER3 (PM27.5).
        task pace (sporadic, binds = TIMER3, mit = 20.ms(), mit_policy = defer, deadline = 2.ms(),
                   budget = 450.us(), priority = 7, resources = [timing]);
        // Released by the Chamber's edge counter reaching its compare.
        task sense_v (sporadic, binds = TIMER2, mit = 50.ms(), mit_policy = defer, deadline = 3.ms(),
                      budget = 450.us(), priority = 6, resources = [timing]);
        task sense_a (sporadic, binds = TIMER1, mit = 50.ms(), mit_policy = defer, deadline = 4.ms(),
                      budget = 450.us(), priority = 5, resources = [timing]);
        task control (periodic, period = 20.ms(), deadline = 20.ms(), budget = 5.ms(), priority = 4,
                      resources = [timing]);
    }

    // The DCM link (design PD-11, Table 2; task 15.3). SPIM1's window is
    // read-only to the Partition: its registers go through the Kernel.
    partition comms (level = C, crate = pacemaker_comms, code = 16.KiB(), ram = 8.KiB(), stack = 2.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [SPIM1];
        state link.started: u32 = 0;
        // Transfers started, and Jobs that could not start one.
        state link.transfers: u32 = 0;
        state link.stuck: u32 = 0;
        state link.proto: crate::Link = crate::Link::new();
        state link.tx: rsk::DmaBuf<32> = rsk::DmaBuf::new();
        state link.tx_held: rsk::DmaHeld = rsk::DmaHeld::none();
        state link.rx: rsk::DmaBuf<32> = rsk::DmaBuf::new();
        state link.rx_held: rsk::DmaHeld = rsk::DmaHeld::none();
        task link (periodic, period = 50.ms(), deadline = 50.ms(), budget = 2.ms(), priority = 2);
    }

    // The Endpoints between `comms` and `therapy` (design, "Endpoints";
    // task 16.1). `program`: one message for every frame of the DCM's that
    // `link` hears, a command's or the mark that one was heard; `control`
    // takes up to four in a Job, and `therapy`'s validation function
    // accepts exactly the messages of `pm_core::wire` (R42.4: from level C
    // to level A). `status`: what `control` tells `link`. The channel of
    // `status` toward `control` exists and is never read; its validation
    // function accepts nothing.
    endpoint program: [u32; 30] { dcm => comms.link ops = 2, pg => therapy.control validate = crate::validate_program ops = 4 } capacity = 4, payload = 120;
    endpoint status: [u32; 4] { pg => therapy.control validate = crate::validate_status, dcm => comms.link ops = 2 } capacity = 2, payload = 16;

    // History and diagnostics (design PD-07, Table 2; tasks 22.1 to 22.3):
    // no peripheral, and nothing of it reaches `therapy`.
    partition diag (level = D, crate = pacemaker_diag, code = 16.KiB(), ram = 16.KiB(), stack = 2.KiB(),
                    fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                    mit_threshold = 10) {
        // The histograms, the counts, the Marker buffer and the trend.
        state history.store: crate::History = crate::History::new();
        task history (periodic, period = 100.ms(), deadline = 100.ms(), budget = 2.ms(), priority = 1);
    }

    // The event records from `therapy` to `diag` (task 22.1): a batch of
    // up to eight records of three words after four words of counts
    // (`pm_core::record`). `control` sends at most one in a period;
    // `history` takes up to two in one of its own, which is five of
    // `control`'s: a batch the full Endpoint does not take stays in the
    // ring of `therapy` and goes with the next (16 records in 100 ms is
    // above any event rate the rig gives; a storm on both sense lines is
    // some 45 a second). The capacity is two because the Endpoints'
    // storage is the Kernel's RAM, and in the builds with the Kernel's
    // trace what is left of that RAM is the Kernel's stack: with a capacity
    // of four it was 20 bytes less than the stack's bound (finding B44).
    // The channel toward `control` exists and is never read; its
    // validation function accepts nothing.
    endpoint events: [u32; 28] { pg => therapy.control validate = crate::validate_events ops = 1, log => diag.history ops = 2 } capacity = 2, payload = 112;

    // What leaves `diag`, and what is asked of it (task 22.4). `records`:
    // one record of six words, a link frame's payload, which `comms`'s
    // validation function admits only as a record of `pm_core::record`
    // (R42.4: from level D to level C). `request`: a code and two values
    // from `link`; the channel toward `link` carries nothing. One message
    // a Job each way and a capacity of two: `history` sends one record in
    // 100 ms and `link` takes one in 50 ms.
    endpoint records: [u32; 6] { log => diag.history ops = 1, dcm => comms.link validate = crate::validate_records ops = 1 } capacity = 2, payload = 24;
    endpoint request: [u32; 3] { dcm => comms.link validate = crate::validate_nothing ops = 1, log => diag.history ops = 1 } capacity = 2, payload = 12;

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
