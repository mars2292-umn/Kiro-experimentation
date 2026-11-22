//! The Rust code blocks of `docs/tutorial.md` compile.
//!
//! Every ```` ```rust ```` block of the tutorial is preceded by a comment
//! line `<!-- compiled by xtask/tests/tutorial.rs: <id> -->`. This test
//! takes the blocks out of the document by those identifiers and builds
//! the applications the tutorial's steps describe from them, as scratch
//! workspaces (see `scratch/mod.rs`), and runs `cargo check` on each. A
//! declaration block is compiled by the `rsk::app!` macro, which runs the
//! Generator on it, so a declaration the Generator rejects fails here too.
//!
//! Many blocks are fragments (the lines of a Partition, one Task body, a
//! few statements). What the tutorial leaves to the reader in prose, this
//! file supplies and marks as `GLUE`: the manifests, the crate attributes,
//! a body the text only describes. Nothing of a block is altered.
//!
//! | Step | Blocks | Application built |
//! |---|---|---|
//! | 1 | `ex1-decl`, `ex1-part`, `ex1-system` | the three crates, each block a whole file |
//! | 2 | `ex2-tick` | Step 1 with the body replaced |
//! | 3 | `ex3-decl`, `ex3-halt` | Step 1 with the second Task and the safe state that reads the event log |
//! | 4 | `ex4-decl`, `ex4-bodies` | the fixture's Partition and bodies |
//! | 5a | `ex5a-decl`, `ex5a-bodies` | the interrupt-released Task |
//! | 5b | `ex5b-decl`, `ex5b-bodies` | two Partitions and the plain Endpoint |
//! | 5c | `ex5c-decl`, `ex5c-alpha` | Step 4's Partition with the Protocol-governed Endpoint |
//! | 5c | `skeleton`, `skeleton-filled` | the same, the role played by the skeleton and by its filled-in copy; `skeleton` is also compared with what the Generator writes |
//!
//! A block without a marker, a marker this file does not use, and a
//! diagnostic of any kind in a compiled block each fail the test.
#![forbid(unsafe_code)]

mod scratch;

use std::collections::BTreeMap;
use std::fs;

use scratch::{manifest, member_dep, repo_dep, root, Scratch};

const MARKER: &str = "<!-- compiled by xtask/tests/tutorial.rs: ";

/// The identifiers this file compiles, in the document's order.
const BLOCKS: [&str; 16] = [
    "ex1-decl", "ex1-part", "ex1-system", "ex2-tick", "ex3-decl", "ex3-halt", "ex4-decl", "ex4-bodies", "ex5a-decl", "ex5a-bodies", "ex5b-decl", "ex5b-bodies", "ex5c-decl", "ex5c-alpha", "skeleton",
    "skeleton-filled",
];

/// The tutorial's Rust blocks by identifier. Panics on a block without its
/// marker on the line before the fence, and on an identifier used twice.
fn blocks() -> BTreeMap<String, String> {
    let text = fs::read_to_string(root().join("docs/tutorial.md")).expect("the tutorial");
    let lines: Vec<&str> = text.lines().collect();
    let mut out = BTreeMap::new();
    let mut order = Vec::new();
    let mut i = 0;
    while i < lines.len() {
        if lines[i].trim_end() == "```rust" {
            let id = i
                .checked_sub(1)
                .and_then(|p| lines[p].strip_prefix(MARKER))
                .and_then(|rest| rest.strip_suffix(" -->"))
                .unwrap_or_else(|| panic!("docs/tutorial.md:{}: a Rust block without `{MARKER}<id> -->` on the line before it", i + 1));
            let end = (i + 1..lines.len()).find(|&j| lines[j].trim_end() == "```").unwrap_or_else(|| panic!("docs/tutorial.md:{}: an open block", i + 1));
            let body: String = lines[i + 1..end].iter().map(|l| format!("{l}\n")).collect();
            assert!(out.insert(id.to_string(), body).is_none(), "docs/tutorial.md:{}: identifier `{id}` twice", i + 1);
            order.push(id.to_string());
            i = end;
        }
        i += 1;
    }
    assert_eq!(order, BLOCKS, "the tutorial's Rust blocks and the ones this test compiles differ");
    out
}

/// `text` up to the first line that starts with `start` (after indentation).
fn before<'a>(text: &'a str, start: &str) -> &'a str {
    let mut at = 0;
    for line in text.split_inclusive('\n') {
        if line.trim_start().starts_with(start) {
            return &text[..at];
        }
        at += line.len();
    }
    panic!("no line starting with `{start}` in:\n{text}");
}

// GLUE: the two attributes every application crate starts with (Step 1).
const ATTRS: &str = "#![no_std]\n#![forbid(unsafe_code)]\n\n";

/// GLUE: a declaration crate whose block holds `items` under the header of
/// Step 1's declaration.
fn declaration(ex1_decl: &str, items: &str) -> String {
    let header = before(ex1_decl, "partition ");
    assert!(header.contains("rsk::app! {") && header.contains("safe_state = crate::halt"), "{header}");
    format!("{header}{items}}}\n")
}

