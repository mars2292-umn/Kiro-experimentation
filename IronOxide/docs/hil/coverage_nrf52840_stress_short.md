# HIL coverage run (task 19.2, R34.4) over the stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 204 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8054 cycles (125.8 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.610 ms (103032 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.274 ms (17528 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.806 ms (435600 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11954 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2102 cycles (32.8 µs), mean 1891 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1962 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11953 response edges inside the run (target counted 11953) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.1 % of the run, the Kernel 40.4 % over 213638 entries; runs per Task [18053, 6002, 20997, 5474, 12050, 7366, 2177] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22512 cycles (0.352 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 18053 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 586 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 298 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8017 / 125.3 | 3370 | 2^12:17903 2^13:96 |
| 2 | 8999 | 8054 / 125.8 | 3133 | 2^12:8872 2^13:127 |
| 3 | 4499 | 4874 / 76.2 | 3423 | 2^12:4498 2^13:1 |
| 5 | 7199 | 7906 / 123.5 | 3436 | 2^12:7160 2^13:39 |
| 6 | 1799 | 7574 / 118.3 | 3756 | 2^12:1781 2^13:18 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.610 | 0 | 0 | 0 |
| 1 | 5999 | 0.274 | 0 | 0 | 0 |
| 2 | 8999 | 6.806 | 0 | 0 | 0 |
| 3 | 3375 | 4.340 | 0 | 0 | 0 |
| 4 | 11953 | 6.864 | 11954 | 2102 | 1891 |
| 5 | 6300 | 7.423 | 0 | 0 | 0 |
| 6 | 900 | 12.172 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22512 / 0.352 (min 8862, mean 12058) | 0 above the Budget; 2^14:17800 2^15:200 |
| Task 1 entry | 5999 | 9403 / 0.147 (min 3097, mean 3210) | 0 above the Budget; 2^12:5978 2^13:1 2^14:20 |
| Task 2 entry | 8999 | 408233 / 6.379 (min 398157, mean 399778) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204666 / 3.198 (min 190610, mean 195738) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11953 | 11889 / 0.186 (min 3225, mean 3369) | 0 above the Budget; 2^12:11703 2^13:164 2^14:86 |
| Task 5 entry | 6300 | 35726 / 0.558 (min 20314, mean 26710) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 36711 / 0.574 (min 17537, mean 27522) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9880 / 0.154 | |
| Task 0 on Resource 1 | 18000 | 9866 / 0.154 | |
| Task 2 on Resource 0 | 8999 | 391004 / 6.109 | |
| Task 2 on Resource 1 | 17998 | 6299 / 0.098 | |
| Task 5 on Resource 2 | 12600 | 26885 / 0.420 | |
| Task 6 on Resource 2 | 900 | 381 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58451 | 2827 / 44.17 | 2632 |
| timer | 64562 | 4613 / 72.08 | 2894 |
| intake | 11954 | 2217 / 34.64 | 1999 |
| lock | 77397 | 483 / 7.55 | 481 |
| unlock | 76497 | 1768 / 27.62 | 1618 |
| job_end | 55526 | 1982 / 30.97 | 1754 |
| now | 6991042 | 237 / 3.70 | 237 |
| raise | 6000 | 2023 / 31.61 | 1953 |
| dma | 8999 | 816 / 12.75 | 569 |
| epilogue | 58451 | 1206 / 18.84 | 584 |
| restart | 900 | 4613 / 72.08 | 4483 |
| enforce | 2925 | 4613 / 72.08 | 3512 |
| view | 72057 | 310 / 4.84 | 279 |

Entries that interrupted an executing Job: Task 0: 11426, Task 1: 41, Task 2: 31022, Task 3: 5299, Task 4: 348, Task 5: 3310, Task 6: 3333

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3313 cycles
- Task 0 at 179940.000 ms, 3046 cycles
- Task 2 at 179945.000 ms, 3046 cycles
- Task 0 at 179950.000 ms, 3637 cycles
- Task 5 at 179950.000 ms, 3637 cycles
- Task 0 at 179960.000 ms, 3286 cycles
- Task 3 at 179960.000 ms, 3286 cycles
- Task 2 at 179965.000 ms, 3046 cycles
- Task 0 at 179970.000 ms, 3457 cycles
- Task 5 at 179975.000 ms, 3046 cycles
- Task 0 at 179980.000 ms, 3046 cycles
- Task 0 at 179990.000 ms, 2970 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 318440 | 18053 | 0.340 |
| 1 | 23388 | 6002 | 0.140 |
| 2 | 5668527 | 20997 | 6.260 |
| 3 | 1484867 | 5474 | 4.040 |
| 4 | 48554 | 12050 | 0.130 |
| 5 | 332081 | 7366 | 0.890 |
| 6 | 113175 | 2177 | 0.880 |
| none | 12440792 | 44846 | 16409.360 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/coverage/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11953
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=586
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=298
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=0
dma_after_stop=0
spim0_enable=7
spim0_shorts=0
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=179301
  detail=51923
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
  at_ms=179324
  detail=256508
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=52103
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
  at_ms=179484
  detail=256767
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=51847
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
  detail=52103
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
  at_ms=179644
  detail=256775
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=51915
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
  detail=52035
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
  at_ms=179804
  detail=256740
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179804
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=51923
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
  at_ms=179964
  detail=256767
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=52103
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
     Finished in 7.73s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 86 % (19 of 22 branch outcomes), part1.pulse 90 % (9 of 10 branch outcomes), part2.filter 70 % (17 of 24 branch outcomes), part2.sampler 59 % (13 of 22 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 5 | 4 | 4 | 100 % | pass |
| `part0.echo` | 3 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 7 | 6 | 5 | 83 % | pass |
| `part1.log` | 9 | 22 | 19 | 86 % | pass |
| `part1.pulse` | 7 | 10 | 9 | 90 % | pass |
| `part2.filter` | 16 | 24 | 17 | 70 % | pass |
| `part2.filter#cs:stats` | 16 | 24 | 17 | 70 % | pass |
| `part2.sampler` | 15 | 22 | 13 | 59 % | pass |
| `part2.sampler#cs:stats` | 15 | 22 | 13 | 59 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::traffic_step` | 10 | 5 | 115:8 true, 117:12 true, 117:12 false, 127:12 true, 147:5 false |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::traffic_step::{closure}` | 0 | 0 |  |
| `p1_demo_part2::Stats::record` | 2 | 1 | 253:12 false |
| `p1_demo_part2::filter` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 6 | 6 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 4 | 4 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 2 | 0 | 372:34 true, 372:34 false |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 8 | 8 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::log::{closure}` | 2 | 0 | 220:34 true, 220:34 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::inject::spin_ms` | 4 | 3 | 30:84 false |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 237:80 false |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part0::control` | 4 | 4 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::control::{closure}` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::echo` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::sensor` | 0 | 0 |  |
| `p1_demo_part0::rsk_jobs::control` | 0 | 0 |  |
