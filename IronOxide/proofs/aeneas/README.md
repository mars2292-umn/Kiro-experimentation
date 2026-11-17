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

## Property 18 over the Kernel's own functions (Verification_Plan 7.3)

`lean/Sched/Kernel.lean` is the Aeneas translation of the Kernel's entry
points as `rsk-kernel` gives them to Kernel_Arch (`KernelOps` for `Bound`,
`crates/rsk-kernel/src/arch/bound.rs`: `init`, `start`, `eligible`,
`select`, `dispatch`, `job_return`, `svc_lock`, `svc_unlock`, `intake`,
`timer_event`) and of everything in `rsk_logic::logic` they call: the
scheduler state functions and `select` (`logic/sched.rs`), the Kernel's
transitions (`logic/kernel.rs`: `dispatch`, `job_return`, `svc_lock`,
`svc_unlock`, `intake_release`, `periodic_release`, `deferred_release`,
`deadline_miss`, `budget_event`, `apply_response`, ...), `Timing`, `Slots`,
`Budget` and `Log`. One translation, from the Verus-erased build.

The handwritten files:

- `lean/Sched/Model.lean`, `lean/Sched/Locks.lean`: what each extracted
  `Sched` function returns (`make_pending_ok`, `defer_ok`,
  `clear_release_ok`, `kill_ok`, `start_ok`, `lock_ok`, `unlock_ok`,
  `finish_top_ok`) and what the eligibility test returns (`eligible_ok`);
- `lean/Sched/Views.lean`: the scheduler state as functions over the
  naturals;
- `lean/Sched/Run.lean`: the specification-level run (`Step`, `Reach`):
  the scheduler state's transitions, each Kernel section at its
  transition, and the structure of the scheduler state in a run, the
  Stack Resource Policy included (`Wf.step`);
- `lean/Sched/Window.lean`: the busy-window argument over the naturals;
- `lean/Sched/Stage.lean`: the bound over the run (`stage4`);
- `lean/Sched/Link.lean`: the run's parameters from the Analyzer's inputs
  (`runParams`) and the bound with the response time `analyse_all` reports
  (`property18`);
- `lean/Sched/Entry.lean`, `lean/Sched/Calls.lean`, `lean/Sched/Select.lean`:
  what the Kernel's own functions do to the scheduler state whenever they
  return (`dispatch_eff`, `job_return_eff`, `svc_lock_eff`,
  `svc_unlock_eff`, `periodic_release_eff`, `deferred_release_eff`,
  `deadline_miss_eff`, `intake_release_eff`, `budget_event_eff`,
  `apply_response_r`, `select_some`, `select_none`), and two facts on the
  code alone (`intake_decision_accept`, `budget_event_reported`);
- `lean/Sched/KRun.lean`: the run over the Kernel's functions (`KStep`,
  `KReach`), the proof that each such run is a specification-level run
  (`ksim`), and `property18_kernel`;
- `lean/Sched/Arrive.lean`: the timed run. A Periodic_Task's Job arrives at
  the Task's nominal instant in the Kernel's state, once per instant, with no
  spacing condition; `treach_kreach` derives the spacing `KRun.lean` asks,
  and `property18_timed` is Property 18 over it. The hand-modelled
  transitions keep the nominal instants by condition.
- `lean/Sched/SpFrame.lean`, `lean/Sched/SpIntake.lean`,
  `lean/Sched/Sporadic.lean`: the run with sporadic arrivals. `SpFrame`
  shows that every Kernel function of the run but `intake_release` and
  `deferred_release` leaves `Timing`'s `has_last`, `last_due` and
  `deferred_until` (generated from the `TimeKept` proofs of `Time.lean`
  with the fields exchanged); `SpIntake` says what those two do to them
  together with the scheduler state; `Sporadic` defines the run in which a
  Sporadic_Task's Job arrives with the transition that accepts its event, at
  the instant the Kernel records, or at the instant a deferred event was
  deferred to, with no spacing condition, proves that each such run is a
  run of `KRun.lean` (`sreach_kreach`), and states `property18_sporadic`.
  The hand-modelled transitions leave the sporadic timing fields by
  condition.
