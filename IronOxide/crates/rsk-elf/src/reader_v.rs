//! The reader over the bytes of an ELF file, verified (2026-11-26).
//!
//! Everything `lib.rs` learns from the file's bytes it learns through the
//! functions of this module: the header, the program headers, the section
//! headers and the extent of each section's name (`parse`), a section's
//! extent in the file (`section_range`), the symbols of every symbol table
//! (`symbols`), the first symbol of a given name (`lookup`), and the bytes
//! at a virtual address (`read_at`). What `lib.rs` adds is text: the names
//! as `String`s and the wording of a refusal.
//!
//! | Function | Proven |
//! |---|---|
//! | every function | it returns for every byte string and every argument: no index out of range, no arithmetic overflow (offsets are computed in `u64` and compared with the length before any byte is read) |
//! | `parse` | when it returns `p`: the file starts with the ELF32 little-endian identification; `p.header` is `header_of` the bytes; program header `i` is `seg_at` offset `phoff + i * phentsize`, section header `i` is `sec_at` offset `shoff + i * shentsize`, each wholly inside the file; name `i` is the extent of the zero-terminated string at the name offset of section `i` in the section-name string table (`name_ok`) |
//! | `symbols` | when it returns `v` (`syms_ok`): every element is an entry of a symbol table (`sym_ok`: its value, size, info and section index are the entry's fields, its name extent is the zero-terminated string at the entry's name offset in the linked string table); the elements are in table order, entries in order; and every entry of every symbol table is there (`valid_entry` implies `has`) |
//! | `lookup` | `Ok(Some(s))`: `s` is an entry of a symbol table (`sym_ok`), its name bytes are exactly the name asked for (and it is an object when asked), and no entry before it in table order is such; `Ok(None)`: no entry of any symbol table is such |
//! | `read_at` | when it returns `v`: there is a first allocated, non-empty section `i` holding the address, the range ends inside it, and `v` is the file's bytes from `offset + (addr - section addr)` on, or zeros for a section without file content |
//!
//! Not stated: which refusal a malformed file gets (the refusals are those
//! the reader gave before this module existed; the tests fix their text).

use vstd::prelude::*;

verus! {

/// Section flag: writable.
pub const SHF_WRITE: u32 = 0x1;
/// Section flag: the section occupies memory at run time.
pub const SHF_ALLOC: u32 = 0x2;
/// Section flag: executable instructions.
pub const SHF_EXECINSTR: u32 = 0x4;
/// Section type: no bits in the file (`.bss`).
pub const SHT_NOBITS: u32 = 8;
/// Section type: symbol table.
pub const SHT_SYMTAB: u32 = 2;
/// Program header type: loadable segment.
pub const PT_LOAD: u32 = 1;
/// Machine: ARM.
pub const EM_ARM: u16 = 40;

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

/// A section header as the file has it: the ten words.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SecHdr {
    /// The offset of the name in the section-name string table.
    pub name: u32,
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

/// A symbol-table entry: where it stands, the extent of its name in the
/// file, and its fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RawSym {
    /// The index of the symbol table's section.
    pub table: usize,
    /// The entry's index in that table.
    pub index: usize,
    /// The file offsets of the first byte of the name and of the byte
    /// after its last.
    pub name_start: usize,
    pub name_end: usize,
    pub value: u32,
    pub size: u32,
    pub info: u8,
    pub shndx: u16,
}

/// Why the bytes are not a usable ELF32 little-endian file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfErr {
    NotElf,
    NotClass32,
    NotLittleEndian,
    /// No `u16` at this offset.
    NoU16(u64),
    /// No `u32` at this offset.
    NoU32(u64),
    /// A string offset beyond its table.
    StrOutside(u64),
    /// The section of this index lies outside the file.
    SectionOutside(usize),
    /// A symbol table whose link names no section.
    MissingStrtab,
}

/// Why there are no bytes at a virtual address.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AtErr {
    /// The address is in no allocated section.
    NoSection,
    /// The range crosses the end of the section of this index.
    Crosses(usize),
    /// The section's content lies outside the file.
    Outside,
}

/// What `parse` reads.
pub struct Parsed {
    pub header: Header,
    pub segments: Vec<Segment>,
    pub sections: Vec<SecHdr>,
    /// Per section the extent of its name in the file; `(0, 0)` for every
    /// section when the header names no section-name string table.
    pub names: Vec<(usize, usize)>,
}

// ------------------------------------------------------ the statements --

/// A `u16`, least significant byte first, at `off`.
pub open spec fn le16(d: Seq<u8>, off: int) -> u16 {
    (d[off] as u16) | ((d[off + 1] as u16) << 8)
}

/// A `u32`, least significant byte first, at `off`.
pub open spec fn le32(d: Seq<u8>, off: int) -> u32 {
    (d[off] as u32) | ((d[off + 1] as u32) << 8) | ((d[off + 2] as u32) << 16) | ((d[off + 3] as u32) << 24)
}

/// The ELF32 header's fields (the System V ABI's offsets).
pub open spec fn header_of(d: Seq<u8>) -> Header {
    Header {
        kind: le16(d, 16),
        machine: le16(d, 18),
        entry: le32(d, 24),
        flags: le32(d, 36),
        phoff: le32(d, 28),
        shoff: le32(d, 32),
        phentsize: le16(d, 42),
        phnum: le16(d, 44),
        shentsize: le16(d, 46),
        shnum: le16(d, 48),
        shstrndx: le16(d, 50),
    }
}

