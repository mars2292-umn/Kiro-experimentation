# HIL coverage run (R34.4, R53.3) over the response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| target run | pass | the report ran after 27.2 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 45 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 250 ms: filter's fault \| end_job at 250 ms: END_JOB as part2's fault response \| overrun at 412 ms: log's Budget overrun \| end_job at 412 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| message_rejected at 610 ms: log's invalid reply rejected by control's validation function (R42.4) \| partition_restart at 610 ms: RESTART_PARTITION as part1's fault response to the rejected message \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| protocol_violation at 1870 ms: control's dropped role state reported as a Protocol violation (R41.2), no response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1313 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 54 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 2 (2 expected: the restarts of 600 and 800 ms), pingpong_peer 2 (2 expected), pingpong_rejected 1 (1 expected: the invalid reply of 600 ms), pingpong_reps 41 (about 43: one round per log Job but the restarts') |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 39 answered pulses before part1's restart of 800 ms, outside the 100 ms around its restart for the rejected reply (at least 32 required), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 99 matched in all, latency 100 to 15170 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.108 | overrun | 0 | 0 | 0xfdb5 |
| 2 | 250.566 | memmanage | 5 | 2 | 0x0 |
| 3 | 250.566 | end_job | 5 | 2 | 0x2 |
| 4 | 412.636 | overrun | 3 | 1 | 0x5e103 |
| 5 | 412.636 | end_job | 3 | 1 | 0x6 |
| 6 | 501.624 | overrun | 5 | 2 | 0xca6a |
| 7 | 501.624 | partition_restart | - | 2 | 0x6 |
| 8 | 501.624 | resource_released | 5 | 2 | 0x1 |
| 9 | 610.018 | message_rejected | 3 | 1 | 0x0 |
| 10 | 610.018 | partition_restart | - | 1 | 0x1a |
| 11 | 800.375 | memmanage | 3 | 1 | 0x0 |
| 12 | 800.375 | partition_restart | - | 1 | 0x2 |
| 13 | 1313.337 | overrun | 2 | 0 | 0x7a14c |
| 14 | 1325.010 | deadline_miss | 2 | 0 | 0x0 |
| 15 | 1325.010 | release_overlap | 2 | 0 | 0x0 |
| 16 | 1325.147 | deadline_miss | 4 | 1 | 0x0 |
| 17 | 1330.010 | endpoint_full | 0 | 0 | 0x0 |
| 18 | 1340.017 | endpoint_full | 0 | 0 | 0x0 |
| 19 | 1345.010 | deadline_miss | 2 | 0 | 0x0 |
| 20 | 1345.010 | mit_violation | 2 | 0 | 0x0 |
| 21 | 1350.011 | deadline_miss | 5 | 2 | 0x0 |
| 22 | 1350.011 | partition_stop | - | 2 | 0x7 |
| 23 | 1350.011 | endpoint_full | 0 | 0 | 0x0 |
| 24 | 1360.020 | deadline_miss | 3 | 1 | 0x0 |
| 25 | 1360.020 | mit_violation | 3 | 1 | 0x0 |
| 26 | 1360.020 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1365.016 | mit_violation | 2 | 0 | 0x0 |
| 28 | 1370.010 | endpoint_full | 0 | 0 | 0x0 |
| 29 | 1380.013 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1385.020 | mit_violation | 2 | 0 | 0x0 |
| 31 | 1390.013 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1400.010 | mit_violation | 3 | 1 | 0x0 |
| 33 | 1400.010 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1405.011 | mit_violation | 2 | 0 | 0x0 |
| 35 | 1410.017 | endpoint_full | 0 | 0 | 0x0 |
| 36 | 1420.011 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1425.020 | mit_violation | 2 | 0 | 0x0 |
| 38 | 1430.015 | endpoint_full | 0 | 0 | 0x0 |
| 39 | 1870.017 | protocol_violation | 0 | 0 | 0x0 |
| 40 | 1870.576 | protocol_violation | 0 | 0 | 0x0 |
| 41 | 1880.331 | protocol_violation | 0 | 0 | 0x0 |
| 42 | 1890.014 | protocol_violation | 0 | 0 | 0x0 |
| 43 | 1900.010 | memmanage | 0 | 0 | 0x0 |
| 44 | 1900.010 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 4806 / 75.1 | 3665 | 2^12:176 2^13:14 |
| 2 | 90 | 4101 / 64.1 | 3293 | 2^12:89 2^13:1 |
| 3 | 45 | 4101 / 64.1 | 3593 | 2^12:44 2^13:1 |
| 5 | 53 | 4329 / 67.6 | 3687 | 2^12:43 2^13:10 |
| 6 | 13 | 4101 / 64.1 | 3900 | 2^12:12 2^13:1 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.469 | 0 | 0 | 0 |
| 1 | 63 | 0.221 | 0 | 0 | 0 |
| 2 | 90 | 125.876 | 0 | 0 | 0 |
| 3 | 44 | 119.699 | 0 | 0 | 0 |
| 4 | 98 | 126.216 | 99 | 2279 | 2102 |
| 5 | 51 | 6.251 | 0 | 0 | 0 |
| 6 | 13 | 12.857 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 150026 / 2.344 (min 17286, mean 20425) | 1 above the Budget; 2^15:190 |
| Task 1 entry | 63 | 3864 / 0.060 (min 3607, mean 3730) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7736013 / 120.875 (min 336202, mean 421632) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 214250 / 3.348 (min 148086, mean 208278) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 98 | 7548 / 0.118 (min 3740, mean 4169) | 0 above the Budget; 2^12:88 2^13:10 |
| Task 5 entry | 51 | 12767 / 0.199 (min 8807, mean 9575) | 0 above the Budget; 2^14:51 |
| Task 6 entry | 13 | 12487 / 0.195 (min 7303, mean 9925) | 0 above the Budget; 2^13:5 2^14:8 |
| Task 0 on Resource 0 | 190 | 4208 / 0.066 | |
| Task 0 on Resource 1 | 190 | 515 / 0.008 | |
| Task 2 on Resource 0 | 90 | 325120 / 5.080 | |
| Task 2 on Resource 1 | 180 | 4218 / 0.066 | |
| Task 5 on Resource 2 | 51 | 4215 / 0.066 | |
| Task 6 on Resource 2 | 13 | 595 / 0.009 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 554 | 3260 / 50.94 | 3044 |
| timer | 519 | 4410 / 68.91 | 3026 |
| intake | 99 | 2403 / 37.55 | 2234 |
| lock | 715 | 462 / 7.22 | 460 |
| unlock | 714 | 1932 / 30.19 | 1710 |
| job_end | 549 | 2071 / 32.36 | 1807 |
| now | 52581 | 228 / 3.56 | 226 |
| raise | 68 | 2681 / 41.89 | 2043 |
| dma | 2 | 854 / 13.34 | 854 |
| fault | 3 | 2613 / 40.83 | 2117 |
| epilogue | 553 | 1245 / 19.45 | 626 |
| restart | 3 | 4410 / 68.91 | 3235 |
| stop | 1 | 4317 / 67.45 | 4317 |
| enforce | 4 | 4410 / 68.91 | 3518 |
| view | 698 | 308 / 4.81 | 274 |
| send | 277 | 2397 / 37.45 | 2235 |
| recv | 369 | 934 / 14.59 | 871 |

