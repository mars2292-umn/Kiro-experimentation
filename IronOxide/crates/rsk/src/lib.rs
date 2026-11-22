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

    // An Endpoint's model is its (Endpoint index, role) pair, opaque (the
    // fields are private): `new` establishes it, the word-level operations'
    // contracts name it (A-P2, `channel`).
    impl<T: super::Plain, const W: usize, V: super::Validate<T>> View for super::Endpoint<T, W, V> {
        type ViewTy = (u8, u8);
        #[logic(opaque)]
        fn view(self) -> (u8, u8) {
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

/// The Kernel's answer to a send as this library's result (R42.3, R40.5;
/// 2026-11-22). The codes are the Kernel's own constants, not copies of
/// their values, and the reading is checked for every code by a
/// Kani_Harness (`answers_kani`): the peer-state error is returned for the
/// Kernel's peer-state report and for nothing else, `Ok` for an accepted
/// send only, `Full` for a full channel only.
#[inline]
fn send_result(code: u32) -> Result<(), SendError> {
    use rsk_kernel::arch::cm4::entry::{ENDPOINT_FULL, ENDPOINT_OK, ENDPOINT_PEER};
    if code == ENDPOINT_OK {
        Ok(())
    } else if code == ENDPOINT_FULL {
        Err(SendError::Full)
    } else if code == ENDPOINT_PEER {
        Err(SendError::PeerChanged)
    } else {
        Err(SendError::Refused)
    }
}

/// The Kernel's answer to a receive (R42.3, R40.4; 2026-11-22): a message
/// was copied out, the peer-state report, or nothing (an empty channel, or
/// a refusal the Kernel has recorded).
enum RecvAnswer {
    Message,
    PeerChanged,
    Nothing,
}

/// Reads the Kernel's code for a receive; checked for every code by the
/// same Kani_Harness module.
#[inline]
fn recv_answer(code: u32) -> RecvAnswer {
    use rsk_kernel::arch::cm4::entry::{ENDPOINT_OK, ENDPOINT_PEER};
    if code == ENDPOINT_OK {
        RecvAnswer::Message
    } else if code == ENDPOINT_PEER {
        RecvAnswer::PeerChanged
    } else {
        RecvAnswer::Nothing
    }
}

/// R42.3 (2026-11-22): the two readings above against the Kernel's result
/// codes, for every 32-bit code. The peer-state error is the Kernel's
/// peer-state report and only that; a message is delivered or a send
/// reported accepted for `ENDPOINT_OK` only; the four codes the Kernel
/// returns are told apart.
#[cfg(kani)]
mod answers_kani {
    use super::*;
    use rsk_kernel::arch::cm4::entry::{ENDPOINT_EMPTY, ENDPOINT_FULL, ENDPOINT_OK, ENDPOINT_PEER, ENDPOINT_REFUSED};

    #[kani::proof]
    fn the_send_result_is_the_kernels_answer_for_every_code() {
        let code: u32 = kani::any();
        let r = send_result(code);
        assert!(matches!(r, Ok(())) == (code == ENDPOINT_OK));
        assert!(matches!(r, Err(SendError::Full)) == (code == ENDPOINT_FULL));
        assert!(matches!(r, Err(SendError::PeerChanged)) == (code == ENDPOINT_PEER));
        assert!(matches!(r, Err(SendError::Refused)) == (code != ENDPOINT_OK && code != ENDPOINT_FULL && code != ENDPOINT_PEER));
        // The Kernel's refusal is read as a refusal.
        assert!(code != ENDPOINT_REFUSED || matches!(r, Err(SendError::Refused)));
    }

    #[kani::proof]
    fn the_receive_answer_is_the_kernels_answer_for_every_code() {
        let code: u32 = kani::any();
        let a = recv_answer(code);
        assert!(matches!(a, RecvAnswer::Message) == (code == ENDPOINT_OK));
        assert!(matches!(a, RecvAnswer::PeerChanged) == (code == ENDPOINT_PEER));
        assert!(matches!(a, RecvAnswer::Nothing) == (code != ENDPOINT_OK && code != ENDPOINT_PEER));
        // An empty channel and a refusal deliver nothing.
        assert!((code != ENDPOINT_EMPTY && code != ENDPOINT_REFUSED) || matches!(a, RecvAnswer::Nothing));
    }
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
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result@ == (id, role)))]
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
        send_result(stubs::svc_send(self.arg(peer), words.0.as_ptr() as usize as u32, (4 * W) as u32))
    }

    /// Receives the oldest message role `peer` sent to this role (task
    /// 21.7); the contract of `recv` otherwise.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn recv_from(&self, peer: u8) -> Result<Option<T>, RecvError> {
        let () = Self::FITS;
        let mut words = Words::<W>::zero();
        match recv_answer(stubs::svc_recv(self.arg(peer), words.0.as_mut_ptr() as usize as u32, (4 * W) as u32)) {
            RecvAnswer::Message => {
                let m = message_from_words::<T, W>(&words);
                if V::accept(&m) {
                    Ok(Some(m))
                } else {
                    report_rejected_message(self.id, self.role, peer);
                    Err(RecvError::Rejected)
                }
            }
            RecvAnswer::PeerChanged => Err(RecvError::PeerChanged),
            RecvAnswer::Nothing => Ok(None),
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

/// The channel facts the generated role modules' contracts speak of
/// (A-P2's message half, Verification_Plan 5.15, 2026-11-09): opaque
/// predicates that the two word-level operations below establish under
/// their trusted contracts and that each generated `send_<label>` and
/// `try_recv` then proves for its own label. The Kernel's side of the link
/// is `copy_in`'s model (the first word copied is the slot's label,
/// `docs/proof_assumptions.toml`): together, the label sequence of a channel
/// is the sequence of labels the role modules sent on it.
#[cfg(feature = "creusot")]
pub mod channel {
    use creusot_std::prelude::*;

    /// The Kernel's send on Endpoint `id`, from `role` to `peer`, accepted
    /// a message whose first word is `w0` (`copy_in` records it as the
    /// slot's label).
    #[logic(opaque)]
    pub fn sent(id: u8, role: u8, peer: u8, w0: u32) -> bool {
        dead
    }

    /// The Kernel's receive on Endpoint `id`, from `from` to `role`,
    /// delivered a message whose first word is `w0` (the channel's oldest
    /// label, `recv_labels_ok`).
    #[logic(opaque)]
    pub fn received(id: u8, from: u8, role: u8, w0: u32) -> bool {
        dead
    }

    /// The Kernel answered an operation of `role` on Endpoint `id` with
    /// the peer-state report (A-P6, R42.3; 2026-11-21): the role's flag
    /// was set and is cleared, and nothing was sent or received
    /// (`endpoint_send_core`'s and `endpoint_recv_core`'s contracts).
    #[logic(opaque)]
    pub fn peer_told(id: u8, role: u8) -> bool {
        dead
    }

    /// The Kernel answered an operation of `role` on Endpoint `id` with
    /// anything but the peer-state report. A fact of its own and not the
    /// negation of `peer_told`: both are established by the word-level
    /// operations below, one per answer, so a role operation can prove
    /// the one that matches the answer it was given and no other.
    #[logic(opaque)]
    pub fn not_told(id: u8, role: u8) -> bool {
        dead
    }
}

impl<const W: usize, V: Validate<[u32; W]>> Endpoint<[u32; W], W, V> {
    /// Sends the words `w` to role `peer`: the generated role modules' form
    /// of `send_to` (task 21.3: word 0 the label's index, the payload's
    /// words after it). Contract (A-P2, 2026-11-09): an accepted send
    /// handed the Kernel a message whose first word is `w[0]`, which the
    /// Kernel's `copy_in` records as the slot's label.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(match result { Ok(()) => channel::sent((*self)@.0, (*self)@.1, peer, (*w)@[0]), Err(_) => true }))]
    // A-P6 (2026-11-21): the peer-state error is the Kernel's report, and
    // only that.
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(match result { Err(SendError::PeerChanged) => channel::peer_told((*self)@.0, (*self)@.1), _ => channel::not_told((*self)@.0, (*self)@.1) }))]
    pub fn send_words(&self, peer: u8, w: &[u32; W]) -> Result<(), SendError> {
        self.send_to(peer, w)
    }

    /// Receives the oldest words role `peer` sent to this role: the
    /// generated role modules' form of `recv_from`. Contract (A-P2): a
    /// delivered message's first word is the one the Kernel's channel
    /// held as its oldest label.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(match result { Ok(Some(w)) => channel::received((*self)@.0, peer, (*self)@.1, w@[0]), _ => true }))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(match result { Err(RecvError::PeerChanged) => channel::peer_told((*self)@.0, (*self)@.1), _ => channel::not_told((*self)@.0, (*self)@.1) }))]
    pub fn recv_words(&self, peer: u8) -> Result<Option<[u32; W]>, RecvError> {
        self.recv_from(peer)
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
pub fn report_protocol_violation(id: u8, role: u8) {
    // The Endpoint and role (task 21.7, finding 14(f) of the review): the
    // Kernel records the violation and re-initializes the Endpoint.
    stubs::svc_violation(2 | (id as u32) << 8 | ((role as u32) & 0xF) << 16)
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

/// The SPIM registers the library itself drives to end a transfer (the
/// Kernel mediates EasyDMA for the SPIM class only in this version, DD-08;
/// offsets in the peripheral's 4 KiB window).
mod spim {
    pub const TASKS_STOP: u32 = 0x014;
    pub const EVENTS_STOPPED: u32 = 0x104;
    pub const EVENTS_END: u32 = 0x118;
    pub const SHORTS: u32 = 0x200;
    pub const ENABLE: u32 = 0x500;
    /// ENABLE's value for an enabled SPIM.
    pub const ENABLED: u32 = 7;
}

/// How many times [`DmaTransfer::end`] reads the peripheral's STOPPED
/// event before it gives up (PR-16: the loop's bound). A stop is answered
/// within the byte in flight, 64 µs at the slowest SPIM rate; the polls
/// take longer than that at 64 MHz.
#[cfg(not(kani))]
pub const DMA_STOP_POLLS: u32 = 1024;
/// Under Kani the wait is two reads: the harness decides what each read
/// returns, and a longer wait adds iterations and no behaviour.
#[cfg(kani)]
pub const DMA_STOP_POLLS: u32 = 2;

/// One EasyDMA channel of the calling Partition's peripherals: its index
/// in the system's DMA table and the base address of its peripheral. The
/// Generator emits a Partition's channels as constants
/// (`rsk_target::DMA`), from the same table the Kernel's is generated
/// from; Task code names a channel by one of those and supplies no index
/// or address of its own.
#[derive(Clone, Copy)]
pub struct DmaChannel {
    index: u8,
    window: u32,
}

impl DmaChannel {
    /// Constructed by generated code only.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn generated(index: u8, window: u32) -> DmaChannel {
        DmaChannel { index, window }
    }

    /// The channel's index, as [`dma_write`] takes it.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn index(&self) -> u8 {
        self.index
    }
}

