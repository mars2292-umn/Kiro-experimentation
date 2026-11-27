//! rsk-elf: a minimal reader for 32-bit little-endian ELF files, the format
//! of every Target binary (`thumbv7em-none-eabihf`).
//!
//! It serves three tools: the Build_System hashes the loadable segments of a
//! Flight_Build for the Build_Manifest (R33.3), the Link_Checker reads
//! symbols and code (R37, PR-17, PR-18), and the Config_Checker extracts the
//! `CONFIG` tables from the binary (R26.4). All three need the same few
//! structures, so one dependency-free host crate provides them. Nothing here
//! is linked into a Flight_Build.
//!
//! Scope: ELF32 little-endian only; section headers, program headers,
//! symbol tables, and byte access by file offset or by virtual address. Any
//! other class, encoding, or malformed structure is an error, never a panic.
//!
//! Since 2026-11-26 everything read from the file's bytes is read by
//! `reader_v`, verified with Verus (`cargo verus verify -p rsk-elf`) and, as
//! compiled, in Lean (`proofs/aeneas/lean/Tools/ElfProofs.lean`): no read
//! out of range for any bytes, the symbol lookup by name, and the bytes at
//! an address. This file adds the names as text, the wording of a refusal,
//! and the loadable image for the binary hash (ordinary Rust, tested).
#![forbid(unsafe_code)]
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports))]

use std::fmt;

pub mod reader_v;

pub use reader_v::{Header, Segment, EM_ARM, PT_LOAD, SHF_ALLOC, SHF_EXECINSTR, SHF_WRITE, SHT_NOBITS, SHT_SYMTAB};
use reader_v::{AtErr, ElfErr, RawSym, SecHdr};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section {
    pub index: usize,
    pub name: String,
    pub kind: u32,
    pub flags: u32,
    pub addr: u32,
    pub offset: u32,
    pub size: u32,
    pub link: u32,
    pub info: u32,
    pub addralign: u32,
    pub entsize: u32,
}

impl Section {
    pub fn is_alloc(&self) -> bool {
        self.flags & SHF_ALLOC != 0
    }

    pub fn is_exec(&self) -> bool {
        self.flags & SHF_EXECINSTR != 0
    }

    pub fn is_nobits(&self) -> bool {
        self.kind == SHT_NOBITS
    }

    /// Whether the virtual address lies inside the section.
    pub fn contains(&self, addr: u32) -> bool {
        addr >= self.addr && (addr as u64) < self.addr as u64 + self.size as u64
    }
}

/// Symbol binding and type, from `st_info`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SymbolKind {
    NoType,
    Object,
    Func,
    Section,
    File,
    Other(u8),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Symbol {
    pub name: String,
    pub value: u32,
    pub size: u32,
    pub kind: SymbolKind,
    /// 0 local, 1 global, 2 weak.
    pub bind: u8,
    pub shndx: u16,
}

impl Symbol {
    pub fn is_func(&self) -> bool {
        self.kind == SymbolKind::Func
    }

    pub fn is_object(&self) -> bool {
        self.kind == SymbolKind::Object
    }

    /// The address with the Thumb bit cleared (function symbols on Thumb
    /// targets carry bit 0 set).
    pub fn address(&self) -> u32 {
        if self.is_func() {
            self.value & !1
        } else {
            self.value
        }
    }
}

/// A parsed ELF32 file.
#[derive(Debug, Clone)]
pub struct Elf {
    data: Vec<u8>,
    /// The section headers as `reader_v::parse` read them, which the
    /// verified functions take; `sections` is the same headers with their
    /// names as text.
    raw: Vec<SecHdr>,
    pub header: Header,
    pub sections: Vec<Section>,
    pub segments: Vec<Segment>,
}

/// The bytes `[start, end)` of the file as text (invalid UTF-8 replaced).
fn text(d: &[u8], start: usize, end: usize) -> String {
    String::from_utf8_lossy(d.get(start..end).unwrap_or(&[])).into_owned()
}

