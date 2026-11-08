# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.9 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6710 cycles (104.8 µs) against J_rel 12500 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 15025 cycles (234.8 µs) over 11262755 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10584 rises and 10000 response edges in the capture); latency 90 to 12590 µs, median 2320 µs, 0 later than 15 ms; Δ_stamp observed 4176 cycles (65.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.4 % of the run, the Kernel 63.3 % over 1045968 entries; runs per Task [18171, 6001, 32008, 5506, 9999, 7230, 1804] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22379 cycles (0.350 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21760 cycles (0.340 ms at 100000 samples/s; 18171 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 1097 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 2984 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 275 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6710 / 104.8 | 3718 | 2^12:16491 2^13:1508 |
| 2 | 8991 | 6710 / 104.8 | 3726 | 2^12:8008 2^13:983 |
| 3 | 4499 | 6710 / 104.8 | 3873 | 2^12:3556 2^13:943 |
| 5 | 7199 | 6666 / 104.2 | 3794 | 2^12:5780 2^13:1419 |
| 6 | 1799 | 6666 / 104.2 | 4179 | 2^12:885 2^13:914 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.729 | 0 | 0 | 0 |
| 1 | 5999 | 0.290 | 0 | 0 | 0 |
| 2 | 8992 | 54.380 | 0 | 0 | 0 |
| 3 | 4500 | 6.841 | 0 | 0 | 0 |
| 4 | 9999 | 12.612 | 10000 | 2157 | 1989 |
| 5 | 7200 | 11.368 | 0 | 0 | 0 |
| 6 | 1800 | 56.349 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22379 / 0.350 (min 9579, mean 12421) | 0 above the Budget; 2^14:17502 2^15:498 |
| Task 1 entry | 5999 | 7056 / 0.110 (min 3160, mean 3309) | 0 above the Budget; 2^12:5896 2^13:103 |
| Task 2 entry | 8992 | 2950968 / 46.109 (min 783361, mean 788784) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200825 / 3.138 (min 177245, mean 194824) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 10742 / 0.168 (min 3218, mean 3467) | 0 above the Budget; 2^12:9722 2^13:257 2^14:20 |
| Task 5 entry | 7200 | 13797 / 0.216 (min 6397, mean 6652) | 0 above the Budget; 2^13:7035 2^14:165 |
| Task 6 entry | 1800 | 20117 / 0.314 (min 7348, mean 7475) | 0 above the Budget; 2^13:1768 2^14:29 2^15:3 |
| Task 0 on Resource 0 | 18000 | 7438 / 0.116 | |
| Task 0 on Resource 1 | 18000 | 4187 / 0.065 | |
| Task 2 on Resource 0 | 8992 | 773443 / 12.085 | |
| Task 2 on Resource 1 | 17984 | 4032 / 0.063 | |
| Task 5 on Resource 2 | 7200 | 5199 / 0.081 | |
| Task 6 on Resource 2 | 1800 | 4006 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2889 / 45.14 | 2748 |
| timer | 51656 | 4472 / 69.88 | 3067 |
| intake | 10000 | 2283 / 35.67 | 2114 |
| lock | 71976 | 475 / 7.42 | 473 |
| unlock | 71976 | 1839 / 28.73 | 1625 |
| job_end | 56490 | 2109 / 32.95 | 1809 |
| now | 11029962 | 238 / 3.72 | 236 |
| raise | 6000 | 2118 / 33.09 | 1992 |
| dma | 1808 | 734 / 11.47 | 511 |
| epilogue | 56490 | 1209 / 18.89 | 725 |
| enforce | 8 | 3044 / 47.56 | 2956 |
| view | 80550 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 9182, Task 1: 103, Task 2: 54542, Task 3: 2627, Task 4: 297, Task 5: 197, Task 6: 38

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3454 cycles
- Task 2 at 179940.000 ms, 3454 cycles
- Task 0 at 179950.000 ms, 3756 cycles
- Task 5 at 179950.000 ms, 3756 cycles
- Task 0 at 179960.000 ms, 3702 cycles
- Task 2 at 179960.000 ms, 3702 cycles
- Task 3 at 179960.000 ms, 3702 cycles
- Task 0 at 179970.000 ms, 3504 cycles
- Task 5 at 179975.000 ms, 3748 cycles
- Task 0 at 179980.000 ms, 3454 cycles
- Task 2 at 179980.000 ms, 3454 cycles
- Task 0 at 179990.000 ms, 3660 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 11262755 of them, the longest 15025 cycles (234.8 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 11004143 |
| 512 to 1023 (8.0 to 16.0 µs) | 75091 |
| 1024 to 2047 (16.0 to 32.0 µs) | 58668 |
| 2048 to 4095 (32.0 to 64.0 µs) | 54422 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58118 |
| 8192 to 16383 (128.0 to 256.0 µs) | 12313 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49070 | 15025 / 234.8 |
| timer | 45223 | 14684 / 229.4 |
| job_end | 11146227 | 4336 / 67.8 |
| epilogue | 22235 | 3868 / 60.4 |

## The Kernel's events

53 events mirrored (53 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 16, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 328186 | 18171 | 0.340 |
| 1 | 24295 | 6001 | 0.100 |
| 2 | 11184257 | 32008 | 12.240 |
| 3 | 1368605 | 5506 | 3.110 |
| 4 | 40783 | 9999 | 0.160 |
| 5 | 64771 | 7230 | 0.200 |
| 6 | 18333 | 1804 | 0.280 |
| none | 4967799 | 32424 | 7.270 |

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
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=1097
loop:part0:sensor:late=2984
loop:part1:spin_ms:spin=0
loop:part1:log:busy=275
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
     Finished in 7.35s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3688710 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
