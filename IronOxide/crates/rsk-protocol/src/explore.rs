//! The bound k (R41.1) and the safety of every interleaving: the product
//! of the roles' local types over asynchronous channels, explored
//! exhaustively with every send enabled whenever its local type allows it
//! and the channels bounded by the declared capacity.

use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ast::{Branch, Local};
use crate::ProtocolError;

/// One directed channel's bound.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Channel {
    pub from: String,
    pub to: String,
    /// The largest number of messages in transit over every interleaving.
    pub k: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Exploration {
    pub channels: Vec<Channel>,
    /// Configurations visited.
    pub configurations: usize,
}

/// The exploration gives up beyond this many configurations (a Protocol
/// that large is rejected as unanalysable rather than trusted).
pub const MAX_CONFIGURATIONS: usize = 200_000;

/// A role's state as the local type with every `continue` replaced by its
/// `rec` (so that the state is self-contained): computed once per local
/// type by substitution.
fn close(local: &Local, env: &BTreeMap<String, Local>) -> Local {
    match local {
        Local::End => Local::End,
        Local::Continue(n) => env.get(n).cloned().unwrap_or(Local::End),
        Local::Rec { name, body } => {
            // The rec's own closure refers to itself: substitute lazily by
            // keeping the rec as the binding and closing its body with the
            // rec itself in the environment.
            let mut env2 = env.clone();
            env2.insert(name.clone(), local.clone());
            let closed_body = close(body, &env2);
            Local::Rec { name: name.clone(), body: Box::new(closed_body) }
        }
        Local::Send { to, branches } => Local::Send { to: to.clone(), branches: branches.iter().map(|b| Branch { label: b.label.clone(), payload: b.payload.clone(), cont: close(&b.cont, env) }).collect() },
        Local::Recv { from, branches } => Local::Recv { from: from.clone(), branches: branches.iter().map(|b| Branch { label: b.label.clone(), payload: b.payload.clone(), cont: close(&b.cont, env) }).collect() },
    }
}

/// The head of a state: `rec`s unfolded one level (a `rec` whose body is
/// closed over itself unfolds by substituting the rec for its continues).
fn head(state: &Local) -> Local {
    let mut s = state.clone();
    let mut guard = 0;
    while let Local::Rec { name, body } = s {
        let mut env = BTreeMap::new();
        env.insert(name.clone(), Local::Rec { name: name.clone(), body: body.clone() });
        s = close(&body, &env);
        guard += 1;
        if guard > 64 {
            return Local::End;
        }
    }
    s
}

type Config = (Vec<Local>, Vec<VecDeque<String>>);