/// A buffer a Task lends to EasyDMA transfers (R17.5, DD-08): `N` bytes
/// of the Partition's RAM that Task code reaches only while no transfer
/// has them. Declared as per-Task state
/// (`state <task>.<name>: rsk::DmaBuf<N> = rsk::DmaBuf::new();`).
///
/// The bytes are private. [`DmaBuf::bytes`] hands them out only while the
/// buffer is not lent; [`DmaBuf::lend`] asks the Kernel to program a
/// channel ([`DmaChannel`]) with them (R17.1, R17.2) and, when the Kernel accepts, marks
/// the buffer lent and returns the [`DmaTransfer`] that stands for the
/// transfer (kept across Jobs in a [`DmaHeld`]); [`DmaBuf::reclaim`] takes that value back, ends the transfer,
/// and only then makes the bytes reachable again. So no Job reads or
/// writes the buffer between a request and the end of its transfer.
#[repr(C)]
pub struct DmaBuf<const N: usize> {
    /// 1 while a transfer has the bytes. A byte and before the bytes: the
    /// peripheral writes upwards from the buffer's start, and what follows
    /// the last byte is the next field of whatever holds the buffer.
    lent: u8,
    bytes: [u8; N],
}

/// A transfer in progress over a lent [`DmaBuf`]: held by the Task (in its
/// per-Task state across Jobs) until it gives the transfer back with
/// [`DmaBuf::reclaim`]. Not `Copy`, not `Clone`. Dropping it stops the
/// transfer: no transfer runs on that no Task holds. The buffer of a
/// dropped transfer stays lent, and so unreachable, for the rest of the
/// Partition's incarnation (the bytes cannot be shown to be at rest
/// without the value that named their peripheral).
pub struct DmaTransfer {
    channel: u8,
    /// The peripheral's window, for the STOPPED event.
    window: u32,
    /// The lent buffer's address: `reclaim` accepts the buffer's own
    /// transfer only.
    buffer: u32,
    /// The stop has been requested (by `end`): the drop does not ask again.
    stop_requested: bool,
}

