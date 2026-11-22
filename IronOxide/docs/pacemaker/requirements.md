# Requirements Document

**PACEMAKER on rsk: an experimental Application**

Status: Draft 0.1 (requirements phase; nothing implemented; no Task set fixed). Spec: `pacemaker`. Written 2026-11-22 from the source document, the rsk requirements (`.kiro/specs/rsk-ravenscar-profile/requirements.md`, Draft 0.3) and the rsk tree at that date.

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
- Rates are in pulses per minute (ppm), which the source also writes as beats per minute. An interval of a rate r is 60,000 / r ms. The source gives the tolerance of a rate as a time (±8 ms): it is taken as the tolerance of the rate's interval (PM-ORQ-03).
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
| PM-PAR-05 | Tolerance of a pulse width | 0.2 ms; ±0.02 ms for the Pace-Now and POR pulse | [PM App. A], [PM §3.6.3], [PM §3.6.5]; PM-ORQ-04 |
| PM-PAR-06 | Shortest interval between two paces of a chamber | 342.8 ms (175 ppm) | [PM App. A], the highest programmable rate |
| PM-PAR-07 | Longest interval between two paces of a chamber with hysteresis off | 2,000 ms (30 ppm) | [PM App. A], the lowest programmable rate |
| PM-PAR-08 | Kernel time-base tick of the Target | 62.5 ns (16 MHz) | rsk Target table (`nrf52840`) |
| PM-PAR-09 | Task Priority levels available | 7 | PAR-06, PR-33 |
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
| I. Platform | 26–31 |

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
- **PM-NG-06** Persistence of programmed parameters and stored data across a reset, unless PM-ORQ-09 is resolved in its favour.
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
2. THE PG SHALL keep the atrial and the ventricular pulse amplitude independently programmable, and the atrial and the ventricular pulse width independently programmable [PM §3.4.1, §3.4.2].
3. THE PG SHALL deliver each pulse width within PM-PAR-05 of its programmed value [PM App. A].
4. THE PG SHALL support an atrial and a ventricular bipolar pace and sense lead, and SHALL operate normally with lead impedances from 100 to 2,500 Ω; amplitudes and widths are specified into 750 Ω [PM §3.3]. On the Target this criterion is recorded and not tested (PM-NG-03).

### Requirement 4: Sensing (Stage E1) [S]

**User Story:** As a therapy engineer, I want intrinsic events detected per Chamber and rate decided beat by beat, so that the PG responds to the heart's own rhythm.

#### Acceptance Criteria

1. THE PG SHALL detect Senses in each Chamber it senses in the programmed Mode [PM §3.4.3].
2. THE PG SHALL base every rate decision on measured cardiac cycle lengths of the sensed rhythm, evaluated interval by interval [PM §3.4.3].
3. THE PG SHALL keep the atrial and the ventricular sensing threshold independently adjustable by the DCM [PM §3.4.4].
4. WHILE a refractory period of a Chamber is in progress, THE PG SHALL treat a Sense in that Chamber as a Refractory sense: it SHALL neither inhibit nor trigger a Pace (Requirement 12).
5. THE PG SHALL ignore ventricular Senses for the programmed Ventricular Blanking interval of [PM App. A]; the source gives the parameter and no description of its start (PM-ORQ-08).

### Part C — Modes and states

### Requirement 5: Operating modes (Stage E1 for the single-chamber modes; E2 dual-chamber; E4 rate-adaptive) [S]

**User Story:** As a physician, I want the standard bradycardia modes, so that pacing fits the patient's conduction.

#### Acceptance Criteria

