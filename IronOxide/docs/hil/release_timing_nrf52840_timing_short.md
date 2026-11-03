# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.3 s and reset |
| target run | pass | RSK-OK after 189 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6200 cycles (96.9 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10518 rises and 10000 response edges in the capture); latency 80 to 12390 µs, median 2450 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 71.7 % of the run, the Kernel 54.8 % over 3338178 entries; runs per Task [18177, 6002, 31953, 5476, 9999, 7272, 1800] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 17914 cycles (0.280 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 17280 cycles (0.270 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6200 / 96.9 | 3580 | 2^12:17082 2^13:917 |
| 2 | 8991 | 6200 / 96.9 | 3665 | 2^12:8085 2^13:906 |
| 3 | 4499 | 6200 / 96.9 | 3806 | 2^12:3591 2^13:908 |
| 5 | 7199 | 4844 / 75.7 | 3622 | 2^12:6296 2^13:903 |
| 6 | 1799 | 4844 / 75.7 | 4114 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.457 | 0 | 0 | 0 |
| 1 | 5999 | 0.224 | 0 | 0 | 0 |
| 2 | 8992 | 54.230 | 0 | 0 | 0 |
| 3 | 4500 | 6.699 | 0 | 0 | 0 |
| 4 | 9999 | 12.397 | 10000 | 2052 | 1856 |
| 5 | 7200 | 11.193 | 0 | 0 | 0 |
| 6 | 1800 | 39.745 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) |
|---|---|---|
| Task 0 entry | 18000 | 17914 / 0.280 |
| Task 1 entry | 5999 | 6377 / 0.100 |
| Task 2 entry | 8992 | 2951748 / 46.121 |
| Task 3 entry | 4500 | 202850 / 3.170 |
| Task 4 entry | 9999 | 10041 / 0.157 |
| Task 5 entry | 7200 | 15948 / 0.249 |
| Task 6 entry | 1800 | 12982 / 0.203 |
| Task 0 on Resource 0 | 18000 | 3935 / 0.061 |
| Task 0 on Resource 1 | 18000 | 3856 / 0.060 |
| Task 2 on Resource 0 | 8992 | 772674 / 12.073 |
| Task 2 on Resource 1 | 17984 | 3708 / 0.058 |
| Task 5 on Resource 2 | 7200 | 4304 / 0.067 |
| Task 6 on Resource 2 | 1800 | 3685 / 0.058 |

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3464 cycles
- Task 2 at 179940.000 ms, 3464 cycles
- Task 0 at 179950.000 ms, 3496 cycles
- Task 5 at 179950.000 ms, 3496 cycles
- Task 0 at 179960.000 ms, 3656 cycles
- Task 2 at 179960.000 ms, 3656 cycles
- Task 3 at 179960.000 ms, 3656 cycles
- Task 0 at 179970.000 ms, 3296 cycles
- Task 5 at 179975.000 ms, 3448 cycles
- Task 0 at 179980.000 ms, 3412 cycles
- Task 2 at 179980.000 ms, 3412 cycles
- Task 0 at 179990.000 ms, 3332 cycles

## The Kernel's events

55 events mirrored (55 held, 4 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 285196 | 18177 | 0.270 |
| 1 | 21527 | 6002 | 0.090 |
| 2 | 11119663 | 31953 | 12.200 |
| 3 | 1366254 | 5476 | 3.100 |
| 4 | 39132 | 9999 | 0.150 |
| 5 | 56989 | 7272 | 0.240 |
| 6 | 15938 | 1800 | 0.190 |
| none | 5091591 | 32474 | 7.410 |

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
