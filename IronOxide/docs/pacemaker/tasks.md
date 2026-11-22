# Implementation Plan

**PACEMAKER on rsk: an experimental Application**

Status: Draft 0.1; the first wave (tasks 1.1, 2.1, 3.1, 6.1, and 4.1 in part) done 2026-11-22. Plans `requirements.md` Draft 0.4 and `design.md` Draft 0.2 of this folder.

## Overview

The plan has six phases. Phase 0 retires the hardware risk and prepares what every later phase needs: the spike that confirms the register behaviour the design assumes, the audit of the Task limit, the two rsk changes of Requirement 33, and the rig with the Heart_Simulator. Phases E1 to E5 are the stages of the requirements: single-chamber pacing (E1), dual-chamber and triggered Modes with the proofs of the therapy logic (E2), the link, the States and the fault responses (E3), rate adaptation, hysteresis, smoothing, ATR and noise (E4), and history and diagnostics (E5).

Every leaf task names its evidence. Proofs come first wherever a tool applies; tests are labelled as testing evidence; a timing claim is a rig measurement with the run named (PM31.3). "Property Pn" refers to the Correctness Properties in `design.md`, PD-nn to its decisions, PD-On to its open points. `PMn.m` is an acceptance criterion of `requirements.md`. Each phase ends with a checkpoint.

A task is checked when its evidence exists. A note `Done <date>:` under the task then says where the code and the evidence are, and `Open:` what was left.

## Tasks

### Phase 0 — Spike, audit, rsk changes, rig

- [ ] 1. Hardware spike (PD-O1)
  - [x] 1.1 Build the spike in its own Cargo workspace outside rsk's, as the DD-01 spike was: one privileged program on board A that runs TIMER3 at 1 MHz, connects compares 0, 1, 4 and 5 to GPIOTE set and clear tasks by PPI channels it writes itself, and delivers pulses of 0.05 ms, 0.4 ms and 1.9 ms on both pace pins.
    - Evidence: logic-analyzer record of each width against PM-PAR-05
    - Done 2026-11-22: `spikes/pm-timer` (own workspace; `measure.py` judges a capture); `docs/pacemaker/spike/report.md` and `task_1_1_widths.json`: 5,527 pulses of 0.05, 0.4, 0.5, 1.0 and 1.9 ms on both pace pins, each within one analyzer sample (0.25 µs) of its width. Bench finding: a bare-metal program must write APPROTECT.DISABLE at its start, or the board's debug port locks at the next reset (it did once; board A was recovered by the Verification_Plan's procedure).
    - _Requirements: PM28.1, PM28.2_
  - [ ] 1.2 Confirm each register behaviour the design relies on, one probe per behaviour: TIMER3 has six compare registers and one interrupt line; a compare written behind the counter does not fire before the wrap; an event flag is set when its interrupt is not enabled; a GPIOTE task channel holds the pin whatever the port's direction register says; a PPI fork triggers a second task from one event; a timer in counter mode counts fork edges and raises its interrupt at a compare that is moved while it counts, with the counter never cleared.
    - Evidence: spike report with one row per behaviour (confirmed or refuted, with the capture or the register read that shows it)
    - _Requirements: PM27.2, PM29.1, PM33.1_
  - [ ] 1.3 Run the compare write order of "Arming and the lead" on the spike: arm, move and withdraw with the instant swept toward the present in steps of the timer's tick, with an interrupt of the Kernel's measured hold injected between the steps of the sequence.
    - Evidence: rig record of the smallest `LEAD` that never missed, the counts of `TooLate` and late arms, and no pin high longer than its width in any sweep
    - _Requirements: PM27.6, PM28.4_
  - [ ] 1.4 Record the decision: the design stands, or the sections that change. Update `design.md` (status line, PD-O1, the value of `LEAD`).
    - Evidence: decision record in `design.md`
    - _Requirements: PM31.2_

- [x] 2. Audit of the Task limit
  - [x] 2.1 Carry out the read-only audit of PM32.2: which interrupt lines of the nRF52840 could serve as further dispatch vectors, and what in rsk depends on the number seven. No rsk file changes.
    - Evidence: audit report in this folder
    - Done 2026-11-22: `task_limit_audit.md`. Lines 43, 44 and 46 have no interrupt source and lie above the last vector in use; no fixed size of the Kernel is reached at eight Tasks; the dispatch-deferral record (8 slots) must grow from the ninth. The interrupt table is from the chip's register description (`nrf52840.svd`).
    - _Requirements: PM32.1, PM32.2, PM32.7_

