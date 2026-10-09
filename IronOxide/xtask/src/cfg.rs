//! Evaluation of `cfg` predicates against a known configuration.
//!
//! The verification job evaluates the `cfg_attr` conditions of third-party
//! crates in the Flight_Build graph against the configuration each crate was
//! actually compiled with: the Target's or host's `rustc --print cfg` output,
//! the crate's enabled features, and the cfgs its build script emitted (all
//! from Cargo's JSON messages). Builds run with empty `RUSTFLAGS`, so no other
//! `--cfg` reaches rustc. Predicates that the evaluator cannot decide (an
//! unknown predicate form such as `version(...)`) evaluate to
//! [`Tri::Unknown`], which every caller treats as possibly active.

use std::collections::BTreeSet;

use proc_macro2::{Delimiter, TokenTree};

use crate::tokens::split_commas;

/// Three-valued truth (Kleene logic).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tri {
    True,
    False,
    Unknown,
}

impl Tri {
    pub fn from_bool(b: bool) -> Tri {
        if b {
            Tri::True
        } else {
            Tri::False
        }
    }

    pub fn and(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::False, _) | (_, Tri::False) => Tri::False,
            (Tri::True, Tri::True) => Tri::True,
            _ => Tri::Unknown,
        }
    }

    pub fn or(self, other: Tri) -> Tri {
        match (self, other) {
            (Tri::True, _) | (_, Tri::True) => Tri::True,
            (Tri::False, Tri::False) => Tri::False,
            _ => Tri::Unknown,
        }
    }

    pub fn negate(self) -> Tri {
        match self {
            Tri::True => Tri::False,
            Tri::False => Tri::True,
            Tri::Unknown => Tri::Unknown,
        }
    }
}

/// A set of active cfg options: bare names (`unix`, `debug_assertions`) and
/// name-value pairs (`target_os = "none"`, `feature = "std"`).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct CfgSet {
    names: BTreeSet<String>,
    pairs: BTreeSet<(String, String)>,
}

impl CfgSet {
    /// Parses the output of `rustc --print cfg`.
    pub fn from_rustc_print(output: &str) -> CfgSet {
        let mut set = CfgSet::default();
        for line in output.lines().map(str::trim).filter(|l| !l.is_empty()) {
            set.insert_spec(line);
        }
        set
    }

    /// Inserts one option written as `name` or `name="value"`, the format of
    /// `rustc --print cfg` and of Cargo's `build-script-executed` messages.
    pub fn insert_spec(&mut self, spec: &str) {
        match spec.split_once('=') {
            Some((name, value)) => {
                let value = value.trim();
                let value = value
                    .strip_prefix('"')
                    .and_then(|v| v.strip_suffix('"'))
                    .unwrap_or(value);
                self.pairs
                    .insert((name.trim().to_string(), value.to_string()));
            }
            None => {
                self.names.insert(spec.trim().to_string());
            }
        }
    }

    pub fn insert_feature(&mut self, feature: &str) {
        self.pairs
            .insert(("feature".to_string(), feature.to_string()));
    }

    pub fn set_name(&mut self, name: &str, on: bool) {
        if on {
            self.names.insert(name.to_string());
        } else {
            self.names.remove(name);
        }
    }

    /// Evaluates a predicate such as `all(docsrs, not(feature = "std"))`.
    pub fn eval(&self, pred: &[TokenTree]) -> Tri {
        match pred {
            [TokenTree::Ident(id)] => match id.to_string().as_str() {
                "true" => Tri::True,
                "false" => Tri::False,
                name => Tri::from_bool(self.names.contains(name)),
            },
            [TokenTree::Ident(id), TokenTree::Punct(eq), TokenTree::Literal(lit)]
                if eq.as_char() == '=' =>
            {
                match unquote(&lit.to_string()) {
                    Some(value) => Tri::from_bool(self.pairs.contains(&(id.to_string(), value))),
                    None => Tri::Unknown,
                }
            }
            [TokenTree::Ident(id), TokenTree::Group(g)]
                if g.delimiter() == Delimiter::Parenthesis =>
            {
                let body: Vec<TokenTree> = g.stream().into_iter().collect();
                let parts: Vec<Vec<TokenTree>> = split_commas(&body)
                    .into_iter()
                    .filter(|p| !p.is_empty())
                    .collect();
                match id.to_string().as_str() {
                    "all" => parts.iter().fold(Tri::True, |acc, p| acc.and(self.eval(p))),
                    "any" => parts.iter().fold(Tri::False, |acc, p| acc.or(self.eval(p))),
                    "not" if parts.len() == 1 => self.eval(&parts[0]).negate(),
                    _ => Tri::Unknown,
                }
            }
            _ => Tri::Unknown,
        }
    }