Entries that interrupted an executing Job: Task 0: 45, Task 2: 338, Task 3: 33, Task 4: 10, Task 5: 10, Task 6: 9

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 2 (timer), 6276 cycles; Task 1: none; Task 2: 57 (dispatch, timer, intake), 175927 cycles; Task 3: 1 (timer), 2820 cycles; Task 4: 1 (timer), 3000 cycles; Task 5: 1 (timer), 2940 cycles; Task 6: 1 (timer), 2936 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3497 cycles
- Task 3 at 1840.000 ms, 3497 cycles
- Task 2 at 1845.000 ms, 3185 cycles
- Task 0 at 1850.000 ms, 3573 cycles
- Task 0 at 1860.000 ms, 3221 cycles
- Task 2 at 1865.000 ms, 3225 cycles
- Task 0 at 1870.000 ms, 3877 cycles
- Task 0 at 1880.000 ms, 3457 cycles
- Task 3 at 1880.000 ms, 3457 cycles
- Task 2 at 1885.000 ms, 3281 cycles
- Task 0 at 1890.000 ms, 3685 cycles
- Task 0 at 1900.000 ms, 3589 cycles

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 92000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_response/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_response/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=190
sensor=90
log=25
pulse=54
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
  detail=500044
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
  at_ms=1325
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
     Finished in 9.13s
