//! The syntax-directed checks over one parsed file.

use proc_macro2::TokenStream;
use quote::ToTokens;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Attribute, Expr, Item, Meta};

use crate::core_subset::{lookup, Item as CoreItem};
use crate::{cfg, config, verus, Finding, LoopRecord, Scope};

struct FnCtx {
    name: String,
    /// `#[rsk::bounds]`: label -> (bound text, constant?).
    bounds: Vec<(String, String, bool)>,
    used: Vec<String>,
    is_method: bool,
}

pub struct Linter<'a> {
    file: String,
    scope: Scope,
    items: &'a [CoreItem],
    fns: Vec<FnCtx>,
    pub findings: Vec<Finding>,
    pub loops: Vec<LoopRecord>,
}

fn line_of<T: Spanned>(t: &T) -> usize {
    t.span().start().line
}

/// R35.5: an integer literal, a constant path (`N`, `Config::MAX`,
/// `u32::MAX`), a cast, or arithmetic over those.
pub fn constant_expr(e: &Expr) -> bool {
    match e {
        Expr::Lit(l) => matches!(l.lit, syn::Lit::Int(_)),
        Expr::Path(p) => p.path.segments.last().is_some_and(|s| {
            let name = s.ident.to_string();
            name.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
        }),
        Expr::Paren(p) => constant_expr(&p.expr),
        Expr::Group(g) => constant_expr(&g.expr),
        Expr::Cast(c) => constant_expr(&c.expr),
        Expr::Unary(u) => constant_expr(&u.expr),
        Expr::Binary(b) => {
            matches!(b.op, syn::BinOp::Add(_) | syn::BinOp::Sub(_) | syn::BinOp::Mul(_) | syn::BinOp::Div(_) | syn::BinOp::Rem(_) | syn::BinOp::Shl(_) | syn::BinOp::Shr(_))
                && constant_expr(&b.left)
                && constant_expr(&b.right)
        }
        Expr::MethodCall(m) if m.method == "len" && m.args.is_empty() => constant_expr(&m.receiver),
        _ => false,
    }
}

fn text(e: &Expr) -> String {
    e.to_token_stream().to_string().replace(" :: ", "::").replace(" ..", "..").replace(".. ", "..")
}

/// A `while` condition is bounded when some conjunct is `i < N` or
/// `i <= N` (or `N > i`, `N >= i`) with a local variable `i` and a
/// constant `N`: a counter against a constant. A comparison of a
/// computed value (`now() - start < N`) is not an iteration count.
fn bounded_condition(e: &Expr) -> Option<String> {
    fn local(e: &Expr) -> bool {
        match e {
            Expr::Path(p) => p.path.segments.len() == 1 && !constant_expr(e),
            Expr::Cast(c) => local(&c.expr),
            Expr::Paren(p) => local(&p.expr),
            _ => false,
        }
    }
    match e {
        Expr::Paren(p) => bounded_condition(&p.expr),
        Expr::Group(g) => bounded_condition(&g.expr),
        Expr::Binary(b) => match b.op {
            syn::BinOp::And(_) => bounded_condition(&b.left).or_else(|| bounded_condition(&b.right)),
            syn::BinOp::Lt(_) | syn::BinOp::Le(_) if local(&b.left) && constant_expr(&b.right) => Some(text(&b.right)),
            syn::BinOp::Gt(_) | syn::BinOp::Ge(_) if local(&b.right) && constant_expr(&b.left) => Some(text(&b.left)),
            _ => None,
        },
        _ => None,
    }
}

/// A `for` iterator with a compile-time iteration count: a range of
/// constants, or an array literal.
fn bounded_iterator(e: &Expr) -> Option<String> {
    match e {
        Expr::Paren(p) => bounded_iterator(&p.expr),
        Expr::Reference(r) => bounded_iterator(&r.expr),
        Expr::Range(r) => match (&r.start, &r.end) {
            (Some(s), Some(end)) if constant_expr(s) && constant_expr(end) => Some(if text(s) == "0" { text(end) } else { format!("{} - {}", text(end), text(s)) }),
            (None, Some(end)) if constant_expr(end) => Some(text(end)),
            _ => None,
        },
        Expr::Array(a) => Some(a.elems.len().to_string()),
        Expr::MethodCall(m) if matches!(m.method.to_string().as_str(), "iter" | "iter_mut" | "into_iter" | "rev") && m.args.is_empty() => bounded_iterator(&m.receiver),
        Expr::Path(_) if constant_expr(e) => Some(format!("{}.len()", text(e))),
        _ => None,
    }
}

