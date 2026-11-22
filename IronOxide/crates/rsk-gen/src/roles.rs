//! The typestate role APIs (task 21.3; R38.3, R38.4, R38.7, R41.2): for a
//! role of a Protocol-governed Endpoint, the states of its projection, one
//! zero-sized-but-for-a-flag value type per state with exactly the
//! operations the projection permits in it (R38.4: anything else is a
//! missing method, a type error), a `State` enum that holds the role's
//! current state in the Task's `RoleSlot` between Jobs (R38.7, PR-25), and
//! the run-time half of R41.2: a role value dropped in a state other than
//! an end or a recursion state reports a Protocol violation from its `Drop`,
//! a message with a label the state does not expect reports one, and the
//! generated Job wrapper reports one when a Job returns with its slot
//! empty.
//!
//! Messages travel as words over the Kernel's channel (`rsk::Endpoint<[u32;
//! W], W>`): word 0 the label's index in the Protocol, the payload's words
//! after it (`rsk::words`), so the declared `payload` is 4 bytes more than
//! the largest message, which the module asserts at compile time per label.
//!
//! The `State` enum is `#[repr(u8)]` with the initial state as variant 0 and
//! its value's only field a `false` flag, so that the zeroed image a
//! Partition restart leaves (R14.10) is the initial state.
//!
//! Task 21.4: a peer-state change (R42.3, the other Partition's stop or
//! restart) comes back from the next operation as `PeerChanged` carrying
//! the role in its initial state, both roles starting over; a received
//! payload the role's declared validation function rejects (R42.4) is
//! reported as the sending Partition's fault and comes back as `Rejected`
//! with the state unchanged. The module compiles under the App_Prover
//! (`cfg(creusot)`) but for the `Drop` report, so that its code is proven
//! with the Task bodies against `rsk`'s trusted Kernel glue.

use std::fmt::Write;

use rsk_protocol::Local;

use crate::derive::{GenEndpoint, Generated};

/// One state of a role's projection.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Kind {
    Send { to: String, branches: Vec<(String, Option<String>, usize)> },
    Recv { from: String, branches: Vec<(String, Option<String>, usize)> },
    End,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleState {
    pub kind: Kind,
    /// The target of a `rec`: dropping a role value here is no violation (R41.2).
    pub rec_head: bool,
}

/// The states of a role from the verified flattening (task 21.7 follow-up,
/// 2026-11-09; A-P1; R38.4, 2026-11-21): `api` is what each state of the
/// role's table offers, as `rsk_protocol::api::role_ops` derives it and
/// proves it (`role_api`: the kind, the peer, the labels in the
/// projection's order and the state after each are those of the state's
/// local type, entry 0 the head of the projection). This function only
/// renders them: a label index to its name and payload, a role index to
/// its name. It searches nothing and has no default: the state after an
/// operation is the proven one. A state is a recursion head when a
/// transition from a later (or the same) state returns to it.
pub fn states_from_api(api: &[rsk_protocol::api::StateOps], role_names: &[String], labels: &[String], payload_of: &dyn Fn(&str) -> Option<String>) -> Vec<RoleState> {
    use rsk_protocol::api::{KIND_RECV, KIND_SEND};
    let branches_of = |ops: &[rsk_protocol::api::Op]| -> Vec<(String, Option<String>, usize)> {
        ops.iter()
            .map(|op| {
                let name = labels.get(op.label as usize).cloned().unwrap_or_else(|| format!("L{}", op.label));
                (name.clone(), payload_of(&name), op.to)
            })
            .collect()
    };
    let role_name = |r: usize| role_names.get(r).cloned().unwrap_or_else(|| format!("r{r}"));
    let mut out: Vec<RoleState> = api
        .iter()
        .map(|st| match st.kind {
            KIND_SEND => RoleState { kind: Kind::Send { to: role_name(st.peer), branches: branches_of(&st.ops) }, rec_head: false },
            KIND_RECV => RoleState { kind: Kind::Recv { from: role_name(st.peer), branches: branches_of(&st.ops) }, rec_head: false },
            _ => RoleState { kind: Kind::End, rec_head: false },
        })
        .collect();
    for (from, st) in api.iter().enumerate() {
        for op in &st.ops {
            if op.to <= from && op.to < out.len() {
                out[op.to].rec_head = true;
            }
        }
    }
    out
}

