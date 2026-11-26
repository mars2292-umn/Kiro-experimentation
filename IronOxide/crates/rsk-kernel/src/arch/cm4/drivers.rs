//! The register drivers of Kernel_Arch as functions of a register file
//! (2026-11-18): the bodies behind the `Action`s whose step changes a
//! state the hardware keeps (the NVIC's enabled and pending lines and
//! priority bytes, the nRF52840's TIMER0 compare, the boot configuration
//! bits). Each takes the register file it reads and writes (`Regs`), so
//! that the same code runs on the Target, where the file is the
//! memory-mapped registers (`Mmio`, whose three methods are the volatile
//! accesses of UJ-001, UJ-002 and UJ-009), is proven here against the
//! register file's specification (`RegState`, `write_step`), and is read
//! by Aeneas as a function of a register model (`proofs/aeneas/lean/Hw`),
//! where the same statements are proven on the code as compiled. The
//! functions of `unsafe_module::hw` that the Kernel calls are these
//! applied to `Mmio`.
//!
//! No `unsafe`, no assembly: the barriers and the system-register
//! instructions stay in `unsafe_module::hw`.
#![forbid(unsafe_code)]

use vstd::prelude::*;

/// The addresses of the registers the drivers write (re-exported by
/// `unsafe_module::hw::addr`, the Kernel's whole register list).
pub mod addr {
    use vstd::prelude::*;

    verus! {

    pub const ACTLR: u32 = 0xE000_E008;
    pub const NVIC_ISER: u32 = 0xE000_E100;
    pub const NVIC_ICER: u32 = 0xE000_E180;
    pub const NVIC_ISPR: u32 = 0xE000_E200;
    pub const NVIC_ICPR: u32 = 0xE000_E280;
    pub const NVIC_IPR: u32 = 0xE000_E400;
    pub const AIRCR: u32 = 0xE000_ED0C;
    pub const CCR: u32 = 0xE000_ED14;
    pub const SHPR1: u32 = 0xE000_ED18;
    pub const SHPR2: u32 = 0xE000_ED1C;
    pub const SHPR3: u32 = 0xE000_ED20;
    pub const SHCSR: u32 = 0xE000_ED24;
    pub const CPACR: u32 = 0xE000_ED88;
    pub const MPU_CTRL: u32 = 0xE000_ED94;
    pub const DEMCR: u32 = 0xE000_EDFC;
    pub const FPCCR: u32 = 0xE000_EF34;
    pub const FPDSCR: u32 = 0xE000_EF3C;
    pub const DWT_CTRL: u32 = 0xE000_1000;
    pub const DWT_CYCCNT: u32 = 0xE000_1004;
    /// nRF52840 TIMER0 (PS 6.30).
    pub const NRF_TIMER0: u32 = 0x4000_8000;
    /// The NVIC lines the Armv7-M register arrays hold (B3.4.3).
    pub const NVIC_LINES: u32 = 496;

    } // verus!
}

