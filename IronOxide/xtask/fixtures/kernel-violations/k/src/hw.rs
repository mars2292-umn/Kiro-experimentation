//! A Kernel module without its own `#![forbid(unsafe_code)]`, holding a
//! naked function, which the `unsafe_code` lint does not reject.

#[cfg(target_arch = "arm")]
#[unsafe(naked)]
pub extern "C" fn reset() {
    core::arch::naked_asm!("bx lr")
}
