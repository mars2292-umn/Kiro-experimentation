# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:a50935325550697291422fd1648e40d9cd6b481e0a67f867ee8ac112e1093d37

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3685 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9659 cycles (150.9 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 0.658 ms (42128 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 121999 Jobs, response at most 0.360 ms (23052 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 182999 Jobs, response at most 5.907 ms (378032 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242940 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2318 cycles (36.2 µs), mean 2092 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2170 % of the run |
| Kernel events | pass | 137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 1 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54901 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 24286 cycles (379.5 µs) over 114291600 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 242939 response edges inside the run (target counted 242939) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.7 % of the run, the Kernel 34.6 % over 19571389 entries; runs per Task [366763, 122042, 427010, 122025, 244664, 150622, 38151] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23167 cycles (0.362 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 366763 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 427 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 259 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9659 / 150.9 | 3704 | 2^12:334991 2^13:30572 2^14:436 |
| 2 | 182999 | 9047 / 141.4 | 3393 | 2^12:173449 2^13:9449 2^14:101 |
| 3 | 91499 | 9547 / 149.2 | 3655 | 2^12:89889 2^13:1430 2^14:180 |
| 5 | 146399 | 9547 / 149.2 | 3736 | 2^12:117564 2^13:28665 2^14:170 |
| 6 | 36599 | 9547 / 149.2 | 3948 | 2^12:36073 2^13:441 2^14:85 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 0.658 | 0 | 0 | 0 |
| 1 | 121999 | 0.360 | 0 | 0 | 0 |
| 2 | 182999 | 5.907 | 0 | 0 | 0 |
| 3 | 68625 | 4.426 | 0 | 0 | 0 |
| 4 | 242939 | 5.967 | 242940 | 2318 | 2092 |
| 5 | 128100 | 6.554 | 0 | 0 | 0 |
| 6 | 18300 | 11.878 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 23167 / 0.362 (min 10093, mean 11734) | 0 above the Budget; 2^14:361503 2^15:4497 |
| Task 1 entry | 121999 | 13855 / 0.216 (min 3541, mean 3678) | 0 above the Budget; 2^12:121708 2^13:42 2^14:249 |
| Task 2 entry | 182999 | 348899 / 5.452 (min 336262, mean 338227) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 209827 / 3.279 (min 187783, mean 196619) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242939 | 14404 / 0.225 (min 3605, mean 4019) | 0 above the Budget; 2^12:220297 2^13:21017 2^14:1625 |
| Task 5 entry | 128100 | 46912 / 0.733 (min 20952, mean 27778) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 42684 / 0.667 (min 20423, mean 29771) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 11102 / 0.173 | |
| Task 0 on Resource 1 | 366000 | 10923 / 0.171 | |
| Task 2 on Resource 0 | 182999 | 328733 / 5.136 | |
| Task 2 on Resource 1 | 365998 | 7228 / 0.113 | |
| Task 5 on Resource 2 | 256200 | 29628 / 0.463 | |
| Task 6 on Resource 2 | 18300 | 9096 / 0.142 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188439 | 3302 / 51.59 | 3062 |
| timer | 1160256 | 5041 / 78.77 | 3035 |
| intake | 242940 | 2447 / 38.23 | 2206 |
| lock | 1573797 | 456 / 7.12 | 455 |
| unlock | 1555497 | 1881 / 29.39 | 1686 |
| job_end | 1128962 | 2100 / 32.81 | 1814 |
| now | 109125415 | 227 / 3.55 | 225 |
| raise | 122000 | 2152 / 33.62 | 2065 |
| dma | 182999 | 726 / 11.34 | 553 |
| epilogue | 1188437 | 1253 / 19.58 | 620 |
| restart | 18300 | 5041 / 78.77 | 4860 |
| enforce | 59475 | 5041 / 78.77 | 3745 |
| view | 1470360 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 55359, Task 1: 541, Task 2: 606980, Task 3: 131460, Task 4: 24691, Task 5: 74050, Task 6: 55016

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8278 cycles; Task 1: 3 (timer, intake), 8060 cycles; Task 2: 6 (dispatch, timer, intake), 17691 cycles; Task 3: 4 (dispatch, timer, intake), 11195 cycles; Task 4: 3 (dispatch, timer), 9707 cycles; Task 5: 6 (dispatch, timer, intake), 17041 cycles; Task 6: 5 (dispatch, timer, intake), 14106 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 4031 cycles
- Task 0 at 3659940.000 ms, 3231 cycles
- Task 2 at 3659945.000 ms, 3231 cycles
- Task 0 at 3659950.000 ms, 4251 cycles
- Task 5 at 3659950.000 ms, 4251 cycles
- Task 0 at 3659960.000 ms, 3467 cycles
- Task 3 at 3659960.000 ms, 3467 cycles
- Task 2 at 3659965.000 ms, 5187 cycles
- Task 0 at 3659970.000 ms, 3703 cycles
- Task 5 at 3659975.000 ms, 3231 cycles
- Task 0 at 3659980.000 ms, 3231 cycles
- Task 0 at 3659990.000 ms, 3111 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 114291600 of them, the longest 24286 cycles (379.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 108323236 |
| 512 to 1023 (8.0 to 16.0 µs) | 2212906 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1133554 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1026538 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1272916 |
| 8192 to 16383 (128.0 to 256.0 µs) | 319536 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2914 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 1150181 | 24286 / 379.5 |
| timer | 494649 | 20665 / 322.9 |
| job_end | 111974716 | 5675 / 88.7 |
| epilogue | 672054 | 14515 / 226.8 |

## The Kernel's events

137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6110251 | 366763 | 0.350 |
| 1 | 506649 | 122042 | 0.200 |
| 2 | 97784987 | 427010 | 5.360 |
| 3 | 31481260 | 122025 | 4.660 |
| 4 | 1115973 | 244664 | 0.160 |
| 5 | 6908978 | 150622 | 0.890 |
| 6 | 2513130 | 38151 | 1.000 |
| none | 222055956 | 906515 | 16569.140 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182999
log=68625
pulse=242939
echo=121999
blocked_releases=182999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=427
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=259
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=0
dma_after_stop=0
spim0_enable=7
spim0_shorts=0
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=3659301
  detail=58681
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659324
  detail=288976
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659375
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659484
  detail=289044
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659501
  detail=58617
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659575
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659644
  detail=288761
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659701
  detail=58681
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659775
  detail=52225
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659811
  detail=289202
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=58681
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659964
  detail=289044
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659975
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659975
  detail=1
  prs_lo=128
  prs_hi=0
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
expected_sensor=183000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 8.09s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json, docs/hil/kernel_timing_nrf52840_response.json): exit 2. 

Not judged: the Analyzer rejected the inputs: [R28.4] the WCET of Task `part2.filter` (58640 cycles) exceeds its Budget (51200 cycles)
