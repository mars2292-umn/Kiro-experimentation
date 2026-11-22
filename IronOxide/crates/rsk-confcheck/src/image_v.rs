//! The decoding of the `CONFIG` image and the comparison of the decoded
//! tables, verified (R26.4; 2026-11-21).
//!
//! `spec_image` states the image as this crate reads the Kernel's
//! documentation of it: the three counts as little-endian `u32`, then per
//! Task its priority, Partition, kind, offset, period, deadline (`u64`),
//! Budget (`u32`), MIT policy and floating-point flag; per Resource its
//! Ceiling and Partition; the access matrix Task-major, one byte per pair;
//! per Partition its level, three responses and MIT threshold (`u32`); the
//! operating duration (`u64`); the three slot tables; the slot count. The
//! statement is this crate's own (R26.1): nothing of it is imported from
//! the Kernel or the Generator, and that it is the Kernel's order is what
//! the Config_Checker's runs on linked binaries show.
//!
//! | Function | Proven |
//! |---|---|
//! | `decode_tables` | it returns for every byte string (no index out of range, no overflow); when it returns tables `t`, the tables are well formed (`wf`: every table has the length its count says, every access row one entry per Resource) and the input is exactly `spec_image(t@)`: every byte was read, in order, none skipped and none left over, and a flag byte is 0 or 1 |
//! | `tables_agree` | when it returns `true`, the two tables have equal views: every field of every entry |
//! | `image_is` | when it returns `true`, the bytes are exactly `spec_image` of the expected tables |
//!
//! `image_is` is the statement the checker's verdict rests on: `recompute`
//! builds the expected tables from the Task_Model and reports a
//! discrepancy unless `image_is` holds of them and the image extracted
//! from the binary. So a run without a discrepancy means the binary's
//! image is, byte for byte, the encoding of the tables computed from the
//! Task_Model. What stays ordinary Rust: reading the Task_Model, building
//! the expected tables from it, finding the image in the ELF file, and the
//! per-field report for the reader.

use vstd::prelude::*;

verus! {

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

/// Why an image is not a `CONFIG` image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecodeError {
    /// The image ends before the tables do.
    TooShort,
    /// A count above 255.
    Counts,
    /// A flag byte that is neither 0 nor 1, at this offset.
    Flag(usize),
    /// This many bytes after the tables.
    Trailing(usize),
}

/// The tables as values: what `spec_image` and the comparison speak of.
pub struct TablesV {
    pub tasks: Seq<TaskCfg>,
    pub resources: Seq<ResourceCfg>,
    pub access: Seq<Seq<bool>>,
    pub partitions: Seq<PartitionCfg>,
    pub operating_duration: u64,
    pub slot_release: Seq<u8>,
    pub slot_deadline: Seq<u8>,
    pub slot_budget: Seq<u8>,
    pub slot_count: u8,
}

pub open spec fn rows_view(a: Seq<Vec<bool>>) -> Seq<Seq<bool>> {
    Seq::new(a.len(), |i: int| a[i]@)
}

impl Tables {
    pub open spec fn view(&self) -> TablesV {
        TablesV {
            tasks: self.tasks@,
            resources: self.resources@,
            access: rows_view(self.access@),
            partitions: self.partitions@,
            operating_duration: self.operating_duration,
            slot_release: self.slot_release@,
            slot_deadline: self.slot_deadline@,
            slot_budget: self.slot_budget@,
            slot_count: self.slot_count,
        }
    }
}

/// Every table has the length its count says.
pub open spec fn wf(t: TablesV) -> bool {
    &&& t.tasks.len() <= 255 && t.resources.len() <= 255 && t.partitions.len() <= 255
    &&& t.access.len() == t.tasks.len()
    &&& forall|i: int| 0 <= i < t.access.len() ==> (#[trigger] t.access[i]).len() == t.resources.len()
    &&& t.slot_release.len() == t.tasks.len()
    &&& t.slot_deadline.len() == t.tasks.len()
    &&& t.slot_budget.len() == t.tasks.len()
}

// ------------------------------------------------------- the encoding --

/// A `u32`, least significant byte first.
pub open spec fn enc_u32(v: u32) -> Seq<u8> {
    seq![(v & 0xff) as u8, ((v >> 8) & 0xff) as u8, ((v >> 16) & 0xff) as u8, ((v >> 24) & 0xff) as u8]
}

/// A `u64`: its low half, then its high half.
pub open spec fn enc_u64(v: u64) -> Seq<u8> {
    enc_u32((v & 0xffff_ffff) as u32) + enc_u32((v >> 32) as u32)
}

pub open spec fn enc_flag(b: bool) -> Seq<u8> {
    seq![if b { 1u8 } else { 0u8 }]
}

pub open spec fn enc_task(t: TaskCfg) -> Seq<u8> {
    seq![t.priority, t.partition, t.kind] + enc_u64(t.offset) + enc_u64(t.period) + enc_u64(t.deadline) + enc_u32(t.budget) + seq![t.mit_policy] + enc_flag(t.fpu)
}

pub open spec fn enc_tasks(s: Seq<TaskCfg>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_tasks(s.drop_last()) + enc_task(s.last()) }
}

pub open spec fn enc_resource(r: ResourceCfg) -> Seq<u8> {
    seq![r.ceiling, r.partition]
}

pub open spec fn enc_resources(s: Seq<ResourceCfg>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_resources(s.drop_last()) + enc_resource(s.last()) }
}

pub open spec fn enc_flags(s: Seq<bool>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_flags(s.drop_last()) + enc_flag(s.last()) }
}

pub open spec fn enc_rows(s: Seq<Seq<bool>>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_rows(s.drop_last()) + enc_flags(s.last()) }
}

pub open spec fn enc_partition(p: PartitionCfg) -> Seq<u8> {
    seq![p.level, p.fault_response, p.overrun_response, p.deadline_response] + enc_u32(p.mit_threshold)
}

pub open spec fn enc_partitions(s: Seq<PartitionCfg>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_partitions(s.drop_last()) + enc_partition(s.last()) }
}

/// The `CONFIG` image of the tables `t`.
pub open spec fn spec_image(t: TablesV) -> Seq<u8> {
    enc_u32(t.tasks.len() as u32) + enc_u32(t.resources.len() as u32) + enc_u32(t.partitions.len() as u32)
        + enc_tasks(t.tasks) + enc_resources(t.resources) + enc_rows(t.access) + enc_partitions(t.partitions)
        + enc_u64(t.operating_duration) + t.slot_release + t.slot_deadline + t.slot_budget + seq![t.slot_count]
}

// --------------------------------------------------------- the reader --

/// The bytes read so far.
pub open spec fn read(bytes: Seq<u8>, pos: usize) -> Seq<u8> {
    bytes.subrange(0, pos as int)
}

