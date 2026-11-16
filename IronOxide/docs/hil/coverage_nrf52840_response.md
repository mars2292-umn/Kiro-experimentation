# HIL coverage run (R34.4, R53.3) over the response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.5 s and reset |
| target run | pass | the report ran after 27.2 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 39 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 250 ms: filter's fault \| end_job at 250 ms: END_JOB as part2's fault response \| overrun at 412 ms: log's Budget overrun \| end_job at 412 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1313 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 55 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 1 (1 expected), pingpong_peer 1 (1 expected), pingpong_rejected 0 (0 expected), pingpong_reps 44 (about 45: one round per log Job but the restart's) |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 44 answered pulses before part1's restart (of 43), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 100 matched in all, latency 100 to 15000 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.104 | overrun | 0 | 0 | 0xfcac |
| 2 | 250.601 | memmanage | 5 | 2 | 0x0 |
| 3 | 250.601 | end_job | 5 | 2 | 0x2 |
| 4 | 412.551 | overrun | 3 | 1 | 0x5dffc |
| 5 | 412.551 | end_job | 3 | 1 | 0x6 |
| 6 | 501.507 | overrun | 5 | 2 | 0xcc18 |
| 7 | 501.507 | partition_restart | - | 2 | 0x6 |
| 8 | 501.507 | resource_released | 5 | 2 | 0x1 |
| 9 | 800.358 | memmanage | 3 | 1 | 0x0 |
| 10 | 800.358 | partition_restart | - | 1 | 0x2 |
| 11 | 1313.401 | overrun | 2 | 0 | 0x7a831 |
| 12 | 1325.014 | deadline_miss | 2 | 0 | 0x0 |
| 13 | 1325.014 | release_overlap | 2 | 0 | 0x0 |
| 14 | 1328.371 | deadline_miss | 4 | 1 | 0x0 |
| 15 | 1330.018 | endpoint_full | 0 | 0 | 0x0 |
| 16 | 1340.013 | endpoint_full | 0 | 0 | 0x0 |
| 17 | 1345.011 | deadline_miss | 2 | 0 | 0x0 |
| 18 | 1345.011 | mit_violation | 2 | 0 | 0x0 |
| 19 | 1350.018 | deadline_miss | 5 | 2 | 0x0 |
| 20 | 1350.018 | partition_stop | - | 2 | 0x7 |
| 21 | 1350.018 | endpoint_full | 0 | 0 | 0x0 |
| 22 | 1360.013 | deadline_miss | 3 | 1 | 0x0 |
| 23 | 1360.013 | mit_violation | 3 | 1 | 0x0 |
| 24 | 1360.013 | endpoint_full | 0 | 0 | 0x0 |
| 25 | 1365.016 | mit_violation | 2 | 0 | 0x0 |
| 26 | 1370.015 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1380.018 | endpoint_full | 0 | 0 | 0x0 |
| 28 | 1385.018 | mit_violation | 2 | 0 | 0x0 |
| 29 | 1390.017 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1400.012 | mit_violation | 3 | 1 | 0x0 |
| 31 | 1400.012 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1405.018 | mit_violation | 2 | 0 | 0x0 |
| 33 | 1410.018 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1420.011 | endpoint_full | 0 | 0 | 0x0 |
| 35 | 1425.013 | mit_violation | 2 | 0 | 0x0 |
| 36 | 1430.015 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1900.010 | memmanage | 0 | 0 | 0x0 |
| 38 | 1900.010 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 5232 / 81.8 | 3540 | 2^12:186 2^13:4 |
| 2 | 90 | 4280 / 66.9 | 3287 | 2^12:88 2^13:2 |
| 3 | 45 | 4045 / 63.2 | 3584 | 2^12:45 |
| 5 | 53 | 4280 / 66.9 | 3603 | 2^12:51 2^13:2 |
| 6 | 13 | 4045 / 63.2 | 3886 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.444 | 0 | 0 | 0 |
| 1 | 63 | 0.238 | 0 | 0 | 0 |
| 2 | 90 | 125.919 | 0 | 0 | 0 |
| 3 | 44 | 119.709 | 0 | 0 | 0 |
| 4 | 99 | 123.027 | 99 | 2244 | 2081 |
| 5 | 51 | 6.179 | 0 | 0 | 0 |
| 6 | 13 | 12.791 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 148424 / 2.319 (min 16452, mean 19500) | 1 above the Budget; 2^15:190 |
| Task 1 entry | 63 | 3802 / 0.059 (min 3542, mean 3668) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7750065 / 121.095 (min 335427, mean 421569) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 210031 / 3.282 (min 149876, mean 206716) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 99 | 7466 / 0.117 (min 3702, mean 4150) | 0 above the Budget; 2^12:88 2^13:11 |
| Task 5 entry | 51 | 12552 / 0.196 (min 8720, mean 9406) | 0 above the Budget; 2^14:51 |
| Task 6 entry | 13 | 12186 / 0.190 (min 8450, mean 10705) | 0 above the Budget; 2^14:13 |
| Task 0 on Resource 0 | 190 | 4102 / 0.064 | |
| Task 0 on Resource 1 | 190 | 508 / 0.008 | |
| Task 2 on Resource 0 | 90 | 325354 / 5.084 | |
| Task 2 on Resource 1 | 180 | 4050 / 0.063 | |
| Task 5 on Resource 2 | 51 | 4161 / 0.065 | |
| Task 6 on Resource 2 | 13 | 580 / 0.009 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 555 | 3252 / 50.81 | 3032 |
| timer | 521 | 4415 / 68.98 | 3020 |
| intake | 99 | 2349 / 36.70 | 2184 |
| lock | 715 | 473 / 7.39 | 471 |
| unlock | 714 | 1904 / 29.75 | 1703 |
| job_end | 550 | 2138 / 33.41 | 1813 |
| now | 52662 | 231 / 3.61 | 229 |
| raise | 63 | 2158 / 33.72 | 2057 |
| dma | 13 | 850 / 13.28 | 848 |
| fault | 3 | 2619 / 40.92 | 2140 |
| epilogue | 554 | 1228 / 19.19 | 623 |
| restart | 2 | 4415 / 68.98 | 3517 |
| stop | 1 | 4310 / 67.34 | 4310 |
| enforce | 4 | 4415 / 68.98 | 3522 |
| view | 701 | 308 / 4.81 | 274 |
| send | 280 | 2373 / 37.08 | 2223 |
| recv | 371 | 932 / 14.56 | 867 |

