# Requirements Document

**HAMR-DAHN: an IronOxide back end for HAMR**

Status: Draft 0.1 (requirements phase, for the user's review; no design.md or tasks.md yet). Spec: `hamr-ironoxide-backend`. Facts checked on 2026-11-09 against IronOxide revision 423a366 and Sireum kekinian revision e459583; the study behind them is `docs/hamr_study.md`. Mapping choices that design.md must record as design decisions are marked "DD candidate".

## Introduction

HAMR (part of Sireum) generates code from AADL and SysML v2 models for the JVM, Linux, seL4 (CAmkES and Microkit) and ROS 2. IronOxide ("rsk") is a Ravenscar-style concurrency profile for Rust with a Verus-verified SRP kernel on the nRF52840, a generator for its `rsk::app!` declarations, a source lint, a binary link checker, a configuration checker, and a response-time analyzer. This project builds the bridge between them:

- **A. Inputs and the property set**: which models the back end takes, the `IronOxide` property set, and diagnostics.
- **B. Mapping and generation**: AADL threads, processes, shared data, ports, devices and flows mapped to IronOxide Tasks, Partitions, Resources, Release_Signals, Endpoints, peripherals and chains; refusal of everything outside the profile; the generated Application.
- **C. Contracts**: GUMBO component contracts translated into Creusot proof obligations on the generated task bodies, with a stated correspondence between the two meanings.
- **D. Timing**: AADL timing properties and end-to-end flow latencies carried into IronOxide's Analyzer, and its verdicts reported against the model.
- **E. Translation validation**: a checker whose core is written in Verus that compares the generated Task_Model with a semantic model of the AADL input, instead of verifying HAMR's Scala code.
- **F. Testing**: property-based tests over generated models, differential tests against HAMR's JVM simulation, and proof mutations.
- **G. Case study**: a ground robot whose Raspberry Pi 5 side uses an existing HAMR back end and whose nRF52840 side runs on IronOxide, generated from one model, with the MCU side run on IronOxide's HIL_Rig.
- **H. Verification plan, registers and build**: the evidence for every claim, in the style of IronOxide's own Verification_Plan.

The result must pass IronOxide's own tools unchanged. IronOxide is an external dependency: where this project would need an IronOxide change, the change is written as a proposal in `docs/ironoxide_proposals.md` (IOP-nn) and the requirements below work without it.

### Fixed inputs (decided by the user; not open in this spec)

- Target runtime: IronOxide at the revision of PAR-01, used read-only from the sibling directory `../IronOxide`. Its Profile (version 0.6, Profile_Variant Ravenscar), App_Declaration grammar, Task_Model schema (`rsk-task-model/1`), tools and limits are as `docs/hamr_study.md` sections 11 and 12 record.
- Code generator: HAMR in Sireum kekinian at the revision of PAR-02, cloned into `external/` and built in the workspace.
- Modeling languages: AADL (instantiated by OSATE or FMIDE into HAMR's AIR) and SysML v2 through HAMR's own front end.
- Verification approach: translation validation of the generated configuration, with the checker's core in Verus, in preference to verifying or testing the generator.
- Case study hardware: Raspberry Pi 5 (Linux) and nRF52840-DK, connected by a serial link; IronOxide's HIL_Rig of three nRF52840-DK boards and an 8-channel logic analyzer.
- Phases P1 to P5 as in the table below, plus a later Phase P6 for the physical robot; each Phase ends in a checkpoint that runs every test and proof and asks the user questions.

### Phases

| Phase | Scope |
|---|---|
| P1 | Study (done: `docs/hamr_study.md`); minimal back end: one periodic thread per process, no ports; the property set; diagnostics; the Semantic_Model export; generated Applications that pass IronOxide's tools and run under QEMU |
| P2 | Ports, shared data, Release_Signals, Endpoints, data-port sampling, validation functions, devices and interrupts; GUMBO contracts to Creusot; property-based tests over generated models |
| P3 | Timing: the WCET flow, flows to chains, the Verdict_Report; differential tests against HAMR's JVM simulation on QEMU |
| P4 | Translation validation (TV_Checker in Verus), the mechanized contract correspondence, the mutation suites; differential tests on the HIL_Rig |
| P5 | Robot case study: the two-processor model, the Deployment_Splitter, the Serial_Bridge, both sides generated, the MCU side on the HIL_Rig |
| P6 (later) | Robot integration on the physical robot (specified here, not run in P5) |

Each requirement heading names the earliest Phase in which it must hold.

### Conventions

- `Rn.m` is acceptance criterion m of Requirement n of this document. `IronOxide Rn.m`, `IronOxide DD-nn`, `IronOxide ASM-nn` point into IronOxide's `.kiro/specs/rsk-ravenscar-profile/` documents; `PR-nn` is a restriction of IronOxide's Profile (`IronOxide/docs/profile.md`).
- `MAP-nn` is a row of Table M-1 (mappings and refusals), `DEV-nn` a row of Table M-2 (deviations from HAMR's AADL semantics), `NG-nn` a non-goal, `ASM-nn` an assumption, `ORQ-nn` an open research question, `PAR-nn` a parameter, `IOP-nn` an IronOxide proposal, `F-n` a finding of `docs/hamr_study.md`, and `[RN-nn]` a research note with its source.
- Evidence_Item names follow IronOxide's style: `test-…` (example tests), `pbt-…` (property-based tests), `proof-…` (Verus or Creusot), `check-…` (static checks and IronOxide tool runs), `mut-…` (mutation runs), `diff-…` (differential tests), `hil-…` (HIL_Rig runs), `analysis-…` (Analyzer runs), `review-…` (recorded reviews), `doc-…` (documents). `<model>` stands for each model of the corpus concerned. Each acceptance criterion ends with the Evidence_Items that will show it.
- Every diagnostic of the Backend names the AADL element (instance path and source position), the IronOxide rule (a PR-nn, an IronOxide Rn.m or DD-nn, or a named IronOxide limit with its source file), and the MAP row.
- Priority: a larger value is more urgent, as in IronOxide.
- Time values in AADL are read as integers in picoseconds (HAMR's internal unit) and written to IronOxide with explicit units; IronOxide's own conversion to ticks and cycles then applies (IronOxide R21.2).
- Content from external sources was rephrased for compliance with licensing restrictions.

### Parameters

Values marked "proposal" are initial values for review; changing one requires the procedure of Requirement 41, not edits to the criteria that reference it.

| ID | Parameter | Value | Basis |
|---|---|---|---|
| PAR-01 | IronOxide revision | `423a366881d744034282bd99cb2d6d057e405f17` | user, 2026-11-09 |
| PAR-02 | HAMR revision (Sireum kekinian; `hamr/codegen` submodule) | `e459583680c261d772f6ae87987073b9fd0c7c0b`; `954bf4c0d50838badf4dda16f94bf60785aa492b` | `docs/hamr_study.md` section 1 |
| PAR-03 | Maximum distinct Task Priorities | 7 | IronOxide `crates/rsk-gen/src/validate.rs` `MAX_PRIORITY`; PR-33 |
| PAR-04 | Maximum Tasks per Application | 7 | IronOxide `crates/rsk-gen/src/target.rs` `dispatch_irqs` (7 lines on `nrf52840` and on `qemu_mps2_an386`); IronOxide DD-01 |
| PAR-05 | Endpoints per Application; roles per Endpoint | at most 8; 2 to 4, exactly 2 for a plain Endpoint | `validate.rs` `MAX_ENDPOINTS`, `MAX_ROLES` |
| PAR-06 | Endpoint capacity; payload; total Endpoint storage | 1 to 255 messages per directed channel; 4 to 65,532 bytes, a multiple of 4; at most 8,192 bytes as Σ roles² × capacity × payload | `validate.rs` (`MAX_ENDPOINT_STORAGE`, PR-14) |
| PAR-07 | Peripheral windows per Partition | at most 5 (8 MPU regions: shared code, Partition code, Partition RAM, 5 windows) | `crates/rsk-gen/src/layout.rs` `MAX_WINDOWS`; IronOxide DD-07 |
| PAR-08 | Generated models per property-based test run | 1,000 | proposal |
| PAR-09 | Differential corpus; length of each run | 30 models; at least 100 hyperperiods or 10 s of model time, whichever is longer | proposal |
| PAR-10 | Duration of each HIL_Rig run of the case study | the longer of 1 hour and 1,000 hyperperiods | IronOxide PAR-05 |
| PAR-11 | Margin applied to measured WCETs | 25 % | IronOxide Verification_Plan 1.3 (the margin of IronOxide R34.6) |
| PAR-12 | Serial link line settings | 1,000,000 baud, 8 data bits, no parity, 1 stop bit | proposal |
| PAR-13 | Maximum frame payload on the serial link | 64 bytes | proposal |
| PAR-14 | MCU command timeout | 100 ms | proposal |
| PAR-15 | Stop reaction bound (from detection to zero PWM duty on every motor output) | 20 ms | proposal |
| PAR-16 | Motor control period | 10 ms | proposal |
| PAR-17 | Planted defects per proof artifact | at least 1 per conjunct of each top-level postcondition or theorem | proposal |
| PAR-18 | Random evaluations per GUMBO clause in the contract differential test | 10,000 | proposal |
| PAR-19 | Maximum commanded wheel speed accepted by the MCU | 1.0 m/s | proposal |

### Component traceability

| Component | Requirements |
|---|---|
| A. Inputs, property set, diagnostics | 1–4 |
| B. Mapping and generation | 5–20 |
| C. Contracts | 21–24 |
| D. Timing | 25–27 |
| E. Translation validation | 28–30 |
| F. Testing | 31–33 |
| G. Case study | 34–38 |
| H. Verification plan, registers, build | 39–42 |

## Glossary

- **AADL**: The SAE Architecture Analysis and Design Language, as HAMR supports it [RN-01].
- **AIR**: HAMR's intermediate representation of an instantiated AADL or SysML v2 system (`hamr/air`), serialized as JSON; includes GUMBO annexes as `GclSubclause` values.
- **AIR_Model**: One AIR instance model of a root system.
- **Front_End**: The HAMR component that produces the AIR_Model: OSATE or FMIDE for AADL text, or HAMR's SysML v2 front end (`sireum hamr sysml codegen`).
- **HAMR**: The HAMR code generator of Sireum at PAR-02, with its existing back ends (JVM, Linux, seL4, Microkit, ROS 2).
- **Backend**: The HAMR IronOxide back end that this project builds: it reads an AIR_Model and emits an IronOxide Application, or refuses the model with diagnostics.
- **IronOxide**: The rsk runtime and tool set at PAR-01: Profile, Kernel, Generator, Profile_Lint, Link_Checker, Config_Checker, Analyzer, Protocol_Checker, WCET_Harness, HIL_Rig.
- **IronOxide_Generator**: IronOxide's Generator (`rsk-macros`, `rsk-gen`), which consumes an App_Declaration and emits Generated_Config, the Task_Model and the linker script.
- **Profile_Lint**, **Link_Checker**, **Config_Checker**, **Analyzer**, **Protocol_Checker**, **WCET_Harness**, **Kernel**, **HIL_Rig**: The IronOxide components of those names, as IronOxide's glossary defines them.
- **App_Prover**: Creusot with Why3 and its solvers at IronOxide's pinned versions, the application prover IronOxide designates (IronOxide DD-13).
- **Application**, **App_Declaration**, **Partition**, **Task**, **Job**, **Release_Point**, **Periodic_Task**, **Sporadic_Task**, **Release_Source**, **Release_Signal**, **Resource**, **Critical_Section**, **Endpoint**, **Protocol**, **Chain**, **Budget**, **MIT**, **MIT_Violation**, **Criticality_Level**, **Health_Monitor**, **Task_Model**, **WCET_Record**, **WCRT**, **Kernel_Timing_Parameters**, **Flight_Build**, **Build_Manifest**: As IronOxide's glossary defines them (IronOxide calls a chain "chain" in the App_Declaration). A plain Endpoint is one declared with a payload type and no Protocol.
- **IronOxide_Target**: An IronOxide Target name: `nrf52840` or `qemu_mps2_an386`.
- **IronOxide_Processor**: The one processor of a model whose `IronOxide::Target` property names an IronOxide_Target.
- **Peer_Processor**: A processor of the case-study model other than the IronOxide_Processor, here the Raspberry Pi 5.
- **IronOxide_Property_Set**: The property set `IronOxide` of Table 2-1, as an AADL property set file and as a SysML v2 library package.
- **Dispatch trigger**: The in event or event data port whose arrival dispatches a Sporadic thread, given by `Thread_Properties::Dispatch_Trigger` or, where that property is absent, by the thread's in event and event data ports.
- **Upward connection**: A connection whose destination Partition has a higher Criticality_Level than its source Partition (A is the highest).
- **Port_Cell**: A Resource that the Backend generates to carry one same-Partition port connection: a latest-value cell, an event counter, or a bounded FIFO.
- **Port_API**: The generated interface through which a thread's developer code reads frozen inputs and writes outputs: AADL's Get Value and Put Value run-time services [RN-01].
- **Port_Glue**: The generated code of each Task's Job that performs Receive Input before and Send Output after the developer's compute code, and the generated validation functions.
- **Wire_Type**: The generated `Plain` Rust type (IronOxide R42.1) that carries an AADL data type across Partitions or over the serial link.
- **Validation_Function**: A receiver-side function that IronOxide requires for every role receiving from a lower Criticality_Level (IronOxide R42.4).
- **GUMBO**: HAMR's contract language for AADL components (integration, initialize, compute, state, functions, invariants, handlers, cases) [RN-02].
- **GUMBOX**: HAMR's executable translation of GUMBO clauses (Slang on the JVM, Rust in the Microkit back end), used by tests and monitors.
- **Contract_Translator**: The Backend component that turns GUMBO clauses into Creusot contracts, Validation_Functions, executable predicates and Connection_Obligations.
- **Connection_Obligation**: A proof obligation that the integration guarantee clauses of a connection's source port imply the integration assume clauses of its destination port.
- **Semantic_Model**: A JSON document (`hamr-rsk-semantic-model/1`) that records the parts of the AIR_Model relevant to the mapping, written by the Semantic_Model_Extractor.
- **Semantic_Model_Extractor**: The tool that writes the Semantic_Model from the AIR_Model, implemented independently of the Backend's mapping code.
- **Mapping_Relation**: The relation between a Semantic_Model and a Task_Model that Table M-1 and IronOxide's conversion rules define, stated as Verus specification functions.
- **TV_Checker**: The translation-validation checker that accepts a Semantic_Model and Task_Model pair only if the Mapping_Relation holds; its core is written and proven in Verus.
- **Verdict_Reporter**: The Backend component that maps the Analyzer's results back to model elements; its output is the **Verdict_Report**.
- **Model_Generator**: The property-based test generator of SysML v2 and AIR models.
- **Trace_Body**: A generated deterministic thread body, in Slang for HAMR's JVM platform and in Rust for IronOxide, used by differential tests.
- **Trace_Comparator**: The tool that compares port-level traces of HAMR's JVM simulation with those of the IronOxide Application.
- **Deployment_Splitter**: The tool that splits a two-processor model into one sub-model per processor and inserts the Serial_Bridge.
- **Serial_Bridge**: The generated components on both processors that carry cross-processor connections over the serial link.
- **Frame_Codec**: The generated printer and parser of Serial_Bridge frames.
- **MCU_Side**, **Pi_Side**: The generated software of the nRF52840 and of the Raspberry Pi 5 in the case study.
- **Pi_Stand_In**: The host program or board that plays the Pi_Side's part on the HIL_Rig.
- **Robot_Model**: The case-study model of Requirement 36.
- **Robot_Integration_Plan**: The Phase P6 document of Requirement 38.
- **Build_System**: This workspace's scripted build and verification pipeline (`cargo xtask`), which produces Evidence_Items.
- **Evidence_Item**: A versioned artifact (proof log, test report, analysis report, measurement data, review record) traced to acceptance criteria.
- **Verification_Plan**: This project's verification plan (`docs/verification_plan.md`).
- **Gap_Register**, **Trust_Base_Register**, **Proof_Assumption_Register**, **Proof_Mutation_Register**, **Translation_Mutation_Register**, **Traceability_Matrix**: This project's registers in `docs/`, in the formats of IronOxide's files of the same role.

## Non-Goals

- **NG-01** Modifying IronOxide. Needed changes are proposals (IOP-nn) in `docs/ironoxide_proposals.md`.
- **NG-02** Verifying or testing HAMR's own Scala and Slang code as the main evidence; translation validation of each output is used instead (Requirement 29).
- **NG-03** AADL features outside the profile: modes, dynamic reconfiguration, subprogram calls across components, shared data across processes, and the others that Table M-1 refuses.
- **NG-04** Real-time guarantees for the Pi_Side. Linux timing is not analyzed; the MCU_Side treats the Pi_Side as an untrusted peer that may be late, silent or wrong.
- **NG-05** More than one IronOxide_Processor per model, and multi-core targets (IronOxide NG-01).
- **NG-06** Certification approval or applicant plans. Like IronOxide, the project keeps a DO-178C path visible in its Gap_Register without claiming readiness.
- **NG-07** Changes to the GUMBO language or to HAMR's Front_End; the project adds a property set and back-end code only.
- **NG-08** New HAMR back ends for platforms other than IronOxide.
- **NG-09** HAMR's runtime monitors, attestation and system-testing variants for the IronOxide side; IronOxide's Health_Monitor and HIL evidence play that role.

## Assumptions

Each assumption is flagged for confirmation in design or on hardware. "Affects" lists the dependent requirements.

| ID | Assumption | Affects |
|---|---|---|
| ASM-01 | The Front_End (OSATE or FMIDE for AADL, HAMR's SysML v2 parser for SysML) instantiates the model faithfully into AIR, including positions, properties and GUMBO annexes. Trusted, not checked. | R1, R28, R40.2 |
| ASM-02 | AADL `Thread_Properties::Priority` values in the models this project accepts use "larger is more urgent", as IronOxide does (ORQ-04). | R7 |
| ASM-03 | The AADL defaults used in Table M-1 hold: Deadline inherits Period; Queue_Size is 1; Dequeue_Protocol is OneItem; Overflow_Handling_Protocol is DropOldest; connection Timing is Sampled; the Period of a Sporadic thread is its minimum separation of dispatches. Taken from HAMR's SysML AADL library comments where present and otherwise from the AADL standard, which the project does not hold. | R6, R10, R11 |
| ASM-04 | HAMR's informal semantics [RN-01] is the reference semantics of the Semantic_Model: inputs frozen at dispatch, outputs released at completion, entry points Initialize and Compute only, Periodic and Sporadic dispatch only. | R19, R23, R28, R29, R32 |
| ASM-05 | IronOxide's own evidence at PAR-01 holds as its Verification_Plan states it (Kernel proofs, Config_Checker, Analyzer proofs, HIL results), including its stated trust base and residuals. This project relies on it and does not re-derive it. | R18, R25, R27, R29.7, R37, R39.5 |
| ASM-06 | The serial link corrupts, loses or reorders bytes only in ways that the frame CRC and sequence numbers detect, apart from the CRC's residual undetected-error probability, which the Verification_Plan states. | R35 |
| ASM-07 | The nRF52840-DK's on-board J-Link virtual COM port, or a board-to-board UART wired on the HIL_Rig's spare pins, carries the Pi_Stand_In traffic at PAR-12. To be confirmed on the rig. | R37 |
| ASM-08 | HAMR's JVM simulation executes a model's dispatches in a deterministic order for a given schedule; observed with ART's static scheduler (`art.scheduling.static`) in the isolette run (`docs/hamr_study.md` row 7). | R32 |
| ASM-09 | The Slang semantics of GUMBO expressions over bounded integer types treats an out-of-range result as an error rather than wrapping (ORQ-05; finding F-1 shows HAMR's Verus and GUMBOX translations differ from each other). | R22.3, R23.3 |
| ASM-10 | Each robot actuator can be stopped by driving its PWM output to zero duty, with the motor driver's own enable and brake behaviour documented in the Robot_Integration_Plan. | R36, R38 |

## Requirements

### Part A — Inputs, Property Set, and Diagnostics

### Requirement 1: Model inputs and the IronOxide processor (Phase P1)

**User Story:** As a system modeler, I want the Backend to take the same AADL or SysML v2 model that HAMR's other back ends take, so that one model drives every platform.

#### Acceptance Criteria

1. THE Backend SHALL accept as input an AIR_Model in JSON produced by the Front_End at the HAMR revision of PAR-02, from AADL text through OSATE or FMIDE or from SysML v2 text through HAMR's SysML v2 front end. *Evidence: `test-input-aadl-air`, `test-input-sysml-air`.*
2. THE Backend SHALL generate an Application only for the processes bound to the IronOxide_Processor through `Deployment_Properties::Actual_Processor_Binding`, and only from the elements those processes contain or connect to. *Evidence: `test-processor-selection`, `proof-tv-core`.*
3. IF a model has no processor whose `IronOxide::Target` names an IronOxide_Target, or has more than one, THEN THE Backend SHALL refuse the model with a diagnostic naming each such processor and MAP-27. *Evidence: `test-refusal-diagnostics`.*
4. IF a thread bound to the IronOxide_Processor is connected to a component bound to another processor and the connection has not been replaced by the Deployment_Splitter (Requirement 34), THEN THE Backend SHALL refuse the model, naming the connection and MAP-27. *Evidence: `test-refusal-diagnostics`.*
5. THE Backend SHALL record in every generated artefact and report the SHA-256 digest of the AIR_Model file, the Front_End that produced it, and the revisions of PAR-01 and PAR-02. *Evidence: `test-output-provenance`.*

### Requirement 2: The IronOxide property set (Phase P1)

**User Story:** As a system modeler, I want one documented property set for the values IronOxide needs and AADL does not define, so that criticality levels, fault responses and the other values come from the model rather than from hand edits of generated code.

#### Acceptance Criteria

1. THE Backend SHALL provide the IronOxide_Property_Set with exactly the properties of Table 2-1, as an AADL property set file and as a SysML v2 library package, each property with the type, the component categories it applies to, the default or the marker "required", and the IronOxide rule that Table 2-1 states. *Evidence: `doc-property-set`, `test-property-set-parses` (OSATE-free SysML path, and the AADL file through FMIDE where installed).*
2. THE Backend SHALL take each Partition's code, RAM and stack sizes from `Memory_Properties::Code_Size`, `Memory_Properties::Data_Size` and `Memory_Properties::Stack_Size` of its process. *Evidence: `test-map-07`.*
3. IF a property that Table 2-1 marks required, or a size of criterion 2, is absent, THEN THE Backend SHALL refuse the model naming the element, the property and the IronOxide rule, and SHALL apply no default. *Evidence: `test-refusal-diagnostics`, `mut-refusals`.*
4. IF a property of Table 2-1 has a value outside the type, range or set that Table 2-1 states, THEN THE Backend SHALL refuse the model naming the element, the property and the IronOxide rule. *Evidence: `test-refusal-diagnostics`.*
5. WHERE Table 2-1 gives a default, THE Backend SHALL apply that default only, and SHALL list every applied default with its element in the generation report. *Evidence: `test-defaults-reported`.*
6. WHEN the same model is written in AADL and in SysML v2 with the same property values, THE Backend SHALL produce generated files that are identical apart from source positions and the provenance of R1.5. *Evidence: `test-property-set-equivalence` over every model of the corpus that has both forms.*

**Table 2-1 — The IronOxide_Property_Set**

| Property | Type | Applies to | Default | Feeds |
|---|---|---|---|---|
| `Target` | enumeration (`nrf52840`, `qemu_mps2_an386`) | processor | required on the IronOxide_Processor | header `target` (IronOxide R21.1) |
| `Operating_Duration` | Time | processor | required | header `operating_duration` (IronOxide R9.9, R22.6) |
| `Safe_State` | string, a Rust path into the system's safe-state crate | processor | required | header `safe_state` (IronOxide R14.4, R21.1) |
| `Log_Capacity` | integer 1 to 2^30 − 1 | processor | 32 (IronOxide's default) | header `log_capacity` (IronOxide R14.6) |
| `FP_Flush_To_Zero` | Boolean | processor | false (IronOxide's default) | header `fp_flush_to_zero` (IronOxide R11.8) |
| `Criticality_Level` | enumeration (A, B, C, D, E) | process | required | Partition `level` (IronOxide R15.1) |
| `Fault_Response` | enumeration (END_JOB, RESTART_PARTITION, STOP_PARTITION, SAFE_STATE) | process | required | Partition `fault` (IronOxide R14.5, R15.7) |
| `Overrun_Response` | enumeration (END_JOB, RESTART_PARTITION, STOP_PARTITION, SAFE_STATE, RECORD_AND_CONTINUE) | process | required | Partition `overrun` (IronOxide R18.5, R18.6) |
| `Deadline_Miss_Response` | enumeration (RESTART_PARTITION, STOP_PARTITION, SAFE_STATE, RECORD_ONLY) | process | required | Partition `deadline_miss` (IronOxide R15.1) |
| `MIT_Violation_Threshold` | integer ≥ 0 | process | required | Partition `mit_threshold` (IronOxide R10.4) |
| `Peripherals` | list of strings, IronOxide_Target peripheral names | process | empty list | Partition `peripherals` (PR-29) |
| `Peripheral` | string, an IronOxide_Target peripheral name | device | required for a device connected to the IronOxide_Processor's threads | MAP-20 |
| `Uses_FPU` | Boolean | thread | true where a port, GUMBO state variable or Resource the thread uses has a floating-point type, false otherwise (MAP-28) | Task `fpu` (PR-31) |
| `MIT_Policy` | enumeration (defer, discard) | thread | defer | Task `mit_policy` (IronOxide R10.2) |
| `Validation_Function` | string, a Rust path into the destination Partition's crate | in port | none | role `validate` (IronOxide R42.4; MAP-18) |
| `Serial_Link` | record (baud rate, frame payload bound, device path on the Peer_Processor) | bus | required on a bus that binds a cross-processor connection | MAP-27, Requirement 35 |

### Requirement 3: Diagnostics (Phase P1)

**User Story:** As a system modeler, I want every refusal to point at my model element and at the IronOxide rule it breaks, so that I can fix the model without reading generated code.

#### Acceptance Criteria

1. IF the Backend refuses a model element, THEN THE Backend SHALL report a diagnostic that names the element's instance path and source position (file, line and column, taken from the AIR_Model), the IronOxide rule (a PR-nn, an IronOxide Rn.m, an IronOxide DD-nn, or a named IronOxide limit with the file and constant that define it), and the MAP row of Table M-1. *Evidence: `test-refusal-diagnostics`, `pbt-refusals-named`.*
2. THE Backend SHALL report every refusal it finds in one run, ordered by source position, and SHALL write no generated artefact for a refused model. *Evidence: `test-refusal-diagnostics`.*
3. THE Backend SHALL write its diagnostics as text and as a JSON document that conforms to a published, versioned schema (`hamr-rsk-diagnostics/1`). *Evidence: `test-diagnostics-schema`.*
4. WHEN an IronOxide tool (the IronOxide_Generator, the Profile_Lint, the Link_Checker, the Config_Checker or the Analyzer) reports a diagnostic on Backend output, THE Backend SHALL report the AADL element from which the diagnosed item was generated together with the IronOxide diagnostic. *Evidence: `test-ironoxide-diagnostic-backmap`.*
5. WHEN the Backend reads a property that Table M-1 lists as ignored (MAP-25), THE Backend SHALL report a warning naming the element, the property and MAP-25. *Evidence: `test-map-25`.*

### Requirement 4: Mapping and deviation tables (Phase P1)

**User Story:** As a certification engineer, I want each AADL construct's mapping, refusal and deviation stated once, so that I can trace every generated item and every refusal to a decision.

#### Acceptance Criteria

1. THE Backend SHALL generate every model element that Table M-1 maps as Table M-1 states, and SHALL refuse every construct that Table M-1 refuses with the diagnostic that its row names. *Evidence: `test-map-01` to `test-map-30`, `mut-refusals`, `proof-tv-core`.*
2. IF a model element on the IronOxide_Processor's side of the model is of a kind that Table M-1 neither maps nor lists as ignored, THEN THE Backend SHALL refuse it naming MAP-24. *Evidence: `pbt-refusals-named`.*
3. WHEN a generated Application contains an element that a deviation of Table M-2 affects, THE Backend SHALL list the element and the DEV identifier in the generation report. *Evidence: `test-deviations-reported`.*
4. WHEN Table M-1 or Table M-2 changes, THE Verification_Plan SHALL record the change, its rationale and the affected requirements in its change log, and THE Build_System SHALL fail until the Mapping_Relation of the TV_Checker cites the same table version. *Evidence: `check-mapping-version`.*

**Table M-1 — Mappings and refusals.** "IronOxide mapping" uses the App_Declaration grammar of `docs/hamr_study.md` section 12. "Refused" lists what the row refuses; each refusal's diagnostic names the rule in the "Rule" column.

| Row | AADL construct | IronOxide mapping | Refused | Rule | Why |
|---|---|---|---|---|---|
| MAP-01 | Thread, `Dispatch_Protocol => Periodic` | Periodic_Task: `period` from Period, `offset` from Dispatch_Offset (0 when absent) | no Period | PR-27 | IronOxide's absolute periodic Releases (PR-10) |
| MAP-02 | Thread, `Dispatch_Protocol => Sporadic` | Sporadic_Task: `mit` = Period (AADL's minimum separation), Release_Source from the single Dispatch trigger (MAP-11, MAP-13, MAP-15, MAP-20) | no Period; zero or several Dispatch triggers; Dispatch_Offset | PR-27, PR-11, IronOxide R22.4 | one Release_Source per Task |
| MAP-03 | Thread, `Dispatch_Protocol` Aperiodic, Timed, Hybrid, Background, or absent | none | always | PR-27 (Aperiodic, Background, absent: no period or MIT); PR-11 (Timed: a timeout is an alternative release; Hybrid: two Release_Sources) | RTA needs a period or MIT, and IronOxide has no timeouts |
| MAP-04 | `Deadline`, `Period` values | `deadline` (default: the Period, ASM-03), written in the largest unit that represents the value exactly | Deadline > Period; a value that is not a whole number of ns | IronOxide R22.1, R21.2 | constrained deadlines; no rounding before IronOxide's own |
| MAP-05 | `Compute_Execution_Time` (range) | `budget` = the range's upper bound, covering the whole Job including Port_Glue; measured WCETs checked against it (Requirement 25) | absent (no 50 ms default as in HAMR's Microkit back end); upper bound > Deadline | PR-27, IronOxide R22.1, R28.4 | the Budget is enforced at run time (IronOxide R18) |
| MAP-06 | `Thread_Properties::Priority` | explicit on every thread: rank-preserving map onto 1..k; absent on every thread: deadline-monotonic ranks; ties keep equal Priority, started in instance-path order | present on some threads only; more than PAR-03 distinct values | PR-03, PR-33 | fixed priorities, 7 Task levels (IronOxide DD-02) |
| MAP-07 | Process bound to the IronOxide_Processor | one Partition, one Partition crate (IronOxide DD-11), sizes from Memory_Properties; thread groups flattened into it | no thread; missing sizes or IronOxide properties; two processes with the same sanitized name | IronOxide R15.7, R21.3, PR-28 | the Partition is IronOxide's isolation unit |
| MAP-08 | Virtual processor on the IronOxide_Processor | none | always | PR-28 | IronOxide has one isolation level; HAMR uses virtual processors for Linux guests |
| MAP-09 | `IronOxide::Criticality_Level`, responses, threshold | Partition `level`, `fault`, `overrun`, `deadline_miss`, `mit_threshold` | values outside IronOxide's closed sets; RECORD_AND_CONTINUE below the highest level | IronOxide R14.5, R15.7, R18.5 | temporal isolation by criticality |
| MAP-10 | Data subcomponent of a process, accessed by threads of that process through data-access connections | Resource of that Partition; accessors = those threads; Ceiling derived by IronOxide; read-only access as a shared view | access from another process; `Concurrency_Control_Protocol` other than Priority_Ceiling; no accessor | PR-30, IronOxide Requirement 8, IronOxide R22.5 | SRP locking; Resources stay inside one Partition |
| MAP-11 | Event port connection inside one Partition to a Sporadic thread's Dispatch trigger | Release_Signal of that Partition bound to the destination Task, raised by the source's Port_Glue at completion | a source whose period or MIT is shorter than the destination's MIT | IronOxide R10.2, R10.3 | every raise inside the MIT is an MIT_Violation counted toward the Partition's threshold |
| MAP-12 | Event port connection inside one Partition to a Periodic thread | Port_Cell holding an event count, read and cleared at Job start | — | — | no Release; the count is AADL's event-port value |
| MAP-13 | Event data port connection inside one Partition | Port_Cell holding a bounded FIFO of capacity Queue_Size with the port's overflow policy, plus MAP-11's Release_Signal when the port is a Dispatch trigger | MAP-11's refusal for triggers | IronOxide R10.2 | Endpoints are kept for cross-Partition traffic (PAR-05) |
| MAP-14 | Data port connection inside one Partition | Port_Cell holding the latest value and a freshness flag | — | — | exact latest-value semantics under the ceiling protocol |
| MAP-15 | Event or event data port connection across Partitions | one plain Endpoint per connection; capacity = Queue_Size; receiver `ops` from Dequeue_Protocol; event connections carry a 4-byte token; a Dispatch trigger makes the destination released by its role (`endpoint = NAME`) | fan-in; more than PAR-05 Endpoints; capacity or storage beyond PAR-06 | PR-12, IronOxide R38.1, PR-14 | Endpoints are the only cross-Partition channel (PR-30, PR-32) |
| MAP-16 | Data port connection across Partitions | plain Endpoint of capacity C_sample, receiver `ops` = C_sample, the receiver's glue drains the channel at Job start and keeps the newest value in per-Task state (Requirement 12); IOP-01 would replace this | C_sample > 255; storage beyond PAR-06 | PR-14 | latest-value semantics without a Kernel change |
| MAP-17 | `Overflow_Handling_Protocol` across Partitions | DropNewest and Error: IronOxide's full-channel outcome (send returns `Full`, event recorded against the sender, IronOxide R40.5); DropOldest (the default): accepted where C_sample's bound excludes overflow | DropOldest where overflow is not excluded | IronOxide R40.4, R40.5 | IronOxide cannot drop the receiver's oldest message from the sender's side |
| MAP-18 | Upward connection | the receiving role's Validation_Function: Wire_Type decoding plus the destination port's GUMBO integration assume clauses, or `IronOxide::Validation_Function`; reverse channels of plain Endpoints get a reject-all function where IronOxide requires one | an upward connection with neither source of a predicate | IronOxide R42.4 | data must not flow up in criticality unchecked |
| MAP-19 | Port data types | application type plus Wire_Type (`Plain`, IronOxide R42.1) with encoder and decoder | unbounded integers without `HAMR::Default_Bit_Width`; unbounded arrays, sequences or strings; a Wire_Type over 65,532 bytes | IronOxide R42.1, PR-14 | bounded storage; every bit pattern valid across Partitions |
| MAP-20 | Device with `IronOxide::Peripheral`, connected to threads of one process | the peripheral owned by that Partition; a device out event port connected to a Sporadic thread's Dispatch trigger makes it interrupt-released (`binds`), `mit_policy = defer` | device data or event data ports; device Dispatch_Protocol; connections to more than one process; unknown, Kernel-owned or doubly owned peripherals; EasyDMA without a DMA description; `discard` on an interrupt | IronOxide R21.5, R21.6, PR-28, PR-29, IronOxide DD-06, DD-08 | peripheral ownership and interrupt MIT enforcement |
| MAP-21 | End-to-end flow with `Latency`, entirely on the IronOxide_Processor | Chain over the flow's threads with deadline = the Latency upper bound; same-Partition hops on the flow map to Endpoints (IOP-03 would remove this) | a hop the Chain cannot bound (sporadic destination not released by the hop, or `discard`) | IronOxide R41.4 | IronOxide Chains need Endpoint hops |
| MAP-22 | Modes, mode transitions, mode-dependent properties or connections | none | always | PR-01, PR-03, PR-23, IronOxide NG-08, NG-09 | IronOxide's configuration is static |
| MAP-23 | Subprograms, subprogram groups, subprogram access and calls | none | always | PR-32 | messages travel only over Endpoints; HAMR does not support them either |
| MAP-24 | Other constructs: `in out` ports, feature groups, abstract features, fan-in, port connections to non-thread components other than MAP-20 devices, connection Timing Immediate or Delayed, Queue_Processing_Protocol other than FIFO, entry points other than Initialize and Compute, Dispatch_Jitter, and every kind no row maps | none | always | the PR-nn or IronOxide Rn.m that the diagnostic names for each kind (Requirement 16) | outside HAMR's port semantics or outside the profile |
| MAP-25 | seL4 scheduling properties on IronOxide elements: `Timing_Properties::Frame_Period`, `Clock_Period`, `Slot_Time`, `CASE_Scheduling::*`, `HAMR_Microkit::*` | ignored, with a warning | — | — | IronOxide schedules by fixed-priority preemptive SRP, not by a static domain schedule |
| MAP-26 | Limits | checked by the Backend before the IronOxide_Generator: PAR-03 to PAR-07, `ops` ≤ 255, the MPU layout fit (IronOxide R20.4) | any exceeded limit | the limit's IronOxide constant (`validate.rs`, `target.rs`, `layout.rs`) and PR-14, PR-33, IronOxide DD-01, DD-07 | report against the model instead of against generated code |
| MAP-27 | Processors and cross-processor connections | the IronOxide_Processor's processes form the Application; cross-processor connections go through the Deployment_Splitter and the Serial_Bridge (Requirements 34, 35) | no or several IronOxide_Processors; a cross-processor connection not on a bus with `IronOxide::Serial_Link`; cross-processor data or subprogram access | NG-05, PR-30, PR-32 | one IronOxide Application per model |
| MAP-28 | Floating-point use | Task `fpu` from `IronOxide::Uses_FPU` (default from the types the thread uses) | — | PR-31 (the Link_Checker's scan is the authority) | FP context handling and costs |
| MAP-29 | Initialize entry point | constant initial values from developer `const fn` functions for per-Task state and output data ports (Requirement 20) | initial values put on event or event data output ports | PR-34, PR-25 | IronOxide has no Application code before the first Release |
| MAP-30 | GUMBO clauses | Creusot contracts, Validation_Functions and Connection_Obligations (Requirements 22 to 24); `flows`, `monitor` and `compositions` clauses reported as not carried | compute, initialize, integration or handler clauses that use a construct the Contract_Translator cannot translate | R22.4 | no obligation is dropped silently |

**Table M-2 — Deviations from HAMR's AADL semantics** (ASM-04). Each row states the IronOxide behaviour that differs, why, and the requirement that bounds the difference.

| Row | AADL or HAMR behaviour | IronOxide behaviour | Rationale | Bounded by |
|---|---|---|---|---|
| DEV-01 | Inputs are frozen at the dispatch instant | Inputs are frozen when the Job starts (Receive Input in the Port_Glue), which may be later than the Release by preemption, blocking and J_rel | IronOxide has no Kernel operation at the Release instant on behalf of the Application | R19.2; the start delay is part of the WCRT (IronOxide R29) |
| DEV-02 | A data port reads the newest value; an unread value is overwritten | Across Partitions, the newest value received by the drain at Job start; under a timing failure the newest send is refused and counted instead of overwriting | no overwriting Endpoint exists (IOP-01) | R12 |
| DEV-03 | The Initialize entry point runs developer code | Constant initial values only; no initial event outputs | no Application code runs during IronOxide's Init (PR-34) | R20 |
| DEV-04 | An event port queues events up to Queue_Size with the port's overflow policy | Inside a Partition, a Release_Signal holds one pending Release; an event inside the MIT is an MIT_Violation, recorded and counted toward the Partition's threshold | IronOxide R10.2, R10.3 | R10.5 refuses sources faster than the MIT |
| DEV-05 | Event data overflow follows Overflow_Handling_Protocol | Across Partitions, a full channel refuses the newest message and the Kernel records the event against the sender | IronOxide R40.5 | R11.5, R11.6 |
| DEV-06 | Sporadic dispatch | Under `discard`, an event inside the MIT is dropped rather than dispatched later | IronOxide R10.2 | `defer` is the default (Table 2-1) and the only policy for interrupts (MAP-20) |
| DEV-07 | Equal-priority threads have no defined order in HAMR's back ends | Equal-priority Tasks start in Task declaration order, which the Backend sets to instance-path order | IronOxide DD-03 | R7.5 |
| DEV-08 | `Compute_Execution_Time` is a model property | The upper bound is a Budget the Kernel enforces with the Partition's Overrun_Response | IronOxide R18 | R6.6, R25.2 |

### Part B — Mapping and Generation

### Requirement 5: Threads to Tasks (Phase P1 periodic; P2 sporadic)

**User Story:** As a system modeler, I want each AADL thread to become one IronOxide Task with the release behaviour its dispatch protocol states, so that the model's threads and the analyzed Tasks are the same set.

#### Acceptance Criteria

1. THE Backend SHALL map each thread bound to the IronOxide_Processor to exactly one Task of the Partition of its process, a Periodic_Task for `Dispatch_Protocol => Periodic` (MAP-01) and a Sporadic_Task for `Dispatch_Protocol => Sporadic` (MAP-02). *Evidence: `test-map-01`, `test-map-02`, `proof-tv-core`.*
2. WHEN a thread is Sporadic, THE Backend SHALL give its Task the Release_Source that the thread's single Dispatch trigger determines and an MIT equal to the thread's `Timing_Properties::Period`. *Evidence: `test-map-02`, `proof-tv-core`.*
3. IF a Sporadic thread has no Dispatch trigger or more than one, THEN THE Backend SHALL refuse the thread naming PR-11 and MAP-02. *Evidence: `mut-refusals`.*
4. IF a thread's `Dispatch_Protocol` is Aperiodic, Timed, Hybrid or Background, or is absent, THEN THE Backend SHALL refuse the thread naming MAP-03 and the rule that MAP-03 gives for that protocol. *Evidence: `mut-refusals`.*
5. IF a thread declares an entry point other than Initialize and Compute, THEN THE Backend SHALL refuse the thread naming PR-02 and MAP-24. *Evidence: `mut-refusals`.*

### Requirement 6: Thread timing properties (Phase P1)

**User Story:** As a timing analyst, I want the model's timing properties carried into the Task_Model without silent changes, so that the analysis is of the system I modeled.

#### Acceptance Criteria

1. THE Backend SHALL set each Task's period or MIT from `Timing_Properties::Period`, its offset from `Timing_Properties::Dispatch_Offset` (0 when absent), its relative deadline from `Timing_Properties::Deadline` (the Period when absent, ASM-03), and its Budget from the upper bound of `Timing_Properties::Compute_Execution_Time`. *Evidence: `test-map-04`, `test-map-05`, `proof-tv-core`.*
2. THE Backend SHALL write each time value into the App_Declaration in the largest IronOxide time unit among ns, us, ms and s that represents the AADL value exactly, so that the only rounding is the IronOxide_Generator's (IronOxide R21.2). *Evidence: `pbt-time-exact`, `proof-tv-core`.*
3. IF a time value is not a whole number of nanoseconds, THEN THE Backend SHALL refuse the element naming MAP-04. *Evidence: `mut-refusals`.*
4. IF a thread lacks `Timing_Properties::Period` or `Timing_Properties::Compute_Execution_Time`, THEN THE Backend SHALL refuse the thread naming PR-27 and MAP-01 or MAP-05, and SHALL apply no default execution time. *Evidence: `mut-refusals`.*
5. IF a thread's Deadline exceeds its Period, or its `Compute_Execution_Time` upper bound exceeds its Deadline, THEN THE Backend SHALL refuse the thread naming IronOxide R22.1 and MAP-04 or MAP-05. *Evidence: `mut-refusals`.*
6. THE Backend SHALL treat the `Compute_Execution_Time` upper bound as the Budget of the whole Job, including the Port_Glue, and THE Verdict_Report SHALL state the share of that Budget taken by the Port_Glue's measured WCET once WCET_Records exist (DEV-08). *Evidence: `test-map-05`, `analysis-glue-share-<model>`.*

### Requirement 7: Priorities (Phase P1)

**User Story:** As a timing analyst, I want Priorities taken from the model or derived deadline-monotonically by one documented rule, so that the analysis and the model agree on which thread is more urgent.

#### Acceptance Criteria

1. WHERE every thread bound to the IronOxide_Processor has `Thread_Properties::Priority`, THE Backend SHALL map the distinct values, in increasing order, onto the Priorities 1 to k, so that the map preserves order and equality (ASM-02). *Evidence: `test-map-06`, `proof-tv-core`.*
2. WHERE no thread bound to the IronOxide_Processor has `Thread_Properties::Priority`, THE Backend SHALL assign deadline-monotonic Priorities 1 to k, a shorter relative deadline receiving a higher Priority and equal relative deadlines an equal Priority. *Evidence: `test-map-06`, `proof-tv-core`.*
3. IF some but not all threads bound to the IronOxide_Processor have `Thread_Properties::Priority`, THEN THE Backend SHALL refuse the model naming PR-03 and MAP-06 and listing the threads without the property. *Evidence: `mut-refusals`.*
4. IF the assignment of criterion 1 or 2 yields more than PAR-03 distinct Priorities, THEN THE Backend SHALL refuse the model naming PR-33 and MAP-06. *Evidence: `mut-refusals`.*
5. THE Backend SHALL declare Partitions in ascending order of process instance path and Tasks within each Partition in ascending order of thread instance path, so that equal-priority Tasks start in that order (IronOxide DD-03, DEV-07). *Evidence: `test-declaration-order`, `pbt-order-invariance`.*

### Requirement 8: Processes to Partitions (Phase P1)

**User Story:** As a system integrator, I want each AADL process to become one IronOxide Partition with the criticality level and fault responses I declare, so that spatial and temporal isolation follow the model's process structure.

#### Acceptance Criteria

1. THE Backend SHALL map each process bound to the IronOxide_Processor to exactly one Partition with its own Partition crate (IronOxide DD-11), whose `level`, `fault`, `overrun`, `deadline_miss` and `mit_threshold` are the process's IronOxide_Property_Set values (MAP-07, MAP-09). *Evidence: `test-map-07`, `test-map-09`, `proof-tv-core`.*
2. THE Backend SHALL map the threads of a thread group to Tasks of the Partition of the process that contains the thread group. *Evidence: `test-map-07`.*
3. IF a virtual processor is bound to the IronOxide_Processor, or a process on the IronOxide_Processor is bound to a virtual processor, THEN THE Backend SHALL refuse the model naming PR-28 and MAP-08. *Evidence: `mut-refusals`.*
4. IF a process declares `Overrun_Response => RECORD_AND_CONTINUE` while its Criticality_Level is lower than the highest Criticality_Level of the processes on the IronOxide_Processor, THEN THE Backend SHALL refuse the process naming IronOxide R18.5 and MAP-09. *Evidence: `mut-refusals`.*
5. IF a process on the IronOxide_Processor contains no thread, THEN THE Backend SHALL refuse the process naming MAP-07. *Evidence: `mut-refusals`.*
6. THE Backend SHALL derive Partition, crate and Task names from instance paths by a documented injective sanitization, and SHALL refuse a model in which two elements would receive the same name, naming both and PR-28. *Evidence: `pbt-names-injective`.*

### Requirement 9: Shared data to Resources (Phase P2)

**User Story:** As an application developer, I want AADL shared data inside a process to become an IronOxide Resource with a computed Ceiling, so that threads of one Partition can share state under the ceiling protocol.

#### Acceptance Criteria

1. THE Backend SHALL map each data subcomponent of a process on the IronOxide_Processor that threads of that process access through data-access connections to one Resource of that process's Partition whose accessors are exactly those threads, so that the IronOxide_Generator computes its Ceiling (PR-06, MAP-10). *Evidence: `test-map-10`, `proof-tv-core`.*
2. WHERE a thread's data access feature has `Access_Right => read_only`, THE Backend SHALL give that thread's Port_API only a shared reference to the Resource's value inside the lock closure. *Evidence: `test-map-10`, `check-generated-<model>`.*
3. IF threads of more than one process access one data component, THEN THE Backend SHALL refuse the data component naming PR-30 and MAP-10. *Evidence: `mut-refusals`.*
4. IF a shared data component has `Concurrency_Control_Protocol` with a value other than Priority_Ceiling, THEN THE Backend SHALL refuse the data component naming IronOxide Requirement 8 and MAP-10. *Evidence: `mut-refusals`.*
5. IF a data subcomponent of a process on the IronOxide_Processor is accessed by no thread through a data-access connection, THEN THE Backend SHALL refuse it naming IronOxide R22.5 and MAP-10. *Evidence: `mut-refusals`.*
6. THE Backend SHALL take each Resource's initial value from `Data_Model::Initial_Value` or, where that is absent, from a developer `const fn` that the skeleton declares, and its invariant from the GUMBO invariants of its data type as the Resource invariant of IronOxide R46.4. *Evidence: `test-map-10`, `proof-app-creusot-<model>`.*

### Requirement 10: Port connections inside a Partition (Phase P2)

**User Story:** As an application developer, I want connections between threads of one process carried by Resources and Release_Signals, so that IronOxide's eight Endpoints are kept for traffic between Partitions.

#### Acceptance Criteria

1. WHEN an event port connection links two threads of one Partition and the destination port is the destination thread's Dispatch trigger, THE Backend SHALL map the connection to a Release_Signal declared in that Partition, bound to the destination Task and raised by the source Task's Port_Glue at Job completion (MAP-11). *Evidence: `test-map-11`, `proof-tv-core`.*
2. WHEN an event port connection links two threads of one Partition and the destination thread is Periodic, THE Backend SHALL map the connection to a Port_Cell holding an event count that the source's Port_Glue increments at completion and the destination's Port_Glue reads and clears at Job start (MAP-12). *Evidence: `test-map-12`, `proof-app-glue`.*
3. WHEN an event data port connection links two threads of one Partition, THE Backend SHALL map the connection to a Port_Cell holding a bounded FIFO of capacity `Communication_Properties::Queue_Size` (1 when absent) with the destination port's overflow policy, plus a Release_Signal as in criterion 1 where the destination port is a Dispatch trigger (MAP-13). *Evidence: `test-map-13`, `proof-app-glue`.*
4. WHEN a data port connection links two threads of one Partition, THE Backend SHALL map the connection to a Port_Cell holding the latest value and a freshness flag, written by the source's Port_Glue at Job completion and copied by the destination's Port_Glue at Job start (MAP-14). *Evidence: `test-map-14`, `proof-app-glue`.*
5. IF the source thread of a connection mapped by criterion 1 or 3 has a period or MIT shorter than the destination thread's MIT, THEN THE Backend SHALL refuse the connection naming IronOxide R10.2 and MAP-11 (DEV-04). *Evidence: `mut-refusals`.*
6. THE Port_Glue SHALL access each Port_Cell through IronOxide's lock API at most once per Job of each accessing Task, so that each access is one Critical_Section site of the Task_Model. *Evidence: `check-generated-<model>`, `proof-app-glue`.*

### Requirement 11: Port connections across Partitions (Phase P2)

**User Story:** As a system integrator, I want connections between processes carried by IronOxide Endpoints with the queue size and overflow behaviour the model declares, so that cross-Partition communication is isolated and bounded.

#### Acceptance Criteria

1. WHEN a port connection links threads of two Partitions, THE Backend SHALL map it to one plain Endpoint whose sending role is bound to the source thread's Task and whose receiving role is bound to the destination thread's Task (MAP-15, MAP-16). *Evidence: `test-map-15`, `proof-tv-core`.*
2. WHEN such a connection is an event data connection, THE Backend SHALL set the Endpoint's capacity to the destination port's `Queue_Size` (1 when absent) and the receiving role's `ops` to 1 for `Dequeue_Protocol => OneItem`, to `Dequeued_Items` for MultipleItems, and to the capacity for AllItems. *Evidence: `test-map-15`, `proof-tv-core`.*
3. WHEN such a connection is an event connection, THE Backend SHALL give the Endpoint a 4-byte token payload and the capacity of criterion 2. *Evidence: `test-map-15`.*
4. WHEN the destination port is the destination thread's Dispatch trigger, THE Backend SHALL release the destination Task by its role of that Endpoint (`endpoint = NAME`) with the thread's `IronOxide::MIT_Policy`. *Evidence: `test-map-15`, `proof-tv-core`.*
5. WHERE the destination port's `Overflow_Handling_Protocol` is DropNewest or Error, THE Port_Glue SHALL treat a `Full` result of a send as the overflow, count it in the source's Port_API, and continue, the Kernel having recorded the event against the sending Partition (IronOxide R40.5, DEV-05). *Evidence: `test-map-17`, `proof-app-glue`.*
6. IF the destination port's `Overflow_Handling_Protocol` is DropOldest and the bound of R12.2, applied to the connection, does not exclude overflow, THEN THE Backend SHALL refuse the connection naming IronOxide R40.4 and MAP-17. *Evidence: `mut-refusals`, `proof-tv-sampling-capacity`.*
7. IF a destination port has more than one incoming connection, THEN THE Backend SHALL refuse the port naming PR-12, IronOxide R38.1 and MAP-24. *Evidence: `mut-refusals`.*
8. WHEN a source port has several outgoing connections, THE Backend SHALL map each connection to its own Endpoint, and THE Port_Glue SHALL send the same output value on each at Job completion. *Evidence: `test-map-15`, `proof-app-glue`.*
9. THE Backend SHALL declare each role's `ops` as the number of operations its Task's Port_Glue performs on that Endpoint per Job, so that the Analyzer charges them (IronOxide R40.2). *Evidence: `proof-tv-core`, `check-generated-<model>`.*

### Requirement 12: Data-port sampling semantics across Partitions (Phase P2)

**User Story:** As an application developer, I want a data port between processes to read the newest value, as AADL defines, so that control loops sample their inputs instead of consuming queues.

#### Acceptance Criteria

1. WHEN a data port connection links threads of two Partitions, THE Backend SHALL map it to a plain Endpoint of capacity C_sample whose receiving role has `ops` = C_sample, and THE Port_Glue of the destination SHALL receive at Job start until the channel is empty or C_sample messages are taken and keep the newest one in per-Task state as the port's frozen value (MAP-16, DEV-02). *Evidence: `test-map-16`, `proof-app-glue`.*
2. THE Backend SHALL compute C_sample from the source's period or MIT and relative deadline and the destination's period or MIT and relative deadline by the formula that design.md states, such that no send of the source finds the channel full while both Tasks complete every Job within its relative deadline (DD candidate). *Evidence: `proof-tv-sampling-capacity` (a Verus lemma over the formula), `pbt-sampling-no-full`.*
3. IF C_sample exceeds 255, or the Application's Endpoint storage then exceeds PAR-06, THEN THE Backend SHALL refuse the connection naming PR-14 and MAP-16. *Evidence: `mut-refusals`.*
4. WHILE no message has been received on a cross-Partition data port since instant 0, THE Port_API SHALL return the source thread's initial output value for that port (Requirement 20) and report the value as not fresh. *Evidence: `proof-app-glue`, `test-map-16`.*
5. IF a send on a sampling Endpoint returns `Full`, THEN THE Port_Glue SHALL count the refused send in the source's Port_API, and THE Verdict_Report SHALL list every such count observed under QEMU or on the HIL_Rig as a violation of the timing assumption of criterion 2. *Evidence: `hil-sampling-no-full`, `test-qemu-<model>`.*

### Requirement 13: Validation functions for upward connections (Phase P2)

**User Story:** As a safety engineer, I want every message that flows up in criticality checked against the receiver's stated assumptions, so that higher-criticality code relies on no property of lower-criticality code.

#### Acceptance Criteria

1. WHEN a connection is an Upward connection, THE Backend SHALL declare for its receiving role a Validation_Function that accepts a message exactly when the message decodes to a valid value of the port's data type (R14.2) and that value satisfies the conjunction of the destination port's GUMBO integration assume clauses (MAP-18). *Evidence: `test-map-18`, `proof-app-validators-<model>`.*
2. WHERE the destination port of an Upward connection has `IronOxide::Validation_Function`, THE Backend SHALL use the named developer function after the decoding check, and THE Contract_Translator SHALL emit a Creusot obligation that the function accepts only values that satisfy the port's GUMBO integration assume clauses, where the port has any. *Evidence: `proof-app-validators-<model>`.*
3. IF the destination port of an Upward connection has neither a GUMBO integration assume clause nor `IronOxide::Validation_Function`, THEN THE Backend SHALL refuse the connection naming IronOxide R42.4 and MAP-18. *Evidence: `mut-refusals`.*
4. THE Contract_Translator SHALL generate each Validation_Function so that the App_Prover proves it total and free of panics, with every GUMBO operation that is undefined on the received value making the function reject the message (R22.3). *Evidence: `proof-app-validators-<model>`, `mut-proof-validators`.*
5. WHEN a Validation_Function rejects a message, THE Port_API SHALL present the input as absent for that Job (event data and event ports) or keep the previous frozen value marked not fresh (data ports), and SHALL count the rejection, the Kernel having recorded a fault of the sending Partition (IronOxide R42.4). *Evidence: `proof-app-glue`, `hil-validation-reject`.*
6. WHERE IronOxide R42.4 requires a Validation_Function for a role of a plain Endpoint on a channel that the AADL connection never uses, THE Backend SHALL declare a Validation_Function that rejects every message. *Evidence: `test-map-18`, `check-generated-<model>`.*

### Requirement 14: Data types and wire formats (Phase P2)

**User Story:** As an application developer, I want AADL data types carried between Partitions in a form IronOxide accepts and decoded back exactly, so that my code works with the model's types.

#### Acceptance Criteria

1. THE Backend SHALL generate for each AADL data type used on a port an application Rust type and, for each such type used across Partitions or over the serial link, a Wire_Type that meets IronOxide's `Plain` bound (IronOxide R42.1): Booleans as `u8`, enumerations as `u32` discriminants, fixed-size arrays as arrays, bounded strings as byte arrays with a length, records as `#[repr(C)]` structs of Wire_Types, padded to a multiple of 4 bytes (MAP-19). *Evidence: `test-map-19`, `check-generated-<model>`.*
2. THE Backend SHALL generate an encoder from each application type to its Wire_Type and a decoder from the Wire_Type that rejects every bit pattern that encodes no valid value (a Boolean byte other than 0 or 1, an out-of-range discriminant, a length beyond the string's bound). *Evidence: `pbt-wire-round-trip`, `proof-wire-round-trip`.*
3. FOR ALL values v of an application type, decoding the encoding of v SHALL give v, and FOR ALL Wire_Type values w that the decoder accepts, encoding the decoded value SHALL give w (round trip). *Evidence: `pbt-wire-round-trip`, `proof-wire-round-trip` (Creusot).*
4. IF a port's data type is an integer without a bit width and without `HAMR::Default_Bit_Width`, an unbounded array or sequence, or a string without `HAMR::Max_String_Size`, THEN THE Backend SHALL refuse the port naming PR-14 and MAP-19. *Evidence: `mut-refusals`.*
5. IF a Wire_Type is larger than 65,532 bytes, THEN THE Backend SHALL refuse the port naming PR-14 and MAP-19. *Evidence: `mut-refusals`.*
6. THE Port_API SHALL count, per input port, the messages that its decoder rejected, as HAMR's Microkit Rust API does with `<port>_num_invalid`. *Evidence: `proof-app-glue`, `test-map-19`.*

### Requirement 15: Devices, peripherals and interrupts (Phase P2)

**User Story:** As a system integrator, I want AADL devices and process properties to state which Partition owns which peripheral and which interrupt releases which thread, so that IronOxide's ownership and MIT enforcement follow the model.

#### Acceptance Criteria

1. THE Backend SHALL assign to a Partition each peripheral named in its process's `IronOxide::Peripherals` and the `IronOxide::Peripheral` of each device whose ports connect only to threads of that process (MAP-20). *Evidence: `test-map-20`, `proof-tv-core`.*
2. WHEN a device's out event port is connected to a Sporadic thread's Dispatch trigger, THE Backend SHALL release that thread's Task by the peripheral's interrupt (`binds = PERIPHERAL`) with `mit_policy = defer` (IronOxide DD-06). *Evidence: `test-map-20`, `proof-tv-core`.*
3. IF a peripheral is not in the IronOxide_Target's peripheral table, is Kernel-owned, is assigned to more than one Partition, or is EasyDMA-capable without the Target's DMA register description, THEN THE Backend SHALL refuse it naming IronOxide R21.5, PR-29, PR-28 or IronOxide DD-08 respectively, and MAP-20. *Evidence: `mut-refusals`.*
4. IF a device has a data or event data port, a `Dispatch_Protocol`, or connections to threads of more than one process, THEN THE Backend SHALL refuse the device naming MAP-20. *Evidence: `mut-refusals`.*
5. IF a Partition needs more peripheral windows than PAR-07, THEN THE Backend SHALL refuse the Partition naming IronOxide DD-07 and MAP-26. *Evidence: `mut-refusals`.*
6. IF a thread released by an interrupt has `IronOxide::MIT_Policy => discard`, THEN THE Backend SHALL refuse the thread naming IronOxide DD-06 and MAP-20. *Evidence: `mut-refusals`.*

### Requirement 16: Constructs outside the profile (Phase P1)

**User Story:** As a system modeler, I want features that IronOxide's static profile cannot honour refused by name, so that no generated system silently differs from its model.

#### Acceptance Criteria

1. IF a component, connection or property on the IronOxide_Processor's side of the model has modes, mode transitions or mode-dependent values, THEN THE Backend SHALL refuse it naming PR-01, PR-23 and MAP-22. *Evidence: `mut-refusals`.*
2. IF a component on the IronOxide_Processor's side of the model has a subprogram, subprogram group, subprogram access feature or subprogram call, THEN THE Backend SHALL refuse it naming PR-32 and MAP-23. *Evidence: `mut-refusals`.*
3. IF a thread or process on the IronOxide_Processor has an `in out` port, a feature group, or an abstract feature, THEN THE Backend SHALL refuse the feature naming MAP-24 and the rule of the diagnostic table in design.md. *Evidence: `mut-refusals`.*
4. IF a port connection has `Communication_Properties::Timing` Immediate or Delayed, THEN THE Backend SHALL refuse it naming MAP-24 (DEV-01). *Evidence: `mut-refusals`.*
5. IF a port has `Queue_Processing_Protocol` other than FIFO, or a thread has `Dispatch_Jitter`, THEN THE Backend SHALL refuse it naming MAP-24. *Evidence: `mut-refusals`.*
6. IF a port connection links a thread to a component that is neither a thread nor a device of MAP-20, THEN THE Backend SHALL refuse the connection naming MAP-24. *Evidence: `mut-refusals`.*

### Requirement 17: IronOxide limits (Phase P1)

**User Story:** As a system modeler, I want IronOxide's capacity limits checked against my model, so that I learn which model elements exceed a limit before any Rust is compiled.

#### Acceptance Criteria

1. THE Backend SHALL check each limit of MAP-26 before it invokes the IronOxide_Generator, and SHALL refuse a model that exceeds one, naming the limit, the IronOxide file and constant that define it, and the AADL elements that contribute to it. *Evidence: `test-map-26`, `pbt-limits`.*
2. IF more than PAR-04 threads are bound to the IronOxide_Processor, THEN THE Backend SHALL refuse the model naming IronOxide DD-01, PAR-04 and IOP-02. *Evidence: `mut-refusals`.*
3. IF the model needs more than 8 Endpoints (PAR-05), THEN THE Backend SHALL refuse the model naming PR-14 and listing the connections that need Endpoints. *Evidence: `mut-refusals`.*
4. IF the IronOxide_Generator rejects an App_Declaration that passed the Backend's checks, THEN THE Build_System SHALL fail the run and report the case as a Backend defect with the IronOxide diagnostic and the AADL element of R3.4. *Evidence: `pbt-backend-checks-match-generator`.*

### Requirement 18: The generated Application (Phase P1)

**User Story:** As an application developer, I want a complete IronOxide Application generated from the model that IronOxide's own tools accept as is, so that I write only the thread bodies.

#### Acceptance Criteria

1. THE Backend SHALL generate, for an accepted model, a Cargo workspace with the declaration crate holding the `rsk::app!` App_Declaration, one Partition crate per Partition with Task skeletons and Port_Glue, a shared payload-type crate, the system and binary crates of IronOxide's template (IronOxide DD-11), and the build files. *Evidence: `test-generated-layout`.*
2. THE Backend SHALL generate code that the IronOxide_Generator, the Profile_Lint, the Link_Checker, the Config_Checker and the Analyzer of PAR-01 accept unchanged with no error-level diagnostic, for each IronOxide_Target whose peripheral table holds the model's peripherals. *Evidence: `check-generated-<model>` for every corpus model, `pbt-accepted-models-pass-ironoxide`.*
3. THE Backend SHALL generate only code that applies `#![forbid(unsafe_code)]` and uses no `static mut`, no `#![feature]`, no `core` item outside IronOxide's Core_Subset, no `derive(Debug)`, no recursion, no `dyn` or function pointers, and only loops whose bound is derivable from constants or stated by `#[rsk::bounds]` (PR-05, PR-16, PR-17, PR-18, PR-20, PR-21, PR-36). *Evidence: `check-profile-lint-<model>`, `check-link-<model>`.*
4. WHEN the Backend runs twice on the same AIR_Model with the same Backend version, THE Backend SHALL write byte-identical generated files. *Evidence: `test-determinism`.*
5. WHEN the Backend regenerates into a directory that holds developer code, THE Backend SHALL preserve every developer-owned file and every developer region, and SHALL overwrite only generated files and generated regions. *Evidence: `test-user-code-preserved`.*
6. WHEN a model is accepted, THE Build_System SHALL build the generated Application for `qemu_mps2_an386`, run it under QEMU for the model's `Operating_Duration`, and report the run as passing only when the Kernel's event log records no fault and no deadline miss. *Evidence: `test-qemu-<model>`.*

### Requirement 19: Port API and run-time services (Phase P2)

**User Story:** As an application developer, I want the AADL run-time services (Receive Input, Get Value, Put Value, Send Output) generated around my compute code, so that I write the same input-compute-output code HAMR's other back ends expect.

#### Acceptance Criteria

1. THE Backend SHALL generate for each Task a Port_API that gives the developer's compute code Get Value on every input port (the frozen value, or the frozen queue or count of an event-like port), Put Value on every output port, and no other access to Resources, Release_Signals or Endpoints that the Port_Glue uses. *Evidence: `test-port-api`, `check-generated-<model>`.*
2. WHEN a Job starts, THE Port_Glue SHALL perform Receive Input for every input port of the Task before the developer's compute code runs (DEV-01). *Evidence: `proof-app-glue`.*
3. WHEN the developer's compute code returns, THE Port_Glue SHALL perform Send Output for every output port written in that Job before the Job returns to its Release_Point. *Evidence: `proof-app-glue`.*
4. THE Port_Glue SHALL contain no loop other than the bounded drains of R11.2 and R12.1, each annotated with its bound. *Evidence: `check-profile-lint-<model>`.*
5. THE App_Prover SHALL prove that the Port_Glue passes to the compute code exactly the values taken by Receive Input and sends exactly the values that the compute code put, under the trusted contracts of IronOxide's `rsk` API. *Evidence: `proof-app-glue`, `mut-proof-glue`.*
6. THE Backend SHALL make each Task body the generated wrapper, so that developer compute code is reachable only through the wrapper. *Evidence: `check-link-<model>` (call graph from the Task entries).*

### Requirement 20: Initialization (Phase P2)

**User Story:** As an application developer, I want the model's initial state and initial output values set before the first Release, so that every input has a defined value from instant 0.

#### Acceptance Criteria

1. THE Backend SHALL generate per-Task state (PR-25) for each thread's GUMBO state variables and for the frozen values of its cross-Partition data inputs, initialized by developer `const fn` functions that the skeleton declares (MAP-29, DEV-03). *Evidence: `test-map-29`, `check-generated-<model>`.*
2. THE Backend SHALL take the initial value of each output data port from a developer `const fn` of the source thread's skeleton and use it as the initial content of the port's Port_Cell or of the destination's frozen value. *Evidence: `test-map-29`, `proof-app-glue`.*
3. IF a thread's GUMBO initialize clauses or `Programming_Properties::Initialize_Entrypoint_Source_Text` state that it puts a value on an event or event data output port during initialization, THEN THE Backend SHALL refuse the thread naming PR-34 and MAP-29. *Evidence: `mut-refusals`.*
4. THE Contract_Translator SHALL translate each GUMBO initialize guarantee clause into a Creusot postcondition of the `const fn` functions of criteria 1 and 2. *Evidence: `proof-app-creusot-<model>`.*

### Part C — Contracts

### Requirement 21: Application prover (Phase P2)

**User Story:** As a verification engineer, I want GUMBO contracts to become obligations for the prover IronOxide already uses for Application code, so that one prover, one toolchain and one set of trusted API contracts cover the generated system.

#### Acceptance Criteria

1. THE Contract_Translator SHALL emit GUMBO-derived proof obligations as Creusot (Pearlite) contracts on the generated skeleton functions, because IronOxide designates Creusot as its one App_Prover (IronOxide R46.2, DD-13), its `rsk` API carries Creusot contracts and its Resource invariants are Pearlite (IronOxide R46.4), and its Application crates build with rustc 1.95.0 and no `#![feature]` (PR-36), while HAMR's Verus output needs `#![feature]`, `RUSTC_BOOTSTRAP`, rustc 1.99.1 and a Verus release other than IronOxide's (`docs/hamr_study.md` section 6) (DD candidate). *Evidence: `review-prover-decision`, `check-generated-<model>`.*
2. THE Build_System SHALL run the App_Prover at IronOxide's pinned Creusot, Why3 and solver versions on every Partition crate of every corpus model and fail when a goal is not discharged. *Evidence: `proof-app-creusot-<model>`.*
3. WHERE a Partition's Criticality_Level is A or B, THE App_Prover SHALL prove the absence of panics, arithmetic overflow and out-of-bounds access in its generated code and its developer compute code (IronOxide R46.3). *Evidence: `proof-app-creusot-<model>`.*
4. THE Build_System SHALL build the TV_Checker and every other Verus artefact of this project as host tools outside every Flight_Build, so that no Verus crate is linked into an Application. *Evidence: `check-link-<model>` (no `vstd` symbol), `check-deps`.*

### Requirement 22: GUMBO contract translation (Phase P2)

**User Story:** As an application developer, I want the model's GUMBO contracts to appear as checkable contracts on my compute code, so that proving the code proves what the model states.

#### Acceptance Criteria

1. THE Contract_Translator SHALL translate a thread's GUMBO compute assume clauses into `requires` and its compute guarantee clauses, compute cases (each as an implication from its assumption to its guarantee) and the handler clauses of its Dispatch trigger into `ensures` of the developer's compute function, stated over the Port_API's frozen inputs, the outputs, and the pre-state and post-state. *Evidence: `test-contracts-translation`, `proof-app-creusot-<model>`.*
2. THE Contract_Translator SHALL translate GUMBO state variables into per-Task state fields, GUMBO functions into Creusot logic functions each with an executable twin proven equal to it, `modifies` clauses into frame conditions, and GUMBO data type invariants into Wire_Type validity checks and Resource invariants. *Evidence: `test-contracts-translation`, `proof-app-creusot-<model>`.*
3. THE Contract_Translator SHALL give every GUMBO arithmetic, conversion and indexing operation the semantics that R23.3 fixes, with an operation undefined on its operands making a `requires` clause or Validation_Function false, and adding to an `ensures` clause the obligation that the operation is defined (ASM-09, finding F-1). *Evidence: `proof-app-creusot-<model>`, `diff-contract-gumbox`, `mut-proof-contracts`.*
4. IF a GUMBO compute, initialize, integration or handler clause uses a construct that the Contract_Translator does not translate, THEN THE Contract_Translator SHALL refuse the clause naming the clause, its source position and MAP-30, rather than drop it. *Evidence: `mut-refusals`.*
5. WHEN a thread has GUMBO `flows`, `monitor` or `compositions` clauses, THE Contract_Translator SHALL report each as not carried into the IronOxide Application, naming the clause and MAP-30, and THE Trust_Base_Register SHALL list such clauses as unchecked on IronOxide. *Evidence: `test-contracts-not-carried`, `check-registers`.*
6. THE Contract_Translator SHALL write beside each generated contract the GUMBO clause identifier and source position it comes from, so that every App_Prover goal traces to a clause. *Evidence: `test-contracts-traced`.*

### Requirement 23: Contract semantics and their correspondence (Phase P2 statement; P4 proof)

**User Story:** As a certification engineer, I want the meaning of a GUMBO contract and the meaning of the generated obligation stated, and their correspondence proven or argued, so that a proof of the code counts as evidence for the model's contract.

#### Acceptance Criteria

1. THE Verification_Plan SHALL state the meaning of a GUMBO component contract under HAMR's semantics (ASM-04): for every dispatch, if the frozen inputs and the pre-state satisfy the compute assume and integration assume clauses, then the outputs released at completion and the post-state satisfy the compute guarantee clauses, the cases whose assumptions hold, the Dispatch trigger's handler clauses and the output ports' integration guarantee clauses; and the initial state and initial outputs satisfy the initialize guarantee clauses. *Evidence: `doc-contract-semantics`.*
2. THE Verification_Plan SHALL state the meaning of the generated obligation: every Job whose Receive Input values and pre-state satisfy the translated `requires` ends with outputs and a post-state that satisfy the translated `ensures`, as the App_Prover proves it. *Evidence: `doc-contract-semantics`.*
3. THE Verification_Plan SHALL fix the semantics of every GUMBO operation on bounded integers, floating-point numbers, Booleans, enumerations, arrays and records that the Contract_Translator accepts, citing the Slang and GUMBO documentation for each, and SHALL list each operation for which HAMR's Verus translation or GUMBOX executable has a different meaning (finding F-1). *Evidence: `doc-contract-semantics`, `review-contract-semantics`.*
4. THE Verification_Plan SHALL contain an argument (Phase P2), and from Phase P4 a mechanized proof in Verus or Creusot over a model of the Port_Glue, that the generated obligation of criterion 2, the Port_Glue's proven behaviour (R19.5) and the deviations of Table M-2 together imply the meaning of criterion 1 for every Job, with each deviation's effect stated. *Evidence: `review-contract-correspondence` (P2), `proof-contract-correspondence` (P4), `mut-proof-correspondence`.*
5. THE Build_System SHALL evaluate each translated clause's executable twin (R22.2) and HAMR's GUMBOX executable of the same clause on PAR-18 random inputs where both are defined, and fail on any difference in their results. *Evidence: `diff-contract-gumbox`.*
6. THE Trust_Base_Register SHALL list every step from GUMBO text to the App_Prover's verdict that no proof or test of this project covers, at least: the Front_End and the Slang type checking of GUMBO, the AIR serialization, the semantics of criterion 3, Creusot with Why3 and its solvers, and the trusted contracts of IronOxide's `rsk` API. *Evidence: `check-registers`.*

### Requirement 24: Connection obligations (Phase P2)

**User Story:** As a verification engineer, I want each receiver's integration assumptions discharged by its sender's guarantees or by a run-time check, so that no assumption is taken on trust.

#### Acceptance Criteria

1. WHEN a connection is not an Upward connection and its destination port has GUMBO integration assume clauses, THE Contract_Translator SHALL emit a Connection_Obligation that the conjunction of the source port's integration guarantee clauses implies the conjunction of the destination port's integration assume clauses over the port's data type, and THE App_Prover SHALL prove it. *Evidence: `proof-connection-obligations-<model>`.*
2. IF a Connection_Obligation of criterion 1 is not proven, including where the source port has no integration guarantee clause, THEN THE Build_System SHALL fail the proof job naming the connection and both ports. *Evidence: `mut-proof-connections`.*
3. WHEN a connection is an Upward connection, THE Backend SHALL discharge the destination port's integration assume clauses by the Validation_Function of Requirement 13 instead of a Connection_Obligation. *Evidence: `proof-app-validators-<model>`.*

### Part D — Timing

### Requirement 25: Timing in IronOxide's analysis (Phase P3)

**User Story:** As a timing analyst, I want the model's timing analyzed by IronOxide's Analyzer with measured execution times, so that the verdict applies to the generated binary.

#### Acceptance Criteria

1. THE Build_System SHALL run the Analyzer on the Task_Model of every accepted model with the Kernel_Timing_Parameters measured for the IronOxide_Target and with WCET_Records, and SHALL label as development evidence every run that uses placeholder WCET_Records equal to the Budgets. *Evidence: `analysis-rta-<model>`.*
2. WHEN WCET_Records measured by IronOxide's WCET_Harness exist for a generated Application, THE Verdict_Report SHALL give for each thread the measured WCET with the margin of PAR-11, the `Compute_Execution_Time` upper bound and their ratio, and SHALL report as a model defect each thread whose measured WCET exceeds that bound (IronOxide R28.4). *Evidence: `analysis-rta-<model>-measured`.*
3. THE Backend SHALL generate the Port_Glue so that every Critical_Section site and every Endpoint operation of a Task is an item of the Task_Model to which the WCET_Harness attaches a WCET_Record (IronOxide R25.3, R40.2). *Evidence: `check-generated-<model>`, `hil-wcet-<model>`.*
4. WHEN an analysis fails, THE Verdict_Report SHALL state the timing property changes that the Analyzer's report suggests, and THE Backend SHALL leave the model's properties unchanged. *Evidence: `test-verdict-report`.*

### Requirement 26: End-to-end flows to chains (Phase P3)

**User Story:** As a control engineer, I want the model's end-to-end flow latencies checked by IronOxide's chain analysis, so that the sensing-to-actuation path is bounded as specified.

#### Acceptance Criteria

1. WHEN an end-to-end flow with `Communication_Properties::Latency` runs entirely through threads on the IronOxide_Processor, from a thread or from a device's out event port, THE Backend SHALL declare an IronOxide Chain over the flow's threads in flow order whose deadline is the Latency's upper bound (MAP-21). *Evidence: `test-map-21`, `proof-tv-core`.*
2. WHEN a connection between threads of one Partition lies on such a flow, THE Backend SHALL map the connection to an Endpoint as in Requirement 11 instead of a Port_Cell or Release_Signal, so that IronOxide R41.4 applies (IOP-03). *Evidence: `test-map-21`.*
3. IF a hop of such a flow has a Sporadic destination that the hop's Endpoint does not release, or that is released under `discard`, THEN THE Backend SHALL refuse the flow naming IronOxide R41.4 and MAP-21. *Evidence: `mut-refusals`.*
4. WHEN an end-to-end flow crosses from the IronOxide_Processor to another processor, THE Verdict_Report SHALL list the flow as not analyzed, naming the flow and the processor. *Evidence: `test-verdict-report`.*
5. WHEN a flow's Latency has a lower bound greater than zero, THE Verdict_Report SHALL state that the lower bound is not checked. *Evidence: `test-verdict-report`.*

### Requirement 27: Verdicts reported against the model (Phase P3)

**User Story:** As a system modeler, I want the Analyzer's verdict expressed in terms of my threads and flows, so that I can act on it in the model.

#### Acceptance Criteria

1. WHEN the Analyzer has run, THE Verdict_Reporter SHALL write a Verdict_Report, as JSON in a published versioned schema (`hamr-rsk-verdict/1`) and as text, that gives for each thread its instance path, source position, Task, Priority, WCRT or lower bound, deadline, slack and verdict, for each end-to-end flow its Chain bound, Latency and verdict, and the Analyzer's overall result and exit status. *Evidence: `test-verdict-report`, `analysis-rta-<model>`.*
2. THE Verdict_Reporter SHALL copy the Analyzer's verdicts and numbers without recomputing them, and SHALL state the hashes of the Task_Model, WCET_Records, Kernel_Timing_Parameters and Build_Manifest that the Analyzer used (IronOxide R30.2). *Evidence: `test-verdict-report`.*
3. THE Verdict_Reporter SHALL state in every Verdict_Report the limits of IronOxide's verdict at PAR-01: WCETs are measured with a margin, not statically bounded; L_kernel is the measured masked span; and the Analyzer is not proven sound against the Kernel's schedules (`docs/hamr_study.md` section 11). *Evidence: `test-verdict-report`.*

### Part E — Translation Validation

### Requirement 28: The Semantic_Model (Phase P1 format; P4 independence)

**User Story:** As a verification engineer, I want a semantic model of the AADL input extracted independently of the Backend, so that the TV_Checker compares the output with the input rather than with the Backend's own view of it.

#### Acceptance Criteria

1. THE Semantic_Model_Extractor SHALL write, from the AIR_Model, a Semantic_Model in a published versioned JSON schema (`hamr-rsk-semantic-model/1`) that records for the part of the model bound to the IronOxide_Processor every thread with its dispatch and timing properties, every process with its IronOxide_Property_Set values, every port, connection, data access, device, end-to-end flow and GUMBO clause identifier, and every construct of a kind that Table M-1 refuses. *Evidence: `test-semantic-model`.*
2. THE Semantic_Model_Extractor SHALL share no mapping code and no data structure with the Backend other than HAMR's AIR reader and the Semantic_Model schema. *Evidence: `check-extractor-independence` (dependency check), `review-extractor-independence`.*
3. THE Semantic_Model_Extractor SHALL record every time value as an integer number of picoseconds, with the AADL value and unit kept beside it. *Evidence: `test-semantic-model`.*
4. FOR ALL Semantic_Models m, the TV_Checker's reader SHALL parse the extractor's printing of m into a model equal to m (round trip). *Evidence: `pbt-semantic-model-round-trip`.*

### Requirement 29: The translation-validation checker (Phase P4)

**User Story:** As a certification engineer, I want every generated configuration checked against the model by a proven checker, so that HAMR's and the Backend's code need not be trusted for the configuration.

#### Acceptance Criteria

1. THE TV_Checker SHALL take a Semantic_Model and the Task_Model that the IronOxide_Generator wrote for the generated App_Declaration, and SHALL accept the pair only if the Mapping_Relation holds between them. *Evidence: `proof-tv-core`, `test-tv-accepts-<model>`.*
2. THE Mapping_Relation SHALL be stated as Verus specification functions over both models and SHALL require at least: a bijection between threads and Tasks and between processes and Partitions; for each Task, the release kind, Release_Source, period or MIT, offset, deadline and Budget that Table M-1 and IronOxide's conversion rules (IronOxide R21.2) give for its thread; Priorities ordered as MAP-06 requires; each Partition's level, responses, threshold and peripherals; each shared data component's Resource and accessors; each connection's Release_Signal, Port_Cell or Endpoint with its roles, capacity, `ops` and Validation_Function; each end-to-end flow's Chain and deadline; the FPU flags; and no Task_Model item that is not the image of a model element or of Port_Glue that Table M-1 prescribes. *Evidence: `review-mapping-relation`, `proof-tv-core`.*
3. THE TV_Checker's core SHALL be written in Verus with a proof that it returns Accept only if the Mapping_Relation holds, and THE Build_System SHALL run that proof with pinned Verus and Z3 versions and fail when it does not verify. *Evidence: `proof-tv-core`, `mut-proof-tv`.*
4. IF the Mapping_Relation does not hold, THEN THE TV_Checker SHALL reject the pair naming the first violated conjunct of the Mapping_Relation, the model element and the Task_Model item. *Evidence: `test-tv-rejects`, `mut-translation`.*
5. THE Build_System SHALL run the TV_Checker on every generated Application before any IronOxide tool runs on it, and SHALL fail the build when the TV_Checker rejects. *Evidence: `check-tv-<model>`.*
6. THE TV_Checker SHALL use outside its Verus core only a JSON reader for each of the two schemas, each covered by the round-trip test of R28.4 or its Task_Model counterpart and listed in the Trust_Base_Register. *Evidence: `pbt-task-model-reader-round-trip`, `check-registers`.*
7. THE Verification_Plan SHALL state that the evidence from the AADL model to the binary is the TV_Checker (Semantic_Model to Task_Model) followed by IronOxide's Config_Checker (Task_Model to binary, IronOxide R26), and SHALL list what neither covers, at least the Semantic_Model_Extractor, the Port_Glue's behaviour (covered by the App_Prover, R19.5) and developer code. *Evidence: `doc-verification-plan`.*

### Requirement 30: Translation mutations (Phase P4)

**User Story:** As a verification engineer, I want planted defects in generated configurations and in models, so that the TV_Checker and the Backend's refusals are shown to catch what they claim to catch.

#### Acceptance Criteria

1. THE Translation_Mutation_Register SHALL list, for each conjunct of the Mapping_Relation, at least one mutation of a generated App_Declaration or Task_Model that violates only that conjunct, and THE Build_System SHALL report the case as passing only when the TV_Checker rejects it naming that conjunct. *Evidence: `mut-translation`.*
2. THE Translation_Mutation_Register SHALL list, for each refusal of Table M-1, at least one model that contains only that refused construct, and THE Build_System SHALL report the case as passing only when the Backend refuses it with the diagnostic of that row. *Evidence: `mut-refusals`.*
3. THE Build_System SHALL fail when a conjunct of the Mapping_Relation or a refusal of Table M-1 has no registered mutation. *Evidence: `check-registers`.*

### Part F — Testing

### Requirement 31: Property-based tests over generated models (Phase P2; P3 timing)

**User Story:** As a verification engineer, I want randomly generated models pushed through the Backend and IronOxide's tools, so that the claim "everything the Backend accepts passes IronOxide's checks" is tested beyond hand-written examples.

#### Acceptance Criteria

1. THE Model_Generator SHALL generate, with a recorded seed, SysML v2 models in the form of HAMR's AADL libraries and AIR_Models that cover every row of Table M-1, including models at and one step past each limit of MAP-26. *Evidence: `pbt-generator-coverage` (row coverage report).*
2. FOR ALL models the Backend accepts, the generated Application SHALL pass the IronOxide_Generator, the Profile_Lint, the Link_Checker, the Config_Checker and the TV_Checker, and the Analyzer SHALL return PASS or FAIL rather than a rejected input or an internal error. *Evidence: `pbt-accepted-models-pass-ironoxide` over PAR-08 models.*
3. FOR ALL models the Backend refuses, every diagnostic SHALL name an element present in the model, an IronOxide rule and a MAP row (Requirement 3). *Evidence: `pbt-refusals-named`.*
4. FOR ALL accepted models, the Backend's output SHALL be unchanged by permuting the order in which the model declares components, features and connections. *Evidence: `pbt-order-invariance`.*
5. FOR ALL accepted models, increasing one thread's `Compute_Execution_Time` upper bound or decreasing one thread's Period SHALL never turn the Analyzer's verdict from FAIL to PASS (metamorphic property). *Evidence: `pbt-timing-monotone`.*
6. WHEN a property-based test fails, THE Build_System SHALL shrink the failing model, store it with its seed as a regression case, and fail the run. *Evidence: `pbt-regressions`.*

### Requirement 32: Differential tests against HAMR's JVM simulation (Phase P3 QEMU; P4 HIL_Rig)

**User Story:** As a verification engineer, I want the same model run on HAMR's JVM simulation and on IronOxide and the observable traces compared, so that differences in port semantics and dispatching show up as test failures.

#### Acceptance Criteria

1. THE Backend SHALL generate, for each model of the differential corpus, Trace_Bodies in Slang for HAMR's JVM platform and in Rust for the IronOxide Application that compute every output as the same documented deterministic function of the thread's frozen inputs, its state and its dispatch count. *Evidence: `test-trace-bodies`.*
2. THE Build_System SHALL run each model of the differential corpus (PAR-09) on HAMR's JVM simulation and on the IronOxide Application under QEMU (Phase P3) and on the HIL_Rig (Phase P4), recording for each port the values put and the values frozen with their dispatch indices and, on IronOxide, Kernel time stamps. *Evidence: `diff-jvm-qemu-<model>`, `diff-jvm-rig-<model>`.*
3. THE Trace_Comparator SHALL compare the traces under the comparison relation that design.md states, which SHALL at least require, for every event data connection without a recorded overflow, the same value sequence with no loss, duplication or reordering; for every data connection, that every frozen value is a value the source put within the staleness bound that design.md derives from the two Tasks' periods and deadlines and DEV-01; and for every Periodic thread, dispatch counts that agree within one per hyperperiod (DD candidate). *Evidence: `diff-jvm-qemu-<model>`.*
4. IF two traces differ under the comparison relation, THEN THE Trace_Comparator SHALL report the first difference with its connection, dispatch indices and both values, and THE Build_System SHALL fail the run. *Evidence: `diff-jvm-qemu-<model>`, `mut-diff` (a planted port-semantics defect in the Port_Glue of a test build).*
5. THE Trace_Comparator SHALL check both traces against the GUMBOX executables of the model's GUMBO clauses and report every violated clause with its dispatch. *Evidence: `diff-jvm-qemu-<model>`.*
6. THE Verification_Plan SHALL list the models excluded from the differential corpus because HAMR's JVM back end rejects them or does not compile them at PAR-02 (finding F-2). *Evidence: `doc-verification-plan`.*

### Requirement 33: Proof mutations (Phase P2 Creusot; P4 Verus)

**User Story:** As a verification engineer, I want every proof of this project to reject planted defects, so that no proof is vacuous.

#### Acceptance Criteria

1. THE Proof_Mutation_Register SHALL list, in the format of IronOxide's `docs/proof_mutations.toml`, PAR-17 planted defects for every proof artifact of this project: the TV_Checker core, the sampling-capacity lemma, the contract correspondence, the wire and frame round trips, the Port_Glue, the Connection_Obligations and the Validation_Functions. *Evidence: `check-registers`.*
2. THE Build_System SHALL apply each registered mutation to a copy of the tree, run the prover named for it, and count the mutation as killed only when the prover rejects the mutant with output containing the registered text, failing the verification job on any surviving mutant. *Evidence: `mut-proof-tv`, `mut-proof-glue`, `mut-proof-contracts`, `mut-proof-correspondence`, `mut-proof-validators`, `mut-proof-connections`, `mut-proof-codecs`.*
3. IF a registered mutation's search text does not occur exactly once in its file, THEN THE Build_System SHALL fail naming the mutation. *Evidence: `check-registers`.*
4. WHEN a proof artifact is added or its top-level specification changes, THE Build_System SHALL fail until the Proof_Mutation_Register holds the mutations that criterion 1 requires for it. *Evidence: `check-registers`.*

### Part G — Robot Case Study

### Requirement 34: Two-processor models (Phase P5)

**User Story:** As a system modeler, I want to model the robot's two processors in one model and generate each side with its own back end, so that the interface between them has one definition.

#### Acceptance Criteria

1. THE Deployment_Splitter SHALL split a model with one IronOxide_Processor and one Peer_Processor into an IronOxide sub-model and a peer sub-model, each with exactly one bound processor, replacing every connection between the two processors by a connection to a generated Serial_Bridge component on each side with the same port kind, data type and Queue_Size (MAP-27). *Evidence: `test-splitter`, `pbt-splitter`.*
2. THE Deployment_Splitter SHALL produce a peer sub-model that the existing HAMR back end selected for the Pi_Side (ORQ-09) accepts and generates unchanged at PAR-02. *Evidence: `test-splitter-peer-codegen`.*
3. THE Deployment_Splitter SHALL keep every GUMBO clause and property of a component in the sub-model that holds the component, and SHALL attach to each Serial_Bridge port the integration clauses of the port it stands for. *Evidence: `test-splitter`, `proof-tv-core`.*
4. IF a cross-processor connection is a data access or subprogram access, or is bound to no bus that has `IronOxide::Serial_Link`, THEN THE Deployment_Splitter SHALL refuse it naming MAP-27. *Evidence: `mut-refusals`.*
5. THE TV_Checker SHALL check the Task_Model of the IronOxide sub-model against the Semantic_Model of the original model's IronOxide part, with the Serial_Bridge substitutions of criterion 1 part of the Mapping_Relation. *Evidence: `check-tv-robot`.*

### Requirement 35: The serial link bridge (Phase P5)

**User Story:** As a safety engineer, I want the serial link framed, checked and decoded by generated code with a proven parser on the MCU, so that corrupted or hostile bytes from the Pi cannot reach the motor controller unchecked.

#### Acceptance Criteria

1. THE Serial_Bridge SHALL carry each cross-processor connection's messages in frames of at most PAR-13 payload bytes, each with a frame type that identifies the connection, a sequence number and a CRC, at the line settings of PAR-12. *Evidence: `test-serial-frames`, `doc-frame-grammar`.*
2. THE Backend SHALL generate the Frame_Codec for both sides from the model's data types: a printer and a parser for each frame type, following the frame grammar that design.md states. *Evidence: `test-serial-frames`.*
3. FOR ALL valid frames f, parsing the printing of f SHALL give f, and FOR ALL byte strings that the parser accepts, printing the parse result SHALL give the accepted bytes (round trip), on the MCU_Side and on the Pi_Side. *Evidence: `pbt-frame-round-trip-mcu`, `pbt-frame-round-trip-peer`, `proof-frame-parser` (Creusot, MCU_Side), `mut-proof-codecs`.*
4. IF the MCU_Side parser receives bytes that form no valid frame (a CRC mismatch, an unknown frame type, a length beyond PAR-13, or a Wire_Type encoding that R14.2 rejects), THEN THE Serial_Bridge SHALL discard them, count them, and resynchronize at the next frame delimiter without delivering any part of them. *Evidence: `pbt-frame-garbage`, `hil-serial-faults`.*
5. THE Robot_Model SHALL place the MCU_Side Serial_Bridge in a Partition whose Criticality_Level is at most that of every Partition it delivers to, so that every delivery of peer data to a higher Criticality_Level passes a Validation_Function (Requirement 13). *Evidence: `check-tv-robot`, `review-robot-model`.*
6. WHEN the MCU_Side has received no valid frame for PAR-14, THE Serial_Bridge SHALL report link loss to every Task that consumes peer data within one period of the Serial_Bridge's receiving Task. *Evidence: `hil-serial-faults`.*
7. THE MCU_Side Serial_Bridge SHALL use UARTE0 through IronOxide's EasyDMA services (`rsk::dma_request`, `rsk::dma_write`; IronOxide DD-08), with its receiving Task released by the UARTE0 interrupt under an MIT that design.md derives from PAR-12 and PAR-13. *Evidence: `check-generated-robot`, `hil-serial-throughput`.*

### Requirement 36: The robot case-study model (Phase P5)

**User Story:** As the project lead, I want a two-processor ground robot modeled and generated end to end, so that the back end is exercised by a system with real control, safety and communication needs.

#### Acceptance Criteria

1. THE Robot_Model SHALL model, in one model, a ground robot with a Raspberry Pi 5 Peer_Processor running Linux (perception, planning, operator commands, telemetry) and an nRF52840 IronOxide_Processor (wheel motor control at PAR-16, wheel encoders, safety supervision), connected by a serial bus that has `IronOxide::Serial_Link`. *Evidence: `review-robot-model`, `test-robot-model-parses`.*
2. THE Robot_Model SHALL place the MCU functions in at least three processes with distinct Criticality_Levels: the safety supervisor at the highest, motor control below it, and the Serial_Bridge at the lowest. *Evidence: `check-tv-robot`.*
3. THE Robot_Model SHALL state GUMBO contracts for the safety supervisor and the motor controller, at least that commanded wheel speeds stay within PAR-19 and that the outputs of criteria 4 and 5 hold. *Evidence: `proof-app-creusot-robot`.*
4. WHEN the safety supervisor has observed no valid velocity command for PAR-14, THE MCU_Side SHALL drive every motor PWM output to zero duty within PAR-15. *Evidence: `hil-robot-timeout`.*
5. WHEN the emergency-stop input is asserted, THE MCU_Side SHALL drive every motor PWM output to zero duty within PAR-15 and keep it at zero until an operator reset command arrives with the input released. *Evidence: `hil-robot-estop`.*
6. IF a velocity command requests a wheel speed beyond PAR-19, THEN THE MCU_Side SHALL limit the commanded speed to PAR-19 and count the event. *Evidence: `proof-app-creusot-robot`, `hil-robot-limits`.*
7. THE Backend SHALL generate the MCU_Side, and the selected existing HAMR back end SHALL generate the Pi_Side, from the Robot_Model through the Deployment_Splitter, with no edit to generated files outside developer regions. *Evidence: `test-robot-generation`, `check-generated-robot`.*
8. THE Robot_Model SHALL fit the limits of MAP-26 (at most PAR-04 MCU threads, 8 Endpoints, PAR-07 peripheral windows per Partition, one owner per peripheral), and design.md SHALL record how the ownership of the GPIO port P0 and of GPIOTE is resolved among the MCU Partitions (ORQ-11). *Evidence: `check-generated-robot`, `review-robot-model`.*

### Requirement 37: The MCU side on the HIL_Rig (Phase P5)

**User Story:** As a timing analyst, I want the robot's MCU side run on IronOxide's rig with its observed response times compared with the Analyzer's bounds, so that the analysis is confirmed on hardware as in IronOxide's task 19.4.

#### Acceptance Criteria

1. THE Build_System SHALL run the MCU_Side's Flight_Build on the HIL_Rig's target board for PAR-10, with the Pi_Stand_In driving the serial link with traffic generated from the peer sub-model and the stimulus board generating the encoder and emergency-stop signals. *Evidence: `hil-robot-nominal`.*
2. THE HIL_Rig run SHALL record each Job's release and completion, each Chain's end-to-end latency, the PWM outputs and the Kernel's event log. *Evidence: `hil-robot-nominal`.*
3. THE Verdict_Report SHALL compare each observed response time and Chain latency with the Analyzer's WCRT and Chain bound computed from WCET_Records measured on the same binary, and THE Build_System SHALL report the run as passing only when no observation exceeds its bound. *Evidence: `hil-robot-nominal`, `analysis-rta-robot-measured`.*
4. THE Build_System SHALL repeat the run with injected Overruns and with malformed, late and missing frames in the Serial_Bridge's Partition, and SHALL report a pass only when every Job of the safety supervisor completes within its WCRT and R36.4 and R36.5 hold in every injected case (IronOxide R18.7, R18.8). *Evidence: `hil-robot-faults`.*
5. THE Build_System SHALL check before each HIL_Rig run that the binary read back from the target board matches the Build_Manifest's hash (IronOxide R48.4). *Evidence: `hil-robot-nominal`.*

### Requirement 38: Robot integration (Phase P6, later phase)

**User Story:** As the project lead, I want the integration on the physical robot specified now and run later, so that the rig results lead to a planned, hazard-controlled robot test.

#### Acceptance Criteria

1. THE Robot_Integration_Plan SHALL specify, before Phase P6 starts, the physical robot (chassis, motor drivers, encoders, emergency-stop circuit, power), the Raspberry Pi 5 software image that runs the Pi_Side, and the wiring between the nRF52840-DK and the robot (ASM-10). *Evidence: `doc-robot-integration-plan`.*
2. THE Robot_Integration_Plan SHALL state the hazards of powered-wheel tests on the bench and in the field and the measures that limit each hazard before the first powered test. *Evidence: `doc-robot-integration-plan`, `review-robot-hazards`.*
3. WHEN Phase P6 runs, THE Build_System SHALL run the generated Pi_Side on the Raspberry Pi 5 and the MCU_Side on the robot's nRF52840, and THE Verification_Plan SHALL name the tests that show R36.4, R36.5 and R36.6 on the physical robot. *Evidence: `test-robot-field-<n>` (Phase P6).*

### Part H — Verification Plan, Registers, and Build

### Requirement 39: Verification plan and evidence (Phase P1)

**User Story:** As a certification engineer, I want every claim of this project allocated to a tool and an Evidence_Item, so that no claim stands without evidence, as in IronOxide's own plan.

#### Acceptance Criteria

1. THE Verification_Plan SHALL contain a property allocation table that assigns each guarantee this project claims (at least: mapping correctness, refusal completeness, profile conformance of generated code, wire and frame round trips, sampling capacity, Validation_Function totality, contract correspondence, timing verdict mapping, MCU_Side response times and serial-link robustness) to the tool that establishes it, the kind of evidence (proof, test, argument or review), the Evidence_Item, the Phase, and the assumptions and Trust_Base_Register entries it depends on. *Evidence: `doc-verification-plan`.*
2. THE Build_System SHALL produce every Evidence_Item named in this document through one entry point (`cargo xtask verify`), store each with its tool versions, input hashes and verdict, and fail when an Evidence_Item of a completed Phase fails or is missing. *Evidence: `check-evidence`.*
3. THE Verification_Plan SHALL label testing evidence as such and record why no proof applies wherever a property has only testing evidence (as IronOxide R43.4 does). *Evidence: `doc-verification-plan`.*
4. THE Verification_Plan SHALL record, for each new measurement this project takes, the claim of this document it supports or the design change it informs, and THE Build_System SHALL refuse to store a measurement Evidence_Item that cites neither. *Evidence: `check-evidence`.*
5. THE Verification_Plan SHALL cite the IronOxide evidence this project relies on (ASM-05) by IronOxide Evidence_Item name and revision, without re-deriving it. *Evidence: `doc-verification-plan`, `check-registers`.*

### Requirement 40: Registers (Phase P1)

**User Story:** As a certification engineer, I want gaps, trusted components, proof assumptions, mutations and traceability kept in checked registers, so that I can see what the evidence does not cover.

#### Acceptance Criteria

1. THE Gap_Register SHALL list each open gap with its status, owner and plan, at least: the unqualified Front_End; HAMR's and the Backend's unverified code, covered by translation validation only; the trusted Semantic_Model_Extractor; measured WCETs; the Analyzer's unproven soundness against the Kernel's schedules; the deviations of Table M-2; the Pi_Side's unanalyzed timing (NG-04); and the proposals of `docs/ironoxide_proposals.md`. *Evidence: `check-registers`.*
2. THE Trust_Base_Register SHALL list each trusted component with what is trusted, why, the evidence that reduces the trust and the residual risk, at least: HAMR at PAR-02 (Front_End, AIR, symbol resolution, Slang type checking of GUMBO), Sireum, the JVM (OpenJDK 25.0.4.1), OSATE or FMIDE, the SysML v2 parser (`hamr-sysml-parser` 4.20260911.6320233), IronOxide at PAR-01 with its own Trust_Base_Register, Verus with Z3, Creusot with Why3 and its solvers, the Rust toolchains, QEMU, the HIL_Rig, the Pi_Stand_In, and the JSON readers of R29.6. *Evidence: `check-registers`.*
3. THE Proof_Assumption_Register SHALL list every assumption construct in this project's proofs (Verus `assume`, `admit`, `external_body` and `assume_specification`; Creusot `trusted`) with its justification and evidence, and THE Build_System SHALL fail when a proof artifact contains a construct the register does not list (as IronOxide R50.2 does). *Evidence: `check-proof-assumptions`.*
4. THE Traceability_Matrix SHALL be generated from this document, design.md, tasks.md, the Verification_Plan, the sources and the evidence store, linking each acceptance criterion to design elements, tasks and Evidence_Items, and SHALL list every criterion that has no Evidence_Item. *Evidence: `check-trace`.*
5. IF an identifier cited by a register or by the Traceability_Matrix (a requirement, MAP, DEV, ASM, ORQ, IOP, PAR or Evidence_Item) does not resolve, THEN THE Build_System SHALL fail naming the identifier and the register. *Evidence: `check-registers`.*

### Requirement 41: Phase checkpoints and changes to criteria (Phase P1)

**User Story:** As the user, I want each Phase to end in a checkpoint that runs everything and asks me questions, and no threshold or obligation weakened without my approval, so that progress is never bought by lowering the bar.

#### Acceptance Criteria

1. WHEN a Phase ends, THE Build_System SHALL run every test, proof, mutation suite and HIL suite of that Phase and of the earlier Phases, and THE Verification_Plan SHALL record the checkpoint's results and the open questions put to the user before the next Phase starts. *Evidence: `doc-checkpoint-p<n>`.*
2. THE Verification_Plan SHALL keep a change log of this document's requirements, Tables M-1 and M-2 and the Parameters table, each entry naming the old and new text or value, the rationale, the affected Evidence_Items and the user's approval. *Evidence: `doc-verification-plan`.*
3. IF a test threshold, a parameter or an acceptance criterion differs from the last version that the change log records as approved by the user, THEN THE Build_System SHALL fail the verification job naming the difference. *Evidence: `check-criteria-approved`.*

### Requirement 42: Pins, reproducibility and the external repositories (Phase P1)

**User Story:** As a verification engineer, I want the HAMR and IronOxide revisions pinned and the build reproducible inside the workspace, so that evidence produced once applies to the next build.

#### Acceptance Criteria

1. THE Build_System SHALL check at the start of every verification run that IronOxide is at PAR-01 and HAMR at PAR-02, and SHALL fail on a mismatch. *Evidence: `check-pins`.*
2. IF the IronOxide working tree differs from PAR-01, THEN THE Build_System SHALL fail naming the changed paths (NG-01). *Evidence: `check-pins`.*
3. THE Build_System SHALL keep every tool installation and cache it creates inside this workspace (Sireum's JDK and caches, Cargo homes and target directories), and SHALL report any tool it would need outside the workspace before using it. *Evidence: `check-workspace-local`.*
4. WHEN a generated Application is rebuilt on a clean checkout from the same model, Backend version and pinned toolchains, THE Build_System SHALL produce identical generated files, Task_Model and Flight_Build binary hash (IronOxide R59.1). *Evidence: `check-rebuild-<model>`.*

## Open Research Questions

Each question must be resolved, or deferred with a recorded rationale, in design.md or the Verification_Plan.

| ID | Question | Affects |
|---|---|---|
| ORQ-01 | How the Backend plugs into HAMR: HAMR's platforms are a closed enumeration in its CLI and `CodeGen.scala` (`docs/hamr_study.md` section 8), and whether an external jar can add plugins to the stock CLI is unconfirmed. Candidates: a separate tool that calls HAMR's Front_End and symbol table as libraries, unmodified; or a HAMR change adding a platform, proposed upstream. | R1, R18, R42 |
| ORQ-02 | The Backend's implementation language: Slang or Scala on HAMR's symbol table and GUMBO AST (reuses HAMR's resolution and typing, BSD-2-Clause), or Rust over AIR JSON (one language with IronOxide, but re-implements resolution). | R1, R22, R28.2 |
| ORQ-03 | Whether the AIR produced by OSATE and by the SysML front end carries data-access connections and `Concurrency_Control_Protocol` for shared data, which HAMR's back ends do not support. | R9 |
| ORQ-04 | The direction of AADL `Priority` values in HAMR models (ASM-02), and whether a property to invert it is needed. | R7 |
| ORQ-05 | The Slang semantics of GUMBO operations on bounded types (overflow, division, conversions), given that HAMR's Verus translation truncates and its GUMBOX Rust panics (finding F-1). | R22.3, R23.3 |
| ORQ-06 | The formula for C_sample and for the DropOldest acceptance check, including release jitter J_rel, the start delay of DEV-01 and Sporadic sources; and its tightness. | R11.6, R12.2 |
| ORQ-07 | Whether freezing inputs at Job start (DEV-01) is acceptable for every connection, or whether some connections need freezing at the nominal release instant (for example a glue buffer indexed by Release instants). | R19.2, R23.4 |
| ORQ-08 | Multiplexing several connections between the same pair of Tasks onto one Protocol-governed Endpoint, to stay within 8 Endpoints, and its effect on the Validation_Functions and the Chains. | R11, R17.3 |
| ORQ-09 | Which existing HAMR back end generates the Pi_Side: Linux (Slang transpiled to C), JVM, or ROS 2; and how its serial-port driver is supplied (HAMR's `Platform_Provided` provenance). | R34.2, R36.7 |
| ORQ-10 | The case-study model language: SysML v2 runs headless today; AADL needs OSATE or FMIDE, which can be installed inside the workspace (`$SIREUM_HOME/bin/install/fmide.cmd`) with the user's approval. | R1, R36 |
| ORQ-11 | Peripheral ownership on the robot's MCU: the GPIO port P0 and GPIOTE are single peripherals with one owner (PR-29) and one interrupt, and the Application holds at most PAR-04 Tasks; whether the design fits, or needs IOP-02 or IOP-04. | R15, R36.8 |
| ORQ-12 | The Trace_Comparator's comparison relation, given that HAMR's JVM simulation runs a static schedule (ASM-08) and IronOxide runs preemptive SRP with time stamps. | R32.3 |
| ORQ-13 | How to capture port-level traces from HAMR's JVM simulation (the generated `src/main/inspector` points at the HAMR Inspector; unconfirmed). | R32.2 |
| ORQ-14 | Whether GUMBO `flows`, `monitor` and `compositions` clauses can be carried later, for example compositions as system-level Connection_Obligations. | R22.5 |
| ORQ-15 | Whether HAMR's JVM back end supports GUMBO spec functions with developer bodies at a later revision (finding F-2), which limits the differential corpus. | R32.6 |
| ORQ-16 | Whether constant initial values (DEV-03) are enough for the case study, or whether some Initialize entry point code needs a first-Job initialization pattern. | R20 |
| ORQ-17 | Whether `IronOxide::Criticality_Level` should also accept the ARINC 653 property set's criticality property as an alias. | R2 |
| ORQ-18 | How the Semantic_Model_Extractor itself gains evidence beyond testing: a second independent AIR reader, a Verus-verified reader, or review. | R28, R40.1 |
| ORQ-19 | Whether IronOxide's Analyzer can be given the Serial_Bridge's receive path as a Chain start (an interrupt event) with the frame time of PAR-12 and PAR-13 added, for cross-processor latency budgets. | R26.4, R35.7 |
| ORQ-20 | The Pi_Stand_In on the HIL_Rig: the J-Link virtual COM port, a USB-UART adapter, or the stimulus board over the spare board-to-board pins (ASM-07). | R37.1 |
| ORQ-21 | The license of `hamr.aadl.library` in sysml-aadl-libraries (no license file found); the models reference it from its clone until it is known. | R2.1, R36.1 |

## Research Notes and Sources

Facts below were checked on 2026-11-09. Content was rephrased for compliance with licensing restrictions.

- **RN-01** HAMR's informal semantics: the supported component categories (system, process, thread group, thread, data as types), unsupported ones (subprograms, shared data blocks, devices as such), unidirectional typed ports, no fan-in on data ports, Periodic and Sporadic dispatch only, Initialize and Compute entry points only, and the input-compute-output model with infrastructure and application port states, inputs frozen at dispatch and outputs released at completion. [HAMR modeling elements and semantics](https://hamr.sireum.org/hamr-doc/hamr-semantics-concepts/)
- **RN-02** HAMR documentation of GUMBO contracts and their translation to Verus in Rust application code. [GUMBO component contracts](https://hamr.sireum.org/hamr-doc/gumbo-component-contracts/), [GUMBO to Verus translation](https://hamr.sireum.org/hamr-doc/gumbo-verus-translation/), [HAMR Rust implementation](https://hamr.sireum.org/hamr-doc/hamr-rust-implementation/), [HAMR documentation index](https://hamr.sireum.org/hamr-doc/)
- **RN-03** HAMR source at PAR-02: platform dispatch (`hamr/codegen/jvm/src/main/scala/org/sireum/hamr/codegen/CodeGen.scala`), plugin interfaces (`common/plugin/Plugin.scala`, `microkit/plugins/MicrokitPlugin.scala`), linters (`common/symbols/Linter.scala`, `microkit/plugins/linters/MicrokitLinterPlugin.scala`), the domain schedule (`microkit/plugins/c/components/CComponentPlugin_DomainScheduler.scala`), the GUMBO-to-Rust translation (`microkit/plugins/gumbo/GumboRustPlugin.scala`), the GUMBO AST (`hamr/air/shared/src/main/scala/org/sireum/hamr/ir/GumboAST.scala`). [sireum/kekinian](https://github.com/sireum/kekinian)
- **RN-04** IronOxide at PAR-01: `.kiro/specs/rsk-ravenscar-profile/{requirements,design,tasks}.md`, `docs/profile.md`, `docs/verification_plan.md`, the registers in `docs/`, `apps/p1-demo/decl/src/lib.rs`, and the crates `rsk-gen`, `rsk-model`, `rsk-analyzer`, `rsk-protocol`, `rsk-lint`, `rsk-link`, `rsk`. [mars2292-umn/kiro-experimentation IronOxide](https://github.com/mars2292-umn/kiro-experimentation)
- **RN-05** INSPECTA models used in the study: `micro-examples/microkit/gumbo-verus/structs_arrays` and `isolette`. [loonwerks/INSPECTA-models](https://github.com/loonwerks/INSPECTA-models)
- **RN-06** The SysML v2 AADL and HAMR libraries that the SysML front end needs; the AADL property defaults of ASM-03 are taken from their comments where present. [santoslab/sysml-aadl-libraries](https://github.com/santoslab/sysml-aadl-libraries)
- **RN-07** The study record of this project: versions, licenses, commands and outcomes, the HAMR architecture, findings F-1 to F-4, and the confirmation of IronOxide's claims. `docs/hamr_study.md`
- **RN-08** The IronOxide change proposals IOP-01 to IOP-04. `docs/ironoxide_proposals.md`
