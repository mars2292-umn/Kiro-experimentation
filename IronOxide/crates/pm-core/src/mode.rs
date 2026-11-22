//! Chambers and Modes.

/// A Chamber of the heart.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
#[repr(u8)]
pub enum Chamber {
    /// The atrium.
    A = 0,
    /// The ventricle.
    V = 1,
}

impl Chamber {
    /// 0 for the atrium, 1 for the ventricle: the index of per-Chamber arrays.
    #[must_use]
    pub const fn index(self) -> usize {
        self as usize
    }

    /// The other Chamber.
    #[must_use]
    pub const fn other(self) -> Chamber {
        match self {
            Chamber::A => Chamber::V,
            Chamber::V => Chamber::A,
        }
    }
}

/// A set of Chambers: the first and the second letter of a Mode (O, A, V, D).
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum Chambers {
    /// O: neither.
    #[default]
    None = 0,
    /// A: the atrium.
    A = 1,
    /// V: the ventricle.
    V = 2,
    /// D: both.
    Both = 3,
}

impl Chambers {
    /// Whether the set contains `c`.
    #[must_use]
    pub const fn has(self, c: Chamber) -> bool {
        matches!((self, c), (Chambers::Both, _) | (Chambers::A, Chamber::A) | (Chambers::V, Chamber::V))
    }
}

/// The response to sensing: the third letter of a Mode.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
#[repr(u8)]
pub enum Response {
    /// O: Paces are delivered without regard to Senses (PM5.3).
    #[default]
    None = 0,
    /// T: a Sense triggers a Pace of its Chamber (PM5.4).
    Triggered = 1,
    /// I: a Sense inhibits the pending Pace of its Chamber (PM5.5).
    Inhibited = 2,
    /// D: an atrial Sense is tracked by a ventricular Pace, and a Sense
    /// inhibits the pending Pace of its own Chamber (PM5.6).
    Tracked = 3,
}

/// A Mode by its four properties (design, "A Mode by its letters"). The step
/// function reads these and never a Mode's name.
///
/// The all-zero value is [`Mode::OFF`].
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Mode {
    /// The Chambers paced.
    pub paces: Chambers,
    /// The Chambers sensed.
    pub senses: Chambers,
    /// The response to sensing.
    pub response: Response,
    /// Whether the rate follows the activity sensor (the fourth letter R).
    pub adaptive: bool,
}

const fn mode(paces: Chambers, senses: Chambers, response: Response, adaptive: bool) -> Mode {
    Mode { paces, senses, response, adaptive }
}

impl Mode {
    /// Off, which is OOO: no pacing, no sensing.
    pub const OFF: Mode = mode(Chambers::None, Chambers::None, Response::None, false);
    pub const AOO: Mode = mode(Chambers::A, Chambers::None, Response::None, false);
    pub const VOO: Mode = mode(Chambers::V, Chambers::None, Response::None, false);
    pub const DOO: Mode = mode(Chambers::Both, Chambers::None, Response::None, false);
    pub const AAI: Mode = mode(Chambers::A, Chambers::A, Response::Inhibited, false);
    pub const VVI: Mode = mode(Chambers::V, Chambers::V, Response::Inhibited, false);
    pub const AAT: Mode = mode(Chambers::A, Chambers::A, Response::Triggered, false);
    pub const VVT: Mode = mode(Chambers::V, Chambers::V, Response::Triggered, false);
    pub const VDD: Mode = mode(Chambers::V, Chambers::Both, Response::Tracked, false);
    pub const DDI: Mode = mode(Chambers::Both, Chambers::Both, Response::Inhibited, false);
    pub const DDD: Mode = mode(Chambers::Both, Chambers::Both, Response::Tracked, false);
    /// Sensing only; temporary operation only (PM5.2).
    pub const OAO: Mode = mode(Chambers::None, Chambers::A, Response::None, false);
    /// Sensing only; temporary operation only (PM5.2).
    pub const OVO: Mode = mode(Chambers::None, Chambers::V, Response::None, false);
    /// Sensing only; temporary operation only (PM5.2).
    pub const ODO: Mode = mode(Chambers::None, Chambers::Both, Response::None, false);

