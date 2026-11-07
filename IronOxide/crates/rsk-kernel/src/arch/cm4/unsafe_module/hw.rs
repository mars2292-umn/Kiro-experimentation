//! Register access with trusted specifications (R44.5, R45.4). Each
//! function is one Hardware_Model step; on the Target it is a volatile
//! access, elsewhere the simulated register file of `sim` records it.
//!
//! Address constants: Armv7-M ARM B3.2 (system control block), B3.4
//! (NVIC), B3.5 (MPU), C1.8 (DWT); Cortex-M4 TRM 4.3 (ACTLR); nRF52840 PS
//! 6.30 (TIMER); CMSDK APB timer for the QEMU mps2-an386 board.

use crate::arch::cm4::plan::BootExpect;
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
    /// MemManage Fault Address Register (valid when CFSR.MMARVALID).
    pub const MMFAR: u32 = 0xE000_ED34;
    pub const CPACR: u32 = 0xE000_ED88;
    pub const MPU_TYPE: u32 = 0xE000_ED90;
    pub const MPU_CTRL: u32 = 0xE000_ED94;
    pub const MPU_RNR: u32 = 0xE000_ED98;
    pub const MPU_RBAR: u32 = 0xE000_ED9C;
    pub const MPU_RASR: u32 = 0xE000_EDA0;
    pub const DEMCR: u32 = 0xE000_EDFC;
    pub const FPCCR: u32 = 0xE000_EF34;
    pub const FPDSCR: u32 = 0xE000_EF3C;
    /// Media and VFP Feature Register 0: non-zero when an FPU is present.
    pub const MVFR0: u32 = 0xE000_EF40;
    /// Debug Halting Control and Status: bit 0 C_DEBUGEN (a debugger is attached, ORQ-30).
    pub const DHCSR: u32 = 0xE000_EDF0;
    pub const DWT_CTRL: u32 = 0xE000_1000;
    pub const DWT_CYCCNT: u32 = 0xE000_1004;
    /// nRF52840 TIMER0 (PS 6.30).
    pub const NRF_TIMER0: u32 = 0x4000_8000;
    /// nRF52840 POWER.RESETREAS (PS 5.3.6): bit 1 DOG, a watchdog reset.
    pub const NRF_RESETREAS: u32 = 0x4000_0400;
    /// nRF52840 APPROTECT.DISABLE (PS 4.3, build codes with hardware access
    /// port protection): 0x5A keeps the debug port open until the next reset.
    pub const NRF_APPROTECT_DISABLE: u32 = 0x4000_0558;
    /// nRF52840 CLOCK.TASKS_HFCLKSTART (PS 6.5): starts the 64 MHz crystal oscillator.
    pub const NRF_HFCLKSTART: u32 = 0x4000_0000;
    /// nRF52840 CLOCK.EVENTS_HFCLKSTARTED (PS 6.5): the crystal oscillator is running and is the source.
    pub const NRF_HFCLKSTARTED: u32 = 0x4000_0100;
    /// nRF52840 CLOCK.HFCLKSTAT (PS 6.5): bit 0 SRC (1 = crystal), bit 16 STATE (running).
    pub const NRF_HFCLKSTAT: u32 = 0x4000_040C;
    /// nRF52840 NVMC.ICACHECNF (PS 6.17): bit 0 CACHEEN.
    pub const NRF_ICACHECNF: u32 = 0x4001_E540;
    /// nRF52840 WDT (PS 6.36), Kernel-owned (ORQ-31).
    pub const NRF_WDT: u32 = 0x4001_0000;
    /// nRF52840 PPI.CHENCLR (PS 6.19): Kernel-owned (DD-08), every channel cleared at boot.
    pub const NRF_PPI_CHENCLR: u32 = 0x4001_F504;
    /// QEMU mps2-an386 CMSDK APB timers.
    pub const CMSDK_TIMER0: u32 = 0x4000_0000;
    pub const CMSDK_TIMER1: u32 = 0x4000_1000;

    pub const ALL: &[u32] = &[
        ACTLR, NVIC_ISER, NVIC_ICER, NVIC_ISPR, NVIC_ICPR, NVIC_IPR, CPUID, ICSR, AIRCR, CCR, SHPR1, SHPR2, SHPR3,
        SHCSR, CFSR, MMFAR, CPACR, MPU_TYPE, MPU_CTRL, MPU_RNR, MPU_RBAR, MPU_RASR, DEMCR, FPCCR, FPDSCR, DWT_CTRL,
        DWT_CYCCNT, MVFR0, DHCSR, NRF_TIMER0, NRF_RESETREAS, NRF_APPROTECT_DISABLE, NRF_HFCLKSTART, NRF_HFCLKSTARTED, NRF_HFCLKSTAT, NRF_ICACHECNF, NRF_WDT, NRF_PPI_CHENCLR, CMSDK_TIMER0,
        CMSDK_TIMER1,
    ];

    /// Whether `a` is a word-aligned address in the system control space
    /// (0xE000_0000..0xE010_0000) or the peripheral region (0x4000_0000..0x6000_0000).
    pub const fn plausible(a: u32) -> bool {
        a % 4 == 0 && ((a >= 0xE000_0000 && a < 0xE010_0000) || (a >= 0x4000_0000 && a < 0x6000_0000))
    }

    /// The NVIC lines the Armv7-M register arrays can address (B3.4.6:
    /// ISER0 to ISER15, IPR0 to IPR123): the bound of `irq_of`.
    pub const NVIC_LINES: u32 = 496;

    /// A word-aligned address of the peripheral region (0x4000_0000 to
    /// 0x6000_0000): where the Target's peripherals and the EasyDMA
    /// registers of the DMA table live.
    pub const fn peripheral(a: u32) -> bool {
        a % 4 == 0 && a >= 0x4000_0000 && a < 0x6000_0000
    }

    /// A word of the private peripheral bus the Kernel is written to
    /// access: an entry of `ALL`, or an NVIC array word for a line below
    /// `NVIC_LINES`. The precise form of UJ-001's precondition for the
    /// PPB: `plausible` admits every word of the PPB, this admits the
    /// Kernel's registers only.
    pub const fn ppb_known(a: u32) -> bool {
        let mut i = 0;
        while i < ALL.len() {
            if ALL[i] == a {
                return true;
            }
            i += 1;
        }
        let words = 4 * NVIC_LINES.div_ceil(32);
        a % 4 == 0
            && ((a >= NVIC_ISER && a < NVIC_ISER + words)
                || (a >= NVIC_ICER && a < NVIC_ICER + words)
                || (a >= NVIC_ISPR && a < NVIC_ISPR + words)
                || (a >= NVIC_ICPR && a < NVIC_ICPR + words)
                || (a >= NVIC_IPR && a < NVIC_IPR + NVIC_LINES))
    }

    /// The precondition of UJ-001 and UJ-002, every word access of
    /// `mmio_read` and `mmio_write`: a Kernel register of the PPB or a
    /// word of the peripheral region. Stated here once; the Kani stub
    /// KS-01 asserts it on every access a harness reaches, so every
    /// harness that runs a Kernel_Arch path checks the addresses that
    /// path computes (verification_plan.md 6.1).
    pub const fn kernel_access_ok(a: u32) -> bool {
        peripheral(a) || ppb_known(a)
    }

    /// The precondition of UJ-009, the byte writes of `mmio_write_u8`: a
    /// byte of the NVIC_IPR array below `NVIC_LINES`.
    pub const fn kernel_byte_access_ok(a: u32) -> bool {
        a >= NVIC_IPR && a < NVIC_IPR + NVIC_LINES
    }
}

