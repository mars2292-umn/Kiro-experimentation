# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.5 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5864 cycles (91.6 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10513 rises and 10001 response edges in the capture); latency 80 to 12410 µs, median 2430 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.4 % over 2101752 entries; runs per Task [18346, 6005, 31947, 5385, 9999, 7244, 1807] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18399 cycles (0.287 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 17920 cycles (0.280 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5864 / 91.6 | 3549 | 2^12:17083 2^13:916 |
| 2 | 8991 | 5762 / 90.0 | 3586 | 2^12:8085 2^13:906 |
| 3 | 4499 | 5610 / 87.7 | 3723 | 2^12:3590 2^13:909 |
| 5 | 7199 | 5094 / 79.6 | 3607 | 2^12:6296 2^13:903 |
| 6 | 1799 | 5094 / 79.6 | 4027 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.487 | 0 | 0 | 0 |
| 1 | 5999 | 0.277 | 0 | 0 | 0 |
| 2 | 8992 | 54.281 | 0 | 0 | 0 |
| 3 | 4500 | 6.809 | 0 | 0 | 0 |
| 4 | 10000 | 12.427 | 10000 | 2128 | 1937 |
| 5 | 7200 | 11.274 | 0 | 0 | 0 |
| 6 | 1800 | 40.017 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18399 / 0.287 (min 8755, mean 11375) | 0 above the Budget; 2^14:17991 2^15:9 |
| Task 1 entry | 5999 | 6557 / 0.102 (min 2945, mean 3060) | 0 above the Budget; 2^12:5947 2^13:52 |
| Task 2 entry | 8992 | 2948283 / 46.067 (min 781879, mean 786837) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200302 / 3.130 (min 181636, mean 195031) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 13442 / 0.210 (min 3037, mean 3252) | 0 above the Budget; 2^12:9704 2^13:293 2^14:3 |
| Task 5 entry | 7200 | 12862 / 0.201 (min 5897, mean 6104) | 0 above the Budget; 2^13:7125 2^14:75 |
| Task 6 entry | 1800 | 14302 / 0.223 (min 6793, mean 9331) | 0 above the Budget; 2^13:486 2^14:1314 |
| Task 0 on Resource 0 | 18000 | 3851 / 0.060 | |
| Task 0 on Resource 1 | 18000 | 3856 / 0.060 | |
| Task 2 on Resource 0 | 8992 | 772824 / 12.075 | |
| Task 2 on Resource 1 | 17984 | 6945 / 0.109 | |
| Task 5 on Resource 2 | 7200 | 4708 / 0.074 | |
| Task 6 on Resource 2 | 1800 | 3710 / 0.058 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2803 / 43.80 | 2628 |
| timer | 52418 | 4339 / 67.80 | 2985 |
| intake | 10000 | 2196 / 34.31 | 2004 |
| lock | 71976 | 449 / 7.02 | 447 |
| unlock | 71976 | 1817 / 28.39 | 1621 |
| job_end | 56491 | 2058 / 32.16 | 1807 |
| now | 14021964 | 240 / 3.75 | 237 |
| raise | 6000 | 2076 / 32.44 | 1972 |
| dma | 1800 | 707 / 11.05 | 706 |
| epilogue | 56491 | 1217 / 19.02 | 728 |
| enforce | 8 | 2927 / 45.73 | 2890 |
| view | 80380 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8802, Task 1: 52, Task 2: 54173, Task 3: 2564, Task 4: 300, Task 5: 136, Task 6: 1328

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3342 cycles
- Task 2 at 179940.000 ms, 3342 cycles
- Task 0 at 179950.000 ms, 3728 cycles
- Task 5 at 179950.000 ms, 3728 cycles
- Task 0 at 179960.000 ms, 3570 cycles
- Task 2 at 179960.000 ms, 3570 cycles
- Task 3 at 179960.000 ms, 3570 cycles
- Task 0 at 179970.000 ms, 3304 cycles
- Task 5 at 179975.000 ms, 3364 cycles
- Task 0 at 179980.000 ms, 3342 cycles
- Task 2 at 179980.000 ms, 3342 cycles
- Task 0 at 179990.000 ms, 3276 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 302676 | 18346 | 0.280 |
| 1 | 23220 | 6005 | 0.090 |
| 2 | 11153374 | 31947 | 12.260 |
| 3 | 1370333 | 5385 | 3.110 |
| 4 | 39762 | 9999 | 0.200 |
| 5 | 60382 | 7244 | 0.190 |
| 6 | 24036 | 1807 | 0.200 |
| none | 5025129 | 32599 | 7.300 |

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
     Finished in 6.91s
```
