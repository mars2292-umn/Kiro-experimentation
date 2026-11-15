# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 192 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6861 cycles (107.2 µs) against J_rel 12500 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 17, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 14703 cycles (229.7 µs) over 10237508 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 10000 stimulus pulses answered (target counted 10000; 10658 rises and 10001 response edges in the capture); latency 90 to 12270 µs, median 2050 µs, 0 later than 15 ms; Δ_stamp observed 3496 cycles (54.6 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 58.7 % over 2103467 entries; runs per Task [18126, 6002, 31605, 5410, 9999, 7226, 1821] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20907 cycles (0.327 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18126 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:380 | 10_000_000 | 994 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:418 | 50_000_000 | 2697 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 250 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6861 / 107.2 | 3654 | 2^12:16955 2^13:1044 |
| 2 | 8991 | 6861 / 107.2 | 3677 | 2^12:8083 2^13:908 |
| 3 | 4499 | 6861 / 107.2 | 3817 | 2^12:3590 2^13:909 |
| 5 | 7199 | 5321 / 83.1 | 3719 | 2^12:6170 2^13:1029 |
| 6 | 1799 | 5321 / 83.1 | 4122 | 2^12:896 2^13:903 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.791 | 0 | 0 | 0 |
| 1 | 5999 | 0.301 | 0 | 0 | 0 |
| 2 | 8992 | 54.433 | 0 | 0 | 0 |
| 3 | 4500 | 6.914 | 0 | 0 | 0 |
| 4 | 10000 | 12.292 | 10000 | 2205 | 2023 |
| 5 | 7200 | 11.411 | 0 | 0 | 0 |
| 6 | 1800 | 56.693 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20907 / 0.327 (min 9815, mean 12612) | 0 above the Budget; 2^14:17619 2^15:381 |
| Task 1 entry | 5999 | 7079 / 0.111 (min 3355, mean 3426) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8992 | 2944027 / 46.000 (min 783887, mean 789296) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201606 / 3.150 (min 181313, mean 195787) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 10000 | 7243 / 0.113 (min 3391, mean 3727) | 0 above the Budget; 2^12:9418 2^13:582 |
| Task 5 entry | 7200 | 11610 / 0.181 (min 6643, mean 6880) | 0 above the Budget; 2^13:7023 2^14:177 |
| Task 6 entry | 1800 | 20493 / 0.320 (min 7667, mean 7847) | 0 above the Budget; 2^13:1748 2^14:50 2^15:2 |
| Task 0 on Resource 0 | 18000 | 11352 / 0.177 | |
| Task 0 on Resource 1 | 18000 | 4245 / 0.066 | |
| Task 2 on Resource 0 | 8992 | 773578 / 12.087 | |
| Task 2 on Resource 1 | 17984 | 6250 / 0.098 | |
| Task 5 on Resource 2 | 7200 | 5395 / 0.084 | |
| Task 6 on Resource 2 | 1800 | 5360 / 0.084 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 3093 / 48.33 | 2922 |
| timer | 50758 | 4370 / 68.28 | 3024 |
| intake | 10000 | 2315 / 36.17 | 2133 |
| lock | 71976 | 462 / 7.22 | 459 |
| unlock | 71976 | 1817 / 28.39 | 1595 |
| job_end | 56491 | 2069 / 32.33 | 1782 |
| now | 10003416 | 224 / 3.50 | 224 |
| raise | 6000 | 2057 / 32.14 | 1950 |
| dma | 1808 | 725 / 11.33 | 501 |
| epilogue | 56491 | 1254 / 19.59 | 754 |
| enforce | 8 | 2938 / 45.91 | 2899 |
| view | 80058 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 8819, Task 1: 3, Task 2: 53378, Task 3: 2599, Task 4: 582, Task 5: 200, Task 6: 74

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3433 cycles
- Task 2 at 179940.000 ms, 3433 cycles
- Task 0 at 179950.000 ms, 3558 cycles
- Task 5 at 179950.000 ms, 3558 cycles
- Task 0 at 179960.000 ms, 3673 cycles
- Task 2 at 179960.000 ms, 3673 cycles
- Task 3 at 179960.000 ms, 3673 cycles
- Task 0 at 179970.000 ms, 3278 cycles
- Task 5 at 179975.000 ms, 3706 cycles
- Task 0 at 179980.000 ms, 3433 cycles
- Task 2 at 179980.000 ms, 3433 cycles
- Task 0 at 179990.000 ms, 3746 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10237508 of them, the longest 14703 cycles (229.7 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9979273 |
| 512 to 1023 (8.0 to 16.0 µs) | 74720 |
| 1024 to 2047 (16.0 to 32.0 µs) | 60983 |
| 2048 to 4095 (32.0 to 64.0 µs) | 54620 |
| 4096 to 8191 (64.0 to 128.0 µs) | 52876 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15036 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48743 | 14703 / 229.7 |
| timer | 44574 | 14699 / 229.7 |
| job_end | 10121471 | 4981 / 77.8 |
| epilogue | 22720 | 3950 / 61.7 |

## The Kernel's events

53 events mirrored (53 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 17, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 329837 | 18126 | 0.310 |
| 1 | 24074 | 6002 | 0.100 |
| 2 | 11194042 | 31605 | 12.240 |
| 3 | 1374440 | 5410 | 3.130 |
| 4 | 42920 | 9999 | 0.100 |
| 5 | 65899 | 7226 | 0.160 |
| 6 | 19107 | 1821 | 0.300 |
| none | 4948745 | 32921 | 7.220 |

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=994
loop:part0:sensor:late=2697
loop:part1:spin_ms:spin=0
loop:part1:log:busy=250
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=1799
dma_after_stop=0
spim0_enable=7
spim0_shorts=131072
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=0
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
     Finished in 8.09s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3682505 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
