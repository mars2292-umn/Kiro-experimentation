//@ scope: application
//@ PR-21: core items outside the Core_Subset, or outside the file's scope, fail.
use core::sync::atomic::AtomicBool;
use core::marker::PhantomData;
use core::arch::asm;

fn f(p: *mut u32) {
    core::ptr::write_volatile(p, 1);
    let _o: core::option::Option<u32> = None;
    let _ = core::cmp::min(1, 2);
}
