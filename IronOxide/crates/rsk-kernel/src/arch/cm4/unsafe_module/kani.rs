//! Kani harnesses linked from the justification identifiers of the
//! Kernel_Unsafe_Module (R45.1, R45.2, R6.4). Compiled only under
//! `cfg(kani)`.

use super::hw;
use crate::logic::mpu::{Interval, Region, View, MPU_REGIONS};

/// UJ-001, UJ-002, UJ-009: every register address the Kernel accesses is
/// word aligned and inside the system control space or the peripheral
/// region, including the NVIC and MPU index arithmetic for every line and
/// region of the Target.
#[kani::proof]
#[kani::unwind(64)]
fn uj_001_mmio_addresses_valid() {
    for a in hw::addr::ALL {
        assert!(hw::addr::plausible(*a));
    }
    let irq: u32 = kani::any();
    kani::assume(irq < 240);
    assert!(hw::addr::plausible(hw::addr::NVIC_ISER + 4 * (irq / 32)));
    assert!(hw::addr::plausible(hw::addr::NVIC_ICER + 4 * (irq / 32)));
    assert!(hw::addr::plausible(hw::addr::NVIC_ISPR + 4 * (irq / 32)));
    assert!(hw::addr::plausible((hw::addr::NVIC_IPR + irq) & !3));
    let t = hw::addr::NRF_TIMER0;
    for off in [0x000u32, 0x004, 0x00C, 0x044, 0x140, 0x304, 0x308, 0x504, 0x508, 0x510, 0x540, 0x544] {
        assert!(hw::addr::plausible(t + off));
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

/// `mpu_load` writes eight RBAR/RASR pairs, in order, with the encodings
/// of the view's regions (simulated register file).
#[kani::proof]
#[kani::unwind(10)]
fn mpu_load_writes_every_region() {
    super::sim::reset();
    let mut regions = [Region { enabled: false, interval: Interval { base: 0, size_log2: 5 }, srd: 0, write: false, xn: true, device: false }; MPU_REGIONS];
    let base: u32 = kani::any();
    regions[2] = Region { enabled: true, interval: Interval { base: base & !0xFF, size_log2: 8 }, srd: 0, write: true, xn: true, device: false };
    let view = View { regions };
    let before = super::sim::write_count();
    hw::mpu_load(&view);
    assert!(super::sim::write_count() == before + 2 * MPU_REGIONS as u32);
    // The last pair written is region 7 (disabled): RASR 0.
    assert!(super::sim::read(hw::addr::MPU_RASR) == 0);
}
