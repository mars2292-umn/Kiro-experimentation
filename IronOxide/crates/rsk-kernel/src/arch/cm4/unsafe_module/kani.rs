//! Kani harnesses linked from the justification identifiers of the
//! Kernel_Unsafe_Module (R45.1, R45.2, R6.4). Compiled only under
//! `cfg(kani)`.
//!
//! The harnesses run the register functions that Flight_Builds run: the
//! only code that differs between the Target and the host is the volatile
//! instruction in `hw::raw`, replaced by the stub KS-01 (`sim`), which
//! asserts the preconditions of UJ-001, UJ-002, and UJ-009 on every access
//! (verification_plan.md 6.1). `apply_every_action_is_total` drives every
//! `Action` of Kernel_Arch through `Cm4Hw::apply`, the function the
//! verified entry points call.
//!
//! Bounds (R45.8): the loops here are the address table (40 entries), the
//! stub's table walk per access, and the eight MPU regions; every scalar
//! input is symbolic over its whole type unless the harness says
//! otherwise.

use super::hw;
use super::hw::addr;
use super::sim;
use crate::arch::cm4::entry::{Action, Cm4Hw, KernelArch};
use crate::arch::cm4::plan::BootExpect;
use crate::logic::mpu::{Interval, Region, View, MPU_REGIONS};

/// The bound of every loop a register access can take: the stub walks
/// the address table (40 entries) once per access.
const UNWIND: usize = 48;

/// The HW-05 register word and bit of an NVIC line.
fn line(base: u32, irq: u32) -> (u32, u32) {
    (base + 4 * (irq / 32), 1 << (irq % 32))
}

/// UJ-001, UJ-002, UJ-009 (R45.1): every register address the Kernel
/// accesses satisfies the precondition the stub asserts. The address
/// table, the time-base and watchdog offsets, and the NVIC functions for
/// every exception number: a number whose line is below `NVIC_LINES`
/// reaches its HW-05 word and bit (HW-01 for the priority byte), and any
/// other number writes nothing at all.
#[kani::proof]
#[kani::unwind(48)]
fn uj_001_mmio_addresses_valid() {
    for a in addr::ALL {
        assert!(addr::plausible(*a));
        assert!(addr::kernel_access_ok(*a));
    }
    let t = addr::NRF_TIMER0;
    for off in [0x000u32, 0x004, 0x00C, 0x044, 0x140, 0x304, 0x308, 0x504, 0x508, 0x510, 0x540, 0x544] {
        assert!(addr::kernel_access_ok(t + off));
    }
    let w = addr::NRF_WDT;
    for off in [hw::wdt::TASKS_START, hw::wdt::CRV, hw::wdt::RREN, hw::wdt::CONFIG, hw::wdt::RR0] {
        assert!(addr::kernel_access_ok(w + off));
    }
    sim::reset();
    let n: u16 = kani::any();
    let p: u8 = kani::any();
    let before = sim::write_count();
    hw::nvic_enable(n);
    hw::nvic_disable(n);
    hw::nvic_set_pending(n);
    hw::nvic_clear_pending(n);
    hw::nvic_set_priority(n, p);
    if n >= 16 && (n as u32 - 16) < addr::NVIC_LINES {
        let irq = n as u32 - 16;
        assert!(hw::irq_of(n) == Some(irq));
        for base in [addr::NVIC_ISER, addr::NVIC_ICER, addr::NVIC_ISPR, addr::NVIC_ICPR] {
            let (word, bit) = line(base, irq);
            assert!(sim::read(word) & bit != 0);
        }
        let byte = (sim::read((addr::NVIC_IPR + irq) & !3) >> (8 * (irq % 4))) & 0xFF;
        // The stub models the three implemented bits of IPR0 (ASM-03).
        assert!(byte & 0xE0 == (p & 0xE0) as u32);
        assert!(sim::write_count() == before + 5);
    } else {
        assert!(hw::irq_of(n).is_none());
        assert!(sim::write_count() == before);
    }
}

/// UJ-007: the initial frame has the HW-06 layout for any entry and
/// trampoline address.
#[kani::proof]
fn uj_007_frame_layout() {
    let entry: u32 = kani::any();
    let tramp: u32 = kani::any();
    let f = super::frame_words(entry, tramp);
    assert!(f[5] & 1 == 1);
    assert!(f[6] & 1 == 0);
    assert!(f[7] == 1 << 24);
    assert!(f[0] == 0 && f[1] == 0 && f[2] == 0 && f[3] == 0 && f[4] == 0);
}

/// The RASR encoding places every field where B3.5.9 defines it and never
/// enables a disabled region.
#[kani::proof]
fn rasr_encoding_fields() {
    let enabled: bool = kani::any();
    let size_log2: u8 = kani::any();
    kani::assume(size_log2 >= 5 && size_log2 <= 31);
    let srd: u8 = kani::any();
    let write: bool = kani::any();
    let xn: bool = kani::any();
    let device: bool = kani::any();
    let r = Region { enabled, interval: Interval { base: 0, size_log2 }, srd, write, xn, device };
    let v = hw::rasr_encode(&r);
    if !enabled {
        assert!(v == 0);
    } else {
        assert!(v & 1 == 1);
        assert!(((v >> 1) & 0x1F) as u8 == size_log2 - 1);
        assert!(((v >> 8) & 0xFF) as u8 == srd);
        assert!(((v >> 24) & 0x7) == if write { 0b011 } else { 0b010 });
        assert!(((v >> 28) & 1 == 1) == xn);
        // Device windows are Strongly-ordered (TEX = C = B = 0).
        if device {
            assert!((v >> 16) & 0x3F == 0);
        }
    }
}

