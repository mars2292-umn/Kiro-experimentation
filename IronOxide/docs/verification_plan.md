# rsk Verification_Plan v1 (draft)

| | |
|---|---|
| Version | 1, draft, 2026-10-14; evidence and sections 4, 5.9, 6.3, 9.2 updated 2026-10-22; Link_Checker scans (task 12.3) 2026-10-23 |
| Implements | requirements.md component G; tasks 8.1 and 6.3; R43, R45.4, R45.6, R45.8, R46.1, R46.2, R51, R52, R58.3; the HIL frequency of R50.1 (section 11, task 8.2) |
| Certification basis | DO-178C DAL A/B, DO-330 tool qualification, DO-333 formal-methods credit (fixed input, confirmed 2026-10-14). rsk is a research prototype that keeps this path open; it produces no applicant plans (NG-12), and activities only an applicant can perform are marked as such (R54.4). |
| Review status | Not reviewed (section 13). |
| Hardware status | No nRF52840-DK is available. Every HIL item stays open and required; section 11 tracks them. |

Requirement identifiers (Rn.m, PR-nn, ASM-nn, ORQ-nn, RN-nn) point into requirements.md. TBR-nn, GAP-nn, and KS-nn point into the Trust_Base_Register `docs/trust_base_register.toml`, the Gap_Register `docs/gap_register.toml`, and `docs/kani_stubs.toml`. Both registers cite the P-nn rows of section 3, and `cargo xtask check` fails when a cited row, requirement, ASM, ORQ, or gap does not exist (R48, R54). "Proof" means Verus (all reachable states) or Kani (all inputs within stated bounds); "test" means testing evidence (R43.4); "argument" means a pen-and-paper argument recorded in section 5, which is review evidence and earns no DO-333 credit until mechanized.

## 1. Evidence at this revision

From the local `cargo xtask verify` run of 2026-10-22 (evidence store `target/evidence-out`, 15 Evidence_Items, verdict pass; the run of 2026-10-14 had 230 Verus items, 11 Kani harnesses, and 2,510 executable Kernel lines):

