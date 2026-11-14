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
Verus (the `verus!` macro erases the ghost code): `terms::analyse_all` and
everything it calls (`terms::derive_all`, `terms::terms_for`,
`rta::fixpoint`, `terms::response_of`). The handwritten proofs:

- `lean/Rta/Proofs.lean`: the fixed point (`fixpoint_spec`);
- `lean/Rta/Chain.lean`: the terms of a Task's recurrence and the verdict
  for one Task end to end (`terms_for_spec`, `response_of_spec`);
- `lean/Rta/Derive.lean`: the term derivation (`derive_all_spec`).
- `lean/Rta/Whole.lean`: the whole numeric analysis, from the Task set to
  every Task's verdict (`analyse_all_spec`).

`lake build` in `lean/` checks them with the control law's.

Regenerating the translation:

```sh
cd crates/rsk-analyzer
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from 'crate::terms::analyse_all' \
    --dest-file /tmp/rta.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Rta /tmp/rta.llbc
```

`--lib` keeps Charon to the library target. Charon builds the crate's
dependencies with its pinned nightly, `vstd` and the Verus macros among
them; set `CARGO_TARGET_DIR` to keep that build out of the workspace's
target directory. A local variable of the Rust source named like a module
(`terms`, `rta`) shadows the module in the generated Lean and breaks it:
`response_of` names its terms `own_terms` for that reason.

## Property 18 over the Kernel's scheduler (Verification_Plan 7.3)

`lean/Sched/Sched.lean` is the Aeneas translation of the Kernel's scheduler
state functions (`crates/rsk-logic/src/logic/sched.rs`: `new`,
`make_pending`, `start`, `finish_top`, `lock`, `unlock` and what they
call), and `lean/Sched/Eligible.lean` that of the eligibility test as
executed (`Bound::eligible`, `crates/rsk-kernel/src/arch/bound.rs`), both
from the Verus-erased build. The handwritten files:

- `lean/Sched/Model.lean`, `lean/Sched/Locks.lean`: what each extracted
  function returns (`make_pending_ok`, `start_ok`, `eligible_ok`,
  `lock_ok`, `unlock_ok`, `finish_top_ok`), and the correspondence between
  the two translations' copies of the Kernel's types;
- `lean/Sched/Views.lean`: the scheduler state as functions over the
  naturals;
- `lean/Sched/Run.lean`: the run (`Step`, `Reach`) and the structure of the
  scheduler state in it, the Stack Resource Policy included (`Wf.step`);
- `lean/Sched/Window.lean`: the busy-window argument over the naturals;
- `lean/Sched/Stage.lean`: the bound over the run (`stage2`, `stage1`);
- `lean/Sched/Link.lean`: the bound with the response time `response_of`
  returns (`property18_stage2`, `property18_stage1`).

Stages 1 and 2 only (interference and SRP blocking; no overhead, no
jitter); the plan states what is assumed and what is not done.

Regenerating the translations:

```sh
cd crates/rsk-logic
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from 'crate::logic::sched::_::new,crate::logic::sched::_::make_pending,crate::logic::sched::_::start,crate::logic::sched::_::finish_top,crate::logic::sched::_::lock,crate::logic::sched::_::unlock' \
    --dest-file /tmp/sched.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Sched /tmp/sched.llbc

cd ../rsk-kernel
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from '{impl crate::arch::bound::KernelOps for _}::eligible' \
    --include 'rsk_logic::logic::sched::REL_PENDING' --include 'rsk_logic::logic::sched::JOB_NONE' \
    --include 'vstd::seq::Seq' --include 'verus_builtin::Ghost' --include 'verus_builtin::Tracked' \
    --dest-file /tmp/eligible.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Sched /tmp/eligible.llbc
```

The `--include` options make Charon translate the two constants and the
erased ghost types; without them Aeneas declares them as axioms. An
inherent method is named `crate::module::_::name`, a trait method
`{impl Trait for _}::name`. `Sched::select` is left out: Aeneas does not
translate its loop.

