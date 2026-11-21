# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:e2c05626b4cc8411ecba4164194538c5d7419e91a465659a4714ec5917c18f51

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3672 s (operating duration 3660 s) |
| compare arms (HW-12, ASM-19) | pass | 4922283 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (158 below 4 ticks, 313 below 8, 798 below 16); 203934 pended by software, the instant reached |
| release timing (R9.10) | pass | 823330 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8573 cycles (134.0 µs) against J_rel 12500 cycles |
| Kernel events | pass | 100 events mirrored (100 held, 0 of them the post-detach reboot's): overrun of Task 2: 13, deadline_miss of Task 0: 1, deadline_miss of Task 2: 36, release_overlap of Task 2: 36, mit_violation of Task 2: 13, debugger of Task none: 1; the trace froze at event 99 (deadline_miss of Task 0 at 532780.027 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 1 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 99 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 1 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 1 actions recorded (1 held, 1 of them deferrals: vectors 0x24) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 21659 cycles (338.4 µs) over 172461244 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6496 cycles (101.5 µs) over 174285649 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 204004 rises and 203334 response edges in the capture); latency 100 to 12700 µs, median 2490 µs, 0 later than 15 ms; Δ_stamp observed 6648 cycles (103.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.9 % of the run, the Kernel 61.2 % over 50392693 entries; runs per Task [369220, 122051, 648174, 107296, 204073, 147486, 36957] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23476 cycles (0.367 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23040 cycles (0.360 ms at 100000 samples/s; 369220 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:405 | 10_000_000 | 819 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:443 | 50_000_000 | 2222 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 206 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 8573 / 134.0 | 3568 | 2^12:354428 2^13:11563 2^14:8 |
| 2 | 182834 | 8573 / 134.0 | 3502 | 2^12:178592 2^13:4234 2^14:8 |
| 3 | 91499 | 8573 / 134.0 | 3639 | 2^12:88140 2^13:3354 2^14:5 |
| 5 | 146399 | 7728 / 120.8 | 3655 | 2^12:136836 2^13:9563 |
| 6 | 36599 | 7728 / 120.8 | 3934 | 2^12:34024 2^13:2575 |

Releases recorded: 823330; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 10.184 | 0 | 0 | 0 |
| 1 | 121999 | 0.663 | 0 | 0 | 0 |
| 2 | 182835 | 54.660 | 0 | 0 | 0 |
| 3 | 91500 | 7.416 | 0 | 0 | 0 |
| 4 | 203333 | 12.713 | 203334 | 2230 | 1986 |
| 5 | 146400 | 11.566 | 0 | 0 | 0 |
| 6 | 36600 | 97.083 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 23476 / 0.367 (min 10516, mean 13381) | 0 above the Budget; 2^14:312950 2^15:53050 |
| Task 1 entry | 121999 | 15814 / 0.247 (min 3544, mean 3665) | 0 above the Budget; 2^12:121183 2^13:805 2^14:11 |
| Task 2 entry | 182835 | 2935166 / 45.862 (min 785147, mean 791039) | 165 above the Budget; 2^15:182835 |
| Task 3 entry | 91500 | 205181 / 3.206 (min 186893, mean 196408) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 19108 / 0.299 (min 3562, mean 3990) | 0 above the Budget; 2^12:188719 2^13:13989 2^14:617 2^15:8 |
| Task 5 entry | 146400 | 20827 / 0.325 (min 7109, mean 7442) | 0 above the Budget; 2^13:139930 2^14:6463 2^15:7 |
| Task 6 entry | 36600 | 27499 / 0.430 (min 8306, mean 8534) | 0 above the Budget; 2^14:36570 2^15:30 |
| Task 0 on Resource 0 | 366000 | 11171 / 0.175 | |
| Task 0 on Resource 1 | 366000 | 7628 / 0.119 | |
| Task 2 on Resource 0 | 182835 | 777388 / 12.147 | |
| Task 2 on Resource 1 | 365670 | 7705 / 0.120 | |
| Task 5 on Resource 2 | 146400 | 8741 / 0.137 | |
| Task 6 on Resource 2 | 36600 | 13912 / 0.217 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148668 | 3110 / 48.59 | 2857 |
| timer | 1040043 | 4014 / 62.72 | 2810 |
| intake | 203334 | 2374 / 37.09 | 2133 |
| lock | 1463505 | 458 / 7.16 | 434 |
| unlock | 1463505 | 1792 / 28.00 | 1536 |
| job_end | 1148667 | 2047 / 31.98 | 1731 |
| now | 167659320 | 228 / 3.56 | 227 |
| raise | 122000 | 1999 / 31.23 | 1881 |
| dma | 36608 | 725 / 11.33 | 482 |
| epilogue | 1148667 | 1261 / 19.70 | 747 |
| enforce | 165 | 2990 / 46.72 | 2811 |
| view | 1631914 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 182427, Task 1: 836, Task 2: 1090218, Task 3: 49432, Task 4: 15436, Task 5: 7702, Task 6: 1916

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 7542 cycles; Task 1: 3 (dispatch, timer), 9878 cycles; Task 2: 31 (dispatch, timer, intake), 87050 cycles; Task 3: 4 (dispatch, timer, intake), 10551 cycles; Task 4: 4 (dispatch, timer), 12391 cycles; Task 5: 4 (dispatch, timer, intake), 12126 cycles; Task 6: 6 (dispatch, timer), 17717 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3231 cycles
- Task 2 at 3659940.000 ms, 3231 cycles
- Task 0 at 3659950.000 ms, 3800 cycles
- Task 5 at 3659950.000 ms, 3800 cycles
- Task 0 at 3659960.000 ms, 3475 cycles
- Task 2 at 3659960.000 ms, 3475 cycles
- Task 3 at 3659960.000 ms, 3475 cycles
- Task 0 at 3659970.000 ms, 3640 cycles
- Task 5 at 3659975.000 ms, 3564 cycles
- Task 0 at 3659980.000 ms, 3223 cycles
- Task 2 at 3659980.000 ms, 3223 cycles
- Task 0 at 3659990.000 ms, 3708 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 172461244 of them, the longest 21659 cycles (338.4 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 168727845 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1241234 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1087709 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1074891 |
| 8192 to 16383 (128.0 to 256.0 µs) | 329306 |
| 16384 to 32767 (256.0 to 512.0 µs) | 259 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 985299 | 21659 / 338.4 |
| timer | 923003 | 17711 / 276.7 |
| job_end | 170103094 | 5810 / 90.8 |
| epilogue | 449848 | 4230 / 66.1 |

The longest one's 6 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 41 pending | 0 | 2642 | 2642 / 41.3 | 0 / 0.0 |
| 2 | epilogue, exception 41 pending | 2832 | 4333 | 1501 / 23.5 | 190 / 3.0 |
| 3 | dispatch, started a Job, exception 24 pending | 4620 | 8158 | 3538 / 55.3 | 287 / 4.5 |
| 4 | timer, exception 24 pending | 8443 | 11799 | 3356 / 52.4 | 285 / 4.5 |
| 5 | timer, exception 36 pending | 12101 | 17795 | 5694 / 89.0 | 302 / 4.7 |
| 6 | dispatch, started a Job | 18097 | 21659 | 3562 / 55.7 | 302 / 4.7 |

Rust halves 20293 cycles (317.1 µs), between them 1366 cycles (21.3 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 174285649 of them, the longest 6496 cycles (101.5 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3726 / 58.2 |
| timer | 6496 / 101.5 |
| job_end | 2315 / 36.2 |
| epilogue | 4425 / 69.1 |

## The Kernel's events

100 events mirrored (100 held, 0 of them the post-detach reboot's): overrun of Task 2: 13, deadline_miss of Task 0: 1, deadline_miss of Task 2: 36, release_overlap of Task 2: 36, mit_violation of Task 2: 13, debugger of Task none: 1; the trace froze at event 99 (deadline_miss of Task 0 at 532780.027 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 99 | 532780.027 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 7176629 | 369220 | 0.360 |
| 1 | 538489 | 122051 | 0.160 |
| 2 | 228260792 | 648174 | 12.320 |
| 3 | 28024629 | 107296 | 3.170 |
| 4 | 966174 | 204073 | 0.210 |
| 5 | 1470698 | 147486 | 0.230 |
| 6 | 426776 | 36957 | 0.280 |
| none | 99132201 | 665415 | 7.220 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182835
log=91500
pulse=203333
echo=121999
blocked_releases=182955
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=819
loop:part0:sensor:late=2222
loop:part1:spin_ms:spin=0
loop:part1:log:busy=206
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
expected_sensor=182835
expected_late_jobs=165
late_events_seen=30
overload_misses=0
RSK-OK
Firmware exited successfully
     Finished in 8.52s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (19768 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
