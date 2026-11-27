# What Is Proven, and Where

**PACEMAKER on rsk: an experimental Application**

Status: 2026-11-24, after the change to instants of 64 bits (tasks 12.1 and 12.2), with tasks 15, 16 and 17 complete. Kept by hand: bring it up to date when a contract is added, moved or removed. A line number in `step.rs` is that of the function's `fn` line on this date, and its contracts are the attributes directly above it; the function names are what to search for.

This lists every property of the experiment that a prover or the compiler holds, with the place of its statement, the place of its proof, and the mutant that shows the statement is not empty. A property that is only tested is in the last section, so that the two are not confused. The properties are those of `design.md`, "Correctness Properties" (P1 to P12).

## How to run a proof

The App_Prover (Creusot) compiles the real source of each crate as a member of `proofs/creusot`. One member at a time, from that directory:

```sh
cd proofs/creusot
rm -rf verif .why3find target
cargo creusot -p <member> -- --offline          # ends with "Proved (N files)"
cargo creusot -p <member> --why3find-arg=-s -- --offline   # keeps sessions; unproved goals are in verif/<member>/<module>/<function>/why3session.xml
```

A mutant, from the workspace root: `cargo xtask mutations --only <id>` applies it to a copy of the tree and requires the prover to reject it. The contracts are attributes of the form `#[cfg_attr(creusot, creusot_std::macros::ensures(...))]` on the function they are about, so the statement of a proof is in the source file beside the code.

| Member (`-p`) | Source it proves | Files proved, 2026-11-24 |
|---|---|---|
| `pm-core-proof` | `crates/pm-core/src/` | 192 |
| `pm-hw-proof` | `crates/pm-hw/src/` | 63 |
| `pacemaker-therapy-proof` | `apps/pacemaker/therapy/src/` (QEMU board) | 86 |
| `pacemaker-therapy-nrf52840-proof` | `apps/pacemaker/therapy/src/` (nRF52840 board) | 88 |
| `pacemaker-comms-nrf52840-proof` | `apps/pacemaker/comms/src/` (nRF52840 board) | 41 |
| `pacemaker-diag-nrf52840-proof` (since 2026-11-25) | `apps/pacemaker/diag/src/` (nRF52840 board) | 26 |

## The therapy logic: `pm-core`

All in `crates/pm-core/src/step.rs`, member `pm-core-proof`. Each property is stated on `step` and again on every helper it calls, since the prover reasons about a call by the callee's contract; the helpers' statements are what "carries" a property to `step`.

