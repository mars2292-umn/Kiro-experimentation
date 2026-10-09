//! The P1 demo binary has no symbols of its own: the vector table, the
//! reset entry, and the panic handler come from `rsk-entry` (DD-11).
#![no_std]
#![no_main]
#![forbid(unsafe_code)]

use rsk_entry as _;
