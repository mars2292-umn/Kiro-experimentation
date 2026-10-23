//! Target descriptions (R21.1 "the Target and its hardware parameters",
//! R21.5, R12.1): the clocks, the priority bits, the MPU, the memory map,
//! the Kernel-owned peripherals (PR-29, R15.6), the interrupt lines that
//! peripherals use, and the free lines the Generator may assign as
//! dispatch vectors (DD-01).

/// One peripheral of the Target: its name, its register window (4 KiB
/// aligned, DD-07), and its interrupt line, if any.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Peripheral {
    pub name: &'static str,
    pub window_base: u32,
    pub window_size: u32,
    pub irq: Option<u16>,
    /// Owned by the Kernel (PR-29): the time base and the core peripherals.
    pub kernel_owned: bool,
    /// EasyDMA-capable: unprivileged access is read-only (DD-08).
    pub easy_dma: bool,
    /// The EasyDMA registers (DD-08), for EasyDMA-capable peripherals.
    pub dma: Option<&'static DmaRegs>,
}

/// The EasyDMA registers of a peripheral class, as offsets from its base
/// (nRF52840 PS): `channels` are (pointer, count) pairs (count 0 when the
/// peripheral has none), `stops` the stop task registers, `lists` the
/// list-mode registers (cleared at boot and never set, DD-08).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DmaRegs {
    pub channels: &'static [(u32, u32)],
    pub stops: &'static [u32],
    pub lists: &'static [u32],
}

