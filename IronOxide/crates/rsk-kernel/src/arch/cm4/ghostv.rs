//! Ghost values for the entry cores (2026-11-17).
//!
//! A `Ghost(..)` value built in executable code compiles, in the build
//! without Verus, to a closure handed to `Ghost::assume_new_fallback`, which
//! the Lean extraction (Aeneas) rejects in the function that holds it. The
//! entry cores of `entry.rs` therefore build every ghost value through one
//! of the functions here, which the extraction keeps opaque
//! (Verification_Plan 7.3). Each returns a zero-sized value and is inlined:
//! the Flight_Build's bytes are those of before.

#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::plan::EndpointCfg;
use crate::arch::bound::KernelOps;

verus! {

/// The ghost copy of `*x`.
#[inline(always)]
pub fn ghost_snap<V>(x: &V) -> (g: Ghost<V>)
    ensures g@ == *x,
{
    Ghost(*x)
}

/// The ghost integer `v`.
#[inline(always)]
pub fn ghost_int(v: u32) -> (g: Ghost<int>)
    ensures g@ == v as int,
{
    Ghost(v as int)
}

/// The Partition of role `role` of an Endpoint, as a ghost integer.
#[inline(always)]
pub fn ghost_partition(cfg: &EndpointCfg, role: usize) -> (g: Ghost<int>)
    ensures g@ == cfg.partitions[role as int] as int,
{
    Ghost(cfg.partitions[role as int] as int)
}

#[inline(always)]
pub fn ghost_seq_empty() -> (g: Ghost<Seq<int>>)
    ensures g@ == Seq::<int>::empty(),
{
    Ghost(Seq::empty())
}

#[inline(always)]
pub fn ghost_seq_push_u8(c: &Ghost<Seq<int>>, v: u8) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.push(v as int),
{
    Ghost(c@.push(v as int))
}

#[inline(always)]
pub fn ghost_seq_push_usize(c: &Ghost<Seq<int>>, v: usize) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.push(v as int),
{
    Ghost(c@.push(v as int))
}

#[inline(always)]
pub fn ghost_seq_drop_last(c: &Ghost<Seq<int>>) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.drop_last(),
{
    Ghost(c@.drop_last())
}

/// Whether no Job is in progress. Not a ghost value: a comparison at the
/// end of a short-circuit condition, which the extraction prints without
/// its type; as a function with a declared result it is translated as
/// written.
#[inline(always)]
pub fn no_job_in_progress<K: KernelOps>(k: &K) -> (r: bool)
    ensures r == (k.spec_depth() == 0),
{
    k.depth() == 0
}

} // verus!
