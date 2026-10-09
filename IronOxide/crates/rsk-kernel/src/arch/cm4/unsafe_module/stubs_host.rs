//! Host doubles of the assembly entry points, so that the `rsk` API and the
//! arch logic compile and test on the host (no Flight_Build compiles this).

use super::sim;

pub fn job_end_trampoline() {}

pub fn svc_lock(r: u8) -> bool {
    sim::write(0xFFFF_FFF0, r as u32);
    true
}

pub fn svc_unlock(r: u8) -> bool {
    sim::write(0xFFFF_FFF4, r as u32);
    true
}

pub fn svc_now() -> u64 {
    0
}

pub fn svc_panic() {}