fn cfg_excludes(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| match &a.meta {
        Meta::List(l) if l.path.is_ident("cfg") => !cfg::eval(&l.tokens),
        _ => false,
    })
}

/// The `mod x;` declarations of a file whose `cfg` excludes them from the
/// Flight_Build.
pub fn gated_modules(file: &syn::File) -> Vec<String> {
    let mut out = Vec::new();
    for i in &file.items {
        if let Item::Mod(m) = i {
            if m.content.is_none() && cfg_excludes(&m.attrs) {
                out.push(m.ident.to_string());
            }
        }
    }
    out
}

fn item_attrs(i: &Item) -> &[Attribute] {
    match i {
        Item::Const(x) => &x.attrs,
        Item::Enum(x) => &x.attrs,
        Item::ExternCrate(x) => &x.attrs,
        Item::Fn(x) => &x.attrs,
        Item::ForeignMod(x) => &x.attrs,
        Item::Impl(x) => &x.attrs,
        Item::Macro(x) => &x.attrs,
        Item::Mod(x) => &x.attrs,
        Item::Static(x) => &x.attrs,
        Item::Struct(x) => &x.attrs,
        Item::Trait(x) => &x.attrs,
        Item::TraitAlias(x) => &x.attrs,
        Item::Type(x) => &x.attrs,
        Item::Union(x) => &x.attrs,
        Item::Use(x) => &x.attrs,
        _ => &[],
    }
}

fn expr_attrs(e: &Expr) -> &[Attribute] {
    match e {
        Expr::Array(x) => &x.attrs,
        Expr::Assign(x) => &x.attrs,
        Expr::Async(x) => &x.attrs,
        Expr::Await(x) => &x.attrs,
        Expr::Binary(x) => &x.attrs,
        Expr::Block(x) => &x.attrs,
        Expr::Break(x) => &x.attrs,
        Expr::Call(x) => &x.attrs,
        Expr::Cast(x) => &x.attrs,
        Expr::Closure(x) => &x.attrs,
        Expr::Const(x) => &x.attrs,
        Expr::Continue(x) => &x.attrs,
        Expr::Field(x) => &x.attrs,
        Expr::ForLoop(x) => &x.attrs,
        Expr::Group(x) => &x.attrs,
        Expr::If(x) => &x.attrs,
        Expr::Index(x) => &x.attrs,
        Expr::Infer(x) => &x.attrs,
        Expr::Let(x) => &x.attrs,
        Expr::Lit(x) => &x.attrs,
        Expr::Loop(x) => &x.attrs,
        Expr::Macro(x) => &x.attrs,
        Expr::Match(x) => &x.attrs,
        Expr::MethodCall(x) => &x.attrs,
        Expr::Paren(x) => &x.attrs,
        Expr::Path(x) => &x.attrs,
        Expr::Range(x) => &x.attrs,
        Expr::Reference(x) => &x.attrs,
        Expr::Repeat(x) => &x.attrs,
        Expr::Return(x) => &x.attrs,
        Expr::Struct(x) => &x.attrs,
        Expr::Try(x) => &x.attrs,
        Expr::TryBlock(x) => &x.attrs,
        Expr::Tuple(x) => &x.attrs,
        Expr::Unary(x) => &x.attrs,
        Expr::Unsafe(x) => &x.attrs,
        Expr::While(x) => &x.attrs,
        Expr::Yield(x) => &x.attrs,
        _ => &[],
    }
}