- [ ] 3. rsk: fixed PPI channels and TIMER3 (Requirement 33)
  - [x] 3.1 Add fixed PPI channels to the App_Declaration and the Task_Model: an event of a peripheral, a task of a peripheral, an optional fork task.
    - Evidence: Generator unit tests; a declaration fixture with channels; the Task_Model schema test
    - Done 2026-11-22: `ppi NAME (event = PERIPHERAL[OFFSET], task = PERIPHERAL[OFFSET], fork = PERIPHERAL[OFFSET]);` at the top level of `rsk::app!` (rsk-gen `syntax.rs`, `decl.rs`); the channel number is the position; checks of name, count (the Target's `ppi_channels`: 20 on the nRF52840, 0 on QEMU), peripheral, alignment and the task and event offset ranges (`validate.rs`); each register resolved to its address (`derive.rs`); the Task_Model's optional `ppi` list (`model.rs`, rsk-model `task_model.rs` and schema), absent when no channel is declared, so every committed Task_Model is unchanged (`cargo xtask gen --check` passes). Tests: five in rsk-gen (`pm33_*`), one in rsk-model (`ppi_channels_parse_and_meet_the_schema`). Registers are given by offset, not by name: the Target table has no register names.
    - Open, by design until tasks 3.2 and 3.3: the ownership rule of PM33.2 is not checked and the Kernel does not write the channels, so `rsk_gen::outputs` refuses to emit anything for a declaration with a channel (`PPI_KERNEL_SUPPORT`, test `pm33_3_no_output_for_channels_the_kernel_does_not_write_yet`).
    - _Requirements: PM33.1_
  - [ ] 3.2 Generator: reject a channel whose peripherals are not all owned by one Partition; write the channel table into Generated_Config under the configuration checksum; extend the verified acceptance check.
    - Evidence: Verus on `rsk-gen`; negative fixtures (a channel across two Partitions, a Kernel-owned peripheral, a channel number used twice) that the Generator rejects
    - _Requirements: PM33.2, PM33.4_
  - [ ] 3.3 Kernel: write the declared channels and their enable bits at Init and never afterwards.
    - Evidence: the contract of the Init write, proven; a harness for the raw write; a scan showing no other write to the PPI's window in the binary
    - _Requirements: PM33.3, PM33.5_
  - [ ] 3.4 Config_Checker: recompute the channel table from the Task_Model and compare it with the binary's. Add a configuration mutation that changes one channel.
    - Evidence: Config_Checker tests; the mutation is detected
    - _Requirements: PM33.4_
  - [ ] 3.5 Target table of the `nrf52840`: TIMER3 as a peripheral a Partition may own, with its register window and interrupt line.
    - Evidence: Generator and Config_Checker tests on a declaration that binds TIMER3
    - _Requirements: PM33.6_
  - [ ] 3.6 Rig: read back the PPI registers after Init; add the isolation rows (an unprivileged write to the PPI faults; a Partition without TIMER3 cannot reach it).
    - Evidence: HIL read-back record; isolation matrix rows
    - _Requirements: PM33.5_
  - [ ] 3.7 Bring rsk's own documents in line (requirements, design, evidence rows, mutation register) and confirm with one run of the verification job.
    - Evidence: the verification job's result on the changed rsk; `cargo xtask reqdoc --check`
    - _Requirements: PM33.5, PM33.7_

- [ ] 4. Rig and Heart_Simulator
  - [ ] 4.1 Wire the rig for the Application and record it: two sense lines from B to A, two pace lines from A to B and to the analyzer, the magnet pin, the bridge on A's SPIM1 pins. Measure every connection as the existing wiring was measured.
    - Evidence: `hil-rig.toml` entries with the measurement that confirms each line
    - Done in part 2026-11-22 (`hil-rig.toml` `[wiring]`, keys `pacemaker_*`; `spike/report.md`): the two pace pins to the analyzer (A P1.10 to D0, A P1.11 to D1) by the spike's pulses; the two sense lines (P1.01, P1.02) and the two remaining board-to-board lines (P1.03, P1.04) by driving each from one board's debug probe and reading the other board, both directions. No wire was changed.
    - Open: the pace pulses do not reach board B on the pins the analyzer sees. Proposal recorded in the rig file: a fork of each pace channel drives P1.03 and P1.04 (task 1.2's fork probe decides). The bridge on SPIM1 is not wired (needed from E3).
    - Done 2026-11-22: the magnet wire, B P1.07 to A P1.07, added and measured in both directions through the debug probes (`hil-rig.toml`, `pacemaker_magnet`).
    - _Requirements: PM29.5, PM26.1_
  - [ ] 4.2 Build the Heart_Simulator (`apps/heart-sim`), an rsk Application for board B: a scenario table per Chamber (intrinsic interval, conduction delay, per-beat overrides), a 1 ms Periodic_Task that plays it, the sense pulses mirrored to the analyzer in the same register write, and the responsive scenarios that restart a Chamber's timer on a Pace.
    - Evidence: rsk's build checks accept it; analyzer record of a fixed scenario against its table
    - _Requirements: PM2.3, PM26.3_
  - [ ] 4.3 Add the Application's judgement to the rig: decode Paces and Senses from the capture, measure every pace interval, AV delay, refractory period and pulse width, judge each against its tolerance, and write a report that carries the binary's identity and the scenario.
    - Evidence: judge unit tests on recorded and on synthetic captures, including captures that must fail
    - _Requirements: PM24.3, PM31.3_

- [ ] 5. Checkpoint: Phase 0 complete. The spike's decision is recorded, rsk with the two changes passes its checks, and the rig plays and judges a scenario. Ask the user if questions arise.

### Phase E1 — Single-chamber pacing

- [ ] 6. `pm-core`: types, validation and the single-chamber step
  - [x] 6.1 Create `crates/pm-core` (`no_std`, no rsk, no hardware): instants as 32-bit microseconds compared by signed difference, `Chamber`, a Mode as its four properties, `Params`, `State`, `Event`, `Arming`.
    - Evidence: host unit tests of the instant arithmetic across the wrap
    - Done 2026-11-22: `crates/pm-core` (`time.rs`, `mode.rs`, `params.rs`, `state.rs`, `event.rs`), a workspace member built for the host and the Target; 19 host tests, six of them on `Micros` at bases on both sides of the wrap and of the sign change. The all-zero `State` is the Power-On Reset State, with no `Option` in it (a flag beside each value), checked field by field. `Params::NOMINAL`, `POWER_ON_RESET` and `PACE_NOW` are constants; `interval_of_ppm` rounds down.
    - _Requirements: PM25.1, PM26.5_
  - [ ] 6.2 `validate` for the parameters of the E1 Modes: membership in Table 19-1, the nominal values, and the conversion of each rate to its interval, rounded so that the rate is not below the programmed one.
    - Evidence: exhaustive host test per parameter (Property P5 for the E1 rows)
    - _Requirements: PM19.1, PM19.2, PM19.3, PM26.5_
  - [ ] 6.3 `step` for AOO, VOO, AAI and VVI: the escape interval from the LRL, VRP and ARP, inhibition, Refractory senses, the pending instant in the state, and the accounting of a pending Pace when a later event arrives first.
    - Evidence: host tests against hand-worked timing diagrams per Mode; property-based tests of Properties P2, P3 and P9
    - _Requirements: PM4.1, PM4.2, PM4.4, PM5.3, PM5.5, PM5.7, PM9.1, PM9.2, PM9.3, PM12.1, PM12.2, PM27.3_

- [ ] 7. The hardware layer `Hw`
  - [ ] 7.1 Define the trait and a register model of TIMER3, the counters and GPIOTE for the host. Implement `arm`, `disarm`, `fired` and `sensed` over the model with the write order of "Arming and the lead" and the service of "Sense intake".
    - Evidence: host tests of Properties P6 and P7 that let the model's counter advance, and a compare fire, between any two register accesses of the sequence
    - _Requirements: PM27.2, PM28.2, PM28.4_
  - [ ] 7.2 Implement `Hw` for the nRF52840 through `rsk::mmio`, including `configure` (pins, GPIOTE channels, the three timers, the clock offset with its bounded retry).
    - Evidence: Target build under `#![forbid(unsafe_code)]`; the spike's sweeps of task 1.3 repeated through this implementation
    - _Requirements: PM27.2, PM28.1, PM28.3, PM29.1_
  - [ ] 7.3 Implement `Hw` for QEMU: the Partition-ownable timer as the pacing compare and a table of Senses.
    - Evidence: QEMU run delivering a scripted sequence
    - _Requirements: PM26.2_

- [ ] 8. The Application's skeleton
  - [ ] 8.1 Create `apps/pacemaker` after the demo's layout, with the App_Declaration for both Targets: the Partition `therapy` at level A with its five peripherals, the Tasks `pace`, `sense_a`, `sense_v` and `control` of Table 2, the Resource `timing`, the six PPI channels, each Release_Source's MIT and policy, the three responses, and the operating duration. This settles PD-O2: the Generator's layout accepts five peripheral windows.
    - Evidence: the Generator accepts both variants; `summary.txt` reviewed against Table 1 and Table 2
    - _Requirements: PM21.1, PM21.6, PM22.1, PM22.2, PM22.6, PM23.1, PM23.3, PM26.4, PM27.1, PM27.4, PM27.5, PM28.3, PM32.1_
  - [ ] 8.2 Write the four Task bodies as the design gives them: read the hardware, call `step`, apply the arming. Until the link exists the parameter set is a table of the build, selected per rig scenario.
    - Evidence: QEMU run; the Profile_Lint accepts the Partition crate
    - _Requirements: PM2.1, PM2.2, PM3.1, PM22.4, PM27.2_
  - [ ] 8.3 Make the zeroed statics the Power-On Reset State: no initial value other than zero anywhere in `therapy`, and the first Job of `control` recognizable by the state alone.
    - Evidence: a test that builds the state from zeroed bytes and checks it is POR; a rig start from reset
    - _Requirements: PM30.4, PM7.1_
  - [ ] 8.4 Pass rsk's build checks on the Flight build: Profile_Lint, Link_Checker (stack bounds, stubs), Config_Checker.
    - Evidence: the three checkers' reports for both variants
    - _Requirements: PM21.2, PM21.5_

- [ ] 9. Verification of E1
  - [ ] 9.1 Add the QEMU variant to the verification job: the Application with scripted Senses in each E1 Mode.
    - Evidence: job step with the expected event sequence per Mode
    - _Requirements: PM26.2_
  - [ ] 9.2 Measure the Application's own Kernel_Timing_Parameters and WCET_Records on the rig and run the Analyzer on them. This settles PD-O3 for E1: the sporadic Tasks' 2 ms deadlines and the proposals of Table 2 are confirmed or replaced.
    - Evidence: measured records carrying the binary's hash; the Analyzer's PASS; Table 2 rewritten with measured values
    - _Requirements: PM21.3, PM21.4, PM24.4_
  - [ ] 9.3 State each timing chain of "Timing Design" with its measured bound against its tolerance.
    - Evidence: the chain table in the Application's evidence document, each bound from a named record
    - _Requirements: PM24.1, PM24.2, PM27.6_
  - [ ] 9.4 Rig scenarios for E1, each at least PAR-05 long: asynchronous pacing at the lowest, nominal and highest LRL; inhibition in AAI and VVI with intrinsic rates above and below the LRL; Senses inside and outside the refractory period; every pulse width of Table 19-1 including 0.05 ms.
    - Evidence: rig reports with every interval, refractory period and width judged (Property P12)
    - _Requirements: PM3.3, PM4.4, PM9.1, PM12.6, PM24.3, PM28.2_
  - [ ] 9.5 Measure the inhibition window by sweeping a Sense toward a pending Pace, and report the counts of fusions and late arms over every E1 run.
    - Evidence: rig record of the window against PM-PAR-15
    - _Requirements: PM5.5, PM24.3_
  - [ ] 9.6 App_Prover: no panic, overflow or out-of-bounds access in the four Task bodies and in `pm-core` as they stand (Property P10).
    - Evidence: Creusot run on the `therapy` member
    - _Requirements: PM25.2_
  - [ ] 9.7 Start the two standing documents: the traceability table from every acceptance criterion to design, code and evidence, and the findings (where the source could not be met, and what was awkward on rsk).
    - Evidence: the table generated or checked by a script; the findings file
    - _Requirements: PM31.1, PM31.2, PM31.3_

- [ ] 10. Checkpoint: E1 complete. The PG paces and inhibits in AOO, VOO, AAI and VVI within tolerance on the rig, the Analyzer reports PASS on measured records, and the build checks pass. Ask the user if questions arise.

### Phase E2 — Dual-chamber and triggered Modes, proof of the therapy logic

- [ ] 11. `pm-core`: the dual-chamber timing cycle
  - [ ] 11.1 The ventricular-based cycle for DOO, DDI, VDD and DDD: both pending instants, the paced and the sensed AV delay, the Sensed AV Delay Offset, atrial Senses during an AV delay as Refractory senses.
    - Evidence: host tests against hand-worked diagrams per Mode
    - _Requirements: PM5.6, PM5.7, PM9.2, PM11.1, PM11.2, PM11.3, PM11.5, PM11.6_
  - [ ] 11.2 The dynamic AV delay from the previous cardiac cycle's length.
    - Evidence: host tests at the limits of the cycle length and of the delay
    - _Requirements: PM11.4_
  - [ ] 11.3 The URL: no ventricular Pace earlier than one URL interval after the last ventricular event, and tracking limited to the URL.
    - Evidence: property-based test of Property P1; a host test of an atrial rate rising through the URL
    - _Requirements: PM10.1, PM10.2_
  - [ ] 11.4 PVARP, the PVC, PVARP Extension limited to one cycle in two, and ventricular blanking after an atrial Pace.
    - Evidence: host tests; property-based test of Property P8
    - _Requirements: PM4.5, PM12.3, PM12.4, PM12.5_
  - [ ] 11.5 The triggered response (AAT, VVT): a Pace one `LEAD` after a non-refractory Sense.
    - Evidence: host tests
    - _Requirements: PM5.4_
  - [ ] 11.6 Complete `validate`: every parameter of the E2 Modes, the interactive limits of Table 19-5, and the allocation of parameters to Modes of Table 19-3 as amended by Table 19-4.
    - Evidence: exhaustive host test per parameter and per limit (Property P5)
    - _Requirements: PM3.2, PM4.3, PM5.1, PM5.8, PM19.1, PM19.3, PM19.5_

- [ ] 12. Contracts on the therapy logic (App_Prover)
  - [ ] 12.1 State and prove on `step`: Property P1 (the URL), Property P3 (a refractory or blanked Sense changes no pending Pace), Property P4 (the State changes only by the State machine's transitions), Property P8 (the PVARP extension).
    - Evidence: Creusot proofs; one proof mutant per contract that is rejected
    - _Requirements: PM25.3_
  - [ ] 12.2 State Property P2 as an invariant of the state and prove that `step` preserves it: in a Mode that paces a Chamber, a Pace of that Chamber is pending no later than the escape interval in force.
    - Evidence: Creusot proof; a mutant of the invariant that is rejected
    - _Requirements: PM25.3, PM25.4, PM9.1_
  - [ ] 12.3 Prove Property P5 on `validate`: it returns a parameter set exactly for the admitted word sequences.
    - Evidence: Creusot proof
    - _Requirements: PM25.2, PM19.3_
  - [ ] 12.4 Prove Properties P6 and P7 on `Hw::arm` and `Hw::disarm` over the register model, with a clock that advances by a bounded amount between two statements (PD-O5). If the route does not work in Creusot, record why and keep the host tests of task 7.1 as the evidence.
    - Evidence: Creusot proof, or the recorded reason with the test evidence named
    - _Requirements: PM25.3, PM28.4_

- [ ] 13. Verification of E2
  - [ ] 13.1 Re-measure the records with the E2 code and re-run the Analyzer; restate the AV-delay chain, the tightest arming.
    - Evidence: refreshed records and PASS; the chain's measured bound against 30 ms
    - _Requirements: PM21.3, PM21.4, PM24.1, PM24.2_
  - [ ] 13.2 Rig scenarios for E2: AV delays fixed and dynamic, paced and sensed; an atrial rate swept through the URL; PVARP and its extension after PVCs; DDI against DDD on the same scenario; triggered Paces with their latency measured.
    - Evidence: rig reports with every AV delay, interval and refractory period judged
    - _Requirements: PM5.4, PM10.1, PM10.2, PM11.1, PM12.6, PM24.3_
  - [ ] 13.3 Confirm the declared `MIT_PACE` on the rig: the shortest time between two pulse ends that the programmable values allow, with no deferred or discarded release of `pace` in any run.
    - Evidence: rig record of the shortest observed spacing and of the Kernel's log
    - _Requirements: PM27.5_

- [ ] 14. Checkpoint: E2 complete. Every Mode without rate adaptation paces within tolerance on the rig, and the contracts of Properties P1 to P5 and P8 are proven. Ask the user if questions arise.

### Phase E3 — The link, the States, the fault responses

- [ ] 15. The link
  - [ ] 15.1 Fix the frame protocol (PD-O6) and put its encoding in `pm-core`: 32-byte frames, sequence number, kind, length, CRC-16, parameter sets in numbered parts.
    - Evidence: host round-trip and corruption tests
    - _Requirements: PM20.1, PM29.4_
  - [ ] 15.2 The bridge firmware for board C and the DCM_Stub (`tools/dcm-stub`), which holds `pm-core`'s validation to predict the PG's answer.
    - Evidence: host tests of the stub; a loop-back run through the bridge
    - _Requirements: PM1.2, PM20.1_
  - [ ] 15.3 The Partition `comms` at level C with the Task `link` on SPIM1: one frame each way per period, assembly of parameter sets, identification by model and serial number.
    - Evidence: rsk's build checks; a rig run of an interrogation
    - _Requirements: PM20.1, PM20.3, PM22.5, PM29.4_
  - [ ] 15.4 Loss of the link after PM-PAR-11 unanswered polls, with its bound measured.
    - Evidence: rig record of the detection time against the stated bound
    - _Requirements: PM20.2, PM6.4_

- [ ] 16. Programming and the States
  - [ ] 16.1 The Endpoints `program` (validated by `therapy`) and `status`, and `control`'s handling of commands.
    - Evidence: rig runs in which the DCM_Stub's prediction and the PG's answer agree for accepted and for refused sets; the Kernel's report against `comms` for a refused one
    - _Requirements: PM19.3, PM22.3, PM1.2_
  - [ ] 16.2 The State machine in `step`: Permanent, Temporary, Pace-Now, Magnet and POR, with transitions taking effect at the next ventricular event (atrial in the atrial Modes).
    - Evidence: host tests of every transition and of every transition that must not happen (Property P4)
    - _Requirements: PM6.1, PM6.2, PM6.3, PM6.4, PM6.7, PM7.4, PM5.2_
  - [ ] 16.3 Pace-Now: its parameters, and the first pulse within the bound from the DCM_Stub's send.
    - Evidence: rig record of the time from the frame to the pulse; the 1.00 ms width judged at ±0.02 ms
    - _Requirements: PM6.5, PM6.6_
  - [ ] 16.4 The magnet: the pin, the Magnet State's rates by battery level, the return to the State before, and the response programmed Off.
    - Evidence: host tests; rig run with the pin toggled
    - _Requirements: PM8.1, PM8.2, PM8.3, PM8.4, PM29.2_
  - [ ] 16.5 The battery level from the link: the Modes offered at ERT and ERP, temporary pacing disabled, and no pacing below the POR trip level.
    - Evidence: host tests; rig run stepping through the levels
    - _Requirements: PM7.2, PM17.6, PM17.7, PM29.2, PM30.3_
  - [ ] 16.6 Reset: the POR parameters, the first Pace within its stated bound, empty histories.
    - Evidence: rig record of the time from reset to the first Pace; the 0.5 ms width judged at ±0.02 ms
    - _Requirements: PM7.1, PM7.3, PM30.1, PM30.2_

- [ ] 17. Fault responses that keep pacing
  - [ ] 17.1 Restart of `therapy`: inject a fault, an Overrun and a deadline miss, and measure the pause in pacing for each. The pin is low throughout except for delivered pulses.
    - Evidence: rig record of the three pauses, and of the longest high time on each pace pin
    - _Requirements: PM23.2, PM23.3, PM23.6, PM28.4_
  - [ ] 17.2 A lower Partition that faults, overruns, floods `program`, or is stopped: pacing continues as programmed, and the peer-state error of an Endpoint to `comms` is treated as the loss of the link.
    - Evidence: rig runs with a misbehaving `link` build, every interval judged (Property P11); the Analyzer's PASS for `therapy` with the lower Budgets enforced
    - _Requirements: PM22.5, PM23.4, PM1.4_
  - [ ] 17.3 The Kernel's safe state: both pace pins low, and the consequence recorded as a finding.
    - Evidence: rig run to the end of a short declared operating duration
    - _Requirements: PM23.5, PM27.4_

- [ ] 18. Checkpoint: E3 complete. The PG is programmed and interrogated over the link, every State is reachable and left as specified, and pacing survives a restart of its own Partition and the failure of the others. Ask the user if questions arise.

### Phase E4 — Rate adaptation, hysteresis, smoothing, ATR, noise

- [ ] 19. Rate behaviour in `pm-core`
  - [ ] 19.1 Rate-adaptive pacing in integer arithmetic: the activity value from the link, the threshold, the Response Factor, the MSR, and the Reaction and Recovery Times as a step per `control` period.
    - Evidence: host tests of the rise from LRL to MSR and of the fall, against the programmed times; a property test that the sensor rate never exceeds the MSR
    - _Requirements: PM15.1, PM15.2, PM15.3, PM15.4, PM15.5, PM15.6, PM29.3_
  - [ ] 19.2 Hysteresis and Rate Smoothing.
    - Evidence: host tests; a property test that successive intervals differ by no more than the smoothing allows
    - _Requirements: PM16.1, PM16.2, PM16.3_
  - [ ] 19.3 ATR: the detection counter, Duration, Fallback to VVIR with the rate lowered over the Fallback Time, the end of the tachycardia in each phase, and every switch at a ventricular event.
    - Evidence: host tests of onset, cessation in Duration, cessation in Fallback, and of premature beats that must not start it
    - _Requirements: PM14.1, PM14.2, PM14.3, PM14.4, PM14.5, PM14.6, PM14.7, PM14.8_
  - [ ] 19.4 Noise: a Chamber noisy for the cycle by the rule of "Sense intake, MIT and noise", asynchronous pacing in that cycle, the Marker TN.
    - Evidence: host tests with edge counts and service spacings on both sides of PM-PAR-12
    - _Requirements: PM13.1, PM13.2_
  - [ ] 19.5 The rate-adaptive Modes in `validate` and the State machine, and the contracts of task 12 re-proven with the escape interval now the sensor's or the hysteresis one.
    - Evidence: exhaustive `validate` tests for the added parameters; the Creusot proofs of Properties P1 to P5 and P8 pass on the E4 code
    - _Requirements: PM1.1, PM5.1, PM19.1, PM19.3, PM25.3, PM25.4_

- [ ] 20. Verification of E4
  - [ ] 20.1 Re-measure the records (the step is longer) and re-run the Analyzer.
    - Evidence: refreshed records and PASS
    - _Requirements: PM21.3, PM21.4_
  - [ ] 20.2 Rig scenarios: an activity profile from the DCM_Stub with the rate's rise and fall timed; hysteresis and smoothing; an atrial tachycardia played by the Heart_Simulator through onset, Duration, Fallback and cessation.
    - Evidence: rig reports with rates judged against PM-PAR-02 and each ATR switch located at a ventricular event
    - _Requirements: PM14.8, PM15.2, PM15.5, PM15.6, PM16.1, PM16.3, PM24.3_
  - [ ] 20.3 The sense storm: continuous edges on one sense line, then on both. The Partition must not restart, no MIT_Violation may be recorded for masked edges, pacing must be asynchronous, and `therapy`'s deadlines must hold.
    - Evidence: rig record with the Kernel's log, the restart count, and every interval judged
    - _Requirements: PM13.1, PM22.6, PM4.2_

- [ ] 21. Checkpoint: E4 complete. All nineteen Modes run on the rig, and the storm scenario leaves the Therapy_Partition running. Ask the user if questions arise.

### Phase E5 — History and diagnostics

- [ ] 22. The Partition `diag` and what it records
  - [ ] 22.1 The Endpoint `events` and the ring: `control` drains up to eight records per period; an overflow is counted and reported.
    - Evidence: host test of the ring; rig run at the highest event rate with the drop count read back
    - _Requirements: PM18.8, PM22.4_
  - [ ] 22.2 Markers: every Marker of the source with its modifiers and the time since the Chamber's last non-refractory event, supplied over the link.
    - Evidence: rig run comparing the Marker stream's times with the analyzer's edges against PM-PAR-04
    - _Requirements: PM18.5, PM18.6_
  - [ ] 22.3 The Partition `diag` at level D with the Task `history`: rate histograms of the four event kinds with their reset, and the trends of pacing rate and sensor data.
    - Evidence: rsk's build checks; host tests of the binning; rig run against a known scenario
    - _Requirements: PM18.1, PM18.2, PM18.3, PM30.1_
  - [ ] 22.4 The Endpoints `records` (validated by `comms`) and `request`, and the record commands in `link` and the DCM_Stub.
    - Evidence: rig run reading each record kind; a malformed record frame refused
    - _Requirements: PM1.3, PM20.1, PM22.4_
  - [ ] 22.5 Implant data and the measured-data stand-ins: battery voltage and level, wave amplitudes and lead impedance as supplied values marked as such.
    - Evidence: rig run of each interrogation
    - _Requirements: PM17.1, PM17.2, PM17.3, PM17.5, PM18.7, PM19.4_
  - [ ] 22.6 The pacing threshold test: the step-down every fourth cycle with back-up pacing, ended by Stop or by the loss of the link.
    - Evidence: host tests; rig run of an atrial and a ventricular test
    - _Requirements: PM17.4_
  - [ ] 22.7 Electrograms on the seventh Task: decide what an electrogram is at a boundary of digital signals, then implement it or record it as a finding.
    - Evidence: decision record; if implemented, the Analyzer's PASS with seven Tasks and a rig run
    - _Requirements: PM18.4, PM32.1, PM32.3_

- [ ] 23. Closing evidence
  - [ ] 23.1 Complete the traceability table: every acceptance criterion with its design element, code and evidence, or its recorded reason for having none.
    - Evidence: the table's check passes with no criterion unaccounted
    - _Requirements: PM31.1, PM1.5, PM20.4_
  - [ ] 23.2 Complete the findings: every place the source could not be met on rsk or on the Target, the inhibition window, the P0 ownership, whether timed pacing justified a Kernel one-shot event, and whether seven Tasks sufficed.
    - Evidence: the findings document
    - _Requirements: PM31.2, PM27.6, PM32.3_
  - [ ] 23.3 One final campaign on one revision: the verification job with the Application in it, the records refreshed, and every stage's rig scenarios.
    - Evidence: the job's result and the campaign's reports, each naming its run
    - _Requirements: PM31.3, PM21.2, PM21.4_

- [ ] 24. Checkpoint: E5 complete. Every acceptance criterion has evidence or a recorded reason. Ask the user if questions arise.

## Task Dependency Graph

Tasks in the same wave can run in parallel as work; on the rig and under the provers they still run one at a time. Each wave starts after the previous one finishes.

```json
{
  "waves": [
    { "id": 0, "tasks": ["1.1", "2.1", "3.1", "4.1", "6.1"] },
    { "id": 1, "tasks": ["1.2", "3.2", "3.5", "4.2", "4.3", "6.2", "6.3"] },
    { "id": 2, "tasks": ["1.3", "3.3", "3.4", "7.1"] },
    { "id": 3, "tasks": ["1.4", "3.6"] },
    { "id": 4, "tasks": ["3.7"] },
    { "id": 5, "tasks": ["5"] },
    { "id": 6, "tasks": ["7.2", "7.3", "8.1"] },
    { "id": 7, "tasks": ["8.2", "8.3"] },
    { "id": 8, "tasks": ["8.4", "9.1", "9.6", "9.7"] },
    { "id": 9, "tasks": ["9.2"] },
    { "id": 10, "tasks": ["9.3", "9.4", "9.5"] },
    { "id": 11, "tasks": ["10"] },
    { "id": 12, "tasks": ["11.1", "11.5", "12.4"] },
    { "id": 13, "tasks": ["11.2", "11.3", "11.4"] },
    { "id": 14, "tasks": ["11.6", "12.1", "12.2"] },
    { "id": 15, "tasks": ["12.3", "13.1"] },
    { "id": 16, "tasks": ["13.2", "13.3"] },
    { "id": 17, "tasks": ["14"] },
    { "id": 18, "tasks": ["15.1", "16.2"] },
    { "id": 19, "tasks": ["15.2", "15.3"] },
    { "id": 20, "tasks": ["15.4", "16.1"] },
    { "id": 21, "tasks": ["16.3", "16.4", "16.5", "16.6"] },
    { "id": 22, "tasks": ["17.1", "17.2", "17.3"] },
    { "id": 23, "tasks": ["18"] },
    { "id": 24, "tasks": ["19.1", "19.2", "19.3", "19.4"] },
    { "id": 25, "tasks": ["19.5"] },
    { "id": 26, "tasks": ["20.1"] },
    { "id": 27, "tasks": ["20.2", "20.3"] },
    { "id": 28, "tasks": ["21"] },
    { "id": 29, "tasks": ["22.1"] },
    { "id": 30, "tasks": ["22.2", "22.3"] },
    { "id": 31, "tasks": ["22.4", "22.5", "22.6", "22.7"] },
    { "id": 32, "tasks": ["23.1", "23.2"] },
    { "id": 33, "tasks": ["23.3"] },
    { "id": 34, "tasks": ["24"] }
  ]
}
```

## Notes

- Nothing in this plan is started. Every period, MIT, deadline and Budget in `design.md` is a proposal until task 9.2 replaces it with a measurement.
- If the spike (task 1) refutes a register behaviour, tasks 7 and 8.1 follow the revised design; `pm-core` (tasks 6, 11, 19) does not depend on the outcome.
- The spike is built outside the rsk workspace, so rsk's unsafe-code gates do not apply to it. Nothing of it is carried into the Application: task 7.2 writes the registers through `rsk::mmio` under `#![forbid(unsafe_code)]`.
- Task 3 changes rsk itself and is the only task that does (PM33.7). It carries rsk's own rules: no existing proof, check or test is weakened to make it pass, and a broken proof stops the work.
- The verification job, the hour-long rig runs and the full mutation register are run when the user asks. Tasks 3.7 and 23.3 name such runs as their evidence and wait for that. The other tasks use the cheap checks: host tests, fixtures, the Target build, one prover on one crate.
- The rig has one user at a time, and the provers run one at a time. The waves show which work does not depend on other work, not what may run at once on the rig.
- The Analyzer's verdict on margins is reported as it comes. A FAIL that is driven by margin alone is recorded, not engineered away.
- Requirements recorded and not implemented have no task: the clinical content (PM1.5), the lead's electrical properties (PM3.4), the magnet's detection distance (PM8.5), the DCM's own behaviour (PM20.4), and the conditions for more than seven Tasks (PM32.4 to PM32.6), which apply only if task 22.7 needs them. Task 23.1 lists each with its reason.
- Tasks that depend on the frame protocol (15, 16, 22.4) follow the decision of task 15.1, which closes PD-O6.
