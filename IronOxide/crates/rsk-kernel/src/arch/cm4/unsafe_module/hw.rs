//! Register access with trusted specifications (R44.5, R45.4). Each
//! function is one Hardware_Model step; on the Target it is a volatile
//! access, elsewhere the simulated register file of `sim` records it.
//!
//! Address constants: Armv7-M ARM B3.2 (system control block), B3.4
//! (NVIC), B3.5 (MPU), C1.8 (DWT); Cortex-M4 TRM 4.3 (ACTLR); nRF52840 PS
//! 6.30 (TIMER); CMSDK APB timer for the QEMU mps2-an386 board.

use crate::logic::mpu::{View, MPU_REGIONS};

/// Memory-mapped register addresses used by the Kernel. The Kani harness
/// `uj_001_mmio_addresses_valid` checks that every one lies in the system
/// control space or the peripheral region and is word aligned.
pub mod addr {
    pub const ACTLR: u32 = 0xE000_E008;
    pub const NVIC_ISER: u32 = 0xE000_E100;
    pub const NVIC_ICER: u32 = 0xE000_E180;
    pub const NVIC_ISPR: u32 = 0xE000_E200;
    pub const NVIC_ICPR: u32 = 0xE000_E280;
    pub const NVIC_IPR: u32 = 0xE000_E400;
    pub const CPUID: u32 = 0xE000_ED00;
    pub const ICSR: u32 = 0xE000_ED04;
    pub const AIRCR: u32 = 0xE000_ED0C;
    pub const CCR: u32 = 0xE000_ED14;
    pub const SHPR1: u32 = 0xE000_ED18;
    pub const SHPR2: u32 = 0xE000_ED1C;
    pub const SHPR3: u32 = 0xE000_ED20;
    pub const SHCSR: u32 = 0xE000_ED24;
    pub const CFSR: u32 = 0xE000_ED28;
    pub const CPACR: u32 = 0xE000_ED88;
    pub const MPU_TYPE: u32 = 0xE000_ED90;
    pub const MPU_CTRL: u32 = 0xE000_ED94;
    pub const MPU_RNR: u32 = 0xE000_ED98;
    pub const MPU_RBAR: u32 = 0xE000_ED9C;
    pub const MPU_RASR: u32 = 0xE000_EDA0;
    pub const DEMCR: u32 = 0xE000_EDFC;
    pub const FPCCR: u32 = 0xE000_EF34;
    pub const FPDSCR: u32 = 0xE000_EF3C;
    pub const DWT_CTRL: u32 = 0xE000_1000;
    pub const DWT_CYCCNT: u32 = 0xE000_1004;
    /// nRF52840 TIMER0 (PS 6.30).
    pub const NRF_TIMER0: u32 = 0x4000_8000;
    /// QEMU mps2-an386 CMSDK APB timers.
    pub const CMSDK_TIMER0: u32 = 0x4000_0000;
    pub const CMSDK_TIMER1: u32 = 0x4000_1000;

    pub const ALL: &[u32] = &[
        ACTLR, NVIC_ISER, NVIC_ICER, NVIC_ISPR, NVIC_ICPR, NVIC_IPR, CPUID, ICSR, AIRCR, CCR, SHPR1, SHPR2, SHPR3,
        SHCSR, CFSR, CPACR, MPU_TYPE, MPU_CTRL, MPU_RNR, MPU_RBAR, MPU_RASR, DEMCR, FPCCR, FPDSCR, DWT_CTRL,
        DWT_CYCCNT, NRF_TIMER0, CMSDK_TIMER0, CMSDK_TIMER1,
    ];

    /// Whether `a` is a word-aligned address in the system control space
    /// (0xE000_0000..0xE010_0000) or the peripheral region (0x4000_0000..0x6000_0000).
    pub const fn plausible(a: u32) -> bool {
        a % 4 == 0 && ((a >= 0xE000_0000 && a < 0xE010_0000) || (a >= 0x4000_0000 && a < 0x6000_0000))
    }
}