/// The RBAR encoding carries the aligned base, VALID, and the region number.
#[kani::proof]
fn rbar_encoding_fields() {
    let base: u32 = kani::any();
    let number: u32 = kani::any();
    kani::assume(number < MPU_REGIONS as u32);
    let r = Region { enabled: true, interval: Interval { base, size_log2: 8 }, srd: 0, write: true, xn: true, device: false };
    let v = hw::rbar_encode(number, &r);
    assert!(v & 0xF == number);
    assert!(v & (1 << 4) != 0);
    assert!(v & !0x1F == base & !0x1F);
}

/// UJ-010: the BASEPRI value for every ceiling is the HW-01/HW-03 encoding
/// and never masks the Kernel level (R18.2).
#[kani::proof]
fn uj_010_basepri_never_masks_kernel_level() {
    let c: u8 = kani::any();
    kani::assume(c <= 7);
    let v = super::super::basepri_for_ceiling(c);
    if c == 0 {
        assert!(v == 0);
    } else {
        assert!(v == (8 - c) << 5);
        assert!(v > 0, "a non-zero BASEPRI masks only group priorities >= BASEPRI, never priority 0");
    }
}

/// A view of eight symbolic regions.
fn any_view() -> View {
    let mut regions = [Region { enabled: false, interval: Interval { base: 0, size_log2: 5 }, srd: 0, write: false, xn: true, device: false }; MPU_REGIONS];
    let mut i = 0;
    while i < MPU_REGIONS {
        regions[i] = Region {
            enabled: kani::any(),
            interval: Interval { base: kani::any(), size_log2: kani::any() },
            srd: kani::any(),
            write: kani::any(),
            xn: kani::any(),
            device: kani::any(),
        };
        i += 1;
    }
    View { regions }
}

/// `mpu_load` writes eight RBAR/RASR pairs, in order, with the encodings
/// of the view's regions (simulated register file), for any view (HW-09).
#[kani::proof]
#[kani::unwind(48)]
fn mpu_load_writes_every_region() {
    sim::reset();
    let view = any_view();
    let before = sim::write_count();
    hw::mpu_load(&view);
    // Four writes per region: select, disable, base, attributes.
    assert!(sim::write_count() == before + 4 * MPU_REGIONS as u32);
    // The last pair written is region 7's.
    assert!(sim::read(addr::MPU_RNR) == 7);
    assert!(sim::read(addr::MPU_RBAR) == hw::rbar_encode(7, &view.regions[7]));
    assert!(sim::read(addr::MPU_RASR) == hw::rasr_encode(&view.regions[7]));
}

/// R45.2, R45.3, R6.4 (UJ-001, UJ-002, UJ-009): every `Action` of
/// Kernel_Arch, with any arguments, runs `Cm4Hw::apply`, the function the
/// verified entry points call, on the stub without a panic, an overflow,
/// or an access outside the Kernel's registers (the stub's assertion),
/// and leaves the simulated registers as the Hardware_Model rule of the
/// action says: HW-01 (priority byte), HW-03 (BASEPRI), HW-04 (PRIMASK),
/// HW-05 (NVIC pend, enable, disable, clear), HW-06 (the frame),
/// HW-07 (CCR), HW-09 (the MPU view and control), HW-10 (FPCCR, FPDSCR,
/// CPACR), HW-11 (ACTLR), HW-12 (the compare), and the SHPR, SHCSR,
/// AIRCR, DEMCR, DWT, EasyDMA, PPI, and watchdog writes.
/// Task 21.2 (R45.2): the Endpoint message copy, `hw::copy_words`, is total
/// and in bounds for every argument: with aligned ranges that do not wrap
/// it copies exactly `len / 4` words from `src` to `dst`, and with any
/// other argument it touches nothing.
#[kani::proof]
#[kani::unwind(112)]
fn copy_words_is_total_and_in_bounds() {
    sim::reset();
    // Words of the SRAM (the stub asserts the range on every access, UJ-056);
    // alignment and overlap stay free; the loops have constant trip counts
    // so that the unwinding bound is the harness's own.
    let dst: u32 = kani::any();
    let src: u32 = kani::any();
    let len: u32 = kani::any();
    kani::assume(dst >= 0x2000_0000 && dst < 0x2000_0100);
    kani::assume(src >= 0x2000_0000 && src < 0x2000_0100);
    kani::assume(len <= 32);
    let aligned = len % 4 == 0 && dst % 4 == 0 && src % 4 == 0;
    let pattern: u32 = kani::any();
    // Disjoint ranges (the Endpoint storage and a Partition's RAM never
    // overlap, R16.1): the destination holds the source's words afterwards.
    let disjoint = dst >= src + len || src >= dst + len;
    if aligned {
        for k in 0..8u32 {
            if 4 * k < len {
                sim::ram_write(src + 4 * k, pattern ^ k);
            }
        }
    }
    let before = sim::write_count();
    hw::copy_words(dst, src, len);
    let writes = sim::write_count() - before;
    if aligned {
        assert!(writes == len / 4);
        if disjoint {
            for k in 0..8u32 {
                if 4 * k < len {
                    assert!(sim::ram_read(dst + 4 * k) == pattern ^ k);
                }
            }
        }
    } else {
        assert!(writes == 0);
    }
}

