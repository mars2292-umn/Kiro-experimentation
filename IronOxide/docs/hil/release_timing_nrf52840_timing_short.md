# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7329 cycles (114.5 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10558 rises and 10000 response edges in the capture); latency 80 to 12160 µs, median 2280 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.2 % over 1911712 entries; runs per Task [18347, 6001, 31651, 5004, 10109, 7233, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21553 cycles (0.337 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7329 / 114.5 | 3586 | 2^12:16791 2^13:1208 |
| 2 | 8991 | 7329 / 114.5 | 3634 | 2^12:7871 2^13:1120 |
| 3 | 4499 | 7329 / 114.5 | 3773 | 2^12:3492 2^13:1007 |
| 5 | 7199 | 7329 / 114.5 | 3627 | 2^12:6217 2^13:982 |
| 6 | 1799 | 7329 / 114.5 | 4079 | 2^12:869 2^13:930 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.476 | 0 | 0 | 0 |
| 1 | 5999 | 0.286 | 0 | 0 | 0 |
| 2 | 8992 | 54.277 | 0 | 0 | 0 |
| 3 | 4500 | 6.826 | 0 | 0 | 0 |
| 4 | 9999 | 12.183 | 10000 | 2084 | 1923 |
| 5 | 7200 | 11.264 | 0 | 0 | 0 |
| 6 | 1800 | 40.005 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21553 / 0.337 (min 8775, mean 11408) | 0 above the Budget; 2^14:17971 2^15:29 |
| Task 1 entry | 5999 | 6526 / 0.102 (min 2932, mean 3031) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8992 | 2954365 / 46.162 (min 781873, mean 786472) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 203430 / 3.179 (min 181566, mean 195094) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 13412 / 0.210 (min 2994, mean 3356) | 0 above the Budget; 2^12:9464 2^13:420 2^14:115 |
| Task 5 entry | 7200 | 17354 / 0.271 (min 5886, mean 6271) | 0 above the Budget; 2^13:6930 2^14:251 2^15:19 |
| Task 6 entry | 1800 | 14164 / 0.221 (min 6806, mean 9216) | 0 above the Budget; 2^13:557 2^14:1243 |
| Task 0 on Resource 0 | 18000 | 6953 / 0.109 | |
| Task 0 on Resource 1 | 18000 | 3806 / 0.059 | |
| Task 2 on Resource 0 | 8992 | 772456 / 12.070 | |
| Task 2 on Resource 1 | 17984 | 3725 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 7183 / 0.112 | |
| Task 6 on Resource 2 | 1800 | 4466 / 0.070 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2765 / 43.20 | 2612 |
| timer | 52148 | 4338 / 67.78 | 2978 |
| intake | 10000 | 2151 / 33.61 | 1990 |
| lock | 71976 | 450 / 7.03 | 449 |
| unlock | 71976 | 1796 / 28.06 | 1618 |
| job_end | 56490 | 2092 / 32.69 | 1802 |
| now | 13833574 | 238 / 3.72 | 238 |
| raise | 6000 | 2077 / 32.45 | 1975 |
| dma | 1800 | 723 / 11.30 | 723 |
| epilogue | 56490 | 1212 / 18.94 | 714 |
| enforce | 8 | 2914 / 45.53 | 2870 |
| view | 79826 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 9049, Task 1: 1, Task 2: 53180, Task 3: 2129, Task 4: 671, Task 5: 484, Task 6: 1279

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3342 cycles
- Task 2 at 179940.000 ms, 3342 cycles
- Task 0 at 179950.000 ms, 3689 cycles
- Task 5 at 179950.000 ms, 3689 cycles
- Task 0 at 179960.000 ms, 3630 cycles
- Task 2 at 179960.000 ms, 3630 cycles
- Task 3 at 179960.000 ms, 3630 cycles
- Task 0 at 179970.000 ms, 3473 cycles
- Task 5 at 179975.000 ms, 3241 cycles
- Task 0 at 179980.000 ms, 3342 cycles
- Task 2 at 179980.000 ms, 3342 cycles
- Task 0 at 179990.000 ms, 3461 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 303449 | 18347 | 0.330 |
| 1 | 23023 | 6001 | 0.090 |
| 2 | 11144215 | 31651 | 12.210 |
| 3 | 1368897 | 5004 | 3.100 |
| 4 | 42041 | 10109 | 0.200 |
| 5 | 62194 | 7233 | 0.250 |
| 6 | 23793 | 1827 | 0.200 |
| none | 5028666 | 33148 | 7.300 |

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
     Finished in 7.37s
```