1. THE PG SHALL be programmable to each of the Modes Off, DDDR, VDDR, DDIR, DOOR, VOOR, AOOR, VVIR, AAIR, DDD, VDD, DDI, DOO, VOO, AOO, VVI, AAI, VVT and AAT [PM §3.5].
2. THE PG SHALL offer the Modes OVO, OAO, ODO and OOO in temporary operation only [PM §3.5].
3. WHILE the response to sensing is O, THE PG SHALL deliver Paces without regard to Senses [PM §3.5.1].
4. WHILE the response to sensing is T, WHEN a non-refractory Sense occurs in a Chamber, THE PG SHALL deliver a Pace to that Chamber immediately [PM §3.5.2]. The bound of "immediately" is not given (PM-ORQ-05).
5. WHILE the response to sensing is I, WHEN a non-refractory Sense occurs in a Chamber, THE PG SHALL cancel the Pace pending for that Chamber [PM §3.5.3].
6. WHILE the response to sensing is D, WHEN a non-refractory atrial Sense occurs, THE PG SHALL deliver a ventricular Pace one AV delay after it unless a ventricular Sense occurs first [PM §3.5.4].
7. THE PG SHALL pace only the Chambers, and sense only the Chambers, that the Mode's first and second letters name [PM Table 2].
8. THE PG SHALL use, in each Mode, only the parameters that [PM Table 6] marks for that Mode (Table 19-3).

### Requirement 6: States (Stage E3) [S]

**User Story:** As a physician, I want permanent, temporary and emergency operation kept apart, so that a test never changes the therapy the patient leaves with.

#### Acceptance Criteria

1. THE PG SHALL be in exactly one of the States Permanent, Temporary, Pace-Now, Magnet, and Power-On Reset at every instant [PM §3.6].
2. WHILE in the Permanent State, THE PG SHALL pace with the permanent parameters [PM §3.6.1].
3. WHILE in the Temporary State, THE PG SHALL pace with the temporary parameters and SHALL leave the permanent parameters unchanged [PM §3.6.2].
4. WHEN the link to the DCM is broken, a Pace-Now command is received, or the DCM commands the end of temporary pacing, THE PG SHALL leave the Temporary State [PM §3.6.2].
5. WHEN the DCM commands Pace-Now, THE PG SHALL enter the Pace-Now State with the Mode VVI, an LRL of 65 ppm, a ventricular amplitude of 5.0 V ±0.5 V, a pulse width of 1.00 ms ±0.02 ms, a VRP of 320 ms ±8 ms, and a ventricular sensitivity of 1.5 mV [PM §3.6.3].
6. WHEN the DCM commands Pace-Now, THE PG SHALL deliver the first Pace-Now pulse within two cardiac cycles plus 500 ms of the last user action that the command needs [PM §3.6.3]. The Application's share of this time is from the command's arrival at the PG (PM-ORQ-06).
7. WHILE in the Pace-Now State, THE PG SHALL continue Pace-Now pacing until the DCM changes the Mode [PM §3.6.3].

### Requirement 7: Power-On Reset (Stage E3) [S, P]

**User Story:** As a safety engineer, I want a known mode and parameters after a reset, so that the PG paces predictably whatever preceded the reset.

#### Acceptance Criteria

1. WHEN the PG starts after a reset, THE PG SHALL enter the Power-On Reset State with the Mode VVI, an LRL of 65 ppm, a ventricular amplitude of 5.0 V ±0.5 V, a pulse width of 0.5 ms ±0.02 ms, a VRP of 320 ms ±8 ms, and a ventricular sensitivity of 1.5 mV [PM §3.6.5].
2. WHILE the battery voltage is below the POR trip voltage, THE PG SHALL keep all functions disabled [PM §3.6.5]. On the Target the battery-level input stands for the voltage (PM30.3).
3. THE Application SHALL reach its first Pace from a reset within a stated bound that includes the Kernel's boot checks (R12), and the bound SHALL be measured on the HIL_Rig.
4. THE source does not say how the PG leaves the Power-On Reset State; the event is PM-ORQ-07.

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
2. WHILE the Mode tracks atrial Senses, THE PG SHALL track them at rates up to the URL and no higher [PM §5.2]. What the PG does with an atrial Sense whose tracked Pace would fall inside the URL interval (delay it or drop it) is not stated (PM-ORQ-10).

