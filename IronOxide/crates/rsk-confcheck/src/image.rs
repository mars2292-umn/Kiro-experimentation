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

/// The text of a version symbol of the Kernel (R6.7: `RSK_KERNEL_VERSION`,
/// `RSK_PROFILE_VERSION`, `RSK_PROFILE_VARIANT`), a byte string padded with
/// zeros; `None` when the binary has no such symbol.
pub fn version_symbol(elf_bytes: &[u8], name: &str) -> Result<Option<String>, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    let symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?;
    let Some(sym) = symbols.iter().find(|s| s.name == name && s.is_object()) else { return Ok(None) };
    let bytes = elf.read_at(sym.address(), sym.size as usize).map_err(|e| format!("binary: {name}: {e}"))?;
    let end = bytes.iter().position(|b| *b == 0).unwrap_or(bytes.len());
    String::from_utf8(bytes[..end].to_vec()).map(Some).map_err(|_| format!("binary: {name} is not text"))
}

/// Locates and reads `RSK_CONFIG_IMAGE` and `RSK_CONFIG_CHECKSUM`.
pub fn extract(elf_bytes: &[u8]) -> Result<Image, String> {
    extract_named(elf_bytes, "RSK_CONFIG_IMAGE", "RSK_CONFIG_CHECKSUM")
}

/// The content of `RSK_PPI` (docs/pacemaker PM33.4): the channel table the
/// Kernel writes from, three words per channel; `None` when the binary has
/// no such symbol.
pub fn ppi_symbol(elf_bytes: &[u8]) -> Result<Option<Vec<u8>>, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    let symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?;
    let Some(sym) = symbols.iter().find(|s| s.name == "RSK_PPI" && s.is_object()) else { return Ok(None) };
    elf.read_at(sym.address(), sym.size as usize).map(Some).map_err(|e| format!("binary: RSK_PPI: {e}"))
}

/// Locates and reads `RSK_ENDPOINT_IMAGE` and `RSK_ENDPOINT_CHECKSUM`
/// (task 21.2), absent from binaries built before Endpoints existed.
pub fn extract_endpoints(elf_bytes: &[u8]) -> Result<Option<Image>, String> {
    match extract_named(elf_bytes, "RSK_ENDPOINT_IMAGE", "RSK_ENDPOINT_CHECKSUM") {
        Ok(i) => Ok(Some(i)),
        Err(e) if e.contains("RSK_ENDPOINT_IMAGE") => Ok(None),
        Err(e) => Err(e),
    }
}

/// The address and size of the Endpoint storage static of the generated
/// system crate (`<application>_system::ENDPOINT_STORAGE`, found by the
/// path in its mangled name: the static is not exported), `None` when the
/// binary has no such symbol. More than one match is an unusable input.
///
/// The system crate is any crate whose name ends in `_system`
/// (`rsk_system` for the P1 demo). Until 2026-11-24 only `rsk_system` was
/// looked for, and the first other Application with Endpoints (the
/// PACEMAKER experiment's pulse generator, `pacemaker_system`) was
/// reported as having no storage symbol.
pub fn endpoint_storage_symbol(elf_bytes: &[u8]) -> Result<Option<(u32, u32)>, String> {
    let elf = rsk_elf::Elf::parse(elf_bytes.to_vec()).map_err(|e| format!("binary: {e}"))?;
    let symbols = elf.symbols().map_err(|e| format!("binary: {e}"))?;
    let mut found = symbols.iter().filter(|s| s.is_object() && s.name.contains("_system16ENDPOINT_STORAGE"));
    match (found.next(), found.next()) {
        (None, _) => Ok(None),
        (Some(s), None) => Ok(Some((s.address(), s.size))),
        (Some(_), Some(_)) => Err("binary: more than one system crate's ENDPOINT_STORAGE symbol".to_string()),
    }
}

/// Decodes the Endpoint image: the count, the entries, the storage bytes
/// (`image_v::decode_endpoint_image`, proven to accept exactly the
/// encoding of what it returns).
pub fn decode_endpoints(bytes: &[u8]) -> Result<(Vec<EndpointCfg>, u32), String> {
    crate::image_v::decode_endpoint_image(bytes).map_err(|e| refusal("endpoint image", e))
}

/// Locates and reads `RSK_LAYOUT_IMAGE` and `RSK_LAYOUT_CHECKSUM`.
pub fn extract_layout(elf_bytes: &[u8]) -> Result<Image, String> {
    extract_named(elf_bytes, "RSK_LAYOUT_IMAGE", "RSK_LAYOUT_CHECKSUM")
}

/// Decodes the `LAYOUT` image (`rsk_kernel::arch::layout_crc` order;
/// `image_v::decode_layout_image`, proven to accept exactly the encoding
/// of what it returns).
pub fn decode_layout(bytes: &[u8]) -> Result<LayoutImg, String> {
    crate::image_v::decode_layout_image(bytes).map_err(|e| refusal("layout image", e))
}

