# HIL coverage run (task 19.2, R34.4) over the stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 204 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8917 cycles (139.3 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.627 ms (104132 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.279 ms (17832 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.827 ms (436940 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11952 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2158 cycles (33.7 µs), mean 1952 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2026 % of the run |
| Kernel events | pass | 5414 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 115, overrun of Task 5: 91, overrun of Task 6: 2, operating_end of Task none: 1, resource_released of Task 5: 92, partition_restart of Task none: 91, safe_state of Task none: 1, end_job of Task 3: 115, end_job of Task 6: 2; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11951 response edges inside the run (target counted 11951) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.0 % of the run, the Kernel 41.0 % over 255026 entries; runs per Task [18052, 6006, 20997, 7685, 12034, 7420, 1146] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22947 cycles (0.359 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 18052 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 583 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 296 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 26 | unreachable: a hold of 250 µs or 1 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8917 / 139.3 | 3466 | 2^12:17851 2^13:141 2^14:7 |
| 2 | 8999 | 8265 / 129.1 | 3249 | 2^12:8869 2^13:128 2^14:2 |
| 3 | 4499 | 8308 / 129.8 | 3521 | 2^12:4461 2^13:37 2^14:1 |
| 5 | 7199 | 8917 / 139.3 | 3547 | 2^12:7085 2^13:109 2^14:5 |
| 6 | 1799 | 8588 / 134.2 | 3825 | 2^12:1780 2^13:18 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.627 | 0 | 0 | 0 |
| 1 | 5999 | 0.279 | 0 | 0 | 0 |
| 2 | 8999 | 6.827 | 0 | 0 | 0 |
| 3 | 3375 | 4.582 | 0 | 0 | 0 |
| 4 | 11951 | 6.883 | 11952 | 2158 | 1952 |
| 5 | 6303 | 7.470 | 0 | 0 | 0 |
| 6 | 886 | 14.174 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22947 / 0.359 (min 8896, mean 12118) | 0 above the Budget; 2^14:17181 2^15:819 |
| Task 1 entry | 5999 | 9416 / 0.147 (min 3073, mean 3175) | 0 above the Budget; 2^12:5983 2^13:2 2^14:14 |
| Task 2 entry | 8999 | 408207 / 6.378 (min 398303, mean 400149) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204614 / 3.197 (min 190754, mean 195753) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11951 | 12118 / 0.189 (min 3199, mean 3345) | 0 above the Budget; 2^12:11721 2^13:167 2^14:63 |
| Task 5 entry | 6303 | 37578 / 0.587 (min 20098, mean 26885) | 0 above the Budget; 2^15:6303 |
| Task 6 entry | 886 | 26635 / 0.416 (min 17695, mean 25859) | 0 above the Budget; 2^15:886 |
| Task 0 on Resource 0 | 18000 | 9903 / 0.155 | |
| Task 0 on Resource 1 | 18000 | 9778 / 0.153 | |
| Task 2 on Resource 0 | 8999 | 391639 / 6.119 | |
| Task 2 on Resource 1 | 17998 | 6515 / 0.102 | |
| Task 5 on Resource 2 | 12606 | 25243 / 0.394 | |
| Task 6 on Resource 2 | 886 | 381 / 0.006 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 57766 | 2795 / 43.67 | 2611 |
| timer | 64067 | 4738 / 74.03 | 2970 |
| intake | 11952 | 2223 / 34.73 | 2016 |
| lock | 77392 | 494 / 7.72 | 492 |
| unlock | 76489 | 1816 / 28.38 | 1640 |
| job_end | 55513 | 2036 / 31.81 | 1792 |
| now | 7098230 | 237 / 3.70 | 237 |
| raise | 6000 | 2055 / 32.11 | 1977 |
| dma | 9039 | 823 / 12.86 | 573 |
| epilogue | 57766 | 1205 / 18.83 | 607 |
| restart | 903 | 4738 / 74.03 | 4588 |
| enforce | 2253 | 4738 / 74.03 | 3731 |
| view | 73272 | 310 / 4.84 | 280 |

Entries that interrupted an executing Job: Task 0: 11381, Task 1: 30, Task 2: 31919, Task 3: 9024, Task 4: 313, Task 5: 3461, Task 6: 1259

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3401 cycles
- Task 0 at 179940.000 ms, 3124 cycles
- Task 2 at 179945.000 ms, 3128 cycles
- Task 0 at 179950.000 ms, 3857 cycles
- Task 5 at 179950.000 ms, 3857 cycles
- Task 0 at 179960.000 ms, 3368 cycles
- Task 3 at 179960.000 ms, 3368 cycles
- Task 2 at 179965.000 ms, 3553 cycles
- Task 0 at 179970.000 ms, 3649 cycles
- Task 5 at 179975.000 ms, 3132 cycles
- Task 0 at 179980.000 ms, 3124 cycles
- Task 0 at 179990.000 ms, 3044 cycles

## The Kernel's events

5414 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 115, overrun of Task 5: 91, overrun of Task 6: 2, operating_end of Task none: 1, resource_released of Task 5: 92, partition_restart of Task none: 91, safe_state of Task none: 1, end_job of Task 3: 115, end_job of Task 6: 2; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 321889 | 18052 | 0.350 |
| 1 | 23675 | 6006 | 0.140 |
| 2 | 5673936 | 20997 | 6.270 |
| 3 | 1720881 | 7685 | 4.840 |
| 4 | 49275 | 12034 | 0.140 |
| 5 | 334158 | 7420 | 0.920 |
| 6 | 46870 | 1146 | 0.530 |
| none | 12256068 | 42261 | 16374.990 |

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
pulse=11951
echo=5999
blocked_releases=8999
filter=3
filter_state_bits=1107560448
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=583
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=296
loop:part2:filter:hold=26
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
event=11
  task=5
  at_ms=178901
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179013
  detail=384671
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179013
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179101
  detail=51906
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179101
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179101
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179172
  detail=384782
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179172
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179301
  detail=52002
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179301
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179301
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179333
  detail=384989
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179333
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179492
  detail=385010
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179492
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179501
  detail=51842
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179501
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179501
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179653
  detail=384513
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179653
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179701
  detail=51906
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179701
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179701
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179813
  detail=384830
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179813
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179901
  detail=51906
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179901
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179901
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179972
  detail=385010
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179972
  detail=6
  prs_lo=0
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

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 90 % (20 of 22 branch outcomes), part1.pulse 100 % (10 of 10 branch outcomes), part2.filter 79 % (19 of 24 branch outcomes), part2.sampler 68 % (15 of 22 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 5 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 5 | 4 | 4 | 100 % | pass |
| `part0.echo` | 3 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 7 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 7 | 6 | 5 | 83 % | pass |
| `part1.log` | 9 | 22 | 20 | 90 % | pass |
| `part1.pulse` | 7 | 10 | 10 | 100 % | pass |
| `part2.filter` | 16 | 24 | 19 | 79 % | pass |
| `part2.filter#cs:stats` | 16 | 24 | 19 | 79 % | pass |
| `part2.sampler` | 15 | 22 | 15 | 68 % | pass |
| `part2.sampler#cs:stats` | 15 | 22 | 15 | 68 % | pass |

| Function | Branch outcomes | Covered | Uncovered (line:col outcome) |
|---|---|---|---|
| `p1_demo_part2::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part2::traffic_step` | 10 | 6 | 115:8 true, 117:12 true, 117:12 false, 127:12 true |
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
| `p1_demo_part2::inject::spin_ms` | 4 | 4 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 4 | 4 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 2 | 0 | 361:34 true, 361:34 false |
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
| `p1_demo_part1::inject::spin_ms` | 4 | 4 |  |
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