    /// Evaluates a conjunction of predicates (nested `cfg_attr` conditions).
    pub fn eval_all(&self, preds: &[Vec<TokenTree>]) -> Tri {
        preds.iter().fold(Tri::True, |acc, p| acc.and(self.eval(p)))
    }
}

/// The value of a plain string literal; `None` for literals with escapes or
/// of another kind.
fn unquote(literal: &str) -> Option<String> {
    let raw = literal.strip_prefix('r').map(|r| r.trim_matches('#'));
    let quoted = raw.unwrap_or(literal);
    let value = quoted.strip_prefix('"')?.strip_suffix('"')?;
    if raw.is_none() && value.contains('\\') {
        return None;
    }
    Some(value.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokens::lex_str;

    fn eval(set: &CfgSet, pred: &str) -> Tri {
        set.eval(&lex_str(pred).expect("predicate lexes"))
    }

    fn target() -> CfgSet {
        let mut set = CfgSet::from_rustc_print(
            "debug_assertions\npanic=\"abort\"\ntarget_arch=\"arm\"\ntarget_os=\"none\"\ntarget_has_atomic=\"32\"\n",
        );
        set.insert_feature("default");
        set.insert_spec("wrap_proc_macro");
        set
    }

    #[test]
    fn evaluates_names_pairs_and_features() {
        let set = target();
        assert_eq!(eval(&set, "debug_assertions"), Tri::True);
        assert_eq!(eval(&set, "docsrs"), Tri::False);
        assert_eq!(eval(&set, "target_os = \"none\""), Tri::True);
        assert_eq!(eval(&set, "target_os = \"linux\""), Tri::False);
        assert_eq!(eval(&set, "feature = \"default\""), Tri::True);
        assert_eq!(eval(&set, "feature = \"nightly\""), Tri::False);
        assert_eq!(eval(&set, "wrap_proc_macro"), Tri::True);
        assert_eq!(eval(&set, "true"), Tri::True);
        assert_eq!(eval(&set, "false"), Tri::False);
    }

    #[test]
    fn evaluates_combinators() {
        let set = target();
        assert_eq!(
            eval(&set, "all(target_arch = \"arm\", not(docsrs))"),
            Tri::True
        );
        assert_eq!(eval(&set, "any(docsrs, feature = \"nightly\")"), Tri::False);
        assert_eq!(eval(&set, "all()"), Tri::True);
        assert_eq!(eval(&set, "any()"), Tri::False);
        assert_eq!(eval(&set, "not(any(test, doc))"), Tri::True);
    }

    #[test]
    fn undecidable_predicates_are_unknown_and_propagate() {
        let set = target();
        assert_eq!(eval(&set, "version(\"1.80\")"), Tri::Unknown);
        assert_eq!(eval(&set, "any(docsrs, version(\"1.80\"))"), Tri::Unknown);
        assert_eq!(eval(&set, "all(docsrs, version(\"1.80\"))"), Tri::False);
        assert_eq!(eval(&set, "not(version(\"1.80\"))"), Tri::Unknown);
        assert_eq!(eval(&set, "a::b"), Tri::Unknown);
    }

    #[test]
    fn unquotes_plain_and_raw_strings() {
        assert_eq!(unquote("\"abc\""), Some("abc".to_string()));
        assert_eq!(unquote("r#\"abc\"#"), Some("abc".to_string()));
        assert_eq!(unquote("\"a\\nb\""), None);
        assert_eq!(unquote("42"), None);
    }
}
