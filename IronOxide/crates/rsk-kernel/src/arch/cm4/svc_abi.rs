//! The supervisor-call interface as both of its sides read it (2026-11-26).
//!
//! A Partition reaches the Kernel through `rsk`, whose operations are
//! wrappers of the supervisor-call stubs (`unsafe_module::stubs`): each
//! puts a request in R0 to R2, issues `svc #n`, and reads the reply from
//! R0. The Kernel's side is `entry::svc_core`, which is handed the number
//! and the caller's exception frame (R0 to R3 at words 0 to 3) and leaves
//! the reply in word 0. Until this file nothing but reading said that the
//! two sides agree on what a request word means.
//!
//! Here the meaning is stated once, as specification functions, and each
//! side is proven against them:
//!
//! - the request words `rsk` builds (`endpoint_arg`, `violation_word`,
//!   `rejection_word`, `dma_request_word`, `dma_write_word`) with the
//!   lemmas that the Kernel's decoding expressions read back from them
//!   what was put in (`lemma_endpoint_arg` and the like). `rsk` proves that
//!   its wrappers compute these words; the cores of `entry.rs` state their
//!   contracts over the same decoding expressions;
//! - what the Kernel's handler leaves in the frame for each service
//!   (`svc_served`), proven of `svc_core`: the reply word is the service's
//!   result under the service's own contract;
//! - `svc_ret`, the one fact a wrapper has of a supervisor call it made.
//!
//! Trusted still, and stated nowhere but here: that a supervisor call of a
//! Job (`svc_ret(n, r0, r1, r2, out)`) is one run of `svc_core` with that
//! number and a frame whose words 0 to 2 are `r0`, `r1`, `r2`, ending with
//! `out` in word 0. It rests on the exception entry and return (the
//! hardware model HW-06, HW-07; `Sched/StubExc.lean`), the SVC stub
//! (`Sched/StubPaths.lean`), the shell `arch::cm4::svc` that reads the
//! number from the instruction, and the stubs' inline assembly.
#![forbid(unsafe_code)]

use super::entry::*;
use crate::logic::log::{EV_API_MISUSE, EV_ENDPOINT_FULL, EV_PROTOCOL_VIOLATION};
use super::plan::{ArchState, EndpointCfg, MAX_CHANNELS, MAX_ENDPOINTS, MAX_ROLES, MAX_ROLE_SLOTS};
use super::HwModel;
use crate::arch::bound::KernelOps;
use vstd::prelude::*;

