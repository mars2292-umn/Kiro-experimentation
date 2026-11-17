//! R12.3 at build time: the same tables with a Task Priority of 0, which no
//! Task may have (PR-27). `Config::validate` is false for them, and the
//! assertion the Generator emits stops the build.
#![no_std]
#![forbid(unsafe_code)]
use rsk_kernel::logic::{Config, PartitionCfg, TaskCfg, KIND_PERIODIC, MIT_DEFER, RESP_RECORD_AND_CONTINUE, RESP_RECORD_ONLY, RESP_RESTART_PARTITION};

/// One periodic Task in one Partition, with the three timed-event slots
/// the Generator would give it and the two the Kernel adds.
pub const CONFIG: Config<1, 0, 1> = Config {
    tasks: [TaskCfg { priority: 0, partition: 0, kind: KIND_PERIODIC, offset: 0, period: 1000, deadline: 1000, budget: 100, mit_policy: MIT_DEFER, fpu: false }],
    resources: [],
    access: [[]],
    partitions: [PartitionCfg { level: 0, fault_response: RESP_RESTART_PARTITION, overrun_response: RESP_RECORD_AND_CONTINUE, deadline_response: RESP_RECORD_ONLY, mit_threshold: 10 }],
    operating_duration: 100_000,
    slot_release: [0],
    slot_deadline: [1],
    slot_budget: [2],
    slot_count: 5,
};

/// The line the Generator emits after the tables.
const _: () = assert!(CONFIG.validate(), "R12.3: the generated configuration is not well-formed (Config::validate)");
