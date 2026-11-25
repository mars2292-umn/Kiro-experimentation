# What Is Proven, and Where

**PACEMAKER on rsk: an experimental Application**

Status: 2026-11-24, after wave 22 of `tasks.md` (tasks 15 and 16 complete). Kept by hand: bring it up to date when a contract is added, moved or removed. A line number in `step.rs` is that of the function's `fn` line on this date, and its contracts are the attributes directly above it; the function names are what to search for.

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
| `pm-core-proof` | `crates/pm-core/src/` | 167 |
| `pm-hw-proof` | `crates/pm-hw/src/lib.rs` | 49 |
| `pacemaker-therapy-proof` | `apps/pacemaker/therapy/src/` (QEMU board) | 67 |
| `pacemaker-therapy-nrf52840-proof` | `apps/pacemaker/therapy/src/` (nRF52840 board) | 67 |
| `pacemaker-comms-nrf52840-proof` | `apps/pacemaker/comms/src/` (nRF52840 board) | 25 |

## The therapy logic: `pm-core`

All in `crates/pm-core/src/step.rs`, member `pm-core-proof`. Each property is stated on `step` and again on every helper it calls, since the prover reasons about a call by the callee's contract; the helpers' statements are what "carries" a property to `step`.

| Property | What is proven | Stated on | Carried by | Mutant |
|---|---|---|---|---|
| **P4** The therapy State changes only by the State machine | The State the step returns is one the relation `allowed` gives from the State it was given: the diagram's 15 arrows and staying (`state.rs`, `allowed`, line 39). No event makes another change, whatever the parameters, the state and the event | `step`, line 715 | `resolve` 523 (where the State is decided, and since tasks 16.4 and 16.5 the set in force with it: `in_force` 488), `switched` 591, and every helper that leaves the State as it was: `ventricular_event` 191, `atrial_event` 237, `apply_pace` 279, `account` 295, `accounted` 315, `tick` 406, `sense` 630 | `pm-step-p4-therapy-state-changed`, `pm-step-p4-temporary-from-power-on-reset` |
| **P8** The PVARP is extended in at most one of two successive cycles | If the present cycle's PVARP and the one before were not both extended, they are not after the step | `step`, line 715 | `ventricular_event` 191 (where the extension is decided), `switched` 591, and the same helpers | `pm-step-p8-pvarp-extended-twice` |
| **P3** A refractory or blanked Sense changes nothing | A Sense the step marks as a Refractory sense, and a Sense it gives no Marker (not sensed by the Mode, or blanked), return the accounted state unchanged and neither arm nor withdraw | `sense`, line 630; `step` calls it with `accounted(p, s, at)` | `set_marker` 348, `arm_difference` 390 | `pm-step-p3-refractory-sense-withdraws` |
| **P2**, in part: a Pace is pending within the escape interval | For an event that is not a command, with no change of State waiting: (a) in a Mode that paces the ventricle, a ventricular Pace that is pending stays pending; likewise the atrial Pace in a Mode that paces the atrium alone. (b) After a tick one is pending. (c) After a Chamber's own event its Pace is pending at exactly the event plus the escape interval | (a), (b) `step` 715; (c) `ventricular_event` 191, `atrial_event` 237 | `apply_pace` 279, `account` 295, `accounted` 315, `tick` 406, `sense` 630; `Pending::at` (`state.rs` 73), `escape` 110, `ventricular_based` 159 | `pm-step-p2-ventricular-pace-not-pending` |
| **P1**, in part: no ventricular Pace within the URL interval | The ventricular Pace an atrial event leaves pending is the one that was pending, or is not before the last ventricular event plus the URL interval | `atrial_event`, line 237 | `before` and `Micros::is_before` (`time.rs` 22, 60), `Micros::plus` (`time.rs` 33) | `pm-step-p1-url-floor-dropped` |
| **P5** `validate` returns a parameter set exactly for the admitted word sequences | The result is a set exactly when the sequence satisfies `spec::admitted`: 28 words, a Mode code, every word a value of its row of Table 19-1 (`spec::member`), the limits of Table 19-5 for the parameters the Mode uses (`spec::limits`), and a set stage E2 delivers (`spec::stage`) | `validate` (`validate.rs` 352); the specification is the module `spec` (line 32) | each predicate returns what its row says (`stepped`, `is_lrl`, `is_amplitude`, `is_width`, `is_sensitivity`, `is_atr_duration`, `is_rate_smoothing`, `activity_threshold`); `limits_hold` returns `spec::limits`; `Mode::tracks`, `Mode::uses_sensed_av_offset` and `Mode::senses` (`mode.rs`); `interval_of_ppm` (`params.rs`) | `pm-validate-p5-url-row-widened`, `pm-validate-p5-limit-dropped` |
| **P10** No panic, overflow or index out of bounds | Every function of the crate, by the member proving at all: each arithmetic operation, array index and division is a goal. Since task 15.1 the crate includes the link's frames (`frame.rs`: `crc16` 116, `encode` 139, `decode` 166, `set_part` 209, `Assembly::accept` 252) and, since task 16.1, the Endpoints' messages (`wire.rs`: `message_with_set` 67, `set_of` 95, `admitted` 112) | the whole crate | bounds on `validate`'s predicates (`validate.rs` 220 to 268), `limits_hold`'s preconditions (324, 325), `interval_of_ppm` (`params.rs` 17), `Chamber::index` (`mode.rs` 25) | — |

