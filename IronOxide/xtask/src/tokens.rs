//! Token-level source scanning with `proc-macro2`.
//!
//! The scans walk token trees rather than a syntax tree, so they also see the
//! tokens inside macro invocations (for example `verus! { ... }` bodies once
//! the Kernel logic moves there) and inside `macro_rules!` definitions. They
//! ignore `cfg` conditions on items, which makes them conservative: inactive
//! code is scanned too. Comments are not tokens, doc comments become `doc`
//! attributes whose text is a string literal, and string literals are never
//! matched, so neither produces findings. A raw identifier such as `r#unsafe`
//! is not the keyword and is not reported.
//!
//! Limits: code that a proc-macro or a macro from another crate generates is
//! not visible here; the `unsafe_code` lint covers part of it, and the
//! link-level checks of task 12.3 cover the binary.

use std::path::Path;
use std::str::FromStr;

use proc_macro2::{Delimiter, TokenStream, TokenTree};

/// Attributes that Rust 2024 marks unsafe, plus `naked`. The `unsafe_code`
/// lint rejects the first three under `forbid`, but not a naked function
/// written as `#[unsafe(naked)]` (checked on rustc 1.95.0), so the scan
/// reports all four.
pub const UNSAFE_ATTRIBUTES: &[&str] = &["no_mangle", "export_name", "link_section", "naked"];

/// Assembly macros.
pub const ASM_MACROS: &[&str] = &["asm", "global_asm", "naked_asm"];

/// Lexes a Rust source file into its top-level token trees.
pub fn lex_file(path: &Path) -> Result<Vec<TokenTree>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    lex_str(&text).map_err(|e| format!("cannot lex {}: {e}", path.display()))
}

/// Lexes Rust source text. A byte-order mark and a shebang line are removed
/// first; line numbers stay unchanged.
pub fn lex_str(text: &str) -> Result<Vec<TokenTree>, String> {
    let text = text.strip_prefix('\u{feff}').unwrap_or(text);
    let blanked;
    let text = match text.strip_prefix("#!") {
        Some(rest) if !rest.trim_start().starts_with('[') => {
            let end = text.find('\n').unwrap_or(text.len());
            blanked = format!("{}{}", " ".repeat(end), &text[end..]);
            blanked.as_str()
        }
        _ => text,
    };
    let stream = TokenStream::from_str(text).map_err(|e| e.to_string())?;
    Ok(stream.into_iter().collect())
}

/// The 1-based source line on which a token starts.
pub fn line(tt: &TokenTree) -> usize {
    tt.span().start().line
}

pub fn is_ident(tt: &TokenTree, name: &str) -> bool {
    matches!(tt, TokenTree::Ident(id) if *id == name)
}

fn is_punct(tt: Option<&TokenTree>, ch: char) -> bool {
    matches!(tt, Some(TokenTree::Punct(p)) if p.as_char() == ch)
}

fn group_tokens(tt: Option<&TokenTree>, delimiter: Delimiter) -> Option<Vec<TokenTree>> {
    match tt {
        Some(TokenTree::Group(g)) if g.delimiter() == delimiter => {
            Some(g.stream().into_iter().collect())
        }
        _ => None,
    }
}

fn is_group(tt: Option<&TokenTree>) -> bool {
    matches!(tt, Some(TokenTree::Group(_)))
}

/// Renders tokens back to text for messages, without the spaces that
/// `TokenStream`'s `Display` puts before `(` and `,`.
pub fn render(tokens: &[TokenTree]) -> String {
    tokens
        .iter()
        .cloned()
        .collect::<TokenStream>()
        .to_string()
        .replace(" (", "(")
        .replace(" ,", ",")
}

/// Calls `f` on the top-level token sequence and on the body of every group,
/// at any depth.
pub fn for_each_level(tokens: &[TokenTree], f: &mut dyn FnMut(&[TokenTree])) {
    f(tokens);
    for tt in tokens {
        if let TokenTree::Group(g) = tt {
            let body: Vec<TokenTree> = g.stream().into_iter().collect();
            for_each_level(&body, f);
        }
    }
}

