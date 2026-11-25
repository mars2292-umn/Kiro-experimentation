//! pm-core: the therapy logic of the PACEMAKER experiment
//! (`docs/pacemaker/design.md`, decision PD-05).
//!
//! This crate is the only one with therapy knowledge. It has no rsk
//! dependency and no hardware access, so the same code runs in the
//! Therapy_Partition, in the DCM_Stub, and in host tests.
//!
//! Task 6.1 (`docs/pacemaker/tasks.md`) defines the types:
//!
//! - [`Micros`]: an instant of the therapy timer, 32 bits of microseconds,
//!   compared by signed difference so that the counter's wrap is harmless;
//! - [`Chamber`], [`Mode`]: a Mode is its four properties, not a name;
//! - [`Params`]: Table 19-1 with every interval already in microseconds;
//! - [`State`]: everything the step function carries from one event to the
//!   next. Its all-zero value is the Power-On Reset State (PM30.4);
//! - [`Event`], [`Arming`]: what the step function is given and returns.
//!
//! Tasks 6.2 and 6.3 add [`validate`] and [`step`] for the single-chamber
//! Modes of stage E1 (Off, AOO, VOO, AAI, VVI).
#![no_std]
#![forbid(unsafe_code)]

// The App_Prover's library, when this source is compiled under it.
#[cfg(feature = "creusot")]
extern crate creusot_std;

mod event;
pub mod frame;
pub mod wire;
pub mod record;
pub mod threshold;
mod mode;
mod params;
pub mod rate;
mod state;
mod step;
mod time;
mod validate;

pub use event::{Arming, Battery, Command, Event, Marker, Markers, Pace};
pub use mode::{Chamber, Chambers, Mode, Response};
pub use params::{interval_of_ppm, ActivityThreshold, Params};
pub use state::{Atr, AtrPhase, ChamberState, Pending, State, Therapy};
pub use step::{accounted, escape_modifiers, rearmed, step};
pub use time::Micros;
pub use validate::{delivered_in_e1, delivered_in_e2, validate, word, NOMINAL_WORDS, WORDS};