Supporting statements about the Mode, used by the above: `Chambers::has` (`mode.rs` 64), `Mode::paces` (176), `Mode::senses` (184) and `Mode::tracks` (199) return exactly what the Mode's letters say.

**What P1 and P2 do not have.** "No later than the escape interval" and "no earlier than the URL interval" over every event are not proven. Instants wrap every 71.6 minutes and are compared by a signed difference (`Micros::since`, `time.rs` 53); the prover gives the cast to a signed number no meaning from 2^31 up, so only one direction of the comparison is provable: `before(a, b)` implies `a.is_before(b)` (`time.rs` 60). That direction is what P1's statement on `atrial_event` uses. The other statements are checked after every event by the generated run of `crates/pm-core/tests/step_dual.rs` (`generated_runs_keep_the_compares_and_the_state_together`).

## The hardware layer: `pm-hw`

In `crates/pm-hw/src/lib.rs`.

| Property | What holds | Where | Held by |
|---|---|---|---|
| **P6**, the order: an end compare is written only with the start disarmed and its event flag clear | An end compare is written in one function, `write_end` (line 281), which takes and spends a value of the type `Free` (line 232); only `disarm_and_check` (line 264) makes one, after it has put the start compare on the far side of the counter and read its event flag clear. `Free` cannot be copied, so one check allows one write | lines 232 to 285; the two callers are in `arm` (line 307) | The Rust compiler: without the witness the code does not compile. No mutant: removing it is a compile error |
| **P7**, the bound: the arming ends within three rounds | The arming loop runs at most `ROUNDS` times, the report loop twice, the sense service's loop at most `SENSE_ROUNDS` times | variants at lines 316, 394 and 430 | `pm-hw-proof`; rsk's Profile_Lint also requires each variant to equal the loop's bound annotation (`cargo xtask check`) |
| **P10** No panic, overflow or index out of bounds | Every function of the driver | the whole file | `pm-hw-proof` |

Not proven here: that the end compare is the start plus the width (P6's value), that the value written to disarm is far behind the counter, and that after `Armed::Ok` the start is ahead of the counter (P7). The reasons are in `design.md`, "The hardware layer"; the evidence is the register model of `crates/pm-hw/tests/model.rs`.

## The Therapy_Partition

In `apps/pacemaker/therapy/src/`, members `pacemaker-therapy-proof` and `pacemaker-therapy-nrf52840-proof`.

| Property | What is proven | Where |
|---|---|---|
| **P10** No Task of `therapy` panics, overflows or indexes out of bounds | The four Task bodies (`pace`, `sense_v`, `sense_a`, `control`, `lib.rs` lines 495 to 601; on the board `control` with its loop over the Endpoint `program`, bounded by `MESSAGES`, and the validation function `validate_program`, line 134) under the preconditions rsk generates for each (`rsk_spec::<task>_pre`), with everything they call: the event ring, the board layers, the driver and the step as instantiated in the Partition | both members; the board's address arithmetic has its own bounds (`board_nrf.rs` 91, 104, 113) |
| The parameter text of a rig build is read within bounds | `sets::from_text` (`sets.rs` 43), with the set's length as its loop invariant (line 51) | both members |

The board trait's methods have no contract, so these proofs hold whatever values the registers return.

## The Partition `comms`

In `apps/pacemaker/comms/src/`, member `pacemaker-comms-nrf52840-proof` (task 15.3).

| Property | What is proven | Where |
|---|---|---|
| **P10** The Task of `comms` does not panic, overflow or index out of bounds | The Task body `link` (`lib.rs` 86) under the precondition rsk generates (`rsk_spec::link_pre`), with `take_back` (70), its loop over the Endpoint `status` and the protocol it calls: `proto::answer` (`proto.rs` 210), `idle_or_part` (188), `status` (157), `put_word` (180, with its bound on the position) and `pm-core`'s frame functions as instantiated in the Partition | the member |

Nothing is proven about what `link` answers or forwards (the tests of `apps/pacemaker/comms/tests/proto.rs` and of `tools/dcm-stub` show it), nor that an encoded frame decodes to itself, nor that the validation function of `program` admits a set exactly when `validate` does (`crates/pm-core/tests/wire.rs` tests it).

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
| P1, P2: the orderings (above) | Generated run, `crates/pm-core/tests/step_dual.rs` |
| P6's value, P7's "ahead of the counter" | Register model, `crates/pm-hw/tests/model.rs` |
| P9: at most one Marker of each kind per event | The type of `Arming` (`crates/pm-core/src/event.rs`): three optional fields |
| P11: the Therapy_Partition's deadlines | The Analyzer's report, `rig/task_9_8_analysis/`: on the stage E2 records its result is FAIL (the sense Tasks' bounds over their deadlines, task 13.1; recorded as due to margin). The records are of the code before `comms` existed |
| P12: every interval, AV delay and width on the pins | The rig: `rig/task_9_4_e1_runs.json`, `rig/task_13_2_e2_runs.json`, and an hour each of VVI and DDD (`rig/task_9_4_hour_vvi90.json`, `rig/task_13_2_hour_ddd.json`) |

All mutants are in `docs/proof_mutations.toml` under their ids.
