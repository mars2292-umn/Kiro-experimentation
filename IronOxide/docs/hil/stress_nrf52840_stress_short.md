# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.1 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 205 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9041 cycles (141.3 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.662 ms (106364 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.292 ms (18700 cycles) against WCRT 11.364 ms (727300 cycles); part0.sensor: 8999 Jobs, response at most 6.871 ms (439724 cycles) against WCRT 11.512 ms (736800 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11948 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2229 cycles (34.8 µs), mean 2033 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2109 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 2700 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 26451 cycles (413.3 µs) over 6858432 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 11947 response edges inside the run (target counted 11947) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.6 % of the run, the Kernel 48.1 % over 595575 entries; runs per Task [18039, 6002, 20998, 5856, 12041, 7395, 1875] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 24355 cycles (0.381 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 23680 cycles (0.370 ms at 100000 samples/s; 18039 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 552 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 279 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 9041 / 141.3 | 3657 | 2^12:16810 2^13:1166 2^14:23 |
| 2 | 8999 | 8449 / 132.0 | 3353 | 2^12:8815 2^13:180 2^14:4 |
| 3 | 4499 | 9041 / 141.3 | 3672 | 2^12:3853 2^13:639 2^14:7 |
| 5 | 7199 | 8879 / 138.7 | 3708 | 2^12:6093 2^13:1099 2^14:7 |
| 6 | 1799 | 8849 / 138.3 | 3967 | 2^12:1185 2^13:613 2^14:1 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.662 | 0 | 0 | 0 |
| 1 | 5999 | 0.292 | 0 | 0 | 0 |
| 2 | 8999 | 6.871 | 0 | 0 | 0 |
| 3 | 3375 | 4.310 | 0 | 0 | 0 |
| 4 | 11947 | 7.051 | 11948 | 2229 | 2033 |
| 5 | 6300 | 7.517 | 0 | 0 | 0 |
| 6 | 900 | 12.759 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 24355 / 0.381 (min 9433, mean 12813) | 0 above the Budget; 2^14:16886 2^15:1114 |
| Task 1 entry | 5999 | 9954 / 0.156 (min 3158, mean 3254) | 0 above the Budget; 2^12:5987 2^13:0 2^14:12 |
| Task 2 entry | 8999 | 409929 / 6.405 (min 399345, mean 401334) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 205975 / 3.218 (min 191015, mean 196366) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11947 | 16649 / 0.260 (min 3223, mean 3365) | 0 above the Budget; 2^12:11681 2^13:188 2^14:75 2^15:3 |
| Task 5 entry | 6300 | 39128 / 0.611 (min 20592, mean 27554) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 36480 / 0.570 (min 19018, mean 27274) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 10633 / 0.166 | |
| Task 0 on Resource 1 | 18000 | 10850 / 0.170 | |
| Task 2 on Resource 0 | 8999 | 392308 / 6.130 | |
| Task 2 on Resource 1 | 17998 | 6932 / 0.108 | |
| Task 5 on Resource 2 | 12600 | 28780 / 0.450 | |
| Task 6 on Resource 2 | 900 | 4041 / 0.063 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58445 | 2892 / 45.19 | 2716 |
| timer | 64728 | 4948 / 77.31 | 3089 |
| intake | 11948 | 2354 / 36.78 | 2148 |
| lock | 77397 | 454 / 7.09 | 452 |
| unlock | 76497 | 1830 / 28.59 | 1646 |
| job_end | 55520 | 2064 / 32.25 | 1805 |
| now | 6604120 | 235 / 3.67 | 232 |
| raise | 6000 | 2080 / 32.50 | 1993 |
| dma | 8999 | 722 / 11.28 | 548 |
| epilogue | 58445 | 1228 / 19.19 | 586 |
| restart | 900 | 4948 / 77.31 | 4765 |
| enforce | 2925 | 4948 / 77.31 | 3751 |
| view | 72155 | 330 / 5.16 | 296 |