proof fn lemma_le32(b0: u8, b1: u8, b2: u8, b3: u8)
    ensures ({
        let v = (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16) | ((b3 as u32) << 24);
        &&& (v & 0xff) as u8 == b0
        &&& ((v >> 8) & 0xff) as u8 == b1
        &&& ((v >> 16) & 0xff) as u8 == b2
        &&& ((v >> 24) & 0xff) as u8 == b3
    }),
{
    let v = (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16) | ((b3 as u32) << 24);
    assert((v & 0xff) as u8 == b0 && ((v >> 8) & 0xff) as u8 == b1 && ((v >> 16) & 0xff) as u8 == b2 && ((v >> 24) & 0xff) as u8 == b3) by (bit_vector)
        requires v == (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16) | ((b3 as u32) << 24);
}

proof fn lemma_le64(lo: u32, hi: u32)
    ensures ({
        let v = (lo as u64) | ((hi as u64) << 32);
        &&& (v & 0xffff_ffff) as u32 == lo
        &&& (v >> 32) as u32 == hi
    }),
{
    let v = (lo as u64) | ((hi as u64) << 32);
    assert((v & 0xffff_ffff) as u32 == lo && (v >> 32) as u32 == hi) by (bit_vector)
        requires v == (lo as u64) | ((hi as u64) << 32);
}

