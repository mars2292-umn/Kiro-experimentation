//! The App_Declaration as data (R21.1): what the `rsk::app!` front end
//! (rsk-macros) and the `rsk-gen` CLI hand to the Generator core. Every
//! item keeps the name it was declared with; the front end maps names back
//! to source spans for diagnostics (R21.4).

/// A time quantity with its explicit unit (R21.2).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Time {
    pub value: u64,
    pub unit: TimeUnit,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimeUnit {
    Cycles,
    Ticks,
    Ns,
    Us,
    Ms,
    S,
    Min,
    H,
}

impl TimeUnit {
    pub fn parse(name: &str) -> Option<TimeUnit> {
        Some(match name {
            "cycles" => TimeUnit::Cycles,
            "ticks" => TimeUnit::Ticks,
            "ns" => TimeUnit::Ns,
            "us" | "µs" => TimeUnit::Us,
            "ms" => TimeUnit::Ms,
            "s" => TimeUnit::S,
            "min" => TimeUnit::Min,
            "h" => TimeUnit::H,
            _ => return None,
        })
    }

    pub fn name(self) -> &'static str {
        match self {
            TimeUnit::Cycles => "cycles",
            TimeUnit::Ticks => "ticks",
            TimeUnit::Ns => "ns",
            TimeUnit::Us => "us",
            TimeUnit::Ms => "ms",
            TimeUnit::S => "s",
            TimeUnit::Min => "min",
            TimeUnit::H => "h",
        }
    }

    /// Nanoseconds per unit for the wall-clock units.
    pub fn nanos(self) -> Option<u64> {
        Some(match self {
            TimeUnit::Ns => 1,
            TimeUnit::Us => 1_000,
            TimeUnit::Ms => 1_000_000,
            TimeUnit::S => 1_000_000_000,
            TimeUnit::Min => 60_000_000_000,
            TimeUnit::H => 3_600_000_000_000,
            TimeUnit::Cycles | TimeUnit::Ticks => return None,
        })
    }
}

/// A byte size (region sizes), already in bytes.
pub type Bytes = u64;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Level {
    A,
    B,
    C,
    D,
    E,
}

impl Level {
    pub fn parse(s: &str) -> Option<Level> {
        Some(match s {
            "A" => Level::A,
            "B" => Level::B,
            "C" => Level::C,
            "D" => Level::D,
            "E" => Level::E,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Level::A => "A",
            Level::B => "B",
            Level::C => "C",
            Level::D => "D",
            Level::E => "E",
        }
    }
    /// The Kernel encoding (0 = A).
    pub fn code(self) -> u8 {
        match self {
            Level::A => 0,
            Level::B => 1,
            Level::C => 2,
            Level::D => 3,
            Level::E => 4,
        }
    }
}

/// The closed response sets of R14.5 (DD-10).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Response {
    EndJob,
    RestartPartition,
    StopPartition,
    SafeState,
    RecordOnly,
    RecordAndContinue,
}

