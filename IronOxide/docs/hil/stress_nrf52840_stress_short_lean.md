# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8447 cycles (132.0 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.584 ms (101376 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.251 ms (16036 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.776 ms (433668 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11957 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 1942 cycles (30.3 µs), mean 1717 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1782 % of the run |
| Kernel events | pass | 6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11956 response edges inside the run (target counted 11956) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 0.0 % of the run, the Kernel 100.0 % over 1 entries; runs per Task [0, 0, 0, 0, 0, 0, 0] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8447 / 132.0 | 3261 | 2^12:17863 2^13:135 2^14:1 |
| 2 | 8999 | 7972 / 124.6 | 3044 | 2^12:8942 2^13:57 |
| 3 | 4499 | 7932 / 123.9 | 3329 | 2^12:4467 2^13:32 |
| 5 | 7199 | 8447 / 132.0 | 3348 | 2^12:7123 2^13:75 2^14:1 |
| 6 | 1799 | 7932 / 123.9 | 3644 | 2^12:1781 2^13:18 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.584 | 0 | 0 | 0 |
| 1 | 5999 | 0.251 | 0 | 0 | 0 |
| 2 | 8999 | 6.776 | 0 | 0 | 0 |
| 3 | 3375 | 4.053 | 0 | 0 | 0 |
| 4 | 11956 | 6.760 | 11957 | 1942 | 1717 |
| 5 | 6300 | 7.317 | 0 | 0 | 0 |
| 6 | 900 | 13.674 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20471 / 0.320 (min 7382, mean 10405) | 0 above the Budget; 2^13:4893 2^14:12995 2^15:112 |
| Task 1 entry | 5999 | 8591 / 0.134 (min 2581, mean 2684) | 0 above the Budget; 2^12:5988 2^13:4 2^14:7 |
| Task 2 entry | 8999 | 404920 / 6.327 (min 395748, mean 397120) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 202854 / 3.170 (min 190778, mean 194970) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11956 | 12119 / 0.189 (min 2638, mean 2741) | 0 above the Budget; 2^12:11773 2^13:167 2^14:16 |
| Task 5 entry | 6300 | 31923 / 0.499 (min 18875, mean 24514) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 16114 / 0.252 (min 5873, mean 8345) | 0 above the Budget; 2^13:230 2^14:670 |
| Task 0 on Resource 0 | 18000 | 9240 / 0.144 | |
| Task 0 on Resource 1 | 18000 | 9377 / 0.147 | |
| Task 2 on Resource 0 | 8999 | 390486 / 6.101 | |
| Task 2 on Resource 1 | 17998 | 5952 / 0.093 | |
| Task 5 on Resource 2 | 12600 | 24167 / 0.378 | |
| Task 6 on Resource 2 | 900 | 3420 / 0.053 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58454 | 2451 / 38.30 | 2273 |
| timer | 64390 | 3765 / 58.83 | 2776 |
| intake | 11957 | 2014 / 31.47 | 1788 |
| lock | 77397 | 291 / 4.55 | 290 |
| unlock | 76497 | 1604 / 25.06 | 1435 |
| job_end | 55529 | 1829 / 28.58 | 1592 |
| now | 10814029 | 158 / 2.47 | 155 |
| raise | 6000 | 1822 / 28.47 | 1746 |
| dma | 900 | 545 / 8.52 | 544 |
| epilogue | 58454 | 1018 / 15.91 | 417 |
| restart | 900 | 3765 / 58.83 | 3702 |
| enforce | 2925 | 3765 / 58.83 | 3183 |
| view | 73852 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11619, Task 1: 22, Task 2: 31492, Task 3: 8784, Task 4: 219, Task 5: 2573, Task 6: 2566

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3239 cycles
- Task 0 at 179940.000 ms, 2928 cycles
- Task 2 at 179945.000 ms, 2928 cycles
- Task 0 at 179950.000 ms, 3339 cycles
- Task 5 at 179950.000 ms, 3339 cycles
- Task 0 at 179960.000 ms, 3176 cycles
- Task 3 at 179960.000 ms, 3176 cycles
- Task 2 at 179965.000 ms, 3175 cycles
- Task 0 at 179970.000 ms, 3339 cycles
- Task 5 at 179975.000 ms, 2924 cycles
- Task 0 at 179980.000 ms, 2928 cycles
- Task 0 at 179990.000 ms, 2824 cycles

## The Kernel's events

6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| none | 20199424 | 1 | 201994.240 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short_lean/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short_lean/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11956
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
  detail=32431
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
  at_ms=179333
  detail=384465
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179333
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=51638
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
  at_ms=179492
  detail=384690
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179492
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=32375
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
  detail=51638
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
  at_ms=179653
  detail=384658
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179653
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=32431
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
  detail=51582
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
  at_ms=179813
  detail=384498
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179813
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=32431
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
  at_ms=179972
  detail=384690
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179972
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=51638
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
     Finished in 6.58s
```
