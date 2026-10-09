//! A build script that tries to reach the network.
#![forbid(unsafe_code)]

use std::io::ErrorKind;
use std::net::{SocketAddr, TcpStream};
use std::time::Duration;

fn main() {
    let addr = SocketAddr::from(([192, 0, 2, 1], 9));
    match TcpStream::connect_timeout(&addr, Duration::from_secs(3)) {
        Err(e)
            if matches!(
                e.kind(),
                ErrorKind::PermissionDenied | ErrorKind::NetworkUnreachable
            ) =>
        {
            println!("cargo:warning=network-probe build script: connection denied ({e})");
        }
        other => panic!("build script was not denied network access: {other:?}"),
    }
}