/// The payload of a label, as the Protocol declares it (the first message
/// carrying the label; the checker keeps one payload per label).
pub fn payload_of(g: &rsk_protocol::Global, label: &str) -> Option<String> {
    fn walk(g: &rsk_protocol::Global, label: &str) -> Option<Option<String>> {
        match g {
            rsk_protocol::Global::End | rsk_protocol::Global::Continue(_) => None,
            rsk_protocol::Global::Msg(m, rest) => {
                if m.label == label { Some(m.payload.clone()) } else { walk(rest, label) }
            }
            rsk_protocol::Global::Choice { branches, .. } => branches.iter().find_map(|b| walk(b, label)),
            rsk_protocol::Global::Rec { body, .. } => walk(body, label),
        }
    }
    walk(g, label).flatten()
}

/// The states of `role` in `checked`: from the verified tables when the
/// checker produced them (every Protocol since 2026-11-09), else from the
/// local type (the earlier numbering, kept for the comparison tests).
pub fn states_for(checked: &rsk_protocol::Checked, role: &str) -> Vec<RoleState> {
    let ri = checked.locals.iter().position(|(r, _)| r == role);
    match ri {
        Some(ri) if ri < checked.tables.len() => {
            let role_names: Vec<String> = checked.locals.iter().map(|(r, _)| r.clone()).collect();
            let g = &checked.global;
            states_from_api(&checked.tables[ri], &role_names, &checked.labels, &|l| payload_of(g, l))
        }
        // A role the checker produced no table for (none since 2026-11-09:
        // every accepted Protocol has two to four roles and the verified
        // flattening runs for each): no states, never the plain projection.
        _ => Vec::new(),
    }
}

