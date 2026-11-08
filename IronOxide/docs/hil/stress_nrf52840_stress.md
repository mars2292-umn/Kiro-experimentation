# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.2 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3684 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9298 cycles (145.3 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 1.711 ms (109492 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 121999 Jobs, response at most 0.344 ms (22036 cycles) against WCRT 11.364 ms (727300 cycles); part0.sensor: 182999 Jobs, response at most 6.912 ms (442388 cycles) against WCRT 11.512 ms (736800 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242949 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2189 cycles (34.2 µs), mean 1961 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2033 % of the run |
| Kernel events | pass | 137254 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54900 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 26248 cycles (410.1 µs) over 137780666 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 242948 response edges inside the run (target counted 242948) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 44.6 % of the run, the Kernel 41.3 % over 13625511 entries; runs per Task [366932, 122031, 427013, 119100, 244912, 150559, 37876] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 27383 cycles (0.428 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 26880 cycles (0.420 ms at 100000 samples/s; 366932 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:310 | 10_000_000 | 546 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:348 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:179 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:280 | 10_000_000 | 275 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9298 / 145.3 | 3606 | 2^12:361762 2^13:3892 2^14:345 |
| 2 | 182999 | 8618 / 134.7 | 3264 | 2^12:180504 2^13:2379 2^14:116 |
| 3 | 91499 | 9298 / 145.3 | 3602 | 2^12:89568 2^13:1921 2^14:10 |
| 5 | 146399 | 9298 / 145.3 | 3633 | 2^12:143528 2^13:2747 2^14:124 |
| 6 | 36599 | 9298 / 145.3 | 3892 | 2^12:36030 2^13:563 2^14:6 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 1.711 | 0 | 0 | 0 |
| 1 | 121999 | 0.344 | 0 | 0 | 0 |
| 2 | 182999 | 6.912 | 0 | 0 | 0 |
| 3 | 68625 | 4.286 | 0 | 0 | 0 |
| 4 | 242948 | 7.022 | 242949 | 2189 | 1961 |
| 5 | 128100 | 7.537 | 0 | 0 | 0 |
| 6 | 18300 | 12.786 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 27383 / 0.428 (min 9391, mean 12738) | 0 above the Budget; 2^14:342009 2^15:23991 |
| Task 1 entry | 121999 | 9904 / 0.155 (min 3160, mean 3265) | 0 above the Budget; 2^12:121571 2^13:38 2^14:390 |
| Task 2 entry | 182999 | 417725 / 6.527 (min 399213, mean 400950) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 205393 / 3.209 (min 187713, mean 195964) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242948 | 17220 / 0.269 (min 3218, mean 3361) | 0 above the Budget; 2^12:237665 2^13:3580 2^14:1700 2^15:3 |
| Task 5 entry | 128100 | 38988 / 0.609 (min 20540, mean 26994) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 39727 / 0.621 (min 19008, mean 27284) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 10814 / 0.169 | |
| Task 0 on Resource 1 | 366000 | 10811 / 0.169 | |
| Task 2 on Resource 0 | 182999 | 392253 / 6.129 | |
| Task 2 on Resource 1 | 365998 | 10250 / 0.160 | |
| Task 5 on Resource 2 | 256200 | 28868 / 0.451 | |
| Task 6 on Resource 2 | 18300 | 8575 / 0.134 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188448 | 2889 / 45.14 | 2716 |
| timer | 1314267 | 4824 / 75.38 | 2997 |
| intake | 242949 | 2301 / 35.95 | 2073 |
| lock | 1573797 | 475 / 7.42 | 473 |
| unlock | 1555497 | 1782 / 27.84 | 1591 |
| job_end | 1128971 | 2001 / 31.27 | 1733 |
| now | 132614351 | 239 / 3.73 | 237 |
| raise | 122000 | 2039 / 31.86 | 1951 |
| dma | 182999 | 735 / 11.48 | 555 |
| epilogue | 1188446 | 1210 / 18.91 | 580 |
| restart | 18300 | 4824 / 75.38 | 4611 |
| enforce | 59475 | 4824 / 75.38 | 3635 |
| view | 1467356 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 232226, Task 1: 818, Task 2: 631626, Task 3: 128567, Task 4: 7259, Task 5: 69695, Task 6: 54436

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3772 cycles
- Task 0 at 3659940.000 ms, 3170 cycles
- Task 2 at 3659945.000 ms, 3170 cycles
- Task 0 at 3659950.000 ms, 4048 cycles
- Task 5 at 3659950.000 ms, 4048 cycles
- Task 0 at 3659960.000 ms, 3418 cycles
- Task 3 at 3659960.000 ms, 3418 cycles
- Task 2 at 3659965.000 ms, 4070 cycles
- Task 0 at 3659970.000 ms, 3576 cycles
- Task 5 at 3659975.000 ms, 3170 cycles
- Task 0 at 3659980.000 ms, 3170 cycles
- Task 0 at 3659990.000 ms, 3090 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 137780666 of them, the longest 26248 cycles (410.1 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 131998275 |
| 512 to 1023 (8.0 to 16.0 µs) | 2023686 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1305756 |
| 2048 to 4095 (32.0 to 64.0 µs) | 835987 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1190772 |
| 8192 to 16383 (128.0 to 256.0 µs) | 423884 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2306 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 997084 | 26248 / 410.1 |
| timer | 659738 | 19819 / 309.7 |
| job_end | 135450579 | 5165 / 80.7 |
| epilogue | 673203 | 13897 / 217.1 |
| panic | 62 | 5075 / 79.3 |

## The Kernel's events

137254 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6814907 | 366932 | 0.420 |
| 1 | 467087 | 122031 | 0.150 |
| 2 | 115617736 | 427013 | 6.460 |
| 3 | 31394292 | 119100 | 4.610 |
| 4 | 940056 | 244912 | 0.190 |
| 5 | 6787032 | 150559 | 0.900 |
| 6 | 2272745 | 37876 | 0.910 |
| none | 204125985 | 909537 | 16565.970 |

## Commands

    probe-rs download --chip nRF52840_xxAA --probe 1366:1061:001050279833 --disable-progressbars /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stimulus-storm/thumbv7em-none-eabihf/release/hil-stimulus
    probe-rs reset --chip nRF52840_xxAA --probe 1366:1061:001050279833
    sigrok-cli -d fx2lafw -c samplerate=100000 --time 3752000 -O binary -o /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/run/capture.bin
    probe-rs run --chip nRF52840_xxAA --probe 1366:1060:001050217737 --disable-progressbars --catch-hardfault /Users/mars2292/repos/Kiro-experimentation/IronOxide/target/xtask-hil/stress/nrf52840_stress/thumbv7em-none-eabihf/release/p1-demo

## Target console

```

RSK-SAFE-STATE
control=366000
sensor=182999
log=68625
pulse=242948
echo=121999
blocked_releases=182999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=546
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=275
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
  at_ms=3659301
  detail=51825
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659301
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659324
  detail=289219
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659324
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659375
  detail=52034
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659375
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659375
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659485
  detail=288426
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659485
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659501
  detail=52361
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659501
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659575
  detail=52034
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659575
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659575
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659644
  detail=288635
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659644
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659701
  detail=52421
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659701
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659775
  detail=51970
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659775
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659775
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=3659812
  detail=288645
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659812
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=52425
  prs_lo=134217728
  prs_hi=0
event=19
  task=6
  at_ms=3659901
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=3
  at_ms=3659965
  detail=288426
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659965
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659975
  detail=52034
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=3659975
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=3659975
  detail=1
  prs_lo=128
  prs_hi=0
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
expected_sensor=183000
part0_events=0
RSK-OK
Firmware exited successfully
     Finished in 7.74s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` (sha256:9829259f270a6430c42a18198ab6176297309c45a27c62f91c196bb7f122a799) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:7107bc3df75cb35d0728869b8e643cc87dbf300fc64768a39262fe5bb5c6ea43 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 594 cycles (measured, rsk-hil release-timing record, 1573797 runs, max 475 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2313 cycles (measured, rsk-hil release-timing record, 317 runs, max 1850 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3612 cycles (measured, rsk-hil release-timing record, 1188448 runs, max 2889 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4045 cycles (measured, rsk-hil release-timing record, 558 runs, max 3236 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6030 cycles (measured, rsk-hil release-timing record, 1314267 runs, max 4824 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2737 cycles (measured, rsk-hil release-timing record, 242949 runs, max 2189 +25%, scenarios: nrf52840_stress (3660 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 32810 cycles (measured, rsk-hil release-timing record, 137780666 runs, max 26248 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 26248 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 997084 (longest 26248), timer 659738 (longest 19819), job_end 135450579 (longest 5165), epilogue 673203 (longest 13897), panic 62 (longest 5075); the longest single Rust half 4824) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 130 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1961, less Δ_entry of the plain cycle configuration, 2022, is 0, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 130 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6030 cycles (measured, rsk-hil release-timing record, 1314267 runs, max 4824 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6030 cycles (measured, rsk-hil release-timing record, 59475 runs, max 4824 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11623 cycles (measured, rsk-hil release-timing record, 823495 runs, max 9298 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 299 cycles (measured, rsk-hil release-timing record, 132614351 runs, max 239 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 919 cycles (measured, rsk-hil release-timing record, 182999 runs, max 735 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 702 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6030 cycles (measured, rsk-hil release-timing record, 18300 runs, max 4824 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2528 cycles (measured, rsk-hil release-timing record, 149 runs, max 2022 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 149 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 98 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.477 | 10.000 | 1.711 | 366000 | within |
| `part0.echo` | A | 11.838 | 20.000 | 0.344 | 121999 | within |
| `part0.sensor` | A | 14.038 | 20.000 | 6.912 | 182999 | within |
| `part1.log` | C | 40.423 | 40.000 | 4.286 | 68625 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.001 | 15.000 | 7.022 | 242948 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 17.686 | 25.000 | 7.537 | 128100 | within |
| `part2.sampler` | B | 100.521 | 100.000 | 12.786 | 18300 | not schedulable: no WCRT to hold |

Verdict: **pass**
