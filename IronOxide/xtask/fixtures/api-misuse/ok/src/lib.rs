//! Bodies that follow every rule: this crate compiles.
#![no_std]
#![forbid(unsafe_code)]

use rsk::Mutex;

api_decl::rsk_partition_part0!();

/// A message type (task 21.4, R42.1): `#[repr(C)]`, every field `Plain`.
#[derive(Clone, Copy, rsk::Plain)]
#[repr(C)]
pub struct Sample {
    pub seq: u32,
    pub values: [i16; 4],
    pub scale: f32,
}

pub fn alpha(mut cx: rsk_ctx::Alpha) {
    // Task 21.3: the `a` role of `Ping`: a request in S0, the reply in S1.
    {
        use rsk_roles::ping_a::{RecvS1, State};
        use rsk_roles::ping_a::{NotReceived, NotSent};
        let next = match cx.ping.take(State::Empty) {
            State::S0(a) => match a.send_req(1) {
                Ok(a1) => State::S1(a1),
                Err(NotSent::Full(a0)) | Err(NotSent::Refused(a0)) | Err(NotSent::PeerChanged(a0)) => State::S0(a0),
            },
            State::S1(a) => match a.try_recv() {
                Ok(RecvS1::Rep(_, a0)) => State::S0(a0),
                Err(NotReceived::Empty(a1)) | Err(NotReceived::Unexpected(a1)) | Err(NotReceived::Rejected(a1)) => State::S1(a1),
                Err(NotReceived::PeerChanged(a0)) => State::S0(a0),
            },
            other => other,
        };
        cx.ping.store(next);
    }
    // Task 21.4 (R42.1): a derived `Plain` message type of `Plain` fields
    // satisfies the Endpoint payload bound.
    fn is_plain<T: rsk::Plain>() {}
    is_plain::<Sample>();
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
    use rsk_roles::ping_b::{NotReceived, NotSent, RecvS0, State};
    let next = match cx.ping.take(State::Empty) {
        State::S0(b) => match b.try_recv() {
            Ok(RecvS0::Req(v, b1)) => match b1.send_rep(v) {
                Ok(b0) => State::S0(b0),
                Err(NotSent::Full(b1)) | Err(NotSent::Refused(b1)) => State::S1(b1),
                Err(NotSent::PeerChanged(b0)) => State::S0(b0),
            },
            Ok(RecvS0::Stop(e)) => e.finish(),
            Err(NotReceived::Empty(b0)) | Err(NotReceived::Unexpected(b0)) | Err(NotReceived::Rejected(b0)) | Err(NotReceived::PeerChanged(b0)) => State::S0(b0),
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