impl Elf {
    /// A refusal of `reader_v`, in words.
    fn refusal(names: &[Section], e: ElfErr) -> Error {
        Error(match e {
            ElfErr::NotElf => "not an ELF file".to_string(),
            ElfErr::NotClass32 => "not a 32-bit ELF file (only ELFCLASS32 is supported)".to_string(),
            ElfErr::NotLittleEndian => "not a little-endian ELF file".to_string(),
            ElfErr::NoU16(off) => format!("truncated ELF: no u16 at offset {off:#x}"),
            ElfErr::NoU32(off) => format!("truncated ELF: no u32 at offset {off:#x}"),
            ElfErr::StrOutside(off) => format!("string offset {off:#x} is outside the file"),
            ElfErr::SectionOutside(i) => format!("section `{}` lies outside the file", names.get(i).map(|s| s.name.as_str()).unwrap_or("")),
            ElfErr::MissingStrtab => "symbol table links to a missing string table".to_string(),
        })
    }

    /// Parses an ELF32 little-endian image (`reader_v::parse`, verified).
    pub fn parse(data: Vec<u8>) -> Result<Elf, Error> {
        let parsed = reader_v::parse(&data).map_err(|e| Elf::refusal(&[], e))?;
        let sections = parsed
            .sections
            .iter()
            .zip(parsed.names.iter())
            .enumerate()
            .map(|(index, (h, &(start, end)))| Section {
                index,
                name: text(&data, start, end),
                kind: h.kind,
                flags: h.flags,
                addr: h.addr,
                offset: h.offset,
                size: h.size,
                link: h.link,
                info: h.info,
                addralign: h.addralign,
                entsize: h.entsize,
            })
            .collect();
        Ok(Elf {
            data,
            raw: parsed.sections,
            header: parsed.header,
            sections,
            segments: parsed.segments,
        })
    }

    pub fn read(path: &std::path::Path) -> Result<Elf, Error> {
        let data = std::fs::read(path).map_err(|e| Error(format!("cannot read {}: {e}", path.display())))?;
        Elf::parse(data)
    }

