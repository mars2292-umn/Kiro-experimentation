// Skeleton for role `ctl` of Endpoint `relay` under Protocol `PingPong`,
// played by Task `log` of Partition `part1`.
//
// A starting point written by rsk-gen. No build reads this file and rsk-gen
// rewrites it on every run: copy the function into the `part1` crate, edit
// the copy, and call it from the body of `log` as `relay_ctl(&mut cx);`.
//
// The role's projection: rec Loop.sink!{Req(u32).sink?{Rep(u32).Loop}, Stop().end}
// Each Job performs one step: it takes the state from the slot, does what the
// state permits, and stores the state that results. A failed operation hands
// the state back, so the next Job tries again; after `PeerChanged` (the other
// Partition stopped or restarted) the role is back in its initial state S0.
//
// The role is declared `after = pingpong`: its operations take the gate of each
// Endpoint named, and the App_Prover requires it open. Call this after the
// Job's receive on `pingpong`; a Job whose receive delivered
// nothing leaves the gate closed, and the function then returns at once.
//
// Replace every `todo!()`; then delete the `allow` attribute below.
#[allow(unreachable_code, unused_variables, clippy::diverging_sub_expression)]
pub fn relay_ctl(cx: &mut rsk_ctx::Log) {
    use rsk_roles::relay_ctl::{NotReceived, NotSent, RecvS1, State};
    // Nothing was received on `pingpong` in this Job: the role stays as it is.
    if !cx.pingpong_gate.is_open() {
        return;
    }
    let next = match cx.relay.take(State::Empty) {
        // S0: send Req or Stop to `sink`.
        State::S0(role) => {
            if todo!("whether to send Req") {
                match role.send_req(todo!("the Req payload: u32"), &cx.pingpong_gate) {
                    Ok(s1) => State::S1(s1),
                    // The channel is full (a reported violation of the Protocol's bound): try again.
                    Err(NotSent::Full(s0)) => State::S0(s0),
                    // The Kernel refused the send: try again.
                    Err(NotSent::Refused(s0)) => State::S0(s0),
                    // The peer's Partition stopped or restarted: both roles start over.
                    Err(NotSent::PeerChanged(s0)) => State::S0(s0),
                }
            } else {
                match role.send_stop(&cx.pingpong_gate) {
                    Ok(s2) => State::S2(s2),
                    // The channel is full (a reported violation of the Protocol's bound): try again.
                    Err(NotSent::Full(s0)) => State::S0(s0),
                    // The Kernel refused the send: try again.
                    Err(NotSent::Refused(s0)) => State::S0(s0),
                    // The peer's Partition stopped or restarted: both roles start over.
                    Err(NotSent::PeerChanged(s0)) => State::S0(s0),
                }
            }
        }
        // S1: receive Rep from `sink`.
        State::S1(role) => match role.try_recv(&cx.pingpong_gate) {
            Ok(RecvS1::Rep(m, s0)) => {
                todo!("use the Rep payload `m`: u32");
                State::S0(s0)
            }
            // Nothing has arrived yet: look again in the next Job.
            Err(NotReceived::Empty(s1)) => State::S1(s1),
            // A label this state does not expect (a reported Protocol violation).
            Err(NotReceived::Unexpected(s1)) => State::S1(s1),
            // The role's validation function rejected the payload (the sender's reported fault).
            Err(NotReceived::Rejected(s1)) => State::S1(s1),
            // The peer's Partition stopped or restarted: both roles start over.
            Err(NotReceived::PeerChanged(s0)) => State::S0(s0),
        },
        // S2: the Protocol has ended for this role.
        State::S2(role) => role.finish(),
        // Only while a Job holds the state; a Job never finds its own slot empty.
        State::Empty => State::Empty,
    };
    cx.relay.store(next);
}
