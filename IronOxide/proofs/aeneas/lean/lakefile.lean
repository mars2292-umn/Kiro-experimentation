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