### Requirement 11: AV delay (Stage E2) [S]

**User Story:** As a therapy engineer, I want the ventricle paced a programmed time after the atrium, so that the Chambers stay synchronized.

#### Acceptance Criteria

1. WHILE the Mode tracks atrial Senses and the sensed atrial rate is between the LRL and the URL, WHEN no ventricular Sense occurs within the AV delay after an atrial event, THE PG SHALL deliver a ventricular Pace at the end of the AV delay, within PM-PAR-01 [PM §5.3].
2. THE PG SHALL use the paced AV delay after an atrial Pace and the sensed AV delay after an atrial Sense [PM §5.3.1, §5.3.2].
3. WHERE the AV delay is fixed, THE PG SHALL use the programmed Fixed AV Delay [PM §5.3].
4. WHERE the AV delay is dynamic, THE PG SHALL compute the AV delay of each cardiac cycle as the previous cardiac cycle's length multiplied by a stored factor, limited to the range from the programmed minimum to the programmed maximum paced AV delay [PM §5.3.3]. The factor's value, and where the maximum is programmed, are not given (PM-ORQ-11).
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

1. WHILE continuous noise is present on a sense input, THE PG SHALL pace asynchronously [PM §5.5]. What counts as continuous noise is not defined (PM-ORQ-12).
2. THE PG SHALL annotate the condition with the Marker TN [PM Table 5].

### Requirement 14: Atrial Tachycardia Response (Stage E4) [S]

**User Story:** As a physician, I want tracking suspended during an atrial tachycardia, so that the ventricle is not paced at a high rate for long.

#### Acceptance Criteria

1. WHERE ATR is enabled, WHEN the intervals between atrial Senses are predominantly shorter than the URL interval, THE PG SHALL detect the onset of an atrial tachycardia [PM §5.6, §5.6.1]. "Predominantly" is not quantified (PM-ORQ-13).
2. THE PG SHALL detect the end of the tachycardia by the complementary condition; the source's sentence for it repeats the onset condition (PM-ORQ-13).
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
4. THE PG SHALL give a larger rate change for the same activity at a higher Response Factor, the largest at 16 and the smallest at 1 [PM §5.7.3]. The mapping from activity to rate is not given (PM-ORQ-14).
5. THE PG SHALL raise the rate so that sustained activity takes it from the LRL to the MSR in the programmed Reaction Time [PM §5.7.4].
6. WHEN activity falls below the Activity Threshold, THE PG SHALL lower the rate so that it falls from the MSR to the LRL in the programmed Recovery Time [PM §5.7.5].

### Requirement 16: Hysteresis and rate smoothing (Stage E4) [S]

**User Story:** As a physician, I want the PG to wait a little longer after an intrinsic beat and to limit sudden rate changes, so that the heart's own rhythm is favoured and rate changes are gentle.

#### Acceptance Criteria

1. WHERE hysteresis is enabled and the Mode inhibits or tracks, WHEN a single non-refractory Sense occurs in the Chamber that starts the LRL interval (the atrium in AAI, the ventricle in a Mode with ventricular pacing), THE PG SHALL use the Hysteresis Rate Limit interval, which may be longer than the LRL interval, as the interval to the next Pace [PM §5.8].
2. THE source's condition that "the current pacing rate must be faster than the HRL" SHALL be met by PM16.1; its exact reading is PM-ORQ-15.
3. WHERE Rate Smoothing is enabled, THE PG SHALL limit the change of the cardiac cycle interval from one cycle to the next to the programmed percentage of the previous interval, with one percentage for increases of rate (Rate Smoothing Up) and one for decreases (Rate Smoothing Down) [PM §5.9]. [PM App. A] lists one Rate Smoothing parameter (PM-ORQ-16).

### Part E — Diagnostics and stored data

### Requirement 17: Measured data, tests and battery status (Stage E5; PM17.6 and PM17.7 in E3) [S]

