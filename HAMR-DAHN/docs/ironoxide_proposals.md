# Proposed IronOxide changes (IOP-nn)

Status: drafts, not submitted (IOP-01 to IOP-04 with Draft 0.1 of the requirements and IOP-05 to IOP-16 with Draft 0.2, both of 2026-11-09; IOP-17 to IOP-22 with Draft 0.1 of this project's design.md; IOP-13 is optional). IronOxide (`../IronOxide`, revision 423a366) is an external dependency of this project and is not modified here. Each proposal states the requirement text it would add to or change in IronOxide's `requirements.md`, the rationale, and the IronOxide proofs, checks and evidence it would affect. For each one, this project's requirements define a design that works without it; the proposal records what the change would buy. Identifiers in the form `IronOxide Rn.m`, `PR-nn`, `DD-nn` point into IronOxide's spec; `Rn.m`, `MAP-nn`, `DEV-nn`, `ORQ-nn`, `UD-nn` and the table row identifiers (such as `TH-02`, `CR-02`) without prefix point into this project's `.kiro/specs/hamr-ironoxide-backend/requirements.md`. Identifiers written `design.md DD-nn`, `design.md C-nn` and `design.md S-nn`, and quoted section titles named as design.md's, point into this project's `.kiro/specs/hamr-ironoxide-backend/design.md`.

## IOP-01: Sampling Endpoints (latest-value channels)

**Problem.** AADL data ports have latest-value semantics: the receiver reads the most recent value, a new value replaces an unread one, and nothing is queued. IronOxide Endpoints are FIFO queues; a send that finds the channel full returns `SendError::Full` and the Kernel records an event against the sending Partition (IronOxide R40.5). Across Partitions there is no other channel (PR-30, PR-32).

**Workaround used without the change (MAP-16, Requirement 12).** A plain Endpoint whose capacity covers every send that can occur between two drains of the receiver while both Tasks meet their deadlines; the receiver drains the channel each Job (`ops` equal to the capacity) and keeps the newest value in per-Task state. Cost: Kernel storage of 4 × capacity × payload bytes per connection, `ops` × max(Δ_send, Δ_recv) of overhead per receiver Job (IronOxide R40.2), and a recorded event, not a silent overwrite, if the timing assumptions fail.

**Proposed requirement text (new IronOxide R40.7).** "WHERE an Endpoint is declared `sampling`, THE Kernel SHALL hold at most one message per directed channel, a send SHALL replace the held message without recording an event, a receive SHALL return the held message and leave it in place together with the instant of its send, and THE Generator SHALL reject a `sampling` Endpoint that has a Protocol, an arrival-released role, or a capacity other than 1."

**Rationale.** Removes the capacity formula and its timing assumption from the data-port mapping, cuts Kernel storage to one slot per direction, and makes data-port semantics exact: a receiver always reads the newest value sent before its read.

**Affected IronOxide evidence.**
- `crates/rsk-logic/src/logic/endpoint.rs` and the Kernel_Proofs of IronOxide R39.4 and R44.2: a second refinement (a one-place register, not a FIFO) and its invariant.
- `crates/rsk-kernel/src/arch/cm4/entry.rs` send and receive services: the overwrite path, and the A-P2 label-sequence link (`send_labels_ok`), which assumes FIFO order.
- IronOxide R40.5 (no event for an overwrite), R10.8 (sampling Endpoints cannot release Tasks), R41 (excluded from Protocols).
- Generator (`syntax.rs`, `validate.rs`), Task_Model schema (`endpoints[].kind`), Config_Checker recomputation (IronOxide R26.2), Analyzer terms (IronOxide R40.2: one operation per Job), the mutation suites (`docs/proof_mutations.toml`, `docs/config_mutations.toml`), and a conformance case.

## IOP-02: More dispatch vectors on the nRF52840 (more than 7 Tasks)

**Problem.** Each Task needs one dispatch vector (IronOxide DD-01). The nRF52840 Target lists seven free lines (`crates/rsk-gen/src/target.rs`, `dispatch_irqs: [20, 21, 22, 23, 24, 25, 40]`), so an Application has at most 7 Tasks. A two-processor robot's MCU side can need more (Requirement 36, ORQ-11).

**Workaround used without the change.** The back end refuses a model with more than 7 threads on the IronOxide processor (MAP-26) and the case-study model is designed within 7 threads.

**Proposed requirement text (amends IronOxide R22).** "THE Generator SHALL assign dispatch vectors from the EGU/SWI lines and from the interrupt lines of Target peripherals that no Partition and no Kernel item owns, in strictly ascending line order, and SHALL reject an App_Declaration only when the free lines are fewer than its Tasks."

**Rationale.** The Hardware_Model's dispatch lemmas assume only that vectors ascend in Task order (`hw_model::dispatch_vectors_ok`); unused peripheral lines satisfy that if the list is built per Application.

**Affected IronOxide evidence.** `target.rs` (a per-Application list instead of a constant), `hw_model::dispatch_vectors_ok` and the DD-03 tie-order lemma (`lemma_dispatch_selects_srp_choice`), boot validation (IronOxide R12.1, R19.5: lines bound to no Task stay disabled), the Config_Checker's vector table check, the HIL differential test for NVIC arbitration (`hil-dt-hw-*`), and the mutation `dispatch-vector-order`.

## IOP-03: End-to-end chains across same-Partition hops

**Problem.** IronOxide chains (IronOxide R41.4) require every hop to be an Endpoint. Same-Partition AADL connections map to Resources and Release_Signals (MAP-11 to MAP-14), which a chain cannot cross.

**Workaround used without the change (MAP-21).** A same-Partition connection that lies on an end-to-end flow with a `Latency` property maps to an Endpoint instead, at the cost of one of the 8 Endpoints (IronOxide `MAX_ENDPOINTS`).

**Proposed requirement text (amends IronOxide R41.4).** "WHERE a chain hop connects two Tasks of one Partition through a Resource that the receiving Periodic_Task reads once per Job, or through a Release_Signal that releases the receiving Sporadic_Task under `defer`, THE Analyzer SHALL bound that hop by the receiving Task's period, or by its MIT, plus its WCRT, as for an Endpoint hop."

**Rationale.** The latency terms are the ones the Analyzer already uses for Endpoint hops (`crates/rsk-analyzer/src/lib.rs`, the chain loop); only the hop's identification changes.

**Affected IronOxide evidence.** `validate.rs` chain rules, the Task_Model `chains[].hops[].endpoint` field (a hop kind), the Analyzer's chain bound and report, the Config_Checker if it recomputes chains, and the chain test cases of task 21.5.

## IOP-04 (candidate): nRF52840 Target table additions for the robot

**Status.** Candidate only; needed if the case-study design (Requirement 36) chooses quadrature decoding in hardware or a second UART. Not needed for the Phase P5 rig run with the GPIOTE-based design.

**Problem.** The nRF52840 Target table lists one UART (UARTE0) and no QDEC (`target.rs`). Wheel encoders then need GPIOTE interrupts with MIT enforcement, and the GPIO port P0 and GPIOTE are single peripherals, each owned by one Partition (PR-29), which forces the encoder, motor-direction and e-stop pins into one Partition (ORQ-11).

**Proposed requirement text (amends the Target description behind IronOxide R21.5).** "THE Target description of the nRF52840 SHALL list QDEC and UARTE1 with their register windows, interrupt lines and, for UARTE1, the EasyDMA register descriptions that DD-08 needs."

**Affected IronOxide evidence.** `target.rs` and the Config_Checker's own table (`crates/rsk-confcheck/src/image.rs`), the MPU access-matrix HIL case (IronOxide R16.8), the EasyDMA refusal cases (IronOxide R17.7) for UARTE1, and the Profile_Conformance_Suite accepted Application if it uses them.

## IOP-05: Reject out-of-range integers in the App_Declaration parser

**Problem.** The App_Declaration parser reads integers as `u64` and narrows four of them with `as u32` (`crates/rsk-gen/src/syntax.rs`: `log_capacity` at line 308, an Endpoint's `capacity` and `payload` at lines 415 and 416, a Partition's `mit_threshold` at line 486). A declared value of 2^32 + 5 therefore reaches `validate.rs` as 5 and is accepted when 5 lies in the checked range. A Task's `priority` and `core` are clamped with `.min(255)` (lines 658 and 659), so `validate.rs` checks 255 instead of the declared value; a clamped Priority is still rejected under PR-33, but the diagnostic does not show the declared value.

**Workaround used without the change (R2.4, R17.1, R17.4).** The Backend checks every value it writes before the IronOxide_Generator runs: R2.4 refuses Table 2-1 values outside their ranges (`Log_Capacity` at most 2^30 − 1, `MIT_Violation_Threshold` at most 2^32 − 1), and R17.1 computes every count, size and sum as an exact integer without reducing it modulo 2^32. R17.4 counts a Task_Model integer that differs from the declared one as a Backend defect, so a value that IronOxide narrowed cannot pass unnoticed.

**Proposed requirement text (new criterion of IronOxide Requirement 21).** "IF an integer attribute of an App_Declaration lies outside the range of the field that records it (`log_capacity`, `mit_threshold`, and an Endpoint's `capacity` and `payload`: 0 to 2^32 − 1; a Task's `priority` and `core`: 0 to 255), THEN THE Generator SHALL reject the declaration with a diagnostic that names the attribute, the declared value and the range, and SHALL NOT truncate or clamp the value."

**Rationale.** A declaration then means exactly what it states. The check sits in the tool whose output IronOxide's own evidence covers, and the Backend's checks become a second line instead of the only one.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/syntax.rs` (the four `as u32` reads and the two clamps).
- The Generator's property tests (`crates/rsk-gen/tests/properties.rs`, with the declaration generators of `src/arbitrary.rs`): new out-of-range cases for each attribute.
- The error output of the `rsk::app!` macro (`crates/rsk-macros`), which delegates to the same parser (`rsk_gen::syntax::parse_app`).

## IOP-06: Accepted MIT-window behaviour and `discard` on interrupt sources

**Problem.** IronOxide R19.4 requires the Generator to require each interrupt-bound Task to declare which MIT-window behaviour (lost, coalesced or deferred) it accepts, and to reject a declaration that the Design_Document does not provide for that source; IronOxide R21.1 lists "accepted MIT-window behaviour for interrupt Release_Sources (R19.4)" among the items an App_Declaration declares. At PAR-01, neither the grammar (`syntax.rs`) nor `decl.rs` has such a field (`Release::Sporadic` holds `mit`, `policy` and `source` only). IronOxide design.md resolves its ORQ-23 under DD-06 as "discard is not offered for interrupts", but `validate.rs` matches `Release::Sporadic { mit, policy: _, source }` (line 306) and ignores the policy of an interrupt-released Task, so `mit_policy = discard` on an interrupt source is accepted.

**Workaround used without the change (R15.6, MAP-20 (h), DEV-09).** The Backend refuses `IronOxide::MIT_Policy => Discard` on an interrupt-released thread itself (R15.6, MAP-20 (h)); that check is the only enforcement of the case. Table M-2 row DEV-09 records that interrupt events are coalesced while the line is masked and deferred to the end of the MIT window (IronOxide DD-06, R19.4). Because the IronOxide_Generator does not reject the case, `pbt-backend-checks-match-generator` cannot cover R15.6.

**Proposed requirement text.** For the first part no new text is needed: IronOxide R19.4 and R21.1 already require it, and the change implements them. New criterion of IronOxide Requirement 19: "IF a Task released by an interrupt declares `mit_policy = discard`, THEN THE Generator SHALL reject the declaration, naming the Task, its interrupt Release_Source and DD-06." And for R19.4: "THE Generator SHALL record in the Task_Model the accepted MIT-window behaviour that each interrupt-bound Task declares."

**Rationale.** The Generator then enforces what IronOxide's own design states, the Task_Model records the behaviour that the Analyzer's and the Backend's arguments assume, and the Backend's check of R15.6 can be compared with the Generator's.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/syntax.rs` and `decl.rs` (a new Task attribute for interrupt sources), `validate.rs` (the match at line 306), and the Task_Model schema (`crates/rsk-model/schemas/task_model.json`, the task object).
- The P1 demo's declaration (`apps/p1-demo/decl/src/lib.rs`), whose interrupt-released `pulse` Task would declare the behaviour.
- The Generator's property tests and the conformance cases for R19.4.

## IOP-07: Bind `rsk-entry` to an Application outside IronOxide's repository

**Problem.** `rsk-entry` binds an Application's system crate by Cargo feature and path dependency inside IronOxide's tree: `crates/rsk-entry/Cargo.toml` has `default = ["app-p1-demo"]`, `rsk-system = { path = "../../apps/p1-demo/system", … }` and `hil-stimulus-system = { path = "../../apps/hil-stimulus/system", … }`, and `src/lib.rs` selects `use rsk_system::Sys` or `use hil_stimulus_system::Sys` by feature. IronOxide's `cargo xtask` commands `gen`, `qemu` and `verify` work on IronOxide's own root only; only `check --root <dir>` takes another root (`xtask/src/main.rs`). No Application outside IronOxide's tree can therefore be linked without editing IronOxide's files (ORQ-37).

**Workaround used without the change (UD-02, R18.9, ORQ-37).** The Build_System builds each generated Application against a workspace-local extraction of `IronOxide/` at PAR-01 (`git archive`). In the copy it changes only the workspace membership and `rsk-entry`'s binding, the Trust_Base_Register lists each change, and `check-pins` shows that `../IronOxide` is unchanged (R42.2).

**Proposed requirement text (new criterion of the IronOxide Build_System requirements).** "THE Build_System SHALL bind `rsk-entry` to the system crate of an Application workspace outside the IronOxide repository without a change to any Kernel crate's source or manifest, and its `gen`, `qemu` and `verify` commands SHALL accept the root of such a workspace."

**Rationale.** Generated Applications would build against IronOxide's tree as it is pinned, with no edited copy in the trust base, and the Config_Checker's comparison of Task_Model and binary would cover them unchanged.

**Affected IronOxide evidence.**
- `crates/rsk-entry/Cargo.toml` and `src/lib.rs` (the `app-*` features and the `use … ::Sys` lines).
- The Link_Checker's DD-11 template check (`crates/rsk-link/src/scan.rs`, `--system-crate`).
- The workspace policy `[workspace.metadata.rsk]` of IronOxide's root `Cargo.toml` and the steps of `xtask/src/verify.rs`, `gen.rs` and `qemu.rs`.

## IOP-08: A flash load image for each Partition's `.data`

**Problem.** The generated linker script places each Partition crate's `.data` together with its `.bss` in the NOLOAD section `.p{i}.bss` (`crates/rsk-gen/src/emit.rs`, around line 781; `apps/p1-demo/gen/link.x`). `rsk-entry`'s `Reset` zeroes the ranges of `__rsk_bss_table`, which include those sections, and copies only the ranges of `__rsk_data_table`, which holds the Kernel's `.data` alone. A non-zero `INIT` value of a Partition's `resource` or `state` item may therefore read as zero at instant 0 and after RESTART_PARTITION. This was read from source, not observed in a run; the P1 demo uses only zero `INIT`s (ORQ-40).

**Workaround used without the change (R20.5, ORQ-40).** The Port_Glue writes initial values in the Task's first Job after instant 0 and after each restart of its Partition, unprivileged and inside the Job's Budget (R20.5, DEV-08), because IronOxide runs no Application code before instant 0. The `const fn` initial-value functions that a destination's Port_Glue calls must then lie in shared code (IronOxide R16.3). `test-init-first-job` and `test-init-restart` check the result.

**Proposed requirement text (amends IronOxide R14.10 and the reset entry of IronOxide R12.4).** "THE Generator SHALL place each Partition crate's initialized data in a section with a load image in flash, and THE Kernel SHALL copy that image into the Partition's RAM at reset and when it re-initializes the Partition under RESTART_PARTITION, before any Job of the Partition runs."

**Rationale.** `INIT` values then hold at instant 0 and after every restart as the App_Declaration states them, with no run-time work in Jobs and no Budget cost; the first-Job pattern of R20.5 and its tests become unnecessary.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/emit.rs` (the Partition sections and the tables `__rsk_bss_table` and `__rsk_data_table`) and `layout.rs` (flash space for the load images).
- `crates/rsk-entry/src/lib.rs` `Reset` (the copy loop and `MAX_RANGES`) and the Kernel's restart path with its recorded bound (IronOxide R14.10), which the Analyzer counts as interference (IronOxide R29.3).
- The Config_Checker's image checks (`crates/rsk-confcheck/src/image.rs`), the boot checks, and the HIL restart cases.

## IOP-09: A predicate for `rsk::Validate` and a delivery postcondition for `Endpoint::recv`

**Problem.** `rsk::Validate<T>` has only `fn accept(m: &T) -> bool` (`crates/rsk/src/lib.rs`, line 254), with no logic predicate, and `Endpoint::recv` and `Endpoint::recv_from` are trusted for the App_Prover (`creusot_std::macros::trusted`) with no postcondition (lines 324 to 360). IronOxide R42.4 delivers only messages that the role's validation function accepts, but the App_Prover cannot use that fact: it cannot prove that a value received through an Endpoint satisfies the receiving port's integration assume clauses.

**Workaround used without the change (R24.5, Table 24-1, R19.7).** For every in port whose values arrive through an Endpoint, the Proof_Assumption_Register holds a `[[step]]` entry for the Get Value postcondition. For an Upward connection the entry names the Validation_Function and its proofs (R13.1, R13.2, R13.4, R13.7), the initial-value obligation of R24.3, and IronOxide R42.4; for other connections, the Connection_Obligation, the source's Put Value precondition, IronOxide's FIFO refinement (IronOxide R39.4, R42.2) and the Wire_Type round trip (R14.3).

**Proposed requirement text (amends IronOxide R42.4).** "THE `rsk` crate SHALL give `Validate<T>` a logic predicate `valid` with the App_Prover contract that `accept(m)` returns `valid(m)` (for `Accept`, `valid` is true), and THE App_Prover contract of `Endpoint::recv` and `Endpoint::recv_from` SHALL state that a returned `Ok(Some(m))` satisfies `V::valid(m)`."

**Rationale.** The Upward entries of R24.5 become proofs: the App_Prover can derive the receiving port's assume clauses from the Validation_Function's proven contract and the delivery postcondition, leaving only the trusted contract of `recv`, which IronOxide already reviews.

**Affected IronOxide evidence.**
- `crates/rsk/src/lib.rs` (the `Validate` trait, `Accept`, and the contracts of `recv` and `recv_from`) and the validator binding that `crates/rsk-gen/src/emit.rs` generates.
- IronOxide TBR-08, whose residual risk names the trusted API contracts reviewed against the Kernel's Verus postconditions: the review would cover the new postcondition against IronOxide R42.4 and the Kernel's delivery proofs (IronOxide R39.4).
- The App_Prover proofs of every IronOxide Partition that implements `Validate` (each must prove that `accept` returns `valid`), and new entries in `docs/proof_mutations.toml` for the contract.

## IOP-10: Measure and run an external Application's HIL build

**Problem.** IronOxide's measurement and rig code serves IronOxide's own Applications only (ORQ-47). At PAR-01, `crates/rsk-wcet/src/main.rs` prints that it is not implemented (tasks 19.1 to 19.3), `crates/rsk-wcet-agent` holds no code, and the WCET_Records come from `rsk-hil`'s Budget accounting (`crates/rsk-hil/src/wcet_records.rs`) on builds that `cargo xtask hil` makes from IronOxide's root. `rsk_hil::release_timing::run` needs the demo's `RSK-OK` console verdict and runs demo-specific steps (the pulse loopback and the EasyDMA counters), and its trace freeze takes one expected Task (`crates/rsk-hil/src/events.rs`, `expect_task`). `hil-rig.toml` describes the demo's wiring. Kernel_Timing_Parameters are per Application: `docs/hil/kernel_timing_nrf52840_*.json` carry the hashes of IronOxide's demo builds, so the Analyzer would reject them for a generated Application's binary (IronOxide R28.3).

**Workaround used without the change (R25.1, R25.5, R25.7, R37; UD-02).** The Build_System runs IronOxide's HIL code inside the workspace-local extraction of UD-02, with the generated Application bound in place of the demo (R18.9) and its own rig description (ORQ-63), as design.md states for ORQ-47. Until that path works, every Analyzer run uses placeholder records and is labelled development evidence (R25.5), so `analysis-rta-<model>-measured` cannot pass (R25.7), and the rig criteria of R37 depend on the same path.

**Proposed requirement text (amends IronOxide R34 and the HIL_Rig requirements).** "THE WCET_Harness and THE HIL_Rig SHALL provide an entry point that takes the root of an Application workspace outside the IronOxide repository, its Task_Model and a rig description, builds and flashes that Application's HIL build, writes WCET_Records (`rsk-wcet/1`) and Kernel_Timing_Parameters (`rsk-kernel-timing/1`) for that build's binary hash, and runs that Application's release-timing and stress runs with the expected Tasks, event kinds and verdict that the caller states."

**Rationale.** Timing evidence for generated Applications would come from IronOxide's own measurement method, unchanged and cited by name (R39.5), instead of from an edited copy.

**Affected IronOxide evidence.**
- `crates/rsk-wcet`, `crates/rsk-wcet-agent` and `crates/rsk-hil` (`wcet_records.rs`, `kernel_timing.rs`, `release_timing.rs`, `events.rs`).
- `xtask/src/hil.rs` and `main.rs` (the `hil` command and the release-timing and stress steps), and the rig file format `rsk-hil-rig/1`.
- IronOxide's Verification_Plan sections 11.12 to 11.14 and the R34.7 instrumentation argument, which would cover external Applications.

## IOP-11: Chain hops that name their Endpoint

**Problem.** A chain names Tasks only. The Generator takes as each hop the first declared Endpoint whose roles bind both Tasks, in either direction (`crates/rsk-gen/src/derive.rs`, the chain loop near line 473; `validate.rs` checks the chain with the same search near line 658). With a reverse or parallel connection between the two Tasks, the hop can be the wrong Endpoint: IronOxide then rejects the chain under IronOxide R41.4, or accepts it and records the wrong hop in the Task_Model.

**Workaround used without the change (R26.3 (c), R26.6, MAP-21 (c)).** R26.6 fixes the declaration order of Endpoints so that each hop's Endpoint is declared first, and R26.3 (c) refuses, as a Backend restriction, a flow with two hops that join the same two threads, in either direction, over different connections.

**Proposed requirement text (amends IronOxide R41.4).** "THE App_Declaration grammar SHALL let each chain hop name its Endpoint (`P.T -[E]-> P.T`), and THE Generator SHALL use the named Endpoint as that hop and reject a hop whose Endpoint does not bind both Tasks; WHERE a hop names no Endpoint, THE Generator SHALL use the Endpoint whose role releases the receiving Task, or the only Endpoint that binds both Tasks, and SHALL reject the chain when neither exists."

**Rationale.** The hop's identity becomes explicit in the declaration and the Task_Model, R26.3 (c) and R26.6 are no longer needed, and the Analyzer bounds the hop over the connection that the flow actually uses.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/syntax.rs` (chain syntax), `validate.rs` (the chain rules of IronOxide R41.4) and `derive.rs` (hop selection).
- The Task_Model field `chains[].hops` (`crates/rsk-model/schemas/task_model.json`), the Analyzer's chain bound and report (`crates/rsk-analyzer`), and the Config_Checker if it recomputes chains.
- The chain test cases (IronOxide task 21.5).

## IOP-12: Structured Analyzer report and traceable diagnostics

**Problem.** Several IronOxide outputs cannot be traced to a Task_Model item without parsing message text (ORQ-25):
- The Analyzer's report (`rsk-analysis-report/1`) gives each rejection as a `"[rule] message"` string, and gives the utilization-above-1 case (IronOxide R29.9), the overflow case (IronOxide R29.6) and the items behind `b_source` and the interference terms only as text.
- `prepare` (`crates/rsk-analyzer/src/inputs.rs`, line 332) returns at the first Task entry without a WCET_Record, so the Analyzer names only the first missing item and reports REJECTED with exit status 2, while IronOxide R28.2 says "return FAIL and list the missing items"; a WCET above its Budget (IronOxide R28.4, "return FAIL and name the Task") is handled the same way.
- A Link_Checker violation (`crates/rsk-link/src/lib.rs`, `Violation`) holds only a rule and a message; a Config_Checker discrepancy (`crates/rsk-confcheck/src/lib.rs`, `Discrepancy`) holds an item, the expected and the found value, and no rule.

**Workaround used without the change (R3.4, R3.8, R25.2, R27.6).** R3.4 traces each IronOxide diagnostic by the item or location it names, and R3.8 reports one that cannot be traced unchanged, marked as untraced. R25.2 compares every Task's measured WCET with its model bound itself, not only the first Task that the Analyzer names. R27.6 gives a thread "no verdict" with the cause when the Analyzer gives no result for its Task, and never shows the 0 or 2^64 − 1 cycles that the report records in those cases.

**Proposed requirement text (amends IronOxide R30 and the Link_Checker and Config_Checker requirements; the listing of missing items implements IronOxide R28.2 and R28.4 as they stand).** "THE Analyzer's report SHALL give for each Task a status (a WCRT; a lower bound, with the rule under which no WCRT was computed; or not computed, with the rule) and, for each blocking and interference term, the Task_Model item from which the term comes; THE Analyzer SHALL list every missing WCET_Record and every Task whose WCET exceeds its Budget and return FAIL, as IronOxide R28.2 and R28.4 state; THE Link_Checker SHALL name in each violation the symbol, function or section concerned; and THE Config_Checker SHALL name in each discrepancy the rule that it violates." Optional: cryptographic digests of the inputs in place of the FNV-1a 64 fingerprints that `crates/rsk-analyzer/src/main.rs` records.

**Rationale.** The Backend's back-mapping (R3.4) and the Verdict_Report (Requirement 27) would read structured fields instead of parsing messages, which removes a trusted step, and the Analyzer would agree with IronOxide's own criteria R28.2 and R28.4.

**Affected IronOxide evidence.**
- `crates/rsk-analyzer/src/inputs.rs`, `report.rs` and the schema `crates/rsk-model/schemas/analysis_report.json` (a new schema version), with the Analyzer's tests against its reference implementation.
- `crates/rsk-link` (`Violation`) and `crates/rsk-confcheck` (`Discrepancy`), and the mutation runner `xtask/src/confcheck_mutations.rs`, which matches `error[R26.3]` in the Config_Checker's report.

## IOP-13 (optional): Per-Task state and Resource initial values in the Task_Model

**Status.** Optional. R29.7 records the gap without it, and no criterion depends on it.

**Problem.** The Task_Model (`rsk-task-model/1`, `crates/rsk-model/schemas/task_model.json`) records for a Resource only `id`, `core`, `partition`, `ceiling`, `accessors`, `type` and derived values, and for a Task no per-Task state declarations. The TV_Checker, which reads the Task_Model, therefore cannot check the initial values of R9.6, R20.1 and R20.2 or the Resource invariants of R9.7.

**Workaround used without the change (R29.7).** The Verification_Plan lists per-Task state, Resource initial values and invariants among the items that neither the TV_Checker nor the Config_Checker covers. They are shown instead by the App_Prover's initial-value and invariant proofs (R9.7, R20.4) and by the tests of Requirement 20.

**Proposed requirement text (amends the Task_Model schema of IronOxide R26).** "THE Generator SHALL record in the Task_Model, for each Task, its per-Task state declarations with their types and initial values, and for each Resource its initial value and the name of its invariant predicate, each initial value as the bytes of its `INIT` constant."

**Rationale.** The TV_Checker could then relate the initial values and invariants to the Semantic_Model, and the Config_Checker could compare the recorded initial bytes with the binary.

**Affected IronOxide evidence.** The Task_Model schema and `crates/rsk-model`, `crates/rsk-gen/src/derive.rs` and `json.rs`, the Config_Checker's recomputation (IronOxide R26.2) if it compares the new fields, and `docs/config_mutations.toml` for the new fields.

## IOP-14: A Kernel Job-trace ring for test builds

**Problem.** At PAR-01, a HIL build (`trace-ram`) keeps per-Task aggregates in `RELEASE_STATS` (among them the Jobs completed, the largest response time and the largest execution; `crates/rsk-kernel/src/arch/cm4/unsafe_module/mod.rs`), the Budget accounting and the event mirror, but no record per Job of its Release, start and completion instants. Semihosting needs a debugger and halts the core on each write. The differential tests on the HIL_Rig (R32.2) and Chain latencies on the rig (R37.2) therefore have no IronOxide source of per-Job records (ORQ-53).

**Workaround used without the change (R32.1, R37.2, UD-24, ORQ-53).** Under QEMU the Trace_Bodies write their Job records through `rsk::debug`; on the HIL_Rig they write them as design.md records for ORQ-53 (candidates: buffers in per-Task state read through the probe after the run, and the trace pins for Job order), with Budgets that cover the cost (DEV-08). UD-24 requires per-Job records on the rig only where the Task_Model declares a Chain.

**Proposed requirement text (new criterion of IronOxide's HIL_Rig requirements).** "WHERE a build enables a Job-trace feature that no Flight_Build enables, THE Kernel SHALL record for each Job its Task, Release instant, start instant and completion instant in a fixed-capacity ring with a saturating overflow count, which the HIL_Rig reader and the QEMU runner read at the end of the run."

**Rationale.** Exact Release instants also for Sporadic Tasks (narrowing Table 32-1 rows CR-02 and CR-03), no trace code in Application Tasks, and no Budget cost in Task code.

**Affected IronOxide evidence.**
- `crates/rsk-kernel` (a new feature next to `trace-ram`, and the ring in the Kernel_Unsafe_Module), with the Kernel_Timing_Parameters measured on builds that enable it (an instrumentation difference under IronOxide R34.7).
- The HIL reader (`crates/rsk-hil`) and the QEMU runner (`xtask/src/qemu.rs`).
- The checks that keep HIL-only features out of a Flight_Build, and IronOxide's Verification_Plan section 11.

## IOP-15: Byte count of a UARTE receive in progress

**Problem.** A UARTE EasyDMA receive ends only when its buffer is full or on STOPRX (nRF52840 behaviour as the req-35 notes state it; not checked here against Nordic's product specification). PPI is Kernel-owned and configured at Init (IronOxide DD-08); IronOxide design.md states that declaring PPI routes is not offered, that PPI stays unused, and that every PPI channel is cleared at boot. The windows of EasyDMA-capable peripherals are read-only to their Partition (DD-08). A Partition therefore cannot learn how many bytes of a receive in progress have arrived, and a frame shorter than the buffer stays in it until more bytes arrive (ORQ-58).

**Workaround used without the change (R35.7, R35.10, ORQ-58).** design.md answers ORQ-58 with one of two methods: the Pi_Side fills the line with frame delimiters so that buffers fill at the line rate (frame latency then depends on the untrusted peer), or the transmitting Task triggers STOPRX periodically to flush a partial buffer; the receive buffer sizes and the MIT follow from PAR-12, PAR-13 and the receiving Task's WCRT (R35.10).

**Proposed requirement text (amends IronOxide DD-08 and the EasyDMA service of IronOxide R17).** "WHERE a Partition owns a UARTE, THE Kernel SHALL provide a service that returns the number of bytes that the Partition's receive in progress has written to its buffer, or SHALL configure at Init, from a route that the App_Declaration declares, a PPI channel from the UARTE's RXDRDY event to the COUNT task of a TIMER that the same Partition owns in counter mode."

**Rationale.** Frames would be delimited from the actual byte count, independent of what the untrusted peer sends after a frame and without stopping receives.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/target.rs` (the UARTE0 DMA description and the Kernel-owned PPI entry) and DD-08's PPI rule with the boot validation that clears every PPI channel.
- The EasyDMA register-write service (`rsk::dma_write`, task 19.2) or a new Kernel service with its Verus specification, and the proofs about Kernel-owned peripherals (IronOxide R16.2).
- The HIL builds' use of PPI channel 15 for the Δ_stamp probe (IronOxide design.md), the MPU access-matrix HIL case (IronOxide R16.8), and the Config_Checker's peripheral table (`crates/rsk-confcheck/src/image.rs`).

## IOP-16: PPI routes from a Partition's GPIOTE events to its own TIMER counter

**Problem.** Counting wheel-encoder edges with one GPIOTE interrupt per edge needs an MIT no longer than the shortest edge interval at PAR-19 (R36.11): as an illustration, about 170 µs for 1,200 edges per turn of a 65 mm wheel at 1.0 m/s, which the Analyzer may reject (ORQ-61). The Kernel's masking during the MIT window also coalesces edges (DEV-09). On the nRF52840 Target, TIMER1 and TIMER2 can be owned by a Partition (`crates/rsk-gen/src/target.rs`, `kernel_owned: false`), and a TIMER in counter mode counts events on its COUNT task, but only PPI can connect a GPIOTE event to that task, and PPI is Kernel-owned with no declarable routes (DD-08).

**Workaround used without the change (R36.11, ORQ-61, UD-20).** The Robot_Model gives the encoder thread (TH-02) an MIT no longer than the shortest edge interval at PAR-19 (R36.11), which limits the encoder resolution. Other candidates are low-resolution wheel encoders and QDEC under IOP-04.

**Proposed requirement text (amends IronOxide DD-08 and R21.1).** "WHERE an App_Declaration declares a counter route from a GPIOTE event of a Partition to a TIMER that the same Partition owns, THE Kernel SHALL configure at Init one PPI channel from that event to the TIMER's COUNT task, and THE Generator SHALL reject a route whose GPIOTE or TIMER the Partition does not own, a route over a PPI channel that the Kernel reserves, and more routes than free PPI channels."

**Rationale.** Edges are counted in hardware, without an interrupt per edge, so the encoder Task can be Periodic and read the count, with no MIT constraint on the encoder resolution and no new peripheral in the Target table (unlike IOP-04).

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/syntax.rs`, `decl.rs` and `validate.rs` (the route declaration and its checks), the Task_Model schema, and `emit.rs` (Generated_Config for the routes).
- DD-08's PPI rule, the boot validation that clears every PPI channel, the proofs about Kernel-owned peripherals (IronOxide R16.2), and the HIL builds' use of PPI channel 15 for the Δ_stamp probe.
- The Config_Checker's peripheral table (`crates/rsk-confcheck/src/image.rs`) and the MPU access-matrix HIL case (IronOxide R16.8).

## IOP-17: `creusot-std` as a vendored registry dependency of `rsk`

**Problem.** `crates/rsk/Cargo.toml` declares the optional App_Prover dependency as `creusot-std = { git = "file:///Users/mars2292/repos/Kiro-experimentation/creusot", rev = "18f2d371ecb0019d37ecbd25e67f474bed2dda00", optional = true }` (line 15 at PAR-01), and IronOxide's `.cargo/config.toml` replaces that git source with `vendor/`. Cargo records optional dependencies in the lock file, so every workspace that depends on `rsk` by path must copy the machine-local URL into a source replacement of its own. The URL names a checkout on one machine, and the copy ties every dependent workspace to that string.

**Workaround used without the change (R18.1, R18.4, R42.4).** The Backend writes the URL, as a constant tied to PAR-01, into the `.cargo/config.toml` of each generated workspace and of its proof workspace `proofs/creusot/`, with `replace-with = "vendored-sources"` (design.md "Build_System, pins and the IronOxide extraction", design.md DD-89). The constant is not a path that the Backend resolves, so the output does not depend on where the workspace lies. That the generated workspace resolves offline with it is checked by design.md S-03.

**Proposed requirement text (amends the vendored-source rule of IronOxide R58.1).** "THE `rsk` crate SHALL declare its optional `creusot-std` dependency as a registry dependency with an exact version, resolved from the vendored sources like every other third-party crate, so that no manifest and no Cargo configuration of IronOxide or of a workspace that depends on `rsk` names a git URL or a path outside its own repository."

**Rationale.** Dependents then need only the `crates-io` replacement that every vendored build has, and the lock files of IronOxide and of its dependents name no machine-local source. Whether the vendored `creusot-std` 0.15.0-dev can be listed as a registry package without a published release is for IronOxide to check.

**Affected IronOxide evidence.**
- `crates/rsk/Cargo.toml`, `.cargo/config.toml`, the root `Cargo.lock` and `proofs/creusot/Cargo.lock`, and `vendor/creusot-std` with its checksum file.
- The `[creusot]` pin of `toolchain/decision.toml` and the checks that compare it, and the `[source]` check of `cargo xtask verify`.

## IOP-18: RESTART_PARTITION empties every Endpoint channel into the restarted Partition

**Problem.** IronOxide's design.md at PAR-01 says, in the RESTART_PARTITION row of "Closed response sets", that the response "resets its Endpoint roles, whose peers see a peer-state error (R42.3)", and its note on task 21.4 says that a Partition's stop or restart empties both channels of each of its Endpoints. No IronOxide criterion states the emptying: IronOxide R42.3 requires only the peer-state error, and IronOxide R14.10 the re-initialization of Endpoint roles.

**Workaround used without the change (R20.6, R11.11).** R20.6 requires that no event or message sent before a restart is presented after it. For an Endpoint connection (MAP-15) the Port_Glue cannot tell an old message from a new one, so the design relies on the behaviour that IronOxide's design.md describes, and `test-init-restart` observes it (design.md "Initialization", "Port_Glue Job State Machine"). A negative result becomes a conflict with R20.6.

**Proposed requirement text (amends IronOxide R14.10 and R42.3).** "WHEN THE Kernel restarts or stops a Partition, THE Kernel SHALL empty, before any Job of any Partition next runs, both directed channels of every Endpoint that has a role in that Partition, so that no message sent before the restart or stop is received after it, AND THE HIL_Rig SHALL show it with a message queued toward the Partition at the instant of its restart."

**Rationale.** The behaviour that the implementation already has becomes a criterion with its own evidence, and R20.6 then rests on an IronOxide criterion instead of a design note.

**Affected IronOxide evidence.**
- The Kernel's restart and stop path (`ArchState::peer_changed` and the DMA-stop path common to both responses) and the Verus specification of the Endpoint channels (`crates/rsk-logic/src/logic/endpoint.rs`).
- The HIL restart cases and the Kernel_Proofs of IronOxide R39.4 and R42.3.

## IOP-19: An END_JOB inside a Critical_Section

**Problem.** When a Health_Monitor action ends a Job inside a Critical_Section, the Kernel releases the Resource with whatever the closure has written (IronOxide R8.9; the transition `InCS --> Idle` of IronOxide's design.md at PAR-01). IronOxide R46.4 proves Resource invariants only after initialization and at the exits of Critical_Sections, and the App_Prover's model of `Shared` (`crates/rsk/src/lib.rs`) assumes the invariant at every entry. A later Critical_Section can therefore find a value that violates the invariant, and a partly copied value need not be a value of its type. The same holds for a Job ended during a multi-word write of per-Task state.

**Workaround used without the change (R21.5, R24.4, DEV-08).** design.md's Port_Glue operations are total over every bit pattern of their cells (design.md DD-71), so a later Job stays free of panics and in bounds; the possible invariant violation is a residual that the Gap_Register records (design.md "Port_Glue Job State Machine", "What an ended Job leaves behind").

**Proposed requirement text (amends IronOxide R8.9).** "IF a Health_Monitor action ends a Job while the Job holds a Resource, THEN THE Kernel SHALL either escalate the action to RESTART_PARTITION of the Job's Partition, which re-initializes the Resource, or defer the end of the Job to the exit of its outermost Critical_Section, bounded by the Critical_Section's WCET; and until IronOxide adopts one of them, THE Trust_Base_Register SHALL list the case."

**Rationale.** Every Resource then satisfies its invariant whenever a Job enters a Critical_Section on it, which the App_Prover's model of `Shared` already assumes, and the proofs of R46.4 cover every reachable state.

**Affected IronOxide evidence.**
- IronOxide R8.9 and R14.5, the Kernel's Job-end path and its Verus proofs (the lock stack and the System_Ceiling after a forced end).
- The Analyzer's terms if the end is deferred (the Critical_Section's WCET added to the response bound), IronOxide's Trust_Base_Register, and the HIL fault cases that end a Job inside a Critical_Section.

## IOP-20: Per-Task state with a declared invariant

**Problem.** An App_Declaration declares per-Task state as `state <task>.<name>: T = INIT;` (IronOxide design.md at PAR-01), with no invariant, and the App_Prover views a `PerTask<T>` as its current value with no constraint. A property of a GUMBO state variable that one Job's postcondition establishes cannot be assumed at the next Job's entry, because nothing ties the two Jobs in the proof.

**Workaround used without the change (R22.2, R22.7, R40.3).** For each GUMBO state variable, the type invariant of the pre-state at the compute function's call is taken as a `[[step]]` of the Proof_Assumption_Register, discharged by the previous Job's `ensures` and, for the first Job and after a restart, by the initial-value proof (design.md "Contract_Translator and the App_Prover", the call site of the compute function).

**Proposed requirement text (amends IronOxide R21.1 and the App_Prover contracts of `PerTask`).** "THE App_Declaration grammar SHALL let a per-Task state item declare an invariant (`state t.v: T = INIT invariant |v: T| …;`), THE App_Prover SHALL prove it for `INIT` and at every write of the item, AND THE App_Prover contract of `PerTask::get` and `PerTask::with` SHALL state it of the value they give."

**Rationale.** The App_Prover could then prove a GUMBO state variable's type invariant at a Job's entry from the previous Job's exit, and these `[[step]]` entries would disappear.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/syntax.rs`, `decl.rs` and `emit.rs` (the grammar and the emitted invariant), and the Task_Model schema if it records invariants (see IOP-13).
- The contracts of `PerTask` in `crates/rsk/src/lib.rs`, their review under IronOxide TBR-08, and `docs/proof_mutations.toml`.

## IOP-21: `dma_request` on a double-buffered channel during a transfer

**Problem.** Some nRF52840 EasyDMA peripherals have double-buffered pointer registers: UARTE `RXD.PTR` and `RXD.MAXCNT` can take the next buffer while a reception is in progress, and PWM has `SEQ[0]` and `SEQ[1]`. Whether the Kernel at PAR-01 accepts an `rsk::dma_request` on a channel while a transfer over another buffer of that channel is in progress is not confirmed; IronOxide R17.5 forbids Job access to a buffer in transfer, which a Kernel could enforce by refusing any second request.

**Workaround used without the change (R35.7, R35.10, ORQ-58).** design.md's receive protocol for UARTE0 re-arms the second of two receive buffers with `rsk::dma_request` while EasyDMA fills the first, using the `ENDRX_STARTRX` shortcut (design.md DD-103, "Serial_Bridge and Frame_Codec"). design.md S-12 checks on the rig whether the Kernel accepts it. If it does not, R35.10's receive path needs IOP-15.

**Proposed requirement text (amends IronOxide R17 and DD-08).** "WHERE a Target's DMA register description marks a channel as double-buffered, THE Kernel SHALL accept a DMA request on that channel while a transfer over another buffer of the same channel is in progress, provided the new buffer lies in memory that R17.1 permits and overlaps no buffer of a transfer in progress, and SHALL refuse it otherwise."

**Rationale.** Continuous reception and continuous PWM sequences then need no gap between buffers, and the guarantee of R17.5 still holds, because no Job touches a buffer in transfer.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/target.rs` (a double-buffer mark in the DMA register descriptions of UARTE0 and PWM0) and the Kernel's DMA request service with its Verus specification.
- The EasyDMA HIL cases of IronOxide R17.7 and the Config_Checker's peripheral table (`crates/rsk-confcheck/src/image.rs`).

## IOP-22: Protect the trace pins from the Partition that owns `P0`

**Problem.** The `nrf52840` table gives `P0` a 4 KiB window at 0x5000_0000 (`crates/rsk-gen/src/target.rs` line 114 at PAR-01). IronOxide's Kernel drives its trace pins P1.11 to P1.14 through port P1, whose base it takes as 0x5000_0300 (`crates/rsk-kernel/src/arch/cm4/unsafe_module/hw.rs`, `trace_pins`, line 1248), inside that window; design.md S-12 confirms the address against the nRF52840 memory map. A Partition that owns `P0` can then write P1's output registers and disturb the trace pins that the HIL_Rig reads for the executing Task and the Kernel-active flag (RM-03). In this project's Robot_Model the safety Partition owns `P0` (UD-20).

**Workaround used without the change (R37.1, RM-03).** The safety Partition's code writes only `OUTSET`, `OUTCLR` and the `PIN_CNF` words of its own pins, never `OUT` or `DIR` as a whole (design.md "Robot_Model, the MCU side on the HIL_Rig, and robot integration"). Only this coding rule protects the trace pins; the MPU view does not.

**Proposed requirement text (amends the `nrf52840` Target description behind IronOxide R21.5 and DD-07).** "THE Target description of the nRF52840 SHALL give no Partition write access to the registers of port P1 that drive the Kernel's trace pins, either by separating the windows of `P0` and `P1` or by keeping the trace pins on a port that no Partition window covers, AND THE Config_Checker SHALL check it for every build that enables `trace-pins`."

**Rationale.** The trace pins are evidence of the Kernel's timing (IronOxide's per-run check of the Budget accounting against the trace), so no Partition should be able to change them.

**Affected IronOxide evidence.**
- `crates/rsk-gen/src/target.rs` and `layout.rs` (the window of `P0`, or a window for `P1`), and the trace-pin code of `crates/rsk-kernel`.
- The MPU access-matrix HIL case (IronOxide R16.8), the Config_Checker's peripheral table (`crates/rsk-confcheck/src/image.rs`), and `hil-rig.toml`'s trace channels.
