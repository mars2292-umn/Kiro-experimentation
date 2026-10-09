//! rsk-entry: the vector table, the reset entry, the exception handlers,
//! and the panic handler (DD-11; task 5.1). The whole crate belongs to the
//! Kernel_Unsafe_Module (R6.3, PR-20) because binding vectors and the
//! reset path need link-level attributes and raw memory initialization.
//!
//! The crate binds the Kernel_Arch entry points to the system crate's
//! `System` type (`rsk_system::Sys`, a path dependency that the
//! Build_System points at the generated system crate of the Application).
//! The Application never defines a link-level symbol (R24.2).
//!
//! The linker script provides the memory initialization tables
//! (`__rsk_bss_table`, `__rsk_data_table`), which the reset path walks.
#![no_std]
#![allow(unsafe_code)]

use rsk_kernel::arch::cm4 as arch;
use rsk_kernel::arch::cm4::unsafe_module::{hw, stubs};
use rsk_kernel::arch::System;
use rsk_system::Sys;

/// A vector-table entry: a handler or a reserved word.
#[repr(C)]
#[derive(Clone, Copy)]
pub union Vector {
    handler: unsafe extern "C" fn(),
    reserved: u32,
}

/// The initial stack pointer is the first word of the table; the linker
/// script emits it. The reset vector follows.
#[link_section = ".vector_table.reset_vector"]
#[no_mangle]
pub static __RESET_VECTOR: unsafe extern "C" fn() -> ! = Reset;

/// The 14 system exceptions after Reset (Armv7-M ARM B1.5.2).
#[link_section = ".vector_table.exceptions"]
#[no_mangle]
pub static __EXCEPTIONS: [Vector; 14] = [
    Vector { handler: rsk_nmi_entry },          // 2 NMI
    Vector { handler: rsk_hardfault_entry },    // 3 HardFault
    Vector { handler: stubs::rsk_memmanage_entry }, // 4 MemManage
    Vector { handler: stubs::rsk_busfault_entry },  // 5 BusFault
    Vector { handler: stubs::rsk_usagefault_entry }, // 6 UsageFault
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { reserved: 0 },
    Vector { handler: stubs::rsk_svc_entry },   // 11 SVCall
    Vector { handler: rsk_default_entry },      // 12 DebugMonitor
    Vector { reserved: 0 },
    Vector { handler: rsk_default_entry },      // 14 PendSV (unused)
    Vector { handler: rsk_default_entry },      // 15 SysTick (unused)
];

const fn interrupts() -> [Vector; Sys::IRQ_COUNT as usize] {
    [Vector { handler: stubs::rsk_irq_entry }; Sys::IRQ_COUNT as usize]
}

/// Every interrupt line goes through the shared stub, which reads IPSR and
/// consults the system crate's bindings (R19.5: unbound lines are
/// disabled and reported).
#[link_section = ".vector_table.interrupts"]
#[no_mangle]
pub static __INTERRUPTS: [Vector; Sys::IRQ_COUNT as usize] = interrupts();

/// An entry of the linker-provided initialization tables.
#[repr(C)]
struct Range {
    start: u32,
    end: u32,
    load: u32,
}

extern "C" {
    /// `(start, end, 0)` triples of regions to zero, terminated by zeros.
    static __rsk_bss_table: Range;
    /// `(start, end, load)` triples of regions to copy from flash, terminated by zeros.
    static __rsk_data_table: Range;
}

/// The reset entry (R12.4): memory image, boot checks, Init, instant 0,
/// then the idle activity (R7.6, privileged Thread mode on the MSP, WFI).
// SAFETY (UJ-101): the tables come from the linker script of this build;
// each range lies in RAM that nothing else uses before Init, and each load
// range lies in flash.
#[no_mangle]
pub unsafe extern "C" fn Reset() -> ! {
    let mut entry = &__rsk_bss_table as *const Range;
    while (*entry).start != 0 || (*entry).end != 0 {
        let mut a = (*entry).start as usize as *mut u32;
        let end = (*entry).end as usize as *mut u32;
        while a < end {
            core::ptr::write_volatile(a, 0);
            a = a.add(1);
        }
        entry = entry.add(1);
    }
    let mut entry = &__rsk_data_table as *const Range;
    while (*entry).start != 0 || (*entry).end != 0 {
        let mut a = (*entry).start as usize as *mut u32;
        let end = (*entry).end as usize as *mut u32;
        let mut src = (*entry).load as usize as *const u32;
        while a < end {
            core::ptr::write_volatile(a, core::ptr::read_volatile(src));
            a = a.add(1);
            src = src.add(1);
        }
        entry = entry.add(1);
    }
    if !arch::boot::<Sys>() {
        Sys::safe_state();
    }
    arch::start::<Sys>();
    loop {
        hw::wait_for_interrupt();
    }
}

