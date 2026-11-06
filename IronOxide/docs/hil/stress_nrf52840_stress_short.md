# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8426 cycles (131.7 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.607 ms (102872 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.273 ms (17472 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.855 ms (438748 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11954 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2082 cycles (32.5 µs), mean 1884 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1955 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11953 response edges inside the run (target counted 11953) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.1 % of the run, the Kernel 44.9 % over 1249276 entries; runs per Task [18106, 6010, 20997, 7664, 12063, 7397, 1834] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22229 cycles (0.347 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8426 / 131.7 | 3336 | 2^12:17862 2^13:135 2^14:2 |
| 2 | 8999 | 8038 / 125.6 | 3120 | 2^12:8923 2^13:76 |
| 3 | 4499 | 7143 / 111.6 | 3391 | 2^12:4456 2^13:43 |
| 5 | 7199 | 8038 / 125.6 | 3410 | 2^12:7135 2^13:64 |
| 6 | 1799 | 7951 / 124.2 | 3700 | 2^12:1774 2^13:25 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.607 | 0 | 0 | 0 |
| 1 | 5999 | 0.273 | 0 | 0 | 0 |
| 2 | 8999 | 6.855 | 0 | 0 | 0 |
| 3 | 3375 | 4.162 | 0 | 0 | 0 |
| 4 | 11953 | 6.856 | 11954 | 2082 | 1884 |
| 5 | 6300 | 7.410 | 0 | 0 | 0 |
| 6 | 900 | 13.874 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22229 / 0.347 (min 8591, mean 11756) | 0 above the Budget; 2^14:17820 2^15:180 |
| Task 1 entry | 5999 | 9246 / 0.144 (min 2964, mean 3068) | 0 above the Budget; 2^12:5979 2^13:1 2^14:19 |
| Task 2 entry | 8999 | 409383 / 6.397 (min 397629, mean 399281) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204050 / 3.188 (min 187874, mean 195472) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11953 | 11743 / 0.183 (min 3022, mean 3153) | 0 above the Budget; 2^12:11706 2^13:157 2^14:90 |
| Task 5 entry | 6300 | 36986 / 0.578 (min 19964, mean 25688) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 17716 / 0.277 (min 6882, mean 9352) | 0 above the Budget; 2^13:251 2^14:643 2^15:6 |
| Task 0 on Resource 0 | 18000 | 9699 / 0.152 | |
| Task 0 on Resource 1 | 18000 | 9540 / 0.149 | |
| Task 2 on Resource 0 | 8999 | 391124 / 6.111 | |
| Task 2 on Resource 1 | 17998 | 6318 / 0.099 | |
| Task 5 on Resource 2 | 12600 | 24654 / 0.385 | |
| Task 6 on Resource 2 | 900 | 3576 / 0.056 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58451 | 2793 / 43.64 | 2611 |
| timer | 65275 | 4232 / 66.12 | 2839 |
| intake | 11954 | 2149 / 33.58 | 1951 |
| lock | 77397 | 450 / 7.03 | 449 |
| unlock | 76497 | 1759 / 27.48 | 1586 |
| job_end | 55526 | 1988 / 31.06 | 1737 |
| now | 8379508 | 238 / 3.72 | 235 |
| raise | 6000 | 2010 / 31.41 | 1929 |
| dma | 900 | 719 / 11.23 | 718 |
| epilogue | 58451 | 1213 / 18.95 | 604 |
| restart | 900 | 4232 / 66.12 | 4069 |
| enforce | 2925 | 4232 / 66.12 | 3344 |
| view | 73950 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11615, Task 1: 39, Task 2: 31746, Task 3: 8949, Task 4: 358, Task 5: 2592, Task 6: 2570

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3186 cycles
- Task 0 at 179940.000 ms, 2995 cycles
- Task 2 at 179945.000 ms, 2995 cycles
- Task 0 at 179950.000 ms, 3418 cycles
- Task 5 at 179950.000 ms, 3418 cycles
- Task 0 at 179960.000 ms, 3235 cycles
- Task 3 at 179960.000 ms, 3235 cycles
- Task 2 at 179965.000 ms, 3638 cycles
- Task 0 at 179970.000 ms, 3298 cycles
- Task 5 at 179975.000 ms, 2991 cycles
- Task 0 at 179980.000 ms, 2995 cycles
- Task 0 at 179990.000 ms, 2915 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 310741 | 18106 | 0.340 |
| 1 | 22340 | 6010 | 0.140 |
| 2 | 5660939 | 20997 | 6.260 |
| 3 | 1718934 | 7664 | 4.910 |
| 4 | 44841 | 12063 | 0.140 |
| 5 | 321690 | 7397 | 0.870 |
| 6 | 59069 | 1834 | 0.550 |
| none | 12140742 | 42953 | 15583.070 |

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
log=3375
pulse=11953
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
  detail=32843
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
  detail=385033
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
  detail=52043
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
  detail=384831
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
  detail=33587
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
  detail=52043
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
  detail=384663
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
  detail=32843
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
  detail=51975
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
  detail=384823
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
  detail=32843
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
  detail=384831
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
  detail=52043
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
     Finished in 7.06s
```