/// Where a Task keeps a [`DmaTransfer`] between Jobs: per-Task state
/// (`state <task>.<name>: rsk::DmaHeld = rsk::DmaHeld::none();`). A
/// Partition's statics start as zeroed memory, at reset and at every
/// restart of the Partition (R14.5), whatever their declared initial
/// value; the all-zero `DmaHeld` holds nothing, which an
/// `Option<DmaTransfer>` of zeroed memory does not.
#[repr(C)]
pub struct DmaHeld {
    /// 1 while a transfer is kept here.
    present: u8,
    channel: u8,
    stop_requested: bool,
    /// No padding: a Partition static's initial value is checked byte by
    /// byte at compile time (`ResourceCell::for_static`).
    _unused: u8,
    window: u32,
    buffer: u32,
}

impl DmaHeld {
    /// Nothing held: every byte zero.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn none() -> DmaHeld {
        DmaHeld { present: 0, channel: 0, stop_requested: false, _unused: 0, window: 0, buffer: 0 }
    }

    /// Whether a transfer is kept here.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn is_some(&self) -> bool {
        self.present != 0
    }

    /// Takes the transfer out.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn take(&mut self) -> Option<DmaTransfer> {
        if self.present == 0 {
            return None;
        }
        self.present = 0;
        Some(DmaTransfer { channel: self.channel, window: self.window, buffer: self.buffer, stop_requested: self.stop_requested })
    }

    /// Keeps `transfer` (or nothing). A transfer already kept here is
    /// dropped first, which stops it.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn put(&mut self, transfer: Option<DmaTransfer>) {
        let _ = self.take();
        if let Some(mut t) = transfer {
            self.channel = t.channel;
            self.window = t.window;
            self.buffer = t.buffer;
            self.stop_requested = t.stop_requested;
            self.present = 1;
            // Kept, not ended: its drop must not stop it.
            t.stop_requested = true;
        }
    }
}