/// Reads a memory-mapped register. Precondition: `addr::kernel_access_ok(a)`
/// (UJ-001). Every address the Kernel computes is computed above this
/// function by code that Flight_Builds and the Kani harnesses share; only
/// the volatile instruction itself, in `raw`, differs between them.
#[cfg_attr(feature = "coverage", coverage(off))]
pub fn mmio_read(a: u32) -> u32 {
    raw::read_word(a)
}

/// An entry of the linker-provided initialization tables (the layout of
/// `rsk-entry`'s `Range`): `(start, end, load)`.
#[cfg(all(target_arch = "arm", target_os = "none"))]
#[repr(C)]
struct InitRange {
    start: u32,
    end: u32,
    load: u32,
}

#[cfg(all(target_arch = "arm", target_os = "none"))]
extern "C" {
    /// `(start, end, 0)` triples of the RAM ranges zeroed at reset,
    /// terminated by zeros: the Kernel's first, then one per Partition in
    /// declaration order (the Generator's `link.x`, `.rsk.tables`).
    static __rsk_bss_table: InitRange;
}

/// The most table entries walked and the most words zeroed (PR-16; the
/// bounds of `rsk-entry`'s reset path).
const INIT_RANGES: usize = 16;
const RAM_WORDS: usize = 65_536;

#[cfg(all(feature = "coverage", target_arch = "arm", target_os = "none"))]
extern "C" {
    /// `(start, end, 0)` triples of the coverage counter ranges, one per
    /// Partition, terminated by zeros (the Generator's `link.x`; task 19.2).
    static __rsk_cov_table: InitRange;
}

