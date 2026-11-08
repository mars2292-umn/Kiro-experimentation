//! rsk: the Application API (design.md "Application API"; task 5.6):
//! Resource access through `Shared::lock`, the monotonic time base
//! `now()`, and the debug console. Context types with one proxy per
//! declared Resource are generated per Task by the Generator (task 10.5);
//! until then the P1 demo constructs `Shared` values by hand.
//!
//! Every operation is a safe wrapper over the Kernel's SVC services. The
//! closure of `lock` receives `&mut T` and nothing that can wait (PR-08);
//! `lock` takes `&mut self`, so a Job cannot enter the same Resource twice
//! (R8.10); the reference cannot leave the closure (R8.6). A Task body sees
//! only its generated Context: one `Shared` proxy per declared Resource and
//! one `Signal` per Release_Signal it may raise (PR-06, R8.7), and the Job
//! wrapper requires it to return `()` (no `async` body, PR-09). The
//! compile-fail fixtures of `xtask/fixtures/api-misuse` check each rule.
#![no_std]
#![forbid(unsafe_code)]

/// The App_Declaration macro (design.md "App_Declaration and generation
/// pipeline"): validates the declaration and exports the `rsk_system!` and
/// `rsk_partition_<name>!` manifests (R21 to R24).
pub use rsk_macros::app;
/// The App_Prover's specification library (task 17.1), for Partition code
/// compiled under Creusot (`cfg(creusot)`): `#[cfg_attr(creusot,
/// rsk::creusot_std::macros::trusted)]` and the like. Absent from every
/// Flight_Build (the `creusot` feature is off).
#[cfg(feature = "creusot")]
pub use creusot_std;
#[cfg(feature = "creusot")]
#[allow(unused_imports)]
use creusot_std::{invariant::inv, std::ops::FnOnceExt};

/// The App_Prover's logical models (task 17.1): a `PerTask<T>` is viewed
/// as its current value, which `with`, `get`, and `set` relate exactly
/// (only this Task's Jobs touch it); a `Shared<T>` is viewed as the
/// Resource's invariant, a predicate on `T` (R46.4): what a Critical_Section
/// finds is any value satisfying it, and what it leaves must satisfy it.
/// Other Tasks change the value between Critical_Sections, so no stronger
/// model is sound for a Resource.
#[cfg(feature = "creusot")]
mod spec {
    use creusot_std::logic::Mapping;
    use creusot_std::prelude::*;

    impl<'a, T: 'static> View for super::PerTask<'a, T> {
        type ViewTy = T;
        #[logic(opaque)]
        fn view(self) -> T {
            dead
        }
    }

    // The gate's model is opaque (the field is private, so that no body
    // can forge an open gate); the three operations carry the trusted
    // contracts: closed at construction, open after `open`, read by `is_open`.
    impl View for super::Gate {
        type ViewTy = bool;
        #[logic(opaque)]
        fn view(self) -> bool {
            dead
        }
    }

    impl<'a, T: 'static> View for super::Shared<'a, T> {
        type ViewTy = Mapping<T, bool>;
        #[logic(opaque)]
        fn view(self) -> Mapping<T, bool> {
            dead
        }
    }
}
/// `#[rsk::bounds('label = N)]`: the Loop_Bound_Annotation of a function's
/// labelled loops (R35, PR-16).
pub use rsk_macros::bounds;
pub use rsk_kernel::arch::cm4::unsafe_module::ResourceCell;
#[cfg(not(feature = "quiet"))]
use rsk_kernel::arch::cm4::unsafe_module::hw;
use rsk_kernel::arch::cm4::unsafe_module::hw::{message_from_words, message_to_words, Words};
use rsk_kernel::arch::cm4::unsafe_module::stubs;

/// The lock operation of a Resource proxy (design.md "Application API",
/// the shape RTIC uses): a lexically scoped Critical_Section whose
/// `&mut T` cannot outlive the closure (R8.6).
pub trait Mutex {
    type T;
    fn lock<R>(&mut self, f: impl FnOnce(&mut Self::T) -> R) -> R;
}

/// The proxy of one declared Resource in a Task's Context (R8.7).
pub struct Shared<'a, T: 'static> {
    cell: &'a ResourceCell<T>,
    id: u8,
}

