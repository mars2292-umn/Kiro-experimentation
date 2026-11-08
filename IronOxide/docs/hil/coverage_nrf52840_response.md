# HIL coverage run (R34.4, R53.3) over the response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| target run | pass | the report ran after 26.5 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 40 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 251 ms: filter's fault \| end_job at 251 ms: END_JOB as part2's fault response \| overrun at 413 ms: log's Budget overrun \| end_job at 413 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1314 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 54 (about 56 since the restart) |
| peer-state errors after part1's restart (R42.3, task 21.4) | pass | telemetry_peer 1 (1 expected), pingpong_peer 1 (1 expected), pingpong_rejected 0 (0 expected), pingpong_reps 44 (about 45: one round per log Job but the restart's) |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 44 answered pulses before part1's restart (of 43), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 99 matched in all, latency 90 to 6720 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.104 | overrun | 0 | 0 | 0xfce3 |
| 2 | 251.613 | memmanage | 5 | 2 | 0x0 |
| 3 | 251.613 | end_job | 5 | 2 | 0x2 |
| 4 | 413.542 | overrun | 3 | 1 | 0x5e077 |
| 5 | 413.542 | end_job | 3 | 1 | 0x6 |
| 6 | 501.512 | overrun | 5 | 2 | 0xcb23 |
| 7 | 501.512 | partition_restart | - | 2 | 0x6 |
| 8 | 501.512 | resource_released | 5 | 2 | 0x1 |
| 9 | 800.336 | memmanage | 3 | 1 | 0x0 |
| 10 | 800.336 | partition_restart | - | 1 | 0x2 |
| 11 | 1314.286 | overrun | 2 | 0 | 0x89b0a |
| 12 | 1325.010 | deadline_miss | 2 | 0 | 0x0 |
| 13 | 1325.010 | release_overlap | 2 | 0 | 0x0 |
| 14 | 1326.255 | deadline_miss | 4 | 1 | 0x0 |
| 15 | 1330.017 | endpoint_full | 0 | 0 | 0x0 |
| 16 | 1340.019 | endpoint_full | 0 | 0 | 0x0 |
| 17 | 1345.018 | deadline_miss | 2 | 0 | 0x0 |
| 18 | 1345.018 | mit_violation | 2 | 0 | 0x0 |
| 19 | 1350.011 | deadline_miss | 5 | 2 | 0x0 |
| 20 | 1350.011 | partition_stop | - | 2 | 0x7 |
| 21 | 1350.011 | endpoint_full | 0 | 0 | 0x0 |
| 22 | 1360.012 | deadline_miss | 3 | 1 | 0x0 |
| 23 | 1360.012 | mit_violation | 3 | 1 | 0x0 |
| 24 | 1360.012 | endpoint_full | 0 | 0 | 0x0 |
| 25 | 1365.012 | mit_violation | 2 | 0 | 0x0 |
| 26 | 1370.010 | endpoint_full | 0 | 0 | 0x0 |
| 27 | 1380.013 | endpoint_full | 0 | 0 | 0x0 |
| 28 | 1385.010 | mit_violation | 2 | 0 | 0x0 |
| 29 | 1390.010 | endpoint_full | 0 | 0 | 0x0 |
| 30 | 1400.019 | mit_violation | 3 | 1 | 0x0 |
| 31 | 1400.019 | endpoint_full | 0 | 0 | 0x0 |
| 32 | 1405.015 | mit_violation | 2 | 0 | 0x0 |
| 33 | 1410.013 | endpoint_full | 0 | 0 | 0x0 |
| 34 | 1420.010 | endpoint_full | 0 | 0 | 0x0 |
| 35 | 1425.009 | mit_violation | 2 | 0 | 0x0 |
| 36 | 1430.010 | endpoint_full | 0 | 0 | 0x0 |
| 37 | 1440.011 | release_overlap | 3 | 1 | 0x0 |
| 38 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 39 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 4794 / 74.9 | 3532 | 2^12:187 2^13:3 |
| 2 | 90 | 3966 / 62.0 | 3286 | 2^12:90 |
| 3 | 45 | 4041 / 63.1 | 3592 | 2^12:45 |
| 5 | 53 | 4147 / 64.8 | 3603 | 2^12:52 2^13:1 |
| 6 | 13 | 4041 / 63.1 | 3889 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.426 | 0 | 0 | 0 |
| 1 | 63 | 0.246 | 0 | 0 | 0 |
| 2 | 90 | 126.866 | 0 | 0 | 0 |
| 3 | 44 | 121.599 | 0 | 0 | 0 |
| 4 | 98 | 127.063 | 98 | 2207 | 2036 |
| 5 | 51 | 7.190 | 0 | 0 | 0 |
| 6 | 13 | 13.769 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 147296 / 2.301 (min 15116, mean 19416) | 1 above the Budget; 2^14:31 2^15:159 |
| Task 1 entry | 63 | 3418 / 0.053 (min 3201, mean 3306) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7840519 / 122.508 (min 398624, mean 485085) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 210489 / 3.289 (min 151969, mean 204124) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 98 | 6893 / 0.108 (min 3357, mean 3451) | 0 above the Budget; 2^12:97 2^13:1 |
| Task 5 entry | 51 | 12980 / 0.203 (min 8129, mean 8465) | 0 above the Budget; 2^13:5 2^14:46 |
| Task 6 entry | 13 | 11480 / 0.179 (min 7864, mean 9554) | 0 above the Budget; 2^13:7 2^14:6 |
| Task 0 on Resource 0 | 190 | 438 / 0.007 | |
| Task 0 on Resource 1 | 190 | 4138 / 0.065 | |
| Task 2 on Resource 0 | 90 | 391411 / 6.116 | |
| Task 2 on Resource 1 | 180 | 447 / 0.007 | |
| Task 5 on Resource 2 | 51 | 3961 / 0.062 | |
| Task 6 on Resource 2 | 13 | 4073 / 0.064 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 554 | 2829 / 44.20 | 2684 |
| timer | 589 | 4335 / 67.73 | 3024 |
| intake | 98 | 2331 / 36.42 | 2158 |
| lock | 715 | 465 / 7.27 | 463 |
| unlock | 714 | 1857 / 29.02 | 1643 |
| job_end | 549 | 2021 / 31.58 | 1787 |
| now | 63086 | 239 / 3.73 | 238 |
| raise | 63 | 2051 / 32.05 | 1975 |
| dma | 13 | 808 / 12.62 | 807 |
| fault | 3 | 2552 / 39.88 | 2039 |
| epilogue | 553 | 1216 / 19.00 | 615 |
| restart | 2 | 4335 / 67.73 | 3444 |
| stop | 1 | 4295 / 67.11 | 4295 |
| enforce | 4 | 4335 / 67.73 | 3462 |
| view | 703 | 308 / 4.81 | 274 |
| send | 280 | 2164 / 33.81 | 2004 |
| recv | 371 | 722 / 11.28 | 659 |

