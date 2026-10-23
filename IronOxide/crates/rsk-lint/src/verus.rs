//! Loops and `core::` paths inside `verus!` bodies and other macro
//! invocations, found at the token level (the bodies are not Rust syntax
//! that `syn` parses). A Verus loop is bounded by its `decreases` clause,
//! which the Kernel_Proofs check (R35.2); a `for` over a range is bounded
//! by the range.

use proc_macro2::{Delimiter, TokenStream, TokenTree};

pub struct VerusLoop {
    pub line: usize,
    pub function: String,
    pub label: Option<String>,
    pub keyword: &'static str,
    /// `decreases` present, or a `for` over a range.
    pub bounded: bool,
    /// `loop {}` with an empty body.
    pub empty: bool,
}

pub fn loops(ts: &TokenStream, out: &mut Vec<VerusLoop>) {
    scan(ts, None, out);
}

fn scan(ts: &TokenStream, enclosing: Option<String>, out: &mut Vec<VerusLoop>) {
    let v: Vec<TokenTree> = ts.clone().into_iter().collect();
    let mut current = enclosing;
    let mut i = 0;
    while i < v.len() {
        match &v[i] {
            TokenTree::Ident(id) if id == "fn" => {
                if let Some(TokenTree::Ident(name)) = v.get(i + 1) {
                    current = Some(name.to_string());
                }
                i += 1;
            }
            TokenTree::Ident(id) if id == "while" || id == "loop" || id == "for" => {
                let keyword: &'static str = if id == "while" { "while" } else if id == "loop" { "loop" } else { "for" };
                let label = match (v.get(i.wrapping_sub(3)), v.get(i.wrapping_sub(2)), v.get(i.wrapping_sub(1))) {
                    (Some(TokenTree::Punct(q)), Some(TokenTree::Ident(l)), Some(TokenTree::Punct(c))) if q.as_char() == '\'' && c.as_char() == ':' => Some(format!("'{l}")),
                    _ => None,
                };
                // The header runs to the body group.
                let mut j = i + 1;
                let mut has_decreases = false;
                let mut has_in = false;
                let mut has_range = false;
                let mut body: Option<&proc_macro2::Group> = None;
                while j < v.len() {
                    match &v[j] {
                        TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                            body = Some(g);
                            break;
                        }
                        TokenTree::Ident(x) if x == "decreases" => has_decreases = true,
                        TokenTree::Ident(x) if x == "in" => has_in = true,
                        TokenTree::Punct(p) if p.as_char() == '.' => {
                            if matches!(v.get(j + 1), Some(TokenTree::Punct(q)) if q.as_char() == '.') {
                                has_range = true;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }
                // `impl X for Y {}` and `for<'a>` are not loops.
                if keyword == "for" && !has_in {
                    i += 1;
                    continue;
                }
                let empty = keyword == "loop" && body.is_some_and(|g| g.stream().is_empty());
                out.push(VerusLoop { line: id.span().start().line, function: current.clone().unwrap_or_default(), label, keyword, bounded: has_decreases || (keyword == "for" && has_range), empty });
                i += 1;
            }
            TokenTree::Group(g) => {
                scan(&g.stream(), current.clone(), out);
                i += 1;
            }
            _ => i += 1,
        }
    }
}

/// Every `core::a::b` path in a token stream, with its line, expanding
/// `core::x::{a, b}` trees.
pub fn core_paths(ts: &TokenStream, out: &mut Vec<(usize, String)>) {
    let v: Vec<TokenTree> = ts.clone().into_iter().collect();
    let mut i = 0;
    while i < v.len() {
        match &v[i] {
            TokenTree::Ident(id) if id == "core" && is_path_sep(v.get(i + 1), v.get(i + 2)) => {
                let line = id.span().start().line;
                let mut segs = vec!["core".to_string()];
                let mut j = i + 1;
                while is_path_sep(v.get(j), v.get(j + 1)) {
                    match v.get(j + 2) {
                        Some(TokenTree::Ident(s)) => {
                            segs.push(s.to_string());
                            j += 3;
                        }
                        Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                            let prefix = segs.join("::");
                            let mut inner = Vec::new();
                            use_tree(&g.stream(), &prefix, line, &mut inner);
                            out.extend(inner);
                            segs.clear();
                            j += 3;
                            break;
                        }
                        _ => break,
                    }
                }
                if !segs.is_empty() {
                    out.push((line, segs.join("::")));
                }
                i = j;
            }
            TokenTree::Group(g) => {
                core_paths(&g.stream(), out);
                i += 1;
            }
            _ => i += 1,
        }
    }
}

fn is_path_sep(a: Option<&TokenTree>, b: Option<&TokenTree>) -> bool {
    matches!((a, b), (Some(TokenTree::Punct(p)), Some(TokenTree::Punct(q))) if p.as_char() == ':' && q.as_char() == ':')
}

/// `{a, b::c, d::{e, f}}` under `prefix`.
fn use_tree(ts: &TokenStream, prefix: &str, line: usize, out: &mut Vec<(usize, String)>) {
    let v: Vec<TokenTree> = ts.clone().into_iter().collect();
    let mut segs: Vec<String> = Vec::new();
    let mut i = 0;
    let flush = |segs: &mut Vec<String>, out: &mut Vec<(usize, String)>| {
        if !segs.is_empty() {
            out.push((line, format!("{prefix}::{}", segs.join("::"))));
            segs.clear();
        }
    };
    while i < v.len() {
        match &v[i] {
            TokenTree::Ident(s) if s != "as" => {
                segs.push(s.to_string());
                i += 1;
            }
            TokenTree::Ident(_) => {
                // `x as y`: skip the alias.
                i += 2;
            }
            TokenTree::Punct(p) if p.as_char() == ',' => {
                flush(&mut segs, out);
                i += 1;
            }
            TokenTree::Punct(p) if p.as_char() == '*' => {
                segs.push("*".into());
                i += 1;
            }
            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                let sub = format!("{prefix}::{}", segs.join("::"));
                segs.clear();
                use_tree(&g.stream(), sub.trim_end_matches("::"), line, out);
                i += 1;
            }
            _ => i += 1,
        }
    }
    flush(&mut segs, out);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn verus_loops_and_impl_for() {
        let ts: TokenStream = "verus! { impl<T> Tr for S<T> { fn f(&self) { let mut i = 0; while i < n invariant i <= n decreases n - i { i += 1; } for x in 0..n { } 'a: loop { i += 1; } loop decreases k { i += 1; } loop {} } } }".parse().unwrap();
        let mut out = Vec::new();
        loops(&ts, &mut out);
        let kinds: Vec<(&str, bool, bool, Option<String>)> = out.iter().map(|l| (l.keyword, l.bounded, l.empty, l.label.clone())).collect();
        assert_eq!(kinds, vec![("while", true, false, None), ("for", true, false, None), ("loop", false, false, Some("'a".into())), ("loop", true, false, None), ("loop", false, true, None)]);
        assert!(out.iter().all(|l| l.function == "f"));
    }

    #[test]
    fn core_path_trees() {
        let ts: TokenStream = "use core::sync::atomic::{AtomicBool, Ordering}; fn f() { core::ptr::write_volatile(p, 0); let x = ::core::hint::black_box(1); use core::{cell::Cell, mem}; }".parse().unwrap();
        let mut out = Vec::new();
        core_paths(&ts, &mut out);
        let paths: Vec<String> = out.into_iter().map(|(_, p)| p).collect();
        assert_eq!(paths, vec!["core::sync::atomic::AtomicBool", "core::sync::atomic::Ordering", "core::ptr::write_volatile", "core::hint::black_box", "core::cell::Cell", "core::mem"]);
    }
}
