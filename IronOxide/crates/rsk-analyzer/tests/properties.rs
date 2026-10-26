//! Task 15.4: the Analyzer's correctness evidence beyond the Verus proof
//! (R32.2 to R32.5, R32.7). No property-testing crate is vendored, so the
//! generators run on a fixed-seed linear congruential generator: every run
//! sees the same sets, and a failing set prints in full.
//!
//! - Property 17 (R32.2, R32.3): monotonicity and locality.
//! - R32.4: agreement with a reference implementation of the R29 equations
//!   written separately (`reference_rta`: u128 arithmetic, Tasks sorted by
//!   Priority, the recurrence evaluated directly, no shared code).
//! - Property 18 (R32.5): a discrete-time SRP simulator with Overruns by
//!   Tasks of stop-on-overrun Partitions observes no response time above
//!   the computed WCRT for a set reported PASS.
//! - R32.7: published task sets (PAR-04 = 3) reproduced.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;

use rsk_analyzer::inputs::{self, AnalysedTask, Options};
use rsk_analyzer::{placeholders, response_times, Wcrt};
use rsk_model::task_model::TaskModel;

const DEMO: &str = include_str!("../../../apps/p1-demo/gen/task_model.json");

struct Rng(u64);

impl Rng {
    fn next(&mut self) -> u64 {
        self.0 = self.0.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        self.0 >> 33
    }
    fn range(&mut self, lo: u64, hi: u64) -> u64 {
        lo + self.next() % (hi - lo + 1)
    }
}

fn task(id: &str, priority: u8, c: u64, period: u64, deadline: u64) -> AnalysedTask {
    AnalysedTask {
        id: id.into(),
        partition: "p0".into(),
        level: "A".into(),
        priority,
        period_cycles: period,
        deadline_cycles: deadline,
        c,
        b: 0,
        b_source: String::new(),
        o: 0,
        j: 0,
        e: c,
        stops_overruns: false,
        sporadic: false,
        longest_cs: 0,
        wcet_provenance: vec![],
        wcet: c,
    }
}

/// A random set of 2 to 6 Tasks with distinct Priorities, periods 5 to 60,
/// Budgets 1 to 6, implicit deadlines, small blocking, overhead, and jitter
/// terms, and E = C + O (continuing Partitions).
fn random_set(rng: &mut Rng) -> Vec<AnalysedTask> {
    let n = rng.range(2, 6) as usize;
    let mut prios: Vec<u8> = (1..=7).collect();
    for i in (1..prios.len()).rev() {
        let j = rng.range(0, i as u64) as usize;
        prios.swap(i, j);
    }
    (0..n)
        .map(|i| {
            let period = rng.range(5, 60);
            let mut t = task(&format!("t{i}"), prios[i], rng.range(1, 6), period, period);
            t.b = rng.range(0, 3);
            t.o = rng.range(0, 2);
            t.j = rng.range(0, 2);
            t.e = t.c + t.o;
            t
        })
        .collect()
}

fn by_id<'a>(w: &'a [Wcrt], id: &str) -> &'a Wcrt {
    w.iter().find(|x| x.id == id).unwrap()
}

// ------------------------------------------------------------- R32.2

