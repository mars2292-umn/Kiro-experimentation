//! R38.4 (task 21.3): a role value is consumed by its operation; using the
//! old state again is a use after move, E0382.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(cx: rsk_ctx::Alpha) {
    use rsk_roles::ping_a::{NotSent, State};
    let next = match cx.ping.take(State::Empty) {
        State::S0(a) => {
            let first = a.send_req(1);
            let second = a.send_req(2);
            match (first, second) {
                (Ok(a1), _) => State::S1(a1),
                (Err(NotSent::Full(a0)), _) | (Err(NotSent::Refused(a0)), _) | (Err(NotSent::PeerChanged(a0)), _) => State::S0(a0),
            }
        }
        other => other,
    };
    cx.ping.store(next);
}

pub fn beta(cx: rsk_ctx::Beta) {
    cx.ping.store(rsk_roles::ping_b::State::Empty);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
