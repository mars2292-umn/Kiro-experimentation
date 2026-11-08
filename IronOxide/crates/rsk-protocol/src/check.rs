//! Well-formedness (R38.5) and projection with full merge (R38.3).

use crate::ast::{Branch, Global, Local, Message};
use crate::ProtocolError;

fn err(rule: &'static str, message: String) -> ProtocolError {
    ProtocolError { rule, message }
}

/// The first message of a branch, through nothing: a branch of a choice
/// must start with a message.
fn first_message(g: &Global) -> Option<&Message> {
    match g {
        Global::Msg(m, _) => Some(m),
        _ => None,
    }
}

/// R38.5: the structural rules. `roles` are the Endpoint's declared roles.
pub fn well_formed(g: &Global, roles: &[&str]) -> Result<(), ProtocolError> {
    if roles.len() < 2 {
        return Err(err("R38.2", format!("a Protocol has two or more roles; {} declared", roles.len())));
    }
    for (i, r) in roles.iter().enumerate() {
        if roles[..i].contains(r) {
            return Err(err("R38.5", format!("role `{r}` is declared twice")));
        }
    }
    let used = g.roles();
    for r in &used {
        if !roles.contains(&r.as_str()) {
            return Err(err("R38.5", format!("the Protocol names role `{r}`, which the Endpoint does not declare")));
        }
    }
    wf(g, &mut Vec::new())?;
    for r in roles {
        if !used.iter().any(|u| u == r) {
            return Err(err("R38.5", format!("role `{r}` takes part in no interaction of the Protocol")));
        }
    }
    Ok(())
}

/// `recs`: the enclosing `rec` names with whether a message has occurred
/// since each was opened (guardedness).
fn wf(g: &Global, recs: &mut Vec<(String, bool)>) -> Result<(), ProtocolError> {
    match g {
        Global::End => Ok(()),
        Global::Continue(name) => match recs.iter().rev().find(|(n, _)| n == name) {
            None => Err(err("R38.5", format!("`continue {name}` names no enclosing `rec`"))),
            Some((_, true)) => Ok(()),
            Some((_, false)) => Err(err("R38.5", format!("`rec {name}` reaches `continue {name}` without a message: unguarded recursion"))),
        },
        Global::Rec { name, body } => {
            if recs.iter().any(|(n, _)| n == name) {
                return Err(err("R38.5", format!("`rec {name}` is nested inside another `rec {name}`")));
            }
            recs.push((name.clone(), false));
            let r = wf(body, recs);
            recs.pop();
            r
        }
        Global::Msg(m, rest) => {
            if m.from == m.to {
                return Err(err("R38.5", format!("message `{}` is from `{}` to itself", m.label, m.from)));
            }
            let saved: Vec<bool> = recs.iter().map(|(_, g)| *g).collect();
            for r in recs.iter_mut() {
                r.1 = true;
            }
            let res = wf(rest, recs);
            for (r, s) in recs.iter_mut().zip(saved) {
                r.1 = s;
            }
            res
        }
        Global::Choice { at, branches } => {
            if branches.len() < 2 {
                return Err(err("R38.5", format!("the choice at `{at}` has {} branch(es); two or more", branches.len())));
            }
            let mut receiver: Option<&str> = None;
            let mut labels: Vec<&str> = Vec::new();
            for b in branches {
                let Some(m) = first_message(b) else {
                    return Err(err("R38.5", format!("a branch of the choice at `{at}` does not start with a message from `{at}`")));
                };
                if m.from != *at {
                    return Err(err("R38.5", format!("a branch of the choice at `{at}` starts with `{}` from `{}`, not from `{at}`", m.label, m.from)));
                }
                match receiver {
                    None => receiver = Some(&m.to),
                    Some(r) if r != m.to => {
                        return Err(err("R38.5", format!("the branches of the choice at `{at}` start with messages to `{r}` and to `{}`; every branch informs the same role first", m.to)));
                    }
                    _ => {}
                }
                if labels.contains(&m.label.as_str()) {
                    return Err(err("R38.5", format!("two branches of the choice at `{at}` start with the label `{}`", m.label)));
                }
                labels.push(&m.label);
            }
            for b in branches {
                wf(b, recs)?;
            }
            Ok(())
        }
    }
}