```

## Branch coverage of the Task entries under the suite (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 94 % (17 of 18 branch outcomes), part0.echo 83 % (5 of 6 branch outcomes), part0.sensor 83 % (10 of 12 branch outcomes), part1.log 96 % (25 of 26 branch outcomes), part1.pulse 91 % (11 of 12 branch outcomes), part2.filter 81 % (13 of 16 branch outcomes), part2.sampler 71 % (10 of 14 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 29 | 18 | 17 | 94 % | pass |
| `part0.control#cs:state` | 29 | 18 | 17 | 94 % | pass |
| `part0.control#cs:in_cs` | 29 | 18 | 17 | 94 % | pass |
| `part0.echo` | 21 | 6 | 5 | 83 % | pass |
| `part0.sensor` | 24 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:state` | 24 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 24 | 12 | 10 | 83 % | pass |
| `part1.log` | 24 | 26 | 25 | 96 % | pass |
| `part1.pulse` | 22 | 12 | 11 | 91 % | pass |
| `part2.filter` | 17 | 16 | 13 | 81 % | pass |
| `part2.filter#cs:stats` | 17 | 16 | 13 | 81 % | pass |
| `part2.sampler` | 16 | 14 | 10 | 71 % | pass |
| `part2.sampler#cs:stats` | 16 | 14 | 10 | 71 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::plain_request` | 2 | 2 |  |
| `p1_demo_part2::plain_request::{closure}` | 0 | 0 |  |
| `p1_demo_part2::plain_request::{closure}` | 2 | 1 | 74:16 false |
| `p1_demo_part2::plain_request::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 423:12 false |
| `p1_demo_part2::filter` | 4 | 4 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::inject::fault` | 0 | 0 |  |
| `p1_demo_part2::inject::window` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:22 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 728:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::validate_req` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 14 | 14 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::inject::fault` | 0 | 0 |  |
| `p1_demo_part1::inject::window` | 2 | 2 |  |
| `p1_demo_part1::inject::spin_ms` | 4 | 3 | 33:84 false |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::fresh` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>::advance` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::raw` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S0>::try_recv` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S1>::send_rep` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S2>::finish` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::Sink<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part1::rsk_roles::pingpong_sink::State::initial` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part1::rsk_roles::pingpong_sink::State$u20$as$u20$core::convert::From<p1_demo_part1::rsk_roles::pingpong_sink::Sink<p1_demo_part1::rsk_roles::pingpong_sink::S2>>>::from` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::inject::fault` | 0 | 0 |  |
| `p1_demo_part0::inject::window` | 2 | 2 |  |
| `p1_demo_part0::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part0::sensor` | 2 | 2 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 390:80 false |
| `p1_demo_part0::control` | 12 | 12 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::fresh` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>::advance` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::raw` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>::send_req` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>::send_stop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S1>::try_recv` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::State::is_empty` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<S>$u20$as$u20$core::ops::drop::Drop>::drop` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>::finish` | 0 | 0 |  |
| `p1_demo_part0::rsk_roles::pingpong_ctl::State::initial` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S0>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S1>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_roles::pingpong_ctl::State$u20$as$u20$core::convert::From<p1_demo_part0::rsk_roles::pingpong_ctl::Ctl<p1_demo_part0::rsk_roles::pingpong_ctl::S2>>>::from` | 0 | 0 |  |
| `_<p1_demo_part0::rsk_validate::telemetry_ctl$u20$as$u20$rsk::Validate<u32>>::accept` | 0 | 0 |  |
