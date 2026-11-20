# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.3 s and reset |
| target run | pass | the report ran after 26.7 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 45 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 250 ms: filter's fault \| end_job at 250 ms: END_JOB as part2's fault response \| overrun at 412 ms: log's Budget overrun \| end_job at 412 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| message_rejected at 610 ms: log's invalid reply rejected by control's validation function (R42.4) \| partition_restart at 610 ms: RESTART_PARTITION as part1's fault response to the rejected message \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| protocol_violation at 1870 ms: control's dropped role state reported as a Protocol violation (R41.2), no response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1313 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 55 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 2 (2 expected: the restarts of 600 and 800 ms), pingpong_peer 2 (2 expected), pingpong_rejected 1 (1 expected: the invalid reply of 600 ms), pingpong_reps 41 (about 43: one round per log Job but the restarts') |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 39 answered pulses before part1's restart of 800 ms, outside the 100 ms around its restart for the rejected reply (at least 32 required), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 100 matched in all, latency 100 to 14390 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.112 | overrun | 0 | 0 | 0xfe8b |
| 2 | 250.595 | memmanage | 5 | 2 | 0x0 |
| 3 | 250.595 | end_job | 5 | 2 | 0x2 |
| 4 | 412.487 | overrun | 3 | 1 | 0x5de69 |
| 5 | 412.487 | end_job | 3 | 1 | 0x6 |
| 6 | 501.447 | overrun | 5 | 2 | 0xca6a |
| 7 | 501.447 | partition_restart | - | 2 | 0x6 |
| 8 | 501.447 | resource_released | 5 | 2 | 0x1 |
| 9 | 610.017 | message_rejected | 3 | 1 | 0x0 |
| 10 | 610.017 | partition_restart | - | 1 | 0x1a |
| 11 | 800.365 | memmanage | 3 | 1 | 0x0 |
| 12 | 800.365 | partition_restart | - | 1 | 0x2 |
| 13 | 1313.316 | overrun | 2 | 0 | 0x7a069 |
| 14 | 1325.010 | deadline_miss | 2 | 0 | 0x0 |
| 15 | 1325.010 | release_overlap | 2 | 0 | 0x0 |
| 16 | 1328.926 | deadline_miss | 4 | 1 | 0x0 |
| 17 | 1330.015 | endpoint_full | 0 | 0 | 0x0 |
| 18 | 1340.017 | endpoint_full | 0 | 0 | 0x0 |
| 19 | 1345.012 | deadline_miss | 2 | 0 | 0x0 |
| 20 | 1345.012 | mit_violation | 2 | 0 | 0x0 |
| 21 | 1350.012 | deadline_miss | 5 | 2 | 0x0 |
| 22 | 1350.012 | partition_stop | - | 2 | 0x7 |
| 23 | 1350.012 | endpoint_full | 0 | 0 | 0x0 |
| 24 | 1360.010 | deadline_miss | 3 | 1 | 0x0 |
| 25 | 1360.010 | mit_violation | 3 | 1 | 0x0 |
| 26 | 1360.010 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1365.016 | mit_violation | 2 | 0 | 0x0 |
| 28 | 1370.019 | endpoint_full | 0 | 0 | 0x0 |
| 29 | 1380.013 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1385.015 | mit_violation | 2 | 0 | 0x0 |
| 31 | 1390.018 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1400.012 | mit_violation | 3 | 1 | 0x0 |
| 33 | 1400.012 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1405.016 | mit_violation | 2 | 0 | 0x0 |
| 35 | 1410.019 | endpoint_full | 0 | 0 | 0x0 |
| 36 | 1420.012 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1425.015 | mit_violation | 2 | 0 | 0x0 |
| 38 | 1430.010 | endpoint_full | 0 | 0 | 0x0 |
| 39 | 1870.017 | protocol_violation | 0 | 0 | 0x0 |
| 40 | 1870.561 | protocol_violation | 0 | 0 | 0x0 |
| 41 | 1880.322 | protocol_violation | 0 | 0 | 0x0 |
| 42 | 1890.015 | protocol_violation | 0 | 0 | 0x0 |
| 43 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 44 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 4891 / 76.4 | 3747 | 2^12:172 2^13:18 |
| 2 | 90 | 4174 / 65.2 | 3304 | 2^12:89 2^13:1 |
| 3 | 45 | 4090 / 63.9 | 3630 | 2^12:45 |
| 5 | 53 | 4310 / 67.3 | 3729 | 2^12:41 2^13:12 |
| 6 | 13 | 4070 / 63.6 | 3923 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.454 | 0 | 0 | 0 |
| 1 | 63 | 0.219 | 0 | 0 | 0 |
| 2 | 90 | 125.858 | 0 | 0 | 0 |
| 3 | 44 | 119.647 | 0 | 0 | 0 |
| 4 | 99 | 122.407 | 99 | 2220 | 2075 |
| 5 | 51 | 6.165 | 0 | 0 | 0 |
| 6 | 13 | 12.705 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 149073 / 2.329 (min 16621, mean 19493) | 1 above the Budget; 2^15:190 |
| Task 1 entry | 63 | 3785 / 0.059 (min 3529, mean 3652) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7748659 / 121.073 (min 335315, mean 421286) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 209723 / 3.277 (min 149123, mean 207049) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 99 | 14650 / 0.229 (min 3592, mean 5057) | 0 above the Budget; 2^12:78 2^13:11 2^14:10 |
| Task 5 entry | 51 | 12264 / 0.192 (min 8528, mean 9338) | 0 above the Budget; 2^14:51 |
| Task 6 entry | 13 | 11941 / 0.187 (min 6937, mean 9140) | 0 above the Budget; 2^13:6 2^14:7 |
| Task 0 on Resource 0 | 190 | 4102 / 0.064 | |
| Task 0 on Resource 1 | 190 | 477 / 0.007 | |
| Task 2 on Resource 0 | 90 | 325171 / 5.081 | |
| Task 2 on Resource 1 | 180 | 4141 / 0.065 | |
| Task 5 on Resource 2 | 51 | 4050 / 0.063 | |
| Task 6 on Resource 2 | 13 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 555 | 3282 / 51.28 | 3062 |
| timer | 529 | 4484 / 70.06 | 3022 |
| intake | 99 | 2345 / 36.64 | 2205 |
| lock | 715 | 464 / 7.25 | 463 |
| unlock | 714 | 1897 / 29.64 | 1693 |
| job_end | 550 | 2088 / 32.62 | 1812 |
| now | 59038 | 234 / 3.66 | 232 |
| raise | 68 | 2695 / 42.11 | 2043 |
| dma | 2 | 728 / 11.38 | 728 |
| fault | 3 | 2637 / 41.20 | 2145 |
| epilogue | 554 | 1253 / 19.58 | 645 |
| restart | 3 | 4484 / 70.06 | 3272 |
| stop | 1 | 4331 / 67.67 | 4331 |
| enforce | 4 | 4484 / 70.06 | 3544 |
| view | 706 | 329 / 5.14 | 293 |
| send | 277 | 2357 / 36.83 | 2210 |
| recv | 369 | 934 / 14.59 | 875 |

