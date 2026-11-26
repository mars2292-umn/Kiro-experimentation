# Design Document

**PACEMAKER on rsk: an experimental Application**

Status: Draft 1.3 (1.3, 2026-11-24: the stage E2 code measured (task 13.1): "What the measurement settled" has a fourth step, the Analyzer's result is FAIL again and PD-O3 is open again; Table 2's measured column is of this code. 1.2, 2026-11-23: what completing `validate` and stating the contracts settled (tasks 11.6, 12.1, 12.2), under "The timing cycle" and in the Properties table. 1.1, 2026-11-23: "The timing cycle" has what tasks 11.2 to 11.4 settled: the dynamic AV delay, the URL, PVARP, the PVC and its extension, blanking, with two choices for the user to confirm. 1.0, 2026-11-23: stage E2 begun; "The timing cycle" has what the dual-chamber step settled (tasks 11.1, 11.5) and "The hardware layer" what became of the proof of P6 and P7 (task 12.4; PD-O5 closed). 0.9, 2026-11-23, later: the sporadic Budgets are 350 µs by the user's decision, every record was measured again on that build, and the Analyzer passes: bounds 1.690, 2.757, 3.891 and 9.564 ms; Δ_release is accepted as the one nominal value with a term; PD-O3 is closed for stage E1. 0.8, 2026-11-23: "What the measurement settled" rewritten after tasks 9.4, 9.5, 9.8 and 9.9: with the fault entry, the stubs and the stamp lag measured the Analyzer's bound for `sense_a` is 4.036 ms against its 4 ms deadline, a FAIL by 36 µs that is recorded and not acted on; the inhibition window measured is under 0.22 ms; branch coverage of the Task entries 56 to 65 %; PD-O3 is open again. 0.7, 2026-11-23: Table 2 carries the values task 9.2 settled on measured records: Budgets of 400 µs for the sporadic Tasks, deadlines of 2, 3 and 4 ms for `pace`, `sense_v` and `sense_a`, PM-PAR-15 at 4 ms; PD-O3 is closed for stage E1; "Timing Design" has the Analyzer's bounds. 0.6, 2026-11-22: the Application exists for stage E1, `apps/pacemaker`, on QEMU and on the board; what building it changed or settled is in "What the first build settled"; PD-O2 is closed. 0.5, 2026-11-22: the spike's decision is recorded in "The spike's decision" and PD-O1 is closed; the Kernel's write of the PPI channels has run on the board. 0.4, 2026-11-22: "Arming and the lead" and "Sense intake" corrected to what was implemented in `pm-hw` and run on the board: two constants `GUARD` and `LEAD`, the far side kept by a refresh, a Pace reported only after its pulse, the sense flag cleared before the compare is written; and the Kernel's write of the PPI channels placed after the boot checks. 0.2 closed PD-O7: `pace` is released by the end of a pulse, and a Chamber's compares have one write order; 0.3, 2026-11-22: the register behaviour of "Hardware signal flow" is confirmed on the board by the spike, each pace channel gains a fork that carries the pulse to the Heart_Simulator, and the step function's interface is the one implemented for stage E1). Implements `requirements.md` Draft 0.4 of this folder. Requirement references `PMn.m`, `PM-PAR-nn`, `PM-NG-nn`, `PM-ASM-nn`, `PM-ORQ-nn` point into that document; `Rn.m`, `PR-nn`, `DD-nn` into the rsk requirements and design; design decisions of this document are numbered `PD-nn`.

Every number in this document that is not quoted from the source or from an rsk record is a **proposal** to be replaced by a measurement (PM21.3, PM31.3). The nRF52840 register facts it relies on (TIMER3's six compare registers and interrupt line, counter mode, PPI forks, GPIOTE tasks) were confirmed on board A by the spike on 2026-11-22, one probe per behaviour (`spike/report.md`, task 1.2).

## Overview

The pulse generator is an rsk Application of three Partitions and six Tasks. Three ideas shape it:

1. **Hardware makes every edge; Jobs only arm it.** A pace's rising and falling edges are timer compares acting on the pin through fixed PPI channels, and a sense's instant is a timer capture. A Job decides *when* and writes compare registers ahead of time; no edge waits for a Job. Pulse width and every interval between hardware-timed events are then exact to the timer's tick, and what scheduling must guarantee is only that each arming Job finishes before the instant it arms (PD-01, PD-02).
2. **The therapy is one pure step function.** Given the parameters, the timing state and one event, it returns the new state and the instants to arm. The Tasks are thin: read the hardware, call the step, write the hardware. The step is what the App_Prover proves and what host tests exercise, on every Target (PD-05).
3. **Therapy is alone at the top.** The Therapy_Partition (level A) holds the three sporadic Tasks that the hardware releases and one periodic Task for everything slow. Telemetry and history are lower Partitions that reach it only through Endpoints, the programming one validated (PD-07).

The design found three places where the requirements as written could not be kept, and one finding about rsk; they are in "Requirements Impact" and were applied to `requirements.md` (Draft 0.4).

### Design decisions

| ID | Decision | Resolves | Main requirements |
|---|---|---|---|
| PD-01 | One therapy timer, TIMER3 at 1 MHz with six compare registers, owned by the Therapy_Partition: CC0 and CC1 are the next atrial and ventricular Pace instants, CC4 and CC5 the ends of their pulses, CC2 and CC3 capture the atrial and ventricular Sense instants. Fixed PPI channels carry compares to GPIOTE set and clear tasks and GPIOTE input events to the captures. The interrupt of CC4 and CC5, the end of a pulse, is the Release_Source of the Task `pace`, so that Job never runs while its pulse is high. | PM-ORQ-01, PM-ORQ-23 | PM27, PM28, PM33 |
| PD-02 | A Job writes a Chamber's compares in one order: it disarms the start, checks that no Pace of the Chamber fired, and only then writes the end and after it the start. An end compare is therefore never written while its pulse is high or can begin. The Job then checks that the start is still ahead of the present by a lead; if not, it re-arms one lead ahead, at most twice more, and otherwise raises the pin itself and reports it. A compare written in the past would not fire until the counter wraps (71.6 minutes). | — | PM9.1, PM24, PM28.2 |
| PD-03 | Each Chamber's sense line also clocks a timer in counter mode (TIMER1 atrial, TIMER2 ventricular) whose compare interrupt is the Release_Source of that Chamber's sense Task. GPIOTE has one interrupt line for all its channels, and rsk masks a source for one MIT after each event (DD-06): with both Chambers on GPIOTE a Sense in one would hold back the handling of the other. The counters give each Chamber its own line, its own MIT, and a count of edges for noise detection. | PM-ORQ-12 | PM4, PM13, PM22.6, PM29.1 |
| PD-04 | A Sense is acted on by its sense Job, so a Sense that precedes a pending Pace by less than the sense Task's response time does not inhibit it: both occur, the Sense first. The window is PM-PAR-15 and is measured. | — | PM5.5, PM24 |
| PD-05 | The therapy decision is a pure function `step(params, state, event) -> (state, arming)` in a `no_std` crate with no rsk dependency; time is the therapy timer's count in microseconds, 32 bits, compared modulo 2^32. | PM-ORQ-21 | PM25 |
| PD-06 | Timing is ventricular-based in the Modes that pace the ventricle: every escape interval starts at a ventricular event, and the atrial Pace of a dual-chamber Mode is due one LRL (or sensor) interval less the AV delay after it. | — | PM9.2, PM11 |
| PD-07 | Three Partitions: `therapy` (level A), `comms` (level C), `diag` (level D). Six Tasks (Table 2). Parameters and commands reach `therapy` over one validated Endpoint; events leave it in batches over another. | PM-ORQ-22 | PM22, PM32 |
| PD-08 | The Therapy_Partition's three responses are RESTART_PARTITION; its statics, zero at start, are the Power-On Reset State, so a restart *is* a power-on reset of the therapy. Pulse ends are hardware compares, so a pin cannot stay high across a restart, a stop, or the safe state. | PM-ORQ-20 | PM7, PM23, PM28.4, PM30.4 |
| PD-09 | The battery level and the activity value arrive in link frames from the DCM_Stub; the magnet is a pin; sense edges are the Heart_Simulator's. No analog peripheral is used. | PM-ORQ-24 | PM2, PM29 |
| PD-10 | A hardware layer `Hw` with two implementations: the nRF52840 one of PD-01 to PD-03, and a QEMU one in which a Partition-owned timer stands for the pacing compare and a scripted Task for the sense lines. The step function and the Tasks' logic are the same code on both. | — | PM26.2 |
| PD-11 | The DCM link is a Periodic_Task polling SPIM1 with fixed-size frames; three unanswered polls are the loss of the link. | PM-ORQ-19, PM-ORQ-25 | PM20, PM29.4 |
| PD-12 | rsk changes: fixed PPI channels with an optional fork (Requirement 33), and TIMER3 added to the Target table. Nothing else; in particular no Kernel one-shot event and no new EasyDMA class. | — | PM33 |

## Architecture

### Hardware signal flow (nRF52840)

```
 Heart_Simulator (board B)                PG (board A, Therapy_Partition)
   A sense line ───────────► GPIOTE IN[0] ──PPI──► TIMER3.CAPTURE[2]      (instant of the atrial Sense)
                                          └─fork─► TIMER1.COUNT ─► COMPARE[0] irq ─► Task sense_a
   V sense line ───────────► GPIOTE IN[1] ──PPI──► TIMER3.CAPTURE[3]      (instant of the ventricular Sense)
                                          └─fork─► TIMER2.COUNT ─► COMPARE[0] irq ─► Task sense_v

   TIMER3 (1 MHz, free-running, 32 bit)
     COMPARE[0] ──PPI──► GPIOTE SET[2]  (A pace pin high)   ─fork─► GPIOTE SET[4]  (A pace copy high)
     COMPARE[1] ──PPI──► GPIOTE SET[3]  (V pace pin high)   ─fork─► GPIOTE SET[5]  (V pace copy high)
     COMPARE[4] ──PPI──► GPIOTE CLR[2]  (A pace pin low)    ─fork─► GPIOTE CLR[4]   ┐ irq ─► Task pace
     COMPARE[5] ──PPI──► GPIOTE CLR[3]  (V pace pin low)    ─fork─► GPIOTE CLR[5]   ┘

   A pace pin, V pace pin   ─► the logic analyzer
   A pace copy, V pace copy ─► Heart_Simulator inputs (the board-to-board lines)
```

Six PPI channels, each with a fork, and six GPIOTE channels. All connect peripherals of one Partition (PM33.2). The Kernel writes them at Init from Generated_Config and never again (PM33.3).

**How the Kernel writes them (rsk, tasks 3.2 to 3.4).** The configuration checksum of a configuration with channels is the CRC-32 of the Kernel's tables continued over the channel table. The boot core is unchanged: it compares the tables' part, disables every channel and reads the enable register back as zero. After it has passed, and before any Task is released, the boot shell calls one function (`rsk_kernel::arch::cm4::ppi::ppi_boot`) that recomputes the checksum over the tables and the channel table in memory and, only if it is the recorded one, writes each channel's event, task and fork end-point registers and then the enable bits of exactly those channels; otherwise it writes nothing and the boot fails into the safe state (PM33.4). Its contract is proven against the Kernel's register-file specification. A configuration without channels is generated, checked and booted exactly as before. The Config_Checker recomputes the table from the Task_Model, with the ownership rule, and compares it with the image's tail and with the table in the binary. The HIL build's timestamp probe uses channel 15, so an Application measured on the rig has channels 0 to 14.

**The pace copies.** On the rig the pace pins the analyzer sees do not reach board B, and the board-to-board lines are not on the analyzer (`hil-rig.toml`). Each pace compare therefore drives two pins from the one event: the pace pin, which is measured, and a copy on a board-to-board line, which the Heart_Simulator reads. The spike showed one compare event moving both pins and board B reading the copy; that the two edges coincide is the PPI's property and was not measured, and nothing is judged on the copy. The copies exist for the rig: a PG whose pace pins reached the heart would not have them (a finding for PM31.2).

What each requirement gets from this:

- **Pulse width (PM3.3, PM28.2).** The width is CC4 − CC0 (or CC5 − CC1), written by one Job in one Critical_Section. Its error is the timer's tick, 1 µs, against tolerances of 200 µs and 20 µs.
- **Interval accuracy (PM9 to PM12).** An interval from a Pace to the next Pace is a difference of two compare values; from a Sense to a Pace, of a capture and a compare. Both are exact to 1 µs, plus the rounding of a rate to microseconds (under 1 µs). The ±8 ms and ±1 ms tolerances are then a matter of the arming Job being in time (next section), not of jitter.
- **Marker times (PM18.6).** Event instants are captures and compares, so a Marker's time is exact to 1 µs whenever its Job runs.
- **No pin left high (PM28.4).** Whenever a start compare is armed, the end compare of its Chamber is its start plus the width, and it is not written again until the pulse has ended (next section).

### Arming and the lead (PD-02)

Two hazards decide how a Chamber's compares are written. A compare register written with a value the counter has already passed does not fire until the counter wraps. And an end compare rewritten while its pulse is high, or just before a start that is still armed fires, leaves the pin high until the new value: up to a whole escape interval.

The rule against the second: **a Chamber's end compare is written only while its start compare is disarmed and no Pace of the Chamber is unaccounted.** A Pace is unaccounted from the instant its start compare fires until the `pace` Job that its pulse's end releases has cleared the start's event flag; the pulse lies inside that span.

The sequence for one Chamber, inside the Critical_Section on the Resource `timing`, used for arming, moving and withdrawing alike (as implemented in `crates/pm-hw` and run on the board by the spike, Draft 0.4):

1. write the start compare to the far side of the counter (present − `FAR`, 2^30 µs): nothing of this Chamber can fire now;
2. read the start's event flag. If it is set, a Pace fired, before this Job or as it began: write nothing more and return `TooLate`. The end compare in force is that pulse's own, the pending instant stays in the step's state, and the `pace` Job arms from it when the pulse has ended;
3. if the Pace is withdrawn, done;
4. clear the end's event flag (no pulse is in progress, so a set one is stale), write the end compare (start + width), then the start compare;
5. read the present; if start − present, as a signed 32-bit difference, is at least `GUARD`, done: the write came before the instant, so the compare will fire;
6. otherwise repeat from 1 with start set to present + `LEAD`, at most twice more. If step 2 then finds the flag set, the Pace of the round before was delivered at its instant, with its own end in force, and the sequence ends;
7. if the third check fails, disarm and check once more (steps 1 and 2), write the end compare to present + `LEAD` + width, write the GPIOTE set task directly, and record a late-arm event; if the end compare is no longer ahead of the present after that, write the clear task as well and record the pulse as not timed. Until that pulse's end is seen, the Chamber's compares are not written again.

Because the end is written before the start and only with the start disarmed, a start that fires always has its own end in force. Because of step 2, no Job writes an end compare under a pulse.

**Two constants, not one.** Draft 0.2 compared step 5's difference with `LEAD` and retried at present + `LEAD`, which a retry can never pass: the present has moved on by the time it is read again. The check needs only `GUARD` (proposal: 64 µs): the error of the present's reading (half the spread of "The two clocks") and the timer's own latency. `LEAD` (proposal: 500 µs) is how far ahead a retry is placed, and has to cover what can delay the Job between its reading of the present and its write of the start: a Kernel hold (rsk's L_kernel, 138 µs in the hour-long stress record) and entries that follow it. A Pace moved by step 6 is late by at most `LEAD` past the present, far inside PM-PAR-01.

