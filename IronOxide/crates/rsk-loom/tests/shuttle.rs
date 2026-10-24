//! Shuttle exploration of the host model (R47.1): randomized schedules over
//! the larger configuration, whose Loom state space exceeds the time limit;
//! each execution checked against the restated Verus postconditions.
//! Randomized and not sound: testing evidence only (R47.3, RN-37).

#![forbid(unsafe_code)]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc as StdArc;

use rsk_loom::{large_config, large_scripts, Machine, LARGE_NP, LARGE_NR, LARGE_NS, LARGE_NT};
use shuttle::sync::{Arc, Condvar, Mutex};
use shuttle::thread;

type Large = Machine<LARGE_NT, LARGE_NR, LARGE_NP, LARGE_NS>;

struct Shared {
    machine: Mutex<(Large, u32)>,
    wake: Condvar,
}

/// One execution: `ticks` compare matches, `raises` intakes of each of the
/// two Sporadic_Tasks (2 and 4) from their own threads, Thread mode.
fn execution(ticks: u32, raises: u32) -> rsk_loom::Stats {
    let shared = Arc::new(Shared {
        machine: Mutex::new((Large::new(large_config(), large_scripts()), 3)),
        wake: Condvar::new(),
    });
    let mut handles = Vec::new();
    {
        let shared = shared.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..ticks {
                let mut g = shared.machine.lock().unwrap();
                g.0.tick();
                shared.wake.notify_all();
            }
            let mut g = shared.machine.lock().unwrap();
            g.1 -= 1;
            shared.wake.notify_all();
        }));
    }
    for (t, partition) in [(2u8, 1u8), (4u8, 1u8)] {
        let shared = shared.clone();
        handles.push(thread::spawn(move || {
            for _ in 0..raises {
                let mut g = shared.machine.lock().unwrap();
                g.0.raise(t, partition);
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

/// R47.1, R47.2: 20,000 random schedules of four compare matches and three
/// intakes of each Sporadic_Task over five Tasks and three Resources
/// satisfy the restated postconditions; the schedules include preemptions,
/// Releases held back by a Ceiling, and refused locks.
#[test]
fn r47_1_random_schedules_of_the_large_model_satisfy_the_kernel_postconditions() {
    let executions = StdArc::new(AtomicUsize::new(0));
    let preempted = StdArc::new(AtomicUsize::new(0));
    let held_back = StdArc::new(AtomicUsize::new(0));
    let refused = StdArc::new(AtomicUsize::new(0));
    let (e, p, h, r) = (executions.clone(), preempted.clone(), held_back.clone(), refused.clone());
    shuttle::check_random(
        move || {
            let stats = execution(4, 3);
            e.fetch_add(1, Ordering::Relaxed);
            p.fetch_add(stats.preemptions as usize, Ordering::Relaxed);
            h.fetch_add(stats.held_back as usize, Ordering::Relaxed);
            r.fetch_add(stats.lock_refusals as usize, Ordering::Relaxed);
            assert_eq!(stats.dispatches, stats.returns, "{stats:?}");
        },
        20_000,
    );
    eprintln!(
        "shuttle: {} executions; preemptions {}, held back {}, refusals {}",
        executions.load(Ordering::Relaxed),
        preempted.load(Ordering::Relaxed),
        held_back.load(Ordering::Relaxed),
        refused.load(Ordering::Relaxed)
    );
    assert_eq!(executions.load(Ordering::Relaxed), 20_000);
    assert!(preempted.load(Ordering::Relaxed) > 0, "no schedule preempted a Job (R7.1)");
    assert!(held_back.load(Ordering::Relaxed) > 0, "no schedule held a Release back (R8.3)");
    assert!(refused.load(Ordering::Relaxed) > 0, "no schedule refused a lock (R8.7)");
}