/// Why [`DmaBuf::reclaim`] did not release the buffer.
pub enum DmaReclaimError {
    /// The transfer is another buffer's; it is handed back unchanged.
    NotThisBuffer(DmaTransfer),
    /// The peripheral did not report the stop within `DMA_STOP_POLLS`
    /// reads. The stop was requested and the buffer stays lent; the
    /// transfer is handed back so that a later Job can ask again.
    NotStopped(DmaTransfer),
}

impl<const N: usize> DmaBuf<N> {
    /// A zeroed buffer, not lent.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn new() -> DmaBuf<N> {
        DmaBuf { lent: 0, bytes: [0; N] }
    }

    /// The bytes, unless a transfer has them.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn bytes(&mut self) -> Option<&mut [u8; N]> {
        if self.lent != 0 {
            None
        } else {
            Some(&mut self.bytes)
        }
    }

    /// Whether a transfer has the bytes.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn is_lent(&self) -> bool {
        self.lent != 0
    }

    /// The address of the bytes (for a report; no access).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn address(&self) -> u32 {
        self.bytes.as_ptr() as usize as u32
    }

    /// Requests an EasyDMA transfer on `channel` (one of the calling
    /// Partition's, `rsk_target::DMA`) over this buffer. The Kernel programs the peripheral's pointer and
    /// count registers only when the buffer lies in the Partition's
    /// DMA-permitted memory and the channel is the Partition's own;
    /// otherwise the request is refused before any register is written,
    /// the refusal is a spatial violation of the Partition (R17.2), and
    /// `None` is returned (if the Partition's fault response lets the Job
    /// continue). `None` too, and nothing requested, while the buffer is
    /// lent.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn lend(&mut self, channel: DmaChannel) -> Option<DmaTransfer> {
        if self.lent != 0 {
            return None;
        }
        let buffer = self.address();
        let len = if N > u32::MAX as usize { u32::MAX } else { N as u32 };
        if stubs::svc_dma(channel.index, buffer, len) {
            self.lent = 1;
            Some(DmaTransfer { channel: channel.index, window: channel.window, buffer, stop_requested: false })
        } else {
            None
        }
    }

    /// Takes this buffer's transfer back: requests its stop and, once the
    /// peripheral reports it stopped, makes the bytes
    /// reachable again. A transfer that had completed loses nothing by
    /// the stop; one still running is cut short.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn reclaim(&mut self, transfer: DmaTransfer) -> Result<(), DmaReclaimError> {
        if self.lent == 0 || transfer.buffer != self.address() {
            return Err(DmaReclaimError::NotThisBuffer(transfer));
        }
        let mut transfer = transfer;
        transfer.request_stop();
        transfer.stop_requested = true;
        if transfer.stopped() {
            self.lent = 0;
            Ok(())
        } else {
            Err(DmaReclaimError::NotStopped(transfer))
        }
    }
}

/// A [`DmaBuf`] between two guards of eight bytes each, for showing what a
/// transfer wrote (Hardware_Model HW-15: an EasyDMA transfer reads and
/// writes `[PTR, PTR + MAXCNT)` only). Every field is of bytes, so nothing
/// lies between a guard and the buffer's bytes at its upper end.
#[repr(C)]
pub struct DmaGuarded<const N: usize> {
    lo: [u8; 8],
    pub buf: DmaBuf<N>,
    hi: [u8; 8],
}

