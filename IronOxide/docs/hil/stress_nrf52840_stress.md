# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3684 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9098 cycles (142.2 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 0.654 ms (41864 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 121999 Jobs, response at most 0.329 ms (21084 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 182999 Jobs, response at most 5.852 ms (374552 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242949 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2233 cycles (34.9 µs), mean 1990 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2064 % of the run |
| Kernel events | pass | 137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54900 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 25236 cycles (394.3 µs) over 110835466 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 242948 response edges inside the run (target counted 242948) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.5 % of the run, the Kernel 34.6 % over 22983052 entries; runs per Task [366782, 122032, 427021, 118703, 244103, 150538, 37800] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23623 cycles (0.369 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 366782 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:385 | 10_000_000 | 413 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:423 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:418 | 10_000_000 | 251 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9085 / 142.0 | 3552 | 2^12:361286 2^13:4433 2^14:280 |
| 2 | 182999 | 9098 / 142.2 | 3277 | 2^12:179171 2^13:3630 2^14:198 |
| 3 | 91499 | 9085 / 142.0 | 3558 | 2^12:90592 2^13:894 2^14:13 |
| 5 | 146399 | 9085 / 142.0 | 3601 | 2^12:142616 2^13:3648 2^14:135 |
| 6 | 36599 | 9085 / 142.0 | 3861 | 2^12:36261 2^13:307 2^14:31 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 0.654 | 0 | 0 | 0 |
| 1 | 121999 | 0.329 | 0 | 0 | 0 |
| 2 | 182999 | 5.852 | 0 | 0 | 0 |
| 3 | 68625 | 4.333 | 0 | 0 | 0 |
| 4 | 242948 | 6.015 | 242949 | 2233 | 1990 |
| 5 | 128100 | 6.469 | 0 | 0 | 0 |
| 6 | 18300 | 11.792 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 23623 / 0.369 (min 9711, mean 11271) | 0 above the Budget; 2^14:363072 2^15:2928 |
| Task 1 entry | 121999 | 9935 / 0.155 (min 3327, mean 3428) | 0 above the Budget; 2^12:121635 2^13:57 2^14:307 |
| Task 2 entry | 182999 | 349043 / 5.454 (min 335811, mean 337768) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 209658 / 3.276 (min 188422, mean 196769) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242948 | 16477 / 0.257 (min 3391, mean 3759) | 0 above the Budget; 2^12:221205 2^13:20757 2^14:981 2^15:5 |
| Task 5 entry | 128100 | 39526 / 0.618 (min 21050, mean 27200) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 37506 / 0.586 (min 19887, mean 28268) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 10720 / 0.168 | |
| Task 0 on Resource 1 | 366000 | 10513 / 0.164 | |
| Task 2 on Resource 0 | 182999 | 331636 / 5.182 | |
| Task 2 on Resource 1 | 365998 | 7443 / 0.116 | |
| Task 5 on Resource 2 | 256200 | 28597 / 0.447 | |
| Task 6 on Resource 2 | 18300 | 8796 / 0.137 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188446 | 3093 / 48.33 | 2878 |
| timer | 1162162 | 4781 / 74.70 | 2960 |
| intake | 242949 | 2343 / 36.61 | 2092 |
| lock | 1573797 | 462 / 7.22 | 459 |
| unlock | 1555497 | 1754 / 27.41 | 1565 |
| job_end | 1128971 | 1966 / 30.72 | 1710 |
| now | 105635305 | 224 / 3.50 | 221 |
| raise | 122000 | 2011 / 31.42 | 1919 |
| dma | 182999 | 725 / 11.33 | 546 |
| epilogue | 1188446 | 1254 / 19.59 | 612 |
| restart | 18300 | 4781 / 74.70 | 4604 |
| enforce | 59475 | 4781 / 74.70 | 3645 |
| view | 1466058 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 58002, Task 1: 671, Task 2: 603342, Task 3: 129788, Task 4: 23114, Task 5: 74017, Task 6: 54492

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3790 cycles
- Task 0 at 3659940.000 ms, 3165 cycles
- Task 2 at 3659945.000 ms, 3161 cycles
- Task 0 at 3659950.000 ms, 3870 cycles
- Task 5 at 3659950.000 ms, 3870 cycles
- Task 0 at 3659960.000 ms, 3405 cycles
- Task 3 at 3659960.000 ms, 3405 cycles
- Task 2 at 3659965.000 ms, 3553 cycles
- Task 0 at 3659970.000 ms, 3274 cycles
- Task 5 at 3659975.000 ms, 3161 cycles
- Task 0 at 3659980.000 ms, 3165 cycles
- Task 0 at 3659990.000 ms, 3053 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 110835466 of them, the longest 25236 cycles (394.3 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 104718071 |
| 512 to 1023 (8.0 to 16.0 µs) | 2335433 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1326501 |
| 2048 to 4095 (32.0 to 64.0 µs) | 861860 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1310028 |
| 8192 to 16383 (128.0 to 256.0 µs) | 281482 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2091 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 1144073 | 25236 / 394.3 |
| timer | 511215 | 21714 / 339.3 |
| job_end | 108502917 | 5298 / 82.8 |
| epilogue | 677261 | 13998 / 218.7 |

## The Kernel's events

137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 5893869 | 366782 | 0.350 |
| 1 | 475571 | 122032 | 0.150 |
| 2 | 97578102 | 427021 | 5.340 |
| 3 | 31475565 | 118703 | 4.620 |
| 4 | 1045407 | 244103 | 0.180 |
| 5 | 6797877 | 150538 | 0.900 |
| 6 | 2281062 | 37800 | 0.900 |
| none | 222880579 | 910835 | 16159.090 |

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
loop:part0:sensor:hold=413
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=251
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
  detail=51925
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
  detail=289288
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
  detail=51933
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
  at_ms=3659484
  detail=288713
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659484
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659501
  detail=51817
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
  detail=51933
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
  detail=288652
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
  detail=52176
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
  detail=51869
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
  at_ms=3659811
  detail=288918
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659811
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=51925
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
  at_ms=3659964
  detail=288713
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659964
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=3659975
  detail=51933
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
     Finished in 8.06s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` (sha256:281a863c3205a800e9bd7d7047d70434da25b790db6eefe5cc216ffe4537e61a) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:c9966efb6aeb79cc4e8024d3024a395c9044b6e7eefe7186fa28b2950b022aa3 is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 578 cycles (measured, rsk-hil release-timing record, 1573797 runs, max 462 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2295 cycles (measured, rsk-hil release-timing record, 317 runs, max 1836 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3867 cycles (measured, rsk-hil release-timing record, 1188446 runs, max 3093 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4064 cycles (measured, rsk-hil release-timing record, 1116 runs, max 3251 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5977 cycles (measured, rsk-hil release-timing record, 1162162 runs, max 4781 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2792 cycles (measured, rsk-hil release-timing record, 242949 runs, max 2233 +25%, scenarios: nrf52840_stress (3660 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 31545 cycles (measured, rsk-hil release-timing record, 110835466 runs, max 25236 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 25236 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 1144073 (longest 25236), timer 511215 (longest 21714), job_end 108502917 (longest 5298), epilogue 677261 (longest 13998); the longest single Rust half 4781) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 63 cycles (measured, rsk-hil release-timing record, 125 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2123, less Δ_entry of the plain cycle configuration, 2075, is 48, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 63 cycles (measured, rsk-hil release-timing record, 125 runs, max 50 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 2123, less Δ_entry of the plain cycle configuration, 2075, is 48, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5977 cycles (measured, rsk-hil release-timing record, 1162162 runs, max 4781 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 5977 cycles (measured, rsk-hil release-timing record, 59475 runs, max 4781 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11373 cycles (measured, rsk-hil release-timing record, 823495 runs, max 9098 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 287 cycles (measured, rsk-hil release-timing record, 49091 runs, max 229 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 907 cycles (measured, rsk-hil release-timing record, 182999 runs, max 725 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 706 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 5977 cycles (measured, rsk-hil release-timing record, 18300 runs, max 4781 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2594 cycles (measured, rsk-hil release-timing record, 295 runs, max 2075 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 152 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.507 | 10.000 | 0.654 | 366000 | within |
| `part0.echo` | A | 11.966 | 20.000 | 0.329 | 121999 | within |
| `part0.sensor` | A | 14.332 | 20.000 | 5.852 | 182999 | within |
| `part1.log` | C | 40.641 | 40.000 | 4.333 | 68625 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.229 | 15.000 | 6.015 | 242948 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 18.154 | 25.000 | 6.469 | 128100 | within |
| `part2.sampler` | B | 100.404 | 100.000 | 11.792 | 18300 | not schedulable: no WCRT to hold |

Verdict: **pass**