pub fn type_name(role: &str) -> String {
    let mut c = role.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// The module of role `role` of Endpoint `e`, as Rust source for the
/// Partition's glue (`rsk_roles::<endpoint>_<role>`).
pub fn module(g: &Generated, e: &GenEndpoint, role: &str) -> String {
    let mut s = String::new();
    let local = e.locals.iter().find(|(r, _)| r == role).map(|(_, l)| l.clone()).unwrap_or(Local::End);
    // A-P1 (2026-11-09): the states come from the verified flattening when
    // the derivation carried it; the local type is kept for the rendering
    // of the projection in the documentation comment.
    let sts = match e.verified_states.iter().find(|(r, _)| r == role) {
        Some((_, sts)) => sts.clone(),
        None => Vec::new(),
    };
    let ty = type_name(role);
    let ri = e.roles.iter().position(|(r, _)| r == role).unwrap_or(0);
    // Task 21.6 (R41.3, A-P8): the Endpoints gating this role (`after`):
    // every operation takes their gates with the App_Prover precondition
    // that each is open (a receive on that Endpoint completed in this Job).
    let gates: Vec<String> = e.after.get(ri).cloned().unwrap_or_default();
    let gate_params: String = gates.iter().map(|x| format!(", gate_{x}: &::rsk::Gate")).collect();
    // A-P2 (2026-11-09): an accepted send handed the Kernel the label's
    // index as the message's first word (`rsk::channel::sent`, established
    // by `send_words`' contract; proven here because `w[0]` is the label).
    let sent_ensures = |s: &mut String, to: &str, label: &str| {
        let _ = writeln!(s, "            #[cfg_attr(creusot, ::rsk::creusot_std::macros::ensures(match result {{ Ok(_) => ::rsk::channel::sent(ID, ROLE, PEER_{}, LABEL_{}), Err(_) => true }}))]", to.to_uppercase(), label.to_uppercase());
        // A-P6's Task half (2026-11-21): the role comes back in its
        // initial state (`NotSent::PeerChanged` holds an `S0`) exactly when
        // the Kernel reported the peer-state change.
        let _ = writeln!(s, "            #[cfg_attr(creusot, ::rsk::creusot_std::macros::ensures(match result {{ Err(NotSent::PeerChanged(_)) => ::rsk::channel::peer_told(ID, ROLE), _ => ::rsk::channel::not_told(ID, ROLE) }}))]");
    };
    let gate_requires = |s: &mut String| {
        for x in &gates {
            let _ = writeln!(s, "            #[cfg_attr(creusot, ::rsk::creusot_std::macros::requires((*gate_{x})@))]");
        }
    };
    let words = (e.payload / 4) as usize;
    let payload_words = words.saturating_sub(1);
    let protocol = e.protocol.clone().unwrap_or_default();
    let _ = writeln!(s, "    /// Role `{role}` of Endpoint `{}` under Protocol `{protocol}` (task 21.3): the projection `{}`, states S0 to S{}, the initial state S0.", e.name, local.render(), sts.len().saturating_sub(1));
    let _ = writeln!(s, "    #[allow(non_camel_case_types, clippy::large_enum_variant)]");
    let _ = writeln!(s, "    pub mod {}_{role} {{", e.name);
    let _ = writeln!(s, "        use ::core::marker::PhantomData;");
    let _ = writeln!(s, "        const ID: u8 = {};", e.index);
    let _ = writeln!(s, "        const ROLE: u8 = {ri};");
    let _ = writeln!(s, "        /// Words per message (the declared payload / 4): the label, then the payload.");
    let _ = writeln!(s, "        pub const W: usize = {words};");
    let _ = writeln!(s, "        pub const PW: usize = {payload_words};");
    let _ = writeln!(s, "        type Raw = ::rsk::Endpoint<[u32; W], W>;");
    let _ = writeln!(s, "        #[cfg_attr(creusot, ::rsk::creusot_std::macros::ensures(result@ == (ID, ROLE)))]");
    let _ = writeln!(s, "        fn raw() -> Raw {{ ::rsk::Endpoint::new(ID, ROLE) }}");
    // Task 21.7: the index of every role of the Endpoint, the peer of each
    // operation (one channel per ordered pair of roles).
    for (pi, (pname, _)) in e.roles.iter().enumerate() {
        let _ = writeln!(s, "        pub const PEER_{}: u8 = {pi};", pname.to_uppercase());
    }
    for (i, l) in e.labels.iter().enumerate() {
        let _ = writeln!(s, "        pub const LABEL_{}: u32 = {i};", l.to_uppercase());
    }
    // Compile-time: every payload type of this role fits the payload words.
    for st in &sts {
        if let Kind::Send { branches, .. } | Kind::Recv { branches, .. } = &st.kind {
            for (label, payload, _) in branches {
                if let Some(p) = payload {
                    let _ = writeln!(s, "        const _: () = assert!(::core::mem::size_of::<{p}>() <= 4 * PW, \"the payload of `{label}` exceeds Endpoint `{}`'s declared payload less its label word (R38.1)\");", e.name);
                }
            }
        }
    }
    let _ = writeln!(s, "        /// A state marker: whether dropping a role value in the state is a Protocol violation (R41.2: not at an end or a recursion state).");
    let _ = writeln!(s, "        pub trait Tag {{ const REPORT_ON_DROP: bool; }}");
    for (i, st) in sts.iter().enumerate() {
        let report = !st.rec_head && !matches!(st.kind, Kind::End);
        let _ = writeln!(s, "        pub struct S{i};");
        let _ = writeln!(s, "        impl Tag for S{i} {{ const REPORT_ON_DROP: bool = {report}; }}");
    }
    let _ = writeln!(s, "        /// The role in state `S`: its only operations are those the projection permits there (R38.4).");
    let _ = writeln!(s, "        #[repr(C)]");
    let _ = writeln!(s, "        pub struct {ty}<S: Tag> {{ consumed: bool, _s: PhantomData<S> }}");
    let _ = writeln!(s, "        #[cfg(not(creusot))]");
    let _ = writeln!(s, "        impl<S: Tag> Drop for {ty}<S> {{");
    let _ = writeln!(s, "            fn drop(&mut self) {{");
    let _ = writeln!(s, "                if S::REPORT_ON_DROP && !self.consumed {{");
    let _ = writeln!(s, "                    ::rsk::report_protocol_violation(ID, ROLE);");
    let _ = writeln!(s, "                }}");
    let _ = writeln!(s, "            }}");
    let _ = writeln!(s, "        }}");
    let _ = writeln!(s, "        impl<S: Tag> {ty}<S> {{");
    let _ = writeln!(s, "            const fn fresh() -> {ty}<S> {{ {ty} {{ consumed: false, _s: PhantomData }} }}");
    let _ = writeln!(s, "            fn advance<T: Tag>(mut self) -> {ty}<T> {{ self.consumed = true; {ty}::<T>::fresh() }}");
    let _ = writeln!(s, "        }}");
    let _ = writeln!(s, "        /// Why a send did not take the message, with the role: `Full` (a reported violation of the bound, R40.5) and `Refused` keep the state; `PeerChanged` (R42.3, task 21.4: the other Partition was stopped or restarted and the Endpoint re-initialized) gives the role back in its initial state.");
    let _ = writeln!(s, "        pub enum NotSent<S: Tag> {{ Full({ty}<S>), Refused({ty}<S>), PeerChanged({ty}<S0>) }}");
    let _ = writeln!(s, "        /// Why a receive delivered nothing, with the role: `Empty` (R40.4), `Unexpected` (a label the state does not expect, a reported Protocol violation, R41.2), and `Rejected` (a payload the role's validation function refused, the sending Partition's reported fault, R42.4) keep the state; `PeerChanged` (R42.3) gives the role back in its initial state.");
    let _ = writeln!(s, "        pub enum NotReceived<S: Tag> {{ Empty({ty}<S>), Unexpected({ty}<S>), Rejected({ty}<S>), PeerChanged({ty}<S0>) }}");
    for (i, st) in sts.iter().enumerate() {
        match &st.kind {
            Kind::End => {
                let _ = writeln!(s, "        impl {ty}<S{i}> {{");
                let _ = writeln!(s, "            /// The Protocol has ended for this role: the state goes back to the slot as `State::S{i}`.");
                let _ = writeln!(s, "            pub fn finish(self) -> State {{ State::S{i}(self) }}");
                let _ = writeln!(s, "        }}");
            }
            Kind::Send { to, branches } => {
                let _ = writeln!(s, "        impl {ty}<S{i}> {{");
                for (label, payload, next) in branches {
                    let fname = format!("send_{}", label.to_lowercase());
                    match payload {
                        Some(p) => {
                            let _ = writeln!(s, "            /// Sends `{label}({p})` to `{to}` and moves to S{next}; `Err` gives the role back with the reason (a full channel is a reported violation of the bound, R40.5; a peer-state change resets it, R42.3). Never waits (R40.4).");
                            gate_requires(&mut s);
                            sent_ensures(&mut s, to, label);
                            let _ = writeln!(s, "            pub fn {fname}(self, m: {p}{gate_params}) -> Result<{ty}<S{next}>, NotSent<S{i}>> {{");
                            let _ = writeln!(s, "                let mut w = [0u32; W];");
                            let _ = writeln!(s, "                w[0] = LABEL_{};", label.to_uppercase());
                            let _ = writeln!(s, "                let p = ::rsk::words::message_to_words::<{p}, PW>(&m);");
                            let _ = writeln!(s, "                let mut i: usize = 0;");
                            let _ = writeln!(s, "                #[cfg_attr(creusot, ::rsk::creusot_std::macros::invariant(i@ <= PW@))]");
                            let _ = writeln!(s, "                #[cfg_attr(creusot, ::rsk::creusot_std::macros::invariant(w@[0] == LABEL_{}))]", label.to_uppercase());
                            let _ = writeln!(s, "                #[cfg_attr(creusot, ::rsk::creusot_std::macros::variant(PW@ - i@))]");
                            let _ = writeln!(s, "                while i < PW {{ w[i + 1] = p.0[i]; i += 1; }}");
                        }
                        None => {
                            let _ = writeln!(s, "            /// Sends `{label}()` to `{to}` and moves to S{next}; `Err` gives the role back with the reason. Never waits (R40.4).");
                            gate_requires(&mut s);
                            sent_ensures(&mut s, to, label);
                            let _ = writeln!(s, "            pub fn {fname}(self{gate_params}) -> Result<{ty}<S{next}>, NotSent<S{i}>> {{");
                            let _ = writeln!(s, "                let mut w = [0u32; W];");
                            let _ = writeln!(s, "                w[0] = LABEL_{};", label.to_uppercase());
                        }
                    }
                    let _ = writeln!(s, "                match raw().send_words(PEER_{}, &w) {{", to.to_uppercase());
                    let _ = writeln!(s, "                    Ok(()) => Ok(self.advance()),");
                    let _ = writeln!(s, "                    Err(::rsk::SendError::Full) => Err(NotSent::Full(self)),");
                    let _ = writeln!(s, "                    Err(::rsk::SendError::Refused) => Err(NotSent::Refused(self)),");
                    let _ = writeln!(s, "                    Err(::rsk::SendError::PeerChanged) => Err(NotSent::PeerChanged(self.advance::<S0>())),");
                    let _ = writeln!(s, "                }}");
                    let _ = writeln!(s, "            }}");
                }
                let _ = writeln!(s, "        }}");
            }
            Kind::Recv { from, branches } => {
                let _ = writeln!(s, "        /// What S{i} may receive from `{from}`, each with the state after it.");
                let _ = writeln!(s, "        pub enum RecvS{i} {{");
                for (label, payload, next) in branches {
                    match payload {
                        Some(p) => {
                            let _ = writeln!(s, "            {label}({p}, {ty}<S{next}>),");
                        }
                        None => {
                            let _ = writeln!(s, "            {label}({ty}<S{next}>),");
                        }
                    }
                }
                let _ = writeln!(s, "        }}");
                let _ = writeln!(s, "        /// A completed receive in this state: the evidence that opens a gate (task 21.6, A-P8).");
                let _ = writeln!(s, "        impl ::rsk::Received for RecvS{i} {{}}");
                let _ = writeln!(s, "        impl {ty}<S{i}> {{");
                let _ = writeln!(s, "            /// Receives the next message from `{from}`, or `Err` with the role when none is delivered (R40.4): a label the state does not expect is a reported Protocol violation that leaves the state as it is (R41.2), a payload the validation function rejects is the sender's reported fault (R42.4), a peer-state change resets the role (R42.3).");
                gate_requires(&mut s);
                // A-P2 (2026-11-09): the variant returned names the label the
                // Kernel delivered as the message's first word.
                let arms: String = branches.iter().map(|(label, _, _)| format!("Ok(RecvS{i}::{label}(..)) => ::rsk::channel::received(ID, PEER_{}, ROLE, LABEL_{}), ", from.to_uppercase(), label.to_uppercase())).collect();
                let _ = writeln!(s, "            #[cfg_attr(creusot, ::rsk::creusot_std::macros::ensures(match result {{ {arms}Err(_) => true }}))]");
                // A-P6's Task half (2026-11-21), as for a send.
                let _ = writeln!(s, "            #[cfg_attr(creusot, ::rsk::creusot_std::macros::ensures(match result {{ Err(NotReceived::PeerChanged(_)) => ::rsk::channel::peer_told(ID, ROLE), _ => ::rsk::channel::not_told(ID, ROLE) }}))]");
                let _ = writeln!(s, "            pub fn try_recv(self{gate_params}) -> Result<RecvS{i}, NotReceived<S{i}>> {{");
                let _ = writeln!(s, "                match raw().recv_words(PEER_{}) {{", from.to_uppercase());
                let _ = writeln!(s, "                    Ok(None) | Err(::rsk::RecvError::Rejected) => Err(NotReceived::Empty(self)),");
                let _ = writeln!(s, "                    Err(::rsk::RecvError::PeerChanged) => Err(NotReceived::PeerChanged(self.advance::<S0>())),");
                let _ = writeln!(s, "                    Ok(Some(w)) => {{");
                let _ = writeln!(s, "                        let mut p = ::rsk::words::zero::<PW>();");
                let _ = writeln!(s, "                        let mut i: usize = 0;");
                let _ = writeln!(s, "                        #[cfg_attr(creusot, ::rsk::creusot_std::macros::invariant(i@ <= PW@))]");
                let _ = writeln!(s, "                        #[cfg_attr(creusot, ::rsk::creusot_std::macros::variant(PW@ - i@))]");
                let _ = writeln!(s, "                        while i < PW {{ p.0[i] = w[i + 1]; i += 1; }}");
                let _ = writeln!(s, "                        match w[0] {{");
                for (label, payload, next) in branches {
                    match payload {
                        Some(p) => {
                            let validator = e.validators.get(ri).and_then(|v| v.iter().find(|(l, _)| l.as_deref() == Some(label.as_str()))).map(|(_, f)| f.clone());
                            let _ = writeln!(s, "                            LABEL_{} => {{", label.to_uppercase());
                            let _ = writeln!(s, "                                let v = ::rsk::words::message_from_words::<{p}, PW>(&p);");
                            if let Some(f) = validator {
                                let _ = writeln!(s, "                                // R42.4: the declared validation function of `{label}`.");
                                let _ = writeln!(s, "                                if !{f}(&v) {{");
                                let _ = writeln!(s, "                                    ::rsk::report_rejected_message(ID, ROLE, PEER_{});", from.to_uppercase());
                                let _ = writeln!(s, "                                    return Err(NotReceived::Rejected(self));");
                                let _ = writeln!(s, "                                }}");
                            }
                            let _ = writeln!(s, "                                Ok(RecvS{i}::{label}(v, self.advance::<S{next}>()))");
                            let _ = writeln!(s, "                            }}");
                        }
                        None => {
                            let _ = writeln!(s, "                            LABEL_{} => Ok(RecvS{i}::{label}(self.advance::<S{next}>())),", label.to_uppercase());
                        }
                    }
                }
                let _ = writeln!(s, "                            _ => {{ ::rsk::report_protocol_violation(ID, ROLE); Err(NotReceived::Unexpected(self)) }}");
                let _ = writeln!(s, "                        }}");
                let _ = writeln!(s, "                    }}");
                let _ = writeln!(s, "                }}");
                let _ = writeln!(s, "            }}");
                let _ = writeln!(s, "        }}");
            }
        }
    }
    // The State enum: variant i = state i, Empty last; zeroed = S0 fresh.
    let _ = writeln!(s, "        /// The role's state between Jobs (R38.7), held in the Task's `RoleSlot`: `Empty` while a Job holds it. `#[repr(u8)]` with S0 as 0, so a Partition restart's zeroed image is the initial state.");
    let _ = writeln!(s, "        #[repr(u8)]");
    let _ = writeln!(s, "        pub enum State {{");
    for i in 0..sts.len() {
        let _ = writeln!(s, "            S{i}({ty}<S{i}>) = {i},");
    }
    let _ = writeln!(s, "            Empty = {},", sts.len());
    let _ = writeln!(s, "        }}");
    let _ = writeln!(s, "        impl State {{");
    let _ = writeln!(s, "            /// The initial state, the slot's initializer (a `const fn`, not a `const` item: the App_Prover would make the item transparent and refuse the private constructor it calls).");
    let _ = writeln!(s, "            pub const fn initial() -> State {{ State::S0({ty}::<S0>::fresh()) }}");
    let _ = writeln!(s, "            pub fn is_empty(&self) -> bool {{ matches!(self, State::Empty) }}");
    let _ = writeln!(s, "        }}");
    for i in 0..sts.len() {
        let _ = writeln!(s, "        impl From<{ty}<S{i}>> for State {{ fn from(v: {ty}<S{i}>) -> State {{ State::S{i}(v) }} }}");
    }
    let _ = writeln!(s, "    }}");
    let _ = g;
    s
}

/// A starter Task body for one role of a Protocol-governed Endpoint.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Skeleton {
    pub endpoint: String,
    pub role: String,
    /// The Task bound to the role, and its Partition.
    pub task: String,
    pub partition: String,
    pub source: String,
}

