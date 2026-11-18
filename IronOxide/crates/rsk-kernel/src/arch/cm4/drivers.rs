//! The register drivers of Kernel_Arch as functions of a register file
//! (2026-11-18): the bodies behind the `Action`s whose step changes a
//! state the hardware keeps (the NVIC's enabled and pending lines and
//! priority bytes, the nRF52840's TIMER0 compare, the boot configuration
//! bits). Each takes the register file it reads and writes (`Regs`), so
//! that the same code runs on the Target, where the file is the
//! memory-mapped registers (`unsafe_module::hw::Mmio`, whose three
//! methods are the volatile accesses of UJ-001, UJ-002 and UJ-009), and
//! is read by Aeneas as a function of a register model
//! (`proofs/aeneas/lean/Hw`), where each is proven to take the
//! Hardware_Model's step. The functions of `unsafe_module::hw` that the
//! Kernel calls are these applied to `Mmio`.
//!
//! No `unsafe`, no assembly: the barriers and the system-register
//! instructions stay in `unsafe_module::hw`.
#![forbid(unsafe_code)]

use super::unsafe_module::hw::addr;

/// A register file: word reads and writes, and the byte write of the
/// NVIC's priority array.
pub trait Regs {
    fn read(&mut self, a: u32) -> u32;
    fn write(&mut self, a: u32, v: u32);
    fn write_u8(&mut self, a: u32, v: u8);
}

/// The extension of the 32-bit counter to the 64-bit time base (DD-05):
/// the Kernel_Arch state that counts the counter's wraps.
pub trait Extend {
    fn extend(&mut self, low: u32) -> u64;
}

/// The NVIC line of an exception number; `None` for the system exceptions
/// (below 16) and for numbers past the implemented lines.
#[inline]
pub fn irq_of(n: u16) -> Option<u32> {
    if n < 16 || n as u32 - 16 >= addr::NVIC_LINES {
        None
    } else {
        Some(n as u32 - 16)
    }
}

