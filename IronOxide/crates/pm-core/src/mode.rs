//! Chambers and Modes.

// The App_Prover's view operator (`@`) in the contracts below.
#[cfg(creusot)]
#[allow(unused_imports)]
use creusot_std::model::View;

/// A Chamber of the heart.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
#[repr(u8)]
pub enum Chamber {
    /// The atrium.
    A = 0,
    /// The ventricle.
    V = 1,
}

impl Chamber {
    /// 0 for the atrium, 1 for the ventricle: the index of per-Chamber arrays.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result@ < 2))]
    #[cfg_attr(creusot, creusot_std::macros::ensures((self == Chamber::A ==> result@ == 0) && (self == Chamber::V ==> result@ == 1)))]
    pub const fn index(self) -> usize {
        match self {
            Chamber::A => 0,
            Chamber::V => 1,
        }
    }

    /// The other Chamber.
    #[inline(always)]
    #[must_use]
    pub const fn other(self) -> Chamber {
        match self {
            Chamber::A => Chamber::V,
            Chamber::V => Chamber::A,
        }
    }
}

/// A set of Chambers: the first and the second letter of a Mode (O, A, V, D).
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self == Chambers::Both || (self == Chambers::A && c == Chamber::A) || (self == Chambers::V && c == Chamber::V))))]
    pub const fn has(self, c: Chamber) -> bool {
        matches!((self, c), (Chambers::Both, _) | (Chambers::A, Chamber::A) | (Chambers::V, Chamber::V))
    }
}

/// The response to sensing: the third letter of a Mode.
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(not(creusot), derive(PartialEq, Eq))]
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

#[inline(always)]
/// A Mode from its four properties.
pub const fn mode(paces: Chambers, senses: Chambers, response: Response, adaptive: bool) -> Mode {
    Mode { paces, senses, response, adaptive }
}

/// For the App_Prover: the Mode tracks (what [`Mode::tracks`] returns).
#[cfg(creusot)]
#[creusot_std::macros::logic(open)]
pub fn tracks_spec(m: Mode) -> bool {
    use creusot_std::macros::pearlite;
    pearlite! {
        (m.paces == Chambers::V || m.paces == Chambers::Both)
            && (m.senses == Chambers::A || m.senses == Chambers::Both)
            && m.response == Response::Tracked
    }
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
    #[inline(always)]
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
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.paces == Chambers::Both || (self.paces == Chambers::A && c == Chamber::A) || (self.paces == Chambers::V && c == Chamber::V))))]
    pub const fn paces(self, c: Chamber) -> bool {
        self.paces.has(c)
    }

    /// Whether the Mode senses `c` (PM5.7).
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (self.senses == Chambers::Both || (self.senses == Chambers::A && c == Chamber::A) || (self.senses == Chambers::V && c == Chamber::V))))]
    pub const fn senses(self, c: Chamber) -> bool {
        self.senses.has(c)
    }

    /// The Chamber whose events start the escape interval: the ventricle in
    /// every Mode that paces or senses it, the atrium in the atrial Modes
    /// (PM9.2, PM9.3; design PD-06).
    /// Whether the Mode tracks atrial Senses with ventricular Paces: VDD
    /// and DDD and their rate-adaptive forms. These are the Modes Table
    /// 19-3, as amended by Table 19-4, gives the Upper Rate Limit as a limit
    /// of pacing, the Dynamic AV Delay, PVARP Extension and the ATR
    /// parameters.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == tracks_spec(self)))]
    pub const fn tracks(self) -> bool {
        self.paces(Chamber::V) && self.senses(Chamber::A) && matches!(self.response, Response::Tracked)
    }

    /// Whether the Mode uses the Sensed AV Delay Offset (Table 19-3): DDD
    /// and DDDR, the tracking Modes that also pace the atrium.
    #[inline(always)]
    #[must_use]
    #[cfg_attr(creusot, creusot_std::macros::ensures(result == (tracks_spec(self) && self.paces == Chambers::Both)))]
    pub const fn uses_sensed_av_offset(self) -> bool {
        self.tracks() && self.paces(Chamber::A)
    }

    #[inline(always)]
    #[must_use]
    pub const fn timing_chamber(self) -> Chamber {
        if self.paces(Chamber::V) || self.senses(Chamber::V) {
            Chamber::V
        } else {
            Chamber::A
        }
    }
}
