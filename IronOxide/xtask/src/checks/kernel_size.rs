//! R6.2 and PAR-01: the Kernel's executable line count.
//!
//! PAR-01 counts executable Rust lines of the Kernel source built for the
//! Target, excluding blank lines, comments, Verus spec and proof code, and
//! generated code. The count here is computed from the token stream of every
//! library file of every Kernel crate (`kernel = true` in the build policy),
//! by these rules:
//!
//! - A line counts when at least one token on it belongs to executable code.
//!   A line that mixes executable and spec tokens counts.
//! - Comments and blank lines carry no tokens. Attributes (including doc
//!   comments) are not executable.
//! - Inside `verus! { ... }`: `spec fn`, `proof fn`, `broadcast`, `axiom`,
//!   `assume_specification`, `global`, and `ghost`/`tracked` items are spec
//!   or proof code; so are the `requires`, `ensures`, `recommends`,
//!   `decreases`, `returns`, `opens_invariants`, `no_unwind`, and loop
//!   `invariant` clauses, `proof { }` blocks, `assert`/`assume` statements
//!   (with their `by` proofs), `let ghost`/`let tracked` bindings, `reveal`,
//!   `hide`, `broadcast use`, and `calc!`. Everything else inside `verus!`
//!   and everything outside it is executable.
//! - Items under a `cfg` that is false for the Target (`#[cfg(kani)]`,
//!   `#[cfg(test)]`) are not built for the Target and are not counted.
//! - A line whose text consists only of delimiters and punctuation
//!   (`}`, `);`, `},`) is layout, as in Verus's `line_count` with
//!   `--delimiters-are-layout`, and does not count.
//! - A file whose first three lines contain `@generated` is generated code
//!   and is not counted.
//!
//! When the Verus install is available, the verification job also runs
//! Verus's own `line_count` tool on the same files and warns if its
//! executable count exceeds this one (the two tools classify a few
//! constructs differently; this counter is the gate because it needs no
//! external tool, and it errs on the side of counting).

use std::path::Path;

use proc_macro2::{Delimiter, TokenTree};

use crate::cfg::CfgSet;
use crate::diag::{Report, Rule};
use crate::tokens::{is_verus_invocation, lex_str, split_items, Item};
use crate::workspace::Workspace;

/// Classification of a source line.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Class {
    None,
    Spec,
    Exec,
}

/// Function-header and loop-header clauses that belong to the specification.
const CLAUSES: &[&str] = &[
    "requires",
    "ensures",
    "recommends",
    "decreases",
    "returns",
    "opens_invariants",
    "no_unwind",
    "unwind",
    "invariant",
    "invariant_except_break",
    "invariant_ensures",
    "default_ensures",
];

/// Idents that start a spec or proof item inside `verus!`.
const SPEC_ITEM_STARTS: &[&str] = &[
    "broadcast",
    "axiom",
    "global",
    "ghost",
    "tracked",
    "assume_specification",
];

#[derive(Debug, Default)]
pub struct FileCount {
    pub exec: usize,
    pub spec: usize,
}

struct Counter<'a> {
    cfg: &'a CfgSet,
    classes: Vec<Class>,
}

impl<'a> Counter<'a> {
    fn mark_lines(&mut self, first: usize, last: usize, class: Class) {
        if last >= self.classes.len() {
            self.classes.resize(last + 1, Class::None);
        }
        for l in first..=last {
            if class > self.classes[l] {
                self.classes[l] = class;
            }
        }
    }

    /// Marks every line that the token tree spans.
    fn mark_tree(&mut self, tt: &TokenTree, class: Class) {
        let span = tt.span();
        self.mark_lines(span.start().line, span.end().line, class);
    }

    /// Marks only the lines of a group's delimiters.
    fn mark_delims(&mut self, g: &proc_macro2::Group, class: Class) {
        let open = g.span_open().start().line;
        let close = g.span_close().start().line;
        self.mark_lines(open, open, class);
        self.mark_lines(close, close, class);
    }

    fn mark_all(&mut self, toks: &[TokenTree], class: Class) {
        for tt in toks {
            self.mark_tree(tt, class);
        }
    }

    fn items(&mut self, level: &[TokenTree], in_verus: bool) {
        for item in split_items(level) {
            if item.gated_out(self.cfg) || item.tokens.is_empty() {
                continue;
            }
            self.item(&item, in_verus);
        }
    }

