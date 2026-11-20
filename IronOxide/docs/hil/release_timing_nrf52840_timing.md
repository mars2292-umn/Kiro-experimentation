# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:fc48480c2207e3330f9096198fa317caa48ebbcb0629dd9efa240e730520b2bd

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3671 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8460 cycles (132.2 µs) against J_rel 12500 cycles |
| Kernel events | pass | 238 events mirrored (238 held, 0 of them the post-detach reboot's): overrun of Task 2: 36, deadline_miss of Task 0: 1, deadline_miss of Task 2: 82, release_overlap of Task 2: 82, mit_violation of Task 2: 36, debugger of Task none: 1; the trace froze at event 237 (deadline_miss of Task 0 at 738040.011 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 2 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 237 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 2 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2 actions recorded (2 held, 2 of them deferrals: vectors 0x24, 0x28) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 21110 cycles (329.8 µs) over 215735914 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203959 rises and 203334 response edges in the capture); latency 100 to 12650 µs, median 2470 µs, 0 later than 15 ms; Δ_stamp observed 6876 cycles (107.4 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.7 % of the run, the Kernel 58.3 % over 35079175 entries; runs per Task [368873, 122037, 648090, 107236, 204207, 147428, 36924] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24363 cycles (0.381 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 368873 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 1032 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 2816 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 259 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 8460 / 132.2 | 3734 | 2^12:335197 2^13:30796 2^14:6 |
| 2 | 182821 | 8460 / 132.2 | 3742 | 2^12:162265 2^13:20551 2^14:5 |
| 3 | 91499 | 8460 / 132.2 | 3886 | 2^12:72206 2^13:19287 2^14:6 |
| 5 | 146399 | 8332 / 130.2 | 3814 | 2^12:117873 2^13:28525 2^14:1 |
| 6 | 36599 | 8332 / 130.2 | 4189 | 2^12:17650 2^13:18948 2^14:1 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 10.178 | 0 | 0 | 0 |
| 1 | 121999 | 0.639 | 0 | 0 | 0 |
| 2 | 182822 | 54.609 | 0 | 0 | 0 |
| 3 | 91500 | 7.105 | 0 | 0 | 0 |
| 4 | 203333 | 12.665 | 203334 | 2352 | 2121 |
| 5 | 146400 | 11.523 | 0 | 0 | 0 |
| 6 | 36600 | 57.129 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 24363 / 0.381 (min 10075, mean 13050) | 0 above the Budget; 2^14:315626 2^15:50374 |
| Task 1 entry | 121999 | 15468 / 0.242 (min 3549, mean 3684) | 0 above the Budget; 2^12:121201 2^13:795 2^14:3 |
| Task 2 entry | 182822 | 2946916 / 46.046 (min 783950, mean 789793) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 205395 / 3.209 (min 175087, mean 195816) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 18374 / 0.287 (min 3561, mean 4016) | 0 above the Budget; 2^12:188519 2^13:14187 2^14:623 2^15:4 |
| Task 5 entry | 146400 | 15818 / 0.247 (min 6855, mean 7218) | 0 above the Budget; 2^13:139738 2^14:6662 |
| Task 6 entry | 36600 | 23153 / 0.362 (min 7859, mean 8093) | 0 above the Budget; 2^13:35154 2^14:1433 2^15:13 |
| Task 0 on Resource 0 | 366000 | 8000 / 0.125 | |
| Task 0 on Resource 1 | 366000 | 8233 / 0.129 | |
| Task 2 on Resource 0 | 182822 | 776165 / 12.128 | |
| Task 2 on Resource 1 | 365644 | 7025 / 0.110 | |
| Task 5 on Resource 2 | 146400 | 5658 / 0.088 | |
| Task 6 on Resource 2 | 36600 | 9949 / 0.155 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148659 | 3342 / 52.22 | 3086 |
| timer | 1039398 | 4576 / 71.50 | 3070 |
| intake | 203334 | 2483 / 38.80 | 2255 |
| lock | 1463466 | 456 / 7.12 | 437 |
| unlock | 1463466 | 1947 / 30.42 | 1700 |
| job_end | 1148654 | 2214 / 34.59 | 1879 |
| now | 210993478 | 226 / 3.53 | 224 |
| raise | 122000 | 2239 / 34.98 | 2088 |
| dma | 36608 | 724 / 11.31 | 493 |
| epilogue | 1148654 | 1253 / 19.58 | 742 |
| enforce | 178 | 3040 / 47.50 | 2951 |
| view | 1631803 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 180798, Task 1: 803, Task 2: 1089782, Task 3: 49419, Task 4: 15743, Task 5: 7833, Task 6: 1761

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer), 9005 cycles; Task 1: 3 (dispatch, timer), 10333 cycles; Task 2: 32 (dispatch, timer, intake), 97102 cycles; Task 3: 4 (dispatch, timer, intake), 11383 cycles; Task 4: 4 (dispatch, timer), 12696 cycles; Task 5: 3 (dispatch, timer, intake), 8476 cycles; Task 6: 5 (dispatch, timer), 16335 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3463 cycles
- Task 2 at 3659940.000 ms, 3463 cycles
- Task 0 at 3659950.000 ms, 3899 cycles
- Task 5 at 3659950.000 ms, 3899 cycles
- Task 0 at 3659960.000 ms, 3707 cycles
- Task 2 at 3659960.000 ms, 3707 cycles
- Task 3 at 3659960.000 ms, 3707 cycles
- Task 0 at 3659970.000 ms, 3447 cycles
- Task 5 at 3659975.000 ms, 3615 cycles
- Task 0 at 3659980.000 ms, 3463 cycles
- Task 2 at 3659980.000 ms, 3463 cycles
- Task 0 at 3659990.000 ms, 3651 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 215735914 of them, the longest 21110 cycles (329.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 210511072 |
| 512 to 1023 (8.0 to 16.0 µs) | 1502317 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1032744 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1236268 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1140921 |
| 8192 to 16383 (128.0 to 256.0 µs) | 312404 |
| 16384 to 32767 (256.0 to 512.0 µs) | 188 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 984597 | 21110 / 329.8 |
| timer | 923684 | 18548 / 289.8 |
| job_end | 213377837 | 5258 / 82.2 |
| epilogue | 449796 | 4127 / 64.5 |

## The Kernel's events

238 events mirrored (238 held, 0 of them the post-detach reboot's): overrun of Task 2: 36, deadline_miss of Task 0: 1, deadline_miss of Task 2: 82, release_overlap of Task 2: 82, mit_violation of Task 2: 36, debugger of Task none: 1; the trace froze at event 237 (deadline_miss of Task 0 at 738040.011 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 237 | 738040.011 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6910950 | 368873 | 0.370 |
| 1 | 524735 | 122037 | 0.160 |
| 2 | 227904060 | 648090 | 12.310 |
| 3 | 27930318 | 107236 | 3.130 |
| 4 | 951991 | 204207 | 0.190 |
| 5 | 1407653 | 147428 | 0.230 |
| 6 | 398259 | 36924 | 0.250 |
| none | 99967860 | 665500 | 7.260 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182822
log=91500
pulse=203333
echo=121999
blocked_releases=182870
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1032
loop:part0:sensor:late=2816
loop:part1:spin_ms:spin=0
loop:part1:log:busy=259
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=36599
dma_after_stop=0
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=10
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=182822
expected_late_jobs=178
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 7.95s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json, docs/hil/kernel_timing_nrf52840_response.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (19335 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
