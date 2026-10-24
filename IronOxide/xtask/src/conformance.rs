//! The Profile_Conformance_Suite runner (R3.6, R3.7, R3.8, R3.10, R3.11;
//! task 14.1): `docs/conformance_suite.toml` names one case per active
//! restriction and enforcement category, one fault-injection case per RT
//! restriction, the accepted Application per Profile_Variant, and the
//! proof artifacts of the PO restrictions. This module runs them and writes
//! one Evidence_Item per case (`conf-PR-nn-cc`, `fi-PR-nn`,
//! `accepted-<variant>`).

use std::path::{Path, PathBuf};

use toml::{Table, Value};

use crate::cmd::Cmd;
use crate::diag::{Report, Rule};
use crate::evidence::{Item, Kind, Store, Tool, Verdict};
use crate::sandbox::Sandbox;
use crate::workspace::Workspace;

pub const REGISTER: &str = "docs/conformance_suite.toml";

#[derive(Debug, Clone)]
pub enum CaseKind {
    Generator { find: String, replace: String },
    CompileFail { member: String },
    Lint { file: String, scope: String },
    BuildCheck { fixture: String },
    LinkPatch { symbol: String, offset: u32, bytes: Vec<u8> },
    LinkRename { symbol: String, to: String },
    FaultInjection { feature: String, event: u8 },
    Reason(String),
}