/// Reads a memory-mapped register.
pub fn mmio_read(a: u32) -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-001): `a` is one of the register addresses of `addr`
        // (or one derived from them by the NVIC and MPU index arithmetic of
        // this module), all of which the harness `uj_001_mmio_addresses_valid`
        // shows word aligned and inside the system control space or the
        // peripheral region, where reads have the semantics the
        // Hardware_Model gives them (HW-01 to HW-13).
        unsafe { core::ptr::read_volatile(a as usize as *const u32) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::read(a)
    }
}

/// Writes a memory-mapped register.
pub fn mmio_write(a: u32, v: u32) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-002): as UJ-001, for writes.
        unsafe { core::ptr::write_volatile(a as usize as *mut u32, v) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::write(a, v)
    }
}

fn mmio_write_u8(a: u32, v: u8) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-009): as UJ-001; byte access to the NVIC_IPR array,
        // which the Armv7-M ARM defines as byte accessible (B3.4.6).
        unsafe { core::ptr::write_volatile(a as usize as *mut u8, v) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::write_u8(a, v)
    }
}

// ------------------------------------------------------------- core regs

/// MSR BASEPRI (HW-03): privileged only; the Kernel calls it inside SVC.
pub fn write_basepri(v: u8) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-010): executed in Handler mode (privileged); the
        // instruction has no memory effect. `v` is an encoding of HW-01.
        unsafe { core::arch::asm!("msr BASEPRI, {0}", in(reg) v as u32, options(nomem, nostack, preserves_flags)) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::set_basepri(v)
    }
}

pub fn read_basepri() -> u8 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let v: u32;
        // SAFETY (UJ-011): as UJ-010, a read.
        unsafe { core::arch::asm!("mrs {0}, BASEPRI", out(reg) v, options(nomem, nostack, preserves_flags)) }
        v as u8
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::basepri()
    }
}

/// CPSID I (HW-04).
pub fn primask_set() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-012): privileged; no memory effect.
        unsafe { core::arch::asm!("cpsid i", options(nomem, nostack, preserves_flags)) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::set_primask(true)
    }
}

/// CPSIE I.
pub fn primask_clear() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-013): as UJ-012.
        unsafe { core::arch::asm!("cpsie i", options(nomem, nostack, preserves_flags)) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::set_primask(false)
    }
}

/// DSB; ISB: the erratum 838869 workaround before exception returns and
/// the synchronization after MPU and CONTROL writes.
pub fn barrier() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-014): barriers have no memory effect of their own.
        unsafe { core::arch::asm!("dsb", "isb", options(nostack, preserves_flags)) }
    }
}

/// WFI for the idle activity (R7.6).
pub fn wait_for_interrupt() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-015): no memory effect.
        unsafe { core::arch::asm!("wfi", options(nomem, nostack, preserves_flags)) }
    }
}

// ----------------------------------------------------------------- NVIC

pub fn nvic_enable(n: u16) {
    let irq = n.saturating_sub(16) as u32;
    mmio_write(addr::NVIC_ISER + 4 * (irq / 32), 1 << (irq % 32));
}

pub fn nvic_disable(n: u16) {
    let irq = n.saturating_sub(16) as u32;
    mmio_write(addr::NVIC_ICER + 4 * (irq / 32), 1 << (irq % 32));
}

pub fn nvic_set_pending(n: u16) {
    let irq = n.saturating_sub(16) as u32;
    mmio_write(addr::NVIC_ISPR + 4 * (irq / 32), 1 << (irq % 32));
}

pub fn nvic_clear_pending(n: u16) {
    let irq = n.saturating_sub(16) as u32;
    mmio_write(addr::NVIC_ICPR + 4 * (irq / 32), 1 << (irq % 32));
}

pub fn nvic_set_priority(n: u16, encoded: u8) {
    let irq = n.saturating_sub(16) as u32;
    mmio_write_u8(addr::NVIC_IPR + irq, encoded);
}

/// AIRCR.PRIGROUP = 4: with 3 implemented bits (7:5) every bit is a group
/// bit (HW-01). VECTKEY 0x05FA is required for the write.
pub fn set_prigroup_all_preempt() {
    let v = mmio_read(addr::AIRCR) & 0x0000_F8FF;
    mmio_write(addr::AIRCR, (0x05FA << 16) | v | (4 << 8));
}

