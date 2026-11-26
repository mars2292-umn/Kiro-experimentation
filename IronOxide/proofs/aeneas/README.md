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
  and `property18_timed` is Property 18 over it. Every transition keeps
  each nominal instant or moves it by a period, from the code
  (`kstep_nomrel`; `serve_slot_nomrel` for a served slot of any kind).
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
  Every transition changes the sporadic timing fields only as `SpRel`
  allows, from the code (`kstep_sprel`, `serve_slot_sprel`).
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

`lean/Sched/Slots.lean` proves the Kernel's Verus invariant
`budget_slots_owned` a second time, on the extracted functions: an armed
Budget slot is the executing Job's in every state the run reaches
(`kreachb_bo`), from which the premises of the run's Budget transitions
about the slot follow (`budget_slot_top`, `KStep.served_bevent_reach`).

`lean/Sched/Deadline.lean` proves an invariant Verus did not state until
2026-11-25 (`Kernel::deadline_slots_owned`):
outside the safe state, an armed deadline slot belongs to a Task with a
Release pending or a Job in progress (`kreachs_di`), so the Job of a
served deadline slot is not complete (`deadline_slot_incomplete`,
`KStep.served_deadline_reach`).

`lean/Sched/Restore.lean` (2026-11-25) has two statements Verus proves
over its ghost history: the System_Ceiling is restored by the unlock that
matches a lock across whatever other Jobs did in between
(`ceiling_restored`, over the run of `Sched/Run.lean`), and deadlock
freedom (`deadlock_free`, `kreach_deadlock_free`).

`lean/Sched/MaskFrame.lean` and `lean/Sched/Intakes.lean` (2026-11-25)
have Property 9 (R19.1): a source's mask is lifted only once its MIT
window has passed (`KM`, one theorem per function of the run), every
outcome of an intake masks the source (`intake_release_chain`), and two
effective intakes of a source are at least one MIT apart, with the count
bound (`irun_spaced`, `irun_count`). `MaskFrame.lean` is generated from
`SpFrame.lean` with the relation in place of the equalities, then kept by
hand.

Since 2026-11-17 no transition of the run is modelled by hand: each one
that changes the Kernel's state carries the extracted function's result.
`lean/Sched/Endpoint.lean` reads the Endpoint entry cores and `Bound`'s
`intake` (`bound_intake`, `kernel_intake_cases`, `KStep.bound_intake_run`).
The plan lists every condition of the run (7.3, "What remains").

Regenerating the translation:

```sh
cd crates/rsk-kernel
I='{impl crate::arch::bound::KernelOps for _}'
E='crate::arch::cm4::entry'
T='{impl crate::arch::cm4::entry::Tables for _}'
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$I::init,$I::start,$I::eligible,$I::select,$I::dispatch,$I::job_return,$I::svc_lock,$I::svc_unlock,$I::intake,$I::timer_event,$I::fault,$I::fault_of,$I::record_event,$I::settle,$I::job_info,$I::depth,$I::message_permitted,$I::partition_stopped,$I::take_reinit,$I::take_stop_dma,$I::next_compare,$I::basepri_now,$I::top,$I::boot_failed,$I::boot_info,$I::validate,$I::safe_state,$I::task_count,$I::partition_count,$I::source_masked,$I::started,$I::stack_at,$I::is_killed,$I::partition_view,$I::dma_permitted,$I::in_ro_window,$I::config_checksum,$I::layout_checksum,$E::endpoint_send_core,$E::endpoint_recv_core,$E::svc_core,$E::timer_core,$E::irq_entry_core,$E::epilogue_core,$E::fault_core,$E::start_core,$E::boot_core,$E::apply_core,$T::binding,$T::dispatch_vector,$T::intake_vector,$T::stack_top,$T::stack_bottom,$T::task_entry_address,$T::irq_count,$T::config_checksum,$T::layout_checksum,$T::timer_vector,$T::priority_bits,$T::boot_expect,$T::fp_flush_to_zero,$T::tick_hz,$T::dma_count,$T::dma,$T::endpoint_count,$T::endpoint,$T::endpoint_storage,$T::signal_count,$T::signal" \
    --include 'rsk_logic::logic::kernel' --include 'rsk_logic::logic::sched' \
    --include 'rsk_logic::logic::time' --include 'rsk_logic::logic::budget' \
    --include 'rsk_logic::logic::log' --include 'rsk_logic::logic::mpu' \
    --include 'rsk_logic::logic::_' --include 'rsk_logic::hw_model::MIN_COMPARE_DISTANCE' \
    --include 'vstd::seq::Seq' --include 'verus_builtin::Ghost' --include 'verus_builtin::_::assume_new' --include 'verus_builtin::Tracked' \
    --dest-file /tmp/kernel.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest ../../proofs/aeneas/lean/Sched /tmp/kernel.llbc
```

