# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7016 cycles (109.6 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10544 rises and 10000 response edges in the capture); latency 80 to 12410 µs, median 2490 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.1 % over 2203094 entries; runs per Task [18158, 6000, 31924, 5007, 10002, 7290, 1802] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22244 cycles (0.348 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7016 / 109.6 | 3758 | 2^12:16014 2^13:1985 |
| 2 | 8991 | 7016 / 109.6 | 3817 | 2^12:7306 2^13:1685 |
| 3 | 4499 | 7016 / 109.6 | 3959 | 2^12:3550 2^13:949 |
| 5 | 7199 | 7016 / 109.6 | 3823 | 2^12:5310 2^13:1889 |
| 6 | 1799 | 7016 / 109.6 | 4263 | 2^12:199 2^13:1600 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.573 | 0 | 0 | 0 |
| 1 | 5999 | 0.241 | 0 | 0 | 0 |
| 2 | 8992 | 54.353 | 0 | 0 | 0 |
| 3 | 4500 | 6.624 | 0 | 0 | 0 |
| 4 | 9999 | 12.426 | 10000 | 2138 | 1931 |
| 5 | 7200 | 11.351 | 0 | 0 | 0 |
| 6 | 1800 | 43.586 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22244 / 0.348 (min 8825, mean 11663) | 0 above the Budget; 2^14:17756 2^15:244 |
| Task 1 entry | 5999 | 6796 / 0.106 (min 2973, mean 3077) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8992 | 2956845 / 46.201 (min 781750, mean 786826) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200463 / 3.132 (min 177683, mean 195125) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12824 / 0.200 (min 3036, mean 3350) | 0 above the Budget; 2^12:9554 2^13:438 2^14:7 |
| Task 5 entry | 7200 | 21084 / 0.329 (min 5940, mean 6243) | 0 above the Budget; 2^13:7049 2^14:113 2^15:38 |
| Task 6 entry | 1800 | 19204 / 0.300 (min 6825, mean 9312) | 0 above the Budget; 2^13:583 2^14:1216 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4066 / 0.064 | |
| Task 0 on Resource 1 | 18000 | 4023 / 0.063 | |
| Task 2 on Resource 0 | 8992 | 773055 / 12.079 | |
| Task 2 on Resource 1 | 17984 | 3886 / 0.061 | |
| Task 5 on Resource 2 | 7200 | 12124 / 0.189 | |
| Task 6 on Resource 2 | 1800 | 3853 / 0.060 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2849 / 44.52 | 2656 |
| timer | 52843 | 4566 / 71.34 | 3181 |
| intake | 10000 | 2206 / 34.47 | 1998 |
| lock | 71976 | 452 / 7.06 | 450 |
| unlock | 71976 | 1852 / 28.94 | 1640 |
| job_end | 56490 | 2143 / 33.48 | 1839 |
| now | 14117573 | 232 / 3.62 | 229 |
| raise | 6000 | 2091 / 32.67 | 1982 |
| dma | 1800 | 696 / 10.88 | 695 |
| epilogue | 56490 | 1216 / 19.00 | 718 |
| enforce | 8 | 3098 / 48.41 | 3062 |
| view | 80028 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 9381, Task 1: 1, Task 2: 54010, Task 3: 2133, Task 4: 453, Task 5: 330, Task 6: 1224

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3552 cycles
- Task 2 at 179940.000 ms, 3552 cycles
- Task 0 at 179950.000 ms, 3755 cycles
- Task 5 at 179950.000 ms, 3755 cycles
- Task 0 at 179960.000 ms, 3796 cycles
- Task 2 at 179960.000 ms, 3796 cycles
- Task 3 at 179960.000 ms, 3796 cycles
- Task 0 at 179970.000 ms, 3423 cycles
- Task 5 at 179975.000 ms, 3979 cycles
- Task 0 at 179980.000 ms, 3552 cycles
- Task 2 at 179980.000 ms, 3552 cycles
- Task 0 at 179990.000 ms, 3811 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 310329 | 18158 | 0.340 |
| 1 | 23567 | 6000 | 0.090 |
| 2 | 11153763 | 31924 | 12.270 |
| 3 | 1369206 | 5007 | 3.120 |
| 4 | 41407 | 10002 | 0.160 |
| 5 | 62076 | 7290 | 0.260 |
| 6 | 23928 | 1802 | 0.210 |
| none | 5012831 | 32946 | 7.340 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8992
log=4500
pulse=9999
echo=5999
blocked_releases=8992
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
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
expected_sensor=8992
expected_late_jobs=8
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 7.05s
```
