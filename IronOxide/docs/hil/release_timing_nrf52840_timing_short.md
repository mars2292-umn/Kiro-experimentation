# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7509 cycles (117.3 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10536 rises and 10000 response edges in the capture); latency 80 to 12270 µs, median 2380 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.2 % over 2413059 entries; runs per Task [18175, 6000, 31723, 5005, 10108, 7291, 1823] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18733 cycles (0.293 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7509 / 117.3 | 3766 | 2^12:16432 2^13:1567 |
| 2 | 8991 | 7509 / 117.3 | 3810 | 2^12:7808 2^13:1183 |
| 3 | 4499 | 7509 / 117.3 | 3952 | 2^12:3484 2^13:1015 |
| 5 | 7199 | 7509 / 117.3 | 3813 | 2^12:5884 2^13:1315 |
| 6 | 1799 | 7509 / 117.3 | 4262 | 2^12:828 2^13:971 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.488 | 0 | 0 | 0 |
| 1 | 5999 | 0.287 | 0 | 0 | 0 |
| 2 | 8992 | 54.278 | 0 | 0 | 0 |
| 3 | 4500 | 6.829 | 0 | 0 | 0 |
| 4 | 9999 | 12.286 | 10000 | 2093 | 1915 |
| 5 | 7200 | 11.274 | 0 | 0 | 0 |
| 6 | 1800 | 40.014 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18733 / 0.293 (min 8706, mean 11417) | 0 above the Budget; 2^14:17919 2^15:81 |
| Task 1 entry | 5999 | 6638 / 0.104 (min 2875, mean 2976) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8992 | 2954472 / 46.164 (min 781639, mean 786337) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199423 / 3.116 (min 181768, mean 195231) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 13852 / 0.216 (min 2945, mean 3315) | 0 above the Budget; 2^12:9467 2^13:420 2^14:112 |
| Task 5 entry | 7200 | 17573 / 0.275 (min 5825, mean 6276) | 0 above the Budget; 2^13:6857 2^14:308 2^15:35 |
| Task 6 entry | 1800 | 14159 / 0.221 (min 6710, mean 9188) | 0 above the Budget; 2^13:574 2^14:1226 |
| Task 0 on Resource 0 | 18000 | 4020 / 0.063 | |
| Task 0 on Resource 1 | 18000 | 3977 / 0.062 | |
| Task 2 on Resource 0 | 8992 | 772448 / 12.069 | |
| Task 2 on Resource 1 | 17984 | 3880 / 0.061 | |
| Task 5 on Resource 2 | 7200 | 11948 / 0.187 | |
| Task 6 on Resource 2 | 1800 | 4397 / 0.069 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2729 / 42.64 | 2571 |
| timer | 52323 | 4557 / 71.20 | 3147 |
| intake | 10000 | 2163 / 33.80 | 1984 |
| lock | 71976 | 445 / 6.95 | 444 |
| unlock | 71976 | 1808 / 28.25 | 1625 |
| job_end | 56490 | 2096 / 32.75 | 1809 |
| now | 14332203 | 228 / 3.56 | 227 |
| raise | 6000 | 2066 / 32.28 | 1963 |
| dma | 1800 | 705 / 11.02 | 704 |
| epilogue | 56490 | 1163 / 18.17 | 700 |
| enforce | 8 | 3069 / 47.95 | 3029 |

Entries that interrupted an executing Job: Task 0: 9010, Task 1: 2, Task 2: 53391, Task 3: 2124, Task 4: 663, Task 5: 635, Task 6: 1259

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3511 cycles
- Task 2 at 179940.000 ms, 3511 cycles
- Task 0 at 179950.000 ms, 4041 cycles
- Task 5 at 179950.000 ms, 4041 cycles
- Task 0 at 179960.000 ms, 3807 cycles
- Task 2 at 179960.000 ms, 3807 cycles
- Task 3 at 179960.000 ms, 3807 cycles
- Task 0 at 179970.000 ms, 3873 cycles
- Task 5 at 179975.000 ms, 3449 cycles
- Task 0 at 179980.000 ms, 3511 cycles
- Task 2 at 179980.000 ms, 3511 cycles
- Task 0 at 179990.000 ms, 3681 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 305301 | 18175 | 0.290 |
| 1 | 23115 | 6000 | 0.100 |
| 2 | 11142511 | 31723 | 12.220 |
| 3 | 1370287 | 5005 | 3.100 |
| 4 | 42331 | 10108 | 0.210 |
| 5 | 63223 | 7291 | 0.260 |
| 6 | 23875 | 1823 | 0.200 |
| none | 5025631 | 33023 | 7.300 |

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
     Finished in 7.03s
```
