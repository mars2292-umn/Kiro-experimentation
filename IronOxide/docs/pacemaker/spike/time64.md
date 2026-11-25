# Instants of 64 bits: can Properties P1 and P2 be proven in full?

**PACEMAKER on rsk: an experimental Application**

Status: an investigation of 2026-11-24, asked for by the user, **and since the same day made in the Application** on the user's decision (`crates/pm-core/src/time.rs`, the driver's boundary in `crates/pm-hw`, `proofs.md`); this text is kept as the record of the investigation and of its estimate. When it was written nothing of it was in the Application: it was done in a copy of `crates/pm-core` with its own proof workspace. The copy's changes are `time64.patch` beside this file (it applies to `crates/pm-core/src` as committed on that date).

## The answer

Yes. With an instant that does not wrap, "before" is a comparison of integers, and the App_Prover proves the orderings it could not be told before. In the copy:

1. **The whole of `pm-core` proves with no change but the type**: 166 files, after three contracts that spoke of a sum of 32-bit numbers were restated as sums of integers.
2. **P1 is proven in full on the step**, as an invariant every event keeps: a ventricular Pace that is pending is no earlier than the last ventricular event plus the URL interval.
3. **P2's "no later" is proven on the step**: after any event at an instant t, in a Mode that paces the ventricle, the ventricular Pace that was pending is still pending, and is no later than it was or no later than t plus the escape interval.
4. Three mutants, one against each statement, are refused by the prover.

What that took in the copy: the module `time.rs` rewritten (110 lines), and 26 contract lines added to `step.rs`. No line of the step's code changed.

## What was the obstacle, and why this removes it

The therapy timer has 32 bits and wraps every 71.6 minutes, so the Application compares two instants by the sign of their difference. The App_Prover gives the cast of a difference to a signed number no meaning from 2^31 up. Only one direction could be proven ("if a is before b as numbers, the code says so"), and with it only the parts of P1 and P2 that are equalities (`proofs.md`).

An instant of 64 bits counts microseconds from the start of the Partition and never wraps (2^64 µs are 584,000 years). `is_before` is `a < b`, with the contract `result == (a < b)` in both directions. A sum saturates at the top of the range, which the contracts name through one logic function (`time::sum`), so no precondition about the size of an instant is needed anywhere.

## The statements proven in the copy

For the App_Prover (`step.rs` in the patch):

- `p1_inv(p, st)`: if a ventricular event has been seen and a ventricular Pace is pending, its instant is at least the last ventricular event's plus `p.url`.
- `p2_rel(p, old, new, t)`: in a Mode that paces the ventricle, if a ventricular Pace was pending in `old`, one is pending in `new`, at an instant not after `old`'s or not after `t` plus `p.lrl`.

On `step`, for a Paced event, a Sense and a tick, when no change of State is waiting:

| Property | Proven | Supposing |
|---|---|---|
| P1 | `p1_inv` before the event implies `p1_inv` after it | `p1_params`: the URL interval is not longer than the LRL interval; the Mode paces the ventricle and its Paces are not triggered; a Mode with no response senses nothing. For a tick also: its instant is not before the last ventricular event |
| P2, "no later" | `p2_rel` between the state before and after, with t the event's instant | The Mode's Paces are not triggered |

Each helper carries both (`ventricular_event`, `atrial_event`, `apply_pace`, `account`, `accounted`, `tick`, `sense`); the accounting also carries "the last ventricular event is not after the instant accounted to", which the tick's case of P1 needs.

Together with what is proven already (a Pace stays pending; after a Chamber's own event it is pending at exactly the event plus the escape interval; a refractory Sense changes nothing: P3), this is P2 for the ventricle as the design states it.

The mutants, each refused: the URL floor dropped on the atrial path (P1, in `atrial_event`); a tick that moves a pending ventricular Pace to one escape interval from now (P2, in `tick`); a ventricular event that arms the next Pace a microsecond late (in `ventricular_event`).

## What it does not cover

- **A change of State.** The statements are for events with no change waiting, as the present contracts of P2 are. At a change the parameters change, and the invariant has to be stated for the set that comes into force. `switched` starts the new Mode's cycle from the event, so it should carry both; not tried.
- **Commands.** Pace-Now withdraws what is pending and the next tick arms one Pace-Now interval ahead: the bound then runs from that tick. It fits `p2_rel` (t is the tick's instant); not stated.
- **The atrium.** Only the ventricular Pace is stated. A Mode of the atrium alone is the same statement on the other Chamber. The atrial Pace of a dual-chamber Mode hangs on the ventricular one by the AV delay and needs its own statement.
- **Triggered Modes** are P1's own exception and are left out of both.
- **The suppositions about the parameters.** "URL interval not longer than the LRL interval" is what `validate` checks for VDD and DDD (Table 19-5), and its proven specification (P5) could discharge it; in the other Modes the URL limits nothing and P1 says nothing. "A tick is not before the last ventricular event" is a fact about time that no contract of the hardware layer gives the Partition; it would stay a supposition, stated.
- **`rearmed`.** When the hardware layer moves an arming (by at most the lead, 500 µs) the Partition tells the state through `rearmed`, outside the step. P1 is unaffected (the Pace is later); P2's bound is looser by the lead there.

## What the change would cost in the Application

Not done; this is the estimate from the copy.

| Where | What | Size |
|---|---|---|
| `pm-core`, `time.rs` | As in the patch | Done in the copy |
| `pm-core`, layout | An instant of 64 bits has an alignment of 8, and a Partition static may have no padding byte (rsk reads every byte of its initial value). `Pending` grows from 8 to 16 bytes with 7 bytes of padding, `ChamberState` from 32 to 56, `State` from 452 to 512: each needs its spare bytes named, as the reserved fields are today | Four structs; the zero-state tests |
| `pm-core`, tests | 70 compile errors in the copy, all of the kind "a 32-bit number where an instant's is wanted", in six test files; `tests/time.rs` tests the wrap and is to be written again | Mechanical; an hour or two. **Not done: the copy's tests were not run with 64-bit instants** |
| `pm-hw`, the boundary | The timer stays 32 bits. The driver keeps the 64-bit present (a 64-bit count and the 32-bit reading it belongs to), extends every reading and every captured sense instant to the nearest 64-bit instant, and truncates an instant when it writes a compare. It has to read the timer at least once in 35 minutes, which `control` does every 20 ms. Its own comparisons (is the start still ahead by the guard) stay signed 32-bit differences: they are about the counter | A new function and its model tests, with a run across the counter's wrap; the driver's state two words larger |
| `therapy`, `pm-judge`, the rig | The event ring keeps 32-bit instants (truncated); nothing else sees one | Small |
| Proofs | The five members again; the mutants of the step; `proofs.md` | A prover run each |
| Execution time | 64-bit additions and comparisons on a 32-bit core, and a state 60 bytes larger, in Jobs that have 450 µs | To be measured (five runs of 300 s) |

A second way to hold the instant, two 32-bit words, would keep the alignment at 4 and avoid the padding work, at the price of proving the carry arithmetic. The copy did not try it; with the padding named the first way is the plainer one.

## Recommendation

Make the change. The proof is no longer the hard part: it went through in the copy within one session and the statements have teeth. The work is the boundary in `pm-hw` and the layout, both ordinary, and one more measurement series. It would let tasks 12.1 and 12.2 be checked for the ventricle, with the statement of what is left (changes of State, the atrium of the dual-chamber Modes) as its own task.