/// Whether `r` sends or receives anywhere in `g`.
fn participates(g: &Global, r: &str) -> bool {
    match g {
        Global::End | Global::Continue(_) => false,
        Global::Msg(m, rest) => m.from == r || m.to == r || participates(rest, r),
        Global::Choice { branches, .. } => branches.iter().any(|b| participates(b, r)),
        Global::Rec { body, .. } => participates(body, r),
    }
}

/// The projection of a recursion body in which the role takes no part,
/// with the vacuous binder `name` removed: `end` stays `end`, a `continue`
/// of an enclosing recursion stands for the whole, a `continue` of the
/// vacuous recursion itself (a loop the role never acts in) is `end`, and
/// nested vacuous recursions collapse the same way.
fn collapse_vacuous(l: Local, name: &str) -> Local {
    match l {
        Local::End => Local::End,
        Local::Continue(n) => if n == name { Local::End } else { Local::Continue(n) },
        Local::Rec { name: inner, body } => match collapse_vacuous(*body, &inner) {
            Local::Continue(n) if n == name => Local::End,
            other => other,
        },
        other => other,
    }
}

/// R38.3: the projection of `g` onto `r`, with the full merge for the
/// roles a choice does not inform first.
pub fn project(g: &Global, r: &str) -> Result<Local, ProtocolError> {
    match g {
        Global::End => Ok(Local::End),
        Global::Continue(n) => Ok(Local::Continue(n.clone())),
        Global::Rec { name, body } => {
            if participates(body, r) {
                Ok(Local::Rec { name: name.clone(), body: Box::new(project(body, r)?) })
            } else {
                // The role takes no part in the body: the recursion is
                // vacuous for it. Its body projects to `end`, or to a
                // `continue` of an enclosing recursion, which then stands
                // for the whole (`rec Y { continue X }` is `continue X`).
                // Until 2026-11-09 this was `end`, which turned the
                // unbounded stream `rec X { A: d -> a; rec Y { continue X } }`
                // into a one-shot message; the verified exploration of the
                // flattened projections (task 21.6) refused what this
                // accepted, and found it.
                Ok(collapse_vacuous(project(body, r)?, name))
            }
        }
        Global::Msg(m, rest) => {
            let cont = project(rest, r)?;
            if m.from == r {
                Ok(Local::Send { to: m.to.clone(), branches: vec![Branch { label: m.label.clone(), payload: m.payload.clone(), cont }] })
            } else if m.to == r {
                Ok(Local::Recv { from: m.from.clone(), branches: vec![Branch { label: m.label.clone(), payload: m.payload.clone(), cont }] })
            } else {
                Ok(cont)
            }
        }
        Global::Choice { at, branches } => {
            let first: Vec<&Message> = branches.iter().filter_map(first_message).collect();
            let to = first.first().map(|m| m.to.clone()).unwrap_or_default();
            if r == at {
                let mut bs = Vec::new();
                for b in branches {
                    let Global::Msg(m, rest) = b else { unreachable!("well-formed") };
                    bs.push(Branch { label: m.label.clone(), payload: m.payload.clone(), cont: project(rest, r)? });
                }
                Ok(Local::Send { to, branches: bs })
            } else if r == to {
                let mut bs = Vec::new();
                for b in branches {
                    let Global::Msg(m, rest) = b else { unreachable!("well-formed") };
                    bs.push(Branch { label: m.label.clone(), payload: m.payload.clone(), cont: project(rest, r)? });
                }
                Ok(Local::Recv { from: at.clone(), branches: bs })
            } else {
                let mut merged: Option<Local> = None;
                for b in branches {
                    let p = project(b, r)?;
                    merged = Some(match merged {
                        None => p,
                        Some(m) => merge(m, p).map_err(|why| err("R38.5", format!("role `{r}` cannot tell the branches of the choice at `{at}` apart: {why}")))?,
                    });
                }
                Ok(merged.unwrap_or(Local::End))
            }
        }
    }
}

