# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8051 cycles (125.8 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 3.560 ms (227856 cycles) against WCRT 9.873 ms (631900 cycles); part0.echo: 5999 Jobs, response at most 0.302 ms (19320 cycles) against WCRT 11.195 ms (716500 cycles); part0.sensor: 8999 Jobs, response at most 8.768 ms (561136 cycles) against WCRT 11.339 ms (725700 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11956 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2025 cycles (31.6 µs), mean 1857 cycles: the measured Δ_irq; processor time 0.1927 % of the run |
| Kernel events | pass | 5537 events mirrored (1024 held, 4 of them the post-detach reboot's): overrun of Task 3: 171, overrun of Task 5: 187, overrun of Task 6: 59, operating_end of Task none: 1, resource_released of Task 5: 183, partition_restart of Task none: 183, safe_state of Task none: 1, end_job of Task 3: 172, end_job of Task 5: 4, end_job of Task 6: 59; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11955 response edges inside the run (target counted 11955) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 55.4 % of the run, the Kernel 53.6 % over 2932180 entries; runs per Task [18072, 6012, 20999, 12370, 12023, 7390, 1445] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8051 / 125.8 | 3197 | 2^12:17851 2^13:148 |
| 2 | 8999 | 7243 / 113.2 | 3016 | 2^12:8911 2^13:88 |
| 3 | 4499 | 6259 / 97.8 | 3256 | 2^12:4455 2^13:44 |
| 5 | 7199 | 8051 / 125.8 | 3302 | 2^12:7144 2^13:55 |
| 6 | 1799 | 8051 / 125.8 | 3548 | 2^12:1782 2^13:17 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 3.560 | 0 | 0 | 0 |
| 1 | 5999 | 0.302 | 0 | 0 | 0 |
| 2 | 8999 | 8.768 | 0 | 0 | 0 |
| 3 | 3613 | 24.127 | 0 | 0 | 0 |
| 4 | 11955 | 8.881 | 11956 | 2025 | 1857 |
| 5 | 6289 | 9.346 | 0 | 0 | 0 |
| 6 | 898 | 4.406 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3351 cycles
- Task 0 at 179940.000 ms, 3355 cycles
- Task 2 at 179945.000 ms, 2883 cycles
- Task 0 at 179950.000 ms, 3323 cycles
- Task 5 at 179950.000 ms, 3323 cycles
- Task 0 at 179960.000 ms, 3103 cycles
- Task 3 at 179960.000 ms, 3103 cycles
- Task 2 at 179965.000 ms, 2883 cycles
- Task 0 at 179970.000 ms, 3371 cycles
- Task 5 at 179975.000 ms, 2879 cycles
- Task 0 at 179980.000 ms, 2883 cycles
- Task 0 at 179990.000 ms, 2795 cycles

## The Kernel's events

5537 events mirrored (1024 held, 4 of them the post-detach reboot's): overrun of Task 3: 171, overrun of Task 5: 187, overrun of Task 6: 59, operating_end of Task none: 1, resource_released of Task 5: 183, partition_restart of Task none: 183, safe_state of Task none: 1, end_job of Task 3: 172, end_job of Task 5: 4, end_job of Task 6: 59; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 279034 | 18072 | 0.310 |
| 1 | 21598 | 6012 | 0.180 |
| 2 | 7432446 | 20999 | 8.230 |
| 3 | 3080210 | 12370 | 6.060 |
| 4 | 63159 | 12023 | 0.160 |
| 5 | 290351 | 7390 | 0.920 |
| 6 | 37771 | 1445 | 0.960 |
| none | 9006119 | 37919 | 15373.140 |

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
log=3613
pulse=11955
echo=5999
blocked_releases=8999
filter=3
filter_state_bits=1107560448
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=3
  at_ms=178740
  detail=384609
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=178740
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=178900
  detail=32639
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=178900
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=178900
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=178940
  detail=384712
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=178940
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179100
  detail=32707
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179100
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179100
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179299
  detail=384767
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179299
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179300
  detail=32707
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179300
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179300
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179340
  detail=384609
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179340
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179500
  detail=32639
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179500
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179500
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179540
  detail=384712
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179540
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179700
  detail=32707
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179700
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179700
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179899
  detail=384767
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179899
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179900
  detail=32707
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179900
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179900
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
     Finished in 6.59s
```