impl<const N: usize> DmaGuarded<N> {
    const GUARD: u8 = 0x3C;
    // A Partition's statics start zeroed whatever their initial value
    // (R14.5), so the guards are written by `arm`, not by `new`.
    /// [`DmaGuarded::footprint`]: both guards as written, the buffer written.
    pub const WRITTEN_INSIDE: u32 = 1;
    /// Both guards as written, no byte of the buffer changed.
    pub const UNWRITTEN: u32 = 2;
    /// The buffer is lent: its transfer was not ended.
    pub const NOT_STOPPED: u32 = 3;
    /// A guard byte changed.
    pub const GUARD_WRITTEN: u32 = 4;

    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub const fn new() -> DmaGuarded<N> {
        DmaGuarded { lo: [0; 8], buf: DmaBuf::new(), hi: [0; 8] }
    }

    /// Writes the guards and sets every byte of the buffer to `fill`,
    /// before the buffer is lent (nothing while it is lent).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn arm(&mut self, fill: u8) {
        if let Some(bytes) = self.buf.bytes() {
            *bytes = [fill; N];
            self.lo = [Self::GUARD; 8];
            self.hi = [Self::GUARD; 8];
        }
    }

    /// After transfers into the buffer, with the buffer taken back: one
    /// of the four verdicts above, `fill` being what [`DmaGuarded::arm`]
    /// wrote before the buffer was lent. Bounded by the sizes (PR-16).
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn footprint(&mut self, fill: u8) -> u32 {
        // Which guard bytes changed (bits 8 to 15: below the buffer; bits
        // 16 to 23: above it) and the last changed byte's value (bits 24 to
        // 31), beside the verdict in the low byte.
        let mut changed = 0u32;
        let mut value = 0u32;
        let mut i = 0;
        while i < 8 {
            if self.lo[i] != Self::GUARD {
                changed |= 1 << (8 + i);
                value = self.lo[i] as u32;
            }
            if self.hi[i] != Self::GUARD {
                changed |= 1 << (16 + i);
                value = self.hi[i] as u32;
            }
            i += 1;
        }
        if changed != 0 {
            return Self::GUARD_WRITTEN | changed | value << 24;
        }
        match self.buf.bytes() {
            Some(bytes) => {
                let mut j = 0;
                while j < N {
                    if bytes[j] != fill {
                        return Self::WRITTEN_INSIDE;
                    }
                    j += 1;
                }
                Self::UNWRITTEN
            }
            None => Self::NOT_STOPPED,
        }
    }
}

impl DmaTransfer {
    /// The channel the transfer was requested on.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn channel(&self) -> u8 {
        self.channel
    }

    /// Requests the stop: the shortcuts off first (an END_START shortcut
    /// would start the next transfer at this one's end), the events of
    /// this stop cleared, then the STOP task. Through the Kernel
    /// ([`dma_write`]), which refuses a channel that is not the
    /// Partition's.
    fn request_stop(&self) {
        let _ = stubs::svc_dma_write(self.channel, spim::SHORTS, 0);
        let _ = stubs::svc_dma_write(self.channel, spim::EVENTS_STOPPED, 0);
        let _ = stubs::svc_dma_write(self.channel, spim::EVENTS_END, 0);
        let _ = stubs::svc_dma_write(self.channel, spim::TASKS_STOP, 1);
    }

    /// Whether the peripheral reports that it has stopped, read at most
    /// `DMA_STOP_POLLS` times. A peripheral that is not enabled runs no
    /// transfer (its registers were programmed and nothing started), and
    /// counts as stopped at once.
    fn stopped(&self) -> bool {
        let event = self.window.wrapping_add(spim::EVENTS_STOPPED);
        let enabled = mmio::read(self.window.wrapping_add(spim::ENABLE)) == spim::ENABLED;
        let mut i = 0u32;
        let mut stopped = !enabled;
        while i < DMA_STOP_POLLS && !stopped {
            stopped = mmio::read(event) != 0;
            i = i.wrapping_add(1);
        }
        stopped
    }

    /// Ends the transfer without its buffer at hand: requests the stop and
    /// waits as [`DmaBuf::reclaim`] does. Returns whether the peripheral
    /// reported that it stopped. The buffer stays lent either way.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn end(mut self) -> bool {
        self.request_stop();
        self.stop_requested = true;
        self.stopped()
    }
}

impl Drop for DmaTransfer {
    /// A transfer no Task holds any more is stopped.
    fn drop(&mut self) {
        if !self.stop_requested {
            self.request_stop();
        }
    }
}