Entries that interrupted an executing Job: Task 0: 52, Task 2: 330, Task 3: 32, Task 4: 11, Task 5: 9, Task 6: 8

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3493 cycles
- Task 3 at 1840.000 ms, 3493 cycles
- Task 2 at 1845.000 ms, 3173 cycles
- Task 0 at 1850.000 ms, 3483 cycles
- Task 0 at 1860.000 ms, 3213 cycles
- Task 2 at 1865.000 ms, 3213 cycles
- Task 0 at 1870.000 ms, 3531 cycles
- Task 0 at 1880.000 ms, 3453 cycles
- Task 3 at 1880.000 ms, 3453 cycles
- Task 2 at 1885.000 ms, 3213 cycles
- Task 0 at 1890.000 ms, 3887 cycles
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
event=16
  task=255
  at_ms=501
  detail=6
  prs_lo=0
  prs_hi=0
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
  at_ms=1313
  detail=501809
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
     Finished in 9.15s
```

## Branch coverage of the Task entries under the suite (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 87 % (14 of 16 branch outcomes), part0.echo 83 % (5 of 6 branch outcomes), part0.sensor 83 % (10 of 12 branch outcomes), part1.log 95 % (23 of 24 branch outcomes), part1.pulse 91 % (11 of 12 branch outcomes), part2.filter 83 % (10 of 12 branch outcomes), part2.sampler 70 % (7 of 10 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 29 | 16 | 14 | 87 % | pass |
| `part0.control#cs:state` | 29 | 16 | 14 | 87 % | pass |
| `part0.control#cs:in_cs` | 29 | 16 | 14 | 87 % | pass |
| `part0.echo` | 21 | 6 | 5 | 83 % | pass |
| `part0.sensor` | 24 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:state` | 24 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 24 | 12 | 10 | 83 % | pass |
| `part1.log` | 24 | 24 | 23 | 95 % | pass |
| `part1.pulse` | 22 | 12 | 11 | 91 % | pass |
| `part2.filter` | 13 | 12 | 10 | 83 % | pass |
| `part2.filter#cs:stats` | 13 | 12 | 10 | 83 % | pass |
| `part2.sampler` | 13 | 10 | 7 | 70 % | pass |
| `part2.sampler#cs:stats` | 13 | 10 | 7 | 70 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 253:12 false |
| `p1_demo_part2::filter` | 4 | 4 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::inject::fault` | 0 | 0 |  |
| `p1_demo_part2::inject::window` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 2 | 1 | 518:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
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
| `p1_demo_part1::log` | 12 | 12 |  |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 385:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control` | 10 | 9 | 182:12 false |
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
