//! P1 demo Partition 1 (`part1`, level C): the `log` Task (Priority 2),
//! which runs for a few milliseconds and is preempted by both Tasks of
//! Partition 0. Its Context type and Job wrapper come from the
//! App_Declaration through `rsk_partition_part1!` (task 10.5).
#![no_std]
#![forbid(unsafe_code)]

use rsk::ResourceCell;

p1_demo_decl::rsk_partition_part1!();

/// Demo counter (a plain static of the Partition, read by the safe state).
pub static COUNT_LOG: ResourceCell<u32> = ResourceCell::new(0);

/// Task `log`: busy for about 5 ms (bounded iterations, PR-16), then counts.
pub fn log(_cx: rsk_ctx::Log) {
    rsk::spin(250_000);
    COUNT_LOG.with_unprivileged(|c| *c += 1);
    rsk::debug(b"C");
}