/// The coverage build (task 19.2, R34.4): zeroes every Partition's
/// coverage counters at instant 0, as the records are cleared, so that a
/// run's counts are its own and survive the reboot the probe's detach
/// causes (which never reaches instant 0). A Partition restart leaves the
/// counters alone (its range of the zero table is its image alone).
#[cfg(feature = "coverage")]
pub fn coverage_reset() {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-049): the table is the linker's for this build; each
        // range is a Partition's counters above its image, in its own RAM,
        // written here at instant 0 before any Job runs.
        unsafe {
            let mut entry = &__rsk_cov_table as *const InitRange;
            let mut i = 0;
            while i < INIT_RANGES {
                let (start, end) = ((*entry).start, (*entry).end);
                if start == 0 && end == 0 {
                    return;
                }
                let mut a = start as usize as *mut u32;
                let stop = end as usize as *mut u32;
                let mut words = 0;
                while a < stop && words < RAM_WORDS {
                    core::ptr::write_volatile(a, 0);
                    a = a.add(1);
                    words += 1;
                }
                entry = entry.add(1);
                i += 1;
            }
        }
    }
}

/// The coverage build's counter increment (task 19.2): rustc lowers every
/// `-C instrument-coverage` counter update to an atomic 64-bit add, which
/// the Cortex-M4 has no instruction for, so the compiler calls this
/// libcall. Placed in the shared code region (`.text.rsk_shared`), since
/// the Partitions' instrumented bodies call it under their MPU views. Not
/// atomic: a counter belongs to one function, and a lost increment under
/// preemption (two Tasks of a Partition in one shared helper) changes a
/// count, never whether a branch was taken, which is what the report
/// reads (R34.4). HIL builds only; never a Flight_Build function.
#[cfg(feature = "coverage")]
#[no_mangle]
#[link_section = ".text.rsk_shared"]
#[cfg_attr(feature = "coverage", coverage(off))]
pub extern "C" fn __atomic_fetch_add_8(counter: *mut u64, value: u64, _order: i32) -> u64 {
    // SAFETY (UJ-050): the pointer is the compiler's, to a counter of the
    // calling Partition's coverage range (`__rsk_cov_table`), aligned and
    // in that Partition's RAM; read and written once each.
    unsafe {
        let old = core::ptr::read_volatile(counter);
        core::ptr::write_volatile(counter, old.wrapping_add(value));
        old
    }
}

/// R14.5: re-initializes Partition `p`'s RAM image: its range of the
/// linker's zero table (entry `p + 1`; the Kernel's is entry 0) is zeroed.
/// The Generator places every Partition static (per-Task state, Resource
/// cells) in that range as `.bss`/`.data` with no load image, so every
/// static must be zero-initialized, which the generated cells are; a
/// Partition static with another initial value would have been wrong
/// since reset. Called from the Kernel level with every Job of the
/// Partition ended or killed, so nothing reads the range meanwhile.
pub fn partition_reinit(p: u8) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-041): the table is the linker's for this build (the
        // reset path of rsk-entry walks the same table); the entry's range
        // is Partition `p`'s own RAM
        // above its stack, which no Job of `p` uses now (all ended or
        // killed) and no other Partition can address (its MPU view).
        unsafe {
            let mut entry = &__rsk_bss_table as *const InitRange;
            let mut i = 0;
            while i < INIT_RANGES {
                let (start, end) = ((*entry).start, (*entry).end);
                if start == 0 && end == 0 {
                    return;
                }
                if i == p as usize + 1 {
                    let mut a = start as usize as *mut u32;
                    let stop = end as usize as *mut u32;
                    let mut words = 0;
                    while a < stop && words < RAM_WORDS {
                        core::ptr::write_volatile(a, 0);
                        a = a.add(1);
                        words += 1;
                    }
                    return;
                }
                entry = entry.add(1);
                i += 1;
            }
        }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::partition_reinit(p)
    }
}

/// Writes a memory-mapped register. Precondition: `addr::kernel_access_ok(a)`
/// (UJ-002); see `mmio_read`.
#[cfg_attr(feature = "coverage", coverage(off))]
pub fn mmio_write(a: u32, v: u32) {
    raw::write_word(a, v)
}

/// Writes one byte of the NVIC_IPR array. Precondition:
/// `addr::kernel_byte_access_ok(a)` (UJ-009).
fn mmio_write_u8(a: u32, v: u8) {
    raw::write_byte(a, v)
}

/// The volatile accesses: the one leaf of the Kernel_Unsafe_Module's
/// register path that differs between a Flight_Build and the host, where
/// the Kani stub KS-01 (`sim`) stands in and asserts the preconditions
/// above. Nothing is computed here, so the harnesses that run the
/// register functions check the same addresses and values the Target
/// receives (R45.1; verification_plan.md 6.1).
mod raw {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn read_word(a: u32) -> u32 {
        // SAFETY (UJ-001): `a` satisfies `addr::kernel_access_ok` (a
        // Kernel register of the PPB or a word of the peripheral region:
        // word aligned, device memory with the semantics the
        // Hardware_Model gives it, HW-01 to HW-13), which the Kani stub
        // asserts on every access of the harnesses `uj_001_mmio_addresses_valid`
        // and `apply_every_action_is_total` (every `Action` of Kernel_Arch).
        unsafe { core::ptr::read_volatile(a as usize as *const u32) }
    }

