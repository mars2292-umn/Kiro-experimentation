# Requirements Document

**rsk: a Ravenscar-Style Concurrency Profile for Rust**

Status: Draft 0.2 (requirements phase; every requirement refined). Spec: `rsk-ravenscar-profile`. Research facts checked October 2026.

## Introduction

rsk (working name) brings the guarantees of Ada's Ravenscar profile to Rust. It combines:

- **A. Profile** — a numbered restriction specification, analogous to `pragma Profile (Ravenscar)`.
- **B. Kernel** — a small `#![no_std]`, allocation-free runtime based on the Stack Resource Policy (SRP), with machine-checked proofs.
- **C. Task-model DSL** — a proc-macro application declaration that generates the runtime configuration, computes ceilings, and exports a JSON task model.
- **D. Analyzer** — a CLI that runs response-time analysis with SRP blocking terms and emits an UPPAAL model as an independent cross-check.
- **E. WCET pipeline** — measurement-based first, static later, with checked loop bounds.
- **F. Endpoints** — static, bounded, session-typed channels integrated with the ceiling protocol.
- **G. Verification plan** — which property is established by which tool, the trust base, and what remains assumed.

Together these let an application developer prove data-race freedom, deadlock freedom, bounded blocking, and schedulability of a hard real-time Rust application. rsk is a research prototype whose design must keep open a path to DO-178C DAL A/B, including mixed-criticality partitioning on one MCU, DO-330 tool qualification, and DO-333 formal-methods credit. rsk reuses existing work and records every deviation from a reused project (Requirement 61) and from Ada RM D.13 (Requirement 4).

### Fixed inputs (decided by the user; not open in this spec)

- Runtime model: SRP with RTIC as the baseline (tasks as interrupt handlers, the interrupt controller as scheduler, BASEPRI ceilings on Cortex-M, compile-time ceiling analysis), subject to the partitioning requirements.
- Build-time manifest: the whole system is declared statically, following Hubris' approach.
- Tools: Verus (kernel functional proofs), Kani (bounded model checking), UPPAAL (schedulability cross-check), Ferrocene (qualified toolchain), Loom and Shuttle (interleaving tests), one of Prusti, Creusot, or Aeneas for application proofs (selected in the Verification_Plan), Embassy for HAL and driver patterns only.
- Phase 1 hardware: nRF52840-DK (Arm Cortex-M4F at 64 MHz). Application tasks use the FPU.
- Multi-core: not in v1. Partitioned multi-core later (static task-to-core assignment, per-core SRP, cross-core sharing only through endpoints or MSRP/MrsP). Global or migrating scheduling never.
- Certification target: DO-178C DAL A/B. Mixed criticality on one MCU is in scope. Proofs should earn DO-333 credit.
- RISC-V part for Phase 5: open (ORQ-19).

### Phases

| Phase | Scope (user plan) |
|---|---|
| P1 | Profile specification; Verus-proven kernel on one Cortex-M board; Kani harnesses |
| P2 | Task-model DSL; schedulability analyzer; UPPAAL export |
| P3 | Measurement-based WCET pipeline with HIL tests |
| P4 | Session-typed endpoints |
| P5 | Ferrocene build; static WCET; RISC-V port; Jorvik-style extensions |

Each requirement heading names the earliest phase in which the requirement must hold. Partitioning requirements carry P1 because they constrain the Kernel design from the start.

### Conventions

- `Rn.m` is acceptance criterion m of Requirement n. `PR-nn` is a profile restriction, `NG-nn` a non-goal, `ASM-nn` an assumption, `ORQ-nn` an open research question, `PAR-nn` a parameter, and `[RN-nn]` a research note with its source (section "Research Notes and Sources").
- Enforcement categories: **TS** type system, **MC** macro (Generator), **LN** lint, **LK** linker or binary check, **PO** proof obligation, **RT** run-time monitor. RT is added to the five requested categories because some restrictions depend on the environment (for example interrupt arrival rates) and cannot be enforced statically. RT is listed as a supplementary category and is named primary only where no static mechanism can exist.
- Priority: a larger value means more urgent. The mapping to hardware priority encodings is a design matter.
- Time quantities are integers in CPU cycles or Kernel time-base ticks, with explicit units.
- Content from external sources was rephrased for compliance with licensing restrictions.

### Parameters

Values marked "proposal" are initial values for review; changing one requires updating this table, not the criteria that reference it.

| ID | Parameter | Value | Basis |
|---|---|---|---|
| PAR-01 | Maximum Kernel executable lines (excluding blank lines, comments, Verus spec/proof code, generated code) | 5,000 | User goal of a few hundred to a few thousand lines |
| PAR-02 | Minimum measured executions per WCET item | 10,000 | proposal |
| PAR-03 | Minimum UPPAAL cross-check corpus | 50 Task_Models with up to 8 Tasks each | proposal; 8 matches the Target's preemption levels |
| PAR-04 | Published task sets reproduced by the Analyzer | 3 | proposal |
| PAR-05 | Duration of each fault-injection timing test | the longer of 1 hour and 1,000 hyperperiods | proposal |
| PAR-06 | Hardware preemption levels on the Target | 8 (3 NVIC priority bits) | [RN-25] |
| PAR-07 | MPU regions on the Target | 8 | [RN-29], ASM-02 |
| PAR-08 | Target CPU clock | 64 MHz | [RN-28], ASM-05 |

### Component traceability

| Component | Requirements |
|---|---|
| A. Profile | 1–5 |
| B. Kernel | 6–14 |
| Mixed-criticality partitioning (cross-cutting) | 15–20 |
| C. Task-model DSL | 21–27 |
| D. Analyzer | 28–32 |
| E. WCET and resource bounds | 33–37 |
| F. Endpoints | 38–42 |
| G. Verification plan | 43–50 |
| Certification (cross-cutting) | 51–54 |
| Toolchain (cross-cutting) | 55–59 |
| Portability and reuse | 60–61 |

## Glossary

- **rsk**: The complete system specified here: Profile, Kernel, Generator, Profile_Lint, Link_Checker, Config_Checker, Analyzer, WCET_Harness, Static_WCET_Analyzer, Endpoint_Library, Protocol_Checker, and their documents.
- **Application**: A Rust program for the Target built against rsk, consisting of an App_Declaration and Task bodies.
- **App_Declaration**: The proc-macro input that declares the Application's Partitions, Tasks, Resources, Endpoints, peripherals, and Target parameters.
- **Profile**: The rsk restriction specification (Component A).
- **Profile_Variant**: `Ravenscar` (default) or `Jorvik` (optional, P5).
- **Generator**: The rsk proc-macro code generator that consumes the App_Declaration and emits Generated_Config and the Task_Model.
- **Generated_Config**: All Generator output used at run time, as listed in R24.1, emitted as constant data that nothing modifies after Init (R24.6).
- **Profile_Lint**: The custom lint set (Clippy-based or rustc-driver-based) that enforces Profile rules not enforced by the type system or the Generator.
- **Link_Checker**: The post-link tool that checks the linked executable: symbols, call graph, stack usage, instruction classes, allocator absence, and dependency provenance.
- **Config_Checker**: An independently implemented tool that recomputes derived configuration values from the declared values in the Task_Model and compares them with the Task_Model and the linked binary.
- **Profile_Conformance_Suite**: The suite of accepted and rejected example Applications that demonstrates enforcement of every restriction. Its accepted Application for the selected Profile_Variant (R3.7) is the Profile_Conformance_Suite Application.
- **Kernel**: The rsk runtime (Component B) linked into the Application.
- **Kernel_Unsafe_Module**: The clearly bounded part of the Kernel in which `unsafe` code and assembly are permitted: one module of the Kernel crate, plus the Kernel's entry crate, whose only content is the vector table and reset entry. The module may also contain Kani_Harnesses that call its `unsafe` functions. They are compiled only for Kani (`cfg(kani)`), so no Flight_Build contains them.
- **Kernel_Arch**: The architecture-specific part of the Kernel: exception entry and return, interrupt controller, privilege, memory protection, FPU, timers, and cycle counter.
- **Kernel-owned item**: A peripheral or memory region that Generated_Config assigns to the Kernel, including the timer hardware of R9.4 and Endpoint storage (R15.6, R40.1).
- **Kernel-reserved levels**: The priority levels above every Ceiling that the Kernel uses, including the level from which it detects Overruns whatever the System_Ceiling (PR-33, R18.2).
- **Health_Monitor**: The Kernel component that receives fault, overrun, deadline, and violation events and applies the declared responses.
- **Partition fault response**: A member of the closed set of Health_Monitor responses to faults that the Profile defines; each Partition declares from this set its fault response and its deadline-miss response (R14.5, R15.1).
- **Event log**: The Health_Monitor's record of events, with a capacity fixed in Generated_Config (R14.6, R14.7).
- **System safe state**: The Target state, declared in the App_Declaration and recorded in Generated_Config, that the Kernel enters on unrecoverable faults (R9.9, R12.2, R14.4).
- **Task**: A statically declared unit of concurrent execution with one fixed Priority, released periodically or sporadically.
- **Job**: One execution of a Task body, from its Release until it ends: it returns to its Release_Point or a Health_Monitor action ends it (R7.1).
- **Eligible**: Said of a released Job whose Priority is strictly greater than the System_Ceiling and than the Priority of every Job that has started and not ended (R7.1).
- **Release**: The event that makes a Job ready to execute. A Release becomes due at its nominal release instant (periodic) or at its accepted event instant or deferred instant (sporadic); it is pending from then until its Job starts (R9.7), effective once the start conditions of R7.1–R7.3 apply to its Job (R9.2), and deferred while it waits for an MIT to elapse (R10.2).
- **Release_Point**: The single point in a Task's cycle at which the Task waits for its next Release.
- **Periodic_Task**: A Task released at its nominal release instants O + k·T in Kernel time-base ticks (offset O, period T), counted from instant 0, the instant at which the Kernel enables Releases after Init and the checks of Requirement 12 (R9.1).
- **Sporadic_Task**: A Task released by one Release_Source with a declared MIT.
- **FPU-using Task**: A Task declared to execute floating-point instructions (PR-27, PR-31).
- **Absolute deadline**: The instant by which a Job must complete, as R9.6 defines for periodic and sporadic Jobs.
- **Timed event**: A periodic-release, Budget, deadline, or MIT-deferral event held in the Kernel's timer queue (R9.4).
- **Release_Source**: The single event source of a Sporadic_Task: a hardware interrupt, a message arrival at an Endpoint receive role, a Release_Signal, or, in the Jorvik Profile_Variant, a relative delay requested at the Release_Point (R5.3).
- **Release_Signal**: A Kernel object, analogous to a Ravenscar suspension object or a single-entry protected object, that releases exactly one Sporadic_Task or, in the Jorvik Profile_Variant, at most one of up to a declared maximum number of bound Tasks per raise (R5.2).
- **Priority**: The static integer assigned to a Task; a larger value is more urgent.
- **Resource**: A statically declared shared data object accessed only inside Critical_Sections under the ceiling protocol; the analogue of a Ravenscar protected object.
- **Ceiling**: For a Resource r, the maximum Priority of all Tasks that access r, computed at build time; for an Endpoint buffer whose operations the Kernel executes at a Kernel-reserved level, that level (R40.1).
- **System_Ceiling**: At any instant, the maximum Ceiling among currently locked Resources, or the idle level when none is locked.
- **Idle activity**: The Kernel activity that executes below every Task Priority while no released Job is eligible and every started Job has ended (R7.6).
- **Critical_Section**: The interval during which a Job holds a Resource.
- **SRP**: The Stack Resource Policy, as used by RTIC [RN-03].
- **Endpoint**: A statically declared, bounded, session-typed message channel between Protocol roles bound to Tasks.
- **Protocol**: A multiparty session type (global protocol and per-role projections) governing the messages on a set of Endpoints.
- **Protocol_Checker**: The build-time component that checks Protocol well-formedness, projection, compatibility, and buffer bounds.
- **Endpoint_Library**: The `no_std` library implementing Endpoints.
- **Partition**: A statically declared isolation unit grouping Tasks, Resources, Endpoint roles, peripherals, and memory regions, with one Criticality_Level.
- **Criticality_Level**: The DO-178C software level (A, B, C, D, or E) assigned to a Partition.
- **Budget**: The declared maximum execution time of one Job of a Task, in CPU cycles, enforced by the Kernel.
- **Overrun**: A Job's consumed execution time exceeding its Budget.
- **Overrun_Response**: A member of the closed set of Health_Monitor actions defined in the Profile, selected per Partition.
- **MIT**: Minimum inter-arrival time between consecutive accepted Releases of a Sporadic_Task.
- **MIT_Violation**: A Release_Source event that arrives less than one MIT after the previous accepted Release became due, or while a Release of the same Task is pending or deferred (R10.2, R10.3), or a periodic Release that falls due while a Release of the same Task is pending (R9.8).
- **Task_Model**: The machine-readable JSON description of the Application exported by the Generator.
- **WCET**: Worst-case execution time. **WCET_Record**: a WCET value with provenance (method, tool, binary hash, conditions).
- **WCRT**: Worst-case response time computed by the Analyzer.
- **WCET_Harness**: The measurement-based WCET tooling (P3).
- **Static_WCET_Analyzer**: The static WCET tooling (P5).
- **Loop_Bound_Annotation**: A source annotation stating the maximum iteration count of a loop.
- **Analyzer**: The schedulability analysis CLI (Component D).
- **UPPAAL_Exporter**: The Analyzer function that emits an UPPAAL timed-automata model and query file.
- **Kernel_Timing_Parameters**: The set of Kernel overhead bounds defined in Requirement 13.
- **Kernel_Proofs**: The Verus proofs of Kernel properties.
- **Kani_Harnesses**: The Kani bounded-model-checking harnesses.
- **App_Prover**: The verifier selected from Prusti, Creusot, and Aeneas for application-level proofs.
- **Host_Test_Harness**: Loom- and Shuttle-based interleaving tests for host-side and future multi-core components.
- **Hardware_Model**: The formal model of the Target's exception, interrupt-controller, BASEPRI/PRIMASK, privilege, MPU, FPU, timer, and cycle-counter behaviour that the Kernel_Proofs rely on.
- **Verification_Plan**: The document (Component G) allocating properties to tools and evidence.
- **Design_Document**: The rsk design specification (`design.md`) produced in the design phase.
- **Trust_Base_Register**: The document listing every trusted component and assumption, with justification and residual risk.
- **Gap_Register**: The document listing DO-178C, DO-330, and DO-333 objectives with their rsk status.
- **Build_System**: The scripted build and verification pipeline that produces binaries, the Build_Manifest, and Evidence_Items.
- **Build_Manifest**: The per-build record of source revision, lock file, toolchain versions, build configuration, Profile version, Generated_Config checksum, and binary hash (R33.3).
- **Evidence_Item**: A versioned artifact (proof log, test report, analysis report, measurement data) traced to acceptance criteria.
- **Qualified_Toolchain**: The pinned Ferrocene release used for Flight_Builds.
- **Verus_Toolchain**: The rustc version pinned by the selected Verus release.
- **Core_Subset**: The documented allow-list of `core` library items that the Kernel and Applications may use.
- **Target**: In Phase 1, the Nordic nRF52840 on the nRF52840-DK.
- **HIL_Rig**: The hardware-in-the-loop setup of one or more nRF52840-DK boards controlled by a host.
- **Init**: The initialization phase executed before any Release.
- **Init_Arena**: An optional bounded static memory area from which allocation is permitted only during Init.
- **Flight_Build**: A build configuration that represents deployable code (release profile, no test instrumentation).

## Non-Goals

- **NG-01** Multi-core execution in v1. Any model that assigns work to more than one core is rejected (Requirement 27).
- **NG-02** Global or migrating multi-core scheduling, in any version.
- **NG-03** Operation with instruction or data caches enabled, and cache-aware WCET analysis.
- **NG-04** Dynamic loading: runtime code loading, partial or field update of individual Tasks or Partitions.
- **NG-05** General-purpose OS features: file systems, network stacks, processes, virtual memory, shells, POSIX interfaces, dynamic users or permissions.
- **NG-06** Heap allocation after Init in Applications, and heap allocation of any kind in the Kernel.
- **NG-07** Async executors as the scheduling model (Embassy's executor, RTIC v2's per-priority async executors), and `async`/`.await` in Task bodies in v1. Rationale: an Embassy executor polls tasks cooperatively from a run queue and relies on tasks not blocking; preemption exists only between separate executor instances [RN-36]. Within one executor level a task can be delayed by the sum of other tasks' non-preemptive segments rather than by one critical section; future combinators allow select-style waiting (forbidden by PR-11); and Verus and Kani do not support `async`/`.await` [RN-10, RN-16]. Embassy may be consulted for HAL and driver patterns only.
- **NG-08** Dynamic task creation or termination, dynamic priorities, abort, requeue, and rendezvous.
- **NG-09** Mixed-criticality mode-change scheduling (Vestal model, AMC) in v1; a research extension (ORQ-15).
- **NG-10** Soft real-time behaviour and average-case performance optimization.
- **NG-11** Security objectives (DO-326A airworthiness security, confidentiality between Partitions, timing side channels). Partitioning in rsk targets integrity and timing.
- **NG-12** Obtaining certification approval or producing applicant plans; rsk preserves a path, tracked in the Gap_Register.
- **NG-13** Modifying, forking, or qualifying the Rust compiler.
- **NG-14** Closed-source binary components such as the Nordic SoftDevice (which also enables the instruction cache [RN-34]), and radio stacks.
- **NG-15** Dynamic power and clock management beyond idle wait-for-interrupt.
- **NG-16** `std` targets for flight code. Host tools may use `std`.

## Requirements

### Part A — Profile Specification

### Requirement 1: Profile specification document (Phase P1)

**User Story:** As a certification engineer, I want a versioned profile specification in which every restriction states its guarantee and its enforcement, so that I can trace each guarantee to evidence.

#### Acceptance Criteria

1. THE Profile SHALL assign each restriction a unique identifier of the form PR-nn that no later Profile version reassigns to a different restriction, and SHALL keep each retired identifier listed as retired in every later Profile version.
2. THE Profile SHALL state for each active restriction the rule; the guarantee the rule supports; one or more enforcement categories from {TS, MC, LN, LK, PO, RT} with the primary category listed first; the corresponding Ada RM D.13 items, named as in Table 4-1, or the marker "rsk-specific"; and, for each listed category, at least one Evidence_Item that demonstrates enforcement by that category.
3. THE Profile SHALL state, for each active restriction that lists RT, the assumption (ASM-nn) whose violation the run-time monitor detects or, where the violations the monitor detects imply no failed assumption, why the restriction's static enforcement categories, if any, cannot exclude those violations.
4. THE Profile SHALL carry a version identifier that changes whenever the Profile's content changes and is never reused, and THE Generator SHALL record in every Task_Model the identifier of the Profile version that the Generator implements.
5. THE Profile SHALL contain at least the restriction catalogue of Requirement 2, the Ada D.13 correspondence of Requirement 4, and the disallowed-crate list of PR-35.
6. WHEN a Profile version differs from the preceding Profile version, THE Profile SHALL record in a cumulative change log one entry for each restriction, Table 4-1 row, or other Profile section that differs, stating the Profile version that makes the change, the change (for example an added, changed, retired, or activated restriction, or a reclassified Table 4-1 row under R4.6), its rationale, and the affected requirements.

### Requirement 2: Profile restriction catalogue (Phase P1)

**User Story:** As an application developer, I want a precise list of the rules my program must follow, so that the proofs and the timing analysis apply to my program.

#### Acceptance Criteria

1. THE Profile SHALL contain the restrictions PR-01 to PR-36 of Table 2-1 as active restrictions, each with the rule, guarantee, enforcement categories (primary first), and Ada D.13 analogue that Table 2-1 states.
2. THE Profile SHALL state for each active restriction the subjects to which the rule applies, being the subjects that the rule names (for example the Kernel or code reachable from a Task body) or, where the rule names none, the Application, and SHALL state which third-party crates linked into a Flight_Build the rule covers, consistent with the HAL and PAC strategy of R58.3 (ORQ-21).
3. THE Profile SHALL name, for each active restriction and each enforcement category that the restriction lists, at least one Evidence_Item (R1.2) of the kind given for that category: for TS, MC, LN, and LK, a rejected program of the Profile_Conformance_Suite for that restriction and category (R3.6); for PO, a proof of the Kernel_Proofs, the Kani_Harnesses, or the App_Prover under the requirement that the PO entry cites, as named in the Verification_Plan (R3.8); and for RT, a fault-injection test report showing that the Kernel reports a violation injected at run time, with the static checks bypassed, to the Health_Monitor as R3.9 requires.

**Table 2-1 — Profile restrictions** (enforcement categories: primary first; parenthesized text after a category names its mechanism or specifying requirement and, for RT, the assumption or reason that R1.3 requires; Ada D.13 analogue: Table 4-1 items spelled as in Table 4-1, or the marker rsk-specific, each optionally followed by a parenthesized qualifier)

