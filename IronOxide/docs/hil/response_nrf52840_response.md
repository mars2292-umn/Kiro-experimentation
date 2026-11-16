# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| target run | pass | the report ran after 26.0 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 45 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 250 ms: filter's fault \| end_job at 250 ms: END_JOB as part2's fault response \| overrun at 412 ms: log's Budget overrun \| end_job at 412 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| message_rejected at 610 ms: log's invalid reply rejected by control's validation function (R42.4) \| partition_restart at 610 ms: RESTART_PARTITION as part1's fault response to the rejected message \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| protocol_violation at 1870 ms: control's dropped role state reported as a Protocol violation (R41.2), no response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1313 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 55 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 2 (2 expected: the restarts of 600 and 800 ms), pingpong_peer 2 (2 expected), pingpong_rejected 1 (1 expected: the invalid reply of 600 ms), pingpong_reps 41 (about 43: one round per log Job but the restarts') |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 39 answered pulses before part1's restart of 800 ms, outside the 100 ms around its restart for the rejected reply (of 37), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 101 matched in all, latency 100 to 16790 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.105 | overrun | 0 | 0 | 0xfcec |
| 2 | 250.585 | memmanage | 5 | 2 | 0x0 |
| 3 | 250.585 | end_job | 5 | 2 | 0x2 |
| 4 | 412.574 | overrun | 3 | 1 | 0x5dfd5 |
| 5 | 412.574 | end_job | 3 | 1 | 0x6 |
| 6 | 501.447 | overrun | 5 | 2 | 0xcc0c |
| 7 | 501.447 | partition_restart | - | 2 | 0x6 |
| 8 | 501.447 | resource_released | 5 | 2 | 0x1 |
| 9 | 610.017 | message_rejected | 3 | 1 | 0x0 |
| 10 | 610.017 | partition_restart | - | 1 | 0x1a |
| 11 | 800.365 | memmanage | 3 | 1 | 0x0 |
| 12 | 800.365 | partition_restart | - | 1 | 0x2 |
| 13 | 1313.325 | overrun | 2 | 0 | 0x7a295 |
| 14 | 1320.765 | deadline_miss | 4 | 1 | 0x0 |
| 15 | 1325.018 | deadline_miss | 2 | 0 | 0x0 |
| 16 | 1325.018 | release_overlap | 2 | 0 | 0x0 |
| 17 | 1330.016 | endpoint_full | 0 | 0 | 0x0 |
| 18 | 1340.012 | endpoint_full | 0 | 0 | 0x0 |
| 19 | 1345.013 | deadline_miss | 2 | 0 | 0x0 |
| 20 | 1345.013 | mit_violation | 2 | 0 | 0x0 |
| 21 | 1350.012 | deadline_miss | 5 | 2 | 0x0 |
| 22 | 1350.012 | partition_stop | - | 2 | 0x7 |
| 23 | 1350.012 | endpoint_full | 0 | 0 | 0x0 |
| 24 | 1360.015 | deadline_miss | 3 | 1 | 0x0 |
| 25 | 1360.015 | mit_violation | 3 | 1 | 0x0 |
| 26 | 1360.015 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1365.012 | mit_violation | 2 | 0 | 0x0 |
| 28 | 1370.012 | endpoint_full | 0 | 0 | 0x0 |
| 29 | 1380.010 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1385.010 | mit_violation | 2 | 0 | 0x0 |
| 31 | 1390.010 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1400.010 | mit_violation | 3 | 1 | 0x0 |
| 33 | 1400.010 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1405.010 | mit_violation | 2 | 0 | 0x0 |
| 35 | 1410.011 | endpoint_full | 0 | 0 | 0x0 |
| 36 | 1420.010 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1425.010 | mit_violation | 2 | 0 | 0x0 |
| 38 | 1430.014 | endpoint_full | 0 | 0 | 0x0 |
| 39 | 1870.017 | protocol_violation | 0 | 0 | 0x0 |
| 40 | 1870.552 | protocol_violation | 0 | 0 | 0x0 |
| 41 | 1880.323 | protocol_violation | 0 | 0 | 0x0 |
| 42 | 1890.017 | protocol_violation | 0 | 0 | 0x0 |
| 43 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 44 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 4859 / 75.9 | 3622 | 2^12:187 2^13:3 |
| 2 | 90 | 4499 / 70.3 | 3301 | 2^12:88 2^13:2 |
| 3 | 45 | 4077 / 63.7 | 3604 | 2^12:45 |
| 5 | 53 | 4499 / 70.3 | 3673 | 2^12:51 2^13:2 |
| 6 | 13 | 4077 / 63.7 | 3907 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.452 | 0 | 0 | 0 |
| 1 | 63 | 0.221 | 0 | 0 | 0 |
| 2 | 90 | 125.867 | 0 | 0 | 0 |
| 3 | 44 | 119.646 | 0 | 0 | 0 |
| 4 | 99 | 130.580 | 99 | 2275 | 2087 |
| 5 | 51 | 6.208 | 0 | 0 | 0 |
| 6 | 13 | 12.807 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 148949 / 2.327 (min 16465, mean 19243) | 1 above the Budget; 2^15:190 |
| Task 1 entry | 63 | 3687 / 0.058 (min 3431, mean 3557) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7748866 / 121.076 (min 335792, mean 421378) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 212229 / 3.316 (min 151606, mean 206310) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 99 | 7473 / 0.117 (min 3521, mean 4033) | 0 above the Budget; 2^12:86 2^13:13 |
| Task 5 entry | 51 | 15829 / 0.247 (min 8429, mean 9304) | 0 above the Budget; 2^14:51 |
| Task 6 entry | 13 | 11777 / 0.184 (min 8037, mean 9739) | 0 above the Budget; 2^13:7 2^14:6 |
| Task 0 on Resource 0 | 190 | 4021 / 0.063 | |
| Task 0 on Resource 1 | 190 | 477 / 0.007 | |
| Task 2 on Resource 0 | 90 | 325454 / 5.085 | |
| Task 2 on Resource 1 | 180 | 4068 / 0.064 | |
| Task 5 on Resource 2 | 51 | 4029 / 0.063 | |
| Task 6 on Resource 2 | 13 | 488 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 555 | 3254 / 50.84 | 3016 |
| timer | 521 | 4388 / 68.56 | 3021 |
| intake | 99 | 2386 / 37.28 | 2197 |
| lock | 715 | 473 / 7.39 | 472 |
| unlock | 714 | 1867 / 29.17 | 1691 |
| job_end | 550 | 2119 / 33.11 | 1823 |
| now | 59375 | 229 / 3.58 | 228 |
| raise | 68 | 2672 / 41.75 | 2058 |
| dma | 13 | 730 / 11.41 | 728 |
| fault | 3 | 2667 / 41.67 | 2135 |
| epilogue | 554 | 1265 / 19.77 | 640 |
| restart | 3 | 4388 / 68.56 | 3242 |
| stop | 1 | 4367 / 68.23 | 4367 |
| enforce | 4 | 4388 / 68.56 | 3512 |
| view | 699 | 329 / 5.14 | 293 |
| send | 277 | 2345 / 36.64 | 2197 |
| recv | 369 | 898 / 14.03 | 837 |

Entries that interrupted an executing Job: Task 0: 29, Task 2: 337, Task 3: 32, Task 4: 13, Task 5: 13, Task 6: 6

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3509 cycles
- Task 3 at 1840.000 ms, 3509 cycles
- Task 2 at 1845.000 ms, 3229 cycles
- Task 0 at 1850.000 ms, 3802 cycles
- Task 0 at 1860.000 ms, 3221 cycles
- Task 2 at 1865.000 ms, 3221 cycles
- Task 0 at 1870.000 ms, 3794 cycles
- Task 0 at 1880.000 ms, 3513 cycles
- Task 3 at 1880.000 ms, 3513 cycles
- Task 2 at 1885.000 ms, 3221 cycles
- Task 0 at 1890.000 ms, 3794 cycles
- Task 0 at 1900.000 ms, 3617 cycles

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
  detail=500373
  prs_lo=134217728
  prs_hi=0
event=7
  task=4
  at_ms=1320
  detail=0
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
     Finished in 8.46s
```