/// Property 17, monotonicity: increasing C, B, O, J, or E of any Task, or
/// decreasing a period, never decreases a computed WCRT where both results
/// are fixed points, and never turns an unschedulable set schedulable. (A
/// not-schedulable result is the last iterate before the deadline stop, a
/// lower bound that is not itself monotone, so the comparison is on fixed
/// points and on verdicts; verification_plan.md 5.10.)
#[test]
fn r32_2_wcrts_are_monotone_in_every_term() {
    let mut rng = Rng(7);
    let mut compared = 0;
    for _ in 0..300 {
        let base = random_set(&mut rng);
        let before = response_times(&base);
        let mut after_set = base.clone();
        let k = rng.range(0, after_set.len() as u64 - 1) as usize;
        let which = rng.range(0, 5);
        match which {
            0 => {
                after_set[k].c += 1;
                after_set[k].e += 1;
            }
            1 => after_set[k].b += 1,
            2 => {
                after_set[k].o += 1;
                after_set[k].e += 1;
            }
            3 => after_set[k].j += 1,
            4 => after_set[k].e += 1,
            _ => {
                if after_set[k].period_cycles > 1 {
                    after_set[k].period_cycles -= 1;
                }
            }
        }
        let after = response_times(&after_set);
        for (b, a) in before.iter().zip(after.iter()) {
            if b.overflow || a.overflow {
                continue;
            }
            if b.schedulable && a.schedulable {
                assert!(a.r >= b.r, "WCRT of {} decreased from {} to {} after change {which} on t{k}:\n{base:#?}", b.id, b.r, a.r);
                compared += 1;
            }
            assert!(!(a.schedulable && !b.schedulable), "{} became schedulable after a worsening change {which} on t{k}:\n{base:#?}", b.id);
        }
    }
    assert!(compared > 300, "{compared} fixed-point pairs compared");
}

// ------------------------------------------------------------- R32.3

/// Property 17, locality: adding, removing, or changing a Task of lower
/// Priority than Task i that holds no Resource with a Ceiling at or above
/// P_i changes R_i by at most the Kernel overhead the changed Task
/// contributes. In the R29.1 recurrence a lower-Priority Task enters R_i
/// only through B_i, so without a relevant Critical_Section the change is
/// exactly zero; the one shared term is the FP save and restore that the
/// first FPU-using Task adds to every O_i (R29.3).
#[test]
fn r32_3_lower_priority_tasks_without_a_relevant_resource_do_not_change_wcrts() {
    let mut rng = Rng(11);
    for _ in 0..300 {
        let base = random_set(&mut rng);
        let before = response_times(&base);
        let lowest = base.iter().map(|t| t.priority).min().unwrap();
        // Add a new lowest-Priority Task.
        let mut added = base.clone();
        let mut extra = task("extra", 0, rng.range(1, 6), rng.range(5, 60), 60);
        extra.e = extra.c;
        extra.priority = 0;
        added.push(extra);
        let after_add = response_times(&added);
        for t in &base {
            assert_eq!(by_id(&before, &t.id).r, by_id(&after_add, &t.id).r, "adding a lowest-Priority Task changed {}", t.id);
        }
        // Change the lowest existing Task's Budget and interference bound.
        let mut changed = base.clone();
        let li = changed.iter().position(|t| t.priority == lowest).unwrap();
        changed[li].c += 3;
        changed[li].e += 3;
        let after_change = response_times(&changed);
        for t in base.iter().filter(|t| t.priority > lowest) {
            assert_eq!(by_id(&before, &t.id).r, by_id(&after_change, &t.id).r, "changing the lowest Task altered {}", t.id);
        }
        // Remove it.
        let removed: Vec<AnalysedTask> = base.iter().filter(|t| t.priority != lowest).cloned().collect();
        let after_remove = response_times(&removed);
        for t in &removed {
            assert_eq!(by_id(&before, &t.id).r, by_id(&after_remove, &t.id).r, "removing the lowest Task altered {}", t.id);
        }
    }
    // On the P1 demo through `prepare`: without `part1.log` (Priority 2, no
    // Resource) every higher Task keeps its WCRT.
    let m: TaskModel = rsk_model::read_task_model(DEMO).unwrap();
    let manifest = manifest(&m);
    let (w, t) = placeholders::make(&m, "sha256:demo", "off");
    let full = response_times(&inputs::prepare(&m, &w, &t, &manifest, &dev()).unwrap().tasks);
    let mut without = m.clone();
    without.tasks.retain(|x| x.id != "part1.log");
    let reduced = response_times(&inputs::prepare(&without, &w, &t, &manifest, &dev()).unwrap().tasks);
    for x in &reduced {
        let r = by_id(&full, &x.id).r;
        if m.task(&x.id).unwrap().priority > 2 {
            assert_eq!(x.r, r, "{} changed without part1.log", x.id);
        }
    }
    // Adding the first FPU-using Task to a set changes every O_i by exactly
    // Δ_fp_save + Δ_fp_restore (600 placeholder cycles), nothing else.
    let mut no_fpu = m.clone();
    for x in &mut no_fpu.tasks {
        x.fpu = false;
    }
    let plain = inputs::prepare(&no_fpu, &w, &t, &manifest, &dev()).unwrap().tasks;
    let with = inputs::prepare(&m, &w, &t, &manifest, &dev()).unwrap().tasks;
    for (p, q) in plain.iter().zip(with.iter()) {
        assert_eq!(q.o, p.o + 600, "{}", p.id);
        assert_eq!((q.c, q.b, q.j), (p.c, p.b, p.j));
    }
}

