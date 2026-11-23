# Evidence Document

**PACEMAKER on rsk: an experimental Application**

Written by `chains.py` from the records it names; do not edit by hand (`chains.py --check` fails when this file is not what the records give). It holds the timing chains of stage E1 (task 9.3; PM24.1, PM24.2, PM27.6). The other evidence of the experiment is named per task in `tasks.md` and per criterion in `traceability.md`.

## The Analyzer's bounds

From `rig/task_9_8_analysis/report.json`: result **PASS**, utilization 41 %. A response-time bound R is from the Task's Release to the end of its Job and includes the release lag (J_rel), the blocking, the Kernel's entries and the higher-Priority Tasks. The longest response measured is the Kernel's own record over the two 300 s record runs (`rig/task_9_2_run_vvi_90.md`, `rig/task_9_2_run_aai_90.md`).

| Task | Deadline (ms) | R, the Analyzer's bound (ms) | Within its deadline | Longest response measured (ms) |
|---|---|---|---|---|
| `pace` | 2.000 | 1.690 | yes | 0.300 |
| `sense_v` | 3.000 | 2.757 | yes | 0.305 |
| `sense_a` | 4.000 | 3.891 | yes | 0.306 |
| `control` | 20.000 | 9.564 | yes | 0.767 |

The Analyzer's own statement about its inputs: "PLACEHOLDER TIMING DATA: 7 input value(s) carry the method `placeholder` (no measurement or analysis): delta_release, delta_fp_save, delta_fp_restore, delta_dma_write, delta_stop, delta_send, delta_recv. This report is not timing evidence." Which parameters those are and why is in `design.md`, "What the measurement settled".

Measured Kernel parameters of this Application, the largest seen (`rig/task_9_2_kernel_timing_*.json`): release lag J_rel 85.1 µs; from a sense edge to the Kernel's timestamp of it (Δ_stamp, on the atrial source) 2.4 µs; the longest stretch with interrupts held off (L_kernel) 57.3 µs.

## The chains of stage E1

In every chain the edges themselves are hardware: a Pace starts and ends at compares of TIMER3 acting on the pin through PPI, and a Sense's instant is a capture of the same timer (design, "Hardware makes every edge"). A Job only has to finish *before* the edge it arms. So each chain has two figures: the **deadline the software has to meet** (a sum of response-time bounds, which must stay under the time available), and the **error of the interval** once it is met (the timer's 1 µs tick, twice, and under 1 µs for the rounding of a rate to microseconds).

| Requirement | Chain | Bound of the chain | Must stay under | Held | Error of the interval | Tolerance | Measured on the rig |
|---|---|---|---|---|---|---|---|
| Pace to the next Pace of the Chamber: the LRL interval (PM9.1, PM9.2, PM27.6) | start of the pulse → its end releases `pace` → the Job arms the next start compare | the pulse (1.9 ms at most) + R of `pace` (1.690 ms) + `LEAD` (0.5 ms) = 4.090 ms | the shortest escape interval, 342.857 ms (175 ppm) | held | 2 µs + rounding | PM-PAR-01, ±8 ms | 1746 intervals in 37 runs, the farthest 9 µs from its escape interval (`rig/task_9_4_e1_runs.json`) |
| Sense to the next Pace: inhibition and the interval restarted (PM5.5, PM9.3) | sense edge captured → its count releases the sense Task → the Job moves the start compare to the capture plus the escape interval | R of the sense Task (2.757 ms ventricular, 3.891 ms atrial) + `LEAD` = 4.391 ms | the shortest escape interval, 342.857 ms | held | 2 µs + rounding | PM-PAR-01, ±8 ms | the same record: the intervals judged include those that start at a Sense |
| The inhibition window: a Sense this close before a pending Pace need not cancel it (PM5.5, PD-04) | sense edge → the sense Job withdraws the pending start before it fires | R of the sense Task: 2.757 ms ventricular, 3.891 ms atrial | PM-PAR-15, 4 ms | held | — | — | a Sense 73 µs before its Pace was the farthest still followed by it (18 such Paces in 24 runs); none at a nominal 142 µs or beyond, up to 5000 µs (`rig/task_9_5_inhibition_window.json`; ventricle only) |
| Refractory periods (PM12.1, PM12.2, PM4.4) | none: the sense Job compares the capture with the instant of the event that started the period (PM27.3) | — | — | — | 2 µs | PM-PAR-01, ±8 ms | 220 Senses judged effective, 47 refractory, 5 within the tolerance of the boundary, no violation (`rig/task_9_4_e1_runs.json`) |
| Pulse width (PM3.3, PM28.2) | none: the end compare is written with the start, in one Critical_Section | — | — | — | 1 µs | PM-PAR-05, 0.2 ms (0.02 ms for the Pace-Now and POR pulses) | 1779 pulses over the twenty widths of Table 19-1, the farthest 1 µs from its width (`rig/task_9_4_e1_runs.json`) |
| The first Pace after a start, and a parameter set taking effect (PM9.1) | `control`'s Job arms the first escape interval | R of `control`, 9.564 ms | its period, 20.000 ms | held | 2 µs + rounding from the instant the Job reads | PM-PAR-01, ±8 ms | every run begins this way; the first interval is not judged (the capture starts after it) |

All 37 runs of `rig/task_9_4_e1_runs.json` have the verdict PASS; arming too late, moved or forced in them: 0. Each is 30 s of capture, not the hour of PAR-05 (task 9.4). The distance of an interval from its nominal value is as the analyzer's clock sees the board's: a few parts in a million of the interval.

**PM27.6.** The interval error of the scheme is not the release lag and the response time of the pacing Task: those bound when the Job must have finished, and the interval is then exact to the timer. Were the Job to make the edge itself, the error would be up to R of `pace`, 1.690 ms, still within PM-PAR-01 (8 ms) but not within PM-PAR-03 (1 ms) or the pulse-width tolerances; that is why it does not. `GUARD` (64 µs) and `LEAD` (500 µs) are the scheme's own constants (design, "Arming and the lead").

## Chains of later stages, on the bounds of stage E1

Not delivered yet; the figures are stage E1's bounds put into the later chains, to be restated when the stages are built (tasks 13.1, 20.1).

| Requirement | Chain | Bound with E1's figures | Must stay under | Held | Stage |
|---|---|---|---|---|---|
| AV delay after an atrial Pace (PM11) | pulse → `pace` arms the ventricular start | 1.9 + 1.690 + 0.5 = 4.090 ms | the smallest AV delay, 30 ms | held | E2 |
| AV delay after an atrial Sense (PM11, PM5.6) | `sense_a` arms the ventricular start | 3.891 + 0.5 = 4.391 ms | 30 ms | held | E2 |
| Triggered Pace (PM5.4) | Sense → sense Job → Pace at the present plus `LEAD` | 3.891 + 0.5 = 4.391 ms | PM-PAR-01, 8 ms | held | E2 |
| Pace-Now, link loss, magnet, battery (PM6.6, PM20.2, PM8) | frame or pin → `link` → `control` | one `control` period (20.000 ms; R 9.564 ms) and what `comms` adds | as the design's "Timing Design" states | not yet stated | E3 |