    #[cfg(all(target_arch = "arm", target_os = "none"))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn write_word(a: u32, v: u32) {
        // SAFETY (UJ-002): as UJ-001, for writes.
        unsafe { core::ptr::write_volatile(a as usize as *mut u32, v) }
    }

    #[cfg(all(target_arch = "arm", target_os = "none"))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn write_byte(a: u32, v: u8) {
        // SAFETY (UJ-009): as UJ-001; `a` satisfies `addr::kernel_byte_access_ok`,
        // a byte of the NVIC_IPR array, which the Armv7-M ARM defines as
        // byte accessible (B3.4.6).
        unsafe { core::ptr::write_volatile(a as usize as *mut u8, v) }
    }

    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn read_word(a: u32) -> u32 {
        super::super::sim::read(a)
    }

    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn write_word(a: u32, v: u32) {
        super::super::sim::write(a, v)
    }

    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    #[cfg_attr(feature = "coverage", coverage(off))]
    pub fn write_byte(a: u32, v: u8) {
        super::super::sim::write_u8(a, v)
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

/// The NVIC line of exception number `n`, or `None` when `n` is a system
/// exception (below 16) or beyond the register arrays: such a number is
/// not in any binding table the Generator emits, and an NVIC function
/// given one does nothing rather than write the word its arithmetic
/// would reach (before 2026-11-06 the line was `n.saturating_sub(16)`,
/// so a number above the arrays would have reached the SCB).
pub fn irq_of(n: u16) -> Option<u32> {
    if n < 16 || n as u32 - 16 >= addr::NVIC_LINES {
        None
    } else {
        Some(n as u32 - 16)
    }
}

pub fn nvic_enable(n: u16) {
    if let Some(irq) = irq_of(n) {
        mmio_write(addr::NVIC_ISER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

pub fn nvic_disable(n: u16) {
    if let Some(irq) = irq_of(n) {
        mmio_write(addr::NVIC_ICER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

pub fn nvic_set_pending(n: u16) {
    if let Some(irq) = irq_of(n) {
        mmio_write(addr::NVIC_ISPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

pub fn nvic_clear_pending(n: u16) {
    if let Some(irq) = irq_of(n) {
        mmio_write(addr::NVIC_ICPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

pub fn nvic_set_priority(n: u16, encoded: u8) {
    if let Some(irq) = irq_of(n) {
        mmio_write_u8(addr::NVIC_IPR + irq, encoded);
    }
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

/// The configurable fault status and the MemManage fault address, for the
/// HIL trace (R14.3 diagnosis; `trace` builds only read them).
pub fn fault_status() -> (u32, u32) {
    (mmio_read(addr::CFSR), mmio_read(addr::MMFAR))
}

/// The safe state's idle step (ORQ-31): the Kernel-owned watchdog is fed,
/// then the core waits for an interrupt. The Profile_Lint accepts
/// `loop { safe_state_idle() }` beside the idle loop as the two unbounded
/// loops (PR-16); the QEMU board has no watchdog.
pub fn safe_state_idle() {
    #[cfg(all(not(feature = "board-qemu-mps2"), feature = "boot-mutation"))]
    if super::boot_mutation() == crate::arch::cm4::MUT_WATCHDOG {
        wait_for_interrupt();
        return;
    }
    #[cfg(not(feature = "board-qemu-mps2"))]
    watchdog_feed();
    wait_for_interrupt();
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
/// Profile enforcement is MC and LK, not RT). R6.6 and PR-31 (Profile 0.4,
/// 2026-10-22) allow these data-movement instructions in Kernel code and
/// forbid arithmetic, comparison, conversion, and FPSCR writes
/// (verification_plan.md 5.7).
pub fn fpu_configure_eager(flush_to_zero: bool) {
    let v = mmio_read(addr::FPCCR);
    mmio_write(addr::FPCCR, (v | (1 << 31)) & !(1 << 30));
    // FPDSCR: the FPSCR every new FP context starts from (HW-10). FZ (bit
    // 24) when the declaration asks for it (erratum 839676's workaround,
    // task 3.4); the other defaults (DN, AHP, RMode) stay 0.
    mmio_write(addr::FPDSCR, if flush_to_zero { 1 << 24 } else { 0 });
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

/// Loads a Partition view into the eight regions (DD-07). Each region is
/// disabled (RNR, RASR = 0) before its base and attributes are written:
/// rebasing an enabled region would, for the instant between the RBAR and
/// RASR writes, apply the old size and attributes at the new base, and a
/// Partition window region (device, execute-never) rebased to the disabled
/// region's base 0 then covers the Kernel's own code while the Kernel
/// executes it (found 2026-10-23 on QEMU when the first Partition window
/// appeared: an IACCVIOL escalated to HardFault inside this function).
pub fn mpu_load(view: &View) {
    let mut i = 0;
    while i < MPU_REGIONS {
        let r = &view.regions[i];
        mmio_write(addr::MPU_RNR, i as u32);
        mmio_write(addr::MPU_RASR, 0);
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
/// MPU regions, bit 3 DWT, bit 4 priority bits, bit 8 CPUID variant and
/// revision (ASM-01), bit 9 PRIGROUP (ASM-03: the reset value, every
/// priority bit a preemption bit), bit 10 the high-frequency clock source
/// (ASM-05), bit 11 the instruction cache enabled (ASM-04), bit 12 the FPU
/// absent while a Task uses it; bits 5 to 7 and 13 are set by the caller
/// (`arch::cm4::entry::boot_core`): the `CONFIG` checksum of R12.3, the
/// interrupt-line count, a watchdog reset, the `LAYOUT` checksum. The clock and cache
/// checks read nRF52840 registers and are skipped on the QEMU board,
/// which has neither (TBR-23).
pub fn boot_checks(expect: &BootExpect) -> u32 {
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
    // The DWT is readable only with DEMCR.TRCENA set (Armv7-M ARM C1.6.5);
    // `dwt_enable` sets it a step later, and a probe may have left it
    // either way (seen on the HIL_Rig, 2026-11-02: NOCYCCNT read as set
    // after a probe detach cleared TRCENA), so the check sets it first.
    let demcr = mmio_read(addr::DEMCR);
    mmio_write(addr::DEMCR, demcr | (1 << 24));
    if (mmio_read(addr::DWT_CTRL) >> 25) & 1 != 0 {
        failed |= 1 << 3;
    }
    let bits = priority_bits_implemented();
    if bits != expect.priority_bits || bits < 3 {
        failed |= 1 << 4;
    }
    if cpuid != expect.cpuid {
        failed |= 1 << 8;
    }
    if (mmio_read(addr::AIRCR) >> 8) & 0x7 != 0 {
        failed |= 1 << 9;
    }
    // HIL builds keep the debug port open across resets (feature
    // `debug-port`): the first write of the boot, before anything that can
    // reset the chip (the watchdog, a fault response).
    #[cfg(all(not(feature = "board-qemu-mps2"), feature = "debug-port"))]
    mmio_write(addr::NRF_APPROTECT_DISABLE, 0x5A);
    #[cfg(not(feature = "board-qemu-mps2"))]
    {
        // The crystal oscillator is the Kernel's to start (DD-05: the time
        // base and the CPU run from it); after reset the chip runs on the
        // RC oscillator, whose HFCLKSTAT.STATE already reads "running".
        // Started here, before the check that follows, and waited for on
        // EVENTS_HFCLKSTARTED (set once the crystal is the source) over a
        // bounded number of polls (the crystal settles in well under a
        // millisecond; PR-16). Seen on the DK on 2026-10-29: waiting on
        // STATE alone let the check run before the source switched.
        if mmio_read(addr::NRF_HFCLKSTAT) & 1 == 0 {
            mmio_write(addr::NRF_HFCLKSTARTED, 0);
            mmio_write(addr::NRF_HFCLKSTART, 1);
            let mut polls: u32 = 0;
            while polls < 1_000_000 && mmio_read(addr::NRF_HFCLKSTARTED) == 0 {
                polls += 1;
            }
        }
        let hf = mmio_read(addr::NRF_HFCLKSTAT);
        let running = (hf >> 16) & 1 == 1;
        let xtal = hf & 1 == 1;
        if !running || xtal != expect.hfclk_xtal {
            failed |= 1 << 10;
        }
        if mmio_read(addr::NRF_ICACHECNF) & 1 != 0 {
            failed |= 1 << 11;
        }
    }
    if expect.fpu_needed && mmio_read(addr::MVFR0) == 0 {
        failed |= 1 << 12;
    }
    failed
}

/// Boot information for the event log (R12.2 context, ORQ-30, ORQ-31):
/// bit 0 a debugger is attached (DHCSR.C_DEBUGEN), bit 1 the reset was a
/// watchdog reset (nRF52840 RESETREAS.DOG, cleared here so that the next
/// reset reports its own reason). The QEMU board has no RESETREAS.
pub fn boot_info() -> u32 {
    let mut info = 0u32;
    if mmio_read(addr::DHCSR) & 1 != 0 {
        info |= BOOT_INFO_DEBUGGER;
    }
    #[cfg(not(feature = "board-qemu-mps2"))]
    {
        // nRF52840 anomaly 136 (task 3.4): after a pin reset, bits other
        // than RESETPIN may be set too; they are then ignored, and the
        // register is cleared in every case so that the next reset reports
        // its own reason.
        let reasons = mmio_read(addr::NRF_RESETREAS);
        if reasons & 1 == 0 && reasons & (1 << 1) != 0 {
            info |= BOOT_INFO_WATCHDOG_RESET;
        }
        if reasons != 0 {
            mmio_write(addr::NRF_RESETREAS, reasons);
        }
    }
    info
}

pub const BOOT_INFO_DEBUGGER: u32 = 1;
pub const BOOT_INFO_WATCHDOG_RESET: u32 = 2;

// -------------------------------------------------------------- watchdog

/// nRF52840 WDT register offsets (PS 6.36).
pub mod wdt {
    pub const TASKS_START: u32 = 0x000;
    pub const CRV: u32 = 0x504;
    pub const RREN: u32 = 0x508;
    pub const CONFIG: u32 = 0x50C;
    pub const RR0: u32 = 0x600;
    /// The reload request value.
    pub const RELOAD: u32 = 0x6E52_4635;
    /// CONFIG: keep running in sleep (bit 0), pause while halted by a
    /// debugger (bit 3 clear).
    pub const CONFIG_VALUE: u32 = 1;
}

/// Starts the Kernel-owned watchdog (ORQ-31) with `reload` ticks of its
/// 32.768 kHz clock, one reload register. Once started it runs until
/// reset; `watchdog_feed` from the time-base handler keeps it quiet, so a
/// Kernel hang that stops the handler resets the Target. The QEMU board
/// has no WDT: the writes go to unmapped peripheral space and are
/// ignored (TBR-23).
pub fn watchdog_start(reload: u32) {
    let w = addr::NRF_WDT;
    mmio_write(w + wdt::CRV, reload);
    mmio_write(w + wdt::RREN, 1);
    mmio_write(w + wdt::CONFIG, wdt::CONFIG_VALUE);
    mmio_write(w + wdt::TASKS_START, 1);
}

/// Feeds the watchdog (reload request on RR[0]).
pub fn watchdog_feed() {
    mmio_write(addr::NRF_WDT + wdt::RR0, wdt::RELOAD);
}

// ------------------------------------------------------------- EasyDMA (DD-08)

/// Writes an EasyDMA pointer register and, when `cnt_reg` is non-zero, the
/// count register (R17.1; the caller, `dma_request_core`, has validated the
/// buffer). The register addresses come from Generated_Config's DMA table;
/// an address outside the peripheral region is refused here as defence in
/// depth (UJ-002: every address written satisfies `kernel_access_ok`;
/// `plausible` was the guard before 2026-11-06, which admitted a word of
/// the PPB, where no EasyDMA register is).
pub fn dma_program(ptr_reg: u32, ptr: u32, cnt_reg: u32, len: u32) {
    if addr::peripheral(ptr_reg) {
        mmio_write(ptr_reg, ptr);
    }
    if cnt_reg != 0 && addr::peripheral(cnt_reg) {
        mmio_write(cnt_reg, len);
    }
}

/// The Δ_stamp probe of HIL builds (Table 13-1; 2026-11-06): PPI channel
/// 15 routes GPIOTE's IN[0] event (the stimulus edge that releases
/// `pulse`) to TIMER0's CAPTURE[3] task, so that the time base latches
/// the edge's instant in hardware; the intake handler reads CC[3] and
/// records the lag to its own timestamp. Kernel-owned PPI and TIMER0 (DD-08,
/// DD-05); the channel is set after the boot's clearing of every channel.
#[cfg(all(feature = "trace-ram", not(feature = "board-qemu-mps2")))]
pub fn stamp_probe_init() {
    const PPI: u32 = 0x4001_F000;
    const CHENSET: u32 = 0x504;
    const CH15_EEP: u32 = 0x510 + 15 * 8;
    const CH15_TEP: u32 = 0x514 + 15 * 8;
    const GPIOTE_EVENTS_IN0: u32 = 0x4000_6100;
    const TIMER0_TASKS_CAPTURE3: u32 = 0x04C;
    mmio_write(PPI + CH15_EEP, GPIOTE_EVENTS_IN0);
    mmio_write(PPI + CH15_TEP, addr::NRF_TIMER0 + TIMER0_TASKS_CAPTURE3);
    mmio_write(PPI + CHENSET, 1 << 15);
}

/// The time-base instant the probe latched at the last stimulus edge
/// (TIMER0 CC[3]; 0 before any edge).
#[cfg(all(feature = "trace-ram", not(feature = "board-qemu-mps2")))]
pub fn stamp_capture() -> u32 {
    mmio_read(addr::NRF_TIMER0 + 0x54C)
}

#[cfg(all(feature = "trace-ram", feature = "board-qemu-mps2"))]
pub fn stamp_probe_init() {}

#[cfg(all(feature = "trace-ram", feature = "board-qemu-mps2"))]
pub fn stamp_capture() -> u32 {
    0
}

/// The stack pointers, for the UsageFault diagnostic of HIL builds (the
/// INVPC finding of 2026-11-06, Verification_Plan section 13).
#[cfg(feature = "trace-ram")]
pub fn stack_pointers() -> (u32, u32) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        let (psp, msp): (u32, u32);
        // SAFETY (UJ-051): two register reads with no side effect.
        unsafe { core::arch::asm!("mrs {0}, psp", "mrs {1}, msp", out(reg) psp, out(reg) msp, options(nomem, nostack, preserves_flags)) };
        (psp, msp)
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        (0, 0)
    }
}

/// Triggers a stop task (R17.6).
pub fn dma_stop(reg: u32) {
    if addr::peripheral(reg) {
        mmio_write(reg, 1);
    }
}

/// The Kernel-mediated write of one register of an EasyDMA-capable
/// peripheral (DD-08; `dma_write_core` has validated the register and the
/// caller's ownership, task 19.2); an address outside the peripheral
/// region is refused here as for `dma_program`.
pub fn dma_write(reg: u32, value: u32) {
    if addr::peripheral(reg) {
        mmio_write(reg, value);
    }
}

/// Clears a list-mode register (DD-08: list mode forbidden).
pub fn dma_list_off(reg: u32) {
    if addr::peripheral(reg) {
        mmio_write(reg, 0);
    }
}

/// Disables every PPI channel (DD-08: PPI is Kernel-owned; no Partition
/// routes events). The QEMU board has no PPI.
pub fn ppi_disable_all() {
    #[cfg(not(feature = "board-qemu-mps2"))]
    mmio_write(addr::NRF_PPI_CHENCLR, 0xFFFF_FFFF);
}

// ------------------------------------------- Partition peripheral access

/// A Partition's read of one of its peripheral registers (through the
/// `rsk` API). Inlined into the Partition's code, so it executes from the
/// Partition's region under its MPU view: an address outside the
/// Partition's windows is a MemManage fault of that Partition (R16.4),
/// which is the isolation argument.
#[inline(always)]
#[cfg_attr(feature = "coverage", coverage(off))]
pub fn peripheral_read(a: u32) -> u32 {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-036): a volatile word read of an address the MPU view
        // of the executing Partition either permits (an owned peripheral
        // window, strongly-ordered device memory) or faults on; the fault is
        // attributed to the Partition (R14.3). No Rust memory is involved.
        unsafe { core::ptr::read_volatile(a as usize as *const u32) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::partition_read(a)
    }
}

/// A Partition's write of one of its peripheral registers (see `peripheral_read`).
#[inline(always)]
#[cfg_attr(feature = "coverage", coverage(off))]
pub fn peripheral_write(a: u32, v: u32) {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        // SAFETY (UJ-037): as UJ-036, for a volatile word write; EasyDMA
        // peripherals' windows are read-only, so their pointer registers
        // cannot be reached this way (DD-08).
        unsafe { core::ptr::write_volatile(a as usize as *mut u32, v) }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::partition_write(a, v)
    }
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

    /// TIMER0's interrupt line (nRF52840 IRQ 8), for a compare already due.
    const TIMER0_VECTOR: u16 = 16 + 8;

    pub fn arm(at: u64) {
        let t = addr::NRF_TIMER0;
        mmio_write(t + CC0, at as u32);
        mmio_write(t + INTENSET, 1 << 16);
        // The compare is absolute: an instant the counter has already passed
        // matches only after the 32-bit wrap (268 s at 16 MHz), so a due or
        // past instant pends the time-base interrupt instead; the handler
        // serves every due slot by `now`, not by the event. Seen on the DK on
        // 2026-10-29: the first release (instant 0) was armed after the
        // counter started and no tick ever came, so the watchdog reset the
        // chip. The QEMU timer is armed with a relative distance (HW-12).
        if now() >= at {
            super::nvic_set_pending(TIMER0_VECTOR);
        }
    }

    pub fn ack() {
        let t = addr::NRF_TIMER0;
        mmio_write(t + EVENTS_COMPARE0, 0);
    }
}

pub fn timebase_init() {
    timebase::init();
    #[cfg(feature = "trace-pins")]
    trace_pins::init();
}

/// CPU cycles per time-base tick (the Target table: 64 MHz over 16 MHz on
/// the nRF52840; the QEMU board's 25 MHz timer runs at the CPU clock).
#[cfg(not(feature = "board-qemu-mps2"))]
pub const CYCLES_PER_TICK: u32 = 4;
#[cfg(feature = "board-qemu-mps2")]
pub const CYCLES_PER_TICK: u32 = 1;

/// Trace pins (feature `trace-pins`, HIL builds on the nRF52840-DK only):
/// the executing Task's index on P1.11 to P1.13 (7 = none), and P1.14
/// high while a Kernel handler runs; the HIL_Rig's analyzer reads them on
/// D1 to D4 (`hil-rig.toml`). Two register writes per transition.
#[cfg(feature = "trace-pins")]
pub mod trace_pins {
    use super::{mmio_read, mmio_write};
    const P1: u32 = 0x5000_0300;
    const OUTSET: u32 = P1 + 0x508;
    const OUTCLR: u32 = P1 + 0x50C;
    const PIN_CNF: u32 = P1 + 0x700;
    const ID_SHIFT: u32 = 11;
    const ID_MASK: u32 = 0b111 << ID_SHIFT;
    const KERNEL: u32 = 1 << 14;

    pub fn init() {
        #[cfg(not(feature = "board-qemu-mps2"))]
        {
            let mut p = 11;
            while p <= 14 {
                mmio_write(PIN_CNF + 4 * p, 0b11);
                p += 1;
            }
            mmio_write(OUTSET, ID_MASK);
            mmio_write(OUTCLR, KERNEL);
        }
    }

    /// A Kernel handler runs.
    pub fn kernel_enter() {
        #[cfg(not(feature = "board-qemu-mps2"))]
        mmio_write(OUTSET, KERNEL);
    }

    /// The handler returns to `task` (`None`: the idle activity).
    pub fn running(task: Option<u8>) {
        #[cfg(not(feature = "board-qemu-mps2"))]
        {
            let id = match task {
                Some(t) if t < 7 => t as u32,
                _ => 7,
            };
            let bits = id << ID_SHIFT;
            mmio_write(OUTCLR, (ID_MASK & !bits) | KERNEL);
            if bits != 0 {
                mmio_write(OUTSET, bits);
            }
        }
        #[cfg(feature = "board-qemu-mps2")]
        let _ = task;
    }

    #[allow(dead_code)]
    fn _unused() -> u32 {
        mmio_read(OUTSET)
    }
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
#[cfg_attr(feature = "coverage", coverage(off))]
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

/// Whether a debugger is attached (DHCSR.C_DEBUGEN, ORQ-30), for
/// privileged callers (the PPB is privileged-only). On the QEMU board the
/// bit stays clear while semihosting works regardless, so there the
/// answer is always "yes". A board without a debugger must not execute a
/// semihosting breakpoint: it escalates to HardFault, and the watchdog
/// reset that follows would run the Application again every operating
/// duration (seen on the HIL_Rig, 2026-11-01: the target restarted every
/// 200 ms after probe-rs detached, and the next attach found the
/// watchdog-reset boot check failed, as ORQ-31 specifies).
pub fn debugger_attached() -> bool {
    #[cfg(feature = "board-qemu-mps2")]
    {
        true
    }
    #[cfg(not(feature = "board-qemu-mps2"))]
    {
        mmio_read(addr::DHCSR) & 1 != 0
    }
}

/// Semihosting SYS_EXIT_EXTENDED: ends the QEMU run with `code`. On a
/// board the probe resumes the core afterwards; the loop that follows then
/// keeps the Kernel-owned watchdog fed, so the reported safe state is a
/// stable state and not a reset every half second that the next boot would
/// log as a watchdog reset (seen on the DK, 2026-10-29). A hung Kernel
/// never reaches this loop, which is what the watchdog is for (ORQ-31).
/// Without a debugger the breakpoint is skipped (see `debugger_attached`)
/// and the loop is entered directly.
pub fn semihosting_exit(code: u32) -> ! {
    #[cfg(all(target_arch = "arm", target_os = "none"))]
    {
        if debugger_attached() {
            let block: [u32; 2] = [0x20026, code];
            // SAFETY (UJ-017): as UJ-016; the call returns only when a debugger
            // resumes the core, into the loop below.
            unsafe {
                core::arch::asm!("bkpt #0xAB", inout("r0") 0x20u32 => _, in("r1") block.as_ptr(), options(nostack, preserves_flags));
            }
        }
        loop {
            safe_state_idle();
        }
    }
    #[cfg(not(all(target_arch = "arm", target_os = "none")))]
    {
        super::sim::exit(code)
    }
}
