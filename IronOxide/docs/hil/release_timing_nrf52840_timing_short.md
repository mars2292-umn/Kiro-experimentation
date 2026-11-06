# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7422 cycles (116.0 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10564 rises and 10000 response edges in the capture); latency 80 to 12500 µs, median 2250 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.2 % of the run, the Kernel 58.7 % over 2112236 entries; runs per Task [18337, 6001, 31952, 5005, 10234, 7270, 1804] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18640 cycles (0.291 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7422 / 116.0 | 3569 | 2^12:16777 2^13:1222 |
| 2 | 8991 | 7422 / 116.0 | 3649 | 2^12:7857 2^13:1134 |
| 3 | 4499 | 7422 / 116.0 | 3788 | 2^12:3486 2^13:1013 |
| 5 | 7199 | 7422 / 116.0 | 3605 | 2^12:6219 2^13:980 |
| 6 | 1799 | 7422 / 116.0 | 4095 | 2^12:868 2^13:931 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.570 | 0 | 0 | 0 |
| 1 | 5999 | 0.237 | 0 | 0 | 0 |
| 2 | 8992 | 54.342 | 0 | 0 | 0 |
| 3 | 4500 | 6.858 | 0 | 0 | 0 |
| 4 | 9999 | 12.523 | 10000 | 2175 | 1994 |
| 5 | 7200 | 11.335 | 0 | 0 | 0 |
| 6 | 1800 | 43.586 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18640 / 0.291 (min 8884, mean 11460) | 0 above the Budget; 2^14:17841 2^15:159 |
| Task 1 entry | 5999 | 6659 / 0.104 (min 3033, mean 3170) | 0 above the Budget; 2^12:5879 2^13:120 |
| Task 2 entry | 8992 | 2947549 / 46.055 (min 782085, mean 787297) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199221 / 3.113 (min 181445, mean 195161) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 12474 / 0.195 (min 3073, mean 3483) | 0 above the Budget; 2^12:9407 2^13:423 2^14:169 |
| Task 5 entry | 7200 | 13159 / 0.206 (min 5993, mean 6356) | 0 above the Budget; 2^13:6902 2^14:298 |
| Task 6 entry | 1800 | 25610 / 0.400 (min 6877, mean 9097) | 0 above the Budget; 2^13:645 2^14:1153 2^15:2 |
| Task 0 on Resource 0 | 18000 | 3821 / 0.060 | |
| Task 0 on Resource 1 | 18000 | 3834 / 0.060 | |
| Task 2 on Resource 0 | 8992 | 772542 / 12.071 | |
| Task 2 on Resource 1 | 17984 | 3681 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 7095 / 0.111 | |
| Task 6 on Resource 2 | 1800 | 276 / 0.004 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2848 / 44.50 | 2697 |
| timer | 53294 | 4371 / 68.30 | 2968 |
| intake | 10000 | 2242 / 35.03 | 2060 |
| lock | 71976 | 449 / 7.02 | 447 |
| unlock | 71976 | 1870 / 29.22 | 1664 |
| job_end | 56490 | 2110 / 32.97 | 1866 |
| now | 14034606 | 234 / 3.66 | 234 |
| raise | 6000 | 2096 / 32.75 | 2014 |
| dma | 1800 | 692 / 10.81 | 692 |
| epilogue | 56490 | 1217 / 19.02 | 730 |
| enforce | 8 | 2891 / 45.17 | 2838 |
| view | 80268 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8567, Task 1: 120, Task 2: 55001, Task 3: 2179, Task 4: 879, Task 5: 474, Task 6: 1167

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3405 cycles
- Task 2 at 179940.000 ms, 3405 cycles
- Task 0 at 179950.000 ms, 3730 cycles
- Task 5 at 179950.000 ms, 3730 cycles
- Task 0 at 179960.000 ms, 3593 cycles
- Task 2 at 179960.000 ms, 3593 cycles
- Task 3 at 179960.000 ms, 3593 cycles
- Task 0 at 179970.000 ms, 3538 cycles
- Task 5 at 179975.000 ms, 3330 cycles
- Task 0 at 179980.000 ms, 3349 cycles
- Task 2 at 179980.000 ms, 3349 cycles
- Task 0 at 179990.000 ms, 3166 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 305246 | 18337 | 0.290 |
| 1 | 24192 | 6001 | 0.100 |
| 2 | 11160134 | 31952 | 12.220 |
| 3 | 1369414 | 5005 | 3.100 |
| 4 | 44402 | 10234 | 0.150 |
| 5 | 63241 | 7270 | 0.200 |
| 6 | 23277 | 1804 | 0.240 |
| none | 5006358 | 32706 | 7.330 |

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
     Finished in 7.40s
```