impl<'a, T: 'static> Shared<'a, T> {
    /// Constructed by generated code only (PR-06): the Resource identifier
    /// is the one of Generated_Config.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(cell: &'a ResourceCell<T>, id: u8) -> Shared<'a, T> {
        Shared { cell, id }
    }

    /// Enters a Critical_Section on the Resource (R8.1), runs `f`, and
    /// exits it (R8.2). A refused lock means the type-system guarantees
    /// were bypassed; the Kernel has logged the misuse, and the Job panics
    /// into its Partition's fault response (PR-19).
    // App_Prover contract (task 17.1, trusted: the body is the Kernel's SVC
    // pair, R8.1 to R8.3): the closure runs once on the Resource's value and
    // its result is returned; the closure must be ready for any value.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T> inv(v) && (*self)@[*v] ==> f.precondition((v,))))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T, r: R> inv(v) && (*self)@[*v] && f.postcondition_once((v,), r) ==> (*self)@[^v]))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures((^self)@ == (*self)@))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(exists<v: &mut T> inv(v) && (*self)@[*v] && (*self)@[^v] && f.postcondition_once((v,), result)))]
    pub fn lock<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        if !stubs::svc_lock(self.id) {
            panic!("rsk: lock refused");
        }
        let r = self.cell.with(f);
        let _ = stubs::svc_unlock(self.id);
        r
    }
}

impl<'a, T: 'static> Mutex for Shared<'a, T> {
    type T = T;
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    fn lock<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        Shared::lock(self, f)
    }
}

/// Per-Task state (PR-25): declared `state <task>.<name>: T = init;` in
/// the App_Declaration and held in the Task's Context. Only this Task's
/// Jobs touch the cell, and one Job of a Task is in progress at a time
/// (Property 2), so no lock is needed; the Generator emits the cell, so
/// Partition code declares no interior-mutable `static` (PR-05).
pub struct PerTask<'a, T: 'static> {
    cell: &'a ResourceCell<T>,
}

impl<'a, T: 'static> PerTask<'a, T> {
    /// Constructed by generated code only.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(cell: &'a ResourceCell<T>) -> PerTask<'a, T> {
        PerTask { cell }
    }

    /// Runs `f` on the state. App_Prover contract (task 17.1, trusted): the
    /// closure runs once on the state and its result is returned.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(forall<v: &mut T> inv(v) && *v == (*self)@ ==> f.precondition((v,))))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(exists<v: &mut T> inv(v) && *v == (*self)@ && ^v == (^self)@ && f.postcondition_once((v,), result)))]
    pub fn with<R>(&mut self, f: impl FnOnce(&mut T) -> R) -> R {
        self.cell.with_unprivileged(f)
    }

    /// A copy of the state.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result == self@))]
    pub fn get(&self) -> T
    where
        T: Copy,
    {
        self.cell.peek_unprivileged()
    }

    /// Replaces the state.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures((^self)@ == value))]
    pub fn set(&mut self, value: T) {
        self.with(|v| *v = value)
    }
}

/// A Release_Signal a Task may raise (R10.5), held in its Context (PR-06).
pub struct Signal {
    id: u8,
}

impl Signal {
    /// Constructed by generated code only: the identifier is the one of
    /// Generated_Config's signal table.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(id: u8) -> Signal {
        Signal { id }
    }

    /// Raises the Release_Signal: a Release_Source event of the bound
    /// Sporadic_Task at the current instant (R10.1 to R10.3), attributed to
    /// this Partition (R10.7). Never waits (PR-08). Returns whether the
    /// Kernel took the event (a refusal is API misuse and a fault of the
    /// Partition; the Kernel decides what the event does).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn raise(&self) -> bool {
        stubs::svc_raise(self.id)
    }
}

/// A message type an Endpoint may carry (R42.1): `Copy`, no references,
/// pointers, or interior mutability, every bit pattern a valid value. The
/// primitives and arrays of them implement it; `#[derive(rsk::Plain)]`
/// (task 21.4) extends it to `#[repr(C)]` structs of `Plain` fields.
pub use rsk_kernel::arch::cm4::unsafe_module::hw::Plain;
pub use rsk_macros::Plain;

