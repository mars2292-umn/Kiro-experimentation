//! Links the binary with the Generator's linker script of the stimulus
//! Application (`../gen/link.x` and `../gen/memory.x`, written by
//! `cargo xtask gen` from the App_Declaration, R24.1, R24.4). Runs offline.
#![forbid(unsafe_code)]

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    println!("cargo:rustc-link-search={dir}/../gen");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rerun-if-changed=../gen/link.x");
    println!("cargo:rerun-if-changed=../gen/memory.x");
}
