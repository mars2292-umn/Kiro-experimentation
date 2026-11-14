# Aeneas evaluation (task 17.3, R46.1)

One pure control law (`controllaw/`: a fixed-point PI step with anti-windup
and output saturation) translated to Lean by Aeneas and proven there, to
evaluate Aeneas as a Lean-level alternative to the App_Prover (Creusot) for
small computations. The evaluation is recorded in the Verification_Plan,
section 7; the pins in `toolchain/decision.toml` `[aeneas]`.

Layout:

- `controllaw/`: the Rust crate (no dependencies, `#![no_std]`,
  `#![forbid(unsafe_code)]`), excluded from the workspace; its own tests run
  with `cargo test` in the directory.
- `lean/`: the Lean project. `ControlLaw/Controllaw.lean` is the Aeneas
  translation (generated, committed as a reviewable artifact);
  `ControlLaw/Proofs.lean` holds the handwritten proofs. `lake build` checks
  them; the verification job runs it where `lake` is installed.

Regenerating the translation (needs the pinned Charon and Aeneas built in
the sibling checkout `../../../aeneas`, see its README):

```sh
cd proofs/aeneas/controllaw
../../../../aeneas/charon/bin/charon cargo --preset=aeneas      # writes controllaw.llbc
../../../../aeneas/bin/aeneas -backend lean -dest ../lean/ControlLaw controllaw.llbc
```

The `.llbc` file is not committed (`.gitignore`); the Lean translation is.

## The Analyzer on the code as compiled (Verification_Plan 7.2)

`lean/Rta/Rta.lean` is the Aeneas translation of the Analyzer's numeric
chain, taken from `crates/rsk-analyzer` itself as rustc compiles it without
Verus (the `verus!` macro erases the ghost code): `terms::derive_all` with
the functions it calls, `terms::terms_for`, `rta::fixpoint`, and
`terms::response_of`. The handwritten proofs:

- `lean/Rta/Proofs.lean`: the fixed point (`fixpoint_spec`);
- `lean/Rta/Chain.lean`: the terms of a Task's recurrence and the verdict
  for one Task end to end (`terms_for_spec`, `response_of_spec`);
- `lean/Rta/Derive.lean`: the term derivation (`derive_all_spec`).

`lake build` in `lean/` checks them with the control law's.

Regenerating the translation:

```sh
cd crates/rsk-analyzer
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from 'crate::terms::response_of' --start-from 'crate::terms::derive_all' \
    --dest-file /tmp/rta.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Rta /tmp/rta.llbc
```

`--lib` keeps Charon to the library target. Charon builds the crate's
dependencies with its pinned nightly, `vstd` and the Verus macros among
them; set `CARGO_TARGET_DIR` to keep that build out of the workspace's
target directory. A local variable of the Rust source named like a module
(`terms`, `rta`) shadows the module in the generated Lean and breaks it:
`response_of` names its terms `own_terms` for that reason.
