# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

Target binary: sha256:887d246f884c4a30fbdbdda99ef4ac2dace37c826e3e81dada359232cfb9178d

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.3 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 203 s (operating duration 180 s) |
| compare arms (HW-12, ASM-19) | pass | 258849 compares left to the hardware, the closest 1 ticks ahead of the counter captured after the write (1 below 4 ticks, 23 below 8, 45 below 16); 6615 pended by software, the instant reached |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8651 cycles (135.2 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.617 ms (39456 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.300 ms (19228 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.851 ms (374476 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11950 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2196 cycles (34.3 µs), mean 1977 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2051 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 23523 cycles (367.5 µs) over 4949650 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11949 response edges inside the run (target counted 11949) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 35.5 % of the run, the Kernel 42.3 % over 1449483 entries; runs per Task [18043, 6001, 20998, 5989, 12052, 7435, 1865] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20495 cycles (0.320 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18043 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 372 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 226 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:534 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8528 / 133.2 | 3369 | 2^12:17866 2^13:127 2^14:6 |
| 2 | 8999 | 8651 / 135.2 | 3137 | 2^12:8593 2^13:405 2^14:1 |
| 3 | 4499 | 8528 / 133.2 | 3372 | 2^12:4451 2^13:47 2^14:1 |
| 5 | 7199 | 8528 / 133.2 | 3423 | 2^12:7060 2^13:137 2^14:2 |
| 6 | 1799 | 8528 / 133.2 | 3634 | 2^12:1789 2^13:9 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.617 | 0 | 0 | 0 |
| 1 | 5999 | 0.300 | 0 | 0 | 0 |
| 2 | 8999 | 5.851 | 0 | 0 | 0 |
| 3 | 3375 | 4.436 | 0 | 0 | 0 |
| 4 | 11949 | 5.914 | 11950 | 2196 | 1977 |
| 5 | 6300 | 6.490 | 0 | 0 | 0 |
| 6 | 900 | 11.835 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20495 / 0.320 (min 10075, mean 11627) | 0 above the Budget; 2^14:17801 2^15:199 |
| Task 1 entry | 5999 | 10209 / 0.160 (min 3483, mean 3621) | 0 above the Budget; 2^12:5969 2^13:2 2^14:28 |
| Task 2 entry | 8999 | 347189 / 5.425 (min 336597, mean 338595) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206483 / 3.226 (min 187827, mean 196852) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11949 | 13672 / 0.214 (min 3560, mean 3946) | 0 above the Budget; 2^12:10841 2^13:1030 2^14:78 |
| Task 5 entry | 6300 | 36371 / 0.568 (min 21383, mean 27818) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 40000 / 0.625 (min 21402, mean 30593) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10425 / 0.163 | |
| Task 0 on Resource 1 | 18000 | 10498 / 0.164 | |
| Task 2 on Resource 0 | 8999 | 328640 / 5.135 | |
| Task 2 on Resource 1 | 17998 | 6992 / 0.109 | |
| Task 5 on Resource 2 | 12600 | 26254 / 0.410 | |
| Task 6 on Resource 2 | 900 | 4005 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58447 | 3094 / 48.34 | 2869 |
| timer | 57047 | 4958 / 77.47 | 2784 |
| intake | 11950 | 2330 / 36.41 | 2098 |
| lock | 77397 | 450 / 7.03 | 448 |
| unlock | 76497 | 1729 / 27.02 | 1528 |
| job_end | 55522 | 1982 / 30.97 | 1678 |
| now | 4690941 | 228 / 3.56 | 225 |
| raise | 6000 | 1933 / 30.20 | 1871 |
| dma | 8999 | 729 / 11.39 | 552 |
| epilogue | 58447 | 1271 / 19.86 | 632 |
| restart | 900 | 4958 / 77.47 | 4769 |
| enforce | 2925 | 4958 / 77.47 | 3574 |
| view | 72333 | 327 / 5.11 | 292 |

Entries that interrupted an executing Job: Task 0: 2767, Task 1: 58, Task 2: 29719, Task 3: 6426, Task 4: 1232, Task 5: 3764, Task 6: 2684

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 7750 cycles; Task 1: 2 (timer, intake), 4972 cycles; Task 2: 5 (dispatch, timer, intake), 13523 cycles; Task 3: 3 (dispatch, timer, intake), 7916 cycles; Task 4: 3 (dispatch, timer), 8867 cycles; Task 5: 3 (dispatch, timer, intake), 8171 cycles; Task 6: 4 (dispatch, timer, intake), 10557 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3911 cycles
- Task 0 at 179940.000 ms, 2984 cycles
- Task 2 at 179945.000 ms, 2976 cycles
- Task 0 at 179950.000 ms, 3827 cycles
- Task 5 at 179950.000 ms, 3827 cycles
- Task 0 at 179960.000 ms, 4768 cycles
- Task 3 at 179960.000 ms, 4768 cycles
- Task 2 at 179965.000 ms, 6148 cycles
- Task 0 at 179970.000 ms, 3343 cycles
- Task 5 at 179975.000 ms, 5007 cycles
- Task 0 at 179980.000 ms, 2984 cycles
- Task 0 at 179990.000 ms, 4608 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 4949650 of them, the longest 23523 cycles (367.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 512 to 1023 (8.0 to 16.0 µs) | 4763033 |
| 1024 to 2047 (16.0 to 32.0 µs) | 65844 |
| 2048 to 4095 (32.0 to 64.0 µs) | 43952 |
| 4096 to 8191 (64.0 to 128.0 µs) | 62992 |
| 8192 to 16383 (128.0 to 256.0 µs) | 13711 |
| 16384 to 32767 (256.0 to 512.0 µs) | 118 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56544 | 23523 / 367.5 |
| timer | 25041 | 18944 / 296.0 |
| job_end | 4834878 | 4447 / 69.5 |
| epilogue | 33187 | 12515 / 195.5 |

The longest span's 8 entries, in order (cycles from the span's start; the gap is the time since the entry before ended: stubs, unwinding, exception entry):

| # | Entry | Begins | Ends | Rust half (cycles / µs) | Gap before (cycles / µs) |
|---|---|---|---|---|---|
| 1 | now, exception 24 pending | 0 | 520 | 520 / 8.1 | 0 / 0.0 |
| 2 | timer, enforced an Overrun, an unwind follows, exception 24 pending | 720 | 4664 | 3944 / 61.6 | 200 / 3.1 |
| 3 | epilogue, exception 24 pending | 4861 | 5548 | 687 / 10.7 | 197 / 3.1 |
| 4 | timer, exception 22 pending | 5779 | 8767 | 2988 / 46.7 | 231 / 3.6 |
| 5 | intake, exception 40 pending | 9001 | 11563 | 2562 / 40.0 | 234 / 3.7 |
| 6 | dispatch, started a Job, exception 24 pending | 11797 | 15131 | 3334 / 52.1 | 234 / 3.7 |
| 7 | timer, exception 38 pending | 15354 | 19713 | 4359 / 68.1 | 223 / 3.5 |
| 8 | dispatch, started a Job | 19953 | 23523 | 3570 / 55.8 | 240 / 3.8 |

Rust halves 21964 cycles (343.2 µs), between them 1559 cycles (24.4 µs).

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 300714 | 18043 | 0.310 |
| 1 | 24910 | 6001 | 0.150 |
| 2 | 4813024 | 20998 | 5.300 |
| 3 | 1549903 | 5989 | 4.640 |
| 4 | 54934 | 12052 | 0.160 |
| 5 | 340987 | 7435 | 0.860 |
| 6 | 124617 | 1865 | 0.960 |
| none | 13084543 | 44562 | 14753.920 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 272000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress_short/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=18000
sensor=8999
log=3375
pulse=11949
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=372
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=226
loop:part2:filter:hold=0
loop:part2:spin_ms:spin=0
dma_end=0
dma_after_stop=0
spim0_enable=7
spim0_shorts=0
spim0_rxd_maxcnt=64
spim0_txd_maxcnt=64
spim0_events_end=1
spim0_events_stopped=1
first_memmanage_cfsr=0
first_memmanage_mmfar=0
events=32
event=6
  task=6
  at_ms=179301
  detail=58545
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179324
  detail=289052
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179375
  detail=51848
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179484
  detail=289040
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=58457
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179575
  detail=51848
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179644
  detail=289070
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179701
  detail=58509
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179775
  detail=51836
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179811
  detail=289078
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=58545
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=179901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=179964
  detail=289040
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=52596
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179975
  detail=1
  prs_lo=128
  prs_hi=0
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
expected_sensor=9000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 8.11s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:887d246f884c4a30fbdbdda99ef4ac2dace37c826e3e81dada359232cfb9178d) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:887d246f884c4a30fbdbdda99ef4ac2dace37c826e3e81dada359232cfb9178d is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 563 cycles (measured, rsk-hil release-timing record, 77397 runs, max 450 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2162 cycles (measured, rsk-hil release-timing record, 76497 runs, max 1729 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3868 cycles (measured, rsk-hil release-timing record, 58447 runs, max 3094 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4067 cycles (measured, rsk-hil release-timing record, 55522 runs, max 3253 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6198 cycles (measured, rsk-hil release-timing record, 57047 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2745 cycles (measured, rsk-hil release-timing record, 11950 runs, max 2196 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 29404 cycles (measured, rsk-hil release-timing record, 4949650 runs, max 23523 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 23523 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 56544 (longest 23523), timer 25041 (longest 18944), job_end 4834878 (longest 4447), epilogue 33187 (longest 12515); the longest single Rust half 4958) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6198 cycles (measured, rsk-hil release-timing record, 57047 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6198 cycles (measured, rsk-hil release-timing record, 2925 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 10814 cycles (measured, rsk-hil release-timing record, 40495 runs, max 8651 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 285 cycles (measured, rsk-hil release-timing record, 4690941 runs, max 228 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 912 cycles (measured, rsk-hil release-timing record, 8999 runs, max 729 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 409 cycles (measured, rsk-hil release-timing record, 72333 runs, max 327 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6198 cycles (measured, rsk-hil release-timing record, 900 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.363 | 10.000 | 0.617 | 18000 | within |
| `part0.echo` | A | 11.785 | 20.000 | 0.300 | 5999 | within |
| `part0.sensor` | A | 14.154 | 20.000 | 5.851 | 8999 | within |
| `part1.log` | C | 40.393 | 40.000 | 4.436 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.033 | 15.000 | 5.914 | 11949 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 17.947 | 25.000 | 6.490 | 6300 | within |
| `part2.sampler` | B | 99.872 | 100.000 | 11.835 | 900 | within |

Verdict: **pass**