    fn item(&mut self, item: &Item, in_verus: bool) {
        let toks = &item.tokens;
        let (header, body) = match toks.last() {
            Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => {
                (&toks[..toks.len() - 1], Some(g))
            }
            _ => (&toks[..], None),
        };
        if let (true, Some(g)) = (is_verus_invocation(header), body) {
            self.mark_all(header, Class::Exec);
            self.mark_delims(g, Class::Exec);
            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
            self.items(&inner, true);
            return;
        }
        if in_verus && is_spec_item(header) {
            self.mark_all(toks, Class::Spec);
            return;
        }
        let idents: Vec<String> = header
            .iter()
            .filter_map(|t| match t {
                TokenTree::Ident(id) => Some(id.to_string()),
                _ => None,
            })
            .collect();
        if idents.iter().any(|i| i == "fn") {
            let mut spec = false;
            for t in header {
                if in_verus && matches!(t, TokenTree::Ident(id) if CLAUSES.contains(&id.to_string().as_str())) {
                    spec = true;
                }
                self.mark_tree(t, if spec { Class::Spec } else { Class::Exec });
            }
            if let Some(g) = body {
                self.mark_delims(g, Class::Exec);
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                self.body(&inner, in_verus);
            }
            return;
        }
        if idents
            .iter()
            .any(|i| matches!(i.as_str(), "impl" | "trait" | "mod"))
        {
            self.mark_all(header, Class::Exec);
            if let Some(g) = body {
                self.mark_delims(g, Class::Exec);
                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                self.items(&inner, in_verus);
            }
            return;
        }
        self.mark_all(toks, Class::Exec);
    }

    fn body(&mut self, level: &[TokenTree], in_verus: bool) {
        for item in split_items(level) {
            if item.gated_out(self.cfg) {
                continue;
            }
            if in_verus {
                self.stmt(&item.tokens);
            } else {
                self.mark_all(&item.tokens, Class::Exec);
            }
        }
    }

    /// One statement (or statement fragment) inside `verus!` executable code.
    fn stmt(&mut self, toks: &[TokenTree]) {
        let ident = |i: usize| -> Option<String> {
            match toks.get(i) {
                Some(TokenTree::Ident(id)) => Some(id.to_string()),
                _ => None,
            }
        };
        let group = |i: usize, d: Delimiter| -> bool {
            matches!(toks.get(i), Some(TokenTree::Group(g)) if g.delimiter() == d)
        };
        let semi = |i: usize| -> bool { matches!(toks.get(i), Some(TokenTree::Punct(p)) if p.as_char() == ';') };
        let mut i = 0;
        while i < toks.len() {
            let name = ident(i).unwrap_or_default();
            match name.as_str() {
                "proof" if group(i + 1, Delimiter::Brace) => {
                    self.mark_tree(&toks[i], Class::Spec);
                    self.mark_tree(&toks[i + 1], Class::Spec);
                    i += 2;
                }
                "assert" | "assume" if group(i + 1, Delimiter::Parenthesis) => {
                    self.mark_tree(&toks[i], Class::Spec);
                    self.mark_tree(&toks[i + 1], Class::Spec);
                    i += 2;
                    if ident(i).as_deref() == Some("by") {
                        self.mark_tree(&toks[i], Class::Spec);
                        i += 1;
                        if group(i, Delimiter::Parenthesis) {
                            self.mark_tree(&toks[i], Class::Spec);
                            i += 1;
                        }
                        if group(i, Delimiter::Brace) {
                            self.mark_tree(&toks[i], Class::Spec);
                            i += 1;
                        } else {
                            // `by (nonlinear_arith) requires ...;`
                            while i < toks.len() && !semi(i) {
                                self.mark_tree(&toks[i], Class::Spec);
                                i += 1;
                            }
                        }
                    }
                    if semi(i) {
                        self.mark_tree(&toks[i], Class::Spec);
                        i += 1;
                    }
                }
                "assert" if matches!(ident(i + 1).as_deref(), Some("forall" | "exists")) => {
                    while i < toks.len() {
                        self.mark_tree(&toks[i], Class::Spec);
                        let done = group(i, Delimiter::Brace) || semi(i);
                        i += 1;
                        if done {
                            break;
                        }
                    }
                }
                "let" if matches!(ident(i + 1).as_deref(), Some("ghost" | "tracked")) => {
                    while i < toks.len() {
                        self.mark_tree(&toks[i], Class::Spec);
                        let done = semi(i);
                        i += 1;
                        if done {
                            break;
                        }
                    }
                }
                "reveal" | "reveal_with_fuel" | "hide" if group(i + 1, Delimiter::Parenthesis) => {
                    self.mark_tree(&toks[i], Class::Spec);
                    self.mark_tree(&toks[i + 1], Class::Spec);
                    i += 2;
                    if semi(i) {
                        self.mark_tree(&toks[i], Class::Spec);
                        i += 1;
                    }
                }
                "broadcast" if ident(i + 1).as_deref() == Some("use") => {
                    while i < toks.len() {
                        self.mark_tree(&toks[i], Class::Spec);
                        let done = semi(i);
                        i += 1;
                        if done {
                            break;
                        }
                    }
                }
                "calc"
                    if matches!(toks.get(i + 1), Some(TokenTree::Punct(p)) if p.as_char() == '!')
                        && matches!(toks.get(i + 2), Some(TokenTree::Group(_))) =>
                {
                    for t in &toks[i..i + 3] {
                        self.mark_tree(t, Class::Spec);
                    }
                    i += 3;
                }
                "while" | "loop" | "for" => {
                    self.mark_tree(&toks[i], Class::Exec);
                    i += 1;
                    let mut in_clause = false;
                    while i < toks.len() {
                        match &toks[i] {
                            TokenTree::Group(g) if g.delimiter() == Delimiter::Brace => {
                                self.mark_delims(g, Class::Exec);
                                let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                                self.body(&inner, true);
                                i += 1;
                                break;
                            }
                            TokenTree::Ident(id) if CLAUSES.contains(&id.to_string().as_str()) => {
                                in_clause = true;
                                self.mark_tree(&toks[i], Class::Spec);
                            }
                            t => {
                                let class = if in_clause { Class::Spec } else { Class::Exec };
                                self.mark_tree(t, class);
                            }
                        }
                        i += 1;
                    }
                }
                _ => {
                    match &toks[i] {
                        TokenTree::Group(g) => {
                            self.mark_delims(g, Class::Exec);
                            let inner: Vec<TokenTree> = g.stream().into_iter().collect();
                            self.body(&inner, true);
                        }
                        t => self.mark_tree(t, Class::Exec),
                    }
                    i += 1;
                }
            }
        }
    }
}