/// RADIO (PS 6.20): PACKETPTR, length from PCNF1; DISABLE stops.
pub const RADIO_DMA: DmaRegs = DmaRegs { channels: &[(0x504, 0)], stops: &[0x010], lists: &[] };
/// UARTE (PS 6.34): RXD and TXD; STOPRX and STOPTX.
pub const UARTE_DMA: DmaRegs = DmaRegs { channels: &[(0x534, 0x538), (0x544, 0x548)], stops: &[0x014, 0x01C], lists: &[] };
/// SPIM (PS 6.26): RXD and TXD with LIST registers; STOP.
pub const SPIM_DMA: DmaRegs = DmaRegs { channels: &[(0x534, 0x538), (0x544, 0x548)], stops: &[0x014], lists: &[0x53C, 0x54C] };
/// SAADC (PS 6.23): RESULT; STOP.
pub const SAADC_DMA: DmaRegs = DmaRegs { channels: &[(0x62C, 0x630)], stops: &[0x004], lists: &[] };
/// PWM (PS 6.17): SEQ[0] and SEQ[1]; STOP.
pub const PWM_DMA: DmaRegs = DmaRegs { channels: &[(0x500, 0x504), (0x520, 0x524)], stops: &[0x004], lists: &[] };

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MemoryRegion {
    pub name: &'static str,
    pub base: u32,
    pub size: u32,
    pub flash: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Target {
    pub name: &'static str,
    /// For the Task_Model `target` field.
    pub model_name: &'static str,
    pub core_revision: &'static str,
    /// The CPUID word the boot checks expect (ASM-01, R12.1).
    pub cpuid: u32,
    /// Whether the high-frequency clock runs from the crystal (ASM-05, R12.1).
    pub hfclk_xtal: bool,
    pub cpu_hz: u64,
    pub tick_hz: u64,
    /// Implemented NVIC priority bits (ASM-03).
    pub priority_bits: u8,
    /// MPU regions (ASM-02, PAR-07).
    pub mpu_regions: u8,
    /// Number of interrupt lines of the NVIC.
    pub irq_count: u16,
    /// The Kernel time-base interrupt line.
    pub timer_irq: u16,
    /// Interrupt lines free for dispatch vectors, in assignment order.
    /// Strictly ascending, so that Task order is exception-number order
    /// (DD-03; `hw_model::dispatch_vectors_ok` assumes it).
    pub dispatch_irqs: &'static [u16],
    pub peripherals: &'static [Peripheral],
    pub flash: MemoryRegion,
    pub ram: MemoryRegion,
    /// Kernel flash and RAM reservations at the start of each memory.
    pub kernel_flash: u32,
    pub shared_code: u32,
    pub kernel_ram: u32,
}

const NRF52840_PERIPHERALS: &[Peripheral] = &[
    Peripheral { name: "CLOCK", window_base: 0x4000_0000, window_size: 0x1000, irq: Some(0), kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "RADIO", window_base: 0x4000_1000, window_size: 0x1000, irq: Some(1), kernel_owned: false, easy_dma: true, dma: Some(&RADIO_DMA) },
    Peripheral { name: "UARTE0", window_base: 0x4000_2000, window_size: 0x1000, irq: Some(2), kernel_owned: false, easy_dma: true, dma: Some(&UARTE_DMA) },
    Peripheral { name: "SPIM0", window_base: 0x4000_3000, window_size: 0x1000, irq: Some(3), kernel_owned: false, easy_dma: true, dma: Some(&SPIM_DMA) },
    Peripheral { name: "SPIM1", window_base: 0x4000_4000, window_size: 0x1000, irq: Some(4), kernel_owned: false, easy_dma: true, dma: Some(&SPIM_DMA) },
    Peripheral { name: "GPIOTE", window_base: 0x4000_6000, window_size: 0x1000, irq: Some(6), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "SAADC", window_base: 0x4000_7000, window_size: 0x1000, irq: Some(7), kernel_owned: false, easy_dma: true, dma: Some(&SAADC_DMA) },
    Peripheral { name: "TIMER0", window_base: 0x4000_8000, window_size: 0x1000, irq: Some(8), kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "TIMER1", window_base: 0x4000_9000, window_size: 0x1000, irq: Some(9), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "TIMER2", window_base: 0x4000_A000, window_size: 0x1000, irq: Some(10), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "RTC0", window_base: 0x4000_B000, window_size: 0x1000, irq: Some(11), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "WDT", window_base: 0x4001_0000, window_size: 0x1000, irq: Some(16), kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "RTC1", window_base: 0x4001_1000, window_size: 0x1000, irq: Some(17), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "PWM0", window_base: 0x4001_C000, window_size: 0x1000, irq: Some(28), kernel_owned: false, easy_dma: true, dma: Some(&PWM_DMA) },
    Peripheral { name: "PPI", window_base: 0x4001_F000, window_size: 0x1000, irq: None, kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "P0", window_base: 0x5000_0000, window_size: 0x1000, irq: None, kernel_owned: false, easy_dma: false, dma: None },
];

/// nRF52840 (Cortex-M4F r0p1): 64 MHz CPU, TIMER0 at 16 MHz as the time
/// base (DD-05), 3 priority bits, 8 MPU regions, 48 interrupt lines, SWI/EGU
/// lines 20 to 25 as dispatch vectors.
pub const NRF52840: Target = Target {
    name: "nrf52840",
    model_name: "nrf52840",
    core_revision: "r0p1",
    cpuid: 0x410F_C241,
    hfclk_xtal: true,
    cpu_hz: 64_000_000,
    tick_hz: 16_000_000,
    priority_bits: 3,
    mpu_regions: 8,
    irq_count: 48,
    timer_irq: 8,
    dispatch_irqs: &[20, 21, 22, 23, 24, 25, 40],
    peripherals: NRF52840_PERIPHERALS,
    flash: MemoryRegion { name: "FLASH", base: 0x0000_0000, size: 0x10_0000, flash: true },
    ram: MemoryRegion { name: "RAM", base: 0x2000_0000, size: 0x4_0000, flash: false },
    kernel_flash: 0x1_0000,
    shared_code: 0x1_0000,
    kernel_ram: 0x8000,
};

const MPS2_PERIPHERALS: &[Peripheral] = &[
    Peripheral { name: "CMSDK_TIMER0", window_base: 0x4000_0000, window_size: 0x1000, irq: Some(8), kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "CMSDK_TIMER1", window_base: 0x4000_1000, window_size: 0x1000, irq: Some(9), kernel_owned: true, easy_dma: false, dma: None },
    Peripheral { name: "CMSDK_DUALTIMER", window_base: 0x4000_2000, window_size: 0x1000, irq: Some(10), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "CMSDK_UART0", window_base: 0x4000_4000, window_size: 0x1000, irq: Some(0), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "CMSDK_UART1", window_base: 0x4000_5000, window_size: 0x1000, irq: Some(2), kernel_owned: false, easy_dma: false, dma: None },
    Peripheral { name: "CMSDK_GPIO0", window_base: 0x4001_0000, window_size: 0x1000, irq: None, kernel_owned: false, easy_dma: false, dma: None },
];

/// QEMU `mps2-an386` (Cortex-M4 model): 25 MHz CPU and time base (CMSDK
/// TIMER0 free-running, TIMER1 compare), 8 priority bits in QEMU, 8 MPU
/// regions, 32 interrupt lines, lines 20 to 31 free for dispatch vectors.
/// Not the Target of the Profile (ASM-01): the pre-hardware spike board.
pub const QEMU_MPS2_AN386: Target = Target {
    name: "qemu_mps2_an386",
    model_name: "qemu-mps2-an386",
    core_revision: "qemu",
    // QEMU's cortex-m4 model reports r0p0.
    cpuid: 0x410F_C240,
    hfclk_xtal: false,
    cpu_hz: 25_000_000,
    tick_hz: 25_000_000,
    priority_bits: 8,
    mpu_regions: 8,
    irq_count: 32,
    timer_irq: 9,
    dispatch_irqs: &[20, 21, 22, 23, 24, 25, 26],
    peripherals: MPS2_PERIPHERALS,
    flash: MemoryRegion { name: "CODE", base: 0x0000_0000, size: 0x40_0000, flash: true },
    ram: MemoryRegion { name: "RAM", base: 0x2000_0000, size: 0x40_0000, flash: false },
    kernel_flash: 0x1_0000,
    shared_code: 0x1_0000,
    kernel_ram: 0x8000,
};

pub const TARGETS: &[&Target] = &[&NRF52840, &QEMU_MPS2_AN386];

impl Target {
    pub fn by_name(name: &str) -> Option<&'static Target> {
        TARGETS.iter().copied().find(|t| t.name == name)
    }

    pub fn peripheral(&self, name: &str) -> Option<&'static Peripheral> {
        self.peripherals.iter().find(|p| p.name == name)
    }

    /// The Kernel feature of `rsk-kernel` that selects this board.
    pub fn kernel_feature(&self) -> &'static str {
        match self.name {
            "qemu_mps2_an386" => "board-qemu-mps2",
            _ => "board-nrf52840",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// DD-03 and `hw_model::dispatch_vectors_ok`: the Generator assigns
    /// dispatch vectors from `dispatch_irqs` in Task order, so the list must
    /// be strictly ascending for equal-Priority Tasks to start in
    /// declaration order.
    #[test]
    fn dispatch_irqs_ascend_for_dd03() {
        for t in TARGETS {
            assert!(
                t.dispatch_irqs.windows(2).all(|w| w[0] < w[1]),
                "{}: dispatch_irqs is not strictly ascending",
                t.name
            );
        }
    }
}