fn manifest(model: &TaskModel) -> serde_json::Value {
    serde_json::json!({
        "schema": "rsk-build-manifest/1",
        "generated_config_checksum": model.config_checksum,
        "build": {"icache": "off"},
        "binaries": [{"hash": "sha256:demo", "package": "p1-demo"}]
    })
}

fn dev() -> Options {
    Options { allow_placeholders: true, prefer_method: None }
}

// ------------------------------------------------------------- R32.4

/// A reference implementation of R29.1, R29.5, R29.7, and R29.9 written
/// without the Analyzer's code: Tasks sorted by Priority, the recurrence
/// evaluated as written, u128 arithmetic, iteration until the value repeats
/// or passes the deadline.
fn reference_rta(tasks: &[AnalysedTask]) -> Vec<(String, u128, bool)> {
    let mut order: Vec<&AnalysedTask> = tasks.iter().collect();
    order.sort_by_key(|t| std::cmp::Reverse(t.priority));
    let mut out = Vec::new();
    for t in &order {
        let hep: Vec<&AnalysedTask> = order.iter().copied().filter(|u| u.priority >= t.priority && u.id != t.id).collect();
        let mut w: u128 = t.c as u128 + t.b as u128 + t.o as u128;
        let d = t.deadline_cycles as u128;
        let j = t.j as u128;
        let result = loop {
            if w + j > d {
                break (w, false);
            }
            let mut next: u128 = t.c as u128 + t.b as u128 + t.o as u128;
            for u in &hep {
                let p = u.period_cycles.max(1) as u128;
                let jobs = (w + u.j as u128 + p - 1) / p;
                next += jobs * u.e as u128;
            }
            if next == w {
                break (w, true);
            }
            w = next;
        };
        out.push((t.id.clone(), result.0 + j, result.1 && result.0 + j <= d));
    }
    out
}

#[test]
fn r32_4_the_analyzer_agrees_with_the_reference_implementation() {
    let mut rng = Rng(23);
    for _ in 0..500 {
        let set = random_set(&mut rng);
        let ours = response_times(&set);
        let reference = reference_rta(&set);
        for (id, r_ref, ok_ref) in &reference {
            let w = by_id(&ours, id);
            assert!(!w.overflow);
            assert_eq!((w.r as u128, w.schedulable), (*r_ref, *ok_ref), "{id} in\n{set:#?}");
        }
    }
    let m: TaskModel = rsk_model::read_task_model(DEMO).unwrap();
    let (w, t) = placeholders::make(&m, "sha256:demo", "off");
    let prepared = inputs::prepare(&m, &w, &t, &manifest(&m), &dev()).unwrap().tasks;
    let ours = response_times(&prepared);
    for (id, r_ref, ok_ref) in reference_rta(&prepared) {
        let w = by_id(&ours, &id);
        assert_eq!((w.r as u128, w.schedulable), (r_ref, ok_ref), "{id}");
    }
}

// ------------------------------------------------------------- R32.5

/// A Task of the simulated set: synchronous periodic release, execution
/// C, one optional Critical_Section of length L at the end of its
/// execution on a Resource with a Ceiling, a Partition, and whether that
/// Partition stops overrunning Jobs.
#[derive(Clone, Debug)]
struct SimTask {
    id: String,
    priority: u8,
    period: u64,
    c: u64,
    cs: Option<(usize, u64)>, // (resource, length)
    partition: usize,
    stops: bool,
}