impl Skeleton {
    /// The file under the output directory: `skeletons/<endpoint>_<role>.rs`.
    pub fn file_name(&self) -> String {
        format!("{}/{}_{}.rs", crate::SKELETON_DIR, self.endpoint, self.role)
    }
}

/// The skeletons of every role of every Protocol-governed Endpoint, in
/// declaration order. They are written next to the configuration summary
/// for the user to copy into the Partition crate; no build reads them, so
/// the Generator rewriting them never touches the user's code.
pub fn skeletons(g: &Generated) -> Vec<Skeleton> {
    let mut out = Vec::new();
    for e in g.endpoints.iter().filter(|e| e.protocol.is_some()) {
        for (role, ti) in &e.roles {
            let Some(task) = g.tasks.get(*ti) else { continue };
            let partition = g.partitions.get(task.partition).map(|p| p.name.clone()).unwrap_or_default();
            out.push(Skeleton { endpoint: e.name.clone(), role: role.clone(), task: task.name.clone(), partition, source: skeleton(g, e, role) });
        }
    }
    out
}

/// The skeleton of role `role` of Endpoint `e`, in the shape the tutorial
/// teaches: take the role's state from its slot, match on every state,
/// store the result back. Every state has its arm, every receive its
/// variants, and every failed operation an arm that hands its state back;
/// `todo!()` stands where the Task's own logic goes. A role declared `after`
/// an Endpoint returns while that gate is closed, and the Task's receive on
/// the gating Endpoint opens it (R41.3, A-P8). The function compiles as
/// written in the Partition crate of the role's Task.
pub fn skeleton(g: &Generated, e: &GenEndpoint, role: &str) -> String {
    let mut s = String::new();
    let sts = e.verified_states.iter().find(|(r, _)| r == role).map(|(_, sts)| sts.clone()).unwrap_or_default();
    let ri = e.roles.iter().position(|(r, _)| r == role).unwrap_or(0);
    let task = e.roles.get(ri).and_then(|(_, ti)| g.tasks.get(*ti));
    let task_name = task.map(|t| t.name.clone()).unwrap_or_default();
    let partition = task.and_then(|t| g.partitions.get(t.partition)).map(|p| p.name.clone()).unwrap_or_default();
    let projection = e.projections.iter().find(|(r, _)| r == role).map(|(_, l)| l.clone()).unwrap_or_default();
    let protocol = e.protocol.clone().unwrap_or_default();
    let gates: Vec<String> = e.after.get(ri).cloned().unwrap_or_default();
    let gate_args: Vec<String> = gates.iter().map(|x| format!("&cx.{x}_gate")).collect();
    // Whether a role of the same Task is declared `after` this Endpoint:
    // then a completed receive here opens the Task's gate of that name.
    let opens = e.roles.get(ri).is_some_and(|(_, ti)| crate::emit::gating_endpoints(g, *ti).contains(&e.name));
    let name = &e.name;
    let fname = format!("{name}_{role}");

    let _ = writeln!(s, "// Skeleton for role `{role}` of Endpoint `{name}` under Protocol `{protocol}`,");
    let _ = writeln!(s, "// played by Task `{task_name}` of Partition `{partition}`.");
    let _ = writeln!(s, "//");
    let _ = writeln!(s, "// A starting point written by rsk-gen. No build reads this file and rsk-gen");
    let _ = writeln!(s, "// rewrites it on every run: copy the function into the `{partition}` crate, edit");
    let _ = writeln!(s, "// the copy, and call it from the body of `{task_name}` as `{fname}(&mut cx);`.");
    let _ = writeln!(s, "//");
    let _ = writeln!(s, "// The role's projection: {projection}");
    let _ = writeln!(s, "// Each Job performs one step: it takes the state from the slot, does what the");
    let _ = writeln!(s, "// state permits, and stores the state that results. A failed operation hands");
    let _ = writeln!(s, "// the state back, so the next Job tries again; after `PeerChanged` (the other");
    let _ = writeln!(s, "// Partition stopped or restarted) the role is back in its initial state S0.");
    if !gates.is_empty() {
        let _ = writeln!(s, "//");
        let named = gates.iter().map(|x| format!("`{x}`")).collect::<Vec<_>>().join(", ");
        let _ = writeln!(s, "// The role is declared `after = {}`: its operations take the gate of each", gates.join(", "));
        let _ = writeln!(s, "// Endpoint named, and the App_Prover requires it open. Call this after the");
        let _ = writeln!(s, "// Job's receive on {named}; a Job whose receive delivered");
        let _ = writeln!(s, "// nothing leaves the gate closed, and the function then returns at once.");
    }
    let _ = writeln!(s, "//");
    let _ = writeln!(s, "// Replace every `todo!()`; then delete the `allow` attribute below.");

    // The imports the body uses, and no others.
    let mut imports: Vec<String> = Vec::new();
    if sts.iter().any(|st| matches!(st.kind, Kind::Recv { .. })) {
        imports.push("NotReceived".to_string());
    }
    if sts.iter().any(|st| matches!(st.kind, Kind::Send { .. })) {
        imports.push("NotSent".to_string());
    }
    for (i, st) in sts.iter().enumerate() {
        if matches!(st.kind, Kind::Recv { .. }) {
            imports.push(format!("RecvS{i}"));
        }
    }
    imports.push("State".to_string());

    let _ = writeln!(s, "#[allow(unreachable_code, unused_variables, clippy::diverging_sub_expression)]");
    let _ = writeln!(s, "pub fn {fname}(cx: &mut rsk_ctx::{}) {{", crate::emit::ctx_name(&task_name));
    let _ = writeln!(s, "    use rsk_roles::{fname}::{{{}}};", imports.join(", "));
    if !gates.is_empty() {
        let closed: Vec<String> = gates.iter().map(|x| format!("!cx.{x}_gate.is_open()")).collect();
        let _ = writeln!(s, "    // Nothing was received on {} in this Job: the role stays as it is.", gates.iter().map(|x| format!("`{x}`")).collect::<Vec<_>>().join(" or "));
        let _ = writeln!(s, "    if {} {{", closed.join(" || "));
        let _ = writeln!(s, "        return;");
        let _ = writeln!(s, "    }}");
    }
    let _ = writeln!(s, "    let next = match cx.{name}.take(State::Empty) {{");
    for (i, st) in sts.iter().enumerate() {
        match &st.kind {
            Kind::End => {
                let _ = writeln!(s, "        // S{i}: the Protocol has ended for this role.");
                let _ = writeln!(s, "        State::S{i}(role) => role.finish(),");
            }
            Kind::Send { to, branches } => {
                let labels: Vec<&str> = branches.iter().map(|(l, _, _)| l.as_str()).collect();
                let _ = writeln!(s, "        // S{i}: send {} to `{to}`.", labels.join(" or "));
                // One send: `open` is what precedes the `match`, `pad` the
                // indentation of the match's closing brace.
                let send = |s: &mut String, open: &str, pad: &str, close: &str, label: &str, payload: &Option<String>, next: usize| {
                    let mut args: Vec<String> = Vec::new();
                    if let Some(p) = payload {
                        args.push(format!("todo!(\"the {label} payload: {p}\")"));
                    }
                    args.extend(gate_args.iter().cloned());
                    let _ = writeln!(s, "{open}match role.send_{}({}) {{", label.to_lowercase(), args.join(", "));
                    let _ = writeln!(s, "{pad}    Ok(s{next}) => State::S{next}(s{next}),");
                    let _ = writeln!(s, "{pad}    // The channel is full (a reported violation of the Protocol's bound): try again.");
                    let _ = writeln!(s, "{pad}    Err(NotSent::Full(s{i})) => State::S{i}(s{i}),");
                    let _ = writeln!(s, "{pad}    // The Kernel refused the send: try again.");
                    let _ = writeln!(s, "{pad}    Err(NotSent::Refused(s{i})) => State::S{i}(s{i}),");
                    let _ = writeln!(s, "{pad}    // The peer's Partition stopped or restarted: both roles start over.");
                    let _ = writeln!(s, "{pad}    Err(NotSent::PeerChanged(s0)) => State::S0(s0),");
                    let _ = writeln!(s, "{pad}}}{close}");
                };
                match branches.as_slice() {
                    [(label, payload, next)] => send(&mut s, &format!("        State::S{i}(role) => "), "        ", ",", label, payload, *next),
                    _ => {
                        // A choice of this role: one `if` per alternative, the last the `else`.
                        let _ = writeln!(s, "        State::S{i}(role) => {{");
                        let last = branches.len().saturating_sub(1);
                        for (b, (label, payload, next)) in branches.iter().enumerate() {
                            let open = if b == 0 {
                                format!("            if todo!(\"whether to send {label}\") {{")
                            } else if b < last {
                                format!("            }} else if todo!(\"whether to send {label}\") {{")
                            } else {
                                "            } else {".to_string()
                            };
                            let _ = writeln!(s, "{open}");
                            send(&mut s, "                ", "                ", "", label, payload, *next);
                        }
                        let _ = writeln!(s, "            }}");
                        let _ = writeln!(s, "        }}");
                    }
                }
            }
            Kind::Recv { from, branches } => {
                let labels: Vec<&str> = branches.iter().map(|(l, _, _)| l.as_str()).collect();
                let _ = writeln!(s, "        // S{i}: receive {} from `{from}`.", labels.join(" or "));
                let _ = writeln!(s, "        State::S{i}(role) => match role.try_recv({}) {{", gate_args.join(", "));
                // The arms of the messages: of the `Result` itself, or, where
                // the receive opens a gate, of the outcome once it has.
                let (pad, wrap) = if opens { ("                    ", ("", "")) } else { ("            ", ("Ok(", ")")) };
                if opens {
                    let _ = writeln!(s, "            Ok(received) => {{");
                    let _ = writeln!(s, "                // The completed receive opens the gate of this Job's operations declared `after = {name}`.");
                    let _ = writeln!(s, "                cx.{name}_gate.open(&received);");
                    let _ = writeln!(s, "                match received {{");
                }
                for (label, payload, next) in branches {
                    match payload {
                        Some(p) => {
                            let _ = writeln!(s, "{pad}{}RecvS{i}::{label}(m, s{next}){} => {{", wrap.0, wrap.1);
                            let _ = writeln!(s, "{pad}    todo!(\"use the {label} payload `m`: {p}\");");
                        }
                        None => {
                            let _ = writeln!(s, "{pad}{}RecvS{i}::{label}(s{next}){} => {{", wrap.0, wrap.1);
                            let _ = writeln!(s, "{pad}    todo!(\"react to {label}\");");
                        }
                    }
                    let _ = writeln!(s, "{pad}    State::S{next}(s{next})");
                    let _ = writeln!(s, "{pad}}}");
                }
                if opens {
                    let _ = writeln!(s, "                }}");
                    let _ = writeln!(s, "            }}");
                }
                let _ = writeln!(s, "            // Nothing has arrived yet: look again in the next Job.");
                let _ = writeln!(s, "            Err(NotReceived::Empty(s{i})) => State::S{i}(s{i}),");
                let _ = writeln!(s, "            // A label this state does not expect (a reported Protocol violation).");
                let _ = writeln!(s, "            Err(NotReceived::Unexpected(s{i})) => State::S{i}(s{i}),");
                let _ = writeln!(s, "            // The role's validation function rejected the payload (the sender's reported fault).");
                let _ = writeln!(s, "            Err(NotReceived::Rejected(s{i})) => State::S{i}(s{i}),");
                let _ = writeln!(s, "            // The peer's Partition stopped or restarted: both roles start over.");
                let _ = writeln!(s, "            Err(NotReceived::PeerChanged(s0)) => State::S0(s0),");
                let _ = writeln!(s, "        }},");
            }
        }
    }
    let _ = writeln!(s, "        // Only while a Job holds the state; a Job never finds its own slot empty.");
    let _ = writeln!(s, "        State::Empty => State::Empty,");
    let _ = writeln!(s, "    }};");
    let _ = writeln!(s, "    cx.{name}.store(next);");
    let _ = writeln!(s, "}}");
    s
}
