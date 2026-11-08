//! The Protocol_Checker (Component F, task 21.1; R38.2, R38.3, R38.5,
//! R38.6, R41.1): a global Protocol in a Scribble-like text, its
//! well-formedness, its projection onto every role (the local types the
//! typestate APIs of task 21.3 are generated from), and the bound `k` on
//! the messages in transit on every directed channel, found by exploring
//! every interleaving of the roles' local types over asynchronous
//! channels.
//!
//! # Syntax
//!
//! ```text
//! rec Loop {
//!     choice at ctl {
//!         Fix(u32): ctl -> sink;
//!         Ack(u32): sink -> ctl;
//!         continue Loop;
//!     } or {
//!         Stop(u32): ctl -> sink;
//!         end;
//!     }
//! }
//! ```
//!
//! A statement is a message `Label(Type): from -> to;` (the type may be
//! absent: `Label(): a -> b;`), a `choice at Role { .. } or { .. }` with two
//! or more branches each starting with a message from the choosing role to
//! one same receiver under distinct labels, a `rec Name { .. }` with
//! `continue Name;` inside it (guarded by at least one message on every
//! path), or `end;`. A block ends with `end`, `continue`, a `choice`, or a
//! `rec`.
//!
//! # What the theory gives and where this checker deviates (R38.6, R61)
//!
//! Projection and merge are the standard ones of multiparty session types
//! (the full merge of external choices, as Rumpsteak's checker and Scribble
//! implement them, RN-22). The bound differs on purpose: Rumpsteak's
//! k-multiparty compatibility (Lange and Yoshida) asks for a `k` under which
//! *some* k-bounded execution fires every send, an existential bound that
//! proves safety for unbounded channels. The Kernel's channels are finite
//! and a full one is a reported fault (R40.5), so this checker computes the
//! *universal* bound: the largest occupancy any interleaving of the local
//! types reaches, with every send enabled whenever its role's local type
//! allows it. A Protocol whose fastest sender can outrun its receiver
//! without a message back (`rec X { M: a -> b; continue X }`) has no such
//! bound and is rejected against the declared capacity (R41.1); a
//! request-response loop has `k = 1` each way. The exploration also checks
//! what k-MC checks: no deadlock, no orphan message, no unspecified
//! reception, over every interleaving within the declared capacity.
//!
//! Rumpsteak itself is not vendored (R58.1) and is not run here; the
//! differential evidence for Property 20 is the agreement, over generated
//! Protocols, between the synchronous traces of the global type and those
//! of the product of its projections (`tests/property20.rs`).

#![forbid(unsafe_code)]

pub mod ast;
pub mod check;
pub mod explore;
pub mod parse;

pub use ast::{Global, Local, Message};
pub use check::{project, well_formed};
pub use explore::{explore, Channel, Exploration};

/// The result of checking a Protocol for one Endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub global: Global,
    /// (role, its local type), in the declared roles' order.
    pub locals: Vec<(String, Local)>,
    /// The bound of every directed channel the Protocol uses.
    pub channels: Vec<Channel>,
    /// The largest bound over the channels (0 for a Protocol with no message).
    pub k: u32,
}

/// Why a Protocol is rejected (R38.5): the rule and the offending
/// interaction, in words.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolError {
    /// `R38.2`, `R38.3`, `R38.5`, or `R41.1`.
    pub rule: &'static str,
    pub message: String,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}", self.rule, self.message)
    }
}

/// Parses, checks, projects, and bounds a Protocol for an Endpoint whose
/// roles are `roles` and whose channels hold `capacity` messages each
/// (R22.7: the Generator rejects the Endpoint on `Err`).
pub fn check_protocol(text: &str, roles: &[&str], capacity: u32) -> Result<Checked, ProtocolError> {
    let global = parse::parse(text).map_err(|m| ProtocolError { rule: "R38.2", message: m })?;
    well_formed(&global, roles)?;
    let mut locals = Vec::new();
    for r in roles {
        locals.push(((*r).to_string(), project(&global, r)?));
    }
    let exploration = explore(&locals, capacity)?;
    let k = exploration.channels.iter().map(|c| c.k).max().unwrap_or(0);
    Ok(Checked { global, locals, channels: exploration.channels, k })
}