#[derive(Clone, Debug)]
struct SimSet {
    tasks: Vec<SimTask>,
    /// Ceiling of each Resource (max Priority of its accessors).
    ceilings: Vec<u8>,
    /// Δ_detect + Δ_enforce of the simulated Kernel.
    delta: u64,
}

fn random_sim_set(rng: &mut Rng) -> SimSet {
    let n = rng.range(2, 5) as usize;
    let mut prios: Vec<u8> = (1..=7).collect();
    for i in (1..prios.len()).rev() {
        let j = rng.range(0, i as u64) as usize;
        prios.swap(i, j);
    }
    let mut tasks: Vec<SimTask> = (0..n)
        .map(|i| SimTask {
            id: format!("t{i}"),
            priority: prios[i],
            period: rng.range(6, 30),
            c: rng.range(1, 4),
            cs: None,
            partition: rng.range(0, 1) as usize,
            stops: false,
        })
        .collect();
    // Partition 1 stops overrunning Jobs; Partition 0 lets them continue.
    for t in &mut tasks {
        t.stops = t.partition == 1;
    }
    // One Resource shared by up to two Tasks of the same Partition (PR-30),
    // held for the last L ≤ C units of execution.
    let mut ceilings = Vec::new();
    let p = rng.range(0, 1) as usize;
    let members: Vec<usize> = (0..n).filter(|&i| tasks[i].partition == p).collect();
    if members.len() >= 2 {
        let a = members[0];
        let b = members[1];
        let la = rng.range(1, tasks[a].c);
        let lb = rng.range(1, tasks[b].c);
        tasks[a].cs = Some((0, la));
        tasks[b].cs = Some((0, lb));
        ceilings.push(tasks[a].priority.max(tasks[b].priority));
    }
    SimSet { tasks, ceilings, delta: 2 }
}

/// The R29 terms of a simulated set (o = 0, j = 0): B_i from the lower
/// Tasks' Critical_Sections on Resources whose Ceiling is at least P_i: the
/// site's length, or Budget_l + Δ when Task l belongs to another Partition
/// that stops overrunning Jobs (R29.2: the Resource stays within Task l's
/// Partition under PR-30, but its Ceiling blocks every lower Priority,
/// across Partitions; the simulator found this clause necessary); E_j =
/// C + Δ + L_j for stopping Partitions, else C.
fn analysed(set: &SimSet) -> Vec<AnalysedTask> {
    set.tasks
        .iter()
        .map(|t| {
            let l = t.cs.map_or(0, |(_, l)| l);
            let mut a = task(&t.id, t.priority, t.c, t.period, t.period);
            a.partition = format!("p{}", t.partition);
            a.stops_overruns = t.stops;
            a.longest_cs = l;
            a.e = if t.stops { t.c + set.delta + l } else { t.c };
            a.b = set
                .tasks
                .iter()
                .filter(|u| u.priority < t.priority)
                .filter_map(|u| u.cs.map(|(r, l)| (u, set.ceilings[r], l)))
                .filter(|(_, ceiling, _)| *ceiling >= t.priority)
                .map(|(u, _, l)| if u.partition != t.partition && u.stops { u.c + set.delta } else { l })
                .max()
                .unwrap_or(0);
            a
        })
        .collect()
}

#[derive(Clone, Debug)]
struct Job {
    task: usize,
    release: u64,
    /// Units still to execute, including the Overrun extension.
    remaining: u64,
    /// Total units this Job executes (C, or C + extension for an Overrun).
    total: u64,
    executed: u64,
    overran: bool,
}