verus! {

/// What the register file holds: the last value written to each plain
/// register, the NVIC's enabled and pending lines by word of 32 lines, the
/// time base's interrupt enable, and the NVIC's priority bytes.
pub struct RegState {
    pub mem: Map<u32, u32>,
    pub enabled: Map<int, u32>,
    pub pending: Map<int, u32>,
    pub inten: u32,
    pub prio: Map<u32, u8>,
}

/// The word of lines a set or clear register at `a` addresses, for the 16
/// registers from `base`.
pub open spec fn nvic_word(a: u32, base: u32) -> Option<int> {
    if base <= a && a < base + 64 && (a - base) % 4 == 0 {
        Some((a - base) / 4)
    } else {
        None
    }
}

/// A word write (HW-05; nRF52840 PS 6.30): the NVIC's set registers set
/// the written bits of their word and its clear registers clear them, for
/// the enabled and for the pending lines; TIMER0's INTENSET and INTENCLR do
/// the same for its interrupt enable; any other register keeps the value.
pub open spec fn write_step(s: RegState, a: u32, v: u32) -> RegState {
    if let Some(w) = nvic_word(a, addr::NVIC_ISER) {
        RegState { enabled: s.enabled.insert(w, s.enabled[w] | v), ..s }
    } else if let Some(w) = nvic_word(a, addr::NVIC_ICER) {
        RegState { enabled: s.enabled.insert(w, s.enabled[w] & !v), ..s }
    } else if let Some(w) = nvic_word(a, addr::NVIC_ISPR) {
        RegState { pending: s.pending.insert(w, s.pending[w] | v), ..s }
    } else if let Some(w) = nvic_word(a, addr::NVIC_ICPR) {
        RegState { pending: s.pending.insert(w, s.pending[w] & !v), ..s }
    } else if a == addr::NRF_TIMER0 + timer0::INTENSET {
        RegState { inten: s.inten | v, ..s }
    } else if a == addr::NRF_TIMER0 + timer0::INTENCLR {
        RegState { inten: s.inten & !v, ..s }
    } else {
        RegState { mem: s.mem.insert(a, v), ..s }
    }
}

/// A register file: word reads and writes, and the byte write of the
/// NVIC's priority array.
pub trait Regs {
    spec fn st(&self) -> RegState;

    fn read(&mut self, a: u32) -> (v: u32)
        ensures final(self).st() == old(self).st(), v == old(self).st().mem[a];

    fn write(&mut self, a: u32, v: u32)
        ensures final(self).st() == write_step(old(self).st(), a, v);

    fn write_u8(&mut self, a: u32, v: u8)
        ensures final(self).st() == (RegState { prio: old(self).st().prio.insert(a, v), ..old(self).st() });
}

/// The extension of the 32-bit counter to the 64-bit time base (DD-05):
/// the Kernel_Arch state that counts the counter's wraps.
///
/// What an extension returns for a counter reading and the state it
/// leaves are the two specification functions (2026-11-25), so that the
/// drivers below can say which reading they extended, as the Lean proofs
/// of the same functions do (`Hw/Model.lean`, `now_spec`): nothing is said
/// of the functions themselves here (the rule behind `Clock`'s is proven
/// on `ArchState::extend`, plan.rs).
pub trait Extend: Sized {
    spec fn reads(&self, low: u32) -> u64;

    spec fn then(&self, low: u32) -> Self;

    fn extend(&mut self, low: u32) -> (t: u64)
        ensures t == old(self).reads(low), *final(self) == old(self).then(low);
}

/// The memory-mapped registers as a register file. Trusted
/// (docs/proof_assumptions.toml): its three methods are the volatile
/// accesses of UJ-001, UJ-002 and UJ-009, and that the registers behave as
/// `write_step` says is the Hardware_Model's HW-05 and HW-12, tested on
/// the rig.
pub struct Mmio {
    pub model: Ghost<RegState>,
}

impl Mmio {
    pub fn new() -> Mmio {
        Mmio { model: Ghost(arbitrary()) }
    }
}

impl Regs for Mmio {
    open spec fn st(&self) -> RegState {
        self.model@
    }

    #[verifier::external_body]
    #[inline(always)]
    fn read(&mut self, a: u32) -> u32 {
        super::unsafe_module::hw::mmio_read(a)
    }

    #[verifier::external_body]
    #[inline(always)]
    fn write(&mut self, a: u32, v: u32) {
        super::unsafe_module::hw::mmio_write(a, v)
    }

    #[verifier::external_body]
    #[inline(always)]
    fn write_u8(&mut self, a: u32, v: u8) {
        super::unsafe_module::hw::mmio_write_u8(a, v)
    }
}

/// The Kernel_Arch state's count of the counter's wraps
/// (`unsafe_module::extend`).
/// Its two specification functions are uninterpreted functions of a ghost
/// value standing for that state: no property of them is assumed.
pub struct Clock {
    pub model: Ghost<int>,
}

pub uninterp spec fn clock_reads(m: int, low: u32) -> u64;

pub uninterp spec fn clock_then(m: int, low: u32) -> int;

impl Clock {
    pub fn new() -> Clock {
        Clock { model: Ghost(arbitrary()) }
    }
}

impl Extend for Clock {
    open spec fn reads(&self, low: u32) -> u64 {
        clock_reads(self.model@, low)
    }

    open spec fn then(&self, low: u32) -> Self {
        Clock { model: Ghost(clock_then(self.model@, low)) }
    }

    #[verifier::external_body]
    #[inline(always)]
    fn extend(&mut self, low: u32) -> u64 {
        super::unsafe_module::extend(low)
    }
}

/// The NVIC line of an exception number; none for the system exceptions
/// (below 16) and for numbers past the implemented lines.
pub open spec fn spec_irq_of(n: u16) -> Option<u32> {
    if n < 16 || n - 16 >= addr::NVIC_LINES {
        None
    } else {
        Some((n - 16) as u32)
    }
}

#[inline]
pub fn irq_of(n: u16) -> (r: Option<u32>)
    ensures r == spec_irq_of(n),
{
    if n < 16 || n as u32 - 16 >= addr::NVIC_LINES {
        None
    } else {
        Some(n as u32 - 16)
    }
}

/// The bit of a line in its word.
pub open spec fn line_mask(irq: u32) -> u32 {
    1u32 << (irq % 32)
}

/// The lines with line `irq` set, and with it cleared.
pub open spec fn set_line(m: Map<int, u32>, irq: u32) -> Map<int, u32> {
    m.insert((irq / 32) as int, m[(irq / 32) as int] | line_mask(irq))
}

pub open spec fn clear_line(m: Map<int, u32>, irq: u32) -> Map<int, u32> {
    m.insert((irq / 32) as int, m[(irq / 32) as int] & !line_mask(irq))
}

/// `Action::Enable`: a number that names a line sets that line's bit in
/// the enabled lines and changes nothing else; any other number changes
/// nothing.
#[inline]
pub fn nvic_enable<R: Regs>(r: &mut R, n: u16)
    ensures final(r).st() == match spec_irq_of(n) {
        None => old(r).st(),
        Some(irq) => RegState { enabled: set_line(old(r).st().enabled, irq), ..old(r).st() },
    },
{
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ISER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

/// `Action::Disable`.
#[inline]
pub fn nvic_disable<R: Regs>(r: &mut R, n: u16)
    ensures final(r).st() == match spec_irq_of(n) {
        None => old(r).st(),
        Some(irq) => RegState { enabled: clear_line(old(r).st().enabled, irq), ..old(r).st() },
    },
{
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ICER + 4 * (irq / 32), 1 << (irq % 32));
    }
}

