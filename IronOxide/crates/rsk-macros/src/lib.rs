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
