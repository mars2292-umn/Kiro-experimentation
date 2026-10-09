//! The MPU layout (DD-07, R20.4, R24.5, Property 12): one code and one RAM
//! region per Partition, power-of-two sized and aligned, placed largest
//! first after the Kernel's reservations, with the waste of a non-power-of-
//! two request trimmed by subregion disables; peripheral windows grouped
//! into 32 KiB blocks with subregion disables for the slots the Partition
//! does not own. A layout that needs more than the Target's regions, or
//! that does not fit the memories, is rejected naming the Partition.
//!
//! The numbers produced here are the `rsk_kernel::logic::mpu::Layout` that
//! the system crate embeds; the Kernel validates them again at boot with
//! the verified `Layout::validate` (R12.3), and `logic::mpu::view` (proven,
//! Property 12) derives the MPU registers from them.

use crate::decl::{Declaration, Diagnostic, ItemRef};
use crate::target::Target;

/// The maximum peripheral windows per Partition (8 regions: shared code,
/// Partition code, Partition RAM, and 5 windows).
pub const MAX_WINDOWS: usize = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Region {
    pub base: u32,
    pub size_log2: u8,
    /// Subregion-disable bits (bit s disables subregion s).
    pub srd: u8,
    /// Bytes usable from `base` (the enabled subregions).
    pub usable: u32,
}

