# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8567 cycles (133.9 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.604 ms (38656 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.290 ms (18544 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.841 ms (373856 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11949 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2149 cycles (33.6 µs), mean 1938 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2010 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 19880 cycles (310.6 µs) over 5408719 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11948 response edges inside the run (target counted 11948) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 34.9 % of the run, the Kernel 42.0 % over 1175431 entries; runs per Task [18030, 6001, 20999, 5852, 12069, 7408, 1858] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20204 cycles (0.316 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19200 cycles (0.300 ms at 100000 samples/s; 18030 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 409 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 250 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8567 / 133.9 | 3516 | 2^12:17816 2^13:177 2^14:6 |
| 2 | 8999 | 8306 / 129.8 | 3263 | 2^12:8825 2^13:173 2^14:1 |
| 3 | 4499 | 7047 / 110.1 | 3574 | 2^12:4381 2^13:118 |
| 5 | 7199 | 8567 / 133.9 | 3588 | 2^12:7072 2^13:125 2^14:2 |
| 6 | 1799 | 7047 / 110.1 | 3862 | 2^12:1770 2^13:29 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.604 | 0 | 0 | 0 |
| 1 | 5999 | 0.290 | 0 | 0 | 0 |
| 2 | 8999 | 5.841 | 0 | 0 | 0 |
| 3 | 3375 | 4.314 | 0 | 0 | 0 |
| 4 | 11948 | 5.874 | 11949 | 2149 | 1938 |
| 5 | 6300 | 6.455 | 0 | 0 | 0 |
| 6 | 900 | 11.716 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20204 / 0.316 (min 9601, mean 11133) | 0 above the Budget; 2^14:17793 2^15:207 |
| Task 1 entry | 5999 | 9706 / 0.152 (min 3315, mean 3407) | 0 above the Budget; 2^12:5988 2^13:5 2^14:6 |
| Task 2 entry | 8999 | 347650 / 5.432 (min 335684, mean 337782) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206181 / 3.222 (min 188449, mean 196748) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11948 | 13502 / 0.211 (min 3377, mean 3777) | 0 above the Budget; 2^12:10830 2^13:1023 2^14:95 |
| Task 5 entry | 6300 | 38769 / 0.606 (min 21149, mean 27188) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 37233 / 0.582 (min 19781, mean 28165) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10497 / 0.164 | |
| Task 0 on Resource 1 | 18000 | 10466 / 0.164 | |
| Task 2 on Resource 0 | 8999 | 328227 / 5.129 | |
| Task 2 on Resource 1 | 17998 | 6857 / 0.107 | |
| Task 5 on Resource 2 | 12600 | 25861 / 0.404 | |
| Task 6 on Resource 2 | 900 | 8792 / 0.137 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58446 | 3078 / 48.09 | 2852 |
| timer | 57047 | 4683 / 73.17 | 2957 |
| intake | 11949 | 2260 / 35.31 | 2041 |
| lock | 77397 | 457 / 7.14 | 454 |
| unlock | 76497 | 1711 / 26.73 | 1515 |
| job_end | 55521 | 1913 / 29.89 | 1654 |
| now | 5154986 | 229 / 3.58 | 228 |
| raise | 6000 | 1946 / 30.41 | 1872 |
| dma | 8999 | 710 / 11.09 | 542 |
| epilogue | 58446 | 1252 / 19.56 | 612 |
| restart | 900 | 4683 / 73.17 | 4602 |
| enforce | 2925 | 4683 / 73.17 | 3653 |
| view | 72180 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 2773, Task 1: 17, Task 2: 29791, Task 3: 6353, Task 4: 1252, Task 5: 3653, Task 6: 2664

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3403 cycles
- Task 0 at 179940.000 ms, 3154 cycles
- Task 2 at 179945.000 ms, 3158 cycles
- Task 0 at 179950.000 ms, 3839 cycles
- Task 5 at 179950.000 ms, 3839 cycles
- Task 0 at 179960.000 ms, 3386 cycles
- Task 3 at 179960.000 ms, 3386 cycles
- Task 2 at 179965.000 ms, 3634 cycles
- Task 0 at 179970.000 ms, 3431 cycles
- Task 5 at 179975.000 ms, 3158 cycles
- Task 0 at 179980.000 ms, 3154 cycles
- Task 0 at 179990.000 ms, 3074 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5408719 of them, the longest 19880 cycles (310.6 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5134048 |
| 512 to 1023 (8.0 to 16.0 µs) | 89665 |
| 1024 to 2047 (16.0 to 32.0 µs) | 65215 |
| 2048 to 4095 (32.0 to 64.0 µs) | 41253 |
| 4096 to 8191 (64.0 to 128.0 µs) | 64056 |
| 8192 to 16383 (128.0 to 256.0 µs) | 14319 |
| 16384 to 32767 (256.0 to 512.0 µs) | 163 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56402 | 19880 / 310.6 |
| timer | 24324 | 19730 / 308.3 |
| job_end | 5294933 | 4024 / 62.9 |
| epilogue | 33060 | 10670 / 166.7 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 285029 | 18030 | 0.300 |
| 1 | 22890 | 6001 | 0.140 |
| 2 | 4797782 | 20999 | 5.290 |
| 3 | 1547825 | 5852 | 4.570 |
| 4 | 51434 | 12069 | 0.150 |
| 5 | 334068 | 7408 | 0.860 |
| 6 | 112223 | 1858 | 0.900 |
| none | 13321581 | 44713 | 16525.560 |

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
pulse=11948
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=409
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=250
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
  detail=52040
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
  detail=289184
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
  detail=52092
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
  at_ms=179485
  detail=288372
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179485
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=52016
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
  detail=52092
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
  detail=288860
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
  detail=52088
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
  detail=52024
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
  detail=289125
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
  detail=52040
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
  detail=289304
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
  detail=52092
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
     Finished in 8.06s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:9c914eba3ab2ef81c01338158ff040f52929b7bd88d7c5c7015f88cd5ba28ed1) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:95476a3e75d1025f107b5332e8908b4263fb251c92f55318c40566c2d78f0b02 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 572 cycles (measured, rsk-hil release-timing record, 317 runs, max 457 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2217 cycles (measured, rsk-hil release-timing record, 397 runs, max 1773 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3848 cycles (measured, rsk-hil release-timing record, 58446 runs, max 3078 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 3998 cycles (measured, rsk-hil release-timing record, 448 runs, max 3198 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5854 cycles (measured, rsk-hil release-timing record, 57047 runs, max 4683 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2687 cycles (measured, rsk-hil release-timing record, 11949 runs, max 2149 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 24850 cycles (measured, rsk-hil release-timing record, 5408719 runs, max 19880 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 19880 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 56402 (longest 19880), timer 24324 (longest 19730), job_end 5294933 (longest 4024), epilogue 33060 (longest 10670); the longest single Rust half 4683) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 98 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2175, less Δ_entry of the plain cycle configuration, 2121, is 54, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 98 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 98 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2175, less Δ_entry of the plain cycle configuration, 2121, is 54, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 98 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5854 cycles (measured, rsk-hil release-timing record, 57047 runs, max 4683 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 5854 cycles (measured, rsk-hil release-timing record, 2925 runs, max 4683 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 10709 cycles (measured, rsk-hil release-timing record, 40495 runs, max 8567 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 287 cycles (measured, rsk-hil release-timing record, 5154986 runs, max 229 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 893 cycles (measured, rsk-hil release-timing record, 20 runs, max 714 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 565 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 5854 cycles (measured, rsk-hil release-timing record, 900 runs, max 4683 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2652 cycles (measured, rsk-hil release-timing record, 118 runs, max 2121 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 118 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.210 | 10.000 | 0.604 | 18000 | within |
| `part0.echo` | A | 11.652 | 20.000 | 0.290 | 5999 | within |
| `part0.sensor` | A | 13.971 | 20.000 | 5.841 | 8999 | within |
| `part1.log` | C | 40.248 | 40.000 | 4.314 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 14.955 | 15.000 | 5.874 | 11948 | within |
| `part2.filter` | B | 17.851 | 25.000 | 6.455 | 6300 | within |
| `part2.sampler` | B | 99.634 | 100.000 | 11.716 | 900 | within |

Verdict: **pass**
