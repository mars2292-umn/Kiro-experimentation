//! The `CONFIG` tables of the linked binary (R26.4): located through the
//! `RSK_CONFIG_IMAGE` and `RSK_CONFIG_CHECKSUM` symbols, decoded with the
//! Kernel's documented table encoding (`rsk_kernel::arch::bound::config_crc`
//! states the byte order; the kind, policy, and response codes are those of
//! `rsk_logic::logic`), and checksummed with CRC-32 (IEEE 802.3), the
//! Kernel's own choice (R12.3). Restated here, not imported: the Config_Checker
//! shares no code with the Generator or the Kernel (R26.1).

/// Kind codes (`rsk_logic::logic::KIND_*`).
pub const KIND_PERIODIC: u8 = 0;
pub const KIND_SPORADIC: u8 = 1;
/// MIT policy codes (`MIT_*`).
pub const MIT_DEFER: u8 = 0;
pub const MIT_DISCARD: u8 = 1;
/// Response codes (`RESP_*`).
pub const RESP_END_JOB: u8 = 0;
pub const RESP_RESTART_PARTITION: u8 = 1;
pub const RESP_STOP_PARTITION: u8 = 2;
pub const RESP_SAFE_STATE: u8 = 3;
pub const RESP_RECORD_ONLY: u8 = 4;
pub const RESP_RECORD_AND_CONTINUE: u8 = 5;
/// No release slot (`NO_SLOT`).
pub const NO_SLOT: u8 = 0xFF;

/// The response code of a declared response name.
pub fn response_code(name: &str) -> Option<u8> {
    Some(match name {
        "END_JOB" => RESP_END_JOB,
        "RESTART_PARTITION" => RESP_RESTART_PARTITION,
        "STOP_PARTITION" => RESP_STOP_PARTITION,
        "SAFE_STATE" => RESP_SAFE_STATE,
        "RECORD_ONLY" => RESP_RECORD_ONLY,
        "RECORD_AND_CONTINUE" => RESP_RECORD_AND_CONTINUE,
        _ => return None,
    })
}

/// The level code of a declared level (`A` = 0 ... `E` = 4).
pub fn level_code(name: &str) -> Option<u8> {
    Some(match name {
        "A" => 0,
        "B" => 1,
        "C" => 2,
        "D" => 3,
        "E" => 4,
        _ => return None,
    })
}

/// `MAX_WINDOWS` of the Kernel's `PartitionLayout`.
pub const MAX_WINDOWS: usize = 5;

/// An image as found in the binary.
pub struct Image {
    pub address: u32,
    pub bytes: Vec<u8>,
    pub stored_checksum: u32,
}

/// Locates and reads an image symbol and its checksum symbol.
pub fn extract_named(elf_bytes: &[u8], image_symbol: &str, checksum_symbol: &str) -> Result<Image, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    let symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?;
    let image = symbols
        .iter()
        .find(|s| s.name == image_symbol && s.is_object())
        .ok_or_else(|| format!("binary: no {image_symbol} symbol (is this a Flight_Build?)"))?;
    let checksum = symbols
        .iter()
        .find(|s| s.name == checksum_symbol && s.is_object())
        .ok_or_else(|| format!("binary: no {checksum_symbol} symbol"))?;
    let bytes = elf.read_at(image.address(), image.size as usize).map_err(|e| format!("binary: {image_symbol}: {e}"))?;
    let c = elf.read_at(checksum.address(), 4).map_err(|e| format!("binary: {checksum_symbol}: {e}"))?;
    Ok(Image { address: image.address(), bytes, stored_checksum: u32::from_le_bytes([c[0], c[1], c[2], c[3]]) })
}

/// Locates and reads `RSK_CONFIG_IMAGE` and `RSK_CONFIG_CHECKSUM`.
pub fn extract(elf_bytes: &[u8]) -> Result<Image, String> {
    extract_named(elf_bytes, "RSK_CONFIG_IMAGE", "RSK_CONFIG_CHECKSUM")
}

/// Locates and reads `RSK_LAYOUT_IMAGE` and `RSK_LAYOUT_CHECKSUM`.
pub fn extract_layout(elf_bytes: &[u8]) -> Result<Image, String> {
    extract_named(elf_bytes, "RSK_LAYOUT_IMAGE", "RSK_LAYOUT_CHECKSUM")
}