- `lean/Sched/Time.lean`: the time base on the code. The Kernel's reading
  `now` is written by `observe` alone and never goes back; the responses and
  slot handlers leave it and the nominal instants (`TimeKept`); a timer entry
  serves its slots under one reading (`timer_event_reading`); a
  Periodic_Task's nominal instant moves by exactly its period and the release
  slot is armed for it (`advance_periodic_cases`, `periodic_release_nominal`).
  Not joined to the run's instants (Verification_Plan 7.3).
- `lean/Sched/Timer.lean`: the timer entry on the code. `serve_slot` gives a
  slot the handler of its kind for the slot's own Task (`serve_slot_cases`),
  a returning `timer_event` is a chain of served slots each armed and due
  when served (`timer_event_served`), and a served slot of each kind is the
  run's transition of that kind (`KStep.served_*`). The slot of the end of
  the operating duration is the run's `opend` transition (`served_end`), and
  a Budget event as an entry of its own is read from `budget_event`: kept
  scheduler state without an Overrun or under a recording response
  (`served_bevent`, `served_bevent_recorded`), `Sched::finish_top` under the
  response that ends the Job (`served_overrun`), and `finish_top` followed
  by a response under one that stops or restarts the Partition
  (`served_overrun_partition`). Since 2026-11-17 the run's three Budget
  transitions carry `budget_event`'s own Kernel state. The time base is not
  covered.

The plan lists the transitions that are still modelled by hand and every
condition of the run.

Regenerating the translation:

```sh
cd crates/rsk-kernel
I='{impl crate::arch::bound::KernelOps for _}'
E='crate::arch::cm4::entry'
G='crate::arch::cm4::ghostv'
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$I::init,$I::start,$I::eligible,$I::select,$I::dispatch,$I::job_return,$I::svc_lock,$I::svc_unlock,$I::intake,$I::timer_event,$I::fault,$I::fault_of,$I::record_event,$I::settle,$I::job_info,$I::depth,$I::message_permitted,$I::partition_stopped,$I::take_reinit,$I::take_stop_dma,$I::next_compare,$I::basepri_now,$E::endpoint_send_core,$E::endpoint_recv_core,$E::svc_core,$E::timer_core,$E::irq_entry_core,$E::epilogue_core,$E::fault_core,$E::start_core,$E::boot_core,$E::apply_core" \
    --include 'rsk_logic::logic::kernel' --include 'rsk_logic::logic::sched' \
    --include 'rsk_logic::logic::time' --include 'rsk_logic::logic::budget' \
    --include 'rsk_logic::logic::log' --include 'rsk_logic::logic::mpu' \
    --include 'rsk_logic::logic::_' --include 'rsk_logic::hw_model::MIN_COMPARE_DISTANCE' \
    --opaque 'rsk_logic::logic::kernel::_::no_history' \
    --opaque "$G::ghost_snap" --opaque "$G::ghost_int" --opaque "$G::ghost_partition" \
    --opaque "$G::ghost_seq_empty" --opaque "$G::ghost_seq_push_u8" --opaque "$G::ghost_seq_push_usize" \
    --opaque "$G::ghost_seq_drop_last" \
    --include 'vstd::seq::Seq' --include 'verus_builtin::Ghost' --include 'verus_builtin::Tracked' \
    --dest-file /tmp/kernel.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Sched /tmp/kernel.llbc
```

Since 2026-11-17 the start set also names the entry cores of
`arch/cm4/entry.rs` (the Endpoint operations, the SVC, timer, interrupt,
unwind, fault, start and boot cores, and through them the dispatch,
intake, EasyDMA and channel functions) and the rest of `KernelOps` they
call: 315 functions, none without a body. The cores are generic over
`KernelOps`, `KernelArch` and `Tables`, which Aeneas translates as
structures of functions. A `Ghost(..)` value built in executable code is
a closure in the build without Verus, which Aeneas rejects in the function
that holds it; the cores build every ghost value through a function of
`arch/cm4/ghostv.rs`, and those seven are kept opaque (declared `axiom` in
the output, like `no_history`). `lean/Sched/Endpoint.lean` is the first
proof over the entry cores: what `endpoint_send_core` and
`endpoint_recv_core` do to the Kernel (at most one interface call:
`fault`, `record_event` or `intake`), and that an Endpoint operation is
the run's `svc_h` transition, followed by `faulted` for an API misuse.

