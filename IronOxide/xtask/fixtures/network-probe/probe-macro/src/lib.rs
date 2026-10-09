//! A proc-macro that tries to reach the network while it expands.
#![forbid(unsafe_code)]

use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

use proc_macro::TokenStream;

/// Expands to a constant if the connection attempt was denied by a sandbox,
/// and to a `compile_error!` otherwise.
#[proc_macro]
pub fn expansion_probe(_input: TokenStream) -> TokenStream {
    let addr = SocketAddr::from(([192, 0, 2, 1], 9));
    let code = match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::PermissionDenied | ErrorKind::NetworkUnreachable
            ) =>
        {
            "pub const NETWORK_DENIED_AT_EXPANSION: bool = true;".to_string()
        }
        other => format!(
            "compile_error!({:?});",
            format!("proc-macro was not denied network access: {other:?}")
        ),
    };
    code.parse().expect("generated code is valid Rust")
}
