# HIL stress run: Overrun and interrupt-storm fault injection (task 7.5, R18.8, R19.3)

Verdict: **pass**

Operating duration 3660 s on the target 1366:1060:001050217737 with the stimulus board 1366:1061:001050279833; analyzer fx2lafw at 100000 Hz; J_rel bound 12500 cycles.

## Steps

| Step | Result | Detail |
|---|---|---|
| stimulus running | pass | flashed in 5.4 s and reset |
| EasyDMA traffic (R17.4) | pass | SPIM0 of part2 chaining 64-byte transfers on both channels at 8 Mbps: a transfer ended before 0 of 0 sampler Jobs; transfers still running after the Kernel's stop at a restart: 0; ENABLE 7, SHORTS 0x0, RXD.MAXCNT 64, TXD.MAXCNT 64, END 1, STOPPED 1 at the end |
| loop iterations (R34.9, R35.4) | pass | 8 Application loops in the export, 7 observed: 0 reached the bound, 0 exceeded, 2 below the bound with the justification recorded, 0 below without one, 5 never completed in this configuration, 1 not observed |
| target run | pass | RSK-OK after 3684 s (operating duration 3660 s) |
| release timing (R9.10) | pass | 823495 periodic Releases recorded over 5 Tasks; every nominal instant O + k*T: yes; lag max 9817 cycles (153.4 µs) against J_rel 12500 cycles |
| WCRT of the protected Partition (R18.8) | pass | part0.control: 366000 Jobs, response at most 0.653 ms (41824 cycles) against WCRT 8.934 ms (571800 cycles); part0.echo: 121999 Jobs, response at most 0.355 ms (22700 cycles) against WCRT 9.341 ms (597800 cycles); part0.sensor: 182999 Jobs, response at most 5.881 ms (376412 cycles) against WCRT 9.489 ms (607300 cycles) |
| interrupt storm (R19.3) | pass | Task 4's source stormed for 3660 s: 242936 intakes against at most 244001 MIT windows of 15 ms (99.6 % of the windows); intake handler at most 2332 cycles (36.4 µs), mean 2109 cycles: the measured Δ_irq (bound 2600 cycles, R13.4); processor time 0.2187 % of the run |
| Kernel events | pass | 137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run |
| counter wraps | pass | the 32-bit time base wraps every 268 s: 13 wraps in the run; the trace's last tick carries high word 13 (instants ordered across the wrap, R9.5) |
| Kernel trace | pass | 512 entries; no interrupt re-entered without a dispatch |
| dispatch deferrals (5.8a) | pass | 0 entry-window occurrence(s) handled (a dispatch vector deferred, or an unwind owed to the stub, 0x1xxxx); 54900 actions recorded (32 held, 0 of them deferrals) |
| masked span (L_kernel) | pass | the longest stretch without a Kernel-level interrupt taken: 24158 cycles (377.5 µs) over 113791184 spans, closed by a dispatch entry; against the record's L_kernel 35000 cycles |
| responses under the storm | pass | 242935 response edges inside the run (target counted 242935) against 244001 MIT windows; 0 stimulus rises on the mirror |
| dispatch timeline | pass | Tasks executing 39.7 % of the run, the Kernel 35.0 % over 20085803 entries; runs per Task [366974, 122055, 427023, 121874, 244541, 150880, 38149] |
| agent against the trace (task 19.1) | pass | Task 0: the Budget accounting's largest Job execution 23943 cycles (0.374 ms, cycle-counter read cost 1 cycles) against the trace pin's longest run 5849600 cycles (91.400 ms at 100000 samples/s; 366974 runs for 366000 Jobs) |

## Loop iterations (R34.9, R35.4)

