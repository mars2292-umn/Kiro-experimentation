# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8631 cycles (134.9 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.613 ms (103212 cycles) against WCRT 9.878 ms (632200 cycles); part0.echo: 5999 Jobs, response at most 0.273 ms (17500 cycles) against WCRT 11.309 ms (723800 cycles); part0.sensor: 8999 Jobs, response at most 6.811 ms (435920 cycles) against WCRT 11.458 ms (733300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11950 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2088 cycles (32.6 µs), mean 1866 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.1936 % of the run |
| Kernel events | pass | 6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11949 response edges inside the run (target counted 11949) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 40.2 % of the run, the Kernel 45.0 % over 1370160 entries; runs per Task [18041, 6005, 20997, 7741, 12009, 7363, 1810] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22816 cycles (0.356 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8631 / 134.9 | 3573 | 2^12:17887 2^13:106 2^14:6 |
| 2 | 8999 | 8280 / 129.4 | 3400 | 2^12:8810 2^13:176 2^14:13 |
| 3 | 4499 | 8388 / 131.1 | 3655 | 2^12:4449 2^13:47 2^14:3 |
| 5 | 7199 | 8388 / 131.1 | 3651 | 2^12:7065 2^13:131 2^14:3 |
| 6 | 1799 | 8388 / 131.1 | 3938 | 2^12:1793 2^13:4 2^14:2 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.613 | 0 | 0 | 0 |
| 1 | 5999 | 0.273 | 0 | 0 | 0 |
| 2 | 8999 | 6.811 | 0 | 0 | 0 |
| 3 | 3375 | 4.227 | 0 | 0 | 0 |
| 4 | 11949 | 6.859 | 11950 | 2088 | 1866 |
| 5 | 6300 | 7.413 | 0 | 0 | 0 |
| 6 | 900 | 13.844 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22816 / 0.356 (min 8573, mean 11861) | 0 above the Budget; 2^14:17183 2^15:817 |
| Task 1 entry | 5999 | 9420 / 0.147 (min 2973, mean 3094) | 0 above the Budget; 2^12:5972 2^13:3 2^14:24 |
| Task 2 entry | 8999 | 407738 / 6.371 (min 397758, mean 399427) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204239 / 3.191 (min 190811, mean 195381) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11949 | 12251 / 0.191 (min 3028, mean 3156) | 0 above the Budget; 2^12:11729 2^13:163 2^14:57 |
| Task 5 entry | 6300 | 37681 / 0.589 (min 19789, mean 25878) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 14834 / 0.232 (min 6861, mean 9427) | 0 above the Budget; 2^13:268 2^14:632 |
| Task 0 on Resource 0 | 18000 | 6488 / 0.101 | |
| Task 0 on Resource 1 | 18000 | 6413 / 0.100 | |
| Task 2 on Resource 0 | 8999 | 391223 / 6.113 | |
| Task 2 on Resource 1 | 17998 | 6410 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 25101 / 0.392 | |
| Task 6 on Resource 2 | 900 | 275 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58447 | 2821 / 44.08 | 2623 |
| timer | 65227 | 4498 / 70.28 | 3105 |
| intake | 11950 | 2156 / 33.69 | 1933 |
| lock | 77397 | 452 / 7.06 | 450 |
| unlock | 76497 | 1744 / 27.25 | 1578 |
| job_end | 55522 | 1983 / 30.98 | 1744 |
| now | 8495926 | 232 / 3.62 | 229 |
| raise | 6000 | 1984 / 31.00 | 1909 |
| dma | 900 | 698 / 10.91 | 696 |
| epilogue | 58447 | 1219 / 19.05 | 604 |
| restart | 900 | 4498 / 70.28 | 4338 |
| enforce | 2925 | 4498 / 70.28 | 3624 |
| view | 73916 | 327 / 5.11 | 293 |

Entries that interrupted an executing Job: Task 0: 11335, Task 1: 51, Task 2: 31857, Task 3: 9148, Task 4: 281, Task 5: 2467, Task 6: 2470

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3511 cycles
- Task 0 at 179940.000 ms, 5336 cycles
- Task 2 at 179945.000 ms, 3256 cycles
- Task 0 at 179950.000 ms, 3799 cycles
- Task 5 at 179950.000 ms, 3799 cycles
- Task 0 at 179960.000 ms, 3496 cycles
- Task 3 at 179960.000 ms, 3496 cycles
- Task 2 at 179965.000 ms, 3699 cycles
- Task 0 at 179970.000 ms, 3491 cycles
- Task 5 at 179975.000 ms, 3248 cycles
- Task 0 at 179980.000 ms, 3256 cycles
- Task 0 at 179990.000 ms, 3144 cycles

## The Kernel's events

6757 events mirrored (512 held, 4 of them the post-detach reboot's): overrun of Task 3: 84, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 84, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 313335 | 18041 | 0.350 |
| 1 | 22620 | 6005 | 0.140 |
| 2 | 5663545 | 20997 | 6.260 |
| 3 | 1719557 | 7741 | 4.830 |
| 4 | 45076 | 12009 | 0.130 |
| 5 | 324011 | 7363 | 0.890 |
| 6 | 59365 | 1810 | 0.610 |
| none | 12116427 | 42979 | 15414.700 |

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
pulse=11949
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
  detail=32751
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
  detail=384933
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
  detail=52218
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
  detail=384904
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
  detail=32687
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
  detail=52218
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
  detail=32751
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
  detail=52154
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
  detail=384712
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
  detail=32751
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
  detail=384904
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
  detail=52218
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
     Finished in 7.07s
```
