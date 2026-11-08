//! R38.4 (task 21.3): an operation the role's state does not permit is a
//! type error. `a` in S0 may send `Req` or `Stop`; `send_rep` belongs to
//! `b`'s S1 and `try_recv` to `a`'s S1: no such methods on `A<S0>`, E0599.
#![no_std]
#![forbid(unsafe_code)]

api_decl::rsk_partition_part0!();

pub fn alpha(cx: rsk_ctx::Alpha) {
    use rsk_roles::ping_a::State;
    let next = match cx.ping.take(State::Empty) {
        State::S0(a) => match a.try_recv() {
            Ok(_) => State::Empty,
            Err(a0) => State::S0(a0),
        },
        other => other,
    };
    cx.ping.store(next);
}

pub fn beta(cx: rsk_ctx::Beta) {
    cx.ping.store(rsk_roles::ping_b::State::Empty);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}
