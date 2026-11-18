//! R9.6, the finding of 2026-11-17 (Verification_Plan 7.3): the deadline
//! event of a Sporadic_Task's ended Job is disarmed when a deferred Release
//! of the Task is held at the Job's end.
//!
//! Until that day `Kernel::job_return` disarmed the ended Job's deadline
//! slot only when no Release of the Task was held. A held Release that is
//! pending has armed the slot for its own deadline (audit finding 5), which
//! is why the slot is kept then; a held Release that is deferred has not,
//! so the slot still held the ended Job's deadline, and the timer entry at
//! that instant logged a Deadline miss for a Job that had ended in time.
#![forbid(unsafe_code)]

use rsk_logic::logic::kernel::Kernel;
use rsk_logic::logic::log::EV_DEADLINE_MISS;
use rsk_logic::logic::sched::{JOB_NONE, REL_DEFERRED, REL_NONE, REL_PENDING};
use rsk_logic::logic::time::SLOT_DEADLINE;
use rsk_logic::logic::MIT_DEFER;
use rsk_loom::{small_config, NE, NL, SMALL_NP, SMALL_NR, SMALL_NS, SMALL_NT};

fn misses(k: &Kernel<SMALL_NT, SMALL_NR, SMALL_NP, NL, SMALL_NS, NE>) -> usize {
    (0..k.log.len).filter(|&i| k.log.get(i).kind == EV_DEADLINE_MISS).count()
}

#[test]
fn a_job_that_ended_in_time_is_not_reported_late_when_a_deferred_release_was_held_r9_6() {
    let cfg = small_config();
    assert!(cfg.validate());
    // Task 2: sporadic, MIT 100, deadline 100, deferral policy.
    let t = 2u8;
    assert_eq!((cfg.tasks[2].mit_policy, cfg.tasks[2].period, cfg.tasks[2].deadline), (MIT_DEFER, 100, 100));
    let sd = cfg.slot_deadline[2] as usize;
    let mut k = Kernel::<SMALL_NT, SMALL_NR, SMALL_NP, NL, SMALL_NS, NE>::new(&cfg);
    k.start(&cfg);
    // An event at instant 10 is accepted: the deadline event is armed for 110.
    k.intake(&cfg, t, 1, 10, t);
    assert_eq!(k.sched.release[2], REL_PENDING);
    assert!(k.slots.slots[sd].armed && k.slots.slots[sd].kind == SLOT_DEADLINE && k.slots.slots[sd].at == 110);
    let _ = k.settle(&cfg);
    k.dispatch(&cfg, t, 0);
    assert_eq!(k.sched.release[2], REL_NONE);
    // A second event at instant 20, inside the MIT, with the Job in progress: deferred.
    k.intake(&cfg, t, 1, 20, t);
    assert_eq!(k.sched.release[2], REL_DEFERRED);
    // The Job ends at instant 20, ninety ticks before its deadline.
    k.job_return(&cfg, 0);
    assert_eq!(k.sched.job[2], JOB_NONE);
    assert_eq!(misses(&k), 0);
    // The ended Job's deadline event is disarmed; the deferred Release is held.
    assert!(!k.slots.slots[sd].armed);
    assert_eq!(k.sched.release[2], REL_DEFERRED);
    // At instant 110 the timer entry makes the deferred Release and arms its
    // deadline event; no Deadline miss is logged for the Job that ended at 20.
    k.timer_event(&cfg, 110, 0);
    assert_eq!(misses(&k), 0);
    assert_eq!(k.sched.release[2], REL_PENDING);
    assert!(k.slots.slots[sd].armed && k.slots.slots[sd].kind == SLOT_DEADLINE && k.slots.slots[sd].at == 210);
}
