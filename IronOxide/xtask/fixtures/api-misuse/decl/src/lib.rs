//! A valid App_Declaration for the API fixtures: two Resources, a signal,
//! two periodic Tasks (`alpha` accesses both Resources and raises `tick`;
//! `beta` accesses `state` only) and the sporadic `gamma` bound to `tick`.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 100.ms(),
    safe_state = crate::safe_state, log_capacity = 16;

    partition part0 (level = A, crate = api_ok, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [];
        resource state: u32 = 0;
        resource other: u32 = 0;
        signal tick;
        task alpha (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 7,
                    resources = [state, other], raises = [tick]);
        task beta (periodic, period = 20.ms(), deadline = 20.ms(), budget = 1.ms(), priority = 5,
                   resources = [state]);
        task gamma (sporadic, signal = tick, mit = 5.ms(), mit_policy = defer, deadline = 5.ms(),
                    budget = 1.ms(), priority = 3);
    }
}