| Property | What is proven | Stated on | Carried by | Mutant |
|---|---|---|---|---|
| **P4** The therapy State changes only by the State machine | The State the step returns is one the relation `allowed` gives from the State it was given: the diagram's 15 arrows and staying (`state.rs`, `allowed`, line 39). No event makes another change, whatever the parameters, the state and the event | `step`, line 1043 | `resolve` 804 (where the State is decided, and with it the set in force: `in_force` 765), `switched` 877, and every helper that leaves the State as it was: `ventricular_event` 344, `atrial_event` 486, `apply_pace` 544, `account` 564, `accounted` 588, `tick` 682, `sense` 953 | `pm-step-p4-therapy-state-changed`, `pm-step-p4-temporary-from-power-on-reset` |
| **P8** The PVARP is extended in at most one of two successive cycles | If the present cycle's PVARP and the one before were not both extended, they are not after the step | `step`, line 1043 | `ventricular_event` 344 (where the extension is decided), `switched` 877, and the same helpers | `pm-step-p8-pvarp-extended-twice` |
| **P3** A refractory or blanked Sense changes nothing | A Sense the step marks as a Refractory sense, and a Sense it gives no Marker (not sensed by the Mode, or blanked), return the accounted state unchanged and neither arm nor withdraw | `sense`, line 953; `step` calls it with `accounted(p, s, at)` | `set_marker` 621, `arm_difference` 663 | `pm-step-p3-refractory-sense-withdraws` |
| **P1** No ventricular Pace within the URL interval | **In full for the ventricle since 2026-11-24** (instants of 64 bits), restated the same day for the escape interval of stage E4. The statement about a state, `p1_inv` (line 113): a ventricular Pace that is pending is no earlier than the last ventricular event plus the URL interval. A Paced event, a Sense and a tick each keep it, when no change of State is waiting and under `p1_params` (165): no escape interval the set allows is shorter than the URL interval (`url_floor`, 152: the URL interval is not longer than the LRL interval and, **in a rate-adaptive Mode, not longer than the MSR interval**), the Mode paces the ventricle and its Paces are not triggered, a Mode with no response senses nothing; for a tick also that its instant is not before the last ventricular event | `step`, line 1043 (three statements, one per kind of event) | `escape` 237 (its result is between `shortest` 203 and `longest` 192), `ventricular_event` 344, `atrial_event` 486, `apply_pace` 544, `account` 564, `accounted` 588 (which also carry "the last ventricular event is not after the instant accounted to"), `tick` 682, `sense` 953; `Micros::plus`, `time::sum` and `time::sum_i` (`time.rs`) | `pm-step-p1-url-floor-dropped`, `pm-step-e4-escape-floor-dropped` |
| **P2** A Pace is pending within the escape interval | **For the ventricle since 2026-11-24**, restated the same day for stage E4. (a) In a Mode that paces the ventricle, a ventricular Pace that is pending stays pending through every event; likewise the atrial Pace in a Mode that paces the atrium alone. (b) After a tick one is pending. (c) After a Chamber's own event its Pace is pending no earlier than the event plus the shortest escape interval the set allows (the URL's, or the MSR's in a rate-adaptive Mode) and no later than the event plus the longest (the LRL's, or the HRL's where hysteresis makes it longer); for a set with none of the E4 behaviour (`plain`, 220), at exactly the event plus the LRL interval, as before. (d) "No later", `p2_rel` (line 124): after a Paced event, a Sense or a tick at an instant t, with no change of State waiting and the Mode's Paces not triggered, the ventricular Pace that was pending is pending no later than it was, or no later than t plus the longest escape interval; and the same of the atrial Pace in a Mode that paces the atrium alone (`p2_rel_a`, line 137) | (a), (b), (d) `step` 1043; (c) `ventricular_event` 344, `atrial_event` 486 | `apply_pace` 544, `account` 564, `accounted` 588, `tick` 682, `sense` 953; `Pending::at` (`state.rs`), `escape` 237, `ventricular_based`; `instant_of` 178; `atr_ventricular` 397 (it changes the ATR phase and neither a Chamber nor the change waiting) | `pm-step-p2-ventricular-pace-not-pending`, `pm-step-p2-tick-moves-a-pending-pace`, `pm-step-p2-ventricular-pace-a-microsecond-late`, `pm-step-e4-escape-ceiling-dropped` |
| **P5** `validate` returns a parameter set exactly for the admitted word sequences | The result is a set exactly when the sequence satisfies `spec::admitted`: 29 words, a Mode code, every word a value of its row of Table 19-1 (`spec::member`; Rate Smoothing Down has Rate Smoothing's row), and the limits of Table 19-5 for the parameters the Mode uses (`spec::limits`: since task 19.5 with LRL ≤ MSR in the rate-adaptive Modes and HRL ≤ LRL in the Modes that use hysteresis). Until task 19.5 a third predicate, `spec::stage`, refused the rate-adaptive Modes, hysteresis, Rate Smoothing and ATR; every Mode and feature is delivered now and the predicate is gone, so the admitted sequences are more, and the statement is still "exactly" | `validate` (`validate.rs` 357); the specification is the module `spec` (line 31) | each predicate returns what its row says (`stepped`, `is_lrl`, `is_amplitude`, `is_width`, `is_sensitivity`, `is_atr_duration`, `is_rate_smoothing`, `activity_threshold`); `limits_hold` returns `spec::limits`; `Mode::tracks`, `Mode::uses_hysteresis`, `Mode::uses_rate_smoothing`, `Mode::uses_sensed_av_offset` and `Mode::senses` (`mode.rs`); `interval_of_ppm` (`params.rs`) | `pm-validate-p5-url-row-widened`, `pm-validate-p5-limit-dropped`, `pm-validate-p5-sensor-limit-dropped`, `pm-validate-p5-hysteresis-limit-in-every-mode` |
| **P10** No panic, overflow or index out of bounds | Every function of the crate, by the member proving at all: each arithmetic operation, array index and division is a goal. Since task 15.1 the crate includes the link's frames (`frame.rs`: `crc16` 116, `encode` 139, `decode` 166, `set_part` 209, `Assembly::accept` 252) and, since task 16.1, the Endpoints' messages (`wire.rs`: `message_with_set` 67, `set_of` 95, `admitted` 112) | the whole crate | bounds on `validate`'s predicates (`validate.rs` 220 to 268), `limits_hold`'s preconditions (324, 325), `interval_of_ppm` (`params.rs` 17), `Chamber::index` (`mode.rs` 25) | — |

Supporting statements about the Mode, used by the above: `Chambers::has` (`mode.rs` 64), `Mode::paces` (176), `Mode::senses` (184) and `Mode::tracks` (199) return exactly what the Mode's letters say.

**Stage E4 and the escape interval (2026-11-24).** Until stage E4 the escape interval was the LRL interval and the contracts said so. It is now the interval in force (`escape`): the LRL's, the HRL's under hysteresis, the sensor's in a rate-adaptive Mode, the Fallback's under ATR, held by Rate Smoothing, and clamped last between the shortest and the longest interval the set allows. The contracts state those two bounds for every set and the exact LRL interval for a set with none of this (`plain`). Three things are weaker than the statements they replace, each because the old statement is false of the new behaviour: the instant after a Chamber's own event is bounded, not exact, for a set with E4 behaviour; P2's bound under hysteresis is the HRL interval; and P1 has a supposition more in a rate-adaptive Mode (URL interval not longer than the MSR interval), without which the sensor paces faster than the URL as the source intends. What the bounds do not say: which of the intervals is in force at an event (that the sensor's rate, the smoothing percentage or the Fallback line is followed is the host tests', `crates/pm-core/tests/rate.rs`). ATR, the sensor step and the noise rule have no contract of their own beyond freedom from overflow (P10).

**The MSR and the URL (2026-11-25).** P1 supposes `url_floor`: no escape interval the set allows is shorter than its URL interval, which in a rate-adaptive Mode asks that the MSR interval is not. On the user's decision `validate` now refuses a set of VDDR or DDDR, the two rate-adaptive Modes the URL limits, whose MSR is above its URL: the limit is part of `spec::limits` and so of P5, with the mutant `pm-validate-p5-msr-above-url-admitted` and a host test over every pair of values that also compares the two intervals of each set returned. That `validate`'s result satisfies `url_floor` is not stated as a contract: it follows from P5's limits (MSR ≤ URL, LRL ≤ URL) through the conversion of a rate to its interval, and is tested, not proven. In the rate-adaptive Modes that do not track the supposition stays one.

**ATR's change of Mode and stage E5 (2026-11-25).** The Mode now changes at the ventricular event at which Fallback begins or ends (finding B41): the tick foresees the event and marks a change as waiting, and the event goes through `switched`. No contract changed and none states ATR: P1 and P2 are stated for events at which no change is waiting, as before, and the events of ATR's changes are now among those at which one is. The event record of stage E5 (`record.rs`, `escape_modifiers`, and in `therapy` the ring, the batch and the Marks) is proven free of run-time errors with the rest of the two crates and has no contract of its own but `record::word1`'s. `diag` (level D) has no member. Tasks 22.4 to 22.6 added, under the same proof and with no contract of their own, the requests and records (`record.rs`), the threshold test's settings (`threshold.rs`), the step's command `StepTemporary` (which keeps the State: P4 is proven over it with the rest), the link's handling of them in `comms`, and `control`'s count of the test's cycles. The file counts above are of 2026-11-25, each member proven with each pair of solvers alone; `pm-hw-proof` was last proven by the job of the same day, before these changes, which do not touch it. **Later the same day** this paragraph's "no contract of their own" and "`diag` has no member" stopped being true: the records, the threshold test's settings, the frames, `escape_modifiers`, the noise rule and the ERT rule have the contracts of the sections below, `diag` has the member `pacemaker-diag-nrf52840-proof`, and all six members, `pm-hw-proof` among them, were proven again with each pair of solvers alone, with the 40 mutants of this experiment each killed.

**The noise rule, in part (PM13.1, PM13.2; finding B42; contracts of 2026-11-25).** Until this date no contract stated the noise rule, and the defect of finding B42 passed every proof before and after its fix. Now stated and proven:

| What is proven | Stated on | Mutant |
|---|---|---|
| A ventricular event begins a noisy cycle of the ventricle and, in the ventricular-based cycle, of the atrium too (the fix of B42) | `ventricular_event` | `pm-step-b42-atrial-cycle-not-begun-noisy` |
| A service is found noisy only in a cycle still noisy, only of a Chamber the Mode senses, and only when it reports several edges or a service came before it; the cycle then stays noisy; and no service makes noisy a cycle that was not | `serviced` | `pm-step-noise-found-in-a-quiet-cycle` |
| A service found noisy arms nothing and withdraws nothing: the pending Paces stay (PM13.2, asynchronous pacing); the noise Marker is given only of a Chamber the Mode senses, and no Sense gives it | `step`; `sense` | `pm-step-noise-withdraws-the-pace` |

Not proven of the noise rule: the time. "Close" is within PM-PAR-12 of the service before, and "after the refractory period" is by the instants; no contract states either, nor that a cycle with one service that is not close is quiet from then on to its end in terms of what is paced. The host tests (`tests/rate.rs`) and the rig's storms have those.

**The modifiers of a Pace (PM18.5; contract of 2026-11-25).** `escape_modifiers` returns one of the nine values the Markers have (hysteresis or the sensor or neither, with Rate Smoothing Up or Down or neither); none for a set without stage E4's behaviour; and each modifier only of what it names: hysteresis of an HRL interval in force, the sensor of a rate-adaptive Mode with a sensor interval, Rate Smoothing of a percentage programmed and a cycle known. Mutant `pm-step-modifier-sensor-without-adaptation`. Not proven: that the modifiers agree with the interval `escape` returns (the two functions compose the interval in the same order, each on its own).

**The records, the threshold test and the frames (contracts of 2026-11-25).**

| What is proven | Stated on | Mutant |
|---|---|---|
| The parts each kind of record has (`parts_of`), and that `admitted`, `comms`'s validation function for `diag`'s records, admits exactly a known kind with a part it has and nothing above (`admitted_word`); the fields of a record's word 1 | `record.rs`: `parts`, `admitted`, `kind_of`, `chamber_of`, `modifiers_of`, `augmentation_of` | `pm-record-trend-a-part-short`, `pm-record-admitted-part-unchecked` |
| The next lower pulse width and amplitude are lower, are settings of Table 19-1, are at most one step of the table away, and are none exactly at or below the lowest (PM17.4) | `threshold.rs`: `lower_width`, `lower_amplitude` | `pm-threshold-width-skips-settings`, `pm-threshold-amplitude-below-the-lowest` |
| The set a step of the threshold test gives differs from the set before in the value under test alone, which is lower; the Mode, the rates and the refractory periods are the same | `threshold.rs`: `step_down` | `pm-threshold-step-changes-the-other-chamber` |
| A frame `decode` returns is well formed (a kind of the 27, a length within the payload, a part its kind may have), and its fields and payload are the bytes', with zero beyond the length and in bytes 28 and 29. What `encode` returns has the frame's fields and payload byte for byte, zero beyond the length. `kind_known` is the list `known` | `frame.rs`: `decode`, `encode`, `kind_known`, `set_part` | `pm-frame-decode-part-unchecked`, `pm-frame-decode-bytes-beyond-the-length`, `pm-frame-kind-known-one-too-few` |

Not proven: that what is encoded decodes to itself. The two contracts give it but for the CRC, which `decode` computes again over the same bytes; nothing states that the CRC of the same bytes is the same, nor what the CRC detects. Tested instead, over every one of the 256 kinds with every part and every length (`tests/frame.rs`, `the_round_trip_over_every_kind_part_and_length`), and the other way over 200,000 blocks (`what_is_decoded_encodes_to_the_same_bytes`). That the protocol serves no kind the decoder refuses, which was the fault of task 22.4, is a test of `comms` (`every_kind_the_protocol_serves_is_one_the_decoder_knows`).

**How the orderings became provable, and what P1 and P2 still do not have.** Until 2026-11-24 an instant had the timer's 32 bits and two instants were compared by the sign of their difference; the prover gives the cast to a signed number no meaning from 2^31 up, so only equalities and one direction of a comparison could be proven. An instant now has 64 bits, counts from the start of the Partition and does not wrap (`time.rs`): `is_before` is a comparison of integers with a contract in both directions, and a sum saturates at the top of the range, which the contracts name through `time::sum`. The timer's 32 bits are the hardware layer's (below). Not stated or proven: the events at which a change of State comes into force (the set in force changes there; `switched` starts the new Mode's cycle from the event); commands (after Pace-Now the bound runs from the next tick); the atrial Pace of a dual-chamber Mode, which hangs on the ventricular one by the AV delay; The suppositions of P1 are stated, not discharged: `validate` checks the URL against the LRL for VDD and DDD (and P5 is its proof), and that a tick is not before the last ventricular event is a fact about time no contract of the hardware layer gives. When the hardware layer moves an arming, the Partition tells the state through `rearmed`, outside the step: the Pace is then later by at most the lead (500 µs). The generated runs of `crates/pm-core/tests/step_dual.rs` and `step.rs` go on checking both properties after every event, the changes of State included.