/// A decoder's refusal, in words.
fn refusal(image: &str, e: DecodeError) -> String {
    match e {
        DecodeError::TooShort => "image too short".to_string(),
        DecodeError::Counts => format!("{image}: implausible table sizes"),
        DecodeError::Flag(at) => format!("{image}: the flag byte at offset {at} is neither 0 nor 1"),
        DecodeError::Trailing(n) => format!("{image}: {n} trailing bytes after the tables"),
    }
}

/// CRC-32 (IEEE 802.3, reflected, init and final XOR all ones) over `bytes`:
/// `image_v::crc32`, proven to compute `image_v::crc32_of` (2026-11-22).
pub fn crc32(bytes: &[u8]) -> u32 {
    crate::image_v::crc32(bytes)
}

pub use crate::image_v::{DecodeError, EndpointCfg, IntervalImg, LayoutImg, PartitionCfg, PartitionLayoutImg, ResourceCfg, Tables, TaskCfg, MAX_KERNEL_WINDOWS, MAX_ROLES, MAX_WINDOWS};

/// Decodes the image in the Kernel's byte order (`config_crc`): NT, NR, NP
/// as u32; per Task priority, partition, kind, offset, period, deadline
/// (u64), budget (u32), mit_policy, fpu; per Resource ceiling, partition;
/// the access matrix Task-major; per Partition level, fault, overrun,
/// deadline responses, mit_threshold (u32); operating_duration (u64); the
/// three slot tables; slot_count. The decoding is `image_v::decode_tables`,
/// proven to accept exactly the encoding of the tables it returns (since
/// 2026-11-21 a flag byte other than 0 or 1 is refused); this function
/// words its refusals.
pub fn decode(bytes: &[u8]) -> Result<Tables, String> {
    crate::image_v::decode_tables(bytes).map_err(|e| refusal("image", e))
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
        // R26.4 (2026-11-21): the image is the encoding of the decoded
        // tables and of no others: with any one byte changed it is not.
        assert!(crate::image_v::image_is(&img, &t));
        for at in 0..img.len() {
            let mut other = img.clone();
            other[at] ^= 1;
            assert!(!crate::image_v::image_is(&other, &t), "byte {at} does not matter to the comparison");
        }
        // A flag byte is 0 or 1 (the Task's floating-point flag, byte 44).
        let mut flag = img.clone();
        flag[44] = 2;
        assert!(decode(&flag).unwrap_err().contains("flag byte at offset 44"));
        img.push(0);
        assert!(decode(&img).unwrap_err().contains("trailing"));
    }

    /// R16.7, R26.4: a `LAYOUT` image and an Endpoint image are the
    /// encodings of what they decode to and of nothing else: every byte
    /// matters to the whole-image comparison.
    #[test]
    fn every_byte_of_a_layout_and_an_endpoint_image_matters() {
        // Three Kernel intervals, the window count, eight window slots, one
        // Partition: two intervals, the stack size, the window count, five
        // window slots (the last byte of each a flag).
        let mut layout: Vec<u8> = (0..18u8).collect();
        layout.push(2);
        layout.extend((0..40u8).map(|b| b.wrapping_mul(7)));
        layout.extend_from_slice(&1u32.to_le_bytes());
        layout.extend((0..17u8).map(|b| b.wrapping_mul(3)));
        for w in 0..5u8 {
            layout.extend_from_slice(&[w, 1, 2, 3, 5, w % 2]);
        }
        let l = decode_layout(&layout).unwrap();
        assert_eq!((l.kernel_windows.len(), l.partitions[0].windows.len()), (MAX_KERNEL_WINDOWS, MAX_WINDOWS));
        assert!(crate::image_v::layout_image_is(&layout, &l));
        for at in 0..layout.len() {
            let mut other = layout.clone();
            other[at] ^= 1;
            assert!(!crate::image_v::layout_image_is(&other, &l), "layout byte {at} does not matter");
        }

        // One Endpoint: capacity, payload, roles, four Tasks, four
        // Partitions, four release flags, the storage offset, sixteen
        // validation flags; then the storage bytes.
        let mut ep = 1u32.to_le_bytes().to_vec();
        ep.extend_from_slice(&[2, 8, 0, 2, 0, 1, 255, 255, 0, 1, 255, 255, 1, 0, 0, 0]);
        ep.extend_from_slice(&0u32.to_le_bytes());
        ep.extend((0..16u8).map(|b| b % 2));
        ep.extend_from_slice(&64u32.to_le_bytes());
        let (table, storage) = decode_endpoints(&ep).unwrap();
        assert_eq!((table[0].payload, table[0].validates[0][1], storage), (8, true, 64));
        assert!(crate::image_v::endpoint_image_is(&ep, &table, storage));
        for at in 0..ep.len() {
            let mut other = ep.clone();
            other[at] ^= 1;
            assert!(!crate::image_v::endpoint_image_is(&other, &table, storage), "endpoint byte {at} does not matter");
        }
    }
}