#[derive(Debug, Clone)]
pub struct Case {
    pub id: String,
    pub restriction: String,
    pub category: String,
    pub kind: CaseKind,
    pub expect: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Accepted {
    pub variant: String,
    pub binary: Option<String>,
    pub reason: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ProofArtifact {
    pub evidence: String,
    pub restriction: String,
    pub items: Vec<String>,
    pub note: Option<String>,
}

pub struct Suite {
    pub declaration: String,
    pub binary: String,
    pub cases: Vec<Case>,
    pub accepted: Vec<Accepted>,
    pub proofs: Vec<ProofArtifact>,
}

fn str_of(t: &Table, k: &str, ctx: &str) -> Result<String, String> {
    t.get(k).and_then(Value::as_str).map(str::to_string).ok_or_else(|| format!("{REGISTER}: {ctx}: missing `{k}`"))
}

fn list_of(t: &Table, k: &str) -> Vec<String> {
    t.get(k).and_then(Value::as_array).map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect()).unwrap_or_default()
}

pub fn load(root: &Path) -> Result<Suite, String> {
    let text = std::fs::read_to_string(root.join(REGISTER)).map_err(|e| format!("{REGISTER}: {e}"))?;
    let doc: Table = text.parse().map_err(|e| format!("{REGISTER}: {e}"))?;
    let declaration = str_of(&doc, "declaration", "header")?;
    let binary = str_of(&doc, "binary", "header")?;
    let mut cases = Vec::new();
    for (i, c) in doc.get("case").and_then(Value::as_array).into_iter().flatten().enumerate() {
        let Some(c) = c.as_table() else { return Err(format!("{REGISTER}: case {i} is not a table")) };
        let ctx = format!("case {i}");
        let id = str_of(c, "id", &ctx)?;
        let kind_name = str_of(c, "kind", &id)?;
        let kind = match kind_name.as_str() {
            "generator" => CaseKind::Generator { find: str_of(c, "find", &id)?, replace: str_of(c, "replace", &id)? },
            "compile-fail" => CaseKind::CompileFail { member: str_of(c, "member", &id)? },
            "lint" => CaseKind::Lint { file: str_of(c, "file", &id)?, scope: str_of(c, "scope", &id)? },
            "build-check" => CaseKind::BuildCheck { fixture: str_of(c, "fixture", &id)? },
            "link-mutation" => {
                if let Ok(to) = str_of(c, "to", &id) {
                    CaseKind::LinkRename { symbol: str_of(c, "rename", &id)?, to }
                } else {
                    let hex = str_of(c, "bytes", &id)?;
                    let bytes = (0..hex.len() / 2).map(|k| u8::from_str_radix(&hex[2 * k..2 * k + 2], 16)).collect::<Result<Vec<u8>, _>>().map_err(|e| format!("{REGISTER}: {id}: bytes: {e}"))?;
                    let offset = c.get("offset").and_then(Value::as_integer).unwrap_or(0) as u32;
                    CaseKind::LinkPatch { symbol: str_of(c, "symbol", &id)?, offset, bytes }
                }
            }
            "fault-injection" => CaseKind::FaultInjection { feature: str_of(c, "feature", &id)?, event: c.get("event").and_then(Value::as_integer).ok_or(format!("{REGISTER}: {id}: missing `event`"))? as u8 },
            "reason" => CaseKind::Reason(str_of(c, "reason", &id)?),
            other => return Err(format!("{REGISTER}: {id}: unknown kind `{other}`")),
        };
        cases.push(Case { restriction: str_of(c, "restriction", &id)?, category: str_of(c, "category", &id)?, kind, expect: list_of(c, "expect"), note: c.get("note").and_then(Value::as_str).map(String::from), id });
    }
    let mut accepted = Vec::new();
    for a in doc.get("accepted").and_then(Value::as_array).into_iter().flatten() {
        let Some(a) = a.as_table() else { continue };
        accepted.push(Accepted { variant: str_of(a, "variant", "accepted")?, binary: a.get("binary").and_then(Value::as_str).map(String::from), reason: a.get("reason").and_then(Value::as_str).map(String::from) });
    }
    let mut proofs = Vec::new();
    for p in doc.get("proof").and_then(Value::as_array).into_iter().flatten() {
        let Some(p) = p.as_table() else { continue };
        proofs.push(ProofArtifact { evidence: str_of(p, "evidence", "proof")?, restriction: str_of(p, "restriction", "proof")?, items: list_of(p, "items"), note: p.get("note").and_then(Value::as_str).map(String::from) });
    }
    Ok(Suite { declaration, binary, cases, accepted, proofs })
}

/// R3.6: every active restriction has a case for each of its TS, MC, LN,
/// and LK categories, and every RT restriction a fault-injection case.
pub fn check_coverage(root: &Path, suite: &Suite, report: &mut Report) {
    let Ok(text) = std::fs::read_to_string(root.join("profile/profile.toml")) else { return };
    let Ok(doc) = text.parse::<Table>() else { return };
    let mut missing = Vec::new();
    for r in doc.get("restriction").and_then(Value::as_array).into_iter().flatten() {
        let Some(r) = r.as_table() else { continue };
        let id = r.get("id").and_then(Value::as_str).unwrap_or("");
        if r.get("status").and_then(Value::as_str).is_some_and(|s| s != "active") {
            continue;
        }
        for e in r.get("enforcement").and_then(Value::as_array).into_iter().flatten() {
            let Some(cat) = e.get("category").and_then(Value::as_str) else { continue };
            let found = suite.cases.iter().any(|c| c.restriction == id && c.category == cat);
            if !found && matches!(cat, "TS" | "MC" | "LN" | "LK" | "RT") {
                missing.push(format!("{id} {cat}"));
            }
        }
    }
    if missing.is_empty() {
        report.note(format!("R3.6/R3.11: every active restriction has a case for each of its TS, MC, LN, LK, and RT categories ({} cases, {} of them reasons)", suite.cases.len(), suite.cases.iter().filter(|c| matches!(c.kind, CaseKind::Reason(_))).count()));
    } else {
        report.error(Rule::Profile, Some(REGISTER.into()), format!("R3.6: no conformance case for {}", missing.join(", ")));
    }
}

/// What the runner needs from its caller.
pub struct Context<'a> {
    pub root: &'a Path,
    pub ws: &'a Workspace,
    /// Where the Flight_Build of the suite's binary is (and where fault-injection builds go).
    pub target_dir: &'a Path,
    pub sandbox: Option<&'a Sandbox>,
    pub store: &'a mut Store,
    pub manifest_id: Option<String>,
    pub only: Option<String>,
}

fn contains_all(text: &str, expect: &[String]) -> Vec<String> {
    expect.iter().filter(|e| !text.contains(e.as_str())).cloned().collect()
}