/// Why a send did not take the message (R40.5, R42.3).
// No derived `PartialEq`: the App_Prover would need a `DeepModel` for it
// (task 17.1); callers match on the variant.
#[derive(Clone, Copy, Debug)]
pub enum SendError {
    /// The directed channel holds `capacity` messages: the Kernel has
    /// recorded the Protocol buffer-bound violation against this Partition.
    Full,
    /// The Kernel refused the request as API misuse (a fault of the
    /// Partition, whose response may have let the Job continue).
    Refused,
    /// R42.3 (task 21.4): the other role's Partition was stopped or
    /// restarted since this role's last operation; the Endpoint was
    /// re-initialized (both channels empty) and the message not sent. The
    /// next operation proceeds.
    PeerChanged,
}

/// Why a receive delivered no message although one may have been there
/// (task 21.4).
#[derive(Clone, Copy, Debug)]
pub enum RecvError {
    /// R42.3: the other role's Partition was stopped or restarted since
    /// this role's last operation; the Endpoint was re-initialized.
    PeerChanged,
    /// R42.4: the oldest message was taken from the channel and the role's
    /// validation function rejected it; the Kernel has recorded the fault
    /// of the sending Partition. Nothing is delivered for it.
    Rejected,
}

/// A receiver-side validation function (R42.4, task 21.4): the Generator
/// requires one for each role that receives messages from a Partition of
/// lower Criticality_Level and binds it through this trait, so that the
/// `Endpoint` delivers only messages the function accepts. The function is
/// Application code of the receiving Partition, proven total and
/// panic-free by the App_Prover with the Task bodies (level A and B).
pub trait Validate<T> {
    fn accept(m: &T) -> bool;
}

/// The validator of a role that declares none: every message is accepted.
pub struct Accept;

impl<T> Validate<T> for Accept {
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result == true))]
    fn accept(_m: &T) -> bool {
        true
    }
}

/// A Task's role of an Endpoint (task 21.2; DD-09, R38.1, R40.1), held in
/// its Context: `T` is the declared payload type and `W` the words of the
/// declared payload (`payload / 4`), so that `size_of::<T>() <= 4 * W`,
/// which the Generator asserts at compile time; `V` the role's validation
/// function (R42.4), `Accept` without one. A send copies the message into
/// the Kernel-owned channel from this role to the other and a receive
/// copies the oldest message of the channel from the other role to this
/// one; both complete without waiting (R40.4), inside the Kernel at its
/// level (R42.2). The typestate API of a Protocol is task 21.3's.
pub struct Endpoint<T: Plain, const W: usize, V: Validate<T> = Accept> {
    id: u8,
    role: u8,
    _payload: core::marker::PhantomData<(T, V)>,
}

impl<T: Plain, const W: usize, V: Validate<T>> Endpoint<T, W, V> {
    /// R38.1: the payload type fits the declared payload (checked at compile
    /// time for every instantiation; the Generator asserts the same on the
    /// declaration's payload bytes).
    const FITS: () = assert!(Words::<W>::fits::<T>(), "the Endpoint's payload type exceeds its declared payload or is more than 8-byte aligned (R38.1)");

