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

use rsk_protocol::{Branch, Local};

use crate::derive::{GenEndpoint, Generated};

/// One state of a role's projection.
#[derive(Clone, Debug)]
pub enum Kind {
    Send { to: String, branches: Vec<(String, Option<String>, usize)> },
    Recv { from: String, branches: Vec<(String, Option<String>, usize)> },
    End,
}

#[derive(Clone, Debug)]
pub struct RoleState {
    pub kind: Kind,
    /// The target of a `rec`: dropping a role value here is no violation (R41.2).
    pub rec_head: bool,
}

/// Numbers the states of a local type: a `rec` and its body share a state,
/// `continue` returns to it, every `send`, `receive`, and `end` is a state.
pub fn states(local: &Local) -> Vec<RoleState> {
    let mut out: Vec<Option<RoleState>> = Vec::new();
    let mut env: Vec<(String, usize)> = Vec::new();
    number(local, &mut env, &mut out, None);
    out.into_iter().map(|s| s.unwrap_or(RoleState { kind: Kind::End, rec_head: false })).collect()
}

fn number(l: &Local, env: &mut Vec<(String, usize)>, out: &mut Vec<Option<RoleState>>, slot: Option<usize>) -> usize {
    let take_slot = |out: &mut Vec<Option<RoleState>>, slot: Option<usize>| -> usize {
        match slot {
            Some(i) => i,
            None => {
                out.push(None);
                out.len() - 1
            }
        }
    };
    match l {
        Local::Rec { name, body } => {
            let idx = take_slot(out, slot);
            env.push((name.clone(), idx));
            number(body, env, out, Some(idx));
            env.pop();
            if let Some(s) = out[idx].as_mut() {
                s.rec_head = true;
            }
            idx
        }
        Local::Continue(name) => env.iter().rev().find(|(n, _)| n == name).map(|(_, i)| *i).unwrap_or(0),
        Local::End => {
            let idx = take_slot(out, slot);
            out[idx] = Some(RoleState { kind: Kind::End, rec_head: false });
            idx
        }
        Local::Send { to, branches } => {
            let idx = take_slot(out, slot);
            out[idx] = Some(RoleState { kind: Kind::Send { to: to.clone(), branches: Vec::new() }, rec_head: false });
            let bs = branch_states(branches, env, out);
            if let Some(RoleState { kind: Kind::Send { branches: b, .. }, .. }) = out[idx].as_mut() {
                *b = bs;
            }
            idx
        }
        Local::Recv { from, branches } => {
            let idx = take_slot(out, slot);
            out[idx] = Some(RoleState { kind: Kind::Recv { from: from.clone(), branches: Vec::new() }, rec_head: false });
            let bs = branch_states(branches, env, out);
            if let Some(RoleState { kind: Kind::Recv { branches: b, .. }, .. }) = out[idx].as_mut() {
                *b = bs;
            }
            idx
        }
    }
}

fn branch_states(branches: &[Branch], env: &mut Vec<(String, usize)>, out: &mut Vec<Option<RoleState>>) -> Vec<(String, Option<String>, usize)> {
    branches.iter().map(|b| (b.label.clone(), b.payload.clone(), number(&b.cont, env, out, None))).collect()
}

/// `ctl` -> `Ctl`.
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
    let sts = states(&local);
    let ty = type_name(role);
    let ri = e.roles.iter().position(|(r, _)| r == role).unwrap_or(0);
    // Task 21.6 (R41.3, A-P8): the Endpoints gating this role (`after`):
    // every operation takes their gates with the App_Prover precondition
    // that each is open (a receive on that Endpoint completed in this Job).
    let gates: Vec<String> = e.after.get(ri).cloned().unwrap_or_default();
    let gate_params: String = gates.iter().map(|x| format!(", gate_{x}: &::rsk::Gate")).collect();
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
    let _ = writeln!(s, "                    ::rsk::report_protocol_violation();");
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
                            let _ = writeln!(s, "            pub fn {fname}(self, m: {p}{gate_params}) -> Result<{ty}<S{next}>, NotSent<S{i}>> {{");
                            let _ = writeln!(s, "                let mut w = [0u32; W];");
                            let _ = writeln!(s, "                w[0] = LABEL_{};", label.to_uppercase());
                            let _ = writeln!(s, "                let p = ::rsk::words::message_to_words::<{p}, PW>(&m);");
                            let _ = writeln!(s, "                let mut i: usize = 0;");
                            let _ = writeln!(s, "                #[cfg_attr(creusot, ::rsk::creusot_std::macros::invariant(i@ <= PW@))]");
                            let _ = writeln!(s, "                #[cfg_attr(creusot, ::rsk::creusot_std::macros::variant(PW@ - i@))]");
                            let _ = writeln!(s, "                while i < PW {{ w[i + 1] = p.0[i]; i += 1; }}");
                        }
                        None => {
                            let _ = writeln!(s, "            /// Sends `{label}()` to `{to}` and moves to S{next}; `Err` gives the role back with the reason. Never waits (R40.4).");
                            gate_requires(&mut s);
                            let _ = writeln!(s, "            pub fn {fname}(self{gate_params}) -> Result<{ty}<S{next}>, NotSent<S{i}>> {{");
                            let _ = writeln!(s, "                let mut w = [0u32; W];");
                            let _ = writeln!(s, "                w[0] = LABEL_{};", label.to_uppercase());
                        }
                    }
                    let _ = writeln!(s, "                match raw().send_to(PEER_{}, &w) {{", to.to_uppercase());
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
                let _ = writeln!(s, "            pub fn try_recv(self{gate_params}) -> Result<RecvS{i}, NotReceived<S{i}>> {{");
                let _ = writeln!(s, "                match raw().recv_from(PEER_{}) {{", from.to_uppercase());
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
                let _ = writeln!(s, "                            _ => {{ ::rsk::report_protocol_violation(); Err(NotReceived::Unexpected(self)) }}");
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
