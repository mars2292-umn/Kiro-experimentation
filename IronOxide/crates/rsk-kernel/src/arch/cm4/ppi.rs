//! Fixed PPI channels (docs/pacemaker Requirement 33; 2026-11-22).
//!
//! The PPI of the nRF52840 is Kernel-owned (DD-08): `boot_core` disables
//! every channel and its checks read the enable register back as zero. An
//! Application may declare fixed channels, each from one event register to
//! one task register with an optional fork, all of peripherals of one
//! Partition (PM33.2, the Generator's rule). The Generator records them in
//! Generated_Config (`System::PPI`) under the configuration checksum, which
//! for a configuration with channels is the CRC-32 of the Kernel's tables
//! continued over the channel table (`System::PPI_CHECKSUM`,
//! `rsk_gen::derive::ppi_bytes`).
//!
//! [`ppi_boot`] is the whole of the Kernel's dealing with them: once, after
//! the boot checks have passed and before the first Task is released, it
//! recomputes that checksum over the tables and the channel table in
//! memory (PM33.4, R12.3) and, only if it is the recorded one, writes each
//! channel's end-point registers and then the enable bits (PM33.3). It
//! takes the register file it writes (`drivers::Regs`), so the contract
//! below is proven against the register file's specification and the same
//! code runs on the memory-mapped registers.
//!
//! The Kernel's boot core (`entry::boot_core`) is not changed by this: a
//! configuration without channels boots exactly as before.
#![forbid(unsafe_code)]

use super::drivers::Regs;
#[cfg(verus_keep_ghost)]
use super::drivers::{with_mem, write_step, RegState};
use crate::arch::bound::Crc32;
use vstd::prelude::*;

verus! {

/// PPI.CHENSET (nRF52840 PS 6.19): a written 1 enables its channel.
pub const CHENSET: u32 = 0x4001_F504;
/// PPI.CH[0].EEP; channel `i`'s is 8 bytes further per channel.
pub const CH0_EEP: u32 = 0x4001_F510;
/// PPI.CH[0].TEP.
pub const CH0_TEP: u32 = 0x4001_F514;
/// PPI.FORK[0].TEP; channel `i`'s is 4 bytes further per channel.
pub const FORK0_TEP: u32 = 0x4001_F910;
/// The programmable channels of the nRF52840: CH[0] to CH[19].
pub const MAX_CHANNELS: usize = 20;

/// The address of channel `i`'s event end-point register.
pub open spec fn eep(i: int) -> u32 {
    (CH0_EEP + 8 * i) as u32
}

/// The address of channel `i`'s task end-point register.
pub open spec fn tep(i: int) -> u32 {
    (CH0_TEP + 8 * i) as u32
}

/// The address of channel `i`'s fork task end-point register.
pub open spec fn fork(i: int) -> u32 {
    (FORK0_TEP + 4 * i) as u32
}

/// The enable bits of channels 0 to `n` - 1.
pub open spec fn enable_bits(n: int) -> u32 {
    if n <= 0 {
        0
    } else {
        0xFFFF_FFFFu32 >> ((32 - n) as u32)
    }
}

/// The register file with the first `n` channels of `t` connected: each
/// channel's event, task and fork end-point registers hold the table's
/// row, and nothing else has changed.
pub open spec fn connected(s: RegState, t: Seq<[u32; 3]>, n: int) -> RegState
    decreases n,
{
    if n <= 0 {
        s
    } else {
        with_mem(with_mem(with_mem(connected(s, t, n - 1), eep(n - 1), t[n - 1][0]), tep(n - 1), t[n - 1][1]), fork(n - 1), t[n - 1][2])
    }
}

/// **The Init write (PM33.3).** The register file after it: every channel
/// of the table connected, then the enable bits of exactly those channels
/// written to CHENSET.
pub open spec fn written(s: RegState, t: Seq<[u32; 3]>) -> RegState {
    with_mem(connected(s, t, t.len() as int), CHENSET, enable_bits(t.len() as int))
}

/// A write to a register of the PPI's window keeps the value and touches
/// nothing else: the window is none of the registers `write_step` treats
/// as set or clear registers.
proof fn lemma_ppi_write(s: RegState, a: u32, v: u32)
    requires 0x4001_F000 <= a < 0x4002_0000,
    ensures write_step(s, a, v) == with_mem(s, a, v),
{
}

/// Writes the channels of `table` and enables them.
pub fn ppi_write<R: Regs>(r: &mut R, table: &[[u32; 3]])
    requires table@.len() <= MAX_CHANNELS,
    ensures final(r).st() == written(old(r).st(), table@),
{
    let n = table.len();
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n, n == table@.len(), n <= MAX_CHANNELS,
            r.st() == connected(old(r).st(), table@, i as int),
        decreases n - i,
    {
        let row = table[i];
        let step = i as u32;
        proof {
            lemma_ppi_write(r.st(), eep(i as int), row[0]);
            lemma_ppi_write(with_mem(r.st(), eep(i as int), row[0]), tep(i as int), row[1]);
            lemma_ppi_write(with_mem(with_mem(r.st(), eep(i as int), row[0]), tep(i as int), row[1]), fork(i as int), row[2]);
        }
        r.write(CH0_EEP + 8 * step, row[0]);
        r.write(CH0_TEP + 8 * step, row[1]);
        r.write(FORK0_TEP + 4 * step, row[2]);
        i += 1;
    }
    let bits: u32 = if n == 0 { 0 } else { 0xFFFF_FFFFu32 >> (32 - n as u32) };
    proof {
        lemma_ppi_write(r.st(), CHENSET, bits);
    }
    r.write(CHENSET, bits);
}

/// The configuration checksum of a configuration with channels: the CRC-32
/// of the Kernel's tables (`tables_sum`, finished) continued over the
/// channel table's image, which is the count as one byte and then per
/// channel its number as one byte and its three register addresses, each
/// as four bytes, least significant first (`rsk_gen::derive::ppi_bytes`).
pub fn ppi_checksum(tables_sum: u32, table: &[[u32; 3]]) -> u32
    requires table@.len() <= MAX_CHANNELS,
{
    let n = table.len();
    // A finished CRC-32 is the complement of the running value.
    let mut c = Crc32(!tables_sum).byte(n as u8);
    let mut i: usize = 0;
    while i < n
        invariant i <= n, n == table@.len(), n <= MAX_CHANNELS,
        decreases n - i,
    {
        let row = table[i];
        c = c.byte(i as u8).u32(row[0]).u32(row[1]).u32(row[2]);
        i += 1;
    }
    c.finish()
}

/// **The whole of the Kernel's use of the PPI after the boot checks.**
/// With no channel declared, nothing is read or written. Otherwise the
/// table is written exactly when it has no more rows than the PPI has
/// channels and the checksum over the Kernel's tables and the table is the
/// recorded one; if not, no register is written and the answer is no (the
/// caller records a failed boot check and enters the safe state).
pub fn ppi_boot<R: Regs>(r: &mut R, table: &[[u32; 3]], tables_sum: u32, recorded: u32) -> (ok: bool)
    ensures
        table@.len() == 0 ==> ok,
        ok && table@.len() > 0 ==> table@.len() <= MAX_CHANNELS && final(r).st() == written(old(r).st(), table@),
        !ok || table@.len() == 0 ==> final(r).st() == old(r).st(),
{
    let n = table.len();
    if n == 0 {
        return true;
    }
    if n > MAX_CHANNELS {
        return false;
    }
    if ppi_checksum(tables_sum, table) != recorded {
        return false;
    }
    ppi_write(r, table);
    true
}

} // verus!
