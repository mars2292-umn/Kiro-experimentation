# Requirements Document

**PACEMAKER on rsk: an experimental Application**

Status: Draft 0.5 (0.5, 2026-11-23: PM-PAR-15 is 4 ms, from task 9.2; amended by `design.md` Draft 0.2, section "Requirements Impact"; 0.4 changes PM27.5 only; nothing implemented; the Task set is the design's Table 2; every open question decided on 2026-11-22). Spec: `pacemaker`. Written 2026-11-22 from the source document, the rsk requirements (`.kiro/specs/rsk-ravenscar-profile/requirements.md`, Draft 0.3) and the rsk tree at that date.

## Introduction

This document states the requirements of an experimental Application for rsk: the pulse generator of the PACEMAKER System Specification, implemented as Partitions and Tasks on the rsk Kernel. Its purpose is to exercise rsk on a published specification of a real hard real-time system, with real timing tolerances, modes and fault cases, and to find what rsk cannot yet express.

It aggregates three sources, and every requirement names which it comes from:

- **S. The source.** *PACEMAKER System Specification*, Boston Scientific, January 3, 2007 (`docs/pacemaker/PACEMAKER.pdf`, 35 pages), released for the Pacemaker Formal Methods Challenge. Cited as `[PM §n]`, `[PM Table n]`, `[PM App. A]`.
- **I. Inherited.** The rsk Profile and requirements, which bind any Application built on rsk. Cited as `Rn.m` and `PR-nn` of the rsk requirements.
- **P. Platform.** What the Phase 1 Target (the nRF52840-DK) and the rsk tree as it stands allow and do not allow.

**This is not a medical device and not a design for one.** The source's own notice applies to this document and to everything built from it:

> Copyright © 2007 Boston Scientific. DISCLAIMER: This document is provided for academic purposes only, specifically demonstration of formal methodologies and engineering education. Boston Scientific disclaims any use of information contained herein for any product whatsoever, disclaims safety, disclaims efficacy, and disclaims fitness for purpose. Any person or entity who uses this document for anything assumes complete responsibility for its use. Permission to copy is granted, provided the copyright declaration and this disclaimer are included.

### Fixed inputs (decided by the user; not open in this spec)

- The source document is the functional specification. Where it is silent, ambiguous or inconsistent, the point is an open question (`PM-ORQ`), not a decision made here.
- Where a table of the source and its text disagree, the text governs (2026-11-22).
- Where the text is silent, the conventional behaviour of a pacemaker is taken and marked as such in the criterion; each remaining open question is decided at the start of the stage it affects (2026-11-22).
- Sense signals, the battery level and the activity value are simulated; no analog measurement and no accelerometer (2026-11-22).
- rsk changes for this experiment are limited to fixed PPI channels and TIMER3 in the Target table (Requirement 33) and, only if needed, the Task limit (Requirement 32). No Kernel one-shot event, no further EasyDMA classes (2026-11-22).
- The Application is built on rsk as it is: the Ravenscar Profile_Variant, the nRF52840-DK, the HIL_Rig.
- Scope of this draft: requirements only. The Task set, the Partition layout and the implementation are later work.

### Scope

| Part of the source's system | In this spec |
|---|---|
| Pulse generator (PG): pacing, sensing, modes, states, timing, stored data | **In scope**: the Application |
| Leads, the heart, the analog front end (pulse amplitude, sense amplifiers, impedance, battery) | **Substituted**: a Heart_Simulator on a second board and digital lines (Part I) |
| Device Controller-Monitor (DCM): screens, reports, printing, languages | **Out of scope** but recorded (Requirement 20), except the commands and data the PG must accept and supply, which a host-side DCM_Stub exercises |
| Clinical content (indications, life phases, manufacturing, disposal) | **Not requirements of the Application**; summarized in Requirement 1 for context |

### Stages

| Stage | Scope (proposal) |
|---|---|
| E1 | Single-chamber modes without sensing and with inhibition (AOO, VOO, AAI, VVI); LRL; refractory periods; the Heart_Simulator and the timing measurement |
| E2 | Dual-chamber modes (DOO, DDI, DDD, VDD); AV delay; PVARP; URL; triggered modes (AAT, VVT) |
| E3 | States: Temporary, Pace-Now, Magnet, POR; battery levels; parameter programming through an Endpoint with validation |
| E4 | Rate-adaptive modes; hysteresis; rate smoothing; ATR; noise response |
| E5 | Diagnostics: markers, histograms, trending, measured data, implant data; the DCM_Stub's reports |

Each requirement heading names the earliest stage in which it must hold. Inherited and platform requirements hold from E1.

### Conventions

- `PMn.m` is acceptance criterion m of Requirement n of this document. `Rn.m`, `PR-nn`, `ASM-nn`, `PAR-nn`, `DD-nn` are identifiers of the rsk requirements and design, unchanged.
- `PM-PAR-nn` is a parameter, `PM-NG-nn` a non-goal, `PM-ASM-nn` an assumption, `PM-ORQ-nn` an open question.
- Origin of a requirement: **S** source, **I** inherited, **P** platform. A requirement may carry more than one.
- Rates are in pulses per minute (ppm), which the source also writes as beats per minute. An interval of a rate r is 60,000 / r ms. The source gives the tolerance of a rate as a time (±8 ms): it is the tolerance of the rate's interval (decided 2026-11-22).
- A cardiac cycle (cc) is the interval between two successive ventricular events, or atrial events in an AXX mode.
- "Event" without qualification is a paced or a non-refractory sensed event of a chamber. `X` in a mode name is any letter (`DXX`: the dual-chamber paced modes, `OXO`: the sensing-only modes, `XXT`: the triggered modes) [PM §3.5].
- Time quantities in the implementation are integers in Kernel time-base ticks (rsk Conventions); this document states them in ms as the source does.
- Content of the source was rephrased into acceptance criteria; the tables of Requirement 19 reproduce its values.

### Parameters

| ID | Parameter | Value | Basis |
|---|---|---|---|
| PM-PAR-01 | Tolerance of a pacing or refractory interval | ±8 ms | [PM App. A] |
| PM-PAR-02 | Tolerance of the interval of the Maximum Sensor Rate | ±4 ms | [PM App. A] |
| PM-PAR-03 | Tolerance of the Sensed AV Delay Offset | ±1 ms | [PM App. A] |
| PM-PAR-04 | Accuracy of an event marker's time | 1 ms | [PM §4.8] |
| PM-PAR-05 | Tolerance of a pulse width | 0.2 ms; ±0.02 ms for the Pace-Now and POR pulse | [PM App. A], [PM §3.6.3], [PM §3.6.5]; both apply as stated, and hardware timing meets both (PM28.2) |
| PM-PAR-06 | Shortest interval between two paces of a chamber | 342.8 ms (175 ppm) | [PM App. A], the highest programmable rate |
| PM-PAR-07 | Longest interval between two paces of a chamber with hysteresis off | 2,000 ms (30 ppm) | [PM App. A], the lowest programmable rate |
| PM-PAR-08 | Kernel time-base tick of the Target | 62.5 ns (16 MHz) | rsk Target table (`nrf52840`) |
| PM-PAR-09 | Task Priority levels available | 7 | PAR-06, PR-33 |
| PM-PAR-11 | Successive unanswered polls after which the DCM link is lost | 3 (proposal) | PM20.2 |
| PM-PAR-12 | Noise interval: sense edges closer than this, throughout a cycle, are continuous noise | 100 ms (proposal) | PM13.1 |
| PM-PAR-13 | ATR detection count: the counter value at which onset is detected; cessation at zero | 8 (proposal) | PM14.1 |
| PM-PAR-14 | Largest activity value | 255 | PM15.4 |
| PM-PAR-15 | Inhibition window: a Sense this close before a pending Pace need not cancel it | 4 ms (the deadline of the slower sense Task, `sense_a`, which the Analyzer confirmed on measured records, task 9.2; the proposal was 2 ms) | design PD-04, PM5.5 |
| PM-PAR-10 | Release lag J_rel measured on the Target, with margin | 11,590 cycles (181 µs) | `docs/hil/kernel_timing_nrf52840_stress.json`, 2026-11-21; to be re-measured for this Application (PM21.3) |

### Component traceability

| Component | Requirements |
|---|---|
| A. System context | 1–2 |
| B. Pacing output and sensing | 3–4 |
| C. Modes and states | 5–8 |
| D. Bradycardia therapy | 9–16 |
| E. Diagnostics and stored data | 17–18 |
| F. Parameters | 19 |
| G. DCM and telemetry | 20 |
| H. Inherited from rsk | 21–25 |
| I. Platform | 26–33 |

## Glossary

Terms of the rsk Glossary (Application, Partition, Task, Job, Release, Release_Source, Budget, MIT, Resource, Endpoint, Health_Monitor, Generator, Analyzer, Target, HIL_Rig and the others) are used as defined there. Terms of this document:

- **PG**: The pulse generator: the device the Application implements.
- **DCM**: The Device Controller-Monitor of the source: the programmer the physician uses.
- **DCM_Stub**: A host-side program that sends the DCM's commands to the PG and reads its data, for tests. It implements none of the DCM's user interface.
- **Heart_Simulator**: An Application on a second board of the HIL_Rig that produces atrial and ventricular sense signals and records the PG's paces.
- **Chamber**: The atrium (A) or the ventricle (V).
- **Sense**: A detected intrinsic event of a Chamber. **Refractory sense**: a Sense inside a refractory period of its Chamber, which neither inhibits nor triggers pacing.
- **Pace**: A pacing pulse delivered to a Chamber.
- **Mode**: A bradycardia operating mode named by the letters of [PM Table 2]: Chambers paced, Chambers sensed, response to sensing, and optionally R for rate modulation.
- **State**: One of Permanent, Temporary, Pace-Now, Magnet, Power-On Reset [PM §3.6].
- **LRL, URL, MSR, HRL**: Lower Rate Limit, Upper Rate Limit, Maximum Sensor Rate, Hysteresis Rate Limit.
- **AV delay**: The time from an atrial event to the ventricular pace it schedules. **PAV**, **SAV**: the AV delay after an atrial Pace, after an atrial Sense.
- **VRP, ARP, PVARP**: Ventricular, Atrial, and Post-Ventricular Atrial Refractory Period.
- **PVC**: Premature Ventricular Contraction: a ventricular Sense with no atrial event since the previous ventricular event [PM §4.8.2].
- **ATR**: Atrial Tachycardia Response.
- **SIR**: Sensor Indicated Rate: the pacing rate the activity sensor asks for.
- **Marker**: An event annotation of [PM Table 5].
- **BOL, ERN, ERT, ERP**: The battery status levels of [PM Table 3].
- **Therapy_Partition**: The Partition that holds the Tasks that decide and deliver Paces. Named here only so that requirements can refer to it; its Tasks are not fixed in this draft.

## Non-Goals

- **PM-NG-01** Use on or near a patient, clinical validity, and conformance to any medical device standard or regulation.
- **PM-NG-02** The DCM's user interface, languages, printed reports and strip-chart recorder [PM §3.2.1 to §3.2.5]. Requirement 20 records them; nothing implements them.
- **PM-NG-03** Analog electrical behaviour: regulated or unregulated pulse amplitude into a load, sense amplifiers and their thresholds, lead impedance, battery chemistry. The Application carries these as values and the Heart_Simulator supplies their digital stand-ins (Part I).
- **PM-NG-04** Inductive, RF or ultrasound telemetry [PM §3.2.6]. The link is a wire (PM29).
- **PM-NG-05** Electromagnetic compatibility with the equipment of [PM §2.4.2] and electrical safety.
- **PM-NG-06** Persistence of programmed parameters and stored data across a reset (decided 2026-11-22: no non-volatile storage is added to rsk for this experiment).
- **PM-NG-07** Service life: battery longevity, and continuous operation beyond the declared operating duration (PM27.4).

## Requirements

### Part A — System context

### Requirement 1: System definition (Stage E1) [S]

**User Story:** As a reader of this spec, I want the source's system stated once, so that the requirements that follow have their context.

#### Acceptance Criteria

1. THE PG SHALL provide bradycardia pacing support in single-chamber and dual-chamber Modes, with and without rate adaptation, in permanent and in temporary operation [PM §2.1, §2.2.1, §3.1].
2. THE PG SHALL be programmed and interrogated by the DCM over a bidirectional link, so that the Mode and the parameters can be changed without access to the PG [PM §2.2.1].
3. THE PG SHALL supply the history data and the diagnostic data of Part E to the DCM [PM §2.2.1].
4. THE PG SHALL keep its pacing and sensing functions available throughout the ambulatory phase, in which no DCM is connected [PM §2.5.5].
5. This document SHALL record, and not require, the source's clinical content: indications and contraindications [PM §2.3], users [PM §2.4], the life phases from pre-implant to disposal [PM §2.5], and the model table [PM Table 1].

### Requirement 2: The Application's boundary (Stage E1) [S, P]

**User Story:** As a developer, I want the boundary between the PG and everything else fixed, so that each requirement has a place where it can be tested.

#### Acceptance Criteria

1. THE Application SHALL implement the PG's behaviour at a boundary of digital signals: one sense input and one pace output per Chamber, a magnet input, a battery-level input, an activity input, and the link to the DCM_Stub (Part I).
2. THE Application SHALL treat every amplitude, sensitivity, impedance and voltage of the source as a value that it stores, validates, reports, and passes to its outputs, and SHALL NOT claim the electrical behaviour that the value describes (PM-NG-03).
3. THE Heart_Simulator SHALL be the only source of sense signals in tests, and its behaviour for each test SHALL be recorded with the test's result.

### Part B — Pacing output and sensing

### Requirement 3: Pacing pulse (Stage E1) [S]

**User Story:** As a therapy engineer, I want each pace to be a pulse of the programmed amplitude and width, so that pacing is delivered as programmed.

#### Acceptance Criteria

1. THE PG SHALL deliver a Pace to a Chamber as a pulse with the amplitude and the width programmed for that Chamber [PM §3.4].
2. THE PG SHALL keep the atrial and the ventricular pulse amplitude independently programmable, and the atrial and the ventricular pulse width independently programmable [PM §3.4.1, §3.4.2]. THE Application SHALL implement the regulated amplitude values of Table 19-1 only, with the nominal 3.5 V; the unregulated set is not implemented, amplitude being data on the Target (decided 2026-11-22).
3. THE PG SHALL deliver each pulse width within PM-PAR-05 of its programmed value [PM App. A].
4. THE PG SHALL support an atrial and a ventricular bipolar pace and sense lead, and SHALL operate normally with lead impedances from 100 to 2,500 Ω; amplitudes and widths are specified into 750 Ω [PM §3.3]. On the Target this criterion is recorded and not tested (PM-NG-03).

### Requirement 4: Sensing (Stage E1) [S]

**User Story:** As a therapy engineer, I want intrinsic events detected per Chamber and rate decided beat by beat, so that the PG responds to the heart's own rhythm.

#### Acceptance Criteria

1. THE PG SHALL detect Senses in each Chamber it senses in the programmed Mode [PM §3.4.3].
2. THE PG SHALL base every rate decision on measured cardiac cycle lengths of the sensed rhythm, evaluated interval by interval [PM §3.4.3].
3. THE PG SHALL keep the atrial and the ventricular sensing threshold independently adjustable by the DCM [PM §3.4.4].
4. WHILE a refractory period of a Chamber is in progress, THE PG SHALL treat a Sense in that Chamber as a Refractory sense: it SHALL neither inhibit nor trigger a Pace (Requirement 12).
5. WHILE the Mode paces the atrium and senses the ventricle (DDI, DDD and their rate-adaptive forms), THE PG SHALL discard every ventricular Sense that occurs within the programmed Ventricular Blanking interval after an atrial Pace: it SHALL neither inhibit nor trigger a Pace and SHALL produce no Marker [PM App. A]. The source lists the parameter and does not describe it; this is cross-chamber blanking (conventional behaviour, decided 2026-11-22).

### Part C — Modes and states

### Requirement 5: Operating modes (Stage E1 for the single-chamber modes; E2 dual-chamber; E4 rate-adaptive) [S]

**User Story:** As a physician, I want the standard bradycardia modes, so that pacing fits the patient's conduction.

#### Acceptance Criteria

1. THE PG SHALL be programmable to each of the Modes Off, DDDR, VDDR, DDIR, DOOR, VOOR, AOOR, VVIR, AAIR, DDD, VDD, DDI, DOO, VOO, AOO, VVI, AAI, VVT and AAT [PM §3.5].
2. THE PG SHALL offer the Modes OVO, OAO, ODO and OOO in temporary operation only [PM §3.5].
3. WHILE the response to sensing is O, THE PG SHALL deliver Paces without regard to Senses [PM §3.5.1].
4. WHILE the response to sensing is T, WHEN a non-refractory Sense occurs in a Chamber, THE PG SHALL deliver a Pace to that Chamber immediately [PM §3.5.2], which SHALL mean within PM-PAR-01 of the Sense (a choice of this spec, decided 2026-11-22); the measured delay SHALL be reported.
5. WHILE the response to sensing is I, WHEN a non-refractory Sense occurs in a Chamber, THE PG SHALL cancel the Pace pending for that Chamber [PM §3.5.3]. A Sense that precedes the pending Pace by less than PM-PAR-15 need not cancel it: a Sense is acted on by a Job, and a hardware inhibit is outside this design (PD-04); the window SHALL be measured and each such Pace recorded.
6. WHILE the response to sensing is D, WHEN a non-refractory atrial Sense occurs, THE PG SHALL deliver a ventricular Pace one AV delay after it unless a ventricular Sense occurs first [PM §3.5.4].
7. THE PG SHALL pace only the Chambers, and sense only the Chambers, that the Mode's first and second letters name [PM Table 2].
8. THE PG SHALL use, in each Mode, only the parameters that Table 19-3 marks for that Mode, as amended by Table 19-4.

### Requirement 6: States (Stage E3) [S]

**User Story:** As a physician, I want permanent, temporary and emergency operation kept apart, so that a test never changes the therapy the patient leaves with.

#### Acceptance Criteria

1. THE PG SHALL be in exactly one of the States Permanent, Temporary, Pace-Now, Magnet, and Power-On Reset at every instant [PM §3.6].
2. WHILE in the Permanent State, THE PG SHALL pace with the permanent parameters [PM §3.6.1].
3. WHILE in the Temporary State, THE PG SHALL pace with the temporary parameters and SHALL leave the permanent parameters unchanged [PM §3.6.2].
4. WHEN the link to the DCM is broken, a Pace-Now command is received, or the DCM commands the end of temporary pacing, THE PG SHALL leave the Temporary State [PM §3.6.2].
5. WHEN the DCM commands Pace-Now, THE PG SHALL enter the Pace-Now State with the Mode VVI, an LRL of 65 ppm, a ventricular amplitude of 5.0 V ±0.5 V, a pulse width of 1.00 ms ±0.02 ms, a VRP of 320 ms ±8 ms, and a ventricular sensitivity of 1.5 mV [PM §3.6.3].
6. WHEN the DCM commands Pace-Now, THE PG SHALL deliver the first Pace-Now pulse within two cardiac cycles plus 500 ms of the last user action that the command needs [PM §3.6.3]. The PG's share SHALL be: the first pulse no later than one Pace-Now LRL interval (923 ms) after the command arrives at the PG (a choice of this spec, decided 2026-11-22); the HIL_Rig SHALL measure the whole time from the DCM_Stub's send against the source's bound, the two cardiac cycles counted at the rate in force when the command is sent.
7. WHILE in the Pace-Now State, THE PG SHALL continue Pace-Now pacing until the DCM changes the Mode [PM §3.6.3].

### Requirement 7: Power-On Reset (Stage E3) [S, P]

**User Story:** As a safety engineer, I want a known mode and parameters after a reset, so that the PG paces predictably whatever preceded the reset.

#### Acceptance Criteria

1. WHEN the PG starts after a reset, THE PG SHALL enter the Power-On Reset State with the Mode VVI, an LRL of 65 ppm, a ventricular amplitude of 5.0 V ±0.5 V, a pulse width of 0.5 ms ±0.02 ms, a VRP of 320 ms ±8 ms, and a ventricular sensitivity of 1.5 mV [PM §3.6.5].
2. WHILE the battery voltage is below the POR trip voltage, THE PG SHALL keep all functions disabled [PM §3.6.5]. On the Target the battery-level input stands for the voltage (PM30.3).
3. THE Application SHALL reach its first Pace from a reset within a stated bound that includes the Kernel's boot checks (R12), and the bound SHALL be measured on the HIL_Rig.
4. WHEN the DCM programs a set of permanent parameters that the PG accepts (PM19.3), THE PG SHALL leave the Power-On Reset State for the Permanent State, and no other event SHALL end the Power-On Reset State except Pace-Now and the magnet (a choice of this spec, decided 2026-11-22); the source does not say how the State is left.

### Requirement 8: Magnet (Stage E3) [S]

**User Story:** As a clinician, I want a magnet to force fixed-rate pacing whose rate shows the battery level, so that the battery can be checked without a DCM.

#### Acceptance Criteria

1. WHEN the magnet input is asserted and the magnet response is enabled, THE PG SHALL enter the Magnet State and pace asynchronously: AOO if the previous Mode was AXX, VOO if VXX, DOO if DXX, and OOO if it was a sensing-only Mode [PM §3.7].
2. WHILE in the Magnet State, THE PG SHALL pace at 100 ppm at BOL, 90 ppm at ERN, and 85 ppm at ERT [PM §3.7].
3. WHEN the magnet input is released, THE PG SHALL resume the operation that preceded the Magnet State [PM §3.7].
4. WHERE the magnet response is programmed Off, THE PG SHALL ignore the magnet input [PM §3.7].
5. The source's detection distance of 2.5 cm is recorded and not tested (PM-NG-03).

### Part D — Bradycardia therapy

### Requirement 9: Lower Rate Limit (Stage E1) [S]

**User Story:** As a therapy engineer, I want a guaranteed minimum rate, so that the patient is never paced slower than programmed.

#### Acceptance Criteria

1. WHILE hysteresis is disabled, THE PG SHALL deliver a Pace no later than one LRL interval after the event that starts the interval, within PM-PAR-01, in the absence of a Sense that inhibits it and of sensor-driven pacing at a higher rate [PM §5.1].
2. WHILE the Mode is DXX or VXX, THE PG SHALL start the LRL interval at each ventricular event [PM §5.1].
3. WHILE the Mode is AXX, THE PG SHALL start the LRL interval at each atrial event [PM §5.1].

### Requirement 10: Upper Rate Limit (Stage E2) [S]

**User Story:** As a therapy engineer, I want a ceiling on tracked pacing, so that a fast atrial rhythm does not drive the ventricle too fast.

#### Acceptance Criteria

1. THE PG SHALL NOT deliver a ventricular Pace earlier than one URL interval after the previous ventricular event, less PM-PAR-01 [PM §5.2].
2. WHILE the Mode tracks atrial Senses, THE PG SHALL track them at rates up to the URL and no higher [PM §5.2]: WHEN the ventricular Pace that an atrial Sense schedules would fall inside the URL interval, THE PG SHALL delay that Pace to the end of the URL interval and SHALL NOT drop it. The source does not state this; it is the conventional upper-rate behaviour, taken where the text is silent (decided 2026-11-22).

### Requirement 11: AV delay (Stage E2) [S]

**User Story:** As a therapy engineer, I want the ventricle paced a programmed time after the atrium, so that the Chambers stay synchronized.

#### Acceptance Criteria

1. WHILE the Mode tracks atrial Senses and the sensed atrial rate is between the LRL and the URL, WHEN no ventricular Sense occurs within the AV delay after an atrial event, THE PG SHALL deliver a ventricular Pace at the end of the AV delay, within PM-PAR-01 [PM §5.3].
2. THE PG SHALL use the paced AV delay after an atrial Pace and the sensed AV delay after an atrial Sense [PM §5.3.1, §5.3.2].
3. WHERE the AV delay is fixed, THE PG SHALL use the programmed Fixed AV Delay [PM §5.3].
4. WHERE the AV delay is dynamic, THE PG SHALL compute the AV delay of each cardiac cycle as the previous cardiac cycle's length multiplied by a factor, limited to the range from the programmed Minimum Dynamic AV Delay to the maximum [PM §5.3.3]. The maximum SHALL be the programmed Fixed AV Delay, and the factor SHALL be the Fixed AV Delay divided by the LRL interval, so that the AV delay is the maximum at the LRL and shortens in proportion as the rate rises (a choice of this spec, decided 2026-11-22); the source names a stored factor and a programmable maximum and gives neither.
5. WHERE the Sensed AV Delay Offset is enabled, THE PG SHALL shorten the AV delay after a tracked atrial Sense by the programmed offset, within PM-PAR-03, whether the AV delay is fixed or dynamic [PM §5.3.4].
6. WHILE an AV delay is in progress, THE PG SHALL treat atrial Senses as Refractory senses [PM §5.4.5].

### Requirement 12: Refractory periods (Stage E1 for VRP and ARP; E2 for PVARP) [S]

**User Story:** As a therapy engineer, I want senses that follow an event too closely ignored, so that the PG does not respond to its own pulses or to artefacts.

#### Acceptance Criteria

1. WHILE the VRP after a ventricular event is in progress, THE PG SHALL NOT let a ventricular Sense inhibit or trigger a Pace [PM §5.4.1].
2. WHILE the Mode is a single-chamber atrial Mode and the ARP after an atrial event is in progress, THE PG SHALL NOT let an atrial Sense inhibit or trigger a Pace [PM §5.4.2].
3. WHILE the Mode paces the ventricle and senses the atrium and the PVARP after a ventricular event is in progress, THE PG SHALL NOT let an atrial Sense inhibit an atrial Pace or trigger a ventricular Pace [PM §5.4.3].
4. WHERE PVARP Extension is enabled, WHEN a PVC occurs, THE PG SHALL use the extended PVARP for the refractory period that follows the PVC, and the programmed PVARP on the next cardiac cycle [PM §5.4.4].
5. THE PG SHALL extend the PVARP at most once in any two successive cardiac cycles [PM §5.4.4].
6. THE PG SHALL keep each refractory period within PM-PAR-01 of its programmed length [PM App. A].

### Requirement 13: Noise response (Stage E4) [S]

**User Story:** As a safety engineer, I want pacing to continue when sensing is unusable, so that noise cannot inhibit therapy.

#### Acceptance Criteria

1. WHILE continuous noise is present on a sense input, THE PG SHALL pace asynchronously [PM §5.5]. A Chamber SHALL be noisy for a cardiac cycle when, from the end of its refractory period to the instant of the pending Pace, successive sense edges of that Chamber are never further apart than PM-PAR-12; in a noisy cycle THE PG SHALL deliver the pending Pace at its instant whatever was sensed (a choice of this spec, decided 2026-11-22).
2. THE PG SHALL annotate the condition with the Marker TN [PM Table 5].

### Requirement 14: Atrial Tachycardia Response (Stage E4) [S]

**User Story:** As a physician, I want tracking suspended during an atrial tachycardia, so that the ventricle is not paced at a high rate for long.

#### Acceptance Criteria

1. WHERE ATR is enabled, THE PG SHALL keep a detection counter that rises by one, up to PM-PAR-13, at each atrial interval shorter than the URL interval and falls by one, down to zero, at each longer one, and SHALL detect the onset of an atrial tachycardia WHEN the counter reaches PM-PAR-13 [PM §5.6, §5.6.1] (conventional behaviour, decided 2026-11-22); this quantifies the source's "predominately, but not exclusively, faster than URL".
2. THE PG SHALL detect the end of the tachycardia WHEN the counter of PM14.1 returns to zero; the source's sentence for cessation repeats the words of onset and is read as "slower than URL" (decided 2026-11-22).
3. THE PG SHALL choose the detection period so that occasional premature atrial beats do not start ATR, occasional slow beats do not end it, and ATR is neither delayed nor continued unnecessarily [PM §5.6.1].
4. WHEN the onset is detected, THE PG SHALL enter ATR Duration and wait the programmed number of cardiac cycles before Fallback [PM §5.6.2].
5. IF the end of the tachycardia is detected during ATR Duration, THEN THE PG SHALL end ATR Duration immediately and SHALL NOT enter Fallback [PM §5.6.2].
6. WHEN ATR Duration ends with the tachycardia still present, THE PG SHALL enter Fallback, switch to the Fallback Mode VVIR, and lower the pacing rate to the LRL over the programmed Fallback Time [PM §5.6.3].
7. IF the end of the tachycardia is detected during Fallback, THEN THE PG SHALL end Fallback immediately and return to the programmed Mode [PM §5.6.3].
8. THE PG SHALL make every ATR mode switch at a ventricular event [PM §5.6.3].

### Requirement 15: Rate-adaptive pacing (Stage E4) [S]

**User Story:** As a physician, I want the pacing rate to follow physical activity, so that the patient's heart rate rises with exertion.

#### Acceptance Criteria

1. WHILE the Mode is rate-adaptive, THE PG SHALL adjust the pacing interval from the activity sensor's output [PM §5.7].
2. THE PG SHALL NOT pace, under sensor control, faster than the MSR, within PM-PAR-02; the MSR SHALL be programmable independently of the URL and required in every rate-adaptive Mode [PM §5.7.1].
3. WHILE the sensor's output is at or below the Activity Threshold, THE PG SHALL NOT raise the rate for activity [PM §5.7.2].
4. THE PG SHALL give a larger rate change for the same activity at a higher Response Factor, the largest at 16 and the smallest at 1 [PM §5.7.3]. The target rate SHALL be the LRL plus (MSR − LRL) × e × f / 16, limited to the MSR, where f is the Response Factor and e, from 0 to 1, is the activity above the Activity Threshold as a fraction of the range above it; activity is a value from 0 to PM-PAR-14 and the seven Activity Thresholds are evenly spaced over that range (a choice of this spec, decided 2026-11-22). The source gives no mapping.
5. THE PG SHALL raise the rate so that sustained activity takes it from the LRL to the MSR in the programmed Reaction Time [PM §5.7.4].
6. WHEN activity falls below the Activity Threshold, THE PG SHALL lower the rate so that it falls from the MSR to the LRL in the programmed Recovery Time [PM §5.7.5].

### Requirement 16: Hysteresis and rate smoothing (Stage E4) [S]

**User Story:** As a physician, I want the PG to wait a little longer after an intrinsic beat and to limit sudden rate changes, so that the heart's own rhythm is favoured and rate changes are gentle.

#### Acceptance Criteria

1. WHERE hysteresis is enabled and the Mode inhibits or tracks, WHEN a single non-refractory Sense occurs in the Chamber that starts the LRL interval (the atrium in AAI, the ventricle in a Mode with ventricular pacing), THE PG SHALL use the Hysteresis Rate Limit interval, which may be longer than the LRL interval, as the interval to the next Pace [PM §5.8].
2. WHEN a Pace is delivered to the Chamber that starts the LRL interval, THE PG SHALL return to the LRL interval for the next escape interval, so that a Pace ends hysteresis (conventional behaviour, decided 2026-11-22). The source's condition that "the current pacing rate must be faster than the HRL" SHALL be the interactive limit that the HRL is not above the LRL (Table 19-5).
3. WHERE Rate Smoothing is enabled, THE PG SHALL keep each pacing interval within the previous cardiac cycle interval less the Rate Smoothing Up percentage of it and plus the Rate Smoothing Down percentage of it, and within the LRL and the URL or MSR in force [PM §5.9]. Rate Smoothing Up and Rate Smoothing Down SHALL be two parameters, each with the values Table 19-1 lists for Rate Smoothing: [PM App. A] lists one, and the text governs (decided 2026-11-22).

### Part E — Diagnostics and stored data

### Requirement 17: Measured data, tests and battery status (Stage E5; PM17.6 and PM17.7 in E3) [S]

**User Story:** As a clinician, I want the PG's measurements and battery level, so that leads and battery can be judged at follow-up.

#### Acceptance Criteria

1. WHEN the DCM commands it, THE PG SHALL measure P and R wave amplitudes, with the increment and tolerance of Table 19-2 [PM §4.2, App. B].
2. WHEN the DCM commands it, THE PG SHALL measure lead impedance, in the Temporary State, at a default of 5.5 V [PM §4.3].
3. THE PG SHALL supply the battery voltage, the battery status level, and the date of the last interrogation [PM §4.4].
4. THE PG SHALL support the DCM's automatic pacing threshold test in AAI, VVI and DDD: a start value of amplitude or pulse width, a step down of one setting about every fourth cardiac cycle, back-up pacing at a programmable rate (DDD or AAI for an atrial test, DDD or VVI for a ventricular test), and an end on a Stop command or the loss of the link [PM §4.5].
5. On the Target the measurements of PM17.1 to PM17.3 SHALL return values supplied by the Heart_Simulator or fixed by the test, marked as simulated (PM-NG-03).
6. THE PG SHALL take the battery status level BOL, ERN, ERT or ERP from the battery input [PM §4.4, Table 3]; on the Target the input is the level itself, so no voltage thresholds are defined (decided 2026-11-22).
7. WHILE the battery status level is ERT or ERP, THE PG SHALL offer only non-rate-adaptive single-chamber Modes and SHALL disable temporary programming, automatic threshold testing, measured data, electrograms and Markers; WHILE it is ERP, the pacing rate MAY decrease as the voltage decreases [PM Table 3]. WHEN the level becomes ERT, THE PG SHALL change the Mode in force at a ventricular event (an atrial event in an AXX Mode): rate adaptation is removed; DOO becomes VOO and every other Mode that paces or senses both Chambers, and VDD, becomes VVI; an atrial Mode stays atrial (a choice of this spec, decided 2026-11-22). "ERP" of [PM Table 3] and "post-ERT" of [PM §3.7] are one level.

### Requirement 18: History, electrograms, markers and implant data (Stage E5) [S]

**User Story:** As a clinician, I want a record of what the PG did and saw, so that parameters can be adjusted from evidence.

#### Acceptance Criteria

1. THE PG SHALL record rate histograms of paced atrial, sensed atrial, paced ventricular and sensed ventricular events, each interval beginning and ending at the events of [PM Table 4], and the counts of PVCs and of ATR episodes [PM §4.6.1].
2. THE PG SHALL keep the histograms from their last reset to the present, and WHEN the DCM commands a reset, SHALL clear all of them together [PM §4.6.1].
3. THE PG SHALL record pacing rate and sensor data, separately or together, with a recording duration of Fixed (from now until storage is full, time-stamped at the beginning) or Continuous (the latest data in a circular buffer, time-stamped at the end) [PM §4.6.2, §4.6.3].
4. THE PG SHALL supply real-time internal electrograms of the atrial and of the ventricular lead, and WHEN a broken link is re-established during transmission, SHALL resume the transmission [PM §4.7]. On the Target an electrogram is the sampled sense input (PM-NG-03).
5. THE PG SHALL supply the Markers of [PM Table 5] with each event: AS, AP, AT (an atrial Sense faster than the URL), VS, VP, PVC, TN; the modifiers for a Refractory sense, a hysteresis Pace, a sensor-rate Pace, and a Pace under rate smoothing up or down; and the augmentation Markers ATR-Dur, ATR-FB, ATR-End and PVP→ [PM §4.8].
6. THE PG SHALL give each Marker the time since the most recent non-refractory event of its Chamber, accurate to PM-PAR-04, and SHALL produce at most one atrial, one ventricular and one augmentation Marker in any 1 ms [PM §4.8].
7. THE PG SHALL store the device model and serial number, the implant date, the lead implant date and polarity, pacing thresholds, P and R wave amplitudes, lead impedance, and the patient's indications for pacing [PM §3.8].
8. THE PG SHALL hold all recorded data in storage whose capacity is fixed at build time (PR-14), and SHALL state for each record what is kept and what is discarded when the storage is full.

### Part F — Parameters

### Requirement 19: Programmable and measured parameters (Stage E1 for the parameters of the E1 Modes; the others with their features) [S]

**User Story:** As a physician, I want each parameter limited to its published values, so that the PG can only be programmed to settings that were specified.

#### Acceptance Criteria

1. THE PG SHALL accept for each parameter of Table 19-1 only the values the table lists, and SHALL hold the nominal value until one is programmed.
2. THE PG SHALL meet the tolerance of Table 19-1 for each parameter that states one.
3. IF a programmed value is outside Table 19-1, or a set of values violates an interactive limit of Table 19-5, THEN THE PG SHALL reject the whole set and keep the parameters in force (PM22.3). The source requires that interactive limit errors be indicated and lists no limits [PM §4.9.2]; Table 19-5 is the list (a choice of this spec, decided 2026-11-22).
4. THE PG SHALL report measured values with the increments and tolerances of Table 19-2.
5. THE PG SHALL apply Table 19-3, as amended by Table 19-4, as the allocation of parameters to Modes (PM5.8).

**Table 19-1 — Programmable parameters** [PM Table 7]

| Parameter | Values | Increment | Nominal | Tolerance |
|---|---|---|---|---|
| Mode | Off, DDD, VDD, DDI, DOO, AOO, AAI, VOO, VVI, AAT, VVT, DDDR, VDDR, DDIR, DOOR, AOOR, AAIR, VOOR, VVIR | — | DDD | — |
| Lower Rate Limit | 30–50 ppm; 50–90 ppm; 90–175 ppm | 5 ppm; 1 ppm; 5 ppm | 60 ppm | ±8 ms |
| Upper Rate Limit | 50–175 ppm | 5 ppm | 120 ppm | ±8 ms |
| Maximum Sensor Rate | 50–175 ppm | 5 ppm | 120 ppm | ±4 ms |
| Fixed AV Delay | 70–300 ms | 10 ms | 150 ms | ±8 ms |
| Dynamic AV Delay | Off, On | — | Off | — |
| Minimum Dynamic AV Delay | 30–100 ms | 10 ms | 50 ms | not given |
| Sensed AV Delay Offset | Off, −10 to −100 ms | −10 ms | Off | ±1 ms |
| A or V Pulse Amplitude, regulated | Off; 0.5–3.2 V; 3.5–7.0 V | 0.1 V; 0.5 V | 3.5 V | ±12 % |
| A or V Pulse Amplitude, unregulated | Off, 1.25, 2.5, 3.75, 5.0 V | — | 3.75 V | — |
| A or V Pulse Width | 0.05 ms; 0.1–1.9 ms | —; 0.1 ms | 0.4 ms | 0.2 ms |
| A or V Sensitivity | 0.25, 0.5, 0.75 mV; 1.0–10 mV | —; 0.5 mV | A 0.75 mV, V 2.5 mV | ±20 % |
| Ventricular Refractory Period | 150–500 ms | 10 ms | 320 ms | ±8 ms |
| Atrial Refractory Period | 150–500 ms | 10 ms | 250 ms | ±8 ms |
| PVARP | 150–500 ms | 10 ms | 250 ms | ±8 ms |
| PVARP Extension | Off, 50–400 ms | 50 ms | Off | ±8 ms |
| Hysteresis Rate Limit | Off, or the values of the LRL | — | Off | ±8 ms |
| Rate Smoothing | Off, 3, 6, 9, 12, 15, 18, 21, 25 % | — | Off | ±1 % |
| ATR Mode | On, Off | — | Off | — |
| ATR Duration | 10 cc; 20–80 cc; 100–2,000 cc | —; 20 cc; 100 cc | 20 cc | ±1 cc |
| ATR Fallback Time | 1–5 min | 1 min | 1 min | ±1 cc |
| Ventricular Blanking | 30–60 ms | 10 ms | 40 ms | — |
| Activity Threshold | V-Low, Low, Med-Low, Med, Med-High, High, V-High | — | Med | — |
| Reaction Time | 10–50 s | 10 s | 30 s | ±3 s |
| Response Factor | 1–16 | 1 | 8 | — |
| Recovery Time | 2–16 min | 1 min | 5 min | ±30 s |

**Table 19-2 — Measured parameters** [PM Table 8]

| Parameter | Increment | Tolerance |
|---|---|---|
| P and R wave measurements | 0.1 mV | ±5 %, referred to the connector |
| Lead impedance | 50 Ω | 100 to 500 Ω: the greater of ±100 Ω and ±30 %; 500 to 2,000 Ω: ±25 %; 2,000 to 2,500 Ω: ±30 % |
| Battery voltage | 0.01 V | ±2 % |

**Table 19-3 — Parameters by Mode** [PM Table 6]. Transcribed on 2026-11-22 from an image of the source's table supplied by the user; the number of Modes marked in each row agrees with the count recovered from the PDF's text. X: the parameter is meaningful in the Mode.

| Parameter | AAT | VVT | AOO | AAI | VOO | VVI | VDD | DOO | DDI | DDD | AOOR | AAIR | VOOR | VVIR | VDDR | DOOR | DDIR | DDDR |
|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|---|
| Lower Rate Limit | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X |
| Upper Rate Limit | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X | X |
| Maximum Sensor Rate |  |  |  |  |  |  |  |  |  |  | X | X | X | X | X | X | X | X |
| Fixed AV Delay |  |  |  |  |  |  | X | X | X | X |  |  |  |  | X | X | X | X |
| Dynamic AV Delay |  |  |  |  |  |  | X |  |  | X |  |  |  |  | X |  |  | X |
| Sensed AV Delay Offset |  |  |  |  |  |  |  |  |  | X |  |  |  |  |  |  |  | X |
| Atrial Amplitude | X |  | X | X |  |  |  | X | X | X | X | X |  |  |  | X | X | X |
| Ventricular Amplitude |  | X |  |  | X | X | X | X | X | X |  |  | X | X | X | X | X | X |
| Atrial Pulse Width | X |  | X | X |  |  |  | X | X | X | X | X |  |  |  | X | X | X |
| Ventricular Pulse Width |  | X |  |  | X | X | X | X | X | X |  |  | X | X | X | X | X | X |
| Atrial Sensitivity | X |  |  | X |  |  |  |  | X | X |  | X |  |  |  |  | X | X |
| Ventricular Sensitivity |  | X |  |  |  | X | X |  | X | X |  |  |  | X | X |  | X | X |
| VRP |  | X |  |  |  | X | X |  | X | X |  |  |  | X | X |  | X | X |
| ARP | X |  |  | X |  |  |  |  | X | X |  | X |  |  |  |  | X | X |
| PVARP | X |  |  | X |  |  |  |  | X | X |  | X |  |  |  |  | X | X |
| PVARP Extension |  |  |  |  |  |  | X |  |  | X |  |  |  |  | X |  |  | X |
| Hysteresis |  |  |  | X |  | X |  |  |  | X |  | X |  | X |  |  |  | X |
| Rate Smoothing |  |  |  | X |  | X | X |  |  | X |  | X |  | X | X |  |  | X |
| ATR Duration |  |  |  |  |  |  | X |  |  | X |  |  |  |  | X |  |  | X |
| ATR Fallback Mode |  |  |  |  |  |  | X |  |  | X |  |  |  |  | X |  |  | X |
| ATR Fallback Time |  |  |  |  |  |  | X |  |  | X |  |  |  |  | X |  |  | X |
| Activity Threshold |  |  |  |  |  |  |  |  |  |  | X | X | X | X | X | X | X | X |
| Reaction Time |  |  |  |  |  |  |  |  |  |  | X | X | X | X | X | X | X | X |
| Response Factor |  |  |  |  |  |  |  |  |  |  | X | X | X | X | X | X | X | X |
| Recovery Time |  |  |  |  |  |  |  |  |  |  | X | X | X | X | X | X | X | X |

**Table 19-4 — Where the text governs** (decided by the user, 2026-11-22: where [PM Table 6] and the text disagree, the text governs). Table 19-3 is the source's table as printed; the allocation in force is Table 19-3 with these four rows replaced.

| Parameter | Text | Modes in force | Change from the source's table |
|---|---|---|---|
| Upper Rate Limit | [PM §5.2]: the maximum rate at which ventricular pacing tracks atrial Senses | VDD, DDD, VDDR, DDDR | Removed from the 14 Modes that do not track |
| Atrial Sensitivity | [PM §3.4.4]: the sensing threshold of the atrial sense channel | AAT, AAI, VDD, DDI, DDD, AAIR, VDDR, DDIR, DDDR | VDD and VDDR added |
| ARP | [PM §5.4.2]: for single-chamber atrial Modes | AAT, AAI, AAIR | DDI, DDD, DDIR and DDDR removed |
| PVARP | [PM §5.4.3]: in Modes with ventricular pacing and atrial sensing | VDD, DDI, DDD, VDDR, DDIR, DDDR | VDD and VDDR added; AAT, AAI and AAIR removed |

The Upper Rate Limit stays a stored, programmable value in every Mode, since ATR detection and the AT Marker compare atrial intervals with it (PM14.1, PM18.5); the row says where it limits pacing.

**Table 19-5 — Interactive limits** (a choice of this spec, decided 2026-11-22; the source lists none). A parameter set is accepted only if every limit holds for the parameters its Mode uses.

| Limit | Reason |
|---|---|
| LRL ≤ URL | The tracking ceiling is not below the floor |
| LRL ≤ MSR | The sensor ceiling is not below the floor |
| HRL ≤ LRL, where hysteresis is on | Hysteresis only lengthens the escape interval |
| Minimum Dynamic AV Delay ≤ Fixed AV Delay | The dynamic range is not empty |
| VRP < URL interval; ARP < URL interval | A Chamber is not refractory for a whole cycle at the highest rate |
| PVARP + Fixed AV Delay ≤ URL interval | The atrium can be sensed and tracked up to the URL |
| Fixed AV Delay + Sensed AV Delay Offset ≥ 30 ms | The shortened AV delay stays at or above the smallest AV delay the source allows |

### Part G — DCM and telemetry

### Requirement 20: The DCM's commands and the DCM itself (Stage E3 for the commands; the rest recorded only) [S]

**User Story:** As a test engineer, I want the commands the PG must answer separated from the programmer's user interface, so that the PG can be tested without building a programmer.

#### Acceptance Criteria

1. THE PG SHALL accept from the link, and the DCM_Stub SHALL be able to send, each of these commands: interrogate the parameters and the stored data; program the permanent parameters; start and end temporary pacing; Pace-Now; set the clock; reset the histograms; start and stop electrogram and Marker transmission; start and stop a threshold test; measure P and R waves, lead impedance, and battery status [PM §2.2.2, §3.2.3, §3.6, §4].
2. THE PG SHALL detect the loss of the link within a stated bound, since the loss ends temporary pacing (PM6.4) and a threshold test (PM17.4). The link is polled periodically (PM29.4) and is lost when PM-PAR-11 successive polls go unanswered, so the bound is PM-PAR-11 poll periods plus the polling Task's response time; the source gives none.
3. THE PG SHALL identify itself by model and serial number in answer to an interrogation, so that a DCM can tell that it has approached a different device [PM §3.2.2].
4. This document SHALL record, and nothing in this spec implements (PM-NG-02): the DCM's eight languages [PM §3.2.1]; its windows, inputs, parameter display, and indications of communication, of loss of telemetry by range or by noise, and of a different device [PM §3.2.2]; its About, Set Clock, New Patient and Quit functions [PM §3.2.3]; its ten printed reports and their header [PM §3.2.4]; its real-time display and strip-chart printing of up to three traces with Markers, electrogram selection, gain and filtering [PM §3.2.5, §4.7.1]; the display of only the trending durations that apply to the Mode [PM §4.6.3]; off-line sensor trending analysis [PM §4.6.4]; the telemetry medium and its 0 to 5 cm range [PM §3.2.6]; and the indication of DCM malfunctions and printer errors [PM §4.9].

### Part H — Inherited from rsk

### Requirement 21: The Application conforms to the Profile (Stage E1) [I]

**User Story:** As a verification engineer, I want the PG to be an ordinary rsk Application, so that every rsk guarantee and every rsk tool applies to it unchanged.

#### Acceptance Criteria

1. THE Application SHALL be declared by one App_Declaration naming its Partitions, Tasks, Resources, Endpoints, peripherals and Target (R21), and SHALL be accepted by the Generator with no diagnostic (R22).
2. THE Application SHALL satisfy every active Profile restriction PR-01 to PR-36 (R2.1), and SHALL be accepted by the Profile_Lint, the Link_Checker and the Config_Checker (R3, R26, R37).
3. THE Application's Kernel_Timing_Parameters and WCET_Records SHALL be measured on the Application's own build on the HIL_Rig (R13, R33, R34); the records of the P1 demo SHALL NOT be reused.
4. THE Analyzer SHALL report PASS for the Application (R29, R30) before any timing claim of Parts B to E is made, with every Task's Budget at least its measured WCET with the margin (R28.4).
5. THE Application's Partition crates SHALL compile under `#![forbid(unsafe_code)]` (PR-20) and SHALL allocate nothing after Init (NG-06).
6. THE Application SHALL declare a maximum continuous operating duration (R9.5, R22.6) and SHALL state what the PG does when it is reached (PM27.4).

### Requirement 22: Partitions and criticality (Stage E1; PM22.3 in E3) [I]

**User Story:** As a safety engineer, I want therapy separated from diagnostics and telemetry, so that a fault in a report cannot stop a pace.

#### Acceptance Criteria

1. THE Application SHALL place the Tasks that decide and deliver Paces in the Therapy_Partition, at the highest Criticality_Level it declares, and SHALL place telemetry, history and diagnostics in one or more Partitions of lower Criticality_Level (R15).
2. THE Therapy_Partition SHALL own the pace outputs and the sense inputs, and no other Partition SHALL be able to write them (R16, R19).
3. THE Application SHALL carry every programmed parameter set from the telemetry Partition to the Therapy_Partition over an Endpoint with a declared receiver-side validation function (R42.4), and that function SHALL accept exactly the sets PM19.3 accepts.
4. THE Application SHALL carry Markers, histogram updates and measured data from the Therapy_Partition to lower Partitions over Endpoints only (PR-30), and a full buffer on such an Endpoint SHALL NOT delay or change a Pace (R40.4, R40.5).
5. THE Application SHALL give each lower Partition Budgets and an Overrun_Response under which the Analyzer's result for the Therapy_Partition's Tasks does not depend on that Partition's behaviour (R18).
6. THE Application SHALL declare, for each sense input, an MIT and an MIT policy (R10), so that a storm of sense edges cannot take more Kernel time than the analysis counts (R19); the relation of the MIT to the blanking and refractory periods is a design decision constrained by PM4.5 and PM13.1. Sense edges are attributed to the Therapy_Partition, and MIT_Violations past a Partition's threshold bring a Health_Monitor response on it (R10, R19): the design shows ("Sense intake, MIT and noise") that edges arriving while the line is masked are latched and not MIT_Violations, so a noisy sense input ends in the asynchronous pacing of PM13.1 and not in a restart; the HIL_Rig's storm scenario SHALL demonstrate it before Stage E4.

### Requirement 23: Fault responses that keep pacing (Stage E3) [I, S]

**User Story:** As a safety engineer, I want every fault response of the Kernel mapped to a pacing outcome, so that no fault leaves the patient without the specified back-up.

#### Acceptance Criteria

1. THE Application SHALL declare for the Therapy_Partition a fault response, an Overrun_Response and a deadline-miss response (R14.5, R15.7), and SHALL state for each the pacing behaviour that follows.
2. WHEN the Therapy_Partition is restarted by the Health_Monitor, THE PG SHALL resume pacing with the Power-On Reset parameters of PM7.1 within a stated bound, measured on the HIL_Rig.
3. THE Application SHALL declare RESTART_PARTITION as the Therapy_Partition's fault response, Overrun_Response and deadline-miss response, so that every fault confined to that Partition ends in PM23.2 and not in the safe state (decided 2026-11-22).
4. WHEN a lower Partition faults, overruns or is stopped, THE PG SHALL continue pacing as programmed (R16, R18), and THE Therapy_Partition SHALL treat the peer-state error of an Endpoint to that Partition (R42.3) as the loss of the link where the Partition is the telemetry Partition (PM6.4).

5. THE Application SHALL NOT leave a pace output asserted in the Kernel's safe state (PM28.4), and this document SHALL record that the experiment has no pacing in the safe state: rsk's safe state stops every Job, the board has no hardware back-up pacing, and none is simulated. A device for use would need one outside the processor (PM-NG-01).
6. THE HIL_Rig SHALL measure the pause in pacing across a restart of the Therapy_Partition (PM23.2) for each of the three responses of PM23.3.

### Requirement 24: Timing claims by analysis and by measurement (Stage E1) [I, S]

**User Story:** As a timing analyst, I want each tolerance of the source tied to a response-time bound and to a measurement, so that "within ±8 ms" is a result and not a hope.

#### Acceptance Criteria

1. THE Application SHALL state, for each timed requirement of Parts B to D, the chain from the event that starts the interval to the pace output's edge, and the bound on that chain as the sum of the release lag, the response times of its Tasks from the Analyzer's report, and any Endpoint hop (R41.4).
2. THE bound of each chain SHALL be within the tolerance the requirement states (PM-PAR-01 to PM-PAR-05), with the quantization of the interval's representation included.
3. THE HIL_Rig SHALL measure, with the logic analyzer, every pace interval, AV delay, refractory period and pulse width of each test run, and SHALL judge each against its tolerance; a run SHALL be at least PAR-05 long for the timing configuration.
4. THE Application SHALL keep the deadline of each Task on a pacing chain no longer than its period or MIT (the Analyzer's input condition).

### Requirement 25: Proof of the therapy logic (Stage E2) [I]

**User Story:** As a verification engineer, I want the pacing decisions proven against this document, so that the experiment tests rsk's proof path and not only its scheduler.

#### Acceptance Criteria

1. THE Application SHALL keep the therapy decision (given the Mode, the parameters, the timing state and an event, which Paces are scheduled or cancelled and which periods start) in functions without side effects, separate from the Tasks that call them.
2. THE App_Prover SHALL prove absence of panics, overflow and out-of-bounds access in every Task body and validation function of the Therapy_Partition (R46.3).
3. THE Application SHALL state, as contracts the App_Prover checks, at least: no Pace of a Chamber earlier than the URL interval or the refractory rule allows (PM10.1, PM12); a Pace scheduled no later than the LRL or hysteresis interval after the event that starts it (PM9.1, PM16.1); the States mutually exclusive (PM6.1); the validation function accepts exactly Table 19-1 (PM19.1).
4. THE properties of PM25.3 that span more than one Job (an interval from one event to the next Pace) SHALL be stated as invariants of the therapy state that one step preserves: the state holds the instant of each Chamber's pending Pace, and the invariant bounds that instant by the last event plus the escape interval in force. THE App_Prover SHALL prove them; no further tool is introduced (decided 2026-11-22). That the Pace then occurs at that instant is PM24's.

### Part I — Platform

### Requirement 26: Target and rig (Stage E1) [P]

**User Story:** As a developer, I want the hardware the experiment runs on stated, so that platform limits are requirements and not surprises.

#### Acceptance Criteria

1. THE Application SHALL run on the nRF52840-DK as rsk's Target `nrf52840` describes it: a 64 MHz core, a 16 MHz Kernel time base (PM-PAR-08), 3 priority bits (PM-PAR-09), 8 MPU regions (PAR-07), and the Kernel-owned TIMER0, CLOCK, WDT and PPI (PR-29).
2. THE Application SHALL also build for the QEMU Target `qemu_mps2_an386` with the sense inputs driven by a scripted source, so that the therapy logic runs in the verification job without the rig.
3. THE Heart_Simulator SHALL run on the HIL_Rig's second board and SHALL be an rsk Application of its own, in the manner of `apps/hil-stimulus`.
4. THE Application SHALL use at most the 7 Task Priority levels the Target provides (PM-PAR-09), and at most as many Tasks as the Target table lists dispatch vectors (Requirement 32).
5. THE Application SHALL represent every interval as an integer number of time-base ticks and every rate by its interval, rounded so that PM-PAR-01 holds with the rounding included (R29.8).

### Requirement 27: Timed pacing on a Kernel with periodic and sporadic Tasks only (Stage E1) [P, I]

**User Story:** As a developer, I want a stated way to pace "one interval after the last event", so that the central timing pattern of a pacemaker has a home in the Profile.

#### Acceptance Criteria

1. THE Application SHALL realize each escape interval (the LRL, hysteresis, AV and URL intervals and the refractory periods, each started by an event and not at a fixed phase) with the release kinds the Profile has: a Periodic_Task at O + k·T (R9.1) or a Sporadic_Task released by a Release_Source (R10). The Profile has no one-shot delay that a Job can arm.
2. THE Application SHALL time each Chamber's pending Pace with a compare of one hardware timer the Therapy_Partition owns (TIMER3, design PD-01), whose compare interrupt is the Release_Source of the pacing Task; a Job arms the compare for the instant of the next Pace and re-arms or withdraws it when a Sense inhibits or reschedules the Pace. THE Application SHALL NOT poll for escape intervals with a Periodic_Task, and rsk gains no Kernel-served one-shot event for this experiment (decided 2026-11-22; one timer for both Chambers since the design).
3. THE Application SHALL decide refractory and blanking periods without a timer, by comparing the time of a Sense with the time of the event that started the period, and SHALL NOT read the time base to busy-wait for an instant (PR-16, R35).
4. WHEN the declared operating duration ends, the Kernel enters the safe state (R9.5). THE Application SHALL declare a duration that its tests never reach, and this document SHALL record that continuous operation over a service life is outside the experiment (PM-NG-07).

5. THE Application SHALL declare for each pacing timer's Release_Source an MIT no longer than the shortest interval between two compare events of that timer that the programmable values allow, since the Kernel masks a source until its MIT has passed and drops events that arrive before (R10); the design SHALL state that interval for each timer and show that only one compare of each Pace raises the interrupt: the compare that ends the pulse, not the compare that starts it, so that the pacing Job never runs while its pulse is high (design PD-01, PD-02; amended 2026-11-22).
6. THE interval error of this scheme (the release lag of the timer's intake and the pacing Task's response time) SHALL be shown within PM-PAR-01 and PM-PAR-03 by PM24, and whether the scheme was awkward enough to justify a Kernel-served one-shot event SHALL be recorded as a finding (PM31.2).

### Requirement 28: Pace output and pulse width (Stage E1) [P, S]

**User Story:** As a developer, I want the pulse edges timed by hardware where software cannot meet the tolerance, so that pulse width is not at the mercy of scheduling.

#### Acceptance Criteria

1. THE Application SHALL drive each pace output as a digital line whose high time is the programmed pulse width, and SHALL report the programmed amplitude with each Pace as data (PM2.2).
2. THE Application SHALL meet PM-PAR-05 for the pulse width by producing at least the falling edge of every pulse in hardware: a compare of the Chamber's timer acts on the pace pin through a GPIOTE task over a fixed PPI channel (Requirement 33), so that the width is exact to the timer's tick. With a release lag of the order of PM-PAR-10 a Job cannot time a 0.05 ms pulse or a ±0.02 ms tolerance by itself (decided 2026-11-22).
3. THE Therapy_Partition SHALL own GPIOTE, P0, TIMER1, TIMER2 and TIMER3; the PPI stays Kernel-owned (PR-29) and PWM is not used (it is not assignable to a Partition in rsk v1, R17 scope note).
4. THE Application SHALL NOT leave a pace output high when its Partition is stopped, restarted or the Kernel enters the safe state (PM23.5).

### Requirement 29: Sense, magnet, battery and activity inputs; the link (Stage E1 for sense; E3 for magnet, battery, link; E4 for activity) [P]

**User Story:** As a developer, I want each analog or wireless element of the source replaced by something the board has, so that the logic can be exercised end to end.

#### Acceptance Criteria

1. THE Application SHALL take each Chamber's sense signal as a digital edge from the Heart_Simulator. Each Chamber's edge SHALL be the Release_Source of that Chamber's own Sporadic_Task (R10, R19) through a timer in counter mode (design PD-03), not through GPIOTE's one interrupt line, so that a Sense in one Chamber does not hold back the other under rsk's masking of a source for one MIT (DD-06); the edge's instant SHALL be a hardware capture. SAADC is not assignable to a Partition in rsk v1, so the Application measures no amplitude: sensitivity is a stored value, and a "sub-threshold" beat is one the Heart_Simulator does not send.
2. THE Application SHALL take the magnet as a digital input and the battery level as a value the DCM_Stub sets over the link, and SHALL NOT claim a voltage measurement (PM17.5).
3. THE Application SHALL take activity as a value the DCM_Stub supplies over the link; no accelerometer is fitted (decided 2026-11-22).
4. THE Application SHALL carry the DCM link on SPIM1, owned by the telemetry Partition, with the HIL_Rig's third board as a bridge to the host's DCM_Stub; the bridge is test equipment and need not be an rsk Application. A Periodic_Task polls the link with fixed-size frames. UARTE and RADIO are not used, and rsk's mediated EasyDMA classes are not extended for this experiment (decided 2026-11-22).
5. THE HIL_Rig's wiring for the Application (which pins carry sense, pace, magnet and link, and which reach the logic analyzer) SHALL be recorded in `hil-rig.toml` before the first test.

### Requirement 30: Storage and reset (Stage E3) [P, S]

**User Story:** As a developer, I want the source's "device memory" mapped to what the Target keeps, so that nobody assumes data survives a reset.

#### Acceptance Criteria

1. THE Application SHALL hold programmed parameters, implant data, histograms and trends in statically allocated RAM of its Partitions (PR-14). The Target table gives a Partition no non-volatile memory controller.
2. WHEN the PG is reset, THE Application SHALL start from the Power-On Reset parameters (PM7.1) with empty histories; that programmed parameters do not survive a reset is a deviation from a device with non-volatile memory and SHALL be recorded (PM-NG-06, PM-ORQ-09).
3. THE Application SHALL model the POR trip voltage of PM7.2 by the battery-level input: below the trip level no Pace is delivered; at or above it the Power-On Reset State is entered.
4. THE Application's statics SHALL start in their empty state when zeroed, since a Partition's statics are zero at start whatever initial value is declared.

### Requirement 31: Evidence (Stage E1) [P, I]

**User Story:** As a reviewer, I want the experiment's results recorded the way rsk's are, so that they can be compared and trusted to the same degree.

#### Acceptance Criteria

1. THE experiment SHALL keep a traceability table from every acceptance criterion of this document to its design element, its code, and its evidence (a proof, a test on QEMU, or a rig measurement), in the manner of R50.5.
2. THE experiment SHALL record every place where the source could not be met on rsk or on the Target, with the reason: these findings are a result of the experiment.
3. THE experiment SHALL state for each result whether it was obtained from a run and which run, and SHALL NOT report a timing number that no run produced.

### Requirement 32: The number of Tasks (Stage E1 for PM32.1 and PM32.2; the others when an eighth Task is needed) [P, I]

**User Story:** As a developer, I want the Task limit stated with the conditions for raising it, so that the Application is designed to the limit first and the limit is raised on evidence, not in advance.

#### Acceptance Criteria

1. THE Application SHALL be designed to at most 7 Tasks, the number of dispatch vectors rsk's Target table lists for the `nrf52840` (the Generator gives each Task its own vector and rejects a declaration with more Tasks than vectors).
2. Before the design of Stage E1, a read-only audit SHALL report: which interrupt lines of the nRF52840 have no interrupt source, or only a source of a peripheral the Application does not use, and so can serve as dispatch vectors; and every fixed size in the Kernel, the Generator, the Config_Checker and their proofs that the number of Tasks can reach (among them the 8 slots of the dispatch-deferral record, whose exhaustion is a Kernel fault, and the 16 Partition stack floors), with whether each is bounded by the number of Tasks, of Priority levels, or of Partitions.
3. IF a stage's Task set cannot be reduced to the limit without merging work whose Budgets, deadlines or Criticality_Levels differ, THEN the limit MAY be raised by lengthening the Target's dispatch vector list, and the need SHALL be recorded as a finding of the experiment (PM31.2).
4. WHERE the dispatch vector list is lengthened, it SHALL remain strictly ascending (DD-03), every added line SHALL be one the audit of PM32.2 found free, and every fixed size the audit found reachable SHALL be shown sufficient or enlarged with its proof.
5. WHERE the Application has more Tasks than Task Priority levels, so that Tasks share a Priority, the order of equal-Priority Tasks SHALL be the order of DD-03, and it SHALL be demonstrated on the HIL_Rig before any timing claim that depends on it: no configuration measured so far has two Tasks at one Priority.
6. WHERE the limit is raised, the Kernel_Timing_Parameters, the hold, the release lag and the WCET_Records SHALL be measured again on the larger configuration (PM21.3), since each Task adds timed-event slots and the slot scans are the largest share of a Kernel entry; the Kani harness sizes (Requirement 45) and the per-configuration table proofs SHALL be those of the larger configuration.
7. THE Task Priority levels SHALL remain 7: the Target implements 3 priority bits and the Kernel reserves one level (PAR-06, PR-33).

### Requirement 33: Fixed PPI channels in rsk (Stage E1) [P, I]

**User Story:** As a developer, I want a timer compare wired to a pin without a Job in between, so that a pulse's width does not depend on scheduling, while the PPI stays the Kernel's.

#### Acceptance Criteria

1. THE App_Declaration SHALL be able to declare fixed PPI channels, each naming one event of a peripheral, one task of a peripheral, and optionally a second task (the channel's fork).
2. THE Generator SHALL reject a channel whose event's peripheral and tasks' peripherals are not all owned by one Partition, so that a channel is no path between Partitions (PR-29, R16, R19), and a channel that names a Kernel-owned peripheral.
3. THE Kernel SHALL write the declared channels and their enable bits during Init and SHALL NOT write the PPI afterwards; the PPI's register window SHALL remain Kernel-owned and unwritable by every Partition (R16).
4. THE Generated_Config SHALL contain the channels (R24.1), THE boot check SHALL cover them with the configuration checksum (R12.3), and THE Config_Checker SHALL recompute them from the Task_Model and compare them with the binary (R26).
5. THE change SHALL carry the evidence rsk requires of a Kernel change: the contract of the Init write, a read-back on the HIL_Rig of the PPI registers after boot, and an isolation test in which a Partition's write to the PPI window faults.
6. THE Target table of the `nrf52840` SHALL gain TIMER3 as a peripheral a Partition may own, with its register window and interrupt line, after both are checked against the product specification.
7. PM33.1 to PM33.6 are the only changes to rsk that Stage E1 of the experiment requires.

## Assumptions

| ID | Assumption | Affects |
|---|---|---|
| PM-ASM-01 | The Heart_Simulator's edges stand for sensed events after the analog front end: detection thresholds, filtering and blanking of the pulse artefact are outside the Application. | PM4, PM13, PM29.1 |
| PM-ASM-02 | A rate's tolerance of ±8 ms is the tolerance of its interval (Conventions). | PM-PAR-01, PM9 to PM12 |
| PM-ASM-03 | The release lag and the Kernel_Timing_Parameters of the Application's build are of the order of the P1 demo's (PM-PAR-10), far below 8 ms; to be confirmed by measurement (PM21.3). | PM24, PM28.2 |
| PM-ASM-04 | The logic analyzer's sample period is small against 0.02 ms, so that pulse widths can be judged. | PM24.3, PM28.2 |
| PM-ASM-05 | One Kernel time base serves both the therapy timing and the time stamps of Markers and trends; no real-time clock is needed beyond a value the DCM_Stub sets. | PM18.3, PM18.6, PM20.1 |

## Open Questions

Each question is to be resolved, or deferred with a recorded reason, before the design of the stage it affects. All 25 were decided by the user on 2026-11-22; the rows record each decision and the criteria that carry it. A criterion marked "conventional behaviour" takes the usual behaviour of a pacemaker where the source is silent; one marked "a choice of this spec" is this document's own.

| ID | Question | Affects |
|---|---|---|
| PM-ORQ-01 | **Resolved 2026-11-22 (user).** Each Chamber's pending Pace is timed by a Partition-owned hardware timer whose compare interrupt is a Release_Source; refractory and blanking periods are decided from time stamps; no polling and no Kernel one-shot event (PM27.2 to PM27.6). Whether rsk needs a Kernel-served one-shot event stays a finding to report. | PM27, PM9 to PM12, PM24 |
| PM-ORQ-02 | **Resolved 2026-11-22 (user): the text governs.** [PM Table 6] disagrees with the text for the Upper Rate Limit, Atrial Sensitivity, ARP and PVARP; Table 19-4 gives the allocation in force. | PM5.8, PM12.2, PM12.3, PM19.5 |
| PM-ORQ-03 | **Resolved 2026-11-22 (user).** A rate's ±8 ms is the tolerance of its interval. | PM-ASM-02 |
| PM-ORQ-04 | **Resolved 2026-11-22 (user).** Both pulse-width tolerances apply as stated; hardware timing meets both (PM28.2). Only the regulated amplitude set is implemented, nominal 3.5 V (PM3.2). | PM3.2, PM3.3, PM6.5, PM7.1, PM28.2 |
| PM-ORQ-05 | **Resolved 2026-11-22 (user).** Within PM-PAR-01 of the Sense, measured (PM5.4). | PM5.4 |
| PM-ORQ-06 | **Resolved 2026-11-22 (user).** The PG's share is one Pace-Now LRL interval from the command's arrival; the whole bound is measured from the DCM_Stub's send (PM6.6). | PM6.6 |
| PM-ORQ-07 | **Resolved 2026-11-22 (user).** The first accepted programming of permanent parameters (PM7.4). | PM7.4 |
| PM-ORQ-08 | **Resolved 2026-11-22 (user).** Cross-chamber blanking after an atrial Pace, in the Modes that pace the atrium and sense the ventricle (PM4.5). | PM4.5 |
| PM-ORQ-09 | **Resolved 2026-11-22 (user).** No non-volatile storage (PM-NG-06, PM30.2). | PM30, PM18.7 |
| PM-ORQ-10 | **Resolved 2026-11-22 (user).** A tracked Pace that would violate the URL interval is delayed to its end, not dropped (PM10.2). | PM10.2 |
| PM-ORQ-11 | **Resolved 2026-11-22 (user).** The maximum is the Fixed AV Delay and the factor is derived from it and the LRL interval (PM11.4). | PM11.4 |
| PM-ORQ-12 | **Resolved 2026-11-22 (user).** Noise is defined in the Application by PM-PAR-12 (PM13.1). The design shows that a masked line's edges are not MIT_Violations (PM22.6); the rig's storm scenario demonstrates it. | PM13, PM22.6 |
| PM-ORQ-13 | **Resolved 2026-11-22 (user).** An up-down counter with onset at PM-PAR-13 and cessation at zero; the cessation sentence is read as "slower" (PM14.1, PM14.2). | PM14.1 to PM14.3 |
| PM-ORQ-14 | **Resolved 2026-11-22 (user).** A linear map from activity above the threshold, scaled by the Response Factor (PM15.4); the slew is PM15.5 and PM15.6. | PM15 |
| PM-ORQ-15 | **Resolved 2026-11-22 (user).** A Sense starts the HRL interval, a Pace returns to the LRL interval; the source's condition is the limit HRL ≤ LRL (PM16.1, PM16.2). | PM16.1, PM16.2 |
| PM-ORQ-16 | **Resolved 2026-11-22 (user).** Two parameters, Up and Down, by the rule that the text governs (PM16.3). | PM16.3 |
| PM-ORQ-17 | **Resolved 2026-11-22 (user).** The input is the level; the Mode mapping at ERT is PM17.7; ERP and post-ERT are one level. | PM17.6, PM17.7, PM8.2 |
| PM-ORQ-18 | **Resolved 2026-11-22 (user).** Table 19-5. | PM19.3, PM22.3 |
| PM-ORQ-19 | **Resolved 2026-11-22 (user).** The link is lost after PM-PAR-11 unanswered polls (PM20.2). | PM20.2, PM6.4 |
| PM-ORQ-20 | **Resolved 2026-11-22 (user).** The Therapy_Partition's three responses are RESTART_PARTITION, the pause is measured, and the experiment has no pacing in the Kernel's safe state, which is recorded (PM23.3, PM23.5, PM23.6). | PM23, PM28.4 |
| PM-ORQ-21 | **Resolved 2026-11-22 (user).** One-step invariants over a therapy state that holds the pending Pace instants, proven by the App_Prover (PM25.4). | PM25.4 |
| PM-ORQ-22 | **Resolved 2026-11-22.** The 7 dispatch vectors are a choice of the Target table, not a hardware limit; the 7 Task Priority levels are the hardware's. The Application is designed to 7 Tasks, and Requirement 32 states the audit to make first and the conditions for raising the limit. Whether a pacemaker's Task set fits is answered stage by stage (PM32.3). | PM26.4, PM32 |
| PM-ORQ-23 | **Resolved 2026-11-22 (user).** The falling edge is hardware-timed through a fixed PPI channel, an rsk feature this experiment adds (PM28.2, Requirement 33). | PM28.2, PM28.3, PM33 |
| PM-ORQ-24 | **Resolved 2026-11-22 (user).** Activity is a simulated value (PM29.3). | PM29.3, PM15 |
| PM-ORQ-25 | **Resolved 2026-11-22 (user).** SPIM1 to the rig's third board as a bridge (PM29.4). | PM29.4, PM20 |

## Sources

- **PM** *PACEMAKER System Specification*, Boston Scientific, January 3, 2007. `docs/pacemaker/PACEMAKER.pdf`. Sections 1 to 5, Appendices A and B, Tables 1 to 8. Read in full for this draft from the PDF's extracted text; Table 6, whose layout the extraction lost, was transcribed from an image of the page.
- **rsk** `.kiro/specs/rsk-ravenscar-profile/requirements.md`, Draft 0.3: Requirements 9, 10, 12 to 19, 21, 22, 26, 28, 29, 33 to 35, 37, 40 to 42, 46, 50; restrictions PR-14, PR-16, PR-20, PR-29, PR-30, PR-33; non-goal NG-06; parameters PAR-05 to PAR-07.
- **Target** `crates/rsk-gen/src/target.rs` (the `nrf52840` and `qemu_mps2_an386` tables: clocks, dispatch vectors, peripherals and which are Kernel-owned or EasyDMA-capable) and the scope note of R17 (SPIM is the only mediated EasyDMA class), as of 2026-11-22.
- **Rig** `hil-rig.toml` and `docs/hil/kernel_timing_nrf52840_stress.json` (the release lag quoted in PM-PAR-10).