/// One byte, or `None` at the end of the image.
fn rd_u8(bytes: &[u8], pos: &mut usize) -> (r: Option<u8>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        match r {
            Some(b) => *final(pos) == *old(pos) + 1 && read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + seq![b],
            None => *final(pos) == *old(pos),
        },
{
    if *pos >= bytes.len() {
        return None;
    }
    let b = bytes[*pos];
    proof {
        assert(bytes@.subrange(0, *pos + 1) == bytes@.subrange(0, *pos as int) + seq![b]);
    }
    *pos = *pos + 1;
    Some(b)
}

/// A flag byte: 0 or 1, anything else refused.
fn rd_flag(bytes: &[u8], pos: &mut usize) -> (r: Result<bool, DecodeError>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Ok ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_flag(r.unwrap()),
{
    let at = *pos;
    match rd_u8(bytes, pos) {
        None => Err(DecodeError::TooShort),
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        Some(_) => Err(DecodeError::Flag(at)),
    }
}

fn rd_u32(bytes: &[u8], pos: &mut usize) -> (r: Option<u32>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Some ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_u32(r.unwrap()),
{
    let ghost start = read(bytes@, *pos);
    let b0 = match rd_u8(bytes, pos) { Some(b) => b, None => { return None; } };
    let b1 = match rd_u8(bytes, pos) { Some(b) => b, None => { return None; } };
    let b2 = match rd_u8(bytes, pos) { Some(b) => b, None => { return None; } };
    let b3 = match rd_u8(bytes, pos) { Some(b) => b, None => { return None; } };
    let v = (b0 as u32) | ((b1 as u32) << 8) | ((b2 as u32) << 16) | ((b3 as u32) << 24);
    proof {
        lemma_le32(b0, b1, b2, b3);
        assert(enc_u32(v) =~= seq![b0, b1, b2, b3]);
        assert(read(bytes@, *pos) =~= start + enc_u32(v));
    }
    Some(v)
}

fn rd_u64(bytes: &[u8], pos: &mut usize) -> (r: Option<u64>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Some ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_u64(r.unwrap()),
{
    let ghost start = read(bytes@, *pos);
    let lo = match rd_u32(bytes, pos) { Some(v) => v, None => { return None; } };
    let hi = match rd_u32(bytes, pos) { Some(v) => v, None => { return None; } };
    let v = (lo as u64) | ((hi as u64) << 32);
    proof {
        lemma_le64(lo, hi);
        assert(read(bytes@, *pos) =~= start + enc_u64(v));
    }
    Some(v)
}

fn rd_task(bytes: &[u8], pos: &mut usize) -> (r: Result<TaskCfg, DecodeError>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Ok ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_task(r.unwrap()),
{
    let ghost start = read(bytes@, *pos);
    let priority = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let partition = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let kind = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let offset = match rd_u64(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let period = match rd_u64(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let deadline = match rd_u64(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let budget = match rd_u32(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let mit_policy = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let fpu = match rd_flag(bytes, pos) { Ok(v) => v, Err(e) => { return Err(e); } };
    let t = TaskCfg { priority, partition, kind, offset, period, deadline, budget, mit_policy, fpu };
    proof {
        assert(read(bytes@, *pos) =~= start + enc_task(t));
    }
    Ok(t)
}

fn rd_partition(bytes: &[u8], pos: &mut usize) -> (r: Option<PartitionCfg>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Some ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_partition(r.unwrap()),
{
    let ghost start = read(bytes@, *pos);
    let level = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let fault_response = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let overrun_response = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let deadline_response = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let mit_threshold = match rd_u32(bytes, pos) { Some(v) => v, None => { return None; } };
    let p = PartitionCfg { level, fault_response, overrun_response, deadline_response, mit_threshold };
    proof {
        assert(read(bytes@, *pos) =~= start + enc_partition(p));
    }
    Some(p)
}

/// `n` bytes as they are (a slot table).
fn rd_bytes(bytes: &[u8], pos: &mut usize, n: usize) -> (r: Option<Vec<u8>>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Some ==> r.unwrap()@.len() == n && read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + r.unwrap()@,
{
    let ghost start = read(bytes@, *pos);
    let mut out: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n,
            out@.len() == i,
            *pos <= bytes@.len(),
            read(bytes@, *pos) == start + out@,
        decreases n - i,
    {
        let b = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
        out.push(b);
        proof {
            assert(read(bytes@, *pos) =~= start + out@);
        }
        i += 1;
    }
    Some(out)
}

/// One row of the access matrix: `n` flags.
fn rd_row(bytes: &[u8], pos: &mut usize, n: usize) -> (r: Result<Vec<bool>, DecodeError>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Ok ==> r.unwrap()@.len() == n && read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_flags(r.unwrap()@),
{
    let ghost start = read(bytes@, *pos);
    let mut out: Vec<bool> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n,
            out@.len() == i,
            *pos <= bytes@.len(),
            read(bytes@, *pos) == start + enc_flags(out@),
        decreases n - i,
    {
        let ghost before = out@;
        let b = match rd_flag(bytes, pos) { Ok(v) => v, Err(e) => { return Err(e); } };
        out.push(b);
        proof {
            assert(out@.drop_last() =~= before);
            assert(read(bytes@, *pos) =~= start + enc_flags(out@));
        }
        i += 1;
    }
    Ok(out)
}

/// R26.4: the tables of a `CONFIG` image. Returns for every input; what it
/// accepts is exactly the encoding of what it returns.
pub fn decode_tables(bytes: &[u8]) -> (r: Result<Tables, DecodeError>)
    ensures r is Ok ==> wf(r.unwrap()@) && bytes@ == spec_image(r.unwrap()@),
{
    let mut pos: usize = 0;
    assert(read(bytes@, 0) =~= Seq::<u8>::empty());
    let nt32 = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let nr32 = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let np32 = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    if nt32 > 255 || nr32 > 255 || np32 > 255 {
        return Err(DecodeError::Counts);
    }
    let nt = nt32 as usize;
    let nr = nr32 as usize;
    let np = np32 as usize;
    let ghost head = read(bytes@, pos);
    assert(head =~= enc_u32(nt32) + enc_u32(nr32) + enc_u32(np32));

    let mut tasks: Vec<TaskCfg> = Vec::new();
    let mut i: usize = 0;
    while i < nt
        invariant
            i <= nt, tasks@.len() == i, pos <= bytes@.len(),
            read(bytes@, pos) == head + enc_tasks(tasks@),
        decreases nt - i,
    {
        let ghost before = tasks@;
        let t = match rd_task(bytes, &mut pos) { Ok(v) => v, Err(e) => { return Err(e); } };
        tasks.push(t);
        proof {
            assert(tasks@.drop_last() =~= before);
            assert(read(bytes@, pos) =~= head + enc_tasks(tasks@));
        }
        i += 1;
    }
    let ghost after_tasks = read(bytes@, pos);

    let mut resources: Vec<ResourceCfg> = Vec::new();
    let mut i: usize = 0;
    while i < nr
        invariant
            i <= nr, resources@.len() == i, pos <= bytes@.len(),
            read(bytes@, pos) == after_tasks + enc_resources(resources@),
        decreases nr - i,
    {
        let ghost before = resources@;
        let ghost start = read(bytes@, pos);
        let ceiling = match rd_u8(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        let partition = match rd_u8(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        let res = ResourceCfg { ceiling, partition };
        resources.push(res);
        proof {
            assert(read(bytes@, pos) =~= start + enc_resource(res));
            assert(resources@.drop_last() =~= before);
            assert(read(bytes@, pos) =~= after_tasks + enc_resources(resources@));
        }
        i += 1;
    }
    let ghost after_resources = read(bytes@, pos);

    let mut access: Vec<Vec<bool>> = Vec::new();
    let mut i: usize = 0;
    while i < nt
        invariant
            i <= nt, access@.len() == i, pos <= bytes@.len(),
            forall|k: int| 0 <= k < i ==> (#[trigger] access@[k])@.len() == nr,
            read(bytes@, pos) == after_resources + enc_rows(rows_view(access@)),
        decreases nt - i,
    {
        let ghost before = rows_view(access@);
        let row = match rd_row(bytes, &mut pos, nr) { Ok(v) => v, Err(e) => { return Err(e); } };
        let ghost row_v = row@;
        access.push(row);
        proof {
            assert(rows_view(access@).drop_last() =~= before);
            assert(rows_view(access@).last() == row_v);
            assert(read(bytes@, pos) =~= after_resources + enc_rows(rows_view(access@)));
        }
        i += 1;
    }
    let ghost after_access = read(bytes@, pos);

    let mut partitions: Vec<PartitionCfg> = Vec::new();
    let mut i: usize = 0;
    while i < np
        invariant
            i <= np, partitions@.len() == i, pos <= bytes@.len(),
            read(bytes@, pos) == after_access + enc_partitions(partitions@),
        decreases np - i,
    {
        let ghost before = partitions@;
        let p = match rd_partition(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        partitions.push(p);
        proof {
            assert(partitions@.drop_last() =~= before);
            assert(read(bytes@, pos) =~= after_access + enc_partitions(partitions@));
        }
        i += 1;
    }
    let ghost after_partitions = read(bytes@, pos);

    let operating_duration = match rd_u64(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let slot_release = match rd_bytes(bytes, &mut pos, nt) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let slot_deadline = match rd_bytes(bytes, &mut pos, nt) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let slot_budget = match rd_bytes(bytes, &mut pos, nt) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let slot_count = match rd_u8(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    if pos != bytes.len() {
        return Err(DecodeError::Trailing(bytes.len() - pos));
    }
    let t = Tables { tasks, resources, access, partitions, operating_duration, slot_release, slot_deadline, slot_budget, slot_count };
    proof {
        let v = t@;
        assert(v.tasks.len() as u32 == nt32 && v.resources.len() as u32 == nr32 && v.partitions.len() as u32 == np32);
        assert(read(bytes@, pos) =~= bytes@);
        assert(forall|k: int| 0 <= k < v.access.len() ==> (#[trigger] v.access[k]).len() == v.resources.len());
        assert(bytes@ =~= spec_image(v));
    }
    Ok(t)
}

// ----------------------------------------------------- the comparison --

fn bytes_agree(a: &Vec<u8>, b: &Vec<u8>) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len()
        invariant i <= a@.len(), a@.len() == b@.len(), forall|k: int| 0 <= k < i ==> a@[k] == b@[k],
        decreases a@.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    assert(a@ =~= b@);
    true
}

fn flags_agree(a: &Vec<bool>, b: &Vec<bool>) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len()
        invariant i <= a@.len(), a@.len() == b@.len(), forall|k: int| 0 <= k < i ==> a@[k] == b@[k],
        decreases a@.len() - i,
    {
        if a[i] != b[i] {
            return false;
        }
        i += 1;
    }
    assert(a@ =~= b@);
    true
}

fn task_agrees(a: &TaskCfg, b: &TaskCfg) -> (r: bool)
    ensures r ==> *a == *b,
{
    a.priority == b.priority && a.partition == b.partition && a.kind == b.kind && a.offset == b.offset && a.period == b.period && a.deadline == b.deadline && a.budget == b.budget
        && a.mit_policy == b.mit_policy && a.fpu == b.fpu
}

/// R26.4: `true` only when the two tables agree in every field of every
/// entry.
pub fn tables_agree(a: &Tables, b: &Tables) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if a.tasks.len() != b.tasks.len() || a.resources.len() != b.resources.len() || a.access.len() != b.access.len() || a.partitions.len() != b.partitions.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.tasks.len()
        invariant i <= a.tasks@.len(), a.tasks@.len() == b.tasks@.len(), forall|k: int| 0 <= k < i ==> a.tasks@[k] == b.tasks@[k],
        decreases a.tasks@.len() - i,
    {
        if !task_agrees(&a.tasks[i], &b.tasks[i]) {
            return false;
        }
        i += 1;
    }
    let mut i: usize = 0;
    while i < a.resources.len()
        invariant i <= a.resources@.len(), a.resources@.len() == b.resources@.len(), forall|k: int| 0 <= k < i ==> a.resources@[k] == b.resources@[k],
        decreases a.resources@.len() - i,
    {
        if a.resources[i].ceiling != b.resources[i].ceiling || a.resources[i].partition != b.resources[i].partition {
            return false;
        }
        i += 1;
    }
    let mut i: usize = 0;
    while i < a.access.len()
        invariant i <= a.access@.len(), a.access@.len() == b.access@.len(), forall|k: int| 0 <= k < i ==> a.access@[k]@ == b.access@[k]@,
        decreases a.access@.len() - i,
    {
        if !flags_agree(&a.access[i], &b.access[i]) {
            return false;
        }
        i += 1;
    }
    let mut i: usize = 0;
    while i < a.partitions.len()
        invariant i <= a.partitions@.len(), a.partitions@.len() == b.partitions@.len(), forall|k: int| 0 <= k < i ==> a.partitions@[k] == b.partitions@[k],
        decreases a.partitions@.len() - i,
    {
        let (p, q) = (&a.partitions[i], &b.partitions[i]);
        if p.level != q.level || p.fault_response != q.fault_response || p.overrun_response != q.overrun_response || p.deadline_response != q.deadline_response || p.mit_threshold != q.mit_threshold {
            return false;
        }
        i += 1;
    }
    if a.operating_duration != b.operating_duration || a.slot_count != b.slot_count {
        return false;
    }
    if !bytes_agree(&a.slot_release, &b.slot_release) || !bytes_agree(&a.slot_deadline, &b.slot_deadline) || !bytes_agree(&a.slot_budget, &b.slot_budget) {
        return false;
    }
    proof {
        assert(a.tasks@ =~= b.tasks@);
        assert(a.resources@ =~= b.resources@);
        assert(a.partitions@ =~= b.partitions@);
        assert(rows_view(a.access@) =~= rows_view(b.access@));
    }
    true
}

/// R26.4: `true` only when `bytes` is, byte for byte, the `CONFIG` image of
/// the expected tables.
pub fn image_is(bytes: &[u8], expected: &Tables) -> (r: bool)
    ensures r ==> bytes@ == spec_image(expected@),
{
    match decode_tables(bytes) {
        Ok(found) => tables_agree(&found, expected),
        Err(_) => false,
    }
}

// ============================================================ LAYOUT ==

/// `MAX_WINDOWS` of the Kernel's `PartitionLayout`.
pub const MAX_WINDOWS: usize = 5;
/// `MAX_KERNEL_WINDOWS` of the Kernel's `Layout` (R16.2).
pub const MAX_KERNEL_WINDOWS: usize = 8;

/// An interval of the `LAYOUT` image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    /// `MAX_WINDOWS` slots, (base, size_log2, read-only); unused slots are (0, 5, false).
    pub windows: Vec<(u32, u8, bool)>,
}

/// The decoded `LAYOUT` image.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LayoutImg {
    pub kernel_flash: IntervalImg,
    pub shared_code: IntervalImg,
    pub kernel_ram: IntervalImg,
    pub kernel_window_count: u8,
    /// `MAX_KERNEL_WINDOWS` slots, (base, size_log2); unused slots are (0, 5).
    pub kernel_windows: Vec<(u32, u8)>,
    pub partitions: Vec<PartitionLayoutImg>,
}

pub struct PartitionLayoutV {
    pub code: IntervalImg,
    pub ram: IntervalImg,
    pub stack_size: u32,
    pub window_count: u8,
    pub windows: Seq<(u32, u8, bool)>,
}

pub struct LayoutV {
    pub kernel_flash: IntervalImg,
    pub shared_code: IntervalImg,
    pub kernel_ram: IntervalImg,
    pub kernel_window_count: u8,
    pub kernel_windows: Seq<(u32, u8)>,
    pub partitions: Seq<PartitionLayoutV>,
}

impl PartitionLayoutImg {
    pub open spec fn view(&self) -> PartitionLayoutV {
        PartitionLayoutV { code: self.code, ram: self.ram, stack_size: self.stack_size, window_count: self.window_count, windows: self.windows@ }
    }
}

pub open spec fn parts_view(p: Seq<PartitionLayoutImg>) -> Seq<PartitionLayoutV> {
    Seq::new(p.len(), |i: int| p[i]@)
}

impl LayoutImg {
    pub open spec fn view(&self) -> LayoutV {
        LayoutV {
            kernel_flash: self.kernel_flash,
            shared_code: self.shared_code,
            kernel_ram: self.kernel_ram,
            kernel_window_count: self.kernel_window_count,
            kernel_windows: self.kernel_windows@,
            partitions: parts_view(self.partitions@),
        }
    }
}

/// Every window table has all its slots, and the count fits a byte.
pub open spec fn layout_wf(l: LayoutV) -> bool {
    &&& l.kernel_windows.len() == MAX_KERNEL_WINDOWS
    &&& l.partitions.len() <= 255
    &&& forall|i: int| 0 <= i < l.partitions.len() ==> (#[trigger] l.partitions[i]).windows.len() == MAX_WINDOWS
}

pub open spec fn enc_interval(i: IntervalImg) -> Seq<u8> {
    enc_u32(i.base) + seq![i.size_log2, i.srd]
}

pub open spec fn enc_kernel_window(w: (u32, u8)) -> Seq<u8> {
    enc_u32(w.0) + seq![w.1]
}

pub open spec fn enc_kernel_windows(s: Seq<(u32, u8)>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_kernel_windows(s.drop_last()) + enc_kernel_window(s.last()) }
}

pub open spec fn enc_window(w: (u32, u8, bool)) -> Seq<u8> {
    enc_u32(w.0) + seq![w.1] + enc_flag(w.2)
}

pub open spec fn enc_windows(s: Seq<(u32, u8, bool)>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_windows(s.drop_last()) + enc_window(s.last()) }
}

pub open spec fn enc_partition_layout(p: PartitionLayoutV) -> Seq<u8> {
    enc_interval(p.code) + enc_interval(p.ram) + enc_u32(p.stack_size) + seq![p.window_count] + enc_windows(p.windows)
}

pub open spec fn enc_partition_layouts(s: Seq<PartitionLayoutV>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_partition_layouts(s.drop_last()) + enc_partition_layout(s.last()) }
}

/// The `LAYOUT` image: the Kernel's three intervals, its window count and
/// all its window slots, the Partition count, then per Partition its code
/// and RAM intervals, stack size, window count and all its window slots.
pub open spec fn spec_layout_image(l: LayoutV) -> Seq<u8> {
    enc_interval(l.kernel_flash) + enc_interval(l.shared_code) + enc_interval(l.kernel_ram) + seq![l.kernel_window_count] + enc_kernel_windows(l.kernel_windows)
        + enc_u32(l.partitions.len() as u32) + enc_partition_layouts(l.partitions)
}

fn rd_interval(bytes: &[u8], pos: &mut usize) -> (r: Option<IntervalImg>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Some ==> read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_interval(r.unwrap()),
{
    let ghost start = read(bytes@, *pos);
    let base = match rd_u32(bytes, pos) { Some(v) => v, None => { return None; } };
    let size_log2 = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let srd = match rd_u8(bytes, pos) { Some(v) => v, None => { return None; } };
    let i = IntervalImg { base, size_log2, srd };
    proof {
        assert(read(bytes@, *pos) =~= start + enc_interval(i));
    }
    Some(i)
}

fn rd_partition_layout(bytes: &[u8], pos: &mut usize) -> (r: Result<PartitionLayoutImg, DecodeError>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Ok ==> r.unwrap()@.windows.len() == MAX_WINDOWS && read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_partition_layout(r.unwrap()@),
{
    let ghost start = read(bytes@, *pos);
    let code = match rd_interval(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let ram = match rd_interval(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let stack_size = match rd_u32(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let window_count = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let ghost head = read(bytes@, *pos);
    let mut windows: Vec<(u32, u8, bool)> = Vec::new();
    let mut i: usize = 0;
    while i < MAX_WINDOWS
        invariant
            i <= MAX_WINDOWS, windows@.len() == i, *pos <= bytes@.len(),
            read(bytes@, *pos) == head + enc_windows(windows@),
        decreases MAX_WINDOWS - i,
    {
        let ghost before = windows@;
        let ghost at = read(bytes@, *pos);
        let base = match rd_u32(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        let size_log2 = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        let ro = match rd_flag(bytes, pos) { Ok(v) => v, Err(e) => { return Err(e); } };
        windows.push((base, size_log2, ro));
        proof {
            assert(read(bytes@, *pos) =~= at + enc_window((base, size_log2, ro)));
            assert(windows@.drop_last() =~= before);
            assert(read(bytes@, *pos) =~= head + enc_windows(windows@));
        }
        i += 1;
    }
    let p = PartitionLayoutImg { code, ram, stack_size, window_count, windows };
    proof {
        assert(read(bytes@, *pos) =~= start + enc_partition_layout(p@));
    }
    Ok(p)
}

/// R26.4, R16.7: the `LAYOUT` image. Returns for every input; what it
/// accepts is exactly the encoding of what it returns.
pub fn decode_layout_image(bytes: &[u8]) -> (r: Result<LayoutImg, DecodeError>)
    ensures r is Ok ==> layout_wf(r.unwrap()@) && bytes@ == spec_layout_image(r.unwrap()@),
{
    let mut pos: usize = 0;
    assert(read(bytes@, 0) =~= Seq::<u8>::empty());
    let kernel_flash = match rd_interval(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let shared_code = match rd_interval(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let kernel_ram = match rd_interval(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let kernel_window_count = match rd_u8(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let ghost head = read(bytes@, pos);
    assert(head =~= enc_interval(kernel_flash) + enc_interval(shared_code) + enc_interval(kernel_ram) + seq![kernel_window_count]);
    let mut kernel_windows: Vec<(u32, u8)> = Vec::new();
    let mut i: usize = 0;
    while i < MAX_KERNEL_WINDOWS
        invariant
            i <= MAX_KERNEL_WINDOWS, kernel_windows@.len() == i, pos <= bytes@.len(),
            read(bytes@, pos) == head + enc_kernel_windows(kernel_windows@),
        decreases MAX_KERNEL_WINDOWS - i,
    {
        let ghost before = kernel_windows@;
        let ghost at = read(bytes@, pos);
        let base = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        let size_log2 = match rd_u8(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
        kernel_windows.push((base, size_log2));
        proof {
            assert(read(bytes@, pos) =~= at + enc_kernel_window((base, size_log2)));
            assert(kernel_windows@.drop_last() =~= before);
            assert(read(bytes@, pos) =~= head + enc_kernel_windows(kernel_windows@));
        }
        i += 1;
    }
    let ghost after_windows = read(bytes@, pos);
    let np32 = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    if np32 > 255 {
        return Err(DecodeError::Counts);
    }
    let np = np32 as usize;
    let ghost after_count = read(bytes@, pos);
    let mut partitions: Vec<PartitionLayoutImg> = Vec::new();
    let mut i: usize = 0;
    while i < np
        invariant
            i <= np, partitions@.len() == i, pos <= bytes@.len(),
            forall|k: int| 0 <= k < i ==> (#[trigger] partitions@[k])@.windows.len() == MAX_WINDOWS,
            read(bytes@, pos) == after_count + enc_partition_layouts(parts_view(partitions@)),
        decreases np - i,
    {
        let ghost before = parts_view(partitions@);
        let p = match rd_partition_layout(bytes, &mut pos) { Ok(v) => v, Err(e) => { return Err(e); } };
        let ghost pv = p@;
        partitions.push(p);
        proof {
            assert(parts_view(partitions@).drop_last() =~= before);
            assert(parts_view(partitions@).last() == pv);
            assert(read(bytes@, pos) =~= after_count + enc_partition_layouts(parts_view(partitions@)));
        }
        i += 1;
    }
    if pos != bytes.len() {
        return Err(DecodeError::Trailing(bytes.len() - pos));
    }
    let l = LayoutImg { kernel_flash, shared_code, kernel_ram, kernel_window_count, kernel_windows, partitions };
    proof {
        let v = l@;
        assert(v.partitions.len() as u32 == np32);
        assert(read(bytes@, pos) =~= bytes@);
        assert(forall|k: int| 0 <= k < v.partitions.len() ==> (#[trigger] v.partitions[k]).windows.len() == MAX_WINDOWS);
        assert(after_count =~= after_windows + enc_u32(np32));
        assert(bytes@ =~= spec_layout_image(v));
    }
    Ok(l)
}

fn interval_agrees(a: &IntervalImg, b: &IntervalImg) -> (r: bool)
    ensures r ==> *a == *b,
{
    a.base == b.base && a.size_log2 == b.size_log2 && a.srd == b.srd
}

fn windows_agree(a: &Vec<(u32, u8, bool)>, b: &Vec<(u32, u8, bool)>) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if a.len() != b.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.len()
        invariant i <= a@.len(), a@.len() == b@.len(), forall|k: int| 0 <= k < i ==> a@[k] == b@[k],
        decreases a@.len() - i,
    {
        if a[i].0 != b[i].0 || a[i].1 != b[i].1 || a[i].2 != b[i].2 {
            return false;
        }
        i += 1;
    }
    assert(a@ =~= b@);
    true
}

/// R26.4: `true` only when the two layouts agree in every field.
pub fn layouts_agree(a: &LayoutImg, b: &LayoutImg) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if !interval_agrees(&a.kernel_flash, &b.kernel_flash) || !interval_agrees(&a.shared_code, &b.shared_code) || !interval_agrees(&a.kernel_ram, &b.kernel_ram) {
        return false;
    }
    if a.kernel_window_count != b.kernel_window_count || a.kernel_windows.len() != b.kernel_windows.len() || a.partitions.len() != b.partitions.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.kernel_windows.len()
        invariant i <= a.kernel_windows@.len(), a.kernel_windows@.len() == b.kernel_windows@.len(), forall|k: int| 0 <= k < i ==> a.kernel_windows@[k] == b.kernel_windows@[k],
        decreases a.kernel_windows@.len() - i,
    {
        if a.kernel_windows[i].0 != b.kernel_windows[i].0 || a.kernel_windows[i].1 != b.kernel_windows[i].1 {
            return false;
        }
        i += 1;
    }
    let mut i: usize = 0;
    while i < a.partitions.len()
        invariant i <= a.partitions@.len(), a.partitions@.len() == b.partitions@.len(), forall|k: int| 0 <= k < i ==> a.partitions@[k]@ == b.partitions@[k]@,
        decreases a.partitions@.len() - i,
    {
        let (p, q) = (&a.partitions[i], &b.partitions[i]);
        if !interval_agrees(&p.code, &q.code) || !interval_agrees(&p.ram, &q.ram) || p.stack_size != q.stack_size || p.window_count != q.window_count || !windows_agree(&p.windows, &q.windows) {
            return false;
        }
        i += 1;
    }
    proof {
        assert(a.kernel_windows@ =~= b.kernel_windows@);
        assert(parts_view(a.partitions@) =~= parts_view(b.partitions@));
    }
    true
}

/// R26.4, R16.7: `true` only when `bytes` is, byte for byte, the `LAYOUT`
/// image of the expected layout.
pub fn layout_image_is(bytes: &[u8], expected: &LayoutImg) -> (r: bool)
    ensures r ==> bytes@ == spec_layout_image(expected@),
{
    match decode_layout_image(bytes) {
        Ok(found) => layouts_agree(&found, expected),
        Err(_) => false,
    }
}

// ========================================================= ENDPOINTS ==

/// The role slots of an Endpoint entry (`MAX_ROLES`).
pub const MAX_ROLES: usize = 4;

/// One Endpoint of the Kernel's table (task 21.2). The four tables have
/// `MAX_ROLES` slots each, `validates` that many rows of that many.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EndpointCfg {
    pub capacity: u8,
    pub payload: u16,
    /// Task 21.7: the role count.
    pub roles: u8,
    pub tasks: Vec<u8>,
    pub partitions: Vec<u8>,
    pub release: Vec<bool>,
    pub storage: u32,
    /// R42.4 (audit finding 2): `validates[r][f]`, the edges on which the
    /// Kernel accepts a rejection report of role `r` against role `f`.
    pub validates: Vec<Vec<bool>>,
}

pub struct EndpointV {
    pub capacity: u8,
    pub payload: u16,
    pub roles: u8,
    pub tasks: Seq<u8>,
    pub partitions: Seq<u8>,
    pub release: Seq<bool>,
    pub storage: u32,
    pub validates: Seq<Seq<bool>>,
}

impl EndpointCfg {
    pub open spec fn view(&self) -> EndpointV {
        EndpointV {
            capacity: self.capacity,
            payload: self.payload,
            roles: self.roles,
            tasks: self.tasks@,
            partitions: self.partitions@,
            release: self.release@,
            storage: self.storage,
            validates: rows_view(self.validates@),
        }
    }
}

pub open spec fn endpoints_view(e: Seq<EndpointCfg>) -> Seq<EndpointV> {
    Seq::new(e.len(), |i: int| e[i]@)
}

pub open spec fn endpoint_wf(e: EndpointV) -> bool {
    &&& e.tasks.len() == MAX_ROLES && e.partitions.len() == MAX_ROLES && e.release.len() == MAX_ROLES
    &&& e.validates.len() == MAX_ROLES
    &&& forall|r: int| 0 <= r < e.validates.len() ==> (#[trigger] e.validates[r]).len() == MAX_ROLES
}

pub open spec fn enc_u16(v: u16) -> Seq<u8> {
    seq![(v & 0xff) as u8, (v >> 8) as u8]
}

pub open spec fn enc_endpoint(e: EndpointV) -> Seq<u8> {
    seq![e.capacity] + enc_u16(e.payload) + seq![e.roles] + e.tasks + e.partitions + enc_flags(e.release) + enc_u32(e.storage) + enc_rows(e.validates)
}

pub open spec fn enc_endpoints(s: Seq<EndpointV>) -> Seq<u8>
    decreases s.len(),
{
    if s.len() == 0 { Seq::empty() } else { enc_endpoints(s.drop_last()) + enc_endpoint(s.last()) }
}

/// The Endpoint image: the count, the entries, the storage bytes.
pub open spec fn spec_endpoint_image(e: Seq<EndpointV>, storage_bytes: u32) -> Seq<u8> {
    enc_u32(e.len() as u32) + enc_endpoints(e) + enc_u32(storage_bytes)
}

proof fn lemma_le16(b0: u8, b1: u8)
    ensures ({
        let v = (b0 as u16) | ((b1 as u16) << 8);
        &&& (v & 0xff) as u8 == b0
        &&& (v >> 8) as u8 == b1
    }),
{
    let v = (b0 as u16) | ((b1 as u16) << 8);
    assert((v & 0xff) as u8 == b0 && (v >> 8) as u8 == b1) by (bit_vector)
        requires v == (b0 as u16) | ((b1 as u16) << 8);
}

fn rd_endpoint(bytes: &[u8], pos: &mut usize) -> (r: Result<EndpointCfg, DecodeError>)
    requires *old(pos) <= bytes@.len(),
    ensures
        *final(pos) <= bytes@.len(),
        r is Ok ==> endpoint_wf(r.unwrap()@) && read(bytes@, *final(pos)) == read(bytes@, *old(pos)) + enc_endpoint(r.unwrap()@),
{
    let ghost start = read(bytes@, *pos);
    let capacity = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let p0 = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let p1 = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let payload = (p0 as u16) | ((p1 as u16) << 8);
    proof {
        lemma_le16(p0, p1);
        assert(enc_u16(payload) =~= seq![p0, p1]);
    }
    let roles = match rd_u8(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let tasks = match rd_bytes(bytes, pos, MAX_ROLES) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let partitions = match rd_bytes(bytes, pos, MAX_ROLES) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let release = match rd_row(bytes, pos, MAX_ROLES) { Ok(v) => v, Err(e) => { return Err(e); } };
    let storage = match rd_u32(bytes, pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    let ghost head = read(bytes@, *pos);
    assert(head =~= start + seq![capacity] + enc_u16(payload) + seq![roles] + tasks@ + partitions@ + enc_flags(release@) + enc_u32(storage));
    let mut validates: Vec<Vec<bool>> = Vec::new();
    let mut i: usize = 0;
    while i < MAX_ROLES
        invariant
            i <= MAX_ROLES, validates@.len() == i, *pos <= bytes@.len(),
            forall|k: int| 0 <= k < i ==> (#[trigger] validates@[k])@.len() == MAX_ROLES,
            read(bytes@, *pos) == head + enc_rows(rows_view(validates@)),
        decreases MAX_ROLES - i,
    {
        let ghost before = rows_view(validates@);
        let row = match rd_row(bytes, pos, MAX_ROLES) { Ok(v) => v, Err(e) => { return Err(e); } };
        let ghost row_v = row@;
        validates.push(row);
        proof {
            assert(rows_view(validates@).drop_last() =~= before);
            assert(rows_view(validates@).last() == row_v);
            assert(read(bytes@, *pos) =~= head + enc_rows(rows_view(validates@)));
        }
        i += 1;
    }
    let e = EndpointCfg { capacity, payload, roles, tasks, partitions, release, storage, validates };
    proof {
        let v = e@;
        assert(forall|r: int| 0 <= r < v.validates.len() ==> (#[trigger] v.validates[r]).len() == MAX_ROLES);
        assert(read(bytes@, *pos) =~= start + enc_endpoint(v));
    }
    Ok(e)
}

/// R26.4: the Endpoint image. Returns for every input; what it accepts is
/// exactly the encoding of what it returns.
pub fn decode_endpoint_image(bytes: &[u8]) -> (r: Result<(Vec<EndpointCfg>, u32), DecodeError>)
    ensures
        r is Ok ==> {
            let e = endpoints_view(r.unwrap().0@);
            &&& e.len() <= 8
            &&& forall|i: int| 0 <= i < e.len() ==> endpoint_wf(#[trigger] e[i])
            &&& bytes@ == spec_endpoint_image(e, r.unwrap().1)
        },
{
    let mut pos: usize = 0;
    assert(read(bytes@, 0) =~= Seq::<u8>::empty());
    let n32 = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    if n32 > 8 {
        return Err(DecodeError::Counts);
    }
    let n = n32 as usize;
    let ghost head = read(bytes@, pos);
    let mut out: Vec<EndpointCfg> = Vec::new();
    let mut i: usize = 0;
    while i < n
        invariant
            i <= n, out@.len() == i, pos <= bytes@.len(),
            forall|k: int| 0 <= k < i ==> endpoint_wf((#[trigger] out@[k])@),
            read(bytes@, pos) == head + enc_endpoints(endpoints_view(out@)),
        decreases n - i,
    {
        let ghost before = endpoints_view(out@);
        let e = match rd_endpoint(bytes, &mut pos) { Ok(v) => v, Err(e) => { return Err(e); } };
        let ghost ev = e@;
        out.push(e);
        proof {
            assert(endpoints_view(out@).drop_last() =~= before);
            assert(endpoints_view(out@).last() == ev);
            assert(read(bytes@, pos) =~= head + enc_endpoints(endpoints_view(out@)));
        }
        i += 1;
    }
    let storage_bytes = match rd_u32(bytes, &mut pos) { Some(v) => v, None => { return Err(DecodeError::TooShort); } };
    if pos != bytes.len() {
        return Err(DecodeError::Trailing(bytes.len() - pos));
    }
    proof {
        let e = endpoints_view(out@);
        assert(e.len() as u32 == n32);
        assert(read(bytes@, pos) =~= bytes@);
        assert(forall|k: int| 0 <= k < e.len() ==> endpoint_wf(#[trigger] e[k]));
        assert(bytes@ =~= spec_endpoint_image(e, storage_bytes));
    }
    Ok((out, storage_bytes))
}

fn endpoint_agrees(a: &EndpointCfg, b: &EndpointCfg) -> (r: bool)
    ensures r ==> a@ == b@,
{
    if a.capacity != b.capacity || a.payload != b.payload || a.roles != b.roles || a.storage != b.storage {
        return false;
    }
    if !bytes_agree(&a.tasks, &b.tasks) || !bytes_agree(&a.partitions, &b.partitions) || !flags_agree(&a.release, &b.release) || a.validates.len() != b.validates.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < a.validates.len()
        invariant i <= a.validates@.len(), a.validates@.len() == b.validates@.len(), forall|k: int| 0 <= k < i ==> a.validates@[k]@ == b.validates@[k]@,
        decreases a.validates@.len() - i,
    {
        if !flags_agree(&a.validates[i], &b.validates[i]) {
            return false;
        }
        i += 1;
    }
    proof {
        assert(rows_view(a.validates@) =~= rows_view(b.validates@));
    }
    true
}

/// R26.4: `true` only when `bytes` is, byte for byte, the Endpoint image of
/// the expected table and storage size.
pub fn endpoint_image_is(bytes: &[u8], expected: &Vec<EndpointCfg>, storage_bytes: u32) -> (r: bool)
    ensures r ==> bytes@ == spec_endpoint_image(endpoints_view(expected@), storage_bytes),
{
    let (found, found_storage) = match decode_endpoint_image(bytes) {
        Ok(v) => v,
        Err(_) => {
            return false;
        }
    };
    if found_storage != storage_bytes || found.len() != expected.len() {
        return false;
    }
    let mut i: usize = 0;
    while i < found.len()
        invariant i <= found@.len(), found@.len() == expected@.len(), forall|k: int| 0 <= k < i ==> found@[k]@ == expected@[k]@,
        decreases found@.len() - i,
    {
        if !endpoint_agrees(&found[i], &expected[i]) {
            return false;
        }
        i += 1;
    }
    proof {
        assert(endpoints_view(found@) =~= endpoints_view(expected@));
    }
    true
}

// ================================================= storage placement ==

/// 2^n.
pub open spec fn pow2(n: nat) -> nat
    decreases n,
{
    if n == 0 { 1 } else { 2 * pow2((n - 1) as nat) }
}

/// One past the last byte of an interval of the `LAYOUT` image.
pub open spec fn interval_end(i: IntervalImg) -> int {
    i.base as int + pow2(i.size_log2 as nat) as int
}

/// An Endpoint's slots lie in the Endpoint storage at a word-aligned
/// offset: `roles * roles` channels of `capacity` slots of `payload` bytes.
pub open spec fn endpoint_in_storage(e: EndpointV, total: int) -> bool {
    &&& e.storage as int % 4 == 0
    &&& e.storage as int + (e.roles as int * e.roles as int) * e.capacity as int * e.payload as int <= total
}

/// A Partition's RAM interval does not reach the last byte of the address
/// space and shares no byte with the storage `[base, base + total)`.
pub open spec fn ram_apart(ram: IntervalImg, base: int, total: int) -> bool {
    &&& interval_end(ram) <= 0xFFFF_FFFF
    &&& (interval_end(ram) <= base || base + total <= ram.base as int)
}

/// Where the Endpoint storage of a binary lies (R16.1, R39.2; 2026-11-21):
/// it starts at a word, every Endpoint's slots lie inside it at a
/// word-aligned offset, and it shares no byte with any Partition's RAM
/// interval, none of which reaches the last byte of the address space.
/// This is the Kernel's `storage_placed` (`rsk-kernel`, `entry.rs`) read
/// off the images: a message buffer the Kernel admits is a range of the
/// calling Partition's RAM interval, so under this no admitted buffer
/// shares a byte with the storage. The statement is this crate's own
/// (R26.1); that it is the Kernel's is by reading the two side by side.
pub open spec fn storage_placed(layout: LayoutV, endpoints: Seq<EndpointV>, base: int, total: int) -> bool {
    &&& base % 4 == 0
    &&& forall|e: int| 0 <= e < endpoints.len() ==> endpoint_in_storage(#[trigger] endpoints[e], total)
    &&& forall|p: int| 0 <= p < layout.partitions.len() ==> ram_apart((#[trigger] layout.partitions[p]).ram, base, total)
}

proof fn lemma_pow2_32()
    ensures pow2(32) == 0x1_0000_0000,
{
    assert(pow2(32) == 0x1_0000_0000) by (compute);
}

proof fn lemma_pow2_mono(a: nat, b: nat)
    requires a <= b,
    ensures pow2(a) <= pow2(b),
    decreases b,
{
    if a < b {
        lemma_pow2_mono(a, (b - 1) as nat);
    }
}

/// 2^k for k up to 32, by a bounded loop.
fn pow2_u64(k: u8) -> (r: u64)
    requires k <= 32,
    ensures r == pow2(k as nat), r <= 0x1_0000_0000,
{
    let mut r: u64 = 1;
    let mut i: u8 = 0;
    proof {
        lemma_pow2_32();
    }
    while i < k
        invariant i <= k, k <= 32, r == pow2(i as nat), pow2(32) == 0x1_0000_0000,
        decreases k - i,
    {
        proof {
            lemma_pow2_mono((i + 1) as nat, 32);
        }
        r = r * 2;
        i = i + 1;
    }
    proof {
        lemma_pow2_mono(k as nat, 32);
    }
    r
}

fn endpoint_in_storage_ok(e: &EndpointCfg, total: u32) -> (r: bool)
    ensures r ==> endpoint_in_storage(e@, total as int),
{
    let n = e.roles as u64;
    let cap = e.capacity as u64;
    let pl = e.payload as u64;
    assert(n * n <= 255 * 255) by (nonlinear_arith)
        requires n <= 255;
    let channels: u64 = n * n;
    assert(channels * cap <= 255 * 255 * 255) by (nonlinear_arith)
        requires channels <= 255 * 255, cap <= 255;
    let slots: u64 = channels * cap;
    assert(slots * pl <= 255 * 255 * 255 * 65535) by (nonlinear_arith)
        requires slots <= 255 * 255 * 255, pl <= 65535;
    let bytes: u64 = slots * pl;
    e.storage % 4 == 0 && e.storage as u64 + bytes <= total as u64
}

fn ram_apart_ok(ram: &IntervalImg, base: u32, total: u32) -> (r: bool)
    ensures r ==> ram_apart(*ram, base as int, total as int),
{
    if ram.size_log2 > 32 {
        return false;
    }
    let end: u64 = ram.base as u64 + pow2_u64(ram.size_log2);
    end <= 0xFFFF_FFFF && (end <= base as u64 || base as u64 + total as u64 <= ram.base as u64)
}

/// The Endpoint storage lies inside the Kernel's RAM interval of the
/// `LAYOUT` image (R16.1, 2026-11-22): the premise under which the Kernel
/// proves that the storage keeps its words while Jobs run
/// (`entry::theorem_storage_kept_between_entries`), since no Partition's
/// MPU view grants a write in that interval.
pub open spec fn storage_in_kernel_ram(kernel_ram: IntervalImg, base: int, total: int) -> bool {
    kernel_ram.base as int <= base && base + total <= interval_end(kernel_ram)
}

/// `true` only when `storage_in_kernel_ram` holds.
pub fn storage_in_kernel_ram_ok(kernel_ram: &IntervalImg, base: u32, total: u32) -> (r: bool)
    ensures r ==> storage_in_kernel_ram(*kernel_ram, base as int, total as int),
{
    if kernel_ram.size_log2 > 32 {
        return false;
    }
    let end: u64 = kernel_ram.base as u64 + pow2_u64(kernel_ram.size_log2);
    kernel_ram.base <= base && base as u64 + total as u64 <= end
}

/// R16.1 for the Endpoint storage of a binary: `true` only when
/// `storage_placed` holds of the decoded `LAYOUT` image, the decoded
/// Endpoint table and the storage's address and size.
pub fn storage_placed_ok(layout: &LayoutImg, endpoints: &Vec<EndpointCfg>, base: u32, total: u32) -> (r: bool)
    ensures r ==> storage_placed(layout@, endpoints_view(endpoints@), base as int, total as int),
{
    if base % 4 != 0 {
        return false;
    }
    let mut e: usize = 0;
    while e < endpoints.len()
        invariant e <= endpoints@.len(), forall|k: int| 0 <= k < e ==> endpoint_in_storage(#[trigger] endpoints_view(endpoints@)[k], total as int),
        decreases endpoints@.len() - e,
    {
        if !endpoint_in_storage_ok(&endpoints[e], total) {
            return false;
        }
        e += 1;
    }
    let mut p: usize = 0;
    while p < layout.partitions.len()
        invariant p <= layout.partitions@.len(), forall|k: int| 0 <= k < p ==> ram_apart((#[trigger] layout@.partitions[k]).ram, base as int, total as int),
        decreases layout.partitions@.len() - p,
    {
        if !ram_apart_ok(&layout.partitions[p].ram, base, total) {
            return false;
        }
        p += 1;
    }
    true
}

} // verus!