**Measured on the board (task 1.3, `spike/report.md`).** With the present read exactly and an interrupt of 140 µs taken at spacings of 150 to 900 µs beyond its hold: no forced pulse (step 7) at any `LEAD` from 100 µs up, in 1,601 instants per value swept a tick at a time from 1.5 ms ahead to 0.1 ms behind; the same at 300, 500 and 800 µs under an interrupt of 410 µs. At a `LEAD` of 50 µs, below `GUARD`, every moved Pace was forced, as it must be. With `GUARD` at 0 one Pace in 1,601 was reported armed and did not fire; at 1 µs and above none. No Pace reported armed failed to fire with `GUARD` at 64 µs, and every pulse not forced was 400 µs to the analyzer's sample. The values are decided in "The spike's decision".

**The far side has to be kept.** A start compare left behind the counter is reached again when the counter comes round: `FAR` behind the present, that is 2^32 − 2^30 µs (53.7 minutes) later. A Chamber that paces is rewritten every escape interval; a Chamber the Mode does not pace, or one whose Pace was withdrawn, is not. `control` therefore refreshes every Chamber that has no Pace armed in each period (`Hw::refresh`: the start compare to the far side again). Draft 0.2 put the far side one tick behind the present, where an error of the present's reading could place it ahead.

The loop has a constant bound of three (PR-16).

**`pace` runs after the pulse.** The end compares (4 and 5) raise TIMER3's interrupt and the start compares do not, so the `pace` Job of a Pace is released when the pin has gone low. It finds which Chambers paced by their start flags (an event flag is set whether or not its interrupt is enabled), and for a Chamber whose end flag is set it clears both, gives the step a `Paced` event, and arms from the pending instant in the state. A Chamber whose start flag is set and whose end flag is not is in its pulse: it is left alone, flag and all, so that step 2 keeps every Job from its compares, and its own end releases `pace` again. An end flag without a start flag is the end compare of a withdrawn Pace coming past: it is cleared and nothing is reported. It runs up to one pulse width (1.9 ms at most) later than a Job released by the start would; the tightest chain it is in has 30 ms ("Timing Design").

**Events in the order they happened.** A sense Job can run between a Pace and that Pace's `pace` Job. The step therefore accounts for a pending Pace itself: given an event at or after the instant of a pending Pace that was not withdrawn, it first applies that Pace (its refractory and blanking periods, the next pending instants), then the event. The armed compare fires without software, so the Pace is a fact once its instant has passed. The `Paced` event that arrives later is then a confirmation and changes nothing but the arming of step 4.

### The spike's decision (task 1.4, 2026-11-22)

**The design stands, with the corrections of Drafts 0.3 and 0.4.** The hardware signal flow is as drawn: each of its six register behaviours was confirmed on the board (task 1.2). The write order of this section, as corrected while it was implemented (two constants, the far side kept by a refresh, a Pace reported only after its pulse), held on the board under an injected interrupt (task 1.3). The pace copies by forks are adopted. Nothing in `pm-core` depended on the outcome.

**`LEAD` = 500 µs.** On the board no forced pulse was needed at any `LEAD` from 100 µs up, under an interrupt of the Kernel's measured hold (140 µs) and of its longest run of entries (410 µs, tried at 300, 500 and 800 µs). 500 µs is five times the smallest value that passed and exceeds the longest run of entries with `GUARD` to spare, so one such run cannot spoil a round. Its cost is how late a Pace is when its instant cannot be met: 602 µs at most in the sweep under the 140 µs hold and 893 µs under the 410 µs one, against a tolerance of 8 ms (PM-PAR-01), and it is the delay of a triggered Pace (PM5.4), again against 8 ms.

**`GUARD` = 64 µs.** The timer itself needed 1 µs: with 0, one Pace in 1,601 was reported armed and did not fire. The rest of the 64 µs is for the error of the present's reading in the Application, which comes from the Kernel's time base through the offset of "The two clocks" (half of `OFFSET_SPREAD`, 32 µs). The spike read the present exactly from the timer, so this part is a bound by construction and not yet a measurement.

**What the spike did not settle, and where it goes.** The present through the Kernel's clock, and with it `OFFSET_SPREAD` and the 64 µs: task 7.2 repeats the sweep through the Application's own `Hw`. That a task and its fork act at one instant: not measurable on this rig's wiring, and nothing is judged on the copies. One sweep per configuration: the claim is "none in 1,601 instants", and the rig runs of task 9 are what lengthen it.

### What the first build settled (tasks 7.2, 7.3, 8.1 to 8.3; 2026-11-22)

- **The present through the Kernel's clock.** `control`'s first Job measures the offset as "The two clocks" says. On the board the two readings of the Kernel's time base around the capture were 3 µs apart at the first attempt, so the offset's error is under 2 µs, against the 32 µs that `GUARD` allows for it. The arming sweep of the spike, repeated through the Application's own hardware layer under the Kernel, found no Pace reported armed that did not fire and no forced pulse in 5,203 instants (`rig/task_7_2_sweep.json`). `GUARD` at 64 µs and `OFFSET_SPREAD` at 64 µs stand; both are far above what was measured.
- **A Partition static has no padding byte.** rsk reads every byte of a static's initial value at compile time and refuses a non-zero one (its check of R14.5), and a padding byte cannot be read. So every type kept in the Resource `timing` carries its spare bytes as a named field that is always zero (`reserved` in `Params`, `Pending`, `ChamberState`, `State`, and in the Partition's own records). This is PM30.4 enforced by the build: the Application does not compile unless the Resource's value at start is all zero bytes.
- **A debugger's access restarts the therapy.** The access halts the core; the Jobs due meanwhile miss their deadlines; with the Deadline_Miss_Response RESTART_PARTITION the Partition restarts and its state is cleared. Found when the sweep's record read back as zeros. It is the declared response working, and it means a record kept in the Partition cannot be read from a running PG through the probe: the sweep has a declaration variant whose overrun and deadline-miss responses record only, and stage E3's restart measurements (task 17.1) must read the Kernel's log, not the Partition.
- **QEMU.** The Target has no PPI and one Partition-ownable timer. The board layer keeps the roles of the registers in memory, sets the dual timer's one-shot to the earliest armed start, and treats a start the present has reached as fired, its pulse included. The sense Tasks are released by two Release_Signals that `control` raises as it plays a script. The driver and the step are the code the board runs; only the board layer differs.
- **Budgets.** The first build had 1 ms for each sporadic Task and 5 ms for `control` (Table 2 gave none), unmeasured. Tasks 9.2 and 9.8 measured them and replaced the sporadic Tasks' with 350 µs: "Timing Design".
- **What rsk's build checks changed (task 8.4).** The Partition's stack is 8 KiB: the Link_Checker's worst case with all four Tasks on it was 4,848 bytes. (Since task 16.2 it is 12 KiB: with the State machine's state the worst case is 9,632 bytes on the board.) `pm-core` and `pm-hw` have no code of their own: rsk allows executable symbols from the Kernel, `rsk`, the system crate and the Partition crates only, so every function of the two is inline and is instantiated in the Partition that calls it; a second Partition that uses `pm-core` gets its own copy. Both are in `findings.md` (B1, B5).
- **`control` owes two things to the hardware layer**, both now in its body: one sense service per Chamber at Init, and a refresh of every Chamber with no Pace armed in each period.

### The two clocks

`rsk::now()` is the Kernel time base (TIMER0, 16 MHz, 64 bits). The therapy timer is TIMER3 at 1 MHz, 32 bits, started by the first Job of `control`. Both run from the crystal oscillator, so they do not drift against each other; they differ by a constant offset. `control`'s first Job measures the offset once: it triggers a TIMER3 capture between two readings of `rsk::now()` and takes their midpoint; if the readings are more than `OFFSET_SPREAD` (proposal: 64 µs) apart it repeats, at most eight times, and faults otherwise. The offset's error is half the spread and enters only the comparison of step 3 above (covered by `LEAD`) and the time stamps of events that have no capture.

**Instants (since 2026-11-24).** The therapy logic counts in microseconds from the start of the Partition, in 64 bits, which do not wrap (584,000 years): `pm_core::Micros`. Two instants are ordered as numbers, and that is what lets the App_Prover prove Properties P1 and P2 (`proofs.md`; `spike/time64.md` is the investigation). The timer keeps its 32 bits, and its wrap every 71.6 minutes is the hardware layer's alone: the driver holds the present as a 64-bit instant with the counter value it belongs to, moves both on at every reading of the counter (`control` reads it every 20 ms; the reading must come within 35.8 minutes), gives a counter value near the present its instant (`extend`: a compare just written, a sense edge just captured) and writes an instant to a compare as its low 32 bits (`count_of`). Its own questions (is the start still ahead of the counter by the guard) stay signed differences of counter values (`pm_hw::Count`). The event ring keeps the low 32 bits. Until that date every instant was the timer's 32-bit count, compared by signed difference.

