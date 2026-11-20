# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:2b371e899dfccda96fe50601e7257cd1bdce41467ef36669bae2cf68cd2e053b

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 204 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9275 cycles (144.9 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.632 ms (40472 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.307 ms (19628 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.862 ms (375180 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11948 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2316 cycles (36.2 µs), mean 2092 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2170 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 24286 cycles (379.5 µs) over 5623255 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11947 response edges inside the run (target counted 11947) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 35.3 % of the run, the Kernel 41.9 % over 962360 entries; runs per Task [18045, 6003, 20999, 5995, 12037, 7401, 1878] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23113 cycles (0.361 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 18045 runs for 18000 Jobs) |

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
| 0 | 17999 | 9275 / 144.9 | 3690 | 2^12:16957 2^13:1022 2^14:20 |
| 2 | 8999 | 8783 / 137.2 | 3390 | 2^12:8511 2^13:486 2^14:2 |
| 3 | 4499 | 9191 / 143.6 | 3644 | 2^12:4440 2^13:47 2^14:12 |
| 5 | 7199 | 9191 / 143.6 | 3717 | 2^12:6141 2^13:1056 2^14:2 |
| 6 | 1799 | 9191 / 143.6 | 3942 | 2^12:1775 2^13:22 2^14:2 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.632 | 0 | 0 | 0 |
| 1 | 5999 | 0.307 | 0 | 0 | 0 |
| 2 | 8999 | 5.862 | 0 | 0 | 0 |
| 3 | 3375 | 4.426 | 0 | 0 | 0 |
| 4 | 11947 | 5.941 | 11948 | 2316 | 2092 |
| 5 | 6300 | 6.505 | 0 | 0 | 0 |
| 6 | 900 | 11.825 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 23113 / 0.361 (min 10093, mean 11717) | 0 above the Budget; 2^14:17780 2^15:220 |
| Task 1 entry | 5999 | 10261 / 0.160 (min 3541, mean 3674) | 0 above the Budget; 2^12:5988 2^13:3 2^14:8 |
| Task 2 entry | 8999 | 347246 / 5.426 (min 336266, mean 338398) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206491 / 3.226 (min 190871, mean 196576) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11947 | 14002 / 0.219 (min 3605, mean 4018) | 0 above the Budget; 2^12:10837 2^13:1030 2^14:80 |
| Task 5 entry | 6300 | 36596 / 0.572 (min 20956, mean 27764) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 39180 / 0.612 (min 20423, mean 29764) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 11102 / 0.173 | |
| Task 0 on Resource 1 | 18000 | 10803 / 0.169 | |
| Task 2 on Resource 0 | 8999 | 328673 / 5.136 | |
| Task 2 on Resource 1 | 17998 | 7185 / 0.112 | |
| Task 5 on Resource 2 | 12600 | 26365 / 0.412 | |
| Task 6 on Resource 2 | 900 | 4143 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58445 | 3302 / 51.59 | 3062 |
| timer | 57026 | 5041 / 78.77 | 3035 |
| intake | 11948 | 2429 / 37.95 | 2206 |
| lock | 77397 | 456 / 7.12 | 455 |
| unlock | 76497 | 1881 / 29.39 | 1686 |
| job_end | 55520 | 2100 / 32.81 | 1814 |
| now | 5369168 | 226 / 3.53 | 225 |
| raise | 6000 | 2152 / 33.62 | 2065 |
| dma | 8999 | 726 / 11.34 | 553 |
| epilogue | 58445 | 1253 / 19.58 | 620 |
| restart | 900 | 5041 / 78.77 | 4861 |
| enforce | 2925 | 5041 / 78.77 | 3746 |
| view | 72301 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 2636, Task 1: 19, Task 2: 29953, Task 3: 6457, Task 4: 1214, Task 5: 3609, Task 6: 2720

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8094 cycles; Task 1: 2 (timer, intake), 5219 cycles; Task 2: 5 (dispatch, timer, intake), 14682 cycles; Task 3: 3 (dispatch, timer, intake), 8620 cycles; Task 4: 3 (dispatch, timer), 9583 cycles; Task 5: 3 (dispatch, timer, intake), 8752 cycles; Task 6: 4 (dispatch, timer, intake), 11307 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3851 cycles
- Task 0 at 179940.000 ms, 3231 cycles
- Task 2 at 179945.000 ms, 3231 cycles
- Task 0 at 179950.000 ms, 4171 cycles
- Task 5 at 179950.000 ms, 4171 cycles
- Task 0 at 179960.000 ms, 3467 cycles
- Task 3 at 179960.000 ms, 3467 cycles
- Task 2 at 179965.000 ms, 4947 cycles
- Task 0 at 179970.000 ms, 3431 cycles
- Task 5 at 179975.000 ms, 3231 cycles
- Task 0 at 179980.000 ms, 3231 cycles
- Task 0 at 179990.000 ms, 3111 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5623255 of them, the longest 24286 cycles (379.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5329185 |
| 512 to 1023 (8.0 to 16.0 µs) | 109148 |
| 1024 to 2047 (16.0 to 32.0 µs) | 55849 |
| 2048 to 4095 (32.0 to 64.0 µs) | 50389 |
| 4096 to 8191 (64.0 to 128.0 µs) | 64110 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14432 |
| 16384 to 32767 (256.0 to 512.0 µs) | 142 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56582 | 24286 / 379.5 |
| timer | 24373 | 19625 / 306.6 |
| job_end | 5509336 | 4673 / 73.0 |
| epilogue | 32964 | 14394 / 224.9 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 300042 | 18045 | 0.350 |
| 1 | 24877 | 6003 | 0.150 |
| 2 | 4810973 | 20999 | 5.300 |
| 3 | 1548022 | 5995 | 4.630 |
| 4 | 54849 | 12037 | 0.160 |
| 5 | 339641 | 7401 | 0.860 |
| 6 | 123606 | 1878 | 1.000 |
| none | 13199142 | 44590 | 15780.760 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11947
echo=5999
blocked_releases=8999
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
  at_ms=179301
  detail=58681
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179324
  detail=289600
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179484
  detail=288964
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=58617
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179575
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179644
  detail=289177
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=61378
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179775
  detail=52225
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179811
  detail=289042
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=58681
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179964
  detail=288964
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179975
  detail=1
  prs_lo=128
  prs_hi=0
event=10
  task=255
  at_ms=180000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=180000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=18000
expected_sensor=9000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 8.07s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json, docs/hil/kernel_timing_nrf52840_response.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:2b371e899dfccda96fe50601e7257cd1bdce41467ef36669bae2cf68cd2e053b) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:16ef3ddeff8b449deecb26c19f66902614bdbbb3c80e2d4d29ecb9681350291e is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,response)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 580 cycles (measured, rsk-hil release-timing record, 715 runs, max 464 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2372 cycles (measured, rsk-hil release-timing record, 714 runs, max 1897 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4128 cycles (measured, rsk-hil release-timing record, 58445 runs, max 3302 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4192 cycles (measured, rsk-hil release-timing record, 55520 runs, max 3353 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6302 cycles (measured, rsk-hil release-timing record, 57026 runs, max 5041 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2895 cycles (measured, rsk-hil release-timing record, 11948 runs, max 2316 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 30358 cycles (measured, rsk-hil release-timing record, 5623255 runs, max 24286 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 24286 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 56582 (longest 24286), timer 24373 (longest 19625), job_end 5509336 (longest 4673), epilogue 32964 (longest 14394); the longest single Rust half 5041) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 134 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2095, less Δ_entry of the plain cycle configuration, 2069, is 26, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 134 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 134 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2095, less Δ_entry of the plain cycle configuration, 2069, is 26, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 134 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6302 cycles (measured, rsk-hil release-timing record, 57026 runs, max 5041 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6302 cycles (measured, rsk-hil release-timing record, 2925 runs, max 5041 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_hm: 3297 cycles (measured, rsk-hil release-timing record, 3 runs, max 2637 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one fault entry applying the Partition's fault response (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11594 cycles (measured, rsk-hil release-timing record, 40495 runs, max 9275 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 293 cycles (measured, rsk-hil release-timing record, 59038 runs, max 234 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 920 cycles (measured, rsk-hil release-timing record, 1 runs, max 736 +25%, scenarios: nrf52840_cycles (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 706 runs, max 329 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6302 cycles (measured, rsk-hil release-timing record, 900 runs, max 5041 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_stop: 5414 cycles (measured, rsk-hil release-timing record, 1 runs, max 4331 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: an entry that stopped a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2587 cycles (measured, rsk-hil release-timing record, 163 runs, max 2069 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 163 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_send: 2947 cycles (measured, rsk-hil release-timing record, 277 runs, max 2357 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one Endpoint send SVC: the checks, the slot, the word copy into the channel, and the arrival release when declared (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_recv: 1168 cycles (measured, rsk-hil release-timing record, 369 runs, max 934 +25%, scenarios: nrf52840_response (2 s): one injected fault per response of R14.5, conditions: one Endpoint receive SVC: the checks, the oldest slot, and the word copy into the Partition (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 100 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.592 | 10.000 | 0.632 | 18000 | within |
| `part0.echo` | A | 12.195 | 20.000 | 0.307 | 5999 | within |
| `part0.sensor` | A | 14.671 | 20.000 | 5.862 | 8999 | within |
| `part1.log` | C | 41.656 | 40.000 | 4.426 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.606 | 15.000 | 5.941 | 11947 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 18.650 | 25.000 | 6.505 | 6300 | within |
| `part2.sampler` | B | 102.766 | 100.000 | 11.825 | 900 | not schedulable: no WCRT to hold |

Verdict: **pass**
