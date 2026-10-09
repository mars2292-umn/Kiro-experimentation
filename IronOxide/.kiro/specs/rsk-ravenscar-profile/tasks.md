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
    - Done 2026-10-08: `profile/core_subset.toml` (23 items with scope and per-item certification evidence or substitute rsk evidence; Ferrocene's per-item list is customer-only, recorded as assumed/unconfirmed, ORQ-24); checked and rendered by `cargo xtask profile`; Gap_Register entry in task 8.2.
    - _Requirements: 57.1, 57.4, 57.5_

- [ ] 3. Hardware risk retirement and Hardware_Model
  - [x] 3.1 Build the dispatch spike for DD-01: three dispatch vectors at different levels; stubs that enter unprivileged Thread mode with the vector active; SVC lock and unlock through BASEPRI; SVC job-end with the two-step return; MPU view switching between two Partitions; eager FP stacking; a DSB before every exception return. Record the go or fallback decision (ORQ-01 option b) in the Design_Document.
    - Evidence: HIL report on preemption order, preservation of core and FP register patterns, faults on unprivileged PPB access, and BASEPRI ceilings that mask only Task levels; decision record
    - Done 2026-10-08 on QEMU (`cargo xtask qemu`, `apps/p1-demo`: 3 Tasks at levels 7/5/2 in 2 Partitions, SVC lock/unlock with a Ceiling-7 Resource held across a control Release, MPU view switching, eager FP stacking, DSB before every return, operating-duration end into the safe state; `RSK-OK`, 20/10/5 Jobs, 0 faults, 0 misses). **Decision: DD-01 revised, not the option (b) fallback.** The spike found that an exception return always deactivates the returning vector, so "Thread mode with the vector active" cannot exist; the executing Job's level is now enforced by BASEPRI and Job end unwinds the level record (design.md "Running Jobs unprivileged", ORQ-01, ORQ-32). Also found: a cross-section `cbnz` assembles to `nop` without a diagnostic (ORQ-32), and the Partition stack floor must follow the preempted Job's live PSP. The verification job runs the spike on QEMU as step 10b (Evidence_Item `test-qemu-spike-p1-demo`). Open for hardware (3.3/7.x): register-pattern preservation, faults on unprivileged PPB access, Budget accounting (QEMU has no DWT cycle counter).
    - _Requirements: 7.7, 11.1, 16.9, 20.1, 20.2, 20.5_
  - [x] 3.2 Write the Hardware_Model in Verus specification code for every behaviour listed under "Kernel proofs" in design.md, cite the defining manual section of each rule, and list the behaviours left out with the reason no proof depends on them.
    - Evidence: `hw_model` verifies under Verus; citation table; review record
    - Done 2026-10-08: `crates/rsk-kernel/src/hw_model.rs` (rules HW-01 to HW-14 with manual sections; lemmas `lemma_nvic_takes_iff_eligible` tying R7.1 to NVIC arbitration under BASEPRI, `lemma_mpu_grant_covered` tying the hardware MPU decision to the view over-approximation; errata and not-formalized list). Open: HIL differential tests (3.3) and the hardware confirmation of 838869 applicability and the TIMER near-counter rule (3.1).
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
    - Done 2026-10-08: `crates/rsk-kernel/src/logic/{mod,sched,time,budget,log,mpu,kernel}.rs` inside `verus!`, depending on the vendored `vstd` (no_std); `cargo xtask verify` step 9 runs `cargo-verus verify` with the Verus 0.2026.10.04 release (rustc 1.98.1, Z3 4.16.0) and writes `proof-kernel-verus` with the versions; the assumption gate (R50.2) reads `docs/proof_assumptions.toml`. Result at this revision: 198 items verified, 0 errors, 0 assumption constructs; 921 executable Kernel lines (PAR-01).
    - _Requirements: 6.5, 44.6_
  - [x] 4.2 Implement and prove the dispatch, completion, LIFO-resumption, and idle transitions.
    - Evidence: Verus proofs of Properties 1 and 2 (logic part)
    - Done: `Sched::select`, `start`, `complete` (sched.rs) with `inv`; DD-03 tie order; `hw_model::lemma_nvic_takes_iff_eligible` for the hardware side of R7.1.
    - _Requirements: 7.1, 7.2, 7.3, 7.4, 7.5, 7.6, 20.3, 44.1_
  - [x] 4.3 Implement and prove the lock and unlock services with the lock stack, including the check against the calling Task's declared Resources.
    - Evidence: Verus proofs of Properties 3, 4, and 5
    - Done: `Sched::lock`/`unlock` (ceiling mirror, LIFO, re-entry refusal, accessor check), `lemma_no_other_holder` (mutual exclusion), `lemma_blocked_by_one_holder` (state part of bounded blocking; the trace argument is recorded in the Verification_Plan).
    - _Requirements: 8.1, 8.2, 8.3, 8.4, 8.5, 44.1_
  - [x] 4.4 Implement and prove forced Job ending and the closed response sets of DD-10, including END_JOB escalation and Partition isolation for every response.
    - Evidence: Verus proofs of Property 6 and of R14.9 for each response
    - Done: `Sched::end_job`/`kill`/`finish_top` (Property 6; lazy release for preempted Jobs documented), `Kernel::apply_response` with END_JOB escalation (R18.4), RESTART/STOP/SAFE_STATE (kernel.rs). Open: R14.9 as an explicit frame lemma per response (every transition proves `inv`; the other-Partition-unchanged statement is not yet a separate theorem) and the R14.10 bound (measured in P3).
    - _Requirements: 8.9, 14.5, 14.8, 14.9, 14.10, 18.4_
  - [x] 4.5 Implement and prove the 64-bit time base, the timed-event slot table, periodic release, deadline detection, release overlap and discard, and the operating-duration limit.
    - Evidence: Verus proofs of Property 7 and the slot-table part of Property 11
    - Done: `time.rs` (`Slots` refining a map, `Timing::advance_periodic` with O + k·T and the operating-duration limit, `lemma_nominal_is_drift_free`), `Kernel::timer_event`/`periodic_release`/`deadline_miss` (R9.6 to R9.9).
    - _Requirements: 9.1, 9.2, 9.3, 9.4, 9.5, 9.6, 9.7, 9.8, 9.9, 44.2, 44.8_
  - [x] 4.6 Implement and prove sporadic release: intake decisions, MIT state, deferral and discard, Release_Signal raise, MIT_Violation attribution and thresholds, and interrupt masking during MIT windows.
    - Evidence: Verus proofs of Properties 8 and 9
    - Done: `Timing::intake_decision`/`accept`/`defer`/`try_unmask`, `Kernel::intake` with attribution and the MIT_Violation threshold (R10.4).
    - _Requirements: 10.1, 10.2, 10.3, 10.4, 10.5, 10.6, 10.7, 10.9, 19.1_
  - [x] 4.7 Implement and prove Budget accounting and Overrun detection.
    - Evidence: Verus proof of Property 10
    - Done: `budget.rs` (attribution to one Task), `Kernel::budget_event`; R18.5/R18.6 in `Config::partition_ok`; detection "whatever the ceiling" is the DD-02 level argument (hw_model HW-02).
    - _Requirements: 18.1, 18.2, 18.3, 18.5, 18.6, 44.3_
  - [x] 4.8 Implement and prove the event-log ring with PR-identifier sets and the saturating overflow counter.
    - Evidence: Verus refinement proof
    - Done: `log.rs` (`Log::push` refines `Seq` with drop-oldest and a saturating overflow counter).
    - _Requirements: 14.6, 14.7, 44.2_
  - [x] 4.9 Implement and prove the MPU view computation for a Partition layout, including stack placement for overflow detection.
    - Evidence: Verus proof of Property 12
    - Done: `mpu.rs` (`Layout::view` and `lemma_view_within_layout`, `lemma_views_disjoint`, `lemma_stack_overflow_faults`; Armv7-M size and alignment rules checked by `Interval::check_rules`).
    - _Requirements: 16.1, 16.2, 16.3, 16.5, 16.6, 44.4_
  - [x] 4.10 Complete the absence-of-runtime-error proofs over `logic`, confirm that only Verus-supported features appear outside trusted functions, and document the panic-freedom argument under the abort strategy.
    - Evidence: Verus run with no unlisted assumptions; Verification_Plan entry for ORQ-07
    - Done: every exec function in `logic` is verified for overflow, bounds, and panic freedom (Verus checks them for all exec code); no `assume`, `admit`, or `external_body` in the Kernel crates (assumption gate); the ORQ-07 entry is in the Verification_Plan (task 8.1).
    - _Requirements: 44.1, 44.5, 44.7_

- [ ] 5. Kernel_Arch for the Cortex-M4F and the `rsk` API
  - [x] 5.1 Implement `arch::cm4` and the Kernel_Unsafe_Module from the spike: the priority map and PRIGROUP, NONBASETHRDENA, dispatch stubs with per-level records, SVC entry and exit, the two-step return, MPU loads, BASEPRI writes, eager FPCCR and FPDSCR, ACTLR.DISDEFWBUF, and DSBs before exception returns. Implement `rsk-entry` with the vector table and reset entry. Give each `unsafe` block, `unsafe fn`, and `unsafe impl` a justification identifier, and write in this task its Kani harness or Verus proof or lemma where one is feasible, and otherwise its recorded review stating why none is feasible.
    - Evidence: Kani harnesses for this task's justification identifiers, passing in the verification job that task 6.1 sets up; justification gate output (task 1.2); HIL dispatch tests carried over from 3.1
    - Done 2026-10-08: `crates/rsk-kernel/src/arch/{mod.rs,cm4/mod.rs}` (safe planning code: priority map, dispatch with per-level records and Partition stack floors, SVC services, unwind, timer/intake/fault handlers, boot) and `cm4/unsafe_module/{hw.rs,stubs.rs,mod.rs,kani.rs}` (register access, naked stubs, cells, frame builder; UJ-001..054 in `unsafe_justifications.toml`, each linked to a Kani harness or a review record, gate R6.4 passing); `crates/rsk-entry` (vector table, reset entry, panic handler, UJ-101..108). NONBASETHRDENA is no longer used (DD-01 revised). HIL-only items remain under 3.3/7.x.
    - _Requirements: 6.3, 6.4, 6.6, 6.7, 7.7, 7.8, 11.1, 11.2, 11.3, 11.7, 11.8, 45.1_
  - [ ] 5.2 Implement boot validation (target values, clocks, configuration checksum, DWT, debug-state logging) and the system safe state.
    - Evidence: host tests of the boot logic; HIL boot-check suite (task 7.2)
    - Partly done 2026-10-08: `arch::cm4::boot` validates `CONFIG` (`Config::validate`, proven `ok == wf`), runs `hw::boot_checks` (priority bits, PRIGROUP, MPU presence/region count, FPU, DWT) and logs the failed-check mask before the safe state (R12.1, R12.2); the Partition layout is validated by `logic::mpu::Layout::validate`. Open: clock checks and the configuration checksum (need the Generator's `rsk-confcheck` tables, task 13.x), DHCSR.C_DEBUGEN logging (ORQ-30).
    - _Requirements: 12.1, 12.2, 12.3, 12.4, 12.5, 45.1_
  - [ ] 5.3 Implement the fault handlers with attribution, the panic handler, HardFault handling, and the Kernel-owned watchdog.
    - Evidence: HIL fault-injection case per fault class; Kani for the handler glue
    - Partly done 2026-10-08: MemManage/BusFault/UsageFault entries with Kernel-vs-Partition attribution by EXC_RETURN bit 3 and PRIMASK (R14.3), the HardFault and NMI paths into the safe state (R14.4), the single panic handler with the unprivileged panic SVC (R14.1, R14.2, PR-19), and the response application (`Kernel::fault`/`apply_response`, proven). Open: the Kernel-owned watchdog (ORQ-31, nRF52840 WDT; no QEMU model) and the HIL fault-injection cases (7.5).
    - _Requirements: 11.5, 14.1, 14.2, 14.3, 14.4, 16.4, 45.1, 45.2_
  - [ ] 5.4 Implement the peripheral intake handlers, source masking during MIT windows, and disabling of unbound sources.
    - Evidence: HIL storm test (task 7.5); Kani for the handler glue
    - Partly done 2026-10-08: `Binding::Intake` in `arch::cm4::irq_entry` (timestamp, `Kernel::intake` with MIT enforcement, source disable for the MIT window via `apply`'s `source_masked` re-enable, R10.9/R19.2/R19.4) and `Binding::Unbound` (disable and `EV_UNBOUND_IRQ`, R19.5). No Sporadic_Task exists in the P1 demo yet; a sporadic demo and the storm test are open (7.5).
    - _Requirements: 10.9, 19.2, 19.4, 19.5, 45.1, 45.2_
  - [ ] 5.5 Implement the EasyDMA mediation service (validated pointer and length writes, list-mode rejection, stop on Partition stop or restart) and the Kernel-owned PPI configuration.
    - Evidence: Kani on the validation; HIL DMA cases (task 7.4)
    - _Requirements: 17.1, 17.2, 17.3, 17.5, 17.6, 45.1, 45.2_
  - [ ] 5.6 Implement the `rsk` API: Context types, `Shared` with `Mutex`, `Signal`, `now()`, and `ResourceCell` with its justified `unsafe impl Sync`.
    - Evidence: trybuild compile-fail cases for re-entry, escaping references, undeclared Resources, and async Task bodies; Kani for the SVC glue
    - Partly done 2026-10-08: `crates/rsk` (`Shared::lock` with `&mut self` and a closure over `&mut T`, `now()`, `debug()`, `spin()`; `ResourceCell` with its justified `unsafe impl Sync`, UJ-050..054), used by the P1 demo. Open: generated Context types and `Signal` (need rsk-gen, task 10.5), trybuild cases (task 10.6).
    - _Requirements: 8.6, 8.7, 8.8, 8.10, 45.1, 45.2_

- [ ] 6. Kani harnesses
  - [x] 6.1 Set up Kani before the first Kernel_Unsafe_Module code (task 5.1): pin Kani and CBMC, run every harness in the verification job, count a run as passing only when every check including every unwinding assertion passes, and fail the job when a harness that a justification identifier links to fails. Build the stub library that replaces inline assembly and memory-mapped register accesses with Hardware_Model steps, and the convention for stating and justifying harness bounds. The harness for each justification identifier of an `unsafe` block or `unsafe fn` is written by the task that adds the identifier.
    - Evidence: Kani run in the verification job on sample harnesses that exercise the stubs, with the Kani and CBMC versions recorded in its Evidence_Item; a fixture harness with a failing unwinding assertion that the job rejects; the stub list for the Trust_Base_Register
    - Done 2026-10-08: Kani 0.68.0 / CBMC 6.11.0 pinned (`toolchain/decision.toml`, CI installs with `--locked`); `cargo xtask verify` step 10 runs every Kernel crate with harnesses and passes only with every check (unwinding assertions included), recording versions in Evidence_Item `proof-kernel-kani`; the arch harnesses in `cm4/unsafe_module/kani.rs` exercise the MMIO address table, the frame builder and the BASEPRI/MPU encoders against the `sim` register double (the stub library, listed in `docs/kani_stubs.toml`, TBR-07). The failing-unwinding fixture is `xtask/fixtures/kani-unwind`: its captured output is checked by `proofs::tests` and the fixture runs live in `xtask/tests/fixtures.rs` when Kani is installed (`proofs::kani_verdict` is the R45.5 rule).
    - _Requirements: 45.4, 45.5, 45.7, 45.8_
  - [x] 6.2 Write cross-check harnesses that run each Kernel operation on bounded nondeterministic states and assert its Verus postcondition.
    - Evidence: Kani run
    - Done: `crates/rsk-kernel/src/logic/kani_harness.rs` (5 harnesses: validator totality, lock/unlock ceiling, forced end and LIFO resume, selection order) at bounds NT=3, NR=2, NL=4; all pass under Kani 0.68.0 / CBMC 6.11.0; run by `cargo xtask verify` step 10.
    - _Requirements: 45.3_
  - [ ] 6.3 Record Kani's limitations and their mitigations in the Verification_Plan.
    - Evidence: Verification_Plan section; review record
    - _Requirements: 45.6_

- [ ] 7. HIL_Rig and Phase 1 hardware tests
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
    - _Requirements: 43.1, 43.2, 43.3, 43.4, 43.5, 46.1, 46.2, 51.1, 51.2, 51.3, 51.4, 51.5, 51.6, 51.7, 52.1, 52.2, 52.3, 52.4, 52.5, 58.3_
  - [ ] 8.2 Write Trust_Base_Register v1 and Gap_Register v1 from the design's trust-base analysis, and wire assumption gating into the verification job.
    - Evidence: documents; failing fixture with an unlisted `external_body`
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
  - [ ] 12.1 Implement ELF reading, Thumb-2 decoding of SP and control-flow effects, the call graph, recursion detection, and indirect-call resolution.
    - Evidence: fixture binaries; differential run against `cargo-call-stack`
    - _Requirements: 3.3, 37.3_
  - [ ] 12.2 Implement per-region stack bounds using the preemption relation and FP storage, and export them for the Config_Checker.
    - Evidence: fixtures with known stack depths; stack painting on the conformance Application over the HIL_Rig
    - _Requirements: 11.4, 37.1, 37.2, 37.4_
  - [ ] 12.3 Implement the instruction-class scans, the allocator, unwinding, and `vstd` symbol checks, the Core_Subset symbol mapping, the dependency-graph checks, and the no-panic-path check from Kernel entry points. The instruction scans also serve as the defence-in-depth check of R16.10, although DD-01 enforces every access class in hardware.
    - Evidence: fixtures per check
    - _Requirements: 6.6, 14.1, 16.10, 55.4, 57.3_

- [ ] 13. Config_Checker
  - [ ] 13.1 Implement the independent parser, recomputation of the R26.2 values, extraction of `CONFIG` from the ELF, and the checksum checks.
    - Evidence: Property 15 for the Config_Checker parser; comparison tests on the conformance Application
    - _Requirements: 16.7, 25.5, 26.1, 26.2, 26.3, 26.4, 26.5_
  - [ ] 13.2 Build the mutation suite of altered binaries and Task_Models.
    - Evidence: Property 19
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
  - [ ]* 17.3 Evaluate Aeneas on one pure control-law function as a Lean-level alternative for small computations.
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
  - [ ]* 25.3 Evaluate a privileged fast path for the highest Criticality_Level (ORQ-01 option c).
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
    { "id": 6, "tasks": ["5.2", "5.3", "5.4", "5.5", "5.6"] },
    { "id": 7, "tasks": ["6.2", "6.3", "7.2", "7.3", "7.4", "7.5"] },
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

- Tasks marked `*` (17.3 and 25.3) are optional evaluations; no requirement depends on their outcome.
- Phase 1 runs on hand-written `Config` tables. Task 14.1 re-runs the Phase 1 suites on Generator output so that both paths are covered.
- If the DD-01 spike (3.1) fails, tasks 4.2, 5.1, and 6.1 switch to the fallback of ORQ-01 option (b). The logic proofs stay; only Kernel_Arch and its proofs change.
- Values still to confirm in Phase 1: PAR-01 to PAR-08, the WCET margin and coverage thresholds, and the Kani bounds. Facts that design.md flags as unconfirmed (NONBASETHRDENA semantics, its availability on Armv8-M, and the applicability of erratum 838869 to r0p1) are checked in tasks 3.1, 3.4, and 25.1.
- R47.4 applies only once the partitioned multi-core extension exists, which is outside this plan (NG-01), so no task references it.
- Each evidence item becomes an Evidence_Item with tool versions and the Build_Manifest identity (R50.3). Task 8.4's matrix tracks which acceptance criteria still lack evidence.
- Every task that adds an `unsafe` block, `unsafe fn`, or `unsafe impl` to the Kernel_Unsafe_Module (5.1 to 5.6, 21.2, 25.1, 25.2) gives it a justification identifier and adds, in the same task, its Kani harness or Verus proof or lemma where one is feasible, and otherwise a recorded review that states why. Kani is set up first (6.1), so the gate of task 1.2 passes after every task.
- The DD-01 spike (3.1) is built in its own Cargo workspace outside the rsk workspace, as the xtask fixtures are, so the unsafe-code gates of tasks 1.1 and 1.2 do not apply to it. Task 5.1 moves its code into the Kernel_Unsafe_Module under those gates.