/// Splits a token sequence at its top-level commas. A trailing comma does not
/// produce an empty last part.
pub fn split_commas(tokens: &[TokenTree]) -> Vec<Vec<TokenTree>> {
    let mut parts: Vec<Vec<TokenTree>> = vec![Vec::new()];
    for tt in tokens {
        if matches!(tt, TokenTree::Punct(p) if p.as_char() == ',') {
            parts.push(Vec::new());
        } else if let Some(last) = parts.last_mut() {
            last.push(tt.clone());
        }
    }
    if parts.len() > 1 && parts.last().is_some_and(Vec::is_empty) {
        parts.pop();
    }
    parts
}

/// An attribute: `#[...]` (outer) or `#![...]` (inner).
#[derive(Clone, Debug)]
pub struct Attr {
    pub inner: bool,
    pub content: Vec<TokenTree>,
    pub line: usize,
}

/// Parses an attribute that starts at `level[i]`, returning it and the index
/// after it.
fn attr_at(level: &[TokenTree], i: usize) -> Option<(Attr, usize)> {
    if !is_punct(level.get(i), '#') {
        return None;
    }
    let attr_line = line(&level[i]);
    if is_punct(level.get(i + 1), '!') {
        let content = group_tokens(level.get(i + 2), Delimiter::Bracket)?;
        return Some((
            Attr {
                inner: true,
                content,
                line: attr_line,
            },
            i + 3,
        ));
    }
    let content = group_tokens(level.get(i + 1), Delimiter::Bracket)?;
    Some((
        Attr {
            inner: false,
            content,
            line: attr_line,
        },
        i + 2,
    ))
}

/// The inner attributes at the start of a file (`#![...]` before the first
/// item). These are the attributes that apply to the module the file defines.
pub fn leading_inner_attrs(tokens: &[TokenTree]) -> Vec<Attr> {
    let mut out = Vec::new();
    let mut i = 0;
    while let Some((attr, next)) = attr_at(tokens, i) {
        if !attr.inner {
            break;
        }
        out.push(attr);
        i = next;
    }
    out
}

/// Every attribute at every depth, including attributes written inside macro
/// bodies.
pub fn all_attrs(tokens: &[TokenTree]) -> Vec<Attr> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        let mut i = 0;
        while i < level.len() {
            match attr_at(level, i) {
                Some((attr, next)) => {
                    out.push(attr);
                    i = next;
                }
                None => i += 1,
            }
        }
    });
    out
}

/// One meta item of an attribute after `cfg_attr` and `unsafe(...)` are
/// expanded. `#![cfg_attr(docsrs, feature(doc_cfg))]` yields the meta
/// `feature(doc_cfg)` with the condition `docsrs`.
#[derive(Clone, Debug)]
pub struct Meta {
    /// `cfg_attr` predicates that must all hold for the meta to apply,
    /// outermost first. Empty for an unconditional meta.
    pub conds: Vec<Vec<TokenTree>>,
    /// The meta's path, such as `forbid` or `rustfmt::skip`.
    pub name: String,
    /// The contents of a parenthesized argument list, if any.
    pub args: Option<Vec<TokenTree>>,
    /// Whether the meta was written inside `unsafe(...)`.
    pub unsafe_wrapped: bool,
    pub line: usize,
}

impl Meta {
    /// Whether the argument list names `ident` at its top level.
    pub fn has_arg(&self, ident: &str) -> bool {
        self.args
            .as_deref()
            .is_some_and(|args| args.iter().any(|t| is_ident(t, ident)))
    }

    pub fn conditional(&self) -> bool {
        !self.conds.is_empty()
    }
}

/// Expands an attribute into its meta items.
pub fn metas(attr: &Attr) -> Vec<Meta> {
    let mut out = Vec::new();
    expand(&attr.content, &mut Vec::new(), false, attr.line, &mut out);
    out
}

fn expand(
    content: &[TokenTree],
    conds: &mut Vec<Vec<TokenTree>>,
    unsafe_wrapped: bool,
    line: usize,
    out: &mut Vec<Meta>,
) {
    let Some((name, rest)) = path_name(content) else {
        return;
    };
    let args = group_tokens(rest.first(), Delimiter::Parenthesis);
    match (name.as_str(), &args) {
        ("cfg_attr", Some(args)) => {
            let mut parts = split_commas(args).into_iter();
            let Some(predicate) = parts.next() else {
                return;
            };
            conds.push(predicate);
            for part in parts.filter(|p| !p.is_empty()) {
                expand(&part, conds, unsafe_wrapped, line, out);
            }
            conds.pop();
        }
        ("unsafe", Some(args)) => {
            for part in split_commas(args).into_iter().filter(|p| !p.is_empty()) {
                expand(&part, conds, true, line, out);
            }
        }
        _ => out.push(Meta {
            conds: conds.clone(),
            name,
            args,
            unsafe_wrapped,
            line,
        }),
    }
}

