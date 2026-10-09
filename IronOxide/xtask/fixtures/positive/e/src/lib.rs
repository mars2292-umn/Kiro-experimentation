//! Entry-like crate: the whole library belongs to the Kernel_Unsafe_Module,
//! so link-level attributes are allowed.
#![no_std]

#[no_mangle]
pub static ENTRY_MARKER: u32 = 0;
