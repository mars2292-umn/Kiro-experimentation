# Implementation Plan

## Overview

The plan follows the five phases of the request. Phase 1 builds the Profile specification, the Verus-proven Kernel on the nRF52840-DK, and the Kani harnesses, and it starts by retiring the main hardware risk: the NONBASETHRDENA dispatch mechanism of design decision DD-01. Until the Generator arrives in Phase 2, Phase 1 runs the Kernel on hand-written `Config` tables. Phase 2 adds the task-model DSL, the independent checkers, the Analyzer, and the UPPAAL export. Phase 3 adds measurement-based WCET on the HIL_Rig. Phase 4 adds session-typed Endpoints. Phase 5 adds the Ferrocene build, static WCET, the Cortex-M33 and RISC-V ports, and the Jorvik-style variant.

Every leaf task names its evidence. Proofs come first wherever a tool applies; tests are labelled as testing evidence. "Property N" refers to the Correctness Properties in design.md, and DD-nn to its design decisions. Each phase ends with a checkpoint.

## Tasks

### Phase 1 — Profile specification, verified Kernel, Kani harnesses

- [ ] 1. Workspace, build pipeline, and evidence store
  - [x] 1.1 Create the Cargo workspace with the crates of the design (`rsk-kernel` with `logic`, `hw_model`, `arch`, and `unsafe_module`; `rsk-entry`; `rsk`; `rsk-model`; `rsk-gen`; `rsk-macros`; `rsk-link`; `rsk-confcheck`; `rsk-analyzer`; `rsk-wcet`; `rsk-hil`). Pin the upstream stable rustc on which the planned Ferrocene release is based, vendor all dependencies, and add Build_System checks for `#![forbid(unsafe_code)]` placement, the absence of `#![feature]` and `RUSTC_BOOTSTRAP`, lock-file-only resolution, and offline build scripts and proc-macros.
    - Evidence: CI verification job building every crate for `thumbv7em-none-eabihf` and the host; negative fixtures (a crate without `forbid`, a `#![feature]` attribute, an unlocked dependency) that the job rejects
    - _Requirements: 6.1, 6.3, 55.1, 55.5, 58.1, 58.4_
  - [x] 1.2 Add the Kernel line-count gate (PAR-01) and the gate that requires a linked justification identifier on every `unsafe` block, `unsafe fn`, and `unsafe impl`.
    - Evidence: gate tests with passing and failing fixtures; CI gate output
    - Done 2026-10-08: `xtask/src/checks/kernel_size.rs` (token-based counter; Verus `line_count` cross-check hook) and `xtask/src/checks/justifications.rs` (`UJ-nnn` markers, `unsafe_justifications.toml` register, links to `#[kani::proof]` harnesses, `verus!` functions, or reviews); fixtures `kernel-too-large` and `unsafe-unjustified`; the PR-36 check was refined to the crate-root rule (rustc ignores `#![feature]` elsewhere).
    - _Requirements: 6.2, 6.4_
  - [x] 1.3 Implement Build_Manifest generation (source revision, lock file, toolchain versions, Profile version, Generated_Config checksum, binary hash over loadable sections), the Evidence_Item format, the clean-checkout rebuild comparison, and the dependency report that gates new or updated crates on Trust_Base_Register entries.
    - Evidence: schema tests; a rebuild with identical hashes; a fixture dependency update rejected until its entry exists
    - Done 2026-10-08: `xtask/src/manifest.rs` (Build_Manifest `rsk-build-manifest/1`, binary hash over loadable segments via `rsk-elf`), `xtask/src/evidence.rs` (`rsk-evidence/1`, one item per job step), `xtask/src/checks/deps.rs` (dependency report; gate on `docs/trust_base_register.toml` `[[crate]]` entries), `xtask/src/rebuild.rs` (`cargo xtask rebuild-check`, git-archive or tree copy, `--remap-path-prefix` in Target builds). Open: the rebuild comparison has no Flight_Build binary to compare until task 5.1 adds the entry crate's binary; the evidence store is `target/xtask-verify/evidence`.
    - _Requirements: 33.3, 50.3, 58.2, 58.5, 59.1, 59.2, 59.3_

