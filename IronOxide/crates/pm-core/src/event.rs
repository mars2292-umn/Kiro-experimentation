//! What the step function is given, and what it returns.

use crate::mode::Chamber;
use crate::params::Params;
use crate::time::Micros;

/// The battery status level (PM17.6), best first. Below `Erp` is the
/// Power-On Reset trip level, at which nothing is paced (PM7.2, PM30.3).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
#[repr(u8)]
pub enum Battery {
    /// Beginning of life.
    #[default]
    Bol = 0,
    /// Elective replacement near.
    Ern = 1,
    /// Elective replacement time.
    Ert = 2,
    /// Elective replacement past.
    Erp = 3,
    /// Below the Power-On Reset trip voltage.
    Tripped = 4,
}

/// A command from the DCM, already validated (PM20.1). The record commands
/// do not reach the step function.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Command {
    /// Program a permanent parameter set.
    Program(Params),
    /// Start temporary pacing with this set.
    StartTemporary(Params),
    /// End temporary pacing.
    EndTemporary,
    /// Pace-Now.
    PaceNow,
    /// The magnet response programmed on or off (PM8.4).
    MagnetResponse(bool),
}

/// One event.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Event {
    /// One service of a Chamber's sense line: `edges` edges since the last
    /// service, the latest at `at`.
    Sense { chamber: Chamber, at: Micros, edges: u32 },
    /// A Pace of `chamber` fired at `at`.
    Paced { chamber: Chamber, at: Micros },
    /// One period of `control`.
    Tick { now: Micros, activity: u8, magnet: bool, battery: Battery, link_up: bool },
    /// A validated command.
    Command(Command),
}

/// What to do with a Chamber's compares.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Pace {
    /// Leave the Chamber as it is armed.
    Keep,
    /// Withdraw the pending Pace.
    Withdraw,
    /// Arm a Pace at `at` with this pulse width.
    At { at: Micros, width: u32 },
}

/// The Markers of the source (PM18.5).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub enum Marker {
    /// AS, VS: a Sense. `refractory`: it fell in a refractory period.
    Sense { refractory: bool },
    /// AP, VP: a Pace. `fused`: a Sense was captured within the inhibition
    /// window before it (design PD-04).
    Pace { fused: bool },
    /// AT: an atrial Sense faster than the URL.
    AtrialTachy,
    /// PVC: a ventricular Sense with no atrial event since the last
    /// ventricular event.
    Pvc,
    /// TN: noise.
    Noise,
}

/// The Markers of one step: at most one atrial, one ventricular and one
/// augmentation Marker (Property P9, by this type).
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Markers {
    pub atrial: Option<(Marker, Micros)>,
    pub ventricular: Option<(Marker, Micros)>,
    pub augmentation: Option<(Marker, Micros)>,
}

impl Markers {
    pub const NONE: Markers = Markers { atrial: None, ventricular: None, augmentation: None };
}

/// What the step function returns beside the new state.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
pub struct Arming {
    pub atrial: Pace,
    pub ventricular: Pace,
    pub markers: Markers,
}

impl Arming {
    /// Nothing to arm, nothing to report.
    pub const NONE: Arming = Arming { atrial: Pace::Keep, ventricular: Pace::Keep, markers: Markers::NONE };

    /// The instruction for `c`.
    #[inline(always)]
    #[must_use]
    pub const fn pace(&self, c: Chamber) -> Pace {
        match c {
            Chamber::A => self.atrial,
            Chamber::V => self.ventricular,
        }
    }
}