### Sense intake, MIT and noise (PD-03)

rsk's intake (DD-06) masks an interrupt line in the NVIC from an event until the later of one MIT after it and the end of the Job, and events that arrive meanwhile stay latched in the peripheral and are delivered at the unmask. Three consequences decide the design:

- **One line per Chamber.** With one line for both, an atrial Sense would delay the handling of a ventricular Sense by up to the MIT. With TIMER1 and TIMER2 as edge counters, each Chamber's line is masked only by its own events.
- **The MIT can be long without hurting therapy.** A Sense that follows another Sense of its Chamber within `MIT_SENSE` (proposal: 50 ms) is handled up to 50 ms late. Every Mode that acts on a Sense has a refractory period of at least 150 ms after an event of that Chamber (Table 19-1), so such a Sense is a Refractory sense and changes no Pace. Its instant is still exact (the capture), and the count says how many edges came.
- **A storm cannot restart the Partition.** While the line is masked, edges are latched and counted, not delivered: each costs nothing, and the next delivery is one intake at the unmask. No MIT_Violation is recorded for them (R10.2 and R10.3 concern events that reach the Kernel), so the Partition's `mit_threshold` is not approached and no Health_Monitor response follows. This answers the point PM22.6 left to be shown, on paper; the rig's storm scenario is the demonstration.

The sense Job's service, for its Chamber, inside the Critical_Section on `timing` (as implemented, Draft 0.4):

1. capture the counter (`n` edges so far);
2. clear the compare event, then set the counter's compare to `n + 1`, so the next edge raises the interrupt. The flag is cleared before the compare is written: the compare still in force is at or behind the count and cannot fire, so no edge is lost between the clear and the write. (Draft 0.2 cleared it last, which loses an edge that arrives between the two.)
3. capture again: if the count has moved, an edge came between the capture and the write and the compare is already passed, so repeat from 2 with the new count, at most three times; if every round lost its race, which only edges faster than a register access can do, set the compare 16 edges ahead and stop;
4. read the Chamber's TIMER3 capture (the instant of the latest edge), and give the step function `Sense { chamber, at, edges }` with the edges since the last service, if there are any.

The first Job of `control` makes one service per Chamber, which arms each counter for its first edge.

The counter is never cleared, so no edge between a read and a clear is lost. It is 32 bits and counts heartbeats: it does not wrap in any run.

**Noise (PM13.1).** The step function marks a Chamber noisy for the cycle when every service since the end of its refractory period reported edges no further apart than PM-PAR-12 (100 ms): with `MIT_SENSE` at 50 ms, a continuous disturbance shows as a service at every unmask with `edges > 1` or an inter-service time under 100 ms. In a noisy cycle the pending Pace is left armed, which is asynchronous pacing.

### The inhibition window (PD-04)

A pending Pace is a compare already armed. A Sense cancels it only when its sense Job runs and disarms the compare. So a non-refractory Sense at instant s does not inhibit a Pace due before s + R_sense, where R_sense is the sense Task's worst response time: the pin goes high, and the `pace` Job that follows finds a Sense captured before its Pace. The step function records both (the Sense, then the Pace, with the fusion noted in the Marker), and starts the next interval from the Pace.

