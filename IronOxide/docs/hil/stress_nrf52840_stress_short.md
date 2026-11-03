# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8353 cycles (130.5 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 3.590 ms (229752 cycles) against WCRT 9.873 ms (631900 cycles); part0.echo: 5999 Jobs, response at most 0.316 ms (20192 cycles) against WCRT 11.195 ms (716500 cycles); part0.sensor: 8999 Jobs, response at most 8.871 ms (567756 cycles) against WCRT 11.339 ms (725700 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11952 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2070 cycles (32.3 µs), mean 1873 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1944 % of the run |
| Kernel events | pass | 6368 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 73, overrun of Task 6: 61, operating_end of Task none: 1, resource_released of Task 5: 69, partition_restart of Task none: 69, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 5: 4, end_job of Task 6: 61; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11951 response edges inside the run (target counted 11951) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 48.7 % of the run, the Kernel 49.2 % over 2557257 entries; runs per Task [18046, 6006, 20997, 7850, 12028, 7383, 1568] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21706 cycles (0.339 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8285 / 129.5 | 3449 | 2^12:17851 2^13:143 2^14:5 |
| 2 | 8999 | 8353 / 130.5 | 3256 | 2^12:8919 2^13:78 2^14:2 |
| 3 | 4499 | 8285 / 129.5 | 3543 | 2^12:4448 2^13:50 2^14:1 |
| 5 | 7199 | 8285 / 129.5 | 3555 | 2^12:7066 2^13:132 2^14:1 |
| 6 | 1799 | 8285 / 129.5 | 3842 | 2^12:1776 2^13:22 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 3.590 | 0 | 0 | 0 |
| 1 | 5999 | 0.316 | 0 | 0 | 0 |
| 2 | 8999 | 8.871 | 0 | 0 | 0 |
| 3 | 3376 | 23.571 | 0 | 0 | 0 |
| 4 | 11951 | 8.803 | 11952 | 2070 | 1873 |
| 5 | 6235 | 9.383 | 0 | 0 | 0 |
| 6 | 893 | 15.855 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) |
|---|---|---|
| Task 0 entry | 18000 | 21706 / 0.339 |
| Task 1 entry | 5999 | 12569 / 0.196 |
| Task 2 entry | 8999 | 534559 / 8.352 |
| Task 3 entry | 3376 | 885837 / 13.841 |
| Task 4 entry | 11951 | 11419 / 0.178 |
| Task 5 entry | 6235 | 39535 / 0.618 |
| Task 6 entry | 893 | 17016 / 0.266 |
| Task 0 on Resource 0 | 18000 | 9684 / 0.151 |
| Task 0 on Resource 1 | 18000 | 9529 / 0.149 |
| Task 2 on Resource 0 | 8999 | 519101 / 8.111 |
| Task 2 on Resource 1 | 17998 | 9413 / 0.147 |
| Task 5 on Resource 2 | 12582 | 27997 / 0.437 |
| Task 6 on Resource 2 | 893 | 7430 / 0.116 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3289 cycles
- Task 0 at 179940.000 ms, 3145 cycles
- Task 2 at 179945.000 ms, 3149 cycles
- Task 0 at 179950.000 ms, 3649 cycles
- Task 5 at 179950.000 ms, 3649 cycles
- Task 0 at 179960.000 ms, 5217 cycles
- Task 3 at 179960.000 ms, 5217 cycles
- Task 2 at 179965.000 ms, 3441 cycles
- Task 0 at 179970.000 ms, 3413 cycles
- Task 5 at 179975.000 ms, 3221 cycles
- Task 0 at 179980.000 ms, 3145 cycles
- Task 0 at 179990.000 ms, 6417 cycles

## The Kernel's events

6368 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 73, overrun of Task 6: 61, operating_end of Task none: 1, resource_released of Task 5: 69, partition_restart of Task none: 69, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 5: 4, end_job of Task 6: 61; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 294067 | 18046 | 0.330 |
| 1 | 21344 | 6006 | 0.190 |
| 2 | 7444322 | 20997 | 8.250 |
| 3 | 1718044 | 7850 | 4.920 |
| 4 | 42093 | 12028 | 0.130 |
| 5 | 294970 | 7383 | 0.550 |
| 6 | 43582 | 1568 | 0.570 |
| none | 10385034 | 42554 | 15234.360 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3376
pulse=11951
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=179301
  detail=32761
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
  at_ms=179335
  detail=384849
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179335
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=32856
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
  at_ms=179494
  detail=384618
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179494
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=33145
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
  detail=32856
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
  at_ms=179655
  detail=384549
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179655
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=32761
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
  detail=32800
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
  at_ms=179815
  detail=384746
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179815
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=32761
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
  at_ms=179974
  detail=384492
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179974
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=32856
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
     Finished in 7.04s
```