**User Story:** As a clinician, I want the PG's measurements and battery level, so that leads and battery can be judged at follow-up.

#### Acceptance Criteria

1. WHEN the DCM commands it, THE PG SHALL measure P and R wave amplitudes, with the increment and tolerance of Table 19-2 [PM §4.2, App. B].
2. WHEN the DCM commands it, THE PG SHALL measure lead impedance, in the Temporary State, at a default of 5.5 V [PM §4.3].
3. THE PG SHALL supply the battery voltage, the battery status level, and the date of the last interrogation [PM §4.4].
4. THE PG SHALL support the DCM's automatic pacing threshold test in AAI, VVI and DDD: a start value of amplitude or pulse width, a step down of one setting about every fourth cardiac cycle, back-up pacing at a programmable rate (DDD or AAI for an atrial test, DDD or VVI for a ventricular test), and an end on a Stop command or the loss of the link [PM §4.5].
5. On the Target the measurements of PM17.1 to PM17.3 SHALL return values supplied by the Heart_Simulator or fixed by the test, marked as simulated (PM-NG-03).
6. THE PG SHALL derive the battery status level BOL, ERN, ERT or ERP from the battery input [PM §4.4, Table 3].
7. WHILE the battery status level is ERT or ERP, THE PG SHALL offer only non-rate-adaptive single-chamber Modes and SHALL disable temporary programming, automatic threshold testing, measured data, electrograms and Markers; WHILE it is ERP, the pacing rate MAY decrease as the voltage decreases [PM Table 3]. The Mode the PG takes when a level change removes the programmed Mode is not stated (PM-ORQ-17).

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
3. IF a programmed value is outside Table 19-1, or a set of values violates an interactive limit, THEN THE PG SHALL reject the whole set and keep the parameters in force (PM22.3). The source requires that interactive limit errors be indicated and lists no limits [PM §4.9.2]; they are PM-ORQ-18.
4. THE PG SHALL report measured values with the increments and tolerances of Table 19-2.
5. THE PG SHALL apply Table 19-3 as the allocation of parameters to Modes (PM5.8).

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

**Table 19-3 — Parameters by Mode** [PM Table 6]. The source's table has 18 Mode columns (AAT, VVT, AOO, AAI, VOO, VVI, VDD, DOO, DDI, DDD, and the eight rate-adaptive Modes). Its cell positions were not recovered when the PDF's text was extracted, only the number of Modes each row marks. The rows below whose Mode set follows from the Mode letters and matches the source's count are given; the others carry the count only and SHALL be completed by reading the figure in the source (PM-ORQ-02).

| Parameter | Modes marked in the source | Mode set |
|---|---|---|
| Lower Rate Limit, Upper Rate Limit | 18 | every Mode |
| Maximum Sensor Rate; Activity Threshold, Reaction Time, Response Factor, Recovery Time | 8 | the rate-adaptive Modes |
| Fixed AV Delay | 8 | VDD, DOO, DDI, DDD and their rate-adaptive forms |
| Atrial Amplitude, Atrial Pulse Width | 11 | the Modes that pace the atrium |
| Ventricular Amplitude, Ventricular Pulse Width | 13 | the Modes that pace the ventricle |
| Ventricular Sensitivity, VRP | 9 | the Modes that sense the ventricle |
| Dynamic AV Delay | 4 | to be read from the source |
| Sensed AV Delay Offset | 2 | to be read from the source |
| Atrial Sensitivity, ARP, PVARP | 7 each | to be read from the source (nine Modes sense the atrium) |
| PVARP Extension | 4 | to be read from the source |
| Hysteresis | 6 | to be read from the source |
| Rate Smoothing | 8 | to be read from the source |
| ATR Duration, ATR Fallback Mode, ATR Fallback Time | 4 each | to be read from the source |

### Part G — DCM and telemetry

### Requirement 20: The DCM's commands and the DCM itself (Stage E3 for the commands; the rest recorded only) [S]

