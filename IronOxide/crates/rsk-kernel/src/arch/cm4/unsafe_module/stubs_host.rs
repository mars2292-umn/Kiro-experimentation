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

pub fn svc_raise(signal: u8) -> bool {
    sim::write(0xFFFF_FFE8, signal as u32);
    true
}

pub fn svc_dma(i: u8, ptr: u32, len: u32) -> bool {
    sim::write(0xFFFF_FFF8, i as u32);
    sim::write(0xFFFF_FFFC, ptr.wrapping_add(len));
    true
}

pub fn svc_dma_write(i: u8, offset: u32, value: u32) -> bool {
    sim::write(0xFFFF_FFF8, 0x8000_0000 | i as u32);
    sim::write(0xFFFF_FFFC, offset.wrapping_add(value));
    true
}

/// The host doubles of the Endpoint services (task 21.2): a send is always
/// taken, a receive always finds the channel empty.
pub fn svc_send(arg: u32, ptr: u32, len: u32) -> u32 {
    sim::write(0xFFFF_FFF0, arg);
    sim::write(0xFFFF_FFF4, ptr.wrapping_add(len));
    0
}

pub fn svc_recv(arg: u32, _ptr: u32, _len: u32) -> u32 {
    sim::write(0xFFFF_FFF0, arg);
    1
}

pub fn svc_violation(arg: u32) {
    sim::write(0xFFFF_FFF0, 10 | (arg << 8));
}

pub fn svc_panic() {}
