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
| release timing (R9.10) | pass | 40487 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 7699 cycles (120.3 µs) against J_rel 12500 cycles |
| Kernel events | pass | 53 events mirrored (53 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 17, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 0 actions recorded (0 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 17606 cycles (275.1 µs) over 10186819 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| loopback over the run | pass | 9999 stimulus pulses answered (target counted 9999; 10641 rises and 10000 response edges in the capture); latency 90 to 12180 µs, median 2330 µs, 0 later than 15 ms; Δ_stamp observed 6028 cycles (94.2 µs) against the exported bound 8000 cycles; 0 unanswered inside the run |
| dispatch timeline | pass | Tasks executing 72.5 % of the run, the Kernel 58.5 % over 2155938 entries; runs per Task [18130, 6001, 31713, 5047, 10097, 7245, 1827] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 19772 cycles (0.309 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18130 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 989 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 2690 | unreachable: the injected late Job's 38 ms by the time base; the 50,000,000-iteration guard is the PR-16 backstop |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 248 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 7699 / 120.3 | 3690 | 2^12:16535 2^13:1464 |
| 2 | 8991 | 7699 / 120.3 | 3710 | 2^12:7930 2^13:1061 |
| 3 | 4499 | 7699 / 120.3 | 3849 | 2^12:3521 2^13:978 |
| 5 | 7199 | 7699 / 120.3 | 3760 | 2^12:5916 2^13:1283 |
| 6 | 1799 | 7699 / 120.3 | 4156 | 2^12:879 2^13:920 |

Releases recorded: 40487; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 9.914 | 0 | 0 | 0 |
| 1 | 5999 | 0.249 | 0 | 0 | 0 |
| 2 | 8992 | 54.560 | 0 | 0 | 0 |
| 3 | 4500 | 6.960 | 0 | 0 | 0 |
| 4 | 9999 | 12.211 | 10000 | 2160 | 1979 |
| 5 | 7200 | 11.396 | 0 | 0 | 0 |
| 6 | 1800 | 56.725 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 19772 / 0.309 (min 9788, mean 12533) | 0 above the Budget; 2^14:17806 2^15:194 |
| Task 1 entry | 5999 | 6887 / 0.108 (min 3315, mean 3410) | 0 above the Budget; 2^12:5996 2^13:3 |
| Task 2 entry | 8992 | 2944780 / 46.012 (min 783680, mean 789186) | 8 above the Budget; 2^15:8992 |
| Task 3 entry | 4500 | 201181 / 3.143 (min 181348, mean 195927) | 0 above the Budget; 2^15:4500 |
| Task 4 entry | 9999 | 14025 / 0.219 (min 3373, mean 3654) | 0 above the Budget; 2^12:9680 2^13:219 2^14:100 |
| Task 5 entry | 7200 | 15180 / 0.237 (min 6581, mean 6870) | 0 above the Budget; 2^13:6944 2^14:256 |
| Task 6 entry | 1800 | 19765 / 0.309 (min 7605, mean 7813) | 0 above the Budget; 2^13:1733 2^14:66 2^15:1 |
| Task 0 on Resource 0 | 18000 | 7985 / 0.125 | |
| Task 0 on Resource 1 | 18000 | 4126 / 0.064 | |
| Task 2 on Resource 0 | 8992 | 773618 / 12.088 | |
| Task 2 on Resource 1 | 17984 | 4125 / 0.064 | |
| Task 5 on Resource 2 | 7200 | 5383 / 0.084 | |
| Task 6 on Resource 2 | 1800 | 5268 / 0.082 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 56490 | 3082 / 48.16 | 2899 |
| timer | 50772 | 4413 / 68.95 | 3021 |
| intake | 10000 | 2273 / 35.52 | 2091 |
| lock | 71976 | 457 / 7.14 | 454 |
| unlock | 71976 | 1781 / 27.83 | 1548 |
| job_end | 56490 | 2008 / 31.38 | 1730 |
| now | 9953523 | 229 / 3.58 | 228 |
| raise | 6000 | 2061 / 32.20 | 1919 |
| dma | 1808 | 710 / 11.09 | 500 |
| epilogue | 56490 | 1249 / 19.52 | 750 |
| enforce | 8 | 2945 / 46.02 | 2896 |
| view | 79935 | 329 / 5.14 | 292 |

Entries that interrupted an executing Job: Task 0: 9020, Task 1: 3, Task 2: 53550, Task 3: 2201, Task 4: 432, Task 5: 315, Task 6: 94

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179940.000 ms, 3426 cycles
- Task 2 at 179940.000 ms, 3426 cycles
- Task 0 at 179950.000 ms, 3947 cycles
- Task 5 at 179950.000 ms, 3947 cycles
- Task 0 at 179960.000 ms, 3658 cycles
- Task 2 at 179960.000 ms, 3658 cycles
- Task 3 at 179960.000 ms, 3658 cycles
- Task 0 at 179970.000 ms, 3943 cycles
- Task 5 at 179975.000 ms, 3743 cycles
- Task 0 at 179980.000 ms, 3426 cycles
- Task 2 at 179980.000 ms, 3426 cycles
- Task 0 at 179990.000 ms, 3559 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 10186819 of them, the longest 17606 cycles (275.1 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 9928205 |
| 512 to 1023 (8.0 to 16.0 µs) | 75762 |
| 1024 to 2047 (16.0 to 32.0 µs) | 61124 |
| 2048 to 4095 (32.0 to 64.0 µs) | 53106 |
| 4096 to 8191 (64.0 to 128.0 µs) | 54578 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14033 |
| 16384 to 32767 (256.0 to 512.0 µs) | 11 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49004 | 17606 / 275.1 |
| timer | 44953 | 14534 / 227.1 |
| job_end | 10071128 | 4175 / 65.2 |
| epilogue | 21734 | 3850 / 60.2 |

## The Kernel's events

53 events mirrored (53 held, 0 of them the post-detach reboot's): overrun of Task 2: 8, deadline_miss of Task 2: 17, release_overlap of Task 2: 17, mit_violation of Task 2: 8, operating_end of Task none: 1, safe_state of Task none: 1, debugger of Task none: 1; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 326680 | 18130 | 0.300 |
| 1 | 23811 | 6001 | 0.100 |
| 2 | 11189999 | 31713 | 12.240 |
| 3 | 1372987 | 5047 | 3.110 |
| 4 | 41943 | 10097 | 0.160 |
| 5 | 65458 | 7245 | 0.210 |
| 6 | 18914 | 1827 | 0.290 |
| none | 4956709 | 33039 | 7.230 |

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
blocked_releases=8993
filter=7200
filter_state_bits=1120403452
sampler=1800
dma_ok=1800
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=989
loop:part0:sensor:late=2690
loop:part1:spin_ms:spin=0
loop:part1:log:busy=248
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

Not judged, as the configuration intends: [R28.4] the WCET of Task `part0.sensor` (3680975 cycles) exceeds its Budget (832000 cycles) (the injected overruns of the release-timing configuration, Verification_Plan 11.6; the schedulability claim is the plain and stress configurations' of 11.14).
