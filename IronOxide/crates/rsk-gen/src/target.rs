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
}

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
    Peripheral { name: "CLOCK", window_base: 0x4000_0000, window_size: 0x1000, irq: Some(0), kernel_owned: true, easy_dma: false },
    Peripheral { name: "RADIO", window_base: 0x4000_1000, window_size: 0x1000, irq: Some(1), kernel_owned: false, easy_dma: true },
    Peripheral { name: "UARTE0", window_base: 0x4000_2000, window_size: 0x1000, irq: Some(2), kernel_owned: false, easy_dma: true },
    Peripheral { name: "SPIM0", window_base: 0x4000_3000, window_size: 0x1000, irq: Some(3), kernel_owned: false, easy_dma: true },
    Peripheral { name: "SPIM1", window_base: 0x4000_4000, window_size: 0x1000, irq: Some(4), kernel_owned: false, easy_dma: true },
    Peripheral { name: "GPIOTE", window_base: 0x4000_6000, window_size: 0x1000, irq: Some(6), kernel_owned: false, easy_dma: false },
    Peripheral { name: "SAADC", window_base: 0x4000_7000, window_size: 0x1000, irq: Some(7), kernel_owned: false, easy_dma: true },
    Peripheral { name: "TIMER0", window_base: 0x4000_8000, window_size: 0x1000, irq: Some(8), kernel_owned: true, easy_dma: false },
    Peripheral { name: "TIMER1", window_base: 0x4000_9000, window_size: 0x1000, irq: Some(9), kernel_owned: false, easy_dma: false },
    Peripheral { name: "TIMER2", window_base: 0x4000_A000, window_size: 0x1000, irq: Some(10), kernel_owned: false, easy_dma: false },
    Peripheral { name: "RTC0", window_base: 0x4000_B000, window_size: 0x1000, irq: Some(11), kernel_owned: false, easy_dma: false },
    Peripheral { name: "WDT", window_base: 0x4001_0000, window_size: 0x1000, irq: Some(16), kernel_owned: true, easy_dma: false },
    Peripheral { name: "RTC1", window_base: 0x4001_1000, window_size: 0x1000, irq: Some(17), kernel_owned: false, easy_dma: false },
    Peripheral { name: "PWM0", window_base: 0x4001_C000, window_size: 0x1000, irq: Some(28), kernel_owned: false, easy_dma: true },
    Peripheral { name: "PPI", window_base: 0x4001_F000, window_size: 0x1000, irq: None, kernel_owned: true, easy_dma: false },
    Peripheral { name: "P0", window_base: 0x5000_0000, window_size: 0x1000, irq: None, kernel_owned: false, easy_dma: false },
];

/// nRF52840 (Cortex-M4F r0p1): 64 MHz CPU, TIMER0 at 16 MHz as the time
/// base (DD-05), 3 priority bits, 8 MPU regions, 48 interrupt lines, SWI/EGU
/// lines 20 to 25 as dispatch vectors.
pub const NRF52840: Target = Target {
    name: "nrf52840",
    model_name: "nrf52840",
    core_revision: "r0p1",
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
    Peripheral { name: "CMSDK_TIMER0", window_base: 0x4000_0000, window_size: 0x1000, irq: Some(8), kernel_owned: true, easy_dma: false },
    Peripheral { name: "CMSDK_TIMER1", window_base: 0x4000_1000, window_size: 0x1000, irq: Some(9), kernel_owned: true, easy_dma: false },
    Peripheral { name: "CMSDK_DUALTIMER", window_base: 0x4000_2000, window_size: 0x1000, irq: Some(10), kernel_owned: false, easy_dma: false },
    Peripheral { name: "CMSDK_UART0", window_base: 0x4000_4000, window_size: 0x1000, irq: Some(0), kernel_owned: false, easy_dma: false },
    Peripheral { name: "CMSDK_UART1", window_base: 0x4000_5000, window_size: 0x1000, irq: Some(2), kernel_owned: false, easy_dma: false },
    Peripheral { name: "CMSDK_GPIO0", window_base: 0x4001_0000, window_size: 0x1000, irq: None, kernel_owned: false, easy_dma: false },
];

/// QEMU `mps2-an386` (Cortex-M4 model): 25 MHz CPU and time base (CMSDK
/// TIMER0 free-running, TIMER1 compare), 8 priority bits in QEMU, 8 MPU
/// regions, 32 interrupt lines, lines 20 to 31 free for dispatch vectors.
/// Not the Target of the Profile (ASM-01): the pre-hardware spike board.
pub const QEMU_MPS2_AN386: Target = Target {
    name: "qemu_mps2_an386",
    model_name: "qemu-mps2-an386",
    core_revision: "qemu",
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
