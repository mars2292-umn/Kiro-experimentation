# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| target run | pass | RSK-OK after 190 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6719 cycles (105.0 µs) against J_rel 10000 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10545 rises and 10000 response edges in the capture); latency 80 to 12560 µs, median 2310 µs, 0 later than 15 ms; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.1 % of the run, the Kernel 58.7 % over 2099498 entries; runs per Task [18328, 6002, 31673, 5086, 9999, 7233, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 18895 cycles (0.295 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 18560 cycles (0.290 ms at 100000 samples/s) |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6719 / 105.0 | 3742 | 2^12:16550 2^13:1449 |
| 2 | 8991 | 6606 / 103.2 | 3781 | 2^12:7928 2^13:1063 |
| 3 | 4499 | 6606 / 103.2 | 3920 | 2^12:3564 2^13:935 |
| 5 | 7199 | 6719 / 105.0 | 3808 | 2^12:5866 2^13:1333 |
| 6 | 1799 | 6606 / 103.2 | 4224 | 2^12:796 2^13:1003 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.605 | 0 | 0 | 0 |
| 1 | 5999 | 0.236 | 0 | 0 | 0 |
| 2 | 8992 | 54.315 | 0 | 0 | 0 |
| 3 | 4500 | 6.853 | 0 | 0 | 0 |
| 4 | 9999 | 12.576 | 10000 | 2088 | 1920 |
| 5 | 7200 | 11.290 | 0 | 0 | 0 |
| 6 | 1800 | 43.555 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 18895 / 0.295 (min 8778, mean 11483) | 0 above the Budget; 2^14:17989 2^15:11 |
| Task 1 entry | 5999 | 6798 / 0.106 (min 2973, mean 3066) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8992 | 2948506 / 46.070 (min 781830, mean 786537) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200177 / 3.128 (min 181450, mean 195335) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10614 / 0.166 (min 3037, mean 3315) | 0 above the Budget; 2^12:9596 2^13:402 2^14:1 |
| Task 5 entry | 7200 | 13418 / 0.210 (min 5917, mean 6300) | 0 above the Budget; 2^13:6929 2^14:271 |
| Task 6 entry | 1800 | 19180 / 0.300 (min 6813, mean 9404) | 0 above the Budget; 2^13:537 2^14:1262 2^15:1 |
| Task 0 on Resource 0 | 18000 | 6713 / 0.105 | |
| Task 0 on Resource 1 | 18000 | 4062 / 0.063 | |
| Task 2 on Resource 0 | 8992 | 772840 / 12.076 | |
| Task 2 on Resource 1 | 17984 | 3925 / 0.061 | |
| Task 5 on Resource 2 | 7200 | 7619 / 0.119 | |
| Task 6 on Resource 2 | 1800 | 4462 / 0.070 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 2813 / 43.95 | 2650 |
| timer | 52070 | 4576 / 71.50 | 3183 |
| intake | 10000 | 2155 / 33.67 | 1986 |
| lock | 71976 | 445 / 6.95 | 443 |
| unlock | 71976 | 1822 / 28.47 | 1617 |
| job_end | 56490 | 2064 / 32.25 | 1808 |
| now | 14015933 | 236 / 3.69 | 234 |
| raise | 6000 | 2055 / 32.11 | 1953 |
| dma | 1800 | 706 / 11.03 | 706 |
| epilogue | 56490 | 1207 / 18.86 | 710 |
| enforce | 8 | 3108 / 48.56 | 3072 |
| view | 79820 | 327 / 5.11 | 289 |

Entries that interrupted an executing Job: Task 0: 8908, Task 1: 2, Task 2: 53217, Task 3: 2256, Task 4: 404, Task 5: 475, Task 6: 1300

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3526 cycles
- Task 2 at 179940.000 ms, 3526 cycles
- Task 0 at 179950.000 ms, 3683 cycles
- Task 5 at 179950.000 ms, 3683 cycles
- Task 0 at 179960.000 ms, 3762 cycles
- Task 2 at 179960.000 ms, 3762 cycles
- Task 3 at 179960.000 ms, 3762 cycles
- Task 0 at 179970.000 ms, 3635 cycles
- Task 5 at 179975.000 ms, 3971 cycles
- Task 0 at 179980.000 ms, 3526 cycles
- Task 2 at 179980.000 ms, 3526 cycles
- Task 0 at 179990.000 ms, 3743 cycles

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 304992 | 18328 | 0.290 |
| 1 | 23121 | 6002 | 0.100 |
| 2 | 11146641 | 31673 | 12.240 |
| 3 | 1370807 | 5086 | 3.110 |
| 4 | 40473 | 9999 | 0.160 |
| 5 | 62243 | 7233 | 0.200 |
| 6 | 24231 | 1827 | 0.210 |
| none | 5023960 | 33154 | 7.300 |

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
     Finished in 7.07s
```
