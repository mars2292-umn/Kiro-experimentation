# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9168 cycles (143.2 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.615 ms (103376 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.274 ms (17544 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.812 ms (435956 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11951 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2083 cycles (32.5 µs), mean 1871 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1941 % of the run |
| Kernel events | pass | 6756 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11950 response edges inside the run (target counted 11950) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.1 % of the run, the Kernel 45.0 % over 1559582 entries; runs per Task [18056, 6009, 20998, 7689, 12029, 7402, 1826] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22763 cycles (0.356 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9168 / 143.2 | 3543 | 2^12:17798 2^13:193 2^14:8 |
| 2 | 8999 | 8223 / 128.5 | 3325 | 2^12:8906 2^13:90 2^14:3 |
| 3 | 4499 | 7679 / 120.0 | 3590 | 2^12:4451 2^13:48 |
| 5 | 7199 | 9168 / 143.2 | 3611 | 2^12:7083 2^13:110 2^14:6 |
| 6 | 1799 | 8691 / 135.8 | 3887 | 2^12:1779 2^13:19 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.615 | 0 | 0 | 0 |
| 1 | 5999 | 0.274 | 0 | 0 | 0 |
| 2 | 8999 | 6.812 | 0 | 0 | 0 |
| 3 | 3376 | 23.571 | 0 | 0 | 0 |
| 4 | 11950 | 6.846 | 11951 | 2083 | 1871 |
| 5 | 6300 | 7.440 | 0 | 0 | 0 |
| 6 | 900 | 13.836 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22763 / 0.356 (min 8441, mean 11719) | 0 above the Budget; 2^14:17626 2^15:374 |
| Task 1 entry | 5999 | 9380 / 0.147 (min 2946, mean 3073) | 0 above the Budget; 2^12:5973 2^13:1 2^14:25 |
| Task 2 entry | 8999 | 407605 / 6.369 (min 397453, mean 399213) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3376 | 986501 / 15.414 (min 190752, mean 195632) | 1 above the Budget; 2^15:3376 |
| Task 4 entry | 11950 | 11837 / 0.185 (min 3004, mean 3142) | 0 above the Budget; 2^12:11705 2^13:178 2^14:67 |
| Task 5 entry | 6300 | 37192 / 0.581 (min 19814, mean 25491) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 17989 / 0.281 (min 6737, mean 9385) | 0 above the Budget; 2^13:246 2^14:649 2^15:5 |
| Task 0 on Resource 0 | 18000 | 10285 / 0.161 | |
| Task 0 on Resource 1 | 18000 | 10146 / 0.159 | |
| Task 2 on Resource 0 | 8999 | 391370 / 6.115 | |
| Task 2 on Resource 1 | 17998 | 6404 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 24679 / 0.386 | |
| Task 6 on Resource 2 | 900 | 7917 / 0.124 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58450 | 2827 / 44.17 | 2605 |
| timer | 64543 | 4518 / 70.59 | 3061 |
| intake | 11951 | 2153 / 33.64 | 1940 |
| lock | 77397 | 445 / 6.95 | 420 |
| unlock | 76497 | 1740 / 27.19 | 1559 |
| job_end | 55524 | 1974 / 30.84 | 1716 |
| now | 8688233 | 232 / 3.62 | 229 |
| raise | 6000 | 2012 / 31.44 | 1915 |
| dma | 900 | 705 / 11.02 | 681 |
| epilogue | 58448 | 1209 / 18.89 | 581 |
| restart | 900 | 4518 / 70.59 | 4212 |
| enforce | 2924 | 4518 / 70.59 | 3551 |
| view | 73937 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11454, Task 1: 51, Task 2: 31837, Task 3: 9029, Task 4: 324, Task 5: 2603, Task 6: 2557

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3676 cycles
- Task 0 at 179940.000 ms, 3195 cycles
- Task 2 at 179945.000 ms, 3195 cycles
- Task 0 at 179950.000 ms, 3652 cycles
- Task 5 at 179950.000 ms, 3652 cycles
- Task 0 at 179960.000 ms, 3431 cycles
- Task 3 at 179960.000 ms, 3431 cycles
- Task 2 at 179965.000 ms, 3712 cycles
- Task 0 at 179970.000 ms, 3476 cycles
- Task 5 at 179975.000 ms, 3187 cycles
- Task 0 at 179980.000 ms, 3195 cycles
- Task 0 at 179990.000 ms, 3087 cycles

## The Kernel's events

6756 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 308511 | 18056 | 0.350 |
| 1 | 22057 | 6009 | 0.140 |
| 2 | 5660177 | 20998 | 6.260 |
| 3 | 1720014 | 7689 | 4.920 |
| 4 | 44215 | 12029 | 0.140 |
| 5 | 319997 | 7402 | 0.890 |
| 6 | 59166 | 1826 | 0.610 |
| none | 12139015 | 42960 | 15501.250 |

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
log=3376
pulse=11950
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
  detail=32621
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
  detail=384992
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
  detail=52182
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
  detail=384498
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
  detail=32549
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
  detail=52182
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
  detail=384908
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
  detail=32621
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
  detail=52114
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
  detail=384626
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
  detail=32621
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
  detail=384506
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
  detail=52182
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
     Finished in 7.07s
```