| Loop | File:line | Bound | Observed | Verdict |
|---|---|---|---|---|
| `loop:part0:spin_ms:spin` | apps/p1-demo/part0/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part0:sensor:hold` | apps/p1-demo/part0/src/lib.rs:389 | 10_000_000 | 424 | unreachable: the Critical_Section's 6 or 12 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part0:sensor:late` | apps/p1-demo/part0/src/lib.rs:427 | 50_000_000 | 0 | not run |
| `loop:part1:spin_ms:spin` | apps/p1-demo/part1/src/lib.rs:33 | 100_000_000 | 0 | not run |
| `loop:part1:log:drain` | apps/p1-demo/part1/src/lib.rs:216 | 4 | - | not observed |
| `loop:part1:log:busy` | apps/p1-demo/part1/src/lib.rs:424 | 10_000_000 | 258 | unreachable: 3 ms by the time base; the 10,000,000-iteration guard is the PR-16 backstop |
| `loop:part2:spin_ms:spin` | apps/p1-demo/part2/src/lib.rs:38 | 100_000_000 | 0 | not run |
| `loop:part2:filter:hold` | apps/p1-demo/part2/src/lib.rs:335 | 10_000_000 | 0 | not run |

## The Kernel's release-timing record

| Task | Releases | Lag max (cycles / µs) | Lag mean (cycles) | Histogram (log2 buckets, first non-empty..last) |
|---|---|---|---|---|
| 0 | 365999 | 9817 / 153.4 | 3683 | 2^12:340068 2^13:25572 2^14:359 |
| 2 | 182999 | 8932 / 139.6 | 3363 | 2^12:174225 2^13:8710 2^14:64 |
| 3 | 91499 | 9224 / 144.1 | 3628 | 2^12:90215 2^13:1147 2^14:137 |
| 5 | 146399 | 9817 / 153.4 | 3720 | 2^12:120198 2^13:26041 2^14:160 |
| 6 | 36599 | 9224 / 144.1 | 3921 | 2^12:36085 2^13:432 2^14:82 |

Releases recorded: 823495; nominal instants not `O + k*T`: 0.

| Task | Jobs completed | Response max (ms) | Intake runs | Intake max (cycles) | Intake mean (cycles) |
|---|---|---|---|---|---|
| 0 | 366000 | 0.653 | 0 | 0 | 0 |
| 1 | 121999 | 0.355 | 0 | 0 | 0 |
| 2 | 182999 | 5.881 | 0 | 0 | 0 |
| 3 | 68625 | 4.361 | 0 | 0 | 0 |
| 4 | 242935 | 5.983 | 242936 | 2332 | 2109 |
| 5 | 128100 | 6.540 | 0 | 0 | 0 |
| 6 | 18300 | 11.840 | 0 | 0 | 0 |

Execution from the Budget accounting (task 19.1; cycle-counter read cost 1 cycles, R34.3):

| Item | Runs | Execution max (cycles / ms) | Completions above the Budget; histogram (log2 buckets) |
|---|---|---|---|
| Task 0 entry | 366000 | 23943 / 0.374 (min 9968, mean 11551) | 0 above the Budget; 2^14:361649 2^15:4351 |
| Task 1 entry | 121999 | 10373 / 0.162 (min 3406, mean 3523) | 0 above the Budget; 2^12:121708 2^13:52 2^14:239 |
| Task 2 entry | 182999 | 348355 / 5.443 (min 336136, mean 338149) | 0 above the Budget; 2^15:182999 |
| Task 3 entry | 68625 | 209265 / 3.270 (min 187757, mean 196767) | 0 above the Budget; 2^15:68625 |
| Task 4 entry | 242935 | 16544 / 0.259 (min 3470, mean 3869) | 0 above the Budget; 2^12:220419 2^13:21072 2^14:1443 2^15:1 |
| Task 5 entry | 128100 | 39555 / 0.618 (min 20795, mean 27750) | 0 above the Budget; 2^15:128100 |
| Task 6 entry | 18300 | 41221 / 0.644 (min 19938, mean 28246) | 0 above the Budget; 2^15:18300 |
| Task 0 on Resource 0 | 366000 | 11024 / 0.172 | |
| Task 0 on Resource 1 | 366000 | 10853 / 0.170 | |
| Task 2 on Resource 0 | 182999 | 328702 / 5.136 | |
| Task 2 on Resource 1 | 365998 | 7572 / 0.118 | |
| Task 5 on Resource 2 | 256200 | 28876 / 0.451 | |
| Task 6 on Resource 2 | 18300 | 9060 / 0.142 | |

