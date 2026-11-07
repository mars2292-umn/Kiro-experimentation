# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 10000 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 4.7 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 7 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 0 not observed |
| target run | pass | RSK-OK after 3684 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 8946 cycles (139.8 µs) against J_rel 10000 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 1.646 ms (105368 cycles) against WCRT 9.887 ms (632800 cycles); part0.echo: 121999 Jobs, response at most 0.326 ms (20876 cycles) against WCRT 11.314 ms (724100 cycles); part0.sensor: 182999 Jobs, response at most 6.851 ms (438448 cycles) against WCRT 11.463 ms (733600 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 243008 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2155 cycles (33.7 µs), mean 1948 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2021 % of the run |
| Kernel events | pass | 137254 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| responses under the storm | pass | 243007 response edges inside the run (target counted 243007) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 44.0 % of the run, the Kernel 37.9 % over 22536690 entries; runs per Task [367630, 122025, 427010, 111298, 244518, 150516, 43938] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 25618 cycles (0.400 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 24960 cycles (0.390 ms at 100000 samples/s; 367630 runs for 366000 Jobs) |

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
| 0 | 365999 | 8946 / 139.8 | 3479 | 2^12:363105 2^13:2671 2^14:223 |
| 2 | 182999 | 8578 / 134.0 | 3214 | 2^12:180855 2^13:2121 2^14:23 |
| 3 | 91499 | 8619 / 134.7 | 3515 | 2^12:90586 2^13:883 2^14:30 |
| 5 | 146399 | 8838 / 138.1 | 3544 | 2^12:144965 2^13:1348 2^14:86 |
| 6 | 36599 | 8815 / 137.7 | 3822 | 2^12:36218 2^13:376 2^14:5 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 1.646 | 0 | 0 | 0 |
| 1 | 121999 | 0.326 | 0 | 0 | 0 |
| 2 | 182999 | 6.851 | 0 | 0 | 0 |
| 3 | 68625 | 4.243 | 0 | 0 | 0 |
| 4 | 243007 | 6.875 | 243008 | 2155 | 1948 |
| 5 | 128100 | 7.421 | 0 | 0 | 0 |
| 6 | 18300 | 12.309 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 25618 / 0.400 (min 8717, mean 11901) | 0 above the Budget; 2^14:362298 2^15:3702 |
| Task 1 entry | 121999 | 9393 / 0.147 (min 2944, mean 3058) | 0 above the Budget; 2^12:121612 2^13:27 2^14:360 |
| Task 2 entry | 182999 | 409410 / 6.397 (min 397942, mean 399690) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 204682 / 3.198 (min 188002, mean 195575) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 243007 | 12549 / 0.196 (min 3006, mean 3130) | 0 above the Budget; 2^12:238351 2^13:3413 2^14:1243 |
| Task 5 entry | 128100 | 37473 / 0.586 (min 20053, mean 25879) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 39671 / 0.620 (min 17116, mean 26986) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 10044 / 0.157 | |
| Task 0 on Resource 1 | 366000 | 10049 / 0.157 | |
| Task 2 on Resource 0 | 182999 | 391375 / 6.115 | |
| Task 2 on Resource 1 | 365998 | 9760 / 0.152 | |
| Task 5 on Resource 2 | 256200 | 25100 / 0.392 | |
| Task 6 on Resource 2 | 18300 | 8028 / 0.125 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188505 | 2808 / 43.88 | 2597 |
| timer | 1310462 | 4798 / 74.97 | 2975 |
| intake | 243008 | 2258 / 35.28 | 2052 |
| lock | 1573797 | 461 / 7.20 | 459 |
| unlock | 1555497 | 1802 / 28.16 | 1631 |
| job_end | 1129030 | 2037 / 31.83 | 1782 |
| now | 163721063 | 241 / 3.77 | 239 |
| raise | 122000 | 2059 / 32.17 | 1980 |
| dma | 182999 | 704 / 11.00 | 543 |
| epilogue | 1188505 | 1226 / 19.16 | 589 |
| restart | 18300 | 4798 / 74.97 | 4619 |
| enforce | 59475 | 4798 / 74.97 | 3614 |
| view | 1465171 | 328 / 5.12 | 293 |

Entries that interrupted an executing Job: Task 0: 227839, Task 1: 747, Task 2: 633341, Task 3: 108492, Task 4: 6176, Task 5: 49962, Task 6: 66398

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3754 cycles
- Task 0 at 3659940.000 ms, 3127 cycles
- Task 2 at 3659945.000 ms, 3131 cycles
- Task 0 at 3659950.000 ms, 3878 cycles
- Task 5 at 3659950.000 ms, 3878 cycles
- Task 0 at 3659960.000 ms, 3359 cycles
- Task 3 at 3659960.000 ms, 3359 cycles
- Task 2 at 3659965.000 ms, 3131 cycles
- Task 0 at 3659970.000 ms, 3514 cycles
- Task 5 at 3659975.000 ms, 3127 cycles
- Task 0 at 3659980.000 ms, 3127 cycles
- Task 0 at 3659990.000 ms, 3047 cycles

## The Kernel's events

137254 events mirrored (512 held, 1 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 67, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6432910 | 367630 | 0.390 |
| 1 | 462641 | 122025 | 0.140 |
| 2 | 115256813 | 427010 | 6.310 |
| 3 | 30189045 | 111298 | 4.110 |
| 4 | 926647 | 244518 | 0.150 |
| 5 | 6605791 | 150516 | 0.900 |
| 6 | 2290345 | 43938 | 0.890 |
| none | 206237216 | 911841 | 16516.790 |

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
pulse=243007
echo=121999
blocked_releases=182999
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
  at_ms=3659301
  detail=51968
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
  detail=257328
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
  detail=51951
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
  detail=256762
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
  detail=51900
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
  detail=51951
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
  detail=256832
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
  detail=51980
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
  detail=51887
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
  at_ms=3659804
  detail=256892
  prs_lo=134217728
  prs_hi=0
event=19
  task=3
  at_ms=3659804
  detail=6
  prs_lo=0
  prs_hi=0
event=6
  task=6
  at_ms=3659901
  detail=51968
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
  detail=256832
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
  detail=51951
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
     Finished in 7.37s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 0. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` (sha256:b61970407b3677dde099b8ad43780e5364b32e059521a7b6a8a2d11b00fc7bd8) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:43bf3c0e2a0625c9699cbb205da250beb2b25c9eca7a28962c2dabeedca0da0d is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 580 cycles (measured, rsk-hil release-timing record, 634 runs, max 464 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2253 cycles (measured, rsk-hil release-timing record, 1555497 runs, max 1802 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 3510 cycles (measured, rsk-hil release-timing record, 1188505 runs, max 2808 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4079 cycles (measured, rsk-hil release-timing record, 1129030 runs, max 3263 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 5998 cycles (measured, rsk-hil release-timing record, 1310462 runs, max 4798 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2694 cycles (measured, rsk-hil release-timing record, 243008 runs, max 2155 +25%, scenarios: nrf52840_stress (3660 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 6578 cycles (measured, rsk-hil release-timing record, 215718 runs, max 5262 +25%, scenarios: nrf52840_cycles (2 s), conditions: the longest Kernel entry of the run (every entry masks preemption, DD-01): the largest Rust half of any kind plus the stub's measured 1800 cycles outside the charge points (Δ_entry) (task 19.3; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 117 cycles (measured, rsk-hil release-timing record, 256 runs, max 93 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (100 cycles: filter's excess over control per entry, 1874, less Δ_entry of the plain cycle configuration, 1800, is 74, below the method's resolution of 100 cycles, which is recorded instead; the FPU Task's 49 Jobs, 125 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 5998 cycles (measured, rsk-hil release-timing record, 1310462 runs, max 4798 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 5998 cycles (measured, rsk-hil release-timing record, 59475 runs, max 4798 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 11183 cycles (measured, rsk-hil release-timing record, 823495 runs, max 8946 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 303 cycles (measured, rsk-hil release-timing record, 135838 runs, max 242 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 890 cycles (measured, rsk-hil release-timing record, 40 runs, max 712 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 410 cycles (measured, rsk-hil release-timing record, 1401 runs, max 328 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 5998 cycles (measured, rsk-hil release-timing record, 18300 runs, max 4798 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2250 cycles (measured, rsk-hil release-timing record, 440 runs, max 1800 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 144 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **PASS** (utilization 96 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.444 | 10.000 | 1.646 | 366000 | within |
| `part0.echo` | A | 11.743 | 20.000 | 0.326 | 121999 | within |
| `part0.sensor` | A | 13.542 | 20.000 | 6.851 | 182999 | within |
| `part1.log` | C | 39.570 | 40.000 | 4.243 | 68625 | within |
| `part1.pulse` | C | 14.500 | 15.000 | 6.875 | 243007 | within |
| `part2.filter` | B | 17.225 | 25.000 | 7.421 | 128100 | within |
| `part2.sampler` | B | 78.765 | 100.000 | 12.309 | 18300 | within |

Verdict: **pass**
