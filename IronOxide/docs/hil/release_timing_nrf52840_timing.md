# HIL release-timing run (task 7.3, R9.10)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 36599 of 36600 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x20000, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 0 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 3 below the bound with the justification recorded, 0 below without one, 4 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3672 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823317 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9501 cycles (148.5 µs) against J_rel 12500 cycles |
| Kernel events | pass | 1123 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 2: 81, deadline_miss of Task 2: 174, release_overlap of Task 2: 174, mit_violation of Task 2: 81, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 2 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2 actions recorded (2 held, 2 of them deferrals: vectors 0x24) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 19799 cycles (309.4 µs) over 208167671 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 203332 stimulus pulses answered (target counted 203332; 203991 rises and 203334 response edges in the capture); latency 90 to 12620 µs, median 2450 µs, 0 later than 15 ms; Δ_stamp observed 6740 cycles (105.3 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 58.7 % over 42773396 entries; runs per Task [368842, 122044, 647797, 106936, 204105, 147330, 36875] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22963 cycles (0.359 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 368842 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 994 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2715 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 250 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9501 / 148.5 | 3685 | 2^12:337145 2^13:28849 2^14:5 |
| 2 | 182821 | 9501 / 148.5 | 3698 | 2^12:162630 2^13:20188 2^14:3 |
| 3 | 91499 | 9501 / 148.5 | 3838 | 2^12:72238 2^13:19257 2^14:4 |
| 5 | 146399 | 8713 / 136.1 | 3748 | 2^12:119525 2^13:26873 2^14:1 |
| 6 | 36599 | 8713 / 136.1 | 4143 | 2^12:17990 2^13:18608 2^14:1 |

Releases recorded: 823317; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 9.958 | 0 | 0 | 0 |
| 1 | 121999 | 0.307 | 0 | 0 | 0 |
| 2 | 182822 | 54.514 | 0 | 0 | 0 |
| 3 | 91500 | 7.043 | 0 | 0 | 0 |
| 4 | 203332 | 12.633 | 203333 | 2245 | 2028 |
| 5 | 146400 | 11.474 | 0 | 0 | 0 |
| 6 | 36600 | 56.974 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 22963 / 0.359 (min 9815, mean 12616) | 0 above the Budget; 2^14:360850 2^15:5150 |
| Task 1 entry | 121999 | 10607 / 0.166 (min 3327, mean 3444) | 0 above the Budget; 2^12:121180 2^13:818 2^14:1 |
| Task 2 entry | 182822 | 2950532 / 46.102 (min 783675, mean 789694) | 178 above the Budget; 2^15:182822 |
| Task 3 entry | 91500 | 205414 / 3.210 (min 181281, mean 195920) | 0 above the Budget; 2^15:91500 |
| Task 4 entry | 203332 | 14338 / 0.224 (min 3391, mean 3686) | 0 above the Budget; 2^12:194782 2^13:7900 2^14:650 |
| Task 5 entry | 146400 | 15298 / 0.239 (min 6643, mean 6892) | 0 above the Budget; 2^13:142546 2^14:3854 |
| Task 6 entry | 36600 | 22660 / 0.354 (min 7667, mean 7877) | 0 above the Budget; 2^13:35121 2^14:1467 2^15:12 |
| Task 0 on Resource 0 | 366000 | 7892 / 0.123 | |
| Task 0 on Resource 1 | 366000 | 7705 / 0.120 | |
| Task 2 on Resource 0 | 182822 | 773721 / 12.089 | |
| Task 2 on Resource 1 | 365644 | 6546 / 0.102 | |
| Task 5 on Resource 2 | 146400 | 5567 / 0.087 | |
| Task 6 on Resource 2 | 36600 | 7840 / 0.122 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1148657 | 3117 / 48.70 | 2923 |
| timer | 1039895 | 4494 / 70.22 | 3022 |
| intake | 203333 | 2355 / 36.80 | 2137 |
| lock | 1463466 | 478 / 7.47 | 459 |
| unlock | 1463466 | 1822 / 28.47 | 1595 |
| job_end | 1148653 | 2084 / 32.56 | 1784 |
| now | 203430775 | 225 / 3.52 | 224 |
| raise | 122000 | 2092 / 32.69 | 1956 |
| dma | 36608 | 722 / 11.28 | 501 |
| epilogue | 1148653 | 1253 / 19.58 | 756 |
| enforce | 178 | 3038 / 47.47 | 2898 |
| view | 1630971 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 180195, Task 1: 820, Task 2: 1098981, Task 3: 49167, Task 4: 9376, Task 5: 4859, Task 6: 1759

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659940.000 ms, 3433 cycles
- Task 2 at 3659940.000 ms, 3433 cycles
- Task 0 at 3659950.000 ms, 3690 cycles
- Task 5 at 3659950.000 ms, 3690 cycles
- Task 0 at 3659960.000 ms, 3673 cycles
- Task 2 at 3659960.000 ms, 3673 cycles
- Task 3 at 3659960.000 ms, 3673 cycles
- Task 0 at 3659970.000 ms, 3358 cycles
- Task 5 at 3659975.000 ms, 3658 cycles
- Task 0 at 3659980.000 ms, 3433 cycles
- Task 2 at 3659980.000 ms, 3433 cycles
- Task 0 at 3659990.000 ms, 3358 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 208167671 of them, the longest 19799 cycles (309.4 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 202921536 |
| 512 to 1023 (8.0 to 16.0 µs) | 1521389 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1237150 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1089344 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1086376 |
| 8192 to 16383 (128.0 to 256.0 µs) | 311670 |
| 16384 to 32767 (256.0 to 512.0 µs) | 206 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 993994 | 19799 / 309.4 |
| timer | 911625 | 18414 / 287.7 |
| job_end | 205811160 | 6213 / 97.1 |
| epilogue | 450892 | 6363 / 99.4 |

## The Kernel's events

1123 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 2: 81, deadline_miss of Task 2: 174, release_overlap of Task 2: 174, mit_violation of Task 2: 81, operating_end of Task none: 1, safe_state of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6709848 | 368842 | 0.350 |
| 1 | 494945 | 122044 | 0.150 |
| 2 | 227733053 | 647797 | 12.290 |
| 3 | 27951297 | 106936 | 3.170 |
| 4 | 864789 | 204105 | 0.200 |
| 5 | 1344795 | 147330 | 0.220 |
| 6 | 389583 | 36875 | 0.210 |
| none | 100505585 | 666328 | 7.280 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/release-timing/nrf52840_timing/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182822
log=91500
pulse=203332
echo=121999
blocked_releases=182843
filter=146400
filter_state_bits=1120403452
sampler=36600
dma_ok=36600
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=994
loop:part0:sensor:late=2715
loop:part1:spin_ms:spin=0
loop:part1:log:busy=250
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=36599
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
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
event=18
  task=255
  at_ms=3660000
  detail=0
  prs_lo=0
  prs_hi=0
expected_control=366000
expected_sensor=182822
expected_late_jobs=178
late_events_seen=30
RSK-OK
Firmware exited successfully
     Finished in 8.04s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_timing.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 2. 

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3688165 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
