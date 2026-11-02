# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 3682 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8283 cycles (129.4 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 3.575 ms (228780 cycles) against WCRT 9.873 ms (631900 cycles); part0.echo: 121999 Jobs, response at most 0.319 ms (20436 cycles) against WCRT 11.195 ms (716500 cycles); part0.sensor: 182999 Jobs, response at most 8.797 ms (563032 cycles) against WCRT 11.339 ms (725700 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 243090 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2046 cycles (32.0 µs), mean 1857 cycles: the measured Δ_irq; processor time 0.1927 % of the run |
| Kernel events | pass | 119547 events mirrored (1024 held, 3 of them the post-detach reboot's): overrun of Task 3: 145, overrun of Task 5: 146, overrun of Task 6: 145, operating_end of Task none: 1, resource_released of Task 5: 146, partition_restart of Task none: 146, safe_state of Task none: 1, end_job of Task 3: 145, end_job of Task 6: 146; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 243089 response edges inside the run (target counted 243089) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 61.9 % of the run, the Kernel 48.3 % over 59671444 entries; runs per Task [367107, 122302, 427021, 251601, 244464, 150265, 33102] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 8051 / 125.8 | 3179 | 2^12:363157 2^13:2842 |
| 2 | 182999 | 8283 / 129.4 | 3021 | 2^12:181068 2^13:1927 2^14:4 |
| 3 | 91499 | 7855 / 122.7 | 3256 | 2^12:90740 2^13:759 |
| 5 | 146399 | 8283 / 129.4 | 3303 | 2^12:145278 2^13:1117 2^14:4 |
| 6 | 36599 | 8051 / 125.8 | 3552 | 2^12:36249 2^13:350 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 3.575 | 0 | 0 | 0 |
| 1 | 121999 | 0.319 | 0 | 0 | 0 |
| 2 | 182999 | 8.797 | 0 | 0 | 0 |
| 3 | 73421 | 24.258 | 0 | 0 | 0 |
| 4 | 243089 | 8.904 | 243090 | 2046 | 1857 |
| 5 | 127895 | 9.439 | 0 | 0 | 0 |
| 6 | 18280 | 24.138 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3055 cycles
- Task 0 at 3659940.000 ms, 3039 cycles
- Task 2 at 3659945.000 ms, 2883 cycles
- Task 0 at 3659950.000 ms, 3415 cycles
- Task 5 at 3659950.000 ms, 3415 cycles
- Task 0 at 3659960.000 ms, 3103 cycles
- Task 3 at 3659960.000 ms, 3103 cycles
- Task 2 at 3659965.000 ms, 2883 cycles
- Task 0 at 3659970.000 ms, 3007 cycles
- Task 5 at 3659975.000 ms, 2879 cycles
- Task 0 at 3659980.000 ms, 2883 cycles
- Task 0 at 3659990.000 ms, 2795 cycles

## The Kernel's events

119547 events mirrored (1024 held, 3 of them the post-detach reboot's): overrun of Task 3: 145, overrun of Task 5: 146, overrun of Task 6: 145, operating_end of Task none: 1, resource_released of Task 5: 146, partition_restart of Task none: 146, safe_state of Task none: 1, end_job of Task 3: 145, end_job of Task 6: 146; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 5687100 | 367107 | 0.340 |
| 1 | 437729 | 122302 | 0.180 |
| 2 | 151131525 | 427021 | 8.270 |
| 3 | 62674734 | 251601 | 6.530 |
| 4 | 1287949 | 244464 | 0.160 |
| 5 | 5909094 | 150265 | 0.960 |
| 6 | 958672 | 33102 | 0.960 |
| none | 140148717 | 774294 | 15518.320 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182999
log=73421
pulse=243089
echo=121999
blocked_releases=182999
filter=1
filter_state_bits=1095237632
sampler=0
dma_ok=0
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=19
  task=3
  at_ms=3659140
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659154
  detail=32599
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659154
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659154
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=3659301
  detail=32824
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659354
  detail=32527
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659354
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659354
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659499
  detail=384491
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659499
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659501
  detail=32760
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659539
  detail=384555
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659539
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659554
  detail=32599
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659554
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659554
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=3659701
  detail=32832
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659740
  detail=384781
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659740
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659754
  detail=32599
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659754
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659754
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=32824
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659954
  detail=32527
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659954
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659954
  detail=1
  prs_lo=128
  prs_hi=0
event=10
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=183000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 6.70s
```