| ID | Rule | Guarantee supported | Enforcement | Ada D.13 analogue |
|---|---|---|---|---|
| PR-01 | Every Task is declared in the App_Declaration; no Task is created after build time. | Finite, known task set for RTA (R29); static memory and MPU layout (R24) | MC, TS (no spawn API; Task handles constructible only by generated code) | No_Task_Allocators, No_Task_Hierarchy |
| PR-02 | Every Task exists from the end of Init until reset; a Job ends only by returning to its Release_Point or by a Health_Monitor action (R14, R18). | Stable interference set; no Resource left inconsistent by abort | TS (no terminate or abort API), PO (R44: the Task set is unchanged after Init, and only a Health_Monitor action ends a Job before its Release_Point) | No_Task_Termination, No_Abort_Statements |
| PR-03 | Each Task has exactly one Priority, fixed at build time. | Fixed-priority RTA; static Ceilings | MC, TS (no priority-setting API) | No_Dynamic_Priorities |
| PR-04 | Each Task, Resource, and Endpoint has a build-time core assignment; in v1 the assignment is core 0. | Single-core SRP guarantees; door kept open for partitioned multi-core (R27) | MC | No_Dynamic_CPU_Assignment, No_Dependence => System.Multiprocessors.Dispatching_Domains |
| PR-05 | Tasks share mutable state only through Resources or Endpoints; `static mut` items and `static` items with interior mutability exist only in Kernel code. | Data-race freedom; analyzable blocking | TS (Send/Sync bounds on Resource and payload types, as RTIC infers [RN-06]), LN, PO (lock soundness, R44) | Locking_Policy (Ceiling_Locking) |
| PR-06 | Every Resource and its set of accessing Tasks is declared in the App_Declaration; Ceilings are computed at build time. | Static Ceilings for SRP and blocking terms | MC | No_Local_Protected_Objects, No_Protected_Type_Allocators |
| PR-07 | Resources are accessed only through lexically scoped lock operations, so nested Critical_Sections are released in LIFO order; no Job enters a Critical_Section on a Resource it already holds (R8.10), so the maximum nesting depth, derived at build time, does not exceed the number of Resources declared for the Task. | SRP LIFO requirement; stack discipline | TS (closure-scoped lock API), MC, PO (R44.1) | rsk-specific (counterpart of Ada's protected-action semantics, which no D.13 item states) |
| PR-08 | A Job executes no operation that waits, suspends, or reaches a Release_Point inside a Critical_Section. | Blocking bounded by one Critical_Section; deadlock freedom | TS (the lock closure is synchronous and receives no wait-capable handle), LN | Detect_Blocking (rsk rejects statically instead of detecting at run time) |
| PR-09 | Each Job runs to completion: a Task body has exactly one Release_Point per cycle and no other suspension point; `async` functions, `async` blocks, and `.await` are absent from Task bodies. | SRP run-to-completion; RTA without self-suspension; Verus and Kani applicability [RN-10, RN-16] | TS (Task body is a synchronous function), LN | rsk-specific (SRP requirement [RN-03]) |
| PR-10 | Periodic Releases are absolute instants (offset plus multiples of the period); the Application contains no relative-delay operation. | No cumulative drift; analyzable release jitter | TS (only absolute-time release API), LN (denies relative-delay APIs of third-party crates such as RTIC monotonic `delay` [RN-08]) | No_Relative_Delay |
| PR-11 | Each Task has exactly one Release_Source (its period for a Periodic_Task); no Task waits on alternative events, and no operation has a timeout. | Single release point for RTA; no select semantics | MC, LN (denies select and join combinators) | No_Select_Statements, No_Requeue_Statements (rsk has no requeue) |
| PR-12 | Each Release_Signal, interrupt Release_Source, and Endpoint receive role releases exactly one Task, and each Task has at most one pending Release (R9.7); a Release that falls due while one is pending counts as an MIT_Violation (R9.8, R10.3). | One waiter per release object; bounded release state; no multi-task release | MC, TS, RT (ASM-10) | Max_Entry_Queue_Length => 1, Max_Protected_Entries => 1, No_Dependence => Ada.Synchronous_Barriers |
| PR-13 | A release condition is a single Kernel-maintained flag or message availability; the Kernel evaluates no application-supplied predicate. | Constant-time, bounded release logic | TS, MC | Simple_Barriers, Max_Protected_Entries => 1 (a Resource carries no release condition; relaxed together with Simple_Barriers in the Jorvik Profile_Variant, R5.7) |
| PR-14 | Every queue and buffer (Endpoint buffers, release state, timer queue, event log) has a capacity fixed at build time. | Bounded memory; bounded queue operation time | TS (const-generic capacities), MC, PO (no overflow, R44.2) | Max_Entry_Queue_Length => 1 (generalized to a build-time capacity for every queue and buffer) |
| PR-15 | The Kernel performs no dynamic allocation; the Application obtains storage statically, on the stack, or, WHERE the Init_Arena option is enabled, from the Init_Arena during Init only. | Bounded memory; no allocation failure after Init | LK (no global allocator symbols in Flight_Builds without Init_Arena), MC, RT (Init_Arena sealed at the end of Init; R1.3 reason: whether code shared by Init and Task bodies allocates after Init depends on run-time control flow) | No_Implicit_Heap_Allocations, No_Task_Allocators, No_Protected_Type_Allocators |
| PR-16 | Every loop in Task bodies and in the Kernel has a Loop_Bound_Annotation or an iteration count derivable from compile-time constants, and every annotated bound is verified. | Job termination; finite WCET | LN (missing bound, R35.1), PO (R35.2), LK (from P5, the Static_WCET_Analyzer's report of a Loop_Bound_Annotation that matches no loop in the binary, R36.3) | rsk-specific |
| PR-17 | No function in the Kernel or reachable from a Task body is directly or indirectly recursive. | Bounded stack; finite WCET | LK (call-graph cycle check on the linked binary), LN | rsk-specific |
| PR-18 | Every indirect call reachable from a Task body or the Kernel has a finite target set resolved by the Link_Checker. | Computable worst-case stack and WCET | LK, LN | rsk-specific |
| PR-19 | Panics abort without unwinding; the Kernel and the code of Criticality_Level A and B Partitions are proven panic-free; every panic that occurs is delivered to the Health_Monitor as a fault of the running Partition or, for a panic in Kernel code, of the Kernel (R14.2, R14.4). | Defined fault behaviour; no unwinding through Critical_Sections | LK (abort strategy; single rsk panic handler, R14.1), PO (R44, R45, R46), RT (ASM-09, ASM-15: a panic in code proven panic-free) | rsk-specific |
| PR-20 | Every rsk and Application crate applies `#![forbid(unsafe_code)]`, except that the Kernel's crates apply it to every module outside the Kernel_Unsafe_Module (R6.3); every `unsafe` block, `unsafe fn`, and `unsafe impl` in the Kernel_Unsafe_Module carries a justification identifier linked to a Kernel_Proofs proof or lemma or a Kani_Harnesses harness where one is feasible, and otherwise to a recorded review that states why none is feasible (R6.4, R45.1). | Memory safety rests on a small, audited unsafe base | LN (attribute presence), LK (Build_System check of justification links, R6.4), PO (R44, R45) | rsk-specific |
| PR-21 | The Kernel and the Application use only `core` items in the Core_Subset. | Trust in `core` limited to a documented subset (R57) | LN (allow-list), LK (R57.3) | rsk-specific |
| PR-22 | Only the Kernel accesses interrupt-masking state (PRIMASK, FAULTMASK, BASEPRI), NVIC configuration, MPU registers, SCB priority and fault registers, and FPU context-control registers; Application code contains no inline or global assembly. | The Kernel alone controls the System_Ceiling and isolation; blocking bounds hold | TS (core peripherals owned by the Kernel), LN (denies assembly macros and core-peripheral crates outside the Kernel), LK (instruction scan outside the Kernel), RT (ASM-14; MPU and privilege, per ORQ-01) | rsk-specific (supports Ceiling_Locking) |
| PR-23 | Interrupt-to-Task bindings and interrupt priorities are fixed at build time; the Kernel changes no interrupt priority after Init. | Static interference set; static Ceilings | MC, PO (R44) | No_Dynamic_Attachment |
| PR-24 | The Application creates no timers or timing events; timed behaviour is expressed only as declared Periodic_Task Releases. | Bounded timer queue; analyzable timer interrupts | TS (no timer API), MC | No_Local_Timing_Events, No_Dependence => Ada.Execution_Time.Timers |
| PR-25 | Per-Task state is declared statically in the App_Declaration; no dynamic per-Task attribute storage exists. | Bounded memory | MC, TS | No_Dependence => Ada.Task_Attributes |
| PR-26 | The Application reads time only from the monotonic Kernel time base; no wall-clock source influences scheduling. | Monotonic, drift-free release computation | TS | No_Dependence => Ada.Calendar |
| PR-27 | Every Task declares its release kind, period or MIT, offset (Periodic_Task), relative deadline D with D at most the period or MIT, Budget, MIT policy (Sporadic_Task), FPU use, and Partition. | Complete Task_Model; constrained-deadline RTA | MC | rsk-specific |
| PR-28 | Every Task, Resource, and Endpoint role, and every peripheral and memory region not owned by the Kernel (PR-29), belongs to exactly one Partition, and every Partition declares a Criticality_Level. | Static isolation domains | MC | rsk-specific |
| PR-29 | Each peripheral, including each EasyDMA-capable peripheral, is owned by exactly one Partition or by the Kernel. | No cross-Partition peripheral interference | MC, TS (peripheral singletons), RT (ASM-14; MPU), PO (MPU layout, R44.4) | rsk-specific |
| PR-30 | A Resource is accessed only by Tasks of one Partition, except that an Endpoint buffer is a Kernel-owned Resource shared only through Endpoint_Library operations (R40.1); data exchange between Partitions uses Endpoints only. | Spatial isolation; Budget enforcement cannot leave another Partition's Resource inconsistent | MC | rsk-specific |
| PR-31 | Each Task declares whether its Jobs use the FPU; code of FPU-free Tasks executes no floating-point instructions, and Kernel code executes none other than the floating-point context-preservation instructions of the R11 policy, in the Kernel_Unsafe_Module. | Correct FP context handling; FP costs included in WCET and stack bounds | MC, LK (instruction-class scan of code reachable from FPU-free Tasks and the Kernel) | rsk-specific |
| PR-32 | Messages between Tasks travel only over declared Endpoints, each governed by a declared Protocol. | Protocol compliance; communication deadlock freedom (R41) | MC, TS | rsk-specific |
| PR-33 | The number of distinct Task Priorities plus Kernel-reserved levels does not exceed the Target's preemption levels (PAR-06). | Every Priority and Ceiling is representable in hardware | MC | rsk-specific |
| PR-34 | Every Resource and Endpoint is initialized during Init, and no Release occurs before Init completes. | No access to uninitialized shared state; defined start-up | TS (Resources handed to Tasks only after Init returns), PO (R44) | rsk-specific |
| PR-35 | The Kernel and Task bodies depend on neither `std` nor any async executor, and the Application links no crate on the Profile's disallowed-crate list (R1.5). | No unanalyzable scheduling or OS dependency | LK (dependency-graph check), LN | rsk-specific |
| PR-36 | Every crate that a Flight_Build compiles, including proc-macro crates such as the Generator but excluding the libraries shipped with the Qualified_Toolchain, compiles without `#![feature]` attributes and without `RUSTC_BOOTSTRAP`. | Compatibility with the Qualified_Toolchain and the Verus_Toolchain | LK (Build_System check) | rsk-specific |

### Requirement 3: Enforcement and diagnostics (Phase P2)

**User Story:** As an application developer, I want every violation reported at build time with its restriction identifier, so that I can fix violations before any analysis runs.

#### Acceptance Criteria

1. WHEN the Generator detects a violation of a restriction whose categories include MC, THE Generator SHALL fail compilation with a diagnostic that names the PR identifier and the source span of each declaration involved.
2. WHEN the Profile_Lint detects a violation of a restriction whose categories include LN, THE Profile_Lint SHALL emit an error-level diagnostic naming the PR identifier and the source span.
3. WHEN the Link_Checker detects a violation of a restriction whose categories include LK, THE Link_Checker SHALL exit with non-zero status and report the PR identifier, the offending symbol, address, or crate, and the Task entry or Kernel entry point from which any offending symbol or address is reachable.
4. THE Profile_Lint SHALL report as a violation of the corresponding PR identifier each attribute (for example `#[allow(...)]`) and each compiler flag (for example `--cap-lints`) that lowers or caps the level of a Profile_Lint lint below error.
5. THE Build_System SHALL run the Generator checks, the Profile_Lint, the Link_Checker, and the Build_System checks of category LK on every Flight_Build, covering every crate that R2.2 subjects to a restriction, and SHALL fail the build when any of them reports a violation, naming the PR identifier, or when the Profile version or Profile_Variant that the Generator, the Profile_Lint, the Link_Checker, or the Kernel declares is absent or differs from the value recorded in the Task_Model.
6. THE Profile_Conformance_Suite SHALL contain, for each Profile_Variant that rsk implements, each active restriction, and each of the categories TS, MC, LN, and LK that the restriction lists, at least one rejected program that violates only that restriction, apart from any restriction the case names with a reason, and THE Build_System SHALL report the case as passing only when the check of that category rejects the program with a diagnostic that names the PR identifier or, for TS, with the compiler error that the case records.
7. THE Profile_Conformance_Suite SHALL contain, for each Profile_Variant that rsk implements, at least one accepted Application that uses every construct the Profile_Variant permits, including each release kind, each Release_Source kind, and each option such as the Init_Arena, and THE Build_System SHALL report the case as passing only when the build emits zero error-level diagnostics and the checks of R3.5 report no violation.
8. THE Verification_Plan SHALL name, for each active restriction that lists PO, the proof artifact that discharges each proof obligation of the restriction.
9. WHEN the Kernel detects at run time a violation of an active restriction that lists RT, THE Kernel SHALL report the violation to the Health_Monitor with the PR identifier or, where the event identifies no single restriction, with the PR identifier of each restriction that the detecting mechanism enforces.
10. IF a proof artifact named under R3.8 fails or is absent, THEN THE Build_System SHALL fail the verification job and name the PR identifier of each restriction whose proof obligation the artifact discharges.
11. THE Profile_Conformance_Suite SHALL contain, for each active restriction that lists RT, a fault-injection case that bypasses the static checks and injects a violation at run time, and THE Build_System SHALL report the case as passing only when the Health_Monitor's event log records the violation with that restriction's PR identifier (R2.3, R3.9).

### Requirement 4: Ravenscar and Jorvik correspondence (Phase P1)

**User Story:** As a researcher, I want each Ravenscar and Jorvik rule of Ada RM D.13 mapped to its rsk counterpart, so that I can argue that rsk meets the Ravenscar bar and see every deviation.

#### Acceptance Criteria

1. THE Profile SHALL give Table 4-1 exactly one row for each of the 3 pragmas and 26 restrictions that define the Ravenscar profile in the Ada 2022 edition of Ada RM D.13 [RN-01], and no other row, and SHALL classify each row as exactly one of: Enforced, citing the PR identifiers or requirements that make every Application satisfy the item; Not applicable by construction, naming the construct that rsk does not provide and the PR identifiers or requirements that prevent the Application from introducing it; or Deviation, stating the rsk behaviour that differs from the item, the rationale, and the PR identifiers or requirements that constrain the difference.
2. THE Profile SHALL mark in the Jorvik Profile_Variant column of Table 4-1 each Jorvik relaxation (removal of No_Implicit_Heap_Allocations, No_Relative_Delay, Max_Entry_Queue_Length => 1, Max_Protected_Entries => 1, No_Dependence => Ada.Calendar, and No_Dependence => Ada.Synchronous_Barriers, and replacement of Simple_Barriers by Pure_Barriers [RN-01]) as either Relaxed, citing the Requirement 5 criterion that defines the relaxed rule and the PR identifier whose enforcement changes, or Kept (deviation), citing the PR identifier that keeps the Ravenscar treatment and stating the rationale, and SHALL mark every other row Same, meaning that the row's Ravenscar classification and treatment apply unchanged in the Jorvik Profile_Variant.
3. WHILE the Design_Document records no resolution of ORQ-17, THE Profile SHALL classify the Task_Dispatching_Policy row of Table 4-1 as Deviation, stating that the order in which the Kernel starts eligible Jobs of equal Priority (R7.3) may differ from FIFO_Within_Priorities, citing ORQ-17, and citing R29.1 as the requirement through which the Analyzer accounts for interference between Tasks of equal Priority.
4. THE Profile SHALL classify the No_Dependence => Ada.Execution_Time.Group_Budgets and No_Dependence => Ada.Execution_Time.Timers rows of Table 4-1 as Deviation, stating that rsk gives the Application no operation to create or arm an execution-time timer (PR-24) or a group budget, that Budgets are fixed in the App_Declaration (PR-27), that the Kernel nevertheless monitors every Job against its Budget for temporal partitioning (Requirement 18), and that the costs of this monitoring are bounded by Δ_timer, Δ_detect, and Δ_enforce (Table 13-1) and included in every WCRT (R29.3, R29.4).
5. THE Profile SHALL keep Table 4-1 consistent with Table 2-1 in every Profile version, such that each row cites every PR identifier whose Ada D.13 analogue in Table 2-1 names that row's item, each PR identifier cited in a row classified Enforced names that row's item in its Ada D.13 analogue, and every cited PR identifier is defined in Table 2-1.
6. WHEN the Design_Document records the resolution of ORQ-17, THE Profile SHALL, in the next Profile version, state the resolved equal-priority start order in the Task_Dispatching_Policy row, classify that row as Enforced if the resolution emulates FIFO_Within_Priorities or as Deviation if it adopts any other order, and record the reclassification in the change log of R1.6.
7. THE Profile SHALL classify the No_Abort_Statements and No_Task_Termination rows of Table 4-1 as Deviation, stating that the Application has no operation that aborts or terminates a Task (PR-02), that a Health_Monitor action can nevertheless end a Job before the Job reaches its Release_Point or stop or restart a Partition, naming each such action from the closed sets of responses defined under R14.5, and citing R14.9 and R18.4 as the requirements that bound the effect on other Partitions and on Resources.

**Table 4-1 — Ada RM D.13 correspondence** [RN-01]

Classification as defined in R4.1; Jorvik Profile_Variant markers (Same, Relaxed, Kept (deviation)) as defined in R4.2. In Deviation rows, "Difference" states the rsk behaviour that differs from the item.

| Ada RM D.13 item (Ravenscar) | Classification | rsk treatment | Jorvik Profile_Variant |
|---|---|---|---|
| Task_Dispatching_Policy (FIFO_Within_Priorities) | Deviation | Fixed-priority preemptive SRP dispatch (Requirement 7). Difference: the start order of eligible equal-priority Jobs (R7.3) may differ from FIFO_Within_Priorities (R4.3, R4.6); rationale: the order depends on the dispatch mechanism, which ORQ-17 leaves open; constrained by R29.1, whose hep(i) includes every equal-priority Task. | Same |
| Locking_Policy (Ceiling_Locking) | Deviation | PR-05, PR-06, PR-22, Requirement 8 (SRP). Ceilings are computed at build time from the declared accessors (PR-06, R8.7), so no Job locks a Resource whose Ceiling is below the Job's Priority. Difference: a Job may enter a nested Critical_Section on a Resource whose Ceiling is below that of an enclosing Critical_Section, which Ceiling_Locking treats as a ceiling violation that raises Program_Error [RN-42]; rationale: on one core R8.1 keeps the System_Ceiling at the maximum held Ceiling, as RTIC's BASEPRI_MAX lock does [RN-07]; constrained by R8.3–R8.5. | Same |
| Detect_Blocking | Enforced | PR-08 (static rejection at build time instead of run-time detection) | Same |
| No_Abort_Statements | Deviation | PR-02: the Application has no operation that aborts a Task. Difference: a Health_Monitor action can end a Job before it reaches its Release_Point (R4.7); rationale: fault containment and temporal partitioning between Partitions (Requirements 14 and 18); constrained by R14.9, R18.4. | Same |
| No_Dynamic_Attachment | Enforced | PR-23 | Same |
| No_Dynamic_CPU_Assignment | Enforced | PR-04 | Same |
| No_Dynamic_Priorities | Enforced | PR-03 | Same |
| No_Implicit_Heap_Allocations | Enforced | PR-15 | Kept (deviation; PR-15): Jorvik removes it; rsk keeps PR-15; rationale: bounded memory and no allocation failure after Init (PR-15's guarantee), required for DAL A/B (R5.4) |
| No_Local_Protected_Objects | Enforced | PR-06 | Same |
| No_Local_Timing_Events | Enforced | PR-24 | Same |
| No_Protected_Type_Allocators | Enforced | PR-06, PR-15 | Same |
| No_Relative_Delay | Enforced | PR-10 | Relaxed (R5.3; PR-10): relative delays are permitted at the Release_Point only, and the Kernel converts each to an absolute release instant |
| No_Requeue_Statements | Not applicable by construction | rsk provides no entry and no requeue operation, and no Task waits other than at its Release_Point (PR-09, PR-11) | Same |
| No_Select_Statements | Enforced | PR-11 | Same |
| No_Specific_Termination_Handlers | Not applicable by construction | rsk provides no operation that installs a per-Task termination handler: Tasks do not terminate (PR-02), the Kernel's panic handler is the only panic handler (PR-19, R14.1), and faults go to the Health_Monitor, which applies only declared responses (Requirement 14) | Same |
| No_Task_Allocators | Enforced | PR-01, PR-15 | Same |
| No_Task_Hierarchy | Enforced | PR-01 | Same |
| No_Task_Termination | Deviation | PR-02: every Task exists from the end of Init until reset, and the Application has no operation that terminates a Task. Difference: a Health_Monitor action can stop or restart a Partition, including its Tasks (R4.7); rationale: fault containment and temporal partitioning between Partitions (Requirements 14 and 18); constrained by R14.9, R18.4. | Same |
| Simple_Barriers | Enforced | PR-13 | Relaxed (R5.5; PR-13): Pure_Barriers analogue: a release condition may be a side-effect-free Boolean function of one Resource's state with a proven termination bound |
| Max_Entry_Queue_Length => 1 | Enforced | PR-12, PR-14 | Relaxed (R5.2; PR-12): a Release_Signal may have several waiting Tasks up to its declared maximum waiter count; PR-14 stays in force |
| Max_Protected_Entries => 1 | Enforced | PR-12, PR-13 | Relaxed (R5.7; PR-13): several Release_Signals whose release conditions read one Resource, up to a declared maximum |
| Max_Task_Entries => 0 | Not applicable by construction | Tasks have no entries and rsk provides no rendezvous; PR-09 permits no suspension point other than the Release_Point | Same |
| No_Dependence => Ada.Asynchronous_Task_Control | Not applicable by construction | The Kernel provides no operation that holds or continues a Task, and PR-22 reserves to the Kernel the NVIC configuration through which a Task's interrupt Release_Source could be disabled | Same |
| No_Dependence => Ada.Calendar | Enforced | PR-26 | Kept (deviation; PR-26): Jorvik removes it; rsk keeps monotonic time only; rationale: monotonic, drift-free release computation (PR-26's guarantee) |
| No_Dependence => Ada.Execution_Time.Group_Budgets | Deviation | rsk gives the Application no group-budget operation. Difference: the Kernel monitors per-Job Budgets internally (R4.4); rationale: temporal partitioning (Requirement 18); constrained by R29.3, R29.4. | Same |
| No_Dependence => Ada.Execution_Time.Timers | Deviation | PR-24: the Application creates no timers. Difference: the Kernel uses Budget timers internally (R4.4); rationale: temporal partitioning (Requirement 18); constrained by R29.3, R29.4. | Same |
| No_Dependence => Ada.Synchronous_Barriers | Enforced | PR-12 | Kept (deviation; PR-12): Jorvik removes it; rsk provides no construct that releases a group of Tasks together once a declared number of them are waiting; rationale: bounded release state with no multi-task release (PR-12's guarantee) |
| No_Dependence => Ada.Task_Attributes | Enforced | PR-25 | Same |
| No_Dependence => System.Multiprocessors.Dispatching_Domains | Enforced | PR-04, NG-02: static core assignment, matching D.13's advice of fully partitioned multiprocessor dispatching [RN-01] | Same |

### Requirement 5: Jorvik-style profile variant (Phase P5, optional)

**User Story:** As an application developer, I want an optional Jorvik-style variant, so that I can use bounded multi-waiter release objects, relative delays, and richer release conditions where the analysis supports them.

#### Acceptance Criteria

1. WHERE the Jorvik Profile_Variant is selected, THE Generator and THE Profile_Lint SHALL relax only PR-10, PR-12, and PR-13, as the Table 4-1 entries marked Relaxed define, and SHALL enforce unchanged every other restriction and clause, including the absolute periodic Releases of PR-10 and every PR-12 clause except that each Release_Signal releases exactly one Task.
2. WHERE the Jorvik Profile_Variant is selected, THE Generator SHALL require a declared maximum waiter count for every Release_Signal bound to more than one Task and reject excess bindings, THE Kernel SHALL release at most one bound Task per raise of the Release_Signal (PR-12), and THE Analyzer SHALL include the resulting additional release and blocking costs in every affected WCRT.
3. WHERE the Jorvik Profile_Variant is selected, WHEN a Task requests a relative delay of d ticks at the Task's Release_Point (PR-09 stays in force; ORQ-18), THE Kernel SHALL make the Task's next Release due at the absolute Kernel time-base instant d ticks after the request, subject to the Task's MIT with deferral (R10.2).
4. THE Profile SHALL keep PR-15 in force in the Jorvik Profile_Variant for every Partition (NG-06) and SHALL record in Table 4-1 the deviation from Jorvik's removal of No_Implicit_Heap_Allocations, with the DAL A/B rationale.
5. WHERE the Jorvik Profile_Variant is selected, THE Generator SHALL accept as an application-supplied release condition only a Boolean function of one Resource's state with a declared termination bound, THE App_Prover or THE Kani_Harnesses SHALL prove that each such condition is free of side effects and panics and terminates within the declared bound, and THE Analyzer SHALL include the cost of evaluating each such condition in every affected WCRT.
6. WHEN a Task_Model uses a Jorvik relaxation, THE Analyzer SHALL reject the Task_Model unless the Task_Model records the Jorvik Profile_Variant and the Analyzer version supports that relaxation (R28.6).
7. WHERE the Jorvik Profile_Variant is selected, THE Generator SHALL require, for every Resource read by the release conditions (R5.5) of several Release_Signals, a declared maximum number of such Release_Signals and SHALL reject an App_Declaration that exceeds that maximum.

### Part B — Kernel

### Requirement 6: Kernel construction constraints (Phase P1)

**User Story:** As a kernel developer, I want the Kernel small, `no_std`, allocation-free, and with `unsafe` confined to one module, so that the whole Kernel can be proven.

#### Acceptance Criteria

1. THE Kernel SHALL consist of `#![no_std]` crates whose dependency graphs for the Target, including transitive dependencies, contain neither `std` nor `alloc` (PR-15, PR-35).
2. THE Kernel SHALL contain at most PAR-01 lines of executable Rust, counted as PAR-01 defines over the Kernel source built for the Target, and THE Build_System SHALL report the count on every build and fail each build whose count exceeds PAR-01.
3. THE Kernel SHALL confine every `unsafe` block, `unsafe fn`, `unsafe trait`, `unsafe impl`, unsafe attribute (such as `no_mangle`), and inline or global assembly to the Kernel_Unsafe_Module, and SHALL apply `#![forbid(unsafe_code)]` to every other Kernel module.
4. THE Kernel_Unsafe_Module SHALL annotate each `unsafe` block, `unsafe fn`, and `unsafe impl` that a Flight_Build compiles with a justification identifier, and SHALL link each justification identifier to a Kernel_Proofs proof or lemma or a Kani_Harnesses harness where one is feasible, and otherwise to a recorded review that states why none is feasible. THE Build_System SHALL fail the build if any of them lacks a justification identifier, if any justification identifier has no link, if a linked proof, lemma, harness, or review does not exist, or if a justification identifier linked only to a review lacks that statement (R44, R45).
5. THE Kernel SHALL allocate statically every Kernel data structure other than local variables of Kernel functions, taking the capacity of every Kernel table and queue from Generated_Config (PR-14).
6. THE Kernel SHALL execute no floating-point instructions other than the context-preservation instructions of the R11 policy in the Kernel_Unsafe_Module (PR-31).
7. THE Kernel SHALL expose as linked symbols its version identifier and the Profile version and Profile_Variant that it implements (R3.5), and THE Config_Checker SHALL report a mismatch if the Kernel version symbol is absent or differs from the Kernel version recorded in the Task_Model.

### Requirement 7: SRP dispatching (Phase P1)

**User Story:** As an application developer, I want Jobs dispatched by fixed priority under the Stack Resource Policy, so that response-time analysis with single-critical-section blocking applies.

#### Acceptance Criteria

1. THE Kernel SHALL start a released Job only at an instant at which the Job is eligible, where a released Job is eligible while its Priority is strictly greater than the System_Ceiling and strictly greater than the Priority of every Job that has started and not ended, and a Job ends when it returns to its Release_Point or when a Health_Monitor action ends it (PR-02).
2. WHILE one or more released Jobs are eligible, THE Kernel SHALL start the eligible Job of highest Priority, choosing among eligible Jobs of equal Priority by R7.3.
3. WHILE two or more eligible Jobs share the highest Priority, THE Kernel SHALL start them in the deterministic order that the Design_Document records as the resolution of ORQ-17 (R4.6), an order that depends only on Generated_Config and on the instants at which the Releases of those Jobs became effective (R9.2).
4. WHILE a Job is executing, THE Kernel SHALL permit that Job to be preempted only by an eligible Job (R7.1) or by Kernel execution whose duration a Kernel_Timing_Parameter of Table 13-1 bounds.
5. WHEN a Job ends, THE Kernel SHALL start the eligible Job that R7.2 and R7.3 select if one exists, and otherwise SHALL resume the most recently preempted Job that has not ended (last-in, first-out), with the register state and the System_Ceiling that held when that Job was preempted, or execute the idle activity (R7.6) where no such Job exists.
6. WHILE no released Job is eligible and every Job that has started has ended, THE Kernel SHALL execute the idle activity at a level below every Task Priority.
7. THE Kernel SHALL dispatch Jobs by the mechanism that the Design_Document records as the resolution of ORQ-01 (Requirement 20), following RTIC's mapping of Tasks to interrupt vectors and of the System_Ceiling to the priority-mask hardware [RN-03, RN-07] except where that resolution requires otherwise, and THE Design_Document SHALL record each departure from that mapping as a deviation with its rationale (R61).
8. THE Kernel SHALL start the Job that R7.2 and R7.3 select within Δ_dispatch of the instant at which that Job becomes the selected Job, plus Δ_wake where the idle activity was executing at that instant, and SHALL complete each transition of R7.5 within Δ_complete (Table 13-1).

### Requirement 8: Resource locking and the ceiling protocol (Phase P1)

**User Story:** As an application developer, I want scoped locks on Resources that raise and restore the System_Ceiling, so that shared data is accessed with mutual exclusion, without deadlock, and with bounded blocking.

#### Acceptance Criteria

1. WHEN a Job enters a Critical_Section on Resource r, THE Kernel SHALL set the System_Ceiling to the maximum of its value immediately before the entry and Ceiling(r), before the Job's first access to r within that Critical_Section.
2. WHEN a Job exits a Critical_Section on Resource r, THE Kernel SHALL restore the System_Ceiling to the value it held immediately before the matching entry, after the Job's last access to r within that Critical_Section.
3. THE Kernel SHALL admit at most one Job at a time into Critical_Sections on any one Resource (mutual exclusion) and SHALL ensure that every lock entry finds the Resource held by no other Job, so that no Job waits at lock entry, as proven by the Kernel_Proofs (R44.1).
4. THE Kernel SHALL limit the blocking of each Job, meaning the time during which the Job is released, has not started, and is not eligible (R7.1) only because its Priority does not exceed the System_Ceiling, to at most one interval per Release, no longer than one Critical_Section, including the Critical_Sections nested in it, of one lower-priority Job on a Resource whose Ceiling is at least the blocked Job's Priority, as proven by the Kernel_Proofs (R44.1).
5. THE Kernel SHALL keep every reachable Kernel state free of any set of Jobs in which each Job waits for a Resource held by another Job of the set (deadlock freedom), as proven by the Kernel_Proofs (R44.1).
6. THE Kernel SHALL provide access to Resource data only through a lexically scoped lock operation whose reference to the data cannot outlive the Critical_Section, so that the nested Critical_Sections of a Job exit in the reverse order of their entry (last-in, first-out), enforced by the type system (PR-07).
7. THE Generator SHALL compute Ceiling(r) for each Resource r at build time as the maximum Priority of the Tasks that the App_Declaration declares as accessing r (Requirement 23), and SHALL expose to each Task body only the Resources declared for that Task, so that a lock attempt on any other Resource is a compile-time error (PR-06).
8. THE Kernel SHALL complete each lock entry within Δ_lock_in and each lock exit within Δ_lock_out (Table 13-1).
9. IF a Health_Monitor action ends a Job while the Job holds one or more Resources, THEN THE Kernel SHALL release every Resource that the Job holds, so that the System_Ceiling becomes the maximum Ceiling of the Resources still held by Jobs that have not ended, and SHALL report each released Resource to the Health_Monitor.
10. IF an Application contains a Task body that can enter a Critical_Section on a Resource that the same Job already holds (re-entry), THEN THE Build_System SHALL reject the Application at build time, naming the Resource and PR-07, so that the nesting depth of a Job's Critical_Sections never exceeds the number of Resources declared for its Task.

### Requirement 9: Periodic release and timer dispatch (Phase P1)

**User Story:** As a control engineer, I want periodic Tasks released at exact absolute instants without drift, so that sampling periods and analysis assumptions hold.

#### Acceptance Criteria

1. THE Kernel SHALL release Job k (k = 0, 1, 2, …) of each Periodic_Task at the nominal release instant O + k·T of the Kernel time base, within the bound of R9.2, except where R9.8 discards the Release or a Health_Monitor response applied to the Task's Partition suppresses it (R14.5); O and T are the Task's offset and period in Kernel time-base ticks as recorded in Generated_Config (R21.2), and instant 0 is the instant at which the Kernel enables Releases after completing Init and the checks of Requirement 12 (PR-34).
2. WHEN the Kernel time base reaches the nominal instant of a periodic Release other than a Release excepted in R9.1, THE Kernel SHALL make that Release effective no earlier than that instant and no later than J_rel after it, whatever the System_Ceiling and the executing Job at that instant, and including when the instants of several timed events of R9.4 coincide, where a Release is effective once its Job is one of the released Jobs to which the start conditions of R7.1–R7.3 apply.
3. THE Kernel SHALL compute the nominal release instant of each Job of a Periodic_Task from the Task's offset and period only, so that the nominal instant of Job k equals O + k·T regardless of the completion and effective release instants of earlier Jobs, Releases discarded under R9.8, Overruns, deadline misses, and Health_Monitor responses applied to the Task's Partition.
4. THE Kernel SHALL serve all periodic Releases, Budget timers, deadline timers, and MIT-deferral timers (R10.2) from timer hardware that Generated_Config assigns to the Kernel (PR-29), using a timer queue whose capacity, fixed in Generated_Config, equals the number of declared timed events: one release event per Periodic_Task, one Budget event and one deadline event per Task, and one deferral event per Sporadic_Task whose MIT policy is deferral (PR-14, R23.4).
5. THE Kernel SHALL compute every nominal release instant, absolute deadline (R9.6), and timer-event instant from instant 0 to the end of the maximum continuous operating duration declared in the App_Declaration without arithmetic overflow and without wrap-around ambiguity, so that distinct instants have distinct representations ordered as the instants occur, including across every wrap of the counter of the timer hardware of R9.4.
6. IF a Job has not completed when the Kernel time base reaches its absolute deadline, THEN THE Kernel SHALL report a deadline miss naming the Task and its Partition to the Health_Monitor within Δ_detect after that instant, where the absolute deadline is the nominal release instant plus the Task's relative deadline D for a Job of a Periodic_Task, and the instant of the accepted Release_Source event (R10.1) or the deferred instant (R10.2) plus D for a Job of a Sporadic_Task; a Job whose Release is still pending at that instant (R9.7) has not completed.
7. IF a Release of a Periodic_Task becomes due at its nominal instant while the previous Job of that Task has not completed and no Release of that Task is pending (a Release is pending from the instant it becomes due until its Job starts), THEN THE Kernel SHALL hold that Release as the Task's single pending Release, with its nominal instant and absolute deadline unchanged and its Job started only after the previous Job ends (R7.1), and report the event, naming the Task, to the Health_Monitor (PR-12).
8. IF a Release of a Periodic_Task becomes due while a Release of that Task is pending, THEN THE Kernel SHALL discard the newly due Release, leave the pending Release unchanged, and record an MIT_Violation for the Task's Partition (PR-12, R10.4).
9. IF the Kernel time base reaches instant 0 plus the maximum continuous operating duration declared in the App_Declaration, THEN THE Kernel SHALL report the event to the Health_Monitor and enter the system safe state recorded in Generated_Config, so that no Release whose nominal instant lies beyond that duration becomes effective.
10. THE HIL_Rig SHALL execute the Profile_Conformance_Suite Application in a measurement build for at least PAR-05, recording for each periodic Release its nominal instant and the CPU cycles from the Kernel time base reaching that instant to the Release becoming effective, in configurations that include coinciding nominal instants of all Periodic_Tasks, a Critical_Section on the Resource with the highest Ceiling in progress at a nominal instant, at least one wrap of the counter of the timer hardware of R9.4, and a Job of a Periodic_Task that has not completed by the next two nominal instants of its Task (for example in a Partition whose Overrun_Response lets the Job complete, R18.6), and SHALL report a pass only when every recorded nominal instant equals O + k·T, every recorded interval lies between 0 and J_rel, and that Task produces the outcomes of R9.6, R9.7, and R9.8.

### Requirement 10: Sporadic release and MIT enforcement (Phase P1)

**User Story:** As a system integrator, I want sporadic Tasks released by interrupts, messages, or signals with their MIT enforced, so that analysis assumptions hold even when an event source misbehaves.

#### Acceptance Criteria

1. WHEN a Release_Source event occurs while no Release of the bound Sporadic_Task is pending (R9.7) or deferred (R10.2), and either no Release of that Task has become due since instant 0 (R9.1) or at least one MIT has elapsed since the previous accepted Release of that Task became due, THE Kernel SHALL accept the event as a Release of that Task that becomes due, and so pending, at the instant of the event.
2. IF a Release_Source event occurs while no Release of the bound Task is pending or deferred and less than one MIT after the previous accepted Release of that Task became due, THEN THE Kernel SHALL record an MIT_Violation and apply the Task's declared MIT policy: under deferral, the Release is deferred until the instant one MIT after the previous accepted Release became due, at which instant it becomes due and pending; under discard, the Kernel discards the event.
3. IF a Release_Source event occurs while a Release of the bound Task is pending or deferred, THEN THE Kernel SHALL discard the event, leave that pending or deferred Release unchanged, and record an MIT_Violation (PR-12).
4. WHEN the number of MIT_Violations attributed to a Partition (R10.7) since instant 0 or since that Partition's last restart, including those recorded under R9.8, exceeds the threshold declared for that Partition, THE Health_Monitor SHALL apply the Partition's declared fault response.
5. WHEN a Task raises a Release_Signal, THE Kernel SHALL treat the raise as a Release_Source event of the single Sporadic_Task bound to that Release_Signal (R10.1–R10.3) or, WHERE the Jorvik Profile_Variant binds several Tasks to the Release_Signal, of the one bound Task that R5.2 selects.
6. WHEN a sporadic Release becomes due (R10.1, R10.2), THE Kernel SHALL make that Release effective (R9.2) no earlier than the instant at which it becomes due and no later than J_rel after that instant.
7. THE Kernel SHALL attribute each MIT_Violation to the Partition that owns its event source: for an interrupt, the Partition that owns the peripheral (PR-29); for an Endpoint message, the sending Partition; for a Release_Signal, the Partition of the raising Task; and for R9.8, the Partition of the Task itself.
8. WHERE a Sporadic_Task's Release_Source is an Endpoint receive role, THE Kernel SHALL leave in the Endpoint buffer every message whose arrival R10.2 or R10.3 discards as an event, so that MIT enforcement discards Releases but never messages (R39.4).
9. THE Kernel SHALL apply R10.1–R10.3 to the interrupt events that reach the Kernel after any loss or coalescing that R19.4 records for the interrupt source (ORQ-23).

### Requirement 11: FPU context management (Phase P1)

**User Story:** As an application developer who uses floating point, I want my FP registers preserved across preemption with known costs, so that FP Tasks compute correctly and their timing is analyzable.

#### Acceptance Criteria

1. THE Kernel SHALL preserve the complete floating-point register state (S0–S31 and FPSCR) of each Job of an FPU-using Task across every preemption by another Job and every interruption by Kernel execution, so that on resumption the Job observes the values it held when it was preempted or interrupted.
2. THE Kernel SHALL implement one floating-point context-preservation policy, named in the Design_Document together with the list of floating-point instructions that the Kernel_Unsafe_Module executes under that policy (R6.6, PR-31), and THE Hardware_Model SHALL model that policy, including the lazy stacking that the FPCCR enables by default [RN-31] wherever the policy keeps lazy stacking enabled (ORQ-16).
3. THE Kernel SHALL export as the Kernel_Timing_Parameters Δ_fp_save and Δ_fp_restore the worst-case costs of saving and restoring floating-point state under that policy, including any save that the policy defers to a later instant (for example a lazy save triggered by the first floating-point instruction of a preempting Job), and THE Analyzer SHALL include these costs in every WCRT that they can increase (R29.3).
4. THE Link_Checker SHALL include in every stack bound, and in the bound of any other save area that the policy uses, the storage for floating-point state wherever a context with active floating-point state can be preempted or interrupted, including the extended exception frame (R37.1).
5. IF a fault occurs during floating-point context preservation, THEN THE Health_Monitor SHALL attribute the fault to the Partition whose floating-point state was being preserved only when the fault is an overflow of that Partition's stack region, and SHALL otherwise treat it as a Kernel fault (Requirement 14).
6. THE Kernel SHALL apply the workaround for each Cortex-M4F erratum that the Hardware_Model lists as applicable to the Target's core revision (ASM-01, ORQ-16), and THE Verification_Plan SHALL name the Evidence_Item that demonstrates each workaround.
7. WHERE any Task is declared FPU-using, THE Kernel SHALL enable the FPU during Init, before the first Release.
8. THE Kernel SHALL start every Job of an FPU-using Task with the floating-point control settings (rounding mode, flush-to-zero mode, and default-NaN mode) recorded in Generated_Config, whatever settings earlier Jobs left.

### Requirement 12: Boot-time configuration validation (Phase P1)

**User Story:** As a safety engineer, I want the Kernel to check at boot that the hardware matches the assumptions behind the proofs and the analysis, so that a mismatched board or configuration cannot run silently.

#### Acceptance Criteria

1. WHEN the Target leaves reset, THE Kernel SHALL verify, before the first Release, that each of the following matches the value recorded in Generated_Config: the CPUID core revision (ASM-01); the number of MPU regions (ASM-02); the number of implemented NVIC priority bits and the priority-grouping setting (ASM-03); the disabled state of the instruction cache (ASM-04); the running high-frequency clock source (ASM-05); where the Kernel time base uses the 32.768 kHz clock, the running low-frequency clock source (ASM-16); and, where any Task is FPU-using, the presence of the FPU and the floating-point context-preservation settings of R11.2.
2. IF any check of R12.1, R12.3, or R12.5 fails, THEN THE Kernel SHALL enter the system safe state recorded in Generated_Config without releasing any Task and SHALL record the identifier of the failed check where the HIL_Rig can read it.
3. WHEN Init completes, THE Kernel SHALL verify, before the first Release, that the checksum computed over the Generated_Config tables in memory equals the checksum recorded in Generated_Config (R24.1).
4. THE Kernel SHALL complete Init, the checks of R12.1, R12.3, and R12.5, and MPU configuration before enabling any interrupt that can cause a Release, including the interrupts of the timer hardware of R9.4 (PR-34).
5. WHERE a measurement build is configured, or the Budget enforcement of Requirement 18 uses the DWT cycle counter, THE Kernel SHALL verify the presence of the DWT cycle counter during boot validation (ASM-06).
6. THE HIL_Rig SHALL demonstrate, for each check of R12.1, R12.3, and R12.5, that a build whose Generated_Config records a value differing from the Target's, or whose Generated_Config tables are corrupted, enters the system safe state without releasing any Task and records the identifier of the failed check.

### Requirement 13: Kernel timing parameters (Phase P1 definition; P3 measurement)

**User Story:** As a timing analyst, I want every Kernel overhead bounded and exported, so that response-time analysis accounts for the Kernel itself.

#### Acceptance Criteria

1. THE Kernel SHALL define at least the Kernel_Timing_Parameters of Table 13-1, each as an upper bound in CPU cycles that holds for the Application's Generated_Config under every Application behaviour, including MIT_Violations, Overruns, and faults in any Partition.
2. THE Kernel SHALL provide each Kernel_Timing_Parameter as a WCET_Record (Requirement 33) that states whether the bound is measured or statically analysed and identifies the binary to which it applies.
3. THE Analyzer SHALL include every Kernel_Timing_Parameter in the WCRT computation (R29.3) and SHALL reject an analysis whose inputs lack a WCET_Record for any Kernel_Timing_Parameter (Requirement 28).
4. WHEN the WCET_Harness measures a Kernel operation, THE WCET_Harness SHALL measure each interval between the start and end instants that Table 13-1 defines for its parameter and SHALL report a failure if any observed execution or interval exceeds the exported bound.
5. THE Kernel SHALL complete every Kernel operation in a number of loop iterations bounded by Generated_Config constants, as proven by the Kernel_Proofs (PR-16).
6. WHERE the resolution of ORQ-01 or ORQ-14 introduces a Kernel operation that no row of Table 13-1 bounds, THE Design_Document SHALL define a Kernel_Timing_Parameter for that operation, to which R13.1–R13.4 apply.

**Table 13-1 — Kernel_Timing_Parameters** (each interval runs from the first instant named to the second)

| Symbol | Bound on |
|---|---|
| Δ_lock_in, Δ_lock_out | Lock entry, from the lock call to the first access to the Resource; lock exit, from the last access to the return from the lock operation (R8.8) |
| Δ_release | Making one Job ready: recording one Release as due and pending (R9.7, R10.1, R10.2), including, in the Jorvik Profile_Variant, selecting the released Task among the Tasks bound to a Release_Signal (R5.2) |
| Δ_dispatch, Δ_complete | Starting the Job that R7.2 and R7.3 select, including any MPU reconfiguration and privilege change; completing a Job and making the transition of R7.5 (R7.8) |
| Δ_timer | Handling one timed event of R9.4 (periodic release, Budget, deadline, or MIT deferral) |
| Δ_irq | Kernel handling of one interrupt event, including events that produce no Release and events discarded under R10.2 or R10.3 |
| Δ_stamp | From a Release_Source interrupt event to the Kernel's timestamp of that event |
| L_kernel | Longest Kernel-internal section that masks preemption by any Job |
| Δ_fp_save, Δ_fp_restore | Floating-point state save and restore under the policy of R11.2, including saves that the policy defers (R11.3) |
| Δ_detect, Δ_enforce | Δ_detect: from the instant a Job's consumed execution time exceeds its Budget, or the time base reaches the Job's absolute deadline (R9.6), to the Health_Monitor's receipt of the report; Δ_enforce: from that receipt to the completion of the Overrun_Response |
| Δ_hm | Any Health_Monitor response other than an Overrun_Response |
| Δ_wake | Wake-up from the idle activity (R7.8) |
| J_rel | Release jitter: from the instant a Release becomes due (the nominal instant of a periodic Release, or the accepted event instant or deferred instant of a sporadic Release) to the instant it becomes effective (R9.2, R10.6) |

### Requirement 14: Fault handling, panic policy, and Health_Monitor (Phase P1)

**User Story:** As a safety engineer, I want every fault, panic, and violation routed to a health monitor that applies a declared, partition-specific response, so that faults are contained and visible.

#### Acceptance Criteria

1. THE Build_System SHALL build every Flight_Build with the abort panic strategy and with the Kernel's panic handler as the only panic handler, and SHALL fail the build if the Link_Checker finds unwinding symbols or a second panic handler in the linked binary (PR-19).
2. WHEN a panic occurs, THE Kernel SHALL deliver to the Health_Monitor, without unwinding, a fault event that names the running Task and its Partition or, for a panic in Kernel code, names the Kernel (PR-19).
3. WHEN a MemManage, BusFault, UsageFault, or HardFault exception occurs, THE Health_Monitor SHALL attribute the fault to the Partition whose execution caused it, or to the Kernel where Kernel execution caused it, and SHALL apply the response declared for that Partition, or R14.4 for the Kernel; THE Design_Document SHALL state how each fault class is attributed, including imprecise BusFaults and faults during exception entry or floating-point state preservation (R11.5).
4. IF a fault is attributed to the Kernel, THEN THE Health_Monitor SHALL record the fault in the event log (R14.6) and place the Target in the system safe state recorded in Generated_Config.
5. THE Profile SHALL define the closed sets of Partition fault responses and Overrun_Responses, stating for each response: its effect on the Partition's Tasks, Resources, Endpoint roles, and peripherals; whether it ends the running Job, stops the Partition, or restarts the Partition (R4.7); which fault responses can serve as the deadline-miss response of R15.1; that a stop or restart first stops the EasyDMA transfers of the Partition's peripherals (R17.6); how each Resource released under R8.9 reaches a defined state before any Job next accesses it; and, for a restart, that the first Release of each Periodic_Task is its next nominal instant (R9.3) and that the MIT history of each Sporadic_Task (R10.1) is preserved, so that no two Releases of a Task become due less than one period or MIT apart.
6. THE Health_Monitor SHALL record each event in an event log whose capacity is fixed in Generated_Config, with its event type, Task, Partition, time-base instant, and the PR identifier of every restriction concerned (R3.9), where the event types include at least panics, the exceptions of R14.3, Overruns, deadline misses (R9.6), release overlaps (R9.7), MIT_Violations (R9.8, R10.2, R10.3), and the end of the operating duration (R9.9).
7. IF the event log is full when an event arrives, THEN THE Health_Monitor SHALL overwrite the oldest entry and increment a saturating overflow counter.
8. THE Health_Monitor SHALL complete each Overrun_Response within Δ_enforce and each other response within Δ_hm (Table 13-1).
9. WHEN a Partition is stopped or restarted, THE Kernel SHALL leave the Releases, Resources, Endpoint roles, and Budgets of every other Partition unchanged, except that Endpoints connected to the affected Partition report the peer state change (R42.3).
10. THE Kernel SHALL complete the work of stopping or restarting a Partition, including stopping its EasyDMA transfers (R17.6) and re-initializing its Resources and Endpoint roles, within a bound recorded in Generated_Config, and THE Analyzer SHALL include that work as interference in the WCRT of every Task whose response time it can increase (R29.3).
11. THE HIL_Rig SHALL inject, for each response in the closed sets of R14.5, a fault that triggers the response in a Partition declaring it, and SHALL report a pass only when the observed effects match those that R14.5 states and R14.9 holds for every other Partition.

### Part P — Mixed-Criticality Partitioning (cross-cutting)

### Requirement 15: Partition declaration (Phase P1 Kernel; P2 Generator)

**User Story:** As a system integrator, I want to declare Partitions with criticality levels and owned resources, so that software of different DO-178C levels can share one MCU.

#### Acceptance Criteria

1. THE Generator SHALL accept Partition declarations that state a name, a Criticality_Level from {A, B, C, D, E}, code, data, and stack region sizes, owned peripherals, an MIT_Violation threshold (R10.4), and, each chosen from the closed sets of R14.5, a fault response, an Overrun_Response, and a deadline-miss response that the Health_Monitor applies to deadline misses (R9.6) and release overlaps (R9.7) of the Partition's Tasks.
2. THE Generator SHALL reject an App_Declaration in which a Task, Resource, Endpoint role, peripheral, or memory region, other than an item owned by the Kernel, is not assigned to exactly one Partition (PR-28).
3. THE Generator SHALL reject an App_Declaration in which Tasks of more than one Partition access the same Resource (PR-30).
4. THE Generator SHALL export each Partition in the Task_Model with its Criticality_Level, members, region sizes, MIT_Violation threshold, and responses.
5. WHERE an App_Declaration declares a single Partition, THE Generator and THE Kernel SHALL apply every partitioning requirement to that single Partition, including its separation from the Kernel (R16.1).
6. THE Generator SHALL reject an App_Declaration that assigns to a Partition a peripheral that Generated_Config assigns to the Kernel, including the timer hardware of R9.4, naming the peripheral and PR-29.
7. THE Generator SHALL reject an App_Declaration in which a Partition omits a field that R15.1 requires or declares a response outside the closed sets of R14.5, naming the Partition and the field.

### Requirement 16: Spatial isolation (Phase P1)

**User Story:** As a certification engineer, I want robust spatial partitioning (DO-178C §2.4), so that no Partition can corrupt another Partition's memory, stack, or peripherals.

#### Acceptance Criteria

1. THE Kernel SHALL prevent each Partition from writing memory owned by another Partition or by the Kernel, including stacks, Resource storage, Endpoint storage (which the Kernel owns, R40.1), Kernel data, and Generated_Config tables.
2. THE Kernel SHALL prevent each Partition from reading or writing the registers of peripherals that the Partition does not own, including the core peripherals that PR-22 reserves to the Kernel, because peripheral reads can have side effects.
3. THE Kernel SHALL restrict instruction fetch by each Partition to code regions assigned to that Partition and to shared code regions recorded in Generated_Config.
4. IF a Partition attempts an access prohibited by R16.1–R16.3 in an access class that the Design_Document records as hardware-enforced under R16.9, THEN THE Kernel SHALL suppress the access, raise a fault attributed to that Partition, and invoke the Health_Monitor (R14.3).
5. THE Kernel SHALL detect each overflow of each stack region before the overflow modifies memory outside that region, and SHALL raise a fault attributed to the Partition that owns the stack region, or to the Kernel for a Kernel stack (R14.3).
6. THE Kernel_Proofs SHALL prove that the MPU configuration computed for each Partition grants that Partition no access to any address owned by another Partition or by the Kernel, under the Target's region-count and alignment rules (R44.4, ASM-02).
7. THE Config_Checker SHALL verify R16.1–R16.3 against the MPU configuration and memory map of each linked Flight_Build, and SHALL report each address range that more than one Partition can write (Requirement 26).
8. THE HIL_Rig SHALL execute, for every ordered pair of Partitions in the Profile_Conformance_Suite Application and for every Partition against the Kernel, an access attempt to each protected region class (code, data, stack, Endpoint storage, peripheral, Kernel), and SHALL report a pass only when every attempt in a hardware-enforced access class produces the outcome of R16.4.
9. THE Design_Document SHALL state, for each Criticality_Level, whether isolation relies on unprivileged execution with hardware enforcement or on privileged execution with MPU restrictions plus language-level guarantees, and THE Trust_Base_Register SHALL list the residual trusted components of each choice (ORQ-01).
10. WHERE R16.9 records that an access class of a Partition is excluded by language-level guarantees instead of hardware enforcement, THE Link_Checker SHALL verify that the code reachable from that Partition's Task bodies contains no instruction or address constant that performs an access of that class (PR-22), and THE Trust_Base_Register SHALL list that check.

### Requirement 17: DMA isolation (Phase P1)

**User Story:** As a certification engineer, I want DMA-capable peripherals contained, so that a Partition cannot corrupt another Partition's memory through a bus master that the CPU's MPU does not check.

#### Acceptance Criteria

1. THE Kernel SHALL ensure that every EasyDMA transfer of a peripheral owned by Partition P reads and writes only statically allocated memory owned by P, excluding stack regions and Endpoint storage (EasyDMA is an AHB bus master with direct Data RAM access [RN-35]).
2. IF a Partition requests a DMA transfer whose buffer lies outside the memory that R17.1 permits for that Partition, THEN THE Kernel SHALL refuse the request before any byte is transferred and report a spatial violation attributed to that Partition to the Health_Monitor (R14.3).
3. THE Design_Document SHALL state the mechanism that enforces R17.1 when the owning Partition could write the peripheral's DMA pointer registers directly, and THE Verification_Plan SHALL name the evidence for that mechanism (ORQ-03).
4. THE WCET_Harness SHALL include worst-case concurrent EasyDMA traffic of all Partitions in the measurement scenarios of every Task (R34.5, ASM-07).
5. WHILE an EasyDMA transfer is in progress, THE Kernel SHALL ensure that no Job reads or writes the transfer's buffer other than through the peripheral's transfer-completion interface, so that DMA does not break data-race freedom within a Partition (PR-05, ORQ-21).
6. WHEN a Partition is stopped or restarted, THE Kernel SHALL stop every EasyDMA transfer of the Partition's peripherals before re-initializing or reassigning any memory that those transfers can access (R14.9).
7. THE HIL_Rig SHALL demonstrate, for each EasyDMA-capable peripheral class that the Profile_Conformance_Suite Application uses, that a transfer request with a buffer outside the memory R17.1 permits is refused before any byte is transferred (R17.2), and that a Partition restart stops the Partition's transfers before its memory is re-initialized (R17.6).

### Requirement 18: Temporal isolation and Budget enforcement (Phase P1)

**User Story:** As a certification engineer, I want every Job's execution time budgeted, every overrun detected, and the response chosen by criticality, so that a lower-criticality Partition cannot make a higher-criticality Partition miss deadlines.

#### Acceptance Criteria

1. THE Kernel SHALL measure the execution time consumed by each Job against the Job's Budget, excluding intervals during which the Job is preempted by another Job or interrupted by Kernel execution performed for another Task, so that no other Partition's activity consumes the Job's Budget.
2. WHEN a Job's consumed execution time exceeds its Budget, THE Kernel SHALL detect the Overrun within Δ_detect, whatever the System_Ceiling at that instant and including while the Job holds Resources, and report the Overrun to the Health_Monitor.
3. WHEN an Overrun is reported, THE Health_Monitor SHALL apply the Overrun_Response declared for the Job's Partition within Δ_enforce.
4. WHILE the Partition of an overrunning Job continues to execute after the Overrun_Response, THE Kernel SHALL ensure that every Job of that Partition observes each Resource of that Partition only in a state produced by a completed Critical_Section or by the Resource's initialization (R8.9, R14.5).
5. THE Generator SHALL reject an App_Declaration in which a Partition whose Criticality_Level is lower than the highest Criticality_Level in the Application declares an Overrun_Response that lets an overrunning Job continue executing.
6. WHERE a Partition has the highest Criticality_Level in the Application, THE Generator SHALL accept an Overrun_Response that records the Overrun and lets the Job complete.
7. WHILE every Task of Partition P executes within its Budget and every Release_Source of P respects its MIT, THE Kernel SHALL complete each Job of P within the WCRT computed by the Analyzer, regardless of the execution behaviour of Tasks in Partitions whose Overrun_Response stops overrunning Jobs and of MIT_Violations (Requirement 10) and faults (Requirement 14) in any other Partition, where the Analyzer bounds the interference and blocking that P suffers from those Partitions by values that the Kernel enforces (R29.4).
8. THE HIL_Rig SHALL demonstrate R18.7 by fault injection in which Tasks of lower-criticality Partitions execute unbounded loops, exceed their Budgets both outside and inside Critical_Sections, hold Resources for their maximum Critical_Section lengths, and exceed the MITs of their Release_Sources, and SHALL report a pass only when no Job of the highest-criticality Partition exceeds its computed WCRT during PAR-05.
9. WHERE more than one Partition has the highest Criticality_Level and any of them declares an Overrun_Response that lets an overrunning Job continue executing, THE Analyzer SHALL state in the analysis report (Requirement 30) that R18.7 does not protect those Partitions from one another.

### Requirement 19: Interrupt-source isolation (Phase P1)

**User Story:** As a system integrator, I want interrupt storms from one Partition's peripherals contained, so that they cannot take unbounded processor time from other Partitions.

#### Acceptance Criteria

1. THE Kernel SHALL limit the processor time consumed by the handling of each interrupt source, including Kernel handling of events that produce no Release, to at most ⌈t / MIT⌉ · Δ_irq in every time interval of length t, where MIT is that of the Task bound to the source, whatever rate the source generates events at.
2. THE Analyzer SHALL model each interrupt source as interference of ⌈t / MIT⌉ · Δ_irq in every interval of length t on every Task whose execution that handling can preempt, in addition to the interference of the bound Task's accepted Releases (R29.3).
3. THE HIL_Rig SHALL drive each interrupt source of the Profile_Conformance_Suite Application at the highest rate the source can generate for PAR-05, and SHALL report a pass only when the processor time attributed to that source satisfies R19.1 and every Job of other Partitions completes within its computed WCRT.
4. THE Design_Document SHALL state, for each interrupt source, whether events arriving within an MIT window are lost, coalesced, or deferred, and THE Generator SHALL require each interrupt-bound Task to declare which of these behaviours the Task accepts and SHALL reject a declaration that the Design_Document does not provide for that source (ORQ-23).
5. THE Kernel SHALL keep disabled every interrupt source that Generated_Config binds neither to a Task nor to the Kernel, and SHALL report to the Health_Monitor as a Kernel fault any exception from an interrupt source that Generated_Config does not bind (PR-23).

### Requirement 20: Privilege model resolution (Phase P1)

**User Story:** As a kernel developer, I want the conflict between hardware-accelerated SRP and privilege-separated Partitions recorded and resolved explicitly, so that the design weakens neither guarantee silently.

#### Acceptance Criteria

1. THE Design_Document SHALL record the resolution of ORQ-01, stating which code executes privileged, how code of each Partition raises and restores the System_Ceiling, how Handler-mode execution (always privileged on Armv7-M [RN-30]) is reconciled with unprivileged Partitions, how the Kernel detects Overruns whatever the System_Ceiling (R18.2), and the Kernel_Timing_Parameters that the mechanism adds (R13.6).
2. THE Verification_Plan SHALL state, for each Criticality_Level used in the Profile_Conformance_Suite Application, whether the selected mechanism satisfies Requirements 16, 17, 18, and 19 by hardware enforcement, by proof, by a static check of the Link_Checker (R16.10), or by an assumption listed in the Trust_Base_Register.
3. THE Kernel_Proofs SHALL prove R7.1–R7.5 and R8.1–R8.5 under the selected mechanism.
4. THE Generator SHALL reject an App_Declaration whose MPU layout cannot be realized within PAR-07 regions and the Armv7-M region size and alignment rules, naming the Partition whose layout fails (ORQ-02).
5. THE Design_Document SHALL compare the selected mechanism with each other candidate that ORQ-01 lists on Kernel size (PAR-01), Kernel_Timing_Parameters, residual trusted components, and required evidence, and SHALL state why each other candidate was rejected.

### Part C — Task-Model DSL

### Requirement 21: App_Declaration content (Phase P2)

**User Story:** As an application developer, I want to declare my whole system in one proc-macro block, so that configuration, ceilings, and the analyzable model come from a single source of truth.

#### Acceptance Criteria

1. THE Generator SHALL accept an App_Declaration that declares: the Target and its hardware parameters; the Profile_Variant; Partitions (R15.1); Tasks with release kind, period or MIT, offset, relative deadline, Priority, core, Partition, Budget, FPU use, MIT policy, Release_Source, accepted MIT-window behaviour for interrupt Release_Sources (R19.4), accessed Resources, and Endpoint roles; Resources with type and initialization; Release_Signals with their bound Task (or, in the Jorvik Profile_Variant, Tasks) and the Tasks that may raise them; Endpoints with Protocol, roles, role-to-Task bindings, and capacities; peripheral ownership; the Init_Arena option and size (PR-15); the system safe state; and the maximum continuous operating duration.
2. THE Generator SHALL accept time quantities only with explicit units (cycles, ticks, ns, µs, ms, s) and SHALL reject a time quantity without a unit; it SHALL convert each to Kernel time-base ticks and CPU cycles, rounding in the direction that cannot make the analysis optimistic (Budgets up; periods, MITs, and deadlines down) and rounding offsets to the nearest tick, SHALL use the same converted value in Generated_Config and the Task_Model, and SHALL report every conversion that changes a value.
3. THE Generator SHALL reject a declaration that omits any attribute required by PR-27, PR-28, or R21.1.
4. THE Generator SHALL report each error at the source span of the offending item in the App_Declaration, naming the PR identifier or requirement that the item violates (R3.1).
5. WHERE a Task is bound to an interrupt Release_Source, THE Generator SHALL require the interrupt to exist in the Target's device description.
6. THE Generator SHALL reject an App_Declaration in which a Task is bound to the interrupt of a peripheral that the Task's Partition does not own (PR-29).

### Requirement 22: Declaration validation (Phase P2)

**User Story:** As an application developer, I want inconsistent or unanalyzable declarations rejected at compile time, so that every accepted Application satisfies the Profile.

#### Acceptance Criteria

1. THE Generator SHALL reject a Task whose period or MIT, after the conversion of R21.2, is zero, whose relative deadline is zero or exceeds its period or MIT, or whose Budget is zero or exceeds its relative deadline.
2. THE Generator SHALL reject an App_Declaration whose number of distinct Task Priorities plus Kernel-reserved levels exceeds PAR-06 (PR-33; 3 NVIC priority bits on the nRF52840 [RN-25]).
3. THE Generator SHALL reject an App_Declaration in which an interrupt or Endpoint receive role releases more than one Task, or in which a Release_Signal releases more than one Task or, in the Jorvik Profile_Variant, more Tasks than its declared maximum waiter count (PR-12, R5.2).
4. THE Generator SHALL reject an App_Declaration in which a Task has more than one Release_Source, a Periodic_Task has any Release_Source, or a Sporadic_Task has none (PR-11).
5. THE Generator SHALL reject an App_Declaration that declares a Resource with no accessing Task, an Endpoint role bound to no Task, or a Release_Signal bound to no Task.
6. THE Generator SHALL reject an App_Declaration whose Kernel time base cannot represent every declared period, offset, and deadline, or cannot operate for the declared maximum continuous operating duration without wrap-around ambiguity (R9.5).
7. THE Generator SHALL reject an App_Declaration containing an Endpoint whose Protocol the Protocol_Checker rejects (R38.5).

### Requirement 23: Ceiling and derived-value computation (Phase P2)

**User Story:** As an application developer, I want Ceilings and other derived values computed automatically and exported, so that I never hand-maintain values that the proofs and the analysis depend on.

#### Acceptance Criteria

1. THE Generator SHALL compute Ceiling(r) of each Resource r as the maximum Priority of the Tasks declared to access r, consistent with RTIC's ownership and ceiling analysis [RN-06], and the Ceiling of each Endpoint buffer as Requirement 40 defines.
2. THE Generator SHALL compute the same Ceilings for every permutation of the declaration order of Tasks, Resources, and Endpoints.
3. THE Generator SHALL compute, for each Task, the set of Resources the Task may lock directly or nested and the maximum lock-nesting depth, which does not exceed the number of Resources declared for the Task (PR-07, R8.10).
4. THE Generator SHALL compute the capacity of each Kernel queue from the declarations (PR-14), including the timer queue by the rule of R9.4, and SHALL take each Endpoint buffer capacity from the Protocol_Checker bound (R41.1).
5. THE Generator SHALL export every derived value (Ceilings, nesting depths, capacities, priority-to-hardware mapping, Kernel-reserved levels, MPU layout) in the Task_Model, marked as derived.
6. WHEN the Generator runs twice on the same App_Declaration with the same Generator version, THE Generator SHALL produce identical derived values.

### Requirement 24: Generated configuration (Phase P2)

**User Story:** As a kernel developer, I want the Kernel's configuration tables generated from the declaration, so that the run-time configuration and the analyzed model cannot diverge.

#### Acceptance Criteria

1. THE Generator SHALL emit Generated_Config containing the task table, interrupt bindings and priorities, timer table, Ceilings, queue capacities, Budget and MIT tables, MPU configuration per Partition, stack region sizes, Health_Monitor policy table, the expected Target values that R12.1 checks, the Kernel-owned peripherals (R15.6), the floating-point control settings (R11.8), the Partition stop and restart bounds (R14.10), the system safe state, the maximum continuous operating duration (R9.9), in the Jorvik Profile_Variant the declared maximums of R5.2 and R5.7, and a checksum over these tables.
2. THE Generator SHALL emit code that contains no `unsafe` blocks, `unsafe fn`, `unsafe impl`, or assembly, so that Application crates compile under `#![forbid(unsafe_code)]` (PR-20), and THE Design_Document SHALL record this as a deviation from RTIC's generated code where applicable (R61).
3. THE Generator SHALL emit a human-readable configuration summary that lists every value of R24.1 for review.
4. WHEN the Generator runs twice on identical inputs with the same Generator version, THE Generator SHALL emit byte-identical Generated_Config and Task_Model files.
5. THE Generator SHALL compute an MPU layout within the Target's constraints or reject the App_Declaration per R20.4.
6. THE Generator SHALL emit Generated_Config as constant data that neither the Kernel nor any Partition modifies after Init, so that the values checked by R12.3 and the Config_Checker are the values in force until reset.

### Requirement 25: Task_Model export (Phase P2)

**User Story:** As a timing analyst, I want a versioned, machine-readable task model, so that the Analyzer, the Config_Checker, and external tools consume exactly what the Generator saw.

#### Acceptance Criteria

1. THE Generator SHALL export the Task_Model as a JSON document that conforms to a published, versioned JSON Schema, and SHALL validate each document against that schema before writing it.
2. THE Generator SHALL include in the Task_Model the Profile version, Profile_Variant, Generator version, Kernel version, Target identifier, the Generated_Config checksum (R24.1), every declared value of R21.1, every derived value of R23.5, and a stable identifier for each Task, Resource, Endpoint, Critical_Section site, release condition (R5.5), and Partition.
3. THE Generator SHALL include in the Task_Model the identifiers that the WCET_Harness and the Static_WCET_Analyzer use to associate WCET_Records with each Task entry point, each Critical_Section site, and each release condition.
4. THE Generator SHALL represent every time value in the Task_Model as an integer with an explicit unit (CPU cycles or Kernel time-base ticks) and SHALL emit keys in canonical order, so that equal models produce byte-identical documents.
5. THE Analyzer and THE Config_Checker SHALL each parse every document that the Task_Model printer produces for a valid Task_Model m into a Task_Model equal to m (round-trip property).
6. IF a Task_Model document fails schema validation or declares a schema version that the reading tool does not support, THEN THE Analyzer and THE Config_Checker SHALL reject the document and report each violated schema rule with its JSON path, or the unsupported version.

### Requirement 26: Independent configuration check (Phase P2)

**User Story:** As a certification engineer, I want the generated configuration checked independently against the binary, so that the Generator's output is verified and the Generator need not be qualified at TQL-1 (DAL A) or TQL-2 (DAL B).

#### Acceptance Criteria

1. THE Config_Checker SHALL be implemented without sharing derivation code, parsing code, or data structures with the Generator, except the Task_Model schema.
2. THE Config_Checker SHALL recompute, from the declared values in the Task_Model, the Ceilings, the priority-to-hardware mapping, the nesting depths, the capacities of R23.4 (including the timer-queue rule of R9.4), the Budget and MIT tables, the Kernel-owned peripheral assignment (R15.6), and the MPU permissions, and SHALL compare them with the derived values in the Task_Model and with the values extracted from the linked binary; it SHALL also check each stack region size against the Link_Checker's stack bound for that region (R37.4).
3. IF any recomputed value differs from the Task_Model or from the linked binary, THEN THE Config_Checker SHALL exit with non-zero status and report the item, the expected value, and the found value.
4. THE Config_Checker SHALL extract configuration values from the linked binary itself, not from Generator intermediate files, and SHALL verify that the Generated_Config checksum stored in the binary matches the extracted tables and the checksum recorded in the Task_Model (R25.2).
5. THE Build_System SHALL run the Config_Checker on every Flight_Build and fail the build on any discrepancy.
6. THE Verification_Plan SHALL classify the Config_Checker as a verification tool and state its DO-330 criteria and TQL (Requirement 51).
7. THE Profile_Conformance_Suite SHALL contain, for each value class of R26.2, a linked binary or Task_Model in which a value of that class was altered after generation, and THE Build_System SHALL report each case as passing only when the Config_Checker reports the altered item.

### Requirement 27: Partitioned multi-core readiness (Phase P2)

**User Story:** As a researcher, I want the v1 task model to carry core assignments while v1 tools reject multi-core models, so that partitioned multi-core can be added later without breaking the schema.

#### Acceptance Criteria

1. THE Task_Model schema SHALL contain a core field for every Task, a core-or-global field for every Resource, and a core pair for every Endpoint, each of them required in every v1 Task_Model with no default value.
2. WHEN an App_Declaration or Task_Model assigns any item to a core other than core 0, or marks a Resource global, THE Generator, THE Analyzer, and THE Config_Checker SHALL reject the input with a diagnostic that names the item and cites NG-01.
3. THE Profile SHALL reserve restriction identifiers, marked inactive in v1, for static Task-to-core assignment, per-core SRP, and cross-core sharing only through Endpoints or a spin-based global resource protocol (MSRP or MrsP).
4. THE Kernel_Proofs SHALL state the single-core assumption as an explicit precondition, so that the extension can identify every proof that depends on that assumption.
5. THE Analyzer SHALL state in every analysis report that the analysis assumes a single core, so that no report can be read as evidence for a multi-core configuration.

### Part D — Schedulability Analyzer

### Requirement 28: Analyzer inputs (Phase P2)

**User Story:** As a timing analyst, I want the Analyzer to consume the Task_Model, WCET_Records, and Kernel_Timing_Parameters and to reject inconsistent inputs, so that the result applies to the binary I will deploy.

#### Acceptance Criteria

1. THE Analyzer SHALL accept one Task_Model, one WCET_Record set, one Kernel_Timing_Parameters set, and one Build_Manifest, each as JSON conforming to its published schema, and SHALL reject any input that fails its schema, reporting each violated rule with its JSON path (R25.6).
2. IF any Task entry, Critical_Section site, release condition (R5.5), or Kernel_Timing_Parameter referenced by the Task_Model lacks a WCET_Record, THEN THE Analyzer SHALL return FAIL and list the missing items.
3. IF the binary hash in a WCET_Record differs from the binary hash in the Build_Manifest, or the record's clock, cache, or build configuration differs from the Build_Manifest (R33.4), THEN THE Analyzer SHALL reject that WCET_Record.
4. IF the WCET of any Task exceeds the Task's Budget, THEN THE Analyzer SHALL return FAIL and name the Task.
5. WHEN the Task_Model assigns any item to a core other than core 0, THE Analyzer SHALL reject the input (R27.2).
6. IF the Task_Model uses a construct, Profile version, or Profile_Variant that the Analyzer version does not support, THEN THE Analyzer SHALL reject the input and name the construct or version.
7. WHERE both a measured and a static WCET_Record exist for the same item, THE Analyzer SHALL use the larger value unless the analysis configuration designates one method, and SHALL report the record used.
8. IF the Generated_Config checksum recorded in the Task_Model differs from the checksum that the Build_Manifest records for the analysed binary, THEN THE Analyzer SHALL reject the input, so that the analysed model is the model of the binary.

### Requirement 29: Response-time analysis with SRP blocking (Phase P2)

**User Story:** As a timing analyst, I want fixed-priority response-time analysis with SRP blocking and Kernel overheads, so that I obtain a sound bound on every Task's worst-case response time.

#### Acceptance Criteria

1. THE Analyzer SHALL compute, for each Task i, the WCRT R_i = J_i + w_i, where w_i is the least fixed point of w_i = C_i + B_i + O_i + Σ_{j ∈ hep(i), j ≠ i} ⌈(w_i + J_j) / T_j⌉ · E_j; here C_i is Task i's Budget (which R28.4 keeps at or above Task i's WCET), B_i the blocking term of R29.2, O_i the Kernel overhead of R29.3 attributable to Task i's own Job, J the release jitter, equal to J_rel for every Task (Table 13-1), T the period or MIT, E_j the interference bound of R29.4, and hep(i) the Tasks whose Priority is at least Task i's Priority; every offset is treated as zero (synchronous release).
2. THE Analyzer SHALL compute B_i as the maximum of L_kernel and, over every Critical_Section site, including Endpoint operations, of a Task l whose Priority is lower than Task i's Priority on a Resource whose Ceiling is at least Task i's Priority, of the site's WCET including the Critical_Sections nested in it (R8.4), except that for a site of Application code in a Task l of another Partition whose Overrun_Response stops overrunning Jobs, it SHALL use Budget_l + Δ_detect + Δ_enforce, so that the bound holds while Task l overruns inside the Critical_Section (R18.7).
3. THE Analyzer SHALL include the Kernel overheads of Table 13-1: Δ_release, Δ_dispatch, Δ_complete, and Δ_wake per Job; Δ_timer per timed event of R9.4, including Budget, deadline, and MIT-deferral events; interrupt handling in the form of R19.2; Δ_fp_save and Δ_fp_restore per preemption that involves an FPU-using Task (R11.3); Δ_lock_in plus Δ_lock_out per Critical_Section; Δ_hm per Health_Monitor response that can occur in the analysed interval; and the Partition stop and restart work of R14.10.
4. THE Analyzer SHALL use as E_j: Budget_j + Δ_detect + Δ_enforce + L_j plus Task j's per-Job overheads of R29.3, where L_j is Task j's longest Critical_Section, for Tasks in Partitions whose Overrun_Response stops overrunning Jobs; and Budget_j plus its per-Job overheads for all other Tasks.
5. THE Analyzer SHALL report the Task set as PASS only if R_i ≤ D_i for every Task i and no input was rejected under Requirement 28.
6. THE Analyzer SHALL perform all timing arithmetic on unsigned integers in CPU cycles and SHALL report an error instead of a result on any arithmetic overflow.
7. IF the fixed-point iteration for Task i exceeds D_i, THEN THE Analyzer SHALL stop iterating for Task i and report Task i as not schedulable, with the last iterate as a lower bound on R_i.
8. THE Analyzer SHALL convert Kernel time-base ticks to CPU cycles with the clock ratio recorded in the Task_Model, rounding periods, MITs, and deadlines down and execution times and jitter up (ASM-16).
9. IF the sum over all Tasks of E_j / T_j exceeds 1, THEN THE Analyzer SHALL report FAIL without fixed-point iteration.

### Requirement 30: Analysis report (Phase P2)

**User Story:** As a certification engineer, I want a pass/fail report with per-Task worst-case response times and provenance, so that the analysis can be reviewed and archived as evidence.

#### Acceptance Criteria

1. THE Analyzer SHALL produce a schema-versioned JSON report and a human-readable report containing the overall result (PASS or FAIL), the statements that R18.9 and R27.5 require, and, for each Task, the Partition and its Criticality_Level, the Priority, period or MIT, deadline, C_i, B_i with the Critical_Section that determines B_i, the interference terms, R_i or, for a Task reported as not schedulable, the lower bound of R29.7, and the slack D_i − R_i.
2. THE Analyzer SHALL record in each report the hashes of the Task_Model, the WCET_Record set, the Kernel_Timing_Parameters, and the Build_Manifest, the Profile version and Profile_Variant recorded in the Task_Model, and the Analyzer version.
3. THE Analyzer SHALL report, for each WCET value used, the provenance of its WCET_Record (method, tool, conditions).
4. THE Analyzer SHALL exit with status 0 only when the result is PASS, and SHALL use distinct non-zero statuses for FAIL, rejected input, and internal error.
5. WHEN the Analyzer runs twice on identical inputs, THE Analyzer SHALL produce identical reports, excluding a timestamp field.

### Requirement 31: UPPAAL model export (Phase P2)

**User Story:** As a researcher, I want the task set exported as UPPAAL timed automata with a schedulability query, so that I have an independent cross-check of the response-time analysis.

#### Acceptance Criteria

1. THE UPPAAL_Exporter SHALL emit the model in UPPAAL's XML format (`.xml`) and the queries in a separate query file (`.q`) [RN-24].
2. THE UPPAAL_Exporter SHALL model release generators for each Periodic_Task, release generators constrained by the MIT for each Sporadic_Task, release jitter up to J_rel, each Task's execution time as an interval bounded above by its Budget (R29.1), fixed-priority preemptive dispatching with equal-priority ties broken in the order of R7.3, the SRP start condition with Resource Ceilings, Critical_Section durations, the Kernel overheads of R29.3, and a deadline-miss location for each Task, following published UPPAAL schedulability frameworks where applicable [RN-24]; WHERE the model omits or approximates an overhead term of R29.3, THE Analyzer SHALL record the omission in the report.
3. THE UPPAAL_Exporter SHALL emit a query that is satisfied if and only if no deadline-miss location is reachable.
4. WHERE UPPAAL's command-line verifier is installed, THE Analyzer SHALL run the query, record the verifier's verdict and version in the report, and compare the verdict with the RTA result.
5. IF the verifier reports a reachable deadline miss while the RTA result is PASS, THEN THE Analyzer SHALL report a cross-check discrepancy, attach the verifier's trace, and mark the trace as requiring confirmation, because stopwatch models are verified by over-approximation [RN-24].
6. WHEN the RTA result is FAIL and the verifier finds no reachable deadline miss, THE Analyzer SHALL report the difference as analysis pessimism, not as a discrepancy.
7. THE UPPAAL_Exporter SHALL record in the model, for each automaton, the Task_Model identifier the automaton represents.
8. THE Analyzer SHALL provide a reader for exported models, and for every valid Task_Model m the reader SHALL recover from the export of m each Task's Priority, period or MIT, deadline, and execution-time bound equal to the values in m (round-trip property).
9. IF the verifier exceeds the time or memory limit set in the analysis configuration, THEN THE Analyzer SHALL report the cross-check as inconclusive, not as agreement (ORQ-22).

### Requirement 32: Analyzer correctness evidence (Phase P2)

**User Story:** As a verification engineer, I want the Analyzer's own correctness backed by proof where feasible and by property-based tests and cross-checks, so that the analysis tool does not become an unverified weak link.

#### Acceptance Criteria

1. THE Verification_Plan SHALL include a proof, in Verus or the App_Prover, that the Analyzer's fixed-point computation terminates and returns either the least fixed point of the R29.1 equation, a correct not-schedulable verdict (R29.7), or the overflow error of R29.6.
2. THE Analyzer SHALL pass property-based tests showing that increasing any Budget, WCET, blocking term, jitter, or Kernel_Timing_Parameter, or decreasing any period or MIT, never decreases any computed WCRT (monotonicity).
3. THE Analyzer SHALL pass property-based tests showing that adding, removing, or modifying a Task of lower Priority than Task i that accesses no Resource whose Ceiling is at least Task i's Priority changes the WCRT of Task i by no more than the Kernel overhead terms of R29.3 that the changed Task contributes (metamorphic property).
4. THE Analyzer SHALL agree with an independently written reference implementation of the R29 equations on every generated Task_Model (model-based testing).
5. THE Analyzer SHALL pass property-based tests in which a discrete-event simulator of SRP scheduling, including simulated Overruns by Tasks of Partitions whose Overrun_Response stops overrunning Jobs, observes for every generated Task_Model reported as PASS no response time greater than the computed WCRT.
6. THE Analyzer SHALL be cross-checked against UPPAAL on a corpus of at least PAR-03, and THE Verification_Plan SHALL record each discrepancy and its disposition.
7. THE Analyzer SHALL reproduce the response times of at least PAR-04 task sets published in the real-time scheduling literature, with each Budget set equal to the published execution time.

### Part E — WCET and Resource Bounds

### Requirement 33: WCET record flow (Phase P2 format; P3 measurement)

**User Story:** As a timing analyst, I want WCET values to carry provenance and to be bound to a specific binary, so that the Analyzer never consumes stale or mismatched timing data.

#### Acceptance Criteria

1. THE WCET_Harness and THE Static_WCET_Analyzer SHALL emit WCET_Records as JSON conforming to a published, versioned schema, each record stating the item identifier (Task entry, Critical_Section site, Endpoint operation, release condition, or Kernel operation), the WCET in CPU cycles, the method (measured or static), the tool and version, the binary hash, the Target and core revision, the clock and cache configuration, the conditions, and, for measured records, the number of runs, the maximum observed value, the applied margin, the interference scenarios (R34.5), and the coverage achieved (R34.4).
2. THE Analyzer SHALL parse every document that the WCET_Record printer produces for a valid record set into a record set equal to the original (round-trip property).
3. THE Build_System SHALL compute the binary hash over the loadable sections of each Flight_Build and record the hash and the Generated_Config checksum in the Build_Manifest (R28.8).
4. IF a WCET_Record's clock, cache, or build configuration differs from the Build_Manifest, THEN THE Analyzer SHALL reject that WCET_Record.
5. IF the maximum observed value of a measured WCET_Record exceeds the bound of a static WCET_Record for the same item and binary, THEN THE Analyzer SHALL report a WCET inconsistency naming the item and SHALL return FAIL.

### Requirement 34: Measurement-based WCET (Phase P3)

**User Story:** As a timing analyst, I want an automated hardware-in-the-loop measurement pipeline on the cache-less nRF52840, so that I obtain WCET estimates for every Task, Critical_Section, and Kernel operation of the actual binary.

#### Acceptance Criteria

1. THE WCET_Harness SHALL measure execution times on the Target with the DWT cycle counter, with the instruction cache disabled and the clock, MPU, and floating-point configuration identical to the Flight_Build (ASM-04, ASM-05, ASM-06).
2. THE WCET_Harness SHALL measure each Task entry, each Critical_Section site, each Endpoint operation, each release condition (R5.5), and each Kernel_Timing_Parameter with at least PAR-02 executions per item.
3. THE WCET_Harness SHALL account for the cost of reading the cycle counter using a calibration measured on the Target in the same build.
4. THE WCET_Harness SHALL report the branch coverage achieved by the measurement inputs for each item and SHALL report a failure for each item whose coverage is below the threshold recorded in the Verification_Plan.
5. THE WCET_Harness SHALL run each measurement under the interference scenarios recorded in the Verification_Plan, including worst-case concurrent EasyDMA traffic (R17.4), floating-point context preservation triggered and not triggered, and preemption at the maximum rate the Task_Model permits.
6. THE WCET_Harness SHALL compute each measured WCET as the maximum observed value multiplied by (1 + m), where m is the margin recorded in the Verification_Plan, rounded up to a whole cycle, and SHALL label the record as measurement-based rather than a guaranteed bound (ASM-11, ORQ-13).
7. THE WCET_Harness SHALL measure the Flight_Build binary, or SHALL quantify the timing effect of every instrumentation difference and add the effect to the affected WCET_Records.
8. THE WCET_Harness SHALL run unattended on the HIL_Rig when invoked by the Build_System and store the raw measurement data as Evidence_Items.
9. THE WCET_Harness SHALL include, for each loop with a Loop_Bound_Annotation, inputs that drive the loop to its annotated bound unless the Verification_Plan records that bound as unreachable with a justification, and SHALL report the maximum iteration count observed for each loop (R35.4).

### Requirement 35: Loop bounds (Phase P2)

**User Story:** As an application developer, I want to annotate loop bounds once in source and have them checked and consumed by the WCET tools, so that no unbounded loop reaches a Task body.

#### Acceptance Criteria

1. THE Profile_Lint SHALL report as an error naming PR-16 each `loop`, `while`, and `for` expression in a Task body or Kernel function that has neither a Loop_Bound_Annotation nor an iteration count derivable from compile-time constants (PR-16, R3.2).
2. THE Kernel_Proofs (for Kernel loops) and THE App_Prover or THE Kani_Harnesses (for Task-body loops) SHALL prove that each loop executes at most the number of iterations stated by its Loop_Bound_Annotation.
3. THE Build_System SHALL export every Loop_Bound_Annotation, with its source location and enclosing function, to a machine-readable file consumed by the Static_WCET_Analyzer and the WCET_Harness (R34.9).
4. WHEN the WCET_Harness measures an instrumented test build, THE WCET_Harness SHALL report as a failure each loop whose observed iteration count exceeds its Loop_Bound_Annotation.
5. THE Profile_Lint SHALL accept as a Loop_Bound_Annotation only an integer constant or an expression over compile-time constants and Generated_Config constants, and SHALL reject any other form.

### Requirement 36: Static WCET (Phase P5)

**User Story:** As a certification engineer, I want static WCET bounds computed from the binary, so that DAL A timing evidence does not rest on measurement alone.

#### Acceptance Criteria

1. THE Static_WCET_Analyzer SHALL compute a WCET bound for each Task entry, Critical_Section site, Endpoint operation, release condition (R5.5), and Kernel operation from the linked binary, or from LLVM IR with a documented mapping to the binary, using the Target's pipeline and memory timing with the instruction cache disabled and the bus-contention assumptions recorded in the Verification_Plan (ASM-07).
2. THE Static_WCET_Analyzer SHALL use the Loop_Bound_Annotations (R35.3) and the call graph and indirect-call targets resolved by the Link_Checker (PR-18).
3. IF the Static_WCET_Analyzer cannot associate a Loop_Bound_Annotation with a loop in the binary, or finds a loop in code reachable from a Task entry or Kernel entry point, including `core` and third-party code, that has neither a matched annotation nor a bound it can derive, THEN THE Static_WCET_Analyzer SHALL report the annotation or loop naming PR-16 and produce no WCET for the enclosing function.
4. IF a static WCET bound is lower than the maximum execution time observed for the same item by the WCET_Harness, THEN THE Static_WCET_Analyzer SHALL mark that bound invalid and report a discrepancy.
5. THE Verification_Plan SHALL record the selected static WCET tool, its hardware-model assumptions, and its DO-330 criteria and TQL (Requirement 51, ORQ-12).

### Requirement 37: Worst-case stack usage (Phase P2)

**User Story:** As a kernel developer, I want worst-case stack usage computed from the binary and checked against the declared stack regions, so that stack overflow is excluded statically and detected at run time only as a backstop.

#### Acceptance Criteria

1. THE Link_Checker SHALL compute the worst-case stack usage of each Task entry and each Kernel entry point, including the Health_Monitor, from the linked binary, including exception frames and the floating-point storage of R11.4 for contexts with active floating-point state.
2. THE Link_Checker SHALL compute the worst-case usage of each stack region by combining the per-Task and per-Kernel-entry-point values according to the preemption relation of the Task_Model (only Jobs of strictly higher Priority preempt, R7.1) and the assignment of Tasks and Kernel entry points to stack regions that the Design_Document defines, and SHALL fail the build if any declared stack region is smaller than its worst-case usage.
3. IF the Link_Checker cannot bound the stack usage of a function, THEN THE Link_Checker SHALL fail the build naming the function and the relevant PR identifier (PR-17, PR-18).
4. THE Link_Checker SHALL export the computed bound of each stack region as an Evidence_Item and in a form that the Config_Checker consumes (R26.2).

### Part F — Endpoints

### Requirement 38: Endpoint and Protocol declaration (Phase P4)

**User Story:** As an application developer, I want to declare typed, bounded channels governed by protocols, so that inter-task communication is checked at compile time.

#### Acceptance Criteria

1. THE Generator SHALL accept Endpoint declarations that name a Protocol, the roles of the Protocol, the Task bound to each role, the payload type of each message, and the buffer capacity of each directed channel (R41.1).
2. THE Protocol_Checker SHALL accept Protocols with two or more roles, sequencing, labelled choice, and recursion.
3. THE Protocol_Checker SHALL compute the projection of each global Protocol onto each role, and THE Generator SHALL emit for each role a typestate API whose permitted operations in each state are exactly the actions of the projection in that state.
4. THE Endpoint_Library SHALL make each operation that the current protocol state of a role does not permit a compile-time type error.
5. IF a Protocol is ill-formed or its projections are incompatible, THEN THE Protocol_Checker SHALL reject the Protocol and report the offending interaction.
6. THE Protocol_Checker SHALL reuse established multiparty session-type theory, such as the k-multiparty compatibility check used by Rumpsteak [RN-22], and THE Design_Document SHALL record each deviation with its rationale (R61).
7. THE Endpoint_Library SHALL let the protocol state of a role persist across the Jobs of its Task in statically declared per-Task state (PR-25), so that a Protocol interaction can span several Releases while every Job still runs to completion (PR-09).

### Requirement 39: Static, heap-free, executor-free implementation (Phase P4)

**User Story:** As a kernel developer, I want Endpoints implemented with static storage and without an async runtime, so that they fit the Profile and can be verified.

#### Acceptance Criteria

1. THE Endpoint_Library SHALL compile as `#![no_std]` without the `alloc` crate and without any async executor or runtime (PR-15, PR-35); existing crates do not meet this as published (Rumpsteak uses `std` and async receive; Ferrite depends on tokio [RN-22, RN-23]).
2. THE Endpoint_Library SHALL keep every message buffer in statically allocated storage whose capacity the Generator sets from the declaration and the bound of R41.1 (PR-14).
3. THE Endpoint_Library SHALL bound the execution time of each send and receive operation by a WCET_Record (Requirement 33) and contain no loop without a verified bound (PR-16).
4. THE Kernel_Proofs SHALL prove that each Endpoint buffer refines a FIFO queue of capacity N under every interleaving of sender and receiver Jobs that the ceiling protocol permits (R40.1): no message is lost, duplicated, or reordered, and the stored count never exceeds N.

### Requirement 40: Ceiling integration and message-driven release (Phase P4)

**User Story:** As a timing analyst, I want Endpoint operations integrated with the ceiling protocol and with Task releases, so that their blocking and interference appear in the response-time analysis.

#### Acceptance Criteria

1. THE Generator SHALL treat each Endpoint buffer as a Kernel-owned Resource, to which PR-30 and R15.3 do not apply, whose accessors are the Tasks bound to its roles, and SHALL compute its Ceiling per R23.1 or, where the Kernel executes Endpoint operations at a Kernel-reserved level, record that level as its Ceiling.
2. THE Analyzer SHALL include the WCETs of Endpoint send and receive operations as Critical_Sections in the blocking terms and in each Task's execution-time bound.
3. WHERE a Sporadic_Task's Release_Source is an Endpoint receive role, THE Kernel SHALL release that Task when a message arrives at that role, subject to MIT enforcement (Requirement 10), and SHALL keep in the buffer each message whose arrival event is discarded (R10.8).
4. THE Endpoint_Library SHALL complete each send and receive without waiting: a receive on an empty buffer returns an emptiness indication, and a send returns without waiting for space.
5. IF a send finds its buffer full, THEN THE Endpoint_Library SHALL return an error to the sender and report a violation of the Protocol buffer bound, attributed to the sending Partition, to the Health_Monitor.
6. IF a Health_Monitor action ends a Job during an Endpoint operation, THEN THE Kernel SHALL complete or undo that operation before releasing the buffer (R8.9), so that R39.4 holds for every connected role.

### Requirement 41: Protocol safety, progress, and linear use (Phase P4)

**User Story:** As a researcher, I want communication to be deadlock-free by construction even though Rust values are affine rather than linear, so that Endpoint-based designs inherit the Profile's deadlock-freedom guarantee.

#### Acceptance Criteria

1. THE Protocol_Checker SHALL compute, for each Protocol, a bound k on the messages in transit on each directed channel, and THE Generator SHALL reject an Endpoint declaration whose buffer capacity is smaller than k.
2. THE Endpoint_Library SHALL detect each session value dropped in a state other than an end or recursion state, including a role state that a Job fails to store for its next Job (R38.7): at compile time where the mechanism selected for ORQ-11 allows, and otherwise at run time by reporting a protocol violation to the Health_Monitor.
3. THE Verification_Plan SHALL contain a proof, naming its method or tool, that every Application satisfying the Profile, whose Protocols pass the Protocol_Checker, has no reachable state in which each Task of some set awaits a message, as its next Release or within its Protocol, that only another Task of the set can send, and in which no Task waits for a Resource; and SHALL list the proof's assumptions.
4. WHERE an App_Declaration declares an end-to-end deadline for a Protocol interaction chain, THE Analyzer SHALL compute an end-to-end latency bound for that chain, including Δ_stamp where the chain starts at an interrupt event (Table 13-1), and SHALL report PASS for the chain only if the bound does not exceed the deadline.

### Requirement 42: Cross-partition messaging (Phase P4)

**User Story:** As a certification engineer, I want Endpoints to be the only channel between Partitions and to carry plain data, so that communication cannot become a path for spatial interference.

#### Acceptance Criteria

1. THE Generator SHALL accept an Endpoint whose roles are bound to Tasks of different Partitions only if every payload type crossing the Partition boundary contains no references, raw pointers, function pointers, or interior mutability, and every bit pattern of the type's size is a valid value of the type, enforced through a trait bound on the payload type.
2. THE Kernel SHALL transfer each cross-partition message by copying the payload so that neither Partition gains write access to memory owned by the other.
3. WHEN a Partition is stopped or restarted, THE Endpoint_Library SHALL deliver a protocol-level peer-state error to the next operation of each connected role in other Partitions.
4. THE Generator SHALL require a declared receiver-side validation function for each Endpoint that carries messages from a lower-criticality Partition to a higher-criticality Partition, THE App_Prover SHALL prove each such function total and panic-free, and THE Endpoint_Library SHALL deliver to the receiver only messages that the function accepts, reporting each rejected message to the Health_Monitor as a fault of the sending Partition.

### Part G — Verification Plan

### Requirement 43: Property allocation (Phase P1)

**User Story:** As a verification engineer, I want a table that assigns every guarantee to the tool that establishes it and the evidence it produces, so that no guarantee is claimed without an owner.

#### Acceptance Criteria

1. THE Verification_Plan SHALL contain a property allocation table covering at least: data-race freedom; mutual exclusion; ceiling raise and restore; the SRP start condition; bounded blocking; deadlock freedom; drift-free periodic release and release jitter; MIT enforcement; Kernel queue and Endpoint buffer correctness; LIFO preemption invariants; floating-point context preservation; absence of runtime errors in the Kernel; absence of undefined behaviour in the Kernel_Unsafe_Module; boot-time configuration validation; configuration consistency between Generated_Config, the Task_Model, and the binary; spatial isolation, including MPU configuration correctness and DMA isolation; temporal isolation, including Budget-monitor correctness and interrupt-source isolation; Health_Monitor responses; schedulability; WCET; stack bounds; Protocol compliance and communication deadlock freedom; application-level absence of runtime errors; and host-side concurrency correctness.
2. THE Verification_Plan SHALL state for each property the acceptance criteria it covers, the tool (type system, Verus, Kani, App_Prover, Analyzer, UPPAAL, Loom, Shuttle, HIL test, or review), whether the evidence is a proof or a test, the Evidence_Item, the Phase in which the evidence is first produced, and the ASM identifiers and Trust_Base_Register entries the evidence depends on.
3. THE Verification_Plan SHALL state the guarantees relied on from Rust's type system (Send/Sync-based data-race freedom in safe code, contingent on sound `unsafe` code and a correct compiler) and the guarantees the type system does not provide (deadlock freedom, bounded blocking, timing, schedulability, panic freedom, stack bounds, and freedom from leaks).
4. WHERE a property is established by testing only, THE Verification_Plan SHALL label the evidence as testing evidence rather than proof and record why no proof tool applies.
5. THE Verification_Plan SHALL list every acceptance criterion of this document that no row of the property allocation table covers, with the reason it needs no verification evidence or the Phase in which evidence will be added.

### Requirement 44: Kernel proofs with Verus (Phase P1)

**User Story:** As a kernel developer, I want full functional proofs of the Kernel's scheduling, locking, and isolation logic, so that the core guarantees hold for all executions rather than for tested ones.

#### Acceptance Criteria

1. THE Kernel_Proofs SHALL prove, over the Hardware_Model, R7.1–R7.5, R8.1–R8.5, and R8.9; that the Kernel removes no Task other than by a Health_Monitor action (PR-02), changes no interrupt priority after Init (PR-23), and makes no Release effective before Init and the checks of Requirement 12 complete (PR-34); and the absence of arithmetic overflow, out-of-bounds access, and panics in Kernel code.
2. THE Kernel_Proofs SHALL prove that every Kernel queue (timer queue, release state, event log, Endpoint buffers) refines a mathematical sequence model with the capacity bound of PR-14.
3. THE Kernel_Proofs SHALL prove that the Kernel reports every Overrun, and every deadline miss (R9.6), within Δ_detect as modeled in cycles, whatever the System_Ceiling (R18.2), and that MIT enforcement makes no two Releases of a Sporadic_Task due less than one MIT apart (R10.1–R10.3).
4. THE Kernel_Proofs SHALL prove that the MPU configuration function grants each Partition no access to addresses owned by other Partitions or by the Kernel (R16.6), and that each stack region is configured so that its overflow is detected before memory outside the region is modified (R16.5).
5. THE Kernel_Proofs SHALL cover all Kernel code except the functions with trusted specifications that the Trust_Base_Register lists; outside those functions, Kernel code SHALL use only Rust features that the pinned Verus version supports (async functions, function pointer types, and hardware intrinsics are not supported [RN-10]), and inline assembly and register access SHALL occur only inside those functions [RN-14].
6. THE Build_System SHALL run the Kernel_Proofs with the Verus_Toolchain and the Z3 version pinned by the selected Verus release [RN-11] and record both versions in each proof Evidence_Item.
7. THE Verification_Plan SHALL state which Verus specification establishes panic freedom of Kernel functions under the abort panic strategy (ORQ-07; [RN-13]).
8. THE Kernel_Proofs SHALL prove that the nominal release instant of Job k of each Periodic_Task equals O + k·T for every k up to the declared maximum continuous operating duration without overflow or wrap-around ambiguity (R9.1, R9.3, R9.5), and that each Task has at most one pending or deferred Release, with the discard semantics of R9.8 and R10.3.

### Requirement 45: Kani harnesses (Phase P1)

**User Story:** As a kernel developer, I want bounded model checking of `unsafe` code, HAL glue, and the Verus specifications, so that low-level errors outside Verus' reach are caught.

#### Acceptance Criteria

1. THE Kani_Harnesses SHALL include at least one harness per justification identifier of an `unsafe` block or `unsafe fn` in the Kernel_Unsafe_Module for which a harness is feasible, checking pointer validity, alignment, bounds, arithmetic overflow, and panics for all inputs within the documented preconditions. Every other justification identifier, including each justification identifier of an `unsafe impl`, SHALL be linked as R6.4 requires: to a Kernel_Proofs proof or lemma or a harness where one is feasible, and otherwise to a recorded review that states why none is feasible.
2. THE Kani_Harnesses SHALL check Kernel and HAL glue functions for arithmetic overflow, out-of-bounds access, division by zero, and panics.
3. THE Kani_Harnesses SHALL include cross-check harnesses that run each Kernel operation's implementation on bounded nondeterministic inputs and assert the postcondition of the operation's Verus specification.
4. THE Kani_Harnesses SHALL replace inline assembly and memory-mapped register accesses with stubs whose behaviour matches the Hardware_Model, because Kani does not support inline assembly [RN-16, RN-17, RN-18], THE Trust_Base_Register SHALL list each stub, and THE Verification_Plan SHALL name the differential test of Requirement 49 that validates each stub against the Target.
5. THE Build_System SHALL treat a Kani run as passing only when every check, including every unwinding assertion, succeeds.
6. THE Verification_Plan SHALL record the Kani limitations relevant to rsk (sequential code only; no detection of aliasing-model violations, mutation of immutable data, or invalid values unless enabled; bounded verification [RN-17]) and assign each to a mitigation or to the Trust_Base_Register.
7. THE Build_System SHALL record the Kani and CBMC versions in each Kani Evidence_Item.
8. THE Kani_Harnesses SHALL state each harness's input and unwinding bounds, and THE Verification_Plan SHALL justify that each bound covers the Generated_Config sizes of the Profile_Conformance_Suite Application or record the residual risk in the Trust_Base_Register.

### Requirement 46: Application-level prover (Phase P1 selection; P2 use)

**User Story:** As a developer of a DAL A or B Partition, I want a SPARK-like prover for Task bodies, so that I can prove absence of runtime errors and functional properties in application code.

#### Acceptance Criteria

1. THE Verification_Plan SHALL record an evaluation of Prusti, Creusot, and Aeneas against at least: maintenance activity and release recency; supported Rust subset (loops, mutable borrows, traits, generics, interior mutability, `unsafe`); `no_std` support; soundness argument and trusted base; compatibility with the Verus_Toolchain and Qualified_Toolchain versions; ability to prove panic and overflow freedom, loop bounds (R35.2), release-condition obligations (R5.5), and validation-function totality (R42.4); and prospects for DO-330 and DO-333 credit (ORQ-10).
2. THE Verification_Plan SHALL designate exactly one App_Prover and record the justification, and THE Build_System SHALL pin the App_Prover and back-end versions and record them in each App_Prover Evidence_Item.
3. THE App_Prover SHALL prove absence of panics, arithmetic overflow, and out-of-bounds access in every Task body, validation function (R42.4), and release condition (R5.5) of every Partition with Criticality_Level A or B.
4. THE App_Prover SHALL prove that each Resource invariant declared in the App_Declaration holds after the Resource's initialization and at the exit of every Critical_Section on that Resource in Partitions with Criticality_Level A or B.
5. WHERE the App_Prover cannot express a Task-body construct, THE Verification_Plan SHALL record the construct, the affected Tasks, and the substitute evidence.

### Requirement 47: Interleaving tests with Loom and Shuttle (Phase P2)

**User Story:** As a verification engineer, I want host-side interleaving exploration of concurrent components, so that concurrency bugs are found early, while the results are treated as testing evidence rather than proof.

#### Acceptance Criteria

1. THE Host_Test_Harness SHALL test every host-side rsk component that uses threads or atomics with Loom, and with Shuttle where the Loom state space exceeds the CI time limit recorded in the Verification_Plan.
2. THE Host_Test_Harness SHALL test a host model of Kernel dispatch and locking that runs the Kernel's portable logic compiled for the host, not a reimplementation, in which interrupt preemption is represented by threads constrained to the Hardware_Model's preemption rules, and SHALL check every explored execution against the postconditions of the Kernel's Verus specifications (R44.1).
3. THE Verification_Plan SHALL label all Host_Test_Harness results as testing evidence and record the documented limitations of both tools (Loom does not explore every C11 behaviour; Shuttle is randomized and not sound [RN-37]).
4. WHERE the partitioned multi-core extension adds cross-core components, THE Host_Test_Harness SHALL test those components with Loom.

### Requirement 48: Trust base register (Phase P1)

**User Story:** As a certification engineer, I want every trusted component and assumption listed with its justification, so that I can see exactly what the proofs and tests do not cover.

#### Acceptance Criteria

1. THE Trust_Base_Register SHALL list at least: the front end and LLVM back end of the Qualified_Toolchain and of the Verus_Toolchain; the linker; the `core` library and the Core_Subset; proc-macros that take part in Flight_Builds (the Generator and the Verus ghost-erasure macro); HAL and PAC crates; the Hardware_Model; interrupt-controller, MPU, FPU, and DWT semantics; applicable silicon errata; Verus with its SMT solver, `vstd` specifications, and every `external_body`, `assume_specification`, and `assume` item; Kani with CBMC, its SAT solver, and every stub; the App_Prover and its back ends; UPPAAL; the Analyzer; the WCET_Harness; the Static_WCET_Analyzer; the Config_Checker; the Link_Checker, including the static checks used as partitioning evidence (R16.10); the Profile_Lint; the Protocol_Checker; the HIL_Rig, its debug probe, and the flashing tool; and the Build_System.
2. THE Trust_Base_Register SHALL state for each entry what is trusted, why, the evidence that reduces the trust, the residual risk, and the requirements and properties that depend on the entry.
3. THE Trust_Base_Register SHALL link each entry to its related ASM and ORQ identifiers.
4. THE HIL_Rig SHALL read back each binary loaded on the Target before a test or measurement and compare its hash with the binary hash in the Build_Manifest, so that the trust placed in the flashing tool rests on a check (R48.2).

### Requirement 49: Hardware model (Phase P1)

**User Story:** As a verification engineer, I want a formal, validated model of the Cortex-M4F behaviours the proofs depend on, so that the proofs connect to the real chip and the model's validity can be argued under DO-333.

#### Acceptance Criteria

1. THE Hardware_Model SHALL formalize, for the Target: exception entry and return, including tail-chaining and late arrival; NVIC pending, enable, active, and priority arbitration, including priority grouping and the order among pending exceptions of equal priority (ORQ-17); BASEPRI, BASEPRI_MAX, and PRIMASK semantics; privileged and unprivileged execution, including Handler-mode privilege; MPU region matching, overlap precedence, background region, and fault generation; precise and imprecise BusFault generation; floating-point context preservation for the selected policy, including the default floating-point context settings applied at exception entry; and the timer and cycle-counter behaviour used by the Kernel, including compare events set close to the current counter value.
2. THE Hardware_Model SHALL cite, for each formalized rule, the defining section of the Armv7-M Architecture Reference Manual, the Cortex-M4 Technical Reference Manual, or the nRF52840 Product Specification.
3. THE HIL_Rig SHALL execute, for each formalized rule, a differential test that compares the Target's observed behaviour with the behaviour the Hardware_Model predicts, and THE Verification_Plan SHALL record each mismatch and its disposition (ORQ-09).
4. THE Hardware_Model SHALL record the core revision it models and each erratum it includes or excludes (ASM-01).
5. THE Hardware_Model SHALL be written in Verus specification code, so that the Kernel_Proofs use the model directly.
6. THE Hardware_Model SHALL list each Target behaviour that it does not formalize and state why no Kernel_Proofs obligation depends on that behaviour.

### Requirement 50: Proof hygiene and evidence management (Phase P1)

**User Story:** As a certification engineer, I want proofs and tests run automatically, assumptions gated, and every result archived with tool versions, so that the evidence is reproducible and trustworthy.

#### Acceptance Criteria

1. THE Build_System SHALL run, as one verification job on every change to the main branch, the Kernel_Proofs, Kani_Harnesses, App_Prover proofs, Analyzer, Config_Checker, Link_Checker, and Profile_Conformance_Suite, SHALL run the HIL_Rig suites at the frequency that the Verification_Plan records, and SHALL store each result as an Evidence_Item.
2. IF a proof artifact contains an assumption construct (Verus `assume`, `admit`, `external_body`, or `assume_specification`; a Kani stub; or the App_Prover's equivalent) that the Trust_Base_Register does not list, THEN THE Build_System SHALL fail the verification job and name the construct.
3. THE Build_System SHALL record in each Evidence_Item the source revision, the Profile version, tool and solver versions, command lines, the verdict, and, where the item concerns a binary, the Build_Manifest identity.
4. WHEN a proof Evidence_Item is re-run from its archived inputs and tool versions, THE Build_System SHALL produce the same verdict.
5. THE Verification_Plan SHALL maintain a traceability matrix linking each acceptance criterion of this document to design elements, implementation units, and Evidence_Items.
6. THE Build_System SHALL report each acceptance criterion of this document that lacks a linked Evidence_Item.

### Certification (cross-cutting)

### Requirement 51: DO-330 tool qualification assessment (Phase P1)

**User Story:** As a certification engineer, I want each tool's qualification need determined per DO-178C Table 12-1, so that I know which tools need qualification and at which TQL for DAL A and DAL B.

#### Acceptance Criteria

1. THE Verification_Plan SHALL classify each tool that produces or verifies Flight_Build artifacts (Qualified_Toolchain, Generator, Verus ghost-erasure macro, Config_Checker, Link_Checker, Profile_Lint, Protocol_Checker, Analyzer, UPPAAL, WCET_Harness, Static_WCET_Analyzer, Verus, Kani, App_Prover, coverage tool, HIL_Rig test automation, flashing tool, Build_System) under DO-178C tool-qualification criterion 1, 2, or 3, or as needing no qualification because its output is verified, with a justification for each classification.
2. THE Verification_Plan SHALL state each tool's TQL for DAL A and for DAL B from DO-178C Table 12-1: criterion 1 gives TQL-1 at DAL A and TQL-2 at DAL B; criterion 2 gives TQL-4 at both; criterion 3 gives TQL-5 at both [RN-38].
3. THE Verification_Plan SHALL argue that the Config_Checker (Requirement 26) and review verify the Generator's outputs, so that the Generator needs no TQL-1 or TQL-2 qualification, citing the mutation cases of R26.7 as evidence of the Config_Checker's detection ability, and SHALL list every Generator output that this verification does not cover.
4. THE Verification_Plan SHALL state, for Verus and for Kani, the certification credit claimed, whether that credit places the tool under criterion 2 (TQL-4) or criterion 3 (TQL-5), and the qualification gap, since no DO-330 qualification of either tool is known (ORQ-06).
5. THE Verification_Plan SHALL classify the Analyzer under criterion 3 when its result serves only schedulability verification, or under criterion 2 when its result eliminates other verification such as timing tests, and state the resulting TQL.
6. THE Verification_Plan SHALL state that qualification applies per project and that rsk tools can at most be qualifiable [RN-38].
7. THE Verification_Plan SHALL state, for each tool that verifies the output of another tool or of a development activity (for example the Config_Checker for the Generator), the independence of development and review between them that the DO-178C objectives with independence at DAL A and DAL B require.

### Requirement 52: DO-333 formal-methods credit (Phase P1 plan)

**User Story:** As a certification engineer, I want each formal analysis to meet DO-333's expectations on soundness, model validation, and property preservation, so that the proofs can replace some testing.

#### Acceptance Criteria

1. THE Verification_Plan SHALL provide a soundness justification for each formal method used for credit (Verus, Kani, App_Prover, Protocol_Checker, the proven Analyzer core, and UPPAAL and the Static_WCET_Analyzer if credit is claimed), covering the tool's semantics of Rust or of the binary, its trusted base, and its known soundness issues, such as Kani's vtable-restriction limitation and Verus' ghost-erasure feature flags [RN-12, RN-17].
2. THE Verification_Plan SHALL describe how each formal model and each formalized requirement is validated, including the Hardware_Model validation of R49.3 and a review of formal specifications against the natural-language requirements.
3. THE Verification_Plan SHALL provide a property-preservation argument from the source analysed by Verus, Kani, and the App_Prover to the Flight_Build object code produced by the Qualified_Toolchain, addressing differing rustc versions, ghost erasure, and compiler optimization (ORQ-08).
4. THE Verification_Plan SHALL list, per DO-178C verification objective, the objectives that formal analysis is claimed to satisfy and the testing that remains.
5. THE Verification_Plan SHALL state, for each formal analysis used for credit, which acceptance criteria its proven properties cover completely, which they cover only in part, and the evidence that covers the remainder.

### Requirement 53: Structural coverage and source-to-object traceability (Phase P3)

**User Story:** As a certification engineer, I want a defined way to obtain MC/DC (DAL A) and decision coverage (DAL B) for Rust code, and source-to-object traceability for DAL A, so that the structural coverage objectives can be met.

#### Acceptance Criteria

1. THE Verification_Plan SHALL select a structural coverage method for Rust code that provides MC/DC for DAL A code, decision coverage for DAL B code, and statement coverage for code of Partitions with Criticality_Level C, recording the tool and whether coverage is measured at source or object level, on host or on the Target (ORQ-05; upstream rustc MC/DC instrumentation was removed in late 2025 and restoring it is a 2026 project goal, while GNATcoverage documents Rust MC/DC [RN-40]).
2. WHERE source-level structural coverage is used for DAL A code, THE Verification_Plan SHALL provide a source-to-object-code traceability analysis for the Kernel and DAL A Task code that identifies object code not directly traceable to source, including compiler-generated checks, inlined `core` code, and Generator output.
3. WHEN the coverage tool is selected, THE Build_System SHALL produce structural coverage reports for the Kernel and for the code of Partitions with Criticality_Level A, B, or C from requirements-based tests as Evidence_Items.
4. THE Verification_Plan SHALL state which structural coverage objectives, if any, formal-analysis credit is claimed to replace under DO-333, with justification (Requirement 52).

### Requirement 54: Certification gap register (Phase P1)

**User Story:** As the project lead, I want a living register of certification gaps, so that the research prototype preserves a credible DAL A/B path without overstating readiness.

#### Acceptance Criteria

1. THE Gap_Register SHALL list each DO-178C Annex A objective applicable at DAL A and DAL B, and at each lower Criticality_Level that a Partition of the Profile_Conformance_Suite Application uses, with its rsk status (addressed, partially addressed, not addressed, or not applicable) and references to Evidence_Items or ORQ identifiers.
2. THE Gap_Register SHALL record as open gaps at least: compiler assurance beyond Ferrocene's DO-178C DAL C support (ORQ-04); Rust MC/DC tooling (ORQ-05); DO-330 qualification of Verus and Kani (ORQ-06); certification of `core` outside DO-178C (Requirement 57); measurement-based WCET (ORQ-13); robust partitioning evidence under the selected privilege model, including any reliance on static checks (ORQ-01, R16.10); and DMA isolation without an IOMMU (ORQ-03).
3. WHEN an acceptance criterion, ASM, or ORQ of this document changes status, THE Gap_Register SHALL be updated in the same change.
4. THE Gap_Register SHALL distinguish research-prototype deliverables from activities that only a certification applicant can perform, such as planning documents, verification independence, and certification liaison.

### Toolchain (cross-cutting)

### Requirement 55: Language subset and dual-toolchain compatibility (Phase P1; dual build from P5)

**User Story:** As a kernel developer, I want the code to build with both Verus' pinned compiler and Ferrocene, so that the verified source is the source that ships.

#### Acceptance Criteria

1. THE Build_System SHALL compile every crate that a Flight_Build compiles, and the Profile_Conformance_Suite, without `#![feature]` attributes and without setting `RUSTC_BOOTSTRAP` (PR-36).
2. THE Build_System SHALL compile the Kernel with the Verus_Toolchain (verification build) and with the Qualified_Toolchain (Flight_Build) on every change from Phase P5, and with the upstream stable rustc version on which the Qualified_Toolchain is based before Phase P5, failing if either build fails.
3. THE Build_System SHALL archive the macro-expanded, ghost-erased Kernel source of each Flight_Build, and THE Verification_Plan SHALL describe how that source is shown to equal the executable code that Verus verified (ORQ-08; [RN-12]).
4. THE Link_Checker SHALL report a violation if the Kernel Flight_Build contains any executable symbol that originates from the `vstd` crate.
5. THE Build_System SHALL use one Rust edition, supported by both the Verus_Toolchain and the Qualified_Toolchain, for all rsk crates.

### Requirement 56: Ferrocene flight build (Phase P5)

**User Story:** As a certification engineer, I want flight binaries built by a pinned Ferrocene release for a qualified target and within its documented constraints, so that the compiler argument starts from a qualified tool.

#### Acceptance Criteria

1. THE Build_System SHALL produce Flight_Builds for the Target with a pinned Ferrocene release for the `thumbv7em-none-eabihf` target, which became a Ferrocene qualified target in release 25.05 [RN-39].
2. THE Build_System SHALL apply the constraints of the pinned release's Safety Manual and User Manual for that target, and THE Verification_Plan SHALL list each constraint with its compliance evidence [RN-41].
3. THE Verification_Plan SHALL record the qualification scope of the pinned release (ISO 26262 ASIL D, IEC 61508 SIL 3, IEC 62304 Class C, with support for customer efforts toward IEC 61508 SIL 4 and DO-178C DAL C [RN-39]) and record the gap to DAL A/B as ORQ-04.
4. THE Verification_Plan SHALL record whether proc-macro expansion (Generator, Verus ghost-erasure macro), build scripts, and Cargo fall within the pinned release's qualification scope (ORQ-20).
5. THE Build_System SHALL record the Ferrocene release identifier in each Flight_Build Evidence_Item, and THE Verification_Plan SHALL assess each Known Problem published for that release against rsk code [RN-41].

### Requirement 57: Core_Subset (Phase P1)

**User Story:** As a certification engineer, I want the `core` APIs used by the Kernel and Applications restricted to a documented subset, so that the trusted library surface is minimal and maps to available certification evidence.

#### Acceptance Criteria

1. THE Profile SHALL define the Core_Subset as an allow-list of `core` items that starts from the subset Ferrocene certifies (IEC 61508 SIL 2, and ISO 26262 ASIL B since release 26.02 [RN-39]) where that subset's contents are published, and adds only items justified in the Verification_Plan (ORQ-24).
2. THE Profile_Lint SHALL report as an error naming PR-21 each use of a `core` item outside the Core_Subset in the Kernel, the Endpoint_Library, code emitted by the Generator, and Application crates (PR-21).
3. THE Link_Checker SHALL report each `core` symbol in a Flight_Build that maps to no Core_Subset item, including symbols introduced by macro expansion or compiler lowering, such as memory intrinsics and panic and formatting machinery.
4. THE Gap_Register SHALL record that Ferrocene's `core` certification is not a DO-178C certification and list the evidence needed for DAL A/B use of the Core_Subset.
5. THE Profile SHALL record, for each Core_Subset item, the certification evidence that covers it or the rsk verification evidence that substitutes for that certification evidence.

### Requirement 58: Dependency control and third-party unsafe code (Phase P1)

**User Story:** As a certification engineer, I want every dependency pinned, vendored, and assessed for `unsafe` code, so that third-party code cannot silently enlarge the trust base.

#### Acceptance Criteria

1. THE Build_System SHALL build only from a committed lock file with exact versions and vendored sources, and SHALL fail a build that resolves a dependency absent from the lock file.
2. THE Build_System SHALL report each crate in the Flight_Build dependency graph that contains `unsafe` code, assembly, a build script, or a proc-macro, and THE Trust_Base_Register SHALL contain an entry for each such crate other than the Kernel crate.
3. THE Verification_Plan SHALL state the HAL and PAC strategy that keeps Partition code free of `unsafe` (PR-20) while providing the peripheral access declared in the App_Declaration (ORQ-21).
4. THE Build_System SHALL execute build scripts and proc-macros without network access.
5. WHEN a change adds a crate to the Flight_Build dependency graph or changes the version of a crate in it, THE Build_System SHALL fail the build until the Trust_Base_Register entry that R58.2 requires exists for that crate version.

### Requirement 59: Reproducible builds (Phase P2)

**User Story:** As a certification engineer, I want bit-identical rebuilds, so that evidence produced for one build applies to the deployed binary.

#### Acceptance Criteria

1. WHEN a Flight_Build is rebuilt on a clean checkout from the same source revision, lock file, and toolchain, on the same or another build host, THE Build_System SHALL produce loadable sections with an identical binary hash, and Generated_Config and Task_Model files identical to the original (R24.4).
2. THE Build_System SHALL record the binary hash of each Flight_Build in its Build_Manifest and in every Evidence_Item derived from that build.
3. THE Build_System SHALL verify R59.1 for each Flight_Build that is released or used to produce Evidence_Items by rebuilding it on a clean checkout and comparing the hashes, and SHALL fail the build on any mismatch.

### Portability and Reuse

### Requirement 60: Architecture abstraction and later targets (Phase P5)

**User Story:** As a kernel developer, I want architecture-specific code isolated, so that Cortex-M33 and RISC-V ports reuse the portable Kernel logic and its proofs.

#### Acceptance Criteria

1. THE Kernel SHALL confine architecture-specific code to the Kernel_Arch, behind an interface whose operations have Verus specifications.
2. THE Kernel_Proofs SHALL verify the portable Kernel logic against the Kernel_Arch interface specifications, so that a port needs new proofs only for its Kernel_Arch implementation and its Hardware_Model.
3. WHERE a Cortex-M33 (Armv8-M Mainline) target is added, THE Kernel SHALL support that target's MPU (including its region rules and the stack-limit registers) and interrupt-priority model, and THE Hardware_Model SHALL be extended to that target (ORQ-19).
4. WHERE a RISC-V target is added, THE Verification_Plan SHALL record the selected part, its interrupt controller (CLIC or PLIC), the mapping of SRP Ceilings onto that controller's priority and threshold mechanism, its memory-protection mechanism, and the Ferrocene qualification status of its target (ORQ-19).
5. WHEN a target is added, THE Build_System SHALL run the Kernel_Proofs, the Kani_Harnesses, the differential tests of R49.3, and the Profile_Conformance_Suite for that target, and SHALL report the target as supported only when all of them pass.

### Requirement 61: Reuse and deviation record (Phase P1)

**User Story:** As the project lead, I want each reused project's contribution and each deviation recorded with its rationale, so that rsk builds on existing work instead of reinventing it.

#### Acceptance Criteria

1. THE Design_Document SHALL record, for RTIC, Hubris, the Rust type system, Verus, Kani, Prusti, Creusot, Aeneas, Loom, Shuttle, Rumpsteak, Ferrite, UPPAAL, Ferrocene, Clippy and custom lints, and Embassy, what rsk reuses (code, design, or theory), the integration point, and the version reviewed.
2. THE Design_Document SHALL record each deviation from a reused project's design together with the rsk requirement that forces the deviation, and SHALL refer to Table 4-1 for deviations from Ada RM D.13 (Requirement 4).
3. THE Design_Document SHALL analyse RTIC's macro design, ceiling computation, and BASEPRI lock implementation [RN-06, RN-07] and record which parts rsk reuses unchanged.
4. THE Design_Document SHALL record how Hubris' build-time manifest and start-up table validation [RN-09] are adopted, and how Hubris' synchronous IPC and supervisor model differ from rsk's run-to-completion Tasks and Health_Monitor.
5. THE Design_Document SHALL map each soundness fix in RTIC's change log [RN-05] to the rsk proof, harness, or check that would detect the corresponding defect class.
6. THE Design_Document SHALL state which Embassy HAL and driver patterns rsk adopts and why Embassy's async executor is not the scheduling model (NG-07), with the rsk requirements that the executor would violate.

## Assumptions

Each assumption is flagged for confirmation in design or on hardware. "Affects" lists the dependent requirements.

| ID | Assumption | Affects |
|---|---|---|
| ASM-01 | The nRF52840 core is Cortex-M4 revision r0p1, as debugger logs report [RN-33]; erratum applicability is assessed for that revision and confirmed by the boot check. | R11.6, R12.1, R49.4 |
| ASM-02 | The nRF52840 implements the optional Cortex-M4 MPU with 8 regions (PAR-07; the Cortex-M4 MPU has 8 regions when present [RN-29]); confirmed by the boot check. | R12.1, R16, R20.4, R24.5 |
| ASM-03 | The NVIC implements 3 priority bits (PAR-06 [RN-25]) and priority grouping uses all implemented bits as preemption bits. | PR-33, R7, R22.2 |
| ASM-04 | The NVMC instruction cache is disabled at reset (ICACHECNF resets to 0 [RN-26]) and no software enables it, so flash access timing is fixed for a given clock configuration. | NG-03, NG-14, R12.1, R34.1, R36.1 |
| ASM-05 | The CPU runs at PAR-08 from the high-frequency crystal from the end of Init until reset, with no clock changes. | NG-15, R12.1, R29.8, R34.1 |
| ASM-06 | The DWT cycle counter is implemented, counts core cycles, is 32 bits wide (wrapping after 2^32 cycles, about 67 s at 64 MHz), and is accessible only to privileged code because it lies in the Private Peripheral Bus. Not confirmed from a Nordic source; to be confirmed against the Armv7-M ARM and on hardware. | R12.5, R18.1, R34 |
| ASM-07 | Interference of EasyDMA and other AHB bus masters with CPU memory accesses is bounded and is exercised by the measurement scenarios; no formal bound is available [RN-35]. | R17.4, R34.5, R36.1 |
| ASM-08 | The Target behaves as specified by the Armv7-M ARM, the Cortex-M4 TRM, and the nRF52840 Product Specification, apart from errata listed in the Hardware_Model. | R44, R49 |
| ASM-09 | For the Rust subset rsk uses, the semantics assumed by the Verus_Toolchain, Kani, and the App_Prover agree with the semantics implemented by the Qualified_Toolchain. | PR-19, R44, R45, R46, R52.3, R55 |
| ASM-10 | External event sources normally respect their declared MITs; violations are detected and contained by the Kernel. | PR-12, R10, R19, R29 |
| ASM-11 | Measurement-based WCET with a margin is adequate for the research prototype but is not a guaranteed upper bound. | R29, R34.6, R54.2 |
| ASM-12 | In flight configuration no bus master other than the CPU and EasyDMA peripherals writes RAM, and the debug port does not halt or modify the Target. | R16, R17, R34 |
| ASM-13 | UPPAAL is used under its free non-commercial academic license [RN-24]; commercial or certification use needs a separate license. | R31, R32.6 |
| ASM-14 | Rust's safe-code guarantees (Send/Sync data-race freedom, borrow checking) hold given sound `unsafe` code and a correct compiler. | PR-05, PR-22, PR-29, R8.6, R43.3 |
| ASM-15 | Each formal tool is sound within its documented limitations for the features rsk uses (Verus with its pinned Z3 [RN-11]; Kani with its pinned CBMC [RN-17]; the App_Prover with its back end). | PR-19, R44, R45, R46, R52.1 |
| ASM-16 | The Kernel time base comes from a clock with a fixed, known ratio to the CPU clock (such as a TIMER peripheral fed from the high-frequency clock); if an RTC on the 32.768 kHz clock is used [RN-08], the ratio tolerance is bounded and applied conservatively. | R9, R12.1, R21.2, R29.8 |
| ASM-17 | The project has access to Ferrocene releases, their Safety Manuals, and their Known Problems lists for Phase P5. | R55.2, R56 |
| ASM-18 | Drift-free release (R9.3) is relative to the Kernel time base, which deviates from physical time by the tolerance of its clock source (ASM-16); the Kernel does not correct this deviation, and an Application that needs physical-time accuracy allows for it. | R9.1, R9.3, R29.8 |
| ASM-19 | The timer hardware of the Kernel time base may not generate a compare event for a compare value set too close to the current counter value (Nordic documents such a limit for its RTC); the Kernel never loses a timed event because of it. To be confirmed against the nRF52840 Product Specification and formalized in the Hardware_Model. | R9.2, R9.4, R49.1 |

## Open Research Questions

Each question must be resolved, or explicitly deferred with a recorded rationale, in the Design_Document or the Verification_Plan.

| ID | Question | Affects |
|---|---|---|
| ORQ-01 | **Privilege model versus hardware SRP (central tension).** RTIC runs Tasks as interrupt handlers and locks by writing BASEPRI [RN-03, RN-07]. On Armv7-M, Handler mode is always privileged [RN-30], BASEPRI writes need privilege, and the MPU offers 8 regions with power-of-two size and alignment [RN-29]. Candidate approaches, none chosen here: (a) privileged Handler-mode Tasks with per-Partition MPU restrictions plus language-level guarantees (open sub-question: whether the MPU can restrict privileged accesses to the Private Peripheral Bus, where the NVIC, SCB, and MPU registers live); (b) unprivileged Thread-mode Tasks with Kernel-mediated dispatch and SVC-based locks in the style of Hubris [RN-09], at the cost of software SRP and larger Kernel_Timing_Parameters; (c) a hybrid with hardware-dispatched privileged Tasks for the highest Criticality_Level and unprivileged lower-criticality Partitions; (d) the Armv8-M security extension on Cortex-M33. Constraints that the refined requirements fix for every approach: no Critical_Section may mask Overrun detection (R18.2), so the Kernel-reserved levels sit above every Ceiling and RTIC's interrupt-free critical section at the maximum priority [RN-07] is unavailable to Application Resources; and each access class that the approach does not enforce in hardware rests on the Link_Checker's static checks (R16.10, ORQ-28). | R7.4, R7.7, R8, R13.6, R16.9, R16.10, R18, R18.2, R20, R20.5, R29.3, R61, PR-31 |
| ORQ-02 | MPU layout: whether each Partition's code, data, stack, Endpoint storage, and peripherals fit PAR-07 regions under power-of-two rules and subregions; the layout algorithm; memory waste; reconfiguration cost per dispatch. | R16.6, R20.4, R24.5 |
| ORQ-03 | DMA isolation without an IOMMU: Kernel-mediated DMA configuration, forbidding DMA in some Partitions, or monitoring (the nRF52840 has a Memory Watch Unit [RN-27]; whether it observes EasyDMA accesses is unknown). | R17 |
| ORQ-04 | Compiler assurance for DAL A/B beyond Ferrocene's DO-178C DAL C support [RN-39]: object-code verification, applicant-led qualification, translation validation, or source-to-object analysis. | R53.2, R54.2, R56.3 |
| ORQ-05 | MC/DC and decision coverage for Rust: GNATcoverage's Rust support or restored upstream instrumentation [RN-40]; on Target or on host; object-code coverage as an alternative. | R53 |
| ORQ-06 | DO-330 qualification of Verus and Kani (none known): tool operational requirements and qualification test suites; independent proof checking, for example the experimental Lean 4 back end for Verus in which the Lean kernel is the only checker [RN-15]; dual-solver cross-checks. | R44, R45, R51.4 |
| ORQ-07 | DO-333 soundness details: Verus' trusted base (Z3, `vstd` specifications, `external_body` and `assume_specification`, and ghost-erasure cfg flags that the Verus developers note can introduce unsoundness [RN-12]); Verus' panic semantics under the abort strategy (a general abort specification was still a proposal in 2026 [RN-13]); Kani's sequential-only, bounded checking and its known vtable-restriction issue [RN-17]; the App_Prover's back end. | R44.7, R45.6, R52.1 |
| ORQ-08 | Property preservation from verified source to Flight_Build object code: different rustc versions in the Verus_Toolchain and the Qualified_Toolchain, ghost erasure by the `verus!` macro inside the flight build path, and LLVM optimization; and whether the verification build needs `#![feature]` attributes or `RUSTC_BOOTSTRAP` (for `vstd` or the `verus!` macro), which R55.1 and PR-36 forbid only for crates that a Flight_Build compiles. | R52.3, R55.1, R55.3 |
| ORQ-09 | Hardware_Model scope and validation: which behaviours to model (tail-chaining, late arrival, lazy-stacking corner cases), differential-test coverage, and use of machine-readable architecture specifications if available. | R49 |
| ORQ-10 | App_Prover selection. Status found: Prusti's latest GitHub release dates from February 2024 and pins nightly-2023-09-15 [RN-19]; Creusot is active (v0.13.0 on 30 July 2026), uses Why3 with SMT solvers, and offers ghost ownership for interior mutability and raw pointers [RN-20]; Aeneas handles a safe-Rust subset (no `unsafe` or concurrency yet, some loop forms unsupported), its Lean and HOL4 back ends are the most mature, and a 2026 report verifies 16.7 KLOC of Rust with it [RN-21]. | R46 |
| ORQ-11 | Session types without heap, async, or linear types: Rumpsteak's typestate needs no heap, but the crate uses `std` and async receive [RN-22]; Ferrite depends on tokio [RN-23]; how to make dropping an unfinished session detectable in affine Rust; how to compute buffer bounds k (for example by k-multiparty compatibility). | R38, R38.7, R39, R41 |
| ORQ-12 | Static WCET tool for Rust-compiled Cortex-M4 binaries; transfer of loop bounds from source to optimized binary; tool qualification. | PR-16, R35.3, R36 |
| ORQ-13 | Adequacy of measurement-based WCET: margin selection, path coverage, and combination with static analysis. | R34, R54.2 |
| ORQ-14 | Temporal partitioning model: priority bands with Budget enforcement or time windows (ARINC 653-like); RTA for budget-enforced Tasks; the closed set of Overrun_Responses. | R13.6, R14.5, R18, R29.4 |
| ORQ-15 | Mixed-criticality scheduling (Vestal model, AMC mode changes) as a post-v1 extension. | NG-09, R18 |
| ORQ-16 | FPU: lazy or eager stacking with MPU-protected stacks (lazy stacking is on by default [RN-31]); applicability to r0p1 of Cortex-M4F core errata listed in vendor errata sheets, including the VDIV/VSQRT short-ISR erratum and the store-immediate-overlapping-exception-return erratum [RN-32]; FP costs in RTA; whether LLVM emits floating-point loads and stores for Kernel memory copies on `thumbv7em-none-eabihf`, which PR-31 and R6.6 would then have to allow or prevent. | R6.6, R11, PR-31 |
| ORQ-17 | Equal-priority dispatch order: NVIC arbitration among equal-priority pending interrupts is expected to follow exception number rather than arrival order (to be confirmed in the Hardware_Model), unlike FIFO_Within_Priorities; accept a static order or emulate FIFO. | R4.3, R4.6, R7.3, R31.2, R49.1 |
| ORQ-18 | Multiple suspension points per Job (async, self-suspension; Jorvik-style extension) and their schedulability analysis. | PR-09, R5 |
| ORQ-19 | Later targets: the Cortex-M33 board and phase are not fixed; the RISC-V part is not chosen (CLIC or PLIC; RTIC's RISC-V backends are marked unstable [RN-04, RN-05]; Ferrocene qualification of candidate RISC-V targets is unknown). | R60 |
| ORQ-20 | Ferrocene qualification scope for proc-macro expansion (Generator, `verus!` erasure), build scripts, and Cargo. | R51.1, R56.4 |
| ORQ-21 | HAL and PAC strategy under `#![forbid(unsafe_code)]` outside the Kernel_Unsafe_Module, given that PAC and HAL crates use `unsafe` internally; and whether the `unsafe_code` lint reports `global_asm!` and `#[no_mangle]`, or a Build_System check must enforce R6.3 for them. | PR-20, R2.2, R6.3, R17.5, R44.5, R58.3 |
| ORQ-22 | UPPAAL fidelity and scalability: over-approximation for stopwatch models [RN-24], state-space growth, validation of the mapping from Task_Model semantics to automata, licensing. | R31, R31.9, R32.6 |
| ORQ-23 | Interrupt MIT enforcement: masking a source during an MIT window may lose or coalesce peripheral events; per-source semantics. | R10, R19.4 |
| ORQ-24 | Core_Subset contents: which `core` items Ferrocene's certified subset covers, and whether compiler-lowered calls (memory intrinsics, panic and formatting machinery) fall inside it. | R57 |
| ORQ-25 | Idle activity content: Kernel-only, or Application idle code at the lowest level under the Profile; if Application idle code may lock Resources, its Critical_Sections enter the blocking term of R29.2. | R7.6, R29.2 |
| ORQ-26 | MIT_Violation threshold counting (R10.4): cumulative since instant 0 or the last restart, as written, or over a sliding window; a cumulative count eventually trips on rare benign violations during long operation. | R10.4, R14.5, R15.1 |
| ORQ-27 | Attribution of imprecise BusFaults (R14.3): attributing them to the Kernel lets a lower-criticality Partition force the system safe state, while making BusFaults precise (for example by disabling write buffering) costs performance and must be modelled. | R14.3, R14.4, R16.4, R49.1 |
| ORQ-28 | Whether static Link_Checker checks (R16.10) are acceptable robust-partitioning evidence under DO-178C §2.4 for DAL A/B Partitions, or whether those Partitions need hardware-enforced isolation for every access class. | R16.9, R16.10, R20.2, R54.2 |
| ORQ-29 | Jorvik Release_Signals: which bound Task one raise releases (Priority, FIFO, or declaration order) and the effect on latency and starvation; when and at what priority release conditions are evaluated (for example at Critical_Section exit under the Ceiling) and the effect on L_kernel. | R5.2, R5.5, R10.5, R13.1 |
| ORQ-30 | Whether Flight_Builds check at boot that halting debug is disabled, backing ASM-12, given that HIL tests also run Flight_Builds. | R12.1, ASM-12 |
| ORQ-31 | A hardware watchdog as a backstop for Kernel hangs: whether to require one, its interaction with the system safe state, and its formalization in the Hardware_Model. | R14.4, R49.1 |

## Research Notes and Sources

Facts below were checked by web research in October 2026 (RN-39 to RN-41 were verified by the user). Content was rephrased for compliance with licensing restrictions.

- **RN-01** Ada RM D.13 (Ada 2022 edition, as published on ada-lang.io) defines Ravenscar as FIFO_Within_Priorities dispatching, Ceiling_Locking, Detect_Blocking, and the restriction list mapped in Table 4-1. Jorvik removes six of those restrictions and replaces Simple_Barriers with Pure_Barriers. For multiprocessors the RM advises a fully partitioned approach with disjoint per-processor ready queues. [Ada RM D.13 (ada-lang.io)](https://ada-lang.io/docs/arm/AA-D/AA-D.13)
- **RN-02** SPARK supports tasking under Ravenscar or the more permissive Jorvik profile. [SPARK User's Guide, concurrency](https://docs.adacore.com/spark2014-docs/html/ug/en/source/concurrency.html)
- **RN-03** RTIC v2 builds on SRP: static priorities, single core, run-to-completion, LIFO locking, ceilings computed at compile time, and the system ceiling mapped to BASEPRI or interrupt-source masking. Async software tasks run on generated per-priority executors, and the compiler rejects an await while a resource is held. [RTIC book, preface](https://rtic.rs/2/book/en/)
- **RN-04** RTIC uses BASEPRI ceilings on Armv7-M and Armv8-M Mainline (including Cortex-M33) and source masking on Armv6-M and Armv8-M Baseline. RISC-V backends include ESP32-C3, a machine-mode ecall backend, and a CLINT backend; some emulate the interrupt controller in software. [RTIC target architectures](https://rtic.rs/2/book/en/internals/targets.html)
- **RN-05** RTIC change log: v2 works on stable since 2.1.0; RISC-V and ESP32 support is marked unstable; soundness fixes include a thumbv7 priority-inversion fix (2.1.0), a thumbv6 source-mask race (2.1.1), a Send assertion for init-provided local resources (2.3.0), and an unreleased spawn-race fix. Latest release seen: 2.3.1 (2026-08-20). [RTIC CHANGELOG](https://github.com/rtic-rs/rtic/blob/master/rtic/CHANGELOG.md)
- **RN-06** RTIC classifies each shared resource as owned, co-owned (same priority), or contended, with the ceiling equal to the maximum accessor priority, and derives Send and Sync requirements on resource types. [rtic-macros analyze.rs](https://github.com/rtic-rs/rtic/blob/master/rtic-macros/src/syntax/analyze.rs)
- **RN-07** RTIC's Cortex-M lock raises BASEPRI with a BASEPRI_MAX write and restores the previous value; when the ceiling equals the highest priority it falls back to a global critical section. [RTIC cortex_basepri.rs](https://github.com/rtic-rs/rtic/blob/master/rtic/src/export/cortex_basepri.rs)
- **RN-08** rtic-monotonics supports nRF52840 RTC monotonics at 32,768 ticks per second and 32-bit TIMER monotonics (TIMER0 to TIMER4 on nRF52840); its examples use a relative `delay`. [nRF RTC monotonic](https://docs.rs/rtic-monotonics/latest/rtic_monotonics/nrf/rtc/index.html), [nRF TIMER monotonic](https://docs.rs/rtic-monotonics/latest/rtic_monotonics/nrf/timer/index.html)
- **RN-09** Hubris declares all tasks in `app.toml`, supports no task creation or destruction at run time, runs tasks unprivileged and memory-isolated, validates generated task and region tables at start-up, and delegates fault recovery to a supervisor task. [Hubris reference](https://hubris.oxide.computer/reference/)
- **RN-10** Verus (feature table updated 2026-05-13): async functions, async blocks, and await are unsupported; raw pointers are partially supported; function pointer types, hardware intrinsics, Pin, and user-defined Drop are unsupported; floating point is partial. [Verus supported features](https://verus-lang.github.io/verus/guide/features.html)
- **RN-11** Verus requires a specific rustc toolchain installed through rustup and pins Z3 4.16.0 in its build instructions. [Verus BUILD.md](https://github.com/verus-lang/verus/blob/main/BUILD.md), [Verus INSTALL.md](https://github.com/verus-lang/verus/blob/main/INSTALL.md)
- **RN-12** `verus!` code can be built with plain `cargo build` after ghost erasure; the cfg flags used to keep ghost items are internal and can introduce unsoundness. [Verus discussion #2101](https://github.com/verus-lang/verus/discussions/2101)
- **RN-13** Verus developers note that panic-freedom claims have caveats and that Verus can specify unwinding but has no general mechanism for aborts; a specification scheme was proposed in 2026. [Verus discussion #2258](https://github.com/verus-lang/verus/discussions/2258)
- **RN-14** Verus supports trusted `external_body` wrappers whose specifications are assumed, not verified. [Verus guide, exec attributes](https://verus-lang.github.io/verus/guide/exec_attr.html)
- **RN-15** Vermilion is an experimental Lean 4 back end for Verus that turns verification conditions into Lean theorems checked by the Lean kernel. [Vermilion](https://github.com/ilyasergey/vermilion)
- **RN-16** Kani does not support assembly; reachable unsupported features become failing checks; concurrency is compiled as sequential code; await is unsupported; stack unwinding is unsupported; data races are not detected. [Kani Rust feature support](https://model-checking.github.io/kani/rust-feature-support.html)
- **RN-17** Kani's soundness page: sequential code only; aliasing-model violations, mutation of immutable data, and inline-assembly misuse are not detected; global assembly is ignored with a warning; results hold only if all unwinding assertions pass; CBMC is pinned and regression-tested; `--restrict-vtable` is a known soundness issue. [Kani soundness](https://model-checking.github.io/kani/soundness.html)
- **RN-18** Kani stubbing is unstable (`-Z stubbing`), applies to functions and methods only, and is recommended for unsupported features such as inline assembly; verified contracts can replace functions via `stub_verified`. [Kani stubbing](https://model-checking.github.io/kani/reference/experimental/stubbing.html)
- **RN-19** Prusti describes itself as a prototype verifier built on Viper; its latest release (v-2024-02-28) uses nightly-2023-09-15, with a nightly pre-release from 2024-03-26. [Prusti releases](https://github.com/prusti/prusti/releases), [Prusti README](https://github.com/viperproject/prusti-dev/)
- **RN-20** Creusot is a deductive verifier that translates Rust to Coma and Why3 and uses SMT solvers; it offers Pearlite contracts, prophecies for mutable borrows, termination checking, and ghost ownership. Releases run from v0.7.0 (November 2025) to v0.13.0 (30 July 2026). [Creusot](https://creusot.rs/), [Creusot releases](https://github.com/creusot-rs/creusot/releases), [crates.io dates](https://crates.io/api/v1/crates/creusot-contracts)
- **RN-21** Aeneas translates a safe-Rust subset (via Charon's LLBC) to F*, Coq, HOL4, and Lean; `unsafe` code and concurrency are not yet supported, and some loop forms are not. A September 2026 report verifies 16.7 KLOC of Rust with 237 KLOC of Lean. [Aeneas README](https://github.com/AeneasVerif/aeneas/blob/main/README.md), [arXiv 2609.15648](https://arxiv.org/html/2609.15648v1)
- **RN-22** Rumpsteak (0.1.0, marked work in progress) depends on `futures` and `thiserror` and uses `std`; receive and branch are async; its session typestate is a phantom type over a role reference with pluggable channels; it uses k-multiparty compatibility to check deadlock freedom. [Rumpsteak](https://github.com/zakcutner/rumpsteak), [Cargo.toml](https://docs.rs/crate/rumpsteak/latest/source/Cargo.toml.orig), [lib.rs](https://docs.rs/crate/rumpsteak/latest/source/src/lib.rs), [arXiv 2112.12693](https://arxiv.org/abs/2112.12693)
- **RN-23** Ferrite (ECOOP 2022) depends on tokio with all features, ipc-channel, and serde. [Ferrite](https://github.com/ferrite-rs/ferrite), [Cargo.toml](https://docs.rs/crate/ferrite-session/latest/source/Cargo.toml.orig)
- **RN-24** UPPAAL's native model format is XML (`.xml`); XTA (`.xta` with `.ugi`) is also supported and TA is legacy read-only; queries live in `.q` files; `verifyta` is the command-line verifier. UPPAAL is free for non-commercial academic use. With stopwatches reachability is undecidable, so UPPAAL over-approximates: schedulable verdicts are safe, while deadline-miss witnesses need confirmation. [UPPAAL file formats](https://docs.uppaal.org/toolsandapi/file-formats/), [UPPAAL downloads and license](https://uppaal.org/downloads/), [Herschel revisited, STTT 2014](https://link.springer.com/article/10.1007/s10009-014-0331-4), [Herschel-Planck framework, ISoLA 2010](https://link.springer.com/chapter/10.1007/978-3-642-16561-0_21)
- **RN-25** nRF52840 PAC: `NVIC_PRIO_BITS = 3`. [nrf52840-pac](https://docs.rs/nrf52840-pac/latest/nrf52840_pac/constant.NVIC_PRIO_BITS.html)
- **RN-26** nRF52840 NVMC `ICACHECNF` resets to 0 (cache disabled). [nrf52840-pac ICACHECNF](https://docs.rs/nrf52840-pac/latest/nrf52840_pac/nvmc/icachecnf/struct.ICACHECNF_SPEC.html)
- **RN-27** nRF52840 peripherals include TIMER0–TIMER4, RTC0–RTC2, WDT, EGU/SWI0–5, PPI, MWU (Memory Watch Unit), and ACL. [nrf52840-pac Peripherals](https://docs.rs/nrf52840-pac/latest/nrf52840_pac/struct.Peripherals.html)
- **RN-28** The nRF52840 uses a 64 MHz Cortex-M4 with FPU. [Nordic nRF52840](https://www.nordicsemi.com/Products/nRF52840)
- **RN-29** The Cortex-M4 MPU, when implemented, supports 8 regions. Power-of-two region size and alignment on Armv7-M is taken from the user's brief and is to be confirmed in the Armv7-M ARM. [Arm DUI0553, MPU region number register](https://developer.arm.com/docs/dui0553/b/cortex-m4-peripherals/optional-memory-protection-unit/mpu-region-number-register)
- **RN-30** Handler mode is always privileged; Thread mode can be privileged or unprivileged. [Arm DDI0337E, privileged and user access](https://developer.arm.com/docs/ddi0337/e/programmers-model/privileged-access-and-user-access)
- **RN-31** On Cortex-M4F the FPCCR enables lazy stacking by default. [Arm AN298, lazy stacking](http://infocenter.arm.com/help/topic/com.arm.doc.dai0298a/BCGHEEFD.html)
- **RN-32** Vendor errata sheets list Cortex-M4 core errata, including VDIV/VSQRT results with very short ISRs and store-immediate overlapping exception return. [NXP K32L3A6 errata](https://nxp.com/docs/en/errata/K32L3A6_3N69S.pdf)
- **RN-33** A debugger log on nRF52840 reports a Cortex-M4 r0p1 core. [Nordic DevZone thread](https://devzone.nordicsemi.com/f/nordic-q-a/49634/jlinkrttclient-not-working-on-ubuntu)
- **RN-34** The Nordic SoftDevice enables the instruction cache by default. [Nordic DevZone thread](https://devzone.nordicsemi.com/f/nordic-q-a/66240/nrf52840-icache-performance-measurements)
- **RN-35** EasyDMA is an AHB bus master with direct access to Data RAM and cannot access flash (nRF52 series documentation; nRF52840 text to be confirmed). [Nordic EasyDMA (nRF52833 PS)](https://infocenter.nordicsemi.com/topic/ps_nrf52833/easydma.html), [nrf-hal issue #37](https://github.com/nrf-rs/nrf52-hal/issues/37)
- **RN-36** Embassy tasks are statically allocated; the executor polls tasks from a run queue and relies on tasks not blocking; multiple executors, including interrupt-driven ones, provide preemption between priority levels. [Embassy book](https://embassy.dev/book/)
- **RN-37** Loom permutes executions under the C11 memory model but does not explore some behaviours, so it is not sound; Shuttle is randomized and explicitly not sound. [Loom](https://github.com/tokio-rs/loom), [Shuttle](https://github.com/awslabs/shuttle)
- **RN-38** DO-178C tool-qualification criteria: 1 (output is part of the airborne software), 2 (automates verification and its output eliminates or reduces other processes), 3 (could fail to detect an error). Table 12-1: criterion 1 gives TQL-1/2/3/4 at DAL A/B/C/D; criterion 2 gives TQL-4/4/5/5; criterion 3 gives TQL-5 at all levels. Qualification is project-specific; vendors offer qualifiable tools. [Rapita, DO-330](https://www.rapitasystems.com/do-330), [AFuzion, DO-330 introduction](https://afuzion.com/do-330-introduction-tool-qualification/), [AdaCore, DO-178C standards suite](https://learn.adacore.com/booklets/adacore-technologies-for-airborne-software/chapters/standards.html)
- **RN-39** (User-verified) Ferrocene 26.05 qualification scope and DAL C support: [Ferrocene 26.05](https://ferrous-systems.com/blog/ferrocene-26-05-0/). `core` subset certified to IEC 61508 SIL 2: [libcore news](https://ferrous-systems.com/blog/ferrocene-libcore-news-release/); ISO 26262 ASIL B since 26.02: [eeNews Europe](https://www.eenewseurope.com/en/ferrocene-26-02-0-automotive-core-certification/). `thumbv7em-none-eabihf` qualified since 25.05: [Ferrocene 25.05](https://ferrous-systems.com/blog/ferrocene-25-05-0/)
- **RN-40** (User-verified) Upstream rustc MC/DC instrumentation was added in 2024 and removed in late 2025, with restoration a 2026 project goal: [Rust project goal](https://rust-lang.github.io/goals/2026/mcdc-coverage-support.html). GNATcoverage documents Rust MC/DC coverage: [GNATcoverage Rust](https://docs.adacore.com/gnatcoverage-docs/html/gnatcov/cov_rust.html)
- **RN-41** Each Ferrocene release has a User Manual whose per-target instructions must be followed to stay within qualification scope; stable releases are supported for two years with tracked Known Problems. [Ferrous training: installing Ferrocene](https://rust-training.ferrous-systems.com/latest/book/ferrocene-installing), [Ferrous training: what Ferrocene is](https://rust-training.ferrous-systems.com/latest/book/ferrocene-what-it-is)
- **RN-42** Under Ceiling_Locking, a call to a protected operation checks that the caller's active priority does not exceed the object's ceiling and raises Program_Error otherwise, so a nested call into an object with a lower ceiling fails. [Ada RM D.3 (ada-lang.io)](https://ada-lang.io/docs/arm/AA-D/AA-D.3)
