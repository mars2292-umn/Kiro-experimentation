//! The `CONFIG` tables the Task_Model calls for, built from the model as
//! read, verified (R26.4; 2026-11-26).
//!
//! `recompute::expected_tables` hands `image_v::image_is` the tables this
//! module builds, and `recompute::expected_layout` hands
//! `image_v::layout_image_is` the layout it builds, so the checker's
//! verdict on a binary's `CONFIG` and `LAYOUT` images rests on them. Until
//! 2026-11-26 the builders were ordinary Rust.
//!
//! | Function | Proven |
//! |---|---|
//! | `build_tables` | it returns for every model (no index out of range, no overflow, no truncation: a value with no place in its field is refused); when it returns tables `t`, `tables_ok(m, t@)`: one entry per Task, Resource and Partition of the model, in the model's order, and field by field the model's values: a Task's Priority, kind, offset, period or MIT, deadline, Budget, MIT policy, floating-point flag and three slots; its Partition the index of the first Partition of the id it names; `access[t][r]` exactly when Resource `r` lists the id of Task `t` among its accessors; a Resource's Ceiling the greatest Priority among the Tasks its accessors name (0 when they name none) and its Partition by index; a Partition's level and three responses by their codes and its MIT threshold; the operating duration; the slot count `3 * tasks + 2`; and the tables are well formed (`image_v::wf`) when the model has at most 255 Resources and 255 Partitions (the image has no place for a greater count, and `image_v::image_is` answers `false` for such tables) |
//! | `build_layout` | it returns for every model; when it returns a layout `l`, `layout_ok(m, l@)`: the three Kernel intervals and each Partition's code and RAM intervals are the model's regions (base, size, sub-regions), the stack size the model's; every window table has all its slots, slot `k` the model's window `k` (base, size, and for a Partition the read-only flag) or the unused slot `(0, 5)`, the count the model's; and the layout is well formed (`image_v::layout_wf`) when the model has at most 255 Partitions |
//! | `partition_index`, `task_index` | the first entry of the id, or none when no entry has it |
//! | `level_code`, `response_code` | the code of a name (`level_of`, `response_of`), none for any other text |
//!
//! Not proven here: that the model is the Task_Model's text (the JSON
//! reader, `model::Model::parse`, over `serde_json`); the Endpoint
//! table's builder (`recompute::expected_endpoints`); the wording of a
//! refusal.

use vstd::prelude::*;

use crate::image_v::{IntervalImg, LayoutImg, LayoutV, PartitionCfg, PartitionLayoutImg, PartitionLayoutV, ResourceCfg, Tables, TablesV, TaskCfg, MAX_KERNEL_WINDOWS, MAX_WINDOWS};
use crate::model::{Model, Region};

verus! {

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

/// The field a value has no place in.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Field {
    Priority,
    Budget,
    SlotRelease,
    SlotDeadline,
    SlotBudget,
    Ceiling,
    MitThreshold,
    SlotCount,
}

/// Why the model has no tables.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuildErr {
    /// The value does not fit the field.
    Narrow(Field, u64),
    /// The Task of this index names no Partition (or one beyond 255).
    TaskPartition(usize),
    /// The Resource of this index names no Partition.
    ResourcePartition(usize),
    /// The Partition of this index has no known level.
    Level(usize),
    /// The Partition of this index has no known response: 0 fault, 1
    /// overrun, 2 deadline miss.
    Response(usize, u8),
}

// ------------------------------------------------------ the statements --

