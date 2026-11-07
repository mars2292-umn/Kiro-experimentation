# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 180 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.8 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 202 s (operating duration 180 s) |
| release timing (R9.10) | pass | 40495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8815 cycles (137.7 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 18000 Jobs, response at most 1.610 ms (103040 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 5999 Jobs, response at most 0.276 ms (17660 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 8999 Jobs, response at most 6.833 ms (437332 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 180 s: 11951 intakes against at most 12001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2145 cycles (33.5 µs), mean 1948 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2020 % of the run |
| Kernel events | pass | 6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 0 wraps in the run; the trace's last tick carries high word 0 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 11950 response edges inside the run (target counted 11950) against 12001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.5 % of the run, the Kernel 44.2 % over 1108001 entries; runs per Task [18074, 6000, 20999, 5483, 12049, 7401, 2147] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 22610 cycles (0.353 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 22400 cycles (0.350 ms at 100000 samples/s; 18074 runs for 18000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:237 | 10_000_000 | 678 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:275 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:30 | 100_000_000 | 0 | not run |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:226 | 10_000_000 | 342 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:325 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 17999 | 8815 / 137.7 | 3467 | 2^12:17828 2^13:163 2^14:8 |
| 2 | 8999 | 8295 / 129.6 | 3210 | 2^12:8892 2^13:105 2^14:2 |
| 3 | 4499 | 7971 / 124.5 | 3521 | 2^12:4442 2^13:57 |
| 5 | 7199 | 8815 / 137.7 | 3529 | 2^12:7131 2^13:61 2^14:7 |
| 6 | 1799 | 8815 / 137.7 | 3836 | 2^12:1774 2^13:23 2^14:2 |

Releases recorded: 40495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 18000 | 1.610 | 0 | 0 | 0 |
| 1 | 5999 | 0.276 | 0 | 0 | 0 |
| 2 | 8999 | 6.833 | 0 | 0 | 0 |
| 3 | 3375 | 4.238 | 0 | 0 | 0 |
| 4 | 11950 | 6.849 | 11951 | 2145 | 1948 |
| 5 | 6300 | 7.417 | 0 | 0 | 0 |
| 6 | 900 | 12.249 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 18000 | 22610 / 0.353 (min 8717, mean 11972) | 0 above the Budget; 2^14:17810 2^15:190 |
| Task 1 entry | 5999 | 9381 / 0.147 (min 2944, mean 3057) | 0 above the Budget; 2^12:5982 2^13:0 2^14:17 |
| Task 2 entry | 8999 | 408946 / 6.390 (min 397942, mean 399577) | 0 above the Budget; 2^15:8999 |
| Task 3 entry | 3375 | 204394 / 3.194 (min 190802, mean 195483) | 0 above the Budget; 2^15:3375 |
| Task 4 entry | 11950 | 12549 / 0.196 (min 3006, mean 3142) | 0 above the Budget; 2^12:11696 2^13:174 2^14:80 |
| Task 5 entry | 6300 | 34117 / 0.533 (min 20053, mean 25894) | 0 above the Budget; 2^15:6300 |
| Task 6 entry | 900 | 39587 / 0.619 (min 17116, mean 26894) | 0 above the Budget; 2^15:900 |
| Task 0 on Resource 0 | 18000 | 9896 / 0.155 | |
| Task 0 on Resource 1 | 18000 | 10049 / 0.157 | |
| Task 2 on Resource 0 | 8999 | 391210 / 6.113 | |
| Task 2 on Resource 1 | 17998 | 6414 / 0.100 | |
| Task 5 on Resource 2 | 12600 | 25054 / 0.391 | |
| Task 6 on Resource 2 | 900 | 8028 / 0.125 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 58448 | 2808 / 43.88 | 2597 |
| timer | 64456 | 4754 / 74.28 | 2976 |
| intake | 11951 | 2258 / 35.28 | 2052 |
| lock | 77397 | 461 / 7.20 | 459 |
| unlock | 76497 | 1802 / 28.16 | 1631 |
| job_end | 55523 | 2037 / 31.83 | 1782 |
| now | 8052762 | 241 / 3.77 | 239 |
| raise | 6000 | 2059 / 32.17 | 1980 |
| dma | 8999 | 704 / 11.00 | 543 |
| epilogue | 58448 | 1226 / 19.16 | 589 |
| restart | 900 | 4754 / 74.28 | 4620 |
| enforce | 2925 | 4754 / 74.28 | 3614 |
| view | 72073 | 328 / 5.12 | 293 |

Entries that interrupted an executing Job: Task 0: 11582, Task 1: 34, Task 2: 30797, Task 3: 5366, Task 4: 355, Task 5: 2479, Task 6: 3235

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 179930.000 ms, 3482 cycles
- Task 0 at 179940.000 ms, 3123 cycles
- Task 2 at 179945.000 ms, 3131 cycles
- Task 0 at 179950.000 ms, 3782 cycles
- Task 5 at 179950.000 ms, 3782 cycles
- Task 0 at 179960.000 ms, 3359 cycles
- Task 3 at 179960.000 ms, 3359 cycles
- Task 2 at 179965.000 ms, 3131 cycles
- Task 0 at 179970.000 ms, 3510 cycles
- Task 5 at 179975.000 ms, 3127 cycles
- Task 0 at 179980.000 ms, 3127 cycles
- Task 0 at 179990.000 ms, 3015 cycles

## The Kernel's events

6755 events mirrored (512 held, 2 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 67; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 318321 | 18074 | 0.350 |
| 1 | 22742 | 6000 | 0.140 |
| 2 | 5666216 | 20999 | 6.260 |
| 3 | 1484150 | 5483 | 4.110 |
| 4 | 45928 | 12049 | 0.140 |
| 5 | 325044 | 7401 | 0.900 |
| 6 | 112457 | 2147 | 0.880 |
| none | 12215350 | 44824 | 14346.870 |

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
pulse=11950
echo=5999
blocked_releases=8999
filter=0
filter_state_bits=0
sampler=0
dma_ok=0
loop:part0:spin_ms:spin=0
loop:part0:sensor:hold=678
loop:part0:sensor:late=0
loop:part1:spin_ms:spin=0
loop:part1:log:busy=342
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
  detail=51968
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
  detail=256546
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
  detail=51951
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
  detail=256740
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
  detail=51900
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
  detail=51951
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
  detail=256740
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
  detail=51968
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
  at_ms=179776
  detail=52289
  prs_lo=134217728
  prs_hi=0
event=16
  task=255
  at_ms=179776
  detail=6
  prs_lo=0
  prs_hi=0
event=11
  task=5
  at_ms=179776
  detail=1
  prs_lo=128
  prs_hi=0
event=6
  task=3
  at_ms=179804
  detail=256800
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=179804
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=179901
  detail=51968
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
  detail=256740
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
  detail=51951
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
     Finished in 7.39s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress_short.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress-short)` (sha256:c0dda2dcef5fe5a49322543c331dc0678cd93164fe8bcd7d05ed67f3c1a114b0) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:43bf3c0e2a0625c9699cbb205da250beb2b25c9eca7a28962c2dabeedca0da0d is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 580 cycles (measured, rsk-hil release-timing record, 634 runs, max 464 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2253 cycles (measured, rsk-hil release-timing record, 152994 runs, max 1802 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3510 cycles (measured, rsk-hil release-timing record, 116896 runs, max 2808 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4079 cycles (measured, rsk-hil release-timing record, 111046 runs, max 3263 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5943 cycles (measured, rsk-hil release-timing record, 128912 runs, max 4754 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2682 cycles (measured, rsk-hil release-timing record, 23902 runs, max 2145 +25%, scenarios: nrf52840_stress_short (180 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 6578 cycles (measured, rsk-hil release-timing record, 215718 runs, max 5262 +25%, scenarios: nrf52840_cycles (2 s), conditions: the longest Kernel entry of the run (every entry masks preemption, DD-01): the largest Rust half of any kind plus the stub's measured 1800 cycles outside the charge points (Δ_entry) (task 19.3; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5943 cycles (measured, rsk-hil release-timing record, 128912 runs, max 4754 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 5943 cycles (measured, rsk-hil release-timing record, 5850 runs, max 4754 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11034 cycles (measured, rsk-hil release-timing record, 80990 runs, max 8827 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 303 cycles (measured, rsk-hil release-timing record, 135838 runs, max 242 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 890 cycles (measured, rsk-hil release-timing record, 40 runs, max 712 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 410 cycles (measured, rsk-hil release-timing record, 1401 runs, max 328 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 5943 cycles (measured, rsk-hil release-timing record, 1800 runs, max 4754 +25%, scenarios: nrf52840_stress_short (180 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2250 cycles (measured, rsk-hil release-timing record, 440 runs, max 1800 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 144 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.436 | 10.000 | 1.610 | 18000 | within |
| `part0.echo` | A | 11.729 | 20.000 | 0.276 | 5999 | within |
| `part0.sensor` | A | 13.529 | 20.000 | 6.833 | 8999 | within |
| `part1.log` | C | 39.519 | 40.000 | 4.238 | 3375 | within |
| `part1.pulse` | C | 14.484 | 15.000 | 6.849 | 11950 | within |
| `part2.filter` | B | 17.201 | 25.000 | 7.417 | 6300 | within |
| `part2.sampler` | B | 78.662 | 100.000 | 12.249 | 900 | within |

Verdict: **pass**
