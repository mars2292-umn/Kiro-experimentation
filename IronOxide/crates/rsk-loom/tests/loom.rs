//! Loom exploration of the host model (R47.1, R47.2): every interleaving of
//! the time base, a Release_Source, and Thread mode over the small
//! configuration, each execution checked against the restated Verus
//! postconditions. Testing evidence only (R47.3, RN-37).

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc as StdArc;

use loom::sync::{Arc, Condvar, Mutex};
use loom::thread;
use rsk_loom::{small_config, small_scripts, Machine, SMALL_NP, SMALL_NR, SMALL_NS, SMALL_NT};

type Small = Machine<SMALL_NT, SMALL_NR, SMALL_NP, SMALL_NS>;

/// The shared machine with the count of asynchronous sources still active.
struct Shared {
    machine: Mutex<(Small, u32)>,
    wake: Condvar,
}

/// One execution of the model: the first compare match (instant 0, the
/// first Releases of Tasks 0 and 1) delivered before the threads start,
/// then `ticks - 1` further compare matches from the time base's thread,
/// `raises` intakes of Task 2 from the Release_Source's thread, and Thread
/// mode running every dispatched Job to its return. A source with nothing
/// to deliver has no thread, which keeps the explored space to the
/// interleavings that matter.
fn execution(ticks: u32, raises: u32) -> rsk_loom::Stats {
    assert!(ticks >= 1);
    let mut machine = Small::new(small_config(), small_scripts());
    machine.tick();
    let sources = u32::from(ticks > 1) + u32::from(raises > 0);
    let shared = Arc::new(Shared { machine: Mutex::new((machine, sources)), wake: Condvar::new() });
    let mut handles = Vec::new();
    if ticks > 1 {
        let shared = shared.clone();
        handles.push(thread::spawn(move || {
            for _ in 1..ticks {
                let mut g = shared.machine.lock().unwrap();
                g.0.tick();
                shared.wake.notify_all();
            }
            let mut g = shared.machine.lock().unwrap();
            g.1 -= 1;
            shared.wake.notify_all();
        }));
    }
    if raises > 0 {
        let shared = shared.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..raises {
                let mut g = shared.machine.lock().unwrap();
                g.0.raise(2, 1);
                shared.wake.notify_all();
            }
            let mut g = shared.machine.lock().unwrap();
            g.1 -= 1;
            shared.wake.notify_all();
        }));
    }
    {
        let shared = shared.clone();
        handles.push(thread::spawn(move || {
            // One Kernel entry per lock acquisition: between two steps of a
            // Job the sources' events can land.
            loop {
                let mut g = shared.machine.lock().unwrap();
                if g.0.busy() {
                    g.0.job_step();
                    continue;
                }
                if g.1 == 0 {
                    break;
                }
                let _woken = shared.wake.wait(g).unwrap();
            }
        }));
    }
    for h in handles {
        h.join().unwrap();
    }
    let g = shared.machine.lock().unwrap();
    assert!(!g.0.busy(), "a Job was left in progress: {}", g.0.describe());
    assert_eq!(g.0.kernel.sched.lock_len, 0, "a Resource was left held: {}", g.0.describe());
    g.0.stats
}

/// One Loom exploration: the sums over all executions of the counts that
/// show which behaviours the executions exercised.
fn explore(ticks: u32, raises: u32, bound: Option<usize>) -> (usize, rsk_loom::Stats) {
    let executions = StdArc::new(AtomicUsize::new(0));
    let sums = StdArc::new(std::sync::Mutex::new(rsk_loom::Stats::default()));
    let (e, su) = (executions.clone(), sums.clone());
    let mut builder = loom::model::Builder::new();
    builder.max_threads = 4;
    builder.preemption_bound = bound;
    let start = std::time::Instant::now();
    builder.check(move || {
        let stats = execution(ticks, raises);
        e.fetch_add(1, Ordering::Relaxed);
        assert_eq!(stats.ticks, ticks.min(stats.ticks), "{stats:?}");
        assert_eq!(stats.intakes, raises);
        assert_eq!(stats.dispatches, stats.returns + stats.kills, "{stats:?}");
        let mut g = su.lock().unwrap();
        g.ticks += stats.ticks;
        g.intakes += stats.intakes;
        g.dispatches += stats.dispatches;
        g.preemptions += stats.preemptions;
        g.stale += stats.stale;
        g.held_back += stats.held_back;
        g.locks += stats.locks;
        g.lock_refusals += stats.lock_refusals;
        g.unlocks += stats.unlocks;
        g.returns += stats.returns;
        g.kills += stats.kills;
    });
    let n = executions.load(Ordering::Relaxed);
    let sums = *sums.lock().unwrap();
    eprintln!("loom: ticks {ticks}, raises {raises}, preemption bound {bound:?}: {n} executions in {:?}; {sums:?}", start.elapsed());
    (n, sums)
}

/// R47.2: every interleaving (exhaustive, no preemption bound) of the time
/// base against Thread mode over the small configuration: two compare
/// matches (the first Releases of Tasks 0 and 1 at instant 0, then the
/// second period, which falls while a Job may hold Resource 0) and the
/// Jobs' steps satisfy the restated postconditions of dispatch, lock,
/// unlock, return, and timer event; among the executions a Release is held
/// back by the System_Ceiling (R8.3) and a Job is preempted (R7.1).
#[test]
fn r47_2_every_interleaving_of_the_time_base_and_thread_mode_satisfies_the_kernel_postconditions() {
    let (n, sums) = explore(2, 0, None);
    assert!(n >= 2, "Loom explored {n} execution(s)");
    assert!(sums.held_back > 0, "no execution held a Release back by the Ceiling (R8.3)");
    assert!(sums.preemptions > 0, "no execution preempted a Job (R7.1)");
}

/// R47.2: every interleaving (exhaustive) of a Release_Source against
/// Thread mode: one compare match releasing Tasks 0 and 1, and two intakes
/// of the Sporadic_Task 2 (Priority 7, above every Ceiling) falling
/// anywhere in the Jobs' steps, Critical_Sections included; the second
/// intake within the MIT is deferred or discarded (R10.2, R10.3) and never
/// duplicates a Release; a lock of a Resource the Task may not access is
/// refused (R8.7).
#[test]
fn r47_2_every_interleaving_of_a_release_source_and_thread_mode_satisfies_the_kernel_postconditions() {
    let (n, sums) = explore(1, 2, None);
    assert!(n >= 2, "Loom explored {n} execution(s)");
    assert!(sums.preemptions > 0, "no execution preempted a Job (R7.1)");
    assert!(sums.lock_refusals > 0, "no execution refused a lock (R8.7)");
}

/// R47.1, R47.2: the three sources together (two compare matches, two
/// intakes, Thread mode) under Loom's preemption bound of three: the
/// unbounded space of three threads and some twenty Kernel entries exceeds
/// the time limit the Verification_Plan records, so this run is the
/// bounded exploration and the two-thread runs above are the exhaustive ones.
#[test]
fn r47_1_the_three_sources_together_are_explored_under_a_preemption_bound_within_the_time_limit() {
    let start = std::time::Instant::now();
    let (n, sums) = explore(2, 2, Some(3));
    let elapsed = start.elapsed();
    assert!(n >= 2, "Loom explored {n} execution(s)");
    assert!(sums.preemptions > 0 && sums.held_back > 0 && sums.lock_refusals > 0, "{sums:?}");
    assert!(elapsed.as_secs() < 120, "Loom exploration took {elapsed:?}");
}