**User Story:** As a test engineer, I want the commands the PG must answer separated from the programmer's user interface, so that the PG can be tested without building a programmer.

#### Acceptance Criteria

1. THE PG SHALL accept from the link, and the DCM_Stub SHALL be able to send, each of these commands: interrogate the parameters and the stored data; program the permanent parameters; start and end temporary pacing; Pace-Now; set the clock; reset the histograms; start and stop electrogram and Marker transmission; start and stop a threshold test; measure P and R waves, lead impedance, and battery status [PM §2.2.2, §3.2.3, §3.6, §4].
2. THE PG SHALL detect the loss of the link within a stated bound, since the loss ends temporary pacing (PM6.4) and a threshold test (PM17.4). The source gives no bound (PM-ORQ-19).
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
6. THE Application SHALL declare, for each sense input, an MIT and an MIT policy (R10), so that a storm of sense edges cannot take more Kernel time than the analysis counts (R19); the relation of the MIT to the blanking and refractory periods is a design decision constrained by PM4.5 and PM13.1.

### Requirement 23: Fault responses that keep pacing (Stage E3) [I, S]

**User Story:** As a safety engineer, I want every fault response of the Kernel mapped to a pacing outcome, so that no fault leaves the patient without the specified back-up.

#### Acceptance Criteria

1. THE Application SHALL declare for the Therapy_Partition a fault response, an Overrun_Response and a deadline-miss response (R14.5, R15.7), and SHALL state for each the pacing behaviour that follows.
2. WHEN the Therapy_Partition is restarted by the Health_Monitor, THE PG SHALL resume pacing with the Power-On Reset parameters of PM7.1 within a stated bound, measured on the HIL_Rig.
3. THE Application SHALL state what the pace outputs do in the Kernel's safe state (R14, R21.3). rsk's safe state stops every Job, and the source specifies no hardware back-up pacing: this is PM-ORQ-20.
4. WHEN a lower Partition faults, overruns or is stopped, THE PG SHALL continue pacing as programmed (R16, R18), and THE Therapy_Partition SHALL treat the peer-state error of an Endpoint to that Partition (R42.3) as the loss of the link where the Partition is the telemetry Partition (PM6.4).

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
4. THE properties of PM25.3 that span more than one Job (an interval from one event to the next Pace) SHALL be stated over the sequence of events, and the tool that checks them SHALL be chosen and recorded (PM-ORQ-21).

### Part I — Platform

### Requirement 26: Target and rig (Stage E1) [P]

**User Story:** As a developer, I want the hardware the experiment runs on stated, so that platform limits are requirements and not surprises.

#### Acceptance Criteria

1. THE Application SHALL run on the nRF52840-DK as rsk's Target `nrf52840` describes it: a 64 MHz core, a 16 MHz Kernel time base (PM-PAR-08), 3 priority bits (PM-PAR-09), 8 MPU regions (PAR-07), and the Kernel-owned TIMER0, CLOCK, WDT and PPI (PR-29).
2. THE Application SHALL also build for the QEMU Target `qemu_mps2_an386` with the sense inputs driven by a scripted source, so that the therapy logic runs in the verification job without the rig.
3. THE Heart_Simulator SHALL run on the HIL_Rig's second board and SHALL be an rsk Application of its own, in the manner of `apps/hil-stimulus`.
4. THE Application SHALL use at most the Task Priority levels and dispatch vectors the Target provides (7 of each). Whether this limits the Application to 7 Tasks is PM-ORQ-22.
5. THE Application SHALL represent every interval as an integer number of time-base ticks and every rate by its interval, rounded so that PM-PAR-01 holds with the rounding included (R29.8).

### Requirement 27: Timed pacing on a Kernel with periodic and sporadic Tasks only (Stage E1) [P, I]

**User Story:** As a developer, I want a stated way to pace "one interval after the last event", so that the central timing pattern of a pacemaker has a home in the Profile.

