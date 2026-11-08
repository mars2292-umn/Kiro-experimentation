//! Property 20 (task 21.1; R38.3, R41.1), the Protocol_Checker's half: for
//! any accepted global Protocol, (a) the synchronous traces of the global
//! type are exactly the synchronous traces of the product of its
//! projections (the projection is faithful: the reference against which the
//! typestate API of task 21.3 is generated), (b) every interleaving within
//! the bound k is free of deadlock, orphan, and unspecified reception, and
//! the bound is tight (exploring with capacity k succeeds and with k - 1
//! fails under R41.1 when k > 1), and (c) the projection does not depend on
//! the order the roles are declared in.
//!
//! Rumpsteak's k-MC checker is not vendored (R58.1) and computes an
//! existential bound where this checker computes the universal one (see the
//! crate documentation), so the differential check of the design is against
//! the global type's own semantics.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;

use proptest::prelude::*;
use rsk_protocol::{check_protocol, explore, project, Global, Local, Message};

const ROLES: [&str; 3] = ["a", "b", "c"];

/// Random global types: sequences of messages with a reply in every loop
/// (so that a bound exists), choices informing one receiver first, and a
/// guarded recursion at most two deep.
fn global(depth: u32, in_rec: Option<&'static str>) -> BoxedStrategy<Global> {
    let role = || prop::sample::select(ROLES.to_vec());
    let label = || prop::sample::select(vec!["A", "B", "C", "D"]);
    let payload = || prop::option::of(prop::sample::select(vec!["u8", "u32", "[u8; 4]"]));
    let msg = (role(), role(), label(), payload()).prop_filter("distinct roles", |(f, t, _, _)| f != t);
    if depth == 0 {
        // A terminator: end, or continue inside a rec.
        return match in_rec {
            Some(name) => prop_oneof![Just(Global::End), Just(Global::Continue(name.to_string()))].boxed(),
            None => Just(Global::End).boxed(),
        };
    }
    let rec_name = in_rec;
    prop_oneof![
        3 => (msg.clone(), global(depth - 1, rec_name)).prop_map(|((f, t, l, p), rest)| {
            Global::Msg(Message { label: l.to_string(), payload: p.map(str::to_string), from: f.to_string(), to: t.to_string() }, Box::new(rest))
        }),
        1 => (role(), role()).prop_filter("distinct", |(a, b)| a != b).prop_flat_map(move |(at, to)| {
            (global(depth - 1, rec_name), global(depth - 1, rec_name)).prop_map(move |(r1, r2)| Global::Choice {
                at: at.to_string(),
                branches: vec![
                    Global::Msg(Message { label: "L".into(), payload: None, from: at.to_string(), to: to.to_string() }, Box::new(r1)),
                    Global::Msg(Message { label: "R".into(), payload: None, from: at.to_string(), to: to.to_string() }, Box::new(r2)),
                ],
            })
        }),
        1 => {
            // A guarded loop with a reply: X: a -> b; Y: b -> a; then the body.
            if in_rec.is_none() {
                global(depth - 1, Some("X")).prop_map(|body| Global::Rec {
                    name: "X".into(),
                    body: Box::new(Global::Msg(Message { label: "Req".into(), payload: Some("u8".into()), from: "a".into(), to: "b".into() }, Box::new(Global::Msg(Message { label: "Rep".into(), payload: Some("u8".into()), from: "b".into(), to: "a".into() }, Box::new(body))))),
                }).boxed()
            } else {
                Just(Global::End).boxed()
            }
        },
    ]
    .boxed()
}

/// The synchronous traces of a global type up to `depth` steps: a message
/// is one atomic step `from>to:label`; a choice branches; recursion unfolds.
fn global_traces(g: &Global, depth: usize, recs: &mut Vec<(String, Global)>, prefix: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
    out.insert(prefix.clone());
    if depth == 0 {
        return;
    }
    match g {
        Global::End => {}
        Global::Continue(n) => {
            if let Some((_, r)) = recs.iter().rev().find(|(m, _)| m == n).cloned() {
                global_traces(&r, depth, recs, prefix, out);
            }
        }
        Global::Rec { name, body } => {
            recs.push((name.clone(), g.clone()));
            global_traces(body, depth, recs, prefix, out);
            recs.pop();
        }
        Global::Msg(m, rest) => {
            prefix.push(format!("{}>{}:{}", m.from, m.to, m.label));
            global_traces(rest, depth - 1, recs, prefix, out);
            prefix.pop();
        }
        Global::Choice { branches, .. } => {
            for b in branches {
                global_traces(b, depth, recs, prefix, out);
            }
        }
    }
}

