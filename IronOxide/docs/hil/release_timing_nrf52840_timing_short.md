# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8562 cycles (133.8 µs) against J_rel 12500 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17906 cycles (279.8 µs) over 10495671 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10629 rises and 10000 response edges in the capture); latency 100 to 12380 µs, median 2440 µs, 0 later than 15 ms; Δ_stamp observed 4340 cycles (67.8 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.6 % of the run, the Kernel 59.3 % over 1830661 entries; runs per Task [18150, 6001, 31828, 5504, 10000, 7276, 1809] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20455 cycles (0.320 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18150 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 1021 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2796 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 256 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8562 / 133.8 | 3755 | 2^12:16407 2^13:1591 2^14:1 |
| 2 | 8991 | 8562 / 133.8 | 3755 | 2^12:7994 2^13:996 2^14:1 |
| 3 | 4499 | 8562 / 133.8 | 3896 | 2^12:3550 2^13:948 2^14:1 |
| 5 | 7199 | 6992 / 109.2 | 3851 | 2^12:5699 2^13:1500 |
| 6 | 1799 | 6992 / 109.2 | 4189 | 2^12:883 2^13:916 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.937 | 0 | 0 | 0 |
| 1 | 5999 | 0.257 | 0 | 0 | 0 |
| 2 | 8992 | 54.476 | 0 | 0 | 0 |
| 3 | 4500 | 7.032 | 0 | 0 | 0 |
| 4 | 9999 | 12.405 | 10000 | 2328 | 2159 |
| 5 | 7200 | 11.437 | 0 | 0 | 0 |
| 6 | 1800 | 56.824 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20455 / 0.320 (min 10059, mean 13016) | 0 above the Budget; 2^14:15584 2^15:2416 |
| Task 1 entry | 5999 | 7037 / 0.110 (min 3446, mean 3536) | 0 above the Budget; 2^12:5998 2^13:1 |
| Task 2 entry | 8992 | 2944614 / 46.010 (min 784076, mean 789613) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200210 / 3.128 (min 187062, mean 195141) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 11814 / 0.185 (min 3490, mean 3909) | 0 above the Budget; 2^12:9199 2^13:799 2^14:1 |
| Task 5 entry | 7200 | 14395 / 0.225 (min 6808, mean 7174) | 0 above the Budget; 2^13:6817 2^14:383 |
| Task 6 entry | 1800 | 20866 / 0.326 (min 7818, mean 7949) | 0 above the Budget; 2^13:1767 2^14:31 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4320 / 0.068 | |
| Task 0 on Resource 1 | 18000 | 4325 / 0.068 | |
| Task 2 on Resource 0 | 8992 | 773466 / 12.085 | |
| Task 2 on Resource 1 | 17984 | 7031 / 0.110 | |
| Task 5 on Resource 2 | 7200 | 5578 / 0.087 | |
| Task 6 on Resource 2 | 1800 | 7658 / 0.120 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3233 / 50.52 | 3036 |
| timer | 51003 | 4508 / 70.44 | 3112 |
| intake | 10000 | 2438 / 38.09 | 2267 |
| lock | 71976 | 473 / 7.39 | 453 |
| unlock | 71976 | 1896 / 29.62 | 1718 |
| job_end | 56490 | 2173 / 33.95 | 1885 |
| now | 10262312 | 221 / 3.45 | 221 |
| raise | 6000 | 2190 / 34.22 | 2067 |
| dma | 1808 | 715 / 11.17 | 495 |
| epilogue | 56490 | 1253 / 19.58 | 759 |
| enforce | 8 | 3056 / 47.75 | 3010 |
| view | 80413 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 8976, Task 1: 1, Task 2: 53480, Task 3: 2616, Task 4: 801, Task 5: 484, Task 6: 41

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3496 cycles
- Task 2 at 179940.000 ms, 3496 cycles
- Task 0 at 179950.000 ms, 3701 cycles
- Task 5 at 179950.000 ms, 3701 cycles
- Task 0 at 179960.000 ms, 3732 cycles
- Task 2 at 179960.000 ms, 3732 cycles
- Task 3 at 179960.000 ms, 3732 cycles
- Task 0 at 179970.000 ms, 3973 cycles
- Task 5 at 179975.000 ms, 3665 cycles
- Task 0 at 179980.000 ms, 3496 cycles
- Task 2 at 179980.000 ms, 3496 cycles
- Task 0 at 179990.000 ms, 3629 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10495671 of them, the longest 17906 cycles (279.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 10239272 |
| 512 to 1023 (8.0 to 16.0 µs) | 73569 |
| 1024 to 2047 (16.0 to 32.0 µs) | 40272 |
| 2048 to 4095 (32.0 to 64.0 µs) | 69461 |
| 4096 to 8191 (64.0 to 128.0 µs) | 57739 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15357 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48493 | 17906 / 279.8 |
| timer | 44821 | 15141 / 236.6 |
| job_end | 10380070 | 5086 / 79.5 |
| epilogue | 22287 | 4006 / 62.6 |

## The Kernel's events

55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 340902 | 18150 | 0.310 |
| 1 | 25137 | 6001 | 0.100 |
| 2 | 11201449 | 31828 | 12.250 |
| 3 | 1370099 | 5504 | 3.120 |
| 4 | 45983 | 10000 | 0.160 |
| 5 | 69724 | 7276 | 0.210 |
| 6 | 19295 | 1809 | 0.290 |
| none | 4924017 | 32561 | 7.210 |

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
blocked_releases=8994
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1021
loop:part0:sensor:late=2796
loop:part1:spin_ms:spin=0
loop:part1:log:busy=256
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
     Finished in 8.10s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3681335 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