#### Acceptance Criteria

1. THE Application SHALL realize each escape interval (the LRL, hysteresis, AV and URL intervals and the refractory periods, each started by an event and not at a fixed phase) with the release kinds the Profile has: a Periodic_Task at O + k·T (R9.1) or a Sporadic_Task released by a Release_Source (R10). The Profile has no one-shot delay that a Job can arm.
2. THE choice between a Periodic_Task that polls at a fixed period and a Partition-owned hardware timer whose compare interrupt is a Release_Source SHALL be made in design and recorded, with the interval error each gives against PM-PAR-01 and PM-PAR-03 (PM-ORQ-01).
3. THE Application SHALL NOT read the time base to busy-wait for an instant (PR-16 loop bounds, R35).
4. WHEN the declared operating duration ends, the Kernel enters the safe state (R9.5). THE Application SHALL declare a duration that its tests never reach, and this document SHALL record that continuous operation over a service life is outside the experiment (PM-NG-07).

### Requirement 28: Pace output and pulse width (Stage E1) [P, S]

**User Story:** As a developer, I want the pulse edges timed by hardware where software cannot meet the tolerance, so that pulse width is not at the mercy of scheduling.

#### Acceptance Criteria

1. THE Application SHALL drive each pace output as a digital line whose high time is the programmed pulse width, and SHALL report the programmed amplitude with each Pace as data (PM2.2).
2. THE Application SHALL meet PM-PAR-05 for the pulse width. With a release lag of the order of PM-PAR-10 a Job cannot time a 0.05 ms pulse or a ±0.02 ms tolerance by itself: the falling edge SHALL be produced by a hardware timer compare acting on the pin, or the requirement SHALL be recorded as not met with the measured error (PM-ORQ-23).
3. THE Target's PPI is Kernel-owned and PWM is not assignable to a Partition in rsk v1 (DD-08 scope note of R17). THE design SHALL state which Partition-assignable peripherals (GPIOTE, TIMER1, TIMER2, P0) produce the pulse and what, if anything, rsk must add (PM-ORQ-23).
4. THE Application SHALL NOT leave a pace output high when its Partition is stopped, restarted or the Kernel enters the safe state (PM23.3).

### Requirement 29: Sense, magnet, battery and activity inputs; the link (Stage E1 for sense; E3 for magnet, battery, link; E4 for activity) [P]

**User Story:** As a developer, I want each analog or wireless element of the source replaced by something the board has, so that the logic can be exercised end to end.

#### Acceptance Criteria

1. THE Application SHALL take each Chamber's sense signal as a digital edge from the Heart_Simulator, bound as the Release_Source of a Sporadic_Task (R10, R19). SAADC is not assignable to a Partition in rsk v1, so the Application measures no amplitude: sensitivity is a stored value, and a "sub-threshold" beat is one the Heart_Simulator does not send.
2. THE Application SHALL take the magnet as a digital input and the battery level as a value the Heart_Simulator or the DCM_Stub sets, and SHALL NOT claim a voltage measurement (PM17.5).
3. THE Application SHALL take activity either from an accelerometer on SPIM, the one EasyDMA class a Partition may own (R17), or as a value supplied by the Heart_Simulator; the choice is PM-ORQ-24.
4. THE Application SHALL carry the DCM link on a wire. UARTE and RADIO are not assignable to a Partition in rsk v1; the link SHALL use a peripheral that is (SPIM, or GPIO lines), or rsk SHALL be extended first (PM-ORQ-25).
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

## Assumptions