/// UJ-058 and UJ-059 (task 21.2; R45.1): a message round-trips through
/// the word buffer for a value smaller than the buffer and equal to it,
/// for every value of four types (a type larger than its buffer does not
/// compile).
#[kani::proof]
fn uj_058_059_message_words_round_trip() {
    let a: u32 = kani::any();
    let w = hw::message_to_words::<u32, 1>(&a);
    assert!(w.0[0] == a);
    assert!(hw::message_from_words::<u32, 1>(&w) == a);
    let b: [u8; 6] = kani::any();
    let w = hw::message_to_words::<[u8; 6], 2>(&b);
    assert!(hw::message_from_words::<[u8; 6], 2>(&w) == b);
    assert!(w.0[1] >> 16 == 0);
    let c: u64 = kani::any();
    let w = hw::message_to_words::<u64, 2>(&c);
    assert!(hw::message_from_words::<u64, 2>(&w) == c);
    let d: f32 = kani::any();
    let w = hw::message_to_words::<f32, 1>(&d);
    assert!(hw::message_from_words::<f32, 1>(&w).to_bits() == d.to_bits());
}

#[kani::proof]
#[kani::unwind(48)]
fn apply_every_action_is_total() {
    sim::reset();
    let which: u8 = kani::any();
    kani::assume(which < 33);
    let n: u16 = kani::any();
    let p: u8 = kani::any();
    let on: bool = kani::any();
    let w: u32 = kani::any();
    let x: u32 = kani::any();
    let c: u32 = kani::any();
    let l: u32 = kani::any();
    let at: u64 = kani::any();
    let view = any_view();
    let a = match which {
        0 => Action::SetBasepri(p),
        1 => Action::Pend(n),
        2 => Action::Enable(n),
        3 => Action::Disable(n),
        4 => Action::SetPriority(n, p),
        5 => Action::ArmCompare(at),
        6 => Action::TimebaseAck,
        7 => Action::TimebaseInit,
        8 => Action::DwtEnable,
        9 => Action::LoadView(p, view),
        10 => Action::FpuAccess(on),
        11 => Action::WriteFrame(w, x),
        12 => Action::PrimaskSet,
        13 => Action::PrimaskClear,
        14 => Action::PrigroupAllPreempt,
        15 => Action::Nonbasethrdena(on),
        16 => Action::Disdefwbuf(on),
        17 => Action::FpuEager(on),
        18 => Action::KernelExceptionPriorities,
        19 => Action::MpuEnableBackground,
        20 => Action::ClearFaultStatus,
        21 => Action::ClearPending(n),
        22 => Action::DmaProgram(w, x, c, l),
        23 => Action::DmaStop(w),
        24 => Action::DmaListOff(w),
        25 => Action::PpiDisableAll,
        26 => Action::ReinitPartition(p),
        27 => Action::WatchdogStart(w),
        28 => Action::WatchdogFeed,
        29 => Action::DmaWrite(w, x),
        30 => Action::DmaShortsOff(w),
        31 => {
            // The Endpoint copy (task 21.2): words of the SRAM, as the cores
            // guarantee (`message_permitted`, `endpoint_slot`).
            kani::assume(w >= 0x2000_0000 && w < 0x2000_0100);
            kani::assume(x >= 0x2000_0000 && x < 0x2000_0100);
            Action::Copy(w, x, l & 0x1C)
        }
        _ => Action::Trace(p, w),
    };
    let before = sim::write_count();
    let mut cm4 = Cm4Hw::new();
    cm4.apply(a);
    let writes = sim::write_count() - before;
    let valid_line = n >= 16 && (n as u32 - 16) < addr::NVIC_LINES;
    let irq = (n as u32).wrapping_sub(16);
    match which {
        0 => assert!(sim::basepri() == p),
        1 | 2 | 3 | 21 => {
            let base = match which {
                1 => addr::NVIC_ISPR,
                2 => addr::NVIC_ISER,
                3 => addr::NVIC_ICER,
                _ => addr::NVIC_ICPR,
            };
            if valid_line {
                let (word, bit) = line(base, irq);
                assert!(sim::read(word) == bit);
                assert!(writes == 1);
            } else {
                assert!(writes == 0);
            }
        }
        4 => {
            if valid_line {
                let byte = (sim::read((addr::NVIC_IPR + irq) & !3) >> (8 * (irq % 4))) & 0xFF;
                assert!(byte & 0xE0 == (p & 0xE0) as u32);
                assert!(writes == 1);
            } else {
                assert!(writes == 0);
            }
        }
        5 => {
            // HW-12: the compare is written with the low word; a due instant
            // pends the time-base line instead (the counter reads 0 here).
            assert!(sim::read(addr::NRF_TIMER0 + 0x540) == at as u32);
            assert!(sim::read(addr::NRF_TIMER0 + 0x304) == 1 << 16);
            let (word, bit) = line(addr::NVIC_ISPR, 8);
            assert!((sim::read(word) & bit != 0) == (at == 0));
        }
        6 => assert!(sim::read(addr::NRF_TIMER0 + 0x140) == 0 && writes == 1),
        7 => assert!(sim::read(addr::NRF_TIMER0 + 0x000) == 1 && sim::read(addr::NRF_TIMER0 + 0x508) == 3),
        8 => {
            assert!(sim::read(addr::DEMCR) & (1 << 24) != 0);
            assert!(sim::read(addr::DWT_CYCCNT) == 0);
            assert!(sim::read(addr::DWT_CTRL) & 1 == 1);
        }
        9 => {
            assert!(writes == 4 * MPU_REGIONS as u32);
            assert!(sim::read(addr::MPU_RASR) == hw::rasr_encode(&view.regions[7]));
        }
        10 => assert!(writes == 0),
        11 => {
            let (sp, words) = sim::last_frame();
            assert!(sp == w);
            assert!(words[5] & 1 == 1 && words[6] == x & !1 && words[7] == 1 << 24);
        }
        12 => assert!(sim::primask()),
        13 => assert!(!sim::primask()),
        14 => assert!(sim::read(addr::AIRCR) == (0x05FA << 16) | (4 << 8)),
        15 => assert!((sim::read(addr::CCR) & 1 == 1) == on),
        16 => assert!((sim::read(addr::ACTLR) & 2 == 2) == on),
        17 => {
            let fpccr = sim::read(addr::FPCCR);
            assert!(fpccr & (1 << 31) != 0 && fpccr & (1 << 30) == 0);
            assert!(sim::read(addr::FPDSCR) == if on { 1 << 24 } else { 0 });
            assert!(sim::read(addr::CPACR) & (0xF << 20) == 0xF << 20);
        }
        18 => {
            assert!(sim::read(addr::SHPR1) == 0 && sim::read(addr::SHPR2) == 0 && sim::read(addr::SHPR3) == 0);
            assert!(sim::read(addr::SHCSR) & (7 << 16) == 7 << 16);
        }
        19 => assert!(sim::read(addr::MPU_CTRL) == (1 << 2) | 1),
        20 => assert!(writes == 1),
        31 => {
            // The Endpoint copy (task 21.2): aligned, non-wrapping ranges copy
            // their words; anything else touches nothing.
            let len = l & 0x1C;
            let aligned = w % 4 == 0 && x % 4 == 0;
            assert!(writes == if aligned { len / 4 } else { 0 });
        }
        22 => {
            let ptr_written = addr::peripheral(w);
            let cnt_written = c != 0 && addr::peripheral(c);
            if ptr_written {
                assert!(sim::read(w) == x || (cnt_written && c == w));
            }
            if cnt_written {
                assert!(sim::read(c) == l);
            }
            assert!(writes == ptr_written as u32 + cnt_written as u32);
        }
        23 | 24 | 30 => {
            if addr::peripheral(w) {
                assert!(sim::read(w) == if which == 23 { 1 } else { 0 } && writes == 1);
            } else {
                assert!(writes == 0);
            }
        }
        // The address is typed here from the nRF52840 Product Specification's PPI
        // register table (base 0x4001F000: CHEN 0x500, CHENSET 0x504, CHENCLR 0x508),
        // not taken from `addr`, so that a wrong constant there fails this case
        // (audit finding 3: the shared constant was CHENSET and the case agreed with it).
        25 => assert!(sim::read(0x4001_F508u32) == 0xFFFF_FFFF && writes == 1),
        26 => assert!(writes == 1),
        27 => {
            assert!(sim::read(addr::NRF_WDT + hw::wdt::CRV) == w);
            assert!(sim::read(addr::NRF_WDT + hw::wdt::RREN) == 1);
            assert!(sim::read(addr::NRF_WDT + hw::wdt::CONFIG) == hw::wdt::CONFIG_VALUE);
            assert!(sim::read(addr::NRF_WDT + hw::wdt::TASKS_START) == 1);
        }
        28 => assert!(sim::read(addr::NRF_WDT + hw::wdt::RR0) == hw::wdt::RELOAD && writes == 1),
        29 => {
            if addr::peripheral(w) {
                assert!(sim::read(w) == x && writes == 1);
            } else {
                assert!(writes == 0);
            }
        }
        _ => assert!(writes == 0),
    }
}