/// Reads a `::`-separated path at the start of `tokens`.
fn path_name(tokens: &[TokenTree]) -> Option<(String, &[TokenTree])> {
    let mut name = String::new();
    let mut i = 0;
    loop {
        let Some(TokenTree::Ident(id)) = tokens.get(i) else {
            return None;
        };
        name.push_str(&id.to_string());
        i += 1;
        if is_punct(tokens.get(i), ':') && is_punct(tokens.get(i + 1), ':') {
            name.push_str("::");
            i += 2;
        } else {
            return Some((name, &tokens[i..]));
        }
    }
}

/// Whether `attrs` contains an unconditional inner attribute
/// `#![<level>(..., lint, ...)]` for one of `levels`.
pub fn has_lint_level(attrs: &[Attr], levels: &[&str], lint: &str) -> bool {
    attrs
        .iter()
        .filter(|a| a.inner)
        .flat_map(metas)
        .any(|m| !m.conditional() && levels.contains(&m.name.as_str()) && m.has_arg(lint))
}

/// The `no_std` metas among the inner attributes, with their conditions.
pub fn no_std_metas(attrs: &[Attr]) -> Vec<Meta> {
    attrs
        .iter()
        .filter(|a| a.inner)
        .flat_map(metas)
        .filter(|m| m.name == "no_std")
        .collect()
}

/// Every `feature(...)` meta in the file, at any depth and inside any
/// `cfg_attr`.
pub fn feature_metas(tokens: &[TokenTree]) -> Vec<Meta> {
    all_attrs(tokens)
        .iter()
        .flat_map(metas)
        .filter(|m| m.name == "feature" && m.args.is_some())
        .collect()
}

/// A construct found by a scan.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub line: usize,
    pub what: String,
}

/// Unsafe constructs: the `unsafe` keyword (blocks, functions, traits,
/// impls, extern blocks, and `unsafe(...)` attributes), unsafe attributes,
/// assembly macros, and attributes that lower the `unsafe_code` lint.
pub fn unsafe_constructs(tokens: &[TokenTree]) -> Vec<Finding> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        for (i, tt) in level.iter().enumerate() {
            let TokenTree::Ident(id) = tt else { continue };
            let name = id.to_string();
            if name == "unsafe" {
                out.push(Finding {
                    line: line(tt),
                    what: "`unsafe` keyword".to_string(),
                });
            } else if ASM_MACROS.contains(&name.as_str())
                && is_punct(level.get(i + 1), '!')
                && is_group(level.get(i + 2))
            {
                out.push(Finding {
                    line: line(tt),
                    what: format!("assembly macro `{name}!`"),
                });
            }
        }
    });
    for attr in all_attrs(tokens) {
        for meta in metas(&attr) {
            if UNSAFE_ATTRIBUTES.contains(&meta.name.as_str()) {
                out.push(Finding {
                    line: meta.line,
                    what: format!("unsafe attribute `{}`", meta.name),
                });
            }
            if ["allow", "warn", "expect"].contains(&meta.name.as_str())
                && meta.has_arg("unsafe_code")
            {
                out.push(Finding {
                    line: meta.line,
                    what: format!("`{}(unsafe_code)`, which lowers the lint level", meta.name),
                });
            }
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

/// `include!(...)` invocations and `#[path = ...]` attributes. Both bring in
/// source text from outside the scanned files.
pub fn source_inclusions(tokens: &[TokenTree]) -> Vec<Finding> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        for (i, tt) in level.iter().enumerate() {
            if is_ident(tt, "include")
                && is_punct(level.get(i + 1), '!')
                && is_group(level.get(i + 2))
            {
                out.push(Finding {
                    line: line(tt),
                    what: "`include!`".to_string(),
                });
            }
        }
    });
    for attr in all_attrs(tokens) {
        for meta in metas(&attr) {
            if meta.name == "path" && meta.args.is_none() {
                out.push(Finding {
                    line: meta.line,
                    what: "`#[path]` attribute".to_string(),
                });
            }
        }
    }
    out.sort_by_key(|f| f.line);
    out
}

