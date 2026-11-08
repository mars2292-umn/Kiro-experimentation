//! R41.3 (task 21.6, A-P8): a role cannot declare `after` naming its own
//! Endpoint; the Generator refuses the declaration.
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 100.ms(),
    safe_state = crate::safe_state, log_capacity = 16;

    partition part0 (level = A, crate = api_gating_self, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = RECORD_AND_CONTINUE, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [];
        task alpha (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 7);
        task beta (periodic, period = 20.ms(), deadline = 20.ms(), budget = 1.ms(), priority = 5);
    }

    protocol Ping {
        rec L {
            choice at a {
                Req(u32): a -> b;
                Rep(u32): b -> a;
                continue L;
            } or {
                Stop(): a -> b;
                end;
            }
        }
    }
    endpoint ping: protocol Ping { a => part0.alpha after = ping, b => part0.beta } capacity = 1, payload = 8;
}

pub fn safe_state() {}
