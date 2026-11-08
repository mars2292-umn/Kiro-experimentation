//! The counterexample search for the general theorem of R41.3 (task 21.6
//! follow-up, 2026-11-08): over a broad space of global types (up to four
//! roles, choices at any role, nested recursions with and without replies,
//! three-way choices), every Protocol the structural checks, the
//! projection, and the exploration accept must also pass the verified
//! certificate (`progress::certify`); a refusal naming R41.3 is a Protocol
//! accepted by the checker's other conditions that violates progress, the
//! counterexample the general theorem's hypotheses would have to exclude.
//! `RSK_SEARCH_CASES` sets the number of cases (default 40, a few seconds;
//! the explorer's state space grows with the capacity, so the search runs
//! at capacity 2 and depth 3); the per-rule counts are printed so a run's
//! coverage can be read.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::sync::Mutex;

use proptest::prelude::*;
use proptest::test_runner::{Config, TestRunner};
use rsk_protocol::ast::{Global, Message};
use rsk_protocol::check_protocol;

const ROLES: [&str; 4] = ["a", "b", "c", "d"];

fn global(depth: u32, recs: Vec<&'static str>) -> BoxedStrategy<Global> {
    let role = || prop::sample::select(ROLES.to_vec());
    let label = || prop::sample::select(vec!["A", "B", "C", "D", "E"]);
    let payload = || prop::option::of(prop::sample::select(vec!["u8", "u32"]));
    let msg = (role(), role(), label(), payload()).prop_filter("distinct roles", |(f, t, _, _)| f != t);
    if depth == 0 {
        let mut ends: Vec<BoxedStrategy<Global>> = vec![Just(Global::End).boxed()];
        for r in &recs {
            ends.push(Just(Global::Continue(r.to_string())).boxed());
        }
        return prop::strategy::Union::new(ends).boxed();
    }
    let recs2 = recs.clone();
    let recs3 = recs.clone();
    let names: Vec<&'static str> = ["X", "Y", "Z"].into_iter().filter(|n| !recs.contains(n)).collect();
    let rec_strategy: BoxedStrategy<Global> = if let Some(&name) = names.first() {
        let mut inner = recs.clone();
        inner.push(name);
        global(depth - 1, inner).prop_map(move |body| Global::Rec { name: name.to_string(), body: Box::new(body) }).boxed()
    } else {
        Just(Global::End).boxed()
    };
    prop_oneof![
        4 => (msg.clone(), global(depth - 1, recs2)).prop_map(|((f, t, l, p), rest)| {
            Global::Msg(Message { label: l.to_string(), payload: p.map(str::to_string), from: f.to_string(), to: t.to_string() }, Box::new(rest))
        }),
        2 => (role(), role(), 2usize..=3).prop_filter("distinct", |(a, b, _)| a != b).prop_flat_map(move |(at, to, n)| {
            let recs = recs3.clone();
            prop::collection::vec(global(depth - 1, recs), n).prop_map(move |bodies| Global::Choice {
                at: at.to_string(),
                branches: bodies.into_iter().enumerate().map(|(i, body)| Global::Msg(Message { label: ["L", "R", "M"][i].into(), payload: None, from: at.to_string(), to: to.to_string() }, Box::new(body))).collect(),
            })
        }),
        1 => rec_strategy,
    ]
    .boxed()
}

/// Every Protocol the checker accepts up to the certificate is certified:
/// a refusal naming R41.3 is reported as a counterexample with its text.
#[test]
fn r41_3_accepted_protocols_are_certified() {
    let cases: u32 = std::env::var("RSK_SEARCH_CASES").ok().and_then(|v| v.parse().ok()).unwrap_or(40);
    let counts: Mutex<BTreeMap<String, u64>> = Mutex::new(BTreeMap::new());
    let counterexamples: Mutex<Vec<String>> = Mutex::new(Vec::new());
    let mut runner = TestRunner::new(Config { cases, max_shrink_iters: 0, max_local_rejects: u32::MAX, max_global_rejects: u32::MAX, ..Config::default() });
    let result = runner.run(&global(3, vec![]), |g| {
        let text = g.render();
        let roles: Vec<&str> = ROLES.iter().copied().filter(|r| g.roles().iter().any(|x| x == r)).collect();
        if roles.len() < 2 {
            *counts.lock().unwrap().entry("<2 roles".into()).or_insert(0) += 1;
            return Ok(());
        }
        match check_protocol(&text, &roles, 2) {
            Ok(c) => {
                *counts.lock().unwrap().entry(format!("accepted ({} roles)", roles.len())).or_insert(0) += 1;
                let _ = c;
            }
            Err(e) => {
                *counts.lock().unwrap().entry(e.rule.to_string()).or_insert(0) += 1;
                if e.rule == "R41.3" {
                    counterexamples.lock().unwrap().push(format!("{text}\n    {e}"));
                }
            }
        }
        Ok(())
    });
    result.expect("the search runs");
    let counts = counts.lock().unwrap();
    eprintln!("R41.3 search over {cases} cases:");
    for (k, v) in counts.iter() {
        eprintln!("  {k}: {v}");
    }
    let ce = counterexamples.lock().unwrap();
    assert!(ce.is_empty(), "{} Protocol(s) accepted by structure, projection, and exploration but refused by the certificate:\n{}", ce.len(), ce.join("\n"));
}
