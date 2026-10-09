//! Compiles only if the proc-macro was denied network access while it
//! expanded.
#![forbid(unsafe_code)]

probe_macro::expansion_probe!();
