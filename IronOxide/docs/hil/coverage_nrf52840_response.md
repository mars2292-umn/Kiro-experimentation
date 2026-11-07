# HIL coverage run (R34.4, R53.3) over the response suite: fault injection for every response of R14.5 (task 7.5, R14.11)

Verdict: **pass**

Target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| target run | pass | the report ran after 23.7 s (the safe state of 1.9 s, by design) |
| responses (R14.5, R14.11) | pass | 29 events; overrun at 201 ms: control's Budget overrun, RECORD_AND_CONTINUE \| memmanage at 251 ms: filter's fault \| end_job at 251 ms: END_JOB as part2's fault response \| overrun at 413 ms: log's Budget overrun \| end_job at 413 ms: END_JOB as part1's Overrun_Response \| overrun at 501 ms: filter's Budget overrun inside its Critical_Section \| partition_restart at 501 ms: END_JOB escalated to RESTART_PARTITION (R18.4) \| resource_released at 501 ms: the Resource released by force (R8.9) \| memmanage at 800 ms: log's fault \| partition_restart at 800 ms: RESTART_PARTITION as part1's fault response \| deadline_miss at 1325 ms: sensor's own deadline miss (R9.6; KD-R9.6 fixed), RECORD_ONLY \| deadline_miss at 1345 ms: the deadline miss of sensor's held Release (R9.7), RECORD_ONLY \| deadline_miss at 1350 ms: filter's deadline missed while pending (its Release of 1,325 ms) \| partition_stop at 1350 ms: STOP_PARTITION as part2's deadline-miss response \| memmanage at 1900 ms: control's fault \| safe_state at 1900 ms: SAFE_STATE as part0's fault response \| overrun at 1314 ms: sensor's Budget overrun, RECORD_AND_CONTINUE |
| part0 unchanged by the other Partitions' responses (R14.9) | pass | control 190 (189 or 190 expected), echo 63 (about 63), sensor 90 (90: 95 Releases less the five its 120 ms Job held and discarded) at the safe state of 1.9 s |
| part1 restarted at 800 ms: state re-initialized, Releases resumed (R14.5) | pass | log 25 (about 25 since the restart; 47 had it kept its state), pulse 54 (about 56 since the restart) |
| part2 restarted at 500 ms and stopped at 1,325 ms (R14.5) | pass | filter 32 (about 32 between the restart and the stop; 76 had it run to the end), sampler 8 (about 8) |
| pulse answered throughout (the source restarted with its Partition; R14.9 after part2's stop) | pass | 44 answered pulses before part1's restart (of 43), 24 between the restart and sensor's 120 ms Job (of 23), 24 after part2's stop (of 23); 99 matched in all, latency 90 to 8720 µs |

## The Kernel's events

| Seq | Instant (ms) | Kind | Task | Partition | Detail |
|---|---|---|---|---|---|
| 0 | 0.000 | debugger | - | - | 0x1 |
| 1 | 201.108 | overrun | 0 | 0 | 0xfdca |
| 2 | 251.546 | memmanage | 5 | 2 | 0x0 |
| 3 | 251.546 | end_job | 5 | 2 | 0x2 |
| 4 | 413.411 | overrun | 3 | 1 | 0x5e0ca |
| 5 | 413.411 | end_job | 3 | 1 | 0x6 |
| 6 | 501.326 | overrun | 5 | 2 | 0xcc69 |
| 7 | 501.326 | partition_restart | - | 2 | 0x6 |
| 8 | 501.326 | resource_released | 5 | 2 | 0x1 |
| 9 | 800.277 | memmanage | 3 | 1 | 0x0 |
| 10 | 800.277 | partition_restart | - | 1 | 0x2 |
| 11 | 1314.285 | overrun | 2 | 0 | 0x89c1b |
| 12 | 1321.182 | deadline_miss | 4 | 1 | 0x0 |
| 13 | 1325.014 | deadline_miss | 2 | 0 | 0x0 |
| 14 | 1325.014 | release_overlap | 2 | 0 | 0x0 |
| 15 | 1345.012 | deadline_miss | 2 | 0 | 0x0 |
| 16 | 1345.012 | mit_violation | 2 | 0 | 0x0 |
| 17 | 1350.015 | deadline_miss | 5 | 2 | 0x0 |
| 18 | 1350.015 | partition_stop | - | 2 | 0x7 |
| 19 | 1360.015 | deadline_miss | 3 | 1 | 0x0 |
| 20 | 1360.015 | mit_violation | 3 | 1 | 0x0 |
| 21 | 1365.018 | mit_violation | 2 | 0 | 0x0 |
| 22 | 1385.010 | mit_violation | 2 | 0 | 0x0 |
| 23 | 1400.013 | mit_violation | 3 | 1 | 0x0 |
| 24 | 1405.018 | mit_violation | 2 | 0 | 0x0 |
| 25 | 1425.010 | mit_violation | 2 | 0 | 0x0 |
| 26 | 1440.018 | release_overlap | 3 | 1 | 0x0 |
| 27 | 1900.009 | memmanage | 0 | 0 | 0x0 |
| 28 | 1900.009 | safe_state | - | - | 0x0 |

## The Kernel's record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 190 | 5031 / 78.6 | 3541 | 2^12:185 2^13:5 |
| 2 | 90 | 4169 / 65.1 | 3254 | 2^12:89 2^13:1 |
| 3 | 45 | 4333 / 67.7 | 3580 | 2^12:44 2^13:1 |
| 5 | 53 | 4169 / 65.1 | 3604 | 2^12:51 2^13:2 |
| 6 | 13 | 4072 / 63.6 | 3877 | 2^12:13 |

Releases recorded: 391; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 190 | 2.354 | 0 | 0 | 0 |
| 1 | 63 | 0.244 | 0 | 0 | 0 |
| 2 | 90 | 126.796 | 0 | 0 | 0 |
| 3 | 44 | 121.387 | 0 | 0 | 0 |
| 4 | 98 | 132.065 | 98 | 2126 | 1979 |
| 5 | 51 | 7.087 | 0 | 0 | 0 |
| 6 | 13 | 13.637 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 190 | 142772 / 2.231 (min 10996, mean 15002) | 1 above the Budget; 2^14:144 2^15:46 |
| Task 1 entry | 63 | 3406 / 0.053 (min 3190, mean 3291) | 0 above the Budget; 2^12:63 |
| Task 2 entry | 90 | 7886433 / 123.226 (min 398629, mean 485840) | 1 above the Budget; 2^15:90 |
| Task 3 entry | 44 | 201746 / 3.152 (min 158424, mean 196411) | 0 above the Budget; 2^15:44 |
| Task 4 entry | 98 | 6899 / 0.108 (min 3351, mean 3439) | 0 above the Budget; 2^12:97 2^13:1 |
| Task 5 entry | 51 | 11679 / 0.182 (min 8003, mean 8247) | 0 above the Budget; 2^13:23 2^14:28 |
| Task 6 entry | 13 | 11482 / 0.179 (min 7786, mean 10295) | 0 above the Budget; 2^13:4 2^14:9 |
| Task 0 on Resource 0 | 190 | 4022 / 0.063 | |
| Task 0 on Resource 1 | 190 | 4058 / 0.063 | |
| Task 2 on Resource 0 | 90 | 389172 / 6.081 | |
| Task 2 on Resource 1 | 180 | 417 / 0.007 | |
| Task 5 on Resource 2 | 51 | 3913 / 0.061 | |
| Task 6 on Resource 2 | 13 | 4009 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 554 | 2845 / 44.45 | 2691 |
| timer | 592 | 4335 / 67.73 | 3004 |
| intake | 98 | 2240 / 35.00 | 2094 |
| lock | 715 | 481 / 7.52 | 480 |
| unlock | 714 | 1806 / 28.22 | 1617 |
| job_end | 549 | 2049 / 32.02 | 1780 |
| now | 65385 | 237 / 3.70 | 236 |
| raise | 63 | 2061 / 32.20 | 1957 |
| dma | 13 | 821 / 12.83 | 820 |
| fault | 3 | 2528 / 39.50 | 2035 |
| epilogue | 553 | 1203 / 18.80 | 600 |
| restart | 2 | 4335 / 67.73 | 3432 |
| stop | 1 | 4299 / 67.17 | 4299 |
| enforce | 4 | 4335 / 67.73 | 3451 |
| view | 698 | 309 / 4.83 | 275 |

Entries that interrupted an executing Job: Task 0: 119, Task 2: 364, Task 3: 31, Task 4: 1, Task 5: 3, Task 6: 9

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 1840.000 ms, 3464 cycles
- Task 3 at 1840.000 ms, 3464 cycles
- Task 2 at 1845.000 ms, 3212 cycles
- Task 0 at 1850.000 ms, 3983 cycles
- Task 0 at 1860.000 ms, 3192 cycles
- Task 2 at 1865.000 ms, 3188 cycles
- Task 0 at 1870.000 ms, 3423 cycles
- Task 0 at 1880.000 ms, 3488 cycles
- Task 3 at 1880.000 ms, 3488 cycles
- Task 2 at 1885.000 ms, 3188 cycles
- Task 0 at 1890.000 ms, 3423 cycles
- Task 0 at 1900.000 ms, 3628 cycles

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
first_memmanage_cfsr=2147483778
first_memmanage_mmfar=536870912
events=29
event=23
  task=255
  at_ms=0
  detail=1
  prs_lo=0
  prs_hi=0
event=6
  task=0
  at_ms=201
  detail=64970
  prs_lo=134217728
  prs_hi=0
event=2
  task=5
  at_ms=251
  detail=0
  prs_lo=541065216
  prs_hi=0
event=19
  task=5
  at_ms=251
  detail=2
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=413
  detail=385226
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=413
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=501
  detail=52329
  prs_lo=134217728
  prs_hi=0
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
  at_ms=1314
  detail=564251
  prs_lo=134217728
  prs_hi=0
event=7
  task=4
  at_ms=1321
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
event=9
  task=2
  at_ms=1365
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1385
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=3
  at_ms=1400
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1405
  detail=0
  prs_lo=4096
  prs_hi=0
event=9
  task=2
  at_ms=1425
  detail=0
  prs_lo=4096
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
     Finished in 7.73s
```

## Branch coverage of the Task entries under the suite (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 92 % (13 of 14 branch outcomes), part0.echo 83 % (5 of 6 branch outcomes), part0.sensor 83 % (10 of 12 branch outcomes), part1.log 95 % (21 of 22 branch outcomes), part1.pulse 91 % (11 of 12 branch outcomes), part2.filter 83 % (10 of 12 branch outcomes), part2.sampler 70 % (7 of 10 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 8 | 14 | 13 | 92 % | pass |
| `part0.control#cs:state` | 8 | 14 | 13 | 92 % | pass |
| `part0.control#cs:in_cs` | 8 | 14 | 13 | 92 % | pass |
| `part0.echo` | 6 | 6 | 5 | 83 % | pass |
| `part0.sensor` | 9 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:state` | 9 | 12 | 10 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 9 | 12 | 10 | 83 % | pass |
| `part1.log` | 9 | 22 | 21 | 95 % | pass |
| `part1.pulse` | 9 | 12 | 11 | 91 % | pass |
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
| `p1_demo_part1::log` | 10 | 10 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::inject::fault` | 0 | 0 |  |
| `p1_demo_part1::inject::window` | 2 | 2 |  |
| `p1_demo_part1::inject::spin_ms` | 4 | 3 | 30:84 false |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::inject::fault` | 0 | 0 |  |
| `p1_demo_part0::inject::window` | 2 | 2 |  |
| `p1_demo_part0::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part0::sensor` | 2 | 2 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control` | 8 | 8 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