/// Whether an item header inside `verus!` starts a spec or proof item.
fn is_spec_item(header: &[TokenTree]) -> bool {
    let idents: Vec<String> = header
        .iter()
        .filter_map(|t| match t {
            TokenTree::Ident(id) => Some(id.to_string()),
            _ => None,
        })
        .collect();
    if let Some(fn_pos) = idents.iter().position(|i| i == "fn") {
        if idents[..fn_pos]
            .iter()
            .any(|i| i == "spec" || i == "proof")
        {
            return true;
        }
    }
    // Visibility (`pub`, `pub(crate)`) may precede the keyword.
    let first = idents.iter().find(|i| *i != "pub" && *i != "crate" && *i != "super" && *i != "self");
    first.is_some_and(|i| SPEC_ITEM_STARTS.contains(&i.as_str()))
}

/// Whether a source line is layout only: nothing but delimiters, commas, and
/// semicolons.
fn is_layout(text: &str) -> bool {
    let code = text.split_once("//").map_or(text, |(code, _)| code);
    let t = code.trim();
    !t.is_empty() && t.chars().all(|c| "{}()[];,".contains(c))
}

/// Whether the file is marked as generated.
pub fn is_generated(text: &str) -> bool {
    text.lines().take(3).any(|l| l.contains("@generated"))
}

/// Counts the executable and spec lines of one source text.
pub fn count_text(text: &str, cfg: &CfgSet) -> Result<FileCount, String> {
    let tokens = lex_str(text)?;
    let mut counter = Counter {
        cfg,
        classes: Vec::new(),
    };
    counter.items(&tokens, false);
    let mut count = FileCount::default();
    for (n, text) in text.lines().enumerate() {
        let class = counter.classes.get(n + 1).copied().unwrap_or(Class::None);
        match class {
            Class::Exec if !is_layout(text) => count.exec += 1,
            Class::Spec => count.spec += 1,
            _ => {}
        }
    }
    Ok(count)
}

pub fn count_file(path: &Path, cfg: &CfgSet) -> Result<Option<FileCount>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    if is_generated(&text) {
        return Ok(None);
    }
    count_text(&text, cfg)
        .map(Some)
        .map_err(|e| format!("cannot lex {}: {e}", path.display()))
}

