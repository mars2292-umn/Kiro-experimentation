# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| target run | pass | the report ran after 24.6 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 40 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 251 ms: filter's fault \| end_job at 251 ms: END_JOB as part2's fault response \| overrun at 413 ms: log's Budget overrun \| end_job at 413 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1314 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 55 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 1 (1 expected), pingpong_peer 1 (1 expected), pingpong_rejected 0 (0 expected), pingpong_reps 44 (about 45: one round per log Job but the restart's) |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 44 answered pulses before part1's restart (of 43), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 100 matched in all, latency 90 to 17820 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.108 | overrun | 0 | 0 | 0xfe10 |
| 2 | 251.597 | memmanage | 5 | 2 | 0x0 |
| 3 | 251.597 | end_job | 5 | 2 | 0x2 |
| 4 | 413.518 | overrun | 3 | 1 | 0x5df12 |
| 5 | 413.518 | end_job | 3 | 1 | 0x6 |
| 6 | 501.407 | overrun | 5 | 2 | 0xcb88 |
| 7 | 501.407 | partition_restart | - | 2 | 0x6 |
| 8 | 501.407 | resource_released | 5 | 2 | 0x1 |
| 9 | 800.327 | memmanage | 3 | 1 | 0x0 |
| 10 | 800.327 | partition_restart | - | 1 | 0x2 |
| 11 | 1314.339 | overrun | 2 | 0 | 0x89c95 |
| 12 | 1325.012 | deadline_miss | 2 | 0 | 0x0 |
| 13 | 1325.012 | release_overlap | 2 | 0 | 0x0 |
| 14 | 1327.448 | deadline_miss | 4 | 1 | 0x0 |
| 15 | 1330.017 | endpoint_full | 0 | 0 | 0x0 |
| 16 | 1340.019 | endpoint_full | 0 | 0 | 0x0 |
| 17 | 1345.011 | deadline_miss | 2 | 0 | 0x0 |
| 18 | 1345.011 | mit_violation | 2 | 0 | 0x0 |
| 19 | 1350.017 | deadline_miss | 5 | 2 | 0x0 |
| 20 | 1350.017 | partition_stop | - | 2 | 0x7 |
| 21 | 1350.017 | endpoint_full | 0 | 0 | 0x0 |
| 22 | 1360.015 | deadline_miss | 3 | 1 | 0x0 |
| 23 | 1360.015 | mit_violation | 3 | 1 | 0x0 |
| 24 | 1360.015 | endpoint_full | 0 | 0 | 0x0 |
| 25 | 1365.016 | mit_violation | 2 | 0 | 0x0 |
| 26 | 1370.014 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1380.012 | endpoint_full | 0 | 0 | 0x0 |
| 28 | 1385.010 | mit_violation | 2 | 0 | 0x0 |
| 29 | 1390.019 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1400.010 | mit_violation | 3 | 1 | 0x0 |
| 31 | 1400.010 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1405.018 | mit_violation | 2 | 0 | 0x0 |
| 33 | 1410.017 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1420.019 | endpoint_full | 0 | 0 | 0x0 |
| 35 | 1425.017 | mit_violation | 2 | 0 | 0x0 |
| 36 | 1430.018 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1440.018 | release_overlap | 3 | 1 | 0x0 |
| 38 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 39 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 5137 / 80.3 | 3604 | 2^12:187 2^13:3 |
| 2 | 90 | 4117 / 64.3 | 3267 | 2^12:89 2^13:1 |
| 3 | 45 | 4316 / 67.4 | 3578 | 2^12:44 2^13:1 |
| 5 | 53 | 4117 / 64.3 | 3615 | 2^12:52 2^13:1 |
| 6 | 13 | 4001 / 62.5 | 3859 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.404 | 0 | 0 | 0 |
| 1 | 63 | 0.243 | 0 | 0 | 0 |
| 2 | 90 | 126.848 | 0 | 0 | 0 |
| 3 | 44 | 121.709 | 0 | 0 | 0 |
| 4 | 99 | 125.848 | 99 | 2165 | 2014 |
| 5 | 51 | 7.120 | 0 | 0 | 0 |
| 6 | 13 | 13.739 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 145971 / 2.281 (min 14615, mean 18944) | 1 above the Budget; 2^14:49 2^15:141 |
| Task 1 entry | 63 | 3264 / 0.051 (min 3076, mean 3177) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7841644 / 122.526 (min 398375, mean 484337) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 214346 / 3.349 (min 154192, mean 204662) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 99 | 6677 / 0.104 (min 3165, mean 3241) | 0 above the Budget; 2^12:98 2^13:1 |
| Task 5 entry | 51 | 11307 / 0.177 (min 7783, mean 7931) | 0 above the Budget; 2^13:50 2^14:1 |
| Task 6 entry | 13 | 11148 / 0.174 (min 7456, mean 9695) | 0 above the Budget; 2^13:5 2^14:8 |
| Task 0 on Resource 0 | 190 | 402 / 0.006 | |
| Task 0 on Resource 1 | 190 | 4154 / 0.065 | |
| Task 2 on Resource 0 | 90 | 388914 / 6.077 | |
| Task 2 on Resource 1 | 180 | 3994 / 0.062 | |
| Task 5 on Resource 2 | 51 | 3904 / 0.061 | |
| Task 6 on Resource 2 | 13 | 414 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 555 | 2814 / 43.97 | 2663 |
| timer | 601 | 4401 / 68.77 | 3010 |
| intake | 99 | 2291 / 35.80 | 2139 |
| lock | 715 | 469 / 7.33 | 467 |
| unlock | 714 | 1849 / 28.89 | 1643 |
| job_end | 550 | 2100 / 32.81 | 1792 |
| now | 72020 | 243 / 3.80 | 242 |
| raise | 63 | 2103 / 32.86 | 1987 |
| dma | 13 | 749 / 11.70 | 748 |
| fault | 3 | 2553 / 39.89 | 2036 |
| epilogue | 554 | 1214 / 18.97 | 611 |
| restart | 2 | 4401 / 68.77 | 3477 |
| stop | 1 | 4289 / 67.02 | 4289 |
| enforce | 4 | 4401 / 68.77 | 3471 |
| view | 703 | 329 / 5.14 | 293 |
| send | 280 | 2161 / 33.77 | 2017 |
| recv | 371 | 726 / 11.34 | 667 |

