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
//!
//! # Deadlock freedom (R41.3, task 21.6)
//!
//! The exploration is untrusted; its visited set is checked by
//! `progress::certify`, verified in Verus: the set contains the initial
//! configuration, is closed under every operation the local types enable,
//! holds no channel over the capacity, and has no configuration in which a
//! non-empty set of roles each awaits a message only another role of the
//! set can send. `progress::theorem_r41_3` derives from that certificate
//! the property for every run of the product. The assumptions connecting
//! the product to an Application are in the Verification_Plan (5.15).

#![forbid(unsafe_code)]
#![cfg_attr(not(verus_keep_ghost), allow(unused_imports))]

pub mod ast;
pub mod check;
pub mod explore;
pub mod parse;
pub mod compose;
pub mod progress;
pub mod theory;
pub mod multiparty;
pub mod parse_v;

pub use ast::{Branch, Global, Local, Message};
pub use check::{project, well_formed};
pub use explore::{explore, Channel, Exploration};
pub use theory::{accept_two_role, flatten_two};
pub use multiparty::{accept_multiparty, flatten_n};
pub use progress::explore_system;

/// The result of checking a Protocol for one Endpoint.
/// One state of a role's verified table, as the Generator renders it: the
/// head-normal local type of the state (A-P1, 2026-11-09), the labels in
/// the order of the projection's branches.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RoleEntry {
    End,
    Send { to: usize, labels: Vec<u32> },
    Recv { from: usize, labels: Vec<u32> },
}