/// An interval of the `LAYOUT` image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IntervalImg {
    pub base: u32,
    pub size_log2: u8,
    pub srd: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionLayoutImg {
    pub code: IntervalImg,
    pub ram: IntervalImg,
    pub stack_size: u32,
    pub window_count: u8,
    /// `MAX_WINDOWS` slots, (base, size_log2); unused slots are (0, 5).
    pub windows: Vec<(u32, u8)>,
}

/// The decoded `LAYOUT` image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutImg {
    pub kernel_flash: IntervalImg,
    pub shared_code: IntervalImg,
    pub kernel_ram: IntervalImg,
    pub partitions: Vec<PartitionLayoutImg>,
}

/// Decodes the `LAYOUT` image (`rsk_kernel::arch::layout_crc` order).
pub fn decode_layout(bytes: &[u8]) -> Result<LayoutImg, String> {
    let mut r = Reader { bytes, pos: 0 };
    let interval = |r: &mut Reader| -> Result<IntervalImg, String> {
        Ok(IntervalImg { base: r.u32()?, size_log2: r.u8()?, srd: r.u8()? })
    };
    let kernel_flash = interval(&mut r)?;
    let shared_code = interval(&mut r)?;
    let kernel_ram = interval(&mut r)?;
    let np = r.u32()? as usize;
    if np > 255 {
        return Err(format!("layout image: implausible NP={np}"));
    }
    let mut partitions = Vec::with_capacity(np);
    for _ in 0..np {
        let code = interval(&mut r)?;
        let ram = interval(&mut r)?;
        let stack_size = r.u32()?;
        let window_count = r.u8()?;
        let mut windows = Vec::with_capacity(MAX_WINDOWS);
        for _ in 0..MAX_WINDOWS {
            windows.push((r.u32()?, r.u8()?));
        }
        partitions.push(PartitionLayoutImg { code, ram, stack_size, window_count, windows });
    }
    if r.pos != bytes.len() {
        return Err(format!("layout image: {} trailing bytes", bytes.len() - r.pos));
    }
    Ok(LayoutImg { kernel_flash, shared_code, kernel_ram, partitions })
}

/// CRC-32 (IEEE 802.3, reflected, init and final XOR all ones) over `bytes`.
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc: u32 = 0xFFFF_FFFF;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaskCfg {
    pub priority: u8,
    pub partition: u8,
    pub kind: u8,
    pub offset: u64,
    pub period: u64,
    pub deadline: u64,
    pub budget: u32,
    pub mit_policy: u8,
    pub fpu: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceCfg {
    pub ceiling: u8,
    pub partition: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PartitionCfg {
    pub level: u8,
    pub fault_response: u8,
    pub overrun_response: u8,
    pub deadline_response: u8,
    pub mit_threshold: u32,
}

/// The decoded `CONFIG` tables.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tables {
    pub tasks: Vec<TaskCfg>,
    pub resources: Vec<ResourceCfg>,
    /// `access[t][r]`
    pub access: Vec<Vec<bool>>,
    pub partitions: Vec<PartitionCfg>,
    pub operating_duration: u64,
    pub slot_release: Vec<u8>,
    pub slot_deadline: Vec<u8>,
    pub slot_budget: Vec<u8>,
    pub slot_count: u8,
}

struct Reader<'a> {
    bytes: &'a [u8],
    pos: usize,
}

impl Reader<'_> {
    fn u8(&mut self) -> Result<u8, String> {
        let b = *self.bytes.get(self.pos).ok_or("image too short")?;
        self.pos += 1;
        Ok(b)
    }
    fn u32(&mut self) -> Result<u32, String> {
        Ok(u32::from_le_bytes([self.u8()?, self.u8()?, self.u8()?, self.u8()?]))
    }
    fn u64(&mut self) -> Result<u64, String> {
        let lo = self.u32()? as u64;
        let hi = self.u32()? as u64;
        Ok(lo | (hi << 32))
    }
}