    /// Constructed by generated code only (PR-06): the Endpoint index of
    /// Generated_Config and the Task's role (0 or 1).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn new(id: u8, role: u8) -> Endpoint<T, W, V> {
        Endpoint { id, role, _payload: core::marker::PhantomData }
    }

    /// The SVC argument (task 21.7): the Endpoint, this role, and the peer
    /// role the operation addresses.
    fn arg(&self, peer: u8) -> u32 {
        self.id as u32 | ((self.role as u32 & 0xF) << 8) | ((peer as u32 & 0xF) << 12)
    }

    /// The other role of a two-role Endpoint (the plain form).
    fn other(&self) -> u8 {
        1 - (self.role & 1)
    }

    /// Sends `m` to role `peer` (task 21.7: one channel per ordered pair of
    /// roles); the contract of `send` otherwise.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn send_to(&self, peer: u8, m: &T) -> Result<(), SendError> {
        let () = Self::FITS;
        let words = message_to_words::<T, W>(m);
        match stubs::svc_send(self.arg(peer), words.0.as_ptr() as usize as u32, (4 * W) as u32) {
            0 => Ok(()),
            1 => Err(SendError::Full),
            3 => Err(SendError::PeerChanged),
            _ => Err(SendError::Refused),
        }
    }

    /// Receives the oldest message role `peer` sent to this role (task
    /// 21.7); the contract of `recv` otherwise.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn recv_from(&self, peer: u8) -> Result<Option<T>, RecvError> {
        let () = Self::FITS;
        let mut words = Words::<W>::zero();
        match stubs::svc_recv(self.arg(peer), words.0.as_mut_ptr() as usize as u32, (4 * W) as u32) {
            0 => {
                let m = message_from_words::<T, W>(&words);
                if V::accept(&m) {
                    Ok(Some(m))
                } else {
                    report_rejected_message(self.id, self.role, peer);
                    Err(RecvError::Rejected)
                }
            }
            3 => Err(RecvError::PeerChanged),
            _ => Ok(None),
        }
    }

    /// Sends `m` to the other role: `Ok(())` when the Kernel copied it into
    /// the channel (and released the receiving Task when the arrival is its
    /// Release_Source, R40.3); `Err(SendError::Full)` when the channel holds
    /// `capacity` messages (R40.5); `Err(SendError::PeerChanged)` once after
    /// the other Partition's stop or restart (R42.3). Never waits (PR-08,
    /// R40.4).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn send(&self, m: &T) -> Result<(), SendError> {
        self.send_to(self.other(), m)
    }

    /// Receives the oldest message sent to this role: `Ok(Some(m))` when
    /// one was there and the validation function accepts it, `Ok(None)`
    /// when the channel is empty (R40.4), `Err(RecvError::Rejected)` when
    /// the function rejected it (R42.4: the message is consumed and the
    /// fault of the sending Partition reported), `Err(RecvError::
    /// PeerChanged)` once after the other Partition's stop or restart
    /// (R42.3). A refusal (API misuse, a fault of the Partition) reads as
    /// empty, the Kernel having recorded it.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn recv(&self) -> Result<Option<T>, RecvError> {
        self.recv_from(self.other())
    }
}

/// The word encoding of Protocol messages (task 21.3): the generated role
/// APIs put the label in the first word and the payload's words after it.
/// Trusted wrappers of the Kernel's message glue (UJ-058, UJ-059) for the
/// App_Prover, which translates the generated role modules (task 21.4).
pub mod words {
    pub use rsk_kernel::arch::cm4::unsafe_module::hw::Words;

    /// A zeroed word buffer.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn zero<const W: usize>() -> Words<W> {
        Words::<W>::zero()
    }

    /// The bits of `m` as words (the rest zero).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn message_to_words<T: super::Plain, const W: usize>(m: &T) -> Words<W> {
        rsk_kernel::arch::cm4::unsafe_module::hw::message_to_words::<T, W>(m)
    }

    /// The value whose bits are the first `size_of::<T>()` bytes of `words`
    /// (every bit pattern is a value: `Plain`).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn message_from_words<T: super::Plain, const W: usize>(words: &Words<W>) -> T {
        rsk_kernel::arch::cm4::unsafe_module::hw::message_from_words::<T, W>(words)
    }
}

/// Reports a Protocol violation to the Health_Monitor (task 21.3; R41.2):
/// called by the generated role APIs when a role state is dropped outside
/// an end or recursion state or a message carries a label the state does
/// not expect, and by the Job wrapper when a Job returns with its RoleSlot
/// empty. The Kernel records the event against this Task; the Job goes on.
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn report_protocol_violation() {
    stubs::svc_violation(0)
}

/// Reports a message that the role's validation function rejected (task
/// 21.4; R42.4): the Kernel records the fault against the sending Task and
/// applies the sending Partition's fault response. Called by `Endpoint::
/// recv` and the generated role APIs; `id` and `role` name the receiving
/// role (the Kernel refuses a report for a role the Task is not bound to).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn report_rejected_message(id: u8, role: u8, from: u8) {
    // Task 21.7: the sending role `from` names whose Partition is at fault.
    stubs::svc_violation(1 | (id as u32) << 8 | ((role as u32) & 0xF) << 16 | ((from as u32) & 0xF) << 20)
}