fn rsk() -> String {
    repo_dep("rsk", "crates/rsk", "")
}

/// GLUE: the manifests. A declaration crate depends on `rsk`; a Partition
/// crate on its declaration and `rsk`; the system crate also on the Kernel
/// with the QEMU board (the tutorial's wiring step 3) and on the Partition
/// crate whose state its safe state reads.
fn decl_manifest(member: &str, lib: &str) -> String {
    manifest(member, lib, &rsk())
}

fn part_manifest(member: &str, lib: &str, decl_lib: &str, decl_member: &str) -> String {
    manifest(member, lib, &format!("{}{}", member_dep(decl_lib, decl_member), rsk()))
}

fn system_manifest(member: &str, decl_member: &str, part_member: &str) -> String {
    manifest(
        member,
        "tutorial_system",
        &format!("{}{}{}{}", member_dep("tutorial_decl", decl_member), member_dep("tutorial_part", part_member), rsk(), repo_dep("rsk-kernel", "crates/rsk-kernel", ", default-features = false, features = [\"board-qemu-mps2\"]")),
    )
}

#[test]
fn steps_1_to_3_the_tutorial_application() {
    let b = blocks();
    let mut ws = Scratch::new("tutorial-steps-1-3");

    // Step 1: each block is a whole file.
    ws.member("ex1-decl", &decl_manifest("ex1-decl", "tutorial_decl"), &b["ex1-decl"]);
    ws.member("ex1-part", &part_manifest("ex1-part", "tutorial_part", "tutorial_decl", "ex1-decl"), &b["ex1-part"]);
    ws.member("ex1-system", &system_manifest("ex1-system", "ex1-decl", "ex1-part"), &b["ex1-system"]);

    // Step 2: the body of `tick` replaced.
    let part_head = before(&b["ex1-part"], "pub fn tick(");
    ws.member("ex2-part", &part_manifest("ex2-part", "tutorial_part", "tutorial_decl", "ex1-decl"), &format!("{part_head}{}", b["ex2-tick"]));

    // Step 3: the block replaces the `state` and `task` lines of Step 1's
    // Partition; `tick` gets "a one-line count" and `slow` "the 3 ms loop
    // of Step 2" between its two prints.
    let ex1 = &b["ex1-decl"];
    let state_line = "        state tick.count: u32 = 0;\n";
    let task_line = "        task tick (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 1);\n";
    assert!(ex1.contains(&format!("{state_line}{task_line}")), "Step 1's `state` and `task` lines:\n{ex1}");
    ws.member("ex3-decl", &decl_manifest("ex3-decl", "tutorial_decl"), &ex1.replace(&format!("{state_line}{task_line}"), &b["ex3-decl"]));
    // GLUE: the two bodies the text describes.
    let slow = "pub fn tick(mut cx: rsk_ctx::Tick) {\n    cx.count.with(|c| *c = c.saturating_add(1));\n}\n\n#[allow(unused_labels)]\npub fn slow(mut cx: rsk_ctx::Slow) {\n    rsk::debug(&[b's']);\n    let start = rsk::now();\n    let mut i = 0u32;\n    'busy: while rsk::now().wrapping_sub(start) < rsk_target::ms(3) && i < 10_000_000 {\n        i += 1;\n    }\n    cx.count.with(|c| *c = c.saturating_add(1));\n    rsk::debug(&[b'S']);\n}\n";
    ws.member("ex3-part", &part_manifest("ex3-part", "tutorial_part", "tutorial_decl", "ex3-decl"), &format!("{part_head}{slow}"));
    // The safe state that walks the event log, in Step 1's system crate.
    let system_head = before(&b["ex1-system"], "pub fn halt()");
    ws.member("ex3-system", &system_manifest("ex3-system", "ex3-decl", "ex3-part"), &format!("{system_head}{}", b["ex3-halt"]));

    ws.check().assert_clean("the tutorial's Steps 1 to 3");
}

