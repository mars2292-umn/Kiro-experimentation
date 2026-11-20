//! Ghost values for the entry cores (2026-11-17).
//!
//! A `Ghost(..)` value built in executable code compiles, in the build
//! without Verus, to a closure handed to `Ghost::assume_new_fallback`, which
//! the Lean extraction (Aeneas) rejects in the function that holds it. The
//! entry cores of `entry.rs` therefore build every ghost value through one
//! of the functions here (Verification_Plan 7.3). Each returns a zero-sized
//! value and is inlined: the Flight_Build's bytes are those of before.
//!
//! Each of the seven has two bodies since 2026-11-20. Under Verus
//! (`verus_keep_ghost`) it is the verified function. In every other build
//! `Ghost` is a zero-sized marker and the body constructs it directly
//! (`Ghost::assume_new`), with no closure, so that the Lean translation has
//! the function's body and the entry cores are proven to return there
//! without an assumption about these helpers.

#![forbid(unsafe_code)]

use vstd::prelude::*;

use super::plan::EndpointCfg;
use crate::arch::bound::KernelOps;

verus! {

/// The ghost copy of `*x`.
#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_snap<V>(x: &V) -> (g: Ghost<V>)
    ensures g@ == *x,
{
    Ghost(*x)
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_snap<V>(_x: &V) -> Ghost<V> {
    Ghost::assume_new()
}

/// The ghost integer `v`.
#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_int(v: u32) -> (g: Ghost<int>)
    ensures g@ == v as int,
{
    Ghost(v as int)
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_int(_v: u32) -> Ghost<int> {
    Ghost::assume_new()
}

/// The Partition of role `role` of an Endpoint, as a ghost integer.
#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_partition(cfg: &EndpointCfg, role: usize) -> (g: Ghost<int>)
    ensures g@ == cfg.partitions[role as int] as int,
{
    Ghost(cfg.partitions[role as int] as int)
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_partition(_cfg: &EndpointCfg, _role: usize) -> Ghost<int> {
    Ghost::assume_new()
}

#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_seq_empty() -> (g: Ghost<Seq<int>>)
    ensures g@ == Seq::<int>::empty(),
{
    Ghost(Seq::empty())
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_seq_empty() -> Ghost<Seq<int>> {
    Ghost::assume_new()
}

#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_seq_push_u8(c: &Ghost<Seq<int>>, v: u8) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.push(v as int),
{
    Ghost(c@.push(v as int))
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_seq_push_u8(_c: &Ghost<Seq<int>>, _v: u8) -> Ghost<Seq<int>> {
    Ghost::assume_new()
}

#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_seq_push_usize(c: &Ghost<Seq<int>>, v: usize) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.push(v as int),
{
    Ghost(c@.push(v as int))
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_seq_push_usize(_c: &Ghost<Seq<int>>, _v: usize) -> Ghost<Seq<int>> {
    Ghost::assume_new()
}

#[cfg(verus_keep_ghost)]
#[inline(always)]
pub fn ghost_seq_drop_last(c: &Ghost<Seq<int>>) -> (g: Ghost<Seq<int>>)
    ensures g@ == c@.drop_last(),
{
    Ghost(c@.drop_last())
}

#[cfg(not(verus_keep_ghost))]
#[inline(always)]
pub fn ghost_seq_drop_last(_c: &Ghost<Seq<int>>) -> Ghost<Seq<int>> {
    Ghost::assume_new()
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
