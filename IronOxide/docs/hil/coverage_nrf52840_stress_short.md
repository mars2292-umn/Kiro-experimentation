# HIL coverage run (task 19.2, R34.4) over the stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9064 cycles (141.6 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.665 ms (106568 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.291 ms (18632 cycles) against WCRT 11.364 ms (727300 cycles); part0.sensor: 8999 Jobs, response at most 6.879 ms (440268 cycles) against WCRT 11.512 ms (736800 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11949 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2144 cycles (33.5 µs), mean 1951 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2023 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 22969 cycles (358.9 µs) over 5930729 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11948 response edges inside the run (target counted 11948) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.5 % of the run, the Kernel 44.2 % over 1545679 entries; runs per Task [18051, 5999, 20998, 5852, 12048, 7400, 1858] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24181 cycles (0.378 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 18051 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 472 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 240 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9064 / 141.6 | 3510 | 2^12:17578 2^13:398 2^14:23 |
| 2 | 8999 | 8550 / 133.6 | 3270 | 2^12:8841 2^13:153 2^14:5 |
| 3 | 4499 | 8570 / 133.9 | 3601 | 2^12:4390 2^13:108 2^14:1 |
| 5 | 7199 | 9064 / 141.6 | 3578 | 2^12:6866 2^13:324 2^14:9 |
| 6 | 1799 | 7450 / 116.4 | 3877 | 2^12:1769 2^13:30 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.665 | 0 | 0 | 0 |
| 1 | 5999 | 0.291 | 0 | 0 | 0 |
| 2 | 8999 | 6.879 | 0 | 0 | 0 |
| 3 | 3375 | 4.254 | 0 | 0 | 0 |
| 4 | 11948 | 6.931 | 11949 | 2144 | 1951 |
| 5 | 6300 | 7.502 | 0 | 0 | 0 |
| 6 | 900 | 12.762 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 24181 / 0.378 (min 9504, mean 12840) | 0 above the Budget; 2^14:16996 2^15:1004 |
| Task 1 entry | 5999 | 9930 / 0.155 (min 3233, mean 3341) | 0 above the Budget; 2^12:5979 2^13:1 2^14:19 |
| Task 2 entry | 8999 | 410553 / 6.415 (min 399516, mean 401371) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206020 / 3.219 (min 190808, mean 196473) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11948 | 13401 / 0.209 (min 3365, mean 3519) | 0 above the Budget; 2^12:11705 2^13:160 2^14:83 |
| Task 5 entry | 6300 | 38556 / 0.602 (min 20912, mean 27198) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 36863 / 0.576 (min 19536, mean 27846) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10542 / 0.165 | |
| Task 0 on Resource 1 | 18000 | 10780 / 0.168 | |
| Task 2 on Resource 0 | 8999 | 391889 / 6.123 | |
| Task 2 on Resource 1 | 17998 | 6981 / 0.109 | |
| Task 5 on Resource 2 | 12600 | 28777 / 0.450 | |
| Task 6 on Resource 2 | 900 | 4075 / 0.064 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58446 | 2851 / 44.55 | 2685 |
| timer | 64756 | 4737 / 74.02 | 2997 |
| intake | 11949 | 2269 / 35.45 | 2062 |
| lock | 77397 | 472 / 7.38 | 470 |
| unlock | 76497 | 1756 / 27.44 | 1578 |
| job_end | 55521 | 1988 / 31.06 | 1730 |
| now | 5668238 | 233 / 3.64 | 233 |
| raise | 6000 | 1996 / 31.19 | 1911 |
| dma | 8999 | 807 / 12.61 | 567 |
| epilogue | 58446 | 1219 / 19.05 | 591 |
| restart | 900 | 4737 / 74.02 | 4576 |
| enforce | 2925 | 4737 / 74.02 | 3624 |
| view | 72149 | 308 / 4.81 | 276 |