Entries that interrupted an executing Job: Task 0: 11317, Task 1: 24, Task 2: 31206, Task 3: 6300, Task 4: 366, Task 5: 3452, Task 6: 2711

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3599 cycles
- Task 0 at 179940.000 ms, 3265 cycles
- Task 2 at 179945.000 ms, 3257 cycles
- Task 0 at 179950.000 ms, 3959 cycles
- Task 5 at 179950.000 ms, 3959 cycles
- Task 0 at 179960.000 ms, 3505 cycles
- Task 3 at 179960.000 ms, 3505 cycles
- Task 2 at 179965.000 ms, 3257 cycles
- Task 0 at 179970.000 ms, 3939 cycles
- Task 5 at 179975.000 ms, 3265 cycles
- Task 0 at 179980.000 ms, 3265 cycles
- Task 0 at 179990.000 ms, 3185 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 6858432 of them, the longest 26451 cycles (413.3 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 6563307 |
| 512 to 1023 (8.0 to 16.0 µs) | 109560 |
| 1024 to 2047 (16.0 to 32.0 µs) | 59364 |
| 2048 to 4095 (32.0 to 64.0 µs) | 46343 |
| 4096 to 8191 (64.0 to 128.0 µs) | 58765 |
| 8192 to 16383 (128.0 to 256.0 µs) | 20964 |
| 16384 to 32767 (256.0 to 512.0 µs) | 129 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 49064 | 26451 / 413.3 |
| timer | 32543 | 19325 / 302.0 |
| job_end | 6743935 | 2372 / 37.1 |
| epilogue | 32888 | 12499 / 195.3 |
| panic | 2 | 6888 / 107.6 |

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 339247 | 18039 | 0.370 |
| 1 | 23505 | 6002 | 0.150 |
| 2 | 5691906 | 20998 | 6.290 |
| 3 | 1546709 | 5856 | 4.610 |
| 4 | 47577 | 12041 | 0.180 |
| 5 | 340493 | 7395 | 0.900 |
| 6 | 111832 | 1875 | 0.900 |
| none | 12371563 | 44736 | 16829.200 |

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
loop:part0:sensor:hold=552
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=279
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
  detail=52237
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
  detail=289416
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
  detail=52277
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
  detail=288792
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
  detail=52169
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
  at_ms=179576
  detail=52371
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179576
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179576
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179644
  detail=288920
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
  detail=52285
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
  detail=52209
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
  at_ms=179812
  detail=288922
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179812
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=52237
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
  detail=288792
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
  detail=52277
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
     Finished in 7.75s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:d42f8d366010de3d6f99af4ff0b4a15c5a52bf105330b8deb86be6f384ad8d7b) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d42f8d366010de3d6f99af4ff0b4a15c5a52bf105330b8deb86be6f384ad8d7b is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 568 cycles (measured, rsk-hil release-timing record, 77397 runs, max 454 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2288 cycles (measured, rsk-hil release-timing record, 76497 runs, max 1830 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3615 cycles (measured, rsk-hil release-timing record, 58445 runs, max 2892 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4115 cycles (measured, rsk-hil release-timing record, 55520 runs, max 3292 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6185 cycles (measured, rsk-hil release-timing record, 64728 runs, max 4948 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2787 cycles (measured, rsk-hil release-timing record, 11948 runs, max 2229 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 33064 cycles (measured, rsk-hil release-timing record, 6858432 runs, max 26451 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 26451 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 49064 (longest 26451), timer 32543 (longest 19325), job_end 6743935 (longest 2372), epilogue 32888 (longest 12499), panic 2 (longest 6888); the longest single Rust half 4948) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6185 cycles (measured, rsk-hil release-timing record, 64728 runs, max 4948 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6185 cycles (measured, rsk-hil release-timing record, 2925 runs, max 4948 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11302 cycles (measured, rsk-hil release-timing record, 40495 runs, max 9041 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 294 cycles (measured, rsk-hil release-timing record, 6604120 runs, max 235 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 903 cycles (measured, rsk-hil release-timing record, 8999 runs, max 722 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 413 cycles (measured, rsk-hil release-timing record, 72155 runs, max 330 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6185 cycles (measured, rsk-hil release-timing record, 900 runs, max 4948 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 97 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.454 | 10.000 | 1.662 | 18000 | within |
| `part0.echo` | A | 11.776 | 20.000 | 0.292 | 5999 | within |
| `part0.sensor` | A | 13.958 | 20.000 | 6.871 | 8999 | within |
| `part1.log` | C | 40.170 | 40.000 | 4.310 | 3375 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 14.902 | 15.000 | 7.051 | 11947 | within |
| `part2.filter` | B | 17.559 | 25.000 | 7.517 | 6300 | within |
| `part2.sampler` | B | 99.897 | 100.000 | 12.759 | 900 | within |

Verdict: **pass**