/// The Kernel-mediated write of one register of an EasyDMA-capable
/// peripheral (DD-08, R17.3; task 19.2): `offset` is the register's offset
/// in the 4 KiB window of the peripheral of `channel` (a channel index of
/// the calling Partition's peripherals, as for [`DmaBuf::lend`]). The
/// Kernel writes it only when the channel is the Partition's own and the
/// register is none of the pointer, count, or list-mode registers of the
/// peripheral's channels (those are [`DmaBuf::lend`]'s); otherwise the
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
/// here; their transfers go through [`DmaBuf::lend`] and their other
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

    /// R11.8: sets this Job's rounding mode to round toward zero and says
    /// whether FPSCR then reads so; the Job leaves settings other than the
    /// defaults behind. FPU Tasks only; the last FP use of the Job.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fpscr_set_round_toward_zero() -> bool {
        dt::fpscr_set_round_toward_zero()
    }

    /// An instruction fetch at `addr`: a call, which the MPU refuses
    /// (IACCVIOL) when the address is outside the Partition's view.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn fetch(addr: u32) {
        dt::fetch(addr)
    }

    /// An EasyDMA request over `len` bytes at `ptr` (any address: the
    /// Kernel decides, R17.1, R17.2): the isolation suite's refused buffers,
    /// which no [`super::DmaBuf`] can name.
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn dma_request_raw(channel: u8, ptr: u32, len: u32) -> bool {
        super::stubs::svc_dma(channel, ptr, len)
    }

    /// The stack-check probe (R16.5; the audit's findings 8 and 9): moves
    /// the stack pointer to `sp` and issues an SVC. The Kernel answers with
    /// a stack violation of the Partition; the Job does not continue.
    #[inline(always)]
    #[cfg_attr(feature = "creusot", creusot_std::macros::trusted)]
    pub fn svc_with_sp(sp: u32) {
        dt::svc_with_sp(sp)
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
/// word below this Partition's own stack (R16.5), and a write to this
/// Partition's own first read-only window at an EasyDMA pointer register
/// (DD-08, R17.3; skipped when it owns none; audit finding 1, 2026-11-13).
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
        /// Each RAM region's subregion-disable bits (`rsk_target::layout::RAM_SRD`).
        pub ram_srd: &'a [u8],
        pub stack: &'a [u32],
        pub windows: &'a [[(u32, u32); 5]],
        /// Whether each window is read-only to its Partition (DD-08; `rsk_target::layout::WINDOWS_RO`).
        pub windows_ro: &'a [[bool; 5]],
        pub kernel_flash: (u32, u32),
        pub kernel_ram: (u32, u32),
        /// The Kernel's peripheral intervals, the PPB first (R16.2).
        pub kernel_windows: &'a [(u32, u32)],
    }

    /// Attempts per other Partition and against the Kernel.
    pub const PER_PARTITION: usize = 8;
    pub const KERNEL: usize = 11;
    /// The register offset of the own-read-only-window attempt: SPIM's RXD.PTR
    /// (nRF52840 PS 6.26), the EasyDMA pointer a Partition must not write itself
    /// (DD-08, R17.3; audit finding 1, 2026-11-13).
    pub const OWN_RO_OFFSET: u32 = 0x534;
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
            l.code@.len() >= 1 && l.ram@.len() == l.code@.len() && l.ram_srd@.len() == l.code@.len() && l.stack@.len() == l.code@.len()
                && l.windows@.len() == l.code@.len() && l.windows_ro@.len() == l.code@.len() && l.this@ < l.code@.len()
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
                // A write to this Partition's own first read-only window (skipped when it owns none).
                9 => {
                    let w = &l.windows[l.this];
                    let r = &l.windows_ro[l.this];
                    if r[0] && w[0].1 != 0 {
                        Some((w[0].0.wrapping_add(OWN_RO_OFFSET), 1))
                    } else if r[1] && w[1].1 != 0 {
                        Some((w[1].0.wrapping_add(OWN_RO_OFFSET), 1))
                    } else if r[2] && w[2].1 != 0 {
                        Some((w[2].0.wrapping_add(OWN_RO_OFFSET), 1))
                    } else if r[3] && w[3].1 != 0 {
                        Some((w[3].0.wrapping_add(OWN_RO_OFFSET), 1))
                    } else if r[4] && w[4].1 != 0 {
                        Some((w[4].0.wrapping_add(OWN_RO_OFFSET), 1))
                    } else {
                        None
                    }
                }
                // HW-09: a write into the first disabled subregion of this
                // Partition's own RAM region (skipped when the region has
                // none). The region matches the address and its subregion
                // is disabled, so the access is outside the view.
                10 => {
                    let srd = l.ram_srd[l.this];
                    if srd == 0 {
                        None
                    } else {
                        Some((l.ram[l.this].0.wrapping_add((l.ram[l.this].1 / 8).wrapping_mul(first_disabled(srd))).wrapping_add(0x10), 1))
                    }
                }
                _ => None,
            }
        }
    }

    /// The index of the lowest set bit of a non-zero subregion-disable byte
    /// (the first disabled eighth of the region); 8 for zero.
    #[cfg_attr(feature = "creusot", creusot_std::macros::ensures(result@ <= 8))]
    pub fn first_disabled(srd: u8) -> u32 {
        if srd & 0x01 != 0 { 0 }
        else if srd & 0x02 != 0 { 1 }
        else if srd & 0x04 != 0 { 2 }
        else if srd & 0x08 != 0 { 3 }
        else if srd & 0x10 != 0 { 4 }
        else if srd & 0x20 != 0 { 5 }
        else if srd & 0x40 != 0 { 6 }
        else if srd & 0x80 != 0 { 7 }
        else { 8 }
    }

    /// Performs attempt `i`. Returns only when the access was skipped or
    /// granted: a refused access ends the Job (the Partition's fault
    /// response) and never returns here.
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(wf(l)))]
    #[cfg_attr(feature = "creusot", creusot_std::macros::requires(i@ < (l.code@.len() - 1) * PER_PARTITION@ + KERNEL@))]
    // Never inlined: the Link_Checker's fetch-probe declaration names this
    // function (`rsk_link::ISOLATION_FETCH_PROBE`), whose `hw_dt::fetch` is
    // the one indirect call of the isolation configuration's Partition code.
    #[inline(never)]
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