/// CCR.NONBASETHRDENA (HW-07). The Kernel keeps it 0: no Job executes
/// with an exception active (DD-01 as revised by the P1 spike).
pub fn set_nonbasethrdena(on: bool) {
    let v = mmio_read(addr::CCR);
    mmio_write(addr::CCR, if on { v | 1 } else { v & !1 });
}

/// ACTLR.DISDEFWBUF (HW-11, DD-12).
pub fn set_disdefwbuf(on: bool) {
    let v = mmio_read(addr::ACTLR);
    mmio_write(addr::ACTLR, if on { v | 2 } else { v & !2 });
}

/// MemManage, BusFault, UsageFault, and SVCall at the Kernel level (priority
/// 0) and enabled (SHCSR bits 16 to 18).
pub fn set_kernel_exception_priorities() {
    mmio_write(addr::SHPR1, 0);
    mmio_write(addr::SHPR2, 0);
    mmio_write(addr::SHPR3, 0);
    let v = mmio_read(addr::SHCSR);
    mmio_write(addr::SHCSR, v | (1 << 16) | (1 << 17) | (1 << 18));
}

/// Clears the configurable fault status (write one to clear).
pub fn clear_fault_status() {
    let v = mmio_read(addr::CFSR);
    mmio_write(addr::CFSR, v);
}

// ------------------------------------------------------------------ FPU

/// Eager stacking (HW-10, DD-04): FPCCR.ASPEN = 1, LSPEN = 0; FPDSCR
/// default (round to nearest, no flush-to-zero, no default NaN); CP10 and
/// CP11 enabled for every privilege level.
///
/// ORQ-16 finding (QEMU dispatch spike, 2026-10-08): on
/// `thumbv7em-none-eabihf` LLVM emits VFP loads and stores (`vldr`, `vstr`,
/// `vmov`) for 64-bit data moves in Kernel code; with the coprocessor
/// disabled the first timer tick faulted with UsageFault NOCP. The
/// coprocessor therefore stays enabled while the Kernel runs, and PR-31 is
/// enforced for FPU-free Tasks by the Link_Checker's instruction scan (its
/// Profile enforcement is MC and LK, not RT). The Verification_Plan records
/// the data-movement instructions in Kernel code against R6.6.
pub fn fpu_configure_eager() {
    let v = mmio_read(addr::FPCCR);
    mmio_write(addr::FPCCR, (v | (1 << 31)) & !(1 << 30));
    mmio_write(addr::FPDSCR, 0);
    let c = mmio_read(addr::CPACR) | (0xF << 20);
    mmio_write(addr::CPACR, c);
    barrier();
}

/// Per-Job coprocessor access: kept enabled (see `fpu_configure_eager`);
/// the flag is recorded for the Link_Checker's view of which Tasks may use
/// the FPU (PR-31, LK).
pub fn fpu_set_access(_on: bool) {}

// ------------------------------------------------------------------ MPU

/// MPU_RASR encoding of a region (B3.5.9): ENABLE | SIZE | SRD | memory
/// type (TEX/S/C/B) | AP | XN.
pub fn rasr_encode(r: &crate::logic::mpu::Region) -> u32 {
    if !r.enabled {
        return 0;
    }
    let size = ((r.interval.size_log2 as u32).saturating_sub(1) & 0x1F) << 1;
    let srd = (r.srd as u32) << 8;
    // Normal memory, write-back (TEX=0, C=1, B=1) for RAM and code;
    // Strongly-ordered (TEX=0, C=0, B=0) for device windows (DD-12).
    let attrs = if r.device { 0 } else { (1 << 17) | (1 << 16) };
    // AP = 0b011 full access, 0b010 unprivileged read-only.
    let ap = if r.write { 0b011 << 24 } else { 0b010 << 24 };
    let xn = if r.xn { 1 << 28 } else { 0 };
    1 | size | srd | attrs | ap | xn
}

/// MPU_RBAR encoding with the VALID bit and the region number (B3.5.8).
pub fn rbar_encode(number: u32, r: &crate::logic::mpu::Region) -> u32 {
    (r.interval.base & !0x1F) | (1 << 4) | (number & 0xF)
}

/// Loads a Partition view into the eight regions (DD-07).
pub fn mpu_load(view: &View) {
    let mut i = 0;
    while i < MPU_REGIONS {
        let r = &view.regions[i];
        mmio_write(addr::MPU_RBAR, rbar_encode(i as u32, r));
        mmio_write(addr::MPU_RASR, rasr_encode(r));
        i += 1;
    }
    barrier();
}

