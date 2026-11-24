//! Links the binary with the Generator's linker script of the pulse
//! generator: the declaration variant's (`../gen-nrf52840/`, or the arming
//! sweep's `../gen-nrf52840_sweep/`, the measurement runs'
//! `../gen-nrf52840_measure/`) on the board,
//! the default's (`../gen/`) on QEMU, written by `cargo xtask gen` (R24.1,
//! R24.4). Runs offline.
#![forbid(unsafe_code)]

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    let gen = if std::env::var_os("CARGO_FEATURE_SWEEP").is_some() {
        "gen-nrf52840_sweep"
    } else if std::env::var_os("CARGO_FEATURE_COVERAGE").is_some() {
        "gen-nrf52840_coverage"
    } else if std::env::var_os("CARGO_FEATURE_MEASURE").is_some() {
        "gen-nrf52840_measure"
    } else if std::env::var_os("CARGO_FEATURE_RIG").is_some() {
        "gen-nrf52840_rig"
    } else if std::env::var_os("CARGO_FEATURE_NRF52840").is_some() {
        "gen-nrf52840"
    } else {
        "gen"
    };
    println!("cargo:rustc-link-search={dir}/../{gen}");
    println!("cargo:rustc-link-arg-bins=-Tlink.x");
    println!("cargo:rustc-link-arg-bins=--nmagic");
    println!("cargo:rerun-if-changed=../{gen}/link.x");
    println!("cargo:rerun-if-changed=../{gen}/memory.x");
}
