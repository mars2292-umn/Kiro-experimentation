# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 8000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 189 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5285 cycles (82.6 µs) against J_rel 8000 cycles |
| Kernel events | pass | 68 events mirrored (68 held, 4 of them the post-detach reboot's): overrun of Task 2: 29, deadline_miss of Task 2: 8, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10519 rises and 10000 response edges in the capture); latency 70 to 12330 µs, median 2430 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.5 % of the run, the Kernel 53.7 % over 2955606 entries; runs per Task [18164, 6001, 31733, 5502, 9999, 7237, 1823] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5285 / 82.6 | 3236 | 2^12:17990 2^13:9 |
| 2 | 8991 | 5285 / 82.6 | 3335 | 2^12:8989 2^13:2 |
| 3 | 4499 | 5285 / 82.6 | 3458 | 2^12:4494 2^13:5 |
| 5 | 7199 | 5085 / 79.5 | 3282 | 2^12:7186 2^13:13 |
| 6 | 1799 | 4285 / 67.0 | 3725 | 2^12:1793 2^13:6 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3121 cycles
- Task 2 at 179940.000 ms, 3121 cycles
- Task 0 at 179950.000 ms, 3217 cycles
- Task 5 at 179950.000 ms, 3217 cycles
- Task 0 at 179960.000 ms, 3321 cycles
- Task 2 at 179960.000 ms, 3321 cycles
- Task 3 at 179960.000 ms, 3321 cycles
- Task 0 at 179970.000 ms, 3045 cycles
- Task 5 at 179975.000 ms, 2977 cycles
- Task 0 at 179980.000 ms, 3121 cycles
- Task 2 at 179980.000 ms, 3121 cycles
- Task 0 at 179990.000 ms, 3117 cycles

## The Kernel's events

68 events mirrored (68 held, 4 of them the post-detach reboot's): overrun of Task 2: 29, deadline_miss of Task 2: 8, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 254962 | 18164 | 0.260 |
| 1 | 19934 | 6001 | 0.040 |
| 2 | 11098690 | 31733 | 12.180 |
| 3 | 1364238 | 5502 | 3.090 |
| 4 | 62359 | 9999 | 0.130 |
| 5 | 50758 | 7237 | 0.170 |
| 6 | 14062 | 1823 | 0.170 |
| none | 5131795 | 32682 | 7.390 |

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
     Finished in 6.70s
```
