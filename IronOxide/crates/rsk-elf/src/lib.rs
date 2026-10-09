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
#![forbid(unsafe_code)]

use std::fmt;

/// Section flag: the section occupies memory at run time.
pub const SHF_ALLOC: u32 = 0x2;
/// Section flag: executable instructions.
pub const SHF_EXECINSTR: u32 = 0x4;
/// Section flag: writable.
pub const SHF_WRITE: u32 = 0x1;
/// Section type: no bits in the file (`.bss`).
pub const SHT_NOBITS: u32 = 8;
/// Section type: symbol table.
pub const SHT_SYMTAB: u32 = 2;
/// Program header type: loadable segment.
pub const PT_LOAD: u32 = 1;
/// Machine: ARM.
pub const EM_ARM: u16 = 40;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Error(pub String);

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Error {}

fn err<T>(msg: impl Into<String>) -> Result<T, Error> {
    Err(Error(msg.into()))
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Header {
    /// `e_type`: 1 relocatable, 2 executable.
    pub kind: u16,
    pub machine: u16,
    pub entry: u32,
    pub flags: u32,
    pub phoff: u32,
    pub shoff: u32,
    pub phentsize: u16,
    pub phnum: u16,
    pub shentsize: u16,
    pub shnum: u16,
    pub shstrndx: u16,
}

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

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: u32,
    pub offset: u32,
    pub vaddr: u32,
    pub paddr: u32,
    pub filesz: u32,
    pub memsz: u32,
    pub flags: u32,
    pub align: u32,
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
    pub header: Header,
    pub sections: Vec<Section>,
    pub segments: Vec<Segment>,
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

impl Elf {
    /// Parses an ELF32 little-endian image.
    pub fn parse(data: Vec<u8>) -> Result<Elf, Error> {
        if data.len() < 52 || &data[0..4] != b"\x7fELF" {
            return err("not an ELF file");
        }
        if data[4] != 1 {
            return err("not a 32-bit ELF file (only ELFCLASS32 is supported)");
        }
        if data[5] != 1 {
            return err("not a little-endian ELF file");
        }
        let header = Header {
            kind: u16_at(&data, 16)?,
            machine: u16_at(&data, 18)?,
            entry: u32_at(&data, 24)?,
            phoff: u32_at(&data, 28)?,
            shoff: u32_at(&data, 32)?,
            flags: u32_at(&data, 36)?,
            phentsize: u16_at(&data, 42)?,
            phnum: u16_at(&data, 44)?,
            shentsize: u16_at(&data, 46)?,
            shnum: u16_at(&data, 48)?,
            shstrndx: u16_at(&data, 50)?,
        };
        let mut segments = Vec::new();
        for i in 0..header.phnum as usize {
            let base = header.phoff as usize + i * header.phentsize as usize;
            segments.push(Segment {
                kind: u32_at(&data, base)?,
                offset: u32_at(&data, base + 4)?,
                vaddr: u32_at(&data, base + 8)?,
                paddr: u32_at(&data, base + 12)?,
                filesz: u32_at(&data, base + 16)?,
                memsz: u32_at(&data, base + 20)?,
                flags: u32_at(&data, base + 24)?,
                align: u32_at(&data, base + 28)?,
            });
        }
        let mut raw = Vec::new();
        for i in 0..header.shnum as usize {
            let base = header.shoff as usize + i * header.shentsize as usize;
            raw.push((
                u32_at(&data, base)?,
                Section {
                    index: i,
                    name: String::new(),
                    kind: u32_at(&data, base + 4)?,
                    flags: u32_at(&data, base + 8)?,
                    addr: u32_at(&data, base + 12)?,
                    offset: u32_at(&data, base + 16)?,
                    size: u32_at(&data, base + 20)?,
                    link: u32_at(&data, base + 24)?,
                    info: u32_at(&data, base + 28)?,
                    addralign: u32_at(&data, base + 32)?,
                    entsize: u32_at(&data, base + 36)?,
                },
            ));
        }
        let shstr_off = raw
            .get(header.shstrndx as usize)
            .map(|(_, s)| s.offset as usize);
        let mut sections = Vec::with_capacity(raw.len());
        for (name_off, mut section) in raw {
            if let Some(base) = shstr_off {
                section.name = cstr(&data, base + name_off as usize)?;
            }
            sections.push(section);
        }
        Ok(Elf {
            data,
            header,
            sections,
            segments,
        })
    }

    pub fn read(path: &std::path::Path) -> Result<Elf, Error> {
        let data = std::fs::read(path).map_err(|e| Error(format!("cannot read {}: {e}", path.display())))?;
        Elf::parse(data)
    }

    pub fn section_by_name(&self, name: &str) -> Option<&Section> {
        self.sections.iter().find(|s| s.name == name)
    }

    /// The file bytes of a section (empty for `SHT_NOBITS`).
    pub fn section_data(&self, section: &Section) -> Result<&[u8], Error> {
        if section.is_nobits() {
            return Ok(&[]);
        }
        let start = section.offset as usize;
        let end = start + section.size as usize;
        self.data
            .get(start..end)
            .ok_or_else(|| Error(format!("section `{}` lies outside the file", section.name)))
    }

    /// The section that contains a virtual address.
    pub fn section_at(&self, addr: u32) -> Option<&Section> {
        self.sections
            .iter()
            .find(|s| s.is_alloc() && s.size > 0 && s.contains(addr))
    }

    /// Bytes at a virtual address, taken from the file image of the
    /// containing allocated section. `.bss` reads as zeros.
    pub fn read_at(&self, addr: u32, len: usize) -> Result<Vec<u8>, Error> {
        let section = self
            .section_at(addr)
            .ok_or_else(|| Error(format!("address {addr:#010x} is in no allocated section")))?;
        let end = addr as u64 + len as u64;
        if end > section.addr as u64 + section.size as u64 {
            return err(format!(
                "range {addr:#010x}+{len:#x} crosses the end of section `{}`",
                section.name
            ));
        }
        if section.is_nobits() {
            return Ok(vec![0; len]);
        }
        let start = section.offset as usize + (addr - section.addr) as usize;
        self.data
            .get(start..start + len)
            .map(<[u8]>::to_vec)
            .ok_or_else(|| Error(format!("address {addr:#010x} lies outside the file")))
    }

    /// The symbols of every `SHT_SYMTAB` section, in table order.
    pub fn symbols(&self) -> Result<Vec<Symbol>, Error> {
        let mut out = Vec::new();
        for table in self.sections.iter().filter(|s| s.kind == SHT_SYMTAB) {
            let strtab = self
                .sections
                .get(table.link as usize)
                .ok_or_else(|| Error("symbol table links to a missing string table".into()))?;
            let strings = self.section_data(strtab)?;
            let entries = self.section_data(table)?;
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
                out.push(Symbol {
                    name: cstr(strings, name_off)?,
                    value: u32_at(entry, 4)?,
                    size: u32_at(entry, 8)?,
                    kind,
                    bind: info >> 4,
                    shndx: u16_at(entry, 14)?,
                });
            }
        }
        Ok(out)
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
}
