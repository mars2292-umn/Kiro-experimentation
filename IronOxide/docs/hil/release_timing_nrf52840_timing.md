# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:1aa651dacf1cb7cf35646be32f8f4b7b9e5e0b9c03e7deac93a2b2e006b6a18a

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3672 s (operating duration 3660 s) |
| compare arms (HW-12, ASM-19) | pass | 4922238 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (89 below 4 ticks, 283 below 8, 731 below 16); 204092 pended by software, the instant reached |
| release timing (R9.10) | pass | 823330 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8397 cycles (131.2 µs) against J_rel 12500 cycles |
| Kernel events | pass | 8 events mirrored (8 held, 0 of them the post-detach reboot's): overrun of Task 2: 1, deadline_miss of Task 0: 1, deadline_miss of Task 2: 2, release_overlap of Task 2: 2, mit_violation of Task 2: 1, debugger of Task none: 1; the trace froze at event 7 (deadline_miss of Task 0 at 286780.012 ms); the deliberate overload case: 1 deadline miss(es) of Task 0 (control) after the injected late Job of Task 2 (sensor), whose 12 ms Critical_Section then blocks control for nearly its 10 ms deadline; the trace and the mirror stop at the first, so the events after it are not judged by this step |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 1 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch; the ring is frozen at event 7 and holds the entries leading to it |
| dispatch deferrals (5.8a) | pass | 1 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 1 actions recorded (1 held, 1 of them deferrals: vectors 0x28) |
| masked span (entries back to back) | pass | the longest run of Kernel entries with no return to a Job between them: 21823 cycles (341.0 µs) over 172464936 spans, closed by a dispatch entry (not L_kernel since 2026-11-21: each entry is taken at an exception return) |
| hold (L_kernel) | pass | the longest stretch in which a pending Kernel-level interrupt could not be taken, one handler and its unwind: 6562 cycles (102.5 µs) over 174292004 holds, closed by a timer entry; against the record's L_kernel 11000 cycles |
| loopback over the run | pass | 203333 stimulus pulses answered (target counted 203333; 204008 rises and 203334 response edges in the capture); latency 100 to 12700 µs, median 2480 µs, 0 later than 15 ms; Δ_stamp observed 6676 cycles (104.3 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.9 % of the run, the Kernel 61.2 % over 50393421 entries; runs per Task [369263, 122049, 648091, 106980, 204105, 147452, 36949] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24240 cycles (0.379 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 369263 runs for 366000 Jobs) |

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
| 0 | 365999 | 8397 / 131.2 | 3568 | 2^12:354801 2^13:11196 2^14:2 |
| 2 | 182834 | 8397 / 131.2 | 3499 | 2^12:178845 2^13:3987 2^14:2 |
| 3 | 91499 | 8397 / 131.2 | 3636 | 2^12:88396 2^13:3101 2^14:2 |
| 5 | 146399 | 7840 / 122.5 | 3654 | 2^12:137219 2^13:9180 |
| 6 | 36599 | 7840 / 122.5 | 3930 | 2^12:34269 2^13:2330 |

Releases recorded: 823330; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 10.188 | 0 | 0 | 0 |
| 1 | 121999 | 0.636 | 0 | 0 | 0 |
| 2 | 182835 | 54.782 | 0 | 0 | 0 |
| 3 | 91500 | 7.419 | 0 | 0 | 0 |
| 4 | 203333 | 12.719 | 203334 | 2230 | 1984 |
| 5 | 146400 | 11.564 | 0 | 0 | 0 |
| 6 | 36600 | 97.078 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 24240 / 0.379 (min 10516, mean 13361) | 0 above the Budget; 2^14:313116 2^15:52884 |
| Task 1 entry | 121999 | 17786 / 0.278 (min 3544, mean 3661) | 0 above the Budget; 2^12:121157 2^13:838 2^14:3 2^15:1 |
| Task 2 entry | 182835 | 2934922 / 45.858 (min 785147, mean 791025) | 165 above the Budget; 2^15:182835 |
| Task 3 entry | 91500 | 204513 / 3.196 (min 186913, mean 196440) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203333 | 19168 / 0.299 (min 3562, mean 3992) | 0 above the Budget; 2^12:188452 2^13:14230 2^14:641 2^15:10 |
| Task 5 entry | 146400 | 20499 / 0.320 (min 7109, mean 7439) | 0 above the Budget; 2^13:139755 2^14:6638 2^15:7 |
| Task 6 entry | 36600 | 27051 / 0.423 (min 8306, mean 8530) | 0 above the Budget; 2^14:36565 2^15:35 |
| Task 0 on Resource 0 | 366000 | 7999 / 0.125 | |
| Task 0 on Resource 1 | 366000 | 7804 / 0.122 | |
| Task 2 on Resource 0 | 182835 | 776956 / 12.140 | |
| Task 2 on Resource 1 | 365670 | 5763 / 0.090 | |
| Task 5 on Resource 2 | 146400 | 6099 / 0.095 | |
| Task 6 on Resource 2 | 36600 | 10232 / 0.160 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148670 | 3098 / 48.41 | 2854 |
| timer | 1040156 | 4014 / 62.72 | 2806 |
| intake | 203334 | 2374 / 37.09 | 2131 |
| lock | 1463505 | 458 / 7.16 | 432 |
| unlock | 1463505 | 1792 / 28.00 | 1534 |
| job_end | 1148667 | 2047 / 31.98 | 1730 |
| now | 167665560 | 228 / 3.56 | 227 |
| raise | 122000 | 1999 / 31.23 | 1881 |
| dma | 36608 | 725 / 11.33 | 480 |
| epilogue | 1148667 | 1261 / 19.70 | 745 |
| enforce | 165 | 2870 / 44.84 | 2807 |
| view | 1631514 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 181589, Task 1: 850, Task 2: 1090313, Task 3: 49244, Task 4: 15728, Task 5: 7853, Task 6: 1958

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer), 8014 cycles; Task 1: 4 (dispatch, timer, intake), 11479 cycles; Task 2: 31 (dispatch, timer, intake), 87054 cycles; Task 3: 2 (timer), 5324 cycles; Task 4: 4 (dispatch, timer), 12391 cycles; Task 5: 4 (dispatch, timer, intake), 11958 cycles; Task 6: 6 (dispatch, timer), 17509 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3223 cycles
- Task 2 at 3659940.000 ms, 3223 cycles
- Task 0 at 3659950.000 ms, 3684 cycles
- Task 5 at 3659950.000 ms, 3684 cycles
- Task 0 at 3659960.000 ms, 3467 cycles
- Task 2 at 3659960.000 ms, 3467 cycles
- Task 3 at 3659960.000 ms, 3467 cycles
- Task 0 at 3659970.000 ms, 3556 cycles
- Task 5 at 3659975.000 ms, 3536 cycles
- Task 0 at 3659980.000 ms, 3223 cycles
- Task 2 at 3659980.000 ms, 3223 cycles
- Task 0 at 3659990.000 ms, 3716 cycles

## The Kernel's masked spans (11.13)

Every run of Kernel entries with no return to a Job between them (a pending Kernel-level interrupt is taken at each entry's return), 172464936 of them, the longest 21823 cycles (341.0 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 168731567 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1244066 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1088010 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1079953 |
| 8192 to 16383 (128.0 to 256.0 µs) | 321077 |
| 16384 to 32767 (256.0 to 512.0 µs) | 263 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 984848 | 21823 / 341.0 |
| timer | 923271 | 16611 / 259.5 |
| job_end | 170107029 | 5686 / 88.8 |
| epilogue | 449788 | 6518 / 101.8 |

The longest one's 6 entries, in order (cycles from its start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | job_end, an unwind follows, exception 41 pending | 0 | 2642 | 2642 / 41.3 | 0 / 0.0 |
| 2 | epilogue, exception 41 pending | 2832 | 4333 | 1501 / 23.5 | 190 / 3.0 |
| 3 | dispatch, started a Job, exception 24 pending | 4620 | 8158 | 3538 / 55.3 | 287 / 4.5 |
| 4 | timer, exception 24 pending | 8443 | 11799 | 3356 / 52.4 | 285 / 4.5 |
| 5 | timer, exception 36 pending | 12101 | 17959 | 5858 / 91.5 | 302 / 4.7 |
| 6 | dispatch, started a Job | 18261 | 21823 | 3562 / 55.7 | 302 / 4.7 |

Rust halves 20457 cycles (319.6 µs), between them 1366 cycles (21.3 µs).

## The Kernel's holds (L_kernel)

Every stretch in which a pending Kernel-level interrupt could not be taken (one handler from its Rust half's beginning to its end, with the unwind it runs before it returns), 174292004 of them, the longest 6562 cycles (102.5 µs), closed by a timer entry:

The longest was one entry: timer, exception 36 pending.

| Closing entry | Longest (cycles / µs) |
|---|---|
| dispatch | 3702 / 57.8 |
| timer | 6562 / 102.5 |
| job_end | 2315 / 36.2 |
| epilogue | 4410 / 68.9 |

## The Kernel's events

8 events mirrored (8 held, 0 of them the post-detach reboot's): overrun of Task 2: 1, deadline_miss of Task 0: 1, deadline_miss of Task 2: 2, release_overlap of Task 2: 2, mit_violation of Task 2: 1, debugger of Task none: 1; the trace froze at event 7 (deadline_miss of Task 0 at 286780.012 ms)

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 7 | 286780.012 | deadline_miss | 0 | 0 | 0x0 |

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 7165136 | 369263 | 0.370 |
| 1 | 538429 | 122049 | 0.190 |
| 2 | 228254344 | 648091 | 12.310 |
| 3 | 28027416 | 106980 | 3.180 |
| 4 | 966829 | 204105 | 0.220 |
| 5 | 1470028 | 147452 | 0.230 |
| 6 | 426724 | 36949 | 0.340 |
| none | 99146447 | 665814 | 7.220 |

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
blocked_releases=182954
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
     Finished in 8.31s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.echo` (22233 cycles) exceeds its Budget (19200 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