/// The `#[rsk::bounds(...)]` / `#[bounds(...)]` annotations of a function.
fn bounds_of(attrs: &[Attribute]) -> Vec<(String, String, bool, usize)> {
    let mut out = Vec::new();
    for a in attrs {
        let Meta::List(l) = &a.meta else { continue };
        let is_bounds = l.path.segments.last().is_some_and(|s| s.ident == "bounds") && l.path.segments.len() <= 2 && (l.path.segments.len() == 1 || l.path.segments[0].ident == "rsk" || l.path.segments[0].ident == "rsk_macros");
        if !is_bounds {
            continue;
        }
        let parsed = l.parse_args_with(syn::punctuated::Punctuated::<Bound, syn::Token![,]>::parse_terminated);
        match parsed {
            Ok(list) => {
                for b in list {
                    out.push((b.label.to_string(), text(&b.expr), constant_expr(&b.expr), line_of(&b.expr)));
                }
            }
            Err(_) => out.push(("?".into(), l.tokens.to_string(), false, line_of(a))),
        }
    }
    out
}

struct Bound {
    label: syn::Lifetime,
    expr: Expr,
}

impl syn::parse::Parse for Bound {
    fn parse(input: syn::parse::ParseStream) -> syn::Result<Self> {
        let label: syn::Lifetime = input.parse()?;
        input.parse::<syn::Token![=]>()?;
        let expr: Expr = input.parse()?;
        Ok(Bound { label, expr })
    }
}

