# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.5 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8467 cycles (132.3 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 3.573 ms (228700 cycles) against WCRT 9.873 ms (631900 cycles); part0.echo: 5999 Jobs, response at most 0.314 ms (20072 cycles) against WCRT 11.195 ms (716500 cycles); part0.sensor: 8999 Jobs, response at most 8.798 ms (563052 cycles) against WCRT 11.339 ms (725700 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11953 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2068 cycles (32.3 µs), mean 1888 cycles: the measured Δ_irq; processor time 0.1959 % of the run |
| Kernel events | pass | 5880 events mirrored (1024 held, 4 of them the post-detach reboot's): overrun of Task 3: 147, overrun of Task 5: 156, overrun of Task 6: 130, operating_end of Task none: 1, resource_released of Task 5: 152, partition_restart of Task none: 152, safe_state of Task none: 1, end_job of Task 3: 147, end_job of Task 5: 4, end_job of Task 6: 130; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11952 response edges inside the run (target counted 11952) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 55.5 % of the run, the Kernel 53.7 % over 2936463 entries; runs per Task [18051, 6014, 20997, 12390, 12013, 7349, 1603] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8323 / 130.0 | 3440 | 2^12:17794 2^13:199 2^14:6 |
| 2 | 8999 | 8467 / 132.3 | 3225 | 2^12:8933 2^13:64 2^14:2 |
| 3 | 4499 | 8187 / 127.9 | 3502 | 2^12:4448 2^13:51 |
| 5 | 7199 | 8467 / 132.3 | 3547 | 2^12:7142 2^13:55 2^14:2 |
| 6 | 1799 | 7211 / 112.7 | 3794 | 2^12:1779 2^13:20 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 3.573 | 0 | 0 | 0 |
| 1 | 5999 | 0.314 | 0 | 0 | 0 |
| 2 | 8999 | 8.798 | 0 | 0 | 0 |
| 3 | 3605 | 24.006 | 0 | 0 | 0 |
| 4 | 11952 | 8.877 | 11953 | 2068 | 1888 |
| 5 | 6283 | 9.398 | 0 | 0 | 0 |
| 6 | 898 | 19.922 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3531 cycles
- Task 0 at 179940.000 ms, 3303 cycles
- Task 2 at 179945.000 ms, 3091 cycles
- Task 0 at 179950.000 ms, 3719 cycles
- Task 5 at 179950.000 ms, 3719 cycles
- Task 0 at 179960.000 ms, 3339 cycles
- Task 3 at 179960.000 ms, 3339 cycles
- Task 2 at 179965.000 ms, 3091 cycles
- Task 0 at 179970.000 ms, 3263 cycles
- Task 5 at 179975.000 ms, 3091 cycles
- Task 0 at 179980.000 ms, 3091 cycles
- Task 0 at 179990.000 ms, 3011 cycles

## The Kernel's events

5880 events mirrored (1024 held, 4 of them the post-detach reboot's): overrun of Task 3: 147, overrun of Task 5: 156, overrun of Task 6: 130, operating_end of Task none: 1, resource_released of Task 5: 152, partition_restart of Task none: 152, safe_state of Task none: 1, end_job of Task 3: 147, end_job of Task 5: 4, end_job of Task 6: 130; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 282899 | 18051 | 0.320 |
| 1 | 21367 | 6014 | 0.190 |
| 2 | 7429862 | 20997 | 8.240 |
| 3 | 3084167 | 12390 | 6.120 |
| 4 | 63789 | 12013 | 0.140 |
| 5 | 291450 | 7349 | 0.970 |
| 6 | 46291 | 1603 | 0.630 |
| none | 9012367 | 38102 | 15495.570 |

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
log=3605
pulse=11952
echo=5999
blocked_releases=8999
filter=4
filter_state_bits=1109755648
sampler=1
dma_ok=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=16
  task=255
  at_ms=179075
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179075
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179140
  detail=384567
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179140
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179204
  detail=32879
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179204
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179275
  detail=32714
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179275
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179275
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=179404
  detail=32527
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179404
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179475
  detail=32646
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179475
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179475
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179499
  detail=384819
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179499
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179539
  detail=384674
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179539
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179604
  detail=32527
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179604
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179675
  detail=32714
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179675
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179675
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179740
  detail=384567
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179740
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179804
  detail=32813
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179804
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179875
  detail=32714
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179875
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179875
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
     Finished in 6.67s
```
