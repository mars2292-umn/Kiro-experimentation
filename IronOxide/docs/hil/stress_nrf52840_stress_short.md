# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8939 cycles (139.7 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.620 ms (103676 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.274 ms (17532 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.813 ms (436040 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11951 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2105 cycles (32.9 µs), mean 1894 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1965 % of the run |
| Kernel events | pass | 6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11950 response edges inside the run (target counted 11950) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.2 % of the run, the Kernel 44.9 % over 1642927 entries; runs per Task [18042, 6003, 20998, 7685, 12029, 7396, 1831] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22687 cycles (0.354 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8939 / 139.7 | 3563 | 2^12:17832 2^13:155 2^14:12 |
| 2 | 8999 | 8127 / 127.0 | 3313 | 2^12:8901 2^13:98 |
| 3 | 4499 | 8638 / 135.0 | 3593 | 2^12:4444 2^13:54 2^14:1 |
| 5 | 7199 | 8939 / 139.7 | 3621 | 2^12:7101 2^13:94 2^14:4 |
| 6 | 1799 | 8386 / 131.0 | 3893 | 2^12:1774 2^13:24 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.620 | 0 | 0 | 0 |
| 1 | 5999 | 0.274 | 0 | 0 | 0 |
| 2 | 8999 | 6.813 | 0 | 0 | 0 |
| 3 | 3375 | 4.223 | 0 | 0 | 0 |
| 4 | 11950 | 6.850 | 11951 | 2105 | 1894 |
| 5 | 6300 | 7.445 | 0 | 0 | 0 |
| 6 | 900 | 13.852 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22687 / 0.354 (min 8497, mean 11754) | 0 above the Budget; 2^14:17796 2^15:204 |
| Task 1 entry | 5999 | 9372 / 0.146 (min 2890, mean 3003) | 0 above the Budget; 2^12:5979 2^13:1 2^14:19 |
| Task 2 entry | 8999 | 407589 / 6.369 (min 397453, mean 399107) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204284 / 3.192 (min 190648, mean 195521) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11950 | 11709 / 0.183 (min 2944, mean 3074) | 0 above the Budget; 2^12:11721 2^13:162 2^14:67 |
| Task 5 entry | 6300 | 36642 / 0.573 (min 19854, mean 25798) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 17941 / 0.280 (min 6761, mean 9405) | 0 above the Budget; 2^13:246 2^14:648 2^15:6 |
| Task 0 on Resource 0 | 18000 | 10170 / 0.159 | |
| Task 0 on Resource 1 | 18000 | 10047 / 0.157 | |
| Task 2 on Resource 0 | 8999 | 391385 / 6.115 | |
| Task 2 on Resource 1 | 17998 | 6485 / 0.101 | |
| Task 5 on Resource 2 | 12600 | 24904 / 0.389 | |
| Task 6 on Resource 2 | 900 | 7813 / 0.122 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58448 | 2749 / 42.95 | 2550 |
| timer | 65317 | 4446 / 69.47 | 3036 |
| intake | 11951 | 2175 / 33.98 | 1963 |
| lock | 77397 | 445 / 6.95 | 444 |
| unlock | 76497 | 1759 / 27.48 | 1595 |
| job_end | 55523 | 1991 / 31.11 | 1755 |
| now | 8773431 | 226 / 3.53 | 225 |
| raise | 6000 | 1998 / 31.22 | 1926 |
| dma | 900 | 702 / 10.97 | 702 |
| epilogue | 58448 | 1151 / 17.98 | 594 |
| restart | 900 | 4446 / 69.47 | 4288 |
| enforce | 2925 | 4446 / 69.47 | 3562 |

Entries that interrupted an executing Job: Task 0: 11499, Task 1: 39, Task 2: 31772, Task 3: 9025, Task 4: 308, Task 5: 2598, Task 6: 2573

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3483 cycles
- Task 0 at 179940.000 ms, 3190 cycles
- Task 2 at 179945.000 ms, 3194 cycles
- Task 0 at 179950.000 ms, 3963 cycles
- Task 5 at 179950.000 ms, 3963 cycles
- Task 0 at 179960.000 ms, 3438 cycles
- Task 3 at 179960.000 ms, 3438 cycles
- Task 2 at 179965.000 ms, 3619 cycles
- Task 0 at 179970.000 ms, 3675 cycles
- Task 5 at 179975.000 ms, 3186 cycles
- Task 0 at 179980.000 ms, 3194 cycles
- Task 0 at 179990.000 ms, 3090 cycles

## The Kernel's events

6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 312297 | 18042 | 0.350 |
| 1 | 22490 | 6003 | 0.140 |
| 2 | 5658814 | 20998 | 6.260 |
| 3 | 1719880 | 7685 | 4.860 |
| 4 | 44988 | 12029 | 0.140 |
| 5 | 323904 | 7396 | 0.890 |
| 6 | 59794 | 1831 | 0.610 |
| none | 12094121 | 42964 | 15338.870 |

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
pulse=11950
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
  detail=32913
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
  detail=384784
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
  detail=52038
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
  detail=384562
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
  detail=32849
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
  detail=52038
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
  detail=384524
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
  detail=32913
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
  detail=52126
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
  detail=384678
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
  detail=32913
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
  detail=384562
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
  detail=52038
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
     Finished in 6.98s
```