/// The program header at `b`.
pub open spec fn seg_at(d: Seq<u8>, b: int) -> Segment {
    Segment {
        kind: le32(d, b),
        offset: le32(d, b + 4),
        vaddr: le32(d, b + 8),
        paddr: le32(d, b + 12),
        filesz: le32(d, b + 16),
        memsz: le32(d, b + 20),
        flags: le32(d, b + 24),
        align: le32(d, b + 28),
    }
}

/// The section header at `b`.
pub open spec fn sec_at(d: Seq<u8>, b: int) -> SecHdr {
    SecHdr {
        name: le32(d, b),
        kind: le32(d, b + 4),
        flags: le32(d, b + 8),
        addr: le32(d, b + 12),
        offset: le32(d, b + 16),
        size: le32(d, b + 20),
        link: le32(d, b + 24),
        info: le32(d, b + 28),
        addralign: le32(d, b + 32),
        entsize: le32(d, b + 36),
    }
}

/// `end` is where the zero-terminated string that starts at `from` ends,
/// in a table that ends at `limit`: no zero byte before it, and a zero
/// byte at it unless the table ends first.
pub open spec fn is_str(d: Seq<u8>, from: int, limit: int, end: int) -> bool {
    &&& 0 <= from <= end <= limit <= d.len()
    &&& forall|i: int| from <= i < end ==> #[trigger] d[i] != 0
    &&& end < limit ==> d[end] == 0
}

/// The program header of index `j`, of `e` bytes from `off` on, is in the
/// file and is `s`.
pub open spec fn seg_ok(d: Seq<u8>, off: u32, e: u16, j: int, s: Segment) -> bool {
    off + j * e + 32 <= d.len() && s == seg_at(d, off + j * e)
}

