//! The Partition of the A-P8 fixture (task 21.6, R41.3; Verification_Plan
//! 5.15): `alpha` operates on `pong`'s gated role without the receive on
//! `ping` that opens the gate. The body compiles (the gate is a plain
//! argument) and runs; the App_Prover must fail its proof: `send_req`
//! requires the gate open, and `alpha_pre` says every gate is closed when
//! the Job starts.
#![no_std]
#![forbid(unsafe_code)]

gating_misuse_decl::rsk_partition_part0!();

/// Plays `pong` ungated: the obligation this fixture violates.
#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::alpha_pre(&cx)))]
pub fn alpha(mut cx: rsk_ctx::Alpha) {
    {
        use rsk_roles::ping_a::State;
        // `ping` is left where it was: no receive on it in this Job.
        let state = cx.ping.take(State::Empty);
        cx.ping.store(state);
    }
    {
        use rsk_roles::pong_a::{NotSent, State};
        let state = cx.pong.take(State::Empty);
        let next = match state {
            // Ungated: `cx.ping_gate` is closed here.
            State::S0(c) => match c.send_req(1, &cx.ping_gate) {
                Ok(c1) => State::S1(c1),
                Err(NotSent::Full(c0)) | Err(NotSent::Refused(c0)) | Err(NotSent::PeerChanged(c0)) => State::S0(c0),
            },
            other => other,
        };
        cx.pong.store(next);
    }
}

#[cfg_attr(creusot, rsk::creusot_std::macros::requires(rsk_spec::beta_pre(&cx)))]
pub fn beta(cx: rsk_ctx::Beta) {
    {
        use rsk_roles::ping_b::State;
        let state = cx.ping.take(State::Empty);
        cx.ping.store(state);
    }
    {
        use rsk_roles::pong_b::State;
        let state = cx.pong.take(State::Empty);
        cx.pong.store(state);
    }
}

pub fn safe_state() {}