## The hardware layer: `pm-hw`

In `crates/pm-hw/src/lib.rs`.

| Property | What holds | Where | Held by |
|---|---|---|---|
| **P6**, the order: an end compare is written only with the start disarmed and its event flag clear | An end compare is written in one function, `write_end` (line 324), which takes and spends a value of the type `Free` (line 248); only `disarm_and_check` (line 307) makes one, after it has put the start compare on the far side of the counter and read its event flag clear. `Free` cannot be copied, so one check allows one write | lines 248 to 328; the two callers are in `arm` (line 373) | The Rust compiler: without the witness the code does not compile. No mutant: removing it is a compile error |
| **P7**, the bound: the arming ends within three rounds | The arming loop runs at most `ROUNDS` times, the report loop twice, the sense service's loop at most `SENSE_ROUNDS` times | variants at lines 384, 464 and 500; the start-up loop of `configure` (task 17.1) at 352 | `pm-hw-proof`; rsk's Profile_Lint also requires each variant to equal the loop's bound annotation (`cargo xtask check`) |
| **P10** No panic, overflow or index out of bounds | Every function of the driver, with the boundary between the therapy's 64-bit instants and the timer's 32 bits: `present` (line 279, the present moved on by what the counter advanced), `extend` (292, the instant of a counter value near the present) and `count_of` (211, an instant's low 32 bits); the counter's own type is `Count` (`count.rs`) | the two files | `pm-hw-proof` |

Not proven here: that `extend` and `count_of` agree (an instant's counter value is its low 32 bits, and a counter value's instant has that value: the model tests show it across the counter's wrap), that the end compare is the start plus the width (P6's value), that the value written to disarm is far behind the counter, and that after `Armed::Ok` the start is ahead of the counter (P7). The reasons are in `design.md`, "The hardware layer"; the evidence is the register model of `crates/pm-hw/tests/model.rs`.

## The Therapy_Partition

In `apps/pacemaker/therapy/src/`, members `pacemaker-therapy-proof` and `pacemaker-therapy-nrf52840-proof`.

| Property | What is proven | Where |
|---|---|---|
| **P10** No Task of `therapy` panics, overflows or indexes out of bounds | The four Task bodies (`pace`, `sense_v`, `sense_a`, `control`, `lib.rs` lines 594 to 733, with the parts of `control`'s body, lines 452 to 590; on the board `control` with its loop over the Endpoint `program`, bounded by `MESSAGES`, and the validation function `validate_program`, line 141) under the preconditions rsk generates for each (`rsk_spec::<task>_pre`), with everything they call: the event ring, the board layers, the driver and the step as instantiated in the Partition | both members; the board's address arithmetic has its own bounds (`board_nrf.rs` 91, 104, 113) |
| The parameter text of a rig build is read within bounds | `sets::from_text` (`sets.rs` 43), with the set's length as its loop invariant (line 51) | both members |

The board trait's methods have no contract, so these proofs hold whatever values the registers return.

**Below the trip level (PM7.2, PM30.3; contracts of 2026-11-25).** Until this date `control`'s behaviour below the trip level was tested on the rig only.

| What is proven | Stated on | Mutant |
|---|---|---|
| The first Job below the level leaves the state off (`off`: the Power-On Reset State, nothing programmed, no Pace pending in either Chamber), the ring empty, and the level marked; a later one changes nothing | `trip` | `pm-therapy-trip-keeps-the-state` |
| While the level is marked, a Job of `pace` and a Job of a sense Task change nothing at all | `Timing::on_pace`, `Timing::on_sense` | `pm-therapy-pace-accounted-below-the-trip-level` |
| Once the Partition has started, a Job of `control` keeps `tripped_off` (the state is off whenever the level is marked), and the Job in which the level is first marked ends with the state off and the ring empty | `Timing::on_control`, with `heard`, `taken` and `threshold` leaving the mark alone and `served` (the Job above the level, a function of its own since 2026-11-27, findings B49) clearing it | `pm-therapy-control-steps-below-the-trip-level` |

So by the three Tasks' contracts together, from the Partition's start the state is off for as long as the level is marked, and the step is given no event. Not proven: that the hardware has nothing armed. `trip` calls `disarm` for both Chambers, but the hardware trait has no contract, so the proof does not see what `disarm` does; a pulse already begun ends at its own end compare. That part stays the rig's (`e3_states.py battery`).

## The Partition `comms`

In `apps/pacemaker/comms/src/`, member `pacemaker-comms-nrf52840-proof` (task 15.3).

| Property | What is proven | Where |
|---|---|---|
| **P10** The Task of `comms` does not panic, overflow or index out of bounds | The Task body `link` (`lib.rs` 86) under the precondition rsk generates (`rsk_spec::link_pre`), with `take_back` (70), its loop over the Endpoint `status` and the protocol it calls: `proto::answer` (`proto.rs` 210), `idle_or_part` (188), `status` (157), `put_word` (180, with its bound on the position) and `pm-core`'s frame functions as instantiated in the Partition | the member |

**The battery level and the records (PM17.7, finding A9; contracts of 2026-11-25).**

| What is proven | Stated on | Mutant |
|---|---|---|
| At a battery level of ERT or below (`level_of`: the worse of the DCM's last and the one in `control`'s status, as it was when the frame came), a RECORD frame is sent only of a record the level does not disable (neither a Marker nor the measured data) | `proto::answer`, `idle_or_part`; `level`, `disabled_at_ert` | `pm-comms-ert-record-told-at-ert`, `pm-comms-ert-marker-not-disabled`, `pm-comms-level-ignores-status` |
| At that level no request the level disables is put for `diag` (a read of the measured data or of a Marker, the start of the Marker transmission); one waiting from before stays as it was | `proto::answer`; `asks_disabled` | `pm-comms-ert-request-handed-on` |
| At that level a disabled record that comes from `diag` does not become the record waiting | `proto::record_in` | |

Not proven: that the RECORD frame's payload is the waiting record's words (the kind is taken from the state, not from the frame's bytes); that the SET_BATTERY which brings the level to ERT ends a Marker transmission (tested). Otherwise nothing is proven about what `link` answers or forwards (the tests of `apps/pacemaker/comms/tests/proto.rs` and of `tools/dcm-stub` show it), nor that the validation function of `program` admits a set exactly when `validate` does (`crates/pm-core/tests/wire.rs` tests it).

## The Partition `diag`

In `apps/pacemaker/diag/src/`, member `pacemaker-diag-nrf52840-proof` (2026-11-25; until then `diag` had no proof).

| Property | What is proven | Where | Mutant |
|---|---|---|---|
| **P10** The Task of `diag` does not panic, overflow or index out of bounds | The Task body `history` under the precondition rsk generates (`rsk_spec::history_pre`), its loop over the Endpoint `events` bounded by `BATCHES`, and everything of `history.rs` | `lib.rs`, `history.rs` | `pm-diag-fixed-trend-written-when-full` |
| What `diag` sends, `comms` admits (R42.4) | A request is answered only if it is a read, with the part of the record it named, which satisfies `pm_core::record::admitted_word`; a record of the Marker transmission is a Marker record, sent only while the transmission runs and one is waiting, and nothing changes when none is sent | `History::request`, `read`, `next_marker` | `pm-diag-read-names-another-part` |
| The measured data is marked as simulated (PM17.5) | Part 0 of the measured data has `SIMULATED` as its first word | `History::read`, `request` | `pm-diag-measured-not-marked-simulated` |
| The histograms' bins (PM18.1) | `bin` returns a bin of the histogram: the first below 30 ppm, the last from 180 ppm and for no interval, ten pulses a minute to a bin between | `bin` | `pm-diag-bin-a-rate-low` |
| The reset (PM18.2) | Every bin of every histogram and both counts are zero after it; the Marker buffer, the trend and the supplied data are as they were | `History::reset_histograms` | `pm-diag-reset-leaves-the-atr-count` |
| The trend (PM18.3) | A trend begun is empty, stamped, of a duration and content the trend has; a Fixed trend never holds more than its storage and is not written once full | `History::start_trend`, `job` | `pm-diag-fixed-trend-written-when-full` |
| The Marker buffer and the transmission (PM18.5, PM20.1) | A record taken is the buffer's newest; a batch is counted and its sequence number known; the transmission is switched by its request alone and begins at the next record | `History::record`, `take`, `request` | `pm-diag-stream-from-the-buffer-s-first` |

Not proven of `diag`: which histogram and which count a record is counted in (`record`'s contract states the buffer and what is left alone, not the counting); the loss counted from the sequence numbers; the samples a read of the trend returns and their order; the words a read of a histogram, the counts or the supplied data returns beyond word 0 (the host tests of `apps/pacemaker/diag/tests/history.rs` and the rig's comparison of the store with the capture have these). The misbehaving build (`misbehave`) is not the member's.

## The change to rsk: fixed PPI channels

Proven with rsk's own provers, by rsk's verification job. The job last ran on 2026-11-24, on the tree before waves 19 and 20 (it has not been run since; the App_Prover's members of this experiment were proven one at a time after wave 22, with the file counts above, and the six mutants of the step were run again on the changed step, and Verus was run on `rsk-confcheck` alone after its change of task 16.1: 113 verified): `cargo xtask verify` on this tree **passed: 0 violations**, all 31 steps, 95 minutes. Among them: the Kernel_Proofs (Verus; the proofs of `rsk-logic` twice, 348 verified in both), 24 Kani harnesses of `rsk-kernel` (57 minutes, the model checker at up to 25 GB), the App_Prover on every member under both pairs of solvers, the four of this experiment included (`pm-core` 127 files, `pm-hw` 49, `therapy` 60, `therapy-nrf52840` 56), the Lean project, the Profile_Conformance_Suite, the clean-checkout rebuild, rsk\x27s own rig suites on the boards (self-test, spike, response, isolation with the Hardware_Model tests, boot checks, cycle attribution, stack painting) and the step that runs this Application on QEMU. The job gave two warnings: the committed UPPAAL corpus report is of other sources than this tree\x27s, so that item is inconclusive (UPPAAL was not run, by the user\x27s standing instruction); and Verus\x27s count of executable lines (5,123) is above the Build_System\x27s (4,598), for review. 109 of rsk\x27s 386 acceptance criteria have no Evidence_Item in the store, as before.

| What | Where | Prover | Mutants |
|---|---|---|---|
| The Kernel writes exactly the declared channels: `ppi_write` leaves the registers as the specification `written` gives them; `ppi_boot` writes only if the checksum over the tables and the channel table is the recorded one and the table fits, and otherwise writes nothing | `crates/rsk-kernel/src/arch/cm4/ppi.rs`: `ppi_write` (line 99), `ppi_checksum` (135), `ppi_boot` (159) | Verus (`cargo xtask verify`, or the crate alone) | `ppi-task-register-given-the-event`, `ppi-fork-register-of-another-channel`, `ppi-one-enable-bit-too-many`, `ppi-written-under-a-wrong-checksum` |
| The same write on the raw register stub | `crates/rsk-kernel/src/arch/cm4/unsafe_module/kani.rs`, harness `ppi_write_sets_exactly_the_declared_channels` (line 1258) | Kani | `kani-ppi-fork-register-of-another-channel` |

The Config_Checker's recomputation of the channel table from the Task_Model (`crates/rsk-confcheck/src/recompute.rs`, `expected_ppi`, line 55) is a check on every build, not a proof.

## Properties that are not proven

| Property | Its evidence instead |
|---|---|
| P1, P2: what is left (above: changes of State, commands, the atrium of the dual-chamber Modes) | Generated runs, `crates/pm-core/tests/step_dual.rs` and `step.rs` |
| P6's value, P7's "ahead of the counter" | Register model, `crates/pm-hw/tests/model.rs` |
| P9: at most one Marker of each kind per event | The type of `Arming` (`crates/pm-core/src/event.rs`): three optional fields |
| P11: the Therapy_Partition's deadlines | The Analyzer's report, `rig/task_9_8_analysis/`: on the stage E2 records its result was FAIL (the sense Tasks' bounds over their deadlines, task 13.1; recorded as due to margin). On the stage E3 records (task 17.2) it rejects the input: `pace`'s longest Job with the margin is above its Budget (finding B36). The rig's evidence: pacing judged through a faulting, overrunning, flooding and stopped `link` (`rig/task_17_fault_responses.json`) |
| P12: every interval, AV delay and width on the pins | The rig: `rig/task_9_4_e1_runs.json`, `rig/task_13_2_e2_runs.json`, and an hour each of VVI and DDD (`rig/task_9_4_hour_vvi90.json`, `rig/task_13_2_hour_ddd.json`) |

All mutants are in `docs/proof_mutations.toml` under their ids.
