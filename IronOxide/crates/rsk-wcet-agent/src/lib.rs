//! rsk-wcet-agent: the Target side of the WCET_Harness (task 19.1). It is a
//! separate `no_std` crate so that the host runner (`rsk-wcet`) and the agent
//! each have one target set. Measurement builds link it; Flight_Builds do not.
#![no_std]
#![forbid(unsafe_code)]