verus! {

/// A Job issued the supervisor call `number` with `r0`, `r1`, `r2` in R0
/// to R2 and resumed with `out` in R0. Uninterpreted: what the stubs'
/// trusted specifications say of each call, and nothing else says.
pub uninterp spec fn svc_ret(number: u8, r0: u32, r1: u32, r2: u32, out: u32) -> bool;

/// The clock call: the two reply words, R0 and R1.
pub uninterp spec fn svc_ret_now(lo: u32, hi: u32) -> bool;

// ------------------------------------------------------------ requests

/// The Endpoint services' R0 (task 21.7): the Endpoint, the role that
/// operates, and the role it sends to or receives from.
pub open spec fn endpoint_arg(id: u8, role: u8, peer: u8) -> u32 {
    id as u32 | ((role as u32 & 0xF) << 8) | ((peer as u32 & 0xF) << 12)
}

/// What `endpoint_args` reads from R0 (the expressions of its contract and
/// of `send_labels_ok`, `recv_labels_ok`, `spec_role_slot`).
pub open spec fn arg_endpoint(arg: u32) -> int {
    (arg & 0xFF) as int
}

pub open spec fn arg_role(arg: u32) -> int {
    ((arg >> 8) & 0xF) as int
}

pub open spec fn arg_peer(arg: u32) -> int {
    ((arg >> 12) & 0xF) as int
}

/// The Kernel reads back the Endpoint, role and peer of a request word,
/// and finds no bit it refuses the word's form for.
pub proof fn lemma_endpoint_arg(id: u8, role: u8, peer: u8)
    ensures
        arg_endpoint(endpoint_arg(id, role, peer)) == id as int,
        arg_role(endpoint_arg(id, role, peer)) == (role & 0xF) as int,
        arg_peer(endpoint_arg(id, role, peer)) == (peer & 0xF) as int,
        role < 16 ==> arg_role(endpoint_arg(id, role, peer)) == role as int,
        peer < 16 ==> arg_peer(endpoint_arg(id, role, peer)) == peer as int,
        endpoint_arg(id, role, peer) >> 16 == 0,
{
    let a = id as u32 | ((role as u32 & 0xF) << 8) | ((peer as u32 & 0xF) << 12);
    assert(a & 0xFF == id as u32 && (a >> 8) & 0xF == (role & 0xF) as u32 && (a >> 12) & 0xF == (peer & 0xF) as u32 && a >> 16 == 0) by (bit_vector)
        requires a == id as u32 | ((role as u32 & 0xF) << 8) | ((peer as u32 & 0xF) << 12);
    assert(role < 16 ==> role & 0xF == role) by (bit_vector);
    assert(peer < 16 ==> peer & 0xF == peer) by (bit_vector);
}

/// The other role of a two-role Endpoint.
pub open spec fn other_role(role: u8) -> u8 {
    (1 - (role & 1)) as u8
}

/// The report of a Protocol violation of `role` of Endpoint `id` (R41.2).
pub open spec fn violation_word(id: u8, role: u8) -> u32 {
    2u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16)
}

/// The report of a message `role` of Endpoint `id` received from `from`
/// and rejected (R42.4).
pub open spec fn rejection_word(id: u8, role: u8, from: u8) -> u32 {
    1u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16) | (((from as u32) & 0xF) << 20)
}

/// What `violation_core` reads from a report: its form in the low byte,
/// the Endpoint, the role, and for a rejection the sending role; and the
/// bits above them, which it requires to be zero.
pub open spec fn report_form(arg: u32) -> int {
    (arg & 0xFF) as int
}

pub open spec fn report_endpoint(arg: u32) -> int {
    ((arg >> 8) & 0xFF) as int
}

pub open spec fn report_role(arg: u32) -> int {
    ((arg >> 16) & 0xF) as int
}

pub open spec fn report_from(arg: u32) -> int {
    ((arg >> 20) & 0xF) as int
}

pub proof fn lemma_violation_word(id: u8, role: u8)
    ensures
        report_form(violation_word(id, role)) == 2,
        report_endpoint(violation_word(id, role)) == id as int,
        report_role(violation_word(id, role)) == (role & 0xF) as int,
        role < 16 ==> report_role(violation_word(id, role)) == role as int,
        violation_word(id, role) >> 20 == 0,
{
    let a = 2u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16);
    assert(a & 0xFF == 2 && (a >> 8) & 0xFF == id as u32 && (a >> 16) & 0xF == (role & 0xF) as u32 && a >> 20 == 0) by (bit_vector)
        requires a == 2u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16);
    assert(role < 16 ==> role & 0xF == role) by (bit_vector);
}

pub proof fn lemma_rejection_word(id: u8, role: u8, from: u8)
    ensures
        report_form(rejection_word(id, role, from)) == 1,
        report_endpoint(rejection_word(id, role, from)) == id as int,
        report_role(rejection_word(id, role, from)) == (role & 0xF) as int,
        report_from(rejection_word(id, role, from)) == (from & 0xF) as int,
        role < 16 ==> report_role(rejection_word(id, role, from)) == role as int,
        from < 16 ==> report_from(rejection_word(id, role, from)) == from as int,
        rejection_word(id, role, from) >> 24 == 0,
{
    let a = 1u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16) | (((from as u32) & 0xF) << 20);
    assert(a & 0xFF == 1 && (a >> 8) & 0xFF == id as u32 && (a >> 16) & 0xF == (role & 0xF) as u32 && (a >> 20) & 0xF == (from & 0xF) as u32 && a >> 24 == 0)
        by (bit_vector)
        requires a == 1u32 | ((id as u32) << 8) | (((role as u32) & 0xF) << 16) | (((from as u32) & 0xF) << 20);
    assert(role < 16 ==> role & 0xF == role) by (bit_vector);
    assert(from < 16 ==> from & 0xF == from) by (bit_vector);
}