/// The plain reading of a verified table entry (the one step of plain Rust
/// between the flattening and the role module).
fn entry_of(e: &theory::LE) -> RoleEntry {
    fn labels_of(ls: &theory::LsE) -> Vec<u32> {
        let mut out = Vec::new();
        let mut cur = ls;
        while let theory::LsE::Cons { label, rest, .. } = cur {
            out.push(*label);
            cur = rest;
        }
        out
    }
    match e {
        theory::LE::End => RoleEntry::End,
        theory::LE::Send { to, ls } => RoleEntry::Send { to: *to, labels: labels_of(ls) },
        theory::LE::Recv { from, ls } => RoleEntry::Recv { from: *from, labels: labels_of(ls) },
        // Entries are head normal (`flattens`): a recursion or a variable cannot occur.
        theory::LE::Rec { .. } | theory::LE::Var { .. } => RoleEntry::End,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Checked {
    pub global: Global,
    /// (role, its local type), in the declared roles' order.
    pub locals: Vec<(String, Local)>,
    /// The bound of every directed channel the Protocol uses.
    pub channels: Vec<Channel>,
    /// The largest bound over the channels (0 for a Protocol with no message).
    pub k: u32,
    /// The configurations of the product of the projections, each certified
    /// by the verified check of `progress` (task 21.6, R41.3).
    pub configurations: usize,
    /// The roles' state tables from the verified flattening (A-P1,
    /// 2026-11-09): per role in the declared order, the head-normal local
    /// types of its states, entry 0 the head of its projection
    /// (`theory::flatten_two`, `multiparty::flatten_n`).
    pub tables: Vec<Vec<RoleEntry>>,
    /// The automaton's transitions over those tables (`flattens`,
    /// `flattens_n`): the Generator's role modules are rendered from them.
    pub trans: Vec<progress::Trans>,
    /// The labels in the numbering of the verified conversion (`to_theory`:
    /// by first appearance), the label word of a message.
    pub labels: Vec<String>,
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
    let mut verified_tables: Vec<Vec<RoleEntry>> = Vec::new();
    let mut verified_trans: Vec<progress::Trans> = Vec::new();
    let mut verified_labels: Vec<String> = Vec::new();
    if roles.len() == 2 {
        // The general theorem's hypothesis for two roles (theory.rs,
        // `theorem_two_role_progress`): the structural acceptance decided
        // by the verified `accept_two_role` over the verified conversion
        // `theory::to_theory` of the parsed type (roles by position, labels
        // by first appearance, recursion in de Bruijn form). The
        // structural checks above accept exactly these Protocols, so a
        // refusal here is a discrepancy, reported as such.
        let (ge, _labels) = theory::to_theory(&global, roles);
        if !theory::accept_two_role(&ge) {
            return Err(ProtocolError { rule: "R41.3", message: "the Protocol passes the structural checks but not the verified two-role acceptance (theory::wf2_exec); the two disagree".into() });
        }
        // The verified chain from the syntax to the automaton (2026-11-09):
        // the flattening of the projections (`theory::flatten_two`, which
        // establishes `flattens`) and the exploration of the automaton
        // (`progress::explore_system`, which establishes the certificate
        // and the bound k). The untrusted explorer above remains the
        // diagnosis and the cross-check: its bound must agree.
        let Some((sys, t0, t1)) = theory::flatten_two(&ge, capacity as usize) else {
            return Err(ProtocolError { rule: "R41.3", message: "the verified flattening of the projections refused the Protocol (a state table beyond its bound, an inadmissible entry, or an index overflow)".into() });
        };
        verified_tables = vec![t0.iter().map(entry_of).collect(), t1.iter().map(entry_of).collect()];
        verified_trans = sys.trans.iter().map(|t| progress::Trans { role: t.role, from: t.from, send: t.send, peer: t.peer, label: t.label, to: t.to }).collect();
        verified_labels = _labels;
        let Some((list, verified_k)) = progress::explore_system(&sys) else {
            return Err(ProtocolError { rule: "R41.3", message: "the verified exploration of the automaton refused the Protocol (a waiting set, a channel over the capacity, or more configurations than its bound) where the explorer accepted it; the two disagree".into() });
        };
        if verified_k as u32 != exploration.channels.iter().map(|c| c.k).max().unwrap_or(0) {
            return Err(ProtocolError { rule: "R41.1", message: format!("the verified exploration bounds the channels at {verified_k} message(s) in transit, the explorer at {}; the two disagree", exploration.channels.iter().map(|c| c.k).max().unwrap_or(0)) });
        }
        if !progress::certify(&sys, &list) {
            return Err(ProtocolError { rule: "R41.3", message: "the certificate check refused the verified exploration's own list; the two disagree".into() });
        }
    }
    if roles.len() >= 3 {
        // The multiparty theorem's hypothesis (multiparty.rs,
        // `theorem_multiparty_progress`, 2026-11-09): the acceptance
        // conditions over the syntax (`acceptn`: well formed and projectable
        // onto every role with the full merge) decided by the verified
        // `accept_multiparty` over the verified conversion. The structural
        // checks above accept exactly these Protocols (the one difference,
        // a recursion a role takes no part in, is `end` for the checker and
        // the vacuous loop for the theory, both without an action), so a
        // refusal here is a discrepancy, reported as such.
        let (ge, _labels) = theory::to_theory(&global, roles);
        if !multiparty::accept_multiparty(&ge, roles.len()) {
            return Err(ProtocolError { rule: "R41.3", message: format!("the Protocol passes the structural checks but not the verified {}-role acceptance (multiparty::acceptn_exec); the two disagree", roles.len()) });
        }
        // The verified chain to the automaton for `n` roles (2026-11-09):
        // `multiparty::flatten_n` (establishing `flattens_n`, the hypothesis
        // of `theorem_multiparty_automaton`) and `progress::explore_system`
        // (the certificate and the bound k), the untrusted explorer above
        // the diagnosis and the cross-check.
        let Some((sys, tables)) = multiparty::flatten_n(&ge, roles.len(), capacity as usize) else {
            return Err(ProtocolError { rule: "R41.3", message: "the verified flattening of the projections refused the Protocol (a state table beyond its bound, an inadmissible entry, or an index overflow)".into() });
        };
        verified_tables = tables.iter().map(|t| t.iter().map(entry_of).collect()).collect();
        verified_trans = sys.trans.iter().map(|t| progress::Trans { role: t.role, from: t.from, send: t.send, peer: t.peer, label: t.label, to: t.to }).collect();
        verified_labels = _labels;
        let Some((list, verified_k)) = progress::explore_system(&sys) else {
            return Err(ProtocolError { rule: "R41.3", message: "the verified exploration of the automaton refused the Protocol (a waiting set, a channel over the capacity, or more configurations than its bound) where the explorer accepted it; the two disagree".into() });
        };
        if verified_k as u32 != exploration.channels.iter().map(|c| c.k).max().unwrap_or(0) {
            return Err(ProtocolError { rule: "R41.1", message: format!("the verified exploration bounds the channels at {verified_k} message(s) in transit, the explorer at {}; the two disagree", exploration.channels.iter().map(|c| c.k).max().unwrap_or(0)) });
        }
        if !progress::certify(&sys, &list) {
            return Err(ProtocolError { rule: "R41.3", message: "the certificate check refused the verified exploration's own list; the two disagree".into() });
        }
    }
    let k = exploration.channels.iter().map(|c| c.k).max().unwrap_or(0);
    Ok(Checked { global, locals, channels: exploration.channels, k, configurations: exploration.configurations, tables: verified_tables, trans: verified_trans, labels: verified_labels })
}