This is a deviation from PM5.5 ("a Sense SHALL cancel the pending Pace") of bounded size. PM-PAR-15 states the bound as a requirement (4 ms since task 9.2, the deadline of the slower sense Task; the proposal was 2 ms) and PM24 measures it. The stimulus is harmless in the model (it falls at the start of the Chamber's refractory period), and a hardware inhibit, which is what closes the window in a real device, is not expressible with the PPI channels this design has without also letting noise inhibit pacing.

### Build-time and run-time structure

The Application follows the P1 demo's layout (DD-11):

```
apps/pacemaker/
  decl/        the App_Declaration (rsk::app!), one variant per Target
  therapy/     Partition crate: Tasks pace, sense_a, sense_v, control; the Hw layer
  comms/       Partition crate: Task link
  diag/        Partition crate: Task history
  system/      the generated system crate
  bin/         the binary crate
  gen/         Generated_Config, Task_Model, linker script (per variant)
crates/pm-core/        the step function, parameters, validation: no_std, no rsk, no hardware
apps/heart-sim/        the Heart_Simulator, an rsk Application for board B
tools/dcm-stub/        the DCM_Stub, a host program
```

`pm-core` is the only crate with therapy knowledge. It is used by `therapy` (the step and the validation function), by `comms` (frame encoding only), by the DCM_Stub (the same parameter types and validation, to predict the PG's answer), and by host tests.

## Components and Interfaces

### Partitions, Tasks and Resources

**Table 1 — Partitions**

| Partition | Level | Peripherals | Responses (fault, overrun, deadline miss) | Holds |
|---|---|---|---|---|
| `therapy` | A | GPIOTE, P0, TIMER1, TIMER2, TIMER3 | RESTART_PARTITION ×3 (PM23.3) | Pacing, sensing, the State machine, parameters in force |
| `comms` | C | SPIM1 | RESTART_PARTITION, END_JOB, RECORD_ONLY | The DCM link |
| `diag` | D | none | RESTART_PARTITION, END_JOB, RECORD_ONLY | Histograms, trends, implant data, Marker stream |

`therapy` uses five peripheral windows, the most a Partition's MPU layout holds (DD-07). P0 covers both GPIO ports and has one owner (PR-29), so `therapy` configures every pin at Init, the link's SPI pins included; `comms` only names them in SPIM1's pin-select registers. This cross-Partition dependency is recorded as a finding (PM31.2).

**Table 2 — Tasks** (Priorities are fixed. For `therapy`, the Budgets and deadlines are those tasks 9.2 and 9.8 settled on the measured records of 2026-11-23, and "Measured" is the longest Job of the four 300 s runs on the stage E2 code (VVI, AAI, DDD and DOO sets, 2026-11-23; on the stage E1 code the pairs of runs gave 226 to 228 µs for `pace` and 182 to 234 µs for a sense Job, the difference one Kernel entry inside the Job); `link` and `history` are proposals until stages E3 and E5 build them)

| Task | Partition | Kind | Release | Period or MIT | Deadline | Budget | Measured (longest Job) | Priority | Work |
|---|---|---|---|---|---|---|---|---|---|
| `pace` | therapy | sporadic | TIMER3 compare 4 or 5 (the end of a pulse) | MIT 20 ms | 2 ms | 450 µs | 310.1 µs | 7 | Record the Pace, run the step, arm the Chamber that paced from the state |
| `sense_v` | therapy | sporadic | TIMER2 compare | MIT 50 ms | 3 ms | 450 µs | 303.8 µs | 6 | Service of "Sense intake", run the step, arm or disarm |
| `sense_a` | therapy | sporadic | TIMER1 compare | MIT 50 ms | 4 ms | 450 µs | 324.7 µs | 5 | The same for the atrium |
| `control` | therapy | periodic | — | 20 ms | 20 ms | 5 ms | 488.0 µs | 4 | Init; commands and parameters from the Endpoint; State changes; magnet pin; battery and activity values; rate adaptation; ATR timing; link loss; drain the event ring to `diag` |
| `link` | comms | periodic | — | 50 ms | 50 ms | 2 ms | 539.1 µs | 2 | One SPI frame each way; forward commands, return status and records |
| `history` | diag | periodic | — | 100 ms | 100 ms | 2 ms | not measured | 1 | Histograms, trends, the Marker buffer (tasks 22.1 to 22.3); one request of `link`'s and one record for it a Job (task 22.4) |

Six Tasks of the seven the Target allows (PM32.1); the seventh is kept for Stage E5 (electrogram sampling) or as margin.

`MIT_PACE` of 20 ms is below the shortest time between the ends of two pulses: an atrial and a ventricular Pace are at least the smallest AV delay apart (30 ms, Table 19-5), so their ends are at least 30 ms less the largest difference of two widths (1.85 ms) apart, 28.15 ms; two Paces of one Chamber are at least 150 ms apart. Only compares 4 and 5 raise TIMER3's interrupt; 0 and 1 (pulse starts) and the captures do not (PM27.5).

**Resources.** `timing` (in `therapy`): the step function's state, the parameters in force, the State, and the event ring. Its accessors are all four `therapy` Tasks, so its Ceiling is 7. Every Critical_Section on it is one step and the register writes that follow: no loop but the constant ones of "Arming" and "Sense intake". Because the Ceiling is the top Task Priority, no Task preempts a Critical_Section on `timing`; only Kernel entries do, which is what `LEAD` covers.

### Endpoints

| Endpoint | Roles | Payload | Validation | Release |
|---|---|---|---|---|
| `program` | `comms.link` → `therapy.control` | 30 words (`pm_core::wire`; 29 until task 19.5 gave a set its 29th word): a code and the DCM's frame number, then a parameter set for the two codes that carry one. One message for every frame of the DCM's that `link` hears: a command (PROGRAM, TEMPORARY, END_TEMPORARY, PACE_NOW), or the mark that a frame was heard | `therapy`'s validation function (R42.4: from level C to level A), `wire::admitted`: exactly the five codes, a set exactly when `validate` accepts it (Table 19-1 and Table 19-5; PM19.3, PM22.3), zeros where no set belongs | none: `control` takes up to four messages every period, and at most one command |
| `status` | `therapy.control` → `comms.link` | 4 words: the State, the Mode in force, the battery level, whether `control` has the link as up; the count of verdicts; the last verdict and the frame it answers; the count of refused messages. Sent when it changes, when `comms` has restarted, and once a second | none needed (downward). The channel toward `control` is never read; its validation function accepts nothing | none |
| `events` | `therapy.control` → `diag.history` | 28 words (`pm_core::record`; task 22.1): the count of records, the number of the first, the count of records the ring has lost and the sensor's interval, then up to 8 event records of three words. Capacity 2 | none needed (downward). The channel toward `control` is never read; its validation function accepts nothing | none: `control` sends one batch in a period of `history` at most (every fifth Job), and once a second an empty one; `history` takes up to two |
| `records` | `diag.history` → `comms.link` | 6 words, a link frame's payload (`pm_core::record`; task 22.4): the record's kind and part, then five words of data. Capacity 2 | `comms`'s validation function (R42.4: from level D to level C), `record::admitted`: a kind of record and a part that kind has, nothing above them | none: `history` sends one in a Job, `link` takes one in a Job when the one before has been told |
| `request` | `comms.link` → `diag.history` | 3 words: a code and two values (reset the histograms; begin a trend; read a part of a record; start or stop the Marker transmission; supply a word of the implant data or a measured value's stand-in). Capacity 2 | none needed (downward). The channel toward `link` is never read; its validation function accepts nothing | none: one a Job each side |

No Endpoint releases a Task: every receiver is periodic, so a flood from a lower Partition costs the receiver nothing beyond its own Budget (PM22.5). A full `events` buffer makes `control` keep the batch in the ring and count a drop when the ring overflows; `pace` and the sense Tasks never touch an Endpoint (PM22.4).

Payload types are words and arrays of words, valid for every bit pattern (R42.1); the meaning is checked by the validation functions, not by the type.

### The hardware layer `Hw` (PD-10)

```rust
/// What the therapy Tasks need of the board. Implemented for the nRF52840
/// (TIMER3, TIMER1, TIMER2, GPIOTE through `rsk::mmio`) and for QEMU.
pub trait Hw {
    /// Init: pins, GPIOTE channels, timers started, the clock offset measured.
    fn configure(&mut self) -> Result<(), HwFault>;
    /// The present, in therapy microseconds.
    fn now(&self) -> Micros;
    /// Arm a Pace of `chamber`: high at `start`, low `width` later (PD-02).
    fn arm(&mut self, chamber: Chamber, start: Micros, width: Micros) -> Armed;
    /// Withdraw the armed Pace of `chamber`, if its start has not passed.
    fn disarm(&mut self, chamber: Chamber) -> Disarmed;
    /// Which Paces fired since the last call, with their instants (`pace`).
    fn fired(&mut self) -> Fired;
    /// The edges of `chamber` since the last call and the latest one's instant.
    fn sensed(&mut self, chamber: Chamber) -> Option<Sensed>;
    /// The magnet pin.
    fn magnet(&self) -> bool;
}
```

**Properties P6 and P7 (task 12.4, 2026-11-23; PD-O5).** The route first planned, contracts on `arm` and `disarm` over a register model with a clock that advances by a bounded amount, was not taken: the model would have to be state the prover sees on the `Board` trait, the Partition's board has none (it writes memory-mapped registers, and a contract assumed over a model of a value that does not change is contradictory), the time arithmetic is wrapping and has no contracts, and the bounded advance would be an assumption either way. Instead the order P6 asks is a matter of types: an end compare is written in one function, `write_end`, which spends a witness (`Free`) that only `disarm_and_check` makes when the start compare has just been put on the far side and its event flag read clear; and the loops of the driver carry App_Prover variants equal to their bounds, which is P7's "within three rounds". P6's value relation (the end compare is the start plus the width) and P7's "the start is ahead of the counter after `Ok`" stay with the register model of the host tests, which lets the timer run between any two accesses.

`arm` and `disarm` are the sequence of "Arming and the lead". `disarm` is its steps 1 to 3: it moves the start compare only, never the end, and `Disarmed::TooLate` (the start's event flag is set) is the case of PD-04. `arm` returns `Armed::TooLate` in the same case and writes nothing; the `pace` Job then arms from the state. The trait as implemented (`crates/pm-hw`) has one more method, `refresh`, and its sequences are written over a `Board` that names each register by its role, so that the same code runs on the nRF52840's registers and on the host's register model.

The QEMU implementation uses the Target's Partition-ownable dual timer as the pacing compare (start only; the pulse's end is the Job's) and takes Senses from a table that `control` plays. It is for the logic and the Analyzer's inputs, not for timing claims.

### `pm-core`: the step function (PD-05)

```rust
pub struct Params { /* Table 19-1, intervals already in microseconds */ }
pub struct State  { /* below */ }

pub enum Event {
    Sense { chamber: Chamber, at: Micros, edges: u32 },
    Paced { chamber: Chamber, at: Micros },
    Tick  { now: Micros, activity: u8, magnet: bool, battery: Battery, link_up: bool },
    Command(Command),          // already validated
}

pub enum Pace { Keep, Withdraw, At { at: Micros, width: u32 } }

pub struct Arming {
    pub atrial: Pace,               // Keep: leave as armed; Withdraw: disarm
    pub ventricular: Pace,
    pub markers: Markers,           // at most one atrial, one ventricular, one augmentation
}

pub fn step(p: &Params, s: &State, e: Event) -> (State, Arming);
pub fn accounted(p: &Params, s: &State, instant: Micros) -> State;
pub fn validate(words: &[u32]) -> Option<Params>;
```

`accounted` is the state with every pending Pace due by `instant` applied ("Events in the order they happened"); the step applies every event to it. A parameter set travels as 29 words (`pm_core::word`): one per row of Table 19-1 in the table's units, and a last word for Rate Smoothing Down, since Rate Smoothing is two parameters (PM16.3) with one row in the source's table; the row's word is Rate Smoothing Up.

`step` is total, has no side effect, allocates nothing, loops over nothing unbounded, and uses integer arithmetic only (the rate-adaptive map of PM15.4 included: no FPU in the Therapy_Partition).

**State.** The State (Permanent, Temporary, Pace-Now, Magnet, POR) and the Mode in force with its parameters; per Chamber: the instant of the last event, whether it was a Pace, the end of the refractory period in force, the instant of the pending Pace if any, the noise tracking of the cycle; the last cardiac cycle interval (for dynamic AV delay and rate smoothing); the hysteresis flag; the PVARP-extension flag and the cycle parity that limits it (PM12.5); the ATR counter and phase; the sensor rate and its slew state.

**A Mode by its letters.** The step does not branch on 19 Modes. It reads four properties of the Mode in force: which Chambers it paces, which it senses, the response (none, triggered, inhibited, tracked), and whether the rate is sensor-driven; and the allocation of Table 19-3 as amended by Table 19-4 decides which parameters it reads.

**The timing cycle (PD-06).** With E the escape interval in force (the LRL interval; the sensor interval if shorter in a rate-adaptive Mode; the HRL interval after a Sense when hysteresis is on; each limited by rate smoothing):

| Event (non-refractory) | Modes that pace the ventricle | Atrial Modes (AXX) |
|---|---|---|
| Ventricular event at v | Ventricular Pace pending at v + E. If the Mode paces the atrium: atrial Pace pending at v + E − AV. VRP and PVARP start; blanking does not | — |
| Atrial Pace at a | Ventricular Pace pending at a + PAV (not earlier than the last ventricular event + URL interval). Ventricular Blanking starts | Atrial Pace pending at a + E. ARP starts |
| Atrial Sense at a, response D | Atrial Pace withdrawn; ventricular Pace pending at max(a + SAV, last ventricular event + URL interval) (PM10.2) | — |
| Atrial Sense at a, response I | Atrial Pace withdrawn (DDI: the ventricular Pace stays at v + E) | Atrial Pace pending at a + E (or HRL interval). ARP starts |
| Sense, response T | A Pace of that Chamber at present + `LEAD`, then as for a Pace | The same |
| Sense in a refractory or blanking period | Nothing armed or withdrawn; a Refractory-sense Marker, or none in blanking | The same |

**What implementing the cycle settled (tasks 11.1 and 11.5, 2026-11-23).**

- *An atrial event never moves the ventricular Pace later.* The table's "ventricular Pace pending at a + PAV" is "at a + PAV, or where it already is if that is sooner". After an atrial Pace on time the two are the same instant and nothing is armed; after an atrial Pace the hardware delivered late, the AV delay is shorter by the lead and the LRL interval is kept; a late atrial Sense in VDD leaves the Pace at the LRL. So Property P2 holds for the ventricle as stated, with the escape interval from the last ventricular event.
- *The arming is a difference.* The step returns, per Chamber, what the event changed in the pending Paces, taken from the `accounted` state: a new instant to arm, a withdrawal, or nothing. What accounting itself changed is armed when the accounted Pace is reported, not by the event that found it: a Chamber whose Pace has fired and is not reported is left alone (its compares may not be written during its pulse) and its report arms it from the state, and the report of an accounted ventricular Pace arms the atrial Pace that the ventricular event made pending. A generated run that keeps the compares beside the state found the first version without the last rule.
- *The hardware layer reports a moved arming.* `rearmed(state, chamber, instant)` puts the pending Pace where the driver armed it (`Armed::Ok { moved: true }`, or a forced pulse), so that `accounted` applies a Pace when it has fired and not before. The Partition calls it; the step's interface is otherwise unchanged.
- *A triggered Pace needs no new instruction.* The step asks for a Pace at the Sense's own instant, already past; the arming sequence places it one `LEAD` after the present and `rearmed` tells the state.
- *The atrium's refractory periods in these Modes* are the AV delay (PM11.6) and the PVARP (PM12.3); the ARP is the single-chamber atrial Modes' (PM12.2).

**What tasks 11.2 to 11.4 settled (2026-11-23).**

- *The cardiac cycle* whose length the dynamic AV delay uses is ventricular event to ventricular event, kept at every ventricular event, paced or sensed; there is none until the second event, and until then the delay is the fixed one. The product is taken in 64 bits. The delay after a tracked Sense is the dynamic delay less the offset and is not held to the minimum.
- *The URL holds the Paces that atrial events lead to.* The LRL's own Pace is one escape interval after the event and needs no holding where the URL interval is no longer than the LRL's, which Table 19-5 requires and `validate` will enforce (task 11.6); Property P1's contract (task 12.1) assumes it.
- *A PVC exists only where the atrium is sensed* (a choice, confirmed by the user 2026-11-24; the source's definition would make every ventricular Sense of VVI one). It is marked with the Marker PVC beside the ventricular Sense Marker, in the augmentation place.
- *The extended PVARP is the PVARP plus the PVARP Extension* (a choice, confirmed by the user 2026-11-24; the source gives the parameter a range of 50 to 400 ms and does not say whether it is added or is the whole period). The state keeps whether the present cycle's PVARP is extended, and the next ventricular event does not extend if it is: Property P8 by one flag.
- *Blanking is a flag and an instant on the ventricle*, set by an atrial Pace in a Mode that senses the ventricle and cleared by the next ventricular event; a blanked Sense returns the accounted state and nothing else, as Property P3 has it.

**What tasks 11.6, 12.1 and 12.2 settled (2026-11-23).**

- *The allocation is applied by the step, the limits by `validate`.* `validate` passes every word to its field in any Mode and refuses a set only for a value outside Table 19-1, a limit of Table 19-5 among the parameters the Mode uses, or a feature of a later stage. The step reads the Dynamic AV Delay and PVARP Extension only in the Modes that track and the Sensed AV Delay Offset only in DDD (`Mode::tracks`, `Mode::uses_sensed_av_offset`).
- *Read with the allocation, Table 19-5 has a limit that binds nowhere*: ARP < URL interval, since no Mode uses both. And the limits that name the URL bind in VDD and DDD alone. Both were put to the user and confirmed, 2026-11-24.
- *What the App_Prover holds of the step*: Properties P4 and P8 as stated; Property P3 by what the step marks (a Sense marked refractory, or given no Marker, returns the accounted state and arms nothing); of Property P2, that a pending Pace stays pending, that a tick makes one pending, and that a Chamber's own event puts it within the bounds of the escape interval from the event (exactly the LRL interval for a set without the behaviour of stage E4); of Property P1, that an atrial event leaves the ventricular Pace where it was or not before the last ventricular event plus the URL interval. The contracts are on the helpers too, so that each is carried through.
- *What it cannot be told*: that one instant is before another, in general. Instants wrap, the program compares them by a signed difference, and the App_Prover gives the cast to a signed number no meaning from 2^31 up; one direction of `is_before` is provable and the other is not. "No later than" and "no earlier than" across events therefore stay with the generated tests.

A ventricular Sense with no atrial event since the last ventricular event is a PVC (PM-glossary) and, with PVARP Extension on, lengthens the PVARP that follows, at most every second cycle.

**Later stages, in outline.** *States (E3):* `Command` and `Tick` change the State at the next ventricular event (atrial in AXX), except Pace-Now, which arms a ventricular Pace no later than 923 ms from its arrival (PM6.6); the Magnet State reads the battery level for its rate. *Rate adaptation (E4):* `Tick` moves the sensor rate toward the target of PM15.4 by a step per `control` period sized so that LRL to MSR takes the Reaction Time, and MSR to LRL the Recovery Time. *ATR (E4):* the counter of PM14.1 on each atrial interval; Duration counts ventricular events; Fallback lowers the rate linearly over the Fallback Time; switches at a ventricular event. *Battery (E3):* the Mode mapping of PM17.7 at a ventricular event. These are designed in detail at the start of their stages.

**Stage E4 as built in `pm-core` (tasks 19.1 to 19.4, 2026-11-24).** The arithmetic is in `rate.rs`, in integers: rates in millionths of a ppm, intervals in microseconds.

- *The escape interval in force* (`escape`): start from the LRL interval, or the HRL interval while hysteresis is active; take the sensor's interval if the Mode is rate-adaptive and it is shorter; take the Fallback interval if ATR is in Fallback and it is shorter; hold the result within the Rate Smoothing percentage of the cycle before; then clamp it, no shorter than the URL interval (the MSR interval in a rate-adaptive Mode) and no longer than the LRL or HRL interval it started from, the longer bound winning where the two disagree. The clamps are what the contracts of Properties P1 and P2 rest on (`proofs.md`).
- *The sensor rate*: each tick with a rate-adaptive Mode moves it toward the target (activity above the threshold, scaled by the Response Factor, capped at the MSR) by a step sized so that LRL to MSR takes the Reaction Time and MSR to LRL the Recovery Time. The activity value is an input of `Tick`; `control` passes 0 until the link delivers one.
- *Hysteresis*: active from a Sense in the Chamber that starts the LRL interval, ended by a Pace of it; only in the inhibited and tracking Modes.
- *ATR*: the counter moves at each atrial event (up for an interval shorter than the URL's, down otherwise; onset at 8), the Duration counts ventricular events, Fallback lowers the rate in a straight line to the LRL over the Fallback Time. The set in force during Fallback is VVIR (`in_force`). The phase changes at a ventricular event; the next tick finds the Mode in force changed and marks a change waiting; the Mode switches at the ventricular event after it.
- *Noise*: every service of a sensed, unblanked Chamber is recorded. A Chamber is noisy at a service outside its refractory period when every service of the cycle so far was close to the one before (more than one edge, or within PM-PAR-12 of it); the step then returns the state with the Marker Noise and no inhibition.

**From the link to the step (task 19.5, 2026-11-24).** `validate` admits every Mode of Table 19-1 with hysteresis, Rate Smoothing and ATR; the stage rule that refused them is gone. Two limits of Table 19-5 came with them: LRL ≤ MSR in the rate-adaptive Modes, and HRL ≤ LRL where hysteresis is on in the Modes that use it. Rate Smoothing is two parameters, Up (how much shorter than the cycle before an interval may be) and Down (how much longer), each Off or a percentage. The set in force (`in_force`) carries only what its Mode uses by Table 19-3: hysteresis in AAI, VVI, DDD and their rate-adaptive forms, Rate Smoothing in those and in VDD and VDDR, ATR in the tracking Modes; so the asynchronous Modes of the Magnet State and the Modes a depleted battery leaves have none of a programmed set's hysteresis or smoothing where their Mode has no use for it. The permanent set keeps the values as programmed. The activity value travels as the battery level does: the DCM's frame SET_ACTIVITY, `link`'s message ACTIVITY on `program`, `control`'s word in the Resource, the `Tick`'s input; it is taken as 0 while the link is down and after a restart of `therapy`.

### The Tasks' bodies

All three sporadic Tasks have the same shape:

```rust
pub fn sense_v(cx: rsk_ctx::SenseV) {
    cx.timing.lock(|t| {
        if let Some(s) = t.hw.sensed(Chamber::V) {
            let (state, arming) = pm_core::step(&t.params, &t.state, Event::Sense { chamber: Chamber::V, at: s.at, edges: s.edges });
            t.apply(state, arming);      // arm, disarm, push Markers to the ring
        }
    });
}
```

`pace` calls `fired()`, which reads and clears the start flags, gives the step one `Paced` event per Chamber that fired, the earlier first, and arms each of those Chambers from the pending instant in the state. `control` gives it `Command` events from the `program` Endpoint (after validation) and one `Tick`, then drains the ring into the `events` Endpoint outside the Critical_Section.

### Heart_Simulator and DCM_Stub

**Heart_Simulator** (`apps/heart-sim`, board B): an rsk Application of one Partition. A Periodic_Task of 1 ms plays a scenario table: for each Chamber, an intrinsic interval, a conduction delay, and per-beat overrides (dropped beats, premature beats, bursts for noise and for atrial tachycardia). In the responsive scenarios it reads the PG's pace lines and restarts the Chamber's intrinsic timer on a Pace, as a heart that captures. Its sense pulses are mirrored to the logic analyzer in the same register write, as `apps/hil-stimulus` does. Its 1 ms resolution is the resolution of the stimulus, not of the measurement: the analyzer measures what was actually sent.

**DCM_Stub** (`tools/dcm-stub`, host): sends command frames through the bridge board, decodes status and record frames, and holds `pm-core`'s validation so that a test can assert the PG accepted or rejected a set as predicted. It also sets the simulated battery level and activity value (PD-09).

**Rig wiring (proposal; to be measured and recorded in `hil-rig.toml`, PM29.5).** The four board-to-board lines carry the two sense lines (B to A) and the two pace lines (A to B). The analyzer needs the pace lines, which today it cannot see on the board-to-board wires: the pace pins are taken from the pins of A that already reach the analyzer, and jumpered on to B. The bridge board C connects to A's SPIM1 pins.

**The bridge's pins (wired and measured 2026-11-24; task 4.1, `rig/task_4_1_bridge_wiring.json`).** Four wires and a ground between board A and board C, on pins of the analog header that nothing on either board uses, the same pin on both boards as the rig's other board-to-board lines are; no existing wire moves. A is the SPI master (SPIM1), C the slave; C reaches the host through its own debug probe's serial port, which needs no wire.

| Signal | Board A (PG) | Direction | Board C (bridge) |
|---|---|---|---|
| SCK | P0.28 (A2) | A → C | P0.28 (A2) |
| MOSI | P0.29 (A3) | A → C | P0.29 (A3) |
| MISO | P0.30 (A4) | C → A | P0.30 (A4) |
| CSN | P0.31 (A5), unused: an input at its reset state | — | P0.31 (A5): the bridge's own chip select, moved by its pull resistors |
| Ground | GND | — | GND |

**Bursts and the stub's hold (tasks 20.2 and 20.3, 2026-11-25).** A scenario of the Heart_Simulator may hold two bursts: between two instants, a beat of the Chambers named every so many milliseconds, beside the rhythms and restarting nothing. An atrial burst faster than the URL is the atrial tachycardia of the ATR scenario; a burst of one beat a millisecond is the sense storm (the Task `beat` has a period of 1 ms, so this is the densest it gives: a 100 µs pulse every millisecond). `PM_HEART` takes the bursts as eight more numbers. The DCM_Stub's `hold:<seconds>` answers the polls and does nothing else, so that the link stays up, and with it the activity value, between two operations of a scenario. The rig driver of stage E4 is `rig/e4_runs.py`.

### The link and the bridge (tasks 15.2, 15.3; 2026-11-24)

`link` is the SPI master and exchanges 32 bytes with the bridge in every period of 50 ms; the answer to the frame of one period goes out in the next. A Job takes back the two buffers it lent the Job before, decodes, answers and starts the next transfer, which takes 0.5 ms at 500 kbps and runs after the Job has ended. Three things are as they are because of what the board and rsk allow:

- **No chip select.** An nRF52840 SPIM has none of its own, the GPIO port is `therapy`'s, and a PPI channel may not join two Partitions' peripherals, so `comms` can move no pin. The bridge frames a transfer by the clock's silence: every rise of SCK restarts a timer, 300 µs of silence end the transfer, and the bridge raises and lowers its own chip-select pin. The fourth wire is unused at the pulse generator's end.
- **The buffers are taken back with the SPIM disabled.** `rsk::DmaBuf::reclaim` asks for a stop and waits for the STOPPED event; an SPIM that is enabled and has finished its transfer gives none. `link` disables the peripheral once the END event is there (it then counts as stopped), takes the buffers, and enables it for the next transfer. A transfer that has not ended is stopped with the peripheral enabled, and answers.
- **The bridge's chip select is moved by its pull resistors.** The enabled SPIS makes its CSN pin an input whatever the GPIO direction says.

The bridge knows nothing of frames. From the host it takes 32 bytes to send once, 32 bytes to send whenever nothing else waits (the DCM_Stub's poll), or the order to send nothing (the pulse generator then reads zeros, which are no frame: this is how the loss of the link is made for task 15.4). It reports every transfer to the host with what it had sent and how many bytes were clocked.

**With the Endpoints (task 16.1).** `link` does not judge a parameter set. It answers alone what needs no judgement: IDENTITY to an interrogation (PM20.3), a STATUS with a code to each part of a set and to every command it cannot serve yet, and the last set it put together read back in parts (which is not the set in force: that read-back is open). A complete set and the commands END_TEMPORARY and PACE_NOW go to `control` as a message; `control`'s verdict comes back in its status, and `link` sends it as a STATUS frame naming the DCM's frame: accepted, declined (a valid command the State does not allow), or refused by the validation function. A change of `control`'s status without a verdict (the State, the Mode, the link) is sent as a STATUS with the code 0 in place of the next IDLE.

**A refused set (decided by the user, 2026-11-24).** `link` asks `therapy`'s validation function itself (`wire::admitted`) before it forwards a command, and answers one the function would refuse with a STATUS "refused" that names the DCM's frame; the command goes no further, nothing restarts, and temporary pacing goes on. A DCM's mistake is then no fault of any Partition. `therapy`'s function remains the authority (R42.4): a message it refuses can now only come from a `comms` that is itself at fault, and then the Kernel's rule applies as before: `comms` is restarted, `control` counts the refusal and takes the peer-state error of its Endpoints as the loss of the link (rig: task 17.2, the run of a `link` that forwards an invalid set unchecked). Until that decision `link` forwarded every set, and a refused one restarted `comms` and ended temporary pacing (finding B28).

**The loss of the link (task 15.4; PM20.2, PM-PAR-11 = 3).** `control` has the link as up from a Job that took a message from `program` until eight Jobs in a row have taken none. `link` sends one message per answered poll, every 50 ms; three unanswered polls are 150 ms without a message, and the eighth Job of 20 ms is the first that cannot come before that. So the loss is known to `control`, and told to the step in that Job's tick, between 160 and 180 ms after the Job of `link` that heard the last frame; temporary pacing then ends at the next timing event. A stop or restart of `comms` is the loss at once.

## Data Models

### Parameters and frames

`Params` holds every interval in microseconds, converted once by `validate`: an interval of a rate r ppm is 60,000,000 / r µs, rounded down (the rate is then not below r; the error is under 1 µs). The wire form of a parameter set is one word per parameter of Table 19-1 as its programmed value (ppm, ms, tenths of a volt, an index for enumerations), so that the validation function checks membership in the table and nothing is trusted about units.

A link frame is 32 bytes each way, the same form in both directions (`pm-core`, module `frame`; task 15.1): byte 0 the sender's sequence number, byte 1 the kind, byte 2 the part, byte 3 the length, bytes 4 to 27 the payload (zero beyond the length), bytes 28 and 29 zero, bytes 30 and 31 a CRC-16/CCITT-FALSE over the 30 before, high byte first. Words are sent least significant byte first. An answer names in its first payload byte the sequence number of the frame it answers. A parameter set is five parts: four of six words and one of five (four until task 19.5). A parameter set does not fit one frame and is sent as numbered parts; `link` assembles them and offers the set to `program` only when complete, and `therapy` validates the whole set (PM19.3).

### Event record

Twelve bytes, three words on the Endpoint (`pm_core::record`; tasks 22.1 and 22.2): the instant (the therapy's microseconds, the low 32 bits); a byte each for the Marker, the Chamber, the modifiers of a Pace and the augmentation Marker; and the time since the Chamber's last non-refractory record. The ring in `timing` holds 32 records. A record leaves the ring when the Endpoint has taken the batch it is in; a ring full of records not taken loses its oldest, counts the loss, and tells the count in every batch (PM18.8, PM22.4). `diag` sees a batch that did not arrive by the records' numbers. In a storm of edges on both sense lines the ring wrote 12.5 records a second over the run; the Endpoint takes 80.

As built (2026-11-25):

- **The Markers** (PM18.5): AP and VP (with "fused" for a Pace that had a Sense in the inhibition window before it), AS and VS, a Refractory sense, AT, PVC, TN. AT is an atrial Sense whose interval from the atrial event before (the one ATR counts from, a refractory one included, where ATR is on; else the Chamber's last non-refractory event) is shorter than the URL interval.
- **The modifiers of a Pace**: hysteresis, sensor rate, Rate Smoothing Up and Down. They say what made the escape interval the Pace ended what it was, by `pm_core::escape_modifiers`, which composes the interval as the step's `escape` does and is evaluated when the cycle begins. It decides nothing of the timing and has no contract; it is tested.
- **The augmentation Markers**: ATR-Dur, ATR-FB and ATR-End from the change of ATR's phase in the step, PVP→ when the PVARP is extended after a PVC. One an event, with the event's first record (Property P9).
- **The time since the Chamber's last non-refractory event** (PM18.6) is taken from the records: the instant of the Chamber's last record that was no Refractory sense and no noise. It is not taken from the step's state, which may have accounted a Pace before the Pace's report comes (finding B45).

### Records over the link (task 22.4), the data supplied (task 22.5)

A command of the DCM's for `diag` is one request; `link` answers it with a STATUS of the code REQUESTED and hands the request on, `history` answers a read in its next Job with one record, and `link` tells it as a RECORD frame in a period that has no part of a parameter read to send: the record's part in the frame's, its six words as the payload. From the DCM's frame to the RECORD is at most one period of `history` and two of `link` (200 ms). The records:

| Kind | Parts | Content |
|---|---|---|
| The four histograms | 4 each | 17 bins, five a part |
| Counts | 2 | PVCs, ATR episodes, records taken, records the ring lost, records of batches that did not arrive; resets, batches, restarts of `therapy` seen, records written to the Marker buffer |
| Trend | 65 | The duration, the content, the samples written, the stamp, the size; then two samples a part, oldest first |
| Marker | 1 | One record of the Marker transmission with its number |
| Implant data | 2 | Ten words (PM18.7): the implant date and the lead implant date as days since 2000-01-01, the lead polarity, the pacing thresholds, the P and R wave amplitudes and the lead impedances at implant, the indications. The model and the serial number are `link`'s IDENTITY |
| Measured data | 2 | The mark that the values are simulated; the battery voltage, the P and R wave amplitudes, the lead impedances, the date of the last interrogation |

**The Marker transmission** (PM20.1, PM18.5): from MARKERS_START until MARKERS_STOP `history` sends each record of the Marker buffer once, in order, one a Job, which is 10 a second; a record the buffer lost before its turn is passed over and counted. A transmission the loss of the link broke is begun again by the DCM's next MARKERS_START (PM18.4's resumption, for the Markers).

**The implant data and the measured data are supplied, not measured** (PM17.5, PM-NG-03): the DCM_Stub writes each word (SET_DATA), `diag` keeps it, and a read returns it, the measured data with its mark. Nothing in the Application measures a wave, an impedance or a voltage. The battery status level is the one `therapy` has from the link (PM17.6) and is told in every STATUS.

**At a battery level of ERT or below the measured data and the Markers are disabled** (PM17.7; finding A9, 2026-11-25). It is `link` that does it, because `link` has the level the DCM set and `diag` does not: a read of the measured data (MEASURE, or READ_RECORD of that kind), a read of a Marker and MARKERS_START are answered with a STATUS of the code DECLINED and are not handed to `diag`; the SET_BATTERY that brings the level to ERT or below also hands `diag` the end of the Marker transmission; and a record of either kind that still comes from `diag`, or was taken and not yet told, is not told and is counted (`Link::withheld`): the check is made where a record is told, with the level as it is then, so that a record taken before `control`'s status brought the level is not told either. That is what holds if the request to end the transmission is lost at a full Endpoint, and it is what `answer`'s contract states (`proofs.md`). The histograms, the counts, the trend and the implant data are read at every level, and so is the battery status level, which is in every STATUS; the battery voltage and the date of the last interrogation are words of the measured data and are not read at ERT. Back at a better level the DCM begins a transmission again by MARKERS_START. The level `link` has is 0 (BOL) after a restart of `comms` until the DCM sets it again.

### The pacing threshold test (task 22.6)

The test runs in the Temporary State (PM17.4). The DCM starts temporary pacing with the back-up Mode and rate and the start value, then names the value under test: the amplitude or the pulse width of one Chamber (THRESHOLD_START). `control` counts the cardiac cycles by the records of the Mode's timing Chamber and, after every fourth, gives the step the command `StepTemporary` with the temporary set whose value under test is one setting of Table 19-1 lower (`pm_core::threshold`); the step replaces the temporary set, in the Temporary State only, and the set is in force from the next arming. At the lowest setting the test stays there. THRESHOLD_STOP ends temporary pacing and the test; the loss of the link ends temporary pacing by the State machine, and `control` ends the test when the State is no longer Temporary. At ERT and below temporary pacing is declined, and the test with it (PM17.7).

Only the pulse width can be seen on the rig: an amplitude is a value the Application carries (PM-NG-03), and its test is the same code on another word, host-tested.

### What `diag` keeps (task 22.3)

The Partition `diag` (level D, no peripheral) has one Task, `history`, every 100 ms. Its store is words in the Task's state (`apps/pacemaker/diag/src/history.rs`), empty when all zero:

| Store | Content | When full |
|---|---|---|
| Rate histograms (PM18.1) | Four, of paced and sensed atrial and ventricular events, 17 bins: below 30 ppm, fifteen of 10 ppm from 30 to 179, and 180 or above. An event is counted at the rate of its record's interval. A Refractory sense and noise are in no histogram; AT is a sensed atrial event, PVC a sensed ventricular one | A count stops at the largest a word holds |
| Counts (PM18.1) | PVCs; ATR episodes (the augmentation Marker ATR-Dur) | The same |
| Marker buffer | The last 64 records | The oldest is discarded |
| Trend (PM18.3) | 128 samples, one a second: the pacing interval (the ventricle's last paced interval, the atrium's while the ventricle is not paced) and the sensor's interval, both or either. Continuous (the start's value): the latest samples, stamped at the last. Fixed: from its start until full, stamped at the start | Continuous discards the oldest sample, Fixed every later one |

A reset clears the four histograms and both counts together (PM18.2). The reset and the trend's start are functions of the store with host tests; the commands that call them come over the Endpoint `request` (task 22.4), as does everything that leaves `diag`: until then the store is read on the rig through the debug probe.

### Generated additions (rsk)

Generated_Config gains the PPI channel table: per channel, the event register address, the task register address, and the fork task register address or zero (PM33.4). The Task_Model records the channels so that the Config_Checker can recompute them.

## Therapy State Machine

```
              program accepted                 temp start (link up, level BOL/ERN)
   ┌─────┐ ───────────────────► ┌───────────┐ ───────────────────────────► ┌───────────┐
   │ POR │                      │ Permanent │ ◄─────────────────────────── │ Temporary │
   └─────┘ ◄─── restart ─────── └───────────┘   temp end, link lost         └───────────┘
      │  ▲                        │   ▲   │                                      │
      │  └────── (reset) ─────────┘   │   │ magnet on (response enabled)         │ Pace-Now
      │                               │   ▼                                      ▼
      │  Pace-Now                     │ ┌────────┐                          ┌──────────┐
      └───────────────────────────────┼►│ Magnet │                          │ Pace-Now │
                                      │ └────────┘                          └──────────┘
                                      │   magnet off: back to the State before    │
                                      └───────────── mode programmed ─────────────┘
```

As built (task 16.2, 2026-11-24): an event that asks for a change is remembered, and at the next event of the timing Chamber `resolve` decides the State from what is remembered, so that two requests between two events give one change. The relation `allowed` of `state.rs` is the diagram's arrows (15 of them), and `step` is proven to make no other (Property P4). Where the diagram is silent: Pace-Now is not interrupted by the magnet and drops a staged set; temporary pacing is refused while already Temporary; a set programmed during Temporary becomes the permanent one without leaving Temporary; in a Mode that paces nothing a change is made at the next tick.

**The parameters in force (tasks 16.4, 16.5; 2026-11-24).** Where the State is decided, the set in force is computed from the State's own set and kept in the state (`State::in_force`; `State::own` is the State's own set): at a battery level of ERT or below the Mode loses rate adaptation, DOO becomes VOO, and every other Mode that paces or senses both Chambers becomes VVI (PM17.7); in the Magnet State the Chambers that Mode paces are paced asynchronously (AOO, VOO, DOO, or nothing) at 100 ppm at BOL, 90 at ERN and 85 below (PM8.1, PM8.2). So the magnet, the battery level and the magnet response all take effect at a timing event, as every change of State does, and the programmed set is untouched and in force again when they are gone. Temporary pacing is neither started nor kept at ERT or below.

**Where the inputs come from.** The magnet is the pin. The battery level (PD-09) and the magnet response are commands over the link (SET_BATTERY, SET_MAGNET_RESPONSE: one byte each, forwarded by `link`, admitted by the validation function for the values 0 to 4 and 0 to 1). The magnet response is no row of Table 19-1, so it is no word of a parameter set. Both live in `therapy`'s state. After a restart of the Partition the response is on until the DCM sends it again; the battery level is kept by `link` as well, which sends it again when the Endpoints report that `therapy` has restarted (decided by the user, 2026-11-24: a restart at ERT must not bring dual-chamber pacing back).

**Below the trip level (PM7.2, PM30.3)** nothing in the step changes: `control` treats it as the loss of power it stands for. It withdraws what is armed, puts the state back to the Power-On Reset State with an empty event ring, gives the step no event while the level lasts, declines every command but a battery level, and `pace` and the sense Tasks account nothing. When the level is back the State is Power-On Reset because the state is, and the first Pace comes one Power-On Reset interval after the tick that follows. The State machine's transitions (Property P4) are those of the step and are not widened by this.

**A restart of `therapy` (task 17.1; PM23.2, PM23.3, PM28.4).** All three responses of the Partition are RESTART_PARTITION. Its statics are zero again, so the State is Power-On Reset, the programmed set is gone, and pacing resumes at 65 ppm one Power-On Reset interval after `control`'s first Job. The timer is not the Partition's memory and is as the incarnation before left it: the driver's `configure` therefore puts both start compares on the far side of the counter, drives both pins low and clears the four event flags, so that no Pace of the old incarnation fires or is reported in the new one (finding B33: without this, a restart between a pulse's end and its report made the step pace again 21 ms later). A pulse that is high at the restart is cut short, never lengthened.

**Reset (task 16.6; PM7.1, PM7.3, PM30.2).** A build of the board without a set of its own starts in the Power-On Reset State (VVI, 65 ppm, 0.5 ms) and stays there until a set comes over the link; the rig's builds that name a set are programmed with it at Init as before. The stated bound from a reset to the first Pace: the pins driven within 20 ms (the Kernel's boot and `control`'s first Job), then one Power-On Reset interval, 923.1 ms, with an interval's tolerance of 8 ms: 951.1 ms. From the reset until that first Job the pace pins are not driven at all (finding B30).

Exactly one State holds (PM6.1): it is one field of `State`. Transitions other than into Pace-Now take effect at the next ventricular event (atrial in AXX). The Magnet State remembers the State it interrupted and returns to it (PM8.3). A restart of the Therapy_Partition enters POR from any State (PD-08).

## Correctness Properties

Each property is stated for the step function unless it names hardware. "*For any*" properties are checked by the App_Prover as contracts where the column says so, and by property-based host tests over generated parameter sets and event sequences in every case.

| # | Property | Requirements | Checked by |
|---|---|---|---|
| P1 | *For any* state and event, a ventricular Pace the step arms is no earlier than the last ventricular event plus the URL interval, except a triggered Pace and the Pace-Now and Magnet rates | PM10.1 | Contract, proven as an invariant of the state that a Paced event, a Sense and a tick keep (task 12.1, in full for these since the instants have 64 bits); generated tests for the changes of State |
| P2 | *For any* reachable state in a Mode that paces a Chamber, a Pace of that Chamber is pending, at an instant no later than the Chamber's last starting event plus the escape interval in force (LRL, sensor, or HRL), unless the State or battery level disables pacing | PM9.1, PM16.1 | Contracts, proven for the ventricle and for the atrium of an atrial Mode: a Pace stays pending, its instant at the Chamber's own event, and after any event no later than it was or than the event plus the escape interval (task 12.2); generated tests for the atrial Pace of a dual-chamber Mode and the changes of State |
| P3 | *For any* Sense inside a refractory or blanking period of its Chamber, the step arms and withdraws nothing, and the state it returns is `accounted` of the state it was given: the pending Paces are unchanged but for a Pace that had already fired | PM4.4, PM4.5, PM12 | Contract |
| P4 | *For any* state, exactly one State holds, and the step changes it only by the transitions of the State machine | PM6.1 | By construction (one field); contract on transitions |
| P5 | `validate` returns a parameter set exactly for the word sequences that Table 19-1 and Table 19-5 admit | PM19.1, PM19.3, PM22.3 | Contract, proven (task 12.3); exhaustive test per parameter and per limit |
| P6 | Whenever a Chamber's start compare is armed, its end compare is the start plus the programmed width; an end compare is written only while the start is disarmed and the start's event flag is clear, and both are written in one Critical_Section | PM3.3, PM28.2, PM28.4 | The order: by the driver's types (task 12.4). The value: the register model of the host tests; rig |
| P7 | After `Hw::arm` returns `Armed::Ok`, the start compare is ahead of the counter; the loop ends within three rounds | PM27, PD-02 | The bound: App_Prover variant (task 12.4). Ahead of the counter: the register model of the host tests; rig count of late arms |
| P8 | The PVARP is extended in at most one of any two successive cycles | PM12.5 | Contract |
| P9 | *For any* event, the step produces at most one atrial, one ventricular and one augmentation Marker | PM18.6 | By the type of `Arming` |
| P10 | No Task of `therapy` panics, overflows or indexes out of bounds | PM25.2 | App_Prover |
| P11 | The Therapy_Partition's Tasks meet their deadlines whatever `comms` and `diag` do | PM22.5, PM24 | Analyzer (PASS with their Budgets enforced); rig with a misbehaving `link` |
| P12 | Every pace interval, AV delay, refractory period and pulse width on the pins is within its tolerance | PM24.3 | Rig, logic analyzer |

P2 is the central one and is the reason the pending instants are part of the state: "a Pace within the escape interval" is then an invariant one step preserves, not a statement about a sequence.

## Timing Design

The Analyzer's inputs are Table 2. What has to hold for each tolerance (the same chains with their bounds from the Analyzer's report and what the rig measured are in `evidence.md`, written from the records by `chains.py`; task 9.3):

| Requirement | Chain | Must hold | Margin (with the proposals) |
|---|---|---|---|
| AV delay, the tightest arming (PM11) | atrial event → its Job arms the ventricular Pace | After an atrial Pace: the pulse width (1.9 ms at most), R of `pace` and `LEAD`; after an atrial Sense: R of `sense_a` and `LEAD`; each under the smallest AV delay, 30 ms | 1.9 + 2 + 0.5 ms and 4 + 0.5 ms against 30 ms |
| LRL, URL, refractory (PM9, PM10, PM12) | event → Job arms or decides | The same Jobs; the next instant is at least 150 ms away | Larger |
| Triggered Pace (PM5.4) | Sense → sense Job → Pace at present + `LEAD` | R_sense + `LEAD` ≤ 8 ms | 4 ms + 0.5 ms |
| Inhibition window (PD-04) | Sense → sense Job disarms | R_sense ≤ PM-PAR-15 | The Analyzer's bound for `sense_a` is 4.359 ms against 4 ms on the stage E2 records (not held; 3.891 ms on stage E1's); measured, the window is under 0.15 ms |
| Pace-Now (PM6.6) | frame → the next transfer → `link` → `program` → `control` → armed in that Job | Up to one `link` period for the transfer, one for `link`'s Job, one `control` period, then one Pace-Now interval of 923 ms | 50 + 50 + 20 + 923 ms = 1,043 ms from the DCM's send, against two cycles plus 500 ms (1,833 ms at 90 ppm); measured in task 16.3 |
| Reset (PM7.3) | reset → boot → `control`'s first Job → armed | The pins driven within 20 ms, then one Power-On Reset interval of 923.1 ms and its tolerance | 951.1 ms; measured in task 16.6 |
| Restart of `therapy` (PM23.2) | fault, Overrun or deadline miss → the Kernel restarts the Partition → `control`'s first Job → armed | The pause from the last Pace: the interval that was running, then the restart and `control`'s first Job within 20 ms, then one Power-On Reset interval and its tolerance | the interval in force + 951.1 ms (1,796.1 ms at 71 ppm); measured in task 17.1 |
| Link loss (PM20.2) | three unanswered polls → no message on `program` for eight Jobs of `control` → the tick | 160 to 180 ms after the Job of `link` that heard the last frame (three poll periods, and up to one period of `control` and its rounding) | 180 ms; seen from outside in the transfer 200 ms after that Job (task 15.4) |
| Magnet, battery, State changes | pin or frame → `control` | One `control` period, then the next ventricular event | 20 ms + one cycle |

### What the measurement settled (tasks 9.2 and 9.8, 2026-11-23; PD-O3)

Table 2 was settled in three steps, each on the Application's own measured records.

1. **The proposal failed.** A deadline of 2 ms and a Budget of 1 ms for each sporadic Task: the Analyzer charges a Task its Budget, and with every Kernel entry of a Job added one sporadic Job cost 1.64 ms, so `pace` alone had a bound of 2.27 ms, `sense_v` 3.70 ms and `sense_a` 5.14 ms. No Budget at or above the measured execution times brings all three under 2 ms.
2. **Budgets of 400 µs and deadlines of 2, 3 and 4 ms passed, then failed.** They passed (1.561, 2.510 and 3.400 ms) while thirteen Kernel_Timing_Parameters were the Analyzer's placeholders of 300 cycles. Task 9.8 measured the fault entry, which the Analyzer charges to every Job (91.4 µs with its margin, where the placeholder was 4.7 µs), and took the stubs' cycles from rsk's record (43.8 µs, where it was 4.7 µs): the Kernel's share of a Job went from 0.48 to 0.60 ms and the bound of `sense_a` to 4.036 ms, 36 µs over its deadline.
3. **Budgets of 350 µs pass** (the user's decision of 2026-11-23, in preference to a deadline of 5 ms for `sense_a`, which would have loosened PM-PAR-15 again). Everything was built and measured again on that declaration:

| Task | Budget | Longest Job measured | Record's bound (+25 %) | Deadline | WCRT (Analyzer) | Longest response measured |
|---|---|---|---|---|---|---|
| `pace` | 350 µs | 226.1 µs | 282.6 µs | 2 ms | 1.690 ms | 0.300 ms |
| `sense_v` | 350 µs | 233.4 µs | 291.8 µs | 3 ms | 2.757 ms | 0.305 ms |
| `sense_a` | 350 µs | 234.0 µs | 292.5 µs | 4 ms | 3.891 ms | 0.306 ms |
| `control` | 5 ms | 274.2 µs | 342.8 µs | 20 ms | 9.564 ms | 0.767 ms |

On that build the Analyzer's result was PASS at a utilization of 41 % (the table above is of that build, the last of stage E1).

4. **On the stage E2 code the result is FAIL again** (task 13.1, 2026-11-23; `rig/task_9_8_analysis/report.json`, utilization 43 %; the chains built on these bounds are in `evidence.md`). Nothing was changed for it, and the user decided on 2026-11-24 that it is recorded as a FAIL due to margin:

| Task | Budget | Longest Job measured | Record's bound (+25 %) | Deadline | WCRT (Analyzer) | Longest response measured |
|---|---|---|---|---|---|---|
| `pace` | 350 µs | 248.2 µs | 310.2 µs | 2 ms | 1.925 ms | 0.334 ms |
| `sense_v` | 350 µs | 254.0 µs | 317.6 µs | 3 ms | **3.135 ms** | 0.326 ms |
| `sense_a` | 350 µs | 233.2 µs | 291.5 µs | 4 ms | **4.359 ms** | 0.358 ms |
| `control` | 5 ms | 251.5 µs | 314.3 µs | 20 ms | 10.137 ms | 0.780 ms |

The Jobs themselves are no longer than before in the single-chamber Modes (shorter: the step arms only what an event changed) and some 20 µs longer in DDD. What moved the bounds is the Kernel's share of a sporadic Job, 0.72 ms where it was 0.60 ms, and of that one measurement: the lag from a sense edge to the Kernel's timestamp of it, 2.4 µs in every earlier run, was 40.1 µs once (an edge that arrived during a Kernel entry), and the Analyzer charges that lag twice a Job, as Δ_stamp and as Δ_wake. The Job-end entry was also longer in DDD (50.1 µs against 34.2 µs). Budgets cannot turn the result this time: the record's bound for `sense_v` is 317.6 µs, so the Budget cannot go below 318 µs. Deadlines of 4 and 5 ms for the sense Tasks would, with PM-PAR-15 at 5 ms.

5. **On the stage E3 code, with instants of 64 bits, the sporadic Budgets are 450 µs and the result is FAIL by the margins** (tasks 17.2 and 12; 2026-11-24). With Budgets of 350 µs the DDD set restarted `therapy` in three of three runs (a Job of 261 µs with two of the Kernel's timer entries inside it; finding B36) and the Analyzer rejected the records; the user decided on 450 µs. Measured again (five runs of 300 s, the DCM_Stub polling throughout, so that `link` and the Endpoints run): no Job above its Budget in any run, and no event but the end of the Operating_Duration. The Analyzer accepts the records and its result is FAIL for the three sporadic Tasks, at a utilization of 52 %; by the user's standing decision a FAIL that comes from the margins is recorded and not acted on.

| Task | Budget | Longest Job measured | Deadline | WCRT (Analyzer) | Longest response measured |
|---|---|---|---|---|---|
| `pace` | 450 µs | 310.1 µs | 2 ms | **2.220 ms** | 0.384 ms |
| `sense_v` | 450 µs | 303.8 µs | 3 ms | **3.544 ms** | 0.379 ms |
| `sense_a` | 450 µs | 324.7 µs | 4 ms | **4.861 ms** | 0.400 ms |
| `control` | 5 ms | 488.0 µs | 20 ms | 10.820 ms | 1.056 ms |
| `link` | 2 ms | 539.1 µs | 50 ms | 14.030 ms | 1.457 ms |

6. **On the stage E4 code the Budgets hold and the result is FAIL by the margins again** (task 20.1; 2026-11-25). Six runs of 300 s with the DCM_Stub polling: the four sets of before, the fault-path run, and a new set `dddr-e4` (DDDR at 60 ppm, hysteresis at 50 ppm, Rate Smoothing 6 % up and down, ATR on) during which the stub raises the activity value to 255 for 100 s and returns it to 0. No Job above its Budget in any run and no event but the end of the Operating_Duration. The longest sporadic Job is 309.7 µs (`sense_a`, set `aai-90`) against 450 µs. The Analyzer accepts the records; its result is FAIL for the three sporadic Tasks at a utilization of 53 %, recorded and not acted on as before.

| Task | Budget | Longest Job measured | Deadline | WCRT (Analyzer) | Longest response measured |
|---|---|---|---|---|---|
| `pace` | 450 µs | 300.8 µs | 2 ms | 2.294 ms | 0.572 ms |
| `sense_v` | 450 µs | 300.2 µs | 3 ms | 3.617 ms | 0.375 ms |
| `sense_a` | 450 µs | 309.7 µs | 4 ms | 4.939 ms | 0.385 ms |
| `control` | 5 ms | 550.1 µs | 20 ms | 10.877 ms | 1.075 ms |
| `link` | 2 ms | 541.5 µs | 50 ms | 14.126 ms | 1.476 ms |

The code of `therapy` grew past its 32 KiB region with stage E4 and the region is 64 KiB (finding B37).

The Jobs are longer than on the stage E2 code by some 60 to 90 µs: the State machine and the set in force (tasks 16.2, 16.4), and then the 64-bit instants, which added about 50 µs to a sporadic Job on this 32-bit core (`pace` in VVI went from 218 to 279 µs between the two measurements of 2026-11-24). The bounds are the Budgets' doing: a sporadic Task is charged its Budget wherever it interferes, so 100 µs more of Budget on three Tasks moved `pace`'s bound from 1.925 to 2.220 ms. The longest response measured is 0.40 ms.

The parameters, each with what was done for it (`cargo xtask pm-measure`; the records are bound to their binaries' hashes):

| Parameter | What was done | Value used (with margin) |
|---|---|---|
| J_rel, Δ_irq, L_kernel, Δ_dispatch, Δ_timer, Δ_detect, Δ_lock_in, Δ_lock_out, Δ_complete, Δ_svc, Δ_view | Measured on this Application: builds of the variant `nrf52840_measure` programmed VVI, AAI and DDD at 90 ppm and DOO at 175 ppm, 300 s each against the `fixed` scenario; 15,000 Jobs of `control` and 374 to 1,749 of each sporadic Task per run, no Job above its Budget, no deadline missed (`rig/task_9_2_run_*.md`) | J_rel 124.8 µs (99.8 µs seen), Δ_irq 44.0 µs, L_kernel 81.7 µs (65.3 µs seen), Δ_complete 62.6 µs |
| Δ_stamp, Δ_wake | Measured on this Application: rsk's stamp probe latches GPIOTE's IN[0], which here is the atrial sense edge, so `sense_a`'s intakes give the lag from the pin to the Kernel's timestamp; 375 edges a run, all found the processor idle. 2.4 µs in every run but one, where one edge met a Kernel entry in progress. The other two sources have no probe and take the same handler | 50.2 µs each (40.1 µs seen) |
| Δ_hm, Δ_enforce, Δ_restart | Measured on an injection build of this Application (feature `inject`): `control` overruns its Budget once in four seconds and writes outside its Partition two seconds later; 75 fault entries, 75 enforcements and 150 restarts in 300 s (`rig/task_9_8_run_inject_vvi_90.md`) | 91.4 µs, 111.0 µs, 111.0 µs |
| Δ_entry | Taken from rsk's cycle test on its demonstration (`docs/hil/kernel_timing_nrf52840_cycles.json`): the stubs are the same assembly in every binary, which this Application's build check `entry_stubs` compares with the source instruction by instruction. The Analyzer's report names the record as another binary's | 43.8 µs |
| Δ_release | **Accepted as a nominal value** (the user's decision of 2026-11-23). rsk has no measurement of it either: recording a Release as due is inside the timer entry and the intake entry, whose whole Rust halves are measured as Δ_timer and Δ_irq and charged. It is charged once more per Job at the nominal 300 cycles | 4.7 µs |
| Δ_stop | A placeholder with no effect: `therapy` declares no response that stops it, and the term it enters is the larger of it and Δ_restart, which is measured and larger | — |
| Δ_fp_save, Δ_fp_restore | Placeholders with no term: no Task uses the FPU | — |
| Δ_dma_write, Δ_send, Δ_recv | Since stage E3 `link` makes EasyDMA requests and both `control` and `link` use the Endpoints; the entries are in the measured records of step 5 where `pm-measure` reads them, and placeholders otherwise (not checked entry by entry) | see the report |

Because Δ_release and the six without effect are placeholders, the Analyzer still prints that its report is "not timing evidence"; that line is expected and is what the table above accounts for.

Other things the runs settled:

- **PM-PAR-15 is 4 ms as a stated bound and under 0.15 ms as measured** (task 9.5; `rig/task_9_5_inhibition_window.json`). The 4 ms is the deadline of `sense_a`, which the analysis covered on the stage E1 code (3.891 ms) and does not on the stage E2 code (4.359 ms).
- **A Budget of 350 µs** leaves 96 µs above the longest Job of the stage E2 code (254 µs, `sense_v` in DDD; on the stage E1 code it left 116 µs), between two and three timer entries (a timer entry took up to 44.4 µs). An overrun restarts the Partition, and a Job is charged the Kernel entries that interrupt it without a switch, so its longest execution moves by an entry between runs: the Budget is the thing to watch in every later stage, and the record's bound (+25 %, 292.5 µs) is 57 µs under it.
- **Under a restart the Kernel holds off interrupts for longer**: the longest hold of the injection run was 113.2 µs against 57.3 µs in the plain runs. In stage E1 the Partition restarted is the only one, so the Jobs it would delay are the ones it ends; it becomes a blocking term for `therapy` when `comms` and `diag` exist.
- **Branch coverage of the Task entries is 73, 72, 72 and 64 %** on the stage E2 code with the DDD set added (64, 61, 61 and 55 % on the stage E1 code with two sets) for `pace`, `sense_v`, `sense_a` and `control`, over both sets' coverage runs, against the rig's threshold of 50 % (task 9.9, `rig/task_9_9_coverage.md`, which lists every branch outcome not taken). What the scenarios do not reach: the forced and too-late paths of the arming and its retries, a withdrawal (no Mode change before E3), a refused parameter set and a board that fails to configure, and the step's paths for a fusion and for a response other than inhibition.
- **The sporadic Tasks' records have 374 or 375 executions**, below the 10,000 of rsk's PAR-02 (reached only by `control`). At the heart's rate 10,000 Jobs take over two hours; that run is made when asked for.
- The measurement build has no trace pins: rsk's use P1.11 to P1.14, and P1.11 is the ventricular pace pin.

`MIT_SENSE` of 50 ms and `MIT_PACE` of 20 ms bound the sporadic Tasks' rates in the analysis whatever the heart does. An MIT of 1 ms for the sense Tasks, which would shrink the masked time, was rejected: rsk charges each Job several Kernel entries (0.60 ms a Job in this analysis), so a 1 ms MIT would by itself fill the processor.

## Error Handling

| Condition | Detected by | Outcome |
|---|---|---|
| A parameter set outside Table 19-1 or Table 19-5, or an unknown command | The validation function on `program` | The message is not delivered; the Kernel reports it against `comms` (R42.4) and restarts `comms`; `control` counts the refusal and `status` carries it; the Endpoints' peer-state error ends temporary pacing; parameters in force unchanged |
| The link lost | `control`: no frame for three `link` periods, or the peer-state error of an Endpoint to `comms` (PM23.4) | Temporary State and any test end; pacing as before |
| `comms` or `diag` faults, overruns, floods | The Kernel | Their own responses; `therapy` unaffected (P11) |
| A fault, an Overrun or a deadline miss in `therapy` | The Kernel | RESTART_PARTITION: statics zeroed, `control`'s first Job configures the hardware, POR State, VVI at 65 ppm. An armed pulse end still fires, so no pin stays high. The pause in pacing is measured (PM23.6) |
| A late arm that reached step 5 of "Arming" | `Hw::arm` | The Pace is delivered by the Job; a late-arm event is recorded and counted |
| A disarm too late | `Hw::disarm` | Fusion: both events recorded (PD-04) |
| An arming asked for while a Pace of the Chamber is unaccounted | `Hw::arm` (`TooLate`) | Nothing written; the `pace` Job at the pulse's end arms from the state |
| The clock offset not measurable at Init | `Hw::configure` | A fault of `therapy`: restart and retry |
| Event ring full | `control` | Oldest records kept, new ones dropped and counted; pacing unaffected |
| The Kernel's safe state | — | No pacing (PM23.5); pins low after any armed end |

## Testing Strategy

- **Host, `pm-core`.** Unit tests per Mode against hand-worked timing diagrams; property-based tests of P1 to P5, P8 and P9 over generated parameters and event sequences; `validate` exhaustively per parameter and for each limit of Table 19-5; the DCM_Stub's prediction against the same function.
- **App_Prover (Creusot).** P10 for the four `therapy` Tasks and the validation function; the contracts of P1 to P8 on `step`, `validate` and the register model of `Hw`.
- **QEMU.** The whole Application with scripted Senses in the verification job: the State machine, the Endpoints, the validation refusal, restart into POR, and the Analyzer's PASS on the QEMU variant.
- **Rig.** For each stage, scenarios played by the Heart_Simulator with the logic analyzer on both pace lines and both sense mirrors: every interval and width judged against its tolerance (P12); the inhibition window measured by sweeping a Sense toward a Pace; a sense storm (the Partition must not restart, pacing must be asynchronous); a restart of `therapy` (the pause); a misbehaving `link` (P11); late-arm and fusion counts reported. A timing run is at least PAR-05 long.
- **rsk's own checks.** The Generator, Profile_Lint, Link_Checker and Config_Checker accept the Application; the Analyzer reports PASS on the Application's own measured records.

## Stages

A hardware spike precedes E1, as the P1 spike preceded rsk's design: one Partition that arms TIMER3 compares, with the PPI channels written by a debug build, to confirm on the analyzer the register behaviour this design assumes (six compares on TIMER3, a compare in the past not firing, an event flag set without its interrupt enabled, counter mode with a moving compare, forks, GPIOTE overriding the pin direction) and to measure `LEAD`'s need. The audit of PM32.2 is done with it.

| Stage | Built |
|---|---|
| Spike | The register behaviour above; the audit of PM32.2 |
| rsk | Fixed PPI channels with forks; TIMER3 in the Target table (Requirement 33) |
| E1 | `pm-core` for AOO, VOO, AAI, VVI; `Hw` for both Targets; `therapy` with `pace`, the sense Tasks and a minimal `control`; the Heart_Simulator; the rig's judgement of intervals and widths |
| E2 | Dual-chamber and triggered Modes in `pm-core`; the contracts of P1 to P3 and P8 |
| E3 | `comms`, the DCM_Stub and the bridge; States; validation; restart measurement |
| E4 | Rate adaptation, hysteresis, smoothing, ATR, noise |
| E5 | `diag`, Markers on the link, histograms and trends, measured-data stand-ins |

## Requirements Impact

The design changed the requirements in these places; `requirements.md` is Draft 0.4 with them applied.

| Requirement | Change | Reason |
|---|---|---|
| PM27.2 | One therapy timer (TIMER3) for both Chambers' Paces, where the text had one timer per Chamber (TIMER1, TIMER2) | Paces of both Chambers and Sense instants have to be on one clock; TIMER1 and TIMER2 have four compare registers and TIMER3 six |
| PM28.3 | The Therapy_Partition owns GPIOTE, P0, TIMER1, TIMER2 and TIMER3 | PD-01, PD-03 |
| PM29.1 | Each sense line is the Release_Source of its Task through a timer in counter mode, not through GPIOTE's interrupt | PD-03: GPIOTE has one interrupt line, and rsk masks a line for an MIT after each event |
| PM29.2, PM29.3 | The battery level and the activity value come from the DCM_Stub over the link | PD-09: no further wires, and the Heart_Simulator stays a source of edges only |
| PM33.1 | A fixed PPI channel may name a second task (the fork) | A sense edge both captures its instant and clocks its counter |
| PM33.6, new PM33.7 | rsk's Target table gains TIMER3; the two together are the only rsk changes | PD-01 |
| PM27.5 | The compare that ends a pulse raises the timer's interrupt, and the compare that starts it does not; the text had the reverse | PD-01, PD-02: a Job released by the start could rewrite the end compare while the pin was high |
| New PM-PAR-15, PM5.5 | A Sense within PM-PAR-15 before a pending Pace need not inhibit it | PD-04 |
| PM22.6 | The point left to be shown is answered on paper: a masked line's edges are latched, not MIT_Violations; the rig's storm scenario is the demonstration | "Sense intake, MIT and noise" |

**A finding about rsk.** P0 is one peripheral with one owner, so a Partition that needs one pin configured depends on the Partition that owns the port. A per-pin ownership, or pin configuration in Generated_Config written by the Kernel at Init, would remove the dependency. Not needed for this experiment; recorded for rsk.

## Open Points of the Design

| ID | Point | When |
|---|---|---|
| PD-O1 | **Closed 2026-11-22** ("The spike's decision"). The six register behaviours are confirmed (task 1.2), the write order held under interruption (task 1.3), `LEAD` is 500 µs and `GUARD` 64 µs. What is left is named there and belongs to task 7.2 | — |
| PD-O2 | **Closed 2026-11-22** (task 8.1): the Generator accepts `therapy` with its five peripheral windows, a 32 KiB code and a 32 KiB RAM region, and the Config_Checker passes on the linked binary | — |
| PD-O3 | **Closed 2026-11-24 by the user's decision** ("What the measurement settled", step 4): on the stage E2 records the Analyzer's bounds for `sense_v` and `sense_a` are 3.135 and 4.359 ms against 3 and 4 ms; recorded as a FAIL due to margin (the longest response measured is 0.36 ms), Table 2 unchanged | — |
| PD-O4 | The rig's wiring for two sense lines, two pace lines visible to the analyzer, and the bridge. Measured 2026-11-22 (tasks 4.1): the pace pins reach the analyzer and the sense lines reach board A. The pace pulses reach the Heart_Simulator by a fork of each pace channel onto P1.03 and P1.04 ("Hardware signal flow"; the fork confirmed by the spike), and the magnet line is wired and measured (B P1.07 to A P1.07). Still open: the bridge | Bridge before E3 |
| PD-O5 | **Closed 2026-11-23** (task 12.4, "The hardware layer"): the route was not taken, for the reasons there; the order of P6 is checked by the compiler and the loops' bounds are proven, the rest stays on the model tests | — |
| PD-O6 | **Closed 2026-11-24** (tasks 15.1 to 15.3): the frames are `pm-core`'s module `frame` ("Parameters and frames"), the bridge is `tools/pm-bridge` ("The link and the bridge") | — |
| PD-O7 | **Closed in Draft 0.2.** `pace` is released by the end of its pulse, and an end compare is written only with the start disarmed and no Pace unaccounted ("Arming and the lead") | — |