/// The condition under which an item is compiled, from its `cfg` attributes.
#[derive(Clone, Debug)]
pub enum ItemCfg {
    /// `#[cfg(<predicate>)]`.
    Pred(Vec<TokenTree>),
    /// A `cfg` inside `cfg_attr`, which the scan does not evaluate.
    Unknown,
}

/// An `extern crate` item.
#[derive(Clone, Debug)]
pub struct ExternCrate {
    pub name: String,
    pub cfgs: Vec<ItemCfg>,
    pub line: usize,
}

/// Every `extern crate` item at any depth, with the `cfg` conditions of its
/// outer attributes.
pub fn extern_crates(tokens: &[TokenTree]) -> Vec<ExternCrate> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        let mut pending: Vec<Attr> = Vec::new();
        let mut i = 0;
        while i < level.len() {
            if let Some((attr, next)) = attr_at(level, i) {
                pending.push(attr);
                i = next;
                continue;
            }
            // Optional visibility before `extern crate`.
            let mut j = i;
            if is_ident(&level[j], "pub") {
                j += 1;
                if matches!(level.get(j), Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Parenthesis)
                {
                    j += 1;
                }
            }
            let is_extern_crate = level.get(j).is_some_and(|t| is_ident(t, "extern"))
                && level.get(j + 1).is_some_and(|t| is_ident(t, "crate"));
            if is_extern_crate {
                if let Some(TokenTree::Ident(name)) = level.get(j + 2) {
                    let cfgs = pending
                        .iter()
                        .filter(|a| !a.inner)
                        .flat_map(metas)
                        .filter(|m| m.name == "cfg")
                        .map(|m| match (m.conditional(), m.args) {
                            (false, Some(pred)) => ItemCfg::Pred(pred),
                            _ => ItemCfg::Unknown,
                        })
                        .collect();
                    out.push(ExternCrate {
                        name: name.to_string(),
                        cfgs,
                        line: line(&level[j]),
                    });
                }
                i = j + 2;
            } else {
                i += 1;
            }
            pending.clear();
        }
    });
    out
}


/// One item or statement of a token level: its outer attributes and the
/// tokens from the first non-attribute token up to and including the first
/// brace group or `;` at this level. Splitting a function body by the same
/// rule gives its statements (an `if`/`else` chain becomes several items,
/// which the users of this type tolerate).
#[derive(Clone, Debug)]
pub struct Item {
    pub attrs: Vec<Attr>,
    pub tokens: Vec<TokenTree>,
}

impl Item {
    /// The `cfg` predicates of the item's outer attributes.
    pub fn cfgs(&self) -> Vec<ItemCfg> {
        self.attrs
            .iter()
            .filter(|a| !a.inner)
            .flat_map(metas)
            .filter(|m| m.name == "cfg")
            .map(|m| match (m.conditional(), m.args) {
                (false, Some(pred)) => ItemCfg::Pred(pred),
                _ => ItemCfg::Unknown,
            })
            .collect()
    }

    /// Whether a `cfg` on the item is false under `cfg`, so that the item is
    /// not compiled in that configuration. An undecidable predicate counts
    /// as compiled (conservative for every scan that looks for violations).
    pub fn gated_out(&self, cfg: &crate::cfg::CfgSet) -> bool {
        self.cfgs()
            .iter()
            .any(|c| matches!(c, ItemCfg::Pred(p) if cfg.eval(p) == crate::cfg::Tri::False))
    }

    /// The 1-based line of the item's first token.
    pub fn line(&self) -> Option<usize> {
        self.tokens.first().map(line)
    }

    /// The identifier after the first `fn` keyword, if any.
    pub fn fn_name(&self) -> Option<String> {
        self.tokens.windows(2).find_map(|w| match (&w[0], &w[1]) {
            (a, TokenTree::Ident(name)) if is_ident(a, "fn") => Some(name.to_string()),
            _ => None,
        })
    }
}