/// Unit-time SRP simulation: at each instant the highest-Priority pending
/// Job whose Priority exceeds the system Ceiling, or the Job that holds the
/// Ceiling, runs for one unit. A Job holds its Resource during the last L
/// units of its nominal execution and, if it overruns, until the Kernel
/// kills it at C + Δ (R14.5 releases the Resource). Returns, per Task, the
/// largest response time of Jobs that completed normally, and the
/// Partitions in which an Overrun occurred.
fn simulate(set: &SimSet, horizon: u64, rng: &mut Rng, inject_overruns: bool) -> (Vec<u64>, Vec<usize>) {
    let n = set.tasks.len();
    let mut worst = vec![0u64; n];
    let mut overran_partitions = Vec::new();
    let mut pending: Vec<Job> = Vec::new();
    let mut running: Option<usize> = None; // index into pending
    for t in 0..horizon {
        for (i, task) in set.tasks.iter().enumerate() {
            if t % task.period == 0 {
                let overrun = inject_overruns && task.stops && rng.range(0, 3) == 0;
                let total = if overrun { task.c + rng.range(1, set.delta) } else { task.c };
                pending.push(Job { task: i, release: t, remaining: total, total, executed: 0, overran: overrun });
            }
        }
        // The system Ceiling: the Resources held by Jobs inside their Critical_Section.
        let holds = |j: &Job| -> Option<u8> {
            let task = &set.tasks[j.task];
            let (r, l) = task.cs?;
            // Inside the CS from unit C - L onwards until completion or kill.
            if j.executed >= task.c - l { Some(set.ceilings[r]) } else { None }
        };
        let ceiling = pending.iter().filter_map(holds).max().unwrap_or(0);
        let holder = pending.iter().position(|j| holds(j).is_some());
        // Eligible: Priority above the Ceiling, or the holder itself.
        let mut best: Option<usize> = None;
        for (k, j) in pending.iter().enumerate() {
            let p = set.tasks[j.task].priority;
            if p > ceiling || Some(k) == holder {
                if best.is_none_or(|b| p > set.tasks[pending[b].task].priority) {
                    best = Some(k);
                }
            }
        }
        running = best;
        if let Some(k) = running {
            let j = &mut pending[k];
            j.executed += 1;
            j.remaining -= 1;
            let task = &set.tasks[j.task];
            let killed = j.overran && j.executed >= task.c + set.delta;
            if j.remaining == 0 || killed {
                if !j.overran {
                    worst[j.task] = worst[j.task].max(t + 1 - j.release);
                } else if !overran_partitions.contains(&task.partition) {
                    overran_partitions.push(task.partition);
                }
                let _ = j.total;
                pending.remove(k);
            }
        }
    }
    let _ = running;
    (worst, overran_partitions)
}

fn lcm(a: u64, b: u64) -> u64 {
    fn gcd(a: u64, b: u64) -> u64 {
        if b == 0 { a } else { gcd(b, a % b) }
    }
    a / gcd(a, b) * b
}

/// Property 18: for every generated set the Analyzer reports PASS, the
/// simulated response times never exceed the computed WCRTs: for every
/// Task when no Overrun was injected, and for the Tasks of Partitions in
/// which no Job overran when Overruns by stop-on-overrun Partitions were
/// injected (R18.7 protects Partitions from one another, not a Partition
/// from its own Overruns).
#[test]
fn r32_5_simulated_srp_execution_never_exceeds_the_computed_wcrt() {
    let mut rng = Rng(42);
    let mut passing = 0;
    let mut blocked_cases = 0;
    for _ in 0..400 {
        let set = random_sim_set(&mut rng);
        let a = analysed(&set);
        if inputs::utilization_exceeds_one(&a) {
            continue;
        }
        let w = response_times(&a);
        if !w.iter().all(|x| x.schedulable) {
            continue;
        }
        passing += 1;
        if a.iter().any(|t| t.b > 0) {
            blocked_cases += 1;
        }
        let hyper = set.tasks.iter().map(|t| t.period).fold(1, lcm).min(20_000);
        let horizon = 2 * hyper + set.tasks.iter().map(|t| t.period).max().unwrap();
        let (observed, overran) = simulate(&set, horizon, &mut rng, false);
        assert!(overran.is_empty());
        for (i, t) in set.tasks.iter().enumerate() {
            assert!(observed[i] <= by_id(&w, &t.id).r, "{}: observed {} > WCRT {} in\n{set:#?}\n{a:#?}", t.id, observed[i], by_id(&w, &t.id).r);
        }
        let (observed, overran) = simulate(&set, horizon, &mut rng, true);
        for (i, t) in set.tasks.iter().enumerate() {
            if overran.contains(&t.partition) {
                continue;
            }
            assert!(observed[i] <= by_id(&w, &t.id).r, "{} with Overruns in Partitions {overran:?}: observed {} > WCRT {} in\n{set:#?}\n{a:#?}", t.id, observed[i], by_id(&w, &t.id).r);
        }
    }
    assert!(passing >= 100 && blocked_cases >= 20, "{passing} passing sets, {blocked_cases} with blocking");
}