/// A Task's slot for the state of one Protocol role (task 21.3; R38.7,
/// PR-25): per-Task state the Generator emits, holding the role's typestate
/// between Jobs. A Job takes the state out, advances it through the typed
/// operations, and stores the result before returning; the generated Job
/// wrapper reports a Protocol violation when the slot is left empty
/// (R41.2). `S` is the generated `State` enum of the role, whose `Empty`
/// variant marks a taken slot.
/// Task 21.6 (R41.3, A-P8): the record, within one Job, of a completed
/// receive on a gating Endpoint. A role declared `after = E` takes
/// `&Gate` on every operation, with the App_Prover precondition that the
/// gate is open; only a receive outcome of `E`'s role (a value of its
/// generated `RecvS*` type, which implements `Received`) opens it, and
/// every Job starts with it closed (the generated Context constructs it
/// so, and the body's precondition states it). A body that operates on
/// the gated role before `E`'s receive fails its proof.
pub struct Gate {
    open: bool,
}

impl Gate {
    /// Closed: no receive has completed in this Job.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result@ == false))]
    pub const fn closed() -> Gate {
        Gate { open: false }
    }

    /// Opens the gate on the evidence of a completed receive.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures((^self)@ == true))]
    pub fn open<W: Received>(&mut self, _witness: &W) {
        self.open = true;
    }

    /// Whether a receive has completed in this Job.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result == (*self)@))]
    pub fn is_open(&self) -> bool {
        self.open
    }
}

/// A completed receive's outcome (the generated `RecvS*` enums).
pub trait Received {}

pub struct RoleSlot<'a, S: 'static> {
    cell: &'a ResourceCell<S>,
}

impl<'a, S: 'static> RoleSlot<'a, S> {
    /// Constructed by generated code only.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn new(cell: &'a ResourceCell<S>) -> RoleSlot<'a, S> {
        RoleSlot { cell }
    }

    /// Takes the state out, leaving `empty` in the slot.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn take(&self, empty: S) -> S {
        self.cell.with_unprivileged(|s| core::mem::replace(s, empty))
    }

    /// Stores the state for the Task's next Job.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn store(&self, state: S) {
        self.cell.with_unprivileged(|s| *s = state)
    }

    /// Whether the slot holds a state (`f` reads it).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn check(&self, f: impl FnOnce(&S) -> bool) -> bool {
        self.cell.with_unprivileged(|s| f(s))
    }
}

/// The Kernel time base in ticks (PR-26).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn now() -> u64 {
    stubs::svc_now()
}

/// Requests an EasyDMA transfer on `channel` (a channel index of the
/// calling Partition's peripherals, in the order of the summary) over
/// `buf` (DD-08, R17.1). The Kernel programs the peripheral's pointer and
/// count registers only when `buf` lies in the Partition's DMA-permitted
/// memory; otherwise the request is refused before any register is
/// written, the refusal is a spatial violation of the Partition (R17.2),
/// and `false` is returned (if the Partition's fault response lets the Job
/// continue).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn dma_request(channel: u8, buf: &[u8]) -> bool {
    let len = if buf.len() > u32::MAX as usize { u32::MAX } else { buf.len() as u32 };
    stubs::svc_dma(channel, buf.as_ptr() as usize as u32, len)
}

/// The Kernel-mediated write of one register of an EasyDMA-capable
/// peripheral (DD-08, R17.3; task 19.2): `offset` is the register's offset
/// in the 4 KiB window of the peripheral of `channel` (a channel index of
/// the calling Partition's peripherals, as for [`dma_request`]). The
/// Kernel writes it only when the channel is the Partition's own and the
/// register is none of the pointer, count, or list-mode registers of the
/// peripheral's channels (those are [`dma_request`]'s); otherwise the
/// request is refused before any register is written, the refusal is a
/// spatial violation of the Partition (R17.2), and `false` is returned.
/// A Partition starts and stops its transfers, configures the peripheral,
/// and clears its events this way, since its MPU view maps the window
/// read-only.
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn dma_write(channel: u8, offset: u32, value: u32) -> bool {
    stubs::svc_dma_write(channel, offset, value)
}

/// Register access to the calling Partition's peripherals (R16.2): a
/// read or write of a word at `addr`. The MPU view confines the access to
/// the Partition's peripheral windows; any other address is a MemManage
/// fault of the Partition (R16.4). EasyDMA peripherals are read-only
/// here; their transfers go through [`dma_request`] and their other
/// registers through [`dma_write`] (DD-08).
pub mod mmio {
    use rsk_kernel::arch::cm4::unsafe_module::hw;

    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn read(addr: u32) -> u32 {
        hw::peripheral_read(addr)
    }

    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn write(addr: u32, value: u32) {
        hw::peripheral_write(addr, value)
    }
}