/// R12.1, R12.2, R45.2: `boot_checks` sets exactly the bit of each check
/// that fails, for any register contents and any expectation, and never
/// panics. The crystal is assumed already the clock source (HFCLKSTAT.SRC),
/// so the bounded start-up poll is not taken; the stub implements three
/// priority bits (ASM-03).
#[kani::proof]
#[kani::unwind(48)]
fn boot_checks_report_each_failed_check() {
    sim::reset();
    let cpuid: u32 = kani::any();
    let mpu_type: u32 = kani::any();
    let dwt_ctrl: u32 = kani::any();
    let aircr: u32 = kani::any();
    let mvfr0: u32 = kani::any();
    let hf: u32 = kani::any();
    kani::assume(hf & 1 == 1);
    let icache: u32 = kani::any();
    sim::write(addr::CPUID, cpuid);
    sim::write(addr::MPU_TYPE, mpu_type);
    sim::write(addr::DWT_CTRL, dwt_ctrl);
    sim::write(addr::AIRCR, aircr);
    sim::write(addr::MVFR0, mvfr0);
    sim::write(addr::NRF_HFCLKSTAT, hf);
    sim::write(addr::NRF_ICACHECNF, icache);
    // DD-08 (audit finding 3): the PPI channel enables read back after the boot clear.
    let chen: u32 = kani::any();
    sim::write(addr::NRF_PPI_CHEN, chen);
    let expect = BootExpect { priority_bits: kani::any(), cpuid: kani::any(), hfclk_xtal: kani::any(), fpu_needed: kani::any() };
    let failed = hw::boot_checks(&expect);
    assert!((failed & 1 != 0) == ((cpuid >> 24) != 0x41));
    assert!((failed & (1 << 1) != 0) == ((cpuid >> 4) & 0xFFF != 0xC24));
    assert!((failed & (1 << 2) != 0) == ((mpu_type >> 8) & 0xFF != 8));
    assert!((failed & (1 << 3) != 0) == ((dwt_ctrl >> 25) & 1 != 0));
    assert!((failed & (1 << 4) != 0) == (expect.priority_bits != 3));
    assert!((failed & (1 << 8) != 0) == (cpuid != expect.cpuid));
    assert!((failed & (1 << 9) != 0) == ((aircr >> 8) & 7 != 0));
    assert!((failed & (1 << 10) != 0) == ((hf >> 16) & 1 == 0 || !expect.hfclk_xtal));
    assert!((failed & (1 << 11) != 0) == (icache & 1 != 0));
    assert!((failed & (1 << 12) != 0) == (expect.fpu_needed && mvfr0 == 0));
    assert!((failed & (1 << 14) != 0) == (chen != 0));
    // Bits 5 to 7, 13 and 15 are the caller's; nothing else is reported.
    assert!(failed & !0b101_1111_0001_1111 == 0);
    // The DWT was made readable for its check (DEMCR.TRCENA).
    assert!(sim::read(addr::DEMCR) & (1 << 24) != 0);
}