/// Runs the suite; returns the command lines and whether every provided
/// case passed.
pub fn run(cx: &mut Context, report: &mut Report) -> Result<(Vec<String>, bool), String> {
    let suite = load(cx.root)?;
    check_coverage(cx.root, &suite, report);
    let mut commands = Vec::new();
    let mut all_ok = true;
    let mut passed = 0;
    let mut reasons = 0;
    let decl_text = std::fs::read_to_string(cx.root.join(&suite.declaration)).map_err(|e| format!("{}: {e}", suite.declaration))?;
    let triple = cx.ws.policy.target.clone();
    let elf = cx.target_dir.join(&triple).join("release").join(&suite.binary);
    let model = cx.root.join("apps/p1-demo/gen/task_model.json");
    let work = cx.store.dir.join("conformance");
    let _ = std::fs::create_dir_all(&work);
    let qemu_version = crate::qemu::version();
    let mut fi_target: Option<PathBuf> = None;
    for case in &suite.cases {
        if cx.only.as_ref().is_some_and(|o| o != &case.id) {
            continue;
        }
        let mut notes: Vec<String> = case.note.iter().cloned().collect();
        let mut artifacts = Vec::new();
        let mut command = String::new();
        let outcome: Result<bool, String> = match &case.kind {
            CaseKind::Reason(why) => {
                notes.push(format!("not provided: {why}"));
                reasons += 1;
                Err(String::new())
            }
            CaseKind::Generator { find, replace } => {
                if decl_text.matches(find.as_str()).count() != 1 {
                    Err(format!("`find` occurs {} times in {}", decl_text.matches(find.as_str()).count(), suite.declaration))
                } else {
                    let text = decl_text.replacen(find.as_str(), replace, 1);
                    let output = match rsk_gen::syntax::parse_source(&text) {
                        Err(e) => e,
                        Ok(parsed) => match rsk_gen::outputs(&parsed.decl, "rsk-system") {
                            Ok(_) => String::from("accepted: the Generator produced outputs"),
                            Err(diags) => diags.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n"),
                        },
                    };
                    command = format!("rsk-gen (library) on {} with `{}` -> `{}`", suite.declaration, find.replace('\n', "\\n"), replace.replace('\n', "\\n"));
                    let log = format!("{}.log", case.id);
                    let _ = cx.store.write_artifact(&log, &output);
                    artifacts.push(log);
                    let missing = contains_all(&output, &case.expect);
                    if output.starts_with("accepted") {
                        Err(output)
                    } else if missing.is_empty() {
                        notes.push(output.lines().next().unwrap_or("").to_string());
                        Ok(true)
                    } else {
                        Err(format!("rejected without naming {}: {}", missing.join(", "), output.lines().next().unwrap_or("")))
                    }
                }
            }
            CaseKind::CompileFail { member } => {
                let fixture_root = cx.root.join("xtask/fixtures/api-misuse");
                let cmd = Cmd::cargo().args(["check", "--frozen", "--offline", "-p", member, "--target-dir"]).arg(cx.target_dir.join("api-misuse")).cwd(&fixture_root);
                command = cmd.display();
                match cmd.output(cx.sandbox) {
                    Err(e) => Err(format!("cannot run cargo check: {e}")),
                    Ok(o) => {
                        let text = String::from_utf8_lossy(&o.stderr).into_owned();
                        let log = format!("{}.log", case.id);
                        let _ = cx.store.write_artifact(&log, &text);
                        artifacts.push(log);
                        if o.status.success() {
                            Err(format!("`{member}` compiled"))
                        } else {
                            let missing = contains_all(&text, &case.expect);
                            if missing.is_empty() { Ok(true) } else { Err(format!("`{member}` failed without {}", missing.join(", "))) }
                        }
                    }
                }
            }
            CaseKind::Lint { file, scope } => {
                let path = cx.root.join("crates/rsk-lint/fixtures/ui").join(file);
                let cmd = Cmd::cargo_frozen("run")
                    .args(["--quiet", "-p", "rsk-lint", "--manifest-path"])
                    .arg(cx.root.join("Cargo.toml"))
                    .arg("--target-dir")
                    .arg(cx.target_dir)
                    .arg("--")
                    .arg("--file")
                    .arg(format!("{scope}={}", path.display()))
                    .arg("--profile")
                    .arg(cx.root.join("profile/profile.toml"))
                    .arg("--core-subset")
                    .arg(cx.root.join("profile/core_subset.toml"));
                command = cmd.display();
                match cmd.output(cx.sandbox) {
                    Err(e) => Err(format!("cannot run rsk-lint: {e}")),
                    Ok(o) => {
                        let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                        let log = format!("{}.log", case.id);
                        let _ = cx.store.write_artifact(&log, &text);
                        artifacts.push(log);
                        let missing = contains_all(&text, &case.expect);
                        if o.status.success() {
                            Err(format!("the Profile_Lint accepted {file}"))
                        } else if missing.is_empty() {
                            Ok(true)
                        } else {
                            Err(format!("rejected without {}", missing.join(", ")))
                        }
                    }
                }
            }
            CaseKind::BuildCheck { fixture } => {
                let fixture_root = cx.root.join("xtask/fixtures").join(fixture);
                command = format!("cargo xtask check --root {}", fixture_root.display());
                let found = crate::check_workspace(&fixture_root);
                let text = found.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>().join("\n");
                let log = format!("{}.log", case.id);
                let _ = cx.store.write_artifact(&log, &text);
                artifacts.push(log);
                let missing = contains_all(&text, &case.expect);
                if found.diagnostics.is_empty() {
                    Err(format!("the Build_System accepted fixture `{fixture}`"))
                } else if missing.is_empty() {
                    Ok(true)
                } else {
                    Err(format!("rejected without {}", missing.join(", ")))
                }
            }
            CaseKind::LinkPatch { .. } | CaseKind::LinkRename { .. } => {
                if !elf.is_file() {
                    Err(format!("no Flight_Build at {}", elf.display()))
                } else {
                    match std::fs::read(&elf).map_err(|e| e.to_string()).and_then(|bytes| mutate(&bytes, &case.kind)) {
                        Err(e) => Err(e),
                        Ok(altered) => {
                            let patched = work.join(format!("{}.elf", case.id));
                            let _ = std::fs::write(&patched, &altered);
                            let cmd = Cmd::cargo_frozen("run")
                                .args(["--quiet", "-p", "rsk-link", "--manifest-path"])
                                .arg(cx.root.join("Cargo.toml"))
                                .arg("--target-dir")
                                .arg(cx.target_dir)
                                .arg("--")
                                .arg(&patched)
                                .arg(&model)
                                .arg("--profile")
                                .arg(cx.root.join("profile/profile.toml"))
                                .arg("--core-subset")
                                .arg(cx.root.join("profile/core_subset.toml"));
                            command = cmd.display();
                            match cmd.output(cx.sandbox) {
                                Err(e) => Err(format!("cannot run rsk-link: {e}")),
                                Ok(o) => {
                                    let text = format!("{}{}", String::from_utf8_lossy(&o.stdout), String::from_utf8_lossy(&o.stderr));
                                    let log = format!("{}.log", case.id);
                                    let _ = cx.store.write_artifact(&log, &text);
                                    artifacts.push(log);
                                    let missing = contains_all(&text, &case.expect);
                                    if o.status.success() {
                                        Err("the Link_Checker accepted the altered binary".to_string())
                                    } else if missing.is_empty() {
                                        Ok(true)
                                    } else {
                                        Err(format!("rejected without {}", missing.join(", ")))
                                    }
                                }
                            }
                        }
                    }
                }
            }
            CaseKind::FaultInjection { feature, event } => {
                if qemu_version.is_none() {
                    notes.push("qemu-system-arm is not installed: the fault-injection run was not performed".to_string());
                }
                let fi_dir = fi_target.get_or_insert_with(|| cx.target_dir.join("conformance-fi")).clone();
                if qemu_version.is_none() {
                    Err(String::new())
                } else {
                let build = Cmd::cargo_frozen("build")
                    .args(["--release", "--target", &triple, "-p", &suite.binary, "-p", "p1-demo-part0", "--features"])
                    .arg(format!("p1-demo-part0/{feature}"))
                    .arg("--manifest-path")
                    .arg(cx.root.join("Cargo.toml"))
                    .arg("--target-dir")
                    .arg(&fi_dir)
                    .env("CARGO_ENCODED_RUSTFLAGS", crate::verify::target_rustflags(cx.root));
                command = build.display();
                match build.status(cx.sandbox) {
                    Err(e) => Err(format!("cannot build: {e}")),
                    Ok(false) => Err("the fault-injection build failed".to_string()),
                    Ok(true) => {
                        let fi_elf = fi_dir.join(&triple).join("release").join(&suite.binary);
                        command = format!("{command}; {}", crate::qemu::command_line(&fi_elf));
                        match crate::qemu::run_elf(&fi_elf, 120) {
                            Err(e) => Err(format!("QEMU: {e}")),
                            Ok(run) => {
                                let log = format!("{}.log", case.id);
                                let _ = cx.store.write_artifact(&log, &run.log);
                                artifacts.push(log);
                                let n: u32 = case.restriction.trim_start_matches("PR-").parse().unwrap_or(0);
                                let bit = 1u64 << n;
                                let events = events_of(&run.log);
                                let hit = events.iter().filter(|(k, _)| *k == *event).count();
                                let tagged = events.iter().any(|(k, prs)| *k == *event && prs & bit != 0);
                                notes.push(format!("{} events logged, {hit} of kind {event}, exit {} after {:.1}s", events.len(), run.exit_code, run.seconds));
                                if hit == 0 {
                                    Err(format!("no event of kind {event} in the Health_Monitor log"))
                                } else if !tagged {
                                    Err(format!("the event of kind {event} does not carry the {} bit (R3.9)", case.restriction))
                                } else {
                                    Ok(true)
                                }
                            }
                        }
                    }
                }
                }
            }
        };
        let (verdict, kind) = match (&outcome, &case.kind) {
            (Err(e), CaseKind::Reason(_)) if e.is_empty() => (Verdict::Inconclusive, Kind::Review),
            (Err(e), CaseKind::FaultInjection { .. }) if e.is_empty() => (Verdict::Inconclusive, Kind::Test),
            (Ok(_), CaseKind::FaultInjection { .. }) | (Err(_), CaseKind::FaultInjection { .. }) => (Verdict::from_bool(outcome.is_ok()), Kind::Test),
            (_, CaseKind::CompileFail { .. }) => (Verdict::from_bool(outcome.is_ok()), Kind::Test),
            _ => (Verdict::from_bool(outcome.is_ok()), Kind::Check),
        };
        match &outcome {
            Ok(_) => passed += 1,
            Err(e) if e.is_empty() => {}
            Err(e) => {
                all_ok = false;
                report.error(Rule::Step, Some(REGISTER.into()), format!("{} ({} {}): {e}", case.id, case.restriction, case.category));
            }
        }
        if !command.is_empty() {
            commands.push(command.clone());
        }
        let mut item = Item::new(
            &case.id,
            kind,
            &format!("Profile_Conformance_Suite: {} {} case", case.restriction, case.category),
            Tool { name: "cargo xtask conformance".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] },
            verdict,
        )
        .covers(&[case.restriction.as_str(), if case.category == "RT" { "R3.11" } else { "R3.6" }]);
        item.command_lines = if command.is_empty() { vec![] } else { vec![command] };
        item.notes = notes;
        item.build_manifest = cx.manifest_id.clone();
        item.artifacts = artifacts;
        if let Err(e) = cx.store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }

    // R3.7: the accepted Application per Profile_Variant.
    for a in &suite.accepted {
        if cx.only.is_some() {
            break;
        }
        let id = format!("accepted-{}", a.variant.to_lowercase());
        let (verdict, notes) = match (&a.binary, &a.reason) {
            (Some(bin), _) => {
                let built = cx.target_dir.join(&triple).join("release").join(bin).is_file();
                let checks = [format!("analysis-stack-{bin}"), format!("check-config-{bin}"), format!("test-qemu-spike-{bin}"), "check-profile-lint".to_string()];
                let mut missing = Vec::new();
                for c in &checks {
                    if !item_passed(cx.store, c) {
                        missing.push(c.clone());
                    }
                }
                let standalone = checks.iter().all(|c| !cx.store.item_path(c).is_file());
                if standalone {
                    (Verdict::Inconclusive, vec![format!("Flight_Build `{bin}` built={built}; standalone run: the R3.5 checks ({}) are not in this evidence store, so the verdict is the verification job's", checks.join(", "))])
                } else {
                    let ok = built && missing.is_empty();
                    if !ok {
                        all_ok = false;
                        report.error(Rule::Step, Some(REGISTER.into()), format!("{id}: built={built}; checks not passed in this evidence store: {}", missing.join(", ")));
                    }
                    (Verdict::from_bool(ok), vec![format!("Flight_Build `{bin}` built and the checks of R3.5 passed: {}", checks.join(", "))])
                }
            }
            (None, Some(why)) => (Verdict::Inconclusive, vec![format!("not provided: {why}")]),
            (None, None) => (Verdict::Fail, vec!["neither a binary nor a reason".into()]),
        };
        let mut item = Item::new(&id, Kind::Test, &format!("Profile_Conformance_Suite: accepted Application of the {} variant", a.variant), Tool { name: "cargo xtask conformance".to_string(), version: env!("CARGO_PKG_VERSION").to_string(), solvers: vec![] }, verdict).covers(&["R3.7"]);
        item.notes = notes;
        item.build_manifest = cx.manifest_id.clone();
        if let Err(e) = cx.store.write(&item) {
            report.error(Rule::Evidence, None, e);
        }
    }

    // R3.8, R3.10: the proof artifacts of the PO restrictions.
    if cx.only.is_none() {
        let sources = {
            let mut files = Vec::new();
            for d in ["crates/rsk-logic/src", "crates/rsk-kernel/src"] {
                collect_rs(&cx.root.join(d), &mut files);
            }
            files.iter().filter_map(|f| std::fs::read_to_string(f).ok()).collect::<Vec<_>>().join("\n")
        };
        let verus_passed = item_passed(cx.store, "proof-kernel-verus");
        let verus_known = cx.store.item_path("proof-kernel-verus").is_file();
        let mut named = 0;
        for p in &suite.proofs {
            let absent: Vec<&String> = p.items.iter().filter(|i| !sources.contains(&format!("fn {i}")) && !sources.contains(&format!("fn {i}<"))).collect();
            if !absent.is_empty() {
                all_ok = false;
                report.error(Rule::Proof, Some(REGISTER.into()), format!("R3.10: the proof artifact `{}` of {} names items absent from the verified crates: {}", p.evidence, p.restriction, absent.iter().map(|s| s.as_str()).collect::<Vec<_>>().join(", ")));
            } else if verus_known && !verus_passed {
                all_ok = false;
                report.error(Rule::Proof, Some(REGISTER.into()), format!("R3.10: the Kernel_Proofs failed, so the proof artifact `{}` of {} is not discharged", p.evidence, p.restriction));
            } else {
                named += 1;
            }
        }
        report.note(format!("R3.8/R3.10: {named} of {} PO proof artifacts name items present in the verified crates{}", suite.proofs.len(), if verus_known { if verus_passed { "; the Kernel_Proofs passed" } else { "" } } else { "; no Kernel_Proofs item in this store (standalone run)" }));
    }
    report.note(format!("Profile_Conformance_Suite: {passed} cases passed, {reasons} not provided with a reason, {} accepted-Application entries, {} proof artifacts", suite.accepted.len(), suite.proofs.len()));
    Ok((commands, all_ok))
}

