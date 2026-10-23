//@ scope: kernel-unsafe
//@ PR-21: the Kernel_Unsafe_Module may use the kernel and kernel-unsafe items; `core::fmt` is linked but unreachable.
use core::ptr::{read_volatile, write_volatile};
use core::fmt::Write;

fn f(p: *mut u32) {
    unsafe {
        write_volatile(p, read_volatile(p));
        core::arch::asm!("nop");
    }
    let _m = core::mem::size_of::<u32>();
}
