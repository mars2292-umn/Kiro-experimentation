//! Bodies that follow every rule: this crate compiles.
#![no_std]
#![forbid(unsafe_code)]

use rsk::Mutex;

api_decl::rsk_partition_part0!();

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    // Task 21.3: the `a` role of `Ping`: a request in S0, the reply in S1.
    {
        use rsk_roles::ping_a::{RecvS1, State};
        let next = match cx.ping.take(State::Empty) {
            State::S0(a) => match a.send_req(1) {
                Ok(a1) => State::S1(a1),
                Err((a0, _)) => State::S0(a0),
            },
            State::S1(a) => match a.try_recv() {
                Ok(RecvS1::Rep(_, a0)) => State::S0(a0),
                Err(a1) => State::S1(a1),
            },
            other => other,
        };
        cx.ping.store(next);
    }
    // Through the `Mutex` trait and through the inherent method.
    Mutex::lock(&mut cx.state, |s| *s += 1);
    let n = cx.other.lock(|o| {
        *o += 1;
        *o
    });
    if n > 3 {
        let _ = cx.tick.raise();
    }
}

pub fn beta(mut cx: rsk_ctx::Beta) {
    cx.state.lock(|s| *s = 0);
    // Task 21.3: the `b` role of `Ping`, one step per Job, stored back.
    use rsk_roles::ping_b::{RecvS0, State};
    let next = match cx.ping.take(State::Empty) {
        State::S0(b) => match b.try_recv() {
            Ok(RecvS0::Req(v, b1)) => match b1.send_rep(v) {
                Ok(b0) => State::S0(b0),
                Err((b1, _)) => State::S1(b1),
            },
            Ok(RecvS0::Stop(e)) => e.finish(),
            Err(b0) => State::S0(b0),
        },
        other => other,
    };
    cx.ping.store(next);
}

pub fn gamma(_cx: rsk_ctx::Gamma) {}

/// PR-16: a Loop_Bound_Annotation on a bounded labelled loop compiles.
#[rsk::bounds('scan = 4 * 2)]
pub fn scan(mut cx: rsk_ctx::Alpha) {
    let mut i = 0;
    'scan: while i < 8 {
        i += 1;
    }
    cx.state.lock(|s| *s += i);
}