/// Every one of `n` program headers is in the file and is the element of
/// its index.
pub open spec fn segs_ok(d: Seq<u8>, off: u32, e: u16, n: u16, v: Seq<Segment>) -> bool {
    &&& v.len() == n
    &&& forall|j: int| 0 <= j < n ==> seg_ok(d, off, e, j, #[trigger] v[j])
}

/// The same of a section header.
pub open spec fn sec_ok(d: Seq<u8>, off: u32, e: u16, j: int, s: SecHdr) -> bool {
    off + j * e + 40 <= d.len() && s == sec_at(d, off + j * e)
}

pub open spec fn secs_ok(d: Seq<u8>, off: u32, e: u16, n: u16, v: Seq<SecHdr>) -> bool {
    &&& v.len() == n
    &&& forall|j: int| 0 <= j < n ==> sec_ok(d, off, e, j, #[trigger] v[j])
}

/// The name extent of section `i`: the string at its name offset in the
/// section-name string table, which runs to the end of the file as the
/// reader takes it; `(0, 0)` when the header names no such table.
pub open spec fn name_ok(d: Seq<u8>, secs: Seq<SecHdr>, shstrndx: u16, i: int, nm: (usize, usize)) -> bool {
    if shstrndx < secs.len() {
        nm.0 == secs[shstrndx as int].offset + secs[i].name && is_str(d, nm.0 as int, d.len() as int, nm.1 as int)
    } else {
        nm == (0usize, 0usize)
    }
}

pub open spec fn parsed_ok(d: Seq<u8>, p: Parsed) -> bool {
    let h = p.header;
    &&& d.len() >= 52 && d[0] == 0x7f && d[1] == 0x45 && d[2] == 0x4c && d[3] == 0x46 && d[4] == 1 && d[5] == 1
    &&& h == header_of(d)
    &&& segs_ok(d, h.phoff, h.phentsize, h.phnum, p.segments@)
    &&& secs_ok(d, h.shoff, h.shentsize, h.shnum, p.sections@)
    &&& p.names@.len() == p.sections@.len()
    &&& forall|i: int| 0 <= i < p.names@.len() ==> name_ok(d, p.sections@, h.shstrndx, i, #[trigger] p.names@[i])
}

/// A section's extent in a file of `len` bytes: empty for a section
/// without file content, none when it lies outside the file.
pub open spec fn sec_range(len: int, s: SecHdr) -> Option<(int, int)> {
    if s.kind == SHT_NOBITS {
        Some((0int, 0int))
    } else if s.offset + s.size <= len {
        Some((s.offset as int, s.offset + s.size))
    } else {
        None
    }
}

/// The size of a symbol table's entries (16 when the header leaves it 0).
pub open spec fn ent_size(s: SecHdr) -> int {
    if s.entsize == 0 { 16 } else { s.entsize as int }
}

/// `s` holds the fields of the entry of `e >= 16` bytes at `b`, with its
/// name in the string table `[ss, se)`.
pub open spec fn entry_ok(d: Seq<u8>, ss: int, se: int, b: int, e: int, s: RawSym) -> bool {
    &&& e >= 16
    &&& s.value == le32(d, b + 4)
    &&& s.size == le32(d, b + 8)
    &&& s.info == d[b + 12]
    &&& s.shndx == le16(d, b + 14)
    &&& s.name_start == ss + le32(d, b)
    &&& is_str(d, s.name_start as int, se, s.name_end as int)
}

/// Section `t` is a symbol table with a string table, both in the file,
/// and has an entry of index `k`.
pub open spec fn valid_entry(d: Seq<u8>, secs: Seq<SecHdr>, t: int, k: int) -> bool {
    &&& 0 <= t < secs.len() && 0 <= k
    &&& secs[t].kind == SHT_SYMTAB
    &&& secs[t].link < secs.len()
    &&& sec_range(d.len() as int, secs[t]) is Some
    &&& sec_range(d.len() as int, secs[secs[t].link as int]) is Some
    &&& sec_range(d.len() as int, secs[t]).unwrap().0 + k * ent_size(secs[t]) + ent_size(secs[t]) <= sec_range(d.len() as int, secs[t]).unwrap().1
}

/// `s` is the entry `s.index` of the symbol table `s.table`.
pub open spec fn sym_ok(d: Seq<u8>, secs: Seq<SecHdr>, s: RawSym) -> bool {
    &&& valid_entry(d, secs, s.table as int, s.index as int)
    &&& entry_ok(
        d,
        sec_range(d.len() as int, secs[secs[s.table as int].link as int]).unwrap().0,
        sec_range(d.len() as int, secs[secs[s.table as int].link as int]).unwrap().1,
        sec_range(d.len() as int, secs[s.table as int]).unwrap().0 + s.index * ent_size(secs[s.table as int]),
        ent_size(secs[s.table as int]),
        s,
    )
}

/// The entry `k` of table `t` is among `v`.
pub open spec fn has(v: Seq<RawSym>, t: int, k: int) -> bool {
    exists|j: int| 0 <= j < v.len() && (#[trigger] v[j]).table == t && v[j].index == k
}

/// Table order.
pub open spec fn before(a: RawSym, b: RawSym) -> bool {
    a.table < b.table || (a.table == b.table && a.index < b.index)
}

/// `v` is the entries of the symbol tables among the first `upto`
/// sections: each an entry, in table order, none missing.
pub open spec fn syms_ok(d: Seq<u8>, secs: Seq<SecHdr>, v: Seq<RawSym>, upto: int) -> bool {
    &&& forall|j: int| 0 <= j < v.len() ==> sym_ok(d, secs, #[trigger] v[j]) && v[j].table < upto
    &&& forall|i: int, j: int| 0 <= i < j < v.len() ==> before(#[trigger] v[i], #[trigger] v[j])
    &&& forall|t: int, k: int| t < upto && #[trigger] valid_entry(d, secs, t, k) ==> has(v, t, k)
}

/// The bytes of a symbol's name.
pub open spec fn name_of(d: Seq<u8>, s: RawSym) -> Seq<u8> {
    d.subrange(s.name_start as int, s.name_end as int)
}

/// The symbol asked for: of this name, and an object when `object`.
pub open spec fn wanted(d: Seq<u8>, s: RawSym, name: Seq<u8>, object: bool) -> bool {
    name_of(d, s) == name && (object ==> s.info & 0xf == 1)
}

/// The section holds the address: allocated, not empty, the address in it.
pub open spec fn sec_holds(s: SecHdr, addr: u32) -> bool {
    s.flags & SHF_ALLOC != 0 && s.size > 0 && addr >= s.addr && addr < s.addr + s.size
}

/// Section `i` is the first that holds the address.
pub open spec fn first_holding(secs: Seq<SecHdr>, addr: u32, i: int) -> bool {
    &&& 0 <= i < secs.len() && sec_holds(secs[i], addr)
    &&& forall|j: int| 0 <= j < i ==> !sec_holds(#[trigger] secs[j], addr)
}

/// `v` is the `len` bytes at `addr` of section `s`.
pub open spec fn bytes_at(d: Seq<u8>, s: SecHdr, addr: u32, len: int, v: Seq<u8>) -> bool {
    &&& addr + len <= s.addr + s.size
    &&& if s.kind == SHT_NOBITS {
        v == Seq::new(len as nat, |i: int| 0u8)
    } else {
        s.offset + (addr - s.addr) + len <= d.len() && v == d.subrange(s.offset + (addr - s.addr), s.offset + (addr - s.addr) + len)
    }
}

// ---------------------------------------------------------- the reader --

/// The `u16` at `off`, when the file has two bytes there.
pub fn u16_at(d: &[u8], off: u64) -> (r: Option<u16>)
    ensures r == (if off + 2 <= d@.len() { Some(le16(d@, off as int)) } else { None::<u16> }),
{
    let n = d.len() as u64;
    if n < 2 || off > n - 2 {
        return None;
    }
    let i = off as usize;
    Some((d[i] as u16) | ((d[i + 1] as u16) << 8))
}

/// The `u32` at `off`, when the file has four bytes there.
pub fn u32_at(d: &[u8], off: u64) -> (r: Option<u32>)
    ensures r == (if off + 4 <= d@.len() { Some(le32(d@, off as int)) } else { None::<u32> }),
{
    let n = d.len() as u64;
    if n < 4 || off > n - 4 {
        return None;
    }
    let i = off as usize;
    Some((d[i] as u32) | ((d[i + 1] as u32) << 8) | ((d[i + 2] as u32) << 16) | ((d[i + 3] as u32) << 24))
}

/// Where the string that starts at `from` ends, in a table ending at
/// `limit`.
pub fn str_end(d: &[u8], from: usize, limit: usize) -> (r: usize)
    requires from <= limit <= d@.len(),
    ensures is_str(d@, from as int, limit as int, r as int),
{
    let mut i = from;
    while i < limit && d[i] != 0
        invariant from <= i <= limit, limit <= d@.len(), forall|j: int| from <= j < i ==> #[trigger] d@[j] != 0,
        decreases limit - i,
    {
        i += 1;
    }
    i
}

fn rd_segment(d: &[u8], base: u64) -> (r: Result<Segment, ElfErr>)
    requires base <= 0x2_0000_0000,
    ensures
        r is Ok ==> base + 32 <= d@.len() && r.unwrap() == seg_at(d@, base as int),
        r is Err ==> base + 32 > d@.len(),
{
    let kind = match u32_at(d, base) { Some(v) => v, None => { return Err(ElfErr::NoU32(base)); } };
    let offset = match u32_at(d, base + 4) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 4)); } };
    let vaddr = match u32_at(d, base + 8) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 8)); } };
    let paddr = match u32_at(d, base + 12) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 12)); } };
    let filesz = match u32_at(d, base + 16) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 16)); } };
    let memsz = match u32_at(d, base + 20) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 20)); } };
    let flags = match u32_at(d, base + 24) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 24)); } };
    let align = match u32_at(d, base + 28) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 28)); } };
    Ok(Segment { kind, offset, vaddr, paddr, filesz, memsz, flags, align })
}