Since 2026-11-17 the start set also names the entry cores of
`arch/cm4/entry.rs` (the Endpoint operations, the SVC, timer, interrupt,
unwind, fault, start and boot cores, and through them the dispatch,
intake, EasyDMA and channel functions) and the rest of `KernelOps` they
call: 315 functions on 2026-11-17, none without a body (363 definitions on
2026-11-20, `Layout::validate` the one function among them without a body). The cores are generic over
`KernelOps`, `KernelArch` and `Tables`, which Aeneas translates as
structures of functions. A `Ghost(..)` value built in executable code is
a closure in the build without Verus, which Aeneas rejects in the function
that holds it; the cores build every ghost value through a function of
`arch/cm4/ghostv.rs`, and those seven were kept opaque until 2026-11-20
(declared `axiom` in the output, like `no_history`; see "No opaque function"
below). `lean/Sched/Endpoint.lean` is the first
proof over the entry cores: what `endpoint_send_core` and
`endpoint_recv_core` do to the Kernel (at most one interface call:
`fault`, `record_event` or `intake`), and that an Endpoint operation is
the run's `svc_h` transition, followed by `faulted` for an API misuse.

The name of the `.llbc` file gives the Lean file's name (`Kernel.lean`).
The `--include` options make Charon translate the bodies of the
dependency's items and the erased ghost types (without them Aeneas
declares axioms); an inherent method is named `crate::module::_::name`, a
trait method `{impl Trait for _}::name`, and a `--start-from` must name an
item of the crate being compiled (Charon panics on a dependency's). Until
2026-11-20 Aeneas reported errors for `Layout::validate` and its helpers
(returns from inside a loop) and the function was an `axiom` of the output.
Since that date its five functions (`validate`, `validate_kernel_windows`,
`check_apart`, `validate_partition`, `validate_pair` in
`crates/rsk-logic/src/logic/mpu.rs`) run their loops to a verdict, with the
contracts unchanged, and are translated; `lean/Sched/LayoutTotal.lean`
proves that `Layout::validate` returns for every layout and that a layout
it accepts is well formed (`LWf`, after `Layout::wf`); `lean/Sched/LayoutIso.lean`
derives the isolation theorems' hypothesis `LayoutOk` from it. One more thing Aeneas needs of such code: an interval
read through `self` inside a loop whose branches join (`self.shared_code`
in `validate_kernel_windows`) is copied to a local before the loop ("Could
not match the contexts" otherwise). The start set names seven more methods
of `KernelOps` since then (`stack_at`, `is_killed`, `partition_view`,
`dma_permitted`, `in_ro_window`, `config_checksum`, `layout_checksum`), so
that every method the entry cores call has a body.
Since 2026-11-20 `Config::validate`, `validate_partitions` and
`validate_slots` are translated: their loops run to a verdict and the
functions return after them (`lean/Sched/Boot.lean` derives the
configuration's bounds from `Config::validate` returning `true`). Two
things Aeneas needs of such code: no `return` inside a loop, and no
`let x = a == B || c == D;` (the term it emits is typed as a proposition
by Lean; write `if a == B || c == D { true } else { false }`). The start
set also names `top`, `boot_failed`, `boot_info`, `validate` and five
read-only methods of `KernelOps` since that date.
`no_history` (the empty ghost history of `Kernel::new`) was kept opaque
until 2026-11-20: its body constructs a `Ghost` value, which the erased
build turns into a closure that Aeneas does not translate. It was the one
`axiom` of the file at the time;
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

## The register drivers (Verification_Plan, 2026-11-18)

`lean/Hw/Drivers.lean` is the Aeneas translation of
`crates/rsk-kernel/src/arch/cm4/drivers.rs`: the NVIC, TIMER0 and boot
configuration drivers as functions of a register file. `lean/Hw/Model.lean`
(handwritten) gives the register model and proves each driver's effect on
it. `lake build Hw` checks them.

Regenerating the translation:

```sh
cd crates/rsk-kernel
D='crate::arch::cm4::drivers'
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$D::nvic_enable,$D::nvic_disable,$D::nvic_set_pending,$D::nvic_clear_pending,$D::nvic_set_priority,$D::set_prigroup_all_preempt,$D::set_nonbasethrdena,$D::set_disdefwbuf,$D::set_kernel_exception_priorities,$D::fpu_configure_eager,$D::mpu_enable_background,$D::dwt_enable,$D::timer0::now,$D::timer0::arm,$D::timer0::ack,$D::timer0::cancel,$D::timer0::rearm" \
    --dest-file /tmp/drivers.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest /tmp/drivers-out /tmp/drivers.llbc
cp /tmp/drivers-out/Drivers.lean ../../proofs/aeneas/lean/Hw/Drivers.lean
```

## The Generator's arithmetic and the Config_Checker's core (Verification_Plan, 2026-11-18)

`lean/Tools/Gen.lean` and `lean/Tools/Cc.lean` are the Aeneas translations
of `crates/rsk-gen/src/derive_v.rs` and `crates/rsk-confcheck/src/derive_v.rs`;
`lean/Tools/GenProofs.lean` and `lean/Tools/CcProofs.lean` (handwritten)
prove of each function what its Verus contract states. `lake build Tools`
checks them. The two proof files do not import each other: the tools'
specifications are written apart (R26.1).

Regenerating the translations:

```sh
cd crates/rsk-gen
D='crate::derive_v'
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$D::priority_byte,$D::ceiling,$D::slot_plan,$D::scale,$D::region_log2" \
    --dest-file /tmp/gen.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest /tmp/gen-out /tmp/gen.llbc
cp /tmp/gen-out/Gen.lean ../../proofs/aeneas/lean/Tools/Gen.lean

cd ../rsk-confcheck
../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$D::encode_level,$D::ceiling_of,$D::sum_checked,$D::timer_slots,$D::slot_index,$D::levels_of,$D::size_of,$D::aligned,$D::disjoint,$D::stack_fits" \
    --dest-file /tmp/cc.llbc -- --lib
../../../aeneas/bin/aeneas -backend lean -dest /tmp/cc-out /tmp/cc.llbc
cp /tmp/cc-out/Cc.lean ../../proofs/aeneas/lean/Tools/Cc.lean
```

## No opaque function since 2026-11-20

Until 2026-11-20 the translation kept eight functions opaque (`axiom`):
`Kernel::no_history` and the seven helpers of `arch/cm4/ghostv.rs`, each of
which builds a `Ghost` value, a closure in the build without Verus. Each now
has two bodies: the verified one under `cfg(verus_keep_ghost)`, and in every
other build `Ghost::assume_new()`, which constructs the zero-sized marker
with no closure. The extraction command above therefore has no `--opaque`
option and includes `assume_new`'s body; `Sched/Kernel.lean` declares no
`axiom`, and `Sched/GhostTotal.lean` proves the eight return. What this
costs: for these eight functions the compiled text is not the verified
text. Both produce the same zero-sized value.

## The generated tables (`Tab`) and their adapter

Kernel_Arch reads the generated tables through `SysTables`
(`arch/cm4/mod.rs`), which forwards each method of `Tables` to the system
crate. Since 2026-11-20 the system crate's contract is two traits:
`SystemTables` (the constants and the accessors, pure functions of their
argument) and `System: SystemTables` (the Kernel static, the Task entry
functions, the safe state). `SysTables` depends on `SystemTables` alone, so
that it involves no function-pointer type (Aeneas: "Arrow types are not
supported yet") and is part of `Sched/Kernel.lean` (`$T` in the start set
above). `Sched/TablesAdapter.lean` proves that every method of the adapter
returns when the `SystemTables` under it does (`STabT`), and
`Sched/CoreSys.lean` states the entry cores' theorems with that.

`Tab/<Configuration>/Tables.lean` is the translation of `impl SystemTables
for Sys` of one configuration's system crate, one directory for each of the
P1 demo's thirteen configurations and one for the HIL stimulus application:

```sh
cd apps/p1-demo/system          # or apps/hil-stimulus/system
I='{impl rsk_kernel::arch::SystemTables for _}'
../../../../aeneas/charon/bin/charon cargo --preset=aeneas \
    --start-from "$I::dma,$I::signal,$I::endpoint,$I::endpoint_storage,$I::binding,$I::dispatch_vector,$I::intake_vector,$I::stack_top,$I::stack_bottom" \
    --include 'rsk_kernel::arch::_' --include 'rsk_logic::logic::_' \
    --opaque 'rsk_kernel::arch::cm4::unsafe_module::hw::_::base' --opaque 'crate::ENDPOINTS' \
    --dest-file /tmp/tables.llbc -- --lib <features>
../../../../aeneas/bin/aeneas -backend lean -dest ../../../proofs/aeneas/lean/Tab/<Configuration> /tmp/tables.llbc
```

`<features>` is empty for `Qemu` and `Stimulus`, and `--no-default-features
--features <f>` otherwise, with `<f>` = `qemu-flood` (`QemuFlood`),
`nrf52840`, `timing`, `timing-short`, `response`, `stress`, `stress-short`,
`isolation`, `cycles`, `cycles-fpu`, `plain-measure` (`Measure`),
`coverage-kernel` (`Coverage`).

The Endpoint table `ENDPOINTS` is kept opaque: with its content, Lean took
more than 17 minutes and 11 GB to elaborate the QEMU configuration's file.
`EndpointStorage::base` is a pointer cast of the Kernel_Unsafe_Module and
stays opaque. `Tab/<Configuration>/Proofs.lean` (one template, the table
sizes and the case splits over them differ) proves that every translated
accessor returns for every argument (`endpoint` given that its table
constant evaluates). Until 2026-11-20 `dispatch_vector`, `intake_vector`,
`stack_top` and `stack_bottom` indexed their table with the argument and
failed outside its range; the Generator now emits a test of the argument.

Each `Tab` file is a translation of a system crate, in that crate's
namespace with its own copy of the Kernel's types. No theorem identifies
its functions with the fields of the `SystemTables` that
`Sched/TablesAdapter.lean` speaks of: `STabT` states of an abstract
`SystemTables` what `Tab/<Configuration>/Proofs.lean` proves of the
generated functions, and the two meet by name. A single translation holding
both would need the adapter reachable from an item of the system crate.

## The register drivers return (`Hw/Total.lean`)

`Hw/Total.lean` proves that each driver of `arch/cm4/drivers.rs` returns
for every argument, over a register file whose reads and writes return
(`RegsT`) and a time extension that returns (`ExtendT`). The real
`KernelArch` implementation (`Cm4Hw`, `entry.rs`) and the functions between
it and the drivers (`unsafe_module/hw.rs`) are not translated: Aeneas stops
on raw-pointer dereferences, a function pointer and a ghost closure in
them. `HwT` (`Sched/CoreTotal.lean`) therefore remains an assumption, with
this file as the part of it that is Kernel code outside the
Kernel_Unsafe_Module.
