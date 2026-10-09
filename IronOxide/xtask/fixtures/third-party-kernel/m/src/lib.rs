//! A proc-macro crate that a Flight_Build would compile for the host.
#![forbid(unsafe_code)]

use proc_macro::TokenStream;

#[proc_macro]
pub fn identity(input: TokenStream) -> TokenStream {
    proc_macro2::TokenStream::from(input).into()
}