| ID | Assumption | Affects |
|---|---|---|
| PM-ASM-01 | The Heart_Simulator's edges stand for sensed events after the analog front end: detection thresholds, filtering and blanking of the pulse artefact are outside the Application. | PM4, PM13, PM29.1 |
| PM-ASM-02 | A rate's tolerance of ±8 ms is the tolerance of its interval (Conventions). | PM-PAR-01, PM9 to PM12 |
| PM-ASM-03 | The release lag and the Kernel_Timing_Parameters of the Application's build are of the order of the P1 demo's (PM-PAR-10), far below 8 ms; to be confirmed by measurement (PM21.3). | PM24, PM28.2 |
| PM-ASM-04 | The logic analyzer's sample period is small against 0.02 ms, so that pulse widths can be judged. | PM24.3, PM28.2 |
| PM-ASM-05 | One Kernel time base serves both the therapy timing and the time stamps of Markers and trends; no real-time clock is needed beyond a value the DCM_Stub sets. | PM18.3, PM18.6, PM20.1 |

## Open Questions

Each question is to be resolved, or deferred with a recorded reason, before the design of the stage it affects.

| ID | Question | Affects |
|---|---|---|
| PM-ORQ-01 | **Escape intervals in a Profile without one-shot delays (central tension).** A pacemaker's timing is "pace one interval after the last event unless inhibited". rsk releases Tasks periodically at fixed phase or sporadically on an interrupt. A polling Periodic_Task of period p gives an error up to p plus response time (1 ms polling meets ±8 ms but not the ±1 ms of the Sensed AV Delay Offset with margin, and costs a release every period); a Partition-owned TIMER compare as a Release_Source gives a release at the instant but needs the Partition to re-arm a hardware timer from a Job and an MIT that admits the shortest interval. Does rsk need a Kernel-served one-shot timed event for Applications? | PM27, PM9 to PM12, PM24 |
| PM-ORQ-02 | [PM Table 6]'s cell positions were not recovered from the PDF's text. Complete Table 19-3 from the figure. | PM5.8, PM19.5 |
| PM-ORQ-03 | The source gives rate tolerances as times ("65 ppm ±8 ms"). Interval tolerance is assumed. | PM-ASM-02 |
| PM-ORQ-04 | Pulse width tolerance is 0.2 ms in [PM App. A] and ±0.02 ms for the Pace-Now and POR pulse; the nominal regulated amplitude 3.5 V is in the 0.5 V-step range while the nominal unregulated one is 3.75 V; the POR pulse width is 0.5 ms and the Pace-Now one 1.00 ms. Which tolerance governs? | PM3.3, PM6.5, PM7.1, PM28.2 |
| PM-ORQ-05 | The bound on "immediate" for a triggered Pace. | PM5.4 |
| PM-ORQ-06 | Pace-Now's "two cardiac cycles plus 500 ms" runs from a user action at the DCM. Which part is the PG's, and at which rate are the two cycles counted? | PM6.6 |
| PM-ORQ-07 | How the PG leaves the Power-On Reset State (a DCM command, or the first programming). | PM7.4 |
| PM-ORQ-08 | Ventricular Blanking appears only in [PM App. A]: when it starts (after an atrial Pace, as cross-chamber blanking, is the usual meaning) and which Modes use it. | PM4.5 |
| PM-ORQ-09 | Non-volatile storage for parameters and implant data: none in rsk's Target table. Add a Kernel-mediated one, or keep PM-NG-06? | PM30, PM18.7 |
| PM-ORQ-10 | Upper-rate behaviour: an atrial Sense whose tracked Pace would violate the URL interval is delayed to the URL interval or not tracked. The source states neither. | PM10.2 |
| PM-ORQ-11 | Dynamic AV delay: the factor's value and granularity; the "programmable maximum paced AV delay" is not in [PM App. A] (the Fixed AV Delay may be meant). | PM11.4 |
| PM-ORQ-12 | What "continuous noise" is: a rate of sense edges, a duration, or a signal from the front end. With PM22.6 the Kernel's MIT enforcement masks a storming source; whether that masking is the noise detector is a design question. | PM13 |
| PM-ORQ-13 | ATR detection: "predominately, but not exclusively, faster than URL" is not quantified, and [PM §5.6.1] item 2 states the cessation condition with the same words as onset (slower is presumably meant). A counting rule and its window are needed. | PM14.1 to PM14.3 |
| PM-ORQ-14 | The rate-adaptive transfer function: sensor output to SIR for each Response Factor and Activity Threshold; the sensor's units. | PM15 |
| PM-ORQ-15 | Hysteresis: the reading of "the current pacing rate must be faster than the HRL", and whether a Pace ends hysteresis. | PM16.1, PM16.2 |
| PM-ORQ-16 | Rate smoothing: [PM §5.9] has Up and Down percentages, [PM App. A] one parameter; and whether smoothing applies to sensed-to-paced transitions only. | PM16.3 |
| PM-ORQ-17 | Battery level changes: the Mode taken at ERT when the programmed Mode is dual-chamber or rate-adaptive; the thresholds of the levels; [PM Table 3] names "ERP" while [PM §3.7] says "post-ERT". | PM17.6, PM17.7, PM8.2 |
| PM-ORQ-18 | Interactive limits between parameters (for example LRL below URL, PVARP plus AV delay against the URL interval, HRL not above LRL): the source requires the error and lists no limit. | PM19.3, PM22.3 |
| PM-ORQ-19 | The bound on detecting a broken link. | PM20.2, PM6.4 |
| PM-ORQ-20 | **Safe state and pacing.** rsk's response of last resort stops every Job. A pacemaker's safe behaviour is to keep pacing at fixed parameters. Options: a hardware back-up outside the Application (not on the DK); a response that restarts the Therapy_Partition instead of the safe state wherever R14 allows; or record that the experiment has no pacing in the safe state. | PM23.3, PM28.4 |
| PM-ORQ-21 | The tool for properties over sequences of events (an interval from an event to the next Pace): the App_Prover over a pure step function with a ghost history, UPPAAL over the exported model, or a Lean statement over the run. | PM25.4 |
| PM-ORQ-22 | Whether the Target's 7 dispatch vectors limit an Application to 7 Tasks, and whether a pacemaker's Task set (two sense Tasks, pacing, rate adaptation, telemetry, history, markers) fits. | PM26.4 |
| PM-ORQ-23 | Hardware-timed pulse width with a Kernel-owned PPI: can a Partition own GPIOTE and a TIMER and have a fixed PPI channel configured at Init by the Generator, or must the pulse's falling edge be a Job's (and the 0.05 ms width and ±0.02 ms tolerance be recorded as not met)? | PM28.2, PM28.3 |
| PM-ORQ-24 | Activity input: an SPI accelerometer on SPIM, or a simulated value. | PM29.3, PM15 |
| PM-ORQ-25 | The DCM link's peripheral: SPIM with the board as master, GPIO lines, or adding UARTE to rsk's mediated EasyDMA classes. | PM29.4, PM20 |

## Sources

- **PM** *PACEMAKER System Specification*, Boston Scientific, January 3, 2007. `docs/pacemaker/PACEMAKER.pdf`. Sections 1 to 5, Appendices A and B, Tables 1 to 8. Read in full for this draft; its text was extracted from the PDF and Table 6's layout was lost (PM-ORQ-02).
- **rsk** `.kiro/specs/rsk-ravenscar-profile/requirements.md`, Draft 0.3: Requirements 9, 10, 12 to 19, 21, 22, 26, 28, 29, 33 to 35, 37, 40 to 42, 46, 50; restrictions PR-14, PR-16, PR-20, PR-29, PR-30, PR-33; non-goal NG-06; parameters PAR-05 to PAR-07.
- **Target** `crates/rsk-gen/src/target.rs` (the `nrf52840` and `qemu_mps2_an386` tables: clocks, dispatch vectors, peripherals and which are Kernel-owned or EasyDMA-capable) and the scope note of R17 (SPIM is the only mediated EasyDMA class), as of 2026-11-22.
- **Rig** `hil-rig.toml` and `docs/hil/kernel_timing_nrf52840_stress.json` (the release lag quoted in PM-PAR-10).