/// `p` is the first Partition of the id.
pub open spec fn partition_is(m: Model, id: Seq<char>, p: int) -> bool {
    &&& 0 <= p < m.partitions@.len()
    &&& m.partitions@[p].id@ == id
    &&& forall|q: int| 0 <= q < p ==> (#[trigger] m.partitions@[q]).id@ != id
}

/// `t` is the first Task of the id.
pub open spec fn task_is(m: Model, id: Seq<char>, t: int) -> bool {
    &&& 0 <= t < m.tasks@.len()
    &&& m.tasks@[t].id@ == id
    &&& forall|q: int| 0 <= q < t ==> (#[trigger] m.tasks@[q]).id@ != id
}

/// The level code of a declared level (`A` = 0 ... `E` = 4).
pub open spec fn level_of(name: Seq<char>) -> Option<u8> {
    if name == "A"@ {
        Some(0u8)
    } else if name == "B"@ {
        Some(1u8)
    } else if name == "C"@ {
        Some(2u8)
    } else if name == "D"@ {
        Some(3u8)
    } else if name == "E"@ {
        Some(4u8)
    } else {
        None
    }
}

/// The response code of a declared response name.
pub open spec fn response_of(name: Seq<char>) -> Option<u8> {
    if name == "END_JOB"@ {
        Some(RESP_END_JOB)
    } else if name == "RESTART_PARTITION"@ {
        Some(RESP_RESTART_PARTITION)
    } else if name == "STOP_PARTITION"@ {
        Some(RESP_STOP_PARTITION)
    } else if name == "SAFE_STATE"@ {
        Some(RESP_SAFE_STATE)
    } else if name == "RECORD_ONLY"@ {
        Some(RESP_RECORD_ONLY)
    } else if name == "RECORD_AND_CONTINUE"@ {
        Some(RESP_RECORD_AND_CONTINUE)
    } else {
        None
    }
}

/// The table entry of Task `i` holds the model's values.
pub open spec fn task_ok(m: Model, i: int, c: TaskCfg) -> bool {
    let t = m.tasks@[i];
    &&& c.priority == t.priority
    &&& partition_is(m, t.partition@, c.partition as int)
    &&& c.kind == (if t.sporadic { KIND_SPORADIC } else { KIND_PERIODIC })
    &&& c.offset == t.offset_ticks
    &&& c.period == t.period_ticks
    &&& c.deadline == t.deadline_ticks
    &&& c.budget == t.budget_cycles
    &&& c.mit_policy == (if t.mit_policy is Some && t.mit_policy.unwrap()@ == "discard"@ { MIT_DISCARD } else { MIT_DEFER })
    &&& c.fpu == t.fpu
}

/// Resource `r` lists the id among its accessors.
pub open spec fn accesses(m: Model, r: int, id: Seq<char>) -> bool {
    exists|a: int| 0 <= a < m.resources@[r].accessors@.len() && (#[trigger] m.resources@[r].accessors@[a])@ == id
}

/// `c` is the greatest Priority among the Tasks that the first `upto`
/// accessors of Resource `r` name, 0 when they name none.
pub open spec fn ceiling_is(m: Model, r: int, upto: int, c: u64) -> bool {
    &&& forall|a: int, t: int| 0 <= a < upto && #[trigger] task_is(m, m.resources@[r].accessors@[a]@, t) ==> m.tasks@[t].priority <= c
    &&& c == 0 || exists|a: int, t: int| 0 <= a < upto && #[trigger] task_is(m, m.resources@[r].accessors@[a]@, t) && m.tasks@[t].priority == c
}

/// The table entry of Resource `r` holds the recomputed Ceiling and the
/// Partition.
pub open spec fn resource_ok(m: Model, r: int, c: ResourceCfg) -> bool {
    &&& ceiling_is(m, r, m.resources@[r].accessors@.len() as int, c.ceiling as u64)
    &&& partition_is(m, m.resources@[r].partition@, c.partition as int)
}

/// The table entry of Partition `p` holds the model's values.
pub open spec fn partition_ok(m: Model, p: int, c: PartitionCfg) -> bool {
    let part = m.partitions@[p];
    &&& level_of(part.level@) == Some(c.level)
    &&& response_of(part.fault@) == Some(c.fault_response)
    &&& response_of(part.overrun@) == Some(c.overrun_response)
    &&& response_of(part.deadline_miss@) == Some(c.deadline_response)
    &&& c.mit_threshold == part.mit_threshold
}

/// The tables are the model's: one entry per Task, Resource and Partition,
/// in the model's order, each field the model's value.
pub open spec fn tables_ok(m: Model, t: TablesV) -> bool {
    &&& t.tasks.len() == m.tasks@.len()
    &&& t.resources.len() == m.resources@.len()
    &&& t.partitions.len() == m.partitions@.len()
    &&& t.access.len() == m.tasks@.len()
    &&& t.slot_release.len() == m.tasks@.len()
    &&& t.slot_deadline.len() == m.tasks@.len()
    &&& t.slot_budget.len() == m.tasks@.len()
    &&& forall|i: int| 0 <= i < m.tasks@.len() ==> {
        &&& task_ok(m, i, #[trigger] t.tasks[i])
        &&& t.slot_release[i] == m.tasks@[i].slot_release
        &&& t.slot_deadline[i] == m.tasks@[i].slot_deadline
        &&& t.slot_budget[i] == m.tasks@[i].slot_budget
        &&& t.access[i].len() == m.resources@.len()
        &&& forall|r: int| 0 <= r < m.resources@.len() ==> (#[trigger] t.access[i][r]) == accesses(m, r, m.tasks@[i].id@)
    }
    &&& forall|r: int| 0 <= r < m.resources@.len() ==> resource_ok(m, r, #[trigger] t.resources[r])
    &&& forall|p: int| 0 <= p < m.partitions@.len() ==> partition_ok(m, p, #[trigger] t.partitions[p])
    &&& t.operating_duration == m.operating_duration_ticks
    &&& t.slot_count == 3 * m.tasks@.len() + 2
}

// --------------------------------------------------------- the builder --

/// The first Partition of the id.
pub fn partition_index(m: &Model, id: &String) -> (r: Option<usize>)
    ensures
        match r {
            Some(p) => partition_is(*m, id@, p as int),
            None => forall|q: int| 0 <= q < m.partitions@.len() ==> (#[trigger] m.partitions@[q]).id@ != id@,
        },
{
    let mut i: usize = 0;
    while i < m.partitions.len()
        invariant i <= m.partitions@.len(), forall|q: int| 0 <= q < i ==> (#[trigger] m.partitions@[q]).id@ != id@,
        decreases m.partitions@.len() - i,
    {
        if m.partitions[i].id == *id {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The first Task of the id.
pub fn task_index(m: &Model, id: &String) -> (r: Option<usize>)
    ensures
        match r {
            Some(t) => task_is(*m, id@, t as int),
            None => forall|q: int| 0 <= q < m.tasks@.len() ==> (#[trigger] m.tasks@[q]).id@ != id@,
        },
{
    let mut i: usize = 0;
    while i < m.tasks.len()
        invariant i <= m.tasks@.len(), forall|q: int| 0 <= q < i ==> (#[trigger] m.tasks@[q]).id@ != id@,
        decreases m.tasks@.len() - i,
    {
        if m.tasks[i].id == *id {
            return Some(i);
        }
        i += 1;
    }
    None
}

/// The level code of a declared level (`A` = 0 ... `E` = 4).
pub fn level_code(name: &str) -> (r: Option<u8>)
    ensures r == level_of(name@),
{
    if name == "A" {
        Some(0)
    } else if name == "B" {
        Some(1)
    } else if name == "C" {
        Some(2)
    } else if name == "D" {
        Some(3)
    } else if name == "E" {
        Some(4)
    } else {
        None
    }
}

/// The response code of a declared response name.
pub fn response_code(name: &str) -> (r: Option<u8>)
    ensures r == response_of(name@),
{
    if name == "END_JOB" {
        Some(RESP_END_JOB)
    } else if name == "RESTART_PARTITION" {
        Some(RESP_RESTART_PARTITION)
    } else if name == "STOP_PARTITION" {
        Some(RESP_STOP_PARTITION)
    } else if name == "SAFE_STATE" {
        Some(RESP_SAFE_STATE)
    } else if name == "RECORD_ONLY" {
        Some(RESP_RECORD_ONLY)
    } else if name == "RECORD_AND_CONTINUE" {
        Some(RESP_RECORD_AND_CONTINUE)
    } else {
        None
    }
}

fn narrow_u8(v: u64, f: Field) -> (r: Result<u8, BuildErr>)
    ensures
        r is Ok ==> r.unwrap() == v,
        r is Err ==> v > 255,
{
    if v > 255 {
        Err(BuildErr::Narrow(f, v))
    } else {
        Ok(v as u8)
    }
}

fn narrow_u32(v: u64, f: Field) -> (r: Result<u32, BuildErr>)
    ensures r is Ok ==> r.unwrap() == v,
{
    if v > 0xffff_ffff {
        Err(BuildErr::Narrow(f, v))
    } else {
        Ok(v as u32)
    }
}

/// The index of the first Partition of the id, when it fits the field.
fn partition_u8(m: &Model, id: &String) -> (r: Option<u8>)
    ensures r is Some ==> partition_is(*m, id@, r.unwrap() as int),
{
    match partition_index(m, id) {
        Some(p) => if p > 255 { None } else { Some(p as u8) },
        None => None,
    }
}

/// The table entry of Task `i`.
fn task_cfg(m: &Model, i: usize) -> (r: Result<TaskCfg, BuildErr>)
    requires i < m.tasks@.len(),
    ensures r is Ok ==> task_ok(*m, i as int, r.unwrap()),
{
    let task = &m.tasks[i];
    let priority = match narrow_u8(task.priority, Field::Priority) { Ok(v) => v, Err(e) => { return Err(e); } };
    let partition = match partition_u8(m, &task.partition) { Some(p) => p, None => { return Err(BuildErr::TaskPartition(i)); } };
    let budget = match narrow_u32(task.budget_cycles, Field::Budget) { Ok(v) => v, Err(e) => { return Err(e); } };
    let mit_policy = match &task.mit_policy {
        Some(s) => if s.as_str() == "discard" { MIT_DISCARD } else { MIT_DEFER },
        None => MIT_DEFER,
    };
    Ok(TaskCfg {
        priority,
        partition,
        kind: if task.sporadic { KIND_SPORADIC } else { KIND_PERIODIC },
        offset: task.offset_ticks,
        period: task.period_ticks,
        deadline: task.deadline_ticks,
        budget,
        mit_policy,
        fpu: task.fpu,
    })
}

/// Whether Resource `r` lists the id among its accessors.
fn lists(m: &Model, r: usize, id: &String) -> (b: bool)
    requires r < m.resources@.len(),
    ensures b == accesses(*m, r as int, id@),
{
    let acc = &m.resources[r].accessors;
    let mut a: usize = 0;
    while a < acc.len()
        invariant
            a <= acc@.len(),
            acc@ == m.resources@[r as int].accessors@,
            forall|k: int| 0 <= k < a ==> (#[trigger] acc@[k])@ != id@,
        decreases acc@.len() - a,
    {
        if acc[a] == *id {
            return true;
        }
        a += 1;
    }
    false
}

/// The access row of Task `i`: per Resource whether it lists the Task.
fn access_row(m: &Model, i: usize) -> (row: Vec<bool>)
    requires i < m.tasks@.len(),
    ensures
        row@.len() == m.resources@.len(),
        forall|r: int| 0 <= r < m.resources@.len() ==> (#[trigger] row@[r]) == accesses(*m, r, m.tasks@[i as int].id@),
{
    let mut row: Vec<bool> = Vec::new();
    let mut r: usize = 0;
    while r < m.resources.len()
        invariant
            i < m.tasks@.len(),
            r <= m.resources@.len(),
            row@.len() == r,
            forall|k: int| 0 <= k < r ==> (#[trigger] row@[k]) == accesses(*m, k, m.tasks@[i as int].id@),
        decreases m.resources@.len() - r,
    {
        let b = lists(m, r, &m.tasks[i].id);
        row.push(b);
        r += 1;
    }
    row
}

/// Two first Tasks of one id are one Task.
proof fn lemma_task_unique(m: Model, id: Seq<char>, t: int, u: int)
    requires task_is(m, id, t), task_is(m, id, u),
    ensures t == u,
{
    if t < u {
        assert(m.tasks@[t].id@ != id);
    }
    if u < t {
        assert(m.tasks@[u].id@ != id);
    }
}

/// The Ceiling of Resource `r`: the greatest Priority among the Tasks its
/// accessors name.
fn ceiling(m: &Model, r: usize) -> (c: u64)
    requires r < m.resources@.len(),
    ensures ceiling_is(*m, r as int, m.resources@[r as int].accessors@.len() as int, c),
{
    let acc = &m.resources[r].accessors;
    let mut c: u64 = 0;
    let mut a: usize = 0;
    while a < acc.len()
        invariant
            r < m.resources@.len(),
            a <= acc@.len(),
            acc@ == m.resources@[r as int].accessors@,
            ceiling_is(*m, r as int, a as int, c),
        decreases acc@.len() - a,
    {
        let ghost c0 = c;
        let found = task_index(m, &acc[a]);
        match found {
            Some(t) => {
                if m.tasks[t].priority > c {
                    c = m.tasks[t].priority;
                }
                proof {
                    assert forall|x: int, u: int| 0 <= x < a + 1 && #[trigger] task_is(*m, m.resources@[r as int].accessors@[x]@, u) implies m.tasks@[u].priority <= c by {
                        if x == a {
                            lemma_task_unique(*m, acc@[a as int]@, t as int, u);
                        }
                    }
                    if c != c0 {
                        assert(task_is(*m, m.resources@[r as int].accessors@[a as int]@, t as int));
                    }
                }
            },
            None => {
                proof {
                    assert forall|x: int, u: int| 0 <= x < a + 1 && #[trigger] task_is(*m, m.resources@[r as int].accessors@[x]@, u) implies m.tasks@[u].priority <= c by {
                        if x == a {
                            assert(m.tasks@[u].id@ != acc@[a as int]@);
                        }
                    }
                }
            },
        }
        a += 1;
    }
    c
}

/// The table entry of Resource `r`.
fn resource_cfg(m: &Model, r: usize) -> (x: Result<ResourceCfg, BuildErr>)
    requires r < m.resources@.len(),
    ensures x is Ok ==> resource_ok(*m, r as int, x.unwrap()),
{
    let c = ceiling(m, r);
    let ceiling = match narrow_u8(c, Field::Ceiling) { Ok(v) => v, Err(e) => { return Err(e); } };
    let partition = match partition_u8(m, &m.resources[r].partition) { Some(p) => p, None => { return Err(BuildErr::ResourcePartition(r)); } };
    Ok(ResourceCfg { ceiling, partition })
}

/// The table entry of Partition `p`.
fn partition_cfg(m: &Model, p: usize) -> (x: Result<PartitionCfg, BuildErr>)
    requires p < m.partitions@.len(),
    ensures x is Ok ==> partition_ok(*m, p as int, x.unwrap()),
{
    let part = &m.partitions[p];
    let level = match level_code(part.level.as_str()) { Some(v) => v, None => { return Err(BuildErr::Level(p)); } };
    let fault_response = match response_code(part.fault.as_str()) { Some(v) => v, None => { return Err(BuildErr::Response(p, 0)); } };
    let overrun_response = match response_code(part.overrun.as_str()) { Some(v) => v, None => { return Err(BuildErr::Response(p, 1)); } };
    let deadline_response = match response_code(part.deadline_miss.as_str()) { Some(v) => v, None => { return Err(BuildErr::Response(p, 2)); } };
    let mit_threshold = match narrow_u32(part.mit_threshold, Field::MitThreshold) { Ok(v) => v, Err(e) => { return Err(e); } };
    Ok(PartitionCfg { level, fault_response, overrun_response, deadline_response, mit_threshold })
}

/// The `CONFIG` tables the Task_Model calls for (R26.4): every field of
/// every entry from the declared values, the Ceilings recomputed from the
/// accessors' Priorities, the slot count from the Task count. `Err` names
/// the first value that has no place in the Kernel's table.
pub fn build_tables(m: &Model) -> (x: Result<Tables, BuildErr>)
    ensures
        x is Ok ==> tables_ok(*m, x.unwrap()@),
        x is Ok && m.resources@.len() <= 255 && m.partitions@.len() <= 255 ==> crate::image_v::wf(x.unwrap()@),
{
    let mut tasks: Vec<TaskCfg> = Vec::new();
    let mut access: Vec<Vec<bool>> = Vec::new();
    let mut slot_release: Vec<u8> = Vec::new();
    let mut slot_deadline: Vec<u8> = Vec::new();
    let mut slot_budget: Vec<u8> = Vec::new();
    let mut i: usize = 0;
    while i < m.tasks.len()
        invariant
            i <= m.tasks@.len(),
            tasks@.len() == i,
            access@.len() == i,
            slot_release@.len() == i,
            slot_deadline@.len() == i,
            slot_budget@.len() == i,
            forall|k: int| 0 <= k < i ==> {
                &&& task_ok(*m, k, #[trigger] tasks@[k])
                &&& slot_release@[k] == m.tasks@[k].slot_release
                &&& slot_deadline@[k] == m.tasks@[k].slot_deadline
                &&& slot_budget@[k] == m.tasks@[k].slot_budget
                &&& access@[k]@.len() == m.resources@.len()
                &&& forall|r: int| 0 <= r < m.resources@.len() ==> (#[trigger] access@[k]@[r]) == accesses(*m, r, m.tasks@[k].id@)
            },
        decreases m.tasks@.len() - i,
    {
        let c = match task_cfg(m, i) { Ok(v) => v, Err(e) => { return Err(e); } };
        let row = access_row(m, i);
        let sr = match narrow_u8(m.tasks[i].slot_release, Field::SlotRelease) { Ok(v) => v, Err(e) => { return Err(e); } };
        let sd = match narrow_u8(m.tasks[i].slot_deadline, Field::SlotDeadline) { Ok(v) => v, Err(e) => { return Err(e); } };
        let sb = match narrow_u8(m.tasks[i].slot_budget, Field::SlotBudget) { Ok(v) => v, Err(e) => { return Err(e); } };
        tasks.push(c);
        access.push(row);
        slot_release.push(sr);
        slot_deadline.push(sd);
        slot_budget.push(sb);
        i += 1;
    }
    let mut resources: Vec<ResourceCfg> = Vec::new();
    let mut r: usize = 0;
    while r < m.resources.len()
        invariant
            r <= m.resources@.len(),
            resources@.len() == r,
            forall|k: int| 0 <= k < r ==> resource_ok(*m, k, #[trigger] resources@[k]),
        decreases m.resources@.len() - r,
    {
        let c = match resource_cfg(m, r) { Ok(v) => v, Err(e) => { return Err(e); } };
        resources.push(c);
        r += 1;
    }
    let mut partitions: Vec<PartitionCfg> = Vec::new();
    let mut p: usize = 0;
    while p < m.partitions.len()
        invariant
            p <= m.partitions@.len(),
            partitions@.len() == p,
            forall|k: int| 0 <= k < p ==> partition_ok(*m, k, #[trigger] partitions@[k]),
        decreases m.partitions@.len() - p,
    {
        let c = match partition_cfg(m, p) { Ok(v) => v, Err(e) => { return Err(e); } };
        partitions.push(c);
        p += 1;
    }
    let n = m.tasks.len();
    if n > 84 {
        // `3 * n + 2` has no place in a byte; the value reported is exact
        // for every count below a third of `u64::MAX`.
        let v: u64 = if (n as u64) <= 0x5555_5555_5555_5554u64 { 3 * (n as u64) + 2 } else { u64::MAX };
        return Err(BuildErr::Narrow(Field::SlotCount, v));
    }
    let slot_count: u8 = (3 * n + 2) as u8;
    let t = Tables {
        tasks,
        resources,
        access,
        partitions,
        operating_duration: m.operating_duration_ticks,
        slot_release,
        slot_deadline,
        slot_budget,
        slot_count,
    };
    proof {
        assert(t@.access.len() == m.tasks@.len());
        assert forall|k: int| 0 <= k < m.tasks@.len() implies {
            &&& (#[trigger] t@.access[k]).len() == m.resources@.len()
            &&& forall|q: int| 0 <= q < m.resources@.len() ==> (#[trigger] t@.access[k][q]) == accesses(*m, q, m.tasks@[k].id@)
        } by {
            assert(t@.access[k] == t.access@[k]@);
            assert(task_ok(*m, k, t.tasks@[k]));
        }
    }
    Ok(t)
}

// ------------------------------------------------- the `LAYOUT` image --

/// Why the model has no `LAYOUT` image.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LayoutErr {
    /// A Kernel window's base beyond 32 bits.
    KernelWindowBase,
    /// The Partition of this index: a window's base beyond 32 bits.
    WindowBase(usize),
    /// The Partition of this index: the base of its code (0) or RAM (1)
    /// interval beyond 32 bits.
    PartitionBase(usize, u8),
    /// The Partition of this index: a stack size beyond 32 bits.
    Stack(usize),
    /// The Partition of this index: more windows than the table has slots.
    Windows(usize),
    /// The base of `kernel_flash` (0), `shared_code` (1) or `kernel_ram`
    /// (2) beyond 32 bits.
    KernelBase(u8),
    /// More Kernel windows than the table has slots.
    KernelWindows,
}

/// The interval is the model's region.
pub open spec fn interval_is(r: Region, i: IntervalImg) -> bool {
    i.base == r.base && i.size_log2 == r.size_log2 && i.srd == r.srd
}

/// Slot `k` of the Kernel's window table: the model's window of that
/// index, or the unused slot `(0, 5)`.
pub open spec fn kernel_slot_ok(m: Model, k: int, s: (u32, u8)) -> bool {
    if k < m.kernel_windows@.len() {
        s.0 == m.kernel_windows@[k].region.base && s.1 == m.kernel_windows@[k].region.size_log2
    } else {
        s == (0u32, 5u8)
    }
}

/// Slot `w` of Partition `p`'s window table: the model's window of that
/// index, or the unused slot `(0, 5, false)`.
pub open spec fn window_slot_ok(m: Model, p: int, w: int, s: (u32, u8, bool)) -> bool {
    if w < m.partitions@[p].windows@.len() {
        &&& s.0 == m.partitions@[p].windows@[w].region.base
        &&& s.1 == m.partitions@[p].windows@[w].region.size_log2
        &&& s.2 == m.partitions@[p].windows@[w].read_only
    } else {
        s == (0u32, 5u8, false)
    }
}

/// The layout entry of Partition `p` holds the model's values.
pub open spec fn part_layout_ok(m: Model, p: int, l: PartitionLayoutV) -> bool {
    let part = m.partitions@[p];
    &&& interval_is(part.code, l.code)
    &&& interval_is(part.ram, l.ram)
    &&& l.stack_size == part.stack
    &&& l.window_count == part.windows@.len()
    &&& l.windows.len() == MAX_WINDOWS
    &&& forall|w: int| 0 <= w < MAX_WINDOWS ==> window_slot_ok(m, p, w, #[trigger] l.windows[w])
}

/// The layout is the model's: every interval, every window slot and each
/// Partition's stack size.
pub open spec fn layout_ok(m: Model, l: LayoutV) -> bool {
    &&& interval_is(m.kernel_flash, l.kernel_flash)
    &&& interval_is(m.shared_code, l.shared_code)
    &&& interval_is(m.kernel_ram, l.kernel_ram)
    &&& l.kernel_window_count == m.kernel_windows@.len()
    &&& l.kernel_windows.len() == MAX_KERNEL_WINDOWS
    &&& forall|k: int| 0 <= k < MAX_KERNEL_WINDOWS ==> kernel_slot_ok(m, k, #[trigger] l.kernel_windows[k])
    &&& l.partitions.len() == m.partitions@.len()
    &&& forall|p: int| 0 <= p < m.partitions@.len() ==> part_layout_ok(m, p, #[trigger] l.partitions[p])
}

fn interval(r: &Region) -> (x: Option<IntervalImg>)
    ensures x is Some ==> interval_is(*r, x.unwrap()),
{
    if r.base > 0xffff_ffff {
        None
    } else {
        Some(IntervalImg { base: r.base as u32, size_log2: r.size_log2, srd: r.srd })
    }
}

/// The Kernel's window table: `MAX_KERNEL_WINDOWS` slots.
fn kernel_slots(m: &Model) -> (x: Result<Vec<(u32, u8)>, LayoutErr>)
    ensures
        x is Ok ==> x.unwrap()@.len() == MAX_KERNEL_WINDOWS && forall|k: int| 0 <= k < MAX_KERNEL_WINDOWS ==> kernel_slot_ok(*m, k, #[trigger] x.unwrap()@[k]),
{
    let mut out: Vec<(u32, u8)> = Vec::new();
    let mut k: usize = 0;
    while k < MAX_KERNEL_WINDOWS
        invariant
            k <= MAX_KERNEL_WINDOWS,
            out@.len() == k,
            forall|j: int| 0 <= j < k ==> kernel_slot_ok(*m, j, #[trigger] out@[j]),
        decreases MAX_KERNEL_WINDOWS - k,
    {
        if k < m.kernel_windows.len() {
            let w = &m.kernel_windows[k];
            if w.region.base > 0xffff_ffff {
                return Err(LayoutErr::KernelWindowBase);
            }
            out.push((w.region.base as u32, w.region.size_log2));
        } else {
            out.push((0, 5));
        }
        k += 1;
    }
    Ok(out)
}

/// Partition `p`'s window table: `MAX_WINDOWS` slots.
fn window_slots(m: &Model, p: usize) -> (x: Result<Vec<(u32, u8, bool)>, LayoutErr>)
    requires p < m.partitions@.len(),
    ensures
        x is Ok ==> x.unwrap()@.len() == MAX_WINDOWS && forall|w: int| 0 <= w < MAX_WINDOWS ==> window_slot_ok(*m, p as int, w, #[trigger] x.unwrap()@[w]),
{
    let part = &m.partitions[p];
    let mut out: Vec<(u32, u8, bool)> = Vec::new();
    let mut w: usize = 0;
    while w < MAX_WINDOWS
        invariant
            p < m.partitions@.len(),
            *part == m.partitions@[p as int],
            w <= MAX_WINDOWS,
            out@.len() == w,
            forall|j: int| 0 <= j < w ==> window_slot_ok(*m, p as int, j, #[trigger] out@[j]),
        decreases MAX_WINDOWS - w,
    {
        if w < part.windows.len() {
            let win = &part.windows[w];
            if win.region.base > 0xffff_ffff {
                return Err(LayoutErr::WindowBase(p));
            }
            out.push((win.region.base as u32, win.region.size_log2, win.read_only));
        } else {
            out.push((0, 5, false));
        }
        w += 1;
    }
    Ok(out)
}

/// The layout entry of Partition `p`.
fn part_layout(m: &Model, p: usize) -> (x: Result<PartitionLayoutImg, LayoutErr>)
    requires p < m.partitions@.len(),
    ensures x is Ok ==> part_layout_ok(*m, p as int, x.unwrap()@),
{
    let part = &m.partitions[p];
    let windows = match window_slots(m, p) { Ok(v) => v, Err(e) => { return Err(e); } };
    let code = match interval(&part.code) { Some(v) => v, None => { return Err(LayoutErr::PartitionBase(p, 0)); } };
    let ram = match interval(&part.ram) { Some(v) => v, None => { return Err(LayoutErr::PartitionBase(p, 1)); } };
    if part.stack > 0xffff_ffff {
        return Err(LayoutErr::Stack(p));
    }
    if part.windows.len() > MAX_WINDOWS {
        return Err(LayoutErr::Windows(p));
    }
    Ok(PartitionLayoutImg { code, ram, stack_size: part.stack as u32, window_count: part.windows.len() as u8, windows })
}

/// The `LAYOUT` image the Task_Model calls for (R16.7, R26.4): every
/// interval, every window slot (an unused one is `(0, 5)`, not read-only)
/// and each Partition's stack size, from the model's MPU values.
pub fn build_layout(m: &Model) -> (x: Result<LayoutImg, LayoutErr>)
    ensures
        x is Ok ==> layout_ok(*m, x.unwrap()@),
        x is Ok && m.partitions@.len() <= 255 ==> crate::image_v::layout_wf(x.unwrap()@),
{
    let kernel_windows = match kernel_slots(m) { Ok(v) => v, Err(e) => { return Err(e); } };
    let mut partitions: Vec<PartitionLayoutImg> = Vec::new();
    let mut p: usize = 0;
    while p < m.partitions.len()
        invariant
            p <= m.partitions@.len(),
            partitions@.len() == p,
            forall|k: int| 0 <= k < p ==> part_layout_ok(*m, k, (#[trigger] partitions@[k])@),
        decreases m.partitions@.len() - p,
    {
        let l = match part_layout(m, p) { Ok(v) => v, Err(e) => { return Err(e); } };
        partitions.push(l);
        p += 1;
    }
    let kernel_flash = match interval(&m.kernel_flash) { Some(v) => v, None => { return Err(LayoutErr::KernelBase(0)); } };
    let shared_code = match interval(&m.shared_code) { Some(v) => v, None => { return Err(LayoutErr::KernelBase(1)); } };
    let kernel_ram = match interval(&m.kernel_ram) { Some(v) => v, None => { return Err(LayoutErr::KernelBase(2)); } };
    if m.kernel_windows.len() > MAX_KERNEL_WINDOWS {
        return Err(LayoutErr::KernelWindows);
    }
    let l = LayoutImg { kernel_flash, shared_code, kernel_ram, kernel_window_count: m.kernel_windows.len() as u8, kernel_windows, partitions };
    proof {
        assert forall|k: int| 0 <= k < m.partitions@.len() implies part_layout_ok(*m, k, #[trigger] l@.partitions[k]) by {
            assert(l@.partitions[k] == l.partitions@[k]@);
        }
        assert forall|k: int| 0 <= k < l@.partitions.len() implies (#[trigger] l@.partitions[k]).windows.len() == MAX_WINDOWS by {
            assert(l@.partitions[k] == l.partitions@[k]@);
            assert(part_layout_ok(*m, k, l.partitions@[k]@));
        }
    }
    Ok(l)
}

} // verus!
