# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3685 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9422 cycles (147.2 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 0.679 ms (43428 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 121999 Jobs, response at most 0.360 ms (23020 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 182999 Jobs, response at most 5.901 ms (377660 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242933 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2291 cycles (35.8 µs), mean 2005 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2080 % of the run |
| Kernel events | pass | 137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54900 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 24429 cycles (381.7 µs) over 113828237 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 242932 response edges inside the run (target counted 242932) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.7 % of the run, the Kernel 34.6 % over 20177475 entries; runs per Task [366833, 122033, 427014, 121853, 244509, 150687, 38229] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24475 cycles (0.382 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 366833 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:390 | 10_000_000 | 424 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:428 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 258 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:506 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9422 / 147.2 | 3641 | 2^12:339287 2^13:26371 2^14:341 |
| 2 | 182999 | 9278 / 145.0 | 3376 | 2^12:173515 2^13:9345 2^14:139 |
| 3 | 91499 | 9249 / 144.5 | 3627 | 2^12:90318 2^13:1036 2^14:145 |
| 5 | 146399 | 9422 / 147.2 | 3708 | 2^12:122472 2^13:23658 2^14:269 |
| 6 | 36599 | 9249 / 144.5 | 3942 | 2^12:36111 2^13:393 2^14:95 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 0.679 | 0 | 0 | 0 |
| 1 | 121999 | 0.360 | 0 | 0 | 0 |
| 2 | 182999 | 5.901 | 0 | 0 | 0 |
| 3 | 68625 | 4.419 | 0 | 0 | 0 |
| 4 | 242932 | 5.929 | 242933 | 2291 | 2005 |
| 5 | 128100 | 6.547 | 0 | 0 | 0 |
| 6 | 18300 | 11.859 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 24475 / 0.382 (min 9922, mean 11521) | 0 above the Budget; 2^14:361665 2^15:4335 |
| Task 1 entry | 121999 | 10443 / 0.163 (min 3518, mean 3657) | 0 above the Budget; 2^12:121700 2^13:55 2^14:244 |
| Task 2 entry | 182999 | 351053 / 5.485 (min 336125, mean 338135) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 206917 / 3.233 (min 187521, mean 196879) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242932 | 17678 / 0.276 (min 3585, mean 3993) | 0 above the Budget; 2^12:220415 2^13:21139 2^14:1377 2^15:1 |
| Task 5 entry | 128100 | 39503 / 0.617 (min 20775, mean 27693) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 42719 / 0.667 (min 20386, mean 29763) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 10965 / 0.171 | |
| Task 0 on Resource 1 | 366000 | 10898 / 0.170 | |
| Task 2 on Resource 0 | 182999 | 328584 / 5.134 | |
| Task 2 on Resource 1 | 365998 | 7153 / 0.112 | |
| Task 5 on Resource 2 | 256200 | 29145 / 0.455 | |
| Task 6 on Resource 2 | 18300 | 9043 / 0.141 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188434 | 3298 / 51.53 | 3042 |
| timer | 1160238 | 4958 / 77.47 | 3022 |
| intake | 242933 | 2421 / 37.83 | 2125 |
| lock | 1573797 | 459 / 7.17 | 457 |
| unlock | 1555497 | 1843 / 28.80 | 1606 |
| job_end | 1128955 | 2077 / 32.45 | 1742 |
| now | 108582462 | 225 / 3.52 | 222 |
| raise | 122000 | 2063 / 32.23 | 1978 |
| dma | 182999 | 720 / 11.25 | 550 |
| epilogue | 1188430 | 1244 / 19.44 | 612 |
| restart | 18300 | 4958 / 77.47 | 4778 |
| enforce | 59475 | 4958 / 77.47 | 3720 |
| view | 1470184 | 320 / 5.00 | 286 |

Entries that interrupted an executing Job: Task 0: 53511, Task 1: 543, Task 2: 608471, Task 3: 130971, Task 4: 24349, Task 5: 74648, Task 6: 55353

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 4 (timer, intake), 11227 cycles; Task 1: 2 (timer, intake), 5227 cycles; Task 2: 6 (dispatch, timer, intake), 17713 cycles; Task 3: 3 (dispatch, timer, intake), 8578 cycles; Task 4: 4 (dispatch, timer), 12514 cycles; Task 5: 4 (dispatch, timer, intake), 11145 cycles; Task 6: 5 (dispatch, timer, intake), 14156 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3962 cycles
- Task 0 at 3659940.000 ms, 3217 cycles
- Task 2 at 3659945.000 ms, 3225 cycles
- Task 0 at 3659950.000 ms, 3650 cycles
- Task 5 at 3659950.000 ms, 3650 cycles
- Task 0 at 3659960.000 ms, 3453 cycles
- Task 3 at 3659960.000 ms, 3453 cycles
- Task 2 at 3659965.000 ms, 4741 cycles
- Task 0 at 3659970.000 ms, 3394 cycles
- Task 5 at 3659975.000 ms, 3233 cycles
- Task 0 at 3659980.000 ms, 3217 cycles
- Task 0 at 3659990.000 ms, 3101 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 113828237 of them, the longest 24429 cycles (381.7 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 108303795 |
| 512 to 1023 (8.0 to 16.0 µs) | 1753503 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1315948 |
| 2048 to 4095 (32.0 to 64.0 µs) | 894965 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1272731 |
| 8192 to 16383 (128.0 to 256.0 µs) | 284335 |
| 16384 to 32767 (256.0 to 512.0 µs) | 2960 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 1149902 | 24429 / 381.7 |
| timer | 512365 | 22040 / 344.4 |
| job_end | 111495679 | 5778 / 90.3 |
| epilogue | 670291 | 14416 / 225.2 |

## The Kernel's events

137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 5970738 | 366833 | 0.370 |
| 1 | 495457 | 122033 | 0.150 |
| 2 | 97740399 | 427014 | 5.410 |
| 3 | 31506762 | 121853 | 4.670 |
| 4 | 1092542 | 244509 | 0.200 |
| 5 | 6872975 | 150687 | 0.870 |
| 6 | 2508180 | 38229 | 0.990 |
| none | 222296275 | 906677 | 16689.680 |

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
pulse=242932
echo=121999
blocked_releases=182999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=424
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=258
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
  detail=58524
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
  detail=288619
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
  detail=51828
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
  detail=289034
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
  detail=58408
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
  detail=51828
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
  detail=289034
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
  detail=58524
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
  detail=52069
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
  detail=289208
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
  detail=58524
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
  detail=289034
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
  detail=51828
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
     Finished in 8.08s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` (sha256:21accb2bfc9d8dcdd0636247ac52f17151253f31b777df7d25ea5feab13e413e) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:21accb2bfc9d8dcdd0636247ac52f17151253f31b777df7d25ea5feab13e413e is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 574 cycles (measured, rsk-hil release-timing record, 1573797 runs, max 459 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2304 cycles (measured, rsk-hil release-timing record, 1555497 runs, max 1843 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4123 cycles (measured, rsk-hil release-timing record, 1188434 runs, max 3298 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4152 cycles (measured, rsk-hil release-timing record, 1128955 runs, max 3321 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6198 cycles (measured, rsk-hil release-timing record, 1160238 runs, max 4958 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2864 cycles (measured, rsk-hil release-timing record, 242933 runs, max 2291 +25%, scenarios: nrf52840_stress (3660 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 30537 cycles (measured, rsk-hil release-timing record, 113828237 runs, max 24429 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 24429 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 1149902 (longest 24429), timer 512365 (longest 22040), job_end 111495679 (longest 5778), epilogue 670291 (longest 14416); the longest single Rust half 4958) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6198 cycles (measured, rsk-hil release-timing record, 1160238 runs, max 4958 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6198 cycles (measured, rsk-hil release-timing record, 59475 runs, max 4958 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11778 cycles (measured, rsk-hil release-timing record, 823495 runs, max 9422 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 282 cycles (measured, rsk-hil release-timing record, 108582462 runs, max 225 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 900 cycles (measured, rsk-hil release-timing record, 182999 runs, max 720 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 400 cycles (measured, rsk-hil release-timing record, 1470184 runs, max 320 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6198 cycles (measured, rsk-hil release-timing record, 18300 runs, max 4958 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.488 | 10.000 | 0.679 | 366000 | within |
| `part0.echo` | A | 11.925 | 20.000 | 0.360 | 121999 | within |
| `part0.sensor` | A | 14.323 | 20.000 | 5.901 | 182999 | within |
| `part1.log` | C | 40.727 | 40.000 | 4.419 | 68625 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.206 | 15.000 | 5.929 | 242932 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 18.132 | 25.000 | 6.547 | 128100 | within |
| `part2.sampler` | B | 100.403 | 100.000 | 11.859 | 18300 | not schedulable: no WCRT to hold |

Verdict: **pass**
