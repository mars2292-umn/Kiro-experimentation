//! An App_Declaration that is valid in every respect the Generator checks,
//! with one per-Task state whose initial value is not zero (R14.5): the
//! Partition crate `static_nonzero` does not compile.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 100.ms(),
    safe_state = crate::safe_state, log_capacity = 16;

    partition part0 (level = A, crate = api_static_nonzero, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [];
        state alpha.level: u32 = 5;
        task alpha (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 7);
    }
}

pub fn safe_state() -> ! {
    loop {}
}