impl<'a> Linter<'a> {
    pub fn new(file: &str, scope: Scope, items: &'a [CoreItem]) -> Linter<'a> {
        Linter { file: file.to_string(), scope, items, fns: Vec::new(), findings: Vec::new(), loops: Vec::new() }
    }

    pub fn run(&mut self, file: &syn::File) {
        for a in &file.attrs {
            self.lint_levels(a);
        }
        for item in &file.items {
            self.visit_item(item);
        }
    }

    fn err(&mut self, rule: &'static str, line: usize, message: String) {
        self.findings.push(Finding { rule, file: self.file.clone(), line, message });
    }

    fn fn_name(&self) -> String {
        self.fns.last().map(|f| f.name.clone()).unwrap_or_default()
    }

    fn enter_fn(&mut self, name: &str, attrs: &[Attribute], is_method: bool, asyncness: bool, line: usize) {
        if asyncness {
            self.err("PR-09", line, format!("`{name}` is an `async` function; a Task body has exactly one Release_Point and no other suspension point (R20)"));
        }
        let mut bounds = Vec::new();
        for (label, expr, ok, bline) in bounds_of(attrs) {
            if !ok {
                self.err("PR-16", bline, format!("the bound `{expr}` of `{label}` in `{name}` is not an integer constant or an expression over compile-time constants (R35.5)"));
            }
            bounds.push((label, expr, ok));
        }
        self.fns.push(FnCtx { name: name.to_string(), bounds, used: Vec::new(), is_method });
    }

    fn leave_fn(&mut self, line: usize) {
        if let Some(f) = self.fns.pop() {
            for (label, _, _) in &f.bounds {
                if !f.used.contains(label) && label != "?" {
                    self.err("PR-16", line, format!("`#[rsk::bounds]` of `{}` names `{label}`, which labels no loop of the function", f.name));
                }
            }
        }
    }

    /// A loop outside `verus!`.
    fn record_loop(&mut self, keyword: &str, label: Option<&syn::Label>, derived: Option<String>, exempt: bool, line: usize) {
        let label = label.map(|l| l.name.to_string());
        let function = self.fn_name();
        if let Some(l) = &label {
            if let Some(f) = self.fns.last_mut() {
                if let Some((_, expr, ok)) = f.bounds.iter().find(|(x, _, _)| x == l).cloned() {
                    f.used.push(l.clone());
                    if ok {
                        self.loops.push(LoopRecord { file: self.file.clone(), line, function, label, kind: "annotated", bound: Some(expr) });
                    }
                    return;
                }
            }
        }
        if let Some(b) = derived {
            self.loops.push(LoopRecord { file: self.file.clone(), line, function, label, kind: "derived", bound: Some(b) });
            return;
        }
        if exempt {
            self.loops.push(LoopRecord { file: self.file.clone(), line, function, label, kind: "exempt", bound: None });
            return;
        }
        let what = match &label {
            Some(l) => format!("`{l}: {keyword}`"),
            None => format!("`{keyword}`"),
        };
        self.err("PR-16", line, format!("{what} in `{function}` has no Loop_Bound_Annotation (`#[rsk::bounds('label = N)]` on the function) and no iteration count derivable from compile-time constants (R35.1)"));
    }

    fn lint_levels(&mut self, a: &Attribute) {
        let name = a.path().segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if !matches!(name.as_str(), "allow" | "warn" | "expect" | "cfg_attr") {
            return;
        }
        let s: String = a.meta.to_token_stream().to_string().chars().filter(|c| !c.is_whitespace()).collect();
        for level in ["allow(", "warn(", "expect("] {
            let mut rest = s.as_str();
            while let Some(k) = rest.find(level) {
                let after = &rest[k + level.len()..];
                let end = after.find(')').unwrap_or(after.len());
                for lint in after[..end].split(',') {
                    if config::is_profile_lint(lint) {
                        self.err("R3.4", line_of(a), format!("`#[{}]` lowers the Profile_Lint lint `{lint}` below error (PR-10, PR-11, PR-22, PR-35)", &level[..level.len() - 1]));
                    }
                }
                rest = after;
            }
        }
    }

    fn core_path(&mut self, line: usize, path: &str) {
        match lookup(path, self.items) {
            None => self.err("PR-21", line, format!("`{path}` is no Core_Subset item (R57.2)")),
            Some(item) => {
                let mine = self.scope.core_scope();
                let ok = item.scope.iter().any(|s| s == mine) || (self.scope == Scope::KernelUnsafe && item.scope.iter().any(|s| s == "kernel"));
                if !ok {
                    self.err("PR-21", line, format!("`{path}` (Core_Subset item `{}`) is not admitted in {} code: its scope is [{}] (R57.2)", item.raw, mine, item.scope.join(", ")));
                }
            }
        }
    }

    fn macro_tokens(&mut self, tokens: &TokenStream, is_verus: bool) {
        let mut paths = Vec::new();
        verus::core_paths(tokens, &mut paths);
        for (line, p) in paths {
            self.core_path(line, &p);
        }
        if is_verus {
            let mut loops = Vec::new();
            verus::loops(tokens, &mut loops);
            for l in loops {
                let function = l.function.clone();
                if l.bounded {
                    self.loops.push(LoopRecord { file: self.file.clone(), line: l.line, function, label: l.label, kind: "verus", bound: Some("decreases".into()) });
                } else if l.empty {
                    self.loops.push(LoopRecord { file: self.file.clone(), line: l.line, function, label: l.label, kind: "exempt", bound: None });
                } else {
                    self.err("PR-16", l.line, format!("`{}` in `{}` inside `verus!` has no `decreases` clause (R35.2)", l.keyword, l.function));
                }
            }
            // Token-level async check inside the macro body.
            let s: Vec<proc_macro2::TokenTree> = tokens.clone().into_iter().collect();
            for t in &s {
                if let proc_macro2::TokenTree::Ident(id) = t {
                    if id == "async" || id == "await" {
                        self.err("PR-09", id.span().start().line, format!("`{id}` inside `verus!`"));
                    }
                }
            }
        }
    }
}

impl<'ast, 'a> Visit<'ast> for Linter<'a> {
    fn visit_item(&mut self, i: &'ast Item) {
        if cfg_excludes(item_attrs(i)) {
            return;
        }
        for a in item_attrs(i) {
            self.lint_levels(a);
        }
        visit::visit_item(self, i);
    }

    fn visit_impl_item(&mut self, i: &'ast syn::ImplItem) {
        let attrs = match i {
            syn::ImplItem::Const(x) => &x.attrs[..],
            syn::ImplItem::Fn(x) => &x.attrs[..],
            syn::ImplItem::Type(x) => &x.attrs[..],
            syn::ImplItem::Macro(x) => &x.attrs[..],
            _ => &[],
        };
        if cfg_excludes(attrs) {
            return;
        }
        for a in attrs {
            self.lint_levels(a);
        }
        visit::visit_impl_item(self, i);
    }

