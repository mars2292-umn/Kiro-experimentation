//! rsk-macros: the `rsk::app!` front end, which delegates to `rsk-gen`
//! (task 10.2, 10.5). Every Flight_Build compiles this crate for the host,
//! so PR-36 applies to it and to its dependencies.
//!
//! `rsk::app! { ... }` in the declaration crate validates the
//! App_Declaration (R21, R22; every violation is a compile error at the
//! span of the offending item naming the PR identifier, R21.4) and
//! exports two kinds of `macro_rules!` manifests:
//!
//! - `rsk_system!()` for the system crate: `CONFIG`, `LAYOUT`, the
//!   `System` implementation (R24.1);
//! - `rsk_partition_<name>!()` for each Partition crate: Resource cells,
//!   Context types, and Job wrappers.
//!
//! The proc-macro writes no files (R59); the `rsk-gen` CLI produces the
//! Task_Model and the linker script from the same source and parser, and
//! the Config_Checker compares the two (R26).
#![forbid(unsafe_code)]

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{format_ident, quote};

#[proc_macro]
pub fn app(input: TokenStream) -> TokenStream {
    expand(input.into()).into()
}

fn expand(input: TokenStream2) -> TokenStream2 {
    let parsed = match rsk_gen::syntax::parse_app(input) {
        Ok(p) => p,
        Err(e) => return e.to_compile_error(),
    };
    let outputs = match rsk_gen::outputs(&parsed.decl, "rsk-system") {
        Ok(o) => o,
        Err(diagnostics) => {
            let errors = parsed.spans.resolve(&diagnostics);
            let mut out = TokenStream2::new();
            for e in errors {
                out.extend(syn::Error::new(e.span, e.message).to_compile_error());
            }
            return out;
        }
    };
    let system: TokenStream2 = match outputs.system_source.parse() {
        Ok(t) => t,
        Err(e) => return syn::Error::new(proc_macro2::Span::call_site(), format!("rsk-gen produced unparsable code: {e}")).to_compile_error(),
    };
    let mut out = quote! {
        /// The system crate manifest (R24.1): expands to `CONFIG`, `LAYOUT`,
        /// and the `System` implementation. Invoke once at the root of the
        /// system crate, which depends on `rsk-kernel` and every Partition crate.
        #[macro_export]
        macro_rules! rsk_system {
            () => { #system };
        }
    };
    for (name, source) in &outputs.partition_sources {
        let mac = format_ident!("rsk_partition_{}", name);
        let body: TokenStream2 = match source.parse() {
            Ok(t) => t,
            Err(e) => return syn::Error::new(proc_macro2::Span::call_site(), format!("rsk-gen produced unparsable code for Partition `{name}`: {e}")).to_compile_error(),
        };
        let doc = format!("The manifest of Partition `{name}`: Resource cells, Context types, and Job wrappers. Invoke once at the root of the Partition crate.");
        out.extend(quote! {
            #[doc = #doc]
            #[macro_export]
            macro_rules! #mac {
                () => { #body };
            }
        });
    }
    out
}

/// `#[rsk::bounds('label = BOUND, ...)]` (R35, PR-16): the Loop_Bound_Annotation
/// of the labelled loops of one function. Each bound is an integer
/// constant or an expression over compile-time constants and
/// Generated_Config constants (R35.5); each label must name a loop of the
/// function. The attribute expands to the function unchanged: the
/// Profile_Lint (`rsk-lint`) reads the annotation from the source and
/// exports it (R35.3); instrumented builds are task 14.3.
#[proc_macro_attribute]
pub fn bounds(args: TokenStream, item: TokenStream) -> TokenStream {
    let item2: TokenStream2 = item.clone().into();
    match bounds::check(args.into(), item2) {
        // The labels exist for the annotation and the Profile_Lint, not for
        // `break`/`continue`; `unused_labels` is a rustc lint, not a
        // Profile_Lint lint (R3.4), so allowing it here lowers nothing.
        Ok(()) => {
            let item: TokenStream2 = item.into();
            quote!(#[allow(unused_labels)] #item).into()
        }
        Err(e) => e.to_compile_error().into(),
    }
}

mod bounds {
    use proc_macro2::TokenStream;
    use syn::parse::{Parse, ParseStream};
    use syn::punctuated::Punctuated;
    use syn::{Expr, Lifetime, Token};

    struct Bound {
        label: Lifetime,
        expr: Expr,
    }

    impl Parse for Bound {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            let label: Lifetime = input.parse()?;
            input.parse::<Token![=]>()?;
            let expr: Expr = input.parse()?;
            Ok(Bound { label, expr })
        }
    }

    struct Bounds(Punctuated<Bound, Token![,]>);

    impl Parse for Bounds {
        fn parse(input: ParseStream) -> syn::Result<Self> {
            Ok(Bounds(Punctuated::parse_terminated(input)?))
        }
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
            _ => false,
        }
    }

    fn labels_of(f: &syn::ItemFn) -> Vec<String> {
        let mut out = Vec::new();
        collect_labels(&f.block.to_token_stream(), &mut out);
        out
    }

    use quote::ToTokens;

    /// Every `'label:` before `for`/`while`/`loop` in the token stream.
    fn collect_labels(ts: &TokenStream, out: &mut Vec<String>) {
        let tokens: Vec<proc_macro2::TokenTree> = ts.clone().into_iter().collect();
        for (i, t) in tokens.iter().enumerate() {
            match t {
                proc_macro2::TokenTree::Group(g) => collect_labels(&g.stream(), out),
                proc_macro2::TokenTree::Punct(p) if p.as_char() == '\'' => {
                    if let (Some(proc_macro2::TokenTree::Ident(name)), Some(proc_macro2::TokenTree::Punct(colon))) = (tokens.get(i + 1), tokens.get(i + 2)) {
                        if colon.as_char() == ':' {
                            if let Some(proc_macro2::TokenTree::Ident(kw)) = tokens.get(i + 3) {
                                if kw == "for" || kw == "while" || kw == "loop" {
                                    out.push(format!("'{name}"));
                                }
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }

    pub fn check(args: TokenStream, item: TokenStream) -> syn::Result<()> {
        let bounds: Bounds = syn::parse2(args)?;
        let f: syn::ItemFn = syn::parse2(item).map_err(|e| syn::Error::new(e.span(), format!("PR-16: `#[rsk::bounds]` applies to a function: {e}")))?;
        let labels = labels_of(&f);
        for b in &bounds.0 {
            if !constant_expr(&b.expr) {
                return Err(syn::Error::new_spanned(&b.expr, format!("PR-16 (R35.5): the bound of `{}` must be an integer constant or an expression over compile-time constants", b.label)));
            }
            if !labels.contains(&b.label.to_string()) {
                return Err(syn::Error::new_spanned(&b.label, format!("PR-16: `{}` names no labelled loop of `{}`", b.label, f.sig.ident)));
            }
        }
        Ok(())
    }
}