/// R12.2, ORQ-30, ORQ-31, R45.2: `boot_info` reports a debugger from
/// DHCSR and a watchdog reset from RESETREAS (anomaly 136: ignored after
/// a pin reset), clearing the reasons it read, for any register contents.
#[kani::proof]
#[kani::unwind(48)]
fn boot_info_reports_debugger_and_watchdog() {
    sim::reset();
    let dhcsr: u32 = kani::any();
    let reasons: u32 = kani::any();
    sim::write(addr::DHCSR, dhcsr);
    sim::write(addr::NRF_RESETREAS, reasons);
    let before = sim::write_count();
    let info = hw::boot_info();
    assert!((info & hw::BOOT_INFO_DEBUGGER != 0) == (dhcsr & 1 != 0));
    assert!((info & hw::BOOT_INFO_WATCHDOG_RESET != 0) == (reasons & 1 == 0 && reasons & 2 != 0));
    assert!(info & !3 == 0);
    assert!(sim::write_count() - before == (reasons != 0) as u32);
}

/// The audit's contested finding B (2026-11-13), ORQ-31, R45.2: the safe
/// state's busy wait between two watchdog feeds ends for any contents of
/// the cycle counter, a counter that does not advance included (the poll
/// bound), and its two bounds are inside the watchdog's reload on the
/// nRF52840: the feed interval in cycles, and the poll bound at the
/// stated cycles per poll, each converted to watchdog ticks at the 64 MHz
/// CPU clock (`CYCLES_PER_TICK` cycles per tick of the 16 MHz time base),
/// against `watchdog_reload` for that time base. One idle step feeds the
/// watchdog exactly once.
#[kani::proof]
#[kani::unwind(48)]
fn safe_state_feed_interval_is_inside_the_watchdog_reload() {
    use crate::arch::cm4::entry::{watchdog_reload, WATCHDOG_HZ};
    let tick_hz: u64 = 16_000_000;
    let cpu_hz: u64 = tick_hz * crate::logic::kernel::CYCLES_PER_TICK;
    let reload = watchdog_reload(tick_hz) as u64;
    let by_counter = hw::SAFE_STATE_FEED_CYCLES as u64 * WATCHDOG_HZ / cpu_hz + 1;
    let by_polls = hw::SAFE_STATE_FEED_POLLS as u64 * hw::SAFE_STATE_POLL_CYCLES_MAX as u64 * WATCHDOG_HZ / cpu_hz + 1;
    assert!(by_counter < reload && by_polls < reload);
    // The wait itself, at small bounds: it returns whatever the counter reads.
    sim::reset();
    sim::write(addr::DWT_CYCCNT, kani::any());
    let cycles: u32 = kani::any();
    let before = sim::write_count();
    hw::safe_state_wait::<4>(cycles);
    assert!(sim::write_count() == before);
}

