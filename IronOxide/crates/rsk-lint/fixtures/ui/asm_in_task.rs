//@ scope: application
//@ PR-22: inline assembly in a Task body.
pub fn job() {
    unsafe {
        core::arch::asm!("nop");
    }
}