/// Splits one token level into items.
pub fn split_items(level: &[TokenTree]) -> Vec<Item> {
    let mut out = Vec::new();
    let mut i = 0;
    while i < level.len() {
        let mut attrs = Vec::new();
        while let Some((attr, next)) = attr_at(level, i) {
            attrs.push(attr);
            i = next;
        }
        if i >= level.len() {
            if !attrs.is_empty() {
                out.push(Item {
                    attrs,
                    tokens: Vec::new(),
                });
            }
            break;
        }
        let start = i;
        loop {
            let tt = &level[i];
            i += 1;
            let ends = match tt {
                TokenTree::Group(g) => g.delimiter() == Delimiter::Brace,
                TokenTree::Punct(p) => p.as_char() == ';',
                _ => false,
            };
            if ends || i >= level.len() {
                break;
            }
        }
        out.push(Item {
            attrs,
            tokens: level[start..i].to_vec(),
        });
    }
    out
}

/// Calls `f` on the items of the top level and of every group inside items
/// that `cfg` compiles. Items gated out by a `cfg` (for example
/// `#[cfg(kani)]` or `#[cfg(test)]`) are skipped together with their bodies.
pub fn for_each_compiled_level(
    tokens: &[TokenTree],
    cfg: &crate::cfg::CfgSet,
    f: &mut dyn FnMut(&[Item]),
) {
    let items = split_items(tokens);
    f(&items);
    for item in &items {
        if item.gated_out(cfg) {
            continue;
        }
        for tt in &item.tokens {
            if let TokenTree::Group(g) = tt {
                let body: Vec<TokenTree> = g.stream().into_iter().collect();
                for_each_compiled_level(&body, cfg, f);
            }
        }
    }
}

/// The kind of an `unsafe` construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnsafeKind {
    Block,
    Fn,
    Impl,
    Trait,
    ExternBlock,
}

impl UnsafeKind {
    pub fn describe(self) -> &'static str {
        match self {
            UnsafeKind::Block => "`unsafe` block",
            UnsafeKind::Fn => "`unsafe fn`",
            UnsafeKind::Impl => "`unsafe impl`",
            UnsafeKind::Trait => "`unsafe trait`",
            UnsafeKind::ExternBlock => "`unsafe extern` block",
        }
    }
}

/// An `unsafe` construct that a Flight_Build compiles.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct UnsafeItem {
    pub line: usize,
    pub kind: UnsafeKind,
    /// The function name for `unsafe fn`.
    pub name: Option<String>,
}

/// Every `unsafe` block, function, impl, trait, and extern block that `cfg`
/// compiles. `unsafe(...)` attribute wrappers are not constructs of R6.4 and
/// are left to the unsafe-attribute scan.
pub fn flight_unsafe_items(tokens: &[TokenTree], cfg: &crate::cfg::CfgSet) -> Vec<UnsafeItem> {
    let mut out = Vec::new();
    for_each_compiled_level(tokens, cfg, &mut |items| {
        for item in items {
            if item.gated_out(cfg) {
                continue;
            }
            let toks = &item.tokens;
            for (i, tt) in toks.iter().enumerate() {
                if !is_ident(tt, "unsafe") {
                    continue;
                }
                let mut j = i + 1;
                let mut saw_extern = false;
                if toks.get(j).is_some_and(|t| is_ident(t, "extern")) {
                    saw_extern = true;
                    j += 1;
                    if matches!(toks.get(j), Some(TokenTree::Literal(_))) {
                        j += 1;
                    }
                }
                let (kind, name) = match toks.get(j) {
                    // `unsafe fn name` is a function; `unsafe fn(...)` (no
                    // name) is a function-pointer type, which executes
                    // nothing: the call through it is the justified block.
                    Some(t) if is_ident(t, "fn") => match toks.get(j + 1) {
                        Some(TokenTree::Ident(id)) => (UnsafeKind::Fn, Some(id.to_string())),
                        _ => continue,
                    },
                    Some(t) if is_ident(t, "impl") => (UnsafeKind::Impl, None),
                    Some(t) if is_ident(t, "trait") => (UnsafeKind::Trait, None),
                    Some(TokenTree::Group(g)) if g.delimiter() == Delimiter::Brace => (
                        if saw_extern {
                            UnsafeKind::ExternBlock
                        } else {
                            UnsafeKind::Block
                        },
                        None,
                    ),
                    _ => continue,
                };
                out.push(UnsafeItem {
                    line: line(tt),
                    kind,
                    name,
                });
            }
        }
    });
    out.sort_by_key(|u| u.line);
    out
}

