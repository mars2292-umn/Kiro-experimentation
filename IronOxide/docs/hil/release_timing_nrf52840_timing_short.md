# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.3 s and reset |
| target run | pass | RSK-OK after 189 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 5739 cycles (89.7 µs) against J_rel 10000 cycles |
| Kernel events | pass | 52 events mirrored (52 held, 4 of them the post-detach reboot's): overrun of Task 2: 5, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10523 rises and 10000 response edges in the capture); latency 80 to 12420 µs, median 2420 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.6 % of the run, the Kernel 54.5 % over 3365407 entries; runs per Task [18162, 6002, 31718, 5291, 9998, 7200, 1801] |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 5739 / 89.7 | 3522 | 2^12:17085 2^13:914 |
| 2 | 8991 | 5739 / 89.7 | 3621 | 2^12:8088 2^13:903 |
| 3 | 4499 | 5739 / 89.7 | 3764 | 2^12:3594 2^13:905 |
| 5 | 7199 | 4871 / 76.1 | 3564 | 2^12:6296 2^13:903 |
| 6 | 1799 | 4871 / 76.1 | 4071 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.326 | 0 | 0 | 0 |
| 1 | 5999 | 0.266 | 0 | 0 | 0 |
| 2 | 8992 | 54.202 | 0 | 0 | 0 |
| 3 | 4500 | 6.742 | 0 | 0 | 0 |
| 4 | 9999 | 12.441 | 10000 | 1993 | 1821 |
| 5 | 7200 | 11.208 | 0 | 0 | 0 |
| 6 | 1800 | 39.788 | 0 | 0 | 0 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3375 cycles
- Task 2 at 179940.000 ms, 3375 cycles
- Task 0 at 179950.000 ms, 3839 cycles
- Task 5 at 179950.000 ms, 3839 cycles
- Task 0 at 179960.000 ms, 3619 cycles
- Task 2 at 179960.000 ms, 3619 cycles
- Task 3 at 179960.000 ms, 3619 cycles
- Task 0 at 179970.000 ms, 3327 cycles
- Task 5 at 179975.000 ms, 3411 cycles
- Task 0 at 179980.000 ms, 3415 cycles
- Task 2 at 179980.000 ms, 3415 cycles
- Task 0 at 179990.000 ms, 3191 cycles

## The Kernel's events

52 events mirrored (52 held, 4 of them the post-detach reboot's): overrun of Task 2: 5, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 264202 | 18162 | 0.270 |
| 1 | 21528 | 6002 | 0.100 |
| 2 | 11105806 | 31718 | 12.220 |
| 3 | 1366880 | 5291 | 3.100 |
| 4 | 66659 | 9998 | 0.140 |
| 5 | 52544 | 7200 | 0.130 |
| 6 | 14788 | 1801 | 0.140 |
| none | 5105028 | 32963 | 7.390 |

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
