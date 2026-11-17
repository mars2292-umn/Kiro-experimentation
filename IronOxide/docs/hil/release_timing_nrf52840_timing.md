# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3671 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8377 cycles (130.9 µs) against J_rel 12500 cycles |
| Kernel events | pass | 356 events mirrored (356 held, 0 of them the post-detach reboot's): overrun of Task 2: 54, deadline_miss of Task 0: 1, deadline_miss of Task 2: 123, release_overlap of Task 2: 123, mit_violation of Task 2: 54, debugger of Task none: 1; the trace froze at event 355 (deadline_miss of Task 0 at 1107040.012 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 4 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 355 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 18596 cycles (290.6 µs) over 213473325 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 203959 rises and 203334 response edges in the capture); latency 100 to 12650 µs, median 2470 µs, 0 later than 15 ms; Δ_stamp observed 6788 cycles (106.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.6 % of the run, the Kernel 58.4 % over 37328163 entries; runs per Task [368813, 122037, 648024, 107251, 203981, 147438, 36921] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23167 cycles (0.362 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 368813 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 1021 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 2803 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 256 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:506 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 8377 / 130.9 | 3741 | 2^12:334436 2^13:31562 2^14:1 |
| 2 | 182821 | 8377 / 130.9 | 3730 | 2^12:162368 2^13:20452 2^14:1 |
| 3 | 91499 | 8377 / 130.9 | 3870 | 2^12:72286 2^13:19212 2^14:1 |
| 5 | 146399 | 8158 / 127.5 | 3820 | 2^12:117115 2^13:29284 |
| 6 | 36599 | 8158 / 127.5 | 4182 | 2^12:17633 2^13:18966 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 10.143 | 0 | 0 | 0 |
| 1 | 121999 | 0.588 | 0 | 0 | 0 |
| 2 | 182822 | 54.594 | 0 | 0 | 0 |
| 3 | 91500 | 7.083 | 0 | 0 | 0 |
| 4 | 203333 | 12.665 | 203334 | 2327 | 2042 |
| 5 | 146400 | 11.479 | 0 | 0 | 0 |
| 6 | 36600 | 57.105 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 23167 / 0.362 (min 9927, mean 12873) | 0 above the Budget; 2^14:328548 2^15:37452 |
| Task 1 entry | 121999 | 15523 / 0.243 (min 3530, mean 3671) | 0 above the Budget; 2^12:121097 2^13:901 2^14:1 |
| Task 2 entry | 182822 | 2947501 / 46.055 (min 783869, mean 789619) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 205193 / 3.206 (min 175505, mean 195664) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 18485 / 0.289 (min 3541, mean 3996) | 0 above the Budget; 2^12:189006 2^13:13798 2^14:523 2^15:6 |
| Task 5 entry | 146400 | 15760 / 0.246 (min 6766, mean 7141) | 0 above the Budget; 2^13:139654 2^14:6746 |
| Task 6 entry | 36600 | 23008 / 0.359 (min 7786, mean 8031) | 0 above the Budget; 2^13:35105 2^14:1481 2^15:14 |
| Task 0 on Resource 0 | 366000 | 8037 / 0.126 | |
| Task 0 on Resource 1 | 366000 | 8150 / 0.127 | |
| Task 2 on Resource 0 | 182822 | 774612 / 12.103 | |
| Task 2 on Resource 1 | 365644 | 6457 / 0.101 | |
| Task 5 on Resource 2 | 146400 | 9230 / 0.144 | |
| Task 6 on Resource 2 | 36600 | 7958 / 0.124 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148655 | 3322 / 51.91 | 3075 |
| timer | 1039258 | 4504 / 70.38 | 3063 |
| intake | 203334 | 2461 / 38.45 | 2173 |
| lock | 1463466 | 459 / 7.17 | 443 |
| unlock | 1463466 | 1908 / 29.81 | 1624 |
| job_end | 1148654 | 2191 / 34.23 | 1819 |
| now | 208729193 | 226 / 3.53 | 225 |
| raise | 122000 | 2158 / 33.72 | 2003 |
| dma | 36608 | 720 / 11.25 | 495 |
| epilogue | 1148654 | 1243 / 19.42 | 738 |
| enforce | 178 | 3040 / 47.50 | 2965 |
| view | 1631530 | 320 / 5.00 | 282 |

Entries that interrupted an executing Job: Task 0: 180415, Task 1: 904, Task 2: 1090067, Task 3: 49460, Task 4: 15029, Task 5: 7902, Task 6: 1822

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8198 cycles; Task 1: 3 (dispatch, timer), 10395 cycles; Task 2: 31 (dispatch, timer, intake), 93499 cycles; Task 3: 3 (dispatch, timer, intake), 8396 cycles; Task 4: 4 (dispatch, timer), 12834 cycles; Task 5: 3 (dispatch, timer, intake), 8440 cycles; Task 6: 5 (dispatch, timer), 16397 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3453 cycles
- Task 2 at 3659940.000 ms, 3453 cycles
- Task 0 at 3659950.000 ms, 3618 cycles
- Task 5 at 3659950.000 ms, 3618 cycles
- Task 0 at 3659960.000 ms, 3689 cycles
- Task 2 at 3659960.000 ms, 3689 cycles
- Task 3 at 3659960.000 ms, 3689 cycles
- Task 0 at 3659970.000 ms, 3994 cycles
- Task 5 at 3659975.000 ms, 3682 cycles
- Task 0 at 3659980.000 ms, 3453 cycles
- Task 2 at 3659980.000 ms, 3453 cycles
- Task 0 at 3659990.000 ms, 4574 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 213473325 of them, the longest 18596 cycles (290.6 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 208253943 |
| 512 to 1023 (8.0 to 16.0 µs) | 1495842 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1226953 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1045285 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1139170 |
| 8192 to 16383 (128.0 to 256.0 µs) | 311919 |
| 16384 to 32767 (256.0 to 512.0 µs) | 213 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 984738 | 18596 / 290.6 |
| timer | 924218 | 16460 / 257.2 |
| job_end | 211114519 | 5451 / 85.2 |
| epilogue | 449850 | 6359 / 99.4 |

## The Kernel's events

356 events mirrored (356 held, 0 of them the post-detach reboot's): overrun of Task 2: 54, deadline_miss of Task 0: 1, deadline_miss of Task 2: 123, release_overlap of Task 2: 123, mit_violation of Task 2: 54, debugger of Task none: 1; the trace froze at event 355 (deadline_miss of Task 0 at 1107040.012 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 355 | 1107040.012 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6796742 | 368813 | 0.350 |
| 1 | 518325 | 122037 | 0.160 |
| 2 | 227836480 | 648024 | 12.290 |
| 3 | 27901509 | 107251 | 3.160 |
| 4 | 937072 | 203981 | 0.180 |
| 5 | 1381323 | 147438 | 0.220 |
| 6 | 393006 | 36921 | 0.300 |
| none | 100231154 | 665772 | 7.270 |

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
blocked_releases=182858
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1021
loop:part0:sensor:late=2803
loop:part1:spin_ms:spin=0
loop:part1:log:busy=256
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
     Finished in 7.65s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (19404 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
