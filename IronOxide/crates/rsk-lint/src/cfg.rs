//! `#[cfg(...)]` evaluation for a Flight_Build: `test`, `kani`, `doc`,
//! `miri`, and `debug_assertions` are false; the Target is `arm`/`none`;
//! every feature is taken as enabled (the lint covers every feature-gated
//! path, which over-approximates any one build); unknown predicates are
//! true for the same reason.

use proc_macro2::{Delimiter, TokenStream, TokenTree};

pub fn eval(tokens: &TokenStream) -> bool {
    let v: Vec<TokenTree> = tokens.clone().into_iter().collect();
    eval_list(&v).unwrap_or(true)
}

fn eval_list(v: &[TokenTree]) -> Option<bool> {
    let mut i = 0;
    let mut result = None;
    while i < v.len() {
        let TokenTree::Ident(id) = &v[i] else { i += 1; continue };
        let name = id.to_string();
        let value = match v.get(i + 1) {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis => {
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                match name.as_str() {
                    "not" => !eval_list(&inner).unwrap_or(true),
                    "all" => split(&inner).iter().all(|p| eval_list(p).unwrap_or(true)),
                    "any" => split(&inner).iter().any(|p| eval_list(p).unwrap_or(true)),
                    _ => true,
                }
            }
            Some(TokenTree::Punct(p)) if p.as_char() == '=' => {
                let lit = v.get(i + 2).map(|t| t.to_string().trim_matches('"').to_string()).unwrap_or_default();
                match name.as_str() {
                    "feature" => true,
                    "target_arch" => lit == "arm",
                    "target_os" => lit == "none",
                    "target_env" => lit.is_empty(),
                    "target_pointer_width" => lit == "32",
                    "target_endian" => lit == "little",
                    "target_has_atomic" => true,
                    _ => true,
                }
            }
            _ => {
                !matches!(name.as_str(), "test" | "kani" | "doc" | "doctest" | "miri" | "debug_assertions" | "verus_keep_ghost" | "verus_verify_core" | "verus_keep_ghost_body")
            }
        };
        result = Some(value);
        break;
    }
    result
}

fn split(v: &[TokenTree]) -> Vec<Vec<TokenTree>> {
    let mut out = vec![Vec::new()];
    for t in v {
        match t {
            TokenTree::Punct(p) if p.as_char() == ',' => out.push(Vec::new()),
            other => out.last_mut().unwrap().push(other.clone()),
        }
    }
    out.retain(|x| !x.is_empty());
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    fn e(s: &str) -> bool {
        eval(&s.parse().unwrap())
    }
    #[test]
    fn predicates() {
        assert!(!e("test"));
        assert!(!e("kani"));
        assert!(e("feature = \"trace\""));
        assert!(e("all(target_arch = \"arm\", target_os = \"none\")"));
        assert!(!e("not(all(target_arch = \"arm\", target_os = \"none\"))"));
        assert!(!e("any(kani, test, not(all(target_arch = \"arm\", target_os = \"none\")))"));
        assert!(e("not(feature = \"board-qemu-mps2\")") == false);
    }
}
