//! The starter Task bodies `rsk-gen` writes for the roles of
//! Protocol-governed Endpoints (`gen/skeletons/<endpoint>_<role>.rs`)
//! compile in the Partition crate of their Task, as written and filled in.
//!
//! Each test builds scratch Partition crates (see `scratch/mod.rs`) whose
//! source ends with the skeletons of the roles the Partition's Tasks play,
//! as a user would paste them, and runs `cargo check` on them.
//!
//! | Test | Declaration | What is compiled |
//! |---|---|---|
//! | `the_demo_skeletons_compile_as_written_and_filled_in` | the P1 demo's, and every variant of it with a Protocol-governed Endpoint | the files on disk under `gen*/skeletons/`, and each again with its `todo!()`s replaced and its `allow` removed |
//! | `the_shapes_the_demo_lacks_compile` | one written here: a choice of three, a role behind two gates, an array payload, a role that only sends | the same two forms |
//!
//! "Compile" means without a warning either: as written the skeleton
//! carries the one `allow` its `todo!()`s need, and its header promises
//! that the `allow` can go once they are replaced.
#![forbid(unsafe_code)]

mod scratch;

use std::fs;

use rsk_gen::roles::Skeleton;
use scratch::{manifest, member_dep, repo_dep, root, Scratch};

const ALLOW: &str = "#[allow(unreachable_code, unused_variables, clippy::diverging_sub_expression)]\n";

/// The skeleton as a user completes it: every `todo!()` replaced (a choice
/// by a condition, a payload to send by a value, a received payload by a
/// use of it), the `allow` deleted. The function gets the name `<name>_filled`
/// so that both forms fit in one crate.
fn filled(sk: &Skeleton) -> String {
    let name = format!("{}_{}", sk.endpoint, sk.role);
    let body = sk.source.find(ALLOW).map(|at| &sk.source[at + ALLOW.len()..]).unwrap_or_else(|| panic!("no `allow` in:\n{}", sk.source));
    let mut out = String::new();
    for line in body.lines() {
        let line = line.replace(&format!("pub fn {name}("), &format!("pub fn {name}_filled("));
        let trimmed = line.trim_start();
        let pad = &line[..line.len() - trimmed.len()];
        if trimmed.starts_with("todo!(\"use the ") {
            out.push_str(&format!("{pad}let _ = m;\n"));
        } else if trimmed.starts_with("todo!(\"react to ") {
            // Nothing to do on a message without a payload.
        } else {
            out.push_str(&replace_todos(&line));
            out.push('\n');
        }
    }
    assert!(!out.contains("todo!"), "a `todo!()` is left in:\n{out}");
    out
}

/// `if todo!("whether ..")` becomes a condition, `todo!("the X payload: T")` a value of `T`.
fn replace_todos(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(at) = rest.find("todo!(\"") {
        out.push_str(&rest[..at]);
        let after = &rest[at + 7..];
        let end = after.find("\")").unwrap_or_else(|| panic!("an open `todo!` in `{line}`"));
        let text = &after[..end];
        if text.starts_with("whether to send ") {
            out.push_str("rsk::now() % 2 == 0");
        } else if text.starts_with("the ") && text.contains(" payload: ") {
            out.push_str("Default::default()");
        } else {
            panic!("a `todo!()` this test does not know: `{text}`");
        }
        rest = &after[end + 2..];
    }
    out.push_str(rest);
    out
}

/// A Partition crate's source followed by both forms of each skeleton.
fn with_skeletons(lib: &str, skeletons: &[&Skeleton]) -> String {
    assert!(!skeletons.is_empty());
    let mut lib = lib.to_string();
    for sk in skeletons {
        lib.push('\n');
        lib.push_str(&sk.source);
        lib.push('\n');
        lib.push_str(&filled(sk));
    }
    lib
}

