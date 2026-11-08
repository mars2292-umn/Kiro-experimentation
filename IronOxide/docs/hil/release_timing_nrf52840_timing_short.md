# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 1799 of 1800 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 191 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7702 cycles (120.3 µs) against J_rel 10000 cycles |
| Kernel events | pass | 55 events mirrored (55 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 1 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 1 actions recorded (1 held, 1 of them deferrals: vectors 0x28) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 15755 cycles (246.2 µs) over 11269372 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10632 rises and 10000 response edges in the capture); latency 90 to 12460 µs, median 2460 µs, 0 later than 15 ms; Δ_stamp observed 4040 cycles (63.1 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.4 % of the run, the Kernel 63.3 % over 1046753 entries; runs per Task [18169, 6002, 31943, 5060, 10028, 7198, 1804] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22215 cycles (0.347 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 21120 cycles (0.330 ms at 100000 samples/s; 18169 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 1097 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 2990 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 275 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7702 / 120.3 | 3711 | 2^12:16577 2^13:1422 |
| 2 | 8991 | 7702 / 120.3 | 3726 | 2^12:7980 2^13:1011 |
| 3 | 4499 | 7702 / 120.3 | 3871 | 2^12:3543 2^13:956 |
| 5 | 7199 | 7178 / 112.2 | 3780 | 2^12:5884 2^13:1315 |
| 6 | 1799 | 7178 / 112.2 | 4178 | 2^12:882 2^13:917 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.839 | 0 | 0 | 0 |
| 1 | 5999 | 0.246 | 0 | 0 | 0 |
| 2 | 8992 | 54.402 | 0 | 0 | 0 |
| 3 | 4500 | 6.912 | 0 | 0 | 0 |
| 4 | 9999 | 12.463 | 10000 | 2157 | 1979 |
| 5 | 7200 | 11.369 | 0 | 0 | 0 |
| 6 | 1800 | 56.838 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22215 / 0.347 (min 9579, mean 12381) | 0 above the Budget; 2^14:17593 2^15:407 |
| Task 1 entry | 5999 | 6964 / 0.109 (min 3184, mean 3259) | 0 above the Budget; 2^12:5997 2^13:2 |
| Task 2 entry | 8992 | 2951692 / 46.120 (min 783405, mean 788721) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201133 / 3.143 (min 181561, mean 195460) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 18264 / 0.285 (min 3218, mean 3494) | 0 above the Budget; 2^12:9670 2^13:263 2^14:65 2^15:1 |
| Task 5 entry | 7200 | 13945 / 0.218 (min 6397, mean 6646) | 0 above the Budget; 2^13:7037 2^14:163 |
| Task 6 entry | 1800 | 21078 / 0.329 (min 7348, mean 9722) | 0 above the Budget; 2^13:636 2^14:1161 2^15:3 |
| Task 0 on Resource 0 | 18000 | 7438 / 0.116 | |
| Task 0 on Resource 1 | 18000 | 4163 / 0.065 | |
| Task 2 on Resource 0 | 8992 | 773917 / 12.092 | |
| Task 2 on Resource 1 | 17984 | 4076 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 4218 / 0.066 | |
| Task 6 on Resource 2 | 1800 | 4062 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56491 | 2881 / 45.02 | 2748 |
| timer | 52339 | 4452 / 69.56 | 3066 |
| intake | 10000 | 2283 / 35.67 | 2104 |
| lock | 71976 | 475 / 7.42 | 473 |
| unlock | 71976 | 1839 / 28.73 | 1623 |
| job_end | 56490 | 2060 / 32.19 | 1805 |
| now | 11036076 | 238 / 3.72 | 236 |
| raise | 6000 | 2051 / 32.05 | 1980 |
| dma | 1808 | 734 / 11.47 | 511 |
| epilogue | 56490 | 1211 / 18.92 | 717 |
| enforce | 8 | 2988 / 46.69 | 2944 |
| view | 80037 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 9029, Task 1: 2, Task 2: 54104, Task 3: 2209, Task 4: 397, Task 5: 165, Task 6: 1172

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
- Task 0 at 179990.000 ms, 3612 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 11269372 of them, the longest 15755 cycles (246.2 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 11011908 |
| 512 to 1023 (8.0 to 16.0 µs) | 72930 |
| 1024 to 2047 (16.0 to 32.0 µs) | 59262 |
| 2048 to 4095 (32.0 to 64.0 µs) | 54363 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58902 |
| 8192 to 16383 (128.0 to 256.0 µs) | 12007 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49168 | 15755 / 246.2 |
| timer | 46159 | 14720 / 230.0 |
| job_end | 11151770 | 3898 / 60.9 |
| epilogue | 22275 | 3881 / 60.6 |

## The Kernel's events

55 events mirrored (55 held, 2 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 16, release_overlap of Task 2: 18, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 326883 | 18169 | 0.330 |
| 1 | 23621 | 6002 | 0.100 |
| 2 | 11183043 | 31943 | 12.260 |
| 3 | 1370821 | 5060 | 3.120 |
| 4 | 41276 | 10028 | 0.180 |
| 5 | 64527 | 7198 | 0.210 |
| 6 | 24668 | 1804 | 0.280 |
| none | 4962096 | 32937 | 7.290 |

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
loop:part0:sensor:hold=1097
loop:part0:sensor:late=2990
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
     Finished in 7.68s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3689615 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