/// Writes to the debug console (semihosting). Not for Flight_Builds
/// without a debugger (the Link_Checker reports it there).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn debug(s: &[u8]) {
    #[cfg(not(feature = "quiet"))]
    hw::semihosting_write(s);
    #[cfg(feature = "quiet")]
    let _ = s;
}

/// A bounded busy loop of `N` iterations, for the demo Tasks (PR-16: the
/// bound is the constant argument, which the Profile_Lint derives).
#[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
pub fn spin<const N: u32>() {
    let mut i = 0u32;
    while i < N {
        core::hint::black_box(i);
        i = i.wrapping_add(1);
    }
}

/// The HIL differential tests' Task-side probes (task 3.3, R49.3;
/// feature `hw-dt`, HIL builds only): the register patterns a Job keeps
/// across preemptions (HW-06, HW-10, R7.5), the FPSCR a Job starts with
/// (HW-10), an instruction fetch at an address (the isolation suite's
/// fetch attempts, HW-09), and a DMA request by address (the isolation
/// suite's refused buffers, R17.2). Each is inlined into the Partition's
/// code and runs unprivileged under its MPU view.
#[cfg(feature = "hw-dt")]
pub mod hw_dt {
    use rsk_kernel::arch::cm4::unsafe_module::dt;

    /// Bit i set for the i-th of R2 to R5, R8, R10, R12 that changed over
    /// `iterations` of a spin (0: every register preserved).
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn register_patterns(iterations: u32) -> u32 {
        dt::register_patterns(iterations)
    }

    /// As [`register_patterns`], with S0 to S31 (bits 8 to 23: S0 to S15;
    /// bits 24 to 27: how many of S16 to S31 changed). FPU Tasks only.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn register_patterns_fp(iterations: u32) -> u32 {
        dt::register_patterns_fp(iterations)
    }

    /// FPSCR as the Job finds it. FPU Tasks only.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fpscr() -> u32 {
        dt::fpscr_read()
    }

    /// An instruction fetch at `addr`: a call, which the MPU refuses
    /// (IACCVIOL) when the address is outside the Partition's view.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fetch(addr: u32) {
        dt::fetch(addr)
    }

    /// An EasyDMA request over `len` bytes at `ptr` (any address: the
    /// Kernel decides, R17.1, R17.2), as [`super::dma_request`] over a slice.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn dma_request_raw(channel: u8, ptr: u32, len: u32) -> bool {
        super::stubs::svc_dma(channel, ptr, len)
    }
}

