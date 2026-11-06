# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.6 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7497 cycles (117.1 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10516 rises and 10001 response edges in the capture); latency 80 to 12310 µs, median 2100 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.4 % over 2519702 entries; runs per Task [18158, 6001, 31765, 5005, 10223, 7268, 1822] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 21475 cycles (0.336 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7497 / 117.1 | 3784 | 2^12:16505 2^13:1494 |
| 2 | 8991 | 7420 / 115.9 | 3823 | 2^12:7731 2^13:1260 |
| 3 | 4499 | 7497 / 117.1 | 3965 | 2^12:3511 2^13:988 |
| 5 | 7199 | 7497 / 117.1 | 3837 | 2^12:5893 2^13:1306 |
| 6 | 1799 | 7497 / 117.1 | 4263 | 2^12:696 2^13:1103 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.524 | 0 | 0 | 0 |
| 1 | 5999 | 0.291 | 0 | 0 | 0 |
| 2 | 8992 | 54.302 | 0 | 0 | 0 |
| 3 | 4500 | 6.746 | 0 | 0 | 0 |
| 4 | 10000 | 12.334 | 10000 | 2152 | 1972 |
| 5 | 7200 | 11.294 | 0 | 0 | 0 |
| 6 | 1800 | 43.589 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 21475 / 0.336 (min 8837, mean 11473) | 0 above the Budget; 2^14:17947 2^15:53 |
| Task 1 entry | 5999 | 10500 / 0.164 (min 2994, mean 3169) | 0 above the Budget; 2^12:5874 2^13:124 2^14:1 |
| Task 2 entry | 8992 | 2947677 / 46.057 (min 781713, mean 786925) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199696 / 3.120 (min 181548, mean 195226) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 14126 / 0.221 (min 3056, mean 3595) | 0 above the Budget; 2^12:9175 2^13:648 2^14:177 |
| Task 5 entry | 7200 | 13580 / 0.212 (min 5954, mean 6330) | 0 above the Budget; 2^13:6924 2^14:276 |
| Task 6 entry | 1800 | 26364 / 0.412 (min 6829, mean 9249) | 0 above the Budget; 2^13:616 2^14:1183 2^15:1 |
| Task 0 on Resource 0 | 18000 | 4076 / 0.064 | |
| Task 0 on Resource 1 | 18000 | 4049 / 0.063 | |
| Task 2 on Resource 0 | 8992 | 772726 / 12.074 | |
| Task 2 on Resource 1 | 17984 | 3956 / 0.062 | |
| Task 5 on Resource 2 | 7200 | 7518 / 0.117 | |
| Task 6 on Resource 2 | 1800 | 7311 / 0.114 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2851 / 44.55 | 2690 |
| timer | 52877 | 4550 / 71.09 | 3177 |
| intake | 10000 | 2222 / 34.72 | 2041 |
| lock | 71976 | 445 / 6.95 | 444 |
| unlock | 71976 | 1846 / 28.84 | 1645 |
| job_end | 56491 | 2113 / 33.02 | 1838 |
| now | 14435792 | 232 / 3.62 | 229 |
| raise | 6000 | 2120 / 33.12 | 2008 |
| dma | 1800 | 703 / 10.98 | 703 |
| epilogue | 56491 | 1209 / 18.89 | 718 |
| enforce | 8 | 3098 / 48.41 | 3062 |
| view | 80080 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8419, Task 1: 126, Task 2: 54252, Task 3: 2192, Task 4: 1115, Task 5: 445, Task 6: 1210

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3539 cycles
- Task 2 at 179940.000 ms, 3539 cycles
- Task 0 at 179950.000 ms, 4140 cycles
- Task 5 at 179950.000 ms, 4140 cycles
- Task 0 at 179960.000 ms, 3779 cycles
- Task 2 at 179960.000 ms, 3779 cycles
- Task 3 at 179960.000 ms, 3779 cycles
- Task 0 at 179970.000 ms, 3812 cycles
- Task 5 at 179975.000 ms, 3884 cycles
- Task 0 at 179980.000 ms, 3539 cycles
- Task 2 at 179980.000 ms, 3539 cycles
- Task 0 at 179990.000 ms, 3400 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 303752 | 18158 | 0.330 |
| 1 | 23742 | 6001 | 0.150 |
| 2 | 11156405 | 31765 | 12.220 |
| 3 | 1369727 | 5005 | 3.110 |
| 4 | 45440 | 10223 | 0.210 |
| 5 | 62406 | 7268 | 0.200 |
| 6 | 23745 | 1822 | 0.250 |
| none | 5013738 | 32899 | 7.330 |

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
     Finished in 6.87s
```