/// R17.5 by bounded model checking (2026-11-16): for every sequence of four
/// operations on one buffer (reading its bytes, lending it, keeping or
/// dropping the transfer, taking the transfer back), with the peripheral's
/// ENABLE register and STOPPED event set to any values before each one,
/// the bytes are handed out exactly when no accepted transfer of the buffer
/// is outstanding, and a transfer stops being outstanding only by a
/// `reclaim` that found the peripheral not enabled or reporting STOPPED.
/// The supervisor calls and the registers are the host doubles
/// (`docs/kani_stubs.toml`): a request is always accepted.
#[cfg(kani)]
mod dma_kani {
    use super::*;

    const WINDOW: u32 = 0x4000_3000;

    #[kani::proof]
    #[kani::unwind(12)]
    fn bytes_are_out_of_reach_while_a_transfer_is_outstanding() {
        let channel = DmaChannel::generated(0, WINDOW);
        let mut buf: DmaBuf<2> = DmaBuf::new();
        let mut held = DmaHeld::none();
        // Ghost: an accepted transfer of `buf` has not been taken back.
        let mut outstanding = false;
        let mut step = 0;
        while step < 4 {
            let enabled: bool = kani::any();
            let stopped: bool = kani::any();
            mmio::write(WINDOW + spim::ENABLE, if enabled { spim::ENABLED } else { 0 });
            mmio::write(WINDOW + spim::EVENTS_STOPPED, stopped as u32);
            let op: u8 = kani::any();
            kani::assume(op < 4);
            if op == 0 {
                assert!(buf.bytes().is_some() == !outstanding);
            } else if op == 1 {
                let t = buf.lend(channel);
                assert!(t.is_some() == !outstanding);
                if t.is_some() {
                    outstanding = true;
                }
                // Kept for a later step, or dropped here (the buffer then
                // stays lent).
                if kani::any() {
                    held.put(t);
                }
            } else if op == 2 {
                if let Some(t) = held.take() {
                    match buf.reclaim(t) {
                        Ok(()) => {
                            assert!(outstanding && (!enabled || stopped));
                            outstanding = false;
                        }
                        Err(DmaReclaimError::NotStopped(t)) => {
                            assert!(enabled && !stopped);
                            held.put(Some(t));
                        }
                        Err(DmaReclaimError::NotThisBuffer(_)) => assert!(false),
                    }
                }
            } else {
                // A transfer dropped out of the holder.
                held.put(None);
            }
            assert!(buf.is_lent() == outstanding);
            step += 1;
        }
    }