Entries that interrupted an executing Job: Task 0: 11316, Task 1: 39, Task 2: 31233, Task 3: 6327, Task 4: 346, Task 5: 3443, Task 6: 2669

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3892 cycles
- Task 0 at 179940.000 ms, 3182 cycles
- Task 2 at 179945.000 ms, 3182 cycles
- Task 0 at 179950.000 ms, 3608 cycles
- Task 5 at 179950.000 ms, 3608 cycles
- Task 0 at 179960.000 ms, 3422 cycles
- Task 3 at 179960.000 ms, 3422 cycles
- Task 2 at 179965.000 ms, 3182 cycles
- Task 0 at 179970.000 ms, 3944 cycles
- Task 5 at 179975.000 ms, 3182 cycles
- Task 0 at 179980.000 ms, 3182 cycles
- Task 0 at 179990.000 ms, 3102 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5930729 of them, the longest 22969 cycles (358.9 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5654044 |
| 512 to 1023 (8.0 to 16.0 µs) | 88368 |
| 1024 to 2047 (16.0 to 32.0 µs) | 67017 |
| 2048 to 4095 (32.0 to 64.0 µs) | 46062 |
| 4096 to 8191 (64.0 to 128.0 µs) | 56005 |
| 8192 to 16383 (128.0 to 256.0 µs) | 19135 |
| 16384 to 32767 (256.0 to 512.0 µs) | 98 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49075 | 22969 / 358.9 |
| timer | 33257 | 19453 / 304.0 |
| job_end | 5815539 | 5044 / 78.8 |
| epilogue | 32858 | 13293 / 207.7 |

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 338745 | 18051 | 0.370 |
| 1 | 23924 | 5999 | 0.150 |
| 2 | 5691349 | 20998 | 6.340 |
| 3 | 1546674 | 5852 | 4.570 |
| 4 | 49789 | 12048 | 0.150 |
| 5 | 335864 | 7400 | 0.890 |
| 6 | 112502 | 1858 | 0.910 |
| none | 12380129 | 44744 | 16537.100 |

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
pulse=11948
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=472
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=240
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
  detail=52284
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
  detail=289177
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
  detail=51877
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
  detail=288609
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
  detail=52220
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
  detail=51877
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
  detail=288729
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
  detail=52280
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
  detail=51813
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
  at_ms=179812
  detail=289172
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179812
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=52284
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
  detail=288609
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
  detail=51877
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
     Finished in 8.10s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 90 % (20 of 22 branch outcomes), part1.pulse 100 % (10 of 10 branch outcomes), part2.filter 70 % (17 of 24 branch outcomes), part2.sampler 59 % (13 of 22 branch outcomes); 0 below

| Item | Functions | Branch outcomes | Covered | Percent | Verdict |
|---|---|---|---|---|---|
| `part0.control` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:state` | 6 | 4 | 4 | 100 % | pass |
| `part0.control#cs:in_cs` | 6 | 4 | 4 | 100 % | pass |
| `part0.echo` | 4 | 0 | 0 | 100 % | pass |
| `part0.sensor` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:state` | 8 | 6 | 5 | 83 % | pass |
| `part0.sensor#cs:in_cs` | 8 | 6 | 5 | 83 % | pass |
| `part1.log` | 9 | 22 | 20 | 90 % | pass |
| `part1.pulse` | 7 | 10 | 10 | 100 % | pass |
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
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 6 | 6 |  |
| `p1_demo_part2::filter::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 3 | 38:84 false |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 4 | 4 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 2 | 0 | 372:34 true, 372:34 false |
| `p1_demo_part2::rsk_jobs::filter` | 0 | 0 |  |
| `p1_demo_part2::rsk_jobs::sampler` | 0 | 0 |  |
| `p1_demo_part2::Stats::new` | 0 | 0 |  |
| `p1_demo_part1::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::acknowledge` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::start` | 0 | 0 |  |
| `p1_demo_part1::pulse_source::respond` | 6 | 6 |  |
| `p1_demo_part1::log` | 8 | 8 |  |
| `p1_demo_part1::log::{closure}` | 2 | 0 | 274:34 true, 274:34 false |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 2 |  |
| `p1_demo_part1::pulse` | 0 | 0 |  |
| `p1_demo_part1::pulse::{closure}` | 0 | 0 |  |
| `p1_demo_part1::inject::spin_ms` | 4 | 4 |  |
| `p1_demo_part1::rsk_jobs::log` | 0 | 0 |  |
| `p1_demo_part1::rsk_jobs::pulse` | 0 | 0 |  |
| `p1_demo_part0::rsk_target::ms` | 0 | 0 |  |
| `p1_demo_part0::echo` | 0 | 0 |  |
| `p1_demo_part0::echo::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 0 | 0 |  |
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 310:80 false |
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
| `p1_demo_part0::validate_rep` | 0 | 0 |  |