#[test]
fn the_demo_skeletons_compile_as_written_and_filled_in() {
    let root = root();
    let demo = root.join("apps/p1-demo");
    // The declaration and its variants: `src/lib.rs` writes `gen/`,
    // `src/<variant>.rs` writes `gen-<variant>/` (xtask/src/gen.rs).
    let mut sources: Vec<(Option<String>, std::path::PathBuf)> = vec![(None, demo.join("decl/src/lib.rs"))];
    let mut variants: Vec<String> = fs::read_dir(demo.join("decl/src"))
        .expect("the declaration crate's sources")
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_string_lossy().strip_suffix(".rs").map(str::to_string))
        .filter(|v| v != "lib")
        .collect();
    variants.sort();
    sources.extend(variants.into_iter().map(|v| (Some(v.clone()), demo.join("decl/src").join(format!("{v}.rs")))));

    let mut compiled = Vec::new();
    for (variant, source) in &sources {
        let label = variant.clone().unwrap_or_else(|| "default".to_string());
        let gen = demo.join(variant.as_ref().map(|v| format!("gen-{v}")).unwrap_or_else(|| "gen".to_string()));
        let text = fs::read_to_string(source).expect("the declaration");
        let parsed = rsk_gen::syntax::parse_source(&text).unwrap_or_else(|e| panic!("{label}: {e}"));
        let outputs = rsk_gen::outputs(&parsed.decl, "rsk-system").unwrap_or_else(|e| panic!("{label}: {e:?}"));
        if outputs.skeletons.is_empty() {
            continue;
        }
        // The files on disk are the ones compiled here.
        for sk in &outputs.skeletons {
            let path = gen.join(sk.file_name());
            let on_disk = fs::read_to_string(&path).unwrap_or_else(|e| panic!("{}: {e}; run `cargo xtask gen`", path.display()));
            assert_eq!(on_disk, sk.source, "{} is stale; run `cargo xtask gen`", path.display());
        }

        let mut partitions: Vec<String> = outputs.skeletons.iter().map(|s| s.partition.clone()).collect();
        partitions.sort();
        partitions.dedup();
        let mut ws = Scratch::new(&format!("skeletons-{label}"));
        for partition in &partitions {
            let from = demo.join(partition);
            // The Partition crate's own manifest, its relative paths made
            // absolute, and the declaration's variant selected with the
            // board feature the variant's name starts with.
            let decl_features = variant.as_ref().map(|v| format!(", features = [{v:?}]")).unwrap_or_default();
            let mut manifest = fs::read_to_string(from.join("Cargo.toml"))
                .expect("the Partition crate's manifest")
                .replace("p1-demo-decl = { path = \"../decl\" }", &format!("p1-demo-decl = {{ path = {:?}{decl_features} }}", demo.join("decl")))
                .replace("path = \"../../../crates/rsk\"", &format!("path = {:?}", root.join("crates/rsk")));
            assert!(!manifest.contains("path = \"../"), "a relative path is left in {partition}'s manifest:\n{manifest}");
            if variant.as_ref().is_some_and(|v| v.starts_with("nrf52840")) && manifest.contains("\nnrf52840 = [") {
                manifest = manifest.replace("[features]\n", "[features]\ndefault = [\"nrf52840\"]\n");
            }
            let lib = fs::read_to_string(from.join("src/lib.rs")).expect("the Partition crate's source");
            let own: Vec<&Skeleton> = outputs.skeletons.iter().filter(|s| &s.partition == partition).collect();
            ws.member(partition, &manifest, &with_skeletons(&lib, &own));
        }
        ws.check().assert_clean(&format!("the skeletons of the P1 demo ({label})"));
        compiled.push(label);
    }
    // The four configurations with a Protocol-governed Endpoint (the QEMU
    // set, its flood variant, the plain measurement and the response rig
    // configurations): a declaration that loses its Endpoints must not
    // quietly empty this test.
    assert_eq!(compiled, ["default", "nrf52840_measure", "nrf52840_response", "qemu_flood"]);
}

/// A declaration with what the demo's Protocols do not have: a choice of
/// three alternatives (an `else if` in the skeleton), a role declared
/// `after` two Endpoints (a guard over both gates), an array payload, a
/// message without a payload that a role must answer, and a role that
/// only ever sends.
const SHAPES: &str = r#"
#![no_std]
#![forbid(unsafe_code)]