Kernel entries by kind (task 19.3, Table 13-1; the Rust half's cycles, the stub's outside are the interrupted Job's):

| Entry | Runs | Max (cycles / µs) | Mean (cycles) |
|---|---|---|---|
| dispatch | 1188434 | 3213 / 50.20 | 2980 |
| timer | 1160188 | 4885 / 76.33 | 3039 |
| intake | 242936 | 2438 / 38.09 | 2211 |
| lock | 1573797 | 455 / 7.11 | 454 |
| unlock | 1555497 | 1867 / 29.17 | 1687 |
| job_end | 1128958 | 2082 / 32.53 | 1803 |
| now | 108606373 | 221 / 3.45 | 218 |
| raise | 122000 | 2111 / 32.98 | 2028 |
| dma | 182999 | 718 / 11.22 | 539 |
| epilogue | 1188433 | 1254 / 19.59 | 616 |
| restart | 18300 | 4885 / 76.33 | 4705 |
| enforce | 59475 | 4885 / 76.33 | 3708 |
| view | 1470345 | 329 / 5.14 | 295 |

Entries that interrupted an executing Job: Task 0: 53540, Task 1: 530, Task 2: 608135, Task 3: 130984, Task 4: 24354, Task 5: 75211, Task 6: 55038

The last recorded Releases (Task, nominal instant, lag):

- Task 0 at 3659930.000 ms, 3893 cycles
- Task 0 at 3659940.000 ms, 3220 cycles
- Task 2 at 3659945.000 ms, 3220 cycles
- Task 0 at 3659950.000 ms, 4261 cycles
- Task 5 at 3659950.000 ms, 4261 cycles
- Task 0 at 3659960.000 ms, 3460 cycles
- Task 3 at 3659960.000 ms, 3460 cycles
- Task 2 at 3659965.000 ms, 4540 cycles
- Task 0 at 3659970.000 ms, 3813 cycles
- Task 5 at 3659975.000 ms, 3228 cycles
- Task 0 at 3659980.000 ms, 3220 cycles
- Task 0 at 3659990.000 ms, 3104 cycles

## The Kernel's masked spans (11.13)

Every stretch without a Kernel-level interrupt taken, 113791184 of them, the longest 24158 cycles (377.5 µs), closed by a dispatch entry:

| Span (cycles) | Spans |
|---|---|
| 256 to 511 (4.0 to 8.0 µs) | 107847018 |
| 512 to 1023 (8.0 to 16.0 µs) | 2179867 |
| 1024 to 2047 (16.0 to 32.0 µs) | 1108562 |
| 2048 to 4095 (32.0 to 64.0 µs) | 1061207 |
| 4096 to 8191 (64.0 to 128.0 µs) | 1307060 |
| 8192 to 16383 (128.0 to 256.0 µs) | 284441 |
| 16384 to 32767 (256.0 to 512.0 µs) | 3029 |

| Closing entry | Spans | Longest (cycles / µs) |
|---|---|---|
| dispatch | 1149891 | 24158 / 377.5 |
| timer | 506208 | 20672 / 323.0 |
| job_end | 111464653 | 5713 / 89.3 |
| epilogue | 670432 | 14277 / 223.1 |

## The Kernel's events

137253 events mirrored (512 held, 0 of them the post-detach reboot's): overrun of Task 3: 85, overrun of Task 5: 68, overrun of Task 6: 68, operating_end of Task none: 1, resource_released of Task 5: 68, partition_restart of Task none: 68, safe_state of Task none: 1, end_job of Task 3: 85, end_job of Task 6: 68; the trace never froze during the run

## Dispatch timeline (trace pins)

| Task | Executing (samples) | Runs | Longest run (ms) |
|---|---|---|---|
| 0 | 6064894 | 366974 | 91.400 |
| 1 | 493055 | 122055 | 0.150 |
| 2 | 97724063 | 427023 | 5.340 |
| 3 | 31505382 | 121874 | 4.650 |
| 4 | 1084908 | 244541 | 0.210 |
| 5 | 6921143 | 150880 | 0.870 |
| 6 | 2465678 | 38149 | 1.000 |
| none | 222144333 | 906522 | 15859.860 |

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
pulse=242935
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
  detail=58369
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
  detail=289196
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
  detail=52435
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
  detail=288913
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
  detail=58305
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
  detail=52435
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
  detail=288632
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
  detail=58552
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
  detail=52323
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
  detail=288950
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
  detail=58369
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
  detail=288913
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
  detail=52435
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
     Finished in 8.05s
```

## Observed response times against the measured WCRTs (task 19.4, R18.7, R29.5)

The Analyzer on this run's records (docs/hil/kernel_timing_nrf52840_stress.json, with docs/hil/kernel_timing_nrf52840_cycles.json, docs/hil/kernel_timing_nrf52840_cycles_fpu.json): exit 1. Statement: MEASURED Kernel_Timing_Parameters (task 19.3; measurement-based with a margin, not a guaranteed bound, R34.6): measured WCET_Records of the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,stress)` (sha256:e8248637159871ef64ce03c447755f703394fc2989175a10f8517789e77c31bf) of the Build_Manifest, not the analysed Flight_Build; the Kernel_Timing_Parameters' binary sha256:d4faae210fcad55506e286ec2510e80a1398acb827111fa97f1d4dd36f9080fc is the HIL build `p1-demo (nrf52840, nrf52840,debug-port,quiet,trace-ram,trace-pins,cycles-fpu)` of the Build_Manifest, not the analysed Flight_Build; delta_lock_in: 569 cycles (measured, rsk-hil release-timing record, 1573797 runs, max 455 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one lock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_lock_out: 2377 cycles (measured, rsk-hil release-timing record, 794 runs, max 1901 +25%, scenarios: nrf52840_cycles (2 s), conditions: one unlock SVC (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dispatch: 4033 cycles (measured, rsk-hil release-timing record, 1054 runs, max 3226 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one dispatch entry: the Job selected, its view loaded, its frame built (the Rust half; the stub's cycles are Δ_entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_complete: 4170 cycles (measured, rsk-hil release-timing record, 1116 runs, max 3336 +25%, scenarios: nrf52840_cycles (2 s), conditions: the Job-end SVC plus the unwind's epilogue (the Rust halves), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_timer: 6107 cycles (measured, rsk-hil release-timing record, 1160188 runs, max 4885 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one time-base entry: every due timed event served (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_irq: 2915 cycles (measured, rsk-hil release-timing record, 242936 runs, max 2332 +25%, scenarios: nrf52840_stress (3660 s), interrupt storm/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: one run of the intake handler for Task 4's source under the interrupt storm (R19.3), the Kernel's own record/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); l_kernel: 30198 cycles (measured, rsk-hil release-timing record, 113791184 runs, max 24158 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the longest masked span of the run measured end to end, from the first Rust half of a Kernel entry to the last one returning to a context in which a Kernel-level interrupt is taken (tail-chained entries and unwinds inside one span; 24158 cycles, closed by a dispatch entry; the spans by closing entry: dispatch 1149891 (longest 24158), timer 506208 (longest 20672), job_end 111464653 (longest 5713), epilogue 670432 (longest 14277); the longest single Rust half 4885) (task 19.3, 11.13; a static cycle bound needs a timing model the loop-bound report of PR-16 does not give)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_save: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's stacking at an exception entry with the context active: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_fp_restore: 103 cycles (measured, rsk-hil release-timing record, 262 runs, max 82 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: the extended frame's unstacking at the return: half of the floating-point context stacking per interrupting entry (163 cycles: filter's excess over control per entry, 2267, less Δ_entry of the plain cycle configuration, 2104, is 163; the FPU Task's 49 Jobs, 132 interrupting entries; task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_detect: 6107 cycles (measured, rsk-hil release-timing record, 1160188 runs, max 4885 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the time-base entry that serves a Budget or deadline event (the Rust half of a timer entry), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_enforce: 6107 cycles (measured, rsk-hil release-timing record, 59475 runs, max 4885 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that recorded an Overrun and applied the Overrun_Response (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); j_rel: 12272 cycles (measured, rsk-hil release-timing record, 823495 runs, max 9817 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: the lag from a periodic nominal instant to the Release becoming effective, the Kernel's own record (R9.10)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_svc: 284 cycles (measured, rsk-hil release-timing record, 98606 runs, max 227 +25%, scenarios: nrf52840_cycles (2 s), conditions: the now SVC, the service with the least work (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_dma_write: 898 cycles (measured, rsk-hil release-timing record, 40 runs, max 718 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one EasyDMA request SVC, validation and the register writes (the Rust half), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_view: 412 cycles (measured, rsk-hil release-timing record, 1404 runs, max 329 +25%, scenarios: nrf52840_cycles_fpu (2 s), conditions: one MPU view load inside a dispatch (its own cycles; inside Δ_dispatch, exported for review, R13.6), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_restart: 6107 cycles (measured, rsk-hil release-timing record, 18300 runs, max 4885 +25%, scenarios: nrf52840_stress (3660 s)/worst-case concurrent EasyDMA traffic (R17.4): SPIM0, the Application's only EasyDMA-capable peripheral (part2), chaining 64-byte transfers on both channels at 8 Mbps for the whole run through the END_START shortcut (0 transfer-ended observations, 0 still running after a stop)/floating-point context preservation (R34.5): triggered by every Job of `filter` (an FPU Task, its context saved and restored around every preemption) and not triggered by the other Tasks' Jobs/preemption at the maximum rate the Task_Model permits (R34.5): every periodic Task released at its period and offset, every sporadic Task at its MIT, the stormed source at the stimulus board's highest rate, conditions: an entry that restarted a Partition (the Rust half, whole), the Kernel's entry record (task 19.3)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included); delta_entry: 2630 cycles (measured, rsk-hil release-timing record, 317 runs, max 2104 +25%, scenarios: nrf52840_cycles (2 s), conditions: the cycles a Kernel entry charges the Job it interrupts outside the charge points: log's excess over control (49 Jobs) spread over the 157 entries that interrupted log's Jobs (the cycle test, R18.1)/HIL build (trace-ram, trace-pins): the instrumentation's cost is included).

Result **FAIL** (utilization 98 %). | Task | Level | WCRT (ms) | Deadline (ms) | Observed max (ms) | Jobs | Verdict |
|---|---|---|---|---|---|---|
| `part0.control` | A | 9.508 | 10.000 | 0.653 | 366000 | within |
| `part0.echo` | A | 11.996 | 20.000 | 0.355 | 121999 | within |
| `part0.sensor` | A | 14.413 | 20.000 | 5.881 | 182999 | within |
| `part1.log` | C | 41.033 | 40.000 | 4.361 | 68625 | not schedulable: no WCRT to hold |
| `part1.pulse` | C | 15.320 | 15.000 | 5.983 | 242935 | not schedulable: no WCRT to hold |
| `part2.filter` | B | 18.286 | 25.000 | 6.540 | 128100 | within |
| `part2.sampler` | B | 101.168 | 100.000 | 11.840 | 18300 | not schedulable: no WCRT to hold |

Verdict: **pass**