- [ ] 2. Profile specification (Component A)
  - [x] 2.1 Write the Profile as a machine-readable source: restrictions PR-01 to PR-36 with subjects and crate coverage, Evidence_Item kinds per enforcement category, the Table 4-1 correspondence with Jorvik markers (including the resolved equal-priority order of DD-03), the disallowed-crate list, the version identifier, and the change log. Generate `docs/profile.md` and the `PROFILE_VERSION` constant from it.
    - Evidence: CI checker enforcing R1.1–R1.6 and the two-way consistency of Table 2-1 and Table 4-1; review record
    - Done 2026-10-08: `profile/profile.toml` (version 0.3; PR-01..36 with subjects, crate coverage, evidence kinds, RT assumptions; Table 4-1 with the DD-03 order per R4.6; reserved PR-37..39 per R27.3; disallowed crates; change log with content hashes), `xtask/src/profile_gen.rs` (`cargo xtask profile [--check]`, run by the verification job), generated `docs/profile.md` and the `profile_version.rs` modules of rsk-kernel and rsk-gen.
    - _Requirements: 1.1, 1.2, 1.3, 1.4, 1.5, 1.6, 2.1, 2.2, 2.3, 4.1, 4.2, 4.3, 4.4, 4.5, 4.6, 4.7_
  - [x] 2.2 Derive the initial Core_Subset allow-list from Ferrocene's published certified `core` subset, and record for each item its certification evidence or the substitute rsk evidence.
    - Evidence: allow-list file with sources; Gap_Register entry for the DO-178C gap
    - Done 2026-10-08: `profile/core_subset.toml` (23 items with scope and per-item certification evidence or substitute rsk evidence; Ferrocene's per-item list is customer-only, recorded as assumed/unconfirmed, ORQ-24); checked and rendered by `cargo xtask profile`; Gap_Register entry GAP-07 (task 8.2).
    - Changed 2026-10-22 (Verification_Plan 5.9): the four `certification = "none"` items that no Kernel crate used (`NonZeroU8`, `core::array::{from_fn, map}`, the atomics, `spin_loop`) were removed; 19 items remain, two of them uncertified and in use (`core::arch::asm`, `PanicInfo`).
    - _Requirements: 57.1, 57.4, 57.5_

- [ ] 3. Hardware risk retirement and Hardware_Model
  - [x] 3.1 Build the dispatch spike for DD-01: three dispatch vectors at different levels; stubs that enter unprivileged Thread mode with the vector active; SVC lock and unlock through BASEPRI; SVC job-end with the two-step return; MPU view switching between two Partitions; eager FP stacking; a DSB before every exception return. Record the go or fallback decision (ORQ-01 option b) in the Design_Document.
    - Evidence: HIL report on preemption order, preservation of core and FP register patterns, faults on unprivileged PPB access, and BASEPRI ceilings that mask only Task levels; decision record
    - Done 2026-10-08 on QEMU (`cargo xtask qemu`, `apps/p1-demo`: 3 Tasks at levels 7/5/2 in 2 Partitions, SVC lock/unlock with a Ceiling-7 Resource held across a control Release, MPU view switching, eager FP stacking, DSB before every return, operating-duration end into the safe state; `RSK-OK`, 20/10/5 Jobs, 0 faults, 0 misses). **Decision: DD-01 revised, not the option (b) fallback.** The spike found that an exception return always deactivates the returning vector, so "Thread mode with the vector active" cannot exist; the executing Job's level is now enforced by BASEPRI and Job end unwinds the level record (design.md "Running Jobs unprivileged", ORQ-01, ORQ-32). Also found: a cross-section `cbnz` assembles to `nop` without a diagnostic (ORQ-32), and the Partition stack floor must follow the preempted Job's live PSP. The verification job runs the spike on QEMU as step 10b (Evidence_Item `test-qemu-spike-p1-demo`). Open for hardware (3.3/7.x): register-pattern preservation, faults on unprivileged PPB access, Budget accounting (QEMU has no DWT cycle counter).
    - _Requirements: 7.7, 11.1, 16.9, 20.1, 20.2, 20.5_
  - [x] 3.2 Write the Hardware_Model in Verus specification code for every behaviour listed under "Kernel proofs" in design.md, cite the defining manual section of each rule, and list the behaviours left out with the reason no proof depends on them.
    - Evidence: `hw_model` verifies under Verus; citation table; review record
    - Done 2026-10-08: `crates/rsk-logic/src/hw_model.rs` (rules HW-01 to HW-14 with manual sections; lemmas `lemma_nvic_takes_iff_eligible` tying R7.1 to NVIC arbitration under BASEPRI, `lemma_mpu_grant_covered` tying the hardware MPU decision to the view over-approximation; errata and not-formalized list). Open: HIL differential tests (3.3) and the hardware confirmation of 838869 applicability and the TIMER near-counter rule (3.1).
    - Updated 2026-10-11 for the revised DD-01: `basepri_for` (BASEPRI from the higher of the executing Job's Priority and the System_Ceiling) and HW-05 arbitration among pending exceptions (`nvic_selects`: most urgent group priority, then lowest exception number) are now formalized; `encode_level(0)` is 0x00 as in DD-02 and `arch::cm4::encode_level` (it was 256), and `lemma_encoding_order` is stated for the Task levels 1..=7. `lemma_nvic_takes_iff_eligible` is kept as the original-DD-01 form (in-progress vectors active); Property 1's hardware side is in task 4.2.
    - _Requirements: 49.1, 49.2, 49.5, 49.6_
  - [ ] 3.3 Implement one HIL differential test per Hardware_Model rule and record each mismatch with its disposition.
    - Evidence: HIL differential-test report per rule
    - _Requirements: 49.3_
  - [ ] 3.4 Review the Cortex-M4 r0p1 and nRF52840 errata and record applicability, workaround, and test for each one in the Hardware_Model.
    - Evidence: errata table; a test for each applied workaround
    - _Requirements: 11.6, 49.4_

- [ ] 4. Verified Kernel logic (`rsk-kernel::logic`, Verus)
  - [x] 4.1 Define the core types, the `Config` tables, `KernelState`, `Action`, and the invariant `inv`, with hand-written test configurations. Run Verus in CI with the pinned Z3.
    - Evidence: Verus run in CI; Evidence_Item recording the Verus and Z3 versions
    - Done 2026-10-08: `crates/rsk-logic/src/logic/{mod,sched,time,budget,log,mpu,kernel}.rs` inside `verus!`, depending on the vendored `vstd` (no_std); `cargo xtask verify` step 9 runs `cargo-verus verify` with the Verus 0.2026.10.04 release (rustc 1.98.1, Z3 4.16.0) and writes `proof-kernel-verus` with the versions; the assumption gate (R50.2) reads `docs/proof_assumptions.toml`. Result at this revision: 198 items verified, 0 errors, 0 assumption constructs; 921 executable Kernel lines (PAR-01). Current (2026-10-11, after the Hardware_Model compositions of tasks 4.2, 4.7, and 4.9, the entry-point specifications of tasks 4.2 to 4.4 and 4.7, R14.9 in task 4.4, and the Property 8 state parts in task 4.6): 230 items verified, 0 errors, 0 assumption constructs; 2510 executable Kernel lines. Verus counts functions, so strengthened postconditions leave the count unchanged.
    - _Requirements: 6.5, 44.6_
  - [x] 4.2 Implement and prove the dispatch, completion, LIFO-resumption, and idle transitions.
    - Evidence: Verus proofs of Properties 1 and 2 (logic part)
    - Done: `Sched::select`, `start`, `complete` (sched.rs) with `inv`; DD-03 tie order.
    - Done 2026-10-11, the dispatch stub entry and exit carry these specifications: `Kernel::dispatch` ensures `Sched::start`'s postcondition, `Kernel::job_return` ensures `ended_top` (`Sched::finish_top`, now stated with `end_job`'s full postcondition) and, for a Job holding nothing, an unchanged System_Ceiling (R7.5); `Kernel::select` adds that no Job is selected only in the safe state, before instant 0, or when none is eligible; `Kernel::start` makes no Release and arms every Periodic_Task's first release slot at its nominal instant and the end-of-operation slot (PR-34, R9.1, R9.9).
    - Done 2026-10-11, Property 1 hardware side under the revised DD-01 (`hw_model.rs`): `lemma_dispatch_takes_iff_eligible` (the NVIC takes a pending Task's vector exactly when `Sched::eligible` holds; R7.1, R20.3), `lemma_dispatch_selects_srp_choice` (once the selected Job's vector is pending, NVIC arbitration takes it and no other; R7.2, R7.3, DD-03), `lemma_dispatch_idle_when_none_eligible`; `Kernel::select` now ensures `srp_selection`, so the vector that Kernel_Arch pends is the one the lemmas cover. The vector-order assumption is load-bearing (reversing it fails the tie step) and is checked for every Target by the rsk-gen test `dispatch_irqs_ascend_for_dd03`.
    - Open: the lemmas' Kernel_Arch preconditions hold by inspection of `arch::cm4`, not by proof (arch is outside `verus!`): BASEPRI = `basepri_for` after every transition, PRIMASK clear in Thread mode, vector priorities from `encode_level` at boot, and no stale pending bit (the entry check rejects one and the NVIC re-arbitrates, a trace argument). Property 2 (context preservation) is not covered.
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 20.3, 44.1_
  - [x] 4.3 Implement and prove the lock and unlock services with the lock stack, including the check against the calling Task's declared Resources.
    - Evidence: Verus proofs of Properties 3, 4, and 5
    - Done: `Sched::lock`/`unlock` (ceiling mirror, LIFO, re-entry refusal, accessor check), `lemma_no_other_holder` (mutual exclusion), `lemma_blocked_by_one_holder` (state part of bounded blocking; the trace argument is recorded in the Verification_Plan).
    - Done 2026-10-11: `Kernel::svc_lock`/`svc_unlock` (the SVC entry points) carry `Sched::lock`/`unlock`'s postconditions (one push or pop, ceiling raise and restore, R8.3 free-on-entry), and `Sched::lock` now also states that the entries below a push are untouched and that a refused lock changes nothing.
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 44.1_
  - [x] 4.4 Implement and prove forced Job ending and the closed response sets of DD-10, including END_JOB escalation and Partition isolation for every response.
    - Evidence: Verus proofs of Property 6 and of R14.9 for each response
    - Done: `Sched::end_job`/`kill`/`finish_top` (Property 6; lazy release for preempted Jobs documented), `Kernel::apply_response` with END_JOB escalation (R18.4), RESTART/STOP/SAFE_STATE (kernel.rs). Open: the R14.10 bound (measured in P3). R14.9, open at this point, is done below (2026-10-11).
    - Done 2026-10-11, response effects on the faulting Partition (R14.4, R14.5): `Kernel::fault` ensures the safe state for a Kernel fault (no Release left, Jobs and lock stack untouched) and otherwise `response_applied` for the Partition's fault response: RECORD_* changes nothing; END_JOB on a Job holding nothing is `ended_top` (`Sched::end_job` lifted); escalated END_JOB and RESTART_PARTITION leave the Partition `partition_ended` (no Release pending or deferred, no Job running: each one ended or killed) with re-initialization and EasyDMA stop requested and its MIT count reset; STOP_PARTITION the same with `stopped`; SAFE_STATE leaves no Release. Still open: the lazy release of killed Jobs (trace argument), and the per-Resource reporting of Property 6 (one event with a count).
    - Changed 2026-10-11: `Kernel::job_return` logs the two causes of a Job ending with Resources still held apart: a killed Job's forced release as `EV_RESOURCE_RELEASED`, a Job that returned to its Release_Point holding a Resource (a PR-07 breach the type system should reject) as `EV_API_MISUSE`; the Resources are released in both cases. Before, both branches logged `EV_RESOURCE_RELEASED`, and the branch condition (`top_holds_nothing`) could never select the other one, since a Job holding nothing releases nothing. The event kind is not part of any postcondition (the log's contents are not specified).
    - Done 2026-10-11, R14.9 for each response: `response_applied` includes `others_unchanged` for RECORD_*, END_JOB (relative to the ended Job's Partition), RESTART_PARTITION and its escalation, and STOP_PARTITION, proven through `end_top`, `end_partition_jobs`, `restart_partition`, and `stop_partition`; SAFE_STATE is system-wide by design (R14.4). "Unchanged" is per Partition because the System_Ceiling is global: every other Partition's Release and Job states, Budget accounts, timing state (nominal instants, MIT history, deadline references, masks), release and deadline events, stack positions, held lock entries, and Partition flags are unchanged; its Budget event is unchanged except for the Job that resumes when the affected Partition's executing Job ends (re-armed for its remaining Budget); the System_Ceiling does not rise and still covers every Ceiling another Partition imposes (`lemma_others_ceilings_in_force`). Endpoint roles (R14.9's fourth item) do not exist until Phase 4 (task 21.2). Mutation checks: killing other Partitions' Jobs, or resetting a neighbouring Task's Budget account, fails the proof.
    - Open: `timer_event` and `intake` apply responses (deadline miss, Overrun, MIT_Violation threshold) through `apply_response`, so R14.9 holds at each application; `intake` and `periodic_release` now state the MIT_Violation case through `violation_applied`, which includes `response_applied` (task 4.6), while `timer_event`'s own postcondition does not restate it (Property 7). An MIT_Violation's response is chosen by the source Partition while END_JOB ends the released Task's Job; the Generator's intake bindings make the two Partitions equal and Release_Signal raise is not implemented yet, so this cannot differ today, but the logic does not require it.
    - _Requirements: 8.9, 14.5, 14.8, 14.9, 14.10, 18.4_
  - [x] 4.5 Implement and prove the 64-bit time base, the timed-event slot table, periodic release, deadline detection, release overlap and discard, and the operating-duration limit.
    - Evidence: Verus proofs of Property 7 and the slot-table part of Property 11
    - Done: `time.rs` (`Slots` refining a map, `Timing::advance_periodic` with O + k·T and the operating-duration limit, `lemma_nominal_is_drift_free`), `Kernel::timer_event`/`periodic_release`/`deadline_miss` (R9.6 to R9.9).
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 44.2, 44.8_
  - [x] 4.6 Implement and prove sporadic release: intake decisions, MIT state, deferral and discard, Release_Signal raise, MIT_Violation attribution and thresholds, and interrupt masking during MIT windows.
    - Evidence: Verus proofs of Properties 8 and 9
    - Done: `Timing::intake_decision`/`accept`/`defer`/`try_unmask`, `Kernel::intake` with attribution and the MIT_Violation threshold (R10.4).
    - Done 2026-10-11, the state parts of Property 8 (kernel.rs): `Kernel::inv` now requires that a deferred Release belongs to a Sporadic_Task under the deferral policy, that a previous accepted Release exists, and that `deferred_until == last_due + MIT`. `Kernel::intake` states every outcome: inactive (safe state, before instant 0, stopped Partition, or R9.9 horizon) changes nothing but the log; R10.1 `accepted` (Release pending and due now, previous due instant moved to now, source masked, deadline armed, no violation); R10.2 `deferred` (deferred until `last_due + MIT`, deferral event armed); discard under R10.2 or R10.3 (timing unchanged). Both violation cases end in `violation_applied` against the source Partition: within the threshold the count rises by one (saturating) and nothing else changes, so the existing Release is unchanged; past it the Partition's fault response is applied to the counted state (R10.4). `periodic_release` states R9.8 the same way; `deferred_release` states the deferred Release becoming due exactly one MIT after the previous one (or dropped under R9.9). Every Kernel_Arch entry point ensures `mit_spaced`. Mutation checks: clearing a pending Release on discard, making a deferred Release due at the current instant, and deferring a periodic Release each fail the proof. 230 items verified.
    - Open: the statement over a whole event sequence ("no two accepted Releases less than one MIT apart") is by induction from `mit_spaced` and the per-entry-point facts, not a theorem over a trace; the log entry of each violation is not stated (the counted state ignores the log); Release_Signal raise (R10.5) is not implemented; Property 9 (masking bound) is not covered. The Kani cross-check harness on `intake` exists since 2026-10-22 (task 6.2).
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.9, 19.1_
  - [x] 4.7 Implement and prove Budget accounting and Overrun detection.
    - Evidence: Verus proof of Property 10
    - Done: `budget.rs` (attribution to one Task), `Kernel::budget_event`; R18.5/R18.6 in `Config::partition_ok`; detection "whatever the ceiling" is the DD-02 level argument (hw_model HW-02).
    - Done 2026-10-11, attribution at the entry points (R18.1): `Kernel::dispatch` charges the preempted Job alone and starts the new Job from zero, `Kernel::job_return` charges the ended Job alone; both state the Budget event they arm or disarm. `budget_event` itself still ensures only `inv` (it runs inside `timer_event`).
    - Done 2026-10-11, the Kernel level is never masked (R18.2, `hw_model.rs`): `lemma_kernel_level_never_masked` (a Kernel-level exception is taken whatever BASEPRI holds and whatever Task-level exceptions are active), `lemma_kernel_level_taken_in_every_state` (so in every state satisfying `Sched::inv`, whatever the System_Ceiling and the executing Job), `lemma_kernel_level_wins_arbitration` (it is taken ahead of every Task vector), `lemma_job_cannot_set_mask` (HW-04 with HW-08: CPS and MSR change PRIMASK and FAULTMASK only privileged). The proof depends on `encode_level(0)` being 0x00 (the old value 256 fails it). 218 items verified, 0 errors.
    - Open: the reporting bound Δ_detect is a timing property (HIL, R18.8); the lemmas' Kernel_Arch preconditions hold by inspection, not by proof: every Kernel exception is programmed at priority 0 (`set_kernel_exception_priorities`, `encode_level(0)` for intake and Kernel lines), PRIMASK is set only by Kernel code (the dispatch and unwind stubs and `arch::cm4::start`) and cleared before every return to a Job, nothing sets FAULTMASK, and Jobs run with CONTROL.nPRIV = 1 (`irq_entry` installs CONTROL = 0b001).
    - _Requirements: 18.1, 18.2, 18.3, 18.5, 18.6, 44.3_
  - [x] 4.8 Implement and prove the event-log ring with PR-identifier sets and the saturating overflow counter.
    - Evidence: Verus refinement proof
    - Done: `log.rs` (`Log::push` refines `Seq` with drop-oldest and a saturating overflow counter).
    - _Requirements: 14.6, 14.7, 44.2_
  - [x] 4.9 Implement and prove the MPU view computation for a Partition layout, including stack placement for overflow detection.
    - Evidence: Verus proof of Property 12
    - Done: `mpu.rs` (`Layout::view` and `lemma_view_within_layout`, `lemma_views_disjoint`, `lemma_stack_overflow_faults`; Armv7-M size and alignment rules checked by `Interval::check_rules`).
    - Done 2026-10-11, Property 12 over the Hardware_Model (`hw_model.rs`): `lemma_hw_partition_isolation` composes `lemma_mpu_grant_covered` (HW-09 decision) with the view lemmas, so for any layout passing `Layout::validate` the MPU itself grants Partition p only its own code, RAM, windows, and the shared code (writes only RAM and windows, fetches only code), nothing in Kernel flash or RAM, nothing another Partition owns (code, RAM, or windows, including read-only code, which `lemma_views_disjoint` did not exclude), and refuses the write below p's stack; every enabled region satisfies the Armv7-M rules. `lemma_hw_write_isolation` is the pairwise form. 220 items verified, 0 errors.
    - Open, by inspection rather than proof: Kernel_Arch loads exactly these views and enables the MPU; the RBAR/RASR encoding (Kani field-placement harnesses only); Kernel-owned peripherals and the PPB are not intervals of the layout, so they are excluded only while no Partition window covers them (Generator, PR-29; Config_Checker, R16.7, task 13.1).
    - _Requirements: 16.1, 16.2, 16.3, 16.5, 16.6, 44.4_
  - [x] 4.10 Complete the absence-of-runtime-error proofs over `logic`, confirm that only Verus-supported features appear outside trusted functions, and document the panic-freedom argument under the abort strategy.
    - Evidence: Verus run with no unlisted assumptions; Verification_Plan entry for ORQ-07
    - Done: every exec function in `logic` is verified for overflow, bounds, and panic freedom (Verus checks them for all exec code); no `assume`, `admit`, or `external_body` in the Kernel crates (assumption gate); the ORQ-07 entry is in the Verification_Plan (task 8.1).
    - _Requirements: 44.1, 44.5, 44.7_
  - [x] 4.11 Add a ghost history to the Kernel model: a `Ghost<Seq<Step>>` trace of the transitions (dispatch, Job end, lock, unlock, release due, kill) with the instant and Task of each, carried by `Kernel` and extended by every entry point, so that the arguments of Verification_Plan section 5 can be stated as theorems over a run rather than per transition. Keep the trace ghost-only (no executable cost, PAR-01 unchanged) and the existing postconditions intact.
    - Evidence: Verus run with the trace invariant proven at every entry point; the Kani cross-check harnesses unchanged and passing (the trace is erased)
    - Added 2026-10-22: prerequisite of 4.12 to 4.14, which mechanize arguments 5.1 to 5.5 of the Verification_Plan (review evidence today, no DO-333 credit until mechanized).
    - Done 2026-10-22: `crates/rsk-logic/src/logic/history.rs` (`Step` with kind, Task, observed instant, and one argument; the six kinds ST_DISPATCH, ST_END, ST_LOCK, ST_UNLOCK, ST_RELEASE, ST_KILL; `history_wf`, the trace invariant: Task and Resource indices of the configuration, instants nondecreasing and never beyond `Kernel::now`; `extends`). `Kernel` carries `history: Ghost<Seq<Step>>`; `inv` carries `history_wf`; every mutating Kernel function ensures `history_extends(old)` and every loop keeps it; `record` appends one step with ghost arguments only, called from `dispatch`, `job_return` (arg 1 for a killed Job), `svc_lock`/`svc_unlock` on success, the three Release paths (`intake` accept, `periodic_release`, `deferred_release`; arg = due instant), `end_top` (forced end, arg 1), and the kill loop of `end_partition_jobs`. Evidence: rsk-logic 241 items verified, rsk-kernel 112; the 12 Kani harnesses unchanged and passing; mutations `history-step-after-now` and `history-step-dropped` killed; the p1-demo Flight_Build has section sizes identical to the build before the change (`.kernel.text` 29,092 bytes both), so the history emits no code. PAR-01 counts 34 more lines (the ghost assignments and `record` calls, which the counter's conservative rule classes as executable; the identical binary shows they are not). A first draft computed the step's Task with exec reads and grew `.kernel.text` by 120 bytes (dead index loads keep their bounds checks); the `Ghost<int>` arguments removed that.
    - _Requirements: 44.1, 52.1, 52.5_
  - [x] 4.12 Prove bounded blocking over the history (Verification_Plan argument 5.1, Property 5): a Job released at r and pending but not eligible only because of the System_Ceiling waits for at most one outermost Critical_Section (with its nested sections) of one lower-Priority Job that started before r.
    - Evidence: Verus theorem over the trace of 4.11; mutation fixture in `docs/proof_mutations.toml` showing a second blocking section failing it
    - Done 2026-10-23: `Step` gains `top` (the executing Task after the step, -1 for none) and `history_wf` the relation `top_rel` (a dispatch sets the top to its Task and requires it eligible after the previous step: above the recorded ceiling and top; a lock or unlock is by the top and keeps it; a Release or kill mark keeps it; a Job end leaves it free); `Kernel::inv` adds `top_match`; `record` requires `top_rel`, supplied by `lemma_top_kept` at the stack-preserving sites and by `lemma_ceiling_recorded` plus `Kernel::dispatch`'s `eligible` at the dispatch site, which now records before `arm_budget`; the kill step moved into `kill_at` so the kill loop's query stays within the resource limit; the reveals in `lemma_prefix_kept` and `lemma_ceiling_restored` were localized for the same reason. Theorems: `history::lemma_no_lower_dispatch_while_waiting` (step 1), `Sched::lemma_one_low_blocker` (step 3), `history::lemma_no_new_blocking_section` and `Sched::lemma_low_locker_nested` (step 4), with `lemma_blocked_by_one_holder` (step 2); the premise "J never eligible before its dispatch" is the A4 promptness of Kernel_Arch and stays the caller's. Mutations `lock-by-other-than-top` and `dispatch-below-ceiling` killed; `history-step-after-now` and `history-step-dropped` re-anchored (22 of 22). rsk-logic 264 items verified; Kani unchanged; the p1-demo Flight_Build keeps section sizes identical to the pre-4.11 build. Row P-05 moved to proof.
  - [x] 4.13 Prove ceiling restore across preemptions and lazy release over the history (arguments 5.3 and 5.2, Properties 3 and 6): after a Job's unlock the System_Ceiling equals its value before the matching lock whatever preemptions occurred between them, and the lock entries a killed Job retains until the unwind delay no Job that the Jobs above it would not delay.
    - Evidence: Verus theorems over the trace of 4.11; mutation fixtures
    - Done 2026-10-22: every `Step` now carries a snapshot of the lock stack (`locks`) and the System_Ceiling (`ceiling`) after it; `history_wf` (opaque, revealed only in the history lemmas, which keeps the entry points' queries small) adds `step_rel` between consecutive snapshots and `ceiling == ceiling_of(locks)`; `Kernel::inv` adds `locks_match` (the last snapshot is the scheduler's lock stack), re-established in `record` from the `Sched` postconditions through `lemma_rel_shrink`, `lemma_rel_lock`, and `lemma_rel_unlock`; `record` moved after `finish_top` and `end_job` so the snapshot is the post-state, and `end_top` split into `end_top_release` and the resume half so each proof fits the resource limit. Theorems: `history::lemma_ceiling_restored` (argument 5.3: after the matching unlock the stack and the System_Ceiling are those before the lock, whatever happened in between; via `lemma_prefix_kept`) and `Sched::lemma_lazy_release_delays_nothing` (argument 5.2: a pending Task blocked only by a killed Job's retained entries has Priority below every Job above it, so is not eligible anyway; via `lemma_max_ceiling_attained`). Mutations `history-end-forgets-prefix` and `lazy-release-no-job-above` killed (18 of 18). rsk-logic 252 items verified; Kani unchanged; the p1-demo Flight_Build keeps section sizes identical to the build before 4.11. Not mechanized: that the unwind releases the entries in the same Kernel entry (Kernel_Arch, section 4) and the stub order UJ-023.
    - _Requirements: 8.2, 8.9, 14.5, 44.1_
  - [x] 4.14 Prove MIT spacing over a run and deadlock freedom over the history (arguments 5.4 and 5.5, Property 8 and R8.5): no two accepted Releases of a Sporadic_Task become due less than one MIT apart in any run, by induction over the trace from `mit_spaced`; and no wait-for cycle exists because a lock never waits and a pending Job waits only for higher-Priority Jobs to end.
    - Evidence: Verus theorems over the trace of 4.11; the Verification_Plan rows P-05, P-06, P-08 move from "argument" to "proof"
    - Done 2026-10-22: `history_wf` adds `releases_spaced` (two Release steps of one Sporadic_Task are due at least one MIT apart) and `lemma_history_push` requires `release_spaced_after`; `Kernel::inv` adds `releases_linked`, a function of the history and the `has_last`/`last_due` arrays only and opaque, so that it carries over every other state change without a proof step (the first draft quantified over `self` and failed in 20 functions for want of a trigger); `record` requires the link for every Task the step does not release and, for a Release of a Sporadic_Task, that `last_due` now equals its due instant, one MIT after every Release step of that Task so far; the call sites supply it with `lemma_release_match_kept` (timing of the other Tasks unchanged) and `lemma_release_spaced` (from `Timing::accept`'s rule or the deferral clause); `push_event`, `observe`, and `arm_budget` now ensure the history exactly unchanged. Theorems: `history::lemma_mit_spacing_over_run` (argument 5.4: for Release steps i < j of a Sporadic_Task, `h[j].arg >= h[i].arg + MIT`) and `Sched::lemma_deadlock_free` (argument 5.5: a lock never waits; every pending non-eligible Job waits for its own Job in progress or a `blocked_by` blocker; a blocker is in progress and blocked by nothing, so every wait-for path has length one). Mutations `mit-run-spacing-dropped` and `deadlock-equal-priority-unblocked` killed, and `restart-kills-one-job`/`kill-other-partitions` re-anchored (20 of 20). rsk-logic 257 items verified; Kani unchanged; the p1-demo Flight_Build keeps section sizes identical to the pre-4.11 build. Rows P-06 and P-08 moved to proof; P-05 (bounded blocking) stays with task 4.12.
    - _Requirements: 8.5, 10.1, 10.2, 44.1_

- [ ] 5. Kernel_Arch for the Cortex-M4F and the `rsk` API
  - [x] 5.1 Implement `arch::cm4` and the Kernel_Unsafe_Module from the spike: the priority map and PRIGROUP, NONBASETHRDENA, dispatch stubs with per-level records, SVC entry and exit, the two-step return, MPU loads, BASEPRI writes, eager FPCCR and FPDSCR, ACTLR.DISDEFWBUF, and DSBs before exception returns. Implement `rsk-entry` with the vector table and reset entry. Give each `unsafe` block, `unsafe fn`, and `unsafe impl` a justification identifier, and write in this task its Kani harness or Verus proof or lemma where one is feasible, and otherwise its recorded review stating why none is feasible.
    - Evidence: Kani harnesses for this task's justification identifiers, passing in the verification job that task 6.1 sets up; justification gate output (task 1.2); HIL dispatch tests carried over from 3.1
    - Done 2026-10-08: `crates/rsk-kernel/src/arch/{mod.rs,cm4/mod.rs}` (safe planning code: priority map, dispatch with per-level records and Partition stack floors, SVC services, unwind, timer/intake/fault handlers, boot) and `cm4/unsafe_module/{hw.rs,stubs.rs,mod.rs,kani.rs}` (register access, naked stubs, cells, frame builder; UJ-001..054 in `unsafe_justifications.toml`, each linked to a Kani harness or a review record, gate R6.4 passing); `crates/rsk-entry` (vector table, reset entry, panic handler, UJ-101..108). NONBASETHRDENA is no longer used (DD-01 revised). HIL-only items remain under 3.3/7.x.
    - Changed 2026-10-22 (GAP-10, first step): the bound Kernel moved into `verus!` as `arch/bound.rs` (`Bound`, `KernelOps` with a specification on every method, the configuration CRC, and `encode_level`/`basepri_for_ceiling`/`basepri_for` with postconditions equal to the Hardware_Model encodings). Verus now checks that every call from `Bound` into the logic satisfies its `requires` (Verification_Plan fact A9), that `init` establishes the invariant with `is_view_of` for every Partition view (A6) and every method keeps it, that `eligible` decides exactly `Sched::eligible` and `select` names an eligible Job (A4), and that `basepri_now` is `hw_model::basepri_for(job_level, ceiling)` (A1). 286 items verified, 0 errors. Two pre-existing proofs (`Slots::new`, `Layout::view`) that relied on the array-fill axiom's trigger became unstable in the larger context and now apply the axiom explicitly; `end_top` carries `rlimit(40)`. `arch::cm4` (register writes, frames, level records) stays outside `verus!` (Verification_Plan section 4).
    - _Requirements: 6.3, 6.4, 6.6, 6.7, 7.7, 7.8, 11.1, 11.2, 11.3, 11.7, 11.8, 45.1_
  - [x] 5.2 Implement boot validation (target values, clocks, configuration checksum, DWT, debug-state logging) and the system safe state.
    - Evidence: host tests of the boot logic; HIL boot-check suite (task 7.2)
    - Partly done 2026-10-08: `arch::cm4::boot` validates `CONFIG` (`Config::validate`, proven `ok == wf`), runs `hw::boot_checks` (priority bits, PRIGROUP, MPU presence/region count, FPU, DWT) and logs the failed-check mask before the safe state (R12.1, R12.2); the Partition layout is validated by `logic::mpu::Layout::validate`. Open: clock checks and the configuration checksum (need the Generator's `rsk-confcheck` tables, task 13.x), DHCSR.C_DEBUGEN logging (ORQ-30).
    - Done 2026-10-23: `hw::boot_checks` takes a `BootExpect` (priority bits, the CPUID word, the HFCLK source, whether any Task uses the FPU; emitted by the Generator as the `System` constants `CPUID`, `HFCLK_XTAL`, `FPU_NEEDED`, `TICK_HZ` from the Target table, which gained `cpuid` and `hfclk_xtal`) and checks, each with its own bit: CPUID implementer, part, and full word (ASM-01), MPU regions (ASM-02), priority bits and the PRIGROUP reset value (ASM-03), the instruction cache disabled (ASM-04, NVMC.ICACHECNF), the HFCLK running from the expected source (ASM-05, CLOCK.HFCLKSTAT), the FPU present when needed (MVFR0), the DWT (ASM-06); the configuration checksum (bit 5) was already compared in `boot_core` against `System::CONFIG_CHECKSUM`. `hw::boot_info` (the fifth trusted function, docs/proof_assumptions.toml) reads DHCSR.C_DEBUGEN and RESETREAS.DOG; `boot_core` logs them through `KernelOps::boot_info` (`Kernel::boot_info`: events `EV_DEBUGGER`, new kind 23, and `EV_WATCHDOG_RESET`) before the checks' verdict. The clock and cache registers exist only on the nRF52840 (`cfg(not(feature = "board-qemu-mps2"))`); QEMU exercises the rest. Host tests: `boot_checks_pass_on_the_expected_board_and_fail_one_bit_per_check`, `boot_info_reports_the_debugger_and_a_watchdog_reset_and_clears_the_reason`. Not checked: the low-frequency clock (ASM-16 does not apply: the time base is TIMER0 at 16 MHz) and the FP context-preservation settings of R11.2, which the Kernel writes itself after the checks (`Action::FpuEager`); the system safe state is the generated `System::safe_state`, entered by every shell when a core says so.
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 45.1_
  - [x] 5.3 Implement the fault handlers with attribution, the panic handler, HardFault handling, and the Kernel-owned watchdog.
    - Evidence: HIL fault-injection case per fault class; Kani for the handler glue
    - Partly done 2026-10-08: MemManage/BusFault/UsageFault entries with Kernel-vs-Partition attribution by EXC_RETURN bit 3 and PRIMASK (R14.3), the HardFault and NMI paths into the safe state (R14.4), the single panic handler with the unprivileged panic SVC (R14.1, R14.2, PR-19), and the response application (`Kernel::fault`/`apply_response`, proven). Open: the Kernel-owned watchdog (ORQ-31, nRF52840 WDT; no QEMU model) and the HIL fault-injection cases (7.5).
    - Done 2026-10-23: the Kernel-owned watchdog. `Action::WatchdogStart(reload)` and `Action::WatchdogFeed` drive the nRF52840 WDT (`hw::watchdog_start`: CRV, one reload register, run in sleep, pause under a debugger, start; `hw::watchdog_feed`: the reload request on RR[0]); the ghost `HwModel` gains `watchdog_running` and `watchdog_fed`. Proven: `boot_core` starts the watchdog before the time base (`booted` requires `watchdog_running`), and `timer_core` feeds it as its last action (`ensures watchdog_fed`), so a handler that does not complete does not feed. The reload is `watchdog_reload(tick_hz)`: four housekeeping periods in 32.768 kHz ticks (16,384 at 16 MHz; 10,485 on QEMU), saturated. A watchdog reset is logged at the next boot and is boot-check bit 7, so the system enters the safe state (design.md, ORQ-31). QEMU has no WDT: the writes go to unmapped space and the reset path cannot be exercised (TBR-23); `hil-watchdog` (section 11) stays hardware-pending. Host tests: `watchdog_start_and_feed_write_the_wdt_registers`, `watchdog_reload_covers_four_housekeeping_periods`. The fault handlers are verified cores since task 5.7 (`fault_core`); the HIL fault-injection cases stay with 7.5.
    - _Requirements: 11.5, 14.1, 14.2, 14.3, 14.4, 16.4, 45.1, 45.2_
  - [x] 5.4 Implement the peripheral intake handlers, source masking during MIT windows, and disabling of unbound sources.
    - Evidence: HIL storm test (task 7.5); Kani for the handler glue
    - Partly done 2026-10-08: `Binding::Intake` in `arch::cm4::irq_entry` (timestamp, `Kernel::intake` with MIT enforcement, source disable for the MIT window via `apply`'s `source_masked` re-enable, R10.9/R19.2/R19.4) and `Binding::Unbound` (disable and `EV_UNBOUND_IRQ`, R19.5). No Sporadic_Task exists in the P1 demo yet; a sporadic demo and the storm test are open (7.5).
    - Done 2026-10-23: the P1 demo has the sporadic Task `pulse` (part1, Priority 4, MIT 15 ms, deferral) bound to the CMSDK dual timer that `log`'s first Job starts at 18 ms through the new `rsk::mmio` accessors (`hw::peripheral_read`/`peripheral_write`, UJ-036/UJ-037, inlined into the Partition and confined by its MPU window); it runs on QEMU: 10 accepted events in 200 ms, no MIT_Violation, no deadline miss (`RSK-OK`), and the Analyzer accepts the set (R = 11.5 ms for pulse under placeholder timing). The demo found two defects. (1) A source masked for the MIT window was never unmasked when its Job ended inside the window (`try_unmask` only at Job end): `Kernel::end_mit_window` now arms `SLOT_UNMASK` (new slot kind, on the Task's deadline slot) at `last_due + MIT`, served by `serve_slot` through `Timing::unmask`; a Partition restart keeps a masked source's unmask event; `job_return` ensures the deadline slot holds at most the unmask event. (2) `mpu_load` rebased enabled regions, so the first Partition window made the view switch fault inside `mpu_load` (IACCVIOL escalated to HardFault): each region is now disabled before it is rebased (Kani `mpu_load_writes_every_region` updated). Also: `apply_core` keeps a stopped Partition's sources disabled (`KernelOps::partition_stopped`) and clears a source's pending state before unmasking it (`Action::ClearPending`). A first demo version with a 2 ms deadline reported the expected deadline misses whenever a Release coincided with sensor's 8 ms Critical_Section (R8.4 blocking), and a 10 ms deadline failed the Analyzer's response-time test (sensor's full Budget interferes), which is why the deadline is 15 ms. The storm test stays with 7.5 (HIL).
    - _Requirements: 10.9, 19.2, 19.4, 19.5, 45.1, 45.2_
  - [x] 5.5 Implement the EasyDMA mediation service (validated pointer and length writes, list-mode rejection, stop on Partition stop or restart) and the Kernel-owned PPI configuration.
    - Evidence: Kani on the validation; HIL DMA cases (task 7.4)
    - Done 2026-10-23: `PartitionLayout::dma_permitted` (R17.1: the buffer lies in the Partition's RAM above its stack region and touches only enabled subregions) with `dma_permitted_exec` proven equal to it and cross-checked by Kani (`dma_permitted_matches_recomputation`, rsk-logic); `KernelOps::dma_permitted` on `Bound`; `dma_request_core` in `arch::cm4::entry` (SVC 6 from `rsk::dma_request(channel, buf)`, stub `svc_dma` UJ-035): the channel must belong to the executing Job's Partition and the buffer be permitted, then `Action::DmaProgram` writes the pointer and count registers, else `Kernel::fault(EV_DMA_REJECTED)` applies the Partition's fault response before any write (R17.2) and the SVC returns 1; `dma_stop_partition` triggers every stop task of a stopped or restarted Partition's channels from `apply_core` on `take_stop_dma` (R17.6); `dma_boot` clears list-mode registers and every PPI channel (DD-08). Generated_Config: `DMA: [DmaCfg; ND]` and `System::dma` from the Target table's new `DmaRegs` per peripheral class (RADIO, UARTE, SPIM, SAADC, PWM; `Peripheral.dma`), emitted in Partition then peripheral-name order; the Generator no longer rejects EasyDMA peripherals (only ones without register descriptions). The trusted `apply` refuses implausible register addresses as defence in depth. Mutation `dma-buffer-in-stack`. Not built: declared PPI routes (PPI stays cleared) and the EasyDMA class table beyond the five classes; the hardware effect waits for `hil-dma` (QEMU has no EasyDMA; the P1 demo owns no EasyDMA peripheral, so its table is empty).
  - [x] 5.6 Implement the `rsk` API: Context types, `Shared` with `Mutex`, `Signal`, `now()`, and `ResourceCell` with its justified `unsafe impl Sync`.
    - Evidence: trybuild compile-fail cases for re-entry, escaping references, undeclared Resources, and async Task bodies; Kani for the SVC glue
    - Partly done 2026-10-08: `crates/rsk` (`Shared::lock` with `&mut self` and a closure over `&mut T`, `now()`, `debug()`, `spin()`; `ResourceCell` with its justified `unsafe impl Sync`, UJ-050..054), used by the P1 demo. Open: generated Context types and `Signal` (need rsk-gen, task 10.5), trybuild cases (task 10.6).
    - Done 2026-10-23: the generated Context types exist since task 10.5 (`rsk_ctx::<Task>` with one `Shared` per declared Resource) and now carry one `rsk::Signal` per Release_Signal the Task `raises`; `rsk::Mutex` (the design's trait) implemented for `Shared`, the inherent `lock` kept; `rsk::Signal::raise` over SVC 7 (`svc_raise`, UJ-038) to the verified `raise_core`, which takes the raise as `Kernel::intake` of the bound Sporadic_Task at `now`, attributed to the raising Partition (R10.5, R10.7), and treats a signal of another Partition, an unbound signal, or an index beyond Generated_Config's `SIGNALS` table (new, with `System::SIGNAL_COUNT`/`signal`) as API misuse with the Partition's fault response; the Job wrapper binds the body's result with `let () =` (an `async` body is a type error, PR-09). Compile-fail evidence in place of trybuild (not vendored, R58.1): the fixture workspace `xtask/fixtures/api-misuse` (a valid declaration with two Resources, a signal, and three Tasks; `api_ok` compiles through both the trait and the inherent lock and raises; `api_reentry` E0499, `api_escape` lifetime error, `api_undeclared` E0609, `api_asyncbody` E0308), checked by `api_misuse_is_rejected_by_the_type_system` in `xtask/tests/fixtures.rs` with `cargo check` on each member from the vendored sources. The SVC glue is verified by Verus (`svc_core`, `raise_core`, `dma_request_core`) rather than Kani; the asm wrappers stay review (UJ-030 to UJ-038). A raise runs on QEMU since the same day: `control` raises the signal `tick` on every third Job and the sporadic `echo` (part0, Priority 6, MIT 20 ms) counts six Releases in 200 ms (`RSK-OK`); `rsk::debug` and `rsk::spin` stay demo aids.
    - _Requirements: 8.6, 8.7, 8.8, 8.10, 45.1, 45.2_
  - [x] 5.7 Move the remaining Kernel_Arch planning code (`arch::cm4`: `apply`, `irq_entry`, `svc`, `timer`, `epilogue`, the fault handlers, `boot`, `start`) into `verus!` behind a `KernelArch` trait whose `apply` specification is tied to the Hardware_Model, giving each register-access function of the Kernel_Unsafe_Module a trusted specification (`assume_specification`, listed in `docs/proof_assumptions.toml` with its UJ identifier and Kani harness or HIL differential test). This turns the inspection facts A1 to A8 and A11 of Verification_Plan section 4 into proof obligations and closes GAP-10 for the planning code.
    - Evidence: Verus run with every `arch::cm4` function verified; assumption gate listing exactly the register-access specifications; TBR-24 reduced to rsk-entry and the stubs; QEMU spike unchanged
    - Added 2026-10-22, after `arch::bound` (`Bound`, `KernelOps`, the CRC, and the priority encodings) moved into `verus!` under task 5.1.
    - Done 2026-10-22: `arch/cm4/entry.rs` inside `verus!` holds every entry point as a verified core (`boot_core`, `start_core`, `apply_core`, `irq_entry_core`, `timer_core`, `svc_core`, `epilogue_core`, `fault_core`) generic over `KernelOps`, `KernelArch`, and `Tables`. `KernelArch::apply` performs one `Action` (BASEPRI, NVIC pend/enable/disable/priority, TIMER compare, MPU view load, FPU access, frame write, PRIMASK, the boot configuration) and is specified as the step of the ghost `HwModel`; `Cm4Hw` implements it with four `#[verifier::external_body]` functions (`apply`, `now`, `cycles`, `boot_checks`), the whole register-facing surface, each listed in `docs/proof_assumptions.toml` (the assumption gate now reports 4 constructs, all listed). Proven: A1 (`basepri_matches` after every writing transition), A3 (boot programming at the model), A4 (only the selected, eligible vector pended), A5 (`CONTROL_JOB`, `EXC_RETURN_THREAD_PSP`), A6 (the dispatched and resumed Partition's view loaded; `Bound::inv` ties it to the layout), A9, A11 (`validate` then `init` first). rsk-kernel: 112 items verified. The shells in `arch/cm4/mod.rs` only fetch the Kernel cell, the hardware, the `Tables` adapter of `System`, and the Kernel_Arch state, call the core, and enter `System::safe_state` when told. Mutations `apply-skips-basepri`, `boot-skips-init`, `dispatch-privileged` registered. Two behaviour changes, both toward the safe state: a frame that would underflow or a Partition index beyond the 16 floors is a Kernel fault, and an interrupt-line count above 496 fails the boot checks (bit 6). Still by inspection: PRIMASK in the stubs (A2), the RBAR/RASR encoding (A7, Kani), the distinctness of intake and dispatch vectors (A8, Generator), reentrancy (A10), and the boot-before-start order in rsk-entry.
    - First increment of the same day: `arch/cm4/plan.rs` inside `verus!` with `plan_frame` (the Job's initial frame is 8-byte aligned, inside the Partition stack region, and the highest such address; HW-06, R16.5), `compare_target` (equal to `hw_model::compare_for`, HW-12), `find_level` (the unwind's level lookup), and the `LevelRecord`/`ArchState` types; `irq_entry`, `arm_compare`, and `epilogue` call them. rsk-kernel: 67 items verified. Mutations `frame-overlaps-floor` and `compare-ignores-near-counter` registered. Next increments: trusted specifications for the register-access functions (`assume_specification`, docs/proof_assumptions.toml), the `System` trait inside `verus!`, then `apply`, `irq_entry`, `svc`, `timer`, `epilogue`, `boot`.
    - Crate layout 2026-10-22: the proof core moved to the new crate `rsk-logic` (logic, hw_model; no unsafe code), re-exported by `rsk-kernel`, so the logic and Kernel_Arch proofs are separate Verus crates with separate SMT contexts (the instability seen when `arch::bound` joined the single crate does not recur; `end_top` verifies at the default rlimit again). Every `rsk_kernel::logic::...` path still resolves.
    - _Requirements: 44.1, 44.5, 45.1, 50.2_

- [ ] 6. Kani harnesses
  - [x] 6.1 Set up Kani before the first Kernel_Unsafe_Module code (task 5.1): pin Kani and CBMC, run every harness in the verification job, count a run as passing only when every check including every unwinding assertion passes, and fail the job when a harness that a justification identifier links to fails. Build the stub library that replaces inline assembly and memory-mapped register accesses with Hardware_Model steps, and the convention for stating and justifying harness bounds. The harness for each justification identifier of an `unsafe` block or `unsafe fn` is written by the task that adds the identifier.
    - Evidence: Kani run in the verification job on sample harnesses that exercise the stubs, with the Kani and CBMC versions recorded in its Evidence_Item; a fixture harness with a failing unwinding assertion that the job rejects; the stub list for the Trust_Base_Register
    - Done 2026-10-08: Kani 0.68.0 / CBMC 6.11.0 pinned (`toolchain/decision.toml`, CI installs with `--locked`); `cargo xtask verify` step 10 runs every Kernel crate with harnesses and passes only with every check (unwinding assertions included), recording versions in Evidence_Item `proof-kernel-kani`; the arch harnesses in `cm4/unsafe_module/kani.rs` exercise the MMIO address table, the frame builder and the BASEPRI/MPU encoders against the `sim` register double (the stub library, listed in `docs/kani_stubs.toml`, TBR-07). The failing-unwinding fixture is `xtask/fixtures/kani-unwind`: its captured output is checked by `proofs::tests` and the fixture runs live in `xtask/tests/fixtures.rs` when Kani is installed (`proofs::kani_verdict` is the R45.5 rule).
    - _Requirements: 45.4, 45.5, 45.7, 45.8_
  - [x] 6.2 Write cross-check harnesses that run each Kernel operation on bounded nondeterministic states and assert its Verus postcondition.
    - Evidence: Kani run
    - Done: `crates/rsk-logic/src/logic/kani_harness.rs` (5 harnesses: validator totality, lock/unlock ceiling, forced end and LIFO resume, selection order) at bounds NT=3, NR=2, NL=4; all pass under Kani 0.68.0 / CBMC 6.11.0; run by `cargo xtask verify` step 10.
    - Done 2026-10-22: `intake_outcomes_match_r10`, the `Kernel::intake` cross-check that design.md Property 8 named: three events of a Sporadic_Task at nondeterministic instants under both MIT policies, asserting R10.1 acceptance, R10.3 while a Release is pending, and R10.1/R10.2 acceptance, deferral, or discard after the Job ended, with R10.7 attribution (NS = 11, NE = 8, unwind 13). 12 harnesses pass.
    - _Requirements: 45.3_
  - [ ] 6.3 Record Kani's limitations and their mitigations in the Verification_Plan.
    - Evidence: Verification_Plan section; review record
    - Partly done 2026-10-14: `docs/verification_plan.md` section 6: the RN-16/17/18 limitations, each with a mitigation or TBR entry; the host-build finding (flight-only volatile and assembly paths are compiled out, so Kani covers address tables, frame words, and encoders only); stub-to-differential-test mapping (R45.4); bounds and their residual risk (R45.8, TBR-07 updated). Open: the review record.
    - _Requirements: 45.6_

- [ ] 7. HIL_Rig and Phase 1 hardware tests
  - Hardware-pending since 2026-10-14: no nRF52840-DK is available. Tasks 7.1 to 7.5, 3.3, the test half of 3.4, and the HIL parts of 3.1, 5.2 to 5.5 are deferred, not dropped: they stay open, task 9 still requires them, and QEMU results are recorded as pre-hardware evidence only. `docs/verification_plan.md` section 11 lists each item, the claims that wait on it, and the work that can proceed without boards (test procedures and predicted outcomes, `rsk-hil` dry runs on QEMU).
  - [ ] 7.1 Build the HIL_Rig: target and stimulus nRF52840-DK boards, probe-rs orchestration, unattended runs, and the read-back hash check.
    - Evidence: rig self-test report
    - _Requirements: 34.8, 48.4_
  - [ ] 7.2 Run the boot-check suite with mismatched configuration values and corrupted tables.
    - Evidence: HIL report
    - _Requirements: 12.6_
  - [ ] 7.3 Run the release-timing test for PAR-05, covering counter wraps, coinciding releases, maximal Critical_Sections, and late Jobs.
    - Evidence: HIL report
    - _Requirements: 9.10_
  - [ ] 7.4 Run the MPU access matrix and the DMA isolation cases.
    - Evidence: HIL report
    - _Requirements: 16.8, 17.7_
  - [ ] 7.5 Run Overrun, interrupt-storm, and response fault injection.
    - Evidence: HIL reports
    - _Requirements: 14.11, 18.8, 19.3_

- [ ] 8. Phase 1 assurance documents
  - [ ] 8.1 Write Verification_Plan v1: the property allocation table, the App_Prover evaluation and designation (Creusot), the tool-qualification classification, the DO-333 plan, the Analyzer correctness plan, and the HAL and PAC strategy for Partitions (ORQ-21).
    - Evidence: document and review record
    - Partly done 2026-10-14: `docs/verification_plan.md` v1 draft against DO-178C DAL A/B with DO-330 and DO-333: property allocation table with current status (R43.1, R43.2, R43.4); type-system guarantees (R43.3); the Kernel_Arch facts the proofs assume (section 4); the arguments that code comments defer to the plan (bounded blocking, lazy release, ceiling restore across preemptions, MIT spacing over a run, deadlock freedom, ORQ-07, the R6.6 VFP deviation, stale pending bits, uncertified Core_Subset items); App_Prover evaluation and Creusot designation (R46.1, R46.2); tool classification and TQLs, the Generator argument with its uncovered outputs, and independence (R51); soundness, model validation with the vacuity checks run so far, property preservation, objectives, and per-criterion coverage (R52); HAL and PAC strategy (R58.3); hardware-pending evidence (section 11). Open: the review record (independent reviewer, an applicant activity, R54.4); R43.5 at criterion level (needs task 8.4); App_Prover pinning and the facts marked "to confirm" (task 17.1); DO-178C and DO-333 objective numbering, now taken from the Gap_Register (section 9.4) and still to confirm against the standards' texts (OPEN-01, OPEN-02). The HIL frequency of R50.1 was added to section 11 with task 8.2.
    - _Requirements: 43.1, 43.2, 43.3, 43.4, 43.5, 46.1, 46.2, 51.1, 51.2, 51.3, 51.4, 51.5, 51.6, 51.7, 52.1, 52.2, 52.3, 52.4, 52.5, 58.3_
  - [x] 8.2 Write Trust_Base_Register v1 and Gap_Register v1 from the design's trust-base analysis, and wire assumption gating into the verification job.
    - Evidence: documents; failing fixture with an unlisted `external_body`
    - Done 2026-10-14: `docs/trust_base_register.toml` v1.1: TBR-01 to TBR-24, each with what is trusted, why, evidence, residual risk, dependent requirements and Verification_Plan properties (R48.2), and ASM and ORQ links (R48.3); new TBR-22 (Static_WCET_Analyzer), TBR-23 (QEMU as pre-hardware evidence), TBR-24 (Kernel_Arch planning code and the inspection facts A1 to A11); an `[[r48_1]]` table mapping all 21 R48.1 items to entries (R48.1). `docs/gap_register.toml` v1: all 71 DO-178C Annex A objectives with applicability, status, owner (prototype, applicant, or shared), evidence or gaps (R54.1, R54.4); DO-333 and DO-330 items; GAP-01 to GAP-19, of which GAP-01 to GAP-07 are the R54.2 gaps; all 32 ORQs with their design.md status (R54.3). `xtask/src/checks/registers.rs` (`Rule::TrustBaseRegister`, `Rule::GapRegister`) checks both in `cargo xtask check` and step 1 of the verification job, including that every ORQ status in the Gap_Register equals the one design.md gives. The assumption gate (R50.2) now runs with the workspace checks, covers the Kernel crates and every crate that opts into Verus (rsk-analyzer included), and names constructs inside `proof fn` bodies correctly; fixture `xtask/fixtures/assumption-unlisted` (an unlisted `external_body` in a Kernel crate and an unlisted `admit` in a Verus opt-in host crate) fails naming R50.2. The Verification_Plan records the HIL frequency (section 11, R50.1). Open: Level C applicability and per-objective independence to confirm against the DO-178C text (OPEN-01), and the DO-333 and DO-330 items (OPEN-02); R54.3 for criterion and ASM status changes is kept by review until the traceability matrix (task 8.4); R50.1 in part, because HIL suites are hardware-pending (GAP-09) and the Phase 2 checkers are not built (GAP-13); R50.4 in part, because proof re-runs from archived inputs are not automated (GAP-16); independent review of both registers is an applicant activity (GAP-08).
    - _Requirements: 48.1, 48.2, 48.3, 50.1, 50.2, 50.4, 54.1, 54.2, 54.3, 54.4_
  - [ ] 8.3 Complete the reuse record: the RTIC macro, ceiling, and BASEPRI analysis, the RTIC soundness-fix mapping, the Hubris adoption, and the Embassy rationale.
    - Evidence: Design_Document addendum and review record
    - _Requirements: 61.1, 61.2, 61.3, 61.4, 61.5, 61.6_
  - [ ] 8.4 Build the traceability matrix and the report of acceptance criteria that lack Evidence_Items.
    - Evidence: generated matrix; CI report
    - _Requirements: 50.5, 50.6_

- [ ] 9. Checkpoint: Phase 1 complete. Ensure all proofs, Kani runs, and HIL suites pass, and ask the user if questions arise.

### Phase 2 — Task-model DSL, schedulability analyzer, UPPAAL export

- [ ] 10. Schemas and Generator (Component C)
  - [ ] 10.1 Implement `rsk-model` with JSON Schemas for the Task_Model, WCET_Records, and Kernel_Timing_Parameters.
    - Evidence: PBT round trips (Property 15, Task_Model and WCET_Record parts); schema validation tests
    - _Requirements: 25.1, 25.4, 25.5, 25.6, 33.1, 33.2_
  - [ ] 10.2 Implement the `rsk::app!` parser and declaration validation, with source-span diagnostics that name PR identifiers.
    - Evidence: trybuild cases for each rejection of R15, R21, and R22
    - _Requirements: 3.1, 15.1, 15.2, 15.3, 15.6, 15.7, 21.1, 21.3, 21.4, 21.5, 21.6, 22.1, 22.2, 22.3, 22.4, 22.5, 22.6_
  - [ ] 10.3 Implement the derived values: Ceilings (Endpoint buffers at the Kernel level), nesting depths, capacities including the timer-slot rule of R9.4, the priority-to-hardware map, and time conversions.
    - Evidence: PBT for Property 13 (order independence and determinism) and Property 14 (conversion direction)
    - _Requirements: 21.2, 23.1, 23.2, 23.3, 23.4, 23.5, 23.6, 40.1_
  - [ ] 10.4 Generate the MPU layout and linker script from the verified view function, and reject layouts that cannot be realized.
    - Evidence: PBT on layout realizability; Property 12 carried over from 4.9
    - _Requirements: 20.4, 24.5_
  - [ ] 10.5 Implement the expansions: Partition manifest macros, Context types, the system crate's `CONFIG`, and the `rsk-entry` binding through the vendored path dependency. Validate the binding in a build spike.
    - Evidence: example Application that builds with `#![forbid(unsafe_code)]` in every Application crate; Link_Checker template check of `rsk-entry`
    - _Requirements: 15.5, 24.1, 24.2, 24.6_
  - [ ] 10.6 Implement the `rsk-gen` CLI: Task_Model printer with schema self-validation, configuration summary, and byte-identical output.
    - Evidence: determinism tests (Property 13)
    - _Requirements: 15.4, 24.3, 24.4, 25.2, 25.3_
  - [ ] 10.7 Add core fields and multi-core rejection to the Generator, the Analyzer, and the Config_Checker.
    - Evidence: rejection tests whose diagnostics cite NG-01
    - _Requirements: 27.1, 27.2, 27.3, 27.4, 27.5_

- [ ] 11. Profile_Lint
  - [ ] 11.1 Configure the Clippy deny-lists and implement Dylint lints for async Task bodies, loop labels with `#[rsk::bounds]`, the Core_Subset, and attempts to lower lint levels.
    - Evidence: UI tests per lint; bypass fixtures rejected
    - _Requirements: 3.2, 3.4, 35.1, 35.5, 57.2_

- [ ] 12. Link_Checker
  - [x] 12.1 Implement ELF reading, Thumb-2 decoding of SP and control-flow effects, the call graph, recursion detection, and indirect-call resolution.
    - Evidence: fixture binaries; differential run against `cargo-call-stack`
    - Done 2026-10-23: `crates/rsk-link` (dependencies: the shared ELF reader rsk-elf and serde_json). `thumb.rs` decodes the Thumb-2 forms that move SP or transfer control (16-bit push/pop, `add`/`sub sp`, `bx`/`blx`, `mov sp, Rm`, pc-relative `ldr`, `b`, `b<cond>`, `cbz`/`cbnz`, `svc`; 32-bit `push.w`/`pop.w`, `vpush`/`vpop`, pre-indexed `str`/`strd` to SP, `sub.w`/`subw`/`add.w`/`addw sp`, register forms of SP arithmetic, `movw`/`movt`, `ldr.w` literals, `tbb`/`tbh`, `bl`, `b.w`, `b<cond>.w`), each form checked against a hand-assembled encoding in its unit tests; every other instruction is skipped by length. `graph.rs` builds the call graph over the function symbols: a function's frame is the sum of its SP decrements (the compiler's prologue discipline bounded from above), direct `bl` and out-of-function branches are edges, a `blx Rm` resolves to the function addresses the body materialises (literal pool, `movw`/`movt`) or, failing that, to every function address stored in the data sections (the vtable and function-table universe, finite, PR-18), a `bx Rm` with nothing materialised in a vector-table stub is the exception return (UJ-020, UJ-023); recursion is a cycle naming its members (PR-17); a function writing SP from a register is unbounded (R37.3). The panic path (`core::panicking`, `rust_begin_unwind`, the Kernel panic handler) is cut from the walk and charged once per entry point (the "at most one panic" rule), which removes the only cycle of the P1 demo (handler -> `Bound::fault` -> `apply_response` -> ... -> bounds check -> panic); `core::fmt` functions with trait-object calls and no vtable in the image are linked for the panic message the handler never formats and are reported as a note. Demangling of legacy symbols for the Task entry paths (`demangle.rs`); v0 core symbols recognised by substring. On the P1 demo: 72 functions, 138 edges, 6 indirect call sites, all bounded. Not done: the differential run against `cargo-call-stack` (not vendored, R58.1; it is also nightly-only) and fixture binaries beyond the hand-assembled encodings (the P1 demo is the binary under test).
    - _Requirements: 3.3, 37.3_
  - [x] 12.2 Implement per-region stack bounds using the preemption relation and FP storage, and export them for the Config_Checker.
    - Evidence: fixtures with known stack depths; stack painting on the conformance Application over the HIL_Rig
    - Done 2026-10-23: `stack.rs`. A Partition's region bound is the sum over its Tasks (all can be in progress at once, one per Priority, R7.1) of the Job wrapper's worst-case usage plus the initial exception frame (32) plus the frame the hardware stacks on preemption (32, or 104 with an active FP context, R11.4); the Kernel MSP bound is the worst vector-table entry point plus one 40-byte level record per Job level (104 with S16-S31 when any Task uses the FPU) plus a 32-byte frame from the idle loop, against the Kernel RAM above `.kernel.data` and `.kernel.bss`. A region smaller than its bound, an unbounded Task wrapper or Kernel entry point, or a Task entry absent from the binary fails the build (R37.2, R37.3). The export `rsk-stack-bounds/1` (functions, Tasks, regions with their terms) is an artifact of the verification job (`stack-bounds-<binary>.json`, Evidence_Item `analysis-stack-<binary>`, R37.4) and the Config_Checker's third input: `rsk-confcheck` compares each declared stack region with the Link_Checker's view of it and requires the bound to fit (R26.2). On the P1 demo: part0 2092 of 4096, part1 640 of 4096, MSP 2692 of 30440 bytes. Not done: stack painting on the HIL_Rig (hardware-pending, section 11).
    - _Requirements: 11.4, 37.1, 37.2, 37.4_
  - [x] 12.3 Implement the instruction-class scans, the allocator, unwinding, and `vstd` symbol checks, the Core_Subset symbol mapping, the dependency-graph checks, and the no-panic-path check from Kernel entry points. The instruction scans also serve as the defence-in-depth check of R16.10, although DD-01 enforces every access class in hardware.
    - Evidence: fixtures per check
    - _Requirements: 6.6, 14.1, 16.10, 55.4, 57.3_
    - Done 2026-10-23: `rsk-link` takes `--profile profile/profile.toml --core-subset profile/core_subset.toml` (passed by `cargo xtask verify` step 8a) and runs `scan.rs` over the call graph, classifying each function by section (`.kernel.text` Kernel, `.pN.text` Partition, `.text`/`.shared.rsk` shared). Instruction classes: the decoder recognises VFP instructions by class (data movement, arithmetic/comparison/conversion, FPSCR write, FPSCR read), `msr`/`mrs` with the SYSm name, `cpsie`/`cpsid`, `cbz`/`cbnz`, and `nop`; Kernel code (by section or reachable from a vector-table handler) may contain no FP arithmetic or FPSCR write (R6.6, PR-31); code reachable from an FPU-free Task may contain no FP instruction (PR-31); code reachable from any Task may execute no `msr`/`mrs`/`cps` and materialise no Private Peripheral Bus address (PR-22, R16.10), and may not reach `.kernel.text` except through an SVC (R16.3); naked stubs (`rsk_*`) may contain no cross-section `cbz`/`cbnz` and no `nop` (ORQ-32). Symbols: allocator (`__rust_alloc*`, `__rg_*`, `__rdl_*`; PR-15), unwinding (`_Unwind_*`, `rust_eh_personality`, a non-empty `.ARM.exidx`; PR-19), exactly one `rust_begin_unwind` (R14.1), no `vstd` crate (R55.4), no `std`, `alloc`, or `[disallowed_crates]` crate, by crate-name extraction from legacy and v0 manglings (PR-35, R1.5). Core_Subset: every linked `core::` and `compiler_builtins` symbol must map to a Core_Subset item by module prefix and listed name (R57.3); the new scope `unreachable` marks items that are linked but must be reached by no Task entry or Kernel entry point. No-panic-path: an explicit `panic_fmt`/`panic` call from Kernel code is a violation (PR-19); the compiler's implicit checks are reported per kind with their Kernel callers (R14.1). Every finding names the Task or Kernel entry point that reaches it (R3.3). Profile 0.5 adds `core::panicking`, `compiler_builtins`, and (scope `unreachable`) `core::fmt` and `core::str::count`, which the demo Flight_Build links for the panic message. On the P1 demo: 57 Kernel functions and 13 naked stubs scanned, 25 Task-reachable functions scanned, 17 `core`/intrinsic symbols mapped, two implicit panic kinds reported (bounds check through 21 Kernel functions, remainder by zero through 2), no violation. Fixtures: `scan::tests` (one fixture per check, each producing its violation: R6.6, PR-31, R16.3, PR-22, ORQ-32, PR-15, PR-19 (three forms), R55.4, PR-35, R57.3 (two forms)) and `thumb::tests::privileged_and_floating_point_classes`; a Profile naming `rsk-logic` as disallowed makes the demo fail with PR-35. The `cargo-call-stack` differential run of 12.1 remains not possible offline.

- [ ] 13. Config_Checker
  - [x] 13.1 Implement the independent parser, recomputation of the R26.2 values, extraction of `CONFIG` from the ELF, and the checksum checks.
    - Evidence: Property 15 for the Config_Checker parser; comparison tests on the conformance Application
    - Done 2026-10-23: `crates/rsk-confcheck` (dependencies: serde_json and the shared ELF reader rsk-elf; never rsk-model, rsk-gen, rsk-logic, or rsk-kernel, R26.1). `model.rs` reads the Task_Model JSON by field name into its own structures; `recompute.rs` recomputes every R26.2 class from the declared values and compares (Ceilings as the accessors' maximum Priority; the DD-02 priority map `(8 - p) << 5` and Kernel level 0; nesting depths; capacities: lock stack = Σ depths, timer slots = 3·NT + 2 (R9.4), log > 0; the slot numbering; ascending dispatch vectors; deadline within period or MIT; Kernel-owned peripherals from the checker's own Target list and single ownership; MPU: aligned power-of-two regions, stack within RAM, disjointness among Partitions and from the Kernel regions, 4 KiB windows, read-only exactly for the checker's EasyDMA list) and, against the binary, `image.rs` locates `RSK_CONFIG_IMAGE` and `RSK_CONFIG_CHECKSUM`, decodes the `CONFIG` tables with the Kernel's documented byte order and codes restated here, recomputes CRC-32 (IEEE 802.3), and compares both checksums (R25.2, R26.4) and every table field with the declared values. Every discrepancy is reported as item, expected, found, exit 1 (R26.3). `cargo xtask verify` step 8b runs it on every Flight_Build with a Task_Model (R26.5; Evidence_Item `check-config-<binary>`, Kind check). On the P1 demo: 159 values compared, no discrepancy; a binary with one altered `CONFIG` byte reports the checksum (twice) and the field. Tests: `the_demo_task_model_recomputes_without_discrepancy`, `altered_derived_values_are_reported_per_class` (one altered value per R26.2 class in the Task_Model), CRC check value, synthetic image decoding. Not done then: extraction of the `LAYOUT` from the binary and the mutation suite (both done in 13.2 the same day), the stack sizes against the Link_Checker (task 12), and the Property 15 proof of the parser (it is tested, not proven).
    - _Requirements: 16.7, 25.5, 26.1, 26.2, 26.3, 26.4, 26.5_
  - [x] 13.2 Build the mutation suite of altered binaries and Task_Models.
    - Evidence: Property 19
    - Done 2026-10-23: `docs/config_mutations.toml` registers 16 mutations, at least one per R26.2 value class (ceilings, priority map, nesting depths, capacities, Budget and MIT tables, Kernel-owned peripherals, MPU permissions, checksum): altered Task_Model text (`find`/`replace`), a changed checksum digit (`model-hex-digit`), and altered binaries (one byte of `RSK_CONFIG_IMAGE`, `RSK_LAYOUT_IMAGE`, or a checksum word xor-ed at a symbol offset); `xtask::confcheck_mutations` applies each to a copy of the P1 demo's Task_Model or Flight_Build and requires `rsk-confcheck` to exit non-zero naming every expected item; run by `cargo xtask verify` step 8b (Evidence_Item `test-config-mutations-<binary>`) and `cargo xtask confcheck-mutations <model> <elf>`; a unit test checks the register against the committed demo model and the class coverage. 16 of 16 reported. With it the `LAYOUT` joined the binary comparison: the Generator emits `RSK_LAYOUT_IMAGE` and `RSK_LAYOUT_CHECKSUM` (`derive::layout_bytes`, CRC-32), `System::LAYOUT_CHECKSUM`, and the Task_Model's `layout_checksum`; the Kernel compares `layout_crc(&LAYOUT)` with it at boot (bit 13, R12.3); the checker decodes the image and compares every interval, stack size, and window with the Task_Model (R16.7) and both checksums. The P1 demo now compares 197 values. The P1 demo stands in for the Profile_Conformance_Suite Application (task 14).
    - _Requirements: 26.6, 26.7_

- [ ] 14. Profile_Conformance_Suite
  - [ ] 14.1 Write rejected programs per restriction and static category, the accepted Application per Profile_Variant, and the RT fault-injection cases. Wire R3.5 and R3.10 into the verification job.
    - Evidence: suite run in CI with every case passing
    - _Requirements: 3.5, 3.6, 3.7, 3.8, 3.9, 3.10, 3.11_

- [ ] 15. Analyzer (Component D)
  - [ ] 15.1 Implement input loading and the rejection rules.
    - Evidence: unit test per rejection rule
    - _Requirements: 13.3, 28.1, 28.2, 28.3, 28.4, 28.5, 28.6, 28.7, 28.8_
  - [ ] 15.2 Implement the response-time analysis of R29, with the fixed-point core in Verus.
    - Evidence: Verus proof of Property 16; unit tests
    - _Requirements: 29.1, 29.2, 29.3, 29.4, 29.5, 29.6, 29.7, 29.8, 29.9, 32.1_
  - [ ] 15.3 Implement the JSON and human-readable reports with provenance and distinct exit statuses.
    - Evidence: golden-file tests; determinism test
    - _Requirements: 18.9, 30.1, 30.2, 30.3, 30.4, 30.5_
  - [ ] 15.4 Write the property-based and reference tests: monotonicity, metamorphic locality, the independent reference implementation, the SRP simulator with Overruns, and published task sets.
    - Evidence: PBT for Properties 17 and 18; report reproducing the published response times
    - _Requirements: 32.2, 32.3, 32.4, 32.5, 32.7_

- [ ] 16. UPPAAL export
  - [ ] 16.1 Implement the UPPAAL_Exporter, the query file, verifier invocation, verdict comparison, and inconclusive handling.
    - Evidence: export round trip (Property 15, UPPAAL part); verifier runs on sample models
    - _Requirements: 31.1, 31.2, 31.3, 31.4, 31.5, 31.6, 31.7, 31.8, 31.9_
  - [ ] 16.2 Build the cross-check corpus of PAR-03 models and record each discrepancy with its disposition.
    - Evidence: corpus report
    - _Requirements: 32.6_

- [ ] 17. Application proofs and interleaving tests
  - [ ] 17.1 Add Creusot contracts to the `rsk` API, and prove the conformance Application's level A and B Task bodies, loop bounds, and Resource invariants.
    - Evidence: Creusot proofs with pinned tool versions
    - _Requirements: 35.2, 46.3, 46.4, 46.5_
  - [ ] 17.2 Build the Loom host model of the Kernel logic and Shuttle tests for the larger configurations.
    - Evidence: Loom and Shuttle runs labelled as testing evidence
    - _Requirements: 47.1, 47.2, 47.3_
  - [ ] 17.3 Evaluate Aeneas on one pure control-law function as a Lean-level alternative for small computations.
    - Evidence: evaluation note in the Verification_Plan
    - _Requirements: 46.5_

- [ ] 18. Checkpoint: Phase 2 complete. Ensure all tests and proofs pass, and ask the user if questions arise.

### Phase 3 — Measurement-based WCET with HIL tests

- [ ] 19. WCET_Harness (Component E)
  - [ ] 19.1 Implement the target agent and host runner. Task-entry maxima come from Budget accounting. Measurement builds instrument Critical_Sections, Endpoint operations, and Kernel operations, with the cycle-counter read cost calibrated on the Target.
    - Evidence: calibration report; agent results compared with a logic-analyser trace for one Task
    - _Requirements: 34.1, 34.2, 34.3, 34.7_
  - [ ] 19.2 Implement coverage measurement, interference scenarios (including maximum EasyDMA traffic), loop-bound driving, the margin, and unattended campaigns.
    - Evidence: HIL measurement-campaign report
    - _Requirements: 17.4, 34.4, 34.5, 34.6, 34.8, 34.9, 35.3, 35.4_
  - [ ] 19.3 Emit WCET_Records and Kernel_Timing_Parameter records, including Δ_svc, Δ_dma_write, and Δ_view, and fail on any observed execution above an exported bound.
    - Evidence: record schema validation; the Analyzer rejecting stale or mismatched records
    - _Requirements: 13.1, 13.2, 13.4, 13.5, 13.6, 14.8, 33.1, 33.4_
  - [ ] 19.4 Run the conformance Application end to end: feed measured WCETs to the Analyzer, then confirm on the HIL_Rig that observed response times stay within the computed WCRTs, with and without injected Overruns in lower-criticality Partitions.
    - Evidence: Analyzer report; HIL response-time report
    - _Requirements: 18.7, 29.5_
  - [ ] 19.5 Evaluate structural coverage tools for MC/DC and decision coverage of Rust code, select the method, and produce the first coverage reports.
    - Evidence: evaluation report; coverage reports for the Kernel and the level A, B, and C Partition code
    - _Requirements: 53.1, 53.2, 53.3, 53.4_

- [ ] 20. Checkpoint: Phase 3 complete. Ensure all tests pass, and ask the user if questions arise.

### Phase 4 — Session-typed Endpoints

- [ ] 21. Protocol_Checker and Endpoints (Component F)
  - [ ] 21.1 Implement the Protocol syntax, projection, well-formedness checks, and the k-MC buffer bound.
    - Evidence: PBT for Property 20; differential runs against Rumpsteak's k-MC checker
    - _Requirements: 22.7, 38.2, 38.3, 38.5, 38.6, 41.1_
  - [ ] 21.2 Implement Kernel-owned Endpoint buffers: SVC send and receive with copying, full-buffer errors, completion of operations interrupted by a forced Job end, and retention of messages whose arrival events are discarded.
    - Evidence: Verus proof of the Endpoint part of Property 11; Kani for the copy glue
    - _Requirements: 10.8, 39.1, 39.2, 39.3, 39.4, 40.3, 40.4, 40.5, 40.6, 42.2, 45.1, 45.2_
  - [ ] 21.3 Generate the typestate role APIs and RoleSlots, with run-time detection of dropped or unstored role states.
    - Evidence: trybuild cases for illegal operations; HIL protocol-violation cases
    - _Requirements: 38.1, 38.4, 38.7, 41.2_
  - [ ] 21.4 Implement the cross-partition rules: the `Plain` derive with bit validity, receiver-side validation functions proved in Creusot, and peer-state errors.
    - Evidence: trybuild cases; Creusot proofs; HIL Partition-restart case
    - _Requirements: 42.1, 42.3, 42.4_
  - [ ] 21.5 Extend the Analyzer with Endpoint blocking terms and end-to-end chain latency.
    - Evidence: unit tests; PBT against the simulator
    - _Requirements: 40.2, 41.4_
  - [ ] 21.6 Write the communication deadlock-freedom proof and list its assumptions.
    - Evidence: proof document or mechanized proof; review record
    - _Requirements: 41.3_

- [ ] 22. Checkpoint: Phase 4 complete. Ensure all tests and proofs pass, and ask the user if questions arise.

### Phase 5 — Ferrocene build, static WCET, RISC-V port, Jorvik-style extensions

- [ ] 23. Qualified toolchain
  - [ ] 23.1 Build Flight_Builds with a pinned Ferrocene release, apply its Safety Manual constraints, run the dual build, archive and compare the ghost-erased Kernel source, and assess the release's Known Problems.
    - Evidence: Flight_Build Evidence_Items naming the Ferrocene release; constraint compliance list; Known Problems assessment
    - _Requirements: 55.2, 55.3, 56.1, 56.2, 56.3, 56.4, 56.5_

- [ ] 24. Static WCET
  - [ ] 24.1 Select and integrate a static WCET tool, export loop bounds and indirect-call targets to it, and cross-check its bounds against the measurements.
    - Evidence: static WCET_Records; discrepancy report
    - _Requirements: 33.5, 36.1, 36.2, 36.3, 36.4, 36.5_

- [ ] 25. Ports
  - [ ] 25.1 Port Kernel_Arch to a Cortex-M33 board, confirming NONBASETHRDENA availability or switching that target to the fallback, and extend the Hardware_Model.
    - Evidence: all Phase 1 suites passing on the new target
    - _Requirements: 45.1, 60.1, 60.2, 60.3, 60.5_
  - [ ] 25.2 Select a RISC-V part, record its interrupt controller and ceiling mapping, port Kernel_Arch, and extend the Hardware_Model.
    - Evidence: all Phase 1 suites passing on the RISC-V target
    - _Requirements: 45.1, 60.4, 60.5_
  - [ ] 25.3 Evaluate a privileged fast path for the highest Criticality_Level (ORQ-01 option c).
    - Evidence: Δ_lock_in and Δ_lock_out compared with the SVC path
    - _Requirements: 20.5_

- [ ] 26. Jorvik-style variant
  - [ ] 26.1 Implement the Jorvik Profile_Variant in the Profile, the Generator, the Profile_Lint, the Kernel, and the Analyzer, and update Table 4-1.
    - Evidence: conformance cases for the variant; Verus proofs of the changed transitions
    - _Requirements: 5.1, 5.2, 5.3, 5.4, 5.5, 5.6, 5.7_

- [ ] 27. Checkpoint: Phase 5 complete. Ensure all tests and proofs pass, and ask the user if questions arise.

## Task Dependency Graph

Tasks in the same wave can run in parallel. Each wave starts after the previous one finishes.

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1"] },
    { "id": 1, "tasks": ["1.2", "1.3", "2.1", "2.2", "3.1", "3.4"] },
    { "id": 2, "tasks": ["3.2", "4.1", "7.1"] },
    { "id": 3, "tasks": ["3.3", "4.2", "4.3", "4.5", "4.8", "4.9", "6.1"] },
    { "id": 4, "tasks": ["4.4", "4.6", "4.7"] },
    { "id": 5, "tasks": ["4.10", "5.1"] },
    { "id": 6, "tasks": ["4.11", "5.2", "5.3", "5.4", "5.5", "5.6", "5.7"] },
    { "id": 7, "tasks": ["4.12", "4.13", "4.14", "6.2", "6.3", "7.2", "7.3", "7.4", "7.5"] },
    { "id": 8, "tasks": ["8.1", "8.2", "8.3", "8.4"] },
    { "id": 9, "tasks": ["9"] },
    { "id": 10, "tasks": ["10.1", "11.1", "12.1", "17.2"] },
    { "id": 11, "tasks": ["10.2", "12.2", "12.3", "15.1"] },
    { "id": 12, "tasks": ["10.3", "15.2"] },
    { "id": 13, "tasks": ["10.4", "15.3", "16.1"] },
    { "id": 14, "tasks": ["10.5", "10.6", "15.4", "16.2"] },
    { "id": 15, "tasks": ["10.7", "13.1"] },
    { "id": 16, "tasks": ["13.2", "14.1", "17.1"] },
    { "id": 17, "tasks": ["17.3"] },
    { "id": 18, "tasks": ["18"] },
    { "id": 19, "tasks": ["19.1"] },
    { "id": 20, "tasks": ["19.2", "19.5"] },
    { "id": 21, "tasks": ["19.3"] },
    { "id": 22, "tasks": ["19.4"] },
    { "id": 23, "tasks": ["20"] },
    { "id": 24, "tasks": ["21.1", "21.2"] },
    { "id": 25, "tasks": ["21.3", "21.5", "21.6"] },
    { "id": 26, "tasks": ["21.4"] },
    { "id": 27, "tasks": ["22"] },
    { "id": 28, "tasks": ["23.1", "24.1", "25.1", "25.2", "26.1"] },
    { "id": 29, "tasks": ["25.3"] },
    { "id": 30, "tasks": ["27"] }
  ]
}
```

## Notes

- Certification target, confirmed by the user on 2026-10-14: DO-178C DAL A/B with DO-330 tool qualification and DO-333 formal-methods credit, the fixed input of requirements.md. rsk keeps this path open and produces no applicant plans (NG-12); `docs/verification_plan.md` is written against it.
- Hardware, 2026-10-14: no nRF52840-DK is available. HIL work is deferred, not dropped (task 7 note, Verification_Plan section 11); the Phase 1 checkpoint (task 9) stays open until the HIL suites pass on the Target.
- Tasks marked `*` (17.3 and 25.3) are optional evaluations; no requirement depends on their outcome.
- Phase 1 runs on hand-written `Config` tables. Task 14.1 re-runs the Phase 1 suites on Generator output so that both paths are covered.
- If the DD-01 spike (3.1) fails, tasks 4.2, 5.1, and 6.1 switch to the fallback of ORQ-01 option (b). The logic proofs stay; only Kernel_Arch and its proofs change.
- Values still to confirm in Phase 1: PAR-01 to PAR-08, the WCET margin and coverage thresholds, and the Kani bounds. Facts that design.md flags as unconfirmed (NONBASETHRDENA semantics, its availability on Armv8-M, and the applicability of erratum 838869 to r0p1) are checked in tasks 3.1, 3.4, and 25.1.
- R47.4 applies only once the partitioned multi-core extension exists, which is outside this plan (NG-01), so no task references it.
- Each evidence item becomes an Evidence_Item with tool versions and the Build_Manifest identity (R50.3). Task 8.4's matrix tracks which acceptance criteria still lack evidence.
- Every task that adds an `unsafe` block, `unsafe fn`, or `unsafe impl` to the Kernel_Unsafe_Module (5.1 to 5.6, 21.2, 25.1, 25.2) gives it a justification identifier and adds, in the same task, its Kani harness or Verus proof or lemma where one is feasible, and otherwise a recorded review that states why. Kani is set up first (6.1), so the gate of task 1.2 passes after every task.
- The DD-01 spike (3.1) is built in its own Cargo workspace outside the rsk workspace, as the xtask fixtures are, so the unsafe-code gates of tasks 1.1 and 1.2 do not apply to it. Task 5.1 moves its code into the Kernel_Unsafe_Module under those gates.