// ---------------------------------------------------------------------------
// The review of the audit fixes (2026-11-13): Kernel_Arch's level records
// against the Kernel's preemption stack over a sequence of Kernel entries.

/// The hardware of the sequence harness: a time base and a cycle counter the
/// harness sets, and every `Action` accepted (the register writes are
/// `apply_every_action_is_total`'s subject, not this harness's).
struct SeqHw {
    now: u64,
    cycles: u32,
}

impl KernelArch for SeqHw {
    fn apply(&mut self, _a: Action) {}
    fn copy_in(&mut self, _dst: u32, _src: u32, _len: u32) -> u32 {
        0
    }
    fn copy_out(&mut self, _dst: u32, _src: u32, _len: u32) -> u32 {
        0
    }
    fn now(&self) -> u64 {
        self.now
    }
    fn cycles(&self) -> u32 {
        self.cycles
    }
    fn boot_checks(&self, _expect: &BootExpect) -> u32 {
        0
    }
    fn boot_info(&self) -> u32 {
        0
    }
}

/// The demo's tables as far as the entry cores read them: the dispatch
/// vectors 36 to 42 in Task order, the time base on vector 25, `pulse`'s
/// source on vector 26, the stack regions of the demo's layout, no EasyDMA
/// channel, Endpoint or signal.
struct SeqTables;

const SEQ_RAM: [u32; 3] = [0x2000_8000, 0x2001_0000, 0x2001_8000];
const SEQ_STACK: u32 = 4096;

impl crate::arch::cm4::entry::Tables for SeqTables {
    fn binding(&self, n: u16) -> crate::arch::Binding {
        if n == 25 {
            crate::arch::Binding::Kernel
        } else if n >= 36 && n <= 42 {
            crate::arch::Binding::Dispatch((n - 36) as u8)
        } else if n == 26 {
            crate::arch::Binding::Intake { task: 4, partition: 1 }
        } else {
            crate::arch::Binding::Unbound
        }
    }
    fn dispatch_vector(&self, t: u8) -> u16 {
        36 + t as u16
    }
    fn intake_vector(&self, t: u8) -> Option<u16> {
        if t == 4 { Some(26) } else { None }
    }
    fn stack_top(&self, p: u8) -> u32 {
        if (p as usize) < 3 { SEQ_RAM[p as usize] + SEQ_STACK } else { 0 }
    }
    fn stack_bottom(&self, p: u8) -> u32 {
        if (p as usize) < 3 { SEQ_RAM[p as usize] } else { 0 }
    }
    fn task_entry_address(&self, _t: u8) -> u32 {
        0x0002_0101
    }
    fn irq_count(&self) -> u16 {
        48
    }
    fn config_checksum(&self) -> u32 {
        0
    }
    fn layout_checksum(&self) -> u32 {
        0
    }
    fn timer_vector(&self) -> u16 {
        25
    }
    fn priority_bits(&self) -> u8 {
        3
    }
    fn boot_expect(&self) -> BootExpect {
        BootExpect { priority_bits: 3, cpuid: 0x410F_C241, hfclk_xtal: true, fpu_needed: true }
    }
    fn fp_flush_to_zero(&self) -> bool {
        false
    }
    fn tick_hz(&self) -> u64 {
        16_000_000
    }
    fn dma_count(&self) -> usize {
        0
    }
    fn dma(&self, _i: usize) -> crate::arch::DmaCfg {
        crate::arch::DmaCfg { partition: 0xFF, ptr: 0, maxcnt: 0, stops: [0, 0], lists: [0, 0], shorts: 0 }
    }
    fn endpoint_count(&self) -> usize {
        0
    }
    fn endpoint(&self, _i: usize) -> crate::arch::EndpointCfg {
        crate::arch::EndpointCfg { capacity: 0, payload: 0, roles: 0, tasks: [0xFF; 4], partitions: [0xFF; 4], release: [false; 4], storage: 0, validates: [[false; 4]; 4] }
    }
    fn endpoint_storage(&self) -> (u32, u32) {
        (0, 0)
    }
    fn signal_count(&self) -> usize {
        0
    }
    fn signal(&self, _i: usize) -> crate::arch::SignalCfg {
        crate::arch::SignalCfg { partition: 0xFF, task: 0xFF }
    }
}

type SeqKernel = crate::arch::Bound<7, 3, 3, 6, 23, 32>;