Entries that interrupted an executing Job: Task 0: 117, Task 2: 362, Task 3: 32, Task 4: 1, Task 5: 8, Task 6: 6

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3493 cycles
- Task 3 at 1840.000 ms, 3493 cycles
- Task 2 at 1845.000 ms, 3189 cycles
- Task 0 at 1850.000 ms, 3811 cycles
- Task 0 at 1860.000 ms, 3221 cycles
- Task 2 at 1865.000 ms, 3217 cycles
- Task 0 at 1870.000 ms, 3479 cycles
- Task 0 at 1880.000 ms, 3465 cycles
- Task 3 at 1880.000 ms, 3465 cycles
- Task 2 at 1885.000 ms, 3269 cycles
- Task 0 at 1890.000 ms, 3879 cycles
- Task 0 at 1900.000 ms, 3593 cycles

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
  detail=563978
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
     Finished in 8.45s
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
| `part2.filter` | 12 | 12 | 10 | 83 % | pass |
| `part2.filter#cs:stats` | 12 | 12 | 10 | 83 % | pass |
| `part2.sampler` | 12 | 10 | 7 | 70 % | pass |
| `part2.sampler#cs:stats` | 12 | 10 | 7 | 70 % | pass |

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
| `p1_demo_part2::sampler` | 2 | 1 | 407:8 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
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
| `p1_demo_part1::inject::spin_ms` | 4 | 3 | 30:84 false |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 310:80 false |
| `p1_demo_part0::control` | 10 | 9 | 181:12 false |
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
