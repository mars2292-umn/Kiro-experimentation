# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.4 s and reset |
| target run | pass | RSK-OK after 189 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8199 cycles (128.1 µs) against J_rel 10000 cycles |
| Kernel events | pass | 52 events mirrored (52 held, 4 of them the post-detach reboot's): overrun of Task 2: 5, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10520 rises and 10000 response edges in the capture); latency 80 to 12420 µs, median 2430 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.7 % of the run, the Kernel 54.6 % over 3363808 entries; runs per Task [18140, 6002, 31698, 5080, 9998, 7231, 1820] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8199 / 128.1 | 3540 | 2^12:17006 2^13:992 2^14:1 |
| 2 | 8991 | 8199 / 128.1 | 3633 | 2^12:8055 2^13:935 2^14:1 |
| 3 | 4499 | 8199 / 128.1 | 3777 | 2^12:3578 2^13:920 2^14:1 |
| 5 | 7199 | 6259 / 97.8 | 3567 | 2^12:6282 2^13:917 |
| 6 | 1799 | 6259 / 97.8 | 4083 | 2^12:892 2^13:907 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.328 | 0 | 0 | 0 |
| 1 | 5999 | 0.260 | 0 | 0 | 0 |
| 2 | 8992 | 54.204 | 0 | 0 | 0 |
| 3 | 4500 | 6.754 | 0 | 0 | 0 |
| 4 | 9999 | 12.441 | 10000 | 2025 | 1830 |
| 5 | 7200 | 11.208 | 0 | 0 | 0 |
| 6 | 1800 | 39.806 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3415 cycles
- Task 2 at 179940.000 ms, 3415 cycles
- Task 0 at 179950.000 ms, 3651 cycles
- Task 5 at 179950.000 ms, 3651 cycles
- Task 0 at 179960.000 ms, 4731 cycles
- Task 2 at 179960.000 ms, 4731 cycles
- Task 3 at 179960.000 ms, 4731 cycles
- Task 0 at 179970.000 ms, 3495 cycles
- Task 5 at 179975.000 ms, 3239 cycles
- Task 0 at 179980.000 ms, 3375 cycles
- Task 2 at 179980.000 ms, 3375 cycles
- Task 0 at 179990.000 ms, 3187 cycles

## The Kernel's events

52 events mirrored (52 held, 4 of them the post-detach reboot's): overrun of Task 2: 5, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 262034 | 18140 | 0.260 |
| 1 | 21387 | 6002 | 0.080 |
| 2 | 11103853 | 31698 | 12.190 |
| 3 | 1367758 | 5080 | 3.100 |
| 4 | 70286 | 9998 | 0.150 |
| 5 | 53862 | 7231 | 0.180 |
| 6 | 15744 | 1820 | 0.140 |
| none | 5101365 | 33144 | 7.370 |

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
     Finished in 6.67s
```
