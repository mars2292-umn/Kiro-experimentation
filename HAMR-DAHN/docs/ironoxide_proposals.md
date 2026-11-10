# Proposed IronOxide changes (IOP-nn)

Status: drafts of 2026-11-09, not submitted. IronOxide (`../IronOxide`, revision 423a366) is an external dependency of this project and is not modified here. Each proposal states the requirement text it would add to or change in IronOxide's `requirements.md`, the rationale, and the IronOxide proofs, checks and evidence it would affect. For each one, this project's requirements define a design that works without it; the proposal records what the change would buy. Identifiers in the form `IronOxide Rn.m`, `PR-nn`, `DD-nn` point into IronOxide's spec; `Rn.m`, `MAP-nn`, `ORQ-nn` without prefix point into this project's `.kiro/specs/hamr-ironoxide-backend/requirements.md`.

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