/// The access matrix of the HIL isolation suite (task 7.4, R16.8;
/// feature `hw-dt`, HIL builds only): for the executing Partition, one
/// access attempt per protected class of every other Partition and of
/// the Kernel, enumerated from the generated memory map
/// (`rsk_target::layout`) in a fixed order the rig mirrors
/// (`rsk-hil::isolation`). Against each other Partition `q`, in order:
/// a read, a write, and a fetch of its code; a read and a write of its
/// data (above its stack); a write into its stack; a read and a write of
/// its first peripheral window (skipped when it owns none). Then against
/// the Kernel: a read and a write of its RAM, a read and a fetch of its
/// flash, a read and a write of the Private Peripheral Bus (the NVIC), a
/// read and a write of the first Kernel-owned peripheral's window after
/// the PPB (R16.2; skipped when the Target has none), and a write to the
/// word below this Partition's own stack (R16.5).
/// Every attempt the MPU refuses is a MemManage fault of the Partition
/// (R16.4), the PPB attempts a BusFault (unprivileged, HW-08, HW-11);
/// an attempt that returns is recorded by the caller as a mismatch.
/// Endpoint storage has no attempt: no Endpoint exists before Phase 4.
#[cfg(feature = "hw-dt")]
pub mod matrix {
    /// The memory map, from `rsk_target::layout`.
    pub struct Layout<'a> {
        pub this: usize,
        pub code: &'a [(u32, u32)],
        pub ram: &'a [(u32, u32)],
        pub stack: &'a [u32],
        pub windows: &'a [[(u32, u32); 5]],
        pub kernel_flash: (u32, u32),
        pub kernel_ram: (u32, u32),
        /// The Kernel's peripheral intervals, the PPB first (R16.2).
        pub kernel_windows: &'a [(u32, u32)],
    }

    /// Attempts per other Partition and against the Kernel.
    pub const PER_PARTITION: usize = 8;
    pub const KERNEL: usize = 9;
    /// The private peripheral bus address attempted (NVIC_ISER0).
    pub const PPB: u32 = 0xE000_E100;

    /// What an attempt did. No `PartialEq`: under the App_Prover a derived
    /// equality needs Creusot's `DeepModel` (the finding of 2026-10-24);
    /// callers match on the variant.
    #[derive(Clone, Copy)]
    pub enum Outcome {
        /// No address for the class (the Partition owns no peripheral).
        Skipped,
        /// The access returned: the MPU granted it (a mismatch).
        Survived,
    }

    /// A map with one entry per Partition in every table and `this` among
    /// them (App_Prover, task 17.1: what `target` and `perform` require).
    #[cfg(feature = "creusot")]
    #[allow(unused_imports)] // used by the logic function below under creusot-rustc only
    use creusot_std::{macros::{logic, pearlite}, model::View};

    #[cfg(feature = "creusot")]
    #[logic(open)]
    pub fn wf(l: &Layout<'_>) -> bool {
        pearlite! {
            l.code@.len() >= 1 && l.ram@.len() == l.code@.len() && l.stack@.len() == l.code@.len()
                && l.windows@.len() == l.code@.len() && l.this@ < l.code@.len()
        }
    }

    /// The number of attempts of this Partition.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result@ == (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn count(l: &Layout<'_>) -> usize {
        (l.code.len() - 1) * PER_PARTITION + KERNEL
    }

    /// The address and class of attempt `i`: `(address, class)` with the
    /// class 0 read, 1 write, 2 fetch; `None` when skipped.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(i@ < (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn target(l: &Layout<'_>, i: usize) -> Option<(u32, u8)> {
        let others = l.code.len() - 1;
        if i < others * PER_PARTITION {
            let mut q = i / PER_PARTITION;
            if q >= l.this {
                q += 1;
            }
            let k = i % PER_PARTITION;
            let code = l.code[q].0 + 0x100;
            let data = l.ram[q].0 + l.stack[q] + 0x100;
            let stack = l.ram[q].0 + 0x10;
            let window = l.windows[q][0];
            match k {
                0 => Some((code, 0)),
                1 => Some((code, 1)),
                2 => Some((code, 2)),
                3 => Some((data, 0)),
                4 => Some((data, 1)),
                5 => Some((stack, 1)),
                6 => if window.1 == 0 { None } else { Some((window.0, 0)) },
                _ => if window.1 == 0 { None } else { Some((window.0, 1)) },
            }
        } else {
            match i - others * PER_PARTITION {
                0 => Some((l.kernel_ram.0 + 0x100, 0)),
                1 => Some((l.kernel_ram.0 + 0x100, 1)),
                2 => Some((l.kernel_flash.0 + 0x100, 0)),
                3 => Some((l.kernel_flash.0 + 0x100, 2)),
                4 => Some((PPB, 0)),
                5 => Some((PPB, 1)),
                6 => l.kernel_windows.get(1).map(|w| (w.0 + 0x100, 0)),
                7 => l.kernel_windows.get(1).map(|w| (w.0 + 0x100, 1)),
                8 => Some((l.ram[l.this].0.wrapping_sub(4), 1)),
                _ => None,
            }
        }
    }

    /// Performs attempt `i`. Returns only when the access was skipped or
    /// granted: a refused access ends the Job (the Partition's fault
    /// response) and never returns here.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(i@ < (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    pub fn perform(l: &Layout<'_>, i: usize) -> Outcome {
        match target(l, i) {
            None => Outcome::Skipped,
            Some((a, 0)) => {
                let _ = super::mmio::read(a);
                Outcome::Survived
            }
            Some((a, 1)) => {
                super::mmio::write(a, 0);
                Outcome::Survived
            }
            Some((a, _)) => {
                super::hw_dt::fetch(a);
                Outcome::Survived
            }
        }
    }
}