Entries that interrupted an executing Job: Task 0: 127, Task 2: 354, Task 3: 40, Task 4: 1, Task 5: 3, Task 6: 8

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3465 cycles
- Task 3 at 1840.000 ms, 3465 cycles
- Task 2 at 1845.000 ms, 3169 cycles
- Task 0 at 1850.000 ms, 3810 cycles
- Task 0 at 1860.000 ms, 3201 cycles
- Task 2 at 1865.000 ms, 3201 cycles
- Task 0 at 1870.000 ms, 3774 cycles
- Task 0 at 1880.000 ms, 3433 cycles
- Task 3 at 1880.000 ms, 3433 cycles
- Task 2 at 1885.000 ms, 3201 cycles
- Task 0 at 1890.000 ms, 3762 cycles
- Task 0 at 1900.000 ms, 3569 cycles

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 92000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/response/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/response/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=190
sensor=90
log=25
pulse=55
echo=63
blocked_releases=89
filter=32
filter_state_bits=1120220744
sampler=8
dma_ok=8
telemetry_rx=96
telemetry_last=188
telemetry_full=11
pingpong_reps=44
pingpong_last=186
pingpong_reqs=24
pingpong_rejected=0
pingpong_peer=1
telemetry_peer=1
protocol_violations=0
first_memmanage_cfsr=2147483778
first_memmanage_mmfar=536870912
events=32
event=11
  task=5
  at_ms=501
  detail=1
  prs_lo=128
  prs_hi=0
event=2
  task=3
  at_ms=800
  detail=0
  prs_lo=541065216
  prs_hi=0
event=16
  task=255
  at_ms=800
  detail=2
  prs_lo=0
  prs_hi=0
event=6
  task=2
  at_ms=1314
  detail=564373
  prs_lo=134217728
  prs_hi=0
event=7
  task=2
  at_ms=1325
  detail=0
  prs_lo=134217728
  prs_hi=0
event=8
  task=2
  at_ms=1325
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=4
  at_ms=1327
  detail=0
  prs_lo=134217728
  prs_hi=0
event=25
  task=0
  at_ms=1330
  detail=0
  prs_lo=16384
  prs_hi=0
event=25
  task=0
  at_ms=1340
  detail=0
  prs_lo=16384
  prs_hi=0
event=7
  task=2
  at_ms=1345
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=2
  at_ms=1345
  detail=0
  prs_lo=4096
  prs_hi=0
event=7
  task=5
  at_ms=1350
  detail=0
  prs_lo=134217728
  prs_hi=0
event=17
  task=255
  at_ms=1350
  detail=7
  prs_lo=0
  prs_hi=0
event=25
  task=0
  at_ms=1350
  detail=0
  prs_lo=16384
  prs_hi=0
event=7
  task=3
  at_ms=1360
  detail=0
  prs_lo=134217728
  prs_hi=0
event=9
  task=3
  at_ms=1360
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1360
  detail=0
  prs_lo=16384
  prs_hi=0
event=9
  task=2
  at_ms=1365
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1370
  detail=0
  prs_lo=16384
  prs_hi=0
event=25
  task=0
  at_ms=1380
  detail=0
  prs_lo=16384
  prs_hi=0
event=9
  task=2
  at_ms=1385
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1390
  detail=0
  prs_lo=16384
  prs_hi=0
event=9
  task=3
  at_ms=1400
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1400
  detail=0
  prs_lo=16384
  prs_hi=0
event=9
  task=2
  at_ms=1405
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1410
  detail=0
  prs_lo=16384
  prs_hi=0
event=25
  task=0
  at_ms=1420
  detail=0
  prs_lo=16384
  prs_hi=0
event=9
  task=2
  at_ms=1425
  detail=0
  prs_lo=4096
  prs_hi=0
event=25
  task=0
  at_ms=1430
  detail=0
  prs_lo=16384
  prs_hi=0
event=8
  task=3
  at_ms=1440
  detail=0
  prs_lo=4096
  prs_hi=0
event=2
  task=0
  at_ms=1900
  detail=0
  prs_lo=541065216
  prs_hi=0
event=18
  task=255
  at_ms=1900
  detail=0
  prs_lo=0
  prs_hi=0
RSK-OK
Firmware exited successfully
     Finished in 7.72s
```