/// `Action::Pend`.
#[inline]
pub fn nvic_set_pending<R: Regs>(r: &mut R, n: u16)
    ensures final(r).st() == match spec_irq_of(n) {
        None => old(r).st(),
        Some(irq) => RegState { pending: set_line(old(r).st().pending, irq), ..old(r).st() },
    },
{
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ISPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

/// `Action::ClearPending`.
#[inline]
pub fn nvic_clear_pending<R: Regs>(r: &mut R, n: u16)
    ensures final(r).st() == match spec_irq_of(n) {
        None => old(r).st(),
        Some(irq) => RegState { pending: clear_line(old(r).st().pending, irq), ..old(r).st() },
    },
{
    if let Some(irq) = irq_of(n) {
        r.write(addr::NVIC_ICPR + 4 * (irq / 32), 1 << (irq % 32));
    }
}

/// `Action::SetPriority`: the line's priority byte and nothing else.
#[inline]
pub fn nvic_set_priority<R: Regs>(r: &mut R, n: u16, encoded: u8)
    ensures final(r).st() == match spec_irq_of(n) {
        None => old(r).st(),
        Some(irq) => RegState { prio: old(r).st().prio.insert((addr::NVIC_IPR + irq) as u32, encoded), ..old(r).st() },
    },
{
    if let Some(irq) = irq_of(n) {
        r.write_u8(addr::NVIC_IPR + irq, encoded);
    }
}

/// The state with one plain register written.
pub open spec fn with_mem(s: RegState, a: u32, v: u32) -> RegState {
    RegState { mem: s.mem.insert(a, v), ..s }
}

/// AIRCR.PRIGROUP = 4: with 3 implemented bits (7:5) every bit is a group
/// bit (HW-01). VECTKEY 0x05FA is required for the write.
#[inline]
pub fn set_prigroup_all_preempt<R: Regs>(r: &mut R)
    ensures final(r).st() == with_mem(old(r).st(), addr::AIRCR,
        (0x05FAu32 << 16) | (old(r).st().mem[addr::AIRCR] & 0x0000_F8FF) | (4u32 << 8)),
{
    let v = r.read(addr::AIRCR) & 0x0000_F8FF;
    r.write(addr::AIRCR, (0x05FA << 16) | v | (4 << 8));
}

/// CCR.NONBASETHRDENA (HW-07). The Kernel keeps it 0: no Job executes
/// with an exception active (DD-01 as revised by the P1 spike).
#[inline]
pub fn set_nonbasethrdena<R: Regs>(r: &mut R, on: bool)
    ensures final(r).st() == with_mem(old(r).st(), addr::CCR,
        if on { old(r).st().mem[addr::CCR] | 1 } else { old(r).st().mem[addr::CCR] & !1u32 }),
{
    let v = r.read(addr::CCR);
    r.write(addr::CCR, if on { v | 1 } else { v & !1 });
}

/// ACTLR.DISDEFWBUF (HW-11, DD-12).
#[inline]
pub fn set_disdefwbuf<R: Regs>(r: &mut R, on: bool)
    ensures final(r).st() == with_mem(old(r).st(), addr::ACTLR,
        if on { old(r).st().mem[addr::ACTLR] | 2 } else { old(r).st().mem[addr::ACTLR] & !2u32 }),
{
    let v = r.read(addr::ACTLR);
    r.write(addr::ACTLR, if on { v | 2 } else { v & !2 });
}

/// MemManage, BusFault, UsageFault, and SVCall at the Kernel level (priority
/// 0) and enabled (SHCSR bits 16 to 18).
#[inline]
pub fn set_kernel_exception_priorities<R: Regs>(r: &mut R)
    ensures final(r).st() == with_mem(with_mem(with_mem(with_mem(old(r).st(), addr::SHPR1, 0), addr::SHPR2, 0), addr::SHPR3, 0),
        addr::SHCSR, old(r).st().mem[addr::SHCSR] | (1u32 << 16) | (1u32 << 17) | (1u32 << 18)),
{
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
pub fn fpu_configure_eager<R: Regs>(r: &mut R, flush_to_zero: bool)
    ensures final(r).st() == with_mem(with_mem(with_mem(old(r).st(),
            addr::FPCCR, (old(r).st().mem[addr::FPCCR] | (1u32 << 31)) & !(1u32 << 30)),
            addr::FPDSCR, if flush_to_zero { 1u32 << 24 } else { 0 }),
            addr::CPACR, old(r).st().mem[addr::CPACR] | (0xFu32 << 20)),
{
    let v = r.read(addr::FPCCR);
    r.write(addr::FPCCR, (v | (1 << 31)) & !(1 << 30));
    r.write(addr::FPDSCR, if flush_to_zero { 1 << 24 } else { 0 });
    let c = r.read(addr::CPACR) | (0xF << 20);
    r.write(addr::CPACR, c);
}

/// MPU_CTRL: ENABLE and PRIVDEFENA (HW-09). The barrier is the caller's.
#[inline]
pub fn mpu_enable_background<R: Regs>(r: &mut R)
    ensures final(r).st() == with_mem(old(r).st(), addr::MPU_CTRL, (1u32 << 2) | 1),
{
    r.write(addr::MPU_CTRL, (1 << 2) | 1);
}

/// DEMCR.TRCENA, the cycle counter cleared and enabled (HW-13).
#[inline]
pub fn dwt_enable<R: Regs>(r: &mut R)
    ensures final(r).st() == with_mem(with_mem(with_mem(old(r).st(),
            addr::DEMCR, old(r).st().mem[addr::DEMCR] | (1u32 << 24)),
            addr::DWT_CYCCNT, 0),
            addr::DWT_CTRL, old(r).st().mem[addr::DWT_CTRL] | 1),
{
    let v = r.read(addr::DEMCR);
    r.write(addr::DEMCR, v | (1 << 24));
    r.write(addr::DWT_CYCCNT, 0);
    let c = r.read(addr::DWT_CTRL);
    r.write(addr::DWT_CTRL, c | 1);
}

} // verus!

/// nRF52840 TIMER0 (PS 6.30): 32-bit mode at 16 MHz (prescaler 0), CC[0]
/// for the compare event, CAPTURE[1]/CC[1] to read the counter.
pub mod timer0 {
    use super::{addr, Extend, Regs};
    #[cfg(verus_keep_ghost)]
    use super::{clear_line, set_line, with_mem, RegState};
    use vstd::prelude::*;

    verus! {

    pub const TASKS_CAPTURE1: u32 = 0x044;
    pub const EVENTS_COMPARE0: u32 = 0x140;
    pub const INTENSET: u32 = 0x304;
    pub const INTENCLR: u32 = 0x308;
    pub const CC0: u32 = 0x540;
    pub const CC1: u32 = 0x544;
    /// TIMER0's interrupt line (nRF52840 IRQ 8), for a compare already due.
    pub const TIMER0_VECTOR: u16 = 16 + 8;
    /// The compare interrupt's bit in the interrupt-enable registers.
    pub open spec fn compare_int() -> u32 {
        1u32 << 16
    }

    /// The 64-bit time base: the counter captured and extended.
    #[inline]
    pub fn now<R: Regs, E: Extend>(r: &mut R, e: &mut E) -> (t: u64)
        ensures final(r).st() == with_mem(old(r).st(), (addr::NRF_TIMER0 + TASKS_CAPTURE1) as u32, 1),
            // The reading extended is the capture register's (2026-11-25).
            t == old(e).reads(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
            *final(e) == old(e).then(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
    {
        let t = addr::NRF_TIMER0;
        r.write(t + TASKS_CAPTURE1, 1);
        let low = r.read(t + CC1);
        e.extend(low)
    }

    /// The state after the writes of an arm, before its pending decision.
    pub open spec fn armed(s: RegState, at: u64) -> RegState {
        with_mem(
            RegState { inten: s.inten | compare_int(), ..with_mem(s, (addr::NRF_TIMER0 + CC0) as u32, at as u32) },
            (addr::NRF_TIMER0 + TASKS_CAPTURE1) as u32,
            1,
        )
    }

    /// Arms the compare for `at` and returns the time base captured after
    /// the write. The compare is absolute: an instant the counter has
    /// already passed matches only after the 32-bit wrap (268 s at 16 MHz),
    /// so a due or past instant pends the time-base interrupt instead; the
    /// handler serves every due slot by `now`, not by the event. Seen on
    /// the DK on 2026-10-29: the first release (instant 0) was armed after
    /// the counter started and no tick ever came, so the watchdog reset the
    /// chip. One capture after the write decides (HW-12, ASM-19).
    ///
    /// Proven: the compare register holds the instant's low word, the
    /// compare interrupt is enabled, and the time base's line is set
    /// pending exactly when the reading has reached the instant: armed, or
    /// pending.
    #[inline]
    pub fn arm<R: Regs, E: Extend>(r: &mut R, e: &mut E, at: u64) -> (post: u64)
        ensures final(r).st() == if post >= at {
            RegState { pending: set_line(old(r).st().pending, 8), ..armed(old(r).st(), at) }
        } else {
            armed(old(r).st(), at)
        },
            // The reading that decides is the capture register's, read once
            // after the writes and extended (2026-11-25).
            post == old(e).reads(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
            *final(e) == old(e).then(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
    {
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
    pub fn ack<R: Regs>(r: &mut R)
        ensures final(r).st() == with_mem(old(r).st(), (addr::NRF_TIMER0 + EVENTS_COMPARE0) as u32, 0),
    {
        let t = addr::NRF_TIMER0;
        r.write(t + EVENTS_COMPARE0, 0);
    }

    /// The state after a cancel.
    pub open spec fn cancelled(s: RegState) -> RegState {
        RegState {
            pending: clear_line(s.pending, 8),
            ..with_mem(RegState { inten: s.inten & !compare_int(), ..s }, (addr::NRF_TIMER0 + EVENTS_COMPARE0) as u32, 0)
        }
    }

    /// Cancels the compare that is armed or has matched (2026-11-15: the
    /// Budget event it was armed for is disarmed, Verification_Plan 7.3):
    /// the compare interrupt is disabled, its event cleared, and the
    /// pending state of TIMER0's line cleared in the NVIC. The event is
    /// read back before the NVIC write so that the line is low by then
    /// (the event register drives it).
    #[inline]
    pub fn cancel<R: Regs>(r: &mut R)
        ensures final(r).st() == cancelled(old(r).st()),
    {
        let t = addr::NRF_TIMER0;
        r.write(t + INTENCLR, 1 << 16);
        r.write(t + EVENTS_COMPARE0, 0);
        let _ = r.read(t + EVENTS_COMPARE0);
        super::nvic_clear_pending(r, TIMER0_VECTOR);
    }

    /// The state after the writes of a re-arm, before its pending decision.
    pub open spec fn rearmed(s: RegState, at: u64) -> RegState {
        let c = with_mem(
            with_mem(RegState { inten: s.inten & !compare_int(), ..s }, (addr::NRF_TIMER0 + CC0) as u32, at as u32),
            (addr::NRF_TIMER0 + EVENTS_COMPARE0) as u32,
            0,
        );
        with_mem(
            RegState { pending: clear_line(s.pending, 8), inten: (s.inten & !compare_int()) | compare_int(), ..c },
            (addr::NRF_TIMER0 + TASKS_CAPTURE1) as u32,
            1,
        )
    }

    /// Cancels the compare that is armed or has matched and arms it for
    /// `at`, returning the time base captured after the writes. The new
    /// compare value is written before the old event is cleared, so no
    /// match of the old value survives; a match of the new one that the
    /// clearing erased is caught by the last test, which pends the time
    /// base at once for an instant already reached (as `arm` does): no due
    /// event is lost.
    #[inline]
    pub fn rearm<R: Regs, E: Extend>(r: &mut R, e: &mut E, at: u64) -> (post: u64)
        ensures final(r).st() == if post >= at {
            RegState { pending: set_line(rearmed(old(r).st(), at).pending, 8), ..rearmed(old(r).st(), at) }
        } else {
            rearmed(old(r).st(), at)
        },
            post == old(e).reads(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
            *final(e) == old(e).then(old(r).st().mem[(addr::NRF_TIMER0 + CC1) as u32]),
    {
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

    } // verus!
}