    fn visit_trait_item(&mut self, i: &'ast syn::TraitItem) {
        let attrs = match i {
            syn::TraitItem::Const(x) => &x.attrs[..],
            syn::TraitItem::Fn(x) => &x.attrs[..],
            syn::TraitItem::Type(x) => &x.attrs[..],
            syn::TraitItem::Macro(x) => &x.attrs[..],
            _ => &[],
        };
        if cfg_excludes(attrs) {
            return;
        }
        visit::visit_trait_item(self, i);
    }

    fn visit_stmt(&mut self, s: &'ast syn::Stmt) {
        let attrs: &[Attribute] = match s {
            syn::Stmt::Local(l) => &l.attrs,
            syn::Stmt::Macro(m) => &m.attrs,
            syn::Stmt::Expr(e, _) => expr_attrs(e),
            syn::Stmt::Item(_) => &[],
        };
        if cfg_excludes(attrs) {
            return;
        }
        for a in attrs {
            self.lint_levels(a);
        }
        visit::visit_stmt(self, s);
    }

    fn visit_expr(&mut self, e: &'ast Expr) {
        if cfg_excludes(expr_attrs(e)) {
            return;
        }
        visit::visit_expr(self, e);
    }

    fn visit_item_fn(&mut self, f: &'ast syn::ItemFn) {
        self.enter_fn(&f.sig.ident.to_string(), &f.attrs, false, f.sig.asyncness.is_some(), line_of(&f.sig.ident));
        visit::visit_item_fn(self, f);
        self.leave_fn(line_of(&f.sig.ident));
    }

    fn visit_impl_item_fn(&mut self, f: &'ast syn::ImplItemFn) {
        self.enter_fn(&f.sig.ident.to_string(), &f.attrs, true, f.sig.asyncness.is_some(), line_of(&f.sig.ident));
        visit::visit_impl_item_fn(self, f);
        self.leave_fn(line_of(&f.sig.ident));
    }

    fn visit_trait_item_fn(&mut self, f: &'ast syn::TraitItemFn) {
        self.enter_fn(&f.sig.ident.to_string(), &f.attrs, true, f.sig.asyncness.is_some(), line_of(&f.sig.ident));
        visit::visit_trait_item_fn(self, f);
        self.leave_fn(line_of(&f.sig.ident));
    }

    fn visit_expr_async(&mut self, e: &'ast syn::ExprAsync) {
        self.err("PR-09", line_of(e), format!("`async` block in `{}` (R20)", self.fn_name()));
        visit::visit_expr_async(self, e);
    }

    fn visit_expr_await(&mut self, e: &'ast syn::ExprAwait) {
        self.err("PR-09", line_of(e), format!("`.await` in `{}` (R20)", self.fn_name()));
        visit::visit_expr_await(self, e);
    }

    fn visit_expr_closure(&mut self, c: &'ast syn::ExprClosure) {
        if c.asyncness.is_some() {
            self.err("PR-09", line_of(c), format!("`async` closure in `{}` (R20)", self.fn_name()));
        }
        visit::visit_expr_closure(self, c);
    }

    fn visit_expr_for_loop(&mut self, e: &'ast syn::ExprForLoop) {
        let derived = bounded_iterator(&e.expr);
        self.record_loop("for", e.label.as_ref(), derived, false, line_of(&e.for_token));
        visit::visit_expr_for_loop(self, e);
    }

    fn visit_expr_while(&mut self, e: &'ast syn::ExprWhile) {
        let derived = bounded_condition(&e.cond);
        self.record_loop("while", e.label.as_ref(), derived, false, line_of(&e.while_token));
        visit::visit_expr_while(self, e);
    }

    fn visit_expr_loop(&mut self, e: &'ast syn::ExprLoop) {
        // `loop {}` after a diverging call, and the idle loop (R7.6).
        let exempt = e.body.stmts.is_empty()
            || (e.body.stmts.len() == 1
                && matches!(&e.body.stmts[0], syn::Stmt::Expr(Expr::Call(c), _) if matches!(&*c.func, Expr::Path(p) if p.path.segments.last().is_some_and(|s| s.ident == "wait_for_interrupt"))));
        self.record_loop("loop", e.label.as_ref(), None, exempt, line_of(&e.loop_token));
        visit::visit_expr_loop(self, e);
    }

