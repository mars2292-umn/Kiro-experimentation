import Lake
open Lake DSL

-- The Aeneas Lean library of the pinned checkout (toolchain/decision.toml
-- [aeneas]), a sibling of the repository like the Creusot checkout.
require aeneas from "../../../../aeneas/backends/lean"

package «controllaw» where
  leanOptions := #[⟨`weak.linter.dupNamespace, false⟩]

/-- `ControlLaw/Controllaw.lean`: the Aeneas translation of
`proofs/aeneas/controllaw` (generated, committed); `ControlLaw/Proofs.lean`:
the handwritten proofs. -/
@[default_target] lean_lib «ControlLaw» {
  globs := #[.submodules `ControlLaw]
}

/-- `Rta/Rta.lean`: the Aeneas translation of the Analyzer's response-time
fixed point (`crates/rsk-analyzer/src/rta.rs`, compiled without Verus: the
ghost code erased), generated and committed; `Rta/Proofs.lean`: the
handwritten proofs (Verification_Plan 7.2). -/
@[default_target] lean_lib «Rta» {
  globs := #[.submodules `Rta]
}

/-- `Sched/Kernel.lean`: the Aeneas translation of the Kernel's entry
points as `rsk-kernel` links them (`Bound`'s `init`, `start`, `eligible`,
`select`, `dispatch`, `job_return`, `svc_lock`, `svc_unlock`, `intake`,
`timer_event`, and everything of `rsk_logic::logic` they call: the
scheduler state functions, `Kernel::dispatch`, `periodic_release`,
`intake_release`, `apply_response`, ...), from the crates as rustc compiles
them without Verus, generated and committed. The other files are
handwritten (Verification_Plan 7.3). -/
@[default_target] lean_lib «Sched» {
  globs := #[.submodules `Sched]
}

/-- `Proto/Proto.lean`: the Aeneas translation of `rsk-protocol`'s
`multiparty::accept_multiparty` and what it calls (the well-formedness
check, the full merge, the projection), from the crate as rustc compiles it
without Verus, generated and committed. The other files are handwritten:
`Proto/Spec.lean` states the Protocol's syntax, the acceptance condition and
the asynchronous semantics, `Proto/Progress.lean` proves that an accepted
Protocol has no reachable waiting set (R41.3), and `Proto/Exec.lean` that
the extracted checker returns `true` only for an accepted Protocol
(Verification_Plan 5.20). -/
@[default_target] lean_lib «Proto» {
  globs := #[.submodules `Proto]
}
