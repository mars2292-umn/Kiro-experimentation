//! Links the binary with the Generator's linker script of the demo
//! (`../gen/link.x` and `../gen/memory.x`, written by `cargo xtask gen`
//! from the App_Declaration and checked for drift by the verification
//! job, R24.1, R24.4). Runs offline (R58.4).
#![forbid(unsafe_code)]

fn main() {
    let dir = std::env::var("CARGO_MANIFEST_DIR").expect("manifest dir");
    // The declaration variant's generated files (`gen-<variant>/`), the
    // default's otherwise.
    let gen = if std::env::var_os("CARGO_FEATURE_TIMING").is_some() {
        "gen-nrf52840_timing"
    } else if std::env::var_os("CARGO_FEATURE_TIMING_SHORT").is_some() {
        "gen-nrf52840_timing_short"
    } else if std::env::var_os("CARGO_FEATURE_RESPONSE").is_some() {
        "gen-nrf52840_response"
    } else if std::env::var_os("CARGO_FEATURE_STRESS").is_some() {
        "gen-nrf52840_stress"
    } else if std::env::var_os("CARGO_FEATURE_STRESS_SHORT").is_some() {
        "gen-nrf52840_stress_short"
    } else if std::env::var_os("CARGO_FEATURE_ISOLATION").is_some() {
        "gen-nrf52840_isolation"
    } else if std::env::var_os("CARGO_FEATURE_CYCLES_FPU").is_some() {
        "gen-nrf52840_cycles_fpu"
    } else if std::env::var_os("CARGO_FEATURE_CYCLES").is_some() {
        "gen-nrf52840_cycles"
    } else if std::env::var_os("CARGO_FEATURE_PLAIN_MEASURE").is_some() {
        "gen-nrf52840_measure"
    } else if std::env::var_os("CARGO_FEATURE_COVERAGE_KERNEL").is_some() {
        // The Kernel coverage build (task 19.5): the enlarged-Kernel layout.
        "gen-nrf52840_coverage"
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
