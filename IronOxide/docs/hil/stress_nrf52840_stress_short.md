# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 204 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9161 cycles (143.1 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 0.623 ms (39844 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 5999 Jobs, response at most 0.303 ms (19388 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 8999 Jobs, response at most 5.857 ms (374820 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11948 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2291 cycles (35.8 µs), mean 2005 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2080 % of the run |
| Kernel events | pass | 6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 23329 cycles (364.5 µs) over 5595873 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11947 response edges inside the run (target counted 11947) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 35.2 % of the run, the Kernel 41.8 % over 992027 entries; runs per Task [18047, 5999, 20998, 6007, 12022, 7408, 1878] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 20931 cycles (0.327 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 19840 cycles (0.310 ms at 100000 samples/s; 18047 runs for 18000 Jobs) |

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
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:442 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9161 / 143.1 | 3712 | 2^12:16679 2^13:1299 2^14:21 |
| 2 | 8999 | 8329 / 130.1 | 3372 | 2^12:8511 2^13:487 2^14:1 |
| 3 | 4499 | 9161 / 143.1 | 3628 | 2^12:4447 2^13:43 2^14:9 |
| 5 | 7199 | 9161 / 143.1 | 3735 | 2^12:5861 2^13:1332 2^14:6 |
| 6 | 1799 | 9161 / 143.1 | 3940 | 2^12:1774 2^13:21 2^14:4 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 0.623 | 0 | 0 | 0 |
| 1 | 5999 | 0.303 | 0 | 0 | 0 |
| 2 | 8999 | 5.857 | 0 | 0 | 0 |
| 3 | 3375 | 4.419 | 0 | 0 | 0 |
| 4 | 11947 | 5.911 | 11948 | 2291 | 2005 |
| 5 | 6300 | 6.486 | 0 | 0 | 0 |
| 6 | 900 | 11.833 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 20931 / 0.327 (min 9922, mean 11525) | 0 above the Budget; 2^14:17773 2^15:227 |
| Task 1 entry | 5999 | 10427 / 0.163 (min 3518, mean 3657) | 0 above the Budget; 2^12:5983 2^13:5 2^14:11 |
| Task 2 entry | 8999 | 346849 / 5.420 (min 336125, mean 338103) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 206681 / 3.229 (min 191037, mean 196869) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11947 | 14275 / 0.223 (min 3585, mean 3985) | 0 above the Budget; 2^12:10861 2^13:1024 2^14:62 |
| Task 5 entry | 6300 | 36479 / 0.570 (min 20775, mean 27700) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 39135 / 0.611 (min 20386, mean 29669) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10833 / 0.169 | |
| Task 0 on Resource 1 | 18000 | 10714 / 0.167 | |
| Task 2 on Resource 0 | 8999 | 328336 / 5.130 | |
| Task 2 on Resource 1 | 17998 | 7087 / 0.111 | |
| Task 5 on Resource 2 | 12600 | 26393 / 0.412 | |
| Task 6 on Resource 2 | 900 | 4165 / 0.065 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58445 | 3290 / 51.41 | 3042 |
| timer | 57044 | 4958 / 77.47 | 3022 |
| intake | 11948 | 2421 / 37.83 | 2125 |
| lock | 77397 | 459 / 7.17 | 457 |
| unlock | 76497 | 1840 / 28.75 | 1606 |
| job_end | 55520 | 2076 / 32.44 | 1742 |
| now | 5340835 | 225 / 3.52 | 222 |
| raise | 6000 | 2062 / 32.22 | 1978 |
| dma | 8999 | 720 / 11.25 | 550 |
| epilogue | 58445 | 1244 / 19.44 | 612 |
| restart | 900 | 4958 / 77.47 | 4778 |
| enforce | 2925 | 4958 / 77.47 | 3719 |
| view | 72305 | 320 / 5.00 | 286 |

Entries that interrupted an executing Job: Task 0: 2653, Task 1: 27, Task 2: 29899, Task 3: 6473, Task 4: 1171, Task 5: 3666, Task 6: 2709

Entries that interrupted each Task's longest Job (their Rust halves' cycles are in that Job's execution, charged in full when the entry switched nothing): Task 0: 3 (timer, intake), 8312 cycles; Task 1: 2 (timer, intake), 5213 cycles; Task 2: 5 (dispatch, timer, intake), 14348 cycles; Task 3: 3 (dispatch, timer, intake), 8482 cycles; Task 4: 3 (dispatch, timer), 9727 cycles; Task 5: 3 (dispatch, timer, intake), 8670 cycles; Task 6: 4 (dispatch, timer, intake), 11273 cycles

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3922 cycles
- Task 0 at 179940.000 ms, 3217 cycles
- Task 2 at 179945.000 ms, 3225 cycles
- Task 0 at 179950.000 ms, 4066 cycles
- Task 5 at 179950.000 ms, 4066 cycles
- Task 0 at 179960.000 ms, 3453 cycles
- Task 3 at 179960.000 ms, 3453 cycles
- Task 2 at 179965.000 ms, 4138 cycles
- Task 0 at 179970.000 ms, 3746 cycles
- Task 5 at 179975.000 ms, 3233 cycles
- Task 0 at 179980.000 ms, 3217 cycles
- Task 0 at 179990.000 ms, 3137 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 5595873 of them, the longest 23329 cycles (364.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 5324169 |
| 512 to 1023 (8.0 to 16.0 µs) | 86223 |
| 1024 to 2047 (16.0 to 32.0 µs) | 64772 |
| 2048 to 4095 (32.0 to 64.0 µs) | 41589 |
| 4096 to 8191 (64.0 to 128.0 µs) | 65325 |
| 8192 to 16383 (128.0 to 256.0 µs) | 13658 |
| 16384 to 32767 (256.0 to 512.0 µs) | 137 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 56551 | 23329 / 364.5 |
| timer | 25205 | 19608 / 306.4 |
| job_end | 5481160 | 4578 / 71.5 |
| epilogue | 32957 | 14289 / 223.3 |

## The Kernel's events

6753 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 293759 | 18047 | 0.310 |
| 1 | 24373 | 5999 | 0.150 |
| 2 | 4805893 | 20998 | 5.290 |
| 3 | 1549691 | 6007 | 4.590 |
| 4 | 53562 | 12022 | 0.160 |
| 5 | 338091 | 7408 | 0.860 |
| 6 | 123187 | 1878 | 0.970 |
| none | 13207476 | 44586 | 16191.760 |

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
pulse=11947
echo=5999
blocked_releases=8999
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
  at_ms=179301
  detail=59605
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
  detail=289638
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
  detail=51876
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
  at_ms=179490
  detail=293292
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179490
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179501
  detail=58408
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
  detail=51876
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
  detail=288750
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
  detail=58476
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
  detail=51816
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
  detail=288679
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
  detail=58524
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
  at_ms=179970
  detail=293292
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179970
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=5
  at_ms=179975
  detail=51876
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
     Finished in 7.64s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:c1adbfa8238b23e8a73b690deeca714fe685aa040685845f19a83fd01a4ff7e0) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d4faae210fcad55506e286ec2510e80a1398acb827111fa97f1d4dd36f9080fc is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 574 cycles (measured, rsk-hil release-timing record, 77397 runs, max 459 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2377 cycles (measured, rsk-hil release-timing record, 794 runs, max 1901 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4113 cycles (measured, rsk-hil release-timing record, 58445 runs, max 3290 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4170 cycles (measured, rsk-hil release-timing record, 1116 runs, max 3336 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6198 cycles (measured, rsk-hil release-timing record, 57044 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2864 cycles (measured, rsk-hil release-timing record, 11948 runs, max 2291 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 29162 cycles (measured, rsk-hil release-timing record, 5595873 runs, max 23329 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 23329 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 56551 (longest 23329), timer 25205 (longest 19608), job_end 5481160 (longest 4578), epilogue 32957 (longest 14289); the longest single Rust half 4958) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6198 cycles (measured, rsk-hil release-timing record, 57044 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6198 cycles (measured, rsk-hil release-timing record, 2925 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11452 cycles (measured, rsk-hil release-timing record, 40495 runs, max 9161 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 284 cycles (measured, rsk-hil release-timing record, 98606 runs, max 227 +25%, scenarios: nrf52840_cycles (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 900 cycles (measured, rsk-hil release-timing record, 8999 runs, max 720 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 1404 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6198 cycles (measured, rsk-hil release-timing record, 900 runs, max 4958 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2630 cycles (measured, rsk-hil release-timing record, 317 runs, max 2104 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 157 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 98 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.511 | 10.000 | 0.623 | 18000 | within |
| `part0.echo` | A | 12.011 | 20.000 | 0.303 | 5999 | within |
| `part0.sensor` | A | 14.425 | 20.000 | 5.857 | 8999 | within |
| `part1.log` | C | 41.054 | 40.000 | 4.419 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.338 | 15.000 | 5.911 | 11947 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 18.325 | 25.000 | 6.486 | 6300 | within |
| `part2.sampler` | B | 101.260 | 100.000 | 11.833 | 900 | not schedulable: no WCRT to hold |

Verdict: **pass**