/// The EasyDMA service's R0 for a transfer request: the channel.
pub open spec fn dma_request_word(i: u8) -> u32 {
    i as u32
}

/// And for a register write (DD-08, task 19.2): bit 31 and the channel.
pub open spec fn dma_write_word(i: u8) -> u32 {
    0x8000_0000u32 | i as u32
}

/// The handler's test for the register write, and the channel it reads.
pub open spec fn dma_is_write(r0: u32) -> bool {
    r0 & 0x8000_0000 != 0
}

pub open spec fn dma_write_channel(r0: u32) -> int {
    (r0 & 0xFF) as int
}

pub proof fn lemma_dma_words(i: u8)
    ensures
        !dma_is_write(dma_request_word(i)),
        dma_request_word(i) as int == i as int,
        dma_is_write(dma_write_word(i)),
        dma_write_channel(dma_write_word(i)) == i as int,
{
    let r = i as u32;
    assert(r & 0x8000_0000 == 0) by (bit_vector)
        requires r == i as u32;
    let w = 0x8000_0000u32 | i as u32;
    assert(w & 0x8000_0000 != 0 && w & 0xFF == i as u32) by (bit_vector)
        requires w == 0x8000_0000u32 | i as u32;
}

// ------------------------------------------------------------- replies

/// A send as its service performed it: the functional clauses of
/// `endpoint_send_core`'s contract, from the state the handler found
/// (`k0`, `s0`, `h0`) to the state the service left (`k1`, `s1`, `h1`),
/// before the Partition's fault response, if any, was applied.
pub open spec fn send_served<K: KernelOps, T: Tables>(k0: &K, k1: &K, tables: &T, s0: ArchState, s1: ArchState, h0: HwModel, h1: HwModel, arg: u32, ptr: u32, code: u32) -> bool {
    &&& code == ENDPOINT_OK ==> send_labels_ok(tables, s0, s1, h0, h1, arg)
    &&& code == ENDPOINT_OK ==> send_release(k0, k1, tables, arg)
    &&& code == ENDPOINT_OK ==> send_message_ok(tables, s0, s1, h0, h1, arg, ptr as int)
    &&& code == ENDPOINT_OK && storage_placed(k0, tables) ==> send_premise(tables, arg, ptr as int)
    &&& code != ENDPOINT_OK ==> h1.store == h0.store
    &&& code == ENDPOINT_FULL && k0.spec_depth() > 0 ==> k1.spec_last_event()
        == (EV_ENDPOINT_FULL as int, crate::arch::bound::top_task(k0), k0.spec_part()[crate::arch::bound::top_task(k0)])
    &&& code == ENDPOINT_PEER ==> s0.peer_changed[spec_role_slot(arg)] && !s1.peer_changed[spec_role_slot(arg)] && s1.fifos == s0.fifos && *k1 == *k0 && h1 == h0
    &&& code != ENDPOINT_PEER && code != ENDPOINT_REFUSED ==> 0 <= spec_role_slot(arg) < MAX_ROLE_SLOTS && !s0.peer_changed[spec_role_slot(arg)]
    &&& code != ENDPOINT_PEER ==> s1.peer_changed == s0.peer_changed
}

