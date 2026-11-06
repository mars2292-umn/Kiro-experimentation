# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.5 s and reset |
| target run | pass | RSK-OK after 189 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7091 cycles (110.8 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10511 rises and 10001 response edges in the capture); latency 80 to 12310 µs, median 2190 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.4 % over 2518771 entries; runs per Task [18184, 6001, 31606, 5505, 9999, 7223, 1826] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22203 cycles (0.347 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7091 / 110.8 | 3775 | 2^12:16493 2^13:1506 |
| 2 | 8991 | 7091 / 110.8 | 3800 | 2^12:7858 2^13:1133 |
| 3 | 4499 | 7091 / 110.8 | 3942 | 2^12:3553 2^13:946 |
| 5 | 7199 | 7091 / 110.8 | 3836 | 2^12:5793 2^13:1406 |
| 6 | 1799 | 7091 / 110.8 | 4241 | 2^12:742 2^13:1057 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.513 | 0 | 0 | 0 |
| 1 | 5999 | 0.238 | 0 | 0 | 0 |
| 2 | 8992 | 54.292 | 0 | 0 | 0 |
| 3 | 4500 | 6.740 | 0 | 0 | 0 |
| 4 | 10000 | 12.334 | 10000 | 2198 | 1964 |
| 5 | 7200 | 11.292 | 0 | 0 | 0 |
| 6 | 1800 | 39.939 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22203 / 0.347 (min 8837, mean 11606) | 0 above the Budget; 2^14:17811 2^15:189 |
| Task 1 entry | 5999 | 6800 / 0.106 (min 3022, mean 3110) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8992 | 2947629 / 46.057 (min 781713, mean 786638) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199748 / 3.121 (min 177188, mean 194549) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 14226 / 0.222 (min 3056, mean 3451) | 0 above the Budget; 2^12:9412 2^13:531 2^14:57 |
| Task 5 entry | 7200 | 16972 / 0.265 (min 5954, mean 6319) | 0 above the Budget; 2^13:6948 2^14:243 2^15:9 |
| Task 6 entry | 1800 | 14729 / 0.230 (min 6829, mean 9174) | 0 above the Budget; 2^13:652 2^14:1148 |
| Task 0 on Resource 0 | 18000 | 7724 / 0.121 | |
| Task 0 on Resource 1 | 18000 | 7853 / 0.123 | |
| Task 2 on Resource 0 | 8992 | 772710 / 12.074 | |
| Task 2 on Resource 1 | 17984 | 3848 / 0.060 | |
| Task 5 on Resource 2 | 7200 | 7670 / 0.120 | |
| Task 6 on Resource 2 | 1800 | 4523 / 0.071 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2911 / 45.48 | 2689 |
| timer | 52222 | 4534 / 70.84 | 3182 |
| intake | 10000 | 2268 / 35.44 | 2033 |
| lock | 71976 | 445 / 6.95 | 444 |
| unlock | 71976 | 1846 / 28.84 | 1645 |
| job_end | 56491 | 2113 / 33.02 | 1836 |
| now | 14430056 | 232 / 3.62 | 229 |
| raise | 6000 | 2089 / 32.64 | 2004 |
| dma | 1800 | 703 / 10.98 | 703 |
| epilogue | 56491 | 1209 / 18.89 | 719 |
| enforce | 8 | 3098 / 48.41 | 3062 |
| view | 80156 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8953, Task 1: 1, Task 2: 53391, Task 3: 2621, Task 4: 647, Task 5: 385, Task 6: 1176

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3539 cycles
- Task 2 at 179940.000 ms, 3539 cycles
- Task 0 at 179950.000 ms, 4140 cycles
- Task 5 at 179950.000 ms, 4140 cycles
- Task 0 at 179960.000 ms, 3779 cycles
- Task 2 at 179960.000 ms, 3779 cycles
- Task 3 at 179960.000 ms, 3779 cycles
- Task 0 at 179970.000 ms, 3812 cycles
- Task 5 at 179975.000 ms, 3844 cycles
- Task 0 at 179980.000 ms, 3583 cycles
- Task 2 at 179980.000 ms, 3583 cycles
- Task 0 at 179990.000 ms, 3864 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 307551 | 18184 | 0.340 |
| 1 | 23149 | 6001 | 0.090 |
| 2 | 11151636 | 31606 | 12.220 |
| 3 | 1367490 | 5505 | 3.110 |
| 4 | 41952 | 9999 | 0.220 |
| 5 | 62085 | 7223 | 0.260 |
| 6 | 23539 | 1826 | 0.200 |
| none | 5021661 | 32823 | 7.290 |

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
pulse=10000
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
     Finished in 6.86s
```