fn item_passed(store: &Store, id: &str) -> bool {
    std::fs::read_to_string(store.item_path(id)).map(|t| t.contains("\"verdict\": \"pass\"")).unwrap_or(false)
}

fn collect_rs(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else { return };
    for e in entries.flatten() {
        let p = e.path();
        if p.is_dir() {
            collect_rs(&p, out);
        } else if p.extension().is_some_and(|x| x == "rs") {
            out.push(p);
        }
    }
}

/// `event=<k>` lines followed by their `prs_lo`/`prs_hi` lines in the demo's safe-state report.
pub fn events_of(log: &str) -> Vec<(u8, u64)> {
    let mut out: Vec<(u8, u64)> = Vec::new();
    for line in log.lines() {
        let line = line.trim();
        if let Some(v) = line.strip_prefix("event=") {
            if let Ok(k) = v.trim().parse::<u8>() {
                out.push((k, 0));
            }
        } else if let Some(v) = line.strip_prefix("prs_lo=") {
            if let (Some(last), Ok(n)) = (out.last_mut(), v.trim().parse::<u64>()) {
                last.1 |= n;
            }
        } else if let Some(v) = line.strip_prefix("prs_hi=") {
            if let (Some(last), Ok(n)) = (out.last_mut(), v.trim().parse::<u64>()) {
                last.1 |= n << 32;
            }
        }
    }
    out
}

