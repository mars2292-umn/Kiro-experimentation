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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8337 cycles (130.3 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.619 ms (103624 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.275 ms (17620 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.819 ms (436436 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11952 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2141 cycles (33.5 µs), mean 1950 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2023 % of the run |
| Kernel events | pass | 6754 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11951 response edges inside the run (target counted 11951) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.2 % of the run, the Kernel 40.6 % over 252365 entries; runs per Task [18047, 6000, 20998, 5509, 12043, 7424, 2153] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22619 cycles (0.353 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 18047 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 583 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 296 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8337 / 130.3 | 3463 | 2^12:17868 2^13:128 2^14:3 |
| 2 | 8999 | 7284 / 113.8 | 3211 | 2^12:8876 2^13:123 |
| 3 | 4499 | 8032 / 125.5 | 3530 | 2^12:4450 2^13:49 |
| 5 | 7199 | 8032 / 125.5 | 3532 | 2^12:7141 2^13:58 |
| 6 | 1799 | 8032 / 125.5 | 3830 | 2^12:1782 2^13:17 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.619 | 0 | 0 | 0 |
| 1 | 5999 | 0.275 | 0 | 0 | 0 |
| 2 | 8999 | 6.819 | 0 | 0 | 0 |
| 3 | 3375 | 4.338 | 0 | 0 | 0 |
| 4 | 11951 | 6.863 | 11952 | 2141 | 1950 |
| 5 | 6300 | 7.440 | 0 | 0 | 0 |
| 6 | 900 | 12.278 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22619 / 0.353 (min 8896, mean 12104) | 0 above the Budget; 2^14:17229 2^15:771 |
| Task 1 entry | 5999 | 9416 / 0.147 (min 3073, mean 3175) | 0 above the Budget; 2^12:5982 2^13:1 2^14:16 |
| Task 2 entry | 8999 | 408207 / 6.378 (min 398327, mean 400175) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204614 / 3.197 (min 190754, mean 195813) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11951 | 12042 / 0.188 (min 3199, mean 3359) | 0 above the Budget; 2^12:11668 2^13:207 2^14:76 |
| Task 5 entry | 6300 | 37578 / 0.587 (min 20098, mean 26900) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 39659 / 0.620 (min 17695, mean 27501) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9784 / 0.153 | |
| Task 0 on Resource 1 | 18000 | 9818 / 0.153 | |
| Task 2 on Resource 0 | 8999 | 391639 / 6.119 | |
| Task 2 on Resource 1 | 17998 | 6404 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 27103 / 0.423 | |
| Task 6 on Resource 2 | 900 | 8046 / 0.126 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58449 | 2787 / 43.55 | 2606 |
| timer | 64503 | 4698 / 73.41 | 2965 |
| intake | 11952 | 2206 / 34.47 | 2014 |
| lock | 77397 | 494 / 7.72 | 492 |
| unlock | 76497 | 1806 / 28.22 | 1638 |
| job_end | 55524 | 2036 / 31.81 | 1788 |
| now | 6950901 | 237 / 3.70 | 237 |
| raise | 6000 | 2055 / 32.11 | 1975 |
| dma | 8999 | 825 / 12.89 | 574 |
| epilogue | 58449 | 1205 / 18.83 | 579 |
| restart | 900 | 4698 / 73.41 | 4555 |
| enforce | 2925 | 4698 / 73.41 | 3586 |
| view | 72122 | 310 / 4.84 | 279 |

Entries that interrupted an executing Job: Task 0: 11359, Task 1: 33, Task 2: 31084, Task 3: 5461, Task 4: 383, Task 5: 3489, Task 6: 3250

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3401 cycles
- Task 0 at 179940.000 ms, 3124 cycles
- Task 2 at 179945.000 ms, 3128 cycles
- Task 0 at 179950.000 ms, 3857 cycles
- Task 5 at 179950.000 ms, 3857 cycles
- Task 0 at 179960.000 ms, 3368 cycles
- Task 3 at 179960.000 ms, 3368 cycles
- Task 2 at 179965.000 ms, 3128 cycles
- Task 0 at 179970.000 ms, 3457 cycles
- Task 5 at 179975.000 ms, 3132 cycles
- Task 0 at 179980.000 ms, 3124 cycles
- Task 0 at 179990.000 ms, 3044 cycles

## The Kernel's events

6754 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 321464 | 18047 | 0.340 |
| 1 | 23652 | 6000 | 0.140 |
| 2 | 5674229 | 20998 | 6.270 |
| 3 | 1485773 | 5509 | 4.090 |
| 4 | 49514 | 12043 | 0.140 |
| 5 | 334487 | 7424 | 0.890 |
| 6 | 113140 | 2153 | 0.880 |
| none | 12433709 | 44777 | 16479.100 |

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
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=583
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=296
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
  detail=51714
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
  detail=256986
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
  detail=51806
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
  detail=256514
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
  detail=52174
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
  detail=51806
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
  detail=256514
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
  detail=51714
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
  detail=51746
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
  detail=256514
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
  detail=51714
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
  detail=256901
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
  detail=51806
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
     Finished in 7.72s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 90 % (20 of 22 branch outcomes), part1.pulse 100 % (10 of 10 branch outcomes), part2.filter 70 % (17 of 24 branch outcomes), part2.sampler 63 % (14 of 22 branch outcomes); 0 below

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
| `part2.filter` | 16 | 24 | 17 | 70 % | pass |
| `part2.filter#cs:stats` | 16 | 24 | 17 | 70 % | pass |
| `part2.sampler` | 15 | 22 | 14 | 63 % | pass |
| `part2.sampler#cs:stats` | 15 | 22 | 14 | 63 % | pass |

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
| `p1_demo_part2::filter::{closure}` | 6 | 5 | 325:67 false |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 4 |  |
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
