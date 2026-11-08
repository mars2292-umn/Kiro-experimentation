//! The Endpoint buffer model (Property 11's Endpoint part; R39.4, task
//! 21.2): each directed channel of an Endpoint is a ring of `cap` message
//! slots in Kernel-owned storage (DD-09), and this is its index state. The
//! Kernel executes every send and receive inside an SVC at the Kernel
//! level with PRIMASK held, so each operation is atomic with respect to
//! every Job and every other Kernel entry: the interleavings the ceiling
//! protocol permits are sequences of whole operations, and the refinement
//! is of each operation.
//!
//! `view(cap)` is the sequence of occupied slots, oldest first. `push`
//! appends the next free slot and `pop` removes the oldest, so a message
//! copied into the slot `push` returns is the one `pop` returns after
//! every message pushed before it: nothing is lost, duplicated, or
//! reordered, and the count never exceeds `cap`.

use vstd::prelude::*;
// The modular-arithmetic lemmas are named by path: ghost erasure removes
// them from the stable build.

verus! {

/// The index state of one directed channel: the oldest occupied slot and
/// the number of occupied slots. The capacity comes from Generated_Config
/// (`EndpointCfg::capacity`, 1 to 255).
#[derive(Clone, Copy)]
pub struct Fifo {
    pub head: u8,
    pub count: u8,
}

impl Fifo {
    pub open spec fn wf(&self, cap: int) -> bool {
        &&& 1 <= cap <= 255
        &&& (self.head as int) < cap
        &&& (self.count as int) <= cap
    }

    /// Property 11: the occupied slots, oldest first.
    pub open spec fn view(&self, cap: int) -> Seq<int> {
        Seq::new(self.count as nat, |i: int| (self.head as int + i) % cap)
    }

    pub open spec fn empty_spec() -> Fifo {
        Fifo { head: 0, count: 0 }
    }

    /// The executable well-formedness check (the state lives in
    /// Kernel_Arch's `ArchState`, which carries no proven invariant across
    /// entries; the cores check it before every operation and reset a
    /// state that fails it, which no Kernel path produces).
    pub fn is_wf(&self, cap: u8) -> (ok: bool)
        ensures ok == self.wf(cap as int),
    {
        cap >= 1 && self.head < cap && self.count <= cap
    }

    /// Takes the next free slot: `Some(slot)` with the slot appended to the
    /// view, or `None` with nothing changed when the channel is full
    /// (R40.5).
    pub fn push(&mut self, cap: u8) -> (slot: Option<u8>)
        requires old(self).wf(cap as int),
        ensures
            final(self).wf(cap as int),
            slot.is_some() ==> old(self).count < cap
                && slot.unwrap() as int == (old(self).head as int + old(self).count as int) % (cap as int)
                && final(self).view(cap as int) == old(self).view(cap as int).push(slot.unwrap() as int)
                && final(self).head == old(self).head && final(self).count as int == old(self).count as int + 1,
            slot.is_none() ==> old(self).count == cap && *final(self) == *old(self),
    {
        if self.count >= cap {
            return None;
        }
        let sum: u32 = self.head as u32 + self.count as u32;
        let s32: u32 = sum % (cap as u32);
        let s: u8 = s32 as u8;
        let ghost before = *self;
        self.count = self.count + 1;
        proof {
            assert(self.view(cap as int) =~= before.view(cap as int).push(s as int));
        }
        Some(s)
    }

    /// Takes the oldest occupied slot: `Some(slot)` with the slot removed
    /// from the front of the view, or `None` with nothing changed when the
    /// channel is empty (R40.4).
    pub fn pop(&mut self, cap: u8) -> (slot: Option<u8>)
        requires old(self).wf(cap as int),
        ensures
            final(self).wf(cap as int),
            slot.is_some() ==> old(self).count > 0 && slot.unwrap() == old(self).head
                && final(self).view(cap as int) == old(self).view(cap as int).drop_first()
                && final(self).count as int == old(self).count as int - 1,
            slot.is_none() ==> old(self).count == 0 && *final(self) == *old(self),
    {
        if self.count == 0 {
            return None;
        }
        let s = self.head;
        let ghost before = *self;
        let next: u32 = (self.head as u32 + 1) % (cap as u32);
        self.head = next as u8;
        self.count = self.count - 1;
        proof {
            let c = cap as int;
            let h = before.head as int;
            assert forall|i: int| 0 <= i < self.count as int implies
                #[trigger] self.view(c)[i] == before.view(c).drop_first()[i] by {
                // ((h + 1) % c + i) % c == (h + 1 + i) % c
                vstd::arithmetic::div_mod::lemma_small_mod(i as nat, c as nat);
                vstd::arithmetic::div_mod::lemma_add_mod_noop(h + 1, i, c);
            }
            assert(self.view(c) =~= before.view(c).drop_first());
        }
        Some(s)
    }

    /// Empties the channel (a Partition's restart or stop re-initializes
    /// the buffers its roles receive on, R14.10).
    pub fn clear(&mut self)
        ensures *final(self) == Fifo::empty_spec(),
    {
        self.head = 0;
        self.count = 0;
    }
}

} // verus!