/// Explores every interleaving of `locals` (role name, local type) with
/// channels of `capacity` messages. Errors name the first fault found.
pub fn explore(locals: &[(String, Local)], capacity: u32) -> Result<Exploration, ProtocolError> {
    let n = locals.len();
    let names: Vec<&str> = locals.iter().map(|(r, _)| r.as_str()).collect();
    let index = |r: &str| names.iter().position(|x| *x == r);
    let closed: Vec<Local> = locals.iter().map(|(_, l)| close(l, &BTreeMap::new())).collect();
    let mut channel_k: BTreeMap<(usize, usize), u32> = BTreeMap::new();
    let initial: Config = (closed.iter().map(head).collect(), vec![VecDeque::new(); n * n]);
    let mut seen: BTreeSet<Config> = BTreeSet::new();
    let mut queue: VecDeque<Config> = VecDeque::new();
    seen.insert(initial.clone());
    queue.push_back(initial);
    while let Some((states, chans)) = queue.pop_front() {
        if seen.len() > MAX_CONFIGURATIONS {
            return Err(ProtocolError { rule: "R38.5", message: format!("the Protocol's interleavings exceed {MAX_CONFIGURATIONS} configurations within the declared capacity; it is not analysed") });
        }
        let mut moved = false;
        let all_end = states.iter().all(|s| matches!(s, Local::End));
        for (i, s) in states.iter().enumerate() {
            match s {
                Local::End => {}
                Local::Send { to, branches } => {
                    let Some(j) = index(to) else { continue };
                    let ch = i * n + j;
                    if chans[ch].len() as u32 >= capacity {
                        return Err(ProtocolError {
                            rule: "R41.1",
                            message: format!("`{}` can send `{}` to `{}` with {capacity} message(s) already in transit on that channel: the Protocol's bound exceeds the declared capacity {capacity}", names[i], branches.iter().map(|b| b.label.as_str()).collect::<Vec<_>>().join("/"), to),
                        });
                    }
                    for b in branches {
                        let mut ns = states.clone();
                        let mut nc = chans.clone();
                        nc[ch].push_back(b.label.clone());
                        let len = nc[ch].len() as u32;
                        let e = channel_k.entry((i, j)).or_insert(0);
                        if len > *e {
                            *e = len;
                        }
                        ns[i] = head(&b.cont);
                        moved = true;
                        if seen.insert((ns.clone(), nc.clone())) {
                            queue.push_back((ns, nc));
                        }
                    }
                }
                Local::Recv { from, branches } => {
                    let Some(j) = index(from) else { continue };
                    let ch = j * n + i;
                    if let Some(label) = chans[ch].front() {
                        match branches.iter().find(|b| &b.label == label) {
                            Some(b) => {
                                let mut ns = states.clone();
                                let mut nc = chans.clone();
                                nc[ch].pop_front();
                                ns[i] = head(&b.cont);
                                moved = true;
                                if seen.insert((ns.clone(), nc.clone())) {
                                    queue.push_back((ns, nc));
                                }
                            }
                            None => {
                                return Err(ProtocolError {
                                    rule: "R38.5",
                                    message: format!("unspecified reception: `{}` expects {} from `{from}` and finds `{label}` first in the channel", names[i], branches.iter().map(|b| format!("`{}`", b.label)).collect::<Vec<_>>().join(" or ")),
                                });
                            }
                        }
                    }
                }
                Local::Rec { .. } | Local::Continue(_) => {}
            }
        }
        if !moved {
            if all_end {
                if let Some((ch, q)) = chans.iter().enumerate().find(|(_, q)| !q.is_empty()) {
                    return Err(ProtocolError {
                        rule: "R38.5",
                        message: format!("orphan message: `{}` from `{}` to `{}` is never received", q.front().cloned().unwrap_or_default(), names[ch / n], names[ch % n]),
                    });
                }
            } else {
                let waiting: Vec<String> = states.iter().enumerate().filter(|(_, s)| !matches!(s, Local::End)).map(|(i, s)| format!("`{}` at {}", names[i], s.render())).collect();
                return Err(ProtocolError { rule: "R38.5", message: format!("deadlock: no role can move with {}", waiting.join(", ")) });
            }
        }
    }
    let mut channels: Vec<Channel> = channel_k.into_iter().map(|((i, j), k)| Channel { from: names[i].to_string(), to: names[j].to_string(), k }).collect();
    channels.sort_by(|a, b| (a.from.as_str(), a.to.as_str()).cmp(&(b.from.as_str(), b.to.as_str())));
    Ok(Exploration { channels, configurations: seen.len() })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::check::project;
    use crate::parse::parse;

    fn locals(text: &str, roles: &[&str]) -> Vec<(String, Local)> {
        let g = parse(text).unwrap();
        roles.iter().map(|r| (r.to_string(), project(&g, r).unwrap())).collect()
    }

    #[test]
    fn the_request_response_loop_has_bound_one_each_way() {
        let ls = locals("rec L { choice at a { Q(u8): a -> b; R(u8): b -> a; continue L; } or { S(): a -> b; end; } }", &["a", "b"]);
        let e = explore(&ls, 4).unwrap();
        assert_eq!(e.channels, vec![Channel { from: "a".into(), to: "b".into(), k: 1 }, Channel { from: "b".into(), to: "a".into(), k: 1 }]);
    }

    #[test]
    fn a_stream_without_a_reply_exceeds_any_capacity() {
        let ls = locals("rec L { M(u8): a -> b; continue L; }", &["a", "b"]);
        let e = explore(&ls, 3).unwrap_err();
        assert_eq!(e.rule, "R41.1");
        assert!(e.message.contains("exceeds the declared capacity 3"), "{e}");
    }

    #[test]
    fn two_messages_before_the_reply_need_two_slots() {
        let ls = locals("rec L { A(u8): a -> b; B(u8): a -> b; C(u8): b -> a; continue L; }", &["a", "b"]);
        assert_eq!(explore(&ls, 2).unwrap().channels[0].k, 2);
        assert_eq!(explore(&ls, 1).unwrap_err().rule, "R41.1");
    }

    #[test]
    fn a_pipeline_of_three_roles_is_bounded_and_safe() {
        let ls = locals("rec L { M(u8): a -> b; N(u8): b -> c; K(u8): c -> a; continue L; }", &["a", "b", "c"]);
        let e = explore(&ls, 2).unwrap();
        assert!(e.channels.iter().all(|c| c.k == 1), "{:?}", e.channels);
    }
}
