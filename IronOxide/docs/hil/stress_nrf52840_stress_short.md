# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8359 cycles (130.6 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.607 ms (102852 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.275 ms (17592 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.814 ms (436116 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11952 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2127 cycles (33.2 µs), mean 1896 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1967 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11951 response edges inside the run (target counted 11951) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.1 % of the run, the Kernel 45.3 % over 1248676 entries; runs per Task [18109, 6011, 20998, 7700, 11990, 7352, 1828] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22515 cycles (0.352 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8151 / 127.4 | 3399 | 2^12:17878 2^13:121 |
| 2 | 8999 | 8359 / 130.6 | 3176 | 2^12:8904 2^13:90 2^14:5 |
| 3 | 4499 | 8151 / 127.4 | 3452 | 2^12:4451 2^13:48 |
| 5 | 7199 | 8359 / 130.6 | 3467 | 2^12:7148 2^13:49 2^14:2 |
| 6 | 1799 | 3883 / 60.7 | 3730 | 2^12:1799 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.607 | 0 | 0 | 0 |
| 1 | 5999 | 0.275 | 0 | 0 | 0 |
| 2 | 8999 | 6.814 | 0 | 0 | 0 |
| 3 | 3375 | 4.226 | 0 | 0 | 0 |
| 4 | 11951 | 6.859 | 11952 | 2127 | 1896 |
| 5 | 6300 | 7.448 | 0 | 0 | 0 |
| 6 | 900 | 13.841 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22515 / 0.352 (min 8666, mean 11841) | 0 above the Budget; 2^14:17840 2^15:160 |
| Task 1 entry | 5999 | 9394 / 0.147 (min 3013, mean 3120) | 0 above the Budget; 2^12:5977 2^13:1 2^14:21 |
| Task 2 entry | 8999 | 407519 / 6.367 (min 397783, mean 399429) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204402 / 3.194 (min 190882, mean 195609) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11951 | 11797 / 0.184 (min 3069, mean 3184) | 0 above the Budget; 2^12:11733 2^13:181 2^14:37 |
| Task 5 entry | 6300 | 37247 / 0.582 (min 20162, mean 25798) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 17918 / 0.280 (min 6913, mean 9408) | 0 above the Budget; 2^13:248 2^14:650 2^15:2 |
| Task 0 on Resource 0 | 18000 | 9853 / 0.154 | |
| Task 0 on Resource 1 | 18000 | 9786 / 0.153 | |
| Task 2 on Resource 0 | 8999 | 390938 / 6.108 | |
| Task 2 on Resource 1 | 17998 | 6349 / 0.099 | |
| Task 5 on Resource 2 | 12600 | 24787 / 0.387 | |
| Task 6 on Resource 2 | 900 | 7871 / 0.123 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58449 | 2847 / 44.48 | 2667 |
| timer | 65256 | 4129 / 64.52 | 2898 |
| intake | 11952 | 2197 / 34.33 | 1965 |
| lock | 77397 | 455 / 7.11 | 454 |
| unlock | 76497 | 1765 / 27.58 | 1586 |
| job_end | 55524 | 1981 / 30.95 | 1726 |
| now | 8379055 | 240 / 3.75 | 238 |
| raise | 6000 | 2007 / 31.36 | 1923 |
| dma | 900 | 695 / 10.86 | 694 |
| epilogue | 58449 | 1230 / 19.22 | 621 |
| restart | 900 | 4129 / 64.52 | 4116 |
| enforce | 2925 | 4129 / 64.52 | 3408 |
| view | 73862 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11508, Task 1: 43, Task 2: 31729, Task 3: 9044, Task 4: 261, Task 5: 2426, Task 6: 2537

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3257 cycles
- Task 0 at 179940.000 ms, 3047 cycles
- Task 2 at 179945.000 ms, 3051 cycles
- Task 0 at 179950.000 ms, 3601 cycles
- Task 5 at 179950.000 ms, 3601 cycles
- Task 0 at 179960.000 ms, 3287 cycles
- Task 3 at 179960.000 ms, 3287 cycles
- Task 2 at 179965.000 ms, 3425 cycles
- Task 0 at 179970.000 ms, 3465 cycles
- Task 5 at 179975.000 ms, 3047 cycles
- Task 0 at 179980.000 ms, 3051 cycles
- Task 0 at 179990.000 ms, 2935 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 311712 | 18109 | 0.340 |
| 1 | 22244 | 6011 | 0.140 |
| 2 | 5663336 | 20998 | 6.260 |
| 3 | 1719947 | 7700 | 4.920 |
| 4 | 43939 | 11990 | 0.130 |
| 5 | 322134 | 7352 | 0.850 |
| 6 | 59196 | 1828 | 0.530 |
| none | 12184916 | 43037 | 15716.960 |

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
pulse=11951
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
  detail=32959
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
  detail=384828
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
  detail=52167
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
  detail=385055
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
  detail=32899
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
  detail=52167
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
  detail=384943
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
  detail=32959
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
  detail=52103
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
  detail=384647
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
  detail=32959
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
  detail=385055
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
  detail=52167
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
     Finished in 7.41s
```
