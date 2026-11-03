# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5807 cycles (90.7 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10539 rises and 10000 response edges in the capture); latency 80 to 12520 µs, median 2280 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.7 % of the run, the Kernel 54.7 % over 3366658 entries; runs per Task [18156, 6001, 32015, 5033, 9998, 7201, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 17917 cycles (0.280 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 17920 cycles (0.280 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5807 / 90.7 | 3582 | 2^12:17086 2^13:913 |
| 2 | 8991 | 5807 / 90.7 | 3668 | 2^12:8088 2^13:903 |
| 3 | 4499 | 5807 / 90.7 | 3807 | 2^12:3594 2^13:905 |
| 5 | 7199 | 5759 / 90.0 | 3654 | 2^12:6295 2^13:904 |
| 6 | 1799 | 5019 / 78.4 | 4109 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.467 | 0 | 0 | 0 |
| 1 | 5999 | 0.278 | 0 | 0 | 0 |
| 2 | 8992 | 54.233 | 0 | 0 | 0 |
| 3 | 4500 | 6.713 | 0 | 0 | 0 |
| 4 | 9999 | 12.535 | 10000 | 2045 | 1882 |
| 5 | 7200 | 11.201 | 0 | 0 | 0 |
| 6 | 1800 | 39.755 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 17917 / 0.280 | 0 above the Budget; 2^13:2055 2^14:15631 2^15:314 |
| Task 1 entry | 5999 | 8826 / 0.138 | 0 above the Budget; 2^12:5914 2^13:84 2^14:1 |
| Task 2 entry | 8992 | 2952152 / 46.127 | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 199451 / 3.116 | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10172 / 0.159 | 0 above the Budget; 2^12:9588 2^13:281 2^14:130 |
| Task 5 entry | 7200 | 12692 / 0.198 | 0 above the Budget; 2^13:7033 2^14:167 |
| Task 6 entry | 1800 | 13028 / 0.204 | 0 above the Budget; 2^13:1739 2^14:61 |
| Task 0 on Resource 0 | 18000 | 3924 / 0.061 | |
| Task 0 on Resource 1 | 18000 | 3929 / 0.061 | |
| Task 2 on Resource 0 | 8992 | 774834 / 12.107 | |
| Task 2 on Resource 1 | 17984 | 3714 / 0.058 | |
| Task 5 on Resource 2 | 7200 | 3814 / 0.060 | |
| Task 6 on Resource 2 | 1800 | 7039 / 0.110 | |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3471 cycles
- Task 2 at 179940.000 ms, 3471 cycles
- Task 0 at 179950.000 ms, 3655 cycles
- Task 5 at 179950.000 ms, 3655 cycles
- Task 0 at 179960.000 ms, 3667 cycles
- Task 2 at 179960.000 ms, 3667 cycles
- Task 3 at 179960.000 ms, 3667 cycles
- Task 0 at 179970.000 ms, 3415 cycles
- Task 5 at 179975.000 ms, 3451 cycles
- Task 0 at 179980.000 ms, 3427 cycles
- Task 2 at 179980.000 ms, 3427 cycles
- Task 0 at 179990.000 ms, 3575 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 286969 | 18156 | 0.280 |
| 1 | 22172 | 6001 | 0.130 |
| 2 | 11124111 | 32015 | 12.240 |
| 3 | 1367132 | 5033 | 3.100 |
| 4 | 39069 | 9998 | 0.150 |
| 5 | 56982 | 7201 | 0.190 |
| 6 | 15802 | 1801 | 0.190 |
| none | 5084052 | 32929 | 7.350 |

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
     Finished in 6.98s
```
