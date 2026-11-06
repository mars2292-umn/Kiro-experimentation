# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8413 cycles (131.5 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.609 ms (102984 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.274 ms (17524 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.812 ms (435988 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11954 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2095 cycles (32.7 µs), mean 1898 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1970 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11953 response edges inside the run (target counted 11953) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.1 % of the run, the Kernel 45.2 % over 1372302 entries; runs per Task [18108, 6010, 20999, 7698, 12038, 7404, 1823] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22266 cycles (0.348 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8413 / 131.5 | 3376 | 2^12:17849 2^13:145 2^14:5 |
| 2 | 8999 | 8129 / 127.0 | 3097 | 2^12:8921 2^13:78 |
| 3 | 4499 | 8408 / 131.4 | 3378 | 2^12:4430 2^13:68 2^14:1 |
| 5 | 7199 | 8408 / 131.4 | 3423 | 2^12:7125 2^13:72 2^14:2 |
| 6 | 1799 | 8408 / 131.4 | 3658 | 2^12:1776 2^13:21 2^14:2 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.609 | 0 | 0 | 0 |
| 1 | 5999 | 0.274 | 0 | 0 | 0 |
| 2 | 8999 | 6.812 | 0 | 0 | 0 |
| 3 | 3375 | 4.224 | 0 | 0 | 0 |
| 4 | 11953 | 6.846 | 11954 | 2095 | 1898 |
| 5 | 6300 | 7.448 | 0 | 0 | 0 |
| 6 | 900 | 13.832 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22266 / 0.348 (min 8631, mean 11721) | 0 above the Budget; 2^14:17832 2^15:168 |
| Task 1 entry | 5999 | 9297 / 0.145 (min 3002, mean 3123) | 0 above the Budget; 2^12:5968 2^13:1 2^14:30 |
| Task 2 entry | 8999 | 407519 / 6.367 (min 397891, mean 399605) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204036 / 3.188 (min 188072, mean 195479) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11953 | 11731 / 0.183 (min 3056, mean 3183) | 0 above the Budget; 2^12:11728 2^13:156 2^14:69 |
| Task 5 entry | 6300 | 37321 / 0.583 (min 19977, mean 26037) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 17572 / 0.275 (min 6908, mean 9337) | 0 above the Budget; 2^13:249 2^14:650 2^15:1 |
| Task 0 on Resource 0 | 18000 | 9669 / 0.151 | |
| Task 0 on Resource 1 | 18000 | 9530 / 0.149 | |
| Task 2 on Resource 0 | 8999 | 394307 / 6.161 | |
| Task 2 on Resource 1 | 17998 | 6316 / 0.099 | |
| Task 5 on Resource 2 | 12600 | 24863 / 0.388 | |
| Task 6 on Resource 2 | 900 | 3558 / 0.056 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58451 | 2849 / 44.52 | 2662 |
| timer | 65319 | 4241 / 66.27 | 2833 |
| intake | 11954 | 2167 / 33.86 | 1969 |
| lock | 77397 | 454 / 7.09 | 453 |
| unlock | 76497 | 1761 / 27.52 | 1587 |
| job_end | 55526 | 1977 / 30.89 | 1731 |
| now | 8508273 | 235 / 3.67 | 232 |
| raise | 6000 | 2016 / 31.50 | 1935 |
| dma | 900 | 707 / 11.05 | 706 |
| epilogue | 58451 | 1215 / 18.98 | 605 |
| restart | 900 | 4241 / 66.27 | 4077 |
| enforce | 2925 | 4241 / 66.27 | 3354 |
| view | 73956 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11217, Task 1: 61, Task 2: 32050, Task 3: 9074, Task 4: 312, Task 5: 2610, Task 6: 2557

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3605 cycles
- Task 0 at 179940.000 ms, 2984 cycles
- Task 2 at 179945.000 ms, 2988 cycles
- Task 0 at 179950.000 ms, 4753 cycles
- Task 5 at 179950.000 ms, 4753 cycles
- Task 0 at 179960.000 ms, 3212 cycles
- Task 3 at 179960.000 ms, 3212 cycles
- Task 2 at 179965.000 ms, 3225 cycles
- Task 0 at 179970.000 ms, 3305 cycles
- Task 5 at 179975.000 ms, 2980 cycles
- Task 0 at 179980.000 ms, 2984 cycles
- Task 0 at 179990.000 ms, 2904 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 308461 | 18108 | 0.340 |
| 1 | 22427 | 6010 | 0.140 |
| 2 | 5665821 | 20999 | 6.300 |
| 3 | 1719126 | 7698 | 4.920 |
| 4 | 44588 | 12038 | 0.140 |
| 5 | 324520 | 7404 | 0.880 |
| 6 | 58674 | 1823 | 0.610 |
| none | 12176639 | 42947 | 15708.480 |

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
pulse=11953
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=179301
  detail=32515
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
  at_ms=179333
  detail=384911
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179333
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=51981
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
  at_ms=179492
  detail=384797
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179492
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=32983
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
  detail=51981
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
  at_ms=179653
  detail=384771
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179653
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=32515
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
  detail=51909
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
  at_ms=179813
  detail=384513
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179813
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=32515
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
  at_ms=179972
  detail=385013
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179972
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=51981
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
     Finished in 7.42s
```