The name of the `.llbc` file gives the Lean file's name (`Kernel.lean`).
The `--include` options make Charon translate the bodies of the
dependency's items and the erased ghost types (without them Aeneas
declares axioms); an inherent method is named `crate::module::_::name`, a
trait method `{impl Trait for _}::name`, and a `--start-from` must name an
item of the crate being compiled (Charon panics on a dependency's). Aeneas
reports errors for `Config::validate`, `Layout::validate` and their
helpers, which the include pattern names and nothing extracted calls
(breaks and continues to outer loops); they are not in the output.
`no_history` (the empty ghost history of `Kernel::new`) is kept opaque: its
body constructs a `Ghost` value, which the erased build turns into a
closure that Aeneas does not translate. It is the one `axiom` of the file;
`property18_kernel` does not depend on it (`#print axioms`).

What Aeneas needs of the source, found on this code: no `Ghost(..)`
expression in a function to translate (ghost state is assigned in `proof`
blocks); no `Option` carried across loop iterations (`Sched::select`
carries an index); an `if` whose branches are a literal and a comparison
gets the type of a proposition (each branch a literal).

## The multiparty progress theorem on the checker as compiled (Verification_Plan 5.20)

`lean/Proto/Proto.lean` is the Aeneas translation of `rsk-protocol`'s
`multiparty::accept_multiparty` and everything it calls (`acceptn_exec`,
`wfn_exec`, `projn_exec`, `merge_exec`, `close_rec_exec`, ...), from the
Verus-erased build. No axiom, no `sorry`.

The handwritten files, in import order:

- `lean/Proto/Spec.lean`: the statement. Global and local types, `wfn`,
  the full merge, the projection, `acceptn`, the asynchronous semantics
  (`NStep`, `NReach`) and the waiting set (`NDeadlocked`);
- `Refine`, `Merge`, `Runtime`, `RProj`, `Distinct`, `Binders`,
  `ProjNormal`, `MHead`, `Rel`, `Queue`, `Steps`, `Effects`, `Exists`,
  `Across`, `Node`, `Invariant`: the metatheory, a port of the sections of
  `crates/rsk-protocol/src/multiparty.rs` (refinement, the merge's
  properties, runtime types with messages in transit and their
  projection, the coinductive relation between a role's type and the
  runtime type, its preservation across a send and a receive, the
  invariant of a reachable configuration);
- `lean/Proto/Progress.lean`: `multiparty_progress` and (in
  `Invariant.lean`) `multiparty_reception`, over the syntax;
- `lean/Proto/Exec.lean`: what the extracted functions return
  (`merge_exec_ok`, `projn_exec_ok`, `wfn_exec_ok`, `acceptn_exec_ok`)
  and the two theorems on the checker: `accept_multiparty_progress`,
  `accept_multiparty_reception`.

Regenerating the translation:

```sh
cd crates/rsk-protocol
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from 'crate::multiparty::accept_multiparty' \
    --include 'vstd::seq::Seq' --include 'verus_builtin::Ghost' --include 'verus_builtin::Tracked' \
    --dest-file /tmp/proto.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Proto /tmp/proto.llbc
```

What Aeneas needed of this source: no `Vec::pop` (no model in its library;
`wfn_exec` reads its guard environment and copies it at a recursion), no
`let ... else`, no `match` on a borrowed `Option<&T>` carried past another
call (`merge_ls_exec` takes a copy through `lse_lookup_copy`), and no
short-circuit condition mixing calls and comparisons (`wfn_exec` computes
its three conditions first). The mutually recursive types have no
`induction` tactic in Lean: the structural theorems are written as
mutually recursive functions or by strong induction on a size.