/// The simulator is not vacuous: a set whose analysis says FAIL does miss
/// its deadline in simulation.
#[test]
fn r32_5_the_simulator_observes_a_deadline_miss_the_analysis_predicts() {
    let set = SimSet {
        tasks: vec![
            SimTask { id: "hi".into(), priority: 2, period: 4, c: 2, cs: None, partition: 0, stops: false },
            SimTask { id: "lo".into(), priority: 1, period: 6, c: 4, cs: None, partition: 0, stops: false },
        ],
        ceilings: vec![],
        delta: 2,
    };
    let a = analysed(&set);
    let w = response_times(&a);
    assert!(!by_id(&w, "lo").schedulable);
    let mut rng = Rng(1);
    let (observed, _) = simulate(&set, 48, &mut rng, false);
    assert!(observed[1] > 6, "{observed:?}");
}

// ------------------------------------------------------------- R32.7

/// Published task sets (PAR-04 = 3), each Budget equal to the published
/// execution time, implicit deadlines, no blocking or overhead. The
/// published response times are reproduced exactly.
#[test]
fn r32_7_published_task_sets_are_reproduced() {
    // 1. Burns and Wellings, Real-Time Systems and Programming Languages,
    //    the response-time analysis example: a (T 50, C 12, lowest), b (T 40,
    //    C 10), c (T 30, C 10, highest): R = 10, 20, 52 (a misses 50).
    let bw = vec![task("c", 3, 10, 30, 30), task("b", 2, 10, 40, 40), task("a", 1, 12, 50, 50)];
    let r: Vec<(u64, bool)> = response_times(&bw).iter().map(|w| (w.r, w.schedulable)).collect();
    assert_eq!(r, vec![(10, true), (20, true), (52, false)]);
    // 2. Sha, Klein, and Goodenough, "Rate monotonic analysis for real-time
    //    systems" (1991), the three-task example also in the SEI
    //    Practitioner's Handbook: τ1 (C 20, T 100), τ2 (C 40, T 150),
    //    τ3 (C 100, T 350): completion times 20, 60, 240.
    let sei = vec![task("t1", 3, 20, 100, 100), task("t2", 2, 40, 150, 150), task("t3", 1, 100, 350, 350)];
    let r: Vec<(u64, bool)> = response_times(&sei).iter().map(|w| (w.r, w.schedulable)).collect();
    assert_eq!(r, vec![(20, true), (60, true), (240, true)]);
    // 3. The textbook harmonic set C = (1, 2, 3), T = (4, 6, 12) (Buttazzo,
    //    Hard Real-Time Computing Systems, response-time example; the
    //    attribution is to be confirmed at review, verification_plan.md
    //    5.10): R = 1, 3, 10.
    let harmonic = vec![task("t1", 3, 1, 4, 4), task("t2", 2, 2, 6, 6), task("t3", 1, 3, 12, 12)];
    let r: Vec<(u64, bool)> = response_times(&harmonic).iter().map(|w| (w.r, w.schedulable)).collect();
    assert_eq!(r, vec![(1, true), (3, true), (10, true)]);
    let _ = BTreeMap::<u8, u8>::new();
}