    fn visit_macro(&mut self, m: &'ast syn::Macro) {
        let last = m.path.segments.last().map(|s| s.ident.to_string()).unwrap_or_default();
        if matches!(self.scope, Scope::Application | Scope::Generated) && matches!(last.as_str(), "asm" | "naked_asm" | "global_asm") {
            self.err("PR-22", line_of(m), format!("`{last}!` in Application code: assembly belongs to the Kernel_Unsafe_Module (R6.3, R16.10)"));
        }
        let is_verus = last == "verus";
        self.macro_tokens(&m.tokens, is_verus);
        visit::visit_macro(self, m);
    }

    fn visit_item_use(&mut self, u: &'ast syn::ItemUse) {
        let mut paths = Vec::new();
        verus::core_paths(&u.to_token_stream(), &mut paths);
        for (line, p) in paths {
            self.core_path(line, &p);
        }
    }

    fn visit_path(&mut self, p: &'ast syn::Path) {
        if p.segments.first().is_some_and(|s| s.ident == "core") && p.segments.len() > 1 {
            let path = p.segments.iter().map(|s| s.ident.to_string()).collect::<Vec<_>>().join("::");
            self.core_path(line_of(p), &path);
        }
        visit::visit_path(self, p);
    }

    fn visit_item_static(&mut self, s: &'ast syn::ItemStatic) {
        if matches!(self.scope, Scope::Application) {
            if matches!(s.mutability, syn::StaticMutability::Mut(_)) {
                self.err("PR-05", line_of(&s.ident), format!("`static mut {}`: shared state outside the Kernel is a declared Resource (R15)", s.ident));
            } else {
                let ty = s.ty.to_token_stream().to_string();
                let interior = ["Cell", "RefCell", "UnsafeCell", "OnceCell"].iter().any(|n| ty.split(|c: char| !c.is_alphanumeric() && c != '_').any(|w| w == *n)) || ty.contains("Atomic");
                if interior {
                    self.err("PR-05", line_of(&s.ident), format!("`static {}` has an interior-mutable type (`{ty}`); shared state outside the Kernel is a declared Resource (R15)", s.ident));
                }
            }
        }
        visit::visit_item_static(self, s);
    }

    fn visit_expr_call(&mut self, c: &'ast syn::ExprCall) {
        if let (Some(f), Expr::Path(p)) = (self.fns.last(), &*c.func) {
            let segs = &p.path.segments;
            let last = segs.last().map(|s| s.ident.to_string()).unwrap_or_default();
            if last == f.name && (segs.len() == 1 || (segs.len() == 2 && segs[0].ident == "Self")) {
                let name = f.name.clone();
                self.err("PR-17", line_of(c), format!("`{name}` calls itself (direct recursion)"));
            }
        }
        visit::visit_expr_call(self, c);
    }

    fn visit_expr_method_call(&mut self, m: &'ast syn::ExprMethodCall) {
        if let Some(f) = self.fns.last() {
            if f.is_method && m.method == f.name && matches!(&*m.receiver, Expr::Path(p) if p.path.is_ident("self")) {
                let name = f.name.clone();
                self.err("PR-17", line_of(m), format!("`{name}` calls itself (direct recursion)"));
            }
        }
        visit::visit_expr_method_call(self, m);
    }

    fn visit_type_trait_object(&mut self, t: &'ast syn::TypeTraitObject) {
        if matches!(self.scope, Scope::Application) {
            self.err("PR-18", line_of(t), format!("`{}`: dynamic dispatch in Partition code (static call graph, R37)", t.to_token_stream()));
        }
        visit::visit_type_trait_object(self, t);
    }

    fn visit_type_bare_fn(&mut self, t: &'ast syn::TypeBareFn) {
        if matches!(self.scope, Scope::Application) {
            self.err("PR-18", line_of(t), format!("`{}`: a function-pointer type in Partition code (static call graph, R37)", t.to_token_stream()));
        }
        visit::visit_type_bare_fn(self, t);
    }
}
