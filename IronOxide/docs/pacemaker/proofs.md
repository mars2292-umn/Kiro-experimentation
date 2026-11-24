# What Is Proven, and Where

**PACEMAKER on rsk: an experimental Application**

Status: 2026-11-23, after the sixteenth wave of `tasks.md`. Kept by hand: bring it up to date when a contract is added, moved or removed. Line numbers are those of this date; the function names are what to search for.

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

| Member (`-p`) | Source it proves | Files proved, 2026-11-23 |
|---|---|---|
| `pm-core-proof` | `crates/pm-core/src/` | 127 |
| `pm-hw-proof` | `crates/pm-hw/src/lib.rs` | 49 |
| `pacemaker-therapy-proof` | `apps/pacemaker/therapy/src/` (QEMU board) | 60 |
| `pacemaker-therapy-nrf52840-proof` | `apps/pacemaker/therapy/src/` (nRF52840 board) | 56 |

## The therapy logic: `pm-core`

All in `crates/pm-core/src/step.rs`, member `pm-core-proof`. Each property is stated on `step` and again on every helper it calls, since the prover reasons about a call by the callee's contract; the helpers' statements are what "carries" a property to `step`.

| Property | What is proven | Stated on | Carried by | Mutant |
|---|---|---|---|---|
| **P4** The therapy State changes only by the State machine | The State the step returns is the one it was given (no event changes it until stage E3) | `step`, line 537 | `ventricular_event` 181, `atrial_event` 222, `apply_pace` 267, `account` 282, `accounted` 300, `tick` 391, `sense` 456 | `pm-step-p4-therapy-state-changed` |
| **P8** The PVARP is extended in at most one of two successive cycles | If the present cycle's PVARP and the one before were not both extended, they are not after the step | `step`, line 538 | `ventricular_event` 182 (where the extension is decided), and the same helpers | `pm-step-p8-pvarp-extended-twice` |
| **P3** A refractory or blanked Sense changes nothing | A Sense the step marks as a Refractory sense, and a Sense it gives no Marker (not sensed by the Mode, or blanked), return the accounted state unchanged and neither arm nor withdraw | `sense`, lines 460 and 461; `step` calls it with `accounted(p, s, at)` (line 585) | `set_marker` 336, `arm_difference` 379 | `pm-step-p3-refractory-sense-withdraws` |
| **P2**, in part: a Pace is pending within the escape interval | (a) In a Mode that paces the ventricle, a ventricular Pace that is pending stays pending through every event; likewise the atrial Pace in a Mode that paces the atrium alone. (b) After a tick one is pending. (c) After a Chamber's own event its Pace is pending at exactly the event plus the escape interval | (a) `step` 539, 540; (b) `step` 541; (c) `ventricular_event` 183, `atrial_event` 225 | `apply_pace` 269, `account` 284, `accounted` 302, `tick` 393, `sense` 458; `Pending::at` (`state.rs` 46), `escape` 104, `ventricular_based` 153 | `pm-step-p2-ventricular-pace-not-pending` |
| **P1**, in part: no ventricular Pace within the URL interval | The ventricular Pace an atrial event leaves pending is the one that was pending, or is not before the last ventricular event plus the URL interval | `atrial_event`, line 229 | `before` and `Micros::is_before` (`time.rs` 22, 60), `Micros::plus` (`time.rs` 33) | `pm-step-p1-url-floor-dropped` |
| **P10** No panic, overflow or index out of bounds | Every function of the crate, by the member proving at all: each arithmetic operation, array index and division is a goal | the whole crate | bounds on `validate`'s predicates (`validate.rs` 99 to 142), `limits_hold`'s preconditions (197, 198), `interval_of_ppm` (`params.rs` 15), `Chamber::index` (`mode.rs` 23) | — |

Supporting statements about the Mode, used by the above: `Chambers::has` (`mode.rs` 63) and `Mode::paces` (`mode.rs` 163) return exactly what the Mode's letters say.

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
| **P10** No Task of `therapy` panics, overflows or indexes out of bounds | The four Task bodies (`pace`, `sense_v`, `sense_a`, `control`, `lib.rs` lines 316 to 379) under the preconditions rsk generates for each (`rsk_spec::<task>_pre`), with everything they call: the event ring, the board layers, the driver and the step as instantiated in the Partition | both members; the board's address arithmetic has its own bounds (`board_nrf.rs` 84, 97, 106) |
| The parameter text of a rig build is read within bounds | `sets::from_text` (`sets.rs` 43), with the set's length as its loop invariant (line 51) | both members |

The board trait's methods have no contract, so these proofs hold whatever values the registers return.

## The change to rsk: fixed PPI channels

Proven with rsk's own provers, by rsk's verification job, which has **not run** since the change (task 3.7); each was run by itself when it was written.

| What | Where | Prover | Mutants |
|---|---|---|---|
| The Kernel writes exactly the declared channels: `ppi_write` leaves the registers as the specification `written` gives them; `ppi_boot` writes only if the checksum over the tables and the channel table is the recorded one and the table fits, and otherwise writes nothing | `crates/rsk-kernel/src/arch/cm4/ppi.rs`: `ppi_write` (line 99), `ppi_checksum` (135), `ppi_boot` (159) | Verus (`cargo xtask verify`, or the crate alone) | `ppi-task-register-given-the-event`, `ppi-fork-register-of-another-channel`, `ppi-one-enable-bit-too-many`, `ppi-written-under-a-wrong-checksum` |
| The same write on the raw register stub | `crates/rsk-kernel/src/arch/cm4/unsafe_module/kani.rs`, harness `ppi_write_sets_exactly_the_declared_channels` (line 1258) | Kani | `kani-ppi-fork-register-of-another-channel` |

The Config_Checker's recomputation of the channel table from the Task_Model (`crates/rsk-confcheck/src/recompute.rs`, `expected_ppi`, line 55) is a check on every build, not a proof.

## Properties that are not proven

| Property | Its evidence instead |
|---|---|
| P1, P2: the orderings (above) | Generated run, `crates/pm-core/tests/step_dual.rs` |
| P5: `validate` accepts exactly the admitted sets | Exhaustive tests, `crates/pm-core/tests/validate.rs`; the proof is task 12.3 |
| P6's value, P7's "ahead of the counter" | Register model, `crates/pm-hw/tests/model.rs` |
| P9: at most one Marker of each kind per event | The type of `Arming` (`crates/pm-core/src/event.rs`): three optional fields |
| P11: the Therapy_Partition's deadlines | The Analyzer's report, `rig/task_9_8_analysis/` (of the stage E1 binaries) |
| P12: every interval and width on the pins | The rig, `rig/task_9_4_e1_runs.json` (stage E1) |

All mutants are in `docs/proof_mutations.toml` under their ids.