#[test]
fn steps_4_and_5_resources_interrupts_and_endpoints() {
    let b = blocks();
    let ex1 = &b["ex1-decl"];
    let mut ws = Scratch::new("tutorial-steps-4-5");

    // Step 4: the fixture's Partition under Step 1's header, and its bodies
    // (the block holds the manifest line and the three functions).
    ws.member("ex4-decl", &decl_manifest("ex4-decl", "api_decl"), &declaration(ex1, &b["ex4-decl"]));
    ws.member("ex4-part", &part_manifest("ex4-part", "api_ok", "api_decl", "ex4-decl"), &format!("{ATTRS}{}", b["ex4-bodies"]));

    // Step 5a: the demo's `part1` alone, and its two bodies.
    ws.member("ex5a-decl", &decl_manifest("ex5a-decl", "p1_demo_decl"), &declaration(ex1, &b["ex5a-decl"]));
    ws.member("ex5a-part", &part_manifest("ex5a-part", "p1_demo_part1", "p1_demo_decl", "ex5a-decl"), &format!("{ATTRS}p1_demo_decl::rsk_partition_part1!();\n\n{}", b["ex5a-bodies"]));

    // Step 5b: the Endpoint line between two Partitions.
    // GLUE: the Partitions with the Tasks and the state the fragments use.
    let partitions = "    partition part0 (level = C, crate = ex5b_part0, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),\n                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {\n        peripherals = [];\n        state control.count: u32 = 0;\n        state control.telemetry_full: u32 = 0;\n        task control (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 3);\n    }\n    partition part1 (level = C, crate = ex5b_part1, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),\n                     fault = STOP_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY, mit_threshold = 10) {\n        peripherals = [];\n        state log.telemetry_last: u32 = 0;\n        task log (periodic, period = 40.ms(), deadline = 40.ms(), budget = 6.ms(), priority = 2);\n    }\n";
    ws.member("ex5b-decl", &decl_manifest("ex5b-decl", "ex5b_decl"), &declaration(ex1, &format!("{partitions}{}", b["ex5b-decl"])));
    // The block is two fragments: the statements of `control`, then of `log`.
    let bodies = &b["ex5b-bodies"];
    let in_control = before(bodies, "// in log");
    let in_log = &bodies[in_control.len()..];
    assert!(in_control.trim_start().starts_with("// in control") && in_log.starts_with("// in log"), "{bodies}");
    ws.member("ex5b-part0", &part_manifest("ex5b-part0", "ex5b_part0", "ex5b_decl", "ex5b-decl"), &format!("{ATTRS}ex5b_decl::rsk_partition_part0!();\n\npub fn control(mut cx: rsk_ctx::Control) {{\n{in_control}}}\n"));
    ws.member("ex5b-part1", &part_manifest("ex5b-part1", "ex5b_part1", "ex5b_decl", "ex5b-decl"), &format!("{ATTRS}ex5b_decl::rsk_partition_part1!();\n\npub fn log(mut cx: rsk_ctx::Log) {{\n{in_log}}}\n"));

    // Step 5c: the Protocol and its Endpoint after Step 4's Partition.
    let ex5c_decl = declaration(ex1, &format!("{}{}", b["ex4-decl"], b["ex5c-decl"]));
    ws.member("ex5c-decl", &decl_manifest("ex5c-decl", "api_decl"), &ex5c_decl);
    // GLUE: the `b` role as the text describes it ("mirrors it"), and the
    // sporadic Task's empty body.
    let rest = "pub fn beta(cx: rsk_ctx::Beta) {\n    use rsk_roles::ping_b::{NotReceived, NotSent, RecvS0, State};\n    let next = match cx.ping.take(State::Empty) {\n        State::S0(b) => match b.try_recv() {\n            Ok(RecvS0::Req(v, b1)) => match b1.send_rep(v) {\n                Ok(b0) => State::S0(b0),\n                Err(NotSent::Full(b1)) | Err(NotSent::Refused(b1)) => State::S1(b1),\n                Err(NotSent::PeerChanged(b0)) => State::S0(b0),\n            },\n            Ok(RecvS0::Stop(e)) => e.finish(),\n            Err(NotReceived::Empty(b0)) | Err(NotReceived::Unexpected(b0)) | Err(NotReceived::Rejected(b0)) | Err(NotReceived::PeerChanged(b0)) => State::S0(b0),\n        },\n        other => other,\n    };\n    cx.ping.store(next);\n}\n\npub fn gamma(_cx: rsk_ctx::Gamma) {}\n";
    let head = format!("{ATTRS}api_decl::rsk_partition_part0!();\n\n");
    ws.member("ex5c-part", &part_manifest("ex5c-part", "api_ok", "api_decl", "ex5c-decl"), &format!("{head}{}\n{rest}", b["ex5c-alpha"]));

    // The skeleton of the `a` role: the block is what the Generator writes
    // for this declaration, from the `allow` on.
    let parsed = rsk_gen::syntax::parse_source(&ex5c_decl).expect("Step 5c's declaration parses");
    let outputs = rsk_gen::outputs(&parsed.decl, "rsk-system").unwrap_or_else(|e| panic!("Step 5c's declaration is accepted: {e:?}"));
    let generated = &outputs.skeletons.iter().find(|s| s.file_name() == "skeletons/ping_a.rs").expect("the skeleton of `ping`'s `a`").source;
    let from_allow = &generated[generated.find("#[allow(").expect("the skeleton's `allow`")..];
    assert_eq!(b["skeleton"], from_allow, "the tutorial's skeleton is not what the Generator writes for `ping_a`; copy the Generator's");
    // Both forms as the text says to use them: called from the Task body.
    // GLUE: the call.
    let call = "pub fn alpha(mut cx: rsk_ctx::Alpha) {\n    ping_a(&mut cx);\n}\n\n";
    for id in ["skeleton", "skeleton-filled"] {
        let member = format!("{id}-part");
        ws.member(&member, &part_manifest(&member, "api_ok", "api_decl", "ex5c-decl"), &format!("{head}{call}{}\n{rest}", b[id]));
    }
    // "The `allow` attribute was deleted with the last `todo!()`."
    assert!(!b["skeleton-filled"].contains("todo!") && !b["skeleton-filled"].contains("#[allow"), "{}", b["skeleton-filled"]);

    ws.check().assert_clean("the tutorial's Steps 4 and 5");
}
