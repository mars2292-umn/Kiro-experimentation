//! Derived values (R23): Ceilings, nesting depths, capacities (the timer
//! slots by the rule of R9.4, the lock stack, the event log), the
//! priority-to-hardware map, the dispatch vectors, the converted times
//! (R21.2), and the Generated_Config checksum (R24.1). Every derived value
//! is a function of the validated declaration alone, so two runs on the
//! same declaration give the same values (R23.6), and Ceilings and
//! capacities do not depend on the declaration order (R23.2, Property 13).

use crate::convert::{to_cycles, to_ticks, Change, Rounding};
use crate::decl::{Declaration, Diagnostic, ItemRef, Level, MitPolicy, Release, Response, Source};
use crate::layout::{self, Plan};
use crate::target::Target;

/// "No timed-event slot" (`rsk_kernel::logic::NO_SLOT`).
pub const NO_SLOT: u8 = 0xFF;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Kind {
    Periodic,
    Sporadic,
}

impl Kind {
    pub fn name(self) -> &'static str {
        match self {
            Kind::Periodic => "periodic",
            Kind::Sporadic => "sporadic",
        }
    }
    /// The Kernel encoding (`KIND_PERIODIC` = 0, `KIND_SPORADIC` = 1).
    pub fn code(self) -> u8 {
        match self {
            Kind::Periodic => 0,
            Kind::Sporadic => 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ReleaseSource {
    /// A peripheral interrupt: the peripheral and its NVIC line.
    Interrupt { peripheral: String, irq: u16 },
    Signal(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenTask {
    /// `partition.task`, the stable identifier (R25.2).
    pub id: String,
    pub name: String,
    pub partition: usize,
    pub index: usize,
    pub priority: u8,
    pub kind: Kind,
    pub offset_ticks: u64,
    /// Period (periodic) or MIT (sporadic).
    pub period_ticks: u64,
    pub deadline_ticks: u64,
    pub budget_cycles: u32,
    pub fpu: bool,
    pub mit_policy: MitPolicy,
    pub source: Option<ReleaseSource>,
    /// Global Resource indices, ascending.
    pub resources: Vec<usize>,
    pub raises: Vec<String>,
    /// PR-07: at most the number of declared Resources.
    pub nesting_depth: u8,
    pub dispatch_irq: u16,
    pub slot_release: u8,
    pub slot_deadline: u8,
    pub slot_budget: u8,
    /// The Rust path of the Job wrapper in the Partition crate.
    pub entry_path: String,
}

impl GenTask {
    pub fn vector(&self) -> u16 {
        16 + self.dispatch_irq
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenResource {
    pub id: String,
    pub name: String,
    pub partition: usize,
    pub index: usize,
    pub ceiling: u8,
    /// Global Task indices, ascending.
    pub accessors: Vec<usize>,
    pub ty: String,
    pub init: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct GenPartition {
    pub name: String,
    pub index: usize,
    pub krate: String,
    pub level: Level,
    pub fault: Response,
    pub overrun: Response,
    pub deadline_miss: Response,
    pub mit_threshold: u32,
    pub code_bytes: u64,
    pub ram_bytes: u64,
    pub stack_bytes: u64,
    pub init_arena: u64,
    pub peripherals: Vec<String>,
    pub tasks: Vec<usize>,
    pub resources: Vec<usize>,
    pub signals: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Generated {
    pub target: &'static Target,
    pub variant: String,
    pub operating_duration_ticks: u64,
    pub safe_state: String,
    pub tasks: Vec<GenTask>,
    pub resources: Vec<GenResource>,
    pub partitions: Vec<GenPartition>,
    pub plan: Plan,
    /// Capacities (PR-14): lock stack, timed-event slots, event log.
    pub lock_capacity: usize,
    pub slot_count: usize,
    pub log_capacity: usize,
    /// (Task level, NVIC priority byte) for every level in use, ascending level.
    pub priority_map: Vec<(u8, u8)>,
    pub kernel_levels: Vec<u8>,
    pub changes: Vec<Change>,
    /// CRC-32 over the Generated_Config tables (R24.1, R12.3).
    pub checksum: u32,
}

/// NVIC priority byte of a Task level (DD-02, `arch::cm4::encode_level`).
pub fn encode_level(level: u8) -> u8 {
    if level == 0 {
        0
    } else {
        (8 - level) << 5
    }
}

pub fn derive(decl: &Declaration, target: &'static Target) -> Result<Generated, Vec<Diagnostic>> {
    let (tick_hz, cpu_hz) = (target.tick_hz, target.cpu_hz);
    let mut changes = Vec::new();
    let mut note = |what: String, declared, converted: u64, unit: &'static str, rounded: bool| {
        if rounded {
            changes.push(Change {
                what,
                declared,
                converted,
                unit,
            });
        }
    };
    let (operating_duration_ticks, r) = to_ticks(decl.operating_duration, tick_hz, cpu_hz, Rounding::Down).expect("validated");
    note("operating_duration".into(), decl.operating_duration, operating_duration_ticks, "ticks", r);

    let plan = layout::plan(decl, target)?;

    // Global numbering in declaration order.
    let mut partitions = Vec::new();
    let mut resources = Vec::new();
    let mut tasks = Vec::new();
    for (pi, p) in decl.partitions.iter().enumerate() {
        let first_resource = resources.len();
        for r in &p.resources {
            resources.push(GenResource {
                id: format!("{}.{}", p.name, r.name),
                name: r.name.clone(),
                partition: pi,
                index: resources.len(),
                ceiling: 0,
                accessors: Vec::new(),
                ty: r.ty.clone(),
                init: r.init.clone(),
            });
        }
        let first_task = tasks.len();
        for t in &p.tasks {
            let index = tasks.len();
            let (budget_cycles, rb) = to_cycles(t.budget, tick_hz, cpu_hz, Rounding::Up).expect("validated");
            note(format!("{}.{}.budget", p.name, t.name), t.budget, budget_cycles, "cycles", rb);
            let (deadline_ticks, rd) = to_ticks(t.deadline, tick_hz, cpu_hz, Rounding::Down).expect("validated");
            note(format!("{}.{}.deadline", p.name, t.name), t.deadline, deadline_ticks, "ticks", rd);
            let (kind, period_ticks, offset_ticks, mit_policy, source) = match &t.release {
                Release::Periodic { period, offset } => {
                    let (pt, rp) = to_ticks(*period, tick_hz, cpu_hz, Rounding::Down).expect("validated");
                    note(format!("{}.{}.period", p.name, t.name), *period, pt, "ticks", rp);
                    let ot = match offset {
                        Some(o) => {
                            let (ot, ro) = to_ticks(*o, tick_hz, cpu_hz, Rounding::Nearest).expect("validated");
                            note(format!("{}.{}.offset", p.name, t.name), *o, ot, "ticks", ro);
                            ot
                        }
                        None => 0,
                    };
                    (Kind::Periodic, pt, ot, MitPolicy::Defer, None)
                }
                Release::Sporadic { mit, policy, source } => {
                    let (mt, rm) = to_ticks(*mit, tick_hz, cpu_hz, Rounding::Down).expect("validated");
                    note(format!("{}.{}.mit", p.name, t.name), *mit, mt, "ticks", rm);
                    let src = match source {
                        Source::Interrupt(name) => {
                            let per = target.peripheral(name).expect("validated");
                            ReleaseSource::Interrupt {
                                peripheral: name.clone(),
                                irq: per.irq.expect("validated"),
                            }
                        }
                        Source::Signal(name) => ReleaseSource::Signal(name.clone()),
                    };
                    (Kind::Sporadic, mt, 0, *policy, Some(src))
                }
            };
            let mut res: Vec<usize> = t
                .resources
                .iter()
                .map(|name| first_resource + p.resources.iter().position(|r| &r.name == name).expect("validated"))
                .collect();
            res.sort_unstable();
            for &ri in &res {
                resources[ri].accessors.push(index);
            }
            let nesting_depth = res.len() as u8;
            tasks.push(GenTask {
                id: format!("{}.{}", p.name, t.name),
                name: t.name.clone(),
                partition: pi,
                index,
                priority: t.priority,
                kind,
                offset_ticks,
                period_ticks,
                deadline_ticks,
                budget_cycles: budget_cycles as u32,
                fpu: t.fpu,
                mit_policy,
                source,
                resources: res,
                raises: t.raises.clone(),
                nesting_depth,
                dispatch_irq: target.dispatch_irqs[index],
                slot_release: NO_SLOT,
                slot_deadline: NO_SLOT,
                slot_budget: NO_SLOT,
                entry_path: format!("::{}::rsk_jobs::{}", p.krate, t.name),
            });
        }
        partitions.push(GenPartition {
            name: p.name.clone(),
            index: pi,
            krate: p.krate.clone(),
            level: p.level,
            fault: p.fault,
            overrun: p.overrun,
            deadline_miss: p.deadline_miss,
            mit_threshold: p.mit_threshold,
            code_bytes: p.code,
            ram_bytes: p.ram,
            stack_bytes: p.stack,
            init_arena: p.init_arena,
            peripherals: p.peripherals.clone(),
            tasks: (first_task..tasks.len()).collect(),
            resources: (first_resource..resources.len()).collect(),
            signals: p.signals.iter().map(|s| s.name.clone()).collect(),
        });
    }
    // Ceilings (R23.1): the maximum Priority over the accessor set, a
    // function of the set alone (R23.2).
    for r in &mut resources {
        r.ceiling = r.accessors.iter().map(|&t| tasks[t].priority).max().expect("validated: at least one accessor");
    }
    // Timed-event slots (R9.4, PR-14): one release slot per Periodic_Task
    // or deferring Sporadic_Task, one deadline and one Budget slot per
    // Task, then the end-of-operation and housekeeping slots.
    let mut next = 0u8;
    for t in &mut tasks {
        if t.kind == Kind::Periodic || t.mit_policy == MitPolicy::Defer {
            t.slot_release = next;
            next += 1;
        }
        t.slot_deadline = next;
        next += 1;
        t.slot_budget = next;
        next += 1;
    }
    let slot_count = next as usize + 2;
    if slot_count > 253 {
        return Err(vec![Diagnostic::new(
            "PR-14",
            ItemRef::Declaration,
            format!("{slot_count} timed-event slots exceed the 253 the Kernel addresses"),
        )]);
    }
    // Lock stack (PR-07): every in-progress Job holds at most its declared
    // Resources, so the sum over Tasks bounds the stack.
    let lock_capacity = tasks.iter().map(|t| t.resources.len()).sum::<usize>().max(1);
    let mut levels: Vec<u8> = tasks.iter().map(|t| t.priority).collect();
    levels.sort_unstable();
    levels.dedup();
    let priority_map = levels.iter().map(|&l| (l, encode_level(l))).collect();

    let mut g = Generated {
        target,
        variant: decl.variant.clone(),
        operating_duration_ticks,
        safe_state: decl.safe_state.clone(),
        tasks,
        resources,
        partitions,
        plan,
        lock_capacity,
        slot_count,
        log_capacity: decl.log_capacity as usize,
        priority_map,
        kernel_levels: vec![0],
        changes,
        checksum: 0,
    };
    g.checksum = checksum(&g);
    Ok(g)
}

/// CRC-32 (IEEE 802.3, reflected, as `crc32` tools compute it).
pub fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xFFFF_FFFFu32;
    for &b in bytes {
        crc ^= b as u32;
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xEDB8_8320 & mask);
        }
    }
    !crc
}

/// The canonical byte image of the Generated_Config tables the Kernel
/// holds (R24.1): the same field order that `rsk_kernel::arch::config_bytes`
/// hashes at boot (R12.3) and that the Config_Checker recomputes (R26.4).
pub fn config_bytes(g: &Generated) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(&(g.tasks.len() as u32).to_le_bytes());
    out.extend_from_slice(&(g.resources.len() as u32).to_le_bytes());
    out.extend_from_slice(&(g.partitions.len() as u32).to_le_bytes());
    for t in &g.tasks {
        out.push(t.priority);
        out.push(t.partition as u8);
        out.push(t.kind.code());
        out.extend_from_slice(&t.offset_ticks.to_le_bytes());
        out.extend_from_slice(&t.period_ticks.to_le_bytes());
        out.extend_from_slice(&t.deadline_ticks.to_le_bytes());
        out.extend_from_slice(&t.budget_cycles.to_le_bytes());
        out.push(match t.mit_policy {
            MitPolicy::Defer => 0,
            MitPolicy::Discard => 1,
        });
        out.push(t.fpu as u8);
    }
    for r in &g.resources {
        out.push(r.ceiling);
        out.push(r.partition as u8);
    }
    for t in &g.tasks {
        for r in 0..g.resources.len() {
            out.push(t.resources.contains(&r) as u8);
        }
    }
    for p in &g.partitions {
        out.push(p.level.code());
        out.push(p.fault.code());
        out.push(p.overrun.code());
        out.push(p.deadline_miss.code());
        out.extend_from_slice(&p.mit_threshold.to_le_bytes());
    }
    out.extend_from_slice(&g.operating_duration_ticks.to_le_bytes());
    for t in &g.tasks {
        out.push(t.slot_release);
    }
    for t in &g.tasks {
        out.push(t.slot_deadline);
    }
    for t in &g.tasks {
        out.push(t.slot_budget);
    }
    out.push(g.slot_count as u8);
    out
}

pub fn checksum(g: &Generated) -> u32 {
    crc32(&config_bytes(g))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_reference_vector() {
        assert_eq!(crc32(b"123456789"), 0xCBF4_3926);
        assert_eq!(crc32(b""), 0);
    }

    #[test]
    fn level_encoding_matches_dd02() {
        assert_eq!(encode_level(7), 0x20);
        assert_eq!(encode_level(1), 0xE0);
        assert_eq!(encode_level(0), 0);
    }
}
