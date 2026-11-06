# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8258 cycles (129.0 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.609 ms (102988 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.265 ms (16984 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.808 ms (435684 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11950 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2055 cycles (32.1 µs), mean 1882 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1953 % of the run |
| Kernel events | pass | 6754 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11949 response edges inside the run (target counted 11949) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.2 % of the run, the Kernel 45.0 % over 1308445 entries; runs per Task [18115, 6010, 20997, 7750, 11980, 7403, 1870] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22561 cycles (0.353 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7682 / 120.0 | 3513 | 2^12:17926 2^13:73 |
| 2 | 8999 | 8258 / 129.0 | 3365 | 2^12:8804 2^13:192 2^14:3 |
| 3 | 4499 | 7682 / 120.0 | 3616 | 2^12:4447 2^13:52 |
| 5 | 7199 | 8258 / 129.0 | 3634 | 2^12:7026 2^13:170 2^14:3 |
| 6 | 1799 | 7682 / 120.0 | 3946 | 2^12:1749 2^13:50 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.609 | 0 | 0 | 0 |
| 1 | 5999 | 0.265 | 0 | 0 | 0 |
| 2 | 8999 | 6.808 | 0 | 0 | 0 |
| 3 | 3375 | 4.218 | 0 | 0 | 0 |
| 4 | 11949 | 6.854 | 11950 | 2055 | 1882 |
| 5 | 6300 | 7.395 | 0 | 0 | 0 |
| 6 | 900 | 13.786 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22561 / 0.353 (min 8503, mean 11703) | 0 above the Budget; 2^14:17767 2^15:233 |
| Task 1 entry | 5999 | 9124 / 0.143 (min 2880, mean 3049) | 0 above the Budget; 2^12:5906 2^13:2 2^14:91 |
| Task 2 entry | 8999 | 405499 / 6.336 (min 397742, mean 399368) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204279 / 3.192 (min 190827, mean 195515) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11949 | 10328 / 0.161 (min 2934, mean 3055) | 0 above the Budget; 2^12:11713 2^13:206 2^14:30 |
| Task 5 entry | 6300 | 36489 / 0.570 (min 20005, mean 25448) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 18041 / 0.282 (min 6778, mean 10017) | 0 above the Budget; 2^13:204 2^14:652 2^15:44 |
| Task 0 on Resource 0 | 18000 | 7349 / 0.115 | |
| Task 0 on Resource 1 | 18000 | 6415 / 0.100 | |
| Task 2 on Resource 0 | 8999 | 391196 / 6.112 | |
| Task 2 on Resource 1 | 17998 | 6387 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 24531 / 0.383 | |
| Task 6 on Resource 2 | 900 | 3790 / 0.059 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58447 | 2662 / 41.59 | 2522 |
| timer | 65220 | 4384 / 68.50 | 3062 |
| intake | 11950 | 2123 / 33.17 | 1949 |
| lock | 77397 | 449 / 7.02 | 448 |
| unlock | 76497 | 1763 / 27.55 | 1589 |
| job_end | 55522 | 1977 / 30.89 | 1732 |
| now | 8432392 | 240 / 3.75 | 238 |
| raise | 6000 | 2009 / 31.39 | 1928 |
| dma | 900 | 706 / 11.03 | 705 |
| epilogue | 58447 | 1214 / 18.97 | 606 |
| restart | 900 | 4384 / 68.50 | 4257 |
| enforce | 2925 | 4384 / 68.50 | 3549 |
| view | 73994 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11125, Task 1: 184, Task 2: 31875, Task 3: 9213, Task 4: 267, Task 5: 2576, Task 6: 2681

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3540 cycles
- Task 0 at 179940.000 ms, 3210 cycles
- Task 2 at 179945.000 ms, 3214 cycles
- Task 0 at 179950.000 ms, 3704 cycles
- Task 5 at 179950.000 ms, 3704 cycles
- Task 0 at 179960.000 ms, 3462 cycles
- Task 3 at 179960.000 ms, 3462 cycles
- Task 2 at 179965.000 ms, 3884 cycles
- Task 0 at 179970.000 ms, 3480 cycles
- Task 5 at 179975.000 ms, 3206 cycles
- Task 0 at 179980.000 ms, 3210 cycles
- Task 0 at 179990.000 ms, 3110 cycles

## The Kernel's events

6754 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 311744 | 18115 | 0.340 |
| 1 | 22987 | 6010 | 0.140 |
| 2 | 5661592 | 20997 | 6.260 |
| 3 | 1719848 | 7750 | 4.830 |
| 4 | 44245 | 11980 | 0.140 |
| 5 | 320259 | 7403 | 0.890 |
| 6 | 60623 | 1870 | 0.530 |
| none | 12098062 | 42901 | 15176.450 |

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
pulse=11949
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
  detail=32783
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
  detail=384628
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
  detail=51831
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
  detail=384677
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
  detail=32731
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
  detail=51831
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
  detail=384761
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
  detail=32783
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
  detail=51779
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
  detail=384993
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
  detail=32783
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
  detail=384681
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
  detail=51831
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
     Finished in 7.06s
```
