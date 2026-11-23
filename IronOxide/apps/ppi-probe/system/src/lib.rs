//! The PPI probe's system crate: the tables of Generated_Config come
//! from the App_Declaration (`ppi_probe_decl`, `rsk::app!`) through the
//! `rsk_system!` manifest; this crate adds only the safe state. The board
//! has no console: the safe state idles with the watchdog fed until the
//! board is reset.
#![no_std]
#![forbid(unsafe_code)]

use rsk_kernel::arch::cm4::unsafe_module::hw;

ppi_probe_decl::rsk_system!();

/// The trace freeze's expectation (the Kernel's event mirror, feature
/// `trace-ram`): none stated; the probe's faults are judged from the
/// Partitions' own state.
pub const TRACE_EXPECT: (u32, u8, u32) = (0, 0xFF, 0);

/// The system safe state (R14.4): quiet, watchdog fed, forever.
pub fn halt() -> ! {
    loop {
        hw::safe_state_idle()
    }
}
