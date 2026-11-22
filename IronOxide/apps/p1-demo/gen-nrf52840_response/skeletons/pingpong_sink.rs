// Skeleton for role `sink` of Endpoint `pingpong` under Protocol `PingPong`,
// played by Task `log` of Partition `part1`.
//
// A starting point written by rsk-gen. No build reads this file and rsk-gen
// rewrites it on every run: copy the function into the `part1` crate, edit
// the copy, and call it from the body of `log` as `pingpong_sink(&mut cx);`.
//
// The role's projection: rec Loop.ctl?{Req(u32).ctl!{Rep(u32).Loop}, Stop().end}
// Each Job performs one step: it takes the state from the slot, does what the
// state permits, and stores the state that results. A failed operation hands
// the state back, so the next Job tries again; after `PeerChanged` (the other
// Partition stopped or restarted) the role is back in its initial state S0.
//
// Replace every `todo!()`; then delete the `allow` attribute below.
#[allow(unreachable_code, unused_variables, clippy::diverging_sub_expression)]
pub fn pingpong_sink(cx: &mut rsk_ctx::Log) {
    use rsk_roles::pingpong_sink::{NotReceived, NotSent, RecvS0, State};
    let next = match cx.pingpong.take(State::Empty) {
        // S0: receive Req or Stop from `ctl`.
        State::S0(role) => match role.try_recv() {
            Ok(RecvS0::Req(m, s1)) => {
                todo!("use the Req payload `m`: u32");
                State::S1(s1)
            }
            Ok(RecvS0::Stop(s2)) => {
                todo!("react to Stop");
                State::S2(s2)
            }
            // Nothing has arrived yet: look again in the next Job.
            Err(NotReceived::Empty(s0)) => State::S0(s0),
            // A label this state does not expect (a reported Protocol violation).
            Err(NotReceived::Unexpected(s0)) => State::S0(s0),
            // The role's validation function rejected the payload (the sender's reported fault).
            Err(NotReceived::Rejected(s0)) => State::S0(s0),
            // The peer's Partition stopped or restarted: both roles start over.
            Err(NotReceived::PeerChanged(s0)) => State::S0(s0),
        },
        // S1: send Rep to `ctl`.
        State::S1(role) => match role.send_rep(todo!("the Rep payload: u32")) {
            Ok(s0) => State::S0(s0),
            // The channel is full (a reported violation of the Protocol's bound): try again.
            Err(NotSent::Full(s1)) => State::S1(s1),
            // The Kernel refused the send: try again.
            Err(NotSent::Refused(s1)) => State::S1(s1),
            // The peer's Partition stopped or restarted: both roles start over.
            Err(NotSent::PeerChanged(s0)) => State::S0(s0),
        },
        // S2: the Protocol has ended for this role.
        State::S2(role) => role.finish(),
        // Only while a Job holds the state; a Job never finds its own slot empty.
        State::Empty => State::Empty,
    };
    cx.pingpong.store(next);
}