rsk::app! {
    target = qemu_mps2_an386, profile = Ravenscar, operating_duration = 100.ms(),
    safe_state = crate::halt, log_capacity = 16;

    partition part0 (level = C, crate = shapes_part, code = 32.KiB(), ram = 32.KiB(), stack = 4.KiB(),
                     fault = RESTART_PARTITION, overrun = END_JOB, deadline_miss = RECORD_ONLY,
                     mit_threshold = 10) {
        peripherals = [];
        task alpha (periodic, period = 10.ms(), deadline = 10.ms(), budget = 1.ms(), priority = 3);
        task beta (periodic, period = 20.ms(), deadline = 20.ms(), budget = 1.ms(), priority = 2);
    }

    protocol Three {
        rec L {
            choice at a {
                Pair([u32; 2]): a -> b;
                Got(): b -> a;
                continue L;
            } or {
                Ask(): a -> b;
                Answer(u32): b -> a;
                continue L;
            } or {
                Stop(): a -> b;
                end;
            }
        }
    }
    protocol Once {
        Item(u32): a -> b;
        end;
    }
    endpoint first: protocol Three { a => part0.alpha, b => part0.beta ops = 2 } capacity = 1, payload = 12;
    endpoint second: protocol Once { a => part0.alpha, b => part0.beta } capacity = 1, payload = 8;
    endpoint third: protocol Three { a => part0.beta after = first after = second ops = 2, b => part0.alpha ops = 2 } capacity = 1, payload = 12;
}
"#;

#[test]
fn the_shapes_the_demo_lacks_compile() {
    let parsed = rsk_gen::syntax::parse_source(SHAPES).expect("the declaration parses");
    let outputs = rsk_gen::outputs(&parsed.decl, "rsk-system").unwrap_or_else(|e| panic!("the declaration is accepted: {e:?}"));
    let of = |name: &str| outputs.skeletons.iter().find(|s| s.file_name() == format!("skeletons/{name}.rs")).unwrap_or_else(|| panic!("no skeleton {name}"));
    assert_eq!(outputs.skeletons.len(), 6);

    // The declaration has the shapes it is here for.
    let chooser = &of("first_a").source;
    assert!(chooser.contains("if todo!(\"whether to send Pair\") {") && chooser.contains("} else if todo!(\"whether to send Ask\") {") && chooser.contains("} else {"), "{chooser}");
    assert!(chooser.contains("role.send_pair(todo!(\"the Pair payload: [u32 ; 2]\"))"), "{chooser}");
    let gated = &of("third_a").source;
    assert!(gated.contains("if !cx.first_gate.is_open() || !cx.second_gate.is_open() {"), "{gated}");
    assert!(gated.contains("role.send_ask(&cx.first_gate, &cx.second_gate)"), "{gated}");
    for gating in ["first_b", "second_b"] {
        let s = &of(gating).source;
        assert!(s.contains(&format!("cx.{}_gate.open(&received);", gating.trim_end_matches("_b"))), "{s}");
    }
    let sender = &of("second_a").source;
    assert!(!sender.contains("NotReceived") && !sender.contains("try_recv"), "{sender}");

    let mut ws = Scratch::new("skeletons-shapes");
    ws.member("shapes-decl", &manifest("shapes-decl", "shapes_decl", &repo_dep("rsk", "crates/rsk", "")), SHAPES);
    // The bodies call each of the Task's skeletons, the gating roles'
    // before the gated one, in both forms.
    let mut lib = String::from("#![no_std]\n#![forbid(unsafe_code)]\n\nshapes_decl::rsk_partition_part0!();\n\n");
    for (task, ctx) in [("alpha", "Alpha"), ("beta", "Beta")] {
        lib.push_str(&format!("pub fn {task}(mut cx: rsk_ctx::{ctx}) {{\n"));
        for sk in outputs.skeletons.iter().filter(|s| s.task == task) {
            lib.push_str(&format!("    {0}_{1}(&mut cx);\n    {0}_{1}_filled(&mut cx);\n", sk.endpoint, sk.role));
        }
        lib.push_str("}\n");
    }
    let all: Vec<&Skeleton> = outputs.skeletons.iter().collect();
    ws.member("shapes-part", &manifest("shapes-part", "shapes_part", &format!("{}{}", member_dep("shapes_decl", "shapes-decl"), repo_dep("rsk", "crates/rsk", ""))), &with_skeletons(&lib, &all));
    ws.check().assert_clean("the skeletons of the shapes declaration");
}
