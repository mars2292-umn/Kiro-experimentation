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
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8332 cycles (130.2 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.618 ms (39556 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.267 ms (17084 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.821 ms (372532 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11950 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2147 cycles (33.5 µs), mean 1933 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2005 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 22314 cycles (348.7 µs) over 4878619 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11949 response edges inside the run (target counted 11949) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 34.9 % of the run, the Kernel 39.2 % over 1732321 entries; runs per Task [18041, 5999, 20998, 5871, 12042, 7379, 1833] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20136 cycles (0.315 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18041 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 366 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 223 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8332 / 130.2 | 3516 | 2^12:17162 2^13:836 2^14:1 |
| 2 | 8999 | 7666 / 119.8 | 3180 | 2^12:8900 2^13:99 |
| 3 | 4499 | 8042 / 125.7 | 3480 | 2^12:4466 2^13:33 |
| 5 | 7199 | 8332 / 130.2 | 3550 | 2^12:6357 2^13:841 2^14:1 |
| 6 | 1799 | 7722 / 120.7 | 3793 | 2^12:1754 2^13:45 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.618 | 0 | 0 | 0 |
| 1 | 5999 | 0.267 | 0 | 0 | 0 |
| 2 | 8999 | 5.821 | 0 | 0 | 0 |
| 3 | 3375 | 4.414 | 0 | 0 | 0 |
| 4 | 11949 | 5.869 | 11950 | 2147 | 1933 |
| 5 | 6300 | 6.444 | 0 | 0 | 0 |
| 6 | 900 | 11.715 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20136 / 0.315 (min 9708, mean 11196) | 0 above the Budget; 2^14:17890 2^15:110 |
| Task 1 entry | 5999 | 6983 / 0.109 (min 3462, mean 3546) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8999 | 346015 / 5.406 (min 335859, mean 338105) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 205941 / 3.218 (min 187877, mean 196631) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11949 | 17066 / 0.267 (min 3584, mean 3972) | 0 above the Budget; 2^12:10868 2^13:1029 2^14:51 2^15:1 |
| Task 5 entry | 6300 | 39076 / 0.611 (min 20832, mean 27670) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 37612 / 0.588 (min 20256, mean 28575) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 6768 / 0.106 | |
| Task 0 on Resource 1 | 18000 | 6770 / 0.106 | |
| Task 2 on Resource 0 | 8999 | 328332 / 5.130 | |
| Task 2 on Resource 1 | 17998 | 6795 / 0.106 | |
| Task 5 on Resource 2 | 12600 | 26109 / 0.408 | |
| Task 6 on Resource 2 | 900 | 8786 / 0.137 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58447 | 3126 / 48.84 | 2908 |
| timer | 56956 | 4669 / 72.95 | 2870 |
| intake | 11950 | 2250 / 35.16 | 2036 |
| lock | 77397 | 440 / 6.88 | 438 |
| unlock | 76497 | 1703 / 26.61 | 1507 |
| job_end | 55522 | 1922 / 30.03 | 1664 |
| now | 4620375 | 222 / 3.47 | 220 |
| raise | 6000 | 1920 / 30.00 | 1852 |
| dma | 8999 | 825 / 12.89 | 564 |
| epilogue | 58447 | 1237 / 19.33 | 612 |
| restart | 900 | 4669 / 72.95 | 4505 |
| enforce | 2925 | 4669 / 72.95 | 3562 |
| view | 72120 | 308 / 4.81 | 276 |

Entries that interrupted an executing Job: Task 0: 2427, Task 1: 2, Task 2: 30110, Task 3: 6449, Task 4: 1178, Task 5: 3576, Task 6: 2537

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3460 cycles
- Task 0 at 179940.000 ms, 3230 cycles
- Task 2 at 179945.000 ms, 3078 cycles
- Task 0 at 179950.000 ms, 4108 cycles
- Task 5 at 179950.000 ms, 4108 cycles
- Task 0 at 179960.000 ms, 3314 cycles
- Task 3 at 179960.000 ms, 3314 cycles
- Task 2 at 179965.000 ms, 3146 cycles
- Task 0 at 179970.000 ms, 3168 cycles
- Task 5 at 179975.000 ms, 3070 cycles
- Task 0 at 179980.000 ms, 3078 cycles
- Task 0 at 179990.000 ms, 2962 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 4878619 of them, the longest 22314 cycles (348.7 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 4573057 |
| 512 to 1023 (8.0 to 16.0 µs) | 118086 |
| 1024 to 2047 (16.0 to 32.0 µs) | 66855 |
| 2048 to 4095 (32.0 to 64.0 µs) | 43129 |
| 4096 to 8191 (64.0 to 128.0 µs) | 64191 |
| 8192 to 16383 (128.0 to 256.0 µs) | 13222 |
| 16384 to 32767 (256.0 to 512.0 µs) | 79 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56286 | 22314 / 348.7 |
| timer | 25253 | 18874 / 294.9 |
| job_end | 4764003 | 5377 / 84.0 |
| epilogue | 33077 | 13687 / 213.9 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 285415 | 18041 | 0.300 |
| 1 | 23727 | 5999 | 0.100 |
| 2 | 4802491 | 20998 | 5.290 |
| 3 | 1546772 | 5871 | 4.580 |
| 4 | 54100 | 12042 | 0.180 |
| 5 | 337847 | 7379 | 0.890 |
| 6 | 112063 | 1833 | 0.830 |
| none | 13362641 | 44775 | 16671.700 |

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
pulse=11949
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=366
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=223
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
  detail=51816
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
  detail=289185
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
  detail=52020
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
  detail=288745
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
  detail=51816
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
  detail=52020
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
  detail=288677
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
  detail=51816
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
  detail=51956
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
  at_ms=179811
  detail=288618
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=51816
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
  detail=288745
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
  detail=52020
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
     Finished in 8.43s
```

## Branch coverage of the measurement inputs (R34.4)

branch coverage of the measurement inputs against the 50 % threshold (R34.4): part0.control 100 % (4 of 4 branch outcomes), part0.echo 100 % (0 of 0 branch outcomes), part0.sensor 83 % (5 of 6 branch outcomes), part1.log 90 % (20 of 22 branch outcomes), part1.pulse 100 % (10 of 10 branch outcomes), part2.filter 70 % (17 of 24 branch outcomes), part2.sampler 63 % (14 of 22 branch outcomes); 0 below

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
| `part2.filter` | 17 | 24 | 17 | 70 % | pass |
| `part2.filter#cs:stats` | 17 | 24 | 17 | 70 % | pass |
| `part2.sampler` | 16 | 22 | 14 | 63 % | pass |
| `part2.sampler#cs:stats` | 16 | 22 | 14 | 63 % | pass |

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
| `p1_demo_part2::filter::{closure}` | 6 | 5 | 335:67 false |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}` | 0 | 0 |  |
| `p1_demo_part2::filter::{closure}::{closure}` | 2 | 2 |  |
| `p1_demo_part2::inject::spin_ms` | 4 | 4 |  |
| `p1_demo_part2::lowpass` | 0 | 0 |  |
| `p1_demo_part2::sampler` | 4 | 4 |  |
| `p1_demo_part2::sampler::{closure}` | 0 | 0 |  |
| `p1_demo_part2::sampler::{closure}` | 2 | 0 | 483:34 true, 483:34 false |
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
| `p1_demo_part1::log` | 8 | 8 |  |
| `p1_demo_part1::log::{closure}` | 0 | 0 |  |
| `p1_demo_part1::log::{closure}` | 2 | 0 | 409:38 true, 409:38 false |
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
| `p1_demo_part0::sensor::{closure}` | 4 | 3 | 385:80 false |
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