#[inline]
pub fn nvic_enable<R: Regs>(r: &mut R, n: u16) {
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ISER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

#[inline]
pub fn nvic_disable<R: Regs>(r: &mut R, n: u16) {
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ICER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

#[inline]
pub fn nvic_set_pending<R: Regs>(r: &mut R, n: u16) {
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ISPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

#[inline]
pub fn nvic_clear_pending<R: Regs>(r: &mut R, n: u16) {
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ICPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

#[inline]
pub fn nvic_set_priority<R: Regs>(r: &mut R, n: u16, encoded: u8) {
    if let Some(irq) = irq_of(n) {
        r.write_u8(addr::NVIC_IPR + irq, encoded);
    }
}

/// AIRCR.PRIGROUP = 4: with 3 implemented bits (7:5) every bit is a group
/// bit (HW-01). VECTKEY 0x05FA is required for the write.
#[inline]
pub fn set_prigroup_all_preempt<R: Regs>(r: &mut R) {
    let v = r.read(addr::AIRCR) & 0x0000_F8FF;
    r.write(addr::AIRCR, (0x05FA << 16) | v | (4 << 8));
}

/// CCR.NONBASETHRDENA (HW-07). The Kernel keeps it 0: no Job executes
/// with an exception active (DD-01 as revised by the P1 spike).
#[inline]
pub fn set_nonbasethrdena<R: Regs>(r: &mut R, on: bool) {
    let v = r.read(addr::CCR);
    r.write(addr::CCR, if on { v | 1 } else { v & !1 });
}

/// ACTLR.DISDEFWBUF (HW-11, DD-12).
#[inline]
pub fn set_disdefwbuf<R: Regs>(r: &mut R, on: bool) {
    let v = r.read(addr::ACTLR);
    r.write(addr::ACTLR, if on { v | 2 } else { v & !2 });
}

/// MemManage, BusFault, UsageFault, and SVCall at the Kernel level (priority
/// 0) and enabled (SHCSR bits 16 to 18).
#[inline]
pub fn set_kernel_exception_priorities<R: Regs>(r: &mut R) {
    r.write(addr::SHPR1, 0);
    r.write(addr::SHPR2, 0);
    r.write(addr::SHPR3, 0);
    let v = r.read(addr::SHCSR);
    r.write(addr::SHCSR, v | (1 << 16) | (1 << 17) | (1 << 18));
}

/// The register writes of the eager FP configuration (HW-10, DD-04):
/// FPCCR.ASPEN = 1, LSPEN = 0; FPDSCR, the FPSCR every new FP context
/// starts from, with FZ (bit 24) when the declaration asks for it
/// (erratum 839676's workaround, task 3.4); CP10 and CP11 enabled for
/// every privilege level. The barrier after them is the caller's.
#[inline]
pub fn fpu_configure_eager<R: Regs>(r: &mut R, flush_to_zero: bool) {
    let v = r.read(addr::FPCCR);
    r.write(addr::FPCCR, (v | (1 << 31)) & !(1 << 30));
    r.write(addr::FPDSCR, if flush_to_zero { 1 << 24 } else { 0 });
    let c = r.read(addr::CPACR) | (0xF << 20);
    r.write(addr::CPACR, c);
}

/// MPU_CTRL: ENABLE and PRIVDEFENA (HW-09). The barrier is the caller's.
#[inline]
pub fn mpu_enable_background<R: Regs>(r: &mut R) {
    r.write(addr::MPU_CTRL, (1 << 2) | 1);
}

/// DEMCR.TRCENA, the cycle counter cleared and enabled (HW-13).
#[inline]
pub fn dwt_enable<R: Regs>(r: &mut R) {
    let v = r.read(addr::DEMCR);
    r.write(addr::DEMCR, v | (1 << 24));
    r.write(addr::DWT_CYCCNT, 0);
    let c = r.read(addr::DWT_CTRL);
    r.write(addr::DWT_CTRL, c | 1);
}

/// nRF52840 TIMER0 (PS 6.30): 32-bit mode at 16 MHz (prescaler 0), CC[0]
/// for the compare event, CAPTURE[1]/CC[1] to read the counter.
pub mod timer0 {
    use super::{addr, Extend, Regs};
    pub const TASKS_CAPTURE1: u32 = 0x044;
    pub const EVENTS_COMPARE0: u32 = 0x140;
    pub const INTENSET: u32 = 0x304;
    pub const INTENCLR: u32 = 0x308;
    pub const CC0: u32 = 0x540;
    pub const CC1: u32 = 0x544;
    /// TIMER0's interrupt line (nRF52840 IRQ 8), for a compare already due.
    pub const TIMER0_VECTOR: u16 = 16 + 8;

    /// The 64-bit time base: the counter captured and extended.
    #[inline]
    pub fn now<R: Regs, E: Extend>(r: &mut R, e: &mut E) -> u64 {
        let t = addr::NRF_TIMER0;
        r.write(t + TASKS_CAPTURE1, 1);
        let low = r.read(t + CC1);
        e.extend(low)
    }

    /// Arms the compare for `at` and returns the time base captured after
    /// the write. The compare is absolute: an instant the counter has
    /// already passed matches only after the 32-bit wrap (268 s at 16 MHz),
    /// so a due or past instant pends the time-base interrupt instead; the
    /// handler serves every due slot by `now`, not by the event. Seen on
    /// the DK on 2026-10-29: the first release (instant 0) was armed after
    /// the counter started and no tick ever came, so the watchdog reset the
    /// chip. One capture after the write decides (HW-12, ASM-19).
    #[inline]
    pub fn arm<R: Regs, E: Extend>(r: &mut R, e: &mut E, at: u64) -> u64 {
        let t = addr::NRF_TIMER0;
        r.write(t + CC0, at as u32);
        r.write(t + INTENSET, 1 << 16);
        let post = now(r, e);
        if post >= at {
            super::nvic_set_pending(r, TIMER0_VECTOR);
        }
        post
    }

    #[inline]
    pub fn ack<R: Regs>(r: &mut R) {
        let t = addr::NRF_TIMER0;
        r.write(t + EVENTS_COMPARE0, 0);
    }

    /// Cancels the compare that is armed or has matched (2026-11-15: the
    /// Budget event it was armed for is disarmed, Verification_Plan 7.3):
    /// the compare interrupt is disabled, its event cleared, and the
    /// pending state of TIMER0's line cleared in the NVIC. The event is
    /// read back before the NVIC write so that the line is low by then
    /// (the event register drives it).
    #[inline]
    pub fn cancel<R: Regs>(r: &mut R) {
        let t = addr::NRF_TIMER0;
        r.write(t + INTENCLR, 1 << 16);
        r.write(t + EVENTS_COMPARE0, 0);
        let _ = r.read(t + EVENTS_COMPARE0);
        super::nvic_clear_pending(r, TIMER0_VECTOR);
    }

    /// Cancels the compare that is armed or has matched and arms it for
    /// `at`, returning the time base captured after the writes. The new
    /// compare value is written before the old event is cleared, so no
    /// match of the old value survives; a match of the new one that the
    /// clearing erased is caught by the last test, which pends the time
    /// base at once for an instant already reached (as `arm` does): no due
    /// event is lost.
    #[inline]
    pub fn rearm<R: Regs, E: Extend>(r: &mut R, e: &mut E, at: u64) -> u64 {
        let t = addr::NRF_TIMER0;
        r.write(t + INTENCLR, 1 << 16);
        r.write(t + CC0, at as u32);
        r.write(t + EVENTS_COMPARE0, 0);
        let _ = r.read(t + EVENTS_COMPARE0);
        super::nvic_clear_pending(r, TIMER0_VECTOR);
        r.write(t + INTENSET, 1 << 16);
        let post = now(r, e);
        if post >= at {
            super::nvic_set_pending(r, TIMER0_VECTOR);
        }
        post
    }
}