/// The executable form of `entry::arch_matches`: as many active level
/// records as Jobs in progress, and for every position of the Kernel's
/// stack the record of its Task's level, active, for that Task, naming the
/// Task below as the Job it preempted.
fn seq_records_match(k: &SeqKernel, state: &crate::arch::cm4::plan::ArchState) {
    use crate::arch::KernelOps;
    let depth = k.depth();
    let mut active = 0;
    let mut l = 0;
    while l < 8 {
        if state.levels[l].active {
            active += 1;
        }
        l += 1;
    }
    assert!(active == depth);
    let mut i = 0;
    while i < depth {
        let t = k.stack_at(i);
        let rec = state.levels[k.job_info(t).priority as usize];
        assert!(rec.active && rec.task == t);
        assert!(rec.previous_task == if i == 0 { 0xFF } else { k.stack_at(i - 1) });
        i += 1;
    }
}

/// What a stub does with an unwind plan: unwinds the named level, then
/// every further level the Rust half names.
fn seq_unwind(k: &mut SeqKernel, hw: &mut SeqHw, state: &mut crate::arch::cm4::plan::ArchState, hop: u32) {
    let mut n = hop;
    let mut rounds = 0;
    while n != 0 && rounds < 8 {
        let ((_, _, _, next), safe) = crate::arch::cm4::entry::epilogue_core(k, hw, &SeqTables, state, n);
        assert!(!safe);
        n = next;
        rounds += 1;
    }
    assert!(n == 0);
}

/// The review of the audit fixes (2026-11-13), `entry::arch_matches` as a
/// bounded check (the predicate is an open assumption of the proofs): on
/// the demo's configuration, after each of five Kernel entries with their
/// unwinds, Kernel_Arch's active level records are exactly those of the
/// Kernel's preemption stack. The sequence is the audit's lead 2.1: the
/// time base releases `sampler` and it is dispatched; the time base
/// releases `log`; `log`'s dispatch charges `sampler` past its Budget; the
/// time base's entry at `sampler`'s Budget instant finds it preempted; the
/// time base's entry at `log`'s Budget instant ends `log` and, in the same
/// entry, `sampler` (two Budget events in one timer entry). The stub then
/// unwinds `log`'s level and, by the chain over
/// `LevelRecord::previous_task`, `sampler`'s. With that chain's condition
/// reverted the last check fails: one record stays active with no Job in
/// progress (shown on 2026-11-13).
#[kani::proof]
#[kani::unwind(25)]
fn arch_records_match_the_stack_over_kernel_entries() {
    use crate::arch::cm4::entry::{irq_entry_core, start_core, timer_core};
    use crate::arch::KernelOps;
    let (log, sampler): (u8, u8) = (3, 6);
    let mut cfg = crate::logic::kani_harness::demo_config();
    cfg.tasks[0].offset = 200_000;
    cfg.tasks[2].offset = 400_000;
    cfg.tasks[5].offset = 300_000;
    cfg.tasks[log as usize].offset = 12_480;
    let no_window = Interval { base: 0, size_log2: 5 };
    let part = |code: u32, ram: u32| crate::logic::mpu::PartitionLayout {
        code: Interval { base: code, size_log2: 15 },
        code_srd: 0,
        ram: Interval { base: ram, size_log2: 15 },
        ram_srd: 0,
        stack_size: SEQ_STACK,
        windows: [no_window; 5],
        window_count: 0,
        windows_ro: [false; 5],
    };
    let layout = crate::logic::mpu::Layout {
        kernel_flash: Interval { base: 0x0000_0000, size_log2: 16 },
        shared_code: Interval { base: 0x0001_0000, size_log2: 16 },
        kernel_ram: Interval { base: 0x2000_0000, size_log2: 15 },
        kernel_windows: [Interval { base: 0xE000_0000, size_log2: 20 }, no_window, no_window, no_window, no_window, no_window, no_window, no_window],
        kernel_window_count: 1,
        partitions: [part(0x0002_0000, SEQ_RAM[0]), part(0x0002_8000, SEQ_RAM[1]), part(0x0003_0000, SEQ_RAM[2])],
    };
    let mut k: SeqKernel = crate::arch::Bound { cfg, kernel: crate::arch::placeholder_kernel(), layout, views: [crate::arch::placeholder_view(), crate::arch::placeholder_view(), crate::arch::placeholder_view()] };
    assert!(k.validate());
    k.init();
    let mut hw = SeqHw { now: 0, cycles: 0 };
    let mut state = crate::arch::cm4::plan::ArchState::new();
    // The stack floors as `boot_core` sets them.
    state.floor[0] = SEQ_RAM[0] + SEQ_STACK;
    state.floor[1] = SEQ_RAM[1] + SEQ_STACK;
    state.floor[2] = SEQ_RAM[2] + SEQ_STACK;
    state.booted = true;
    assert!(!start_core(&mut k, &mut hw, &SeqTables, &mut state));
    seq_records_match(&k, &state);
    // Entry 1: the time base at instant 0 releases `sampler`; its dispatch from the idle activity.
    let (hop, safe) = timer_core(&mut k, &mut hw, &SeqTables, &mut state);
    assert!(hop == 0 && !safe);
    let (plan_s, safe) = irq_entry_core(&mut k, &mut hw, &SeqTables, &mut state, 36 + sampler as u32, 0, 0, 0xFFFF_FFF9);
    assert!(!safe && plan_s.psp != 0 && k.top() == Some(sampler));
    seq_records_match(&k, &state);
    // Entry 2: the time base at 12,480 releases `log`.
    hw.now = 12_480;
    hw.cycles = 12_480;
    let (hop, safe) = timer_core(&mut k, &mut hw, &SeqTables, &mut state);
    assert!(hop == 0 && !safe);
    seq_records_match(&k, &state);
    // Entry 3: `log`'s dispatch, 30 cycles later, preempts `sampler` on its own stack.
    hw.cycles = 12_510;
    let (plan_l, safe) = irq_entry_core(&mut k, &mut hw, &SeqTables, &mut state, 36 + log as u32, plan_s.psp, 1, 0xFFFF_FFFD);
    assert!(!safe && plan_l.psp != 0 && k.top() == Some(log) && k.depth() == 2);
    seq_records_match(&k, &state);
    // Entry 4: `sampler`'s Budget instant, with `sampler` preempted.
    hw.now = 12_520;
    hw.cycles = 12_520;
    let (hop, safe) = timer_core(&mut k, &mut hw, &SeqTables, &mut state);
    assert!(hop == 0 && !safe && k.depth() == 2);
    seq_records_match(&k, &state);
    // Entry 5: `log`'s Budget instant: both Jobs are ended by this one entry.
    hw.now = 162_480;
    hw.cycles = 162_530;
    let (hop, safe) = timer_core(&mut k, &mut hw, &SeqTables, &mut state);
    assert!(!safe && hop != 0 && k.depth() == 0);
    seq_unwind(&mut k, &mut hw, &mut state, hop);
    seq_records_match(&k, &state);
}

