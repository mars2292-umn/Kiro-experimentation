# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8928 cycles (139.5 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.622 ms (103832 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.273 ms (17460 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.814 ms (436112 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11951 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2136 cycles (33.4 µs), mean 1900 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1971 % of the run |
| Kernel events | pass | 6756 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11950 response edges inside the run (target counted 11950) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.2 % of the run, the Kernel 45.0 % over 1627918 entries; runs per Task [18059, 6005, 20997, 7693, 12038, 7381, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22863 cycles (0.357 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8824 / 137.9 | 3579 | 2^12:17808 2^13:177 2^14:14 |
| 2 | 8999 | 8928 / 139.5 | 3335 | 2^12:8829 2^13:165 2^14:5 |
| 3 | 4499 | 8603 / 134.4 | 3605 | 2^12:4453 2^13:45 2^14:1 |
| 5 | 7199 | 8099 / 126.5 | 3643 | 2^12:6998 2^13:201 |
| 6 | 1799 | 7859 / 122.8 | 3907 | 2^12:1787 2^13:12 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.622 | 0 | 0 | 0 |
| 1 | 5999 | 0.273 | 0 | 0 | 0 |
| 2 | 8999 | 6.814 | 0 | 0 | 0 |
| 3 | 3376 | 23.489 | 0 | 0 | 0 |
| 4 | 11950 | 6.847 | 11951 | 2136 | 1900 |
| 5 | 6300 | 7.434 | 0 | 0 | 0 |
| 6 | 900 | 13.854 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22863 / 0.357 (min 8373, mean 11711) | 0 above the Budget; 2^14:17748 2^15:252 |
| Task 1 entry | 5999 | 9332 / 0.146 (min 2838, mean 2975) | 0 above the Budget; 2^12:5979 2^13:0 2^14:20 |
| Task 2 entry | 8999 | 407473 / 6.367 (min 397409, mean 399044) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3376 | 1013243 / 15.832 (min 190728, mean 195619) | 1 above the Budget; 2^15:3376 |
| Task 4 entry | 11950 | 11739 / 0.183 (min 2896, mean 3048) | 0 above the Budget; 2^12:11716 2^13:162 2^14:72 |
| Task 5 entry | 6300 | 36762 / 0.574 (min 19698, mean 25758) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 18049 / 0.282 (min 6657, mean 9321) | 0 above the Budget; 2^13:246 2^14:651 2^15:3 |
| Task 0 on Resource 0 | 18000 | 9990 / 0.156 | |
| Task 0 on Resource 1 | 18000 | 9972 / 0.156 | |
| Task 2 on Resource 0 | 8999 | 391408 / 6.116 | |
| Task 2 on Resource 1 | 17998 | 6556 / 0.102 | |
| Task 5 on Resource 2 | 12600 | 24849 / 0.388 | |
| Task 6 on Resource 2 | 900 | 3799 / 0.059 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58448 | 2741 / 42.83 | 2520 |
| timer | 65307 | 4486 / 70.09 | 3044 |
| intake | 11951 | 2206 / 34.47 | 1969 |
| lock | 77397 | 445 / 6.95 | 430 |
| unlock | 76497 | 1789 / 27.95 | 1597 |
| job_end | 55524 | 2019 / 31.55 | 1759 |
| now | 8758903 | 226 / 3.53 | 225 |
| raise | 6000 | 2022 / 31.59 | 1929 |
| dma | 900 | 701 / 10.95 | 688 |
| epilogue | 58448 | 1151 / 17.98 | 580 |
| restart | 900 | 4486 / 70.09 | 4255 |
| enforce | 2924 | 4486 / 70.09 | 3554 |

Entries that interrupted an executing Job: Task 0: 11524, Task 1: 40, Task 2: 31776, Task 3: 9034, Task 4: 324, Task 5: 2537, Task 6: 2553

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3356 cycles
- Task 0 at 179940.000 ms, 3175 cycles
- Task 2 at 179945.000 ms, 3179 cycles
- Task 0 at 179950.000 ms, 3828 cycles
- Task 5 at 179950.000 ms, 3828 cycles
- Task 0 at 179960.000 ms, 3423 cycles
- Task 3 at 179960.000 ms, 3423 cycles
- Task 2 at 179965.000 ms, 3416 cycles
- Task 0 at 179970.000 ms, 3360 cycles
- Task 5 at 179975.000 ms, 3171 cycles
- Task 0 at 179980.000 ms, 3179 cycles
- Task 0 at 179990.000 ms, 3071 cycles

## The Kernel's events

6756 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 312155 | 18059 | 0.350 |
| 1 | 22493 | 6005 | 0.140 |
| 2 | 5657858 | 20997 | 6.260 |
| 3 | 1720235 | 7693 | 6.660 |
| 4 | 45133 | 12038 | 0.130 |
| 5 | 323682 | 7381 | 0.870 |
| 6 | 59669 | 1827 | 0.550 |
| none | 12126807 | 42968 | 15464.970 |

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
  detail=32812
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
  detail=384511
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
  detail=51913
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
  detail=384593
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
  detail=32752
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
  detail=51913
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
  detail=384747
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
  detail=32812
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
  detail=51849
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
  detail=384709
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
  detail=32812
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
  at_ms=179973
  detail=384849
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179973
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=51913
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
