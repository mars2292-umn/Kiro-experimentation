//! `rsk-hil`: the HIL_Rig (task 7.1; design.md "Profile_Conformance_Suite
//! and HIL_Rig"; R34.8, R48.4). The rig is two nRF52840-DK boards, one
//! under test and one generating stimuli, a logic analyzer, and the host
//! running probe-rs and sigrok-cli. This crate holds what the rig does
//! with binaries it is given; building them is the Build_System's
//! (`cargo xtask hil`, the job's HIL steps).
//!
//! - [`config`]: the rig description `hil-rig.toml` (boards by probe
//!   serial, the analyzer and its channels, the wiring, the expectations).
//! - [`probes`]: `probe-rs list`, parsed; every board is selected by serial.
//! - [`board`]: flashing, reset, the read-back hash check against the
//!   loaded ELF and the Build_Manifest (R48.4), and the console run.
//! - [`capture`]: the analyzer capture and the edge analysis (stimulus
//!   edges matched to response edges, latencies).
//! - [`trace`]: the Kernel's RAM trace ring, decoded, with the interrupt
//!   storm check.
//! - [`stats`]: the Kernel's release-timing record (R9.10), decoded.
//! - [`events`]: the Kernel's event mirror (every Health_Monitor event of
//!   the run) and the trace freeze at the first unexpected one.
//! - [`release_timing`]: the release-timing run over PAR-05 (task 7.3),
//!   and its stress mode (task 7.5: Overrun and interrupt-storm fault
//!   injection, R18.8, R19.3).
//! - [`response`]: the response suite (task 7.5, R14.11): one injected
//!   fault per response of R14.5, judged from the event mirror.
//! - [`selftest`]: the rig self-test (the evidence of task 7.1): the
//!   boards identified, both images read back and hashed, the stimulus
//!   board running, a loopback run of the Application under test, the
//!   capture agreeing with the console.
#![forbid(unsafe_code)]

pub mod board;
pub mod capture;
pub mod config;
pub mod events;
pub mod probes;
pub mod release_timing;
pub mod response;
pub mod selftest;
pub mod stats;
pub mod trace;

/// Removes ANSI colour sequences (probe-rs colours its output; logs are
/// kept plain).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                while let Some(&d) = chars.peek() {
                    chars.next();
                    if d.is_ascii_alphabetic() {
                        break;
                    }
                }
            }
            continue;
        }
        out.push(c);
    }
    out
}
