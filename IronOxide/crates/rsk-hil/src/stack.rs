//! Stack painting (Verification_Plan 8.5, R37): the Link_Checker's bound
//! of every stack region against the deepest use a run leaves in it.
//!
//! The rig paints, the firmware does not: after the Application is
//! flashed, one probe command fills every stack region with the run's
//! pattern (a RAM download of one image holding all the regions) and
//! resets the board in the same session. The command is one because the
//! core runs between probe commands (measured 2026-11-16: DHCSR without
//! S_HALT after a download, and a region painted by one command written
//! into by the Application before the next), so a paint spread over
//! several commands is a paint of a running system. The Kernel's boot
//! zeroes `.bss` and copies `.data` only, so a word of a stack region
//! keeps the pattern until a frame is written over it. Before the
//! configuration's operating duration ends the regions are read: the used
//! depth of a region is the distance from its top to the lowest word that
//! no longer holds the pattern. The pattern's low half differs per run
//! (a stale paint of an earlier run cannot stand in for this one's), and
//! a region with no painted word left, or none written, fails. The binary
//! is the configuration's HIL build as every other suite measures it;
//! nothing is added to it.
//!
//! What the figure is: a lower bound on the deepest use (a frame may
//! reserve words it never writes, and a written word may equal the
//! pattern), so the judgement is one-sided: a used depth above the
//! Link_Checker's bound refutes the bound; a depth below it does not prove
//! it.

use std::path::Path;
use std::time::Duration;

use crate::board::Target;
use crate::config::Rig;

/// The paint's fixed half: with any low half no address of the Target's
/// map and no small integer.
pub const PATTERN: u32 = 0xC5A5_0000;

/// The pattern of a run: [`PATTERN`] with the run's 16-bit nonce.
pub fn pattern(nonce: u16) -> u32 {
    PATTERN | nonce as u32
}

/// One stack region with the Link_Checker's worst case.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub name: String,
    /// The lowest address of the region and the address past its top.
    pub base: u32,
    pub top: u32,
    /// The Link_Checker's bound, in bytes from the top.
    pub bound: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Measured {
    pub region: Region,
    /// Bytes from the top to the lowest word without the pattern.
    pub used: u32,
    /// Words below that one that hold the pattern, and words in all.
    pub untouched_words: u32,
    pub words: u32,
}

pub struct Inputs<'a> {
    pub rig: &'a Rig,
    pub target_elf: &'a Path,
    pub stimulus_elf: &'a Path,
    pub work_dir: &'a Path,
    /// How long the Application runs before the regions are read (within
    /// its operating duration: the read is of a running system).
    pub run_secs: u64,
    pub regions: &'a [Region],
    pub flash_stimulus: bool,
    pub configuration: &'a str,
    pub binary_hash: &'a str,
    /// The run's pattern ([`pattern`]).
    pub pattern: u32,
}