/// MPU_CTRL = PRIVDEFENA | ENABLE: the Kernel runs on the background map,
/// Jobs only see their regions (B3.5.7).
pub fn mpu_enable_background() {
    mmio_write(addr::MPU_CTRL, (1 << 2) | 1);
    barrier();
}

// ------------------------------------------------------------------ DWT

pub fn dwt_enable() {
    let v = mmio_read(addr::DEMCR);
    mmio_write(addr::DEMCR, v | (1 << 24));
    mmio_write(addr::DWT_CYCCNT, 0);
    let c = mmio_read(addr::DWT_CTRL);
    mmio_write(addr::DWT_CTRL, c | 1);
}

/// DWT_CYCCNT (HW-13).
pub fn dwt_cyccnt() -> u32 {
    mmio_read(addr::DWT_CYCCNT)
}

// ------------------------------------------------------------- boot checks

/// R12.1: core revision, MPU region count, DWT presence, priority bits
/// (`expected_bits`: 3 on the nRF52840, ASM-03; a board declares what it
/// implements, and at least 3 are needed for the DD-02 map). Returns a
/// bit set of the failed checks (R12.2: the identifier of the failed check
/// is kept in the event log): bit 0 implementer, bit 1 part number, bit 2
/// MPU regions, bit 3 DWT, bit 4 priority bits; bit 5 (set by the caller)
/// is the Generated_Config checksum of R12.3.
pub fn boot_checks(expected_bits: u8) -> u32 {
    let cpuid = mmio_read(addr::CPUID);
    let mut failed = 0u32;
    if (cpuid >> 24) != 0x41 {
        failed |= 1 << 0;
    }
    if (cpuid >> 4) & 0xFFF != 0xC24 {
        failed |= 1 << 1;
    }
    if (mmio_read(addr::MPU_TYPE) >> 8) & 0xFF != 8 {
        failed |= 1 << 2;
    }
    if (mmio_read(addr::DWT_CTRL) >> 25) & 1 != 0 {
        failed |= 1 << 3;
    }
    let bits = priority_bits_implemented();
    if bits != expected_bits || bits < 3 {
        failed |= 1 << 4;
    }
    failed
}

/// The number of implemented NVIC priority bits, by writing 0xFF to an
/// IPR entry and reading back (ASM-03).
pub fn priority_bits_implemented() -> u8 {
    let saved = mmio_read(addr::NVIC_IPR);
    mmio_write(addr::NVIC_IPR, saved | 0xFF);
    let v = mmio_read(addr::NVIC_IPR) & 0xFF;
    mmio_write(addr::NVIC_IPR, saved);
    (v as u8).count_ones() as u8
}

// -------------------------------------------------------------- time base

#[cfg(feature = "board-qemu-mps2")]
mod timebase {
    //! CMSDK APB timers of the QEMU mps2-an386: TIMER0 free-running for
    //! `now`, TIMER1 one-shot with interrupt for the compare (both
    //! down-counters at 25 MHz; CTRL bit0 enable, bit3 IRQ enable).
    use super::{addr, mmio_read, mmio_write};
    pub const CTRL: u32 = 0x0;
    pub const VALUE: u32 = 0x4;
    pub const RELOAD: u32 = 0x8;
    pub const INTCLEAR: u32 = 0xC;

    pub fn init() {
        mmio_write(addr::CMSDK_TIMER0 + CTRL, 0);
        mmio_write(addr::CMSDK_TIMER0 + RELOAD, 0xFFFF_FFFF);
        mmio_write(addr::CMSDK_TIMER0 + VALUE, 0xFFFF_FFFF);
        mmio_write(addr::CMSDK_TIMER0 + CTRL, 1);
        mmio_write(addr::CMSDK_TIMER1 + CTRL, 0);
    }

    /// Elapsed ticks since init, extended to 64 bits by counting wraps.
    pub fn now() -> u64 {
        let low = 0xFFFF_FFFFu32.wrapping_sub(mmio_read(addr::CMSDK_TIMER0 + VALUE));
        super::super::extend(low)
    }

