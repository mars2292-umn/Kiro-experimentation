# HIL response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| target run | pass | the report ran after 26.2 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 45 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 250 ms: filter's fault \| end_job at 250 ms: END_JOB as part2's fault response \| overrun at 412 ms: log's Budget overrun \| end_job at 412 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| message_rejected at 610 ms: log's invalid reply rejected by control's validation function (R42.4) \| partition_restart at 610 ms: RESTART_PARTITION as part1's fault response to the rejected message \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| protocol_violation at 1870 ms: control's dropped role state reported as a Protocol violation (R41.2), no response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1313 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 55 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 2 (2 expected: the restarts of 600 and 800 ms), pingpong_peer 2 (2 expected), pingpong_rejected 1 (1 expected: the invalid reply of 600 ms), pingpong_reps 41 (about 43: one round per log Job but the restarts') |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 39 answered pulses before part1's restart of 800 ms, outside the 100 ms around its restart for the rejected reply (at least 32 required), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 100 matched in all, latency 100 to 17180 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.105 | overrun | 0 | 0 | 0xfcea |
| 2 | 250.595 | memmanage | 5 | 2 | 0x0 |
| 3 | 250.595 | end_job | 5 | 2 | 0x2 |
| 4 | 412.585 | overrun | 3 | 1 | 0x5e093 |
| 5 | 412.585 | end_job | 3 | 1 | 0x6 |
| 6 | 501.553 | overrun | 5 | 2 | 0xcc15 |
| 7 | 501.553 | partition_restart | - | 2 | 0x6 |
| 8 | 501.553 | resource_released | 5 | 2 | 0x1 |
| 9 | 610.014 | message_rejected | 3 | 1 | 0x0 |
| 10 | 610.014 | partition_restart | - | 1 | 0x1a |
| 11 | 800.362 | memmanage | 3 | 1 | 0x0 |
| 12 | 800.362 | partition_restart | - | 1 | 0x2 |
| 13 | 1313.323 | overrun | 2 | 0 | 0x7a270 |
| 14 | 1325.014 | deadline_miss | 2 | 0 | 0x0 |
| 15 | 1325.014 | release_overlap | 2 | 0 | 0x0 |
| 16 | 1326.124 | deadline_miss | 4 | 1 | 0x0 |
| 17 | 1330.019 | endpoint_full | 0 | 0 | 0x0 |
| 18 | 1340.017 | endpoint_full | 0 | 0 | 0x0 |
| 19 | 1345.018 | deadline_miss | 2 | 0 | 0x0 |
| 20 | 1345.018 | mit_violation | 2 | 0 | 0x0 |
| 21 | 1350.018 | deadline_miss | 5 | 2 | 0x0 |
| 22 | 1350.018 | partition_stop | - | 2 | 0x7 |
| 23 | 1350.018 | endpoint_full | 0 | 0 | 0x0 |
| 24 | 1360.012 | deadline_miss | 3 | 1 | 0x0 |
| 25 | 1360.012 | mit_violation | 3 | 1 | 0x0 |
| 26 | 1360.012 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1365.014 | mit_violation | 2 | 0 | 0x0 |
| 28 | 1370.017 | endpoint_full | 0 | 0 | 0x0 |
| 29 | 1380.018 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1385.016 | mit_violation | 2 | 0 | 0x0 |
| 31 | 1390.020 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1400.010 | mit_violation | 3 | 1 | 0x0 |
| 33 | 1400.010 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1405.020 | mit_violation | 2 | 0 | 0x0 |
| 35 | 1410.012 | endpoint_full | 0 | 0 | 0x0 |
| 36 | 1420.012 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1425.012 | mit_violation | 2 | 0 | 0x0 |
| 38 | 1430.018 | endpoint_full | 0 | 0 | 0x0 |
| 39 | 1870.013 | protocol_violation | 0 | 0 | 0x0 |
| 40 | 1870.552 | protocol_violation | 0 | 0 | 0x0 |
| 41 | 1880.319 | protocol_violation | 0 | 0 | 0x0 |
| 42 | 1890.017 | protocol_violation | 0 | 0 | 0x0 |
| 43 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 44 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 5256 / 82.1 | 3702 | 2^12:174 2^13:16 |
| 2 | 90 | 4332 / 67.7 | 3300 | 2^12:89 2^13:1 |
| 3 | 45 | 4064 / 63.5 | 3590 | 2^12:45 |
| 5 | 53 | 4332 / 67.7 | 3708 | 2^12:43 2^13:10 |
| 6 | 13 | 4064 / 63.5 | 3909 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.447 | 0 | 0 | 0 |
| 1 | 63 | 0.214 | 0 | 0 | 0 |
| 2 | 90 | 125.847 | 0 | 0 | 0 |
| 3 | 44 | 119.620 | 0 | 0 | 0 |
| 4 | 99 | 125.197 | 100 | 2248 | 2013 |
| 5 | 51 | 6.197 | 0 | 0 | 0 |
| 6 | 13 | 12.799 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 148700 / 2.323 (min 16408, mean 19213) | 1 above the Budget; 2^15:190 |
| Task 1 entry | 63 | 3783 / 0.059 (min 3503, mean 3642) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7751798 / 121.122 (min 335071, mean 421669) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 212994 / 3.328 (min 149834, mean 206452) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 99 | 7404 / 0.116 (min 3592, mean 4003) | 0 above the Budget; 2^12:89 2^13:10 |
| Task 5 entry | 51 | 13351 / 0.209 (min 8452, mean 9071) | 0 above the Budget; 2^14:51 |
| Task 6 entry | 13 | 11860 / 0.185 (min 6856, mean 9066) | 0 above the Budget; 2^13:6 2^14:7 |
| Task 0 on Resource 0 | 190 | 4062 / 0.063 | |
| Task 0 on Resource 1 | 190 | 483 / 0.008 | |
| Task 2 on Resource 0 | 90 | 323589 / 5.056 | |
| Task 2 on Resource 1 | 180 | 4024 / 0.063 | |
| Task 5 on Resource 2 | 51 | 514 / 0.008 | |
| Task 6 on Resource 2 | 13 | 494 / 0.008 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 555 | 3278 / 51.22 | 3049 |
| timer | 519 | 4386 / 68.53 | 3019 |
| intake | 100 | 2375 / 37.11 | 2140 |
| lock | 715 | 459 / 7.17 | 457 |
| unlock | 714 | 1894 / 29.59 | 1618 |
| job_end | 550 | 2109 / 32.95 | 1733 |
| now | 58982 | 230 / 3.59 | 227 |
| raise | 68 | 2559 / 39.98 | 1980 |
| dma | 2 | 717 / 11.20 | 716 |
| fault | 3 | 2544 / 39.75 | 2038 |
| epilogue | 554 | 1238 / 19.34 | 635 |
| restart | 3 | 4386 / 68.53 | 3163 |
| stop | 1 | 4308 / 67.31 | 4308 |
| enforce | 4 | 4386 / 68.53 | 3506 |
| view | 703 | 320 / 5.00 | 284 |
| send | 277 | 2310 / 36.09 | 2144 |
| recv | 369 | 935 / 14.61 | 874 |

Entries that interrupted an executing Job: Task 0: 31, Task 2: 350, Task 3: 30, Task 4: 10, Task 5: 12, Task 6: 7

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 6238 cycles; Task 1: none; Task 2: 57 (dispatch, timer, intake), 175742 cycles; Task 3: 1 (timer), 2819 cycles; Task 4: 1 (timer), 2979 cycles; Task 5: 2 (dispatch, intake), 5427 cycles; Task 6: 1 (timer), 2915 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3500 cycles
- Task 3 at 1840.000 ms, 3500 cycles
- Task 2 at 1845.000 ms, 3188 cycles
- Task 0 at 1850.000 ms, 3837 cycles
- Task 0 at 1860.000 ms, 3228 cycles
- Task 2 at 1865.000 ms, 3228 cycles
- Task 0 at 1870.000 ms, 3621 cycles
- Task 0 at 1880.000 ms, 3460 cycles
- Task 3 at 1880.000 ms, 3460 cycles
- Task 2 at 1885.000 ms, 3284 cycles
- Task 0 at 1890.000 ms, 3905 cycles
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
  detail=500336
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
  at_ms=1326
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
     Finished in 8.42s
```