/// The names of the functions carrying a `#[kani::proof]` (or
/// `#[kani::proof_for_contract(...)]`) attribute, at any depth and under any
/// `cfg`.
pub fn kani_proof_fns(tokens: &[TokenTree]) -> Vec<String> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        for item in split_items(level) {
            let is_proof = item.attrs.iter().filter(|a| !a.inner).flat_map(metas).any(|m| {
                matches!(
                    m.name.as_str(),
                    "kani::proof" | "kani::proof_for_contract" | "proof" | "proof_for_contract"
                )
            });
            if is_proof {
                if let Some(name) = item.fn_name() {
                    out.push(name);
                }
            }
        }
    });
    out
}

/// The names of every function (`fn`, `proof fn`, `spec fn`) written inside a
/// `verus! { ... }` invocation, at any depth.
pub fn verus_fn_names(tokens: &[TokenTree]) -> Vec<String> {
    let mut out = Vec::new();
    for_each_level(tokens, &mut |level| {
        for (i, tt) in level.iter().enumerate() {
            if is_ident(tt, "verus") && is_punct(level.get(i + 1), '!') {
                if let Some(body) = group_tokens(level.get(i + 2), Delimiter::Brace) {
                    for_each_level(&body, &mut |inner| {
                        for item in split_items(inner) {
                            if let Some(name) = item.fn_name() {
                                out.push(name);
                            }
                        }
                    });
                }
            }
        }
    });
    out
}