#[derive(Debug, Clone)]
pub struct Step {
    pub name: String,
    pub ok: bool,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct Outcome {
    pub steps: Vec<Step>,
    pub commands: Vec<String>,
    pub measured: Vec<Measured>,
}

impl Outcome {
    pub fn ok(&self) -> bool {
        !self.steps.is_empty() && self.steps.iter().all(|s| s.ok)
    }
    fn step(&mut self, name: &str, ok: bool, detail: impl Into<String>) {
        self.steps.push(Step { name: name.to_string(), ok, detail: detail.into() });
    }
}

/// The bytes of a painted region of `len` bytes (a multiple of 4).
pub fn paint(pattern: u32, len: usize) -> Vec<u8> {
    pattern.to_le_bytes().iter().copied().cycle().take(len).collect()
}

/// One Intel HEX image painting every region (extended linear address
/// records; 16 data bytes a record at most, none crossing a 64 KiB line).
pub fn paint_image(pattern: u32, regions: &[Region]) -> String {
    fn record(kind: u8, addr: u16, data: &[u8]) -> String {
        let mut bytes = vec![data.len() as u8, (addr >> 8) as u8, addr as u8, kind];
        bytes.extend_from_slice(data);
        let sum = bytes.iter().fold(0u8, |a, b| a.wrapping_add(*b));
        let mut line = String::from(":");
        for b in bytes.iter().chain(std::iter::once(&sum.wrapping_neg())) {
            line.push_str(&format!("{b:02X}"));
        }
        line.push('\n');
        line
    }
    let word = pattern.to_le_bytes();
    let mut out = String::new();
    let mut high: Option<u16> = None;
    for r in regions {
        let mut a = r.base;
        while a < r.top {
            let n = (r.top - a).min(16).min(0x1_0000 - (a & 0xFFFF));
            if high != Some((a >> 16) as u16) {
                high = Some((a >> 16) as u16);
                out.push_str(&record(4, 0, &((a >> 16) as u16).to_be_bytes()));
            }
            let data: Vec<u8> = (0..n).map(|i| word[((a - r.base + i) % 4) as usize]).collect();
            out.push_str(&record(0, a as u16, &data));
            a += n;
        }
    }
    out.push_str(&record(1, 0, &[]));
    out
}

/// The used depth of a region read back as `bytes` (lowest address
/// first): the bytes from the lowest word that does not hold the pattern
/// to the top, and the number of words below it.
pub fn high_water(pattern: u32, bytes: &[u8]) -> (u32, u32) {
    let words = bytes.chunks_exact(4).map(|c| u32::from_le_bytes([c[0], c[1], c[2], c[3]]));
    let untouched = words.take_while(|w| *w == pattern).count();
    ((bytes.len() - untouched * 4) as u32, untouched as u32)
}

pub fn run(inputs: &Inputs<'_>) -> Outcome {
    let rig = inputs.rig;
    let mut out = Outcome::default();
    let _ = std::fs::create_dir_all(inputs.work_dir);
    let scratch = inputs.work_dir.join("readback.bin");
    if inputs.regions.is_empty() {
        out.step("stack regions", false, "the Link_Checker reported no stack region for this build");
        return out;
    }
    if let Some(r) = inputs.regions.iter().find(|r| r.top <= r.base || (r.top - r.base) % 4 != 0 || r.base % 4 != 0) {
        out.step("stack regions", false, format!("region `{}` is not a word-aligned range: {:#x}..{:#x}", r.name, r.base, r.top));
        return out;
    }
    let stimulus = Target { chip: &rig.chip, serial: &rig.stimulus.serial };
    if inputs.flash_stimulus {
        match stimulus.flash(inputs.stimulus_elf).and_then(|f| stimulus.reset().map(|r| (f, r))) {
            Ok((f, r)) => {
                out.commands.push(f.command);
                out.commands.push(r.command);
                out.step("stimulus running", true, format!("flashed in {:.1} s and reset", f.seconds));
            }
            Err(e) => {
                out.step("stimulus running", false, e);
                return out;
            }
        }
    }
    let target = Target { chip: &rig.chip, serial: &rig.target.serial };
    match target.flash(inputs.target_elf) {
        Ok(f) => {
            out.commands.push(f.command);
            out.step("target flashed", true, format!("in {:.1} s", f.seconds));
        }
        Err(e) => {
            out.step("target flashed", false, e);
            return out;
        }
    }
    // Paint and reset in one probe session.
    let image = inputs.work_dir.join("paint.hex");
    if let Err(e) = std::fs::write(&image, paint_image(inputs.pattern, inputs.regions)) {
        out.step("painted and reset", false, format!("cannot write {}: {e}", image.display()));
        return out;
    }
    match target.download_ram_and_reset(&image) {
        Ok(command) => {
            out.commands.push(command);
            out.step(
                "painted and reset",
                true,
                format!(
                    "{} regions filled with {:#010x} and the board reset by the same command; read after {} s: {}",
                    inputs.regions.len(),
                    inputs.pattern,
                    inputs.run_secs,
                    inputs.regions.iter().map(|r| format!("{} {:#x}..{:#x}", r.name, r.base, r.top)).collect::<Vec<_>>().join(", ")
                ),
            );
        }
        Err(e) => {
            out.step("painted and reset", false, e);
            return out;
        }
    }
    std::thread::sleep(Duration::from_secs(inputs.run_secs));
    // The Application ran: its Kernel's release record counts Releases.
    match target.read_stats(inputs.target_elf, &scratch) {
        Ok(Some(st)) => out.step("the Application ran", st.total() > 0, format!("{} periodic Releases recorded when the regions were read", st.total())),
        Ok(None) => out.step("the Application ran", false, "the build has no release record"),
        Err(e) => out.step("the Application ran", false, e),
    }
    // What the run exercised: the Kernel's events by kind, from its mirror.
    // Informative: a fault-injecting configuration is told from an idle one
    // by the responses and restarts counted here, since no host was
    // attached to take the suite's own report.
    if let Ok(Some(ev)) = target.read_events(inputs.target_elf, &scratch) {
        let mut by_kind: std::collections::BTreeMap<u8, usize> = std::collections::BTreeMap::new();
        for ((kind, _), n) in ev.counts() {
            *by_kind.entry(kind).or_default() += n;
        }
        let kinds: Vec<String> = by_kind.iter().map(|(k, n)| format!("{} {n}", crate::events::kind_name(*k))).collect();
        out.step("events of the run", true, format!("{} mirrored in all; the held ones by kind: {}", ev.written, if kinds.is_empty() { "none".to_string() } else { kinds.join(", ") }));
    }
    for r in inputs.regions {
        let len = (r.top - r.base) as usize;
        match target.read(r.base, len, &scratch) {
            Ok(bytes) => {
                let (used, untouched) = high_water(inputs.pattern, &bytes);
                out.measured.push(Measured { region: r.clone(), used, untouched_words: untouched, words: (len / 4) as u32 });
            }
            Err(e) => {
                out.step("regions read", false, e);
                return out;
            }
        }
    }
    let _ = std::fs::remove_file(&scratch);
    out.step("regions read", true, format!("{} regions", out.measured.len()));
    for m in out.measured.clone() {
        let name = format!("stack of `{}` within the Link_Checker's bound (R37)", m.region.name);
        if m.used == 0 {
            out.step(&name, false, "no word of the region lost the pattern: the region is not the one the code uses");
        } else if m.untouched_words == 0 {
            out.step(&name, false, format!("the region's lowest word does not hold this run's pattern ({} bytes): the paint did not take, an overflow, or the region is not a stack alone", m.used));
        } else {
            out.step(&name, m.used <= m.region.bound, format!("{} bytes used of a bound of {} ({} available)", m.used, m.region.bound, m.region.top - m.region.base));
        }
    }
    out
}

pub fn render(inputs: &Inputs<'_>, out: &Outcome) -> String {
    let mut s = String::new();
    s.push_str(&format!("# Stack painting: `{}`\n\n", inputs.configuration));
    s.push_str(&format!("Binary: `{}`. Verdict: **{}**.\n\n", inputs.binary_hash, if out.ok() { "pass" } else { "FAIL" }));
    s.push_str("The rig fills every stack region with the run's pattern and resets the board in one probe command, lets the Application run, and reads the regions back: the used depth is the distance from a region's top to its lowest word without the pattern. A lower bound on the deepest use, judged against the Link_Checker's bound (R37).\n\n");
    s.push_str("| Step | Verdict | Detail |\n|---|---|---|\n");
    for st in &out.steps {
        s.push_str(&format!("| {} | {} | {} |\n", st.name, if st.ok { "pass" } else { "FAIL" }, st.detail.replace('\n', " ").replace('|', "/")));
    }
    if !out.measured.is_empty() {
        s.push_str("\n| Region | Range | Available | Link_Checker bound | Used | Used of the bound |\n|---|---|---|---|---|---|\n");
        for m in &out.measured {
            let pct = if m.region.bound > 0 { format!("{:.0} %", m.used as f64 * 100.0 / m.region.bound as f64) } else { "-".to_string() };
            s.push_str(&format!("| {} | {:#x}..{:#x} | {} | {} | {} | {} |\n", m.region.name, m.region.base, m.region.top, m.region.top - m.region.base, m.region.bound, m.used, pct));
        }
    }
    s.push_str("\nCommands:\n\n");
    for c in &out.commands {
        s.push_str(&format!("- `{c}`\n"));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R37: the used depth of a painted region is the distance from its
    /// top to the lowest word that lost the pattern.
    #[test]
    fn r37_high_water_is_measured_from_the_lowest_written_word() {
        let p = pattern(0x3C96);
        let mut bytes = paint(p, 64);
        assert_eq!(high_water(p, &bytes), (0, 16));
        // Another run's paint is not this run's.
        assert_eq!(high_water(pattern(0x3C97), &bytes), (64, 0));
        bytes[60..64].copy_from_slice(&1u32.to_le_bytes());
        assert_eq!(high_water(p, &bytes), (4, 15));
        // A deeper write with painted words left above it still counts from the deepest.
        bytes[20..24].copy_from_slice(&0u32.to_le_bytes());
        assert_eq!(high_water(p, &bytes), (44, 5));
        bytes[0] ^= 1;
        assert_eq!(high_water(p, &bytes), (64, 0));
    }

    /// The paint image holds every region's bytes at its addresses, each
    /// record's checksum zeroing its sum.
    #[test]
    fn r37_paint_image_covers_every_region() {
        let regions = [
            Region { name: "kernel".into(), base: 0x2000_FFF4, top: 0x2001_0008, bound: 8 },
            Region { name: "part0".into(), base: 0x2001_8000, top: 0x2001_8020, bound: 8 },
        ];
        let p = pattern(0x1234);
        let text = paint_image(p, &regions);
        let mut memory = std::collections::BTreeMap::new();
        let mut high = 0u32;
        let mut ended = false;
        for line in text.lines() {
            assert!(!ended);
            let raw: Vec<u8> = (1..line.len()).step_by(2).map(|i| u8::from_str_radix(&line[i..i + 2], 16).unwrap()).collect();
            assert_eq!(raw.iter().fold(0u8, |a, b| a.wrapping_add(*b)), 0, "{line}");
            let (n, addr, kind) = (raw[0] as usize, u32::from(raw[1]) << 8 | u32::from(raw[2]), raw[3]);
            assert_eq!(raw.len(), n + 5);
            match kind {
                0 => {
                    assert!(addr as usize + n <= 0x1_0000);
                    for (i, b) in raw[4..4 + n].iter().enumerate() {
                        assert!(memory.insert((high << 16) + addr + i as u32, *b).is_none());
                    }
                }
                1 => ended = true,
                4 => high = u32::from(raw[4]) << 8 | u32::from(raw[5]),
                k => panic!("record kind {k}"),
            }
        }
        assert!(ended);
        assert_eq!(memory.len(), 0x14 + 0x20);
        for r in &regions {
            let bytes: Vec<u8> = (r.base..r.top).map(|a| memory[&a]).collect();
            assert_eq!(bytes, paint(p, (r.top - r.base) as usize));
        }
    }
}
