//! The bounded exhaustive enumeration for the general theorem of R41.3
//! (task 21.6 follow-up, 2026-11-08): every global type over a small
//! grammar is built directly as an `ast::Global` (not parsed), rendered,
//! and handed to `check_protocol` at capacity 2; every result is tallied
//! by its rule, and a refusal naming R41.3 is a Protocol the structural
//! checks (R38.5), the projection (R38.3), and the exploration (R41.1 and
//! the explorer's own deadlock, orphan, and unspecified-reception checks)
//! accept that the verified certificate refuses: a counterexample to the
//! conjecture that every structurally accepted Protocol is deadlock-free.
//! The random search (`tests/r41_3_search.rs`) samples a broader space;
//! this test covers a smaller one completely, so an absent counterexample
//! below the bounds is a statement about every type there, not about a
//! sample. Nothing is deduplicated: role renamings and label renamings
//! are enumerated and counted separately.
//!
//! # The grammar
//!
//! Roles are `a`, `b`, `c` (every ordered pair of distinct roles, 6, is a
//! channel), labels are `A`, `B`, `C`, every payload is absent (payloads
//! play no part in progress). A type of depth `d` is:
//!
//! - `end`, or `continue X` inside the body of the top-level `rec X`
//!   (depth 0; the checker rejects an unguarded `continue` under R38.5);
//! - `L(): r -> s; T` with `r != s`, `L` one of the three labels, `T` of
//!   depth `d - 1` (18 heads);
//! - `choice at r { L1(): r -> s; T1 } or { L2(): r -> s; T2 } [or ..]`
//!   with `r != s`, the branches' labels distinct and taken in every
//!   order (6 ordered pairs for 2 branches, 6 permutations for 3), each
//!   `Ti` of depth `d - 1` (the branch's first message is part of the
//!   choice, so a choice costs one level like a message);
//! - `rec X { T }` only at the top level, `T` the body.
//!
//! # The bounds (the constants below)
//!
//! Three tiers, each exhaustive within its bound and counted separately
//! (a type of a lower tier that also fits a higher one is counted in each;
//! `end` and the 18 single messages appear in tiers 1 and 2):
//!
//! 1. no recursion, depth `NOREC_DEPTH` = 2, choices of 2 branches:
//!    N(0) = 1, N(1) = 1 + 18 + 36 = 55, N(2) = 1 + 18·55 + 36·55² =
//!    109,891 types;
//! 2. no recursion, depth `WIDE_DEPTH` = 1, choices of 2 or 3 branches:
//!    1 + 18 + 36 + 36 = 91 types (the only tier with three-way choices;
//!    at depth 2 they would add 36·91³ ≈ 27 million types);
//! 3. a top-level `rec X { .. }` whose body has depth `REC_DEPTH` = 2 and
//!    choices of 2 branches, with `continue X` an extra terminator:
//!    M(0) = 2, M(1) = 2 + 36 + 36·4 = 182, M(2) = 2 + 18·182 + 36·182² =
//!    1,195,742 types.
//!
//! The counts are asserted, so a change of the constants shows up in the
//! documented totals. The whole run (1,305,724 types) takes about a
//! minute and a half in debug mode; the tallies per rule, and the R38.5
//! refusals by their reason (the explorer's own deadlock, orphan, and
//! unspecified-reception findings are reported under R38.5 too), are
//! printed with `--nocapture`.
#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::time::Instant;

use rsk_protocol::ast::{Global, Message};
use rsk_protocol::check_protocol;

/// The roles, in the Endpoint's declared order.
const ROLES: [&str; 3] = ["a", "b", "c"];
/// The label alphabet.
const LABELS: [&str; 3] = ["A", "B", "C"];
/// The channels' capacity handed to `check_protocol`.
const CAPACITY: u32 = 2;
/// Tier 1: the depth of the types without recursion (2-branch choices).
const NOREC_DEPTH: u32 = 2;
/// Tier 2: the depth of the types without recursion whose choices may
/// have 3 branches.
const WIDE_DEPTH: u32 = 1;
/// Tier 3: the depth of the body of the top-level `rec` (2-branch choices).
const REC_DEPTH: u32 = 2;
/// The name of the top-level recursion.
const REC: &str = "X";

/// The expected totals per tier (documented above).
const EXPECTED: [u64; 3] = [109_891, 91, 1_195_742];

fn msg(label: &str, from: &str, to: &str) -> Message {
    Message { label: label.to_string(), payload: None, from: from.to_string(), to: to.to_string() }
}

/// Every ordered pair of distinct roles.
fn pairs() -> Vec<(&'static str, &'static str)> {
    let mut out = Vec::new();
    for f in ROLES {
        for t in ROLES {
            if f != t {
                out.push((f, t));
            }
        }
    }
    out
}

/// The label sequences of `n` distinct labels, in every order.
fn label_seqs(n: usize) -> Vec<Vec<&'static str>> {
    fn go(n: usize, chosen: &mut Vec<&'static str>, out: &mut Vec<Vec<&'static str>>) {
        if chosen.len() == n {
            out.push(chosen.clone());
            return;
        }
        for l in LABELS {
            if !chosen.contains(&l) {
                chosen.push(l);
                go(n, chosen, out);
                chosen.pop();
            }
        }
    }
    let mut out = Vec::new();
    go(n, &mut Vec::new(), &mut out);
    out
}

/// The terminators: `end`, and `continue X` inside the rec body.
fn terminators(in_rec: bool) -> Vec<Global> {
    let mut out = vec![Global::End];
    if in_rec {
        out.push(Global::Continue(REC.to_string()));
    }
    out
}