Entries that interrupted an executing Job: Task 0: 33, Task 2: 331, Task 3: 27, Task 4: 41, Task 5: 13, Task 6: 7

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 6260 cycles; Task 1: none; Task 2: 57 (dispatch, timer, intake), 175954 cycles; Task 3: none; Task 4: 3 (dispatch, timer), 9937 cycles; Task 5: 1 (timer), 2902 cycles; Task 6: 1 (timer), 2918 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3508 cycles
- Task 3 at 1840.000 ms, 3508 cycles
- Task 2 at 1845.000 ms, 3192 cycles
- Task 0 at 1850.000 ms, 4086 cycles
- Task 0 at 1860.000 ms, 3228 cycles
- Task 2 at 1865.000 ms, 3232 cycles
- Task 0 at 1870.000 ms, 3862 cycles
- Task 0 at 1880.000 ms, 3468 cycles
- Task 3 at 1880.000 ms, 3468 cycles
- Task 2 at 1885.000 ms, 3232 cycles
- Task 0 at 1890.000 ms, 3770 cycles
- Task 0 at 1900.000 ms, 3608 cycles

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
pingpong_reps=41
pingpong_last=182
pingpong_reqs=23
pingpong_rejected=1
pingpong_peer=2
telemetry_peer=2
protocol_violations=4
first_memmanage_cfsr=2147483778
first_memmanage_mmfar=536870912
events=32
event=6
  task=2
  at_ms=1313
  detail=499817
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
  at_ms=1328
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
event=12
  task=0
  at_ms=1870
  detail=0
  prs_lo=16384
  prs_hi=0
event=12
  task=0
  at_ms=1870
  detail=0
  prs_lo=16384
  prs_hi=0
event=12
  task=0
  at_ms=1880
  detail=0
  prs_lo=16384
  prs_hi=0
event=12
  task=0
  at_ms=1890
  detail=0
  prs_lo=16384
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
     Finished in 8.39s
```