/// Applies a link-mutation to the ELF bytes.
pub fn mutate(bytes: &[u8], kind: &CaseKind) -> Result<Vec<u8>, String> {
    let elf = rsk_elf::Elf::parse(bytes.to_vec()).map_err(|e| e.to_string())?;
    let symbols = elf.symbols().map_err(|e| e.to_string())?;
    let mut out = bytes.to_vec();
    match kind {
        CaseKind::LinkPatch { symbol, offset, bytes: patch } => {
            let sym = symbols.iter().find(|s| s.is_func() && s.name.starts_with(symbol.as_str())).ok_or_else(|| format!("no function symbol starting with `{symbol}`"))?;
            let addr = sym.address() + offset;
            let section = elf.section_at(addr).ok_or_else(|| format!("0x{addr:08x} is in no section"))?;
            let file_off = (section.offset + (addr - section.addr)) as usize;
            if file_off + patch.len() > out.len() {
                return Err("patch beyond the file".into());
            }
            out[file_off..file_off + patch.len()].copy_from_slice(patch);
        }
        CaseKind::LinkRename { symbol, to } => {
            let sym = symbols.iter().find(|s| s.is_func() && s.name.starts_with(symbol.as_str())).ok_or_else(|| format!("no function symbol starting with `{symbol}`"))?;
            let strtab = elf.section_by_name(".strtab").ok_or("no .strtab")?;
            let data = elf.section_data(strtab).map_err(|e| e.to_string())?;
            let needle: Vec<u8> = [&[0u8][..], sym.name.as_bytes(), &[0u8][..]].concat();
            let pos = data.windows(needle.len()).position(|w| w == needle.as_slice()).ok_or_else(|| format!("`{}` not found in .strtab", sym.name))?;
            if to.len() > sym.name.len() {
                return Err(format!("`{to}` is longer than `{}`", sym.name));
            }
            let start = strtab.offset as usize + pos + 1;
            out[start..start + to.len()].copy_from_slice(to.as_bytes());
            out[start + to.len()] = 0;
        }
        _ => return Err("not a link mutation".into()),
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// R3.6, R3.11: the conformance register has a case for every active restriction and each of its TS, MC, LN, LK, and RT categories, and maps every proof artifact of the Profile's PO evidence.
    #[test]
    fn the_register_loads_and_covers_every_category() {
        let root = crate::repository_root();
        let suite = load(&root).unwrap();
        assert!(suite.cases.len() >= 60, "{}", suite.cases.len());
        let mut report = Report::default();
        check_coverage(&root, &suite, &mut report);
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics.iter().map(ToString::to_string).collect::<Vec<_>>());
        // Every proof artifact of the Profile's PO evidence is mapped.
        let text = std::fs::read_to_string(root.join("profile/profile.toml")).unwrap();
        for name in ["proof-kernel-mutual-exclusion", "proof-kernel-ceiling-lifo", "proof-kernel-mpu-views-disjoint", "proof-kernel-no-release-before-init", "proof-kernel-static-priorities", "proof-kernel-queues-refine", "proof-kernel-loop-bounds", "proof-kernel-panic-freedom", "proof-kernel-unsafe-justifications", "proof-kernel-pr02-task-set-stable"] {
            assert!(text.contains(name) && suite.proofs.iter().any(|p| p.evidence == name), "{name}");
        }
    }

    #[test]
    fn events_are_parsed_with_their_restriction_bits() {
        let log = "RSK-SAFE-STATE\nevent=1\n  task=1\n  prs_lo=524288\n  prs_hi=0\nevent=10\n  prs_lo=0\n  prs_hi=0\n";
        assert_eq!(events_of(log), vec![(1, 1 << 19), (10, 0)]);
    }
}