/// Every type of depth exactly `depth` or less, materialized (used for the
/// continuations of the outermost level, which is streamed).
fn all(depth: u32, in_rec: bool, branch_counts: &[usize]) -> Vec<Global> {
    let mut out = Vec::new();
    for_each(depth, in_rec, branch_counts, &mut |g| out.push(g));
    out
}

/// Visits every type of depth `depth` or less once.
fn for_each(depth: u32, in_rec: bool, branch_counts: &[usize], f: &mut dyn FnMut(Global)) {
    for t in terminators(in_rec) {
        f(t);
    }
    if depth == 0 {
        return;
    }
    let subs = all(depth - 1, in_rec, branch_counts);
    for (from, to) in pairs() {
        for l in LABELS {
            for s in &subs {
                f(Global::Msg(msg(l, from, to), Box::new(s.clone())));
            }
        }
    }
    for (at, to) in pairs() {
        for &n in branch_counts {
            for labels in label_seqs(n) {
                // Every combination of continuations, one per branch.
                let mut idx = vec![0usize; n];
                loop {
                    let branches: Vec<Global> = (0..n).map(|i| Global::Msg(msg(labels[i], at, to), Box::new(subs[idx[i]].clone()))).collect();
                    f(Global::Choice { at: at.to_string(), branches });
                    // Advance the odometer.
                    let mut i = 0;
                    loop {
                        if i == n {
                            break;
                        }
                        idx[i] += 1;
                        if idx[i] < subs.len() {
                            break;
                        }
                        idx[i] = 0;
                        i += 1;
                    }
                    if i == n {
                        break;
                    }
                }
            }
        }
    }
}

#[derive(Default)]
struct Tally {
    total: u64,
    by_rule: BTreeMap<String, u64>,
    /// The R38.5 refusals by the reason's leading words.
    by_reason: BTreeMap<String, u64>,
    counterexamples: Vec<String>,
}

/// A refusal's message with every quoted name replaced by `_` and cut at
/// its first colon: the reason without the particular roles and labels.
fn reason(message: &str) -> String {
    let mut out = String::new();
    let mut quoted = false;
    for c in message.chars() {
        match c {
            '`' if !quoted => {
                quoted = true;
                out.push('_');
            }
            '`' => quoted = false,
            ':' if !quoted => break,
            _ if quoted => {}
            _ => out.push(c),
        }
    }
    out.trim().to_string()
}

impl Tally {
    fn check(&mut self, g: &Global) {
        self.total += 1;
        let used = g.roles();
        let roles: Vec<&str> = ROLES.iter().copied().filter(|r| used.iter().any(|x| x == r)).collect();
        if roles.len() < 2 {
            *self.by_rule.entry("<2 roles (not checked)".into()).or_insert(0) += 1;
            return;
        }
        let text = g.render();
        match check_protocol(&text, &roles, CAPACITY) {
            Ok(_) => *self.by_rule.entry(format!("accepted ({} roles)", roles.len())).or_insert(0) += 1,
            Err(e) => {
                *self.by_rule.entry(e.rule.to_string()).or_insert(0) += 1;
                if e.rule == "R38.5" {
                    *self.by_reason.entry(reason(&e.message)).or_insert(0) += 1;
                }
                if e.rule == "R41.3" {
                    self.counterexamples.push(format!("{text}\n    {e}"));
                }
            }
        }
    }
}

/// Every type below the bounds is checked; none is refused by the
/// certificate (R41.3).
#[test]
fn r41_3_every_type_below_the_bounds_is_certified_or_refused_before_the_certificate() {
    let start = Instant::now();
    let tiers: [(&str, u32, bool, &[usize]); 3] = [
        ("tier 1: no rec, depth 2, 2-branch choices", NOREC_DEPTH, false, &[2]),
        ("tier 2: no rec, depth 1, 2- or 3-branch choices", WIDE_DEPTH, false, &[2, 3]),
        ("tier 3: rec X { body }, body depth 2, 2-branch choices", REC_DEPTH, true, &[2]),
    ];
    let mut grand = Tally::default();
    for (i, (name, depth, in_rec, branch_counts)) in tiers.iter().enumerate() {
        let tier_start = Instant::now();
        let mut tally = Tally::default();
        for_each(*depth, *in_rec, branch_counts, &mut |g| {
            let g = if *in_rec { Global::Rec { name: REC.to_string(), body: Box::new(g) } } else { g };
            tally.check(&g);
        });
        eprintln!("{name}: {} types in {:.1?}", tally.total, tier_start.elapsed());
        for (k, v) in &tally.by_rule {
            eprintln!("  {k}: {v}");
        }
        for (k, v) in &tally.by_reason {
            eprintln!("    R38.5 {k}: {v}");
        }
        assert_eq!(tally.total, EXPECTED[i], "{name}: the documented count");
        grand.total += tally.total;
        for (k, v) in tally.by_rule {
            *grand.by_rule.entry(k).or_insert(0) += v;
        }
        for (k, v) in tally.by_reason {
            *grand.by_reason.entry(k).or_insert(0) += v;
        }
        grand.counterexamples.extend(tally.counterexamples);
    }
    eprintln!("all tiers: {} types in {:.1?}", grand.total, start.elapsed());
    for (k, v) in &grand.by_rule {
        eprintln!("  {k}: {v}");
    }
    for (k, v) in &grand.by_reason {
        eprintln!("    R38.5 {k}: {v}");
    }
    assert!(
        grand.counterexamples.is_empty(),
        "{} Protocol(s) accepted by structure, projection, and exploration but refused by the certificate:\n{}",
        grand.counterexamples.len(),
        grand.counterexamples.join("\n")
    );
}