/// The `n` program headers of `e` bytes from `off` on.
fn rd_segments(d: &[u8], off: u32, e: u16, n: u16) -> (r: Result<Vec<Segment>, ElfErr>)
    ensures r is Ok ==> segs_ok(d@, off, e, n, r.unwrap()@),
{
    let mut out: Vec<Segment> = Vec::new();
    let mut i: u64 = 0;
    while i < n as u64
        invariant
            i <= n,
            out@.len() == i,
            forall|j: int| 0 <= j < i ==> seg_ok(d@, off, e, j, #[trigger] out@[j]),
        decreases n - i,
    {
        assert(i * (e as u64) <= 0xffff * 0xffff) by (nonlinear_arith)
            requires i <= 0xffff, e <= 0xffff;
        let base = off as u64 + i * (e as u64);
        let s = match rd_segment(d, base) { Ok(v) => v, Err(x) => { return Err(x); } };
        let ghost prev = out@;
        out.push(s);
        proof {
            assert(base as int == off + (i as int) * (e as int));
            assert forall|j: int| 0 <= j < i + 1 implies seg_ok(d@, off, e, j, #[trigger] out@[j]) by {
                if j < i {
                    assert(out@[j] == prev[j]);
                }
            }
        }
        i += 1;
    }
    Ok(out)
}

fn rd_section(d: &[u8], base: u64) -> (r: Result<SecHdr, ElfErr>)
    requires base <= 0x2_0000_0000,
    ensures
        r is Ok ==> base + 40 <= d@.len() && r.unwrap() == sec_at(d@, base as int),
        r is Err ==> base + 40 > d@.len(),
{
    let name = match u32_at(d, base) { Some(v) => v, None => { return Err(ElfErr::NoU32(base)); } };
    let kind = match u32_at(d, base + 4) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 4)); } };
    let flags = match u32_at(d, base + 8) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 8)); } };
    let addr = match u32_at(d, base + 12) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 12)); } };
    let offset = match u32_at(d, base + 16) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 16)); } };
    let size = match u32_at(d, base + 20) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 20)); } };
    let link = match u32_at(d, base + 24) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 24)); } };
    let info = match u32_at(d, base + 28) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 28)); } };
    let addralign = match u32_at(d, base + 32) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 32)); } };
    let entsize = match u32_at(d, base + 36) { Some(v) => v, None => { return Err(ElfErr::NoU32(base + 36)); } };
    Ok(SecHdr { name, kind, flags, addr, offset, size, link, info, addralign, entsize })
}

