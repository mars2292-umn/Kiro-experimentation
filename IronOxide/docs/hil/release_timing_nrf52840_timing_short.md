# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8546 cycles (133.5 µs) against J_rel 12500 cycles |
| Kernel events | pass | 57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17798 cycles (278.1 µs) over 10500890 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10599 rises and 10000 response edges in the capture); latency 100 to 12640 µs, median 2330 µs, 0 later than 15 ms; Δ_stamp observed 6072 cycles (94.9 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.7 % of the run, the Kernel 59.3 % over 1831313 entries; runs per Task [18143, 6000, 32041, 5005, 10243, 7204, 1803] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23339 cycles (0.365 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 18143 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 1021 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2778 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 256 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8546 / 133.5 | 3774 | 2^12:16300 2^13:1698 2^14:1 |
| 2 | 8991 | 8546 / 133.5 | 3795 | 2^12:7833 2^13:1157 2^14:1 |
| 3 | 4499 | 8546 / 133.5 | 3934 | 2^12:3478 2^13:1020 2^14:1 |
| 5 | 7199 | 7737 / 120.9 | 3851 | 2^12:5766 2^13:1433 |
| 6 | 1799 | 7737 / 120.9 | 4229 | 2^12:864 2^13:935 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.962 | 0 | 0 | 0 |
| 1 | 5999 | 0.304 | 0 | 0 | 0 |
| 2 | 8992 | 54.513 | 0 | 0 | 0 |
| 3 | 4500 | 7.023 | 0 | 0 | 0 |
| 4 | 9999 | 12.651 | 10000 | 2328 | 2146 |
| 5 | 7200 | 11.485 | 0 | 0 | 0 |
| 6 | 1800 | 56.898 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 23339 / 0.365 (min 10059, mean 13003) | 0 above the Budget; 2^14:15539 2^15:2461 |
| Task 1 entry | 5999 | 7341 / 0.115 (min 3470, mean 3611) | 0 above the Budget; 2^12:5880 2^13:119 |
| Task 2 entry | 8992 | 2945068 / 46.017 (min 784096, mean 789706) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200142 / 3.127 (min 180869, mean 195848) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 13754 / 0.215 (min 3466, mean 3988) | 0 above the Budget; 2^12:9146 2^13:730 2^14:123 |
| Task 5 entry | 7200 | 14136 / 0.221 (min 6808, mean 7179) | 0 above the Budget; 2^13:6753 2^14:447 |
| Task 6 entry | 1800 | 19926 / 0.311 (min 7818, mean 7966) | 0 above the Budget; 2^13:1756 2^14:42 2^15:2 |
| Task 0 on Resource 0 | 18000 | 4308 / 0.067 | |
| Task 0 on Resource 1 | 18000 | 4265 / 0.067 | |
| Task 2 on Resource 0 | 8992 | 773698 / 12.089 | |
| Task 2 on Resource 1 | 17984 | 6348 / 0.099 | |
| Task 5 on Resource 2 | 7200 | 7768 / 0.121 | |
| Task 6 on Resource 2 | 1800 | 7086 / 0.111 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3233 / 50.52 | 3037 |
| timer | 51599 | 4460 / 69.69 | 3110 |
| intake | 10000 | 2438 / 38.09 | 2254 |
| lock | 71976 | 473 / 7.39 | 453 |
| unlock | 71976 | 1904 / 29.75 | 1715 |
| job_end | 56490 | 2144 / 33.50 | 1883 |
| now | 10267086 | 221 / 3.45 | 221 |
| raise | 6000 | 2147 / 33.55 | 2057 |
| dma | 1808 | 715 / 11.17 | 495 |
| epilogue | 56490 | 1253 / 19.58 | 757 |
| enforce | 8 | 3060 / 47.81 | 3004 |
| view | 80298 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 8979, Task 1: 119, Task 2: 53860, Task 3: 2151, Task 4: 1098, Task 5: 454, Task 6: 48

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3548 cycles
- Task 2 at 179940.000 ms, 3548 cycles
- Task 0 at 179950.000 ms, 3837 cycles
- Task 5 at 179950.000 ms, 3837 cycles
- Task 0 at 179960.000 ms, 3732 cycles
- Task 2 at 179960.000 ms, 3732 cycles
- Task 3 at 179960.000 ms, 3732 cycles
- Task 0 at 179970.000 ms, 3365 cycles
- Task 5 at 179975.000 ms, 3789 cycles
- Task 0 at 179980.000 ms, 3496 cycles
- Task 2 at 179980.000 ms, 3496 cycles
- Task 0 at 179990.000 ms, 3761 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10500890 of them, the longest 17798 cycles (278.1 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 10244392 |
| 512 to 1023 (8.0 to 16.0 µs) | 73564 |
| 1024 to 2047 (16.0 to 32.0 µs) | 41955 |
| 2048 to 4095 (32.0 to 64.0 µs) | 67771 |
| 4096 to 8191 (64.0 to 128.0 µs) | 57414 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15792 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48043 | 17798 / 278.1 |
| timer | 45641 | 15093 / 235.8 |
| job_end | 10385382 | 5122 / 80.0 |
| epilogue | 21824 | 4057 / 63.4 |

## The Kernel's events

57 events mirrored (57 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 19, release_overlap of Task 2: 19, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 340458 | 18143 | 0.350 |
| 1 | 25675 | 6000 | 0.110 |
| 2 | 11203455 | 32041 | 12.250 |
| 3 | 1372361 | 5005 | 3.120 |
| 4 | 48607 | 10243 | 0.160 |
| 5 | 69358 | 7204 | 0.210 |
| 6 | 19309 | 1803 | 0.290 |
| none | 4917072 | 32676 | 7.240 |

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
loop:part0:sensor:late=2778
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
     Finished in 7.71s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3681335 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