/// Whether `header` (the tokens of an item before its body) invokes the
/// `verus!` macro.
pub fn is_verus_invocation(header: &[TokenTree]) -> bool {
    header
        .windows(2)
        .any(|w| is_ident(&w[0], "verus") && matches!(&w[1], TokenTree::Punct(p) if p.as_char() == '!'))
        && header.len() == 2
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cfg::CfgSet;

    fn lex(src: &str) -> Vec<TokenTree> {
        lex_str(src).expect("test source lexes")
    }

    fn whats(src: &str) -> Vec<String> {
        unsafe_constructs(&lex(src))
            .into_iter()
            .map(|f| f.what)
            .collect()
    }

    #[test]
    fn detects_unconditional_forbid_only() {
        let attrs = leading_inner_attrs(&lex(
            "//! doc\n#![no_std]\n#![forbid(unsafe_code, missing_docs)]\nmod a;",
        ));
        assert!(has_lint_level(&attrs, &["forbid"], "unsafe_code"));

        let conditional = leading_inner_attrs(&lex("#![cfg_attr(not(test), forbid(unsafe_code))]"));
        assert!(!has_lint_level(&conditional, &["forbid"], "unsafe_code"));

        // An attribute after the first item does not apply to the module.
        let late = leading_inner_attrs(&lex("mod a;\n#![forbid(unsafe_code)]"));
        assert!(!has_lint_level(&late, &["forbid"], "unsafe_code"));
    }

    #[test]
    fn finds_unsafe_inside_macro_bodies() {
        let found = whats("verus! { pub fn f() { unsafe { core::ptr::null::<u8>().read() }; } }");
        assert_eq!(found, vec!["`unsafe` keyword"]);
    }

    #[test]
    fn finds_unsafe_attributes_and_assembly() {
        let found = whats(
            "#[no_mangle] fn a() {}\n#[unsafe(export_name = \"b\")] fn b() {}\n\
             #[cfg_attr(target_os = \"none\", link_section = \".x\")] static C: u8 = 0;\n\
             core::arch::global_asm!(\"\");\n#[unsafe(naked)] extern \"C\" fn d() { core::arch::naked_asm!(\"bx lr\") }\n\
             #[allow(unsafe_code)] fn e() {}",
        );
        for expected in [
            "unsafe attribute `no_mangle`",
            "unsafe attribute `export_name`",
            "unsafe attribute `link_section`",
            "unsafe attribute `naked`",
            "assembly macro `global_asm!`",
            "assembly macro `naked_asm!`",
            "`allow(unsafe_code)`, which lowers the lint level",
        ] {
            assert!(
                found.iter().any(|f| f == expected),
                "missing {expected}: {found:?}"
            );
        }
    }

    #[test]
    fn ignores_comments_strings_raw_identifiers_and_lint_names() {
        let found = whats(
            "#![forbid(unsafe_code)]\n// unsafe { }\n/// unsafe in docs\nfn f() { let r#unsafe = \"unsafe { asm!() }\"; let asm = 1; let _ = asm != 2; }",
        );
        assert!(found.is_empty(), "{found:?}");
    }

    #[test]
    fn finds_feature_attributes_including_nested_cfg_attr() {
        let metas = feature_metas(&lex(
            "#![feature(never_type)]\n#![cfg_attr(docsrs, feature(doc_cfg))]\n\
             #![cfg_attr(a, cfg_attr(b, feature(x)))]\n#[cfg(feature = \"std\")] fn f() {}",
        ));
        let summary: Vec<(usize, String)> = metas
            .iter()
            .map(|m| (m.conds.len(), render(m.args.as_deref().unwrap_or(&[]))))
            .collect();
        assert_eq!(
            summary,
            vec![
                (0, "never_type".to_string()),
                (1, "doc_cfg".to_string()),
                (2, "x".to_string())
            ]
        );
    }

    #[test]
    fn finds_extern_crates_with_their_cfg() {
        let found = extern_crates(&lex(
            "#![no_std]\n#[cfg(feature = \"std\")]\nextern crate std;\nextern crate alloc as a;\nmod m { pub extern crate core; }",
        ));
        let names: Vec<(&str, usize)> = found
            .iter()
            .map(|e| (e.name.as_str(), e.cfgs.len()))
            .collect();
        assert_eq!(names, vec![("std", 1), ("alloc", 0), ("core", 0)]);
    }

    #[test]
    fn finds_source_inclusions() {
        let found = source_inclusions(&lex(
            "#[path = \"x.rs\"] mod x;\ninclude!(\"y.rs\");\nconst S: &str = include_str!(\"z.txt\");",
        ));
        let whats: Vec<&str> = found.iter().map(|f| f.what.as_str()).collect();
        assert_eq!(whats, vec!["`#[path]` attribute", "`include!`"]);
    }

    #[test]
    fn reports_line_numbers_and_survives_a_shebang() {
        let found = unsafe_constructs(&lex(
            "#!/usr/bin/env run-cargo-script\nfn a() {}\n\nunsafe fn b() {}",
        ));
        assert_eq!(
            found,
            vec![Finding {
                line: 4,
                what: "`unsafe` keyword".to_string()
            }]
        );
    }

    #[test]
    fn finds_flight_unsafe_items_and_skips_cfg_gated_ones() {
        let cfg = CfgSet::from_rustc_print("target_os=\"none\"\n");
        let found = flight_unsafe_items(
            &lex(
                "pub unsafe fn a(p: *const u8) -> u8 { unsafe { *p } }\n\
                 unsafe impl Sync for X {}\n\
                 #[cfg(kani)] mod h { fn k() { unsafe { a(core::ptr::null()) }; } }\n\
                 #[cfg(test)] unsafe fn t() {}\n\
                 #[cfg(target_os = \"none\")] fn f() { let _ = unsafe { a(core::ptr::null()) }; }\n\
                 #[cfg(not(kani))] unsafe extern \"C\" { fn ext(); }\n\
                 #[unsafe(no_mangle)] static S: u8 = 0;\n\
                 static V: unsafe extern \"C\" fn() = a_entry; union U { h: unsafe fn(u8) -> u8, w: u32 }",
            ),
            &cfg,
        );
        let summary: Vec<(usize, UnsafeKind)> = found.iter().map(|u| (u.line, u.kind)).collect();
        assert_eq!(
            summary,
            vec![
                (1, UnsafeKind::Fn),
                (1, UnsafeKind::Block),
                (2, UnsafeKind::Impl),
                (5, UnsafeKind::Block),
                (6, UnsafeKind::ExternBlock),
            ]
        );
        assert_eq!(found[0].name.as_deref(), Some("a"));
    }

    #[test]
    fn finds_kani_harnesses_and_verus_functions() {
        let src = "#[cfg(kani)] mod h { #[kani::proof] fn uj_001() {} #[kani::proof_for_contract(f)] fn uj_002() {} fn helper() {} }\n\
                   verus! { pub proof fn lemma_a() {} impl X { pub fn exec_b(&self) {} } } fn outside() {}";
        assert_eq!(kani_proof_fns(&lex(src)), vec!["uj_001", "uj_002"]);
        assert_eq!(verus_fn_names(&lex(src)), vec!["lemma_a", "exec_b"]);
    }
}
