//! The HIL_Rig stimulus Application's system crate: the tables of
//! Generated_Config come from the App_Declaration (`hil_stimulus_decl`,
//! `rsk::app!`) through the `rsk_system!` manifest; this crate adds only
//! the safe state. The stimulus board has no console: the safe state idles
//! with the watchdog fed (no semihosting, which would halt a board with no
//! debugger attached) until the rig resets the board.
#![no_std]
#![forbid(unsafe_code)]

use rsk_kernel::arch::cm4::unsafe_module::hw;

hil_stimulus_decl::rsk_system!();

/// The stimulus Application expects no Health_Monitor event (the Kernel's
/// event mirror, feature `trace-ram`).
pub const TRACE_EXPECT: (u32, u8) = (0, 0xFF);

/// The system safe state (R14.4): quiet, watchdog fed, forever.
pub fn halt() -> ! {
    loop {
        hw::safe_state_idle()
    }
}