/// A receive as its service performed it: the functional clauses of
/// `endpoint_recv_core`'s contract; the first word delivered is the
/// channel's oldest label.
pub open spec fn recv_served<K: KernelOps, T: Tables>(k0: &K, k1: &K, tables: &T, s0: ArchState, s1: ArchState, h0: HwModel, h1: HwModel, arg: u32, ptr: u32, code: u32) -> bool {
    &&& code == ENDPOINT_OK ==> exists|first: int| recv_labels_ok(tables, s0, s1, h0, h1, arg, first)
    &&& code == ENDPOINT_OK ==> recv_message_ok(tables, s0, s1, h0, h1, arg, ptr as int)
    &&& code == ENDPOINT_OK && storage_placed(k0, tables) ==> recv_premise(tables, arg, ptr as int)
    &&& code != ENDPOINT_REFUSED ==> *k1 == *k0
    &&& code == ENDPOINT_PEER ==> s0.peer_changed[spec_role_slot(arg)] && !s1.peer_changed[spec_role_slot(arg)] && s1.fifos == s0.fifos && h1 == h0
    &&& code != ENDPOINT_PEER && code != ENDPOINT_REFUSED ==> 0 <= spec_role_slot(arg) < MAX_ROLE_SLOTS && !s0.peer_changed[spec_role_slot(arg)]
    &&& code != ENDPOINT_PEER ==> s1.peer_changed == s0.peer_changed
}