| Evidence_Item | Result |
|---|---|
| `proof-kernel-verus` | rsk-logic 278 items verified, 0 errors (the ghost history of task 4.11, the theorems of tasks 4.12 to 4.14, and `dma_permitted_exec` of task 5.5 included); rsk-kernel 137 items verified, 0 errors (`arch::bound` with `layout_crc`, `arch::cm4::plan`, `arch::cm4::entry` with the boot checks, watchdog, intake masking, EasyDMA service, and Release_Signal raise of tasks 5.2 to 5.6 and 13.2); rsk-analyzer 17 verified, 0 errors; Verus 0.2026.10.04.426d8b0, Z3 4.16.0 |
| `check-static-checks` | 5 assumption constructs in the verified crates, all listed (the five trusted functions of `Cm4Hw`, R50.2); 47 `unsafe` constructs, 5 linked to Kani, 42 to review (R6.4); 3,285 executable Kernel lines (PAR-01: rsk-logic 1,316, rsk-kernel 1,880, rsk-entry 89; 49 of them the ghost-history writes of tasks 4.11 to 4.14, which emit no code); 23 proof mutations registered (R52.2); Trust_Base_Register 24 entries with 21 of 21 R48.1 items mapped (R48); Gap_Register 71 Annex A objectives, 19 gaps, 32 ORQs matching design.md (R54) |
| `analysis-stack-p1-demo` | Link_Checker (demo with three Partitions since task 14): 74 functions bounded, 4 stack regions within their worst case (part0 2,252 of 4,096; part1 640 of 4,096; part2 704 of 4,096; MSP 3,532 of 29,936); export `stack-bounds-p1-demo.json` consumed by the Config_Checker (tasks 12.1, 12.2). Scans (task 12.3; the `conf-PR-19-lk`, `conf-PR-21-lk`, `conf-PR-22-lk`, `conf-PR-31-lk`, `conf-PR-35-lk` evidence of the Profile is this item's log): 57 Kernel functions with no FP arithmetic or FPSCR write, 13 naked stubs with no cross-section `cbz` and no `nop` (ORQ-32), 25 Task-reachable functions with no `msr`/`mrs`/`cps`, no PPB address, and no direct Kernel call; one panic handler, no allocator, unwinding, `vstd`, or disallowed-crate symbol; 17 `core`/`compiler_builtins` symbols mapped to Core_Subset items, the `core::fmt` ones unreachable; no explicit Kernel panic, two implicit panic kinds (bounds check through 21 Kernel functions, remainder by zero through 2) |
| `check-config-p1-demo`, `test-config-mutations-p1-demo` | Config_Checker: 202 values compared, no discrepancy (the stack regions against the Link_Checker included); 16 of 16 mutants reported (task 13) |
| `proof-kernel-kani` | 13 harnesses, every check passed (7 in rsk-logic, the `intake` cross-check and the DMA buffer check among them; 6 in rsk-kernel); Kani 0.68.0, CBMC 6.11.0 |
| `test-qemu-spike-p1-demo` | `RSK-OK` on QEMU mps2-an386 (not the Target) |
| `check-profile-lint`, `check-profile-clippy` | Profile_Lint (task 11.1): 33 files, 66 loops bounded (5 annotated, 10 derived from constants, 47 by `decreases`, 4 non-terminating by design), no PR-05, PR-09, PR-16, PR-17, PR-18, PR-21, or R3.4 finding; export `loop-bounds.json` (R35.3); Clippy deny-lists (PR-10, PR-11, PR-22) clean on the Target crates. These items are the `conf-PR-xx-ln` evidence of the Profile for PR-05, PR-09, PR-10, PR-11, PR-16, PR-17, PR-18, PR-21, PR-22 |
| `conf-PR-nn-cc`, `fi-PR-nn`, `accepted-ravenscar` | Profile_Conformance_Suite (task 14.1): 66 cases, 37 provided and passing (10 MC, 4 TS, 7 LN, 11 LK, 4 RT on QEMU with the restriction bit in the Health_Monitor log, 1 accepted Application), 29 declared with a reason; 10 PO proof artifacts mapped to present Verus items (R3.8, R3.10) |
| `test-workspace-tests`, builds, sandbox, `check-evidence` | pass |

CI has the same entry point but does not install QEMU, so a CI run carries no QEMU item. `cargo xtask mutations` (section 9.2) is run separately: 23 of 23 mutants killed on 2026-10-23 (`dma-buffer-in-stack` added with task 5.5).

## 2. What the type system gives (R43.3)

Relied on, given sound `unsafe` code and a correct compiler (ASM-14, TBR-21):

- data-race freedom in safe code through `Send` and `Sync`;
- closure-scoped LIFO locking with no escaping references (R8.6), no re-entry (R8.10), and Resource access only by declared Tasks (R8.7), through the `rsk` API shape;
- synchronous Task bodies (PR-09) and, from Phase 4, Endpoint protocol states within a Job (R38.4).

Not given, and allocated in section 3: deadlock freedom, bounded blocking, timing, schedulability, panic freedom, stack bounds, and freedom from leaks.

## 3. Property allocation (R43.1, R43.2, R43.4)

Status is as of 2026-10-14. "By inspection" points to the Kernel_Arch facts of section 4, which the proofs assume and no tool checks.

| # | Property | Criteria | Tool (kind) | Evidence_Item | Phase | Status | Depends on |
|---|---|---|---|---|---|---|---|
| P-01 | Data-race freedom | PR-05, R8.6, R8.10 | Type system; Verus for mutual exclusion (P-02); Profile_Conformance_Suite (`conf-PR-05-ts`: undeclared Resource E0609; `conf-PR-05-ln`: `static mut` and interior-mutable statics) | builds; `proof-kernel-verus`; `conf-PR-05-ts`, `conf-PR-05-ln` (task 14.1) | P1, P2 | Type rules in force; the PR-05 cases pass (2026-10-23) | ASM-14, TBR-21, TBR-06 (UJ-050..054) |
| P-02 | Mutual exclusion | R8.3 | Verus (proof): `lemma_no_other_holder`, `Sched::lock`, `Kernel::svc_lock` | `proof-kernel-verus` | P1 | Proven for the state model; BASEPRI realisation by inspection | ASM-08, ASM-15, TBR-02, TBR-04 |
| P-03 | Ceiling raise and restore | R8.1, R8.2, R8.6 | Verus (proof per step, and `lemma_ceiling_restored` over the ghost history for restore across preemptions, task 4.13); compile-fail fixtures `xtask/fixtures/api-misuse` (test: re-entry E0499, escaping `&mut T`, undeclared Resource E0609, `async` body E0308; task 5.6) | `proof-kernel-verus`; `test-workspace` (`api_misuse_is_rejected_by_the_type_system`) | P1, P2 | Each lock and unlock proven; restore across preemptions proven over the history (argument 5.3 mechanized 2026-10-22); the type-system rules of R8.6, R8.7, R8.10, PR-06, PR-07, PR-09 checked by compile-fail cases (2026-10-23) | ASM-15, TBR-04 |
| P-04 | SRP start condition | R7.1, R7.2, R7.3, R20.3 | Verus over the Hardware_Model (proof): `Sched::select`, `lemma_dispatch_takes_iff_eligible`, `lemma_dispatch_selects_srp_choice`, `lemma_dispatch_idle_when_none_eligible`; `arch::bound` (`eligible`, `select`, `basepri_now`) | `proof-kernel-verus`; `test-qemu-spike-p1-demo`; `hil-dt-hw-05` (planned) | P1 | Proven; the BASEPRI value and the stale-bit test are proven computations (section 4), their application to the registers (A1, A4), A2 and A3 by inspection; hardware-pending | ASM-03, ASM-08, TBR-02, TBR-04, TBR-05 |
| P-05 | Bounded blocking | R8.4, R29.2 | Verus: `lemma_blocked_by_one_holder`, `lemma_one_low_blocker`, `lemma_low_locker_nested` (state), `lemma_no_new_blocking_section`, `lemma_no_lower_dispatch_while_waiting` (over the ghost history; task 4.12); PBT against the SRP simulator (test, P2) | `proof-kernel-verus` | P1, P2 | One section per Release proven under the A4 promptness premise (argument 5.1 mechanized 2026-10-23); the Kernel overhead term is WCET (P3) | ASM-15, TBR-04 |
| P-06 | Deadlock freedom | R8.5 | Verus: `lemma_no_other_holder` (a lock never waits), `lemma_deadlock_free` (every wait-for path has length one; task 4.14) | `proof-kernel-verus` | P1 | Proven for every state of a run (argument 5.5 mechanized 2026-10-22); that the wait ends is P-17 | ASM-15 |
| P-07 | Drift-free periodic release, release jitter | R9.1, R9.3, R9.5, R9.10, R44.8 | Verus (proof): `Timing::inv`, `advance_periodic`, `Kernel::start`; HIL release timing (test) | `proof-kernel-verus`; `hil-release-timing` (planned) | P1 | O + k·T proven; the link from armed slots to the Releases `timer_event` makes is not stated; jitter J_rel hardware-pending | ASM-16, ASM-18, ASM-19, TBR-15 |
| P-08 | MIT enforcement | R9.7, R9.8, R10.1 to R10.4, R10.7, R10.9, R19.1, R19.2 | Verus: `Kernel::inv` deferral clause, `Kernel::intake`, `violation_applied`, `mit_spaced`, and `lemma_mit_spacing_over_run` over the ghost history (task 4.14); HIL storm (test) | `proof-kernel-verus`; `hil-storm` (planned) | P1 | State parts and spacing over a run proven (argument 5.4 mechanized 2026-10-22); the interrupt-source time bound (Property 9) is not proven; Kani cross-check `intake_outcomes_match_r10` (R10.1 to R10.3, R10.7) | ASM-10 |
| P-09 | Kernel queue and Endpoint buffer correctness | R9.4, R14.6, R14.7, R39.4, R44.2 | Verus (refinement proof): `Slots`, `Log::push` | `proof-kernel-verus` | P1, P4 | Slots and log proven; Endpoints are Phase 4 | ASM-15 |
| P-10 | LIFO preemption invariants | R7.5, R7.6 | Verus (stack, ceiling): `Sched::inv`, `complete`, `end_job`, `Kernel::job_return`; review and QEMU for the per-level register record | `proof-kernel-verus`; `test-qemu-spike-p1-demo`; `hil-register-pattern` (planned) | P1 | Stack and ceiling proven; register restore by the assembly stubs is review-only (UJ-020, UJ-022, UJ-023) | TBR-05, TBR-06 |
| P-11 | Floating-point context preservation | R11.1 to R11.3, R6.6 | Test (QEMU, HIL) and review | `test-qemu-spike-p1-demo` (since 2026-10-23 the demo's `filter` Task, declared `fpu`, keeps an `f32` state across 8 preempted Jobs and the safe state checks its convergence); `hil-register-pattern` (planned) | P1 | Testing evidence only (3.1, and the FP state check of task 14); Kernel VFP data moves recorded in 5.7 | ASM-08, TBR-02, TBR-03 |
| P-12 | Absence of runtime errors in the Kernel | R44.1, R44.7, PR-19 | Verus (logic, proof); Link_Checker no-panic-path check (P2); review (arch) | `proof-kernel-verus`; `conf-PR-19-lk` (the Link_Checker log, in `analysis-stack-<binary>`) | P1, P2 | Logic proven free of overflow, bounds errors, and panics; `arch` and `rsk-entry` are not proven (5.6); the Link_Checker (task 12.3) finds no explicit panic in Kernel code and reports the compiler's implicit checks that remain reachable from the Kernel entry points (bounds checks through 21 Kernel functions, remainder by zero through 2) | ASM-09, TBR-01, TBR-04 |
| P-13 | Absence of UB in the Kernel_Unsafe_Module | R6.4, R45.1, R45.2 | Kani (bounded proof) for 5 identifiers; review for 38 | `proof-kernel-kani`; `check-static-checks` (R6.4 gate) | P1 | Kani covers address arithmetic, frame layout, and encoders, not the volatile and assembly paths (6.1) | TBR-05, TBR-06, TBR-07 |
| P-14 | Boot-time configuration validation | R12.1 to R12.3, R12.6 | Verus: `Config::validate` (ok == wf), `Layout::validate` (ok ⟹ wf); HIL boot suite (test) | `proof-kernel-verus`; `hil-boot-checks` (planned) | P1 | Table validation proven; hardware checks implemented; clock and checksum checks open (task 5.2); hardware-pending | ASM-01, ASM-02, ASM-03, ASM-05, ASM-06 |
| P-15 | Configuration consistency | R24.4, R25, R26 | Generator drift check (check); Config_Checker `rsk-confcheck` on every Flight_Build (check; task 13.1, 2026-10-23: its own Task_Model parser, the R26.2 recomputation, `CONFIG` extracted from the binary, CRC-32 checksum checks); mutation suite (test, task 13.2) | `check-static-checks` (`gen --check`); `check-config-<binary>` (`cargo xtask verify` step 8b) | P2 | Drift check, Config_Checker, and mutation suite in place (197 values compared on the P1 demo: every class of R26.2 against the Task_Model, the `CONFIG` and `LAYOUT` images and their checksums against the binary; 16 of 16 mutants of `docs/config_mutations.toml` reported, task 13.2); the stack sizes await the Link_Checker | TBR-11, TBR-12 |
| P-16 | Spatial isolation: MPU and DMA | R16.1 to R16.6, R16.8, R17, R20.4, R44.4 | Verus over the Hardware_Model: `lemma_hw_partition_isolation`, `lemma_hw_write_isolation`; Kani field encoding; Config_Checker; HIL access matrix | `proof-kernel-verus`; `proof-kernel-kani`; `hil-mpu-matrix`, `hil-dma` (planned) | P1, P2 | MPU decision proven for every validated layout; RBAR and RASR encoding at field level only; EasyDMA mediation built (task 5.5, 2026-10-23: `dma_request_core` and `dma_permitted_exec` proven, Kani cross-check), its hardware effect pending `hil-dma` | ASM-02, ASM-12, TBR-02, TBR-05, TBR-10 |
| P-17 | Temporal isolation: Budget monitor, interrupt sources | R18.1 to R18.3, R18.7, R18.8, R19.1 to R19.3 | Verus: attribution in `Kernel::dispatch` and `job_return`, `lemma_kernel_level_*`; HIL Overrun and storm (test) | `proof-kernel-verus`; `hil-overrun`, `hil-storm` (planned) | P1, P3 | Attribution and the never-masked level proven; Δ_detect, Δ_enforce, and the interrupt bound hardware-pending | ASM-06, ASM-10, TBR-02 |
| P-18 | Health_Monitor responses | R14.3 to R14.5, R14.9 to R14.11, R18.4 | Verus: `response_applied`, `others_unchanged`, `lemma_others_ceilings_in_force`; HIL response suite (test) | `proof-kernel-verus`; `hil-response` (planned) | P1, P3 | Effect on the faulting Partition and R14.9 proven per response; fault attribution (R14.3) by inspection; R14.10 bound measured in P3 | ASM-08 |
| P-19 | Schedulability | R29, R32 | Analyzer: Verus fixed-point core (proof), PBT and reference implementation (test), UPPAAL (cross-check) | `proof-kernel-verus` (rsk-analyzer, 17 items) | P2 | Core proven; term derivation unproven; PBT, simulator, and published task sets not built; UPPAAL ceiling not modelled | ASM-11, ASM-13, TBR-12, TBR-17 |
| P-20 | WCET | R34 to R36 | Measurement (test), static analysis (P5) | `wcet-*` (planned) | P3, P5 | Not started; testing evidence only before P5 | ASM-11, TBR-14 |
| P-21 | Stack bounds | R37, R16.5 | Link_Checker `rsk-link` (static analysis of the linked binary: Thumb-2 SP effects, call graph, PR-17 recursion, PR-18 indirect-call resolution, per-region bounds with the preemption relation and FP frames; tasks 12.1, 12.2, 2026-10-23); MPU overflow backstop (Verus) | `proof-kernel-verus` (`lemma_stack_overflow_faults`, hardware form in P-16); `analysis-stack-<binary>` (`cargo xtask verify` step 8a, export consumed by the Config_Checker) | P1, P2 | Backstop proven; every function and region of the P1 demo bounded (part0 2092 of 4096, part1 640 of 4096, MSP 2692 of 30440); the frame model is the sum of SP decrements per function (a sound upper bound for compiler-generated prologues; dynamic SP fails the build); the panic path is charged once per entry point (its callers are listed by the no-panic-path report of task 12.3); the differential run against `cargo-call-stack` is not possible offline | TBR-12 |
| P-22 | Protocol compliance and communication deadlock freedom | R38 to R41 | Type system; Protocol_Checker; proof of R41.3 | (planned) | P4 | Not started | TBR-12 |
| P-23 | Application absence of runtime errors | R46.3, R46.4, PR-19 | Creusot (proof) | `proof-app-creusot` (planned) | P2 | Not started | TBR-08 |
| P-24 | Host-side concurrency | R47 | Loom, Shuttle (test) | (planned) | P2 | Not started | none |
| P-25 | Hardware_Model fidelity | R49.1 to R49.6 | Citations (review); HIL differential tests (test) | `hil-dt-hw-01` to `-14` (planned) | P1 | Rules cited; differential tests hardware-pending | ASM-08, TBR-02, TBR-03 |

### Testing-only evidence (R43.4)

| Row | Why no proof tool applies |
|---|---|
| P-11 | Preservation is done by hardware stacking (HW-10) and assembly stubs. Verus has no assembly and Kani does not model it (RN-16). |
| P-07 (jitter), P-17 (Δ_detect, Δ_enforce), P-14 (hardware boot checks) | Physical timing and silicon behaviour, which a source-level proof cannot observe. |
| P-20 | No static WCET tool before P5 (ORQ-12); measurement is not a bound (ASM-11). |
| P-24 | Loom and Shuttle are explicitly not sound (RN-37). |
| P-25 | A model cannot validate itself against silicon. |

## 4. What holds by inspection (the Kernel_Arch boundary)

Since 2026-10-22 (task 5.7) every Kernel_Arch entry point is a verified core in `arch/cm4/entry.rs`, generic over `KernelOps` (the bound Kernel, `arch/bound.rs`), `KernelArch` (the hardware, driven through `Action`s whose specification is a step of the ghost `HwModel`, the Kernel's record of what it wrote), and `Tables` (the generated system crate's inputs). Outside `verus!` remain: the shells in `arch/cm4/mod.rs` (fetch the Kernel, the hardware, the tables, and the Kernel_Arch state; call the core; enter `System::safe_state`, which never returns), the `System` trait and its `Tables` adapter, the Kernel_Unsafe_Module (register access, assembly stubs), and rsk-entry. The register-facing surface of the planning code is the `Cm4Hw` implementation of `KernelArch`: five trusted functions (`apply`, `now`, `cycles`, `boot_checks`, `boot_info`; `#[verifier::external_body]`, listed in `docs/proof_assumptions.toml`), of which `apply` calls exactly one register-access function per action.

| # | Fact | Assumed by | Status |
|---|---|---|---|
| A1 | BASEPRI = `basepri_for(top, ceiling)` after every transition | P-04, P-17 | **Proven at the model** (`basepri_matches`): `apply_core`, the dispatch path of `irq_entry_core`, `svc_core` (lock, unlock), `timer_core`, `epilogue_core`, and `fault_core` end with the model's BASEPRI equal to `hw_model::basepri_for(job_level, ceiling)`, or 0 in the safe state. Trusted: `Cm4Hw::apply(SetBasepri(v))` writes `v` to BASEPRI (`hw::write_basepri`, UJ-010) |
| A2 | PRIMASK is set only by Kernel stubs and `start` and is clear on every return to Thread mode | P-04, P-17 | `start_core` proven to clear PRIMASK (model) unless the safe state is entered; the assembly stubs by inspection (`stubs.rs`, UJ-011, UJ-012) |
| A3 | Every dispatch vector has NVIC priority `encode_level(priority)`; every Kernel exception has priority 0 | P-04, P-17 | `boot_core` proven to program each bound vector at `encode_level(priority)` (dispatch) or `encode_level(0)` (intake, Kernel) and to apply `KernelExceptionPriorities` (model flag); the IPR and SHPR writes trusted (`Cm4Hw::apply`) |
| A4 | Only the vector `Kernel::select` returns is pended; a stale pending bit is rejected at entry (argument 5.8) | P-04 | Proven: `apply_core` pends exactly `tables.dispatch_vector(t)` for the `t` that `KernelOps::select` returns (an eligible Job), and `irq_entry_core` refuses a Task that `eligible` rejects. That `dispatch_vector(t)` is Task t's vector: the Generator (A12) |
| A5 | FAULTMASK is never set; Jobs run with CONTROL = 0b001 | P-17 | Proven: a dispatch returns `CONTROL_JOB` (0b001) and `EXC_RETURN_THREAD_PSP`; no `Action` touches FAULTMASK |
| A6 | The views are `Layout::view` of a layout that passed `validate`, loaded at dispatch and resume, with the MPU enabled from boot | P-16 | Proven: `Bound::inv` (every view `is_view_of` the validated layout); `irq_entry_core` loads the dispatched Job's Partition view (model `view`) and `epilogue_core` the resumed Job's; `boot_core` enables the MPU (model flag). Trusted: `Cm4Hw::apply(LoadView)` is `hw::mpu_load` |
| A7 | RBAR and RASR words encode the view | P-16 | Kani (field placement) and review: `hw::rasr_encode`, `rbar_encode`, `mpu_load` |
| A8 | The time-base TIMER, dispatch vectors, and system handlers are never disabled after boot | P-17 | By inspection of the cores (now a few lines): `Disable` is applied to intake vectors from `tables.intake_vector`, to the entered vector of an intake or unbound exception, and in `boot_core` before programming. That intake vectors are distinct from dispatch and timer vectors: the Generator |
| A9 | The runtime guards in `Bound` mirror the `requires` clauses of the logic entry points | every Kernel proof | **Proven**: Verus checks every call in `arch/bound.rs` and every call of the cores into `KernelOps` |
| A10 | The Kernel cell is never entered reentrantly (DD-02) | every Kernel proof | By inspection: UJ-003 to UJ-006 (review) |
| A11 | `placeholder_kernel` is replaced by `init` before the first transition | `inv` at the first call | **Proven**: `boot_core` calls `validate` then `init` before any other `KernelOps` method and establishes `inv`; the mutation `boot-skips-init` shows a dropped `init` failing. That rsk-entry calls `boot` before `start` and before enabling exceptions: by inspection (rsk-entry) |

A12, the ascending dispatch-vector order (DD-03, `dispatch_vectors_ok`), is checked by the rsk-gen test `dispatch_irqs_ascend_for_dd03` for every built-in Target, not by inspection.

Earlier in task 5.7 (2026-10-22, `arch/cm4/plan.rs`): the new Job's frame address is proven 8-byte aligned (HW-06), at or above the Partition stack's bottom and wholly below the floor (R16.5), and the highest such address (`plan_frame`; the overflow case raises the Kernel fault instead of writing below the region); the TIMER compare value is proven equal to `hw_model::compare_for` (HW-12, `compare_target`); the unwind's level lookup is proven to return the greatest matching level (`find_level`).

Crate layout for proof stability (2026-10-22): the proof core (`logic`, `hw_model`) is the crate `rsk-logic`, and Kernel_Arch (`arch::bound`, `arch::cm4`, the Kernel_Unsafe_Module) is `rsk-kernel`, which re-exports it. Verus verifies each crate's items in its own SMT context and imports the other's specifications only, so a change on one side cannot destabilize the other's proofs; the `rlimit` raise that `end_top` needed in the single crate was removed after the split and the proof passes at the default limit. The explicit array-fill axiom applications in `Slots::new` and `Layout::view` are kept as cheap robustness.

What the model does not cover, and the evidence that does: the hardware's own response to the written state (the NVIC taking a vector, the MPU decision) is the Hardware_Model's domain (`hw_model` lemmas over the values the cores are proven to write); the stub-level register save and restore (Property 2, review UJ-020 to UJ-023 and the HIL register-pattern test); the five trusted functions (Kani harnesses on the encoders and address table, host tests of the boot checks, HIL differential tests, section 11). Trust_Base_Register entry TBR-24 lists the remaining facts; Gap_Register entry GAP-10 is mitigated for the planning code and stays open for the Kani flight-path restructuring.

## 5. Arguments recorded here

These close gaps that the Verus specifications leave between single transitions. Each is review evidence (TBR-06 style), cites the proven facts it uses, and names what mechanizing it would take. Mechanizing 5.1 to 5.5 is tasks 4.12 to 4.14: the ghost history they need exists since 2026-10-22 (task 4.11, `logic::history`): `Kernel::history` records every dispatch, Job end, lock, unlock, Release due, and kill with its observed instant; `Kernel::inv` carries the trace invariant `history_wf` and every entry point ensures that it only appended to the history. Arguments 5.1 to 5.5 are mechanized (tasks 4.12 to 4.14, below); each entry names what stays outside its theorems.

### 5.1 Bounded blocking: one Critical_Section per Release (R8.4)

Let J be a Job of Priority p released at r, pending but not eligible only because p ≤ System_Ceiling.

1. While J is pending, no Job of Priority below p starts. `Sched::select` takes the highest-Priority pending Release, and if that one is not eligible no pending Release is (proven); a lower-Priority Job is not eligible while p ≤ ceiling.
2. By `lemma_blocked_by_one_holder`, some lock entry with Ceiling ≥ p is held by a Job L of Priority below p, and by step 1 L started before r.
3. At most one such L exists. If L₁ below L₂ on the stack both held Resources with Ceiling ≥ p, then L₂ started above the ceiling of that instant (invariant 1), which is at least the Ceiling of L₁'s Resource (invariant 4), so Priority(L₂) > p, a contradiction.
4. When L's outermost unlock drops the ceiling below p, `apply` pends J's vector at once (A4, P-04) and J preempts L before L executes again, so L cannot lock a second Resource with Ceiling ≥ p in J's way.

So J waits for at most one outermost Critical_Section (with its nested sections) of one lower-Priority Job, plus Kernel overhead. The PBT against the SRP simulator (task 15.4) is the planned test.

**Mechanized 2026-10-23 (task 4.12), in four lemmas.** Every step of the ghost history now records the executing Task after it (`top`); `history_wf` relates consecutive tops by the step's kind (`top_rel`: a dispatch makes its Task the top and, as `Kernel::dispatch` requires, that Task was eligible after the previous step, above the recorded ceiling and the recorded top; a lock or unlock is by the top and keeps it); `Kernel::inv` ties the last recorded top to the scheduler (`top_match`). The premise of the run-level lemmas is that J waits: its Release is step `r`, its dispatch (if any) step `d`, and after no step before `d - 1` is J eligible on the recorded ceiling and top (`eligible_after`); that premise is the Kernel_Arch promptness of A4 (an eligible Job is pended at once, `apply_core`) together with J's Release staying pending, and is the caller's. Step 1: `history::lemma_no_lower_dispatch_while_waiting`, no Job of Priority below p starts in the window. Step 3: `Sched::lemma_one_low_blocker`, at most one Job of Priority below p holds an entry with Ceiling at least p, in every state. Step 4: `history::lemma_no_new_blocking_section`, a lock in the window by a Job of Priority below p finds the System_Ceiling already at least p (so it opens no new blocking section), and `Sched::lemma_low_locker_nested`, in that state the locker itself holds the entry that blocks J (so the lock is nested in its own open section). Step 2 is `lemma_blocked_by_one_holder` as before. Together: the only blocking section is the one open at J's Release, held by one lower-Priority Job, and it is never followed by another before J starts. What remains outside the theorems is the premise above and the Kernel overhead term. The mutations `lock-by-other-than-top` and `dispatch-below-ceiling` fail steps 4 and 1.

### 5.2 Lazy release of killed Jobs delays nothing (R8.9, R14.5)

`Sched::kill` marks a preempted Job K but leaves its lock entries until Kernel_Arch unwinds to it, when `finish_top` releases them. Every Job above K started above the ceiling of its start instant (invariant 1), which covers the Ceilings of K's Resources (invariant 4). A Job X blocked only by K's retained Resources has Priority at most their Ceiling, so below the executing Job's Priority, and is not eligible while any Job above K is in progress. When the last Job above K ends, the unwind reaches K and releases its entries in the same Kernel entry (`epilogue`). The retained entries therefore never delay a Job that the Jobs above K would not delay anyway. Assumes A1 and the unwind order of the stubs (UJ-023, review).

**Mechanized 2026-10-22 (task 4.13):** `Sched::lemma_lazy_release_delays_nothing`, the logic content of the argument as a theorem over any state satisfying `Sched::inv`: with Job K at stack position `k` killed and some Job in progress above it, a pending Task X that the System_Ceiling blocks although no entry of a holder other than K blocks it has Priority below every Job above K, the executing one included, so X is not eligible whether or not K's entries are released. The proof uses the attained maximum of the mirror (`lemma_max_ceiling_attained`), invariant 4 (K's entries have Ceilings at most the start ceiling of each Job above K), and invariant 1. The mutation `lazy-release-no-job-above` (dropping the Job above K) fails it. What stays outside the theorem: that the unwind to K releases the entries in the same Kernel entry, which is `epilogue_core` (Verification_Plan section 4) and the stub order UJ-023.

### 5.3 Ceiling restore across preemptions (R8.2)

`Sched::unlock` proves that after an unlock the ceiling is the maximum over the remaining entries. The remaining entries are exactly those present before the matching lock:

- `lock` pushes at the top and leaves the entries below untouched (proven);
- entries are ordered by holder (invariant 3), so a Job that preempts and later ends removes only its own entries (`complete` and `end_job` leave `lock_res` and `lock_depth` unchanged and shrink `lock_len` only by the top's entries, proven);
- an unlock succeeds only on the top entry owned by the executing Job (proven).

Hence the ceiling after the unlock equals its value before the matching lock.

**Mechanized 2026-10-22 (task 4.13):** `logic::history::lemma_ceiling_restored`. Every step of the ghost history carries a snapshot of the lock stack and the System_Ceiling after it; `history_wf` relates consecutive snapshots by the step's kind (`step_rel`: a lock pushes one entry, an unlock pops the top one, a Job end drops a suffix, the rest keep the stack) and ties each ceiling to its snapshot (`ceiling_of`); `Kernel::inv` ties the last snapshot to the scheduler (`locks_match`), which every entry point maintains and `record` re-establishes from the `Sched` postconditions (`lemma_rel_shrink`, `lemma_rel_lock`, `lemma_rel_unlock`). The theorem: for a lock step `i` and the matching unlock `j` (the first later step at which the stack is back to its depth before `i`, the stack never having gone below the locked entry in between), the stack after `j` is the stack before `i` and the ceiling after `j` equals the ceiling before `i`, whatever other Jobs locked, unlocked, started, or ended in between. The mutation `history-end-forgets-prefix` (a Job end allowed to change entries below the released suffix) fails it. This row of section 3 is now proof evidence at the logic level.

### 5.4 MIT spacing over a run (R10.1, R10.2)

Every accepted Release of a Sporadic_Task sets `last_due` to its own due instant (`accepted`, `deferred_release`, proven). Acceptance requires `now ≥ last_due + MIT` and a deferred Release becomes due at exactly `last_due + MIT` (proven, `Kernel::inv`). Every entry point ensures `mit_spaced`: `last_due` is never forgotten and moves only by at least one MIT (proven per entry point; restarts keep it, R14.5). By induction over the sequence of entry points, consecutive accepted Releases become due at least one MIT apart. `lemma_mit_spaced_trans` is the inductive step for one composition.

**Mechanized 2026-10-22 (task 4.14):** `logic::history::lemma_mit_spacing_over_run`. The trace invariant `history_wf` now states `releases_spaced`: any two Release steps of one Sporadic_Task in the history are due at least one MIT apart. `Kernel::inv` ties the history to the MIT state (`releases_linked`: for a Sporadic_Task, every recorded Release is due no later than `Timing::last_due`, which, once set, is the due instant of a recorded Release); `record` re-establishes both for a Release step from the acceptance rule of `Timing::accept` (`now >= last_due + MIT`) or the deferral clause of `inv` (`deferred_until == last_due + MIT`), through `lemma_release_spaced` and `lemma_release_match_kept`. The theorem reads the invariant back for any two Release steps `i < j` of a Sporadic_Task: `h[j].arg >= h[i].arg + MIT`, so fewer than two Releases of the Task become due in any window shorter than the MIT. The induction the text describes is the one `record` performs at every step. The mutation `mit-run-spacing-dropped` (the invariant weakened to ordering) fails the theorem.

### 5.5 Deadlock freedom (R8.5)

A lock never waits: its outcome depends only on the accessor check, re-entry, and capacity, never on another holder (`lemma_no_other_holder`, proven). No Job suspends inside a Critical_Section (PR-08, PR-09). There is therefore no wait-for edge between Jobs; a pending Job waits only for higher-Priority Jobs to end, which Budget enforcement bounds (P-17).

**Mechanized 2026-10-22 (task 4.14):** `Sched::lemma_deadlock_free`, for every state satisfying `Sched::inv` (so for every state of a run). The waiting relation of the Kernel is `blocked_by(t, i)`: the pending Job of Task `t`, with no Job of `t` in progress and not eligible, is kept from starting by the Job at stack position `i`, which is the executing Job with Priority at least `t`'s or holds a Resource whose Ceiling is at least `t`'s Priority. The theorem states (1) a lock never waits for another Job (`lemma_no_other_holder`, restated), (2) every pending non-eligible Job either waits for its Task's own Job in progress (R9.7) or has a `blocked_by` blocker (the executing Job, or the holder of the entry that attains the System_Ceiling, by `lemma_max_ceiling_attained`), and (3) a blocker is in progress and is blocked by nothing, so every wait-for path has length one and no cycle exists. Not part of the theorem: that waiting ends, which is Budget enforcement (P-17) and the Jobs above ending (`epilogue_core`). The mutation `deadlock-equal-priority-unblocked` (a blocker must have strictly higher Priority) fails clause (2) for a pending Job of the executing Job's Priority. Note the correction to the text above: the blocked Job's Priority may exceed the holder's (bounded inversion); the theorem states the blocker by Ceiling, not by Priority.

### 5.6 Panic freedom under abort (ORQ-07)

- `logic` and `hw_model`: Verus checks every arithmetic operation and index of executable code in range, with no assumption construct. RN-13 notes that Verus has no general mechanism for specifying aborts; the claim is panic freedom, not abort behaviour.
- `arch`, `arch/cm4`, and `rsk-entry`: safe Rust outside `verus!`, with ordinary indexing and arithmetic. It is not proven panic-free. A panic there reaches the single panic handler and the system safe state (R14.4), so it is a detected Kernel fault, not undefined behaviour.
- The Link_Checker's no-panic-path check (task 12.3, 2026-10-23) shows that no Kernel code calls `panic_fmt` or `panic` explicitly (a violation if it did, PR-19) and lists the implicit checks the compiler kept: `panic_bounds_check` reachable through 21 Kernel functions and `panic_const_rem_by_zero` through 2. The Build_System passes the Link_Checker the module paths whose files hold a `verus!` block (`--verified`: `rsk_logic::*`, `rsk_kernel::arch::bound`, `rsk_kernel::arch::cm4::{entry, plan}`), and the checker classifies each caller: 14 of the 21 bounds-check callers and both remainder callers are inside those modules, where the Kernel_Proofs show the index is in range and the divisor nonzero (R44.7), so the compiler's check is unreachable in fact; the other 7 are the shells and stub halves (`Reset`, `rsk_svc_rust`, `rsk_irq_entry_rust`, `rsk_epilogue_rust`, `arch::cm4::fault`) and the demo's safe-state reporter (`print_kv`, `report_and_exit`, Application code that the Kernel's safe state calls), which inline the verified cores; the binary carries no debug information, so a check inside them cannot be attributed to its source. The checks cannot be removed at source level without `unsafe` indexing (PR-20): Verus proves the index in range but the compiler does not use that fact. They remain detected Kernel faults (R14.4), and the stack bound charges the panic path once per entry point.

ORQ-07 stays partly resolved: the explicit-panic half is checked on every Flight_Build; the implicit half is enumerated and classified by proof coverage.

### 5.7 Kernel VFP data moves and R6.6

The DD-01 spike found that LLVM emits `vldr`, `vstr`, and `vmov` for 64-bit data moves in Kernel code on `thumbv7em-none-eabihf`, so the coprocessor stays enabled (hw.rs, ORQ-16 finding). R6.6 as written allows only the context-preservation instructions. These moves do not change a Job's FP context: with eager stacking, entry from a context with an active FP state pushes S0 to S15 and FPSCR before Kernel code runs; S16 to S31 are callee-saved under the AAPCS and are also pushed by the stubs; data moves do not write FPSCR. Resolved 2026-10-22: R6.6 (requirements.md Draft 0.3) and PR-31 (Profile 0.4) were amended to allow floating-point data-movement instructions in Kernel code and to forbid arithmetic, comparison, and conversion instructions and FPSCR writes; GAP-17 is closed. Evidence: the Link_Checker instruction scan (task 12.3, 2026-10-23) classifies every VFP instruction in Kernel code and rejects arithmetic, comparison, conversion, and FPSCR writes; on the P1 demo the only VFP instructions in the Kernel are the `vpush`/`vpop` of the stubs (the compiler emitted no data moves in this build, so the amendment is exercised by the check, not the demo). The HIL register-pattern test remains to add.

### 5.8 Stale pending dispatch bits (A4)

When a Release is suppressed after its vector was pended, the NVIC may still take that vector. Taking it clears the pending bit; `irq_entry` finds the Task not eligible and returns unchanged, and the NVIC arbitrates again among the remaining pending vectors, where P-04 applies. Each stale bit is consumed once, so a suppression event causes at most NT stale entries, each costing one stub entry. Whether the Analyzer's Kernel overhead terms include that cost is to be confirmed (task 15.2).

### 5.9 Core_Subset items without Ferrocene certification

`profile/core_subset.toml` lists six items with `certification = "none"`. Only `core::arch::asm` (stubs, evidence section 6) and `core::panic::PanicInfo` (the panic handler, `location()` only) are used. `NonZeroU8`, `core::array::{from_fn, map}`, the atomics, and `core::hint::spin_loop` are not used by the Kernel crates, and some recorded substitutes do not match the code (for example, the event-log overflow counter is a plain saturating `u32`, not an atomic). Done 2026-10-22: the four unused items were removed from `profile/core_subset.toml` (19 items remain; `cargo xtask profile` regenerated `docs/profile.md`); an item returns with its evidence when first used.

## 6. Kani (R45.4, R45.6, R45.8; task 6.3)

### 6.1 Limitations and mitigations (R45.6)

| Limitation [RN-16, RN-17, RN-18] | Effect on rsk | Mitigation or TBR |
|---|---|---|
| Sequential code only; data races not detected | Kernel concurrency is preemption between levels | Single-core, non-reentrant precondition (A10, TBR-20); Loom model (P2, R47); review UJ-003 to UJ-006 |
| Inline assembly unsupported; global assembly ignored | All of `stubs.rs` and the asm in `hw.rs` | Stubs KS-01 to KS-07; review identifiers; HIL differential tests (6.2) |
| Harnesses build for the host, so every `cfg(all(target_arch = "arm", target_os = "none"))` branch holding the volatile access, raw write, or `asm!` is compiled out | Harnesses check address tables, frame words, and encoders, not the unsafe operation | Kani credit is limited to those pure computations; the operations rest on review (TBR-06). Open: restructure so the flight path calls the checked function with no cfg split. |
| Aliasing-model violations, mutation of immutable data, and invalid values not detected unless enabled | `KernelCell`, `ResourceCell`, frame writes through raw pointers | Review (UJ-003 to UJ-006, UJ-050 to UJ-054, UJ-102 to UJ-104); evaluate Kani's optional invalid-value checks |
| Bounded verification | Logic harnesses at NT = 3, NR = 2 | Verus covers every size; 6.3 |
| Results hold only if every unwinding assertion passes | none | R45.5 verdict rule; fixture `xtask/fixtures/kani-unwind` |
| Stubbing is unstable (`-Z stubbing`) | Not used: no `#[kani::stub]`; the `sim` double is selected by cfg | TBR-07; revisit if stubs are adopted |
| `--restrict-vtable` soundness issue | Not used; the Kernel has no trait objects | Note only; the job does not pass the flag |
| Harnesses restate the Verus postconditions, written by the same author | Not an independent specification | Independence gap (8.5, GAP-08) |

### 6.2 Stubs and the differential tests that validate them (R45.4)

| Stub | Hardware_Model rules | Differential test | Status |
|---|---|---|---|
| KS-01 MMIO read and write | HW-01 (priority read-back), HW-09, HW-12, HW-13 | `hil-dt-hw-01`, `-09`, `-12`, `-13` | pending (hardware) |
| KS-02 BASEPRI | HW-03 | `hil-dt-hw-03` | pending (hardware) |
| KS-03 PRIMASK | HW-04 | `hil-dt-hw-04` | no harness depends on it |
| KS-04 frame record | HW-06 | `hil-dt-hw-06` | QEMU spike only |
| KS-05 SVC number | HW-06 | `hil-dt-hw-06` | QEMU spike only |
| KS-06 console and exit | none | none | debug I/O; no claim depends on it |
| KS-07 host SVC wrappers | HW-07 | `hil-dt-hw-07` | QEMU spike only |

### 6.3 Bounds (R45.8)

| Harnesses | Bounds | Covers |
|---|---|---|
| `logic/kani_harness.rs` (5 on `Sched` and `Config`) | NT = 3, NR = 2, NP = 2, NL = 4, 11 slots, unwind 6 | The p1-demo configuration (NT = 3, NR = 1, NP = 2, NL = 2, NS = 11) |
| `intake_outcomes_match_r10` (`Kernel`, 2026-10-22) | the same, NS = 11, NE = 8, three events at instants up to 10,000 ticks, both MIT policies, unwind 13 | R10.1 acceptance, R10.3 while pending, R10.1/R10.2 after the Job ended (accept, defer, discard) with R10.7 attribution |
| `uj_001_mmio_addresses_valid` | symbolic IRQ < 240, unwind 64 | Every NVIC line of the nRF52840 (48). The callers' bound is not checked: `nvic_*` take an exception number and subtract 16, so an out-of-range argument is excluded only by the binding tables (review) |
| `mpu_load_writes_every_region` | 8 regions, unwind 10 | The 8 regions of the Target (ASM-02) |
| other arch harnesses | symbolic scalar inputs, no loops | All inputs |

The Profile_Conformance_Suite Application does not exist yet (task 14.1); design.md proposes 16 Tasks, 16 Resources, 48 slots, and lock depth 8, which the logic harnesses do not reach. The residual risk is recorded in TBR-07. It is small because the Verus proofs cover every size and Kani is used here as a cross-check of the erased code. Raise the bounds when the conformance Application is defined.

## 7. Application prover (R46.1, R46.2)

| Criterion | Prusti | Creusot | Aeneas |
|---|---|---|---|
| Maintenance, release recency | Latest release 2024-02-28 on nightly-2023-09-15; describes itself as a prototype (RN-19) | Releases v0.7.0 (November 2025) to v0.13.0 (30 July 2026) (RN-20) | Active; a September 2026 report verifies 16.7 KLOC of Rust (RN-21) |
| Rust subset | Not re-evaluated (inactive) | Mutable borrows through prophecies, loops with invariants, closures, traits (RN-20); interior mutability and `unsafe` stay behind the `rsk` API with contracts | Safe subset; no `unsafe` or concurrency; some loop forms unsupported (RN-21) |
| `no_std` | to confirm | to confirm at pinning (task 17.1) | to confirm |
| Soundness argument, trusted base | Viper and its SMT back end | Translation to Coma and Why3, then SMT solvers (TBR-08) | Translation through Charon's LLBC, then the Lean or HOL4 kernel |
| Toolchain compatibility | Its 2023 nightly predates rustc 1.95.0, so rsk crates may not compile | Each prover drives its own pinned rustc; the version and its gap to 1.95.0 and 1.98.1 are to be recorded at pinning (ORQ-08 applies, 9.3) | as Creusot |
| Panic and overflow freedom | yes | yes (overflow behaviour to confirm at pinning) | yes, through the translation |
| Loop bounds (R35.2) | limited | termination checking with variants (RN-20) | termination in Lean |
| Release conditions (R5.5), validation totality (R42.4) | not evaluated | pure functions with contracts and termination | possible for small pure functions |
| DO-330 and DO-333 prospects | none known | none known (ORQ-10); several Why3 solvers allow cross-solver confirmation | none known; kernel-checked proofs give the strongest soundness argument |

Designation: Creusot (ORQ-10, design.md). It is the only active candidate that covers the constructs Task bodies use, closures in particular. Aeneas stays an option for small pure computations such as control laws (task 17.3). Pinning Creusot, Why3, and the solvers in the Build_System and recording them in each `proof-app-creusot` Evidence_Item (R46.2) is open, task 17.1. No Task body is proven yet, so R46.5 has no entries.

## 8. Tool qualification (R51)

### 8.1 Classification (R51.1, R51.2)

DO-178C Table 12-1 (RN-38): criterion 1 gives TQL-1 at DAL A and TQL-2 at DAL B; criterion 2 gives TQL-4 at both; criterion 3 gives TQL-5 at both.

| Tool | Criterion | TQL at DAL A / DAL B | Justification | State |
|---|---|---|---|---|
| Qualified_Toolchain (Ferrocene, LLVM, linker) | 1 | TQL-1 / TQL-2 | Its output is the airborne code | Upstream rustc 1.95.0 until P5; Ferrocene supports up to DAL C (RN-39), ORQ-04, GAP-01 |
| Generator (rsk-macros, rsk-gen) | none, if its output is verified (8.2); otherwise 1 | none (otherwise TQL-1 / TQL-2) | Config_Checker, Kernel boot validation, and review verify its output | Config_Checker built (task 13.1) and run on every Flight_Build; the argument holds for the values of R26.2 (8.2) once the mutation suite (13.2) shows its detection ability |
| Verus ghost-erasure macro (verus_builtin_macros) | 1, unless the erased source is shown equal to the verified code (R55.3) | TQL-1 / TQL-2 | It produces the source that becomes airborne code | No comparison yet (task 23.1) |
| Config_Checker | 2 | TQL-4 / TQL-4 | Its verdict removes the need to qualify the Generator (R26.6) | Built (task 13.1) and run on every Flight_Build (R26.5) with the mutation suite of R26.7 (task 13.2), 2026-10-23 |
| Link_Checker | 2 for stack bounds; 3 for the defence-in-depth scans | TQL-4 / TQL-4 (stack bounds); TQL-5 / TQL-5 (scans) | Stack bounds replace stack-usage testing | Stack bounds, the instruction-class and symbol scans, the Core_Subset mapping, and the no-panic-path check built (tasks 12.1 to 12.3, 2026-10-23) and run on every Flight_Build; each check has a fixture that produces its violation (`scan::tests`) |
| Profile_Lint | 3 | TQL-5 / TQL-5 | Backed by LK checks and compile errors | Built (task 11.1, 2026-10-23): `rsk-lint` (source lint over `syn`) and the Clippy deny-lists, run on every Flight_Build (verify step 8c); one UI fixture per lint and the configuration-bypass test |
| Protocol_Checker | 2 if its k-MC result replaces protocol testing, else 3 | TQL-4 / TQL-4, else TQL-5 / TQL-5 | | P4 |
| Analyzer | 3 in v1; 2 if it replaces timing tests (R51.5) | TQL-5 / TQL-5 in v1 | v1 keeps the HIL response-time tests (task 19.4) | Core proven, terms not |
| UPPAAL | 3 | TQL-5 / TQL-5 | Cross-check only; no credit claimed; licence ASM-13 | Exporter exists, ceiling not modelled |
| WCET_Harness | 3 | TQL-5 / TQL-5 | Measurements are checked against observed executions and the static cross-check (P5) | P3 |
| Static_WCET_Analyzer | 2 if it replaces measurement | TQL-4 / TQL-4 | | P5 |
| Verus | 2 where proofs replace tests | TQL-4 / TQL-4 | 8.3 | In use, unqualified |
| Kani | 2 where credit is claimed | TQL-4 / TQL-4 | 8.3 | In use, unqualified |
| App_Prover (Creusot) | 2 | TQL-4 / TQL-4 | Replaces robustness and requirements-based testing of Task bodies | P2 |
| Coverage tool | 3 | TQL-5 / TQL-5 | | P3, ORQ-05 |
| HIL_Rig test automation | 3 | TQL-5 / TQL-5 | | Not built (section 11) |
| Flashing tool (probe-rs) | none: output verified | none | Read-back hash check (R48.4) | Not built |
| Build_System (xtask) | 3 | TQL-5 / TQL-5 | It aggregates verdicts and could fail to detect a failure; rebuild comparison (R59.3) and archived logs back it | In use |

Qualification is per project; rsk tools can at most be qualifiable, and qualification itself is an applicant activity (R51.6, R54.4).

### 8.2 The Generator needs no TQL-1 or TQL-2 (R51.3)

The argument: every Generator output that reaches the Flight_Build is either recomputed and compared by the Config_Checker from the Task_Model and the linked binary (R26.2, R26.4), checked at boot by the proven `Config::validate` and `Layout::validate` (well-formedness, not equality to the declaration), or covered by review. The mutation suite of R26.7 (task 13.2) is the planned evidence of the Config_Checker's detection ability. Since 2026-10-23 the Config_Checker exists (`rsk-confcheck`, task 13.1) and runs on every Flight_Build (verify step 8b): on the P1 demo it compares 197 values of every R26.2 class with the Task_Model and the `CONFIG` and `LAYOUT` images and their checksums with the binary. The mutation suite (task 13.2, `docs/config_mutations.toml`) alters a Task_Model or a binary in each class and every one of its 16 mutants is reported, so the detection ability the argument needs is demonstrated on the P1 demo; the Profile_Conformance_Suite Application (task 14) is to replace the demo as its subject. Outside the argument still: the stack region sizes (Link_Checker, task 12) and the outputs listed below.

Generator outputs that R26.2 as written does not cover, which need an extended Config_Checker or review:

- the dispatch-vector assignment and its order (DD-03; the ascending order of the built-in Target tables is only unit-tested);
- the intake IRQ table and the `binding` function from exception numbers to Tasks;
- the `task_entry` table from Task index to Job entry function (a wrong entry runs the wrong code);
- periods, offsets, and deadlines (R26.2 names the Budget and MIT tables only);
- `System` constants: `TIMER_VECTOR`, `IRQ_COUNT`, `PRIORITY_BITS` (the last is also boot-checked), stack bounds and the safe-state path;
- linker-script archive placement, beyond what the memory-map comparison of R16.7 checks;
- the Partition Context types (type-level; wrong types fail compilation or show in review);
- `summary.txt` (informational, no verification needed).

### 8.3 Verus and Kani (R51.4)

- Verus: credit claimed for the Kernel logic properties of section 3 (compliance of the logic source with its formalized low-level requirements, freedom from overflow, bounds errors, and panics) and for the Analyzer fixed-point core. That credit replaces verification activities, so Verus is criterion 2, TQL-4. No DO-330 qualification of Verus is known (ORQ-06). The Vermilion Lean 4 back end (RN-15) is followed as a route to independent proof checking.
- Kani: credit claimed only for bounded absence of UB in the pure computations of the Kernel_Unsafe_Module (6.1) and as a cross-check of the Verus postconditions on erased code. Criterion 2, TQL-4 where that credit removes review; no qualification known.

### 8.4 Analyzer (R51.5)

Criterion 3, TQL-5, while its result serves only schedulability verification and the HIL response-time tests stay. If those tests are dropped on the strength of its result, criterion 2, TQL-4.

### 8.5 Independence (R51.7)

| Verifying tool | Verified item | Tool independence | People independence |
|---|---|---|---|
| Config_Checker | Generator | Required by R26.1 and met by construction (task 13.1): `rsk-confcheck` depends on serde_json and the shared ELF reader only, never on rsk-gen, rsk-model, rsk-logic, or rsk-kernel; it reads the Task_Model with its own parser and restates the Kernel's table encoding | Not achieved: one author. Applicant activity |
| Kani cross-checks | Verus proofs | Different technique (bounded model checking on erased code) | Same author writes the specification and the harness |
| HIL differential tests | Hardware_Model | Silicon against model | Not yet built |
| Analyzer reference implementation, simulator | Analyzer | Planned independent implementation (task 15.4) | Not achieved |
| Link_Checker | Stack and call graph | Differential run against `cargo-call-stack` (task 12.1): not run, the tool is not vendored (R58.1) and needs a nightly compiler; the decoder's encodings are unit-tested against hand-assembled instructions, and the bounds are to be checked by stack painting on the HIL_Rig (section 11) | Not achieved |

Every DAL A and DAL B verification objective that requires independence needs a reviewer other than the author. The prototype cannot supply that; Gap_Register entry GAP-08 records it as an applicant activity (R54.4). Which Annex A objectives require independence at each level is to be confirmed against the standard (Gap_Register OPEN-01).

## 9. Formal-methods credit (R52)

### 9.1 Soundness justification (R52.1)

| Method | Semantics and trusted base | Known issues and how rsk handles them |
|---|---|---|
| Verus (Kernel, Hardware_Model, Analyzer core) | SMT encoding of a Rust subset; trusts Verus, Z3 4.16.0, and `vstd` with its `assume_specification` items (TBR-04) | No assumption construct in the Kernel crates or rsk-analyzer (R50.2 gate); ghost-erasure cfg flags can introduce unsoundness (RN-12): rsk never sets `verus_keep_ghost` itself (only the Verus toolchain does), and Flight_Builds compile the erased code; no abort specification (RN-13, 5.6) |
| Kani | CBMC bounded model checking of MIR for the host; trusts Kani, CBMC 6.11.0, the SAT solver, and the `sim` double (TBR-07) | RN-17 issues in 6.1; `--restrict-vtable` unused; the host build compiles out the flight branches |
| Creusot | Translation to Coma and Why3, SMT solvers (TBR-08) | Not in use yet; soundness of the Why3 translation is the residual risk; discharge by two solvers where feasible |
| Protocol_Checker | k-multiparty compatibility (RN-22) | P4 |
| UPPAAL | Timed automata model checking | No credit claimed; the exported model does not yet model the SRP ceiling |
| Static_WCET_Analyzer | Binary-level analysis | P5 |

### 9.2 Validating models and formal specifications (R52.2)

- **Hardware_Model.** Every rule cites its manual section (R49.2); the not-formalized list states why no proof depends on each omission (R49.6); the HIL differential test per rule (R49.3) is hardware-pending (section 11). QEMU runs are pre-hardware checks only.
- **Specifications against requirements.** Every Verus specification names the criteria it formalizes in its doc comment. A review of each specification against the natural-language criterion is open (section 13).
- **Vacuity checks.** During 2026-10-11 to 2026-10-14 each new specification was checked by breaking the code or model and confirming that Verus fails:

| Change | Failed specification |
|---|---|
| Reverse the dispatch-vector order | `lemma_dispatch_selects_srp_choice` (tie step) |
| `encode_level(0)` = 256 | `lemma_encoding_order`, `lemma_kernel_level_never_masked` |
| Count the shared code as Partition-owned | `lemma_hw_partition_isolation` |
| Let the MPU grant every write | `lemma_mpu_grant_covered` |
| Kill only one Job of a restarted Partition; drop a deadline disarm | `end_partition_jobs`, `job_return` postconditions |
| Kill other Partitions' Jobs; reset a neighbouring Budget | `others_unchanged` (R14.9) |
| Clear a pending Release on discard; make a deferred Release due now; defer a periodic Release | `intake`, `deferred_release`, `Kernel::inv` |

Since 2026-10-22 these checks are fixtures: `docs/proof_mutations.toml` registers nine mutations (the seven above, two of them in equivalent form: `shared-code-writable` for the shared-code row and `restart-kills-one-job` for the kill row, plus two for `arch::bound`), and `cargo xtask mutations` applies each to a copy of the tree, runs Verus, and fails unless the mutant is rejected with the registered diagnostic; `cargo xtask mutations --check` (part of the repository checks) keeps the register applicable to the current sources. The run is not part of `cargo xtask verify` (nine Verus runs); it is run before a Verification_Plan revision and recorded here. Run of 2026-10-22 (Verus 0.2026.10.04.426d8b0): all nine mutants killed, each by the registered diagnostic (`dispatch-vector-order`, `kernel-level-encoding`, `shared-code-writable`, `mpu-grants-every-write`, `restart-kills-one-job`, `kill-other-partitions`, `discard-clears-release`, `bound-dispatch-unguarded`, `basepri-lower-level`). Not reproduced from the manual list: the dropped deadline disarm, the deferred Release made due now, and the deferred periodic Release; they are to be added to the register.

### 9.3 Property preservation to object code (R52.3, ORQ-08)

1. Verus checks the `verus!` source, ghost code included, on rustc 1.98.1.
2. Ghost erasure removes spec and proof code; the same source files then build for the Target with upstream rustc 1.95.0 in every verification job (Ferrocene 26.05 in P5). Equality of the erased executable code to the verified code rests on `verus_builtin_macros` (criterion 1, 8.1). Mitigations: the Kani cross-check harnesses run the erased code against restated postconditions (bounded); archiving and comparing the erased source (R55.3) is planned for task 23.1.
3. rustc 1.98.1 and 1.95.0 differ. The Kernel uses no feature stabilized after 1.95.0, which the 1.95.0 build enforces; the remaining assumption is that both compile the subset with the same semantics (ASM-09, TBR-01).
4. LLVM optimization: no translation validation (ORQ-08 open). Mitigations: requirements-based HIL tests on the Flight_Build binary (hardware-pending) and source-to-object traceability (R53.2, P3).
5. The Build_Manifest binary hash (R33.3) ties every Evidence_Item to the binary tested.

### 9.4 Objectives claimed and testing that remains (R52.4)

The project does not hold the DO-178C or DO-333 texts. The Gap_Register (`docs/gap_register.toml`) lists every DO-178C Annex A objective by table and number, as public summaries give them, with its applicability, status, owner, and gaps; the DO-333 items are described, not numbered. Both are to be confirmed against the standards' texts (Gap_Register OPEN-01, OPEN-02). The Annex A column below uses the Gap_Register's numbering.

| Objective (description) | Annex A | Formal credit claimed | Testing or review that remains |
|---|---|---|---|
| Source code complies with the low-level requirements | A-5.1 | Kernel logic and Analyzer core: the Verus specifications are the formalized low-level requirements | Kernel_Arch, unsafe module, and `rsk-entry`: review and HIL (GAP-10) |
| Source code is accurate and consistent (overflow, exception handling, stack, timing) | A-5.6 | Overflow, bounds, and panic freedom of the logic | Stack (Link_Checker), timing (WCET), Kernel_Arch panics (5.6; GAP-12, GAP-13, GAP-04) |
| Partitioning integrity is confirmed | A-4.13 | MPU decision (P-16), R14.9 (P-18), Kernel never masked (P-17) | HIL access matrix, DMA, fault injection (GAP-05, GAP-06, GAP-09) |
| Low-level requirements are accurate and consistent | A-4.2 | Verus checks that specifications are well formed | Review against natural-language criteria; vacuity fixtures (GAP-18) |
| Executable object code complies with the low-level requirements, including robustness | A-6.3, A-6.4 | Logic properties carried to object code through 9.3 | HIL requirements-based and robustness tests on the Target (GAP-09, GAP-11) |
| Executable object code is compatible with the target computer | A-6.5 | none | HIL testing in full (GAP-09) |
| Verification coverage of requirements and of structure | A-7.3 to A-7.8 | Formal coverage of criteria (9.5) | Structural coverage: no objective replaced in v1 (R53.4); MC/DC tooling open (ORQ-05, GAP-02) |

### 9.5 Criteria covered by each formal analysis (R52.5)

"Complete" means the criterion is stated as a proven specification with no gap at the logic level; the hardware realisation still rests on TBR-02, TBR-05, and section 4.

| Analysis | Complete | Partial, with the remainder |
|---|---|---|
| Verus, Kernel | R7.1, R7.2, R7.3, R20.3; R8.1 (per step), R8.2 (restore across preemptions over the history, task 4.13), R8.3, R8.4 (one blocking section per Release over the history under the A4 premise, task 4.12), R8.5 (deadlock freedom for every state, task 4.14), R8.9 (a killed Job's retained entries delay nothing, task 4.13), R8.10; R9.3, R9.4; R10.1, R10.2 (per event, and spacing over a run by `lemma_mit_spacing_over_run`, task 4.14), R10.3 (within the threshold), R10.7 (attribution to the given Partition); R12.3 (tables); R14.5 (effects), R14.6, R14.7, R14.9 (no Endpoints yet); R16.1, R16.3, R16.5, R16.6 (validated layouts); R18.1; R18.2 (level part); R44.1 and R44.7 (logic) | R7.5 (registers: review, HIL); R8.9 (one event with a count instead of one per Resource; the unwind releasing in the same entry is Kernel_Arch, section 4); R9.1, R9.5 (slot-to-Release link open); R9.6 to R9.8; R10.4 (counting proven, the windowed form is ORQ-26); R10.9, R19.1, R19.2 (local masking only; the bound is open); R16.2 (Kernel-owned peripherals: Generator, Config_Checker); R44.8 |
| Verus, Analyzer core | R32.1 (least fixed point, termination, overflow) | R29 (term derivation unproven: tests, task 15.4) |
| Kani | none on its own | R45.1, R45.2, R45.3 within the bounds of 6.3 |

## 10. HAL and PAC strategy for Partitions (R58.3, ORQ-21)

- Partition crates keep `#![forbid(unsafe_code)]` (PR-20); the Build_System checks every crate root.
- Peripheral access goes through rsk drivers over the PAC. Third-party crates with `unsafe` (PAC, HAL) are allowed as dependencies of a Partition only with a Trust_Base_Register entry per crate and version (R58.2, R58.5, TBR-13); the dependency gate enforces the entries.
- The MPU confines their effects to the Partition's own peripheral windows (P-16). The Generator places only declared, Partition-owned peripherals in windows and rejects Kernel-owned ones (PR-29); the PPB is not a peripheral of any Target table. The Config_Checker is to confirm both on the linked binary (R16.7).
- EasyDMA pointer and length registers are written only through the Kernel's mediation service (DD-08, task 5.5, built 2026-10-23: `rsk::dma_request` over SVC 6 and `dma_request_core`), because a DMA master bypasses the MPU (ORQ-03). The service accepts a channel of the executing Job's own Partition and a buffer in that Partition's RAM above its stack region and within enabled subregions (R17.1, `PartitionLayout::dma_permitted`); anything else is refused before any register is written and applies the Partition's fault response (R17.2, `EV_DMA_REJECTED`). List-mode registers are cleared at boot and never written; PPI channels are cleared at boot; a stop or restart triggers every stop task of the Partition's channels before its memory is re-initialized (R17.6). The channel table is Generated_Config's `DMA` (one entry per channel of every EasyDMA-capable peripheral a Partition owns, from the Target table's register offsets).
- The p1-demo uses no peripherals, so no HAL or PAC crate is in the Flight_Build graph yet.

## 11. Hardware-pending evidence

No nRF52840-DK is available as of 2026-10-14. These items are deferred, not dropped: they stay open, keep their tasks, and are required for the Phase 1 checkpoint (task 9). QEMU results are recorded as pre-hardware evidence and never as their substitute; QEMU mps2-an386 differs from the Target in its timer (CMSDK, not the nRF TIMER), priority bits (8, not 3), cycle counting (none), EasyDMA (absent), and errata (not modelled). What QEMU is trusted for is Trust_Base_Register entry TBR-23; the open gap is Gap_Register entry GAP-09.

| ID | Evidence (criteria) | Establishes | Claims waiting on it | Before hardware | Task |
|---|---|---|---|---|---|
| `hil-dt-hw-01` to `-14` | Hardware_Model differential tests (R49.3) | Each HW rule matches the silicon | Every Hardware_Model lemma (P-04, P-16, P-17, P-25); TBR-02; stubs KS-01, KS-02 (R45.4) | Citations; QEMU exercises HW-02, HW-03, HW-05 to HW-07, HW-09 on a model | 3.3 |
| `hil-errata` | Errata tests (R49.4, R11.6) | Erratum 838869 applicability on r0p1 and its DSB workaround | TBR-03 | Review in the `hw_model.rs` header | 3.4 |
| `hil-spike` | Hardware part of the DD-01 spike (R7.7, R11.1, R16.4) | Register patterns preserved across preemption; unprivileged PPB access faults; a ceiling masks only Task levels | P-04, P-10, P-11 | QEMU spike (`RSK-OK`) | 3.1 |
| `hil-rig` | Rig self-test (R34.8, R48.4) | Two-board rig, probe-rs, read-back hash | Every HIL item; TBR-18 | none | 7.1 |
| `hil-boot-checks` | Boot-check suite (R12.6) | Mismatched values and corrupted tables reach the safe state; each check of R12.1 (CPUID, MPU regions, priority bits and PRIGROUP, instruction cache, HFCLK source, FPU presence, DWT), the `CONFIG` and `LAYOUT` checksums of R12.3, and a watchdog reset fail with their own bit | P-14 | Verus `validate` proofs and `boot_core`; the host test `boot_checks_pass_on_the_expected_board_and_fail_one_bit_per_check` (every check on simulated registers); QEMU boot (CPUID, MPU, PRIGROUP, DWT, checksum; no clock or cache registers) | 7.2 (5.2 done 2026-10-23) |
| `hil-release-timing` | Release timing over PAR-05 (R9.10) | Jitter, counter wraps, coinciding releases, late Jobs | P-07; HW-12; ASM-19 | none | 7.3 |
| `hil-mpu-matrix` | MPU access matrix (R16.8) | Every protected class faults as R16.4 requires | P-16 | `lemma_hw_partition_isolation`; QEMU view switching | 7.4 |
| `hil-dma` | DMA isolation (R17.7) | Kernel-mediated EasyDMA: a request with a buffer outside the permitted memory is refused before any register write; a restart stops the Partition's transfers | P-16; ORQ-03 | Verus: `dma_request_core` programs a channel only for the executing Job's own Partition with `dma_permitted` (`PartitionLayout::dma_permitted_exec`, proven equal to its specification) and `apply_core` triggers the stop tasks on `take_stop_dma`; Kani `dma_permitted_matches_recomputation`; mutation `dma-buffer-in-stack`; no QEMU model of EasyDMA | 7.4 (5.5 done 2026-10-23) |
| `hil-overrun`, `hil-storm`, `hil-response` | Fault injection (R18.8, R19.3, R14.11) | Δ_detect, Δ_enforce, the interrupt-source bound, responses on silicon | P-08 (Property 9), P-17, P-18 | Verus response and attribution proofs | 7.5 |
| `hil-watchdog` | Kernel-owned watchdog (ORQ-31) | A hung Kernel resets, and the next boot logs the watchdog reset and enters the safe state | R14.4 path | Verus: `boot_core` starts the WDT before the time base (`booted` requires `watchdog_running`) and `timer_core` feeds it last (`watchdog_fed`); host tests of the WDT register writes and of the RESETREAS read and clear; no QEMU model | 7.5 (5.3 done 2026-10-23) |
| `hil-cycles` | Budget accounting with DWT (R18.1, ASM-06) | Cycle attribution on the Target | P-17 | none (QEMU has no cycle counter) | 7.5 |

Work that can proceed without boards, so these items stay ready:

- write each test's procedure and the outcome the Hardware_Model predicts, so the tests are defined before the rig exists;
- implement the `rsk-hil` orchestration and dry-run it against QEMU where QEMU models the behaviour;
- record here the date boards become available, and re-run section 1 on the Target.

### HIL frequency (R50.1)

Once the HIL_Rig exists (task 7.1), the verification job runs the HIL suites of the table above at this frequency, each run starting with the read-back hash check (R48.4):

| When | Suites |
|---|---|
| Every change to the main branch that touches a Kernel crate, the Generator, a Target table, the linker script, or a toolchain pin | `hil-rig`, `hil-spike`, `hil-boot-checks`, `hil-mpu-matrix` |
| Nightly, on the head of the main branch | Every suite |
| Before each Phase checkpoint and each release-candidate Flight_Build | Every suite, on the exact binary whose Build_Manifest the checkpoint cites |

Until boards are available no HIL suite runs, the verification job produces no HIL Evidence_Item, and R50.1 is met only in part (GAP-09, GAP-13).

## 12. Criteria outside the allocation table (R43.5)

At requirement level, the criteria that no row of section 3 covers fall into four groups. The per-criterion list is produced by the traceability matrix (task 8.4, `cargo xtask trace`), which does not exist yet.

| Group | Requirements | Evidence |
|---|---|---|
| Documents, whose evidence is the document and its review | R1, R2, R4 (Profile; checked by `cargo xtask profile --check`); R43, R46.1, R46.2, R51, R52 (this plan); R49.2, R49.4, R49.6 (Hardware_Model text); R54 (Gap_Register); R61 (reuse record) | `check-static-checks`; review records |
| Build_System checks | R6.1 to R6.4, R33.3, R50, R55, R58, R59 | `check-*` items and the xtask fixtures |
| Phase P2 tools | R3, R15, R21 to R28, R30, R31, R33.1, R33.2 | Phase P2 |
| Later phases | R34 to R36, R53 (P3); R38 to R42 (P4); R5, R56, R60 (P5) | Their phases |

## 13. Review record and open items

Review status: not reviewed. For DAL A/B the review must be done by someone other than the author, which is an applicant activity (R54.4). The review covers:

1. each Verus specification against its natural-language criterion (R52.2);
2. each argument of section 5;
3. the inspection facts of section 4 against the Kernel_Arch code;
4. the tool classifications of section 8.

Open items for v2:

- R43.5 per-criterion list (task 8.4);
- App_Prover pinning and the facts marked "to confirm" in section 7 (task 17.1);
- the Kani flight-path restructuring (6.1);
- confirmation of the Gap_Register's Annex A applicability, independence, and DO-333 and DO-330 items against the standards' texts (OPEN-01, OPEN-02);
- every hardware-pending item of section 11.
