//! The Health_Monitor event log: a fixed-capacity ring with a saturating
//! overflow counter (Property 11, event-log part; R14.6, R14.7).
//!
//! The ring refines a sequence of at most `NE` events, oldest first
//! (`view`): `push` appends, and when the ring is full it drops the oldest
//! entry and increments the overflow counter, saturating at `u32::MAX`.
//! Indices are computed with one conditional wrap instead of a modulo so
//! that the proofs stay linear.
#![forbid(unsafe_code)]

use vstd::prelude::*;

verus! {

/// Event kinds (R14.6).
pub const EV_PANIC: u8 = 1;
pub const EV_MEMMANAGE: u8 = 2;
pub const EV_BUSFAULT: u8 = 3;
pub const EV_USAGEFAULT: u8 = 4;
pub const EV_HARDFAULT: u8 = 5;
pub const EV_OVERRUN: u8 = 6;
pub const EV_DEADLINE_MISS: u8 = 7;
pub const EV_RELEASE_OVERLAP: u8 = 8;
pub const EV_MIT_VIOLATION: u8 = 9;
pub const EV_OPERATING_END: u8 = 10;
pub const EV_RESOURCE_RELEASED: u8 = 11;
pub const EV_PROTOCOL_VIOLATION: u8 = 12;
pub const EV_DMA_REJECTED: u8 = 13;
pub const EV_BOOT_CHECK: u8 = 14;
pub const EV_WATCHDOG_RESET: u8 = 15;
pub const EV_PARTITION_RESTART: u8 = 16;
pub const EV_PARTITION_STOP: u8 = 17;
pub const EV_SAFE_STATE: u8 = 18;
pub const EV_END_JOB: u8 = 19;
pub const EV_INIT_FAULT: u8 = 20;
pub const EV_UNBOUND_IRQ: u8 = 21;
pub const EV_API_MISUSE: u8 = 22;

/// A bit set of Profile restriction identifiers: bit n stands for PR-n
/// (R3.9, R14.6).
pub open spec fn pr_bit(n: int) -> u64 {
    (1u64 << (n as u64)) as u64
}

#[derive(Clone, Copy)]
pub struct Event {
    pub kind: u8,
    /// Task identifier, or 0xFF when none.
    pub task: u8,
    /// Partition identifier, or 0xFF for the Kernel.
    pub partition: u8,
    /// Bit set of PR identifiers concerned (`pr_bit`).
    pub prs: u64,
    pub instant: u64,
    pub detail: u32,
}

pub struct Log<const NE: usize> {
    pub entries: [Event; NE],
    /// Index of the oldest entry.
    pub head: usize,
    pub len: usize,
    /// Events overwritten because the ring was full (saturating, R14.7).
    pub overflow: u32,
    /// Total events ever pushed (saturating), the sequence number of the next event.
    pub seq: u32,
}

/// The ring index of logical position `i` (one wrap at most).
pub open spec fn ring_index<const NE: usize>(head: int, i: int) -> int {
    if head + i >= NE { head + i - NE } else { head + i }
}

impl<const NE: usize> Log<NE> {
    /// `NE` stays below 2^30 so that `head + len` fits a 32-bit `usize`.
    pub open spec fn wf(&self) -> bool {
        &&& 0 < NE < 0x4000_0000
        &&& self.head < NE
        &&& self.len <= NE
    }

    /// The logged events, oldest first.
    pub open spec fn view(&self) -> Seq<Event> {
        Seq::new(self.len as nat, |i: int| self.entries[ring_index::<NE>(self.head as int, i)])
    }

    pub fn new() -> (l: Self)
        requires 0 < NE < 0x4000_0000,
        ensures l.wf(), l.view() =~= Seq::<Event>::empty(), l.overflow == 0, l.seq == 0,
    {
        Log {
            entries: [Event { kind: 0, task: 0xFF, partition: 0xFF, prs: 0, instant: 0, detail: 0 }; NE],
            head: 0,
            len: 0,
            overflow: 0,
            seq: 0,
        }
    }

    /// R14.6, R14.7: records `e`; when full, the oldest entry is
    /// overwritten and the overflow counter saturates upward.
    pub fn push(&mut self, e: Event)
        requires old(self).wf(),
        ensures final(self).wf(),
            old(self).len < NE ==> final(self).view() =~= old(self).view().push(e) && final(self).overflow == old(self).overflow,
            old(self).len == NE ==> final(self).view() =~= old(self).view().drop_first().push(e)
                && final(self).overflow == (if old(self).overflow == u32::MAX { u32::MAX } else { (old(self).overflow + 1) as u32 }),
            final(self).seq == (if old(self).seq == u32::MAX { u32::MAX } else { (old(self).seq + 1) as u32 }),
    {
        self.seq = self.seq.saturating_add(1);
        if self.len < NE {
            let pos = if self.head + self.len >= NE { self.head + self.len - NE } else { self.head + self.len };
            self.entries[pos] = e;
            self.len = self.len + 1;
            proof {
                assert forall|i: int| 0 <= i < old(self).len implies self.view()[i] == old(self).view()[i] by {
                    assert(ring_index::<NE>(self.head as int, i) != pos as int);
                }
                assert(self.view()[old(self).len as int] == e);
                assert(self.view() =~= old(self).view().push(e));
            }
        } else {
            let pos = self.head;
            self.entries[pos] = e;
            self.head = if self.head + 1 >= NE { 0 } else { self.head + 1 };
            self.overflow = self.overflow.saturating_add(1);
            proof {
                let old_view = old(self).view();
                assert forall|i: int| 0 <= i < NE - 1 implies self.view()[i] == old_view[i + 1] by {
                    assert(ring_index::<NE>(self.head as int, i) == ring_index::<NE>(old(self).head as int, i + 1));
                    assert(ring_index::<NE>(self.head as int, i) != pos as int);
                }
                assert(ring_index::<NE>(self.head as int, NE - 1) == pos as int);
                assert(self.view()[NE - 1] == e);
                assert(self.view() =~= old_view.drop_first().push(e));
            }
        }
    }

    /// The i-th logged event, oldest first (for the HIL_Rig reader).
    pub fn get(&self, i: usize) -> (e: Event)
        requires self.wf(), i < self.len,
        ensures e == self.view()[i as int],
    {
        let pos = if self.head + i >= NE { self.head + i - NE } else { self.head + i };
        self.entries[pos]
    }
}

} // verus!