    /// R17.5 for sequences of any length (2026-11-21), by induction on the
    /// operations where the harness above checks every sequence of four.
    /// The invariant: the buffer's lent flag is set exactly while an
    /// accepted transfer of it is outstanding (the definition of
    /// `outstanding` below). The step: from any state of the buffer (any
    /// flag byte, any bytes), with any transfer value at all in hand (any
    /// channel, window, buffer address and stop flag: more than Task code
    /// can construct, the fields being private) and any values of that
    /// transfer's ENABLE register and STOPPED event, each operation of the
    /// library
    ///
    /// - hands out the bytes exactly when no transfer is outstanding
    ///   (`bytes`),
    /// - starts a transfer exactly when none is outstanding, and names this
    ///   buffer and the channel's peripheral in the value it returns
    ///   (`lend`),
    /// - ends "outstanding" only in `reclaim`, only for a transfer value
    ///   that names this buffer, and only when the peripheral that value
    ///   names is not enabled or reports STOPPED; any other `reclaim` hands
    ///   the value back with the buffer still lent,
    /// - and otherwise leaves the flag alone (keeping a transfer in a
    ///   `DmaHeld` and taking it out again, which returns the same value
    ///   once; ending or dropping a transfer without its buffer).
    ///
    /// With `a_new_buffer_is_not_lent` as the base case, the invariant holds
    /// after any number of operations. What stays as in the harness above:
    /// the supervisor calls and registers are the host doubles (a request
    /// is always accepted; the refused request returns before the flag is
    /// written), the wait for STOPPED is two reads, and that a peripheral
    /// reporting STOPPED has stopped is the vendor's.
    #[kani::proof]
    #[kani::unwind(4)]
    fn any_operation_from_any_state_keeps_the_bytes_out_of_reach() {
        let channel = DmaChannel::generated(kani::any(), WINDOW);
        let mut buf: DmaBuf<2> = DmaBuf { lent: kani::any(), bytes: kani::any() };
        let addr = buf.address();
        let lent0 = buf.lent;
        // The invariant, as the definition of the ghost.
        let outstanding = buf.lent != 0;
        // Any transfer value. Its window stays clear of the two cells the
        // supervisor-call doubles record their arguments in.
        let tw: u32 = kani::any();
        kani::assume(tw <= 0xF000_0000);
        let (tc, tb, ts): (u8, u32, bool) = (kani::any(), kani::any(), kani::any());
        let token = DmaTransfer { channel: tc, window: tw, buffer: tb, stop_requested: ts };
        let enable: u32 = kani::any();
        let event: u32 = kani::any();
        mmio::write(tw + spim::ENABLE, enable);
        mmio::write(tw + spim::EVENTS_STOPPED, event);
        let at_rest = enable != spim::ENABLED || event != 0;
        let op: u8 = kani::any();
        kani::assume(op < 5);
        if op == 0 {
            assert!(buf.bytes().is_some() == !outstanding);
            assert!(buf.lent == lent0);
        } else if op == 1 {
            let t = buf.lend(channel);
            assert!(t.is_some() == !outstanding);
            match &t {
                Some(t) => {
                    assert!(buf.lent != 0);
                    assert!(t.buffer == addr && t.window == WINDOW && t.channel == channel.index && !t.stop_requested);
                }
                None => assert!(buf.lent == lent0),
            }
        } else if op == 2 {
            match buf.reclaim(token) {
                Ok(()) => {
                    assert!(outstanding && tb == addr && at_rest);
                    assert!(buf.lent == 0);
                }
                Err(DmaReclaimError::NotStopped(t)) => {
                    assert!(outstanding && tb == addr && !at_rest);
                    assert!(buf.lent == lent0);
                    assert!(t.channel == tc && t.window == tw && t.buffer == tb && t.stop_requested);
                }
                Err(DmaReclaimError::NotThisBuffer(t)) => {
                    assert!(!outstanding || tb != addr);
                    assert!(buf.lent == lent0);
                    assert!(t.channel == tc && t.window == tw && t.buffer == tb && t.stop_requested == ts);
                }
            }
            return;
        } else if op == 3 {
            // Any holder, holding something or not.
            let mut held = DmaHeld { present: kani::any(), channel: kani::any(), stop_requested: kani::any(), _unused: 0, window: kani::any(), buffer: kani::any() };
            held.put(Some(token));
            assert!(held.is_some());
            match held.take() {
                Some(t) => assert!(t.channel == tc && t.window == tw && t.buffer == tb && t.stop_requested == ts),
                None => assert!(false),
            }
            assert!(!held.is_some() && held.take().is_none());
            assert!(buf.lent == lent0);
            return;
        } else {
            if kani::any() {
                let _ = token.end();
            } else {
                drop(token);
            }
            assert!(buf.lent == lent0);
            return;
        }
        drop(token);
        assert!(buf.lent != 0 || buf.bytes().is_some());
    }

    /// The base case of the induction above: a new buffer is not lent, its
    /// bytes are at hand, and a new holder holds nothing.
    #[kani::proof]
    fn a_new_buffer_is_not_lent() {
        let mut buf: DmaBuf<2> = DmaBuf::new();
        assert!(!buf.is_lent() && buf.lent == 0);
        assert!(buf.bytes().is_some());
        let mut held = DmaHeld::none();
        assert!(!held.is_some() && held.take().is_none());
    }
}