    pub fn arm(at: u64) {
        let n = now();
        let distance = at.saturating_sub(n).clamp(2, 0xFFFF_FFFE) as u32;
        mmio_write(addr::CMSDK_TIMER1 + CTRL, 0);
        mmio_write(addr::CMSDK_TIMER1 + RELOAD, distance);
        mmio_write(addr::CMSDK_TIMER1 + VALUE, distance);
        mmio_write(addr::CMSDK_TIMER1 + CTRL, 0b1001);
    }

    pub fn ack() {
        mmio_write(addr::CMSDK_TIMER1 + INTCLEAR, 1);
        mmio_write(addr::CMSDK_TIMER1 + CTRL, 0);
    }
}

#[cfg(not(feature = "board-qemu-mps2"))]
mod timebase {
    //! nRF52840 TIMER0 (PS 6.30): 32-bit mode at 16 MHz (prescaler 0),
    //! CC[0] for the compare event, CAPTURE[1]/CC[1] to read the counter.
    use super::{addr, mmio_read, mmio_write};
    const TASKS_START: u32 = 0x000;
    const TASKS_STOP: u32 = 0x004;
    const TASKS_CLEAR: u32 = 0x00C;
    const TASKS_CAPTURE1: u32 = 0x044;
    const EVENTS_COMPARE0: u32 = 0x140;
    const INTENSET: u32 = 0x304;
    const INTENCLR: u32 = 0x308;
    const MODE: u32 = 0x504;
    const BITMODE: u32 = 0x508;
    const PRESCALER: u32 = 0x510;
    const CC0: u32 = 0x540;
    const CC1: u32 = 0x544;

    pub fn init() {
        let t = addr::NRF_TIMER0;
        mmio_write(t + TASKS_STOP, 1);
        mmio_write(t + MODE, 0);
        mmio_write(t + BITMODE, 3);
        mmio_write(t + PRESCALER, 0);
        mmio_write(t + INTENCLR, 0xFFFF_FFFF);
        mmio_write(t + TASKS_CLEAR, 1);
        mmio_write(t + TASKS_START, 1);
    }

    pub fn now() -> u64 {
        let t = addr::NRF_TIMER0;
        mmio_write(t + TASKS_CAPTURE1, 1);
        let low = mmio_read(t + CC1);
        super::super::extend(low)
    }

    pub fn arm(at: u64) {
        let t = addr::NRF_TIMER0;
        mmio_write(t + CC0, at as u32);
        mmio_write(t + INTENSET, 1 << 16);
    }

    pub fn ack() {
        let t = addr::NRF_TIMER0;
        mmio_write(t + EVENTS_COMPARE0, 0);
    }
}

pub fn timebase_init() {
    timebase::init();
}

pub fn timebase_now() -> u64 {
    timebase::now()
}

pub fn timebase_arm(at: u64) {
    timebase::arm(at);
}

pub fn timebase_ack() {
    timebase::ack();
}

// ---------------------------------------------------------- semihosting

/// Semihosting SYS_WRITE0 (QEMU `-semihosting`, HIL probe console).
/// Inlined so that unprivileged callers execute it from their own region.
#[inline(always)]
pub fn semihosting_write(s: &[u8]) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let mut buf = [0u8; 128];
        let n = s.len().min(127);
        buf[..n].copy_from_slice(&s[..n]);
        buf[n] = 0;
        // SAFETY (UJ-016): `bkpt 0xAB` is the semihosting call; r1 points
        // at a NUL-terminated buffer that outlives the call.
        unsafe {
            core::arch::asm!("bkpt #0xAB", inout("r0") 0x04u32 => _, in("r1") buf.as_ptr(), options(nostack, preserves_flags))
        }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::console(s);
    }
}

/// Semihosting SYS_EXIT_EXTENDED: ends the QEMU run with `code`.
pub fn semihosting_exit(code: u32) -> ! {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let block: [u32; 2] = [0x20026, code];
        // SAFETY (UJ-017): as UJ-016; the call does not return.
        unsafe {
            core::arch::asm!("bkpt #0xAB", inout("r0") 0x20u32 => _, in("r1") block.as_ptr(), options(nostack, preserves_flags));
        }
        loop {
            wait_for_interrupt();
        }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::exit(code)
    }
}