// SAFETY (UJ-102): the stub stores a pointer to four writable words on the
// MSP in `out` and reads them back after the call.
#[no_mangle]
pub unsafe extern "C" fn rsk_irq_entry_rust(ipsr: u32, psp: u32, control: u32, exc_return: u32, out: *mut [u32; 4]) {
    let plan = arch::irq_entry::<Sys>(ipsr, psp, control, exc_return);
    *out = [plan.psp, plan.control, plan.exc_return, plan.hop];
}

// SAFETY (UJ-103): `frame` is the exception frame the SVC handler located
// (HW-06), eight words on the caller's stack.
#[no_mangle]
pub unsafe extern "C" fn rsk_svc_rust(frame: *mut [u32; 8]) -> u32 {
    arch::svc::<Sys>(&mut *frame)
}

// SAFETY (UJ-104): as UJ-102; `level` is the exception number of the
// level `rsk_unwind` is unwinding.
#[no_mangle]
pub unsafe extern "C" fn rsk_epilogue_rust(level: u32, out: *mut [u32; 4]) {
    let (psp, control, exc_return, next) = arch::epilogue::<Sys>(level);
    *out = [psp, control, exc_return, next];
}

#[no_mangle]
pub extern "C" fn rsk_timer_rust() -> u32 {
    arch::timer::<Sys>()
}

#[no_mangle]
pub extern "C" fn rsk_memmanage_rust(in_kernel: u32) -> u32 {
    arch::mem_manage::<Sys>(in_kernel != 0)
}

#[no_mangle]
pub extern "C" fn rsk_busfault_rust(in_kernel: u32) -> u32 {
    arch::bus_fault::<Sys>(in_kernel != 0)
}

#[no_mangle]
pub extern "C" fn rsk_usagefault_rust(in_kernel: u32) -> u32 {
    arch::usage_fault::<Sys>(in_kernel != 0)
}

/// HardFault: always a Kernel fault into the safe state (R14.3, R14.4).
// SAFETY (UJ-105): a plain handler that never returns.
#[no_mangle]
pub unsafe extern "C" fn rsk_hardfault_entry() {
    arch::hard_fault::<Sys>()
}

// SAFETY (UJ-106): as UJ-105.
#[no_mangle]
pub unsafe extern "C" fn rsk_nmi_entry() {
    arch::hard_fault::<Sys>()
}

/// Unused system exceptions (DebugMonitor, PendSV, SysTick): a Kernel fault.
// SAFETY (UJ-107): as UJ-105.
#[no_mangle]
pub unsafe extern "C" fn rsk_default_entry() {
    arch::hard_fault::<Sys>()
}

/// The single panic handler of a Flight_Build (PR-19, R14.1). It lives in
/// the shared code region because unprivileged Jobs execute it.
#[panic_handler]
#[link_section = ".text.rsk_shared.panic"]
fn panic(_info: &core::panic::PanicInfo<'_>) -> ! {
    // In Handler mode (IPSR != 0) the panic is the Kernel's; in Thread mode
    // it is the executing Job's.
    let ipsr: u32 = ipsr();
    arch::panic::<Sys>(ipsr != 0)
}

fn ipsr() -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let v: u32;
        // SAFETY (UJ-108): MRS IPSR has no memory effect.
        unsafe { core::arch::asm!("mrs {0}, ipsr", out(reg) v, options(nomem, nostack, preserves_flags)) };
        v
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        0
    }
}