/// Decodes the image in the Kernel's byte order (`config_crc`): NT, NR, NP
/// as u32; per Task priority, partition, kind, offset, period, deadline
/// (u64), budget (u32), mit_policy, fpu; per Resource ceiling, partition;
/// the access matrix Task-major; per Partition level, fault, overrun,
/// deadline responses, mit_threshold (u32); operating_duration (u64); the
/// three slot tables; slot_count.
pub fn decode(bytes: &[u8]) -> Result<Tables, String> {
    let mut r = Reader { bytes, pos: 0 };
    let nt = r.u32()? as usize;
    let nr = r.u32()? as usize;
    let np = r.u32()? as usize;
    if nt > 255 || nr > 255 || np > 255 {
        return Err(format!("image: implausible table sizes NT={nt} NR={nr} NP={np}"));
    }
    let mut tasks = Vec::with_capacity(nt);
    for _ in 0..nt {
        tasks.push(TaskCfg {
            priority: r.u8()?,
            partition: r.u8()?,
            kind: r.u8()?,
            offset: r.u64()?,
            period: r.u64()?,
            deadline: r.u64()?,
            budget: r.u32()?,
            mit_policy: r.u8()?,
            fpu: r.u8()? != 0,
        });
    }
    let mut resources = Vec::with_capacity(nr);
    for _ in 0..nr {
        resources.push(ResourceCfg { ceiling: r.u8()?, partition: r.u8()? });
    }
    let mut access = Vec::with_capacity(nt);
    for _ in 0..nt {
        let mut row = Vec::with_capacity(nr);
        for _ in 0..nr {
            row.push(r.u8()? != 0);
        }
        access.push(row);
    }
    let mut partitions = Vec::with_capacity(np);
    for _ in 0..np {
        partitions.push(PartitionCfg {
            level: r.u8()?,
            fault_response: r.u8()?,
            overrun_response: r.u8()?,
            deadline_response: r.u8()?,
            mit_threshold: r.u32()?,
        });
    }
    let operating_duration = r.u64()?;
    let mut slot_release = Vec::with_capacity(nt);
    for _ in 0..nt {
        slot_release.push(r.u8()?);
    }
    let mut slot_deadline = Vec::with_capacity(nt);
    for _ in 0..nt {
        slot_deadline.push(r.u8()?);
    }
    let mut slot_budget = Vec::with_capacity(nt);
    for _ in 0..nt {
        slot_budget.push(r.u8()?);
    }
    let slot_count = r.u8()?;
    if r.pos != bytes.len() {
        return Err(format!("image: {} trailing bytes after the tables", bytes.len() - r.pos));
    }
    Ok(Tables { tasks, resources, access, partitions, operating_duration, slot_release, slot_deadline, slot_budget, slot_count })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_ieee_check_value() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    /// R26.4: the CONFIG tables are decoded from their binary image encoding,
    /// and an image with bytes beyond the tables is rejected.
    #[test]
    fn a_synthetic_image_decodes_and_rejects_trailing_bytes() {
        let mut img = Vec::new();
        for n in [1u32, 1, 1] {
            img.extend_from_slice(&n.to_le_bytes());
        }
        img.extend_from_slice(&[7, 0, KIND_PERIODIC]);
        img.extend_from_slice(&0u64.to_le_bytes());
        img.extend_from_slice(&250_000u64.to_le_bytes());
        img.extend_from_slice(&250_000u64.to_le_bytes());
        img.extend_from_slice(&25_000u32.to_le_bytes());
        img.extend_from_slice(&[MIT_DEFER, 0]);
        img.extend_from_slice(&[7, 0]);
        img.push(1);
        img.extend_from_slice(&[0, RESP_RESTART_PARTITION, RESP_RECORD_AND_CONTINUE, RESP_RECORD_ONLY]);
        img.extend_from_slice(&10u32.to_le_bytes());
        img.extend_from_slice(&5_000_000u64.to_le_bytes());
        img.extend_from_slice(&[0, 1, 2, 5]);
        let t = decode(&img).unwrap();
        assert_eq!(t.tasks[0].period, 250_000);
        assert_eq!(t.resources[0].ceiling, 7);
        assert!(t.access[0][0]);
        assert_eq!(t.partitions[0].mit_threshold, 10);
        assert_eq!(t.slot_count, 5);
        img.push(0);
        assert!(decode(&img).unwrap_err().contains("trailing"));
    }
}