/// Endpoint `e` re-initialized (`endpoint_reinit_one`; R41.2, R42.4, task
/// 21.6): its channels are empty, its delivery latches clear, each of its
/// roles is marked to be told of the change, and no other Endpoint's
/// channel, latch or mark has changed.
pub open spec fn endpoint_restarted(s0: ArchState, s1: ArchState, e: int) -> bool {
    &&& forall|i: int| e * (MAX_ROLES * MAX_ROLES) as int <= i < (e + 1) * (MAX_ROLES * MAX_ROLES) as int ==> #[trigger] s1.fifos[i] == crate::logic::endpoint::Fifo::empty_spec()
    &&& forall|i: int| e * (MAX_ROLES * MAX_ROLES) as int <= i < (e + 1) * (MAX_ROLES * MAX_ROLES) as int ==> !(#[trigger] s1.delivered[i])
    &&& forall|i: int| e * MAX_ROLES as int <= i < (e + 1) * MAX_ROLES as int ==> #[trigger] s1.peer_changed[i]
    &&& forall|i: int| 0 <= i < MAX_CHANNELS && !(e * (MAX_ROLES * MAX_ROLES) as int <= i < (e + 1) * (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] s1.fifos[i] == s0.fifos[i]
    &&& forall|i: int| 0 <= i < MAX_CHANNELS && !(e * (MAX_ROLES * MAX_ROLES) as int <= i < (e + 1) * (MAX_ROLES * MAX_ROLES) as int) ==> #[trigger] s1.delivered[i] == s0.delivered[i]
    &&& forall|i: int| 0 <= i < MAX_ROLE_SLOTS && !(e * MAX_ROLES as int <= i < (e + 1) * MAX_ROLES as int) ==> #[trigger] s1.peer_changed[i] == s0.peer_changed[i]
}

/// A report word is a well-formed report of a Protocol violation (R41.2)
/// by the executing Job: form 2, no bit above the role, an Endpoint of the
/// table, and a role of it bound to the executing Job's Task.
pub open spec fn violation_report_ok<K: KernelOps, T: Tables>(k0: &K, tables: &T, arg: u32) -> bool {
    let e = report_endpoint(arg);
    let role = report_role(arg);
    let cfg = spec_endpoint_of(tables, e);
    &&& report_form(arg) == 2
    &&& arg >> 20 == 0
    &&& e < spec_endpoint_count_of(tables) && e < MAX_ENDPOINTS
    &&& role < MAX_ROLES && role < cfg.roles as int
    &&& cfg.tasks[role] as int == crate::arch::bound::top_task(k0)
}

/// A report word is a well-formed report of a rejected message (R42.4) by
/// the executing Job: form 1, no bit above the sending role, an Endpoint
/// of the table, two different roles of it, and the reporting role bound
/// to the executing Job's Task.
pub open spec fn rejection_report_ok<K: KernelOps, T: Tables>(k0: &K, tables: &T, arg: u32) -> bool {
    let e = report_endpoint(arg);
    let role = report_role(arg);
    let from = report_from(arg);
    let cfg = spec_endpoint_of(tables, e);
    &&& report_form(arg) == 1
    &&& arg >> 24 == 0
    &&& e < spec_endpoint_count_of(tables) && e < MAX_ENDPOINTS
    &&& role < MAX_ROLES && from < MAX_ROLES
    &&& role < cfg.roles as int && from < cfg.roles as int && from != role
    &&& cfg.tasks[role] as int == crate::arch::bound::top_task(k0)
}

/// The report service as `violation_core` performed it on the report word
/// `arg`, from the state found (`k0`, `s0`) to the state it left (`k1`,
/// `s1`), before `apply_core` (2026-11-26).
///
/// - With no Job executing nothing changes.
/// - A well-formed violation report (`violation_report_ok`): the event
///   `EV_PROTOCOL_VIOLATION` is the log's newest entry, against the
///   executing Job's Task and that Task's Partition, and the Endpoint is
///   re-initialized (`endpoint_restarted`).
/// - A well-formed rejection report (`rejection_report_ok`) the Kernel
///   permits (`spec_rejection_permitted`: a validated upward edge and a
///   delivery since the Endpoint's last re-initialization): the Endpoint
///   is re-initialized (the sending Partition's fault response is
///   `KernelOps::fault_of`'s, which has no statement here); one it does
///   not permit is API misuse of the caller and changes nothing of the
///   Endpoints.
/// - Any other word is API misuse of the caller: the fault
///   `EV_API_MISUSE` of the executing Job's Partition on the state found
///   (`KernelOps::spec_fault_of`), and nothing of the Endpoints changes.
pub open spec fn report_served<K: KernelOps, T: Tables>(k0: &K, k1: &K, tables: &T, s0: ArchState, s1: ArchState, arg: u32) -> bool {
    let t = crate::arch::bound::top_task(k0);
    let e = report_endpoint(arg);
    let cfg = spec_endpoint_of(tables, e);
    let permitted = spec_rejection_permitted(cfg, s0, e, report_role(arg), report_from(arg));
    &&& k0.spec_depth() == 0 ==> *k1 == *k0 && s1 == s0
    &&& k0.spec_depth() > 0 && violation_report_ok(k0, tables, arg) ==> k1.spec_last_event() == (EV_PROTOCOL_VIOLATION as int, t, k0.spec_part()[t])
            && endpoint_restarted(s0, s1, e)
    &&& k0.spec_depth() > 0 && rejection_report_ok(k0, tables, arg) && permitted ==> endpoint_restarted(s0, s1, e)
    &&& k0.spec_depth() > 0 && rejection_report_ok(k0, tables, arg) && !permitted ==> s1 == s0 && k1.spec_fault_of(k0, EV_API_MISUSE, false)
    &&& k0.spec_depth() > 0 && !violation_report_ok(k0, tables, arg) && !rejection_report_ok(k0, tables, arg)
            ==> s1 == s0 && k1.spec_fault_of(k0, EV_API_MISUSE, false)
}

/// The clock service's reply (2026-11-26): the two reply words are the low
/// and the high half of one value the time base's reading returned during
/// the call (`KernelArch::clock_reading`). What that value is, is the
/// time base's (DD-05, HW-12; `drivers.rs`, `Hw/Model.lean`).
pub open spec fn now_served<A: KernelArch>(hw: &A, lo: u32, hi: u32) -> bool {
    exists|t: u64| #[trigger] hw.clock_reading(t) && lo == t as u32 && hi == (t >> 32) as u32
}

/// The two halves the clock service replies with put together as `rsk`
/// does (`k_now`: `(hi << 32) | lo`) are the reading.
pub proof fn lemma_now_words(t: u64)
    ensures ((((t >> 32) as u32) as u64) << 32) | ((t as u32) as u64) == t,
{
    assert(((((t >> 32) as u32) as u64) << 32) | ((t as u32) as u64) == t) by (bit_vector);
}

/// What `svc_core` leaves in word 0 of the frame for a request (`r0`,
/// `r1`, `r2` the frame's words 0 to 2 as it found them), by service.
///
/// - Lock and unlock: 1 exactly when the scheduler grants the request on
///   the state found (`KernelOps::spec_lock_grants`, `spec_unlock_grants`;
///   the Resource is the low byte of R0), else 0.
/// - EasyDMA request: 0 exactly when the Kernel accepts it
///   (`dma_accepts`, R17.1, R17.2), else 1.
/// - EasyDMA register write: 0 only for a channel of the executing Job's
///   Partition and a register the Kernel does not mediate (R17.3), else 1.
/// - Raise: 0 exactly when the raise is taken as an event
///   (`raise_accepts`, R10.5, R10.7), else 1.
/// - Send and receive: the service's code, with the service's contract
///   from the state found to the state the service left.
/// - The report: 0, with the report service's contract from the state
///   found to the state the service left (`report_served`).
pub open spec fn svc_served<K: KernelOps, T: Tables>(k0: &K, tables: &T, s0: ArchState, h0: HwModel, number: u8, r0: u32, r1: u32, r2: u32, out: u32) -> bool {
    &&& number == SVC_LOCK || number == SVC_UNLOCK ==> out == 0 || out == 1
    &&& number == SVC_LOCK ==> (out == 1 <==> k0.spec_lock_grants(r0 as u8 as int))
    &&& number == SVC_UNLOCK ==> (out == 1 <==> k0.spec_unlock_grants(r0 as u8 as int))
    &&& number == SVC_DMA ==> out == 0 || out == 1
    &&& number == SVC_DMA && !dma_is_write(r0) ==> (out == 0 <==> dma_accepts(k0, tables, r0 as usize as int, r1 as int, r2 as int))
    &&& number == SVC_DMA && dma_is_write(r0) && out == 0 ==> dma_write_accepted(k0, tables, dma_write_channel(r0), r1)
    &&& number == SVC_RAISE ==> out == 0 || out == 1
    &&& number == SVC_RAISE ==> (out == 0 <==> raise_accepts(k0, tables, r0 as usize as int))
    &&& number == SVC_SEND ==> exists|k1: K, s1: ArchState, h1: HwModel| #[trigger] send_served(k0, &k1, tables, s0, s1, h0, h1, r0, r1, out)
    &&& number == SVC_RECV ==> exists|k1: K, s1: ArchState, h1: HwModel| #[trigger] recv_served(k0, &k1, tables, s0, s1, h0, h1, r0, r1, out)
    &&& number == SVC_VIOLATION ==> out == 0
    &&& number == SVC_VIOLATION ==> exists|k1: K, s1: ArchState| #[trigger] report_served(k0, &k1, tables, s0, s1, r0)
}

/// An accepted register write (R17.3): the channel is one of the executing
/// Job's Partition, the offset a word of its window, and the register none
/// of the pointer, count or list-mode registers of that window's channels.
pub open spec fn dma_write_accepted<K: KernelOps, T: Tables>(k0: &K, tables: &T, i: int, offset: u32) -> bool {
    &&& 0 <= i < spec_dma_count_of(tables)
    &&& k0.spec_depth() > 0
    &&& k0.spec_part()[k0.spec_stack(k0.spec_depth() - 1)] == spec_dma_of(tables, i).partition as int
    &&& offset < 0x1000 && offset % 4 == 0
    &&& dma_reg_unmediated(tables, i, dma_write_reg(tables, i, offset))
}

} // verus!