/// The full merge: identical types merge to themselves; external choices
/// from the same role merge by the union of their labels, shared labels
/// merging their continuations (equal payloads); anything else does not.
pub fn merge(a: Local, b: Local) -> Result<Local, String> {
    if a == b {
        return Ok(a);
    }
    match (a, b) {
        (Local::Recv { from: fa, branches: ba }, Local::Recv { from: fb, branches: bb }) if fa == fb => {
            let mut out: Vec<Branch> = ba;
            for x in bb {
                match out.iter_mut().find(|y| y.label == x.label) {
                    Some(y) => {
                        if y.payload != x.payload {
                            return Err(format!("label `{}` from `{fa}` carries different payloads in two branches", x.label));
                        }
                        y.cont = merge(y.cont.clone(), x.cont)?;
                    }
                    None => out.push(x),
                }
            }
            Ok(Local::Recv { from: fa, branches: out })
        }
        (Local::Rec { name: na, body: ba }, Local::Rec { name: nb, body: bb }) if na == nb => Ok(Local::Rec { name: na, body: Box::new(merge(*ba, *bb)?) }),
        (a, b) => Err(format!("its behaviours `{}` and `{}` differ and are not both external choices from one role", a.render(), b.render())),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse::parse;

    const LOOP: &str = "rec Loop { choice at ctl { Fix(u32): ctl -> sink; Ack(u32): sink -> ctl; continue Loop; } or { Stop(): ctl -> sink; end; } }";

    #[test]
    fn projects_the_request_response_loop() {
        let g = parse(LOOP).unwrap();
        well_formed(&g, &["ctl", "sink"]).unwrap();
        assert_eq!(project(&g, "ctl").unwrap().render(), "rec Loop.sink!{Fix(u32).sink?{Ack(u32).Loop}, Stop().end}");
        assert_eq!(project(&g, "sink").unwrap().render(), "rec Loop.ctl?{Fix(u32).ctl!{Ack(u32).Loop}, Stop().end}");
    }

    #[test]
    fn merges_an_uninformed_role_by_its_external_choices() {
        // c learns the branch from b's distinct labels.
        let g = parse("choice at a { L(): a -> b; X(): b -> c; end; } or { R(): a -> b; Y(): b -> c; end; }").unwrap();
        well_formed(&g, &["a", "b", "c"]).unwrap();
        assert_eq!(project(&g, "c").unwrap().render(), "b?{X().end, Y().end}");
        // c cannot tell them apart when b stays silent in one branch.
        let g = parse("choice at a { L(): a -> b; X(): b -> c; end; } or { R(): a -> b; end; }").unwrap();
        well_formed(&g, &["a", "b", "c"]).unwrap();
        let e = project(&g, "c").unwrap_err();
        assert_eq!(e.rule, "R38.5");
        assert!(e.message.contains("cannot tell the branches of the choice at `a` apart"), "{e}");
    }

    #[test]
    fn rejects_the_structural_faults_naming_them() {
        let bad = [
            ("M(u8): a -> a; end", "to itself"),
            ("rec X { continue X; }", "unguarded"),
            ("M(u8): a -> b; continue Y;", "names no enclosing"),
            ("choice at a { M(): a -> b; end; }", "two or more"),
            ("choice at a { M(): b -> a; end; } or { N(): a -> b; end; }", "not from `a`"),
            ("choice at a { M(): a -> b; end; } or { M(): a -> b; end; }", "two branches of the choice at `a` start with the label"),
            ("M(u8): a -> c; end", "does not declare"),
        ];
        for (text, fragment) in bad {
            let g = parse(text).unwrap();
            let e = well_formed(&g, &["a", "b"]).unwrap_err();
            assert!(e.message.contains(fragment), "{text}: {e}");
        }
        let g = parse("M(u8): a -> b; end").unwrap();
        assert!(well_formed(&g, &["a", "b", "c"]).unwrap_err().message.contains("takes part in no interaction"));
    }
}