impl Region {
    pub fn size(&self) -> u64 {
        1u64 << self.size_log2
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Window {
    pub base: u32,
    pub size_log2: u8,
    pub srd: u8,
    pub peripherals: Vec<String>,
    /// Read-only for the Partition (EasyDMA-capable peripherals, DD-08).
    pub read_only: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionRegions {
    pub name: String,
    pub code: Region,
    pub ram: Region,
    pub stack: u32,
    pub windows: Vec<Window>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Plan {
    pub kernel_flash: Region,
    pub shared_code: Region,
    pub kernel_ram: Region,
    /// In declaration order.
    pub partitions: Vec<PartitionRegions>,
}

fn log2_ceil(bytes: u64, min_log2: u8) -> u8 {
    let mut k = min_log2;
    while (1u64 << k) < bytes {
        k += 1;
    }
    k
}

/// A region of at least `bytes` usable bytes: the smallest power of two
/// that holds them, with the unused top subregions disabled when the
/// region has subregions (256 bytes or more).
fn region_for(bytes: u64, min_log2: u8) -> (u8, u8, u64) {
    let size_log2 = log2_ceil(bytes, min_log2);
    let size = 1u64 << size_log2;
    if size_log2 < 8 {
        return (size_log2, 0, size);
    }
    let sub = size / 8;
    let used = bytes.div_ceil(sub).max(1);
    let mut srd = 0u8;
    for s in used..8 {
        srd |= 1 << s;
    }
    (size_log2, srd, used * sub)
}

fn fixed(base: u32, bytes: u32) -> Region {
    Region {
        base,
        size_log2: log2_ceil(bytes as u64, 5),
        srd: 0,
        usable: bytes,
    }
}

/// Places the regions (R24.5). Partitions are placed largest first, ties
/// by name, so that the result does not depend on the declaration order
/// (Property 13) while each Partition keeps its declared identity.
pub fn plan(decl: &Declaration, target: &Target) -> Result<Plan, Vec<Diagnostic>> {
    let mut errors = Vec::new();
    let kernel_flash = fixed(target.flash.base, target.kernel_flash);
    let shared_code = fixed(target.flash.base + target.kernel_flash, target.shared_code);
    let kernel_ram = fixed(target.ram.base, target.kernel_ram);

    let mut order: Vec<usize> = (0..decl.partitions.len()).collect();
    order.sort_by(|&a, &b| {
        let pa = &decl.partitions[a];
        let pb = &decl.partitions[b];
        pb.code.cmp(&pa.code).then(pb.ram.cmp(&pa.ram)).then(pa.name.cmp(&pb.name))
    });

    let mut code_cursor = (target.flash.base + target.kernel_flash + target.shared_code) as u64;
    let code_end = target.flash.base as u64 + target.flash.size as u64;
    let mut ram_cursor = (target.ram.base + target.kernel_ram) as u64;
    let ram_end = target.ram.base as u64 + target.ram.size as u64;
    let mut placed: Vec<Option<PartitionRegions>> = vec![None; decl.partitions.len()];

    for &i in &order {
        let p = &decl.partitions[i];
        let (code_log2, code_srd, code_usable) = region_for(p.code, 5);
        let (ram_log2, ram_srd, ram_usable) = region_for(p.ram, 8);
        let code_size = 1u64 << code_log2;
        let ram_size = 1u64 << ram_log2;
        let code_base = code_cursor.div_ceil(code_size) * code_size;
        let ram_base = ram_cursor.div_ceil(ram_size) * ram_size;
        if code_base + code_size > code_end {
            errors.push(Diagnostic::new(
                "R20.4",
                ItemRef::PartitionField(p.name.clone(), "code".into()),
                format!(
                    "the code region ({} bytes, placed as a {}-byte aligned region at 0x{code_base:08x}) does not fit the Target's {} ({} bytes)",
                    p.code, code_size, target.flash.name, target.flash.size
                ),
            ));
        }
        if ram_base + ram_size > ram_end {
            errors.push(Diagnostic::new(
                "R20.4",
                ItemRef::PartitionField(p.name.clone(), "ram".into()),
                format!(
                    "the RAM region ({} bytes, placed as a {}-byte aligned region at 0x{ram_base:08x}) does not fit the Target's {} ({} bytes)",
                    p.ram, ram_size, target.ram.name, target.ram.size
                ),
            ));
        }
        code_cursor = code_base + code_size;
        ram_cursor = ram_base + ram_size;

        // Peripheral windows: one 4 KiB region per owned peripheral (no
        // subregion grouping yet: a grouped block would need per-window
        // subregion bits in `PartitionLayout`, which the verified view
        // function does not take; ORQ-02 keeps grouping as an optimization).
        let mut names = p.peripherals.clone();
        names.sort();
        let mut split: Vec<Window> = Vec::new();
        for name in &names {
            let Some(per) = target.peripheral(name) else { continue };
            split.push(Window {
                base: per.window_base,
                size_log2: 12,
                srd: 0,
                peripherals: vec![name.clone()],
                read_only: per.easy_dma,
            });
        }
        if split.len() > MAX_WINDOWS {
            errors.push(Diagnostic::new(
                "R20.4",
                ItemRef::PartitionField(p.name.clone(), "peripherals".into()),
                format!(
                    "the owned peripherals need {} MPU windows; at most {MAX_WINDOWS} fit the Target's {} regions (ORQ-02)",
                    split.len(),
                    target.mpu_regions
                ),
            ));
        }
        placed[i] = Some(PartitionRegions {
            name: p.name.clone(),
            code: Region {
                base: code_base as u32,
                size_log2: code_log2,
                srd: code_srd,
                usable: code_usable as u32,
            },
            ram: Region {
                base: ram_base as u32,
                size_log2: ram_log2,
                srd: ram_srd,
                usable: ram_usable as u32,
            },
            stack: p.stack as u32,
            windows: split,
        });
    }
    if !errors.is_empty() {
        return Err(errors);
    }
    Ok(Plan {
        kernel_flash,
        shared_code,
        kernel_ram,
        partitions: placed.into_iter().map(|p| p.expect("placed")).collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn regions_round_up_to_powers_of_two_and_trim_with_subregions() {
        assert_eq!(region_for(32 * 1024, 5), (15, 0, 32 * 1024));
        // 40 KiB -> 64 KiB region, 8 KiB subregions, 5 used, top 3 disabled.
        assert_eq!(region_for(40 * 1024, 8), (16, 0b1110_0000, 40 * 1024));
        // 100 bytes of code -> 128-byte region, no subregions below 256 bytes.
        assert_eq!(region_for(100, 5), (7, 0, 128));
        // RAM minimum is 256 bytes.
        assert_eq!(region_for(10, 8), (8, 0b1111_1110, 32));
    }
}