    pub fn section_by_name(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// The file bytes of a section (empty for `SHT_NOBITS`); the extent is
    /// `reader_v::section_range` (verified).
    pub fn section_data(&self, section: &Section) -> Result<&[u8], Error> {
        let header = SecHdr {
            name: 0,
            kind: section.kind,
            flags: section.flags,
            addr: section.addr,
            offset: section.offset,
            size: section.size,
            link: section.link,
            info: section.info,
            addralign: section.addralign,
            entsize: section.entsize,
        };
        reader_v::section_range(self.data.len(), &header)
            .and_then(|(start, end)| self.data.get(start..end))
            .ok_or_else(|| Error(format!("section `{}` lies outside the file", section.name)))
    }

    /// The section that contains a virtual address
    /// (`reader_v::section_at`, verified: the first allocated, non-empty
    /// section holding it).
    pub fn section_at(&self, addr: u32) -> Option<&Section> {
        reader_v::section_at(&self.raw, addr).and_then(|i| self.sections.get(i))
    }

    /// Bytes at a virtual address, taken from the file image of the
    /// containing allocated section. `.bss` reads as zeros
    /// (`reader_v::read_at`, verified).
    pub fn read_at(&self, addr: u32, len: usize) -> Result<Vec<u8>, Error> {
        reader_v::read_at(&self.data, &self.raw, addr, len).map_err(|e| {
            Error(match e {
                AtErr::NoSection => format!("address {addr:#010x} is in no allocated section"),
                AtErr::Crosses(i) => format!(
                    "range {addr:#010x}+{len:#x} crosses the end of section `{}`",
                    self.sections.get(i).map(|s| s.name.as_str()).unwrap_or("")
                ),
                AtErr::Outside => format!("address {addr:#010x} lies outside the file"),
            })
        })
    }

    /// An entry as `reader_v` read it, with its name as text.
    fn symbol(&self, s: &RawSym) -> Symbol {
        Symbol {
            name: text(&self.data, s.name_start, s.name_end),
            value: s.value,
            size: s.size,
            kind: match s.info & 0xf {
                0 => SymbolKind::NoType,
                1 => SymbolKind::Object,
                2 => SymbolKind::Func,
                3 => SymbolKind::Section,
                4 => SymbolKind::File,
                other => SymbolKind::Other(other),
            },
            bind: s.info >> 4,
            shndx: s.shndx,
        }
    }

    /// The symbols of every `SHT_SYMTAB` section, in table order
    /// (`reader_v::symbols`, verified).
    pub fn symbols(&self) -> Result<Vec<Symbol>, Error> {
        let raw = reader_v::symbols(&self.data, &self.raw).map_err(|e| Elf::refusal(&self.sections, e))?;
        Ok(raw.iter().map(|s| self.symbol(s)).collect())
    }

    /// The first symbol, in table order, whose name is exactly `name`, an
    /// object symbol when `object` (`reader_v::lookup`, verified: the
    /// name's bytes in the string table are `name`'s, the fields are that
    /// entry's, and `None` means no entry of any symbol table is such).
    /// The comparison is of bytes; for a `name` without the replacement
    /// character it finds what `symbols()` followed by a search for an
    /// equal `name` finds.
    pub fn symbol_named(&self, name: &str, object: bool) -> Result<Option<Symbol>, Error> {
        let found = reader_v::lookup(&self.data, &self.raw, name.as_bytes(), object).map_err(|e| Elf::refusal(&self.sections, e))?;
        Ok(found.map(|s| self.symbol(&s)))
    }

    /// The loadable image: for each `PT_LOAD` segment with file content, in
    /// ascending physical address, the physical address, the file size, and
    /// the bytes. For a relocatable file (no program headers) the allocated
    /// sections with content stand in, in ascending address order. This is
    /// the input of the binary hash (R33.3): it covers exactly what a
    /// flashing tool writes and nothing that only describes it (debug
    /// information, symbol tables, section names).
    pub fn loadable_image(&self) -> Result<Vec<(u32, Vec<u8>)>, Error> {
        let mut parts = Vec::new();
        let mut loads: Vec<&Segment> = self
            .segments
            .iter()
            .filter(|s| s.kind == PT_LOAD && s.filesz > 0)
            .collect();
        if !loads.is_empty() {
            loads.sort_by_key(|s| (s.paddr, s.offset));
            for s in loads {
                let start = s.offset as usize;
                let end = start + s.filesz as usize;
                let bytes = self
                    .data
                    .get(start..end)
                    .ok_or_else(|| Error("a PT_LOAD segment lies outside the file".into()))?;
                parts.push((s.paddr, bytes.to_vec()));
            }
            return Ok(parts);
        }
        let mut sections: Vec<&Section> = self
            .sections
            .iter()
            .filter(|s| s.is_alloc() && !s.is_nobits() && s.size > 0)
            .collect();
        sections.sort_by_key(|s| (s.addr, s.offset));
        for s in sections {
            parts.push((s.addr, self.section_data(s)?.to_vec()));
        }
        Ok(parts)
    }

    /// The bytes over which the binary hash is computed: for each part of
    /// [`Elf::loadable_image`], the 4-byte little-endian address, the
    /// 4-byte little-endian length, and the content.
    pub fn hash_input(&self) -> Result<Vec<u8>, Error> {
        let mut out = Vec::new();
        for (addr, bytes) in self.loadable_image()? {
            out.extend_from_slice(&addr.to_le_bytes());
            out.extend_from_slice(&(bytes.len() as u32).to_le_bytes());
            out.extend_from_slice(&bytes);
        }
        Ok(out)
    }

    pub fn is_arm(&self) -> bool {
        self.header.machine == EM_ARM
    }

    pub fn raw(&self) -> &[u8] {
        &self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Builds a small ELF32 executable image with one `.text` section, one
    /// `.data` section, a `.bss` section, a symbol table, and two PT_LOAD
    /// segments.
    fn sample() -> Vec<u8> {
        let text: Vec<u8> = vec![0x70, 0x47, 0x00, 0xbf]; // bx lr; nop
        let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8];
        let shstrtab = b"\0.text\0.data\0.bss\0.symtab\0.strtab\0.shstrtab\0".to_vec();
        let strtab = b"\0entry\0CONFIG\0".to_vec();
        // Layout: header (52) | phdrs 2*32 | text | data | shstrtab | strtab | symtab | shdrs
        let phoff = 52u32;
        let text_off = phoff + 64;
        let data_off = text_off + text.len() as u32;
        let shstr_off = data_off + data.len() as u32;
        let strtab_off = shstr_off + shstrtab.len() as u32;
        let symtab_off = strtab_off + strtab.len() as u32;
        let mut symtab = vec![0u8; 16]; // null symbol
        let mut sym = |name: u32, value: u32, size: u32, info: u8, shndx: u16| {
            symtab.extend_from_slice(&name.to_le_bytes());
            symtab.extend_from_slice(&value.to_le_bytes());
            symtab.extend_from_slice(&size.to_le_bytes());
            symtab.push(info);
            symtab.push(0);
            symtab.extend_from_slice(&shndx.to_le_bytes());
        };
        sym(1, 0x1001, 4, 0x12, 1); // entry: global func, Thumb bit set
        sym(7, 0x2000_0000, 8, 0x11, 2); // CONFIG: global object
        let shoff = symtab_off + symtab.len() as u32;
        let mut out = Vec::new();
        out.extend_from_slice(b"\x7fELF\x01\x01\x01");
        out.extend_from_slice(&[0; 9]);
        out.extend_from_slice(&2u16.to_le_bytes()); // ET_EXEC
        out.extend_from_slice(&EM_ARM.to_le_bytes());
        out.extend_from_slice(&1u32.to_le_bytes()); // version
        out.extend_from_slice(&0x1001u32.to_le_bytes()); // entry
        out.extend_from_slice(&phoff.to_le_bytes());
        out.extend_from_slice(&shoff.to_le_bytes());
        out.extend_from_slice(&0u32.to_le_bytes()); // flags
        out.extend_from_slice(&52u16.to_le_bytes());
        out.extend_from_slice(&32u16.to_le_bytes());
        out.extend_from_slice(&2u16.to_le_bytes());
        out.extend_from_slice(&40u16.to_le_bytes());
        out.extend_from_slice(&7u16.to_le_bytes()); // shnum (null + 6)
        out.extend_from_slice(&6u16.to_le_bytes()); // shstrndx (.shstrtab)
        assert_eq!(out.len(), 52);
        let mut ph = |kind: u32, off: u32, vaddr: u32, paddr: u32, filesz: u32, memsz: u32| {
            for v in [kind, off, vaddr, paddr, filesz, memsz, 5, 4] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        };
        ph(PT_LOAD, text_off, 0x1000, 0x1000, 4, 4);
        ph(PT_LOAD, data_off, 0x2000_0000, 0x1004, 8, 8);
        out.extend_from_slice(&text);
        out.extend_from_slice(&data);
        out.extend_from_slice(&shstrtab);
        out.extend_from_slice(&strtab);
        out.extend_from_slice(&symtab);
        assert_eq!(out.len() as u32, shoff);
        let mut sh = |name: u32, kind: u32, flags: u32, addr: u32, off: u32, size: u32, link: u32, info: u32, entsize: u32| {
            for v in [name, kind, flags, addr, off, size, link, info, 4, entsize] {
                out.extend_from_slice(&v.to_le_bytes());
            }
        };
        sh(0, 0, 0, 0, 0, 0, 0, 0, 0);
        sh(1, 1, SHF_ALLOC | SHF_EXECINSTR, 0x1000, text_off, 4, 0, 0, 0);
        sh(7, 1, SHF_ALLOC | SHF_WRITE, 0x2000_0000, data_off, 8, 0, 0, 0);
        sh(13, SHT_NOBITS, SHF_ALLOC | SHF_WRITE, 0x2000_0008, data_off + 8, 16, 0, 0, 0);
        sh(18, SHT_SYMTAB, 0, 0, symtab_off, symtab.len() as u32, 5, 1, 16);
        sh(26, 3, 0, 0, strtab_off, strtab.len() as u32, 0, 0, 0);
        sh(34, 3, 0, 0, shstr_off, shstrtab.len() as u32, 0, 0, 0);
        out
    }

    #[test]
    fn parses_sections_segments_and_symbols() {
        let elf = Elf::parse(sample()).expect("parses");
        assert!(elf.is_arm());
        let names: Vec<&str> = elf.sections.iter().map(|s| s.name.as_str()).collect();
        assert_eq!(names, vec!["", ".text", ".data", ".bss", ".symtab", ".strtab", ".shstrtab"]);
        let syms = elf.symbols().expect("symbols");
        let entry = syms.iter().find(|s| s.name == "entry").expect("entry symbol");
        assert!(entry.is_func());
        assert_eq!(entry.address(), 0x1000);
        let config = syms.iter().find(|s| s.name == "CONFIG").expect("CONFIG symbol");
        assert_eq!(config.value, 0x2000_0000);
        assert_eq!(elf.read_at(0x2000_0000, 8).expect("data"), vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(elf.read_at(0x2000_0008, 4).expect("bss"), vec![0; 4]);
        assert!(elf.read_at(0x2000_0006, 4).is_err());
        assert!(elf.read_at(0x3000_0000, 1).is_err());
    }

    /// R33.3: the hash input is the loadable sections only, so symbol names and
    /// other metadata do not change the binary hash.
    #[test]
    fn loadable_image_follows_physical_addresses_and_excludes_metadata() {
        let elf = Elf::parse(sample()).expect("parses");
        let image = elf.loadable_image().expect("image");
        assert_eq!(image.len(), 2);
        assert_eq!(image[0].0, 0x1000);
        assert_eq!(image[0].1, vec![0x70, 0x47, 0x00, 0xbf]);
        assert_eq!(image[1].0, 0x1004);
        assert_eq!(image[1].1.len(), 8);
        let input = elf.hash_input().expect("hash input");
        assert_eq!(input.len(), 2 * 8 + 4 + 8);
        // Changing a symbol name does not change the hash input.
        let mut altered = sample();
        let pos = altered.windows(6).position(|w| w == b"CONFIG").expect("name present");
        altered[pos] = b'X';
        let altered = Elf::parse(altered).expect("parses");
        assert_eq!(altered.hash_input().expect("hash input"), input);
    }

    #[test]
    fn rejects_other_classes_and_truncation() {
        assert!(Elf::parse(b"not an elf".to_vec()).is_err());
        let mut sample64 = sample();
        sample64[4] = 2;
        assert!(Elf::parse(sample64).is_err());
        let truncated = sample()[..60].to_vec();
        assert!(Elf::parse(truncated).is_err());
    }

    /// The reader as it was before `reader_v` (2026-11-26), kept as the
    /// reference the verified reader is compared with: what it parses,
    /// the symbols it lists, and the text of each refusal.
    mod reference {
        use super::super::{Error, Symbol, SymbolKind, SHF_ALLOC, SHT_NOBITS, SHT_SYMTAB};

        pub struct Sec {
            pub name: String,
            pub kind: u32,
            pub flags: u32,
            pub addr: u32,
            pub offset: u32,
            pub size: u32,
            pub link: u32,
            pub entsize: u32,
        }

        fn err<T>(msg: impl Into<String>) -> Result<T, Error> {
            Err(Error(msg.into()))
        }

        fn u16_at(d: &[u8], off: usize) -> Result<u16, Error> {
            match d.get(off..off + 2) {
                Some(b) => Ok(u16::from_le_bytes([b[0], b[1]])),
                None => err(format!("truncated ELF: no u16 at offset {off:#x}")),
            }
        }

        fn u32_at(d: &[u8], off: usize) -> Result<u32, Error> {
            match d.get(off..off + 4) {
                Some(b) => Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]])),
                None => err(format!("truncated ELF: no u32 at offset {off:#x}")),
            }
        }

        fn cstr(d: &[u8], off: usize) -> Result<String, Error> {
            let bytes = d.get(off..).ok_or_else(|| Error(format!("string offset {off:#x} is outside the file")))?;
            let end = bytes.iter().position(|&b| b == 0).unwrap_or(bytes.len());
            Ok(String::from_utf8_lossy(&bytes[..end]).into_owned())
        }

        pub fn parse(data: &[u8]) -> Result<(Vec<Sec>, Vec<[u32; 8]>), Error> {
            if data.len() < 52 || &data[0..4] != b"\x7fELF" {
                return err("not an ELF file");
            }
            if data[4] != 1 {
                return err("not a 32-bit ELF file (only ELFCLASS32 is supported)");
            }
            if data[5] != 1 {
                return err("not a little-endian ELF file");
            }
            let (phoff, shoff) = (u32_at(data, 28)?, u32_at(data, 32)?);
            let (phentsize, phnum, shentsize, shnum, shstrndx) = (u16_at(data, 42)?, u16_at(data, 44)?, u16_at(data, 46)?, u16_at(data, 48)?, u16_at(data, 50)?);
            let mut segments = Vec::new();
            for i in 0..phnum as usize {
                let base = phoff as usize + i * phentsize as usize;
                let mut words = [0u32; 8];
                for (k, w) in words.iter_mut().enumerate() {
                    *w = u32_at(data, base + 4 * k)?;
                }
                segments.push(words);
            }
            let mut raw = Vec::new();
            for i in 0..shnum as usize {
                let base = shoff as usize + i * shentsize as usize;
                raw.push((
                    u32_at(data, base)?,
                    Sec {
                        name: String::new(),
                        kind: u32_at(data, base + 4)?,
                        flags: u32_at(data, base + 8)?,
                        addr: u32_at(data, base + 12)?,
                        offset: u32_at(data, base + 16)?,
                        size: u32_at(data, base + 20)?,
                        link: u32_at(data, base + 24)?,
                        entsize: { u32_at(data, base + 28)?; u32_at(data, base + 32)?; u32_at(data, base + 36)? },
                    },
                ));
            }
            let shstr_off = raw.get(shstrndx as usize).map(|(_, s)| s.offset as usize);
            let mut sections = Vec::with_capacity(raw.len());
            for (name_off, mut section) in raw {
                if let Some(base) = shstr_off {
                    section.name = cstr(data, base + name_off as usize)?;
                }
                sections.push(section);
            }
            Ok((sections, segments))
        }

        fn section_data<'a>(data: &'a [u8], section: &Sec) -> Result<&'a [u8], Error> {
            if section.kind == SHT_NOBITS {
                return Ok(&[]);
            }
            let start = section.offset as usize;
            let end = start + section.size as usize;
            data.get(start..end).ok_or_else(|| Error(format!("section `{}` lies outside the file", section.name)))
        }

        pub fn symbols(data: &[u8], sections: &[Sec]) -> Result<Vec<Symbol>, Error> {
            let mut out = Vec::new();
            for table in sections.iter().filter(|s| s.kind == SHT_SYMTAB) {
                let strtab = sections.get(table.link as usize).ok_or_else(|| Error("symbol table links to a missing string table".into()))?;
                let strings = section_data(data, strtab)?;
                let entries = section_data(data, table)?;
                let entsize = if table.entsize == 0 { 16 } else { table.entsize as usize };
                for entry in entries.chunks_exact(entsize) {
                    let name_off = u32_at(entry, 0)? as usize;
                    let info = entry.get(12).copied().unwrap_or(0);
                    let kind = match info & 0xf {
                        0 => SymbolKind::NoType,
                        1 => SymbolKind::Object,
                        2 => SymbolKind::Func,
                        3 => SymbolKind::Section,
                        4 => SymbolKind::File,
                        other => SymbolKind::Other(other),
                    };
                    out.push(Symbol { name: cstr(strings, name_off)?, value: u32_at(entry, 4)?, size: u32_at(entry, 8)?, kind, bind: info >> 4, shndx: u16_at(entry, 14)? });
                }
            }
            Ok(out)
        }

        pub fn read_at(data: &[u8], sections: &[Sec], addr: u32, len: usize) -> Result<Vec<u8>, Error> {
            let section = sections
                .iter()
                .find(|s| s.flags & SHF_ALLOC != 0 && s.size > 0 && addr >= s.addr && (addr as u64) < s.addr as u64 + s.size as u64)
                .ok_or_else(|| Error(format!("address {addr:#010x} is in no allocated section")))?;
            let end = addr as u64 + len as u64;
            if end > section.addr as u64 + section.size as u64 {
                return err(format!("range {addr:#010x}+{len:#x} crosses the end of section `{}`", section.name));
            }
            if section.kind == SHT_NOBITS {
                return Ok(vec![0; len]);
            }
            let start = section.offset as usize + (addr - section.addr) as usize;
            data.get(start..start + len).map(<[u8]>::to_vec).ok_or_else(|| Error(format!("address {addr:#010x} lies outside the file")))
        }
    }

    /// Everything the reader says of `bytes`, and the reference's saying
    /// of the same, compared: the parse or its refusal, the symbols or
    /// their refusal, the lookups, and reads at the given addresses.
    fn compare(bytes: &[u8], addresses: &[(u32, usize)]) {
        let new = Elf::parse(bytes.to_vec());
        let old = reference::parse(bytes);
        let (elf, (sections, segments)) = match (new, old) {
            (Ok(n), Ok(o)) => (n, o),
            (Err(n), Err(o)) => {
                assert_eq!(n, o);
                return;
            }
            (n, o) => panic!("the reader and the reference disagree on parsing: {:?} against {:?}", n.map(|_| ()), o.map(|_| ())),
        };
        assert_eq!(elf.sections.len(), sections.len());
        for (a, b) in elf.sections.iter().zip(&sections) {
            assert_eq!((&a.name, a.kind, a.flags, a.addr, a.offset, a.size, a.link, a.entsize), (&b.name, b.kind, b.flags, b.addr, b.offset, b.size, b.link, b.entsize));
        }
        let words: Vec<[u32; 8]> = elf.segments.iter().map(|s| [s.kind, s.offset, s.vaddr, s.paddr, s.filesz, s.memsz, s.flags, s.align]).collect();
        assert_eq!(words, segments);
        let old_symbols = reference::symbols(bytes, &sections);
        assert_eq!(elf.symbols(), old_symbols);
        for name in ["entry", "CONFIG", "", "absent", "XONFIG", "CONFI"] {
            for object in [false, true] {
                let found = elf.symbol_named(name, object);
                let expected = old_symbols.clone().map(|all| all.into_iter().find(|s| s.name == name && (!object || s.is_object())));
                assert_eq!(found, expected, "symbol `{name}`, object {object}");
            }
        }
        for &(addr, len) in addresses {
            assert_eq!(elf.read_at(addr, len), reference::read_at(bytes, &sections, addr, len), "read of {len} at {addr:#x}");
        }
    }

    /// The verified reader (`reader_v`) says of the sample, of each of its
    /// truncations, and of the sample with each byte altered in three
    /// ways, exactly what the reader before it said: the same sections,
    /// segments, symbols, lookups and bytes, and the same words of refusal.
    #[test]
    fn the_verified_reader_agrees_with_the_reader_before_it() {
        let addresses = [(0x1000, 4), (0x1002, 2), (0x1002, 3), (0x2000_0000, 8), (0x2000_0004, 5), (0x2000_0008, 16), (0x2000_0010, 9), (0x3000_0000, 1), (0, 0), (0x1000, 0)];
        let sample = sample();
        compare(&sample, &addresses);
        for len in 0..sample.len() {
            compare(&sample[..len], &addresses);
        }
        for at in 0..sample.len() {
            for change in [1u8, 0x80, 0xff] {
                let mut altered = sample.clone();
                altered[at] ^= change;
                compare(&altered, &addresses);
            }
        }
    }

    /// The lookup by name: the first entry in table order, by bytes; an
    /// object symbol only when asked; a name that is a prefix of another
    /// or has it as a prefix is not it.
    #[test]
    fn a_symbol_is_found_by_its_exact_name() {
        let elf = Elf::parse(sample()).expect("parses");
        let config = elf.symbol_named("CONFIG", true).expect("symbols").expect("CONFIG");
        assert_eq!((config.value, config.size, config.shndx, config.kind), (0x2000_0000, 8, 2, SymbolKind::Object));
        assert_eq!(elf.read_at(config.address(), config.size as usize).expect("content"), vec![1, 2, 3, 4, 5, 6, 7, 8]);
        assert_eq!(elf.symbol_named("entry", false).expect("symbols").expect("entry").value, 0x1001);
        assert_eq!(elf.symbol_named("entry", true).expect("symbols"), None, "entry is a function");
        for other in ["CONFI", "CONFIGS", "ONFIG", "config"] {
            assert_eq!(elf.symbol_named(other, false).expect("symbols"), None, "`{other}`");
        }
    }
}
