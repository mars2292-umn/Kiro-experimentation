//! Diagnostics that name the violated rule.

use std::fmt;

/// The rule a diagnostic enforces. [`Rule::id`] gives the requirement and
/// Profile restriction identifiers printed in every diagnostic.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Rule {
    /// PR-20 and R6.3: `unsafe` code is confined to the Kernel_Unsafe_Module.
    UnsafeConfinement,
    /// R6.1 (PR-15, PR-35): the Kernel is `no_std` without `std` or `alloc`.
    KernelNoStd,
    /// R6.2 and PAR-01: the Kernel's executable line count.
    KernelSize,
    /// PR-20 and R6.4: every `unsafe` construct of the Kernel_Unsafe_Module
    /// carries a justification identifier linked to a proof, harness, or review.
    UnsafeJustification,
    /// PR-36 and R55.1: no `#![feature]` and no `RUSTC_BOOTSTRAP`.
    NoUnstable,
    /// R55.2: the upstream stable rustc on which the Qualified_Toolchain is based.
    Toolchain,
    /// R55.5: one edition for all rsk crates.
    Edition,
    /// R58.1: lock-file-only resolution from vendored sources.
    Locked,
    /// R58.4: build scripts and proc-macros run without network access.
    Offline,
    /// R58.2 and R58.5: Flight_Build crates with unsafe code, assembly, build
    /// scripts, or proc-macros have Trust_Base_Register entries.
    TrustBase,
    /// R59.1 and R59.3: clean-checkout rebuilds give identical binary hashes.
    Reproducible,
    /// R50.3 and R33.3: Build_Manifest and Evidence_Item generation.
    Evidence,
    /// Requirements 1, 2, 4, 27.3, and 57: the Profile source and its
    /// generated outputs.
    Profile,
    /// R44.6, R50.2, R3.10: the Kernel_Proofs and the assumption gate.
    Proof,
    /// R45: the Kani_Harnesses.
    Kani,
    /// The build policy table is missing or inconsistent.
    Policy,
    /// A build, test, or tool step failed.
    Step,
    /// R21 to R25: the Generator rejected a declaration, or the generated
    /// files are not current (R24.4).
    Generator,
}

impl Rule {
    pub fn id(self) -> &'static str {
        match self {
            Rule::UnsafeConfinement => "PR-20/R6.3",
            Rule::KernelNoStd => "R6.1",
            Rule::KernelSize => "R6.2/PAR-01",
            Rule::UnsafeJustification => "PR-20/R6.4",
            Rule::NoUnstable => "PR-36/R55.1",
            Rule::Toolchain => "R55.2",
            Rule::Edition => "R55.5",
            Rule::Locked => "R58.1",
            Rule::Offline => "R58.4",
            Rule::TrustBase => "R58.2/R58.5",
            Rule::Reproducible => "R59.1/R59.3",
            Rule::Evidence => "R50.3",
            Rule::Profile => "PROFILE",
            Rule::Proof => "R44.6/R50.2",
            Rule::Kani => "R45.5",
            Rule::Policy => "BUILD-POLICY",
            Rule::Step => "BUILD-STEP",
            Rule::Generator => "GENERATOR",
        }
    }
}

/// One violation, with an optional `path[:line]` location relative to the
/// workspace root.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Diagnostic {
    pub rule: Rule,
    pub location: Option<String>,
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "error[{}]", self.rule.id())?;
        if let Some(location) = &self.location {
            write!(f, " {location}")?;
        }
        write!(f, ": {}", self.message)
    }
}

/// Collected results of a set of checks.
#[derive(Debug, Default)]
pub struct Report {
    pub diagnostics: Vec<Diagnostic>,
    /// Informational lines that document what a check covered.
    pub notes: Vec<String>,
    /// Conditions worth attention that do not fail the job.
    pub warnings: Vec<String>,
}

impl Report {
    pub fn error(&mut self, rule: Rule, location: Option<String>, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic {
            rule,
            location,
            message: message.into(),
        });
    }

    pub fn note(&mut self, note: impl Into<String>) {
        self.notes.push(note.into());
    }

    pub fn warn(&mut self, warning: impl Into<String>) {
        self.warnings.push(warning.into());
    }

    pub fn has_rule(&self, rule: Rule) -> bool {
        self.diagnostics.iter().any(|d| d.rule == rule)
    }

    /// Moves everything from `other` into `self`.
    pub fn absorb(&mut self, other: Report) {
        self.diagnostics.extend(other.diagnostics);
        self.notes.extend(other.notes);
        self.warnings.extend(other.warnings);
    }
}