/// The `n` section headers of `e` bytes from `off` on.
fn rd_sections(d: &[u8], off: u32, e: u16, n: u16) -> (r: Result<Vec<SecHdr>, ElfErr>)
    ensures r is Ok ==> secs_ok(d@, off, e, n, r.unwrap()@),
{
    let mut out: Vec<SecHdr> = Vec::new();
    let mut i: u64 = 0;
    while i < n as u64
        invariant
            i <= n,
            out@.len() == i,
            forall|j: int| 0 <= j < i ==> sec_ok(d@, off, e, j, #[trigger] out@[j]),
        decreases n - i,
    {
        assert(i * (e as u64) <= 0xffff * 0xffff) by (nonlinear_arith)
            requires i <= 0xffff, e <= 0xffff;
        let base = off as u64 + i * (e as u64);
        let s = match rd_section(d, base) { Ok(v) => v, Err(x) => { return Err(x); } };
        let ghost prev = out@;
        out.push(s);
        proof {
            assert(base as int == off + (i as int) * (e as int));
            assert forall|j: int| 0 <= j < i + 1 implies sec_ok(d@, off, e, j, #[trigger] out@[j]) by {
                if j < i {
                    assert(out@[j] == prev[j]);
                }
            }
        }
        i += 1;
    }
    Ok(out)
}

/// The name extents of the sections, in the string table at `base`.
fn rd_names(d: &[u8], secs: &Vec<SecHdr>, base: u32) -> (r: Result<Vec<(usize, usize)>, ElfErr>)
    ensures
        r is Ok ==> r.unwrap()@.len() == secs@.len() && forall|i: int| 0 <= i < secs@.len() ==> {
            &&& (#[trigger] r.unwrap()@[i]).0 == base + secs@[i].name
            &&& is_str(d@, r.unwrap()@[i].0 as int, d@.len() as int, r.unwrap()@[i].1 as int)
        },
{
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i: usize = 0;
    while i < secs.len()
        invariant
            i <= secs@.len(),
            out@.len() == i,
            forall|j: int| 0 <= j < i ==> {
                &&& (#[trigger] out@[j]).0 == base + secs@[j].name
                &&& is_str(d@, out@[j].0 as int, d@.len() as int, out@[j].1 as int)
            },
        decreases secs@.len() - i,
    {
        let off = base as u64 + secs[i].name as u64;
        if off > d.len() as u64 {
            return Err(ElfErr::StrOutside(off));
        }
        let s = off as usize;
        let e = str_end(d, s, d.len());
        out.push((s, e));
        i += 1;
    }
    Ok(out)
}

/// `n` empty name extents.
fn no_names(n: usize) -> (r: Vec<(usize, usize)>)
    ensures r@.len() == n, forall|i: int| 0 <= i < n ==> #[trigger] r@[i] == (0usize, 0usize),
{
    let mut out: Vec<(usize, usize)> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant i <= n, out@.len() == i, forall|j: int| 0 <= j < i ==> #[trigger] out@[j] == (0usize, 0usize),
        decreases n - i,
    {
        out.push((0, 0));
        i += 1;
    }
    out
}

/// The header, the program headers, the section headers and the extents
/// of the section names of an ELF32 little-endian file.
pub fn parse(d: &[u8]) -> (r: Result<Parsed, ElfErr>)
    ensures r is Ok ==> parsed_ok(d@, r.unwrap()),
{
    if d.len() < 52 || d[0] != 0x7f || d[1] != 0x45 || d[2] != 0x4c || d[3] != 0x46 {
        return Err(ElfErr::NotElf);
    }
    if d[4] != 1 {
        return Err(ElfErr::NotClass32);
    }
    if d[5] != 1 {
        return Err(ElfErr::NotLittleEndian);
    }
    let kind = match u16_at(d, 16) { Some(v) => v, None => { return Err(ElfErr::NoU16(16)); } };
    let machine = match u16_at(d, 18) { Some(v) => v, None => { return Err(ElfErr::NoU16(18)); } };
    let entry = match u32_at(d, 24) { Some(v) => v, None => { return Err(ElfErr::NoU32(24)); } };
    let phoff = match u32_at(d, 28) { Some(v) => v, None => { return Err(ElfErr::NoU32(28)); } };
    let shoff = match u32_at(d, 32) { Some(v) => v, None => { return Err(ElfErr::NoU32(32)); } };
    let flags = match u32_at(d, 36) { Some(v) => v, None => { return Err(ElfErr::NoU32(36)); } };
    let phentsize = match u16_at(d, 42) { Some(v) => v, None => { return Err(ElfErr::NoU16(42)); } };
    let phnum = match u16_at(d, 44) { Some(v) => v, None => { return Err(ElfErr::NoU16(44)); } };
    let shentsize = match u16_at(d, 46) { Some(v) => v, None => { return Err(ElfErr::NoU16(46)); } };
    let shnum = match u16_at(d, 48) { Some(v) => v, None => { return Err(ElfErr::NoU16(48)); } };
    let shstrndx = match u16_at(d, 50) { Some(v) => v, None => { return Err(ElfErr::NoU16(50)); } };
    let header = Header { kind, machine, entry, flags, phoff, shoff, phentsize, phnum, shentsize, shnum, shstrndx };
    let segments = match rd_segments(d, phoff, phentsize, phnum) { Ok(v) => v, Err(x) => { return Err(x); } };
    let sections = match rd_sections(d, shoff, shentsize, shnum) { Ok(v) => v, Err(x) => { return Err(x); } };
    let names = if (shstrndx as usize) < sections.len() {
        match rd_names(d, &sections, sections[shstrndx as usize].offset) { Ok(v) => v, Err(x) => { return Err(x); } }
    } else {
        no_names(sections.len())
    };
    Ok(Parsed { header, segments, sections, names })
}

/// A section's extent in a file of `len` bytes.
pub fn section_range(len: usize, s: &SecHdr) -> (r: Option<(usize, usize)>)
    ensures
        match sec_range(len as int, *s) {
            Some(x) => r == Some((x.0 as usize, x.1 as usize)) && x.0 <= x.1 <= len,
            None => r is None,
        },
{
    if s.kind == SHT_NOBITS {
        return Some((0, 0));
    }
    let end = s.offset as u64 + s.size as u64;
    if end > len as u64 {
        return None;
    }
    Some((s.offset as usize, end as usize))
}

/// The entry of `e` bytes at `b`, of index `k` in table `t`, with its name
/// in the string table `[ss, se)`.
fn sym_entry(d: &[u8], ss: usize, se: usize, b: usize, e: usize, t: usize, k: usize) -> (r: Result<RawSym, ElfErr>)
    requires ss <= se <= d@.len(), b + e <= d@.len(),
    ensures r is Ok ==> entry_ok(d@, ss as int, se as int, b as int, e as int, r.unwrap()) && r.unwrap().table == t && r.unwrap().index == k,
{
    let _n = d.len();
    if e < 4 {
        return Err(ElfErr::NoU32(0));
    }
    let no = match u32_at(d, b as u64) { Some(v) => v, None => { return Err(ElfErr::NoU32(0)); } };
    let info: u8 = if e > 12 {
        assert(b + 12 < _n);
        d[b + 12]
    } else {
        0
    };
    if no as u64 > (se - ss) as u64 {
        return Err(ElfErr::StrOutside(no as u64));
    }
    let name_start = ss + no as usize;
    let name_end = str_end(d, name_start, se);
    if e < 8 {
        return Err(ElfErr::NoU32(4));
    }
    let value = match u32_at(d, b as u64 + 4) { Some(v) => v, None => { return Err(ElfErr::NoU32(4)); } };
    if e < 12 {
        return Err(ElfErr::NoU32(8));
    }
    let size = match u32_at(d, b as u64 + 8) { Some(v) => v, None => { return Err(ElfErr::NoU32(8)); } };
    if e < 16 {
        return Err(ElfErr::NoU16(14));
    }
    let shndx = match u16_at(d, b as u64 + 14) { Some(v) => v, None => { return Err(ElfErr::NoU16(14)); } };
    Ok(RawSym { table: t, index: k, name_start, name_end, value, size, info, shndx })
}

/// The entries of `e` bytes of the table `[ts, te)`, appended to `acc`.
fn sym_entries(d: &[u8], ss: usize, se: usize, ts: usize, te: usize, e: usize, t: usize, acc: Vec<RawSym>) -> (r: Result<Vec<RawSym>, ElfErr>)
    requires ss <= se <= d@.len(), ts <= te <= d@.len(), e >= 1,
    ensures
        r is Ok ==> {
            let v = r.unwrap()@;
            let c = v.len() - acc@.len();
            &&& c >= 0 && v.subrange(0, acc@.len() as int) == acc@
            &&& ts + c * e <= te < ts + (c + 1) * e
            &&& forall|k: int| 0 <= k < c ==> {
                &&& (#[trigger] v[acc@.len() + k]).table == t
                &&& v[acc@.len() + k].index == k
                &&& entry_ok(d@, ss as int, se as int, ts + k * e, e as int, v[acc@.len() + k])
            }
        },
{
    let ghost n0 = acc@.len();
    let ghost a0 = acc@;
    let mut out = acc;
    let mut pos = ts;
    let mut k: usize = 0;
    while te - pos >= e
        invariant
            ss <= se <= d@.len(),
            ts <= pos <= te,
            te <= d@.len(),
            e >= 1,
            pos == ts + k * e,
            k <= pos - ts,
            out@.len() == n0 + k,
            out@.subrange(0, n0 as int) == a0,
            forall|j: int| 0 <= j < k ==> {
                &&& (#[trigger] out@[n0 + j]).table == t
                &&& out@[n0 + j].index == j
                &&& entry_ok(d@, ss as int, se as int, ts + j * e, e as int, out@[n0 + j])
            },
        decreases te - pos,
    {
        let s = match sym_entry(d, ss, se, pos, e, t, k) { Ok(v) => v, Err(x) => { return Err(x); } };
        let ghost prev = out@;
        out.push(s);
        proof {
            assert((k + 1) * e == k * e + e) by (nonlinear_arith);
            assert(out@.subrange(0, n0 as int) =~= prev.subrange(0, n0 as int));
            assert forall|j: int| 0 <= j < k + 1 implies {
                &&& (#[trigger] out@[n0 + j]).table == t
                &&& out@[n0 + j].index == j
                &&& entry_ok(d@, ss as int, se as int, ts + j * e, e as int, out@[n0 + j])
            } by {
                if j < k {
                    assert(out@[n0 + j] == prev[n0 + j]);
                }
            }
        }
        pos += e;
        k += 1;
    }
    proof {
        assert((k + 1) * e == k * e + e) by (nonlinear_arith);
    }
    Ok(out)
}

proof fn lemma_count(c: int, k: int, e: int)
    requires e >= 1, k >= 0, c >= 0, k * e + e < (c + 1) * e,
    ensures k < c,
{
    if k >= c {
        assert(k * e + e >= (c + 1) * e) by (nonlinear_arith)
            requires k >= c, e >= 1;
    }
}

/// A section that is no symbol table has no entry.
proof fn lemma_skip(d: Seq<u8>, secs: Seq<SecHdr>, v: Seq<RawSym>, t: int)
    requires syms_ok(d, secs, v, t), 0 <= t < secs.len(), secs[t].kind != SHT_SYMTAB,
    ensures syms_ok(d, secs, v, t + 1),
{
    assert forall|tt: int, k: int| tt < t + 1 && #[trigger] valid_entry(d, secs, tt, k) implies has(v, tt, k) by {
        if tt == t {
            assert(!valid_entry(d, secs, tt, k));
        }
    }
}

/// The entries of table `t` appended to the entries of the tables before
/// it are the entries of the tables up to it.
proof fn lemma_extend(d: Seq<u8>, secs: Seq<SecHdr>, prev: Seq<RawSym>, v: Seq<RawSym>, t: int, ss: int, se: int, ts: int, te: int, e: int)
    requires
        syms_ok(d, secs, prev, t),
        0 <= t < secs.len(),
        secs[t].kind == SHT_SYMTAB,
        secs[t].link < secs.len(),
        sec_range(d.len() as int, secs[secs[t].link as int]) == Some((ss, se)),
        sec_range(d.len() as int, secs[t]) == Some((ts, te)),
        e == ent_size(secs[t]),
        v.len() >= prev.len(),
        v.subrange(0, prev.len() as int) == prev,
        ts + (v.len() - prev.len()) * e <= te < ts + (v.len() - prev.len() + 1) * e,
        forall|k: int| 0 <= k < v.len() - prev.len() ==> {
            &&& (#[trigger] v[prev.len() + k]).table == t
            &&& v[prev.len() + k].index == k
            &&& entry_ok(d, ss, se, ts + k * e, e, v[prev.len() + k])
        },
    ensures syms_ok(d, secs, v, t + 1),
{
    let n0 = prev.len() as int;
    let c = v.len() - n0;
    assert forall|j: int| 0 <= j < n0 implies v[j] == prev[j] by {
        assert(v[j] == v.subrange(0, n0)[j]);
    }
    assert forall|j: int| 0 <= j < v.len() implies sym_ok(d, secs, #[trigger] v[j]) && v[j].table < t + 1 by {
        if j < n0 {
            assert(v[j] == prev[j]);
        } else {
            let k = j - n0;
            assert(v[j] == v[n0 + k]);
            assert(k * e + e <= c * e) by (nonlinear_arith)
                requires k + 1 <= c, e >= 1;
            assert(valid_entry(d, secs, t, k));
        }
    }
    assert forall|i: int, j: int| 0 <= i < j < v.len() implies before(#[trigger] v[i], #[trigger] v[j]) by {
        if i < n0 {
            assert(v[i] == prev[i]);
        } else {
            assert(v[i] == v[n0 + (i - n0)]);
        }
        if j < n0 {
            assert(v[j] == prev[j]);
        } else {
            assert(v[j] == v[n0 + (j - n0)]);
        }
    }
    assert forall|tt: int, k: int| tt < t + 1 && #[trigger] valid_entry(d, secs, tt, k) implies has(v, tt, k) by {
        if tt < t {
            assert(has(prev, tt, k));
            let j = choose|j: int| 0 <= j < prev.len() && (#[trigger] prev[j]).table == tt && prev[j].index == k;
            assert(v[j] == prev[j]);
        } else {
            lemma_count(c, k, e);
            assert(v[n0 + k].table == tt && v[n0 + k].index == k);
        }
    }
}

/// The symbols of every symbol table, in table order.
pub fn symbols(d: &[u8], secs: &Vec<SecHdr>) -> (r: Result<Vec<RawSym>, ElfErr>)
    ensures r is Ok ==> syms_ok(d@, secs@, r.unwrap()@, secs@.len() as int),
{
    let mut out: Vec<RawSym> = Vec::new();
    let mut t: usize = 0;
    while t < secs.len()
        invariant t <= secs@.len(), syms_ok(d@, secs@, out@, t as int),
        decreases secs@.len() - t,
    {
        if secs[t].kind == SHT_SYMTAB {
            let link = secs[t].link as usize;
            if link >= secs.len() {
                return Err(ElfErr::MissingStrtab);
            }
            let (ss, se) = match section_range(d.len(), &secs[link]) { Some(x) => x, None => { return Err(ElfErr::SectionOutside(link)); } };
            let (ts, te) = match section_range(d.len(), &secs[t]) { Some(x) => x, None => { return Err(ElfErr::SectionOutside(t)); } };
            let e = if secs[t].entsize == 0 { 16 } else { secs[t].entsize as usize };
            let ghost prev = out@;
            out = match sym_entries(d, ss, se, ts, te, e, t, out) { Ok(v) => v, Err(x) => { return Err(x); } };
            proof {
                lemma_extend(d@, secs@, prev, out@, t as int, ss as int, se as int, ts as int, te as int, e as int);
            }
        } else {
            proof {
                lemma_skip(d@, secs@, out@, t as int);
            }
        }
        t += 1;
    }
    Ok(out)
}

/// Whether the symbol's name is `name`, byte for byte.
pub fn name_is(d: &[u8], s: &RawSym, name: &[u8]) -> (r: bool)
    requires s.name_start <= s.name_end <= d@.len(),
    ensures r == (name_of(d@, *s) == name@),
{
    if s.name_end - s.name_start != name.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < name.len()
        invariant
            i <= name@.len(),
            s.name_end - s.name_start == name@.len(),
            s.name_end <= d@.len(),
            forall|j: int| 0 <= j < i ==> d@[s.name_start + j] == name@[j],
        decreases name@.len() - i,
    {
        if d[s.name_start + i] != name[i] {
            assert(name_of(d@, *s)[i as int] != name@[i as int]);
            return false;
        }
        i += 1;
    }
    assert(name_of(d@, *s) =~= name@);
    true
}

/// The index of the first of `syms` that is wanted.
pub fn find(d: &[u8], syms: &Vec<RawSym>, name: &[u8], object: bool) -> (r: Option<usize>)
    requires forall|j: int| 0 <= j < syms@.len() ==> (#[trigger] syms@[j]).name_start <= syms@[j].name_end <= d@.len(),
    ensures
        match r {
            Some(i) => i < syms@.len() && wanted(d@, syms@[i as int], name@, object) && forall|j: int| 0 <= j < i ==> !wanted(d@, #[trigger] syms@[j], name@, object),
            None => forall|j: int| 0 <= j < syms@.len() ==> !wanted(d@, #[trigger] syms@[j], name@, object),
        },
{
    let mut i: usize = 0;
    while i < syms.len()
        invariant
            i <= syms@.len(),
            forall|j: int| 0 <= j < syms@.len() ==> (#[trigger] syms@[j]).name_start <= syms@[j].name_end <= d@.len(),
            forall|j: int| 0 <= j < i ==> !wanted(d@, #[trigger] syms@[j], name@, object),
        decreases syms@.len() - i,
    {
        if (!object || syms[i].info & 0xf == 1) && name_is(d, &syms[i], name) {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// Two entries of one index in one table are one entry.
proof fn lemma_sym_unique(d: Seq<u8>, secs: Seq<SecHdr>, a: RawSym, b: RawSym)
    requires sym_ok(d, secs, a), sym_ok(d, secs, b), a.table == b.table, a.index == b.index,
    ensures a == b,
{
    let se = sec_range(d.len() as int, secs[secs[a.table as int].link as int]).unwrap().1;
    assert(a.name_start == b.name_start);
    assert(is_str(d, a.name_start as int, se, a.name_end as int));
    assert(is_str(d, b.name_start as int, se, b.name_end as int));
    if a.name_end < b.name_end {
        assert(d[a.name_end as int] != 0);
    }
    if b.name_end < a.name_end {
        assert(d[b.name_end as int] != 0);
    }
}

/// The first symbol of the name, an object when `object`: the lookup the
/// Config_Checker finds its images by.
pub fn lookup(d: &[u8], secs: &Vec<SecHdr>, name: &[u8], object: bool) -> (r: Result<Option<RawSym>, ElfErr>)
    ensures
        match r {
            Ok(Some(s)) => {
                &&& sym_ok(d@, secs@, s)
                &&& wanted(d@, s, name@, object)
                &&& forall|o: RawSym| #[trigger] sym_ok(d@, secs@, o) && wanted(d@, o, name@, object) ==> !before(o, s)
            },
            Ok(None) => forall|o: RawSym| #[trigger] sym_ok(d@, secs@, o) ==> !wanted(d@, o, name@, object),
            Err(_) => true,
        },
{
    let syms = match symbols(d, secs) { Ok(v) => v, Err(x) => { return Err(x); } };
    let found = find(d, &syms, name, object);
    proof {
        assert forall|o: RawSym| #[trigger] sym_ok(d@, secs@, o) implies exists|j: int| 0 <= j < syms@.len() && syms@[j] == o by {
            assert(valid_entry(d@, secs@, o.table as int, o.index as int));
            assert(has(syms@, o.table as int, o.index as int));
            let j = choose|j: int| 0 <= j < syms@.len() && (#[trigger] syms@[j]).table == o.table as int && syms@[j].index == o.index as int;
            lemma_sym_unique(d@, secs@, syms@[j], o);
        }
    }
    match found {
        Some(i) => {
            proof {
                assert forall|o: RawSym| #[trigger] sym_ok(d@, secs@, o) && wanted(d@, o, name@, object) implies !before(o, syms@[i as int]) by {
                    let j = choose|j: int| 0 <= j < syms@.len() && syms@[j] == o;
                    if j < i {
                        assert(!wanted(d@, syms@[j], name@, object));
                    }
                    if j > i {
                        assert(before(syms@[i as int], syms@[j]));
                    }
                }
            }
            Ok(Some(syms[i]))
        }
        None => {
            proof {
                assert forall|o: RawSym| #[trigger] sym_ok(d@, secs@, o) implies !wanted(d@, o, name@, object) by {
                    let j = choose|j: int| 0 <= j < syms@.len() && syms@[j] == o;
                    assert(!wanted(d@, syms@[j], name@, object));
                }
            }
            Ok(None)
        }
    }
}

/// The first allocated, non-empty section that holds the address.
pub fn section_at(secs: &Vec<SecHdr>, addr: u32) -> (r: Option<usize>)
    ensures
        match r {
            Some(i) => first_holding(secs@, addr, i as int),
            None => forall|j: int| 0 <= j < secs@.len() ==> !sec_holds(#[trigger] secs@[j], addr),
        },
{
    let mut i: usize = 0;
    while i < secs.len()
        invariant i <= secs@.len(), forall|j: int| 0 <= j < i ==> !sec_holds(#[trigger] secs@[j], addr),
        decreases secs@.len() - i,
    {
        if secs[i].flags & SHF_ALLOC != 0 && secs[i].size > 0 && addr >= secs[i].addr && (addr as u64) < secs[i].addr as u64 + secs[i].size as u64 {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The `len` bytes of the file from `start` on.
fn copy_range(d: &[u8], start: usize, len: usize) -> (r: Vec<u8>)
    requires start + len <= d@.len(),
    ensures r@ == d@.subrange(start as int, start + len),
{
    let _n = d.len();
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < len
        invariant i <= len, start + len <= _n, _n == d@.len(), out@ == d@.subrange(start as int, start + i),
        decreases len - i,
    {
        out.push(d[start + i]);
        assert(out@ =~= d@.subrange(start as int, start + i + 1));
        i += 1;
    }
    out
}

/// `len` zero bytes.
fn zeros(len: usize) -> (r: Vec<u8>)
    ensures r@ == Seq::new(len as nat, |i: int| 0u8),
{
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < len
        invariant i <= len, out@ == Seq::new(i as nat, |j: int| 0u8),
        decreases len - i,
    {
        out.push(0);
        assert(out@ =~= Seq::new((i + 1) as nat, |j: int| 0u8));
        i += 1;
    }
    out
}

/// The `len` bytes at the virtual address `addr`: the file's bytes of the
/// first allocated section that holds the address, zeros for a section
/// without file content.
pub fn read_at(d: &[u8], secs: &Vec<SecHdr>, addr: u32, len: usize) -> (r: Result<Vec<u8>, AtErr>)
    ensures
        r is Ok ==> exists|i: int| first_holding(secs@, addr, i) && bytes_at(d@, #[trigger] secs@[i], addr, len as int, r.unwrap()@),
        r == Err::<Vec<u8>, AtErr>(AtErr::NoSection) ==> forall|j: int| 0 <= j < secs@.len() ==> !sec_holds(#[trigger] secs@[j], addr),
{
    let i = match section_at(secs, addr) { Some(i) => i, None => { return Err(AtErr::NoSection); } };
    let s = secs[i];
    let room = s.addr as u64 + s.size as u64 - addr as u64;
    if len as u64 > room {
        return Err(AtErr::Crosses(i));
    }
    if s.kind == SHT_NOBITS {
        let v = zeros(len);
        assert(bytes_at(d@, secs@[i as int], addr, len as int, v@));
        return Ok(v);
    }
    let start = s.offset as u64 + (addr - s.addr) as u64;
    let n = d.len() as u64;
    if len as u64 > n || start > n - len as u64 {
        return Err(AtErr::Outside);
    }
    let v = copy_range(d, start as usize, len);
    assert(bytes_at(d@, secs@[i as int], addr, len as int, v@));
    Ok(v)
}

} // verus!