/// The synchronous traces of the product of local types: a step is a send
/// matched by the receiver's matching receive.
fn product_traces(locals: &[(String, Local)], depth: usize) -> BTreeSet<Vec<String>> {
    fn unfold(l: &Local, recs: &mut Vec<(String, Local)>) -> Local {
        match l {
            Local::Rec { name, body } => {
                recs.push((name.clone(), l.clone()));
                unfold(body, recs)
            }
            Local::Continue(n) => match recs.iter().rev().find(|(m, _)| m == n).cloned() {
                Some((_, r)) => unfold(&r, recs),
                None => Local::End,
            },
            other => other.clone(),
        }
    }
    fn go(states: &[(String, Local, Vec<(String, Local)>)], depth: usize, prefix: &mut Vec<String>, out: &mut BTreeSet<Vec<String>>) {
        out.insert(prefix.clone());
        if depth == 0 {
            return;
        }
        for (i, (name, l, recs)) in states.iter().enumerate() {
            let mut r1 = recs.clone();
            if let Local::Send { to, branches } = unfold(l, &mut r1) {
                let Some(j) = states.iter().position(|(n, _, _)| n == &to) else { continue };
                let mut r2 = states[j].2.clone();
                if let Local::Recv { from, branches: rb } = unfold(&states[j].1, &mut r2) {
                    if from != *name {
                        continue;
                    }
                    for b in &branches {
                        if let Some(rbr) = rb.iter().find(|x| x.label == b.label) {
                            let mut ns = states.to_vec();
                            ns[i] = (name.clone(), b.cont.clone(), r1.clone());
                            ns[j] = (to.clone(), rbr.cont.clone(), r2.clone());
                            prefix.push(format!("{name}>{to}:{}", b.label));
                            go(&ns, depth - 1, prefix, out);
                            prefix.pop();
                        }
                    }
                }
            }
        }
    }
    let states: Vec<(String, Local, Vec<(String, Local)>)> = locals.iter().map(|(n, l)| (n.clone(), l.clone(), Vec::new())).collect();
    let mut out = BTreeSet::new();
    go(&states, depth, &mut Vec::new(), &mut out);
    out
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(300))]

    #[test]
    fn accepted_protocols_project_faithfully_and_are_bounded_and_safe(g in global(3, None)) {
        let text = g.render();
        let roles: Vec<&str> = ROLES.iter().copied().filter(|r| g.roles().iter().any(|x| x == r)).collect();
        if roles.len() < 2 {
            return Ok(());
        }
        // Many generated types are rejected for a named reason (an
        // unprojectable choice, an unbounded stream); the property is about
        // the accepted ones, with the rejection's rule checked.
        let checked = match check_protocol(&text, &roles, 8) {
            Ok(c) => c,
            Err(e) => {
                prop_assert!(["R38.2", "R38.3", "R38.5", "R41.1"].contains(&e.rule), "{e}");
                return Ok(());
            }
        };
        // (a) The projection is faithful: equal synchronous trace sets.
        let mut gt = BTreeSet::new();
        global_traces(&checked.global, 6, &mut Vec::new(), &mut Vec::new(), &mut gt);
        let pt = product_traces(&checked.locals, 6);
        prop_assert_eq!(&gt, &pt, "global {} vs projections {:?}", text, checked.locals.iter().map(|(r, l)| format!("{r}: {}", l.render())).collect::<Vec<_>>());
        // (b) The bound is tight and the system is safe at it.
        let k = checked.k.max(1);
        prop_assert!(explore(&checked.locals, k).is_ok());
        if k > 1 {
            let e = explore(&checked.locals, k - 1).unwrap_err();
            prop_assert_eq!(e.rule, "R41.1");
        }
        // (c) The projection does not depend on the roles' order.
        let mut rev = roles.clone();
        rev.reverse();
        let again = check_protocol(&text, &rev, 8).unwrap();
        for (r, l) in &checked.locals {
            let other = again.locals.iter().find(|(x, _)| x == r).map(|(_, l)| l.clone());
            prop_assert_eq!(Some(l.clone()), other);
        }
        prop_assert_eq!(checked.k, again.k);
        // The direct projection agrees with the checked one.
        for r in &roles {
            prop_assert_eq!(project(&checked.global, r).unwrap(), checked.locals.iter().find(|(x, _)| x == r).unwrap().1.clone());
        }
    }
}