/// Audit finding 4 (2026-11-13), R45.2: `svc_clear_pending` clears
/// SHCSR.SVCALLPENDED (bit 15) and writes every other bit of the register
/// back as read, the active bits (0 to 11) and the enable bits (16 to 18)
/// among them, for any register contents; with the bit already clear it
/// writes nothing. `svc_of_ended_job`, the one call the timer, interrupt and
/// fault shells make after their core, clears the bit exactly when the
/// entry left fewer Jobs in progress than it found.
#[kani::proof]
#[kani::unwind(48)]
fn svc_clear_pending_clears_only_the_pending_bit() {
    use crate::arch::KernelOps;
    sim::reset();
    let shcsr: u32 = kani::any();
    sim::write(addr::SHCSR, shcsr);
    let before = sim::write_count();
    hw::svc_clear_pending();
    let after = sim::read(addr::SHCSR);
    assert!(after & (1 << 15) == 0);
    assert!(after | (1 << 15) == shcsr | (1 << 15));
    assert!(after & 0x0007_0FFF == shcsr & 0x0007_0FFF);
    if shcsr & (1 << 15) == 0 {
        assert!(sim::write_count() == before && after == shcsr);
    } else {
        assert!(sim::write_count() == before + 1);
    }
    // The shells' call: an entry that ended a Job, and one that did not.
    let no_window = Interval { base: 0, size_log2: 5 };
    let part = |code: u32, ram: u32| crate::logic::mpu::PartitionLayout {
        code: Interval { base: code, size_log2: 15 },
        code_srd: 0,
        ram: Interval { base: ram, size_log2: 15 },
        ram_srd: 0,
        stack_size: SEQ_STACK,
        windows: [no_window; 5],
        window_count: 0,
        windows_ro: [false; 5],
    };
    let layout = crate::logic::mpu::Layout {
        kernel_flash: Interval { base: 0x0000_0000, size_log2: 16 },
        shared_code: Interval { base: 0x0001_0000, size_log2: 16 },
        kernel_ram: Interval { base: 0x2000_0000, size_log2: 15 },
        kernel_windows: [Interval { base: 0xE000_0000, size_log2: 20 }, no_window, no_window, no_window, no_window, no_window, no_window, no_window],
        kernel_window_count: 1,
        partitions: [part(0x0002_0000, SEQ_RAM[0]), part(0x0002_8000, SEQ_RAM[1]), part(0x0003_0000, SEQ_RAM[2])],
    };
    let mut k: SeqKernel = crate::arch::Bound { cfg: crate::logic::kani_harness::demo_config(), kernel: crate::arch::placeholder_kernel(), layout, views: [crate::arch::placeholder_view(), crate::arch::placeholder_view(), crate::arch::placeholder_view()] };
    assert!(k.validate());
    k.init();
    assert!(k.depth() == 0);
    let pending: u32 = kani::any();
    sim::write(addr::SHCSR, pending);
    let before = sim::write_count();
    // Same depth as at the entry's start: the pending SVC is the executing Job's and stays.
    crate::arch::cm4::svc_of_ended_job(&k, 0);
    assert!(sim::read(addr::SHCSR) == pending && sim::write_count() == before);
    // One Job fewer than at the entry's start: the pending SVC was the ended Job's.
    crate::arch::cm4::svc_of_ended_job(&k, 1);
    assert!(sim::read(addr::SHCSR) == pending & !(1 << 15));
}
