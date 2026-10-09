//! P1 demo Partition 0 (`part0`, level A): `control` (Priority 7) and
//! `sensor` (Priority 5) share the Resource `state` (Ceiling 7). The
//! Resource cell, the Context types and the Job wrappers come from the
//! App_Declaration through `rsk_partition_part0!` (task 10.5); Task bodies
//! are synchronous functions that take their Context and return to their
//! Release_Point (PR-09).
#![no_std]
#![forbid(unsafe_code)]

use rsk::ResourceCell;

p1_demo_decl::rsk_partition_part0!();

/// Demo counters (plain statics of the Partition, read by the safe state).
pub static COUNT_CONTROL: ResourceCell<u32> = ResourceCell::new(0);
pub static COUNT_SENSOR: ResourceCell<u32> = ResourceCell::new(0);
/// Times `control` saw `sensor` inside its Critical_Section (set by sensor
/// before locking, cleared after): a control Release during that window
/// was blocked by the Ceiling, not preempting.
pub static BLOCKED_SEEN: ResourceCell<u32> = ResourceCell::new(0);
pub static SENSOR_IN_CS: ResourceCell<u32> = ResourceCell::new(0);

/// Task `control`: reads the state under the lock and counts.
pub fn control(mut cx: rsk_ctx::Control) {
    let in_cs = SENSOR_IN_CS.peek_unprivileged();
    cx.state.lock(|s| {
        let _ = *s;
    });
    if in_cs != 0 {
        BLOCKED_SEEN.with_unprivileged(|b| *b += 1);
    }
    COUNT_CONTROL.with_unprivileged(|c| *c += 1);
    rsk::debug(b"A");
}

/// Task `sensor`: released 5 ms into each 20 ms period, holds the Resource
/// for 8 ms (so that the control Release at 10 ms into the period falls
/// inside the Critical_Section and is blocked by the Ceiling, R8.4) and
/// updates it.
pub fn sensor(mut cx: rsk_ctx::Sensor) {
    SENSOR_IN_CS.with_unprivileged(|v| *v = 1);
    cx.state.lock(|s| {
        // 8 ms by the Kernel time base (25 MHz), bounded in iterations too
        // (PR-16).
        let start = rsk::now();
        let mut i = 0u32;
        while rsk::now() - start < 8 * 25_000 && i < 10_000_000 {
            i += 1;
        }
        *s += 1;
    });
    SENSOR_IN_CS.with_unprivileged(|v| *v = 0);
    COUNT_SENSOR.with_unprivileged(|c| *c += 1);
    rsk::debug(b"B");
}