impl Response {
    pub fn parse(s: &str) -> Option<Response> {
        Some(match s {
            "END_JOB" => Response::EndJob,
            "RESTART_PARTITION" => Response::RestartPartition,
            "STOP_PARTITION" => Response::StopPartition,
            "SAFE_STATE" => Response::SafeState,
            "RECORD_ONLY" => Response::RecordOnly,
            "RECORD_AND_CONTINUE" => Response::RecordAndContinue,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            Response::EndJob => "END_JOB",
            Response::RestartPartition => "RESTART_PARTITION",
            Response::StopPartition => "STOP_PARTITION",
            Response::SafeState => "SAFE_STATE",
            Response::RecordOnly => "RECORD_ONLY",
            Response::RecordAndContinue => "RECORD_AND_CONTINUE",
        }
    }
    /// The Kernel encoding (`rsk_kernel::logic::RESP_*`).
    pub fn code(self) -> u8 {
        match self {
            Response::EndJob => 0,
            Response::RestartPartition => 1,
            Response::StopPartition => 2,
            Response::SafeState => 3,
            Response::RecordOnly => 4,
            Response::RecordAndContinue => 5,
        }
    }
    pub fn usable_as_fault(self) -> bool {
        matches!(self, Response::EndJob | Response::RestartPartition | Response::StopPartition | Response::SafeState)
    }
    pub fn usable_as_overrun(self) -> bool {
        matches!(
            self,
            Response::EndJob
                | Response::RestartPartition
                | Response::StopPartition
                | Response::SafeState
                | Response::RecordAndContinue
        )
    }
    pub fn usable_as_deadline_miss(self) -> bool {
        matches!(
            self,
            Response::RestartPartition | Response::StopPartition | Response::SafeState | Response::RecordOnly
        )
    }
    /// Stops overrunning Jobs (R18.7, R29.2): every Overrun_Response but RECORD_AND_CONTINUE.
    pub fn stops_overruns(self) -> bool {
        !matches!(self, Response::RecordAndContinue)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MitPolicy {
    Defer,
    Discard,
}

impl MitPolicy {
    pub fn parse(s: &str) -> Option<MitPolicy> {
        Some(match s {
            "defer" => MitPolicy::Defer,
            "discard" => MitPolicy::Discard,
            _ => return None,
        })
    }
    pub fn name(self) -> &'static str {
        match self {
            MitPolicy::Defer => "defer",
            MitPolicy::Discard => "discard",
        }
    }
}

/// A Task's release kind (PR-10, PR-11).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Release {
    Periodic {
        period: Time,
        /// Defaults to 0 ticks.
        offset: Option<Time>,
    },
    Sporadic {
        mit: Time,
        policy: MitPolicy,
        /// The single Release_Source: an interrupt of an owned peripheral
        /// (`binds = NAME`) or a Release_Signal (`signal = NAME`).
        source: Source,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Source {
    Interrupt(String),
    Signal(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskDecl {
    pub name: String,
    pub release: Release,
    pub deadline: Time,
    pub budget: Time,
    /// Declared Priority, 1..=7 (PR-33).
    pub priority: u8,
    /// Core assignment (R27.1); v1 accepts only 0 (NG-01).
    pub core: u8,
    pub fpu: bool,
    /// Names of the Resources of the Partition the Task accesses (R8.7).
    pub resources: Vec<String>,
    /// Release_Signals the Task may raise.
    pub raises: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDecl {
    pub name: String,
    /// The Rust type, as written.
    pub ty: String,
    /// The initializer expression, as written.
    pub init: String,
    /// `core` or `global` (R27.1); v1 accepts only core 0.
    pub placement: ResourcePlacement,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ResourcePlacement {
    Core(u8),
    Global,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SignalDecl {
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PartitionDecl {
    pub name: String,
    /// The Rust crate (path) that holds the Partition's code; defaults to the name.
    pub krate: String,
    pub level: Level,
    pub code: Bytes,
    pub ram: Bytes,
    pub stack: Bytes,
    pub fault: Response,
    pub overrun: Response,
    pub deadline_miss: Response,
    pub mit_threshold: u32,
    /// Names of owned peripherals of the Target (R15.1).
    pub peripherals: Vec<String>,
    pub resources: Vec<ResourceDecl>,
    pub tasks: Vec<TaskDecl>,
    pub signals: Vec<SignalDecl>,
    /// Init_Arena size (PR-15), 0 when absent.
    pub init_arena: Bytes,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Declaration {
    /// Target identifier (see `target::Target::by_name`).
    pub target: String,
    /// Profile_Variant name.
    pub variant: String,
    pub operating_duration: Time,
    /// The Rust path of the system safe state function `fn() -> !`.
    pub safe_state: String,
    /// Health_Monitor event log capacity (R14.6).
    pub log_capacity: u32,
    pub partitions: Vec<PartitionDecl>,
}

/// Where a diagnostic points (the front end maps it to a span).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ItemRef {
    Declaration,
    Field(String),
    Partition(String),
    PartitionField(String, String),
    Task(String, String),
    TaskField(String, String, String),
    Resource(String, String),
    Signal(String, String),
    Peripheral(String, String),
}

impl std::fmt::Display for ItemRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ItemRef::Declaration => write!(f, "app"),
            ItemRef::Field(x) => write!(f, "app.{x}"),
            ItemRef::Partition(p) => write!(f, "partition {p}"),
            ItemRef::PartitionField(p, x) => write!(f, "partition {p}.{x}"),
            ItemRef::Task(p, t) => write!(f, "task {p}.{t}"),
            ItemRef::TaskField(p, t, x) => write!(f, "task {p}.{t}.{x}"),
            ItemRef::Resource(p, r) => write!(f, "resource {p}.{r}"),
            ItemRef::Signal(p, s) => write!(f, "signal {p}.{s}"),
            ItemRef::Peripheral(p, x) => write!(f, "peripheral {p}.{x}"),
        }
    }
}

/// A Generator diagnostic: the rule violated (a PR identifier or a
/// requirement), the item, and the message (R3.1, R21.4).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: String,
    pub item: ItemRef,
    pub message: String,
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "error[{}] {}: {}", self.rule, self.item, self.message)
    }
}

impl Diagnostic {
    pub fn new(rule: &str, item: ItemRef, message: impl Into<String>) -> Diagnostic {
        Diagnostic {
            rule: rule.to_string(),
            item,
            message: message.into(),
        }
    }
}