/// The Target configuration used to decide which items are built for the
/// Target: `target_os = "none"` and `target_arch = "arm"` (so that
/// `#[cfg(all(target_arch = "arm", target_os = "none"))]` modules count),
/// nothing else (so that `#[cfg(kani)]`, `#[cfg(test)]`, and feature gates
/// do not).
pub fn target_cfg(ws: &Workspace) -> CfgSet {
    let mut set = CfgSet::from_rustc_print("target_os=\"none\"\ntarget_arch=\"arm\"\n");
    if ws.policy.target.starts_with("riscv") {
        set = CfgSet::from_rustc_print("target_os=\"none\"\ntarget_arch=\"riscv32\"\n");
    }
    set
}

/// The source files counted for a Kernel crate: its library files.
pub fn kernel_sources(ws: &Workspace) -> Vec<(String, std::path::PathBuf)> {
    let mut files = Vec::new();
    for name in ws.policy.kernel_crates() {
        if let Some(pkg) = ws.package_named(name) {
            for file in pkg.lib_files() {
                files.push((pkg.name.clone(), file));
            }
        }
    }
    files
}

pub fn check(ws: &Workspace, report: &mut Report) {
    let cfg = target_cfg(ws);
    let mut total = FileCount::default();
    let mut per_crate: Vec<(String, usize)> = Vec::new();
    let mut generated = 0usize;
    for (crate_name, file) in kernel_sources(ws) {
        match count_file(&file, &cfg) {
            Ok(Some(count)) => {
                total.exec += count.exec;
                total.spec += count.spec;
                match per_crate.iter_mut().find(|(n, _)| *n == crate_name) {
                    Some((_, n)) => *n += count.exec,
                    None => per_crate.push((crate_name.clone(), count.exec)),
                }
            }
            Ok(None) => generated += 1,
            Err(e) => report.error(Rule::KernelSize, Some(ws.rel(&file)), e),
        }
    }
    let limit = ws.policy.max_exec_lines;
    let breakdown: Vec<String> = per_crate.iter().map(|(n, c)| format!("{n}: {c}")).collect();
    report.note(format!(
        "R6.2/PAR-01: the Kernel has {} executable lines (limit {limit}; {} spec/proof lines not counted; \
         {generated} generated files skipped){}",
        total.exec,
        total.spec,
        if breakdown.is_empty() {
            String::new()
        } else {
            format!(" [{}]", breakdown.join(", "))
        }
    ));
    if total.exec as u64 > limit {
        report.error(
            Rule::KernelSize,
            None,
            format!(
                "the Kernel has {} executable lines, which exceeds the PAR-01 limit of {limit}",
                total.exec
            ),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cfg() -> CfgSet {
        CfgSet::from_rustc_print("target_os=\"none\"\ntarget_arch=\"arm\"\n")
    }

    #[test]
    fn counts_executable_lines_and_excludes_spec_proof_comments_and_layout() {
        let src = "\
//! doc
#![no_std]
use core::fmt;              // exec (use counts)

verus! {                    // exec (macro line)
pub struct S { pub n: u8 }  // exec

impl S {
    pub open spec fn inv(&self) -> bool { self.n < 10 }   // spec

    pub fn bump(&mut self)        // exec
        requires old(self).inv(), // spec
        ensures final(self).n <= 10, // spec
    {                              // layout
        proof { assert(self.n < 10); }  // spec
        let ghost g = self.n;      // spec
        self.n = self.n + 1;       // exec
        assert(self.n <= 10);      // spec
        while self.n > 0           // exec
            invariant self.n <= 10, // spec
            decreases self.n,      // spec
        {
            self.n = self.n - 1;   // exec
        }
    }
}

pub proof fn lemma(x: u8) ensures x >= 0 {}   // spec
}

#[cfg(kani)]
mod harness { fn h() { let _ = 1; } }       // not built for the Target
";
        let count = count_text(src, &cfg()).expect("lexes");
        // use, verus!, struct, impl, fn bump, self.n = +1, while, self.n = -1
        assert_eq!(count.exec, 8, "{count:?}");
        assert_eq!(count.spec, 9, "{count:?}");
    }

    #[test]
    fn code_outside_verus_counts_entirely() {
        let src = "pub fn f(a: u32) -> u32 {\n    a.wrapping_add(1)\n}\n#[inline]\nfn g() {}\n";
        let count = count_text(src, &cfg()).expect("lexes");
        assert_eq!(count.exec, 3);
    }

    #[test]
    fn generated_files_are_recognized() {
        assert!(is_generated("// @generated by cargo xtask profile\npub const X: u8 = 1;\n"));
        assert!(!is_generated("//! hand written\n"));
    }
}
