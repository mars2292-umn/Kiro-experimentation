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
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 6517 cycles (101.8 µs) against J_rel 12500 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17170 cycles (268.3 µs) over 10235355 spans, closed by a timer entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10661 rises and 10000 response edges in the capture); latency 90 to 12610 µs, median 2350 µs, 0 later than 15 ms; Δ_stamp observed 4232 cycles (66.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 58.7 % over 2102979 entries; runs per Task [18132, 6000, 31962, 5505, 9998, 7252, 1801] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20423 cycles (0.319 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18132 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:373 | 10_000_000 | 994 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:411 | 50_000_000 | 2701 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 250 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 6517 / 101.8 | 3673 | 2^12:16813 2^13:1186 |
| 2 | 8991 | 6517 / 101.8 | 3696 | 2^12:8004 2^13:987 |
| 3 | 4499 | 6517 / 101.8 | 3836 | 2^12:3555 2^13:944 |
| 5 | 7199 | 6517 / 101.8 | 3734 | 2^12:6102 2^13:1097 |
| 6 | 1799 | 6517 / 101.8 | 4138 | 2^12:885 2^13:914 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.898 | 0 | 0 | 0 |
| 1 | 5999 | 0.303 | 0 | 0 | 0 |
| 2 | 8992 | 54.429 | 0 | 0 | 0 |
| 3 | 4500 | 6.983 | 0 | 0 | 0 |
| 4 | 9999 | 12.626 | 10000 | 2205 | 2031 |
| 5 | 7200 | 11.412 | 0 | 0 | 0 |
| 6 | 1800 | 56.697 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20423 / 0.319 (min 9891, mean 12639) | 0 above the Budget; 2^14:17605 2^15:395 |
| Task 1 entry | 5999 | 10671 / 0.167 (min 3327, mean 3482) | 0 above the Budget; 2^12:5881 2^13:117 2^14:1 |
| Task 2 entry | 8992 | 2946004 / 46.031 (min 783883, mean 789703) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 200571 / 3.134 (min 181313, mean 195487) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 7383 / 0.115 (min 3391, mean 3645) | 0 above the Budget; 2^12:9666 2^13:333 |
| Task 5 entry | 7200 | 15394 / 0.241 (min 6643, mean 6918) | 0 above the Budget; 2^13:6959 2^14:241 |
| Task 6 entry | 1800 | 19871 / 0.310 (min 7667, mean 7772) | 0 above the Budget; 2^13:1777 2^14:22 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7584 / 0.118 | |
| Task 0 on Resource 1 | 18000 | 4231 / 0.066 | |
| Task 2 on Resource 0 | 8992 | 773613 / 12.088 | |
| Task 2 on Resource 1 | 17984 | 6232 / 0.097 | |
| Task 5 on Resource 2 | 7200 | 5491 / 0.086 | |
| Task 6 on Resource 2 | 1800 | 3992 / 0.062 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3093 / 48.33 | 2923 |
| timer | 51532 | 4410 / 68.91 | 3022 |
| intake | 10000 | 2315 / 36.17 | 2140 |
| lock | 71976 | 462 / 7.22 | 459 |
| unlock | 71976 | 1822 / 28.47 | 1597 |
| job_end | 56490 | 2058 / 32.16 | 1786 |
| now | 10001935 | 224 / 3.50 | 224 |
| raise | 6000 | 2092 / 32.69 | 1961 |
| dma | 1808 | 722 / 11.28 | 501 |
| epilogue | 56490 | 1253 / 19.58 | 760 |
| enforce | 8 | 2938 / 45.91 | 2894 |
| view | 80521 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 8940, Task 1: 119, Task 2: 54554, Task 3: 2641, Task 4: 333, Task 5: 296, Task 6: 23

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3433 cycles
- Task 2 at 179940.000 ms, 3433 cycles
- Task 0 at 179950.000 ms, 4062 cycles
- Task 5 at 179950.000 ms, 4062 cycles
- Task 0 at 179960.000 ms, 3673 cycles
- Task 2 at 179960.000 ms, 3673 cycles
- Task 3 at 179960.000 ms, 3673 cycles
- Task 0 at 179970.000 ms, 3574 cycles
- Task 5 at 179975.000 ms, 3406 cycles
- Task 0 at 179980.000 ms, 3433 cycles
- Task 2 at 179980.000 ms, 3433 cycles
- Task 0 at 179990.000 ms, 3762 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10235355 of them, the longest 17170 cycles (268.3 µs), closed by a timer entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9977595 |
| 512 to 1023 (8.0 to 16.0 µs) | 74746 |
| 1024 to 2047 (16.0 to 32.0 µs) | 60812 |
| 2048 to 4095 (32.0 to 64.0 µs) | 54002 |
| 4096 to 8191 (64.0 to 128.0 µs) | 52428 |
| 8192 to 16383 (128.0 to 256.0 µs) | 15771 |
| 16384 to 32767 (256.0 to 512.0 µs) | 1 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 48803 | 14331 / 223.9 |
| timer | 44621 | 17170 / 268.3 |
| job_end | 10119719 | 5041 / 78.8 |
| epilogue | 22212 | 3951 / 61.7 |

## The Kernel's events

55 events mirrored (55 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 18, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 330685 | 18132 | 0.310 |
| 1 | 24804 | 6000 | 0.150 |
| 2 | 11199765 | 31962 | 12.250 |
| 3 | 1372628 | 5505 | 3.120 |
| 4 | 41666 | 9998 | 0.110 |
| 5 | 66493 | 7252 | 0.210 |
| 6 | 18825 | 1801 | 0.290 |
| none | 4941554 | 32453 | 7.270 |

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
loop:part0:sensor:hold=994
loop:part0:sensor:late=2701
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

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3682505 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