    /// The rate-adaptive form of this Mode.
    #[must_use]
    pub const fn rate_adaptive(self) -> Mode {
        Mode { adaptive: true, ..self }
    }

    /// The 19 Modes a parameter set may name (Table 19-1), in the table's
    /// order: the wire code of a Mode is its index here (task 6.2).
    pub const PROGRAMMABLE: [Mode; 19] = [
        Mode::OFF,
        Mode::DDD,
        Mode::VDD,
        Mode::DDI,
        Mode::DOO,
        Mode::AOO,
        Mode::AAI,
        Mode::VOO,
        Mode::VVI,
        Mode::AAT,
        Mode::VVT,
        Mode::DDD.rate_adaptive(),
        Mode::VDD.rate_adaptive(),
        Mode::DDI.rate_adaptive(),
        Mode::DOO.rate_adaptive(),
        Mode::AOO.rate_adaptive(),
        Mode::AAI.rate_adaptive(),
        Mode::VOO.rate_adaptive(),
        Mode::VVI.rate_adaptive(),
    ];

    /// Whether the Mode paces `c` (PM5.7).
    #[must_use]
    pub const fn paces(self, c: Chamber) -> bool {
        self.paces.has(c)
    }

    /// Whether the Mode senses `c` (PM5.7).
    #[must_use]
    pub const fn senses(self, c: Chamber) -> bool {
        self.senses.has(c)
    }

    /// The Chamber whose events start the escape interval: the ventricle in
    /// every Mode that paces or senses it, the atrium in the atrial Modes
    /// (PM9.2, PM9.3; design PD-06).
    #[must_use]
    pub const fn timing_chamber(self) -> Chamber {
        if self.paces(Chamber::V) || self.senses(Chamber::V) {
            Chamber::V
        } else {
            Chamber::A
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn zero_is_off() {
        assert_eq!(Mode::default(), Mode::OFF);
        assert_eq!(Chambers::default() as u8, 0);
        assert_eq!(Response::default() as u8, 0);
    }

    #[test]
    fn the_programmable_modes_are_distinct_and_nineteen() {
        let m = Mode::PROGRAMMABLE;
        for i in 0..m.len() {
            for j in 0..i {
                assert_ne!(m[i], m[j], "{i} and {j}");
            }
        }
        // Eight rate-adaptive forms, as Table 19-1 lists them.
        assert_eq!(m.iter().filter(|x| x.adaptive).count(), 8);
        // Off, DDD (the nominal) at the table's first two places.
        assert_eq!(m[0], Mode::OFF);
        assert_eq!(m[1], Mode::DDD);
    }

    #[test]
    fn letters_decide_chambers() {
        assert!(Mode::VVI.paces(Chamber::V) && !Mode::VVI.paces(Chamber::A));
        assert!(Mode::VDD.paces(Chamber::V) && !Mode::VDD.paces(Chamber::A));
        assert!(Mode::VDD.senses(Chamber::A) && Mode::VDD.senses(Chamber::V));
        assert!(Mode::DOO.paces(Chamber::A) && Mode::DOO.paces(Chamber::V));
        assert!(!Mode::DOO.senses(Chamber::A) && !Mode::DOO.senses(Chamber::V));
        assert!(!Mode::OFF.paces(Chamber::A) && !Mode::OFF.paces(Chamber::V));
    }

    #[test]
    fn the_timing_chamber_is_the_atrium_only_in_atrial_modes() {
        for m in Mode::PROGRAMMABLE {
            let atrial_only = !m.paces(Chamber::V) && !m.senses(Chamber::V);
            assert_eq!(m.timing_chamber() == Chamber::A, atrial_only, "{m:?}");
        }
        assert_eq!(Mode::AAI.timing_chamber(), Chamber::A);
        assert_eq!(Mode::DDD.timing_chamber(), Chamber::V);
        assert_eq!(Mode::OVO.timing_chamber(), Chamber::V);
    }

    #[test]
    fn chamber_indices() {
        assert_eq!(Chamber::A.index(), 0);
        assert_eq!(Chamber::V.index(), 1);
        assert_eq!(Chamber::A.other(), Chamber::V);
        assert_eq!(Chamber::V.other(), Chamber::A);
    }
}
